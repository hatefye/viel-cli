import AVFoundation
import AVKit
import Combine
import MediaPlayer
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
    @State private var dismissedNext = false
    @State private var flash: Flash?
    @State private var flashShown = false
    @State private var flashTask: Task<Void, Never>?
    @State private var fill = false
    @State private var preview: Double?
    @State private var locked = false
    @State private var unlockVisible = false
    @State private var level: Level?
    @State private var levelShown = false
    @State private var levelTask: Task<Void, Never>?
    @State private var dragStart: Double?
    @State private var boosting = false

    init(item: Playable) {
        _current = State(initialValue: item)
    }

    var body: some View {
        GeometryReader { geometry in
            let landscape = geometry.size.width > geometry.size.height
            ZStack {
                Color.black.ignoresSafeArea()
                PlayerSurface(model: model, fill: fill).ignoresSafeArea()
                touchPad
                Group {
                    shade
                    pauseMeta(landscape, size: geometry.size)
                    controlLayer(landscape)
                }
                .opacity(controls && !locked ? 1 : 0)
                .allowsHitTesting(controls && !locked)
                subtitleLayer(landscape)
                if model.status != nil {
                    FinderView(item: current, model: model, retry: { Task { await start(current) } })
                        .transition(.opacity)
                }
                skipLayer(landscape)
                nextLayer
                previewLayer(landscape, size: geometry.size)
                flashLayer
                levelLayer
                statsLayer(landscape)
                panelLayer(landscape, size: geometry.size)
                lockLayer
            }
        }
        .statusBarHidden(true)
        .persistentSystemOverlays(.hidden)
        .animation(.easeInOut(duration: 0.28), value: controls)
        .animation(.smooth(duration: 0.25), value: flashShown)
        .animation(.smooth(duration: 0.25), value: levelShown)
        .animation(.smooth(duration: 0.25), value: locked)
        .animation(.smooth(duration: 0.25), value: unlockVisible)
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
            if let key = current.offline, core.settings.autoDeleteWatched {
                Task { await core.removeDownload(key) }
            }
            if let next = current.next, current.offline == nil {
                let length = model.duration > 0 ? model.duration : Double((next.episode?.runtime ?? 24) * 60)
                Task { await core.saveProgress(next.progress(position: 0, duration: length)) }
            }
            if model.sleepAtEnd {
                dismiss()
            } else if core.settings.autoplay, !dismissedNext, let next = current.next {
                Task { await start(next) }
            } else {
                dismiss()
            }
        }
    }

    private var skipSeconds: Double { Double(core.settings.skipSeconds) }

    private var nextUp: Playable? {
        guard core.settings.autoplay, !dismissedNext, model.status == nil, model.duration > 0, let next = current.next else { return nil }
        return model.duration - model.position <= 20 ? next : nil
    }

    private func start(_ item: Playable) async {
        current = item
        dismissedNext = false
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
        let amount = Int(abs(seconds))
        showFlash(Flash(text: seconds < 0 ? "−\(amount)s" : "+\(amount)s", right: seconds > 0), for: 0.7)
        poke()
    }

    private func showFlash(_ value: Flash, for seconds: Double?) {
        flash = value
        flashShown = true
        flashTask?.cancel()
        guard let seconds else { return }
        flashTask = Task {
            try? await Task.sleep(nanoseconds: UInt64(seconds * 1_000_000_000))
            guard !Task.isCancelled else { return }
            flashShown = false
        }
    }

    private func hideFlash() {
        flashTask?.cancel()
        flashShown = false
    }

    private func showLevel(_ value: Level) {
        level = value
        levelShown = true
        levelTask?.cancel()
        levelTask = Task {
            try? await Task.sleep(nanoseconds: 1_100_000_000)
            guard !Task.isCancelled else { return }
            levelShown = false
        }
    }

    private var touchPad: some View {
        TouchPad(
            onTap: {
                model.log("touch: tap (controls \(controls ? "on" : "off"))")
                toggleControls()
            },
            onDoubleTap: { right in
                model.log("touch: double \(right ? "right" : "left")")
                skip(right ? skipSeconds : -skipSeconds)
            },
            onHold: { down in
                if down {
                    guard model.isPlaying, model.status == nil else { return }
                    boosting = true
                    Haptic.tap()
                    model.boost(true)
                    showFlash(Flash(text: "2× speed", right: nil), for: nil)
                } else if boosting {
                    boosting = false
                    model.boost(false)
                    hideFlash()
                }
            },
            onSwipe: { right, shift, phase in
                guard !boosting else { return }
                switch phase {
                case .began:
                    model.log("touch: swipe \(right ? "volume" : "brightness")")
                    dragStart = right ? Double(SystemVolume.level) : Double(UIScreen.main.brightness)
                case .changed:
                    guard let base = dragStart else { return }
                    let next = min(max(base + shift, 0), 1)
                    if right {
                        SystemVolume.set(Float(next))
                        showLevel(Level(icon: next == 0 ? "speaker.slash.fill" : (next < 0.5 ? "speaker.wave.1.fill" : "speaker.wave.2.fill"), value: next, right: true))
                    } else {
                        UIScreen.main.brightness = next
                        showLevel(Level(icon: next < 0.4 ? "sun.min.fill" : "sun.max.fill", value: next, right: false))
                    }
                case .ended:
                    dragStart = nil
                    levelTask?.cancel()
                    levelTask = Task {
                        try? await Task.sleep(nanoseconds: 600_000_000)
                        guard !Task.isCancelled else { return }
                        levelShown = false
                    }
                }
            },
            onPinch: { wantsFill in
                if wantsFill != fill {
                    Haptic.select()
                    withAnimation(.smooth(duration: 0.3)) { fill = wantsFill }
                    model.log(wantsFill ? "fill screen" : "fit screen")
                }
            }
        )
        .ignoresSafeArea()
        .allowsHitTesting(!locked)
    }

    private func revealUnlock() {
        unlockVisible = true
        hideTask?.cancel()
        hideTask = Task {
            try? await Task.sleep(nanoseconds: 2_500_000_000)
            guard !Task.isCancelled else { return }
            unlockVisible = false
        }
    }

    @ViewBuilder
    private var lockLayer: some View {
        if locked {
            ZStack {
                Color.clear.contentShape(Rectangle())
                    .ignoresSafeArea()
                    .onTapGesture { revealUnlock() }
                if unlockVisible {
                    VStack {
                        Spacer()
                        Button {
                            Haptic.tap()
                            locked = false
                            unlockVisible = false
                            poke()
                        } label: {
                            HStack(spacing: 8) {
                                Image(systemName: "lock.fill").font(.system(size: 15, weight: .semibold))
                                Text("Unlock").font(.system(size: 16, weight: .semibold))
                            }
                            .foregroundStyle(.white)
                            .padding(.horizontal, 12)
                            .padding(.vertical, 6)
                        }
                        .buttonStyle(.glass)
                        .padding(.bottom, 36)
                    }
                    .transition(.opacity)
                }
            }
        }
    }

    @ViewBuilder
    private var levelLayer: some View {
        if let level {
            HStack(spacing: 0) {
                if level.right { Spacer() }
                VStack(spacing: 10) {
                    Image(systemName: level.icon).font(.system(size: 18, weight: .semibold)).frame(height: 22)
                    GeometryReader { geometry in
                        ZStack(alignment: .bottom) {
                            Capsule().fill(Color.white.opacity(0.25))
                            Capsule().fill(.white).frame(height: geometry.size.height * level.value)
                        }
                    }
                    .frame(width: 5, height: 110)
                    Text("\(Int((level.value * 100).rounded()))").font(.system(size: 12, weight: .semibold)).monospacedDigit()
                }
                .foregroundStyle(.white)
                .padding(.horizontal, 12)
                .padding(.vertical, 12)
                .glassEffect(.regular, in: .rect(cornerRadius: 20))
                .padding(.horizontal, 28)
                if !level.right { Spacer() }
            }
            .opacity(levelShown ? 1 : 0)
            .allowsHitTesting(false)
        }
    }

    @ViewBuilder
    private func statsLayer(_ landscape: Bool) -> some View {
        if model.showStats, !model.stats.isEmpty {
            VStack {
                HStack {
                    Spacer()
                    VStack(alignment: .leading, spacing: 3) {
                        ForEach(model.stats, id: \.self) { line in
                            Text(line).font(.system(size: 12, weight: .medium, design: .monospaced))
                        }
                    }
                    .foregroundStyle(.white)
                    .padding(10)
                    .background(Color.black.opacity(0.6), in: RoundedRectangle(cornerRadius: 10, style: .continuous))
                    .padding(.trailing, landscape ? 16 : 12)
                    .padding(.top, landscape ? 64 : 60)
                }
                Spacer()
            }
            .allowsHitTesting(false)
        }
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
            if model.isPlaying || model.status != nil {
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
                .transition(.opacity)
            }
            Spacer()
            if model.sleepAt != nil || model.sleepAtEnd {
                HStack(spacing: 5) {
                    Image(systemName: "moon.zzz.fill").font(.system(size: 13, weight: .semibold))
                    Text(model.sleepLabel).font(.system(size: 13, weight: .semibold)).monospacedDigit()
                }
                .foregroundStyle(Theme.soft)
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
                .glassEffect(.regular, in: .capsule)
            }
        }
        .foregroundStyle(.white)
        .padding(.horizontal, landscape ? 10 : 12)
        .padding(.top, landscape ? 22 : 10)
    }

    private var centerButtons: some View {
        GlassEffectContainer(spacing: 28) {
            HStack(spacing: 28) {
                Button { skip(-skipSeconds) } label: { RoundIcon(icon: "gobackward.\(core.settings.skipSeconds)", size: 56, weight: .regular) }
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
                Button { skip(skipSeconds) } label: { RoundIcon(icon: "goforward.\(core.settings.skipSeconds)", size: 56, weight: .regular) }
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
            } preview: { time in
                preview = time
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
            toolButton("lock.open", "Lock controls") {
                locked = true
                hideTask?.cancel()
                controls = false
                model.log("controls locked")
            }
            RoutePicker()
                .frame(width: 46, height: 46)
                .shadow(color: .black.opacity(0.5), radius: 3)
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
    private func pauseMeta(_ landscape: Bool, size: CGSize) -> some View {
        if !model.isPlaying, model.status == nil {
            let width = landscape ? min(400, max((size.width - 330) / 2, 220)) : size.width - 32
            VStack(alignment: .leading, spacing: 6) {
                Logo(url: model.details?.logo, name: current.title.name, maxWidth: min(width, 220), size: 24)
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
            .frame(width: width, alignment: .leading)
            .padding(.leading, landscape ? 24 : 16)
            .padding(.top, landscape ? 80 : 68)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
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
        let style = core.settings
        return VStack {
            Spacer()
            if let cue = model.cue, model.subsEnabled {
                Text(cue)
                    .font(.system(size: (landscape ? 19 : 18) * style.subSize, weight: .medium))
                    .multilineTextAlignment(.center)
                    .foregroundStyle(.white)
                    .shadow(color: .black.opacity(style.subBackground ? 0 : 0.9), radius: 2)
                    .shadow(color: .black.opacity(style.subBackground ? 0 : 0.9), radius: 1)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(Color.black.opacity(style.subBackground ? 0.5 : 0), in: RoundedRectangle(cornerRadius: 5, style: .continuous))
                    .padding(.horizontal, 24)
                    .padding(.bottom, (controls ? (landscape ? 72 : 118) : 28) + style.subLift)
            }
        }
        .allowsHitTesting(false)
    }

    private func skipLayer(_ landscape: Bool) -> some View {
        VStack {
            Spacer()
            HStack {
                Spacer()
                if let skip = model.skipTarget {
                    Button {
                        Haptic.tap()
                        model.skipMarker()
                        poke()
                    } label: {
                        HStack(spacing: 8) {
                            Text(skip == .intro ? "Skip Intro" : "Skip Outro").font(.system(size: 17, weight: .semibold))
                            Image(systemName: "forward.fill").font(.system(size: 14, weight: .semibold))
                        }
                        .foregroundStyle(.black)
                        .padding(.horizontal, 10)
                        .padding(.vertical, 6)
                    }
                    .buttonStyle(.glassProminent)
                    .tint(.white)
                    .padding(.trailing, landscape ? 24 : 16)
                    .padding(.bottom, controls ? (landscape ? 72 : 124) : 32)
                    .transition(.scale(scale: 0.9).combined(with: .opacity))
                }
            }
        }
        .animation(.smooth(duration: 0.25), value: model.skipTarget)
    }

    private var nextLayer: some View {
        VStack {
            Spacer()
            if let next = nextUp {
                let left = max(model.duration - model.position, 0)
                HStack(spacing: 14) {
                    ZStack {
                        Circle().stroke(Color.white.opacity(0.2), lineWidth: 3)
                        Circle()
                            .trim(from: 0, to: 1 - min(left / 20, 1))
                            .stroke(Color.white, style: StrokeStyle(lineWidth: 3, lineCap: .round))
                            .rotationEffect(.degrees(-90))
                        Text("\(Int(left.rounded(.up)))").font(.system(size: 15, weight: .bold)).monospacedDigit()
                    }
                    .frame(width: 40, height: 40)
                    .animation(.linear(duration: 0.25), value: left)
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Next episode").font(.system(size: 12, weight: .medium)).foregroundStyle(Theme.muted)
                        Text(next.episode?.label(anime: next.isAnime) ?? "").font(.system(size: 15, weight: .semibold)).lineLimit(1)
                    }
                    Button("Play now") {
                        Haptic.tap()
                        Task { await start(next) }
                    }
                    .buttonStyle(.glassProminent)
                    .tint(.white)
                    .foregroundStyle(.black)
                    Button {
                        dismissedNext = true
                    } label: {
                        Image(systemName: "xmark").font(.system(size: 15, weight: .semibold)).frame(width: 36, height: 36)
                    }
                    .buttonStyle(.glass)
                    .buttonBorderShape(.circle)
                }
                .foregroundStyle(.white)
                .padding(14)
                .glassEffect(.regular, in: .rect(cornerRadius: 22))
                .padding(.bottom, controls ? 90 : 34)
                .transition(.move(edge: .bottom).combined(with: .opacity))
            }
        }
        .animation(.smooth(duration: 0.3), value: nextUp == nil)
    }

    @ViewBuilder
    private func previewLayer(_ landscape: Bool, size: CGSize) -> some View {
        if let preview, model.duration > 0 {
            let width: CGFloat = 164
            let inset: CGFloat = landscape ? 78 : 60
            let span = size.width - inset * 2 - width
            let x = inset + span * min(max(preview / model.duration, 0), 1)
            VStack {
                Spacer()
                HStack(spacing: 0) {
                    Spacer().frame(width: max(x, 8))
                    ScrubPreview(model: model, time: preview, width: width)
                    Spacer(minLength: 0)
                }
                .padding(.bottom, landscape ? 64 : 112)
            }
            .allowsHitTesting(false)
            .transition(.opacity)
        }
    }

    @ViewBuilder
    private var flashLayer: some View {
        if let flash {
            let pill = HStack(spacing: 6) {
                if flash.right == nil { Image(systemName: "forward.fill").font(.system(size: 13, weight: .semibold)) }
                Text(flash.text).font(.system(size: 17, weight: .semibold))
            }
            .foregroundStyle(.white)
            .padding(.horizontal, 16)
            .padding(.vertical, 9)
            .glassEffect(.regular, in: .capsule)
            Group {
                if let right = flash.right {
                    HStack(spacing: 0) {
                        pill.frame(maxWidth: .infinity).opacity(right ? 0 : 1)
                        pill.frame(maxWidth: .infinity).opacity(right ? 1 : 0)
                    }
                } else {
                    VStack {
                        pill.padding(.top, 80)
                        Spacer()
                    }
                }
            }
            .opacity(flashShown ? 1 : 0)
            .scaleEffect(flashShown ? 1 : 0.92)
            .allowsHitTesting(false)
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
            SettingsSheet(model: model, item: current, onSubtitles: { sheet = .subtitles }, onClose: { sheet = nil })
        case .subtitles:
            SubtitlesSheet(model: model, onClose: { sheet = nil })
        }
    }
}

struct Flash: Equatable {
    let text: String
    let right: Bool?
}

struct Level: Equatable {
    let icon: String
    let value: Double
    let right: Bool
}

enum SwipePhase {
    case began, changed, ended
}

struct TouchPad: UIViewRepresentable {
    let onTap: () -> Void
    let onDoubleTap: (Bool) -> Void
    let onHold: (Bool) -> Void
    let onSwipe: (Bool, Double, SwipePhase) -> Void
    let onPinch: (Bool) -> Void

    func makeUIView(context: Context) -> UIView {
        let view = UIView()
        view.backgroundColor = .clear
        view.isMultipleTouchEnabled = true
        let owner = context.coordinator
        let single = UITapGestureRecognizer(target: owner, action: #selector(Coordinator.tapped(_:)))
        let double = UITapGestureRecognizer(target: owner, action: #selector(Coordinator.doubleTapped(_:)))
        double.numberOfTapsRequired = 2
        single.require(toFail: double)
        let hold = UILongPressGestureRecognizer(target: owner, action: #selector(Coordinator.held(_:)))
        hold.minimumPressDuration = 0.6
        let pan = UIPanGestureRecognizer(target: owner, action: #selector(Coordinator.panned(_:)))
        pan.maximumNumberOfTouches = 1
        let pinch = UIPinchGestureRecognizer(target: owner, action: #selector(Coordinator.pinched(_:)))
        for recognizer in [single, double, hold, pan, pinch] as [UIGestureRecognizer] {
            recognizer.delegate = owner
            recognizer.cancelsTouchesInView = false
            view.addGestureRecognizer(recognizer)
        }
        return view
    }

    func updateUIView(_ view: UIView, context: Context) {
        context.coordinator.pad = self
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(pad: self)
    }

    final class Coordinator: NSObject, UIGestureRecognizerDelegate {
        var pad: TouchPad
        private var holding = false
        private var swiping = false
        private var swipeRight = false
        private var pending = false
        private var origin = CGPoint.zero
        private var touchDown = CGPoint.zero

        init(pad: TouchPad) {
            self.pad = pad
        }

        @objc func tapped(_ recognizer: UITapGestureRecognizer) {
            pad.onTap()
        }

        @objc func doubleTapped(_ recognizer: UITapGestureRecognizer) {
            guard let view = recognizer.view else { return }
            pad.onDoubleTap(recognizer.location(in: view).x > view.bounds.midX)
        }

        @objc func held(_ recognizer: UILongPressGestureRecognizer) {
            switch recognizer.state {
            case .began:
                guard !swiping else { return }
                holding = true
                pad.onHold(true)
            case .ended, .cancelled, .failed:
                if holding {
                    holding = false
                    pad.onHold(false)
                }
            default:
                break
            }
        }

        @objc func panned(_ recognizer: UIPanGestureRecognizer) {
            guard let view = recognizer.view else { return }
            let bounds = view.bounds
            switch recognizer.state {
            case .began:
                origin = touchDown
                let band: CGFloat = 56
                let inside = origin.y > bounds.height * 0.2 && origin.y < bounds.height * 0.78
                let edge = origin.x < band || origin.x > bounds.width - band
                guard !holding, inside, edge else {
                    recognizer.isEnabled = false
                    recognizer.isEnabled = true
                    return
                }
                pending = true
                swipeRight = origin.x > bounds.midX
            case .changed:
                let shift = recognizer.translation(in: view)
                if pending {
                    guard abs(shift.y) > 24 else { return }
                    guard abs(shift.y) > abs(shift.x) * 2 else {
                        pending = false
                        recognizer.isEnabled = false
                        recognizer.isEnabled = true
                        return
                    }
                    pending = false
                    swiping = true
                    recognizer.setTranslation(.zero, in: view)
                    pad.onSwipe(swipeRight, 0, .began)
                    return
                }
                guard swiping else { return }
                pad.onSwipe(swipeRight, Double(-shift.y / 220), .changed)
            case .ended, .cancelled, .failed:
                pending = false
                guard swiping else { return }
                swiping = false
                pad.onSwipe(swipeRight, 0, .ended)
            default:
                break
            }
        }

        @objc func pinched(_ recognizer: UIPinchGestureRecognizer) {
            guard recognizer.state == .ended else { return }
            pad.onPinch(recognizer.scale > 1)
        }

        func gestureRecognizer(_ gestureRecognizer: UIGestureRecognizer, shouldRecognizeSimultaneouslyWith other: UIGestureRecognizer) -> Bool {
            true
        }

        func gestureRecognizer(_ gestureRecognizer: UIGestureRecognizer, shouldReceive touch: UITouch) -> Bool {
            if gestureRecognizer is UIPanGestureRecognizer, let view = gestureRecognizer.view {
                touchDown = touch.location(in: view)
            }
            return true
        }
    }
}

enum SystemVolume {
    private static let view = MPVolumeView(frame: CGRect(x: -400, y: -400, width: 10, height: 10))

    static var level: Float {
        AVAudioSession.sharedInstance().outputVolume
    }

    static func set(_ value: Float) {
        guard let slider = view.subviews.compactMap({ $0 as? UISlider }).first else { return }
        slider.value = value
        slider.sendActions(for: .valueChanged)
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

struct RoutePicker: UIViewRepresentable {
    func makeUIView(context: Context) -> AVRoutePickerView {
        let view = AVRoutePickerView()
        view.tintColor = .white
        view.activeTintColor = UIColor(Theme.accentHi)
        view.prioritizesVideoDevices = true
        return view
    }

    func updateUIView(_ view: AVRoutePickerView, context: Context) {}
}

struct ScrubPreview: View {
    @ObservedObject var model: PlayerModel
    let time: Double
    let width: CGFloat

    @State private var image: UIImage?

    var body: some View {
        VStack(spacing: 6) {
            if let image {
                Image(uiImage: image)
                    .resizable()
                    .scaledToFill()
                    .frame(width: width, height: width * 9 / 16)
                    .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
                    .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).stroke(Color.white.opacity(0.25), lineWidth: 1))
            }
            Text(Format.clock(time))
                .font(.system(size: 15, weight: .semibold))
                .monospacedDigit()
                .foregroundStyle(.white)
                .padding(.horizontal, 10)
                .padding(.vertical, 5)
                .glassEffect(.regular, in: .capsule)
        }
        .task(id: Int(time / 5)) {
            image = await model.thumbnail(at: time)
        }
    }
}

struct Scrubber: View {
    let position: Double
    let duration: Double
    let onSeek: (Double) -> Void
    let scrubbing: (Bool) -> Void
    var preview: (Double?) -> Void = { _ in }

    @State private var dragging = false
    @State private var dragFraction: Double = 0
    @State private var tick = 0

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
                        if !dragging {
                            scrubbing(true)
                            tick = Int(position / 30)
                        }
                        dragging = true
                        dragFraction = min(max(value.location.x / width, 0), 1)
                        let target = dragFraction * duration
                        let step = Int(target / 30)
                        if step != tick {
                            tick = step
                            Haptic.select()
                        }
                        preview(target)
                    }
                    .onEnded { value in
                        dragging = false
                        let fraction = min(max(value.location.x / width, 0), 1)
                        onSeek(fraction * duration)
                        scrubbing(false)
                        preview(nil)
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
    @Published var skipTarget: Marker?
    @Published var finished = false
    @Published var thumbs: [ThumbCue] = []
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
    @Published var sleepAt: Date?
    @Published var sleepAtEnd = false
    @Published var showStats = false
    @Published var stats: [String] = []

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
    private var passed: Set<Marker> = []
    private var startAt: Double?
    private var generation = 0
    private var attachedAt = Date()
    private var sleepTask: Task<Void, Never>?
    private var lastStats = Date.distantPast

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
            skipTarget = nil
            thumbs = []
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
        passed = []
        startAt = item.startAt
        log("start \(item.heading) from \(Int(item.startAt ?? 0))s\(item.offline == nil ? "" : " (offline)")")
        if item.offline == nil { pollActivity() }
        let started = Date()
        Task {
            let loaded: Details? = try? await core.get("details", ["kind": item.title.kind, "id": String(item.title.id)])
            if mine == self.generation {
                await MainActor.run { self.details = loaded }
            }
        }
        do {
            let reply: PlayResponse = if let key = item.offline {
                try await core.get("offline", ["key": key])
            } else {
                try await core.post("play", item.request)
            }
            guard mine == generation else { return }
            let elapsed = Int(Date().timeIntervalSince(started) * 1000)
            log("resolved [\(reply.streams.map(\.source).joined(separator: ", "))] in \(elapsed)ms")
            let ordered = Memory.order(reply.streams, for: item.title.key)
            await MainActor.run {
                streams = ordered
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
        sleepTask?.cancel()
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

    func boost(_ on: Bool) {
        guard player.currentItem != nil else { return }
        player.rate = on ? max(rate * 2, 2) : rate
        log(on ? "hold: 2x" : "hold released")
    }

    var sleepLabel: String {
        if sleepAtEnd { return "End of episode" }
        guard let sleepAt else { return "Off" }
        let left = max(sleepAt.timeIntervalSinceNow, 0)
        return left >= 60 ? "\(Int(left / 60) + (Int(left) % 60 > 0 ? 1 : 0))m" : "\(Int(left))s"
    }

    func setSleep(minutes: Int?, atEnd: Bool = false) {
        sleepTask?.cancel()
        sleepTask = nil
        sleepAtEnd = atEnd
        guard let minutes else {
            sleepAt = nil
            log(atEnd ? "sleep at end of episode" : "sleep timer off")
            return
        }
        let when = Date().addingTimeInterval(Double(minutes) * 60)
        sleepAt = when
        log("sleep in \(minutes)m")
        sleepTask = Task { [weak self] in
            try? await Task.sleep(nanoseconds: UInt64(minutes) * 60_000_000_000)
            guard !Task.isCancelled, let self else { return }
            await MainActor.run {
                self.player.pause()
                self.sleepAt = nil
                self.log("sleep timer fired")
            }
        }
    }

    private func refreshStats() {
        guard showStats, Date().timeIntervalSince(lastStats) >= 1 else { return }
        lastStats = Date()
        var lines: [String] = []
        lines.append("source   \(stream?.label ?? "—")\(stream?.hls == true ? " · hls" : "")")
        if let item = player.currentItem {
            let size = item.presentationSize
            if size.width > 0 { lines.append("video    \(Int(size.width))×\(Int(size.height))") }
            if let event = item.accessLog()?.events.last {
                if event.indicatedBitrate > 0 { lines.append("bitrate  \(String(format: "%.1f", event.indicatedBitrate / 1_000_000)) Mbit/s") }
                if event.observedBitrate > 0 { lines.append("network  \(String(format: "%.1f", event.observedBitrate / 1_000_000)) Mbit/s") }
                lines.append("stalls   \(event.numberOfStalls) · dropped \(max(event.numberOfDroppedVideoFrames, 0))")
            }
            if let range = item.loadedTimeRanges.last?.timeRangeValue {
                let ahead = range.end.seconds - position
                if ahead.isFinite { lines.append("buffer   \(String(format: "%.1f", max(ahead, 0)))s") }
            }
        }
        lines.append("rate     \(player.rate)x · \(Int(position))s")
        if let sub = subIndex, let stream, stream.subs.indices.contains(sub) { lines.append("subs     \(stream.subs[sub].label) · \(cues.count) cues") }
        stats = lines
    }

    func switchSource(_ index: Int) {
        guard index != streamIndex, streams.indices.contains(index) else { return }
        startAt = position > 1 ? position : nil
        log("switch source → \(streams[index].source)")
        attach(index)
    }

    func reattach() {
        guard streams.indices.contains(streamIndex) else { return }
        startAt = position > 1 ? position : nil
        log("reattach \(streams[streamIndex].source)")
        attach(streamIndex)
    }

    func report() {
        let place = item?.heading ?? "?"
        let via = stream?.source ?? "none"
        log("USER REPORT: \(place) via \(via) at \(Int(position))s, \(status ?? (isPlaying ? "playing" : "paused")), \(errorSummaryText)")
        Task { await core?.flushLogs() }
    }

    private var errorSummaryText: String {
        guard let avItem = player.currentItem else { return "no item" }
        let summary = errorSummary(avItem)
        return summary.isEmpty ? "no errors" : summary
    }

    func disableSubtitles() {
        subsEnabled = false
        if let item { Memory.remember(subtitle: nil, for: item.title.key) }
        log("subtitles off")
    }

    func selectSubtitle(_ index: Int?, remember: Bool = true) {
        subIndex = index
        cues = []
        cue = nil
        guard let index, let stream, stream.subs.indices.contains(index), let url = URL(string: stream.subs[index].url) else {
            if index == nil { log("subtitles off") }
            return
        }
        if remember, let item { Memory.remember(subtitle: stream.subs[index].label, for: item.title.key) }
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

    func skipMarker() {
        guard let target = skipTarget, let span = marker(target), span.count == 2 else { return }
        seek(to: span[1])
        passed.insert(target)
        skipTarget = nil
        log("skipped \(target)")
    }

    private func marker(_ which: Marker) -> [Double]? {
        which == .intro ? stream?.intro : stream?.outro
    }

    func thumbnail(at time: Double) async -> UIImage? {
        guard let cue = thumbs.first(where: { time >= $0.start && time < $0.end }) else { return nil }
        guard let sprite = await SpriteStore.shared.image(for: cue.url) else { return nil }
        let scale = sprite.scale
        let rect = CGRect(x: cue.rect.minX * scale, y: cue.rect.minY * scale, width: cue.rect.width * scale, height: cue.rect.height * scale)
        guard let cropped = sprite.cgImage?.cropping(to: rect) else { return nil }
        return UIImage(cgImage: cropped)
    }

    private func loadThumbs(_ stream: Stream) {
        thumbs = []
        guard let core, let link = stream.thumbs, let url = URL(string: link) else { return }
        let query = URLComponents(string: link)?.queryItems ?? []
        guard let origin = query.first(where: { $0.name == "u" })?.value, let base = URL(string: origin) else { return }
        let referer = query.first(where: { $0.name == "r" })?.value
        let mine = generation
        Task {
            guard let reply = try? await URLSession.shared.data(from: url), (reply.1 as? HTTPURLResponse)?.statusCode == 200 else {
                self.log("thumbnails unavailable")
                return
            }
            let parsed = ThumbCue.parse(String(decoding: reply.0, as: UTF8.self), base: base) { absolute in
                core.relayLink(absolute, referer: referer)
            }
            await MainActor.run {
                if mine == self.generation { self.thumbs = parsed }
            }
            self.log("thumbnails: \(parsed.count) cues")
        }
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
        player.allowsExternalPlayback = url.host != "127.0.0.1"
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
                    if !stream.isOffline, let item = self.item { Memory.remember(source: stream.source, for: item.title.key) }
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

        chooseSubtitle(stream)
        loadThumbs(stream)
        installObservers()
    }

    private func chooseSubtitle(_ stream: Stream) {
        guard !stream.subs.isEmpty else {
            selectSubtitle(nil, remember: false)
            return
        }
        let wanted = item.flatMap { Memory.subtitleLanguage(for: $0.title.key) }
        if wanted == "off" {
            subsEnabled = false
            selectSubtitle(nil, remember: false)
            return
        }
        subsEnabled = true
        let index = wanted.flatMap { label in stream.subs.firstIndex { $0.label == label } } ?? 0
        selectSubtitle(index, remember: false)
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
        var target: Marker?
        for which in [Marker.intro, .outro] where !passed.contains(which) {
            guard let span = marker(which), span.count == 2 else { continue }
            if seconds >= span[1] {
                passed.insert(which)
            } else if seconds >= span[0] && seconds < span[1] - 1 {
                target = which
            }
        }
        if target != skipTarget { skipTarget = target }
        if Date().timeIntervalSince(lastReport) >= 10 {
            report(seconds)
        }
        refreshStats()
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

enum Marker: Hashable {
    case intro, outro
}

struct ThumbCue {
    let start: Double
    let end: Double
    let url: URL
    let rect: CGRect

    static func parse(_ text: String, base: URL, link: (String) -> URL?) -> [ThumbCue] {
        var cues: [ThumbCue] = []
        var pending: (Double, Double)?
        for raw in text.replacingOccurrences(of: "\r", with: "").split(separator: "\n", omittingEmptySubsequences: true) {
            let line = raw.trimmingCharacters(in: .whitespaces)
            if line.contains("-->") {
                let parts = line.components(separatedBy: "-->")
                if parts.count == 2, let a = clock(parts[0]), let b = clock(parts[1]) { pending = (a, b) }
                continue
            }
            guard let span = pending, !line.hasPrefix("WEBVTT"), !line.hasPrefix("NOTE") else { continue }
            pending = nil
            let pieces = line.components(separatedBy: "#xywh=")
            guard pieces.count == 2 else { continue }
            let box = pieces[1].split(separator: ",").compactMap { Double($0) }
            guard box.count == 4, let absolute = URL(string: pieces[0], relativeTo: base)?.absoluteURL, let url = link(absolute.absoluteString) else { continue }
            cues.append(ThumbCue(start: span.0, end: span.1, url: url, rect: CGRect(x: box[0], y: box[1], width: box[2], height: box[3])))
        }
        return cues
    }

    private static func clock(_ raw: String) -> Double? {
        let parts = raw.trimmingCharacters(in: .whitespaces).split(separator: ":").compactMap { Double($0) }
        guard !parts.isEmpty, parts.count <= 3 else { return nil }
        return parts.reduce(0) { $0 * 60 + $1 }
    }
}

actor SpriteStore {
    static let shared = SpriteStore()

    private var images: [URL: UIImage] = [:]
    private var loading: [URL: Task<UIImage?, Never>] = [:]

    func image(for url: URL) async -> UIImage? {
        if let hit = images[url] { return hit }
        if let task = loading[url] { return await task.value }
        let task = Task<UIImage?, Never> {
            guard let reply = try? await URLSession.shared.data(from: url) else { return nil }
            return UIImage(data: reply.0)
        }
        loading[url] = task
        let image = await task.value
        loading[url] = nil
        if let image {
            if images.count > 24 { images.removeAll() }
            images[url] = image
        }
        return image
    }
}
