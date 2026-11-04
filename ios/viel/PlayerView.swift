import AVFoundation
import AVKit
import Combine
import SwiftUI

enum PlayerSheet: String, Identifiable {
    case episodes, settings, subtitles
    var id: String { rawValue }
}

struct PlayerView: View {
    @State private var current: Playable
    @EnvironmentObject private var core: Core
    @Environment(\.dismiss) private var dismiss
    @StateObject private var model = PlayerModel()
    @State private var controls = true
    @State private var hideTask: Task<Void, Never>?
    @State private var scrubbing = false
    @State private var sheet: PlayerSheet?
    @State private var nextUp: Playable?
    @State private var countdown = 0
    @State private var flash: String?

    init(item: Playable) {
        _current = State(initialValue: item)
    }

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()
            PlayerSurface(model: model).ignoresSafeArea()
            gestures
            if controls {
                shade
                pauseMeta
                controlLayer
            }
            subtitleLayer
            if model.status != nil { statusLayer }
            skipLayer
            nextLayer
            if let flash {
                Text(flash)
                    .font(.system(size: 20, weight: .semibold))
                    .foregroundStyle(.white)
                    .padding(.horizontal, 16)
                    .padding(.vertical, 10)
                    .background(Color.black.opacity(0.5), in: Capsule())
                    .transition(.opacity)
            }
        }
        .statusBarHidden(true)
        .persistentSystemOverlays(.hidden)
        .animation(.easeInOut(duration: 0.22), value: controls)
        .animation(.easeInOut(duration: 0.2), value: flash)
        .task { await start(current) }
        .onDisappear { model.stop() }
        .onChange(of: model.isPlaying) { playing in
            if playing { scheduleHide() } else { show() }
        }
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
                await start(next)
            }
        }
        .sheet(item: $sheet) { which in
            sheetContent(which).environmentObject(core)
        }
    }

    private func start(_ item: Playable) async {
        current = item
        show()
        await model.start(item, core: core)
    }

    private func show() {
        hideTask?.cancel()
        controls = true
    }

    private func scheduleHide() {
        hideTask?.cancel()
        hideTask = Task {
            try? await Task.sleep(nanoseconds: 3_500_000_000)
            guard !Task.isCancelled, model.isPlaying, !scrubbing, sheet == nil else { return }
            controls = false
        }
    }

    private func poke() {
        controls = true
        if model.isPlaying { scheduleHide() }
    }

    private func toggleControls() {
        if controls {
            if model.isPlaying { controls = false }
        } else {
            poke()
        }
    }

    private func skip(_ seconds: Double) {
        model.skip(seconds)
        flash = seconds < 0 ? "−10s" : "+10s"
        poke()
        Task {
            try? await Task.sleep(nanoseconds: 700_000_000)
            flash = nil
        }
    }

    private var gestures: some View {
        HStack(spacing: 0) {
            Color.clear.contentShape(Rectangle())
                .onTapGesture(count: 2) { skip(-10) }
                .onTapGesture { toggleControls() }
            Color.clear.contentShape(Rectangle())
                .onTapGesture(count: 2) { skip(10) }
                .onTapGesture { toggleControls() }
        }
        .ignoresSafeArea()
    }

    private var shade: some View {
        VStack(spacing: 0) {
            LinearGradient(colors: [.black.opacity(0.75), .clear], startPoint: .top, endPoint: .bottom).frame(height: 150)
            Spacer()
            LinearGradient(colors: [.clear, .black.opacity(0.85)], startPoint: .top, endPoint: .bottom).frame(height: 240)
        }
        .ignoresSafeArea()
        .allowsHitTesting(false)
    }

    private var controlLayer: some View {
        VStack(spacing: 0) {
            topBar
            Spacer()
            if model.status == nil { centerButtons }
            Spacer()
            bottomBar
        }
    }

    private var topBar: some View {
        HStack(spacing: 14) {
            Button { dismiss() } label: {
                Image(systemName: "arrow.left").font(.system(size: 24, weight: .semibold))
            }
            Text(current.title.name)
                .font(.system(size: 21, weight: .bold))
                .lineLimit(1)
            if let tag = current.episodeTag {
                Text(tag).font(.system(size: 18, weight: .semibold)).foregroundStyle(Theme.muted).lineLimit(1).layoutPriority(1)
                if let name = current.episode?.name, !name.isEmpty {
                    Text(name).font(.system(size: 18)).foregroundStyle(Theme.muted).lineLimit(1)
                }
            }
            Spacer()
        }
        .foregroundStyle(.white)
        .padding(.horizontal, 20)
        .padding(.top, 16)
    }

    private var centerButtons: some View {
        HStack(spacing: 64) {
            Button { skip(-10) } label: {
                Image(systemName: "gobackward.10").font(.system(size: 36, weight: .medium))
            }
            Button { model.togglePlay(); poke() } label: {
                ZStack {
                    Circle().fill(Color.white.opacity(0.16)).frame(width: 96, height: 96)
                    Image(systemName: model.isPlaying ? "pause.fill" : "play.fill").font(.system(size: 40, weight: .bold))
                }
            }
            Button { skip(10) } label: {
                Image(systemName: "goforward.10").font(.system(size: 36, weight: .medium))
            }
        }
        .foregroundStyle(.white)
    }

    private var bottomBar: some View {
        VStack(spacing: 14) {
            HStack(spacing: 14) {
                Text(Format.clock(model.position))
                    .font(.system(size: 21, weight: .regular, design: .rounded))
                    .monospacedDigit()
                Scrubber(position: model.position, duration: model.duration) { target in
                    model.seek(to: target)
                    poke()
                } scrubbing: { active in
                    scrubbing = active
                    if active { show() } else { scheduleHide() }
                }
                Text(Format.clock(model.duration))
                    .font(.system(size: 15, design: .rounded))
                    .monospacedDigit()
                    .foregroundStyle(Theme.muted)
            }
            .padding(.horizontal, 20)
            HStack {
                if !current.isMovie {
                    toolButton("play.square.stack", "Episodes") { sheet = .episodes }
                    Spacer()
                }
                if current.next != nil {
                    toolButton("forward.end.fill", "Next") { Task { await start(current.next!) } }
                    Spacer()
                }
                toolButton("captions.bubble.fill", "Subtitles") { sheet = .subtitles }
                Spacer()
                toolButton("gearshape.fill", "Settings") { sheet = .settings }
                Spacer()
                toolButton("pip.enter", "Picture in Picture") { model.togglePip() }
            }
            .padding(.horizontal, 28)
        }
        .foregroundStyle(.white)
        .padding(.bottom, 14)
    }

    private func toolButton(_ icon: String, _ label: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Image(systemName: icon).font(.system(size: 25, weight: .medium)).frame(width: 44, height: 44)
        }
        .accessibilityLabel(label)
    }

    @ViewBuilder
    private var pauseMeta: some View {
        if !model.isPlaying, model.status == nil {
            VStack(alignment: .leading, spacing: 10) {
                Logo(url: model.details?.logo, name: current.title.name, maxWidth: 240, size: 30)
                Text(metaLine).font(.system(size: 17)).foregroundStyle(Theme.muted)
                if let episode = current.episode {
                    Text(episode.displayName).font(.system(size: 26, weight: .bold)).lineLimit(2)
                }
                if let overview = current.episode.map(\.overview).flatMap({ $0.isEmpty ? nil : $0 }) ?? model.details?.overview {
                    Text(overview).font(.system(size: 16)).foregroundStyle(Theme.soft).lineLimit(4)
                }
            }
            .foregroundStyle(.white)
            .frame(maxWidth: 440, alignment: .leading)
            .padding(.horizontal, 24)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
            .padding(.top, 70)
            .padding(.bottom, 150)
            .allowsHitTesting(false)
        }
    }

    private var metaLine: String {
        var parts: [String] = []
        if let episode = current.episode {
            if !current.isAnime { parts.append("Season \(episode.season)") }
            parts.append("Episode \(episode.numberText)")
            if let runtime = Format.runtime(episode.runtime ?? model.details?.runtime) { parts.append(runtime) }
        } else {
            if !current.title.year.isEmpty { parts.append(current.title.year) }
            if let runtime = Format.runtime(model.details?.runtime) { parts.append(runtime) }
        }
        return parts.joined(separator: " · ")
    }

    private var subtitleLayer: some View {
        VStack {
            Spacer()
            if let cue = model.cue, model.subsEnabled {
                Text(cue)
                    .font(.system(size: 21, weight: .medium))
                    .multilineTextAlignment(.center)
                    .foregroundStyle(.white)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 5)
                    .background(Color.black.opacity(0.5), in: RoundedRectangle(cornerRadius: 6, style: .continuous))
                    .padding(.horizontal, 24)
                    .padding(.bottom, controls ? 150 : 36)
            }
        }
        .allowsHitTesting(false)
    }

    private var statusLayer: some View {
        VStack(spacing: 14) {
            if !model.failed {
                ZStack {
                    Circle().fill(Color.black.opacity(0.55)).frame(width: 72, height: 72)
                    ProgressView().tint(.white).scaleEffect(1.4)
                }
            }
            Text(model.status ?? "")
                .font(.system(size: 15))
                .foregroundStyle(Theme.soft)
                .multilineTextAlignment(.center)
            if let activity = model.activity, !model.failed {
                Text(activity).font(.system(size: 13)).foregroundStyle(Theme.muted)
            }
            if model.failed {
                Button("Try again") { Task { await start(current) } }.buttonStyle(WhitePill())
            }
        }
        .padding(.horizontal, 30)
        .allowsHitTesting(model.failed)
    }

    private var skipLayer: some View {
        VStack {
            Spacer()
            HStack {
                Spacer()
                if model.showSkip {
                    Button { model.skipIntro() } label: {
                        HStack(spacing: 8) {
                            Text("Skip Intro")
                            Image(systemName: "play.fill").font(.system(size: 13))
                        }
                    }
                    .buttonStyle(WhitePill())
                    .padding(.trailing, 20)
                    .padding(.bottom, controls ? 150 : 40)
                }
            }
        }
    }

    private var nextLayer: some View {
        VStack {
            Spacer()
            if let next = nextUp {
                HStack(spacing: 14) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Up next in \(countdown)s").font(.system(size: 13)).foregroundStyle(Theme.muted)
                        Text(next.episode?.label(anime: next.isAnime) ?? "").font(.system(size: 16, weight: .semibold)).lineLimit(1)
                    }
                    Button("Play now") {
                        nextUp = nil
                        countdown = 0
                        Task { await start(next) }
                    }
                    .buttonStyle(WhitePill())
                    Button("Cancel") {
                        nextUp = nil
                        countdown = 0
                        dismiss()
                    }
                    .buttonStyle(GlassPill())
                }
                .foregroundStyle(.white)
                .padding(14)
                .background(Theme.surface.opacity(0.95), in: RoundedRectangle(cornerRadius: 16, style: .continuous))
                .overlay(RoundedRectangle(cornerRadius: 16, style: .continuous).stroke(Theme.borderStrong, lineWidth: 1))
                .padding(.bottom, 40)
            }
        }
    }

    @ViewBuilder
    private func sheetContent(_ which: PlayerSheet) -> some View {
        switch which {
        case .episodes:
            EpisodesSheet(current: current) { item in
                sheet = nil
                Task { await start(item) }
            }
        case .settings:
            SettingsSheet(model: model) { sheet = .subtitles }
        case .subtitles:
            SubtitlesSheet(model: model)
        }
    }
}

struct Scrubber: View {
    let position: Double
    let duration: Double
    let onSeek: (Double) -> Void
    let scrubbing: (Bool) -> Void

    @State private var dragging = false
    @State private var dragFraction: Double = 0

    var body: some View {
        GeometryReader { geometry in
            let width = geometry.size.width
            let fraction = dragging ? dragFraction : (duration > 0 ? min(max(position / duration, 0), 1) : 0)
            ZStack(alignment: .leading) {
                Capsule().fill(Color.white.opacity(0.28)).frame(height: 4)
                Capsule().fill(Theme.accent).frame(width: max(width * fraction, 0), height: 4)
                Circle().fill(.white).frame(width: 14, height: 14).offset(x: max(width * fraction - 7, -7))
            }
            .frame(height: 30)
            .contentShape(Rectangle())
            .gesture(
                DragGesture(minimumDistance: 0)
                    .onChanged { value in
                        if !dragging { scrubbing(true) }
                        dragging = true
                        dragFraction = min(max(value.location.x / width, 0), 1)
                    }
                    .onEnded { value in
                        dragging = false
                        let fraction = min(max(value.location.x / width, 0), 1)
                        onSeek(fraction * duration)
                        scrubbing(false)
                    }
            )
        }
        .frame(height: 30)
    }
}

final class PlayerLayerView: UIView {
    override class var layerClass: AnyClass { AVPlayerLayer.self }
    var playerLayer: AVPlayerLayer { layer as! AVPlayerLayer }
}

struct PlayerSurface: UIViewRepresentable {
    let model: PlayerModel

    func makeUIView(context: Context) -> PlayerLayerView {
        let view = PlayerLayerView()
        view.backgroundColor = .black
        view.playerLayer.player = model.player
        view.playerLayer.videoGravity = .resizeAspect
        if AVPictureInPictureController.isPictureInPictureSupported(), let pip = AVPictureInPictureController(playerLayer: view.playerLayer) {
            pip.canStartPictureInPictureAutomaticallyFromInline = true
            pip.delegate = context.coordinator
            context.coordinator.pip = pip
            model.pip = pip
        }
        return view
    }

    func updateUIView(_ view: PlayerLayerView, context: Context) {}

    func makeCoordinator() -> Coordinator {
        Coordinator(model: model)
    }

    final class Coordinator: NSObject, AVPictureInPictureControllerDelegate {
        let model: PlayerModel
        var pip: AVPictureInPictureController?

        init(model: PlayerModel) {
            self.model = model
        }

        func pictureInPictureControllerDidStartPictureInPicture(_ controller: AVPictureInPictureController) {
            model.log("pip started")
        }

        func pictureInPictureControllerDidStopPictureInPicture(_ controller: AVPictureInPictureController) {
            model.log("pip stopped")
        }

        func pictureInPictureController(_ controller: AVPictureInPictureController, failedToStartPictureInPictureWithError error: Error) {
            model.log("pip failed: \(error.localizedDescription)")
        }
    }
}

final class PlayerModel: ObservableObject {
    let player = AVPlayer()

    @Published var status: String? = "Finding sources…"
    @Published var activity: String?
    @Published var failed = false
    @Published var cue: String?
    @Published var showSkip = false
    @Published var finished = false
    @Published var streams: [Stream] = []
    @Published var streamIndex = 0
    @Published var subIndex: Int?
    @Published var subsEnabled = true
    @Published var isPlaying = false
    @Published var buffering = false
    @Published var position: Double = 0
    @Published var duration: Double = 0
    @Published var rate: Float = 1
    @Published var details: Details?

    var pip: AVPictureInPictureController?

    var stream: Stream? { streams.indices.contains(streamIndex) ? streams[streamIndex] : nil }

    private var item: Playable?
    private var core: Core?
    private var cues: [Cue] = []
    private var timeToken: Any?
    private var observers: [NSObjectProtocol] = []
    private var watchers = Set<AnyCancellable>()
    private var itemWatchers = Set<AnyCancellable>()
    private var watchdog: Task<Void, Never>?
    private var activityTask: Task<Void, Never>?
    private var lastReport = Date.distantPast
    private var introSkipped = false
    private var startAt: Double?
    private var generation = 0
    private var attachedAt = Date()

    func log(_ message: String) {
        core?.log("player", message)
    }

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
            position = 0
            duration = 0
            status = "Finding sources…"
            cleanup()
            player.replaceCurrentItem(with: nil)
        }
        cues = []
        introSkipped = false
        startAt = item.startAt
        log("start \(item.heading) from \(Int(item.startAt ?? 0))s")
        pollActivity()
        let started = Date()
        Task {
            let loaded: Details? = try? await core.get("details", ["kind": item.title.kind, "id": String(item.title.id)])
            if mine == self.generation {
                await MainActor.run { self.details = loaded }
            }
        }
        do {
            let reply: PlayResponse = try await core.post("play", item.request)
            guard mine == generation else { return }
            let elapsed = Int(Date().timeIntervalSince(started) * 1000)
            log("resolved [\(reply.streams.map(\.source).joined(separator: ", "))] in \(elapsed)ms")
            await MainActor.run {
                streams = reply.streams
                if reply.streams.isEmpty {
                    status = "No source has this one right now."
                    failed = true
                    stopActivity()
                } else {
                    attach(0)
                }
            }
        } catch {
            guard mine == generation else { return }
            log("resolve failed: \(error.localizedDescription)")
            await MainActor.run {
                status = error.localizedDescription
                failed = true
                stopActivity()
            }
        }
    }

    func stop() {
        if position.isFinite, position > 0 {
            report(position)
        }
        log("stop at \(Int(position))s")
        generation += 1
        cleanup()
        stopActivity()
        watchers.removeAll()
        if let token = timeToken {
            player.removeTimeObserver(token)
            timeToken = nil
        }
        player.replaceCurrentItem(with: nil)
        Task { await core?.flushLogs() }
    }

    func togglePlay() {
        if isPlaying {
            player.pause()
            log("pause at \(Int(position))s")
        } else {
            player.playImmediately(atRate: rate)
            log("play at \(Int(position))s")
        }
    }

    func skip(_ seconds: Double) {
        seek(to: position + seconds)
    }

    func seek(to target: Double) {
        let bounded = duration > 0 ? min(max(target, 0), duration - 0.5) : max(target, 0)
        position = bounded
        player.seek(to: CMTime(seconds: bounded, preferredTimescale: 600))
        log("seek to \(Int(bounded))s")
    }

    func setRate(_ value: Float) {
        rate = value
        if isPlaying { player.rate = value }
        log("rate \(value)")
    }

    func switchSource(_ index: Int) {
        guard index != streamIndex, streams.indices.contains(index) else { return }
        startAt = position > 1 ? position : nil
        log("switch source → \(streams[index].source)")
        attach(index)
    }

    func selectSubtitle(_ index: Int?) {
        subIndex = index
        cues = []
        cue = nil
        guard let index, let stream, stream.subs.indices.contains(index), let url = URL(string: stream.subs[index].url) else {
            if index == nil { log("subtitles off") }
            return
        }
        let mine = generation
        let lang = stream.subs[index].label
        Task {
            do {
                let reply = try await URLSession.shared.data(from: url)
                let code = (reply.1 as? HTTPURLResponse)?.statusCode ?? 0
                guard code == 200 else {
                    self.log("subtitles \(lang) failed: relay \(code) \(String(decoding: reply.0.prefix(120), as: UTF8.self))")
                    return
                }
                let parsed = Subtitles.parse(String(decoding: reply.0, as: UTF8.self))
                await MainActor.run {
                    if mine == self.generation, self.subIndex == index {
                        self.cues = parsed
                    }
                }
                self.log("subtitles \(lang): \(parsed.count) cues")
            } catch {
                self.log("subtitles \(lang) failed: \(error.localizedDescription)")
            }
        }
    }

    func togglePip() {
        guard let pip else {
            log("pip unsupported")
            return
        }
        if pip.isPictureInPictureActive {
            pip.stopPictureInPicture()
        } else {
            pip.startPictureInPicture()
        }
    }

    func skipIntro() {
        guard let intro = stream?.intro, intro.count == 2 else { return }
        seek(to: intro[1])
        introSkipped = true
        showSkip = false
        log("skipped intro")
    }

    private func attach(_ index: Int) {
        guard streams.indices.contains(index) else { return }
        cleanup()
        streamIndex = index
        let stream = streams[index]
        status = "Loading from \(stream.label)…"
        failed = false
        guard let url = URL(string: stream.url) else {
            failover("\(stream.label) gave a bad link")
            return
        }
        log("attach \(stream.source) (\(stream.hls ? "hls" : "mp4"), \(stream.subs.count) subs)")
        attachedAt = Date()
        let avItem = AVPlayerItem(url: url)
        player.automaticallyWaitsToMinimizeStalling = true
        player.replaceCurrentItem(with: avItem)

        avItem.publisher(for: \.status)
            .receive(on: DispatchQueue.main)
            .sink { [weak self] state in
                guard let self, self.player.currentItem === avItem else { return }
                switch state {
                case .readyToPlay:
                    let ms = Int(Date().timeIntervalSince(self.attachedAt) * 1000)
                    let length = avItem.duration.seconds
                    self.log("ready in \(ms)ms, duration \(length.isFinite ? Int(length) : -1)s")
                    self.status = nil
                    self.stopActivity()
                    if let at = self.startAt, at > 0 {
                        self.player.seek(to: CMTime(seconds: at, preferredTimescale: 600))
                        self.startAt = nil
                    }
                    self.player.playImmediately(atRate: self.rate)
                case .failed:
                    self.log("item failed: \(avItem.error?.localizedDescription ?? "unknown") \(self.errorSummary(avItem))")
                    self.failover("\(stream.label) failed")
                default:
                    break
                }
            }
            .store(in: &itemWatchers)

        let center = NotificationCenter.default
        observers.append(center.addObserver(forName: .AVPlayerItemFailedToPlayToEndTime, object: avItem, queue: .main) { [weak self] note in
            let error = note.userInfo?[AVPlayerItemFailedToPlayToEndTimeErrorKey] as? Error
            self?.log("failed to play to end: \(error?.localizedDescription ?? "unknown")")
            self?.failover("\(stream.label) dropped")
        })
        observers.append(center.addObserver(forName: .AVPlayerItemDidPlayToEndTime, object: avItem, queue: .main) { [weak self] _ in
            guard let self else { return }
            let length = avItem.duration.seconds
            if length.isFinite, length > 0 { self.report(length) }
            self.log("finished")
            self.finished = true
        })
        observers.append(center.addObserver(forName: .AVPlayerItemPlaybackStalled, object: avItem, queue: .main) { [weak self] _ in
            self?.log("stalled at \(Int(self?.position ?? 0))s")
        })
        observers.append(center.addObserver(forName: .AVPlayerItemNewErrorLogEntry, object: avItem, queue: .main) { [weak self] _ in
            guard let self else { return }
            self.log("error log: \(self.errorSummary(avItem))")
        })

        watchdog = Task { [weak self] in
            try? await Task.sleep(nanoseconds: 35_000_000_000)
            guard !Task.isCancelled, let self else { return }
            await MainActor.run {
                if self.player.currentItem === avItem, self.status != nil, !self.failed {
                    self.log("watchdog: no playback after 35s")
                    self.failover("\(stream.label) took too long")
                }
            }
        }

        selectSubtitle(stream.subs.isEmpty ? nil : 0)
        installObservers()
    }

    private func errorSummary(_ avItem: AVPlayerItem) -> String {
        guard let event = avItem.errorLog()?.events.last else { return "" }
        let host = event.uri.flatMap { URL(string: $0)?.host } ?? ""
        return "[\(event.errorStatusCode) \(event.errorComment ?? "") \(host)]"
    }

    private func failover(_ reason: String) {
        let next = streamIndex + 1
        if streams.indices.contains(next) {
            status = "\(reason) · trying \(streams[next].label)"
            log("failover: \(reason) → \(streams[next].source)")
            attach(next)
        } else {
            cleanup()
            player.replaceCurrentItem(with: nil)
            status = "Every source failed · \(reason)"
            failed = true
            stopActivity()
            log("every source failed")
        }
    }

    private func cleanup() {
        itemWatchers.removeAll()
        watchdog?.cancel()
        watchdog = nil
        observers.forEach { NotificationCenter.default.removeObserver($0) }
        observers = []
    }

    private func installObservers() {
        guard timeToken == nil else { return }
        timeToken = player.addPeriodicTimeObserver(forInterval: CMTime(seconds: 0.25, preferredTimescale: 600), queue: .main) { [weak self] time in
            self?.tick(time.seconds)
        }
        player.publisher(for: \.timeControlStatus)
            .receive(on: DispatchQueue.main)
            .sink { [weak self] state in
                guard let self else { return }
                let playing = state == .playing
                if playing != self.isPlaying { self.isPlaying = playing }
                let waiting = state == .waitingToPlayAtSpecifiedRate
                if waiting != self.buffering {
                    self.buffering = waiting
                    if waiting { self.log("waiting: \(self.player.reasonForWaitingToPlay?.rawValue ?? "?") at \(Int(self.position))s") }
                }
            }
            .store(in: &watchers)
    }

    private func tick(_ seconds: Double) {
        guard seconds.isFinite else { return }
        position = seconds
        if let length = player.currentItem?.duration.seconds, length.isFinite, length > 0, length != duration {
            duration = length
        }
        let text = Subtitles.cue(in: cues, at: seconds)
        if text != cue { cue = text }
        if let intro = stream?.intro, intro.count == 2, !introSkipped {
            let visible = seconds >= intro[0] && seconds < intro[1] - 1
            if visible != showSkip { showSkip = visible }
            if seconds >= intro[1] { introSkipped = true }
        }
        if Date().timeIntervalSince(lastReport) >= 10 {
            report(seconds)
        }
    }

    private func report(_ seconds: Double) {
        guard let item, let core, duration.isFinite, duration > 0 else { return }
        lastReport = Date()
        let progress = item.progress(position: seconds, duration: duration)
        Task { await core.saveProgress(progress) }
    }

    private func pollActivity() {
        stopActivity()
        guard let core else { return }
        activityTask = Task {
            struct Activity: Decodable {
                struct Fetching: Decodable {
                    let what: String
                    let seconds: Double
                }
                let served: Int
                let fetching: Fetching?
            }
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: 1_000_000_000)
                guard !Task.isCancelled else { return }
                let reply: Activity? = try? await core.get("activity")
                let text = reply.map { a in
                    a.fetching.map { "\($0.what) · \(Int($0.seconds))s · \(a.served) parts" } ?? "\(a.served) parts fetched"
                }
                await MainActor.run { self.activity = text }
            }
        }
    }

    private func stopActivity() {
        activityTask?.cancel()
        activityTask = nil
        activity = nil
    }
}
