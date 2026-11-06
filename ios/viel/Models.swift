import Foundation

struct Title: Codable, Hashable, Identifiable {
    let id: UInt64
    let kind: String
    let name: String
    let year: String
    let rating: Double
    let overview: String?
    let poster: String?
    let backdrop: String?
    let genres: [String]
    let seasons: [Season]

    var key: String { "\(kind)-\(id)" }
    var isMovie: Bool { kind == "movie" }
    var isAnime: Bool { kind == "anime" }

    var kindLabel: String {
        switch kind {
        case "tv": return "TV Show"
        case "anime": return "Anime"
        default: return "Movie"
        }
    }

    var ratingText: String? {
        rating > 0 ? String(format: "★ %.1f", rating) : nil
    }

    static func bare(id: UInt64, kind: String, name: String, poster: String?) -> Title {
        Title(id: id, kind: kind, name: name, year: "", rating: 0, overview: nil, poster: poster, backdrop: nil, genres: [], seasons: [])
    }
}

struct Season: Codable, Hashable {
    let id: UInt64
    let season: UInt64
    let title: String
    let episodes: UInt64
    let year: String
    let poster: String?
}

struct Details: Decodable {
    let id: UInt64
    let kind: String
    let name: String
    let year: String
    let rating: Double
    let overview: String?
    let poster: String?
    let backdrop: String?
    let genres: [String]
    let seasons: [Season]
    let logo: String?
    let runtime: Int?
    let tagline: String
    let cast: [String]
    let seasonCount: Int

    enum CodingKeys: String, CodingKey {
        case id, kind, name, year, rating, overview, poster, backdrop, genres, seasons, logo, runtime, tagline, cast
        case seasonCount = "season_count"
    }

    var title: Title {
        Title(id: id, kind: kind, name: name, year: year, rating: rating, overview: overview, poster: poster, backdrop: backdrop, genres: genres, seasons: seasons)
    }
}

struct HomeData: Decodable {
    let hero: Details?
    let resume: [Resume]
    let top: [Title]
    let trending: [Title]
    let topRated: [Title]
    let popularTv: [Title]
    let anime: [Title]

    enum CodingKeys: String, CodingKey {
        case hero, top, trending, anime
        case resume = "continue"
        case topRated = "top_rated"
        case popularTv = "popular_tv"
    }
}

struct Resume: Decodable, Hashable, Identifiable {
    let id: UInt64
    let kind: String
    let show: String
    let poster: String?
    let season: Int
    let episode: Int
    let position: Double
    let duration: Double
    let resumeAt: Double?

    enum CodingKeys: String, CodingKey {
        case id, kind, show, poster, season, episode, position, duration
        case resumeAt = "resume_at"
    }

    init(id: UInt64, kind: String, show: String, poster: String?, season: Int, episode: Int, position: Double, duration: Double, resumeAt: Double?) {
        self.id = id
        self.kind = kind
        self.show = show
        self.poster = poster
        self.season = season
        self.episode = episode
        self.position = position
        self.duration = duration
        self.resumeAt = resumeAt
    }

    var key: String { "\(kind)-\(id)-\(season)-\(episode)" }
    var title: Title { Title.bare(id: id, kind: kind, name: show, poster: poster) }
    var fraction: Double { duration > 0 ? min(max(position / duration, 0), 1) : 0 }

    var episodeLabel: String? {
        switch kind {
        case "tv": return "S\(season) · E\(episode)"
        case "anime": return "Episode \(episode)"
        default: return nil
        }
    }

    var label: String {
        let left = Format.clock(max(duration - position, 0))
        if let episodeLabel { return "\(episodeLabel) · \(left) left" }
        return "\(left) left"
    }
}

struct EpisodeList: Decodable {
    let episodes: [Episode]
    let dub: Bool
}

struct Episode: Decodable, Hashable {
    let season: Int
    let number: Double
    let name: String
    let overview: String
    let still: String?
    let runtime: Int?
    let sources: [Source]

    var key: String { "\(season)-\(number)" }

    var numberText: String {
        number.rounded() == number ? String(Int(number)) : String(number)
    }

    func label(anime: Bool) -> String {
        let tag = anime ? "E\(numberText)" : String(format: "S%d · E%d", season, Int(number))
        return name.isEmpty ? tag : "\(tag) · \(name)"
    }

    var displayName: String {
        name.isEmpty ? "Episode \(numberText)" : name
    }
}

struct Source: Codable, Hashable {
    let provider: String
    let id: String
}

struct PlayRequest: Encodable {
    let kind: String
    let id: UInt64
    let title: String
    let season: Int
    let episode: Double
    let dub: Bool
    let sources: [Source]
}

struct PlayResponse: Decodable {
    let streams: [Stream]
}

struct Stream: Decodable, Hashable {
    let source: String
    let url: String
    let hls: Bool
    let subs: [Sub]
    let intro: [Double]?
    let outro: [Double]?
    let thumbs: String?

    var isOffline: Bool { source == "offline" }

    var label: String { Stream.label(for: source) }

    static func label(for source: String) -> String {
        switch source {
        case "cno": return "Cno"
        case "mapple": return "Mapple"
        case "vixsrc": return "VixSrc"
        case "vidbolt": return "VidBolt"
        case "vidrock": return "VidRock"
        case "animeparadise": return "AnimeParadise"
        case "animekai": return "AnimeKai"
        case "kototv": return "KotoTV"
        case "luffy": return "LuffyTV"
        case "offline": return "Downloaded"
        default: return source.capitalized
        }
    }
}

struct Probe: Decodable, Hashable {
    let name: String
    let state: String

    var label: String { Stream.label(for: name) }
}

struct Sub: Decodable, Hashable {
    let lang: String
    let url: String

    var label: String {
        let code = lang.lowercased()
        let names = ["en": "English", "es": "Spanish", "fr": "French", "de": "German", "pt": "Portuguese", "it": "Italian", "ja": "Japanese", "ar": "Arabic", "ru": "Russian", "ko": "Korean", "zh": "Chinese", "tr": "Turkish", "pl": "Polish", "nl": "Dutch", "sv": "Swedish", "el": "Greek", "ro": "Romanian"]
        if let name = names[code] { return name }
        if let name = names[String(code.prefix(2))], code.count <= 5 { return name }
        return lang.isEmpty ? "Subtitle" : lang.capitalized
    }
}

struct Progress: Encodable {
    let kind: String
    let id: UInt64
    let season: Int
    let episode: Int
    let position: Double
    let duration: Double
    let title: String
}

struct LogLine: Encodable {
    let t: UInt64
    let tag: String
    let msg: String
}

struct LogBatch: Encodable {
    let device: String
    let build: String
    let lines: [LogLine]
}

struct Playable: Hashable, Identifiable {
    let title: Title
    let episodes: [Episode]
    let index: Int
    let dub: Bool
    let startAt: Double?
    let chain: [Season]
    var offline: String?

    var id: String { "\(title.key)-\(index)-\(dub)" }

    var downloadKey: String {
        guard let episode else { return "\(title.kind)-\(title.id)" }
        return "\(title.kind)-\(title.id)-\(episode.season)-\(episode.numberText.replacingOccurrences(of: ".", with: "_"))\(dub ? "-dub" : "")"
    }

    var downloadMeta: DownloadMeta {
        DownloadMeta(
            name: title.name,
            label: episode.map { $0.label(anime: isAnime) + (dub ? " (dub)" : "") } ?? (title.year.isEmpty ? title.kindLabel : title.year),
            kind: title.kind,
            id: title.id,
            season: episode?.season ?? 0,
            episode: episode?.number ?? 0,
            poster: title.poster,
            still: episode?.still,
            dub: dub
        )
    }
    var isMovie: Bool { title.isMovie }
    var isAnime: Bool { title.isAnime }
    var episode: Episode? { episodes.indices.contains(index) ? episodes[index] : nil }

    var heading: String {
        guard let episode else { return title.name }
        return "\(title.name) · \(episode.label(anime: isAnime))\(dub ? " (dub)" : "")"
    }

    var episodeTag: String? {
        guard let episode else { return nil }
        return isAnime ? "E\(episode.numberText)" : "S\(episode.season) · E\(Int(episode.number))"
    }

    var next: Playable? {
        guard !isMovie, episodes.indices.contains(index + 1) else { return nil }
        return Playable(title: title, episodes: episodes, index: index + 1, dub: dub, startAt: nil, chain: chain)
    }

    func at(_ index: Int) -> Playable {
        Playable(title: title, episodes: episodes, index: index, dub: dub, startAt: nil, chain: chain)
    }

    func resuming(at position: Double) -> Playable {
        Playable(title: title, episodes: episodes, index: index, dub: dub, startAt: position > 10 ? position : startAt, chain: chain, offline: offline)
    }

    var request: PlayRequest {
        PlayRequest(
            kind: title.kind,
            id: title.id,
            title: heading,
            season: episode?.season ?? 0,
            episode: episode?.number ?? 0,
            dub: dub,
            sources: episode?.sources ?? []
        )
    }

    func progress(position: Double, duration: Double) -> Progress {
        let name = isAnime ? (episode.map { $0.name.isEmpty ? title.name : $0.name } ?? title.name) : title.name
        return Progress(
            kind: title.kind,
            id: title.id,
            season: episode?.season ?? 0,
            episode: Int(episode?.number ?? 0),
            position: position,
            duration: duration,
            title: name
        )
    }
}

enum Format {
    static func runtime(_ minutes: Int?) -> String? {
        guard let minutes, minutes > 0 else { return nil }
        return minutes >= 60 ? "\(minutes / 60)h \(minutes % 60)m" : "\(minutes)m"
    }

    static func clock(_ seconds: Double) -> String {
        guard seconds.isFinite, seconds >= 0 else { return "00:00" }
        let total = Int(seconds.rounded())
        if total >= 3600 {
            return String(format: "%d:%02d:%02d", total / 3600, (total % 3600) / 60, total % 60)
        }
        return String(format: "%02d:%02d", total / 60, total % 60)
    }
}

struct DownloadMeta: Codable, Hashable {
    let name: String
    let label: String
    let kind: String
    let id: UInt64
    let season: Int
    let episode: Double
    let poster: String?
    let still: String?
    let dub: Bool
}

struct DownloadRequest: Encodable {
    let kind: String
    let id: UInt64
    let title: String
    let season: Int
    let episode: Double
    let dub: Bool
    let sources: [Source]
    let key: String
    let meta: DownloadMeta
    let max_height: Int
}

struct AudioTrack: Decodable, Hashable {
    let lang: String
    let name: String
    let `default`: Bool

    var label: String {
        let pretty = Sub(lang: lang, url: "").label
        if pretty.lowercased() == "subtitle" || pretty.lowercased() == lang.lowercased() { return name.isEmpty ? lang : name }
        return name.isEmpty || name.lowercased() == pretty.lowercased() ? pretty : "\(pretty) · \(name)"
    }
}

struct WatchRow: Decodable {
    let id: UInt64
    let kind: String
    let name: String
    let poster: String?

    var title: Title { Title.bare(id: id, kind: kind, name: name, poster: poster) }
}

struct Download: Decodable, Hashable, Identifiable {
    let key: String
    let meta: DownloadMeta?
    let state: String
    let source: String?
    let done: Int
    let total: Int
    let bytes: UInt64
    let error: String?

    var id: String { key }
    var isDone: Bool { state == "done" }
    var isActive: Bool { state == "queued" || state == "fetching" }
    var fraction: Double { total > 0 ? Double(done) / Double(total) : 0 }

    var sizeText: String {
        bytes >= 1_073_741_824 ? String(format: "%.1f GB", Double(bytes) / 1_073_741_824) : "\(bytes / 1_048_576) MB"
    }

    var statusText: String {
        switch state {
        case "queued": return "Waiting…"
        case "fetching": return total > 0 ? "\(Int(fraction * 100))% · \(sizeText)" : "Starting…"
        case "failed": return error ?? "Failed"
        default: return sizeText
        }
    }

    var playable: Playable? {
        guard let meta else { return nil }
        let title = Title.bare(id: meta.id, kind: meta.kind, name: meta.name, poster: meta.poster)
        if meta.kind == "movie" {
            return Playable(title: title, episodes: [], index: -1, dub: false, startAt: nil, chain: [], offline: key)
        }
        let episode = Episode(season: meta.season, number: meta.episode, name: meta.label, overview: "", still: meta.still, runtime: nil, sources: [])
        return Playable(title: title, episodes: [episode], index: 0, dub: meta.dub, startAt: nil, chain: [], offline: key)
    }
}

struct HistoryRow: Decodable, Hashable, Identifiable {
    let id: UInt64
    let kind: String
    let show: String
    let poster: String?
    let season: Int
    let episode: Int
    let position: Double
    let duration: Double
    let updatedAt: UInt64

    enum CodingKeys: String, CodingKey {
        case id, kind, show, poster, season, episode, position, duration
        case updatedAt = "updated_at"
    }

    var key: String { "\(kind)-\(id)-\(season)-\(episode)" }
    var title: Title { Title.bare(id: id, kind: kind, name: show, poster: poster) }
    var fraction: Double { duration > 0 ? min(max(position / duration, 0), 1) : 0 }
    var finished: Bool { duration > 0 && duration - position < 30 }

    var episodeLabel: String? {
        switch kind {
        case "tv": return "S\(season) · E\(episode)"
        case "anime": return "Episode \(episode)"
        default: return nil
        }
    }

    var when: String {
        guard updatedAt > 0 else { return "" }
        let date = Date(timeIntervalSince1970: Double(updatedAt) / 1000)
        return date.formatted(.relative(presentation: .named, unitsStyle: .abbreviated))
    }

    var resume: Resume {
        Resume(id: id, kind: kind, show: show, poster: poster, season: season, episode: episode, position: position, duration: duration, resumeAt: (position > 5 && position < duration - 30) ? position : nil)
    }
}

struct ListPage: Hashable, Identifiable {
    let title: String
    let path: String?
    let items: [Title]

    var id: String { title + (path ?? "") }

    init(title: String, path: String) {
        self.title = title
        self.path = path
        items = []
    }

    init(title: String, items: [Title]) {
        self.title = title
        path = nil
        self.items = items
    }
}
