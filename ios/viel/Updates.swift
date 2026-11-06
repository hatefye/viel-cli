import Foundation

struct AppRelease: Decodable {
    let version: String
    let url: String?
    let notes: String?
}

enum Updates {
    @MainActor
    static func check(core: Core) async -> AppRelease? {
        guard let url = URL(string: "\(core.settings.host)/desktop/version-ios.json") else { return nil }
        var request = URLRequest(url: url)
        request.cachePolicy = .reloadIgnoringLocalCacheData
        request.timeoutInterval = 15
        guard let reply = try? await URLSession.shared.data(for: request), let found = try? JSONDecoder().decode(AppRelease.self, from: reply.0) else { return nil }
        guard newer(found.version, core.build), UserDefaults.standard.string(forKey: "skipVersion") != found.version else { return nil }
        return found
    }

    static func skip(_ version: String) {
        UserDefaults.standard.set(version, forKey: "skipVersion")
    }

    static func newer(_ a: String, _ b: String) -> Bool {
        let x = a.split(separator: ".").map { Int($0) ?? 0 }
        let y = b.split(separator: ".").map { Int($0) ?? 0 }
        for i in 0..<3 {
            let l = i < x.count ? x[i] : 0
            let r = i < y.count ? y[i] : 0
            if l != r { return l > r }
        }
        return false
    }
}
