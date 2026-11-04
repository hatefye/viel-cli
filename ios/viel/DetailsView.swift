import SwiftUI

struct DetailsView: View {
    let title: Title
    let resume: Resume?

    @EnvironmentObject private var core: Core
    @Environment(\.dismiss) private var dismiss
    @State private var details: Details?
    @State private var list: EpisodeList?
    @State private var seasonId: UInt64
    @State private var season: Int
    @State private var dub = false
    @State private var loading = false
    @State private var error: String?
    @State private var playing: Playable?

    init(title: Title, resume: Resume?) {
        self.title = title
        self.resume = resume
        _seasonId = State(initialValue: title.seasons.first?.id ?? title.id)
        _season = State(initialValue: max(resume?.season ?? 1, 1))
    }

    private var isMovie: Bool { title.isMovie }
    private var isAnime: Bool { title.isAnime }
    private var chain: [Season] { title.seasons }

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
            Button { dismiss() } label: { Image(systemName: "xmark") }
                .buttonStyle(GlassCircle(size: 44))
                .padding(.trailing, 16)
                .padding(.top, 8)
        }
        .task {
            dub = core.settings.dub
            await loadDetails()
            if !isMovie { await loadEpisodes() }
        }
        .onChange(of: seasonId) { _ in Task { await loadEpisodes() } }
        .onChange(of: dub) { value in
            if let list, list.dub != value { Task { await loadEpisodes() } }
        }
        .fullScreenCover(item: $playing) { PlayerView(item: $0).environmentObject(core) }
    }

    private var header: some View {
        let shown = details?.title ?? title
        return ZStack(alignment: .bottom) {
            Backdrop(url: shown.backdrop ?? shown.poster, height: 330)
            LinearGradient(
                stops: [
                    .init(color: Theme.bg.opacity(0.35), location: 0),
                    .init(color: .clear, location: 0.25),
                    .init(color: Theme.bg.opacity(0.7), location: 0.75),
                    .init(color: Theme.bg, location: 1),
                ],
                startPoint: .top,
                endPoint: .bottom
            )
            Logo(url: details?.logo, name: shown.name, maxWidth: 300, size: 36)
                .padding(.horizontal, 20)
                .padding(.bottom, 4)
        }
        .frame(height: 330)
        .clipped()
    }

    private var info: some View {
        let shown = details?.title ?? title
        return VStack(alignment: .leading, spacing: 16) {
            HStack(spacing: 10) {
                if !shown.year.isEmpty {
                    Text(shown.year).font(.system(size: 22, weight: .medium))
                }
                Chip("HD")
                if let runtime = Format.runtime(details?.runtime) { Chip(runtime) }
                if let rating = shown.ratingText { Chip(rating) }
            }
            HStack(spacing: 14) {
                Button {
                    play(resuming: true)
                } label: {
                    Label(resume?.resumeAt != nil ? "Resume" : "Play", systemImage: "play.fill")
                }
                .buttonStyle(WhitePill())
                if let resume, resume.resumeAt != nil {
                    Text(resume.label).font(.system(size: 14)).foregroundStyle(Theme.muted)
                }
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
        }
        .padding(.horizontal, 16)
        .padding(.top, 10)
    }

    private var episodes: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 12) {
                Text(isAnime ? "Episodes" : "Season \(season)")
                    .font(.system(size: 24, weight: .bold))
                Spacer()
                if isAnime, chain.count > 1 {
                    Menu {
                        ForEach(chain, id: \.id) { entry in
                            Button("\(entry.season). \(entry.title)") { seasonId = entry.id }
                        }
                    } label: {
                        menuLabel("S\(chain.first(where: { $0.id == seasonId })?.season ?? 1)")
                    }
                } else if !isAnime, seasonNumbers.count > 1 {
                    Menu {
                        ForEach(seasonNumbers, id: \.self) { number in
                            Button("Season \(number)") { season = number }
                        }
                    } label: {
                        menuLabel("S\(season)")
                    }
                }
                if isAnime {
                    Button {
                        dub.toggle()
                    } label: {
                        Text(dub ? "DUB" : "SUB")
                            .font(.system(size: 13, weight: .bold))
                            .foregroundStyle(.white)
                            .padding(.horizontal, 12)
                            .padding(.vertical, 7)
                            .background(dub ? Theme.accent : Color.white.opacity(0.12), in: Capsule())
                    }
                }
            }
            .padding(.horizontal, 16)
            .padding(.top, 28)

            if let list {
                ForEach(visibleEpisodes, id: \.self) { index in
                    let episode = list.episodes[index]
                    let current = resume.map { ($0.season == episode.season || isAnime) && $0.episode == Int(episode.number) } ?? false
                    EpisodeRow(episode: episode, highlighted: current, runtime: episode.runtime ?? details?.runtime, subtitle: current ? resume?.label : nil)
                        .onTapGesture { play(index: index, startAt: current ? resume?.resumeAt : nil) }
                }
                if list.episodes.isEmpty {
                    Text("No episodes yet").foregroundStyle(Theme.muted).padding(.horizontal, 16)
                }
            } else if let error {
                VStack(alignment: .leading, spacing: 10) {
                    Label(error, systemImage: "exclamationmark.triangle").foregroundStyle(Theme.muted)
                    Button("Try again") { Task { await loadEpisodes() } }.buttonStyle(GlassPill())
                }
                .padding(.horizontal, 16)
            } else {
                HStack { Spacer(); ProgressView().tint(.white); Spacer() }.padding(.top, 20)
            }
        }
    }

    private func menuLabel(_ text: String) -> some View {
        HStack(spacing: 6) {
            Text(text).font(.system(size: 17, weight: .bold))
            Image(systemName: "chevron.down").font(.system(size: 13, weight: .semibold))
        }
        .foregroundStyle(.white)
        .padding(.horizontal, 12)
        .padding(.vertical, 7)
        .background(Color.white.opacity(0.08), in: Capsule())
    }

    private func play(resuming: Bool) {
        if isMovie {
            playing = Playable(title: details?.title ?? title, episodes: [], index: -1, dub: false, startAt: resuming ? resume?.resumeAt : nil, chain: [])
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
        HStack(spacing: 14) {
            Poster(url: episode.still, width: 100, height: 58, radius: 8)
            VStack(alignment: .leading, spacing: 4) {
                HStack(alignment: .firstTextBaseline, spacing: 12) {
                    Text(episode.numberText)
                        .font(.system(size: 20))
                        .foregroundStyle(highlighted ? Theme.accentHi : Theme.muted)
                    Text(episode.displayName)
                        .font(.system(size: 18, weight: .medium))
                        .foregroundStyle(highlighted ? Theme.accentHi : .white)
                        .lineLimit(1)
                }
                HStack(spacing: 6) {
                    if let subtitle {
                        Text(subtitle).foregroundStyle(Theme.accentHi).fontWeight(.semibold)
                    } else if let runtime = Format.runtime(runtime) {
                        Text(runtime).foregroundStyle(Theme.muted)
                    }
                }
                .font(.system(size: 14))
            }
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
        .background(highlighted ? Color.white.opacity(0.06) : .clear)
        .contentShape(Rectangle())
    }
}
