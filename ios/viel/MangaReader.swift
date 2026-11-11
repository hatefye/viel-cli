import SwiftUI
import UIKit

struct MangaReader: View {
    let start: String

    @EnvironmentObject private var core: Core
    @Environment(\.dismiss) private var dismiss
    @AppStorage("manga.paged") private var paged = false
    @AppStorage("manga.rtl") private var rightToLeft = true
    @State private var chapterID: String
    @State private var chapter: MangaPages?
    @State private var failed = false
    @State private var page = 0
    @State private var anchor: Int?
    @State private var settling = false
    @State private var chrome = true

    init(start: String) {
        self.start = start
        _chapterID = State(initialValue: start)
    }

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()
            if let chapter {
                if paged {
                    pages(chapter)
                } else {
                    strip(chapter)
                }
            } else if failed {
                VStack(spacing: 12) {
                    Text("This chapter could not be loaded.").foregroundStyle(Theme.muted)
                    Button("Try again") { Task { await load() } }
                }
                .font(.system(size: 15))
            } else {
                ProgressView().tint(.white)
            }
            if chrome || chapter == nil {
                controls.transition(.opacity)
            }
        }
        .statusBarHidden(!chrome && chapter != nil)
        .persistentSystemOverlays(.hidden)
        .preferredColorScheme(.dark)
        .task(id: chapterID) { await load() }
        .onChange(of: anchor) { _, value in
            guard !paged, !settling, let value, let chapter else { return }
            page = min(value, chapter.pages.count - 1)
        }
        .onChange(of: page) { _, value in
            guard let chapter, value < chapter.pages.count else { return }
            MangaStore.shared.record(chapter, page: value)
        }
        .onChange(of: paged) { _, on in
            if !on { Task { await settle(page) } }
        }
        .onDisappear { MangaStore.shared.flush() }
    }

    private func strip(_ chapter: MangaPages) -> some View {
        ScrollView(showsIndicators: false) {
            LazyVStack(spacing: 0) {
                ForEach(Array(chapter.pages.enumerated()), id: \.offset) { index, link in
                    MangaStripPage(link: link, host: core.settings.host).id(index)
                }
                ending(chapter).frame(height: 420).id(chapter.pages.count)
            }
            .scrollTargetLayout()
        }
        .scrollPosition(id: $anchor, anchor: .top)
        .ignoresSafeArea()
        .onTapGesture { toggle() }
        .id(chapter.id)
    }

    private func pages(_ chapter: MangaPages) -> some View {
        TabView(selection: $page) {
            ForEach(Array(chapter.pages.enumerated()), id: \.offset) { index, link in
                MangaZoomPage(link: link, host: core.settings.host, onTap: toggle)
                    .environment(\.layoutDirection, .leftToRight)
                    .tag(index)
            }
            ending(chapter)
                .environment(\.layoutDirection, .leftToRight)
                .tag(chapter.pages.count)
        }
        .tabViewStyle(.page(indexDisplayMode: .never))
        .environment(\.layoutDirection, rightToLeft ? .rightToLeft : .leftToRight)
        .ignoresSafeArea()
        .id(chapter.id)
    }

    private func ending(_ chapter: MangaPages) -> some View {
        VStack(spacing: 16) {
            Text("End of \(MangaChapter.shorten(chapter.name))")
                .font(.system(size: 17, weight: .semibold))
            if let next = chapter.next {
                Button {
                    Haptic.tap()
                    chapterID = next
                } label: {
                    Text("Next chapter")
                        .font(.system(size: 16, weight: .semibold))
                        .foregroundStyle(.black)
                        .padding(.horizontal, 28)
                        .padding(.vertical, 12)
                        .background(.white, in: Capsule())
                }
                .buttonStyle(.plain)
            } else {
                Text("You're caught up.").font(.system(size: 15)).foregroundStyle(Theme.muted)
            }
            Button("Close") { dismiss() }
                .font(.system(size: 15))
                .foregroundStyle(Theme.muted)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var controls: some View {
        VStack {
            HStack(spacing: 12) {
                Button { dismiss() } label: { RoundIcon(icon: "xmark", size: 40) }
                    .buttonStyle(.glass)
                    .buttonBorderShape(.circle)
                VStack(alignment: .leading, spacing: 1) {
                    Text(chapter?.title.title ?? "").font(.system(size: 15, weight: .semibold)).lineLimit(1)
                    Text(chapter?.name ?? "").font(.system(size: 13)).foregroundStyle(Theme.soft).lineLimit(1)
                }
                .shadow(color: .black.opacity(0.8), radius: 6)
                Spacer()
                Menu {
                    Picker("Layout", selection: $paged) {
                        Label("Long strip", systemImage: "rectangle.split.1x2").tag(false)
                        Label("Pages", systemImage: "book.pages").tag(true)
                    }
                    if paged {
                        Toggle("Right to left", isOn: $rightToLeft)
                    }
                } label: {
                    RoundIcon(icon: "slider.horizontal.3", size: 40)
                }
                .buttonStyle(.glass)
                .buttonBorderShape(.circle)
            }
            .padding(.horizontal, 14)
            .padding(.top, 6)
            Spacer()
            if let chapter {
                HStack(spacing: 18) {
                    Button { chapterID = chapter.prev ?? chapterID } label: {
                        Image(systemName: "backward.end.fill").font(.system(size: 15)).frame(width: 36, height: 36)
                    }
                    .disabled(chapter.prev == nil)
                    Text("\(min(page + 1, chapter.pages.count)) / \(chapter.pages.count)")
                        .font(.system(size: 14, weight: .medium))
                        .monospacedDigit()
                        .frame(minWidth: 76)
                    Button { chapterID = chapter.next ?? chapterID } label: {
                        Image(systemName: "forward.end.fill").font(.system(size: 15)).frame(width: 36, height: 36)
                    }
                    .disabled(chapter.next == nil)
                }
                .foregroundStyle(.white)
                .padding(.horizontal, 12)
                .padding(.vertical, 4)
                .glassEffect(.regular, in: .capsule)
                .padding(.bottom, 10)
            }
        }
    }

    private func toggle() {
        withAnimation(.easeOut(duration: 0.18)) { chrome.toggle() }
    }

    private func settle(_ index: Int) async {
        settling = true
        anchor = index
        try? await Task.sleep(nanoseconds: 350_000_000)
        anchor = index
        try? await Task.sleep(nanoseconds: 150_000_000)
        settling = false
    }

    private func load() async {
        failed = false
        chapter = nil
        do {
            let loaded: MangaPages = try await core.get("manga/chapter", ["id": chapterID])
            guard !Task.isCancelled else { return }
            let saved = MangaStore.shared.place(in: loaded.id)
            let begin = saved < loaded.pages.count - 1 ? saved : 0
            settling = true
            page = begin
            chapter = loaded
            MangaStore.shared.record(loaded, page: begin)
            await settle(begin)
        } catch {
            guard !Task.isCancelled else { return }
            failed = true
            core.log("manga", "chapter \(chapterID) failed: \(error.localizedDescription)")
        }
    }
}

struct MangaStripPage: View {
    let link: String
    let host: String

    @State private var image: UIImage?
    @State private var ratio: CGFloat = 0.69
    @State private var failed = false
    @State private var attempt = 0

    var body: some View {
        ZStack {
            if let image {
                Image(uiImage: image).resizable().scaledToFit()
            } else {
                Color.white.opacity(0.03)
                if failed {
                    Button("Tap to retry") {
                        failed = false
                        attempt += 1
                    }
                    .font(.system(size: 14))
                    .foregroundStyle(Theme.muted)
                } else {
                    ProgressView().tint(.white)
                }
            }
        }
        .frame(maxWidth: .infinity)
        .aspectRatio(ratio, contentMode: .fit)
        .task(id: attempt) {
            guard image == nil else { return }
            if let loaded = await MangaImages.load(link, host: host) {
                ratio = loaded.size.width / max(loaded.size.height, 1)
                image = loaded
            } else if !Task.isCancelled {
                failed = true
            }
        }
        .onDisappear { image = nil }
    }
}

struct MangaZoomPage: View {
    let link: String
    let host: String
    let onTap: () -> Void

    @State private var image: UIImage?
    @State private var failed = false
    @State private var attempt = 0

    var body: some View {
        ZStack {
            if let image {
                ZoomSurface(image: image, onTap: onTap)
            } else if failed {
                Button("Tap to retry") {
                    failed = false
                    attempt += 1
                }
                .font(.system(size: 14))
                .foregroundStyle(Theme.muted)
            } else {
                ProgressView().tint(.white)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .task(id: attempt) {
            guard image == nil else { return }
            if let loaded = await MangaImages.load(link, host: host) {
                image = loaded
            } else if !Task.isCancelled {
                failed = true
            }
        }
        .onDisappear { image = nil }
    }
}

struct ZoomSurface: UIViewRepresentable {
    let image: UIImage
    let onTap: () -> Void

    func makeCoordinator() -> Coordinator {
        Coordinator(onTap: onTap)
    }

    func makeUIView(context: Context) -> UIScrollView {
        let scroll = UIScrollView()
        scroll.delegate = context.coordinator
        scroll.minimumZoomScale = 1
        scroll.maximumZoomScale = 4
        scroll.showsVerticalScrollIndicator = false
        scroll.showsHorizontalScrollIndicator = false
        scroll.contentInsetAdjustmentBehavior = .never
        scroll.backgroundColor = .black
        let picture = context.coordinator.picture
        picture.contentMode = .scaleAspectFit
        picture.frame = scroll.bounds
        picture.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        scroll.addSubview(picture)
        let double = UITapGestureRecognizer(target: context.coordinator, action: #selector(Coordinator.zoom(_:)))
        double.numberOfTapsRequired = 2
        let single = UITapGestureRecognizer(target: context.coordinator, action: #selector(Coordinator.tap))
        single.require(toFail: double)
        scroll.addGestureRecognizer(double)
        scroll.addGestureRecognizer(single)
        return scroll
    }

    func updateUIView(_ scroll: UIScrollView, context: Context) {
        context.coordinator.onTap = onTap
        context.coordinator.picture.image = image
    }

    final class Coordinator: NSObject, UIScrollViewDelegate {
        let picture = UIImageView()
        var onTap: () -> Void

        init(onTap: @escaping () -> Void) {
            self.onTap = onTap
        }

        func viewForZooming(in scrollView: UIScrollView) -> UIView? {
            picture
        }

        @objc func tap() {
            onTap()
        }

        @objc func zoom(_ gesture: UITapGestureRecognizer) {
            guard let scroll = gesture.view as? UIScrollView else { return }
            if scroll.zoomScale > 1 {
                scroll.setZoomScale(1, animated: true)
            } else {
                let point = gesture.location(in: picture)
                let size = CGSize(width: scroll.bounds.width / 2.5, height: scroll.bounds.height / 2.5)
                scroll.zoom(to: CGRect(x: point.x - size.width / 2, y: point.y - size.height / 2, width: size.width, height: size.height), animated: true)
            }
        }
    }
}
