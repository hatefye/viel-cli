import SwiftUI

struct MainView: View {
    @EnvironmentObject private var core: Core

    var body: some View {
        TabView {
            Tab("Home", systemImage: "house") {
                RootView()
            }
            Tab("Movies", systemImage: "film") {
                NavigationStack {
                    ListView(page: ListPage(title: "Movies", path: "movie/popular"))
                }
            }
            Tab("Shows", systemImage: "tv") {
                NavigationStack {
                    ListView(page: ListPage(title: "Shows", path: "tv/popular"))
                }
            }
            Tab("Anime", systemImage: "sparkles") {
                NavigationStack {
                    ListView(page: ListPage(title: "Anime", path: "anime/trending"))
                }
            }
            Tab("My List", systemImage: "star") {
                MyListView()
            }
            Tab("Settings", systemImage: "gearshape") {
                AccountView(embedded: true)
            }
            Tab("Search", systemImage: "magnifyingglass", role: .search) {
                SearchView(inTab: true)
            }
        }
        .tabBarMinimizeBehavior(.onScrollDown)
        .tint(.white)
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
        if contains(title) {
            items.removeAll { $0.key == title.key }
        } else {
            items.insert(title, at: 0)
        }
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
                        showHistory = true
                    } label: {
                        Image(systemName: "clock.arrow.circlepath")
                    }
                    .accessibilityLabel("History")
                }
            }
            .navigationDestination(isPresented: $showHistory) { HistoryView() }
        }
        .task { await load() }
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
    }

    private func remove(_ row: Resume) async {
        resume.removeAll { $0.key == row.key }
        await core.removeHistory(kind: row.kind, id: row.id)
    }
}
