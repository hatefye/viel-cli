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
    @State private var fill = false

    init(item: Playable) {
        _current = State(initialValue: item)
    }

    var body: some View {
        GeometryReader { geometry in
            let landscape = geometry.size.width > geometry.size.height
            ZStack {
                Color.black.ignoresSafeArea()
                PlayerSurface(model: model, fill: fill).ignoresSafeArea()
                gestures
                if controls {
                    Group {
                        shade
                        pauseMeta(landscape)
                        controlLayer(landscape)
                    }
                    .transition(.opacity)
                }
                subtitleLayer(landscape)
                if model.status != nil {
                    FinderView(item: current, model: model, retry: { Task { await start(current) } })
                        .transition(.opacity)
                }
                skipLayer(landscape)
                nextLayer
                flashLayer
                panelLayer(landscape, size: geometry.size)
            }
        }
        .statusBarHidden(true)
        .persistentSystemOverlays(.hidden)
        .animation(.easeInOut(duration: 0.28), value: controls)
        .animation(.smooth(duration: 0.2), value: flash)
        .animation(.smooth(duration: 0.25), value: model.isPlaying)
        .animation(.easeInOut(duration: 0.3), value: model.status == nil)
        .animation(.smooth(duration: 0.3), value: sheet)
        .task { await start(current) }
        .onAppear { Orientation.request(.landscape) }
        .onDisappear {
            model.stop()
            Orientation.request(.portrait)
        }
        .onChange(of: model.isPlaying) { _, playing in
            if playing { scheduleHide() } else { show() }
        }
        .onChange(of: model.finished) { _, done in
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
    }

    private func start(_ item: Playable) async {
        current = item
        show()
        await model.start(item, core: core)
    }

    private func show() {
        hideTask?.cancel()
        withAnimation(.easeInOut(duration: 0.28)) { controls = true }
    }

    private func scheduleHide() {
        hideTask?.cancel()
        hideTask = Task {
            try? await Task.sleep(nanoseconds: 3_500_000_000)
            guard !Task.isCancelled, model.isPlaying, !scrubbing, sheet == nil else { return }
            withAnimation(.easeInOut(duration: 0.35)) { controls = false }
        }
    }

    private func poke() {
        withAnimation(.easeInOut(duration: 0.28)) { controls = true }
        if model.isPlaying { scheduleHide() }
    }

    private func toggleControls() {
        if controls {
            if model.isPlaying {
                hideTask?.cancel()
                withAnimation(.easeInOut(duration: 0.28)) { controls = false }
            }
        } else {
            poke()
        }
    }

    private func skip(_ seconds: Double) {
        Haptic.tap()
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
        .simultaneousGesture(
            MagnifyGesture()
                .onEnded { value in
                    let wantsFill = value.magnification > 1
                    if wantsFill != fill {
                        Haptic.select()
                        withAnimation(.smooth(duration: 0.3)) { fill = wantsFill }
                        model.log(wantsFill ? "fill screen" : "fit screen")
                    }
                }
        )
    }

    private var shade: some View {
        VStack(spacing: 0) {
            LinearGradient(colors: [.black.opacity(0.65), .clear], startPoint: .top, endPoint: .bottom).frame(height: 130)
            Spacer()
            LinearGradient(colors: [.clear, .black.opacity(0.8)], startPoint: .top, endPoint: .bottom).frame(height: 220)
        }
        .ignoresSafeArea()
        .allowsHitTesting(false)
    }

    private func controlLayer(_ landscape: Bool) -> some View {
        VStack(spacing: 0) {
            topBar(landscape)
            Spacer()
            if model.status == nil { centerButtons }
            Spacer()
            bottomBar(landscape)
        }
    }

    private func topBar(_ landscape: Bool) -> some View {
        HStack(spacing: 12) {
            Button {
                Haptic.tap()
                dismiss()
            } label: {
                RoundIcon(icon: "chevron.left", size: 40, weight: .semibold)
            }
            .buttonStyle(.glass)
            .buttonBorderShape(.circle)
            VStack(alignment: .leading, spacing: 1) {
                Text(current.title.name)
                    .font(.system(size: 17, weight: .bold))
                    .lineLimit(1)
                if let tag = current.episodeTag {
                    Text([tag, current.episode?.name ?? ""].filter { !$0.isEmpty }.joined(separator: " · "))
                        .font(.system(size: 14))
                        .foregroundStyle(Theme.muted)
                        .lineLimit(1)
                }
            }
            .shadow(color: .black.opacity(0.6), radius: 4)
            Spacer()
        }
        .foregroundStyle(.white)
        .padding(.horizontal, landscape ? 10 : 12)
        .padding(.top, landscape ? 8 : 10)
    }

    private var centerButtons: some View {
        GlassEffectContainer(spacing: 28) {
            HStack(spacing: 28) {
                Button { skip(-10) } label: { RoundIcon(icon: "gobackward.10", size: 56, weight: .regular) }
                    .buttonStyle(.glass)
                    .buttonBorderShape(.circle)
                Button {
                    Haptic.tap()
                    model.togglePlay()
                    poke()
                } label: {
                    Image(systemName: model.isPlaying ? "pause.fill" : "play.fill")
                        .font(.system(size: 32, weight: .semibold))
                        .contentTransition(.symbolEffect(.replace))
                        .frame(width: 78, height: 78)
                }
                .buttonStyle(.glass)
                .buttonBorderShape(.circle)
                Button { skip(10) } label: { RoundIcon(icon: "goforward.10", size: 56, weight: .regular) }
                    .buttonStyle(.glass)
                    .buttonBorderShape(.circle)
            }
        }
        .foregroundStyle(.white)
    }

    private var timeline: some View {
        HStack(spacing: 10) {
            Text(Format.clock(model.position))
                .font(.system(size: 15, weight: .medium))
                .monospacedDigit()
                .contentTransition(.numericText())
            Scrubber(position: model.position, duration: model.duration) { target in
                model.seek(to: target)
                poke()
            } scrubbing: { active in
                scrubbing = active
                if active { show() } else { scheduleHide() }
            }
            Text(Format.clock(model.duration))
                .font(.system(size: 13))
                .monospacedDigit()
                .foregroundStyle(Theme.muted)
        }
    }

    private var tools: some View {
        HStack(spacing: 4) {
            if !current.isMovie {
                toolButton("play.square.stack", "Episodes") { sheet = .episodes }
            }
            if let next = current.next {
                toolButton("forward.end.fill", "Next episode") { Task { await start(next) } }
            }
            toolButton("captions.bubble.fill", "Subtitles") { sheet = .subtitles }
            toolButton("gearshape.fill", "Settings") { sheet = .settings }
            toolButton("pip.enter", "Picture in Picture") { model.togglePip() }
        }
    }

    private func bottomBar(_ landscape: Bool) -> some View {
        Group {
            if landscape {
                HStack(spacing: 16) {
                    timeline
                    tools
                }
            } else {
                VStack(spacing: 2) {
                    timeline
                    HStack {
                        tools
                        Spacer()
                    }
                }
            }
        }
        .foregroundStyle(.white)
        .padding(.horizontal, landscape ? 20 : 14)
        .padding(.bottom, landscape ? 10 : 12)
    }

    private func toolButton(_ icon: String, _ label: String, action: @escaping () -> Void) -> some View {
        Button {
            Haptic.select()
            action()
        } label: {
            Image(systemName: icon)
                .font(.system(size: 24, weight: .medium))
                .frame(width: 46, height: 46)
                .shadow(color: .black.opacity(0.5), radius: 3)
        }
        .buttonStyle(.plain)
        .accessibilityLabel(label)
    }

    @ViewBuilder
    private func pauseMeta(_ landscape: Bool) -> some View {
        if !model.isPlaying, model.status == nil {
            VStack(alignment: .leading, spacing: 6) {
                Logo(url: model.details?.logo, name: current.title.name, maxWidth: landscape ? 200 : 220, size: 26)
                    .padding(.bottom, 6)
                Text(metaLine).font(.system(size: 13, weight: .medium)).foregroundStyle(Theme.muted)
                if let episode = current.episode {
                    Text(episode.displayName).font(.system(size: 16, weight: .bold)).lineLimit(2)
                }
                if let overview = current.episode.map(\.overview).flatMap({ $0.isEmpty ? nil : $0 }) ?? model.details?.overview {
                    Text(overview).font(.system(size: 13.5)).lineSpacing(3).foregroundStyle(Theme.soft).lineLimit(landscape ? 3 : 4)
                }
            }
            .foregroundStyle(.white)
            .frame(maxWidth: landscape ? 360 : 400, alignment: .leading)
            .padding(.horizontal, landscape ? 24 : 16)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
            .padding(.top, landscape ? 56 : 60)
            .padding(.bottom, landscape ? 80 : 130)
            .allowsHitTesting(false)
            .transition(.opacity)
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

    private func subtitleLayer(_ landscape: Bool) -> some View {
        VStack {
            Spacer()
            if let cue = model.cue, model.subsEnabled {
                Text(cue)
                    .font(.system(size: landscape ? 19 : 18, weight: .medium))
                    .multilineTextAlignment(.center)
                    .foregroundStyle(.white)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(Color.black.opacity(0.5), in: RoundedRectangle(cornerRadius: 5, style: .continuous))
                    .padding(.horizontal, 24)
                    .padding(.bottom, controls ? (landscape ? 72 : 118) : 28)
            }
        }
        .allowsHitTesting(false)
    }

    private func skipLayer(_ landscape: Bool) -> some View {
        VStack {
            Spacer()
            HStack {
                Spacer()
                if model.showSkip {
                    Button {
                        Haptic.tap()
                        model.skipIntro()
                    } label: {
                        HStack(spacing: 6) {
                            Text("Skip Intro").font(.system(size: 13, weight: .semibold))
                            Image(systemName: "play.fill").font(.system(size: 10))
                        }
                        .foregroundStyle(.black)
                        .padding(.horizontal, 4)
                    }
                    .buttonStyle(.glassProminent)
                    .tint(.white)
                    .padding(.trailing, 16)
                    .padding(.bottom, controls ? (landscape ? 68 : 118) : 28)
                    .transition(.scale(scale: 0.9).combined(with: .opacity))
                }
            }
        }
        .animation(.smooth(duration: 0.25), value: model.showSkip)
    }

    private var nextLayer: some View {
        VStack {
            Spacer()
            if let next = nextUp {
                HStack(spacing: 14) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Up next in \(countdown)s").font(.system(size: 12)).foregroundStyle(Theme.muted)
                        Text(next.episode?.label(anime: next.isAnime) ?? "").font(.system(size: 15, weight: .semibold)).lineLimit(1)
                    }
                    Button("Play now") {
                        nextUp = nil
                        countdown = 0
                        Task { await start(next) }
                    }
                    .buttonStyle(.glassProminent)
                    .tint(.white)
                    .foregroundStyle(.black)
                    Button("Cancel") {
                        nextUp = nil
                        countdown = 0
                        dismiss()
                    }
                    .buttonStyle(.glass)
                }
                .foregroundStyle(.white)
                .padding(14)
                .glassEffect(.regular, in: .rect(cornerRadius: 20))
                .padding(.bottom, 30)
            }
        }
    }

    @ViewBuilder
    private var flashLayer: some View {
        if let flash {
            Text(flash)
                .font(.system(size: 17, weight: .semibold))
                .foregroundStyle(.white)
                .padding(.horizontal, 16)
                .padding(.vertical, 9)
                .glassEffect(.regular, in: .capsule)
                .transition(.scale(scale: 0.9).combined(with: .opacity))
        }
    }

    @ViewBuilder
    private func panelLayer(_ landscape: Bool, size: CGSize) -> some View {
        if let which = sheet {
            ZStack(alignment: landscape ? .trailing : .bottom) {
                Color.black.opacity(0.35)
                    .ignoresSafeArea()
                    .onTapGesture { sheet = nil }
                sheetContent(which)
                    .frame(width: landscape ? min(440, size.width * 0.52) : nil)
                    .frame(maxHeight: landscape ? .infinity : size.height * 0.62)
                    .glassEffect(.regular, in: .rect(cornerRadius: 28))
                    .padding(landscape ? 10 : 8)
                    .transition(.move(edge: landscape ? .trailing : .bottom).combined(with: .opacity))
            }
        }
    }

    @ViewBuilder
    private func sheetContent(_ which: PlayerSheet) -> some View {
        switch which {
        case .episodes:
            EpisodesSheet(current: current, onPick: { item in
                sheet = nil
                Task { await start(item) }
            }, onClose: { sheet = nil })
        case .settings:
            SettingsSheet(model: model, onSubtitles: { sheet = .subtitles }, onClose: { sheet = nil })
        case .subtitles:
            SubtitlesSheet(model: model, onClose: { sheet = nil })
        }
    }
}

struct FinderView: View {
    let item: Playable
    @ObservedObject var model: PlayerModel
    let retry: () -> Void

    private let columns = [GridItem(.flexible(), spacing: 8), GridItem(.flexible(), spacing: 8)]

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            VStack(alignment: .leading, spacing: 2) {
                Text(item.title.name)
                    .font(.system(size: 30, weight: .black))
                    .lineLimit(1)
                if let episode = item.episode {
                    Text(episode.label(anime: item.isAnime)).font(.system(size: 15)).foregroundStyle(Theme.muted).lineLimit(1)
                }
            }
            HStack(spacing: 14) {
                if model.failed {
                    Image(systemName: "xmark.circle.fill").font(.system(size: 24)).foregroundStyle(Theme.accentHi)
                } else {
                    ProgressView().tint(.white).scaleEffect(1.1)
                }
                VStack(alignment: .leading, spacing: 2) {
                    Text(model.status ?? "").font(.system(size: 16, weight: .bold)).lineLimit(2)
                    if let activity = model.activity, !model.failed {
                        Text(activity).font(.system(size: 13)).foregroundStyle(Theme.muted).lineLimit(1)
                    }
                }
                Spacer(minLength: 0)
                if model.failed {
                    Button("Try again", action: retry)
                        .buttonStyle(.glassProminent)
                        .tint(.white)
                        .foregroundStyle(.black)
                }
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 12)
            .background(Color.white.opacity(0.05), in: RoundedRectangle(cornerRadius: 14, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 14, style: .continuous).stroke(Theme.border, lineWidth: 1))
            if !model.probes.isEmpty {
                LazyVGrid(columns: columns, spacing: 8) {
                    ForEach(model.probes, id: \.name) { probe in
                        HStack(spacing: 10) {
                            probeDot(probe.state)
                            Text(probe.label).font(.system(size: 15, weight: .bold)).foregroundStyle(probe.state == "found" || probe.state == "checking" ? .white : Theme.faint)
                            Spacer(minLength: 0)
                            Text(probeText(probe.state)).font(.system(size: 12)).foregroundStyle(probe.state == "found" ? Color(hex: 0x7be0a6) : Theme.faint)
                        }
                        .padding(.horizontal, 12)
                        .padding(.vertical, 10)
                        .background(Color.white.opacity(probe.state == "found" ? 0.08 : 0.03), in: RoundedRectangle(cornerRadius: 12, style: .continuous))
                    }
                }
                .animation(.smooth(duration: 0.3), value: model.probes)
            }
        }
        .foregroundStyle(.white)
        .padding(18)
        .frame(maxWidth: 560)
        .glassEffect(.regular, in: .rect(cornerRadius: 22))
        .padding(.horizontal, 24)
        .allowsHitTesting(model.failed)
    }

    @ViewBuilder
    private func probeDot(_ state: String) -> some View {
        switch state {
        case "checking":
            ProgressView().tint(.white).scaleEffect(0.7).frame(width: 20, height: 20)
        case "found":
            ZStack {
                Circle().fill(Color(hex: 0x2fbf71)).frame(width: 20, height: 20)
                Image(systemName: "checkmark").font(.system(size: 11, weight: .bold)).foregroundStyle(.black)
            }
        default:
            Circle().stroke(Color.white.opacity(0.15), lineWidth: 2).frame(width: 20, height: 20)
        }
    }

    private func probeText(_ state: String) -> String {
        switch state {
        case "checking": return "Checking…"
        case "found": return "Found"
        case "skipped": return "Slow"
        default: return "No stream"
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
                Capsule().fill(Color.white.opacity(0.28)).frame(height: 3)
                Capsule().fill(Theme.accent).frame(width: max(width * fraction, 0), height: 3)
                Circle().fill(.white).frame(width: 13, height: 13).offset(x: max(width * fraction - 6.5, -6.5))
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
    var fill = false

    func makeUIView(context: Context) -> PlayerLayerView {
        let view = PlayerLayerView()
        view.backgroundColor = .black
        view.playerLayer.player = model.player
        view.playerLayer.videoGravity = fill ? .resizeAspectFill : .resizeAspect
        if AVPictureInPictureController.isPictureInPictureSupported(), let pip = AVPictureInPictureController(playerLayer: view.playerLayer) {
            pip.canStartPictureInPictureAutomaticallyFromInline = true
            pip.delegate = context.coordinator
            context.coordinator.pip = pip
            model.pip = pip
        }
        return view
    }

    func updateUIView(_ view: PlayerLayerView, context: Context) {
        let gravity: AVLayerVideoGravity = fill ? .resizeAspectFill : .resizeAspect
        if view.playerLayer.videoGravity != gravity {
            CATransaction.begin()
            CATransaction.setAnimationDuration(0.3)
            view.playerLayer.videoGravity = gravity
            CATransaction.commit()
        }
    }

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
    @Published var probes: [Probe] = []
    @Published var resolving = false
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
            probes = []
            resolving = true
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
                resolving = false
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
                resolving = false
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
                let sources: [Probe]
            }
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: 500_000_000)
                guard !Task.isCancelled else { return }
                let reply: Activity? = try? await core.get("activity")
                let text = reply.flatMap { a in
                    a.fetching.map { "\($0.what) · \(Int($0.seconds))s" } ?? (a.served > 0 ? "\(a.served) parts fetched" : nil)
                }
                await MainActor.run {
                    self.activity = text
                    if let sources = reply?.sources, sources != self.probes { self.probes = sources }
                }
            }
        }
    }

    private func stopActivity() {
        activityTask?.cancel()
        activityTask = nil
        activity = nil
    }
}
