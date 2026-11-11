import SwiftUI

struct MangaView: View {
    @EnvironmentObject private var core: Core
    @ObservedObject private var store = MangaStore.shared
    @State private var query = ""
    @State private var sort = "popular"
    @State private var items: [MangaCard] = []
    @State private var next: Int?
    @State private var loading = false
    @State private var failed = false
    @State private var reading: MangaJump?

    private let columns = [GridItem(.flexible(), spacing: 10), GridItem(.flexible(), spacing: 10), GridItem(.flexible(), spacing: 10)]
    private let sorts = [("popular", "Popular"), ("latest", "Latest"), ("new", "New")]

    private var trimmed: String { query.trimmingCharacters(in: .whitespaces) }

    var body: some View {
        NavigationStack {
            ZStack {
                Theme.bg.ignoresSafeArea()
                ScrollView(showsIndicators: false) {
                    VStack(alignment: .leading, spacing: 26) {
                        if trimmed.isEmpty, !store.shelf.isEmpty {
                            Row(title: "Continue reading") {
                                ForEach(store.shelf.prefix(20)) { mark in
                                    MangaResumeCard(mark: mark)
                                        .onTapGesture {
                                            Haptic.tap()
                                            reading = MangaJump(chapter: mark.chapter)
                                        }
                                        .contextMenu {
                                            Button(role: .destructive) {
                                                store.remove(mark.id)
                                            } label: {
                                                Label("Remove from Continue Reading", systemImage: "trash")
                                            }
                                        }
                                }
                            }
                        }
                        VStack(alignment: .leading, spacing: 14) {
                            if trimmed.isEmpty {
                                Picker("Sort", selection: $sort) {
                                    ForEach(sorts, id: \.0) { Text($0.1).tag($0.0) }
                                }
                                .pickerStyle(.segmented)
                            } else {
                                Text("Results").font(.system(size: 22, weight: .bold))
                            }
                            grid
                        }
                        .padding(.horizontal, 16)
                    }
                    .padding(.top, 8)
                    .padding(.bottom, 40)
                }
                .refreshable {
                    await store.sync(core: core)
                    await reload()
                }
            }
            .navigationTitle("Manga")
            .toolbarBackground(Theme.bg, for: .navigationBar)
            .searchable(text: $query, prompt: "Search manga")
            .navigationDestination(for: MangaCard.self) { MangaTitleView(card: $0) }
        }
        .task(id: trimmed + "|" + sort) {
            if !items.isEmpty {
                try? await Task.sleep(nanoseconds: 350_000_000)
            }
            guard !Task.isCancelled else { return }
            await reload()
        }
        .task { await store.sync(core: core) }
        .fullScreenCover(item: $reading) { MangaReader(start: $0.chapter).environmentObject(core) }
    }

    @ViewBuilder
    private var grid: some View {
        if items.isEmpty, loading {
            HStack { Spacer(); ProgressView().tint(.white); Spacer() }.padding(.top, 40)
        } else if items.isEmpty, failed {
            VStack(alignment: .leading, spacing: 10) {
                Text("Manga could not be loaded.").foregroundStyle(Theme.muted)
                Button("Try again") { Task { await reload() } }
            }
            .font(.system(size: 15))
        } else if items.isEmpty {
            Text(trimmed.isEmpty ? "Nothing here yet." : "Nothing matches \u{201C}\(trimmed)\u{201D}.")
                .font(.system(size: 15))
                .foregroundStyle(Theme.muted)
        } else {
            LazyVGrid(columns: columns, spacing: 16) {
                ForEach(items) { card in
                    NavigationLink(value: card) {
                        VStack(alignment: .leading, spacing: 6) {
                            GeometryReader { geometry in
                                Poster(url: card.cover, width: geometry.size.width, height: geometry.size.width * 1.5, radius: 10)
                            }
                            .aspectRatio(2 / 3, contentMode: .fit)
                            Text(card.title).font(.system(size: 13, weight: .medium)).lineLimit(1)
                            Text(card.caption).font(.system(size: 12)).foregroundStyle(Theme.muted).lineLimit(1)
                        }
                    }
                    .buttonStyle(.plain)
                    .onAppear {
                        if card.id == items.last?.id { Task { await more() } }
                    }
                }
            }
            if loading {
                HStack { Spacer(); ProgressView().tint(.white); Spacer() }
            }
        }
    }

    private func fetch(_ offset: Int) async throws -> MangaListing {
        try await core.get("manga/list", ["q": trimmed, "sort": sort, "offset": String(offset)])
    }

    private func reload() async {
        loading = true
        defer { loading = false }
        do {
            let page = try await fetch(0)
            guard !Task.isCancelled else { return }
            items = page.items
            next = page.next
            failed = false
        } catch {
            guard !Task.isCancelled else { return }
            items = []
            next = nil
            failed = true
            core.log("manga", "list failed: \(error.localizedDescription)")
        }
    }

    private func more() async {
        guard let offset = next, !loading else { return }
        loading = true
        defer { loading = false }
        guard let page = try? await fetch(offset) else { return }
        let known = Set(items.map(\.id))
        items.append(contentsOf: page.items.filter { !known.contains($0.id) })
        next = page.next
    }
}

struct MangaTitleView: View {
    let card: MangaCard

    @EnvironmentObject private var core: Core
    @ObservedObject private var store = MangaStore.shared
    @State private var title: MangaTitle?
    @State private var failed = false
    @State private var oldestFirst = false
    @State private var expanded = false
    @State private var reading: MangaJump?

    private var chapters: [MangaChapter] {
        let all = title?.chapters ?? []
        return oldestFirst ? all.reversed() : all
    }

    var body: some View {
        ZStack {
            Theme.bg.ignoresSafeArea()
            ScrollView(showsIndicators: false) {
                VStack(alignment: .leading, spacing: 20) {
                    header
                    if let title {
                        actions(title)
                        if let text = title.description, !text.isEmpty {
                            Text(text)
                                .font(.system(size: 15))
                                .foregroundStyle(Theme.soft)
                                .lineLimit(expanded ? nil : 4)
                                .onTapGesture { withAnimation(.easeOut(duration: 0.2)) { expanded.toggle() } }
                        }
                        list(title)
                    } else if failed {
                        VStack(alignment: .leading, spacing: 10) {
                            Text("This title could not be loaded.").foregroundStyle(Theme.muted)
                            Button("Try again") { Task { await load() } }
                        }
                        .font(.system(size: 15))
                    } else {
                        HStack { Spacer(); ProgressView().tint(.white); Spacer() }.padding(.top, 30)
                    }
                }
                .padding(.horizontal, 16)
                .padding(.top, 8)
                .padding(.bottom, 40)
            }
        }
        .navigationBarTitleDisplayMode(.inline)
        .toolbarBackground(Theme.bg, for: .navigationBar)
        .task { await load() }
        .fullScreenCover(item: $reading) { MangaReader(start: $0.chapter).environmentObject(core) }
    }

    private var header: some View {
        HStack(alignment: .top, spacing: 14) {
            Poster(url: title?.cover ?? card.cover, width: 118, height: 172, radius: 12)
            VStack(alignment: .leading, spacing: 7) {
                Text(title?.title ?? card.title)
                    .font(.system(size: 23, weight: .bold))
                    .lineLimit(4)
                    .minimumScaleFactor(0.8)
                if let authors = title?.authors, !authors.isEmpty {
                    Text(authors.prefix(3).joined(separator: ", "))
                        .font(.system(size: 14))
                        .foregroundStyle(Theme.soft)
                }
                Text(title?.caption ?? card.caption)
                    .font(.system(size: 13))
                    .foregroundStyle(Theme.muted)
                if let genres = title?.genres, !genres.isEmpty {
                    Text(genres.prefix(4).joined(separator: ", "))
                        .font(.system(size: 13))
                        .foregroundStyle(Theme.muted)
                        .lineLimit(2)
                }
            }
        }
    }

    @ViewBuilder
    private func actions(_ title: MangaTitle) -> some View {
        let mark = store.mark(for: title.id)
        let current = mark.flatMap { mark in title.chapters.first { $0.id == mark.chapter } }
        if let target = current ?? title.chapters.last {
            Button {
                Haptic.tap()
                reading = MangaJump(chapter: target.id)
            } label: {
                HStack(spacing: 8) {
                    Image(systemName: "book.fill").font(.system(size: 14))
                    Text(current == nil ? "Start reading" : "Continue \(target.short)")
                        .font(.system(size: 16, weight: .semibold))
                }
                .foregroundStyle(.black)
                .frame(maxWidth: .infinity)
                .padding(.vertical, 13)
                .background(.white, in: Capsule())
            }
            .buttonStyle(.plain)
        } else {
            Text("No chapters yet.").font(.system(size: 15)).foregroundStyle(Theme.muted)
        }
    }

    private func list(_ title: MangaTitle) -> some View {
        let done = store.finished(title.id)
        let mark = store.mark(for: title.id)
        return VStack(alignment: .leading, spacing: 4) {
            HStack(alignment: .firstTextBaseline) {
                Text("\(title.chapters.count) chapters").font(.system(size: 20, weight: .bold))
                Spacer()
                Button(oldestFirst ? "Oldest first" : "Newest first") { oldestFirst.toggle() }
                    .font(.system(size: 14))
                    .foregroundStyle(Theme.muted)
            }
            .padding(.bottom, 6)
            LazyVStack(spacing: 0) {
                ForEach(chapters) { chapter in
                    Button {
                        reading = MangaJump(chapter: chapter.id)
                    } label: {
                        HStack(spacing: 10) {
                            Text(chapter.name)
                                .font(.system(size: 15, weight: mark?.chapter == chapter.id ? .semibold : .regular))
                                .foregroundStyle(done.contains(chapter.id) ? Theme.faint : .white)
                                .lineLimit(1)
                            Spacer(minLength: 8)
                            if done.contains(chapter.id) {
                                Image(systemName: "checkmark").font(.system(size: 12, weight: .semibold)).foregroundStyle(Theme.faint)
                            } else if let mark, mark.chapter == chapter.id, mark.pages > 0 {
                                Text("\(mark.page + 1)/\(mark.pages)").font(.system(size: 13)).foregroundStyle(Theme.accentHi)
                            }
                            Text(chapter.day).font(.system(size: 13)).foregroundStyle(Theme.faint).monospacedDigit()
                        }
                        .padding(.vertical, 12)
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    Divider().overlay(Theme.border)
                }
            }
        }
    }

    private func load() async {
        failed = false
        async let state: Void = store.refresh(title: card.id, core: core)
        do {
            title = try await core.get("manga/title", ["id": card.id])
        } catch {
            failed = true
            core.log("manga", "title \(card.id) failed: \(error.localizedDescription)")
        }
        await state
    }
}
