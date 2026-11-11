//! Turns raw model output into the text that gets inserted.
use crate::settings::Settings;
use once_cell::sync::Lazy;
use regex::{NoExpand, Regex};

/// Marks "a sentence started here" so a word after a removed filler gets re-capitalised.
const MARKER: char = '\u{1}';

pub fn process(raw: &str, s: &Settings) -> String {
    let mut text = raw.trim().to_string();
    if text.is_empty() {
        return text;
    }
    if s.remove_fillers {
        text = remove_fillers(&text);
        if text.is_empty() {
            return text;
        }
    }
    for r in &s.replacements {
        let from = r.from.trim();
        if from.is_empty() {
            continue;
        }
        if let Ok(re) = Regex::new(&format!(r"(?i)\b{}\b", regex::escape(from))) {
            text = re.replace_all(&text, NoExpand(&r.to)).into_owned();
        }
    }
    if s.voice_commands {
        text = apply_voice_commands(&text);
    }
    let mut text = text.trim_matches([' ', '\t']).to_string();
    if s.add_trailing_space && text.chars().last().is_some_and(|c| c != '\n') {
        text.push(' ');
    }
    text
}

/// Hesitation sounds the model faithfully transcribes ("um", "uh", "hmm"…).
/// Deliberately excludes "er", "ah", "eh": real words in German and other supported languages.
static FILLER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(u+h*m+|u+h+|h+m+|m{2,}|e+r+m+)\b([,.…]*)").unwrap());

fn is_word_joiner(c: char) -> bool {
    c.is_alphabetic() || matches!(c, '\'' | '’' | '-')
}

pub fn remove_fillers(input: &str) -> String {
    let mut text = String::new();
    let mut cursor = 0;
    for caps in FILLER.captures_iter(input) {
        let whole = caps.get(0).unwrap();
        let word = caps.get(1).unwrap();
        // Skip parts of hyphenated/apostrophised words ("Mm-hmm", "hmm's").
        let before = input[..word.start()].chars().last();
        let after = input[word.end()..].chars().next();
        if before.is_some_and(is_word_joiner) || after.is_some_and(is_word_joiner) {
            continue;
        }
        text.push_str(&input[cursor..whole.start()]);
        let prior = text.trim_end_matches([' ', '\t']).to_string();
        let sentence_start = prior.is_empty()
            || prior.ends_with(MARKER)
            || prior.ends_with(['.', '!', '?', '\n']);
        if sentence_start {
            text.push(MARKER);
        } else if prior.ends_with(',') && whole.as_str().ends_with(',') {
            // "I was, uh, thinking" → "I was thinking": the commas only marked the pause.
            text = prior[..prior.len() - 1].to_string();
            text.push(' ');
        }
        cursor = whole.end();
    }
    text.push_str(&input[cursor..]);

    static CLEANUPS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
        [
            (r"([,;:])\s*(?:[,;:]\s*)+", "$1 "),
            (r"[ \t]{2,}", " "),
            (r"[ \t]+([,.;:!?])", "$1"),
            ("\u{1}[\\s,;:.]+", "\u{1}"),
            (r"^[\s,;:.]+", ""),
            (r"([.!?])[,;:]+", "$1"),
            (r"[,;:]+([.!?])", "$1"),
        ]
        .into_iter()
        .map(|(p, t)| (Regex::new(p).unwrap(), t))
        .collect()
    });
    for (re, rep) in CLEANUPS.iter() {
        text = re.replace_all(&text, *rep).into_owned();
    }

    let mut out = String::with_capacity(text.len());
    let mut capitalize_next = false;
    for ch in text.chars() {
        if ch == MARKER {
            capitalize_next = true;
            continue;
        }
        if capitalize_next && ch.is_alphabetic() {
            out.extend(ch.to_uppercase());
            capitalize_next = false;
            continue;
        }
        if !ch.is_whitespace() {
            capitalize_next = false;
        }
        out.push(ch);
    }
    static SPACES: Lazy<Regex> = Lazy::new(|| Regex::new(r"[ \t]{2,}").unwrap());
    let out = SPACES.replace_all(&out, " ").trim_matches([' ', '\t']).to_string();
    if out.chars().all(|c| ".,!?;:… ".contains(c)) {
        String::new()
    } else {
        out
    }
}

/// Spoken formatting commands. Punctuation before the command is kept; punctuation
/// the model adds after it is dropped ("Hello. New line. World" → "Hello.\nWorld").
static COMMANDS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    [("new paragraph", "\n\n"), ("new line", "\n"), ("next line", "\n")]
        .into_iter()
        .map(|(phrase, out)| {
            let p = format!(r"(?i)\s*\b{}\b[,.;:!?]?\s*", phrase.replace(' ', r"\s+"));
            (Regex::new(&p).unwrap(), out)
        })
        .collect()
});

fn apply_voice_commands(input: &str) -> String {
    let mut text = input.to_string();
    for (re, out) in COMMANDS.iter() {
        text = re.replace_all(&text, NoExpand(out)).into_owned();
    }
    // Capitalise the first letter after a line break.
    let mut result = String::with_capacity(text.len());
    let mut capitalize_next = false;
    for ch in text.chars() {
        if ch == '\n' {
            capitalize_next = true;
            result.push(ch);
            continue;
        }
        if capitalize_next && ch.is_alphabetic() {
            result.extend(ch.to_uppercase());
            capitalize_next = false;
            continue;
        }
        if !ch.is_whitespace() {
            capitalize_next = false;
        }
        result.push(ch);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Replacement;

    fn settings() -> Settings {
        Settings { add_trailing_space: false, ..Default::default() }
    }

    #[test]
    fn removes_fillers() {
        let s = settings();
        assert_eq!(process("Um, so I think, uh, we should ship it.", &s), "So I think we should ship it.");
        assert_eq!(
            process("Hmm. Okay, umm, let's go. Uh, what about iPhone users?", &s),
            "Okay let's go. What about iPhone users?"
        );
        assert_eq!(
            process("I was, uh, thinking about the, um, the summit.", &s),
            "I was thinking about the the summit."
        );
        assert_eq!(process("Uh.", &s), "");
    }

    #[test]
    fn keeps_real_words() {
        let s = settings();
        let t = "Er ist hier, und ah, das Hummus mit Umami ist gut. Mm-hmm, yes.";
        assert_eq!(process(t, &s), t);
    }

    #[test]
    fn voice_commands() {
        let s = settings();
        assert_eq!(process("Hello. New line. It runs offline.", &s), "Hello.\nIt runs offline.");
        assert_eq!(process("Dear team, new paragraph. Thanks.", &s), "Dear team,\n\nThanks.");
        assert_eq!(process("Hello, uh, new line. Um, second line.", &s), "Hello\nSecond line.");
    }

    #[test]
    fn replacements_and_spacing() {
        let mut s = settings();
        s.replacements = vec![Replacement { from: "azad".into(), to: "Aazad".into() }];
        s.add_trailing_space = true;
        assert_eq!(process("Testing azad voice.", &s), "Testing Aazad voice. ");
        assert_eq!(process("One new line", &s), "One\n");
    }
}
