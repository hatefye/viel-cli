import Foundation

struct Title: Codable, Hashable {
    let id: UInt64
    let kind: String
    let name: String
    let year: String
    let rating: Double
    let overview: String?
    let poster: String?
    let seasons: [Season]

    var key: String { "\(kind)-\(id)" }

    var subtitle: String {
        var parts: [String] = []
        if !year.isEmpty { parts.append(year) }
        parts.append(kind)
        if rating > 0 { parts.append(String(format: "★ %.1f", rating)) }
        return parts.joined(separator: " · ")
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

struct Home: Decodable {
    let resume: [Resume]
    let trending: [Title]

    enum CodingKeys: String, CodingKey {
        case resume = "continue"
        case trending
    }
}

struct Resume: Decodable, Hashable {
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

    var key: String { "\(kind)-\(id)-\(season)-\(episode)" }

    var title: Title {
        Title(id: id, kind: kind, name: show, year: "", rating: 0, overview: nil, poster: poster, seasons: [])
    }

    var label: String {
        let left = Clock.short(max(duration - position, 0))
        switch kind {
        case "tv": return "S\(season)E\(episode) · \(left) left"
        case "anime": return "E\(episode) · \(left) left"
        default: return "\(left) left"
        }
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
    let sources: [Source]

    var key: String { "\(season)-\(number)" }

    var numberText: String {
        number.rounded() == number ? String(Int(number)) : String(number)
    }

    func label(anime: Bool) -> String {
        let tag = anime ? "E\(numberText)" : String(format: "S%02dE%02d", season, Int(number))
        return name.isEmpty ? tag : "\(tag) · \(name)"
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
}

struct Sub: Decodable, Hashable {
    let lang: String
    let url: String
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

struct Playable: Hashable, Identifiable {
    let title: Title
    let episodes: [Episode]
    let index: Int
    let dub: Bool
    let startAt: Double?

    var id: String { "\(title.key)-\(index)-\(dub)" }
    var isMovie: Bool { title.kind == "movie" }
    var isAnime: Bool { title.kind == "anime" }
    var episode: Episode? { episodes.indices.contains(index) ? episodes[index] : nil }

    var heading: String {
        guard let episode else { return title.name }
        return "\(title.name) · \(episode.label(anime: isAnime))\(dub ? " (dub)" : "")"
    }

    var next: Playable? {
        guard !isMovie, episodes.indices.contains(index + 1) else { return nil }
        return Playable(title: title, episodes: episodes, index: index + 1, dub: dub, startAt: nil)
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

enum Clock {
    static func short(_ seconds: Double) -> String {
        let total = Int(seconds.rounded())
        return String(format: "%d:%02d", total / 60, total % 60)
    }
}
