import AppIntents

struct ResumeIntent: AppIntent {
    static var title: LocalizedStringResource = "Resume watching"
    static var description = IntentDescription("Picks up where you left off in viel.")
    static var openAppWhenRun = true

    @MainActor
    func perform() async throws -> some IntentResult {
        Shortcuts.shared.pending = Shortcuts.resume
        return .result()
    }
}

struct SearchIntent: AppIntent {
    static var title: LocalizedStringResource = "Search viel"
    static var description = IntentDescription("Opens search in viel.")
    static var openAppWhenRun = true

    @MainActor
    func perform() async throws -> some IntentResult {
        Shortcuts.shared.pending = Shortcuts.search
        return .result()
    }
}

struct VielShortcuts: AppShortcutsProvider {
    static var appShortcuts: [AppShortcut] {
        AppShortcut(intent: ResumeIntent(), phrases: ["Resume on \(.applicationName)", "Continue watching on \(.applicationName)"], shortTitle: "Resume", systemImageName: "play.fill")
        AppShortcut(intent: SearchIntent(), phrases: ["Search \(.applicationName)"], shortTitle: "Search", systemImageName: "magnifyingglass")
    }
}
