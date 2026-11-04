import SwiftUI

enum Theme {
    static let bg = Color(hex: 0x0b0c10)
    static let bg2 = Color(hex: 0x111318)
    static let surface = Color(hex: 0x141418)
    static let soft = Color.white.opacity(0.86)
    static let muted = Color.white.opacity(0.56)
    static let faint = Color.white.opacity(0.34)
    static let border = Color.white.opacity(0.08)
    static let borderStrong = Color.white.opacity(0.16)
    static let glass = Color(hex: 0x16171c).opacity(0.62)
    static let accent = Color(hex: 0xdc2626)
    static let accentHi = Color(hex: 0xf87171)
    static let match = Color(hex: 0x22c55e)
    static let star = Color(hex: 0xe8c15a)
}

extension Color {
    init(hex: UInt32) {
        self.init(
            red: Double((hex >> 16) & 0xff) / 255,
            green: Double((hex >> 8) & 0xff) / 255,
            blue: Double(hex & 0xff) / 255
        )
    }
}

struct WhitePill: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 17, weight: .semibold))
            .foregroundStyle(.black)
            .padding(.horizontal, 26)
            .padding(.vertical, 13)
            .background(Color.white, in: Capsule())
            .opacity(configuration.isPressed ? 0.75 : 1)
    }
}

struct GlassPill: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 17, weight: .semibold))
            .foregroundStyle(.white)
            .padding(.horizontal, 22)
            .padding(.vertical, 13)
            .background(Theme.glass, in: Capsule())
            .overlay(Capsule().stroke(Theme.borderStrong, lineWidth: 1))
            .opacity(configuration.isPressed ? 0.75 : 1)
    }
}

struct GlassCircle: ButtonStyle {
    var size: CGFloat = 44

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: size * 0.42, weight: .medium))
            .foregroundStyle(.white)
            .frame(width: size, height: size)
            .background(Theme.glass, in: Circle())
            .overlay(Circle().stroke(Theme.borderStrong, lineWidth: 1))
            .opacity(configuration.isPressed ? 0.75 : 1)
    }
}

struct Chip: View {
    let text: String

    init(_ text: String) {
        self.text = text
    }

    var body: some View {
        Text(text)
            .font(.system(size: 14, weight: .medium))
            .foregroundStyle(Theme.soft)
            .padding(.horizontal, 11)
            .padding(.vertical, 6)
            .overlay(RoundedRectangle(cornerRadius: 9, style: .continuous).stroke(Theme.borderStrong, lineWidth: 1))
    }
}

struct Tag: View {
    let text: String
    var fill: Color = Color.white.opacity(0.12)

    var body: some View {
        Text(text)
            .font(.system(size: 13, weight: .semibold))
            .foregroundStyle(Theme.soft)
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .background(fill, in: Capsule())
    }
}

struct FlowLayout: Layout {
    var spacing: CGFloat = 8

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let width = proposal.width ?? 0
        var x: CGFloat = 0
        var y: CGFloat = 0
        var rowHeight: CGFloat = 0
        for view in subviews {
            let size = view.sizeThatFits(.unspecified)
            if x + size.width > width, x > 0 {
                x = 0
                y += rowHeight + spacing
                rowHeight = 0
            }
            x += size.width + spacing
            rowHeight = max(rowHeight, size.height)
        }
        return CGSize(width: width, height: y + rowHeight)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var x = bounds.minX
        var y = bounds.minY
        var rowHeight: CGFloat = 0
        for view in subviews {
            let size = view.sizeThatFits(.unspecified)
            if x + size.width > bounds.maxX, x > bounds.minX {
                x = bounds.minX
                y += rowHeight + spacing
                rowHeight = 0
            }
            view.place(at: CGPoint(x: x, y: y), proposal: ProposedViewSize(size))
            x += size.width + spacing
            rowHeight = max(rowHeight, size.height)
        }
    }
}

let movieGenres: [(String, Int)] = [
    ("Action", 28), ("Adventure", 12), ("Animation", 16), ("Comedy", 35), ("Crime", 80),
    ("Documentary", 99), ("Drama", 18), ("Family", 10751), ("Fantasy", 14), ("Horror", 27),
    ("Mystery", 9648), ("Romance", 10749), ("Sci-Fi", 878), ("Thriller", 53), ("War", 10752), ("Western", 37),
]
