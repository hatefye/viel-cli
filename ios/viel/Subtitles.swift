import Foundation

struct Cue {
    let start: Double
    let end: Double
    let text: String
}

enum Subtitles {
    static func parse(_ raw: String) -> [Cue] {
        let text = raw.replacingOccurrences(of: "\r\n", with: "\n").replacingOccurrences(of: "\r", with: "\n")
        if text.hasPrefix("[Script Info]") || text.contains("\n[Events]") {
            return parseAss(text)
        }
        return parseBlocks(text)
    }

    private static func parseBlocks(_ text: String) -> [Cue] {
        var cues: [Cue] = []
        for block in text.components(separatedBy: "\n\n") {
            let lines = block.split(separator: "\n", omittingEmptySubsequences: true).map(String.init)
            guard let at = lines.firstIndex(where: { $0.contains("-->") }) else { continue }
            let times = lines[at].components(separatedBy: "-->")
            guard times.count == 2, let start = time(times[0]), let end = time(times[1]) else { continue }
            let body = strip(lines[(at + 1)...].joined(separator: "\n"))
            if !body.isEmpty {
                cues.append(Cue(start: start, end: end, text: body))
            }
        }
        return cues.sorted { $0.start < $1.start }
    }

    private static func parseAss(_ text: String) -> [Cue] {
        var format: [String] = []
        var cues: [Cue] = []
        for line in text.split(separator: "\n").map(String.init) {
            if line.hasPrefix("Format:") {
                format = line.dropFirst(7).split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }
            } else if line.hasPrefix("Dialogue:") {
                let fields = line.dropFirst(9)
                    .split(separator: ",", maxSplits: max(format.count - 1, 9), omittingEmptySubsequences: false)
                    .map { String($0).trimmingCharacters(in: .whitespaces) }
                guard let si = format.firstIndex(of: "Start"), let ei = format.firstIndex(of: "End"),
                      let ti = format.firstIndex(of: "Text"), fields.count > ti,
                      let start = time(fields[si]), let end = time(fields[ei])
                else { continue }
                let body = strip(fields[ti...].joined(separator: ","))
                if !body.isEmpty {
                    cues.append(Cue(start: start, end: end, text: body))
                }
            }
        }
        return cues.sorted { $0.start < $1.start }
    }

    private static func time(_ raw: String) -> Double? {
        let token = raw.trimmingCharacters(in: .whitespaces).split(separator: " ").first.map(String.init) ?? ""
        let parts = token.replacingOccurrences(of: ",", with: ".").split(separator: ":").map(String.init)
        guard !parts.isEmpty, parts.count <= 3 else { return nil }
        var total = 0.0
        for part in parts {
            guard let value = Double(part) else { return nil }
            total = total * 60 + value
        }
        return total
    }

    private static func strip(_ text: String) -> String {
        var out = text.replacingOccurrences(of: "<[^>]+>", with: "", options: .regularExpression)
        out = out.replacingOccurrences(of: "\\{[^}]*\\}", with: "", options: .regularExpression)
        out = out.replacingOccurrences(of: "\\N", with: "\n")
            .replacingOccurrences(of: "\\n", with: "\n")
            .replacingOccurrences(of: "\\h", with: " ")
        out = out.replacingOccurrences(of: "&amp;", with: "&")
            .replacingOccurrences(of: "&lt;", with: "<")
            .replacingOccurrences(of: "&gt;", with: ">")
        return out.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    static func cue(in cues: [Cue], at position: Double) -> String? {
        cues.first { position >= $0.start && position <= $0.end }?.text
    }
}
