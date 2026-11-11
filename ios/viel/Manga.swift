import ImageIO
import SwiftUI
import UIKit

struct MangaCard: Decodable, Hashable, Identifiable {
    let id: String
    let title: String
    let cover: String?
    let year: Int?
    let status: String?
    let type: String?

    var caption: String {
        [type, status, year.map { String($0) }].compactMap { $0 }.joined(separator: " · ")
    }
}

struct MangaListing: Decodable {
    let items: [MangaCard]
    let next: Int?
}

struct MangaChapter: Decodable, Hashable, Identifiable {
    let id: String
    let name: String
    let date: String?

    var short: String { MangaChapter.shorten(name) }
    var day: String { String((date ?? "").prefix(10)) }

    static func shorten(_ name: String) -> String {
        name.hasPrefix("Chapter ") ? "Ch. " + name.dropFirst(8) : name
    }
}

struct MangaTitle: Decodable {
    let id: String
    let title: String
    let cover: String?
    let description: String?
    let status: String?
    let type: String?
    let year: Int?
    let authors: [String]
    let genres: [String]
    let chapters: [MangaChapter]

    var caption: String {
        [type, status, year.map { String($0) }].compactMap { $0 }.joined(separator: " · ")
    }
}

struct MangaPages: Decodable {
    struct Parent: Decodable {
        let id: String
        let title: String
        let cover: String?
    }

    let id: String
    let name: String
    let title: Parent
    let pages: [String]
    let next: String?
    let prev: String?
}

struct MangaMark: Codable, Hashable, Identifiable {
    let id: String
    var title: String
    var cover: String?
    var chapter: String
    var chapterName: String
    var page: Int
    var pages: Int
    var updatedAt: Double

    var fraction: Double {
        pages > 0 ? min(1, Double(page + 1) / Double(pages)) : 0
    }

    var label: String {
        let chapter = MangaChapter.shorten(chapterName)
        return pages > 0 ? "\(chapter) · \(page + 1)/\(pages)" : chapter
    }
}

struct MangaJump: Identifiable, Hashable {
    let chapter: String
    var id: String { chapter }
}

@MainActor
final class MangaStore: ObservableObject {
    static let shared = MangaStore()

    @Published private(set) var shelf: [MangaMark]
    @Published private(set) var read: [String: [String]]
    private var places: [String: Int]
    private var outbox: [String: MangaMark] = [:]
    private var flushTask: Task<Void, Never>?

    private init() {
        let defaults = UserDefaults.standard
        shelf = defaults.data(forKey: "manga.shelf").flatMap { try? JSONDecoder().decode([MangaMark].self, from: $0) } ?? []
        read = defaults.dictionary(forKey: "manga.read") as? [String: [String]] ?? [:]
        places = defaults.dictionary(forKey: "manga.places") as? [String: Int] ?? [:]
    }

    func mark(for title: String) -> MangaMark? {
        shelf.first { $0.id == title }
    }

    func finished(_ title: String) -> Set<String> {
        Set(read[title] ?? [])
    }

    func place(in chapter: String) -> Int {
        places[chapter] ?? 0
    }

    func record(_ chapter: MangaPages, page: Int) {
        let core = Core.shared
        let total = chapter.pages.count
        guard !core.settings.privateMode, total > 0 else { return }
        let entry = MangaMark(id: chapter.title.id, title: chapter.title.title, cover: chapter.title.cover, chapter: chapter.id, chapterName: chapter.name, page: page, pages: total, updatedAt: Date().timeIntervalSince1970 * 1000)
        shelf.removeAll { $0.id == entry.id }
        shelf.insert(entry, at: 0)
        if places.count > 800 { places = [:] }
        places[chapter.id] = page
        let done = page >= total - 1
        if done, !(read[entry.id] ?? []).contains(chapter.id) {
            read[entry.id, default: []].append(chapter.id)
        }
        persist()
        guard core.username != nil else { return }
        outbox[entry.id] = entry
        if done {
            flush()
        } else if flushTask == nil {
            flushTask = Task {
                try? await Task.sleep(nanoseconds: 2_500_000_000)
                guard !Task.isCancelled else { return }
                flush()
            }
        }
    }

    func flush() {
        flushTask?.cancel()
        flushTask = nil
        let batch = Array(outbox.values)
        outbox = [:]
        guard !batch.isEmpty else { return }
        Task {
            for entry in batch {
                let _: Ok? = try? await Core.shared.post("manga/progress", entry)
            }
        }
    }

    func sync(core: Core) async {
        struct Reply: Decodable {
            let items: [MangaMark]
        }
        guard core.username != nil else { return }
        let reply: Reply? = try? await core.get("manga/shelf")
        guard let reply else { return }
        var merged = Dictionary(shelf.map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
        for item in reply.items where (merged[item.id]?.updatedAt ?? 0) < item.updatedAt {
            merged[item.id] = item
            places[item.chapter] = item.page
        }
        shelf = merged.values.sorted { $0.updatedAt > $1.updatedAt }
        persist()
    }

    func refresh(title: String, core: Core) async {
        struct Reply: Decodable {
            let progress: MangaMark?
            let read: [String]
        }
        guard core.username != nil else { return }
        let reply: Reply? = try? await core.get("manga/state", ["id": title])
        guard let reply else { return }
        if !reply.read.isEmpty {
            read[title] = Array(Set(read[title] ?? []).union(reply.read))
        }
        if let remote = reply.progress, (mark(for: title)?.updatedAt ?? 0) < remote.updatedAt {
            shelf.removeAll { $0.id == title }
            shelf.append(remote)
            shelf.sort { $0.updatedAt > $1.updatedAt }
            places[remote.chapter] = remote.page
        }
        persist()
    }

    func remove(_ title: String) {
        shelf.removeAll { $0.id == title }
        outbox[title] = nil
        persist()
        guard Core.shared.username != nil else { return }
        Task {
            let _: Ok? = try? await Core.shared.post("manga/remove", ["key": title])
        }
    }

    func reset() {
        flushTask?.cancel()
        flushTask = nil
        outbox = [:]
        shelf = []
        read = [:]
        places = [:]
        persist()
    }

    private func persist() {
        let defaults = UserDefaults.standard
        if let data = try? JSONEncoder().encode(shelf) {
            defaults.set(data, forKey: "manga.shelf")
        }
        defaults.set(read, forKey: "manga.read")
        defaults.set(places, forKey: "manga.places")
    }
}

enum MangaImages {
    static let session: URLSession = {
        let config = URLSessionConfiguration.default
        config.urlCache = URLCache(memoryCapacity: 48 << 20, diskCapacity: 600 << 20)
        config.requestCachePolicy = .returnCacheDataElseLoad
        config.timeoutIntervalForRequest = 30
        return URLSession(configuration: config)
    }()

    static func load(_ link: String, host: String) async -> UIImage? {
        guard let direct = URL(string: link) else { return nil }
        var sources = [direct, direct]
        if var relay = URLComponents(string: host + "/api/manga/img") {
            relay.queryItems = [URLQueryItem(name: "u", value: link)]
            if let url = relay.url { sources.append(url) }
        }
        for (attempt, source) in sources.enumerated() {
            if attempt > 0 {
                try? await Task.sleep(nanoseconds: 700_000_000)
            }
            if Task.isCancelled { return nil }
            guard let (data, response) = try? await session.data(from: source) else { continue }
            guard (response as? HTTPURLResponse)?.statusCode == 200 else { continue }
            if let image = await decode(data) { return image }
        }
        return nil
    }

    private static func decode(_ data: Data) async -> UIImage? {
        await Task.detached(priority: .userInitiated) { () -> UIImage? in
            guard let source = CGImageSourceCreateWithData(data as CFData, [kCGImageSourceShouldCache: false] as CFDictionary) else { return nil }
            let info = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any]
            let width = (info?[kCGImagePropertyPixelWidth] as? NSNumber)?.doubleValue ?? 0
            let height = (info?[kCGImagePropertyPixelHeight] as? NSNumber)?.doubleValue ?? 0
            guard width > 0, height > 0 else { return UIImage(data: data) }
            let scale = min(1, 1500 / width, (16_000_000 / (width * height)).squareRoot())
            let options: [CFString: Any] = [
                kCGImageSourceCreateThumbnailFromImageAlways: true,
                kCGImageSourceCreateThumbnailWithTransform: true,
                kCGImageSourceShouldCacheImmediately: true,
                kCGImageSourceThumbnailMaxPixelSize: Int(max(width, height) * scale),
            ]
            guard let image = CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary) else { return nil }
            return UIImage(cgImage: image)
        }.value
    }
}

struct MangaResumeCard: View {
    let mark: MangaMark

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            ZStack(alignment: .bottom) {
                Poster(url: mark.cover, width: 124, height: 186, radius: 12)
                GeometryReader { geometry in
                    ZStack(alignment: .leading) {
                        Rectangle().fill(Color.white.opacity(0.25))
                        Rectangle().fill(Theme.accent).frame(width: geometry.size.width * mark.fraction)
                    }
                }
                .frame(height: 3)
                .padding(.horizontal, 8)
                .padding(.bottom, 8)
            }
            Text(mark.title).font(.system(size: 14, weight: .semibold)).lineLimit(1)
            Text(mark.label).font(.system(size: 12)).foregroundStyle(Theme.muted).lineLimit(1)
        }
        .frame(width: 124, alignment: .leading)
    }
}
