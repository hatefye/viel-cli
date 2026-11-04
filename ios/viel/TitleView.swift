import SwiftUI

struct TitleView: View {
    let title: Title
    let resume: Resume?

    @EnvironmentObject private var core: Core
    @State private var seasonId: UInt64
    @State private var dub = false
    @State private var list: EpisodeList?
    @State private var loading = false
    @State private var error: String?
    @State private var playing: Playable?

    init(title: Title, resume: Resume?) {
        self.title = title
        self.resume = resume
        _seasonId = State(initialValue: title.seasons.first?.id ?? title.id)
    }

    private var isMovie: Bool { title.kind == "movie" }
    private var isAnime: Bool { title.kind == "anime" }

    private var active: Title {
        guard let season = title.seasons.first(where: { $0.id == seasonId }) else { return title }
        return Title(
            id: season.id,
            kind: "anime",
            name: season.title,
            year: season.year,
            rating: 0,
            overview: nil,
            poster: season.poster ?? title.poster,
            seasons: []
        )
    }

    private var seasonNumbers: [Int] {
        Array(Set((list?.episodes ?? []).map(\.season))).sorted()
    }

    var body: some View {
        List {
            Section {
                HStack(alignment: .top, spacing: 14) {
                    Poster(url: title.poster, width: 100, height: 150, radius: 10)
                    VStack(alignment: .leading, spacing: 6) {
                        Text(title.name).font(.title2.bold())
                        Text(title.subtitle).font(.subheadline).foregroundStyle(.secondary)
                        if let overview = title.overview, !overview.isEmpty {
                            Text(overview).font(.footnote).foregroundStyle(.secondary).lineLimit(6)
                        }
                    }
                }
                .listRowSeparator(.hidden)
            }

            if isMovie {
                Section {
                    Button {
                        playing = Playable(title: title, episodes: [], index: -1, dub: false, startAt: resume?.resumeAt)
                    } label: {
                        Label(resume?.resumeAt != nil ? "Resume · \(resume?.label ?? "")" : "Play", systemImage: "play.fill")
                            .font(.headline)
                    }
                }
            } else {
                if title.seasons.count > 1 || isAnime {
                    Section {
                        if title.seasons.count > 1 {
                            Picker("Season", selection: $seasonId) {
                                ForEach(title.seasons, id: \.id) { season in
                                    Text("\(season.season). \(season.title)").tag(season.id)
                                }
                            }
                        }
                        if isAnime {
                            Toggle("Dub", isOn: $dub)
                        }
                    }
                }
                if let resume, resume.resumeAt != nil, let index = resumeIndex {
                    Section {
                        Button {
                            play(index, startAt: resume.resumeAt)
                        } label: {
                            Label("Resume · \(resume.label)", systemImage: "play.fill").font(.headline)
                        }
                    }
                }
                if let list {
                    if isAnime {
                        Section(list.dub ? "Episodes · dub" : "Episodes · sub") {
                            episodeRows(list.episodes.indices.map { $0 })
                        }
                    } else {
                        ForEach(seasonNumbers, id: \.self) { season in
                            Section("Season \(season)") {
                                episodeRows(list.episodes.indices.filter { list.episodes[$0].season == season })
                            }
                        }
                    }
                } else if let error {
                    Section {
                        Label(error, systemImage: "exclamationmark.triangle").foregroundStyle(.secondary)
                        Button("Try again") { Task { await load() } }
                    }
                } else {
                    Section {
                        HStack { Spacer(); ProgressView(); Spacer() }
                    }
                }
            }
        }
        .listStyle(.plain)
        .navigationTitle(title.name)
        .navigationBarTitleDisplayMode(.inline)
        .task {
            dub = core.settings.dub
            if !isMovie { await load() }
        }
        .onChange(of: seasonId) { _ in Task { await load() } }
        .onChange(of: dub) { value in
            if let list, list.dub != value { Task { await load() } }
        }
        .fullScreenCover(item: $playing) { item in
            PlayerView(item: item).environmentObject(core)
        }
    }

    private var resumeIndex: Int? {
        guard let resume, let list else { return nil }
        return list.episodes.firstIndex { episode in
            (isAnime || episode.season == resume.season) && Int(episode.number) == resume.episode
        }
    }

    @ViewBuilder
    private func episodeRows(_ indices: [Int]) -> some View {
        if let list {
            ForEach(indices, id: \.self) { index in
                let episode = list.episodes[index]
                Button {
                    play(index, startAt: nil)
                } label: {
                    HStack(spacing: 12) {
                        Poster(url: episode.still ?? title.poster, width: 96, height: 54, radius: 6)
                        VStack(alignment: .leading, spacing: 3) {
                            Text(episode.label(anime: isAnime)).font(.subheadline.weight(.semibold)).lineLimit(2)
                            if !episode.overview.isEmpty {
                                Text(episode.overview).font(.caption).foregroundStyle(.secondary).lineLimit(2)
                            }
                        }
                    }
                }
                .buttonStyle(.plain)
            }
        }
    }

    private func play(_ index: Int, startAt: Double?) {
        guard let list else { return }
        playing = Playable(title: active, episodes: list.episodes, index: index, dub: list.dub, startAt: startAt)
    }

    private func load() async {
        loading = true
        error = nil
        defer { loading = false }
        do {
            let fetched: EpisodeList = try await core.get("title", ["kind": title.kind, "id": String(active.id), "dub": dub ? "1" : "0"])
            list = fetched
            if fetched.dub != dub { dub = fetched.dub }
        } catch {
            list = nil
            self.error = error.localizedDescription
        }
    }
}
