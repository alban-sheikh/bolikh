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
    if s.remove_repeats {
        text = remove_repeats(&text);
    }
    text = remove_words(&text, &s.remove_words);
    if text.is_empty() {
        return text;
    }
    if s.voice_commands {
        text = apply_scratch(&text);
    }
    for r in &s.replacements {
        let from = r.from.trim();
        if from.is_empty() {
            continue;
        }
        if let Ok(re) = Regex::new(&format!(r"(?i)\b{}\b", regex::escape(from))) {
            // Snippets: "\n" and "\t" typed in the dictionary become a line break and a tab.
            let to = r.to.replace("\\n", "\n").replace("\\t", "\t");
            text = re.replace_all(&text, NoExpand(&to)).into_owned();
        }
    }
    if s.voice_commands {
        text = apply_voice_commands(&text, s.command_lead_word);
    }
    text = fix_app_name(&text);
    if s.spoken_symbols {
        text = spoken_addresses(&text);
        text = spoken_symbols(&text);
    }
    if s.numbers_as_digits {
        text = convert_numbers(&text);
    }
    let mut text = text.trim_matches(' ').to_string();
    if s.add_trailing_space && text.chars().last().is_some_and(|c| c != '\n' && c != '\t') {
        text.push(' ');
    }
    text
}

/// True when the whole dictation is just "scratch that": the caller deletes the previous one.
pub fn is_scratch_only(raw: &str) -> bool {
    let said: String = remove_fillers(raw)
        .chars()
        .filter(|c| c.is_alphabetic() || *c == ' ')
        .collect::<String>()
        .to_lowercase();
    said.split_whitespace().collect::<Vec<_>>() == ["scratch", "that"]
}

/// "Send it Monday. Scratch that. Send it Tuesday." keeps only what follows the command.
fn apply_scratch(input: &str) -> String {
    static SCRATCH: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)^.*\bscratch that\b[,.!?;:]*\s*").unwrap());
    if !SCRATCH.is_match(input) {
        return input.to_string();
    }
    capitalize_marked(&format!("{MARKER}{}", SCRATCH.replace(input, "")), false)
}

/// Capitalises the first letter after each MARKER (and after a line break, if asked), then drops the markers.
fn capitalize_marked(text: &str, after_newline: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut capitalize_next = false;
    for ch in text.chars() {
        if ch == MARKER {
            capitalize_next = true;
            continue;
        }
        if ch == '\n' && after_newline {
            capitalize_next = true;
            out.push(ch);
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
    out
}

/// A stutter on a small common word: "the the summit", "I, I think". Only words that are
/// almost never doubled on purpose, so "had had", "very very" and "no no" are kept.
pub fn remove_repeats(input: &str) -> String {
    const STUTTER: [&str; 24] = [
        "the", "a", "an", "i", "we", "you", "she", "it", "they", "to", "of", "at", "and", "but", "or", "for", "with",
        "are", "was", "my", "your", "our", "this", "if",
    ];
    static WORD: Lazy<Regex> = Lazy::new(|| Regex::new(r"[\p{L}\p{N}'’]+").unwrap());
    let words: Vec<(usize, usize, String)> =
        WORD.find_iter(input).map(|m| (m.start(), m.end(), m.as_str().to_lowercase())).collect();
    let mut out = String::with_capacity(input.len());
    let mut cursor = 0;
    for pair in words.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        let between = &input[a.1..b.0];
        if a.2 == b.2 && STUTTER.contains(&a.2.as_str()) && (between == " " || between == ", ") {
            // Drop the second copy; the first keeps its capital letter.
            out.push_str(&input[cursor..a.1]);
            cursor = b.1;
        }
    }
    out.push_str(&input[cursor..]);
    out
}

/// Hesitation sounds the model faithfully transcribes ("um", "uh", "hmm"…).
/// Also "äh/ähm/öh" (German, Swedish), "euh" (French), "ehm" (Italian, Dutch), "eee/yyy" (Polish), "эм/ээ" (Russian).
/// Deliberately excludes "er", "ah", "eh": real words in German and other supported languages.
static FILLER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(u+h*m+|u+h+|h+m+|m{2,}|e+r+m+|ä+h+m*|ö+h+m*|e+u+h+|e+h+m+|e{3,}|y{3,}|э+м+|э{2,})\b([,.…]*)")
        .unwrap()
});

fn is_word_joiner(c: char) -> bool {
    c.is_alphabetic() || matches!(c, '\'' | '’' | '-')
}

/// Takes out the user's own filler words ("like", "you know", "basically") wherever they are
/// said, together with the commas the model puts around them, and capitalises what follows
/// when the word opened a sentence.
pub fn remove_words(input: &str, words: &[String]) -> String {
    let alternatives: Vec<String> = words
        .iter()
        .map(|w| w.trim())
        .filter(|w| !w.is_empty())
        .map(|w| w.split_whitespace().map(regex::escape).collect::<Vec<_>>().join(r"\s+"))
        .collect();
    if alternatives.is_empty() {
        return input.to_string();
    }
    let Ok(re) = Regex::new(&format!(r"(?i)(?:,\s*)?\b(?:{})\b(?:\s*,)?", alternatives.join("|"))) else {
        return input.to_string();
    };
    let mut text = String::new();
    let mut cursor = 0;
    for m in re.find_iter(input) {
        // Leave parts of hyphenated or apostrophised words alone ("like-minded", "like's").
        let core = m.as_str().trim_matches(|c: char| c == ',' || c.is_whitespace());
        let core_start = m.start() + m.as_str().find(core).unwrap_or(0);
        let before = input[..core_start].chars().last();
        let after = input[core_start + core.len()..].chars().next();
        if before.is_some_and(is_word_joiner) || after.is_some_and(is_word_joiner) {
            continue;
        }
        text.push_str(&input[cursor..m.start()]);
        let prior = text.trim_end();
        let opens_sentence = prior.is_empty() || prior.ends_with(['.', '!', '?', '\n']);
        text.push(if opens_sentence { MARKER } else { ' ' });
        cursor = m.end();
    }
    text.push_str(&input[cursor..]);

    static TIDY: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
        [(r"[ \t]{2,}", " "), (r"[ \t]+([,.;:!?])", "$1"), ("\u{1}[ \t,.;:!?]+", "\u{1}"), (r"^[ \t]+", "")]
            .into_iter()
            .map(|(p, r)| (Regex::new(p).unwrap(), r))
            .collect()
    });
    for (re, to) in TIDY.iter() {
        text = re.replace_all(&text, *to).into_owned();
    }
    capitalize_marked(text.trim_end_matches([' ', '\t']), false)
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
const COMMAND_PHRASES: [(&str, &str); 4] = [
    // The model also writes these as "Newline", "new-line" or in the plural.
    (r"new[\s-]*paragraphs?|next\s+paragraph|change\s+(?:the\s+)?paragraph", "\n\n"),
    (r"new[\s-]*lines?|next\s+line|change\s+(?:the\s+)?line", "\n"),
    (r"bullet\s+points?", "\n• \u{1}"),
    (r"tab\s+key", "\t"),
];

/// How the model writes the app's name when it is said aloud. "Bowling" and "bollocks" are real
/// words, so they only count as the name right before a command.
const NAME_HEARD: &str = r"bow?ll?i(?:kh?|ck?|g|ch|que)|boleek|boling|polikh?";

/// A word said before a command to mark it as one: "Bolikh new line", "create a new line",
/// "insert a bullet point". May be followed by "a" or "the".
static LEAD: Lazy<String> = Lazy::new(|| {
    format!(r"\b(?:{NAME_HEARD}|bowling|bollocks|insert|create|add)\b[,:]?\s+(?:(?:a|the)\s+)?")
});

/// The command table in two forms: lead word optional (index 0) and lead word required (1).
static COMMANDS: Lazy<[Vec<(Regex, &'static str)>; 2]> = Lazy::new(|| {
    let build = |lead_needed: bool| {
        let q = if lead_needed { "" } else { "?" };
        COMMAND_PHRASES
            .iter()
            .map(|(phrase, out)| {
                let p = format!(r"(?i)\s*(?:{}){q}\b(?:{phrase})\b[,.;:!?]?\s*", LEAD.as_str());
                (Regex::new(&p).unwrap(), *out)
            })
            .collect()
    };
    [build(false), build(true)]
});

/// Writes the app's name properly however the model spelled it.
fn fix_app_name(text: &str) -> String {
    static NAME: Lazy<Regex> = Lazy::new(|| Regex::new(&format!(r"(?i)\b(?:{NAME_HEARD})\b")).unwrap());
    NAME.replace_all(text, "Bolikh").into_owned()
}

fn apply_voice_commands(input: &str, lead_needed: bool) -> String {
    static LISTS: Lazy<[Regex; 2]> = Lazy::new(|| {
        let build = |q: &str| {
            Regex::new(&format!(r"(?i)[,;:]?\s*(?:{}){q}\bnumber(?:ed|ing)?\s+lists?\b[,.;:!?]?\s*", LEAD.as_str())).unwrap()
        };
        [build("?"), build("")]
    });
    let list_re = &LISTS[lead_needed as usize];
    static ITEM: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)[,.;:!?]?\s*\bnext\s+items?\b[,.;:!?]?\s*").unwrap());
    static CAPS: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\ball\s+caps\b[,.]?\s+([\p{L}\p{N}'’-]+)").unwrap());

    let mut text = input.to_string();
    // "Numbered list milk, next item eggs" → "1. Milk\n2. Eggs". "Next item" only counts inside a list.
    if let Some(start) = list_re.find(&text) {
        let (mut head, tail) = (text[..start.start()].to_string(), text[start.end()..].to_string());
        // "Shopping list, numbered list milk" → "Shopping list:" introduces the list.
        if head.chars().last().is_some_and(|c| c.is_alphanumeric()) {
            head.push(':');
        }
        let mut n = 1;
        let mut list = format!("\n1. {MARKER}");
        let mut cursor = 0;
        for m in ITEM.find_iter(&tail) {
            n += 1;
            list.push_str(&tail[cursor..m.start()]);
            list.push_str(&format!("\n{n}. {MARKER}"));
            cursor = m.end();
        }
        list.push_str(&tail[cursor..]);
        text = head + &list;
    }
    for (re, out) in COMMANDS[lead_needed as usize].iter() {
        text = re.replace_all(&text, NoExpand(out)).into_owned();
    }
    text = CAPS.replace_all(&text, |c: &regex::Captures| c[1].to_uppercase()).into_owned();
    // A list that opens the dictation doesn't need a line break before it.
    if text.starts_with("\n• ") || text.starts_with("\n1. ") {
        text.remove(0);
    }
    capitalize_marked(&drop_hanging_commas(&text), true)
}

/// The model often puts a comma before a spoken command, which would be left hanging at the end
/// of the line. It is kept after a short opening such as "Dear team," or "Thanks,", where a
/// comma belongs, and dropped after list items and longer lines.
fn drop_hanging_commas(text: &str) -> String {
    if !text.contains('\n') {
        return text.to_string();
    }
    let lines: Vec<&str> = text.split('\n').collect();
    let last = lines.len() - 1;
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_end();
        let hanging = i < last && (trimmed.ends_with(',') || trimmed.ends_with(';'));
        let is_item = trimmed.starts_with("• ") || trimmed.split(". ").next().is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
        let before_list = i < last && (lines[i + 1].starts_with("• ") || lines[i + 1].starts_with("1. "));
        if hanging && !is_item && before_list {
            // "Things to do, bullet point …" → "Things to do:" introduces the list.
            out.push(format!("{}:", &trimmed[..trimmed.len() - 1]));
        } else if hanging && (is_item || trimmed.split_whitespace().count() > 3) {
            out.push(trimmed[..trimmed.len() - 1].to_string());
        } else {
            out.push(line.to_string());
        }
    }
    out.join("\n")
}

/// Top-level domains recognised after a spoken "dot".
const TLD: &str = "com|org|net|in|io|co|ai|dev|app|edu|gov|uk|us|de|fr|es|it|nl|ca|au|me|info|biz|xyz|tech";

/// "google dot com" → google.com, "john at gmail dot com" → john@gmail.com.
/// "at the rate" works too. "Meet me at google dot com" keeps its "at".
pub fn spoken_addresses(input: &str) -> String {
    static DOMAIN: Lazy<Regex> = Lazy::new(|| {
        Regex::new(&format!(r"(?i)\b([a-z0-9-]+(?:\s+dot\s+[a-z0-9-]+)*)\s+dot\s+({TLD})\b")).unwrap()
    });
    static EMAIL: Lazy<Regex> = Lazy::new(|| {
        Regex::new(&format!(
            r"(?i)\b([a-z0-9_-]+(?:(?:\s+dot\s+|\.)[a-z0-9_-]+)*)\s+at(?:\s+the\s+rate(?:\s+of)?)?\s+((?:[a-z0-9-]+\.)+(?:{TLD}))\b"
        ))
        .unwrap()
    });
    static DOT: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)\s+dot\s+").unwrap());
    // Words that come before "at" in a normal sentence rather than in an address.
    const NOT_A_NAME: [&str; 60] = [
        "i", "me", "us", "you", "him", "her", "them", "it", "we", "they", "am", "is", "are", "was", "were", "be",
        "been", "look", "looking", "that", "this", "there", "here", "up", "out", "back", "available", "found",
        "live", "online", "site", "website", "page", "visit", "see", "check", "go", "went", "work", "works",
        "working", "hosted", "registered", "mail", "email", "contact", "reach", "listed", "now", "and", "or",
        "account", "store", "shop", "app", "blog", "docs", "job", "team", "people",
    ];
    let text = DOMAIN.replace_all(input, |c: &regex::Captures| {
        format!("{}.{}", DOT.replace_all(&c[1], "."), &c[2]).to_lowercase()
    });
    EMAIL
        .replace_all(&text, |c: &regex::Captures| {
            let name = DOT.replace_all(&c[1], ".").to_lowercase();
            if NOT_A_NAME.contains(&name.as_str()) {
                return c[0].to_string();
            }
            format!("{name}@{}", c[2].to_lowercase())
        })
        .into_owned()
}

/// How a spoken symbol sits against the words around it.
#[derive(Clone, Copy, PartialEq)]
enum Fit {
    /// Hugs the word before it: , . ? ! ; : %
    Left,
    /// Like `Left`, and the next word starts a sentence.
    End,
    /// Space before, none after: ( [ { “ # $
    Open,
    /// None before, space after: ) ] } ”
    Close,
    /// No spaces at all: @ / \ _ -
    Tight,
    /// Spaces on both sides: & + = *
    Free,
}

/// Punctuation and symbols said out loud. "Period", "dash" and "dot" are left out on purpose:
/// they are everyday words ("a period of time") and the model already adds full stops.
static SYMBOLS: Lazy<Vec<(Regex, &'static str, Fit)>> = Lazy::new(|| {
    [
        ("comma", ",", Fit::Left),
        ("semi ?colon", ";", Fit::Left),
        ("colon", ":", Fit::Left),
        ("percent sign", "%", Fit::Left),
        ("ellipsis|dot dot dot", "…", Fit::Left),
        ("full stop", ".", Fit::End),
        ("question mark", "?", Fit::End),
        ("exclamation (?:mark|point)", "!", Fit::End),
        ("open square bracket", "[", Fit::Open),
        ("open curly (?:bracket|brace)", "{", Fit::Open),
        ("open (?:bracket|parenthesis|paren)", "(", Fit::Open),
        ("open quotes?", "“", Fit::Open),
        ("hashtag|hash sign", "#", Fit::Open),
        ("dollar sign", "$", Fit::Open),
        ("close square bracket", "]", Fit::Close),
        ("close curly (?:bracket|brace)", "}", Fit::Close),
        ("close (?:bracket|parenthesis|paren)", ")", Fit::Close),
        ("(?:close|end) quotes?", "”", Fit::Close),
        ("at sign", "@", Fit::Tight),
        ("back ?slash", "\\", Fit::Tight),
        ("forward slash|slash", "/", Fit::Tight),
        ("underscore", "_", Fit::Tight),
        ("hyphen", "-", Fit::Tight),
        ("ampersand", "&", Fit::Free),
        ("plus sign", "+", Fit::Free),
        ("equals? sign", "=", Fit::Free),
        ("asterisk", "*", Fit::Free),
        ("em dash", "—", Fit::Free),
    ]
    .into_iter()
    .map(|(phrase, symbol, fit)| {
        // The model often wraps the spoken word in its own commas or full stops; those go too.
        let before = match fit {
            Fit::Left | Fit::End => "[,.;:!?]?",
            Fit::Close | Fit::Tight | Fit::Free => ",?",
            Fit::Open => "",
        };
        let after = if matches!(fit, Fit::Left | Fit::End) { "[,.;:!?]?" } else { ",?" };
        let p = format!(r"(?i)[ \t]*{before}[ \t]*\b(?:{})\b{after}[ \t]*", phrase.replace(' ', r"\s+"));
        (Regex::new(&p).unwrap(), symbol, fit)
    })
    .collect()
});

pub fn spoken_symbols(input: &str) -> String {
    let mut text = input.to_string();
    let mut changed = false;
    for (re, symbol, fit) in SYMBOLS.iter() {
        if !re.is_match(&text) {
            continue;
        }
        changed = true;
        let out = match fit {
            Fit::Left | Fit::Close => format!("{symbol} "),
            Fit::End => format!("{symbol} {MARKER}"),
            Fit::Open => format!(" {symbol}"),
            Fit::Tight => symbol.to_string(),
            Fit::Free => format!(" {symbol} "),
        };
        text = re.replace_all(&text, NoExpand(&out)).into_owned();
    }
    if !changed {
        return text;
    }
    static TIDY: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
        [
            (r"[ \t]{2,}", " "),
            (r"[ \t]+([,.;:!?%)\]}”…])", "$1"),
            (r"([(\[{“#$])[ \t]+", "$1"),
            (r"[ \t]+\n", "\n"),
            (r"\n[ \t]+", "\n"),
        ]
        .into_iter()
        .map(|(p, t)| (Regex::new(p).unwrap(), t))
        .collect()
    });
    for (re, rep) in TIDY.iter() {
        text = re.replace_all(&text, *rep).into_owned();
    }
    capitalize_marked(text.trim_matches(' '), false)
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
        "lakh" | "lakhs" => Some(Tok::Scale(100_000)),
        "crore" | "crores" => Some(Tok::Scale(10_000_000)),
        _ => None,
    }
}

/// "first", "twelfth", "twentieth": the number word each ordinal stands in for.
fn ordinal(word: &str) -> Option<Tok> {
    const UNITS: [&str; 9] = ["first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth"];
    const TEENS: [&str; 10] = [
        "tenth", "eleventh", "twelfth", "thirteenth", "fourteenth", "fifteenth", "sixteenth", "seventeenth",
        "eighteenth", "nineteenth",
    ];
    const TENS: [&str; 8] =
        ["twentieth", "thirtieth", "fortieth", "fiftieth", "sixtieth", "seventieth", "eightieth", "ninetieth"];
    if let Some(i) = UNITS.iter().position(|w| *w == word) {
        return Some(Tok::Unit(1 + i as u64));
    }
    if let Some(i) = TEENS.iter().position(|w| *w == word) {
        return Some(Tok::Teen(10 + i as u64));
    }
    if let Some(i) = TENS.iter().position(|w| *w == word) {
        return Some(Tok::Tens(20 + 10 * i as u64));
    }
    match word {
        "hundredth" => Some(Tok::Hundred),
        "thousandth" => Some(Tok::Scale(1_000)),
        _ => None,
    }
}

fn ordinal_suffix(n: u64) -> &'static str {
    match (n % 100, n % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    }
}

/// "may" and "march" are left out: "you may first…" is not a date.
const MONTHS: [&str; 10] =
    ["january", "february", "april", "june", "july", "august", "september", "october", "november", "december"];

fn currency(word: &str) -> Option<&'static str> {
    match word {
        "dollar" | "dollars" => Some("$"),
        "rupee" | "rupees" => Some("₹"),
        "euro" | "euros" => Some("€"),
        _ => None,
    }
}

/// One well-formed number such as "two thousand twenty six". Words that don't fit
/// start a new chunk, and chunks are written side by side: "four eight" → 48,
/// "nineteen ninety nine" → 1999.
#[derive(Clone)]
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
/// "fifty percent" → 50%, "twenty first" → 21st, "fifty rupees" → ₹50, "three thirty pm" → 3:30 PM.
/// A lone "zero" to "ten" and "first" to "tenth" stay words ("one of them"), and so does "a hundred".
pub fn convert_numbers(input: &str) -> String {
    static WORD: Lazy<Regex> = Lazy::new(|| Regex::new(r"[A-Za-z]+").unwrap());
    static PERCENT: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)(\d) percent\b").unwrap());
    static MONEY: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\b(\d+(?:[.,]\d+)*) (dollars?|rupees?|euros?)\b").unwrap());
    static MONTH_DAY: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"\b(january|february|march|april|may|june|july|august|september|october|november|december) (\d{1,2}(?:st|nd|rd|th))\b")
            .unwrap()
    });
    static MERIDIEM: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)^ ([ap])\.? ?m\b(\.?)").unwrap());

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
    let is_month = |i: usize| word_at(i).is_some_and(|w| MONTHS.contains(&w));
    // "November first", "the fifth of November"
    let dated = |first: usize, last: usize| {
        (first > 0 && is_month(first - 1) && sep(first - 1, first) == " ")
            || (word_at(last + 1) == Some("of") && is_month(last + 2))
    };

    let mut out = String::with_capacity(input.len());
    let mut cursor = 0;
    let mut i = 0;
    while i < words.len() {
        // An ordinal on its own: "eleventh" and up, or a small one that is part of a date.
        if let Some(t) = ordinal(&words[i].2) {
            if let Tok::Unit(v) | Tok::Teen(v) | Tok::Tens(v) = t {
                if v > 10 || dated(i, i) {
                    out.push_str(&input[cursor..words[i].0]);
                    out.push_str(&format!("{v}{}", ordinal_suffix(v)));
                    cursor = words[i].1;
                }
            }
            i += 1;
            continue;
        }
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

        // "twenty first" → 21st. "A twenty second clip" is a length, not a position, so
        // "second" only counts after "the", "my"… or in a date.
        let mut is_ordinal = false;
        if let Some(t) = word_at(last + 1).and_then(ordinal) {
            let positional = word_at(last + 1) != Some("second")
                || dated(i, last + 1)
                || (i > 0
                    && sep(i - 1, i) == " "
                    && matches!(word_at(i - 1), Some("the" | "my" | "our" | "his" | "her" | "its" | "their")));
            let mut with = chunk.clone();
            if matches!(sep(last, last + 1), " " | "-") && positional && with.accept(t) {
                chunk = with;
                last += 1;
                count += 1;
                is_ordinal = true;
            }
        }
        chunks.push(chunk.digits());

        let mut text = chunks.concat();
        let mut end = last;
        let mut end_pos = words[last].1;
        let mut forced = is_ordinal;
        if is_ordinal {
            text.push_str(ordinal_suffix(chunk.total + chunk.group));
        } else {
            let mut decimal = false;
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
                    end_pos = words[d].1;
                    decimal = true;
                    forced = true;
                }
            }
            let unit = word_at(end + 1).filter(|_| sep(end, end + 1) == " ");
            if unit == Some("percent") {
                text.push('%');
                end += 1;
                end_pos = words[end].1;
                forced = true;
            } else if let Some(symbol) = unit.and_then(currency) {
                text.insert_str(0, symbol);
                end += 1;
                end_pos = words[end].1;
                forced = true;
            } else if !decimal {
                // "three thirty pm" → 3:30 PM, "nine a.m." → 9 AM
                let hour = chunks[0].parse::<u64>().unwrap_or(0);
                let minute = chunks.get(1).and_then(|m| m.parse::<u64>().ok());
                let clock = (1..=12).contains(&hour)
                    && match (chunks.len(), minute) {
                        (1, _) => true,
                        (2, Some(m)) => (10..=59).contains(&m),
                        _ => false,
                    };
                // "…and three am I": a bare "am" before "I" is the verb.
                let verb = |m: &regex::Captures| {
                    m[0].eq_ignore_ascii_case(" am") && input[end_pos + m[0].len()..].starts_with(" I")
                };
                if let Some(m) = MERIDIEM.captures(&input[end_pos..]).filter(|m| clock && !verb(m)) {
                    text = match minute {
                        Some(min) => format!("{hour}:{min}"),
                        None => hour.to_string(),
                    };
                    text.push_str(if m[1].eq_ignore_ascii_case("a") { " AM" } else { " PM" });
                    end_pos += m.get(0).unwrap().end();
                    // "p.m." at the end of a sentence: its last dot is also the full stop.
                    let rest = &input[end_pos..];
                    let sentence_end = rest.trim_start().chars().next().is_none_or(|c| c.is_uppercase());
                    if !m[2].is_empty() && sentence_end {
                        text.push('.');
                    }
                    forced = true;
                }
            }
        }

        let lone_small = count == 1 && matches!(first, Tok::Unit(_) | Tok::Teen(10));
        if lone_small && !forced {
            i = last + 1;
            continue;
        }
        out.push_str(&input[cursor..words[i].0]);
        out.push_str(&text);
        cursor = end_pos;
        i = end + 1;
        while i < words.len() && words[i].0 < cursor {
            i += 1;
        }
    }
    out.push_str(&input[cursor..]);
    let out = PERCENT.replace_all(&out, "$1%");
    // "november 17th" → "November 17th"
    let out = MONTH_DAY.replace_all(&out, |c: &regex::Captures| {
        let mut month = c[1].to_string();
        month[..1].make_ascii_uppercase();
        format!("{month} {}", &c[2])
    });
    MONEY
        .replace_all(&out, |c: &regex::Captures| {
            format!("{}{}", currency(&c[2].to_lowercase()).unwrap_or(""), &c[1])
        })
        .into_owned()
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
            "I was thinking about the summit."
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
            ("Twenty five hundred rupees.", "₹2500."),
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
            "It is a one-on-one in two-three days.",
            "No one's point was missed.",
            "Ten slotte, zes en een.",
        ] {
            assert_eq!(process(t, &s), t);
        }
        assert_eq!(process("I had had enough, it was very very good, no no.", &s), "I had had enough, it was very very good, no no.");
        let mut off = settings();
        off.numbers_as_digits = false;
        assert_eq!(process("I am forty eight.", &off), "I am forty eight.");
    }

    fn check(cases: &[(&str, &str)]) {
        let s = settings();
        for (said, typed) in cases {
            assert_eq!(process(said, &s), *typed, "said: {said}");
        }
    }

    #[test]
    fn more_number_formats() {
        check(&[
            ("On the twenty first and the thirty-second.", "On the 21st and the 32nd."),
            ("Due November seventeenth.", "Due November 17th."),
            ("Due on november seventeenth, at nine a.m.", "Due on November 17th, at 9 AM."),
            ("On the fifth of November, the twelfth floor.", "On the 5th of November, the 12th floor."),
            ("You may first want the third option.", "You may first want the third option."),
            ("A twenty second clip, one third of it.", "A 20 second clip, one third of it."),
            ("It costs fifty rupees, or five dollars.", "It costs ₹50, or $5."),
            ("Budget is two crore fifty lakh.", "Budget is 25000000."),
            ("Pay 1,200 euros and 20 dollars.", "Pay €1,200 and $20."),
            ("Meet at three thirty pm.", "Meet at 3:30 PM."),
            ("Call at nine a.m. Then lunch at twelve p.m.", "Call at 9 AM. Then lunch at 12 PM."),
            ("I am twenty and three am I.", "I am 20 and three am I."),
        ]);
    }

    #[test]
    fn repeated_words() {
        check(&[
            ("The the plan is to to ship.", "The plan is to ship."),
            ("I, I think we we should go.", "I think we should go."),
            ("Leute, die die Arbeit machen.", "Leute, die die Arbeit machen."),
        ]);
    }

    #[test]
    fn spoken_symbols_and_addresses() {
        check(&[
            ("Hello comma how are you question mark", "Hello, how are you?"),
            ("Is it done. Question mark. yes it is full stop", "Is it done? Yes it is."),
            ("See the notes open bracket page two close bracket for details.", "See the notes (page two) for details."),
            ("Tag it hashtag launch and email me.", "Tag it #launch and email me."),
            ("The file is my underscore notes, in src slash app.", "The file is my_notes, in src/app."),
            ("Tom ampersand Jerry, a well hyphen known pair.", "Tom & Jerry, a well-known pair."),
            ("Write to john at gmail dot com today.", "Write to john@gmail.com today."),
            ("It is john dot smith at the rate example dot co dot in.", "It is john.smith@example.co.in."),
            ("Meet me at Google dot com.", "Meet me at google.com."),
            ("My email is John.smith at the rate example dot co dot in", "My email is john.smith@example.co.in"),
            ("A period of time, a dash of salt.", "A period of time, a dash of salt."),
        ]);
        let mut off = settings();
        off.spoken_symbols = false;
        assert_eq!(process("Hello comma world", &off), "Hello comma world");
    }

    #[test]
    fn removes_the_users_own_filler_words() {
        let mut s = settings();
        s.remove_words = vec!["like".into(), "you know".into(), " Basically ".into(), "".into()];
        for (said, typed) in [
            ("I was, like, going to the shop.", "I was going to the shop."),
            ("It was like really good.", "It was really good."),
            ("Like, I think we should wait.", "I think we should wait."),
            ("So, you know, we waited. Basically it worked.", "So we waited. It worked."),
            ("It was good, you know.", "It was good."),
            ("We are like-minded and I dislike delays.", "We are like-minded and I dislike delays."),
            ("Like.", ""),
            ("It worked. Like. We left.", "It worked. We left."),
        ] {
            assert_eq!(process(said, &s).trim_end(), typed, "said: {said}");
        }
        // Nothing listed: nothing removed.
        assert_eq!(process("I was, like, going.", &settings()).trim_end(), "I was, like, going.");
    }

    #[test]
    fn lead_words_and_more_line_commands() {
        check(&[
            // Sentences a user really dictated.
            ("Let's keep it public. Change line. So plan the content.", "Let's keep it public.\nSo plan the content."),
            ("First point, change the line, second point", "First point,\nSecond point"),
            ("First part. Next paragraph. Second part.", "First part.\n\nSecond part."),
            // A lead word before the command is taken with it.
            ("Hello team. Bolikh new line. Thanks.", "Hello team.\nThanks."),
            ("Hello team, bolic change the line, thanks", "Hello team,\nThanks"),
            ("Hello team. Bowling, new line. Thanks.", "Hello team.\nThanks."),
            ("Notes create a new line done", "Notes\nDone"),
            ("Things to do insert a bullet point call Harsh add a bullet point send it", "Things to do\n• Call Harsh\n• Send it"),
            ("Shopping, create a numbered list, milk, next item eggs", "Shopping:\n1. Milk\n2. Eggs"),
            // The app's name is written properly; real words are left alone.
            ("It's not writing bolic itself.", "It's not writing Bolikh itself."),
            ("bowlic writes the text", "Bolikh writes the text"),
            ("I use Polik and Bolig and bolikh daily.", "I use Bolikh and Bolikh and Bolikh daily."),
            ("The police found a metabolic bollard.", "The police found a metabolic bollard."),
            ("We went bowling on Friday.", "We went bowling on Friday."),
            ("That is symbolic.", "That is symbolic."),
        ]);

        // With "lead word needed" on, the bare phrases are ordinary words.
        let mut strict = settings();
        strict.command_lead_word = true;
        for (said, typed) in [
            ("We opened a new line of business.", "We opened a new line of business."),
            ("Press enter for a new line in the file.", "Press enter for a new line in the file."),
            ("The numbered list is ready.", "The numbered list is ready."),
            ("Hello team. Bolikh new line. Thanks.", "Hello team.\nThanks."),
            ("Hello team, create a new line, thanks", "Hello team,\nThanks"),
            ("Shopping, bolic numbered list, milk, next item eggs", "Shopping:\n1. Milk\n2. Eggs"),
            ("Things to do. Insert a bullet point call Harsh.", "Things to do.\n• Call Harsh."),
        ] {
            assert_eq!(process(said, &strict).trim_end(), typed, "said: {said}");
        }
    }

    /// Prints what the examples in the in-app and website guide really produce:
    /// `cargo test --lib guide_examples -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn guide_examples() {
        let s = settings();
        for said in [
            "Hi Priya new line thanks for the update",
            "First point change line second point",
            "Hello team Bolikh new line thanks",
            "Notes create a bullet point call Harsh",
            "Here is the plan new paragraph first we test",
            "Things to buy bullet point milk bullet point eggs",
            "Shopping numbered list milk next item eggs next item bread",
            "name tab key age",
            "this is all caps urgent",
            "Send it today scratch that send it tomorrow",
            "Hello comma how are you question mark",
            "That is great exclamation mark",
            "see the notes open bracket page two close bracket",
            "he said open quote hello close quote",
            "use hashtag launch",
            "my file underscore name",
            "either slash or",
            "tom ampersand jerry",
            "wait ellipsis",
            "this is the end full stop",
            "two plus sign two equals sign four",
            "state hyphen of hyphen the hyphen art",
            "it costs forty eight",
            "pay fifty rupees",
            "pay twenty dollars",
            "meet at three thirty pm",
            "on the twenty first",
            "on november seventeenth",
            "growth was twelve percent",
            "it is three point five",
            "in nineteen ninety nine",
            "two lakh fifty thousand",
            "mail john at gmail dot com",
            "visit example dot com",
            "I I think the the plan is um good",
        ] {
            println!("{said}  =>  {:?}", process(said, &s));
        }
    }

    #[test]
    fn more_voice_commands() {
        check(&[
            ("Bullet point milk. Bullet point eggs.", "• Milk.\n• Eggs."),
            ("Shopping. Numbered list milk, next item eggs, next item bread.", "Shopping.\n1. Milk\n2. Eggs\n3. Bread."),
            ("The next item is ready.", "The next item is ready."),
            // What the model really wrote for a user: "number list", and commas around commands.
            (
                "Create a shopping list, number list, milk, next item eggs, next item coffee, next item aroma.",
                "Create a shopping list:\n1. Milk\n2. Eggs\n3. Coffee\n4. Aroma.",
            ),
            ("Hello team, Newline. I will be late today, new line, thanks.", "Hello team,\nI will be late today\nThanks."),
            ("Price first. New paragraph. Speed second.", "Price first.\n\nSpeed second."),
            ("Things to do. Bullet point call Harsh, bullet point send the invoice.", "Things to do.\n• Call Harsh\n• Send the invoice."),
            ("I read the newspaper online.", "I read the newspaper online."),
            ("Things to do, bullet point call Harsh, bullet point send the invoice.", "Things to do:\n• Call Harsh\n• Send the invoice."),
            ("This is all caps urgent, please.", "This is URGENT, please."),
            ("Name tab key age", "Name\tage"),
            ("Send it Monday. Scratch that. send it Tuesday.", "Send it Tuesday."),
        ]);
        assert!(is_scratch_only("Um, scratch that."));
        assert!(!is_scratch_only("Scratch that idea."));
        assert_eq!(process("Send it Monday. Scratch that.", &settings()), "");
    }

    #[test]
    fn snippets_and_more_fillers() {
        let mut s = settings();
        s.replacements = vec![Replacement { from: "my sign off".into(), to: "Thanks,\\nAlex".into() }];
        assert_eq!(process("My sign off", &s), "Thanks,\nAlex");
        assert_eq!(process("Das ist, ähm, gut. Euh, oui.", &s), "Das ist gut. Oui.");
    }
}
