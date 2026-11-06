import CoreSpotlight
import UniformTypeIdentifiers

enum Spotlight {
    static func index(_ titles: [Title]) {
        let items = titles.map { title -> CSSearchableItem in
            let attributes = CSSearchableItemAttributeSet(contentType: title.isMovie ? .movie : .video)
            attributes.title = title.name
            attributes.contentDescription = title.overview ?? title.kindLabel
            attributes.keywords = ["viel", title.kindLabel]
            return CSSearchableItem(uniqueIdentifier: "viel://\(title.kind)/\(title.id)", domainIdentifier: "tv.viel.titles", attributeSet: attributes)
        }
        CSSearchableIndex.default().indexSearchableItems(items) { _ in }
    }
}
