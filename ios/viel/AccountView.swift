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
                            Text(message).font(.footnote).foregroundStyle(.red)
                        }
                    }
                }

                Section("Playback") {
                    Picker("Max quality", selection: $core.settings.maxHeight) {
                        ForEach(qualities, id: \.self) { q in
                            Text(q == 2160 ? "4K" : "\(q)p").tag(q)
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
                    Text("Changing the server takes effect the next time the app starts.")
                }
            }
            .navigationTitle(core.username == nil ? "Sign in" : "Account")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
            }
            .onChange(of: core.settings.maxHeight) { _ in Task { await core.pushSettings() } }
            .onChange(of: core.settings.subtitleLang) { _ in Task { await core.pushSettings() } }
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
