import SwiftUI

struct RemoteDevice: Decodable, Identifiable, Hashable {
    let id: String
    let name: String
    let title: String
    let kind: String
    let season: Int
    let episode: Int
    let show: String
    let poster: String
    let video: Bool
    let paused: Bool
    let duration: Double
    let position: Double
    let volume: Double
    let muted: Bool

    var heading: String {
        if !show.isEmpty { return show }
        return kind.isEmpty ? "Nothing playing" : title
    }

    var detail: String {
        switch kind {
        case "tv": return "Season \(season) · Episode \(episode)"
        case "anime": return episode > 0 ? "Episode \(episode)" : "Anime"
        case "movie": return "Movie"
        default: return "Browsing viel.tv"
        }
    }
}

@MainActor
final class Remote: ObservableObject {
    static let shared = Remote()

    @Published private(set) var devices: [RemoteDevice] = []
    @Published private(set) var loaded = false
    @Published var chosen: String? {
        didSet { UserDefaults.standard.set(chosen, forKey: "remote.device") }
    }

    private init() {
        chosen = UserDefaults.standard.string(forKey: "remote.device")
    }

    var target: RemoteDevice? {
        devices.first { $0.id == chosen } ?? devices.first
    }

    func refresh(core: Core) async {
        struct Reply: Decodable {
            let devices: [RemoteDevice]
        }
        guard core.username != nil else {
            devices = []
            loaded = true
            return
        }
        let reply: Reply? = try? await core.get("remote")
        if let reply {
            devices = reply.devices
        }
        loaded = true
    }

    @discardableResult
    func send(_ command: String, value: Double? = nil, path: String? = nil) async -> String? {
        struct Body: Encodable {
            let device: String
            let cmd: String
            let value: Double?
            let path: String?
        }
        guard let device = target else { return "No browser is connected." }
        do {
            let _: Ok = try await Core.shared.post("remote", Body(device: device.id, cmd: command, value: value, path: path))
            return nil
        } catch {
            Core.shared.log("remote", "\(command) failed: \(error.localizedDescription)")
            return error.localizedDescription
        }
    }

    static func path(kind: String, id: UInt64, season: Int, episode: Int) -> String {
        switch kind {
        case "tv": return "/tv/\(id)/\(max(season, 1))/\(max(episode, 1))"
        case "anime": return "/anime/\(id)/\(max(episode, 1))"
        default: return "/movies/\(id)"
        }
    }
}

struct RemoteView: View {
    @EnvironmentObject private var core: Core
    @ObservedObject private var remote = Remote.shared
    @State private var scrub: Double?
    @State private var level: Double?
    @State private var query = ""
    @State private var results: [Title] = []
    @State private var resume: [Resume] = []
    @State private var note: String?

    private var trimmed: String { query.trimmingCharacters(in: .whitespaces) }

    var body: some View {
        ZStack {
            Theme.bg.ignoresSafeArea()
            ScrollView(showsIndicators: false) {
                VStack(alignment: .leading, spacing: 26) {
                    if let device = remote.target {
                        if remote.devices.count > 1 {
                            picker(device)
                        }
                        playing(device)
                        transport(device)
                        volume(device)
                        if let note {
                            Text(note).font(.system(size: 14)).foregroundStyle(Theme.accentHi)
                        }
                        library(device)
                    } else if remote.loaded {
                        empty
                    } else {
                        HStack { Spacer(); ProgressView().tint(.white); Spacer() }.padding(.top, 60)
                    }
                }
                .padding(.horizontal, 18)
                .padding(.top, 12)
                .padding(.bottom, 40)
            }
            .scrollDismissesKeyboard(.immediately)
        }
        .navigationTitle("Remote")
        .navigationBarTitleDisplayMode(.inline)
        .toolbarBackground(Theme.bg, for: .navigationBar)
        .task {
            while !Task.isCancelled {
                await remote.refresh(core: core)
                try? await Task.sleep(nanoseconds: 1_000_000_000)
            }
        }
        .task {
            let home: HomeData? = try? await core.get("home")
            resume = home?.resume ?? []
        }
        .task(id: trimmed) { await search() }
    }

    private var empty: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("No browser found").font(.system(size: 22, weight: .bold))
            Text("Open viel.tv on your computer and sign in as \(core.username ?? "the same account"). It shows up here within a few seconds, whether something is playing or not.")
                .font(.system(size: 15))
                .foregroundStyle(Theme.muted)
        }
        .padding(.top, 30)
    }

    private func picker(_ device: RemoteDevice) -> some View {
        Menu {
            ForEach(remote.devices) { other in
                Button {
                    remote.chosen = other.id
                } label: {
                    Label("\(other.name) · \(other.heading)", systemImage: other.id == device.id ? "checkmark" : "")
                }
            }
        } label: {
            HStack(spacing: 6) {
                Image(systemName: "desktopcomputer").font(.system(size: 13))
                Text(device.name).font(.system(size: 14, weight: .medium))
                Image(systemName: "chevron.down").font(.system(size: 11, weight: .semibold))
            }
            .foregroundStyle(Theme.soft)
            .padding(.horizontal, 12)
            .padding(.vertical, 7)
            .glassEffect(.regular, in: .capsule)
        }
    }

    private func playing(_ device: RemoteDevice) -> some View {
        HStack(alignment: .bottom, spacing: 16) {
            Poster(url: device.poster, width: 104, height: 156, radius: 12)
            VStack(alignment: .leading, spacing: 5) {
                if remote.devices.count <= 1 {
                    Text(device.name.uppercased())
                        .font(.system(size: 11, weight: .semibold))
                        .tracking(0.8)
                        .foregroundStyle(Theme.faint)
                }
                Text(device.heading)
                    .font(.system(size: 22, weight: .bold))
                    .lineLimit(3)
                    .minimumScaleFactor(0.85)
                Text(device.detail).font(.system(size: 14)).foregroundStyle(Theme.muted)
            }
            Spacer(minLength: 0)
        }
    }

    private func transport(_ device: RemoteDevice) -> some View {
        let length = max(device.duration, 1)
        let shown = min(scrub ?? device.position, length)
        return VStack(spacing: 14) {
            VStack(spacing: 2) {
                Slider(value: Binding(get: { shown }, set: { scrub = $0 }), in: 0...length) { editing in
                    guard !editing, let to = scrub else { return }
                    Task {
                        note = await remote.send("seek", value: to)
                        try? await Task.sleep(nanoseconds: 1_300_000_000)
                        scrub = nil
                    }
                }
                .tint(Theme.accent)
                HStack {
                    Text(Format.clock(shown))
                    Spacer()
                    Text(Format.clock(device.duration))
                }
                .font(.system(size: 12))
                .monospacedDigit()
                .foregroundStyle(Theme.muted)
            }
            HStack(spacing: 30) {
                key("gobackward.10", size: 26) { await remote.send("skip", value: -10) }
                Button {
                    Haptic.tap()
                    Task {
                        note = await remote.send(device.paused ? "play" : "pause")
                        await remote.refresh(core: core)
                    }
                } label: {
                    Image(systemName: device.paused ? "play.fill" : "pause.fill")
                        .font(.system(size: 28))
                        .foregroundStyle(.black)
                        .frame(width: 70, height: 70)
                        .background(.white, in: Circle())
                }
                .buttonStyle(.plain)
                key("goforward.10", size: 26) { await remote.send("skip", value: 10) }
                if device.kind == "tv" || device.kind == "anime" {
                    key("forward.end.fill", size: 22) { await remote.send("next") }
                }
            }
            .frame(maxWidth: .infinity)
        }
        .disabled(!device.video)
        .opacity(device.video ? 1 : 0.4)
    }

    private func key(_ icon: String, size: CGFloat, action: @escaping () async -> String?) -> some View {
        Button {
            Haptic.tap()
            Task {
                note = await action()
                try? await Task.sleep(nanoseconds: 250_000_000)
                await remote.refresh(core: core)
            }
        } label: {
            Image(systemName: icon)
                .font(.system(size: size, weight: .medium))
                .foregroundStyle(.white)
                .frame(width: 50, height: 50)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    private func volume(_ device: RemoteDevice) -> some View {
        HStack(spacing: 12) {
            Button {
                Task {
                    note = await remote.send("mute", value: device.muted ? 0 : 1)
                    await remote.refresh(core: core)
                }
            } label: {
                Image(systemName: device.muted || device.volume == 0 ? "speaker.slash.fill" : "speaker.wave.2.fill")
                    .font(.system(size: 16))
                    .foregroundStyle(Theme.soft)
                    .frame(width: 34, height: 34)
            }
            .buttonStyle(.plain)
            Slider(value: Binding(get: { level ?? (device.muted ? 0 : device.volume) }, set: { level = $0 }), in: 0...1) { editing in
                guard !editing, let to = level else { return }
                Task {
                    note = await remote.send("volume", value: to)
                    try? await Task.sleep(nanoseconds: 1_300_000_000)
                    level = nil
                }
            }
            .tint(.white)
        }
        .disabled(!device.video)
        .opacity(device.video ? 1 : 0.4)
    }

    private func library(_ device: RemoteDevice) -> some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Play on \(device.name)").font(.system(size: 20, weight: .bold))
            HStack(spacing: 10) {
                Image(systemName: "magnifyingglass").foregroundStyle(Theme.muted)
                TextField("Search movies, shows, anime…", text: $query)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .submitLabel(.search)
                if !query.isEmpty {
                    Button { query = "" } label: { Image(systemName: "xmark.circle.fill").foregroundStyle(Theme.muted) }
                }
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 11)
            .background(Color.white.opacity(0.06), in: RoundedRectangle(cornerRadius: 14, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 14, style: .continuous).stroke(Theme.border, lineWidth: 1))

            if trimmed.isEmpty {
                if !resume.isEmpty {
                    Text("Continue watching").font(.system(size: 13, weight: .semibold)).foregroundStyle(Theme.muted)
                }
                ForEach(resume, id: \.key) { row in
                    entry(poster: row.poster, name: row.show, caption: row.episodeLabel ?? "Movie") {
                        await open(kind: row.kind, id: row.id, season: row.season, episode: row.episode)
                    }
                }
            } else {
                ForEach(results.prefix(20), id: \.key) { title in
                    entry(poster: title.poster, name: title.name, caption: [title.kindLabel, title.year].filter { !$0.isEmpty }.joined(separator: " · ")) {
                        let last = title.isMovie ? nil : await core.lastRow(kind: title.kind, id: title.id)
                        await open(kind: title.kind, id: title.id, season: last?.season ?? 1, episode: last?.episode ?? 1)
                    }
                }
            }
        }
        .padding(.top, 6)
    }

    private func entry(poster: String?, name: String, caption: String, action: @escaping () async -> Void) -> some View {
        Button {
            Haptic.tap()
            Task { await action() }
        } label: {
            HStack(spacing: 12) {
                Poster(url: poster, width: 44, height: 66, radius: 7)
                VStack(alignment: .leading, spacing: 3) {
                    Text(name).font(.system(size: 16, weight: .medium)).foregroundStyle(.white).lineLimit(1)
                    Text(caption).font(.system(size: 13)).foregroundStyle(Theme.muted).lineLimit(1)
                }
                Spacer(minLength: 8)
                Image(systemName: "play.fill").font(.system(size: 13)).foregroundStyle(Theme.faint)
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    private func open(kind: String, id: UInt64, season: Int, episode: Int) async {
        note = await remote.send("open", path: Remote.path(kind: kind, id: id, season: season, episode: episode))
        try? await Task.sleep(nanoseconds: 1_500_000_000)
        await remote.refresh(core: core)
    }

    private func search() async {
        let q = trimmed
        guard !q.isEmpty else {
            results = []
            return
        }
        try? await Task.sleep(nanoseconds: 300_000_000)
        guard !Task.isCancelled else { return }
        let found: [Title] = (try? await core.get("search", ["q": q])) ?? []
        guard !Task.isCancelled else { return }
        results = found
    }
}
