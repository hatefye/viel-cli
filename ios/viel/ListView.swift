import SwiftUI

struct ListView: View {
    let page: ListPage

    @EnvironmentObject private var core: Core
    @Environment(\.dismiss) private var dismiss
    @State private var items: [Title] = []
    @State private var pageNumber = 1
    @State private var loading = false
    @State private var exhausted = false
    @State private var selected: Title?

    private let columns = [GridItem(.flexible(), spacing: 10), GridItem(.flexible(), spacing: 10), GridItem(.flexible(), spacing: 10)]

    var body: some View {
        ZStack {
            Theme.bg.ignoresSafeArea()
            ScrollView(showsIndicators: false) {
                LazyVGrid(columns: columns, spacing: 16) {
                    ForEach(items, id: \.key) { title in
                        VStack(alignment: .leading, spacing: 6) {
                            GeometryReader { geometry in
                                Poster(url: title.poster, width: geometry.size.width, height: geometry.size.width * 1.5, radius: 10)
                            }
                            .aspectRatio(2 / 3, contentMode: .fit)
                            Text(title.name).font(.system(size: 13, weight: .medium)).lineLimit(1)
                            Text(title.year.isEmpty ? title.kindLabel : title.year).font(.system(size: 12)).foregroundStyle(Theme.muted)
                        }
                        .contentShape(Rectangle())
                        .onTapGesture { selected = title }
                        .onAppear {
                            if title.key == items.last?.key { Task { await loadMore() } }
                        }
                    }
                }
                .padding(16)
                if loading {
                    ProgressView().tint(.white).padding()
                }
            }
        }
        .navigationTitle(page.title)
        .navigationBarTitleDisplayMode(.inline)
        .toolbarBackground(Theme.bg, for: .navigationBar)
        .toolbar {
            if page.path == nil {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button { dismiss() } label: { Image(systemName: "xmark") }
                }
            }
        }
        .task {
            if page.path == nil {
                items = page.items
                exhausted = true
            } else {
                await loadMore()
            }
        }
        .fullScreenCover(item: $selected) { DetailsView(title: $0, resume: nil).environmentObject(core) }
    }

    private func loadMore() async {
        guard let path = page.path, !loading, !exhausted else { return }
        loading = true
        defer { loading = false }
        let found: [Title] = (try? await core.get("list", ["path": path, "page": String(pageNumber)])) ?? []
        if found.isEmpty {
            exhausted = true
        } else {
            let known = Set(items.map(\.key))
            items.append(contentsOf: found.filter { !known.contains($0.key) })
            pageNumber += 1
            if pageNumber > 5 { exhausted = true }
        }
    }
}
