import SwiftUI

struct HeroView: View {
    let items: [Title]
    let onPlay: (Title) -> Void
    let onInfo: (Title) -> Void

    @EnvironmentObject private var core: Core
    @State private var index = 0
    @State private var details: [String: Details] = [:]
    @State private var drift = false

    private var slides: [Title] { Array(items.prefix(6)) }
    private var current: Title? { slides.indices.contains(index) ? slides[index] : nil }

    var body: some View {
        ZStack(alignment: .bottom) {
            GeometryReader { geometry in
                let offset = geometry.frame(in: .global).minY
                let stretch = max(offset, 0)
                ZStack {
                    ForEach(Array(slides.enumerated()), id: \.element.key) { position, title in
                        if position == index {
                            Backdrop(url: title.poster ?? title.backdrop, height: 620 + stretch)
                                .scaleEffect(drift ? 1.08 : 1.0, anchor: .top)
                                .transition(.opacity)
                        }
                    }
                }
                .offset(y: -stretch)
                .mask(
                    LinearGradient(
                        stops: [
                            .init(color: .black, location: 0),
                            .init(color: .black, location: 0.44),
                            .init(color: .black.opacity(0.7), location: 0.58),
                            .init(color: .black.opacity(0.16), location: 0.74),
                            .init(color: .clear, location: 0.92),
                        ],
                        startPoint: .top,
                        endPoint: .bottom
                    )
                )
            }
            LinearGradient(
                stops: [
                    .init(color: Theme.bg.opacity(0.5), location: 0),
                    .init(color: .clear, location: 0.14),
                    .init(color: .clear, location: 0.36),
                    .init(color: Theme.bg.opacity(0.5), location: 0.58),
                    .init(color: Theme.bg.opacity(0.68), location: 0.74),
                    .init(color: Theme.bg.opacity(0.68), location: 0.86),
                    .init(color: Theme.bg, location: 1),
                ],
                startPoint: .top,
                endPoint: .bottom
            )
            if let current {
                VStack(spacing: 12) {
                    slide(current, details[current.key])
                    if slides.count > 1 { dots }
                }
                .padding(.bottom, 4)
                .id(current.key)
                .transition(.asymmetric(insertion: .move(edge: .bottom).combined(with: .opacity), removal: .opacity))
            }
        }
        .frame(height: 620)
        .clipped()
        .animation(.easeInOut(duration: 0.9), value: index)
        .onAppear {
            withAnimation(.easeInOut(duration: 44).repeatForever(autoreverses: true)) { drift = true }
        }
        .task(id: index) {
            guard let current, details[current.key] == nil else { return }
            if let loaded: Details = try? await core.get("details", ["kind": current.kind, "id": String(current.id)]) {
                details[current.key] = loaded
            }
        }
        .task(id: slides.count) {
            guard slides.count > 1 else { return }
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: 9_000_000_000)
                guard !Task.isCancelled else { return }
                index = (index + 1) % slides.count
            }
        }
    }

    private func slide(_ title: Title, _ detail: Details?) -> some View {
        VStack(spacing: 12) {
            Logo(url: detail?.logo, name: title.name, maxWidth: 320, size: 42)
                .padding(.horizontal, 24)
            HStack(spacing: 12) {
                if title.rating > 0 {
                    Text("\(Int((title.rating * 10).rounded()))% match")
                        .foregroundStyle(Theme.match)
                        .fontWeight(.semibold)
                }
                if !title.year.isEmpty { Text(title.year) }
                Chip(title.kindLabel)
                if let runtime = Format.runtime(detail?.runtime) { Text(runtime) }
            }
            .font(.system(size: 17))
            .foregroundStyle(Theme.soft)
            if !title.genres.isEmpty {
                Text(title.genres.prefix(3).joined(separator: " · "))
                    .font(.system(size: 16))
                    .foregroundStyle(Theme.soft)
            }
            if let overview = title.overview ?? detail?.overview {
                Text(overview)
                    .font(.system(size: 15))
                    .foregroundStyle(Theme.soft)
                    .multilineTextAlignment(.center)
                    .lineLimit(2)
                    .padding(.horizontal, 22)
            }
            GlassEffectContainer(spacing: 12) {
                HStack(spacing: 12) {
                    Button {
                        Haptic.tap()
                        onPlay(title)
                    } label: {
                        Label("Play", systemImage: "play.fill")
                            .font(.system(size: 17, weight: .semibold))
                            .foregroundStyle(.black)
                            .padding(.horizontal, 10)
                            .padding(.vertical, 4)
                    }
                    .buttonStyle(.glassProminent)
                    .tint(.white)
                    Button {
                        onInfo(title)
                    } label: {
                        Label("More Info", systemImage: "info.circle")
                            .font(.system(size: 17, weight: .semibold))
                            .padding(.horizontal, 6)
                            .padding(.vertical, 4)
                    }
                    .buttonStyle(.glass)
                }
            }
            .padding(.top, 4)
        }
    }

    private var dots: some View {
        HStack(spacing: 6) {
            ForEach(slides.indices, id: \.self) { position in
                Capsule()
                    .fill(Color.white.opacity(position == index ? 1 : 0.4))
                    .frame(width: position == index ? 22 : 8, height: 8)
                    .onTapGesture { index = position }
            }
        }
        .animation(.smooth(duration: 0.3), value: index)
        .padding(.top, 6)
    }
}
