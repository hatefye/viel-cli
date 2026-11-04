import SwiftUI

struct RootView: View {
    @EnvironmentObject private var core: Core
    @Namespace private var zoom
    @State private var home: HomeData?
    @State private var error: String?
    @State private var selected: Title?
    @State private var resume: Resume?
    @State private var playing: Playable?
    @State private var showSearch = false
    @State private var showAccount = false

    var body: some View {
        NavigationStack {
            ZStack(alignment: .top) {
                Theme.bg.ignoresSafeArea()
                ScrollView(showsIndicators: false) {
                    VStack(alignment: .leading, spacing: 30) {
                        hero
                        if let home {
                            rows(home)
                        } else if let error {
                            Label(error, systemImage: "exclamationmark.triangle")
                                .foregroundStyle(Theme.muted)
                                .padding(.horizontal, 16)
                        } else {
                            HStack { Spacer(); ProgressView().tint(.white); Spacer() }
                        }
                        footer
                    }
                    .padding(.bottom, 40)
                }
                .ignoresSafeArea(edges: .top)
                .refreshable { await load() }
                header
            }
            .toolbar(.hidden, for: .navigationBar)
            .navigationDestination(for: ListPage.self) { ListView(page: $0) }
        }
        .task { await load() }
        .fullScreenCover(item: $selected) { title in
            DetailsView(title: title, resume: nil)
                .navigationTransition(.zoom(sourceID: title.key, in: zoom))
                .environmentObject(core)
        }
        .fullScreenCover(item: $resume) { row in
            DetailsView(title: row.title, resume: row)
                .navigationTransition(.zoom(sourceID: row.key, in: zoom))
                .environmentObject(core)
        }
        .fullScreenCover(item: $playing) { PlayerView(item: $0).environmentObject(core) }
        .fullScreenCover(isPresented: $showSearch) { SearchView().environmentObject(core) }
        .sheet(isPresented: $showAccount, onDismiss: { Task { await load() } }) {
            AccountView().environmentObject(core)
        }
    }

    private var header: some View {
        HStack(spacing: 10) {
            Text("Viel")
                .font(.system(size: 30, weight: .heavy))
                .foregroundStyle(.white)
                .shadow(color: .black.opacity(0.5), radius: 6, y: 2)
            Spacer()
            GlassEffectContainer(spacing: 10) {
                HStack(spacing: 10) {
                    Button { showSearch = true } label: { RoundIcon(icon: "magnifyingglass") }
                        .buttonStyle(.glass)
                        .buttonBorderShape(.circle)
                    Button { showAccount = true } label: { RoundIcon(icon: core.username == nil ? "person" : "person.fill") }
                        .buttonStyle(.glass)
                        .buttonBorderShape(.circle)
                }
            }
        }
        .padding(.horizontal, 16)
        .padding(.top, 6)
    }

    private var hero: some View {
        let detail = home?.hero
        return ZStack(alignment: .bottom) {
            GeometryReader { geometry in
                let offset = geometry.frame(in: .global).minY
                let stretch = max(offset, 0)
                Backdrop(url: detail?.poster ?? detail?.backdrop, height: 620 + stretch)
                    .offset(y: -stretch)
            }
            LinearGradient(
                stops: [
                    .init(color: Theme.bg.opacity(0.55), location: 0),
                    .init(color: .clear, location: 0.18),
                    .init(color: .clear, location: 0.45),
                    .init(color: Theme.bg.opacity(0.85), location: 0.78),
                    .init(color: Theme.bg, location: 1),
                ],
                startPoint: .top,
                endPoint: .bottom
            )
            if let detail {
                VStack(spacing: 12) {
                    Logo(url: detail.logo, name: detail.name, maxWidth: 320, size: 42)
                        .padding(.horizontal, 24)
                    HStack(spacing: 12) {
                        Text("\(Int((detail.rating * 10).rounded()))% match")
                            .foregroundStyle(Theme.match)
                            .fontWeight(.semibold)
                        if !detail.year.isEmpty { Text(detail.year) }
                        Chip(detail.title.kindLabel)
                        if let runtime = Format.runtime(detail.runtime) { Text(runtime) }
                    }
                    .font(.system(size: 17))
                    .foregroundStyle(Theme.soft)
                    if !detail.genres.isEmpty {
                        Text(detail.genres.prefix(3).joined(separator: " · "))
                            .font(.system(size: 16))
                            .foregroundStyle(Theme.soft)
                    }
                    if let overview = detail.overview {
                        Text(overview)
                            .font(.system(size: 17))
                            .foregroundStyle(Theme.soft)
                            .multilineTextAlignment(.center)
                            .lineLimit(2)
                            .padding(.horizontal, 22)
                    }
                    GlassEffectContainer(spacing: 12) {
                        HStack(spacing: 12) {
                            Button {
                                Haptic.tap()
                                open(detail.title, play: true)
                            } label: {
                                Label("Play", systemImage: "play.fill")
                                    .font(.system(size: 17, weight: .semibold))
                                    .foregroundStyle(.black)
                                    .padding(.horizontal, 10)
                                    .padding(.vertical, 4)
                            }
                            .buttonStyle(.glassProminent)
                            .tint(.white)
                            Button {
                                selected = detail.title
                            } label: {
                                Label("More Info", systemImage: "info.circle")
                                    .font(.system(size: 17, weight: .semibold))
                                    .padding(.horizontal, 6)
                                    .padding(.vertical, 4)
                            }
                            .buttonStyle(.glass)
                        }
                    }
                    .padding(.top, 6)
                }
                .padding(.bottom, 6)
            }
        }
        .frame(height: 620)
        .clipped()
    }

    @ViewBuilder
    private func rows(_ home: HomeData) -> some View {
        if !home.resume.isEmpty {
            Row(title: "Continue watching") {
                ForEach(home.resume, id: \.key) { row in
                    ResumeCard(row: row)
                        .matchedTransitionSource(id: row.key, in: zoom)
                        .onTapGesture { resume = row }
                }
            }
        }
        Row(title: "Top 10 today") {
            ForEach(Array(home.top.enumerated()), id: \.element.key) { index, title in
                TopCard(rank: index + 1, title: title)
                    .matchedTransitionSource(id: title.key, in: zoom)
                    .onTapGesture { selected = title }
            }
        }
        Row(title: "Trending now", page: ListPage(title: "Trending now", path: "trending/all/week")) {
            ForEach(home.trending, id: \.key) { title in
                LandscapeCard(title: title)
                    .matchedTransitionSource(id: title.key, in: zoom)
                    .onTapGesture { selected = title }
            }
        }
        Row(title: "Top rated", page: ListPage(title: "Top rated", path: "movie/top_rated")) {
            ForEach(home.topRated, id: \.key) { title in
                Poster(url: title.poster)
                    .matchedTransitionSource(id: title.key, in: zoom)
                    .onTapGesture { selected = title }
            }
        }
        Row(title: "Popular shows", page: ListPage(title: "Popular shows", path: "tv/popular")) {
            ForEach(home.popularTv, id: \.key) { title in
                Poster(url: title.poster)
                    .matchedTransitionSource(id: title.key, in: zoom)
                    .onTapGesture { selected = title }
            }
        }
        if !home.anime.isEmpty {
            Row(title: "Trending anime") {
                ForEach(home.anime, id: \.key) { title in
                    Poster(url: title.poster)
                        .matchedTransitionSource(id: title.key, in: zoom)
                        .onTapGesture { selected = title }
                }
            }
        }
        VStack(alignment: .leading, spacing: 14) {
            Text("Browse by genre")
                .font(.system(size: 22, weight: .bold))
            FlowLayout(spacing: 8) {
                ForEach(movieGenres, id: \.1) { genre in
                    NavigationLink(value: ListPage(title: genre.0, path: "discover/movie?with_genres=\(genre.1)&sort_by=popularity.desc")) {
                        Text(genre.0)
                            .font(.system(size: 14, weight: .medium))
                            .foregroundStyle(Theme.soft)
                            .padding(.horizontal, 14)
                            .padding(.vertical, 8)
                            .glassEffect(.regular.interactive(), in: .capsule)
                    }
                }
            }
        }
        .padding(.horizontal, 16)
    }

    private var footer: some View {
        VStack(alignment: .leading, spacing: 6) {
            Divider().overlay(Theme.border)
            Text("This app does not store any files on its server, it only links to media hosted by third-party services.")
                .font(.system(size: 13))
                .foregroundStyle(Theme.faint)
        }
        .padding(.horizontal, 16)
        .padding(.top, 10)
    }

    private func open(_ title: Title, play: Bool) {
        if play, title.isMovie {
            playing = Playable(title: title, episodes: [], index: -1, dub: false, startAt: nil, chain: [])
        } else {
            selected = title
        }
    }

    private func load() async {
        do {
            home = try await core.get("home")
            error = nil
        } catch {
            self.error = error.localizedDescription
            core.log("home", "load failed: \(error.localizedDescription)")
        }
    }
}

struct Row<Content: View>: View {
    let title: String
    var page: ListPage?
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline) {
                Text(title).font(.system(size: 22, weight: .bold))
                Spacer()
                if let page {
                    NavigationLink(value: page) {
                        HStack(spacing: 2) {
                            Text("View all")
                            Image(systemName: "chevron.right").font(.system(size: 12, weight: .semibold))
                        }
                        .font(.system(size: 15))
                        .foregroundStyle(Theme.muted)
                    }
                }
            }
            .padding(.horizontal, 16)
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(alignment: .top, spacing: 10) { content }
                    .scrollTargetLayout()
                    .padding(.horizontal, 16)
            }
            .scrollTargetBehavior(.viewAligned)
        }
    }
}

struct TopCard: View {
    let rank: Int
    let title: Title

    var body: some View {
        ZStack(alignment: .topLeading) {
            Poster(url: title.poster, width: 124, height: 186, radius: 12)
            VStack(spacing: -3) {
                Text("TOP").font(.system(size: 9, weight: .bold))
                Text(String(format: "%02d", rank)).font(.system(size: 19, weight: .heavy))
            }
            .foregroundStyle(.white)
            .padding(.horizontal, 7)
            .padding(.vertical, 5)
            .background(Theme.accent, in: RoundedRectangle(cornerRadius: 4, style: .continuous))
            .padding(7)
        }
    }
}

struct LandscapeCard: View {
    let title: Title

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Poster(url: title.backdrop ?? title.poster, width: 290, height: 163, radius: 12)
            Text(title.name).font(.system(size: 17, weight: .semibold)).lineLimit(1)
            HStack(spacing: 8) {
                if let genre = title.genres.first { Text(genre).foregroundStyle(Theme.muted) }
                if !title.year.isEmpty { Text(title.year).foregroundStyle(Theme.muted) }
                if let rating = title.ratingText { Text(rating).foregroundStyle(Theme.star) }
            }
            .font(.system(size: 13))
        }
        .frame(width: 290, alignment: .leading)
    }
}

struct ResumeCard: View {
    let row: Resume

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            ZStack(alignment: .bottom) {
                Poster(url: row.poster, width: 124, height: 186, radius: 12)
                GeometryReader { geometry in
                    ZStack(alignment: .leading) {
                        Rectangle().fill(Color.white.opacity(0.25))
                        Rectangle().fill(Theme.accent).frame(width: geometry.size.width * row.fraction)
                    }
                }
                .frame(height: 3)
                .padding(.horizontal, 8)
                .padding(.bottom, 8)
            }
            Text(row.show).font(.system(size: 14, weight: .semibold)).lineLimit(1)
            if let label = row.episodeLabel {
                Text(label).font(.system(size: 12)).foregroundStyle(Theme.muted)
            }
        }
        .frame(width: 124, alignment: .leading)
    }
}
