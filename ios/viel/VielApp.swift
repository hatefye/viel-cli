import AVFoundation
import SwiftUI

@main
struct VielApp: App {
    @StateObject private var core = Core.shared

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
                .tint(.red)
        }
    }
}
