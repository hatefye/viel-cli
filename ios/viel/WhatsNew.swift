import SwiftUI

enum ReleaseNotes {
    static let current: [(String, String, String)] = [
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
