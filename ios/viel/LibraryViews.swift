import SwiftUI

struct DownloadsSection: View {
    let downloads: [Download]
    let onPlay: (Playable) -> Void
    let onRemove: (String) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline) {
                Text("Downloads").font(.system(size: 22, weight: .bold))
                Spacer()
                Text(sizeLine).font(.system(size: 14)).foregroundStyle(Theme.muted)
            }
            .padding(.horizontal, 16)
            VStack(spacing: 2) {
                ForEach(downloads) { item in
                    DownloadRow(item: item)
                        .contentShape(Rectangle())
                        .onTapGesture {
                            guard item.isDone, let playable = item.playable else { return }
                            Haptic.tap()
                            onPlay(playable)
                        }
                        .contextMenu {
                            if item.isDone, let playable = item.playable {
                                Button { onPlay(playable) } label: { Label("Play", systemImage: "play.fill") }
                            }
                            Button(role: .destructive) {
                                onRemove(item.key)
                            } label: {
                                Label(item.isActive ? "Cancel download" : "Delete download", systemImage: "trash")
                            }
                        }
                }
            }
        }
    }

    private var sizeLine: String {
        let total = downloads.filter(\.isDone).reduce(UInt64(0)) { $0 + $1.bytes }
        guard total > 0 else { return "" }
        return total >= 1_073_741_824 ? String(format: "%.1f GB on device", Double(total) / 1_073_741_824) : "\(total / 1_048_576) MB on device"
    }
}

struct DownloadRow: View {
    let item: Download

    var body: some View {
        HStack(spacing: 14) {
            ZStack(alignment: .bottomLeading) {
                Poster(url: item.meta?.still ?? item.meta?.poster, width: 110, height: 64, radius: 8)
                if item.isActive {
                    GeometryReader { geometry in
                        ZStack(alignment: .leading) {
                            Rectangle().fill(Color.white.opacity(0.25))
                            Rectangle().fill(Theme.accent).frame(width: geometry.size.width * item.fraction)
                        }
                    }
                    .frame(height: 3)
                    .padding(.horizontal, 6)
                    .padding(.bottom, 6)
                }
            }
            VStack(alignment: .leading, spacing: 3) {
                Text(item.meta?.name ?? item.key).font(.system(size: 16, weight: .semibold)).lineLimit(1)
                if let label = item.meta?.label {
                    Text(label).font(.system(size: 14)).foregroundStyle(Theme.soft).lineLimit(1)
                }
                HStack(spacing: 6) {
                    if item.isActive {
                        ProgressView().tint(.white).scaleEffect(0.6).frame(width: 12, height: 12)
                    } else if item.isDone {
                        Image(systemName: "checkmark.circle.fill").font(.system(size: 12)).foregroundStyle(Theme.match)
                    } else {
                        Image(systemName: "exclamationmark.circle.fill").font(.system(size: 12)).foregroundStyle(Theme.accentHi)
                    }
                    Text(item.statusText).font(.system(size: 13)).foregroundStyle(Theme.muted).lineLimit(1)
                }
            }
            Spacer(minLength: 0)
            if item.isDone {
                Image(systemName: "play.fill").font(.system(size: 16)).foregroundStyle(Theme.muted)
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }
}

struct HistoryView: View {
    @EnvironmentObject private var core: Core
    @State private var rows: [HistoryRow] = []
    @State private var query = ""
    @State private var loading = true
    @State private var selected: Resume?

    private var visible: [HistoryRow] {
        let needle = query.trimmingCharacters(in: .whitespaces).lowercased()
        guard !needle.isEmpty else { return rows }
        return rows.filter { $0.show.lowercased().contains(needle) || ($0.episodeLabel ?? "").lowercased().contains(needle) }
    }

    var body: some View {
        ZStack {
            Theme.bg.ignoresSafeArea()
            if loading && rows.isEmpty {
                ProgressView().tint(.white)
            } else if rows.isEmpty {
                VStack(spacing: 8) {
                    Image(systemName: "clock.arrow.circlepath").font(.system(size: 34)).foregroundStyle(Theme.faint)
                    Text(core.username == nil ? "Sign in to keep a watch history." : "Nothing watched yet.").foregroundStyle(Theme.muted)
                }
            } else {
                ScrollView(showsIndicators: false) {
                    LazyVStack(spacing: 2) {
                        ForEach(visible) { row in
                            HistoryRowView(row: row)
                                .contentShape(Rectangle())
                                .onTapGesture {
                                    Haptic.tap()
                                    selected = row.resume
                                }
                                .contextMenu {
                                    if !row.finished {
                                        Button {
                                            Task { await markWatched(row) }
                                        } label: {
                                            Label("Mark as watched", systemImage: "checkmark.circle")
                                        }
                                    }
                                    Button(role: .destructive) {
                                        Task { await remove(row) }
                                    } label: {
                                        Label("Remove from history", systemImage: "trash")
                                    }
                                }
                        }
                    }
                    .padding(.vertical, 8)
                }
                .refreshable { await load() }
            }
        }
        .navigationTitle("History")
        .navigationBarTitleDisplayMode(.large)
        .toolbarBackground(Theme.bg, for: .navigationBar)
        .searchable(text: $query, placement: .navigationBarDrawer(displayMode: .always), prompt: "Search history")
        .task { await load() }
        .fullScreenCover(item: $selected) { DetailsView(title: $0.title, resume: $0).environmentObject(core) }
    }

    private func load() async {
        rows = await core.history()
        loading = false
    }

    private func markWatched(_ row: HistoryRow) async {
        let length = row.duration > 0 ? row.duration : 1
        await core.saveProgress(Progress(kind: row.kind, id: row.id, season: row.season, episode: row.episode, position: length, duration: length, title: row.show))
        await load()
    }

    private func remove(_ row: HistoryRow) async {
        rows.removeAll { $0.kind == row.kind && $0.id == row.id }
        await core.removeHistory(kind: row.kind, id: row.id)
    }
}

struct HistoryRowView: View {
    let row: HistoryRow

    var body: some View {
        HStack(spacing: 14) {
            Poster(url: row.poster, width: 54, height: 81, radius: 8)
            VStack(alignment: .leading, spacing: 3) {
                Text(row.show).font(.system(size: 16, weight: .semibold)).lineLimit(1)
                if let label = row.episodeLabel {
                    Text(label).font(.system(size: 14)).foregroundStyle(Theme.soft)
                }
                HStack(spacing: 6) {
                    if row.finished {
                        Image(systemName: "checkmark.circle.fill").font(.system(size: 12)).foregroundStyle(Theme.match)
                        Text("Watched").font(.system(size: 13)).foregroundStyle(Theme.muted)
                    } else {
                        Text("\(Int(row.fraction * 100))% · \(Format.clock(max(row.duration - row.position, 0))) left").font(.system(size: 13)).foregroundStyle(Theme.muted)
                    }
                    if !row.when.isEmpty {
                        Text("· \(row.when)").font(.system(size: 13)).foregroundStyle(Theme.faint)
                    }
                }
                GeometryReader { geometry in
                    ZStack(alignment: .leading) {
                        Capsule().fill(Color.white.opacity(0.15))
                        Capsule().fill(row.finished ? Theme.match : Theme.accent).frame(width: geometry.size.width * (row.finished ? 1 : row.fraction))
                    }
                }
                .frame(height: 3)
                .padding(.top, 2)
            }
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }
}
