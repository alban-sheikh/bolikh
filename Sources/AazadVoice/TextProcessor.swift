import Foundation

/// Turns raw model output into the text that gets inserted.
enum TextProcessor {
    static func process(_ raw: String, settings: AppSettings) -> String {
        var text = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return "" }

        // User dictionary: "aazad" → "Aazad", "my email" → "me@example.com", …
        for r in settings.replacements where !r.from.trimmingCharacters(in: .whitespaces).isEmpty {
            let pattern = "\\b" + NSRegularExpression.escapedPattern(for: r.from) + "\\b"
            text = text.replacingOccurrences(of: pattern, with: NSRegularExpression.escapedTemplate(for: r.to),
                                              options: [.regularExpression, .caseInsensitive])
        }

        if settings.voiceCommands {
            text = applyVoiceCommands(text)
        }

        text = text.trimmingCharacters(in: .whitespaces)
        if settings.addTrailingSpace, let last = text.last, !last.isNewline {
            text += " "
        }
        return text
    }

    /// Spoken formatting commands. The model adds its own punctuation around them,
    /// so we drop the punctuation it adds after the command (e.g. "Hello. New line. World" → "Hello.\nWorld").
    private static let commands: [(String, String)] = [
        ("new paragraph", "\n\n"),
        ("new line", "\n"),
        ("next line", "\n"),
    ]

    private static func applyVoiceCommands(_ input: String) -> String {
        var text = input
        for (phrase, output) in commands {
            let pattern = "\\s*\\b" + phrase.replacingOccurrences(of: " ", with: "\\s+") + "\\b[,.;:!?]?\\s*"
            guard let re = try? NSRegularExpression(pattern: pattern, options: .caseInsensitive) else { continue }
            var result = ""
            var cursor = text.startIndex
            for m in re.matches(in: text, range: NSRange(text.startIndex..., in: text)) {
                guard let r = Range(m.range, in: text) else { continue }
                result += text[cursor..<r.lowerBound] + output
                cursor = r.upperBound
            }
            result += text[cursor...]
            text = capitalizeAfterNewlines(result)
        }
        return text
    }

    private static func capitalizeAfterNewlines(_ s: String) -> String {
        var out = ""
        var capitalizeNext = false
        for ch in s {
            if ch.isNewline { capitalizeNext = true; out.append(ch); continue }
            if capitalizeNext && ch.isLetter { out += ch.uppercased(); capitalizeNext = false; continue }
            if !ch.isWhitespace { capitalizeNext = false }
            out.append(ch)
        }
        return out
    }
}
