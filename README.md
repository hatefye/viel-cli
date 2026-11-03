# viel

A terminal client for viel. Search and watch movies, TV, and anime from the
shell with mpv, and download them to your own machine.

viel asks the server which sources carry a title, then streams and downloads
straight from each source's CDN on your PC. A small relay inside the binary
runs on `127.0.0.1`, sends every CDN the headers it expects, unmasks segments
disguised as images, and hands mpv a clean stream. Sources a browser can't
play because of CORS or header checks work here.

## Install

Grab a build from `dist/` or:

```
cargo install --path .
```

Needs [mpv](https://mpv.io) for playback and [ffmpeg](https://ffmpeg.org) for
downloads. On first run viel offers to install whichever is missing.

## Usage

```
viel
```

- **search** movies and TV, pick a season and episode, and it plays in mpv
- **anime** with sub or dub, every provider's episode list merged into one
- **continue** where you left off, synced with your viel account
- **download** a movie, an episode, a range, a season, or everything

Each title is checked against every source at once, and playback starts on
the best one that answers, in this order:

| movies and TV | anime (sub) | anime (dub) |
|---|---|---|
| cno (up to 4K) | AnimeParadise | AnimeKai |
| mapple | AnimeKai | KotoTV |
| vixsrc | KotoTV | LuffyTV |
| vidbolt (TV only) | LuffyTV | |
| vidrock | | |

If a source fails, the next one takes over. Anime intros are skipped
automatically on Linux and macOS. After an episode ends, viel offers the next.

Downloads fetch eight segments at a time, then remux locally into `.mkv` with
the subtitle in your language embedded. A full episode usually takes one to
two minutes.

## Configuration

`~/.config/viel/config.toml`:

```toml
host = "https://viel.tv"
player = "mpv"
download_dir = "/home/you/Videos/viel"
subtitle_lang = "en"
audio = "sub"
```

`VIEL_HOST` and `VIEL_PLAYER` override the file. Everything is also editable
from the settings menu.

## License

MIT
