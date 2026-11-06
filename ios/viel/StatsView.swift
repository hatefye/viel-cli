import SwiftUI

struct WatchStats {
    let hours: Double
    let finished: Int
    let shows: Int
    let top: [(String, Int)]
    let streak: Int
    let busiest: String

    init(rows: [HistoryRow]) {
        let calendar = Calendar.current
        hours = rows.reduce(0) { $0 + min($1.position, $1.duration > 0 ? $1.duration : $1.position) } / 3600
        finished = rows.filter(\.finished).count
        var perShow: [String: (String, Int)] = [:]
        var days: Set<Date> = []
        var weekdays: [Int: Int] = [:]
        for row in rows {
            let key = "\(row.kind)-\(row.id)"
            perShow[key] = (row.show, (perShow[key]?.1 ?? 0) + 1)
            guard row.updatedAt > 0 else { continue }
            let date = Date(timeIntervalSince1970: Double(row.updatedAt) / 1000)
            days.insert(calendar.startOfDay(for: date))
            weekdays[calendar.component(.weekday, from: date), default: 0] += 1
        }
        shows = perShow.count
        top = perShow.values.sorted { $0.1 > $1.1 }.prefix(3).map { ($0.0, $0.1) }
        let sorted = days.sorted()
        var best = sorted.isEmpty ? 0 : 1
        var run = best
        for (previous, next) in zip(sorted, sorted.dropFirst()) {
            if calendar.dateComponents([.day], from: previous, to: next).day == 1 {
                run += 1
                best = max(best, run)
            } else {
                run = 1
            }
        }
        streak = best
        if let day = weekdays.max(by: { $0.value < $1.value })?.key {
            busiest = calendar.weekdaySymbols[day - 1]
        } else {
            busiest = "—"
        }
    }
}

struct StatsView: View {
    @EnvironmentObject private var core: Core
    @Environment(\.dismiss) private var dismiss
    @State private var stats: WatchStats?
    @State private var card: Image?

    var body: some View {
        NavigationStack {
            ZStack {
                Theme.bg.ignoresSafeArea()
                if let stats {
                    ScrollView(showsIndicators: false) {
                        StatsCard(stats: stats, name: core.username ?? "you")
                            .padding(20)
                    }
                } else {
                    ProgressView().tint(.white)
                }
            }
            .navigationTitle("Your stats")
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(Theme.bg, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") { dismiss() }.foregroundStyle(.white)
                }
                ToolbarItem(placement: .confirmationAction) {
                    if let card {
                        ShareLink(item: card, preview: SharePreview("My viel stats", image: card)) {
                            Image(systemName: "square.and.arrow.up")
                        }
                        .foregroundStyle(.white)
                    }
                }
            }
        }
        .preferredColorScheme(.dark)
        .task {
            let rows = await core.history()
            let computed = WatchStats(rows: rows)
            stats = computed
            let renderer = ImageRenderer(content: StatsCard(stats: computed, name: core.username ?? "you").frame(width: 360).padding(20).background(Theme.bg))
            renderer.scale = 3
            if let image = renderer.uiImage {
                card = Image(uiImage: image)
            }
        }
    }
}

struct StatsCard: View {
    let stats: WatchStats
    let name: String

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            Text("viel · \(name)").font(.system(size: 13, weight: .semibold)).foregroundStyle(Theme.muted)
            Text(stats.hours >= 1 ? String(format: "%.0f hours watched", stats.hours) : String(format: "%.0f minutes watched", stats.hours * 60))
                .font(.system(size: 30, weight: .bold))
            HStack(spacing: 10) {
                tile("\(stats.finished)", "episodes finished")
                tile("\(stats.shows)", "titles")
                tile("\(stats.streak)", stats.streak == 1 ? "day streak" : "day streak")
            }
            if !stats.top.isEmpty {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Most watched").font(.system(size: 14, weight: .semibold)).foregroundStyle(Theme.muted)
                    ForEach(Array(stats.top.enumerated()), id: \.offset) { index, entry in
                        HStack {
                            Text("\(index + 1)").font(.system(size: 15, weight: .bold)).foregroundStyle(Theme.accentHi).frame(width: 18)
                            Text(entry.0).font(.system(size: 16, weight: .medium)).lineLimit(1)
                            Spacer()
                            Text(entry.1 == 1 ? "1 episode" : "\(entry.1) episodes").font(.system(size: 13)).foregroundStyle(Theme.muted)
                        }
                    }
                }
            }
            Text("Busiest day: \(stats.busiest)").font(.system(size: 14)).foregroundStyle(Theme.soft)
        }
        .foregroundStyle(.white)
        .padding(22)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white.opacity(0.05), in: RoundedRectangle(cornerRadius: 22, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 22, style: .continuous).stroke(Theme.border, lineWidth: 1))
    }

    private func tile(_ value: String, _ label: String) -> some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(value).font(.system(size: 24, weight: .bold))
            Text(label).font(.system(size: 12)).foregroundStyle(Theme.muted)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(12)
        .background(Color.white.opacity(0.05), in: RoundedRectangle(cornerRadius: 14, style: .continuous))
    }
}
