import SwiftUI

struct SearchView: View {
    @EnvironmentObject private var core: Core
    @Environment(\.dismiss) private var dismiss
    @FocusState private var focused: Bool
    @State private var query = ""
    @State private var results: [Title] = []
    @State private var trending: [Title] = []
    @State private var searching = false
    @State private var recent: [String] = UserDefaults.standard.stringArray(forKey: "recent") ?? []
    @State private var selected: Title?
    @State private var allResults: ListPage?

    private var trimmed: String { query.trimmingCharacters(in: .whitespaces) }

    var body: some View {
        ZStack(alignment: .top) {
            Theme.bg.opacity(0.94).ignoresSafeArea()
                .onTapGesture { dismiss() }
            VStack(spacing: 14) {
                HStack {
                    Spacer()
                    Button { dismiss() } label: { Image(systemName: "xmark") }
                        .buttonStyle(GlassCircle(size: 40))
                }
                .padding(.horizontal, 16)
                panel
            }
            .padding(.top, 8)
        }
        .task {
            focused = true
            trending = (try? await core.get("list", ["path": "trending/all/day"])) ?? []
        }
        .task(id: query) { await search() }
        .fullScreenCover(item: $selected) { DetailsView(title: $0, resume: nil).environmentObject(core) }
        .fullScreenCover(item: $allResults) { page in
            NavigationStack { ListView(page: page) }.environmentObject(core)
        }
    }

    private var panel: some View {
        VStack(alignment: .leading, spacing: 18) {
            HStack(spacing: 12) {
                Image(systemName: "magnifyingglass")
                    .foregroundStyle(Theme.muted)
                TextField("Search movies, shows, anime…", text: $query)
                    .font(.system(size: 19))
                    .focused($focused)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .submitLabel(.search)
                    .onSubmit { remember(trimmed) }
                if !query.isEmpty {
                    Button { query = "" } label: { Image(systemName: "xmark.circle.fill").foregroundStyle(Theme.muted) }
                }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 14)
            .background(Color.white.opacity(0.05), in: RoundedRectangle(cornerRadius: 20, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 20, style: .continuous).stroke(Theme.borderStrong, lineWidth: 1))

            ScrollView(showsIndicators: false) {
                VStack(alignment: .leading, spacing: 16) {
                    if trimmed.isEmpty {
                        if !recent.isEmpty {
                            Text("Recent").font(.system(size: 22, weight: .semibold))
                            FlowLayout(spacing: 8) {
                                ForEach(recent, id: \.self) { term in
                                    Button { query = term } label: { Tag(text: term, fill: Color.white.opacity(0.06)) }
                                }
                            }
                        }
                        Text("Trending").font(.system(size: 22, weight: .semibold))
                        ForEach(trending.prefix(8), id: \.key) { title in
                            ResultRow(title: title).onTapGesture { selected = title }
                        }
                    } else {
                        Button {
                            remember(trimmed)
                            allResults = ListPage(title: "Results for \u{201c}\(trimmed)\u{201d}", items: results)
                        } label: {
                            Text("See all results for \u{201c}\(trimmed)\u{201d}")
                                .font(.system(size: 20, weight: .medium))
                                .foregroundStyle(.white)
                        }
                        if results.isEmpty {
                            HStack {
                                Spacer()
                                if searching { ProgressView().tint(.white) } else { Text("Nothing found").foregroundStyle(Theme.muted) }
                                Spacer()
                            }
                            .padding(.top, 20)
                        }
                        ForEach(results, id: \.key) { title in
                            ResultRow(title: title).onTapGesture {
                                remember(trimmed)
                                selected = title
                            }
                        }
                    }
                }
                .padding(.horizontal, 6)
                .padding(.bottom, 20)
            }
        }
        .padding(16)
        .background(Theme.surface.opacity(0.92), in: RoundedRectangle(cornerRadius: 26, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 26, style: .continuous).stroke(Theme.border, lineWidth: 1))
        .padding(.horizontal, 12)
        .padding(.bottom, 12)
    }

    private func remember(_ term: String) {
        guard !term.isEmpty else { return }
        var list = recent.filter { $0.lowercased() != term.lowercased() }
        list.insert(term, at: 0)
        recent = Array(list.prefix(6))
        UserDefaults.standard.set(recent, forKey: "recent")
    }

    private func search() async {
        let q = trimmed
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

struct ResultRow: View {
    let title: Title

    var body: some View {
        HStack(spacing: 16) {
            Poster(url: title.poster, width: 58, height: 84, radius: 12)
            VStack(alignment: .leading, spacing: 4) {
                Text(title.name).font(.system(size: 19, weight: .semibold)).lineLimit(2)
                Text(title.kindLabel).font(.system(size: 16)).foregroundStyle(Theme.muted)
            }
            Spacer()
        }
        .contentShape(Rectangle())
    }
}
