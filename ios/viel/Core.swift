import Foundation
import Network
import UIKit

struct Settings: Codable {
    var host = "https://viel.tv"
    var maxHeight = 1080
    var subtitleLang = "en"
    var dub = false
    var autoplay = true
    var subSize = 1.0
    var subBackground = true
    var subLift = 0.0
    var downloadHeight = 1080
    var autoDeleteWatched = false
    var airplay = false
    var audioLang = ""
    var skipSeconds = 10
    var wifiDownloads = false
    var autoDownloadNext = false
    var episodeAlerts = false
    var subDelay = 0.0
    var backgroundAudio = false
    var appLock = false
    var privateMode = false
    var ambient = false
    var cellularHeight = 0

    init() {}

    init(from decoder: Decoder) throws {
        let box = try decoder.container(keyedBy: CodingKeys.self)
        host = try box.decodeIfPresent(String.self, forKey: .host) ?? host
        maxHeight = try box.decodeIfPresent(Int.self, forKey: .maxHeight) ?? maxHeight
        subtitleLang = try box.decodeIfPresent(String.self, forKey: .subtitleLang) ?? subtitleLang
        dub = try box.decodeIfPresent(Bool.self, forKey: .dub) ?? dub
        autoplay = try box.decodeIfPresent(Bool.self, forKey: .autoplay) ?? autoplay
        subSize = try box.decodeIfPresent(Double.self, forKey: .subSize) ?? subSize
        subBackground = try box.decodeIfPresent(Bool.self, forKey: .subBackground) ?? subBackground
        subLift = try box.decodeIfPresent(Double.self, forKey: .subLift) ?? subLift
        downloadHeight = try box.decodeIfPresent(Int.self, forKey: .downloadHeight) ?? downloadHeight
        autoDeleteWatched = try box.decodeIfPresent(Bool.self, forKey: .autoDeleteWatched) ?? autoDeleteWatched
        airplay = try box.decodeIfPresent(Bool.self, forKey: .airplay) ?? airplay
        audioLang = try box.decodeIfPresent(String.self, forKey: .audioLang) ?? audioLang
        skipSeconds = try box.decodeIfPresent(Int.self, forKey: .skipSeconds) ?? skipSeconds
        wifiDownloads = try box.decodeIfPresent(Bool.self, forKey: .wifiDownloads) ?? wifiDownloads
        autoDownloadNext = try box.decodeIfPresent(Bool.self, forKey: .autoDownloadNext) ?? autoDownloadNext
        episodeAlerts = try box.decodeIfPresent(Bool.self, forKey: .episodeAlerts) ?? episodeAlerts
        subDelay = try box.decodeIfPresent(Double.self, forKey: .subDelay) ?? subDelay
        backgroundAudio = try box.decodeIfPresent(Bool.self, forKey: .backgroundAudio) ?? backgroundAudio
        appLock = try box.decodeIfPresent(Bool.self, forKey: .appLock) ?? appLock
        privateMode = try box.decodeIfPresent(Bool.self, forKey: .privateMode) ?? privateMode
        ambient = try box.decodeIfPresent(Bool.self, forKey: .ambient) ?? ambient
        cellularHeight = try box.decodeIfPresent(Int.self, forKey: .cellularHeight) ?? cellularHeight
    }

    static func load() -> Settings {
        guard let data = UserDefaults.standard.data(forKey: "settings"),
              let saved = try? JSONDecoder().decode(Settings.self, from: data)
        else { return Settings() }
        return saved
    }

    func save() {
        if let data = try? JSONEncoder().encode(self) {
            UserDefaults.standard.set(data, forKey: "settings")
        }
    }
}

struct ApiError: LocalizedError {
    let message: String
    var errorDescription: String? { message }
}

struct Ok: Decodable {
    let ok: Bool
}

@MainActor
final class Core: ObservableObject {
    static let shared = Core()

    let port: UInt16
    let device: String
    let build: String
    let downloadsDir: URL
    nonisolated static let downloadsRoot: URL = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first!.appendingPathComponent("Downloads", isDirectory: true)
    private var keepAlive: UIBackgroundTaskIdentifier = .invalid
    private let monitor = NWPathMonitor()
    @Published var username: String?
    @Published var offline = false
    @Published var expensive = false
    @Published var settings: Settings {
        didSet { settings.save() }
    }

    private var lines: [LogLine] = []
    private var flushTask: Task<Void, Never>?

    private init() {
        let settings = Settings.load()
        self.settings = settings
        let token = UserDefaults.standard.string(forKey: "token") ?? ""
        downloadsDir = Core.downloadsRoot
        port = viel_start(settings.host, token, downloadsDir.path)
        let stored = UserDefaults.standard.string(forKey: "device") ?? String(UUID().uuidString.prefix(8)).lowercased()
        UserDefaults.standard.set(stored, forKey: "device")
        device = stored
        build = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "0"
        log("app", "start \(UIDevice.current.systemName) \(UIDevice.current.systemVersion) \(UIDevice.current.model) build \(build) port \(port)")
        Task {
            await pushSettings()
            await refreshUser()
            DownloadManager.shared.activate()
            await DownloadManager.shared.resume(core: self)
            await Watchlist.shared.sync(core: self)
        }
        monitor.pathUpdateHandler = { [weak self] path in
            Task { @MainActor in
                guard let self else { return }
                let wasExpensive = self.expensive
                self.offline = path.status != .satisfied
                self.expensive = path.isExpensive
                guard self.settings.airplay || wasExpensive != self.expensive else { return }
                await self.pushSettings()
            }
        }
        monitor.start(queue: DispatchQueue(label: "tv.viel.network"))
    }

    nonisolated static func wifiAddress() -> String? {
        var list: UnsafeMutablePointer<ifaddrs>?
        guard getifaddrs(&list) == 0, let first = list else { return nil }
        defer { freeifaddrs(list) }
        var found: String?
        for entry in sequence(first: first, next: { $0.pointee.ifa_next }) {
            guard let addr = entry.pointee.ifa_addr, addr.pointee.sa_family == UInt8(AF_INET) else { continue }
            let flags = Int32(entry.pointee.ifa_flags)
            guard flags & IFF_UP != 0, flags & IFF_LOOPBACK == 0, String(cString: entry.pointee.ifa_name) == "en0" else { continue }
            var host = [CChar](repeating: 0, count: Int(NI_MAXHOST))
            if getnameinfo(addr, socklen_t(addr.pointee.sa_len), &host, socklen_t(host.count), nil, 0, NI_NUMERICHOST) == 0 {
                found = String(cString: host)
            }
        }
        return found
    }

    private var base: String { "http://127.0.0.1:\(port)/api/" }

    func get<T: Decodable>(_ path: String, _ query: [String: String] = [:]) async throws -> T {
        var components = URLComponents(string: base + path)!
        if !query.isEmpty {
            components.queryItems = query.map { URLQueryItem(name: $0.key, value: $0.value) }
        }
        var request = URLRequest(url: components.url!)
        request.timeoutInterval = 90
        return try await send(request)
    }

    func post<T: Decodable, B: Encodable>(_ path: String, _ body: B) async throws -> T {
        var request = URLRequest(url: URL(string: base + path)!)
        request.httpMethod = "POST"
        request.httpBody = try JSONEncoder().encode(body)
        request.timeoutInterval = 120
        return try await send(request)
    }

    private func send<T: Decodable>(_ request: URLRequest) async throws -> T {
        let (data, response) = try await URLSession.shared.data(for: request)
        let status = (response as? HTTPURLResponse)?.statusCode ?? 0
        guard status == 200 else {
            let reply = try? JSONDecoder().decode([String: String].self, from: data)
            throw ApiError(message: reply?["error"] ?? "request failed (\(status))")
        }
        return try JSONDecoder().decode(T.self, from: data)
    }

    func login(_ username: String, _ password: String, register: Bool) async throws {
        struct Reply: Decodable {
            let token: String
            let username: String
        }
        let reply: Reply = try await post(register ? "register" : "login", ["username": username, "password": password])
        UserDefaults.standard.set(reply.token, forKey: "token")
        self.username = reply.username
        log("app", "signed in")
        await Watchlist.shared.sync(core: self)
    }

    func logout() async {
        let _: Ok? = try? await post("logout", [String: String]())
        UserDefaults.standard.removeObject(forKey: "token")
        username = nil
    }

    func refreshUser() async {
        struct Me: Decodable {
            let username: String?
        }
        let me: Me? = try? await get("me")
        username = me?.username
    }

    func pushSettings() async {
        struct Body: Encodable {
            let max_height: Int
            let subtitle_lang: String
            let audio_lang: String
            let lan_host: String
            let native_subs: Bool
        }
        let lan = settings.airplay ? (Core.wifiAddress() ?? "") : ""
        let cap = expensive && settings.cellularHeight > 0 ? min(settings.maxHeight, settings.cellularHeight) : settings.maxHeight
        let _: Ok? = try? await post("settings", Body(max_height: cap, subtitle_lang: settings.subtitleLang, audio_lang: settings.audioLang, lan_host: lan, native_subs: true))
    }

    func setCaptions(_ text: String) async {
        var request = URLRequest(url: URL(string: base + "captions")!)
        request.httpMethod = "POST"
        request.httpBody = Data(text.utf8)
        _ = try? await URLSession.shared.data(for: request)
    }

    func audioTracks(for url: String) async -> [AudioTrack] {
        (try? await get("audio", ["url": url])) ?? []
    }

    func recommendations() async -> [Title] {
        guard username != nil else { return [] }
        return (try? await get("recommendations")) ?? []
    }

    func trailer(kind: String, id: UInt64) async -> URL? {
        struct Reply: Decodable {
            let url: String?
        }
        let reply: Reply? = try? await get("trailer", ["kind": kind, "id": String(id)])
        return reply?.url.flatMap { URL(string: $0) }
    }

    func remoteWatchlist() async -> [Title]? {
        guard username != nil else { return nil }
        let rows: [WatchRow]? = try? await get("watchlist")
        return rows?.map(\.title)
    }

    func setWatch(_ title: Title, on: Bool) async {
        guard username != nil else { return }
        struct Body: Encodable {
            let kind: String
            let id: UInt64
            let name: String
            let poster: String?
        }
        let _: Ok? = try? await post(on ? "watchlist" : "watchlist/remove", Body(kind: title.kind, id: title.id, name: title.name, poster: title.poster))
    }

    func saveProgress(_ progress: Progress) async {
        guard !settings.privateMode else { return }
        Memory.remember(progress)
        guard username != nil else { return }
        do {
            let _: Ok = try await post("progress", progress)
        } catch {
            log("progress", "save failed for \(progress.kind) \(progress.id): \(error.localizedDescription)")
        }
    }

    func lastRow(kind: String, id: UInt64) async -> HistoryRow? {
        if username != nil, let row = await history().first(where: { $0.kind == kind && $0.id == id }) {
            return row
        }
        guard let local = Memory.lastPlayed(for: "\(kind)-\(id)") else { return nil }
        return HistoryRow(id: id, kind: kind, show: local.title, poster: nil, season: local.season, episode: local.episode, position: local.position, duration: local.duration, updatedAt: local.at)
    }

    func history() async -> [HistoryRow] {
        (try? await get("history")) ?? []
    }

    func downloads() async -> [Download] {
        (try? await get("downloads")) ?? []
    }

    func download(_ item: Playable, height: Int? = nil) async throws {
        try await DownloadManager.shared.start(item, core: self, height: height)
    }

    func removeDownload(_ key: String) async {
        await DownloadManager.shared.cancel(key)
        let _: Ok? = try? await post("downloads/remove", ["key": key])
        log("download", "removed \(key)")
    }

    nonisolated func relayLink(_ url: String, referer: String?) -> URL? {
        var components = URLComponents(string: "http://127.0.0.1:\(port)/s/thumb.webp")!
        var items = [URLQueryItem(name: "u", value: url)]
        if let referer, !referer.isEmpty { items.append(URLQueryItem(name: "r", value: referer)) }
        components.queryItems = items
        return components.url
    }

    func enteredBackground() {
        Alerts.schedule()
        Task {
            await flushLogs()
            let active = await downloads().contains(where: \.isActive)
            guard active, keepAlive == .invalid else { return }
            keepAlive = UIApplication.shared.beginBackgroundTask(withName: "downloads") { [weak self] in
                self?.endKeepAlive()
            }
        }
    }

    func enteredForeground() {
        endKeepAlive()
        Task { await Alerts.refresh(core: self) }
    }

    private func endKeepAlive() {
        guard keepAlive != .invalid else { return }
        UIApplication.shared.endBackgroundTask(keepAlive)
        keepAlive = .invalid
    }

    func removeHistory(kind: String, id: UInt64) async {
        struct Body: Encodable {
            let kind: String
            let id: UInt64
        }
        let _: Ok? = try? await post("history/remove", Body(kind: kind, id: id))
        log("history", "removed \(kind) \(id)")
    }

    nonisolated func log(_ tag: String, _ message: String) {
        Task { @MainActor in
            self.append(tag, message)
        }
    }

    private func append(_ tag: String, _ message: String) {
        print("[\(tag)] \(message)")
        lines.append(LogLine(t: UInt64(Date().timeIntervalSince1970 * 1000), tag: tag, msg: message))
        if flushTask == nil {
            flushTask = Task {
                try? await Task.sleep(nanoseconds: 4_000_000_000)
                await flushLogs()
            }
        }
    }

    func flushLogs() async {
        flushTask = nil
        guard !lines.isEmpty else { return }
        let batch = LogBatch(device: device, build: build, lines: lines)
        lines = []
        let _: Ok? = try? await post("log", batch)
    }
}

enum Memory {
    private static func map(_ key: String) -> [String: String] {
        UserDefaults.standard.dictionary(forKey: key) as? [String: String] ?? [:]
    }

    private static func store(_ key: String, _ value: [String: String]) {
        UserDefaults.standard.set(value, forKey: key)
    }

    static func preferredSource(for title: String) -> String? {
        map("preferred")[title]
    }

    static func remember(source: String, for title: String) {
        var all = map("preferred")
        all[title] = source
        store("preferred", all)
    }

    static func dub(for title: String) -> Bool? {
        map("dub")[title].map { $0 == "1" }
    }

    static func remember(dub: Bool, for title: String) {
        var all = map("dub")
        all[title] = dub ? "1" : "0"
        store("dub", all)
    }

    static func fill(for title: String) -> Bool {
        map("fill")[title] == "1"
    }

    static func remember(fill: Bool, for title: String) {
        var all = map("fill")
        all[title] = fill ? "1" : nil
        store("fill", all)
    }

    static func order(_ streams: [Stream], for title: String) -> [Stream] {
        guard let liked = preferredSource(for: title), let at = streams.firstIndex(where: { $0.source == liked }), at > 0 else { return streams }
        var out = streams
        out.insert(out.remove(at: at), at: 0)
        return out
    }

    struct LastPlayed: Codable {
        let title: String
        let season: Int
        let episode: Int
        let position: Double
        let duration: Double
        let at: UInt64
    }

    static func lastPlayed(for title: String) -> LastPlayed? {
        let all = UserDefaults.standard.dictionary(forKey: "lastplayed") as? [String: Data] ?? [:]
        return all[title].flatMap { try? JSONDecoder().decode(LastPlayed.self, from: $0) }
    }

    static func remember(_ progress: Progress) {
        let key = "\(progress.kind)-\(progress.id)"
        var all = UserDefaults.standard.dictionary(forKey: "lastplayed") as? [String: Data] ?? [:]
        let entry = LastPlayed(title: progress.title, season: progress.season, episode: progress.episode, position: progress.position, duration: progress.duration, at: UInt64(Date().timeIntervalSince1970 * 1000))
        all[key] = try? JSONEncoder().encode(entry)
        UserDefaults.standard.set(all, forKey: "lastplayed")
        if progress.duration > 0, progress.duration - progress.position < 30 {
            markWatched("\(progress.season)-\(progress.episode)", for: key)
        }
    }

    static func watched(for title: String) -> Set<String> {
        let all = UserDefaults.standard.dictionary(forKey: "watched") as? [String: [String]] ?? [:]
        return Set(all[title] ?? [])
    }

    static func markWatched(_ episode: String, for title: String) {
        var all = UserDefaults.standard.dictionary(forKey: "watched") as? [String: [String]] ?? [:]
        var list = all[title] ?? []
        if !list.contains(episode) { list.append(episode) }
        all[title] = list
        UserDefaults.standard.set(all, forKey: "watched")
    }

    static func subtitleLanguage(for title: String) -> String? {
        map("sublang")[title]
    }

    static func remember(subtitle: String?, for title: String) {
        var all = map("sublang")
        all[title] = subtitle ?? "off"
        store("sublang", all)
    }
}
