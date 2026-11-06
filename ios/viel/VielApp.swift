import AVFoundation
import CoreSpotlight
import SwiftUI
import UIKit
import UserNotifications

final class AppDelegate: NSObject, UIApplicationDelegate, UNUserNotificationCenterDelegate {
    static var orientation: UIInterfaceOrientationMask = .allButUpsideDown

    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil) -> Bool {
        UNUserNotificationCenter.current().delegate = self
        Alerts.register()
        return true
    }

    func userNotificationCenter(_ center: UNUserNotificationCenter, willPresent notification: UNNotification) async -> UNNotificationPresentationOptions {
        [.banner, .sound]
    }

    func userNotificationCenter(_ center: UNUserNotificationCenter, didReceive response: UNNotificationResponse) async {
        guard let raw = response.notification.request.content.userInfo["url"] as? String, let url = URL(string: raw), let link = DeepLink(url) else { return }
        await MainActor.run {
            Shortcuts.shared.pending = Shortcuts.home
            Shortcuts.shared.link = link
        }
    }

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
    @StateObject private var lock = AppLock.shared
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
                .overlay {
                    if lock.locked {
                        LockScreen(lock: lock).transition(.opacity)
                    }
                }
                .onChange(of: phase) { _, value in
                    core.log("app", "scene \(value)")
                    switch value {
                    case .background:
                        core.enteredBackground()
                        lock.background()
                    case .active:
                        core.enteredForeground()
                        lock.foreground(enabled: core.settings.appLock)
                    default: break
                    }
                }
                .onAppear {
                    Shortcuts.install()
                    lock.launch(enabled: core.settings.appLock)
                }
                .onContinueUserActivity(CSSearchableItemActionType) { activity in
                    guard let raw = activity.userInfo?[CSSearchableItemActivityIdentifier] as? String, let url = URL(string: raw), let link = DeepLink(url) else { return }
                    Shortcuts.shared.pending = Shortcuts.home
                    Shortcuts.shared.link = link
                }
        }
    }
}
