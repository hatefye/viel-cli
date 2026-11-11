import SwiftUI

enum ReleaseNotes {
    static let current: [(String, String, String)] = [
        ("book", "Manga", "A new Manga tab with search, a long-strip reader and a page-by-page mode you can pinch to zoom. Your place carries over between the app and the site."),
        ("person.2", "Profiles", "Up to five people on one account, each with their own list, history and manga. Switch under Settings → Profile."),
        ("checklist", "Every source gets a chance", "KotoTV and other slower sources no longer drop off the source list when a faster one answers first."),
        ("pip.fill", "Picture in Picture grew up", "Subtitles now show in the floating window, intros and credits skip themselves, and the next episode starts on its own without leaving PiP."),
        ("forward.end", "Next episode from anywhere", "The skip button on the lock screen, Control Center and your headphones jumps to the next episode."),
        ("play.rectangle.on.rectangle", "Two more anime sources", "KickAssAnime and ReAnime, both 1080p with sub and dub. KickAssAnime often carries extra dubs like Spanish, German and Hindi."),
        ("repeat", "Loop a section", "Set a start and an end from the player settings to replay a scene or an opening."),
        ("arrow.down.circle.dotted", "Pick a download quality", "Long-press an episode for Download in… 480p to 4K."),
        ("antenna.radiowaves.left.and.right.slash", "Cellular cap and offline mode", "Limit quality on cellular under Playback. Offline, the home page points you to your downloads."),
        ("sparkles", "Ambient glow", "A soft blur of the episode art fills the bars around the video. Off by default under Playback."),
        ("arrow.counterclockwise", "Pick up after a crash", "If viel closes while something is playing, it offers to resume on the next launch."),
        ("faceid", "Face ID lock and private mode", "Both under Settings → Privacy. Private mode keeps what you watch out of history."),
        ("chart.bar", "Your stats", "Hours watched, streaks and most watched, from the chart button in My List. Shareable."),
        ("mic", "Siri and Spotlight", "Say \"Resume on viel\", or find titles from My List in iPhone search."),
        ("speaker.wave.2", "Background audio", "Turn it on under Playback and the sound keeps going with the screen off, with lock screen controls."),
        ("film.stack", "Collections", "Movies that belong to a saga show the whole run on their page, in order."),
        ("arrow.down.circle", "Downloads are MP4 files", "Episodes and movies save as a single video file you can open from Files or share."),
        ("bell.badge", "New episode alerts", "Turn them on in Settings and get a notification when a show in My List airs. Upcoming episodes sit at the top of My List."),
        ("square.stack.3d.down.right", "Next episode ahead", "Optionally download the following episode as soon as one starts playing."),
        ("hand.tap", "Two-finger tap", "Toggles subtitles while watching. Pinch to fill the screen is remembered per show."),
        ("captions.bubble", "Subtitle timing", "Nudge subtitles earlier or later from the subtitles sheet."),
        ("moon.zzz", "Softer sleep timer", "The volume fades out over the last twenty seconds."),
        ("shuffle", "Random episode", "In the episode list menu on any show."),
        ("sparkles", "KotoTV is back", "Fast 1080p anime with subs and dubs, first in line again."),
    ]
}

struct WhatsNewView: View {
    let build: String

    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            ZStack {
                Theme.bg.ignoresSafeArea()
                ScrollView(showsIndicators: false) {
                    VStack(alignment: .leading, spacing: 18) {
                        ForEach(ReleaseNotes.current, id: \.1) { note in
                            HStack(alignment: .top, spacing: 14) {
                                Image(systemName: note.0)
                                    .font(.system(size: 22))
                                    .foregroundStyle(Theme.accentHi)
                                    .frame(width: 34)
                                VStack(alignment: .leading, spacing: 3) {
                                    Text(note.1).font(.system(size: 17, weight: .semibold))
                                    Text(note.2).font(.system(size: 15)).foregroundStyle(Theme.soft)
                                }
                            }
                        }
                    }
                    .padding(20)
                }
            }
            .navigationTitle("What's new in \(build)")
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(Theme.bg, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }.foregroundStyle(.white)
                }
            }
        }
        .preferredColorScheme(.dark)
    }
}
