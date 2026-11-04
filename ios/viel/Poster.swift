import SwiftUI

struct Poster: View {
    let url: String?
    var width: CGFloat = 56
    var height: CGFloat = 84
    var radius: CGFloat = 6

    var body: some View {
        AsyncImage(url: url.flatMap { URL(string: $0) }) { phase in
            if let image = phase.image {
                image.resizable().scaledToFill()
            } else {
                Rectangle().fill(.quaternary)
            }
        }
        .frame(width: width, height: height)
        .clipShape(RoundedRectangle(cornerRadius: radius, style: .continuous))
    }
}
