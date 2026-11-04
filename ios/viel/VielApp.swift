import AVFoundation
import SwiftUI

@main
struct VielApp: App {
    @StateObject private var core = Core.shared
    @Environment(\.scenePhase) private var phase

    init() {
        let session = AVAudioSession.sharedInstance()
        try? session.setCategory(.playback, mode: .moviePlayback)
        try? session.setActive(true)
    }

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(core)
                .preferredColorScheme(.dark)
                .tint(.white)
                .onChange(of: phase) { value in
                    core.log("app", "scene \(value)")
                    if value == .background {
                        Task { await core.flushLogs() }
                    }
                }
        }
    }
}
