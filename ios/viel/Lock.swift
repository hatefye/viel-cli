import LocalAuthentication
import SwiftUI

@MainActor
final class AppLock: ObservableObject {
    static let shared = AppLock()

    @Published var locked = false
    private var busy = false
    private var hidAt: Date?

    func launch(enabled: Bool) {
        locked = enabled
        if enabled { unlock() }
    }

    func background() {
        hidAt = Date()
    }

    func foreground(enabled: Bool) {
        guard enabled, hidAt != nil else { return }
        hidAt = nil
        locked = true
        unlock()
    }

    func unlock() {
        guard !busy else { return }
        busy = true
        let context = LAContext()
        context.localizedCancelTitle = "Not now"
        var error: NSError?
        guard context.canEvaluatePolicy(.deviceOwnerAuthentication, error: &error) else {
            locked = false
            busy = false
            return
        }
        context.evaluatePolicy(.deviceOwnerAuthentication, localizedReason: "Unlock viel") { ok, _ in
            Task { @MainActor in
                self.busy = false
                if ok { self.locked = false }
            }
        }
    }
}

struct LockScreen: View {
    @ObservedObject var lock: AppLock

    var body: some View {
        ZStack {
            Theme.bg.ignoresSafeArea()
            VStack(spacing: 18) {
                Image(systemName: "lock.fill").font(.system(size: 42)).foregroundStyle(Theme.soft)
                Text("viel is locked").font(.system(size: 20, weight: .semibold)).foregroundStyle(.white)
                Button {
                    lock.unlock()
                } label: {
                    Text("Unlock")
                        .font(.system(size: 16, weight: .semibold))
                        .foregroundStyle(.black)
                        .padding(.horizontal, 26)
                        .padding(.vertical, 10)
                }
                .buttonStyle(.glassProminent)
                .tint(.white)
            }
        }
    }
}
