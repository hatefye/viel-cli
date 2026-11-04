import AVFoundation
import SwiftUI
import UIKit

final class AppDelegate: NSObject, UIApplicationDelegate {
    static var orientation: UIInterfaceOrientationMask = .allButUpsideDown

    func application(_ application: UIApplication, supportedInterfaceOrientationsFor window: UIWindow?) -> UIInterfaceOrientationMask {
        Self.orientation
    }
}

@main
struct VielApp: App {
    @UIApplicationDelegateAdaptor(AppDelegate.self) private var delegate
    @StateObject private var core = Core.shared
    @Environment(\.scenePhase) private var phase

    init() {
        let session = AVAudioSession.sharedInstance()
        try? session.setCategory(.playback, mode: .moviePlayback)
        try? session.setActive(true)
    }

    var body: some Scene {
        WindowGroup {
            MainView()
                .environmentObject(core)
                .preferredColorScheme(.dark)
                .tint(.white)
                .onChange(of: phase) { _, value in
                    core.log("app", "scene \(value)")
                    if value == .background {
                        Task { await core.flushLogs() }
                    }
                }
        }
    }
}
