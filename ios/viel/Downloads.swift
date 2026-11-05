import Foundation

struct DownloadPlan: Codable {
    struct Part: Codable {
        let url: String
        let file: String
        let ts: Bool
    }

    let key: String
    let source: String
    let referer: String?
    let total: Int
    let parts: [Part]
}

struct FinishReply: Decodable {
    let done: Bool
    let missing: Int?
    let files: [String]?
}

final class DownloadManager: NSObject, URLSessionDownloadDelegate {
    static let shared = DownloadManager()

    var backgroundCompletion: (() -> Void)?

    private let agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36"
    private var retries: [String: Int] = [:]
    private var failures: [String: String] = [:]
    private let lock = NSLock()

    private lazy var session: URLSession = {
        let config = URLSessionConfiguration.background(withIdentifier: "tv.viel.downloads")
        config.isDiscretionary = false
        config.sessionSendsLaunchEvents = true
        config.httpMaximumConnectionsPerHost = 6
        config.timeoutIntervalForResource = 6 * 3600
        return URLSession(configuration: config, delegate: self, delegateQueue: nil)
    }()

    func activate() {
        _ = session
    }

    @MainActor
    func start(_ item: Playable, core: Core) async throws {
        let request = item.request
        let body = DownloadRequest(kind: request.kind, id: request.id, title: request.title, season: request.season, episode: request.episode, dub: request.dub, sources: request.sources, key: item.downloadKey, meta: item.downloadMeta, max_height: core.settings.downloadHeight)
        let plan: DownloadPlan = try await core.post("downloads/plan", body)
        store(plan)
        enqueue(plan, only: nil)
        core.log("download", "queued \(plan.key): \(plan.total) parts via \(plan.source)")
    }

    func cancel(_ key: String) async {
        for task in await session.allTasks where task.taskDescription?.hasPrefix(key + "|") == true {
            task.cancel()
        }
        forget(key)
    }

    @MainActor
    func resume(core: Core) async {
        let pending = await core.downloads().filter { $0.state == "fetching" }
        guard !pending.isEmpty else { return }
        let live = Set(await session.allTasks.compactMap { $0.taskDescription?.split(separator: "|").first.map(String.init) })
        for item in pending where !live.contains(item.key) {
            await settle(item.key)
        }
    }

    private func enqueue(_ plan: DownloadPlan, only: Set<String>?) {
        let wifiOnly = Settings.load().wifiDownloads
        for part in plan.parts where only == nil || only!.contains(part.file) {
            guard let url = URL(string: part.url) else { continue }
            var request = URLRequest(url: url)
            request.allowsCellularAccess = !wifiOnly
            request.setValue(agent, forHTTPHeaderField: "User-Agent")
            if let referer = plan.referer, !referer.isEmpty {
                request.setValue(referer, forHTTPHeaderField: "Referer")
            }
            let task = session.downloadTask(with: request)
            task.taskDescription = "\(plan.key)|\(part.file)|\(part.ts ? 1 : 0)"
            task.resume()
        }
    }

    private func store(_ plan: DownloadPlan) {
        var all = UserDefaults.standard.dictionary(forKey: "plans") as? [String: Data] ?? [:]
        all[plan.key] = try? JSONEncoder().encode(plan)
        UserDefaults.standard.set(all, forKey: "plans")
    }

    private func plan(for key: String) -> DownloadPlan? {
        let all = UserDefaults.standard.dictionary(forKey: "plans") as? [String: Data] ?? [:]
        return all[key].flatMap { try? JSONDecoder().decode(DownloadPlan.self, from: $0) }
    }

    private func forget(_ key: String) {
        var all = UserDefaults.standard.dictionary(forKey: "plans") as? [String: Data] ?? [:]
        all[key] = nil
        UserDefaults.standard.set(all, forKey: "plans")
        lock.lock()
        retries[key] = nil
        failures[key] = nil
        lock.unlock()
    }

    private func settle(_ key: String) async {
        let core = await Core.shared
        let reply: FinishReply? = try? await core.post("downloads/finish", ["key": key])
        guard let reply else {
            await core.log("download", "\(key): finish failed")
            return
        }
        if reply.done {
            forget(key)
            return
        }
        lock.lock()
        let attempt = (retries[key] ?? 0) + 1
        retries[key] = attempt
        let reason = failures[key]
        lock.unlock()
        if attempt <= 3, let plan = plan(for: key), let files = reply.files {
            await core.log("download", "\(key): \(files.count) parts missing, retry \(attempt)")
            enqueue(plan, only: Set(files))
        } else {
            let _: Ok? = try? await core.post("downloads/fail", ["key": key, "error": reason ?? "\(reply.missing ?? 0) parts could not be fetched"])
            forget(key)
        }
    }

    private func unmask(_ data: Data) -> Data {
        guard data.count > 376 else { return data }
        return data.withUnsafeBytes { raw -> Data in
            let bytes = raw.bindMemory(to: UInt8.self)
            if bytes[0] == 0x47, bytes[188] == 0x47 { return data }
            let limit = min(8192, data.count - 376)
            for i in 0..<limit where bytes[i] == 0x47 && bytes[i + 188] == 0x47 && bytes[i + 376] == 0x47 {
                return data.subdata(in: i..<data.count)
            }
            return data
        }
    }

    func urlSession(_ session: URLSession, downloadTask: URLSessionDownloadTask, didFinishDownloadingTo location: URL) {
        guard let description = downloadTask.taskDescription else { return }
        let pieces = description.split(separator: "|").map(String.init)
        guard pieces.count == 3 else { return }
        let code = (downloadTask.response as? HTTPURLResponse)?.statusCode ?? 0
        guard code == 200, var data = try? Data(contentsOf: location) else {
            lock.lock()
            failures[pieces[0]] = "part failed with \(code)"
            lock.unlock()
            return
        }
        if pieces[2] == "1" { data = unmask(data) }
        let folder = Core.downloadsRoot.appendingPathComponent(pieces[0], isDirectory: true)
        try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        try? data.write(to: folder.appendingPathComponent(pieces[1]), options: .atomic)
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: Error?) {
        guard let description = task.taskDescription, let key = description.split(separator: "|").first.map(String.init) else { return }
        if let error, (error as NSError).code != NSURLErrorCancelled {
            lock.lock()
            failures[key] = error.localizedDescription
            lock.unlock()
        }
        session.getAllTasks { tasks in
            let remaining = tasks.contains { $0 !== task && $0.taskDescription?.hasPrefix(key + "|") == true && $0.state != .completed }
            guard !remaining else { return }
            Task { await self.settle(key) }
        }
    }

    func urlSessionDidFinishEvents(forBackgroundURLSession session: URLSession) {
        DispatchQueue.main.async {
            self.backgroundCompletion?()
            self.backgroundCompletion = nil
        }
    }
}
