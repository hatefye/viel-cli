import BackgroundTasks
import SwiftUI
import UserNotifications

struct Upcoming: Decodable, Identifiable, Hashable {
    let kind: String
    let showId: UInt64
    let name: String
    let poster: String?
    let season: Int
    let episode: Int
    let title: String
    let airsAt: Double

    enum CodingKeys: String, CodingKey {
        case kind, name, poster, season, episode, title
        case showId = "id"
        case airsAt = "airs_at"
    }

    var id: String { "\(kind)-\(showId)-\(season)-\(episode)" }
    var date: Date { Date(timeIntervalSince1970: airsAt) }
    var aired: Bool { date <= Date() }
    var episodeLabel: String { kind == "tv" ? "S\(season) · E\(episode)" : "Episode \(episode)" }

    var link: URL? {
        URL(string: kind == "tv" ? "viel://tv/\(showId)/\(season)/\(episode)" : "viel://anime/\(showId)/\(episode)")
    }

    var when: String {
        let calendar = Calendar.current
        let day: String
        if calendar.isDateInToday(date) {
            day = "Today"
        } else if calendar.isDateInTomorrow(date) {
            day = "Tomorrow"
        } else if calendar.isDateInYesterday(date) {
            day = "Yesterday"
        } else if abs(date.timeIntervalSinceNow) < 6 * 86_400 {
            day = date.formatted(.dateTime.weekday(.wide))
        } else {
            day = date.formatted(.dateTime.month(.abbreviated).day())
        }
        if kind == "anime" {
            return "\(day) · \(date.formatted(date: .omitted, time: .shortened))"
        }
        return aired ? "Out \(day.lowercased())" : day
    }
}

enum Alerts {
    static let task = "tv.viel.refresh"

    static func register() {
        BGTaskScheduler.shared.register(forTaskWithIdentifier: task, using: nil) { task in
            guard let task = task as? BGAppRefreshTask else { return }
            schedule()
            let job = Task { @MainActor in
                await refresh(core: Core.shared)
                task.setTaskCompleted(success: true)
            }
            task.expirationHandler = { job.cancel() }
        }
    }

    static func schedule() {
        let request = BGAppRefreshTaskRequest(identifier: task)
        request.earliestBeginDate = Date(timeIntervalSinceNow: 4 * 3600)
        try? BGTaskScheduler.shared.submit(request)
    }

    static func authorize() async -> Bool {
        (try? await UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound, .badge])) ?? false
    }

    @MainActor
    static func fetch(core: Core) async -> [Upcoming] {
        guard core.username != nil else { return [] }
        return (try? await core.get("calendar")) ?? []
    }

    @MainActor
    static func refresh(core: Core) async {
        guard core.settings.episodeAlerts else { return }
        let items = await fetch(core: core)
        let center = UNUserNotificationCenter.current()
        let stale = await center.pendingNotificationRequests().map(\.identifier).filter { $0.hasPrefix("ep-") }
        center.removePendingNotificationRequests(withIdentifiers: stale)
        for item in items.filter({ !$0.aired }).prefix(40) {
            let content = UNMutableNotificationContent()
            content.title = item.name
            content.body = item.title.isEmpty ? "\(item.episodeLabel) is out now" : "\(item.episodeLabel) · \(item.title) is out now"
            content.sound = .default
            if let link = item.link {
                content.userInfo = ["url": link.absoluteString]
            }
            let parts = Calendar.current.dateComponents([.year, .month, .day, .hour, .minute], from: item.date)
            let trigger = UNCalendarNotificationTrigger(dateMatching: parts, repeats: false)
            try? await center.add(UNNotificationRequest(identifier: "ep-" + item.id, content: content, trigger: trigger))
        }
        core.log("alerts", "scheduled \(min(items.filter { !$0.aired }.count, 40)) episode alerts")
    }
}

struct UpcomingCard: View {
    let item: Upcoming

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            ZStack(alignment: .bottomLeading) {
                Poster(url: item.poster, width: 120, height: 180, radius: 10)
                Text(item.episodeLabel)
                    .font(.system(size: 12, weight: .bold))
                    .foregroundStyle(.white)
                    .padding(.horizontal, 7)
                    .padding(.vertical, 4)
                    .background(item.aired ? Theme.accent : Color.black.opacity(0.65), in: RoundedRectangle(cornerRadius: 6, style: .continuous))
                    .padding(8)
            }
            Text(item.name).font(.system(size: 13, weight: .medium)).lineLimit(1)
            Text(item.when).font(.system(size: 12)).foregroundStyle(item.aired ? Theme.accentHi : Theme.muted).lineLimit(1)
        }
        .frame(width: 120)
        .contentShape(Rectangle())
    }
}
