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
    if s.numbers_as_digits {
        text = convert_numbers(&text);
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

/// One spoken English number word.
#[derive(Clone, Copy, PartialEq)]
enum Tok {
    /// zero to nine
    Unit(u64),
    /// ten to nineteen
    Teen(u64),
    /// twenty, thirty … ninety
    Tens(u64),
    Hundred,
    /// thousand, million, billion
    Scale(u64),
}

fn tok(word: &str) -> Option<Tok> {
    const UNITS: [&str; 10] = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine"];
    const TEENS: [&str; 10] = [
        "ten", "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen", "nineteen",
    ];
    const TENS: [&str; 8] = ["twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"];
    if let Some(i) = UNITS.iter().position(|w| *w == word) {
        return Some(Tok::Unit(i as u64));
    }
    if let Some(i) = TEENS.iter().position(|w| *w == word) {
        return Some(Tok::Teen(10 + i as u64));
    }
    if let Some(i) = TENS.iter().position(|w| *w == word) {
        return Some(Tok::Tens(20 + 10 * i as u64));
    }
    match word {
        "hundred" => Some(Tok::Hundred),
        "thousand" => Some(Tok::Scale(1_000)),
        "million" => Some(Tok::Scale(1_000_000)),
        "billion" => Some(Tok::Scale(1_000_000_000)),
        _ => None,
    }
}

/// One well-formed number such as "two thousand twenty six". Words that don't fit
/// start a new chunk, and chunks are written side by side: "four eight" → 48,
/// "nineteen ninety nine" → 1999.
struct Chunk {
    total: u64,
    group: u64,
    hundred: bool,
    /// 0: tens and units still open, 1: tens said, 2: units said
    low: u8,
    last_scale: u64,
    zero: bool,
    started: bool,
}

impl Chunk {
    fn new() -> Self {
        Self { total: 0, group: 0, hundred: false, low: 0, last_scale: u64::MAX, zero: false, started: false }
    }

    /// Adds the word if it continues this number. Leaves the chunk untouched if not.
    fn accept(&mut self, t: Tok) -> bool {
        if self.zero {
            return false;
        }
        match t {
            Tok::Unit(0) => {
                if self.started {
                    return false;
                }
                self.zero = true;
            }
            Tok::Unit(n) => {
                if self.low == 2 {
                    return false;
                }
                self.group += n;
                self.low = 2;
            }
            Tok::Teen(n) | Tok::Tens(n) => {
                if self.low != 0 {
                    return false;
                }
                self.group += n;
                self.low = if matches!(t, Tok::Tens(_)) { 1 } else { 2 };
            }
            Tok::Hundred => {
                if self.hundred || self.low == 0 || self.group == 0 {
                    return false;
                }
                self.group *= 100;
                self.hundred = true;
                self.low = 0;
            }
            Tok::Scale(s) => {
                if self.group == 0 || s >= self.last_scale {
                    return false;
                }
                self.total += self.group * s;
                self.group = 0;
                self.hundred = false;
                self.low = 0;
                self.last_scale = s;
            }
        }
        self.started = true;
        true
    }

    fn digits(&self) -> String {
        (self.total + self.group).to_string()
    }
}

/// Writes spoken English numbers as digits: "forty eight" → 48, "three point five" → 3.5,
/// "fifty percent" → 50%. A lone "zero" to "ten" stays a word ("one of them"), and so do
/// ordinals ("twenty first") and "a hundred".
pub fn convert_numbers(input: &str) -> String {
    static WORD: Lazy<Regex> = Lazy::new(|| Regex::new(r"[A-Za-z]+").unwrap());
    static ORDINAL: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^(first|second|third|fourth|fifth|sixth|seventh|eighth|ninth)$").unwrap()
    });
    static PERCENT: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)(\d) percent\b").unwrap());

    let joins = |c: char| c.is_alphanumeric() || matches!(c, '\'' | '’');
    // Whole words only: not "one's", not a word glued to a digit or an accented letter.
    let words: Vec<(usize, usize, String)> = WORD
        .find_iter(input)
        .filter(|m| {
            !input[..m.start()].chars().next_back().is_some_and(joins)
                && !input[m.end()..].chars().next().is_some_and(joins)
        })
        .map(|m| (m.start(), m.end(), m.as_str().to_lowercase()))
        .collect();
    let sep = |a: usize, b: usize| &input[words[a].1..words[b].0];
    let word_at = |i: usize| words.get(i).map(|w| w.2.as_str());

    let mut out = String::with_capacity(input.len());
    let mut cursor = 0;
    let mut i = 0;
    while i < words.len() {
        let first = match tok(&words[i].2) {
            // "a hundred people", "thousands": not a number we can write.
            Some(Tok::Hundred) | Some(Tok::Scale(_)) | None => {
                i += 1;
                continue;
            }
            Some(t) => t,
        };
        let mut chunks: Vec<String> = Vec::new();
        let mut chunk = Chunk::new();
        chunk.accept(first);
        let (mut last, mut last_tok, mut count) = (i, first, 1);
        loop {
            let n = last + 1;
            let Some(next) = word_at(n) else { break };
            let s = sep(last, n);
            if let Some(t) = tok(next) {
                let hyphenated =
                    s == "-" && matches!(last_tok, Tok::Tens(_)) && matches!(t, Tok::Unit(v) if v > 0);
                if s != " " && !hyphenated {
                    break;
                }
                if !chunk.accept(t) {
                    let mut fresh = Chunk::new();
                    if !fresh.accept(t) {
                        break;
                    }
                    chunks.push(chunk.digits());
                    chunk = fresh;
                }
                (last, last_tok, count) = (n, t, count + 1);
                continue;
            }
            // "two hundred and five"
            if next == "and" && s == " " && matches!(last_tok, Tok::Hundred | Tok::Scale(_)) {
                if let Some(t) = word_at(n + 1).and_then(tok) {
                    let small = matches!(t, Tok::Unit(v) if v > 0) || matches!(t, Tok::Teen(_) | Tok::Tens(_));
                    if small && sep(n, n + 1) == " " && chunk.accept(t) {
                        (last, last_tok, count) = (n + 1, t, count + 1);
                        continue;
                    }
                }
            }
            break;
        }
        chunks.push(chunk.digits());

        // "twenty first" stays as words.
        if matches!(last_tok, Tok::Tens(_))
            && word_at(last + 1).is_some_and(|w| ORDINAL.is_match(w))
            && matches!(sep(last, last + 1), " " | "-")
        {
            i = last + 2;
            continue;
        }

        let mut text = chunks.concat();
        let mut end = last;
        let mut forced = false;
        // "three point one four" → 3.14
        if word_at(end + 1) == Some("point") && sep(end, end + 1) == " " {
            let mut decimals = String::new();
            let mut d = end + 1;
            while let Some(Tok::Unit(v)) = word_at(d + 1).and_then(tok) {
                if sep(d, d + 1) != " " {
                    break;
                }
                decimals.push_str(&v.to_string());
                d += 1;
            }
            if !decimals.is_empty() {
                text.push('.');
                text.push_str(&decimals);
                end = d;
                forced = true;
            }
        }
        if word_at(end + 1) == Some("percent") && sep(end, end + 1) == " " {
            text.push('%');
            end += 1;
            forced = true;
        }

        let lone_small = count == 1 && matches!(first, Tok::Unit(_) | Tok::Teen(10));
        if lone_small && !forced {
            i = last + 1;
            continue;
        }
        out.push_str(&input[cursor..words[i].0]);
        out.push_str(&text);
        cursor = words[end].1;
        i = end + 1;
    }
    out.push_str(&input[cursor..]);
    PERCENT.replace_all(&out, "$1%").into_owned()
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

    #[test]
    fn numbers_as_digits() {
        let s = settings();
        let cases = [
            ("I am forty eight years old.", "I am 48 years old."),
            ("The code is four eight.", "The code is 48."),
            ("Forty-eight people came.", "48 people came."),
            ("It was two thousand twenty six.", "It was 2026."),
            ("Back in nineteen ninety nine.", "Back in 1999."),
            ("In twenty twenty six we ship.", "In 2026 we ship."),
            ("Pay one hundred and twenty five now.", "Pay 125 now."),
            ("About two million three hundred thousand users.", "About 2300000 users."),
            ("Twenty five hundred rupees.", "2500 rupees."),
            ("Version three point one four is out.", "Version 3.14 is out."),
            ("Growth was five percent, then fifty percent.", "Growth was 5%, then 50%."),
            ("Call nine eight seven six five four three two one zero.", "Call 9876543210."),
            ("A twenty-one-year-old and a forty eight second clip.", "A 21-year-old and a 48 second clip."),
            ("Room one twenty, at three fifty.", "Room 120, at 350."),
            ("Already 50 percent done.", "Already 50% done."),
        ];
        for (said, typed) in cases {
            assert_eq!(process(said, &s), typed);
        }
    }

    #[test]
    fn numbers_left_as_words() {
        let s = settings();
        for t in [
            "One of them is here, and the two of us agree.",
            "I have one, two, or three options.",
            "Give me ten minutes.",
            "A hundred people and thousands more.",
            "On the twenty first and the thirty-second.",
            "It is a one-on-one in two-three days.",
            "No one's point was missed.",
            "Ten slotte, zes en een.",
        ] {
            assert_eq!(process(t, &s), t);
        }
        let mut off = settings();
        off.numbers_as_digits = false;
        assert_eq!(process("I am forty eight.", &off), "I am forty eight.");
    }
}
