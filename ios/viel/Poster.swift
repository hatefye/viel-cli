import SwiftUI

struct Poster: View {
    let url: String?
    var width: CGFloat = 110
    var height: CGFloat = 165
    var radius: CGFloat = 10

    var body: some View {
        AsyncImage(url: url.flatMap { URL(string: $0) }) { phase in
            if let image = phase.image {
                image.resizable().scaledToFill()
            } else {
                Rectangle().fill(Color.white.opacity(0.06))
            }
        }
        .frame(width: width, height: height)
        .clipShape(RoundedRectangle(cornerRadius: radius, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: radius, style: .continuous).stroke(Theme.border, lineWidth: 1))
    }
}

struct Backdrop: View {
    let url: String?
    var height: CGFloat = 300

    var body: some View {
        GeometryReader { geometry in
            AsyncImage(url: url.flatMap { URL(string: $0) }) { phase in
                if let image = phase.image {
                    image.resizable().scaledToFill()
                } else {
                    Rectangle().fill(Theme.bg2)
                }
            }
            .frame(width: geometry.size.width, height: height)
            .clipped()
        }
        .frame(height: height)
    }
}

struct Logo: View {
    let url: String?
    let name: String
    var maxWidth: CGFloat = 280
    var size: CGFloat = 40

    var body: some View {
        if let url, let link = URL(string: url) {
            AsyncImage(url: link) { phase in
                if let image = phase.image {
                    image.resizable().scaledToFit().frame(maxWidth: maxWidth, maxHeight: size * 2.8)
                } else {
                    nameText
                }
            }
        } else {
            nameText
        }
    }

    private var nameText: some View {
        Text(name)
            .font(.system(size: size, weight: .black))
            .foregroundStyle(.white)
            .multilineTextAlignment(.center)
            .lineLimit(2)
            .minimumScaleFactor(0.6)
            .shadow(color: .black.opacity(0.6), radius: 8, y: 2)
    }
}
