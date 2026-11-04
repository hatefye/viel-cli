import SwiftUI

struct AccountView: View {
    @EnvironmentObject private var core: Core
    @Environment(\.dismiss) private var dismiss
    @State private var username = ""
    @State private var password = ""
    @State private var busy = false
    @State private var message: String?

    private let qualities = [720, 1080, 1440, 2160]

    var body: some View {
        NavigationStack {
            Form {
                Section("Account") {
                    if let user = core.username {
                        LabeledContent("Signed in as", value: user)
                        Button("Sign out", role: .destructive) {
                            Task { await core.logout() }
                        }
                    } else {
                        TextField("Username", text: $username)
                            .textInputAutocapitalization(.never)
                            .autocorrectionDisabled()
                        SecureField("Password", text: $password)
                        Button("Sign in") { Task { await submit(register: false) } }
                            .disabled(busy || username.isEmpty || password.isEmpty)
                        Button("Create account") { Task { await submit(register: true) } }
                            .disabled(busy || username.isEmpty || password.isEmpty)
                        if let message {
                            Text(message).font(.footnote).foregroundStyle(Theme.accentHi)
                        }
                    }
                }

                Section("Playback") {
                    Picker("Max quality", selection: $core.settings.maxHeight) {
                        ForEach(qualities, id: \.self) { quality in
                            Text(quality == 2160 ? "4K" : "\(quality)p").tag(quality)
                        }
                    }
                    TextField("Subtitle language", text: $core.settings.subtitleLang)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                    Toggle("Prefer dubbed anime", isOn: $core.settings.dub)
                    Toggle("Autoplay next episode", isOn: $core.settings.autoplay)
                }

                Section {
                    TextField("Server", text: $core.settings.host)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .keyboardType(.URL)
                } header: {
                    Text("Server")
                } footer: {
                    Text("Changing the server takes effect the next time the app starts. Device \(core.device) · build \(core.build)")
                }
            }
            .scrollContentBackground(.hidden)
            .background(Theme.bg)
            .tint(Theme.accent)
            .navigationTitle(core.username == nil ? "Sign in" : "Account")
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(Theme.bg, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }.foregroundStyle(.white)
                }
            }
            .onChange(of: core.settings.maxHeight) { _, _ in Task { await core.pushSettings() } }
            .onChange(of: core.settings.subtitleLang) { _, _ in Task { await core.pushSettings() } }
        }
    }

    private func submit(register: Bool) async {
        busy = true
        message = nil
        defer { busy = false }
        do {
            try await core.login(username.trimmingCharacters(in: .whitespaces), password, register: register)
            password = ""
        } catch {
            message = error.localizedDescription
        }
    }
}
