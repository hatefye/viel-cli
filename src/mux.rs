use anyhow::{anyhow, bail, Result};
use mp4::{
    AacConfig, AudioObjectType, AvcConfig, ChannelConfig, FourCC, MediaConfig, Mp4Config,
    Mp4Sample, Mp4Writer, SampleFreqIndex, TrackConfig, TrackType,
};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

const CLOCK: u32 = 90_000;
const JUMP: i64 = 10 * CLOCK as i64;

pub fn remux(folder: &Path, main: &str) -> Result<String> {
    let text = fs::read_to_string(folder.join(main))?;
    let (video, audio) = if crate::hls::is_master(&text) {
        let audio = text
            .lines()
            .find(|l| l.starts_with("#EXT-X-MEDIA") && l.contains("TYPE=AUDIO"))
            .map(|l| crate::hls::raw_uri(l));
        let video = text
            .lines()
            .find(|l| !l.starts_with('#') && !l.trim().is_empty())
            .ok_or_else(|| anyhow!("master without a variant"))?
            .trim()
            .to_string();
        (video, audio)
    } else {
        (main.to_string(), None)
    };
    let video = Playlist::read(folder, &video)?;
    let audio = audio.map(|a| Playlist::read(folder, &a)).transpose()?;
    let target = folder.join("video.mp4.part");
    if video.init.is_some() {
        if audio.is_some() {
            bail!("fragmented audio rendition");
        }
        concat(folder, &video, &target)?;
    } else {
        write_mp4(folder, &video, audio.as_ref(), &target)?;
    }
    fs::rename(&target, folder.join("video.mp4"))?;
    Ok("video.mp4".into())
}

pub fn cleanup(folder: &Path, keep: &str) {
    let Ok(read) = fs::read_dir(folder) else { return };
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == keep || name == "meta.json" || name == "plan.json" || name.starts_with("sub.") {
            continue;
        }
        if name.ends_with(".ts")
            || name.ends_with(".m3u8")
            || name.ends_with(".m4s")
            || name.ends_with(".aac")
            || name.ends_with("-init.mp4")
            || name.ends_with(".key")
        {
            let _ = fs::remove_file(entry.path());
        }
    }
}

struct Playlist {
    parts: Vec<PathBuf>,
    init: Option<PathBuf>,
}

impl Playlist {
    fn read(folder: &Path, name: &str) -> Result<Self> {
        let text = fs::read_to_string(folder.join(name))?;
        let mut parts = Vec::new();
        let mut init = None;
        for line in text.lines().map(str::trim) {
            if line.starts_with("#EXT-X-KEY") && !line.contains("METHOD=NONE") {
                bail!("encrypted segments");
            }
            if line.starts_with("#EXT-X-MAP") {
                init = Some(folder.join(crate::hls::raw_uri(line)));
            } else if !line.starts_with('#') && !line.is_empty() {
                parts.push(folder.join(line));
            }
        }
        if parts.is_empty() {
            bail!("empty playlist");
        }
        Ok(Playlist { parts, init })
    }
}

fn concat(_folder: &Path, list: &Playlist, target: &Path) -> Result<()> {
    let mut out = BufWriter::new(File::create(target)?);
    if let Some(init) = &list.init {
        out.write_all(&fs::read(init)?)?;
    }
    for part in &list.parts {
        out.write_all(&fs::read(part)?)?;
    }
    out.flush()?;
    Ok(())
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Video,
    Audio,
}

struct Sample {
    kind: Kind,
    dts: i64,
    pts: i64,
    sync: bool,
    data: Vec<u8>,
}

#[derive(Default)]
struct Pes {
    buf: Vec<u8>,
}

#[derive(Default)]
struct Demux {
    pmt: Option<u16>,
    video: Option<u16>,
    audio: Option<u16>,
    pes: std::collections::HashMap<u16, Pes>,
    sps: Option<Vec<u8>>,
    pps: Option<Vec<u8>>,
    aac: Option<(u8, u8, u8)>,
    carry: Vec<u8>,
    audio_next: Option<f64>,
    out: Vec<Sample>,
}

impl Demux {
    fn feed(&mut self, data: &[u8]) -> Result<()> {
        let mut at = data.iter().position(|&b| b == 0x47).unwrap_or(data.len());
        while at + 188 <= data.len() {
            let p = &data[at..at + 188];
            if p[0] != 0x47 {
                at += 1;
                continue;
            }
            at += 188;
            let pusi = p[1] & 0x40 != 0;
            let pid = u16::from(p[1] & 0x1F) << 8 | u16::from(p[2]);
            let afc = (p[3] >> 4) & 3;
            let mut start = 4;
            if afc & 2 != 0 {
                start += 1 + usize::from(p[4]);
            }
            if afc & 1 == 0 || start >= 188 {
                continue;
            }
            let payload = &p[start..];
            if pid == 0 {
                self.pat(payload, pusi);
            } else if Some(pid) == self.pmt {
                self.pmt(payload, pusi)?;
            } else if Some(pid) == self.video || Some(pid) == self.audio {
                if pusi {
                    self.flush(pid)?;
                }
                self.pes.entry(pid).or_default().buf.extend_from_slice(payload);
            }
        }
        Ok(())
    }

    fn section(payload: &[u8], pusi: bool) -> Option<&[u8]> {
        if !pusi {
            return None;
        }
        let skip = 1 + usize::from(payload[0]);
        payload.get(skip..)
    }

    fn pat(&mut self, payload: &[u8], pusi: bool) {
        let Some(s) = Self::section(payload, pusi) else { return };
        if s.len() < 12 || s[0] != 0 {
            return;
        }
        let len = usize::from(s[1] & 0x0F) << 8 | usize::from(s[2]);
        let end = (3 + len).saturating_sub(4).min(s.len());
        let mut i = 8;
        while i + 4 <= end {
            let program = u16::from(s[i]) << 8 | u16::from(s[i + 1]);
            let pid = u16::from(s[i + 2] & 0x1F) << 8 | u16::from(s[i + 3]);
            if program != 0 {
                self.pmt = Some(pid);
                return;
            }
            i += 4;
        }
    }

    fn pmt(&mut self, payload: &[u8], pusi: bool) -> Result<()> {
        let Some(s) = Self::section(payload, pusi) else { return Ok(()) };
        if s.len() < 12 || s[0] != 2 {
            return Ok(());
        }
        let len = usize::from(s[1] & 0x0F) << 8 | usize::from(s[2]);
        let end = (3 + len).saturating_sub(4).min(s.len());
        let info = usize::from(s[10] & 0x0F) << 8 | usize::from(s[11]);
        let mut i = 12 + info;
        while i + 5 <= end {
            let kind = s[i];
            let pid = u16::from(s[i + 1] & 0x1F) << 8 | u16::from(s[i + 2]);
            let es = usize::from(s[i + 3] & 0x0F) << 8 | usize::from(s[i + 4]);
            match kind {
                0x1B if self.video.is_none() => self.video = Some(pid),
                0x0F if self.audio.is_none() => self.audio = Some(pid),
                0x24 => bail!("hevc video"),
                0x03 | 0x04 | 0x11 | 0x81 | 0x87 if self.audio.is_none() => bail!("unsupported audio"),
                _ => {}
            }
            i += 5 + es;
        }
        Ok(())
    }

    fn flush(&mut self, pid: u16) -> Result<()> {
        let Some(pes) = self.pes.get_mut(&pid) else { return Ok(()) };
        let buf = std::mem::take(&mut pes.buf);
        if buf.len() < 9 || buf[0] != 0 || buf[1] != 0 || buf[2] != 1 {
            return Ok(());
        }
        let flags = buf[7] >> 6;
        let header = usize::from(buf[8]);
        let pts = (flags & 2 != 0 && buf.len() >= 14).then(|| stamp(&buf[9..14]));
        let dts = (flags == 3 && buf.len() >= 19).then(|| stamp(&buf[14..19]));
        let body = &buf[(9 + header).min(buf.len())..];
        if Some(pid) == self.video {
            let Some(pts) = pts else { return Ok(()) };
            self.video_unit(body, pts, dts.unwrap_or(pts));
        } else {
            self.audio_frames(body, pts);
        }
        Ok(())
    }

    fn video_unit(&mut self, body: &[u8], pts: i64, dts: i64) {
        let mut data = Vec::with_capacity(body.len() + 32);
        let mut sync = false;
        for nal in nals(body) {
            let kind = nal[0] & 0x1F;
            match kind {
                9 | 12 => continue,
                5 => sync = true,
                7 if self.sps.is_none() => self.sps = Some(nal.to_vec()),
                8 if self.pps.is_none() => self.pps = Some(nal.to_vec()),
                _ => {}
            }
            data.extend_from_slice(&(nal.len() as u32).to_be_bytes());
            data.extend_from_slice(nal);
        }
        if data.is_empty() {
            return;
        }
        self.out.push(Sample { kind: Kind::Video, dts, pts, sync, data });
    }

    fn audio_frames(&mut self, body: &[u8], pts: Option<i64>) {
        if self.carry.is_empty() {
            if let Some(pts) = pts {
                self.audio_next = Some(pts as f64);
            }
        }
        self.carry.extend_from_slice(body);
        let mut i = 0;
        while i + 7 <= self.carry.len() {
            let h = &self.carry[i..];
            if h[0] != 0xFF || h[1] & 0xF0 != 0xF0 {
                i += 1;
                continue;
            }
            let len = usize::from(h[3] & 3) << 11 | usize::from(h[4]) << 3 | usize::from(h[5] >> 5);
            if len < 7 || i + len > self.carry.len() {
                break;
            }
            let header = if h[1] & 1 == 1 { 7 } else { 9 };
            let profile = (h[2] >> 6) + 1;
            let freq = (h[2] >> 2) & 0x0F;
            let chan = (h[2] & 1) << 2 | h[3] >> 6;
            if self.aac.is_none() {
                self.aac = Some((profile, freq, chan));
            }
            let rate = RATES.get(usize::from(freq)).copied().unwrap_or(48_000) as f64;
            let at = self.audio_next.unwrap_or(0.0);
            self.audio_next = Some(at + 1024.0 * CLOCK as f64 / rate);
            let stamp = at.round() as i64;
            if len > header {
                self.out.push(Sample {
                    kind: Kind::Audio,
                    dts: stamp,
                    pts: stamp,
                    sync: true,
                    data: self.carry[i + header..i + len].to_vec(),
                });
            }
            i += len;
        }
        self.carry.drain(..i);
    }

    fn end(&mut self) -> Result<()> {
        let pids: Vec<u16> = self.pes.keys().copied().collect();
        for pid in pids {
            self.flush(pid)?;
        }
        Ok(())
    }
}

const RATES: [u32; 13] = [96_000, 88_200, 64_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025, 8_000, 7_350];

fn stamp(b: &[u8]) -> i64 {
    (i64::from(b[0] >> 1 & 7) << 30)
        | (i64::from(b[1]) << 22)
        | (i64::from(b[2] >> 1) << 15)
        | (i64::from(b[3]) << 7)
        | i64::from(b[4] >> 1)
}

fn nals(body: &[u8]) -> Vec<&[u8]> {
    let mut starts = Vec::new();
    let mut i = 0;
    while i + 3 <= body.len() {
        if body[i] == 0 && body[i + 1] == 0 && body[i + 2] == 1 {
            starts.push((i, i + 3));
            i += 3;
        } else {
            i += 1;
        }
    }
    let mut out = Vec::with_capacity(starts.len());
    for (n, &(_, begin)) in starts.iter().enumerate() {
        let mut end = starts.get(n + 1).map(|s| s.0).unwrap_or(body.len());
        while end > begin && body[end - 1] == 0 {
            end -= 1;
        }
        if end > begin {
            out.push(&body[begin..end]);
        }
    }
    out
}

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Bits<'_> {
    fn bit(&mut self) -> u32 {
        let byte = self.data.get(self.pos / 8).copied().unwrap_or(0);
        let v = (byte >> (7 - self.pos % 8)) & 1;
        self.pos += 1;
        u32::from(v)
    }

    fn bits(&mut self, n: u32) -> u32 {
        (0..n).fold(0, |acc, _| acc << 1 | self.bit())
    }

    fn ue(&mut self) -> u32 {
        let mut zeros = 0;
        while self.bit() == 0 && zeros < 32 {
            zeros += 1;
        }
        (1 << zeros) - 1 + self.bits(zeros)
    }

    fn se(&mut self) -> i32 {
        let v = self.ue() as i32;
        if v & 1 == 1 { (v + 1) / 2 } else { -(v / 2) }
    }
}

fn sps_size(sps: &[u8]) -> Option<(u16, u16)> {
    let mut clean = Vec::with_capacity(sps.len());
    let mut zeros = 0;
    for &b in &sps[1..] {
        if zeros >= 2 && b == 3 {
            zeros = 0;
            continue;
        }
        clean.push(b);
        zeros = if b == 0 { zeros + 1 } else { 0 };
    }
    let mut r = Bits { data: &clean, pos: 0 };
    let profile = r.bits(8);
    r.bits(16);
    r.ue();
    let mut chroma = 1;
    if matches!(profile, 100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 | 138 | 139 | 134 | 135) {
        chroma = r.ue();
        if chroma == 3 {
            r.bit();
        }
        r.ue();
        r.ue();
        r.bit();
        if r.bit() == 1 {
            let lists = if chroma != 3 { 8 } else { 12 };
            for i in 0..lists {
                if r.bit() == 1 {
                    let size = if i < 6 { 16 } else { 64 };
                    let mut last = 8i32;
                    let mut next = 8i32;
                    for _ in 0..size {
                        if next != 0 {
                            next = (last + r.se() + 256) % 256;
                        }
                        last = if next == 0 { last } else { next };
                    }
                }
            }
        }
    }
    r.ue();
    match r.ue() {
        0 => {
            r.ue();
        }
        1 => {
            r.bit();
            r.se();
            r.se();
            let n = r.ue();
            for _ in 0..n {
                r.se();
            }
        }
        _ => {}
    }
    r.ue();
    r.bit();
    let width_mbs = r.ue() + 1;
    let height_units = r.ue() + 1;
    let frame_mbs_only = r.bit();
    if frame_mbs_only == 0 {
        r.bit();
    }
    r.bit();
    let (mut left, mut right, mut top, mut bottom) = (0, 0, 0, 0);
    if r.bit() == 1 {
        left = r.ue();
        right = r.ue();
        top = r.ue();
        bottom = r.ue();
    }
    let (unit_x, unit_y) = match chroma {
        0 => (1, 2 - frame_mbs_only),
        1 => (2, 2 * (2 - frame_mbs_only)),
        2 => (2, 2 - frame_mbs_only),
        _ => (1, 2 - frame_mbs_only),
    };
    let width = width_mbs * 16 - unit_x * (left + right);
    let height = (2 - frame_mbs_only) * height_units * 16 - unit_y * (top + bottom);
    (width > 0 && height > 0).then_some((width as u16, height as u16))
}

struct Track {
    id: u32,
    offset: i64,
    last: Option<Sample>,
    last_duration: u32,
    end: i64,
}

struct Muxer<W: Write + std::io::Seek> {
    out: Mp4Writer<W>,
    video: Option<Track>,
    audio: Option<Track>,
    pending: Vec<Sample>,
    base: Option<i64>,
}

impl<W: Write + std::io::Seek> Muxer<W> {
    fn ready(&self) -> bool {
        self.video.is_some()
    }

    fn setup(&mut self, d: &Demux, want_audio: bool) -> Result<()> {
        if self.ready() {
            return Ok(());
        }
        let (Some(sps), Some(pps)) = (&d.sps, &d.pps) else { return Ok(()) };
        if want_audio && d.aac.is_none() {
            return Ok(());
        }
        let (width, height) = sps_size(sps).ok_or_else(|| anyhow!("unreadable sps"))?;
        self.out.add_track(&TrackConfig {
            track_type: TrackType::Video,
            timescale: CLOCK,
            language: "und".into(),
            media_conf: MediaConfig::AvcConfig(AvcConfig {
                width,
                height,
                seq_param_set: sps.clone(),
                pic_param_set: pps.clone(),
            }),
        })?;
        self.video = Some(Track { id: 1, offset: 0, last: None, last_duration: 3_000, end: 0 });
        if let Some((profile, freq, chan)) = d.aac {
            self.out.add_track(&TrackConfig {
                track_type: TrackType::Audio,
                timescale: CLOCK,
                language: "und".into(),
                media_conf: MediaConfig::AacConfig(AacConfig {
                    bitrate: 128_000,
                    profile: AudioObjectType::try_from(profile).unwrap_or(AudioObjectType::AacLowComplexity),
                    freq_index: SampleFreqIndex::try_from(freq).unwrap_or(SampleFreqIndex::Freq48000),
                    chan_conf: ChannelConfig::try_from(chan).unwrap_or(ChannelConfig::Stereo),
                }),
            })?;
            self.audio = Some(Track { id: 2, offset: 0, last: None, last_duration: 1_920, end: 0 });
        }
        let pending = std::mem::take(&mut self.pending);
        for sample in pending {
            self.push(sample)?;
        }
        Ok(())
    }

    fn take(&mut self, d: &mut Demux) -> Result<()> {
        let samples = std::mem::take(&mut d.out);
        for sample in samples {
            if self.ready() {
                self.push(sample)?;
            } else {
                self.pending.push(sample);
            }
        }
        Ok(())
    }

    fn push(&mut self, mut sample: Sample) -> Result<()> {
        let base = *self.base.get_or_insert(sample.dts);
        sample.dts -= base;
        sample.pts -= base;
        let track = match sample.kind {
            Kind::Video => self.video.as_mut(),
            Kind::Audio => self.audio.as_mut(),
        };
        let Some(track) = track else { return Ok(()) };
        sample.dts += track.offset;
        sample.pts += track.offset;
        if sample.dts < track.end || sample.dts > track.end + JUMP {
            let shift = track.end - sample.dts;
            track.offset += shift;
            sample.dts += shift;
            sample.pts += shift;
        }
        if let Some(prev) = track.last.take() {
            let duration = (sample.dts - prev.dts).clamp(1, JUMP) as u32;
            track.last_duration = duration;
            track.end = sample.dts;
            write(&mut self.out, track.id, prev, duration)?;
        }
        track.last = Some(sample);
        Ok(())
    }

    fn finish(mut self) -> Result<W> {
        for track in [self.video.as_mut(), self.audio.as_mut()].into_iter().flatten() {
            if let Some(prev) = track.last.take() {
                write(&mut self.out, track.id, prev, track.last_duration)?;
            }
        }
        self.out.write_end()?;
        Ok(self.out.into_writer())
    }
}

fn write<W: Write + std::io::Seek>(out: &mut Mp4Writer<W>, id: u32, sample: Sample, duration: u32) -> Result<()> {
    out.write_sample(
        id,
        &Mp4Sample {
            start_time: sample.dts.max(0) as u64,
            duration,
            rendering_offset: (sample.pts - sample.dts).clamp(0, i32::MAX as i64) as i32,
            is_sync: sample.sync,
            bytes: sample.data.into(),
        },
    )?;
    Ok(())
}

fn write_mp4(_folder: &Path, video: &Playlist, audio: Option<&Playlist>, target: &Path) -> Result<()> {
    let file = BufWriter::with_capacity(1 << 20, File::create(target)?);
    let out = Mp4Writer::write_start(
        file,
        &Mp4Config {
            major_brand: FourCC::from(*b"isom"),
            minor_version: 512,
            compatible_brands: vec![FourCC::from(*b"isom"), FourCC::from(*b"iso2"), FourCC::from(*b"avc1"), FourCC::from(*b"mp41")],
            timescale: 1000,
        },
    )?;
    let mut mux = Muxer { out, video: None, audio: None, pending: Vec::new(), base: None };
    let mut dv = Demux::default();
    let mut da = Demux::default();
    let count = video.parts.len().max(audio.map(|a| a.parts.len()).unwrap_or(0));
    for i in 0..count {
        if let Some(part) = video.parts.get(i) {
            dv.feed(&fs::read(part)?)?;
            dv.end()?;
            if audio.is_some() && dv.aac.is_none() {
                dv.aac = da.aac;
            }
            mux.setup(&dv, audio.is_some())?;
            mux.take(&mut dv)?;
        }
        if let Some(part) = audio.and_then(|a| a.parts.get(i)) {
            let bytes = fs::read(part)?;
            if part.extension().map(|e| e == "aac").unwrap_or(false) {
                da.audio_frames(&bytes, None);
            } else {
                da.feed(&bytes)?;
                da.end()?;
            }
            if dv.aac.is_none() {
                dv.aac = da.aac;
            }
            mux.setup(&dv, true)?;
            mux.take(&mut da)?;
        }
    }
    if !mux.ready() {
        bail!("no h264 video found");
    }
    let mut file = mux.finish()?;
    file.flush()?;
    Ok(())
}
