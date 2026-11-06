import SwiftUI
import UIKit

struct MainView: View {
    @EnvironmentObject private var core: Core
    @ObservedObject private var shortcuts = Shortcuts.shared
    @State private var tab = "home"
    @State private var notes = false
    @State private var release: AppRelease?
    @State private var showRelease = false
    @AppStorage("seenBuild") private var seenBuild = ""
    @Environment(\.openURL) private var openURL

    var body: some View {
        TabView(selection: $tab) {
            Tab("Home", systemImage: "house", value: "home") {
                RootView()
            }
            Tab("Movies", systemImage: "film", value: "movies") {
                NavigationStack {
                    ListView(page: ListPage(title: "Movies", path: "movie/popular"))
                }
            }
            Tab("Shows", systemImage: "tv", value: "shows") {
                NavigationStack {
                    ListView(page: ListPage(title: "Shows", path: "tv/popular"))
                }
            }
            Tab("Anime", systemImage: "sparkles", value: "anime") {
                NavigationStack {
                    ListView(page: ListPage(title: "Anime", path: "anime/trending"))
                }
            }
            Tab("My List", systemImage: "star", value: "list") {
                MyListView()
            }
            Tab("Settings", systemImage: "gearshape", value: "settings") {
                AccountView(embedded: true)
            }
            Tab("Search", systemImage: "magnifyingglass", value: "search", role: .search) {
                SearchView(inTab: true)
            }
        }
        .tabBarMinimizeBehavior(.onScrollDown)
        .tint(.white)
        .onAppear {
            consume(shortcuts.pending)
            if seenBuild.isEmpty {
                seenBuild = core.build
            } else if seenBuild != core.build, !ReleaseNotes.current.isEmpty {
                notes = true
            }
            Task {
                if let found = await Updates.check(core: core) {
                    release = found
                    showRelease = true
                }
            }
        }
        .alert("viel \(release?.version ?? "") is available", isPresented: $showRelease, presenting: release) { found in
            Button("Download") { openURL(URL(string: found.url ?? "https://viel.tv/desktop") ?? URL(string: "https://viel.tv/desktop")!) }
            Button("Skip this version") { Updates.skip(found.version) }
            Button("Later", role: .cancel) {}
        } message: { found in
            Text(found.notes ?? "Install it with Sideloadly the same way as before.")
        }
        .sheet(isPresented: $notes, onDismiss: { seenBuild = core.build }) {
            WhatsNewView(build: core.build).environmentObject(core)
        }
        .onChange(of: shortcuts.link) { _, value in if value != nil { tab = "home" } }
        .onChange(of: shortcuts.pending) { _, value in consume(value) }
        .onOpenURL { url in
            core.log("app", "open url \(url.absoluteString)")
            if let link = DeepLink(url) {
                tab = "home"
                shortcuts.link = link
            }
        }
    }

    private func consume(_ action: String?) {
        guard let action else { return }
        switch action {
        case Shortcuts.search: tab = "search"
        case Shortcuts.list: tab = "list"
        case Shortcuts.home: tab = "home"
        case Shortcuts.resume:
            tab = "home"
            shortcuts.resumeAsk += 1
        default: break
        }
        shortcuts.pending = nil
    }
}

struct DeepLink: Equatable {
    let kind: String
    let id: UInt64
    let season: Int?
    let episode: Int?

    init?(_ url: URL) {
        guard url.scheme == "viel" else { return nil }
        var parts = [url.host ?? ""] + url.pathComponents.filter { $0 != "/" }
        parts = parts.filter { !$0.isEmpty }
        if parts.first == "open", let path = URLComponents(url: url, resolvingAgainstBaseURL: false)?.queryItems?.first(where: { $0.name == "path" })?.value {
            parts = path.split(separator: "/").map(String.init)
        }
        guard parts.count >= 2 else { return nil }
        let kind = parts[0] == "movies" ? "movie" : parts[0]
        guard ["movie", "tv", "anime"].contains(kind), let id = UInt64(parts[1]) else { return nil }
        self.kind = kind
        self.id = id
        if kind == "tv" {
            season = parts.count > 2 ? Int(parts[2]) : nil
            episode = parts.count > 3 ? Int(parts[3]) : nil
        } else if kind == "anime" {
            season = nil
            episode = parts.count > 2 ? Int(parts[2]) : nil
        } else {
            season = nil
            episode = nil
        }
    }

    var resume: Resume? {
        guard let episode else { return nil }
        return Resume(id: id, kind: kind, show: "", poster: nil, season: season ?? 0, episode: episode, position: 0, duration: 0, resumeAt: nil)
    }
}

final class Shortcuts: ObservableObject {
    static let shared = Shortcuts()
    static let search = "tv.viel.search"
    static let list = "tv.viel.list"
    static let home = "tv.viel.home"
    static let resume = "tv.viel.resume"

    @Published var pending: String?
    @Published var link: DeepLink?
    @Published var resumeAsk = 0

    static func install() {
        UIApplication.shared.shortcutItems = [
            UIApplicationShortcutItem(type: search, localizedTitle: "Search", localizedSubtitle: nil, icon: UIApplicationShortcutIcon(systemImageName: "magnifyingglass")),
            UIApplicationShortcutItem(type: list, localizedTitle: "My List", localizedSubtitle: "Downloads and continue watching", icon: UIApplicationShortcutIcon(systemImageName: "star")),
        ]
    }
}

final class Watchlist: ObservableObject {
    static let shared = Watchlist()

    @Published private(set) var items: [Title]

    private init() {
        if let data = UserDefaults.standard.data(forKey: "watchlist"), let saved = try? JSONDecoder().decode([Title].self, from: data) {
            items = saved
        } else {
            items = []
        }
    }

    func contains(_ title: Title) -> Bool {
        items.contains { $0.key == title.key }
    }

    func toggle(_ title: Title) {
        let adding = !contains(title)
        if adding {
            items.insert(title, at: 0)
        } else {
            items.removeAll { $0.key == title.key }
        }
        persist()
        Task { @MainActor in
            await Core.shared.setWatch(title, on: adding)
        }
    }

    @MainActor
    func sync(core: Core) async {
        guard let remote = await core.remoteWatchlist() else { return }
        let known = Set(remote.map(\.key))
        let localOnly = items.filter { !known.contains($0.key) }
        for title in localOnly {
            await core.setWatch(title, on: true)
        }
        items = remote + localOnly
        persist()
        Spotlight.index(items)
    }

    private func persist() {
        if let data = try? JSONEncoder().encode(items) {
            UserDefaults.standard.set(data, forKey: "watchlist")
        }
    }
}

struct MyListView: View {
    @EnvironmentObject private var core: Core
    @ObservedObject private var watchlist = Watchlist.shared
    @State private var resume: [Resume] = []
    @State private var downloads: [Download] = []
    @State private var selected: Title?
    @State private var resuming: Resume?
    @State private var playing: Playable?
    @State private var showHistory = false
    @State private var upcoming: [Upcoming] = []
    @State private var showStats = false

    private let columns = [GridItem(.flexible(), spacing: 10), GridItem(.flexible(), spacing: 10), GridItem(.flexible(), spacing: 10)]

    var body: some View {
        NavigationStack {
            ZStack {
                Theme.bg.ignoresSafeArea()
                ScrollView(showsIndicators: false) {
                    VStack(alignment: .leading, spacing: 28) {
                        if !downloads.isEmpty {
                            DownloadsSection(downloads: downloads, onPlay: { playing = $0 }, onRemove: { key in
                                downloads.removeAll { $0.key == key }
                                Task { await core.removeDownload(key) }
                            })
                        }
                        if !upcoming.isEmpty {
                            Row(title: "Coming up") {
                                ForEach(upcoming) { item in
                                    UpcomingCard(item: item)
                                        .onTapGesture {
                                            Haptic.tap()
                                            selected = Title.bare(id: item.showId, kind: item.kind, name: item.name, poster: item.poster)
                                        }
                                }
                            }
                        }
                        if !resume.isEmpty {
                            Row(title: "Continue watching") {
                                ForEach(resume, id: \.key) { row in
                                    ResumeCard(row: row)
                                        .onTapGesture { resuming = row }
                                        .contextMenu {
                                            Button(role: .destructive) {
                                                Task { await remove(row) }
                                            } label: {
                                                Label("Remove from Continue Watching", systemImage: "trash")
                                            }
                                        }
                                }
                            }
                        }
                        VStack(alignment: .leading, spacing: 12) {
                            Text("My list").font(.system(size: 22, weight: .bold)).padding(.horizontal, 16)
                            if watchlist.items.isEmpty {
                                Text(core.username == nil && resume.isEmpty ? "Sign in to sync what you watch, and tap + on any title to keep it here." : "Tap + on any title to keep it here.")
                                    .font(.system(size: 15))
                                    .foregroundStyle(Theme.muted)
                                    .padding(.horizontal, 16)
                            } else {
                                LazyVGrid(columns: columns, spacing: 16) {
                                    ForEach(watchlist.items, id: \.key) { title in
                                        VStack(alignment: .leading, spacing: 6) {
                                            GeometryReader { geometry in
                                                Poster(url: title.poster, width: geometry.size.width, height: geometry.size.width * 1.5, radius: 10)
                                            }
                                            .aspectRatio(2 / 3, contentMode: .fit)
                                            Text(title.name).font(.system(size: 13, weight: .medium)).lineLimit(1)
                                        }
                                        .contentShape(Rectangle())
                                        .onTapGesture { selected = title }
                                        .contextMenu {
                                            Button(role: .destructive) {
                                                watchlist.toggle(title)
                                            } label: {
                                                Label("Remove from My List", systemImage: "trash")
                                            }
                                        }
                                    }
                                }
                                .padding(.horizontal, 16)
                            }
                        }
                    }
                    .padding(.top, 8)
                    .padding(.bottom, 40)
                }
                .refreshable { await load() }
            }
            .navigationTitle("My List")
            .toolbarBackground(Theme.bg, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button {
                        showStats = true
                    } label: {
                        Image(systemName: "chart.bar")
                    }
                    .accessibilityLabel("Your stats")
                }
                ToolbarItem(placement: .topBarTrailing) {
                    Button {
                        showHistory = true
                    } label: {
                        Image(systemName: "clock.arrow.circlepath")
                    }
                    .accessibilityLabel("History")
                }
            }
            .navigationDestination(isPresented: $showHistory) { HistoryView() }
            .sheet(isPresented: $showStats) { StatsView().environmentObject(core) }
        }
        .task { await load() }
        .onChange(of: selected) { _, value in if value == nil { Task { await load() } } }
        .onChange(of: resuming) { _, value in if value == nil { Task { await load() } } }
        .onChange(of: playing) { _, value in if value == nil { Task { await load() } } }
        .task {
            while !Task.isCancelled {
                downloads = await core.downloads()
                try? await Task.sleep(nanoseconds: downloads.contains(where: \.isActive) ? 1_000_000_000 : 5_000_000_000)
            }
        }
        .fullScreenCover(item: $selected) { DetailsView(title: $0, resume: nil).environmentObject(core) }
        .fullScreenCover(item: $resuming) { DetailsView(title: $0.title, resume: $0).environmentObject(core) }
        .fullScreenCover(item: $playing) { PlayerView(item: $0).environmentObject(core) }
    }

    private func load() async {
        let home: HomeData? = try? await core.get("home")
        resume = home?.resume ?? []
        downloads = await core.downloads()
        upcoming = await Alerts.fetch(core: core)
    }

    private func remove(_ row: Resume) async {
        resume.removeAll { $0.key == row.key }
        await core.removeHistory(kind: row.kind, id: row.id)
    }
}
