import AVFoundation
import SwiftUI
import UIKit

final class AppDelegate: NSObject, UIApplicationDelegate {
    static var orientation: UIInterfaceOrientationMask = .allButUpsideDown

    func application(_ application: UIApplication, supportedInterfaceOrientationsFor window: UIWindow?) -> UIInterfaceOrientationMask {
        Self.orientation
    }

    func application(_ application: UIApplication, handleEventsForBackgroundURLSession identifier: String, completionHandler: @escaping () -> Void) {
        DownloadManager.shared.backgroundCompletion = completionHandler
        DownloadManager.shared.activate()
    }

    func application(_ application: UIApplication, configurationForConnecting session: UISceneSession, options: UIScene.ConnectionOptions) -> UISceneConfiguration {
        if let item = options.shortcutItem {
            Shortcuts.shared.pending = item.type
        }
        let config = UISceneConfiguration(name: nil, sessionRole: session.role)
        config.delegateClass = SceneDelegate.self
        return config
    }
}

final class SceneDelegate: NSObject, UIWindowSceneDelegate {
    func windowScene(_ windowScene: UIWindowScene, performActionFor item: UIApplicationShortcutItem, completionHandler: @escaping (Bool) -> Void) {
        Shortcuts.shared.pending = item.type
        completionHandler(true)
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
                    switch value {
                    case .background: core.enteredBackground()
                    case .active: core.enteredForeground()
                    default: break
                    }
                }
                .onAppear { Shortcuts.install() }
        }
    }
}
