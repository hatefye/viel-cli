import SwiftUI

struct AccountView: View {
    var embedded = false

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
                    Toggle("Keep playing in the background", isOn: $core.settings.backgroundAudio)
                    Picker("Skip buttons", selection: $core.settings.skipSeconds) {
                        ForEach([10, 15, 30], id: \.self) { seconds in
                            Text("\(seconds) seconds").tag(seconds)
                        }
                    }
                }

                Section {
                    Picker("Download quality", selection: $core.settings.downloadHeight) {
                        ForEach(qualities, id: \.self) { quality in
                            Text(quality == 2160 ? "4K" : "\(quality)p").tag(quality)
                        }
                    }
                    Toggle("Delete downloads after watching", isOn: $core.settings.autoDeleteWatched)
                    Toggle("Download the next episode ahead", isOn: $core.settings.autoDownloadNext)
                    Toggle("Wi-Fi only", isOn: $core.settings.wifiDownloads)
                } header: {
                    Text("Downloads")
                } footer: {
                    Text("Downloads finish as MP4 files under Files → viel → Downloads. Next episode ahead queues the following episode as soon as one starts playing. Wi-Fi only applies to downloads started after you turn it on.")
                }

                Section {
                    Toggle("New episode alerts", isOn: $core.settings.episodeAlerts)
                } header: {
                    Text("Alerts")
                } footer: {
                    Text("A notification when a show or anime in My List gets a new episode. Upcoming episodes also show at the top of My List.")
                }

                Section {
                    Toggle("Stream to AirPlay devices", isOn: $core.settings.airplay)
                } header: {
                    Text("AirPlay")
                } footer: {
                    Text("Lets an Apple TV pull the video straight from this phone over Wi-Fi instead of mirroring the screen. Turn it off if playback stalls when switching networks.")
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
            .navigationTitle(embedded ? "Settings" : (core.username == nil ? "Sign in" : "Account"))
            .navigationBarTitleDisplayMode(embedded ? .large : .inline)
            .toolbarBackground(Theme.bg, for: .navigationBar)
            .toolbar {
                if !embedded {
                    ToolbarItem(placement: .confirmationAction) {
                        Button("Done") { dismiss() }.foregroundStyle(.white)
                    }
                }
            }
            .onChange(of: core.settings.maxHeight) { _, _ in Task { await core.pushSettings() } }
            .onChange(of: core.settings.subtitleLang) { _, _ in Task { await core.pushSettings() } }
            .onChange(of: core.settings.airplay) { _, _ in Task { await core.pushSettings() } }
            .onChange(of: core.settings.episodeAlerts) { _, on in
                guard on else { return }
                Task {
                    if await Alerts.authorize() {
                        await Alerts.refresh(core: core)
                    } else {
                        core.settings.episodeAlerts = false
                    }
                }
            }
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
