import Foundation

/// Turns raw model output into the text that gets inserted.
enum TextProcessor {
    static func process(_ raw: String, settings: AppSettings) -> String {
        var text = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return "" }

        if settings.removeFillers {
            text = removeFillers(text)
            guard !text.isEmpty else { return "" }
        }

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

    /// Hesitation sounds the model faithfully transcribes ("um", "uh", "hmm"…).
    /// Deliberately excludes "er", "ah", "eh" — real words in German and other supported languages.
    private static let fillerPattern = try! NSRegularExpression(
        pattern: "(?<![\\p{L}'’-])(?:u+h*m+|u+h+|h+m+|m{2,}|e+r+m+)(?![\\p{L}'’-])[,.…]*",
        options: .caseInsensitive)

    static func removeFillers(_ input: String) -> String {
        let marker = "\u{1}"   // marks "a sentence started here" so we can re-capitalise
        var text = ""
        var cursor = input.startIndex
        for m in fillerPattern.matches(in: input, range: NSRange(input.startIndex..., in: input)) {
            guard let r = Range(m.range, in: input) else { continue }
            let before = input[cursor..<r.lowerBound]
            text += before
            let prior = (text.isEmpty ? "" : text).trimmingCharacters(in: .whitespaces)
            let sentenceStart = prior.isEmpty || prior.hasSuffix(marker) || ".!?\n".contains(prior.last!)
            if sentenceStart {
                text += marker
            } else if prior.hasSuffix(","), input[r].hasSuffix(",") {
                // "I was, uh, thinking" → "I was thinking": the commas only marked the pause.
                text = String(prior.dropLast()) + " "
            }
            cursor = r.upperBound
        }
        text += input[cursor...]

        // Tidy what's left: ", ," → ",", leading punctuation, double spaces, space before punctuation.
        let cleanups: [(String, String)] = [
            ("([,;:])\\s*(?:[,;:]\\s*)+", "$1 "),
            ("[ \\t]{2,}", " "),
            ("[ \\t]+([,.;:!?])", "$1"),
            ("\u{1}[\\s,;:.]+", "\u{1}"),
            ("^[\\s,;:.]+", ""),
            ("([.!?])[,;:]+", "$1"),
            ("[,;:]+([.!?])", "$1"),
        ]
        for (p, t) in cleanups {
            text = text.replacingOccurrences(of: p, with: t, options: .regularExpression)
        }

        // Capitalise the first letter after each marker, then drop the markers.
        var out = ""
        var capitalizeNext = false
        for ch in text {
            if String(ch) == marker { capitalizeNext = true; continue }
            if capitalizeNext && ch.isLetter { out += ch.uppercased(); capitalizeNext = false; continue }
            if !ch.isWhitespace { capitalizeNext = false }
            out.append(ch)
        }
        out = out.replacingOccurrences(of: "[ \\t]{2,}", with: " ", options: .regularExpression)
            .trimmingCharacters(in: .whitespaces)
        return out.allSatisfy({ ".,!?;:… ".contains($0) }) ? "" : out
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
