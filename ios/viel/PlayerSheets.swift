import SwiftUI

struct SheetFrame<Content: View>: View {
    let onClose: () -> Void
    @ViewBuilder let content: Content

    var body: some View {
        VStack(spacing: 14) {
            Capsule().fill(Color.white.opacity(0.3)).frame(width: 40, height: 5).padding(.top, 8)
            content
            Button { onClose() } label: {
                Text("Close")
                    .font(.system(size: 17, weight: .medium))
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 6)
            }
            .buttonStyle(.glass)
            .padding(.horizontal, 16)
            .padding(.bottom, 12)
        }
    }
}

struct EpisodesSheet: View {
    let current: Playable
    let onPick: (Playable) -> Void
    let onClose: () -> Void

    @EnvironmentObject private var core: Core
    @State private var season: Int
    @State private var chainId: UInt64
    @State private var episodes: [Episode]
    @State private var title: Title
    @State private var query = ""
    @State private var loading = false

    init(current: Playable, onPick: @escaping (Playable) -> Void, onClose: @escaping () -> Void) {
        self.current = current
        self.onPick = onPick
        self.onClose = onClose
        _season = State(initialValue: current.episode?.season ?? 1)
        _chainId = State(initialValue: current.title.id)
        _episodes = State(initialValue: current.episodes)
        _title = State(initialValue: current.title)
    }

    private var seasonNumbers: [Int] {
        Array(Set(episodes.map(\.season))).sorted()
    }

    private var visible: [Int] {
        let needle = query.trimmingCharacters(in: .whitespaces).lowercased()
        return episodes.indices.filter { index in
            let episode = episodes[index]
            if !current.isAnime, episode.season != season { return false }
            if needle.isEmpty { return true }
            return episode.numberText == needle || episode.name.lowercased().contains(needle)
        }
    }

    var body: some View {
        SheetFrame(onClose: onClose) {
            HStack(spacing: 14) {
                Text(current.isAnime ? title.name : "Season \(season)")
                    .font(.system(size: 28, weight: .bold))
                    .lineLimit(1)
                Spacer()
                HStack(spacing: 8) {
                    Text("Auto next").font(.system(size: 17))
                    Toggle("", isOn: $core.settings.autoplay).labelsHidden().tint(Theme.accent)
                }
            }
            .padding(.horizontal, 16)

            if current.isAnime, current.chain.count > 1 {
                seasonStrip(current.chain.map { SeasonChip(id: $0.id, label: "Season \($0.season)") }, selected: chainId) { chainId = $0 }
            } else if !current.isAnime, seasonNumbers.count > 1 {
                seasonStrip(seasonNumbers.map { SeasonChip(id: UInt64($0), label: "Season \($0)") }, selected: UInt64(season)) { season = Int($0) }
            }

            HStack(spacing: 10) {
                Image(systemName: "magnifyingglass").foregroundStyle(Theme.muted)
                TextField(current.isAnime ? "Episode # or title" : "Search", text: $query)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 12)
            .background(Color.white.opacity(0.05), in: Capsule())
            .overlay(Capsule().stroke(Theme.border, lineWidth: 1))
            .padding(.horizontal, 16)

            ScrollViewReader { proxy in
                ScrollView(showsIndicators: false) {
                    LazyVStack(spacing: 0) {
                        if loading {
                            HStack { Spacer(); ProgressView().tint(.white); Spacer() }.padding()
                        }
                        ForEach(visible, id: \.self) { index in
                            let episode = episodes[index]
                            let playing = chainId == current.title.id && index == current.index
                            EpisodeRow(episode: episode, highlighted: playing, runtime: episode.runtime, subtitle: playing ? "Now playing · \(Format.runtime(episode.runtime) ?? "")" : nil)
                                .id(index)
                                .onTapGesture {
                                    onPick(Playable(title: title, episodes: episodes, index: index, dub: current.dub, startAt: nil, chain: current.chain))
                                }
                        }
                    }
                }
                .onAppear {
                    if current.index >= 0 { proxy.scrollTo(current.index, anchor: .center) }
                }
            }
        }
        .onChange(of: chainId) { _, id in
            Task { await loadChain(id) }
        }
    }

    private func seasonStrip(_ entries: [SeasonChip], selected: UInt64, pick: @escaping (UInt64) -> Void) -> some View {
        ScrollViewReader { proxy in
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 8) {
                    ForEach(entries) { entry in
                        Button {
                            Haptic.select()
                            pick(entry.id)
                        } label: {
                            Text(entry.label)
                                .font(.system(size: 15, weight: .semibold))
                                .foregroundStyle(entry.id == selected ? .black : .white)
                                .padding(.horizontal, 14)
                                .padding(.vertical, 8)
                        }
                        .buttonStyle(.glass)
                        .tint(entry.id == selected ? .white : .clear)
                        .id(entry.id)
                    }
                }
                .padding(.horizontal, 16)
            }
            .onAppear { proxy.scrollTo(selected, anchor: .center) }
        }
    }

    private func loadChain(_ id: UInt64) async {
        guard id != title.id, let entry = current.chain.first(where: { $0.id == id }) else { return }
        loading = true
        defer { loading = false }
        let list: EpisodeList? = try? await core.get("title", ["kind": "anime", "id": String(id), "dub": current.dub ? "1" : "0"])
        guard let list else { return }
        title = Title(id: entry.id, kind: "anime", name: entry.title, year: entry.year, rating: 0, overview: nil, poster: entry.poster ?? current.title.poster, backdrop: current.title.backdrop, genres: current.title.genres, seasons: [])
        episodes = list.episodes
    }
}

struct SeasonChip: Identifiable {
    let id: UInt64
    let label: String
}

struct SettingsSheet: View {
    @ObservedObject var model: PlayerModel
    let item: Playable
    let onSubtitles: () -> Void
    let onClose: () -> Void

    @EnvironmentObject private var core: Core
    @State private var tracks: [AudioTrack] = []
    @State private var note: String?
    @State private var downloadNote: String?

    private let qualities = [720, 1080, 1440, 2160]
    private let rates: [Float] = [0.5, 0.75, 1, 1.25, 1.5, 2]
    private let sleeps = [15, 30, 45, 60, 90]

    var body: some View {
        SheetFrame(onClose: onClose) {
            HStack {
                Text("Settings").font(.system(size: 22, weight: .bold))
                Spacer()
                Button { onClose() } label: { Image(systemName: "xmark").font(.system(size: 18, weight: .semibold)) }
            }
            .foregroundStyle(.white)
            .padding(.horizontal, 20)

            ScrollView(showsIndicators: false) {
                VStack(spacing: 14) {
                    HStack(spacing: 12) {
                        Menu {
                            ForEach(qualities, id: \.self) { quality in
                                Button(quality == 2160 ? "4K" : "\(quality)p") {
                                    core.settings.maxHeight = quality
                                    core.log("settings", "quality \(quality)")
                                    Task { await core.pushSettings() }
                                }
                            }
                        } label: {
                            tile(badge: "HQ", icon: nil, label: "Quality", value: core.settings.maxHeight == 2160 ? "4K" : "\(core.settings.maxHeight)p")
                        }
                        Menu {
                            ForEach(model.streams.indices, id: \.self) { index in
                                Button(model.streams[index].label) { model.switchSource(index) }
                            }
                        } label: {
                            tile(badge: nil, icon: "globe", label: "Server", value: model.stream?.label ?? "—")
                        }
                    }
                    HStack(spacing: 12) {
                        Button { onSubtitles() } label: {
                            tile(badge: nil, icon: "captions.bubble", label: "Subtitles", value: subtitleValue)
                        }
                        Menu {
                            if tracks.isEmpty {
                                Text("Only one audio track")
                            }
                            ForEach(tracks, id: \.lang) { track in
                                Button(track.label) {
                                    core.settings.audioLang = track.lang
                                    core.log("settings", "audio \(track.lang)")
                                    Task {
                                        await core.pushSettings()
                                        model.reattach()
                                    }
                                }
                            }
                            if !core.settings.audioLang.isEmpty {
                                Button("Automatic") {
                                    core.settings.audioLang = ""
                                    Task {
                                        await core.pushSettings()
                                        model.reattach()
                                    }
                                }
                            }
                        } label: {
                            tile(badge: nil, icon: "speaker.wave.2", label: "Audio", value: audioValue)
                        }
                    }
                    Menu {
                        ForEach(rates, id: \.self) { rate in
                            Button(rate == 1 ? "Normal" : "\(rate)x") { model.setRate(rate) }
                        }
                    } label: {
                        row(icon: "gauge.with.needle", label: "Playback speed", value: model.rate == 1 ? "Normal" : "\(model.rate)x")
                    }
                    Menu {
                        Button("Off") { model.setSleep(minutes: nil) }
                        ForEach(sleeps, id: \.self) { minutes in
                            Button("\(minutes) minutes") { model.setSleep(minutes: minutes) }
                        }
                        if !item.isMovie {
                            Button("End of this episode") { model.setSleep(minutes: nil, atEnd: true) }
                        }
                    } label: {
                        row(icon: "moon.zzz", label: "Sleep timer", value: model.sleepLabel)
                    }
                    if item.offline == nil {
                        Button {
                            Task {
                                do {
                                    try await core.download(item)
                                    downloadNote = "Added to downloads"
                                } catch {
                                    downloadNote = error.localizedDescription
                                }
                            }
                        } label: {
                            row(icon: "arrow.down.circle", label: item.isMovie ? "Download this movie" : "Download this episode", value: downloadNote ?? "")
                        }
                    }
                    Divider().overlay(Theme.border)
                    HStack(spacing: 14) {
                        Image(systemName: "waveform.path.ecg").font(.system(size: 20)).foregroundStyle(Theme.soft).frame(width: 28)
                        Text("Stats for nerds").font(.system(size: 20, weight: .semibold))
                        Spacer()
                        Toggle("", isOn: $model.showStats).labelsHidden().tint(.white)
                    }
                    .padding(.horizontal, 6)
                    HStack(spacing: 14) {
                        Image(systemName: "captions.bubble").font(.system(size: 20)).foregroundStyle(Theme.soft).frame(width: 28)
                        Text("Enable subtitles").font(.system(size: 20, weight: .semibold))
                        Spacer()
                        Toggle("", isOn: $model.subsEnabled).labelsHidden().tint(.white)
                    }
                    .padding(.horizontal, 6)
                    Button {
                        model.report()
                        note = "Sent. Thanks, that helps."
                    } label: {
                        row(icon: "exclamationmark.bubble", label: "Report a problem", value: note ?? "")
                    }
                    Text("Quality applies to the next stream that loads.")
                        .font(.system(size: 13))
                        .foregroundStyle(Theme.faint)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
                .padding(.horizontal, 16)
            }
        }
        .task {
            guard let stream = model.stream, !stream.isOffline else { return }
            tracks = await core.audioTracks(for: stream.url)
        }
    }

    private var audioValue: String {
        if tracks.isEmpty { return "Default" }
        if core.settings.audioLang.isEmpty { return "Automatic" }
        return tracks.first { $0.lang == core.settings.audioLang }?.label ?? core.settings.audioLang
    }

    private var subtitleValue: String {
        guard model.subsEnabled, let index = model.subIndex, let stream = model.stream, stream.subs.indices.contains(index) else { return "Off" }
        return stream.subs[index].label
    }

    private func tile(badge: String?, icon: String?, label: String, value: String) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 8) {
                if let badge {
                    Text(badge).font(.system(size: 12, weight: .bold)).padding(.horizontal, 7).padding(.vertical, 3).background(Color.white.opacity(0.14), in: RoundedRectangle(cornerRadius: 5))
                }
                if let icon {
                    Image(systemName: icon).font(.system(size: 18))
                }
                Text(label).font(.system(size: 17))
            }
            .foregroundStyle(Theme.muted)
            Text(value).font(.system(size: 24, weight: .bold)).foregroundStyle(.white).lineLimit(1)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(16)
        .background(Color.white.opacity(0.05), in: RoundedRectangle(cornerRadius: 18, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 18, style: .continuous).stroke(Theme.border, lineWidth: 1))
    }

    private func row(icon: String, label: String, value: String) -> some View {
        HStack(spacing: 14) {
            Image(systemName: icon).font(.system(size: 20)).foregroundStyle(Theme.soft).frame(width: 28)
            Text(label).font(.system(size: 20, weight: .semibold)).foregroundStyle(.white)
            Spacer()
            Text(value).font(.system(size: 16)).foregroundStyle(Theme.muted)
            Image(systemName: "chevron.right").font(.system(size: 15, weight: .semibold)).foregroundStyle(Theme.muted)
        }
        .padding(.horizontal, 6)
        .padding(.vertical, 8)
    }
}

struct SubtitlesSheet: View {
    @ObservedObject var model: PlayerModel
    let onClose: () -> Void

    @EnvironmentObject private var core: Core

    private let sizes: [(String, Double)] = [("Small", 0.85), ("Medium", 1), ("Large", 1.2), ("Huge", 1.45)]

    var body: some View {
        SheetFrame(onClose: onClose) {
            HStack(spacing: 14) {
                Text("Subtitles").font(.system(size: 22, weight: .bold))
                Spacer()
                Button { onClose() } label: { Image(systemName: "xmark").font(.system(size: 18, weight: .semibold)) }
            }
            .foregroundStyle(.white)
            .padding(.horizontal, 20)

            ScrollView(showsIndicators: false) {
                VStack(spacing: 4) {
                    option(label: "Off", tag: "No Subtitles", selected: !model.subsEnabled || model.subIndex == nil) {
                        model.disableSubtitles()
                    }
                    if let stream = model.stream {
                        ForEach(stream.subs.indices, id: \.self) { index in
                            option(label: stream.subs[index].label, tag: "Built-in", selected: model.subsEnabled && model.subIndex == index) {
                                model.subsEnabled = true
                                model.selectSubtitle(index)
                            }
                        }
                    }
                    Divider().overlay(Theme.border).padding(.vertical, 10)
                    VStack(alignment: .leading, spacing: 14) {
                        Text("Style").font(.system(size: 20, weight: .bold)).foregroundStyle(.white)
                        HStack(spacing: 8) {
                            ForEach(sizes, id: \.1) { size in
                                Button {
                                    Haptic.select()
                                    core.settings.subSize = size.1
                                } label: {
                                    Text(size.0)
                                        .font(.system(size: 15, weight: .semibold))
                                        .foregroundStyle(core.settings.subSize == size.1 ? .black : .white)
                                        .frame(maxWidth: .infinity)
                                        .padding(.vertical, 9)
                                }
                                .buttonStyle(.glass)
                                .tint(core.settings.subSize == size.1 ? .white : .clear)
                            }
                        }
                        HStack {
                            Text("Background box").font(.system(size: 17)).foregroundStyle(.white)
                            Spacer()
                            Toggle("", isOn: $core.settings.subBackground).labelsHidden().tint(.white)
                        }
                        VStack(alignment: .leading, spacing: 6) {
                            HStack {
                                Text("Position").font(.system(size: 17)).foregroundStyle(.white)
                                Spacer()
                                Text(core.settings.subLift == 0 ? "Bottom" : "+\(Int(core.settings.subLift))").font(.system(size: 14)).foregroundStyle(Theme.muted)
                            }
                            Slider(value: $core.settings.subLift, in: 0...160, step: 8).tint(.white)
                        }
                        Text("The language you pick is remembered for this show.")
                            .font(.system(size: 13))
                            .foregroundStyle(Theme.faint)
                    }
                    .padding(.horizontal, 6)
                }
                .padding(.horizontal, 16)
            }
        }
    }

    private func option(label: String, tag: String, selected: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack {
                Text(label).font(.system(size: 24, weight: selected ? .semibold : .regular)).foregroundStyle(.white)
                Spacer()
                Tag(text: tag)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 18)
            .background(selected ? Color.white.opacity(0.07) : .clear, in: RoundedRectangle(cornerRadius: 18, style: .continuous))
        }
    }
}
