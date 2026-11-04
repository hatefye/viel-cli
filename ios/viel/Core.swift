import Foundation

struct Settings: Codable {
    var host = "https://viel.tv"
    var maxHeight = 1080
    var subtitleLang = "en"
    var dub = false
    var autoplay = true

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

@MainActor
final class Core: ObservableObject {
    static let shared = Core()

    let port: UInt16
    @Published var username: String?
    @Published var settings: Settings {
        didSet { settings.save() }
    }

    private init() {
        let settings = Settings.load()
        self.settings = settings
        let token = UserDefaults.standard.string(forKey: "token") ?? ""
        port = viel_start(settings.host, token)
        Task {
            await pushSettings()
            await refreshUser()
        }
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
        }
        let _: Ok? = try? await post("settings", Body(max_height: settings.maxHeight, subtitle_lang: settings.subtitleLang))
    }

    func saveProgress(_ progress: Progress) async {
        guard username != nil else { return }
        let _: Ok? = try? await post("progress", progress)
    }
}

struct Ok: Decodable {
    let ok: Bool
}
