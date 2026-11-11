import SwiftUI

struct ProfilesView: View {
    @EnvironmentObject private var core: Core
    @State private var list: ProfileList?
    @State private var message: String?
    @State private var busy = false
    @State private var naming = false
    @State private var name = ""
    @State private var editing: Profile?
    @State private var removing: Profile?

    private let palette: [UInt32] = [0xdc2626, 0x2563eb, 0x059669, 0xd97706, 0x7c3aed]

    private var profiles: [Profile] { list?.profiles ?? [] }

    var body: some View {
        Form {
            Section {
                ForEach(profiles) { profile in
                    Button {
                        Task { await use(profile) }
                    } label: {
                        HStack(spacing: 12) {
                            Text(String(profile.name.prefix(1)).uppercased())
                                .font(.system(size: 15, weight: .bold))
                                .foregroundStyle(.white)
                                .frame(width: 34, height: 34)
                                .background(tint(profile), in: Circle())
                            Text(profile.name).foregroundStyle(.white)
                            Spacer()
                            if profile.active {
                                Image(systemName: "checkmark").font(.system(size: 14, weight: .semibold)).foregroundStyle(Theme.accentHi)
                            }
                        }
                    }
                    .disabled(busy)
                    .swipeActions(edge: .trailing, allowsFullSwipe: false) {
                        if profiles.count > 1 {
                            Button("Delete") { removing = profile }.tint(Theme.accent)
                        }
                        Button("Rename") {
                            name = profile.name
                            editing = profile
                            naming = true
                        }
                    }
                }
                if list == nil, message == nil {
                    HStack { Spacer(); ProgressView(); Spacer() }
                }
            } footer: {
                Text("Each profile keeps its own list, watch history and manga. Swipe a profile to rename or delete it.")
            }
            if let list, list.profiles.count < list.max {
                Section {
                    Button("Add a profile") {
                        name = ""
                        editing = nil
                        naming = true
                    }
                    .disabled(busy)
                }
            }
            if let message {
                Section {
                    Text(message).font(.footnote).foregroundStyle(Theme.accentHi)
                }
            }
        }
        .scrollContentBackground(.hidden)
        .background(Theme.bg)
        .tint(Theme.accent)
        .navigationTitle("Profiles")
        .navigationBarTitleDisplayMode(.inline)
        .toolbarBackground(Theme.bg, for: .navigationBar)
        .task { await run { try await core.profiles() } }
        .alert(editing == nil ? "New profile" : "Rename profile", isPresented: $naming) {
            TextField("Name", text: $name)
            Button("Save") { Task { await save() } }
            Button("Cancel", role: .cancel) {}
        }
        .confirmationDialog("Delete \(removing?.name ?? "this profile")?", isPresented: Binding(get: { removing != nil }, set: { if !$0 { removing = nil } }), titleVisibility: .visible) {
            Button("Delete profile", role: .destructive) {
                if let target = removing { Task { await remove(target) } }
            }
        } message: {
            Text("Its list, history and manga progress go with it.")
        }
    }

    private func tint(_ profile: Profile) -> Color {
        if profile.color.count == 7, profile.color.hasPrefix("#"), let value = UInt32(profile.color.dropFirst(), radix: 16) {
            return Color(hex: value)
        }
        return Color(hex: palette[abs(profile.id) % palette.count])
    }

    @discardableResult
    private func run(_ work: () async throws -> ProfileList) async -> Bool {
        busy = true
        defer { busy = false }
        do {
            list = try await work()
            message = nil
            return true
        } catch {
            message = error.localizedDescription
            return false
        }
    }

    private func use(_ profile: Profile) async {
        guard !profile.active else { return }
        Haptic.select()
        if await run({ try await core.editProfiles("use", id: profile.id) }) {
            await core.profileSwitched()
        }
    }

    private func save() async {
        let clean = name.trimmingCharacters(in: .whitespaces)
        guard !clean.isEmpty else { return }
        if let editing {
            await run { try await core.editProfiles("rename", id: editing.id, name: clean) }
        } else {
            await run { try await core.editProfiles("create", name: clean) }
        }
    }

    private func remove(_ profile: Profile) async {
        removing = nil
        if await run({ try await core.editProfiles("remove", id: profile.id) }), profile.active {
            await core.profileSwitched()
        }
    }
}
