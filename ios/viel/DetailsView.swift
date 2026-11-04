import SwiftUI

struct DetailsView: View {
    let title: Title
    let resume: Resume?

    @EnvironmentObject private var core: Core
    @ObservedObject private var watchlist = Watchlist.shared
    @Environment(\.dismiss) private var dismiss
    @State private var details: Details?
    @State private var list: EpisodeList?
    @State private var seasonId: UInt64
    @State private var season: Int
    @State private var dub = false
    @State private var loading = false
    @State private var error: String?
    @State private var playing: Playable?
    @State private var toast: String?

    init(title: Title, resume: Resume?) {
        self.title = title
        self.resume = resume
        _seasonId = State(initialValue: title.seasons.first?.id ?? title.id)
        _season = State(initialValue: max(resume?.season ?? 1, 1))
    }

    private var isMovie: Bool { title.isMovie }
    private var isAnime: Bool { title.isAnime }
    private var chain: [Season] { title.seasons }
    private var shown: Title { details?.title ?? title }

    private var active: Title {
        guard let entry = chain.first(where: { $0.id == seasonId }) else { return title }
        return Title(id: entry.id, kind: "anime", name: entry.title, year: entry.year, rating: 0, overview: nil, poster: entry.poster ?? title.poster, backdrop: title.backdrop, genres: title.genres, seasons: [])
    }

    private var seasonNumbers: [Int] {
        Array(Set((list?.episodes ?? []).map(\.season))).sorted()
    }

    private var visibleEpisodes: [Int] {
        guard let list else { return [] }
        if isAnime { return Array(list.episodes.indices) }
        return list.episodes.indices.filter { list.episodes[$0].season == season }
    }

    private var seasonLabel: String {
        if isAnime, chain.count > 1, let entry = chain.first(where: { $0.id == seasonId }) {
            return "Season \(entry.season)"
        }
        return "Season \(season)"
    }

    var body: some View {
        ZStack(alignment: .topTrailing) {
            Theme.bg.ignoresSafeArea()
            ScrollView(showsIndicators: false) {
                VStack(alignment: .leading, spacing: 0) {
                    header
                    info
                    if !isMovie { episodes }
                }
                .padding(.bottom, 40)
            }
            .ignoresSafeArea(edges: .top)
            Button { dismiss() } label: { RoundIcon(icon: "xmark", size: 42) }
                .buttonStyle(.glass)
                .buttonBorderShape(.circle)
                .padding(.trailing, 16)
                .padding(.top, 8)
        }
        .task {
            dub = core.settings.dub
            await loadDetails()
            if !isMovie { await loadEpisodes() }
        }
        .onChange(of: seasonId) { _, _ in Task { await loadEpisodes() } }
        .onChange(of: dub) { _, value in
            if let list, list.dub != value { Task { await loadEpisodes() } }
        }
        .fullScreenCover(item: $playing) { PlayerView(item: $0).environmentObject(core) }
    }

    private var header: some View {
        ZStack(alignment: .bottom) {
            Backdrop(url: shown.backdrop ?? shown.poster, height: 340)
                .mask(
                    LinearGradient(
                        stops: [
                            .init(color: .black, location: 0),
                            .init(color: .black, location: 0.55),
                            .init(color: .black.opacity(0.5), location: 0.8),
                            .init(color: .clear, location: 1),
                        ],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )
            LinearGradient(colors: [Theme.bg.opacity(0.3), .clear], startPoint: .top, endPoint: .center)
            Logo(url: details?.logo, name: shown.name, maxWidth: 300, size: 36)
                .padding(.horizontal, 20)
                .padding(.bottom, 6)
        }
        .frame(height: 340)
        .clipped()
    }

    private var info: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack(spacing: 10) {
                if !shown.year.isEmpty {
                    Text(shown.year).font(.system(size: 22, weight: .medium))
                }
                Chip("HD")
                if isMovie || isAnime {
                    if let runtime = Format.runtime(details?.runtime) { Chip(runtime) }
                } else if let count = details?.seasonCount, count > 0 {
                    Chip(count == 1 ? "1 season" : "\(count) seasons")
                }
                if let rating = shown.ratingText { Chip(rating) }
            }
            GlassEffectContainer(spacing: 12) {
                HStack(spacing: 12) {
                    Button {
                        Haptic.tap()
                        play(resuming: true)
                    } label: {
                        Label(resume?.resumeAt != nil ? "Resume" : "Play", systemImage: "play.fill")
                            .font(.system(size: 17, weight: .semibold))
                            .foregroundStyle(.black)
                            .padding(.horizontal, 18)
                            .padding(.vertical, 7)
                    }
                    .buttonStyle(.glassProminent)
                    .tint(.white)
                    Button {
                        Haptic.select()
                        watchlist.toggle(shown)
                    } label: {
                        Image(systemName: watchlist.contains(shown) ? "checkmark" : "plus")
                            .font(.system(size: 20, weight: .medium))
                            .frame(width: 50, height: 50)
                            .contentTransition(.symbolEffect(.replace))
                    }
                    .buttonStyle(.glass)
                    .buttonBorderShape(.circle)
                    if isMovie {
                        Button {
                            Haptic.select()
                            download(Playable(title: shown, episodes: [], index: -1, dub: false, startAt: nil, chain: []))
                        } label: {
                            Image(systemName: "arrow.down")
                                .font(.system(size: 20, weight: .medium))
                                .frame(width: 50, height: 50)
                        }
                        .buttonStyle(.glass)
                        .buttonBorderShape(.circle)
                    }
                }
            }
            if let toast {
                Text(toast).font(.system(size: 14, weight: .medium)).foregroundStyle(Theme.match)
                    .transition(.opacity)
            }
            if let resume, resume.resumeAt != nil {
                Text("Continue · \(resume.label)").font(.system(size: 14)).foregroundStyle(Theme.muted)
            }
            if let overview = shown.overview ?? details?.overview {
                Text(overview)
                    .font(.system(size: 17))
                    .foregroundStyle(Theme.soft)
                    .lineSpacing(3)
            }
            if let details, !details.cast.isEmpty {
                (Text(isAnime ? "Studio: " : "Starring: ").fontWeight(.semibold).foregroundColor(.white) + Text(details.cast.joined(separator: ", ")).foregroundColor(Theme.muted))
                    .font(.system(size: 16))
            }
            if !shown.genres.isEmpty {
                (Text("Genres: ").fontWeight(.semibold).foregroundColor(.white) + Text(shown.genres.joined(separator: ", ")).foregroundColor(Theme.muted))
                    .font(.system(size: 16))
            }
            if !isMovie {
                Divider().overlay(Theme.border).padding(.top, 6)
            }
        }
        .padding(.horizontal, 16)
        .padding(.top, 10)
    }

    private var episodes: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 12) {
                if isAnime, chain.count > 1 {
                    Menu {
                        ForEach(chain, id: \.id) { entry in
                            Button("\(entry.season). \(entry.title)") { seasonId = entry.id }
                        }
                    } label: {
                        seasonPill(seasonLabel)
                    }
                } else if !isAnime, seasonNumbers.count > 1 {
                    Menu {
                        ForEach(seasonNumbers, id: \.self) { number in
                            Button("Season \(number)") { season = number }
                        }
                    } label: {
                        seasonPill(seasonLabel)
                    }
                } else {
                    seasonPill(isAnime ? "Episodes" : seasonLabel)
                }
                Spacer()
                if isAnime {
                    Button {
                        Haptic.select()
                        dub.toggle()
                    } label: {
                        Text(dub ? "DUB" : "SUB")
                            .font(.system(size: 13, weight: .bold))
                            .padding(.horizontal, 6)
                    }
                    .buttonStyle(.glass)
                    .tint(dub ? Theme.accent : .white)
                }
            }
            .padding(.horizontal, 16)
            .padding(.top, 14)

            if let list {
                ForEach(visibleEpisodes, id: \.self) { index in
                    let episode = list.episodes[index]
                    let current = resume.map { ($0.season == episode.season || isAnime) && $0.episode == Int(episode.number) } ?? false
                    EpisodeRow(episode: episode, highlighted: current, runtime: episode.runtime ?? details?.runtime, subtitle: current ? resume?.label : nil)
                        .onTapGesture {
                            Haptic.tap()
                            play(index: index, startAt: current ? resume?.resumeAt : nil)
                        }
                        .contextMenu {
                            Button {
                                download(Playable(title: active, episodes: list.episodes, index: index, dub: list.dub, startAt: nil, chain: chain))
                            } label: {
                                Label("Download", systemImage: "arrow.down.circle")
                            }
                            Button {
                                Task { await markWatched(episode) }
                            } label: {
                                Label("Mark as watched", systemImage: "checkmark.circle")
                            }
                        }
                }
                if list.episodes.isEmpty {
                    Text("No episodes yet").foregroundStyle(Theme.muted).padding(.horizontal, 16)
                }
            } else if let error {
                VStack(alignment: .leading, spacing: 10) {
                    Label(error, systemImage: "exclamationmark.triangle").foregroundStyle(Theme.muted)
                    Button("Try again") { Task { await loadEpisodes() } }.buttonStyle(.glass)
                }
                .padding(.horizontal, 16)
            } else {
                HStack { Spacer(); ProgressView().tint(.white); Spacer() }.padding(.top, 20)
            }
        }
    }

    private func seasonPill(_ text: String) -> some View {
        Text(text)
            .font(.system(size: 17, weight: .semibold))
            .foregroundStyle(.black)
            .padding(.horizontal, 22)
            .padding(.vertical, 11)
            .background(Color.white, in: Capsule())
    }

    private func play(resuming: Bool) {
        if isMovie {
            playing = Playable(title: shown, episodes: [], index: -1, dub: false, startAt: resuming ? resume?.resumeAt : nil, chain: [])
            return
        }
        guard let list, !list.episodes.isEmpty else { return }
        if resuming, let resume, let index = list.episodes.firstIndex(where: { (isAnime || $0.season == resume.season) && Int($0.number) == resume.episode }) {
            play(index: index, startAt: resume.resumeAt)
        } else {
            play(index: visibleEpisodes.first ?? 0, startAt: nil)
        }
    }

    private func play(index: Int, startAt: Double?) {
        guard let list else { return }
        playing = Playable(title: active, episodes: list.episodes, index: index, dub: list.dub, startAt: startAt, chain: chain)
    }

    private func download(_ item: Playable) {
        Task {
            do {
                try await core.download(item)
                flash("Added to downloads")
            } catch {
                flash(error.localizedDescription)
            }
        }
    }

    private func markWatched(_ episode: Episode) async {
        let item = Playable(title: active, episodes: [episode], index: 0, dub: list?.dub ?? false, startAt: nil, chain: chain)
        let length = Double((episode.runtime ?? details?.runtime ?? 24) * 60)
        await core.saveProgress(item.progress(position: length, duration: length))
        flash("Marked as watched")
    }

    private func flash(_ text: String) {
        withAnimation(.smooth(duration: 0.25)) { toast = text }
        Task {
            try? await Task.sleep(nanoseconds: 2_200_000_000)
            withAnimation(.smooth(duration: 0.25)) { toast = nil }
        }
    }

    private func loadDetails() async {
        details = try? await core.get("details", ["kind": title.kind, "id": String(title.id)])
    }

    private func loadEpisodes() async {
        loading = true
        error = nil
        defer { loading = false }
        do {
            let fetched: EpisodeList = try await core.get("title", ["kind": title.kind, "id": String(active.id), "dub": dub ? "1" : "0"])
            list = fetched
            if fetched.dub != dub { dub = fetched.dub }
            if !isAnime, !fetched.episodes.contains(where: { $0.season == season }) {
                season = fetched.episodes.first?.season ?? 1
            }
        } catch {
            list = nil
            self.error = error.localizedDescription
            core.log("details", "episodes failed for \(title.kind) \(active.id): \(error.localizedDescription)")
        }
    }
}

struct EpisodeRow: View {
    let episode: Episode
    var highlighted = false
    var runtime: Int?
    var subtitle: String?

    var body: some View {
        HStack(alignment: .top, spacing: 14) {
            ZStack(alignment: .topLeading) {
                Poster(url: episode.still, width: 130, height: 76, radius: 8)
                Text("E\(episode.numberText)")
                    .font(.system(size: 13, weight: .bold))
                    .foregroundStyle(.white)
                    .padding(.horizontal, 7)
                    .padding(.vertical, 3)
                    .background(Color.black.opacity(0.65), in: RoundedRectangle(cornerRadius: 6, style: .continuous))
                    .padding(6)
            }
            VStack(alignment: .leading, spacing: 4) {
                Text(episode.displayName)
                    .font(.system(size: 19, weight: .semibold))
                    .foregroundStyle(highlighted ? Theme.accentHi : .white)
                    .lineLimit(1)
                if !episode.overview.isEmpty {
                    Text(episode.overview)
                        .font(.system(size: 15))
                        .foregroundStyle(Theme.soft)
                        .lineLimit(2)
                }
                if let subtitle {
                    Text(subtitle).font(.system(size: 14, weight: .semibold)).foregroundStyle(Theme.accentHi)
                } else if let runtime = Format.runtime(runtime) {
                    Text(runtime).font(.system(size: 14)).foregroundStyle(Theme.muted)
                }
            }
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
        .background(highlighted ? Color.white.opacity(0.05) : .clear)
        .contentShape(Rectangle())
    }
}
