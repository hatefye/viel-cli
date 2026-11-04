import SwiftUI

struct RootView: View {
    @EnvironmentObject private var core: Core
    @State private var query = ""
    @State private var results: [Title] = []
    @State private var home: Home?
    @State private var searching = false
    @State private var showAccount = false
    @State private var error: String?

    private var browsing: Bool { query.trimmingCharacters(in: .whitespaces).isEmpty }

    var body: some View {
        NavigationStack {
            Group {
                if browsing {
                    homeList
                } else {
                    resultList
                }
            }
            .listStyle(.plain)
            .navigationTitle("viel")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button {
                        showAccount = true
                    } label: {
                        Image(systemName: core.username == nil ? "person.circle" : "person.circle.fill")
                    }
                }
            }
            .navigationDestination(for: Title.self) { TitleView(title: $0, resume: nil) }
            .navigationDestination(for: Resume.self) { TitleView(title: $0.title, resume: $0) }
        }
        .searchable(text: $query, prompt: "Movies, TV and anime")
        .task(id: query) { await search() }
        .task { await loadHome() }
        .sheet(isPresented: $showAccount, onDismiss: { Task { await loadHome() } }) {
            AccountView()
        }
    }

    private var homeList: some View {
        List {
            if let home, !home.resume.isEmpty {
                Section("Continue watching") {
                    ForEach(home.resume, id: \.key) { row in
                        NavigationLink(value: row) {
                            HStack(spacing: 12) {
                                Poster(url: row.poster)
                                VStack(alignment: .leading, spacing: 3) {
                                    Text(row.show).font(.headline).lineLimit(1)
                                    Text(row.label).font(.subheadline).foregroundStyle(.secondary)
                                    ProgressView(value: min(max(row.position / max(row.duration, 1), 0), 1))
                                        .tint(.red)
                                }
                            }
                        }
                    }
                }
            }
            Section(home == nil ? "" : "Trending") {
                if let home {
                    ForEach(home.trending, id: \.key) { title in
                        NavigationLink(value: title) { TitleRow(title: title) }
                    }
                } else if let error {
                    Label(error, systemImage: "exclamationmark.triangle").foregroundStyle(.secondary)
                } else {
                    HStack { Spacer(); ProgressView(); Spacer() }
                }
            }
        }
    }

    private var resultList: some View {
        List {
            if results.isEmpty {
                HStack {
                    Spacer()
                    if searching { ProgressView() } else { Text("Nothing found").foregroundStyle(.secondary) }
                    Spacer()
                }
            }
            ForEach(results, id: \.key) { title in
                NavigationLink(value: title) { TitleRow(title: title) }
            }
        }
    }

    private func loadHome() async {
        do {
            home = try await core.get("home")
            error = nil
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func search() async {
        let q = query.trimmingCharacters(in: .whitespaces)
        guard !q.isEmpty else {
            results = []
            return
        }
        try? await Task.sleep(nanoseconds: 250_000_000)
        guard !Task.isCancelled else { return }
        searching = true
        let found: [Title] = (try? await core.get("search", ["q": q])) ?? []
        guard !Task.isCancelled else { return }
        results = found
        searching = false
    }
}

struct TitleRow: View {
    let title: Title

    var body: some View {
        HStack(spacing: 12) {
            Poster(url: title.poster)
            VStack(alignment: .leading, spacing: 3) {
                Text(title.name).font(.headline).lineLimit(1)
                Text(title.subtitle).font(.subheadline).foregroundStyle(.secondary)
                if let overview = title.overview, !overview.isEmpty {
                    Text(overview).font(.caption).foregroundStyle(.secondary).lineLimit(2)
                }
            }
        }
    }
}
