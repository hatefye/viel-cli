import AVKit
import Combine
import SwiftUI

struct PlayerView: View {
    @State private var current: Playable
    @EnvironmentObject private var core: Core
    @Environment(\.dismiss) private var dismiss
    @StateObject private var model = PlayerModel()
    @State private var nextUp: Playable?
    @State private var countdown = 0

    init(item: Playable) {
        _current = State(initialValue: item)
    }

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()
            VideoHost(player: model.player).ignoresSafeArea()
            overlay
        }
        .statusBarHidden(true)
        .persistentSystemOverlays(.hidden)
        .task { await model.start(current, core: core) }
        .onDisappear { model.stop() }
        .onChange(of: model.finished) { done in
            guard done else { return }
            if core.settings.autoplay, let next = current.next {
                nextUp = next
                countdown = 5
            } else {
                dismiss()
            }
        }
        .task(id: countdown) {
            guard nextUp != nil, countdown > 0 else { return }
            try? await Task.sleep(nanoseconds: 1_000_000_000)
            guard !Task.isCancelled, let next = nextUp else { return }
            if countdown > 1 {
                countdown -= 1
            } else {
                nextUp = nil
                countdown = 0
                current = next
                await model.start(next, core: core)
            }
        }
    }

    private var overlay: some View {
        VStack {
            HStack(alignment: .top) {
                Button {
                    dismiss()
                } label: {
                    Image(systemName: "xmark.circle.fill")
                        .font(.title)
                        .foregroundStyle(.white.opacity(0.85))
                        .shadow(radius: 4)
                }
                Spacer()
                if model.status != nil {
                    Text(current.heading)
                        .font(.subheadline.weight(.semibold))
                        .foregroundStyle(.white.opacity(0.9))
                        .shadow(radius: 4)
                        .lineLimit(1)
                }
                Spacer()
                menu
            }
            .padding(.horizontal, 16)
            .padding(.top, 12)

            Spacer()

            if let status = model.status {
                VStack(spacing: 10) {
                    if !model.failed { ProgressView().tint(.white) }
                    Text(status)
                        .font(.footnote)
                        .foregroundStyle(.white.opacity(0.9))
                        .multilineTextAlignment(.center)
                    if model.failed {
                        Button("Try again") { Task { await model.start(current, core: core) } }
                            .buttonStyle(.borderedProminent)
                    }
                }
                .padding(16)
                .background(.black.opacity(0.6), in: RoundedRectangle(cornerRadius: 12))
                .padding(.horizontal, 24)
            }

            Spacer()

            if let next = nextUp {
                HStack(spacing: 12) {
                    Text("Next in \(countdown)s · \(next.episode?.label(anime: next.isAnime) ?? "")")
                        .font(.subheadline)
                        .lineLimit(1)
                    Button("Cancel") {
                        nextUp = nil
                        countdown = 0
                        dismiss()
                    }
                    .buttonStyle(.bordered)
                }
                .padding(12)
                .background(.black.opacity(0.7), in: RoundedRectangle(cornerRadius: 10))
                .padding(.bottom, 90)
            }

            if let cue = model.cue, nextUp == nil {
                Text(cue)
                    .font(.system(size: 20, weight: .medium))
                    .multilineTextAlignment(.center)
                    .foregroundStyle(.white)
                    .shadow(color: .black, radius: 2)
                    .shadow(color: .black, radius: 2)
                    .padding(.horizontal, 24)
                    .padding(.bottom, 90)
                    .allowsHitTesting(false)
            }

            if model.showSkip {
                HStack {
                    Spacer()
                    Button("Skip intro") { model.skipIntro() }
                        .buttonStyle(.borderedProminent)
                        .padding(.trailing, 20)
                        .padding(.bottom, 100)
                }
            }
        }
    }

    private var menu: some View {
        Menu {
            if let stream = model.stream {
                Picker("Subtitles", selection: Binding(
                    get: { model.subIndex ?? -1 },
                    set: { model.selectSubtitle($0 < 0 ? nil : $0) }
                )) {
                    Text("Off").tag(-1)
                    ForEach(stream.subs.indices, id: \.self) { index in
                        Text(stream.subs[index].lang.isEmpty ? "Subtitle \(index + 1)" : stream.subs[index].lang).tag(index)
                    }
                }
            }
            if model.streams.count > 1 {
                Picker("Source", selection: Binding(
                    get: { model.streamIndex },
                    set: { model.switchSource($0) }
                )) {
                    ForEach(model.streams.indices, id: \.self) { index in
                        Text(model.streams[index].source).tag(index)
                    }
                }
            }
        } label: {
            Image(systemName: "ellipsis.circle.fill")
                .font(.title)
                .foregroundStyle(.white.opacity(0.85))
                .shadow(radius: 4)
        }
    }
}

struct VideoHost: UIViewControllerRepresentable {
    let player: AVPlayer

    func makeUIViewController(context: Context) -> AVPlayerViewController {
        let controller = AVPlayerViewController()
        controller.player = player
        controller.allowsPictureInPicturePlayback = true
        controller.canStartPictureInPictureAutomaticallyFromInline = true
        controller.updatesNowPlayingInfoCenter = true
        controller.videoGravity = .resizeAspect
        return controller
    }

    func updateUIViewController(_ controller: AVPlayerViewController, context: Context) {
        if controller.player !== player {
            controller.player = player
        }
    }
}

final class PlayerModel: ObservableObject {
    let player = AVPlayer()

    @Published var status: String? = "finding sources…"
    @Published var failed = false
    @Published var cue: String?
    @Published var showSkip = false
    @Published var finished = false
    @Published var streams: [Stream] = []
    @Published var streamIndex = 0
    @Published var subIndex: Int?

    var stream: Stream? { streams.indices.contains(streamIndex) ? streams[streamIndex] : nil }

    private var item: Playable?
    private var core: Core?
    private var cues: [Cue] = []
    private var timeToken: Any?
    private var observers: [NSObjectProtocol] = []
    private var statusWatch: AnyCancellable?
    private var watchdog: Task<Void, Never>?
    private var lastReport = Date.distantPast
    private var introSkipped = false
    private var startAt: Double?
    private var generation = 0

    func start(_ item: Playable, core: Core) async {
        self.item = item
        self.core = core
        generation += 1
        let mine = generation
        await MainActor.run {
            finished = false
            failed = false
            cue = nil
            showSkip = false
            streams = []
            status = "finding sources…"
        }
        cues = []
        introSkipped = false
        startAt = item.startAt
        do {
            let reply: PlayResponse = try await core.post("play", item.request)
            guard mine == generation else { return }
            await MainActor.run {
                streams = reply.streams
                if reply.streams.isEmpty {
                    status = "no source has this one right now"
                    failed = true
                } else {
                    attach(0)
                }
            }
        } catch {
            guard mine == generation else { return }
            await MainActor.run {
                status = error.localizedDescription
                failed = true
            }
        }
    }

    func stop() {
        let position = player.currentTime().seconds
        if position.isFinite, position > 0 {
            report(position)
        }
        generation += 1
        cleanup()
        if let token = timeToken {
            player.removeTimeObserver(token)
            timeToken = nil
        }
        player.replaceCurrentItem(with: nil)
    }

    func switchSource(_ index: Int) {
        guard index != streamIndex, streams.indices.contains(index) else { return }
        let position = player.currentTime().seconds
        startAt = position.isFinite && position > 1 ? position : nil
        attach(index)
    }

    func selectSubtitle(_ index: Int?) {
        subIndex = index
        cues = []
        cue = nil
        guard let index, let stream, stream.subs.indices.contains(index), let url = URL(string: stream.subs[index].url) else { return }
        let mine = generation
        Task {
            guard let reply = try? await URLSession.shared.data(from: url) else { return }
            let parsed = Subtitles.parse(String(decoding: reply.0, as: UTF8.self))
            await MainActor.run {
                if mine == self.generation, self.subIndex == index {
                    self.cues = parsed
                }
            }
        }
    }

    func skipIntro() {
        guard let intro = stream?.intro, intro.count == 2 else { return }
        player.seek(to: CMTime(seconds: intro[1], preferredTimescale: 600))
        introSkipped = true
        showSkip = false
    }

    private func attach(_ index: Int) {
        guard streams.indices.contains(index) else { return }
        cleanup()
        streamIndex = index
        let stream = streams[index]
        status = "loading from \(stream.source)…"
        failed = false
        guard let url = URL(string: stream.url) else {
            failover("\(stream.source) gave a bad link")
            return
        }
        let avItem = AVPlayerItem(url: url)
        avItem.preferredForwardBufferDuration = 20
        player.automaticallyWaitsToMinimizeStalling = true
        player.replaceCurrentItem(with: avItem)

        statusWatch = avItem.publisher(for: \.status)
            .receive(on: DispatchQueue.main)
            .sink { [weak self] state in
                guard let self, self.player.currentItem === avItem else { return }
                switch state {
                case .readyToPlay:
                    self.status = nil
                    if let at = self.startAt, at > 0 {
                        self.player.seek(to: CMTime(seconds: at, preferredTimescale: 600))
                        self.startAt = nil
                    }
                    self.player.play()
                case .failed:
                    self.failover("\(stream.source) failed")
                default:
                    break
                }
            }

        let center = NotificationCenter.default
        observers.append(center.addObserver(forName: .AVPlayerItemFailedToPlayToEndTime, object: avItem, queue: .main) { [weak self] _ in
            self?.failover("\(stream.source) dropped")
        })
        observers.append(center.addObserver(forName: .AVPlayerItemDidPlayToEndTime, object: avItem, queue: .main) { [weak self] _ in
            guard let self else { return }
            let duration = avItem.duration.seconds
            if duration.isFinite, duration > 0 { self.report(duration) }
            self.finished = true
        })

        watchdog = Task { [weak self] in
            try? await Task.sleep(nanoseconds: 35_000_000_000)
            guard !Task.isCancelled, let self else { return }
            await MainActor.run {
                if self.player.currentItem === avItem, self.status != nil, !self.failed {
                    self.failover("\(stream.source) took too long")
                }
            }
        }

        selectSubtitle(stream.subs.isEmpty ? nil : 0)
        installTimeObserver()
    }

    private func failover(_ reason: String) {
        let next = streamIndex + 1
        if streams.indices.contains(next) {
            status = "\(reason) · trying \(streams[next].source)"
            attach(next)
        } else {
            cleanup()
            player.replaceCurrentItem(with: nil)
            status = "every source failed · \(reason)"
            failed = true
        }
    }

    private func cleanup() {
        statusWatch?.cancel()
        statusWatch = nil
        watchdog?.cancel()
        watchdog = nil
        observers.forEach { NotificationCenter.default.removeObserver($0) }
        observers = []
    }

    private func installTimeObserver() {
        guard timeToken == nil else { return }
        timeToken = player.addPeriodicTimeObserver(forInterval: CMTime(seconds: 0.5, preferredTimescale: 600), queue: .main) { [weak self] time in
            self?.tick(time.seconds)
        }
    }

    private func tick(_ position: Double) {
        guard position.isFinite else { return }
        let text = Subtitles.cue(in: cues, at: position)
        if text != cue { cue = text }
        if let intro = stream?.intro, intro.count == 2, !introSkipped {
            let visible = position >= intro[0] && position < intro[1] - 1
            if visible != showSkip { showSkip = visible }
            if position >= intro[1] { introSkipped = true }
        }
        if Date().timeIntervalSince(lastReport) >= 10 {
            report(position)
        }
    }

    private func report(_ position: Double) {
        guard let item, let core else { return }
        let duration = player.currentItem?.duration.seconds ?? 0
        guard duration.isFinite, duration > 0 else { return }
        lastReport = Date()
        let progress = item.progress(position: position, duration: duration)
        Task { await core.saveProgress(progress) }
    }
}
