#!/usr/bin/env python3
"""Writes the "how to use Bolikh" guide into the app (ui/index.html) and the website (website/index.html).

The guide lives here, once, so the two copies cannot drift apart. Each copy sits between
<!--GUIDE-START--> and <!--GUIDE-END--> markers. After changing the guide (or adding a voice command):

    python3 scripts/build-guide.py && python3 scripts/build-os-pages.py

Every "typed" value below was produced by the real clean-up code; check new ones with
    cd src-tauri && cargo test --lib guide_examples -- --ignored --nocapture
"""
import html, pathlib

root = pathlib.Path(__file__).resolve().parent.parent

# (title, intro, [(what you say, what is typed)], note)
GROUPS = [
    ("Lines and lists",
     "Say the command in the middle of your sentence, without pausing for it.",
     [("Hi Priya new line thanks for the update", "Hi Priya\nThanks for the update"),
      ("First point change line second point", "First point\nSecond point"),
      ("Here is the plan new paragraph first we test", "Here is the plan\n\nFirst we test"),
      ("Hello team Bolikh new line thanks", "Hello team\nThanks"),
      ("Notes create a bullet point call Harsh", "Notes\n• Call Harsh"),
      ("Things to buy bullet point milk bullet point eggs", "Things to buy\n• Milk\n• Eggs"),
      ("Shopping numbered list milk next item eggs next item bread", "Shopping:\n1. Milk\n2. Eggs\n3. Bread"),
      ("name tab key age", "name	age")],
     "A numbered list keeps counting while you keep talking. When you start a new dictation, say "
     "“numbered list” again and it starts from 1. You can say “Bolikh”, “create”, “insert” or “add” "
     "before any of these commands. To write words like “new line” as ordinary text, switch on "
     "“Commands need a lead word” in Settings; the commands then only work with a lead word."),
    ("Fixing and emphasis",
     "Change your mind while speaking, or right after.",
     [("Send it today scratch that send it tomorrow", "Send it tomorrow"),
      ("scratch that", "Said on its own, it deletes the dictation you just made"),
      ("this is all caps urgent", "this is URGENT")],
     "Pressing Esc while recording cancels it, and nothing is typed."),
    ("Punctuation",
     "Bolikh adds commas, full stops and question marks by itself. Say them when you want to be exact.",
     [("Hello comma how are you question mark", "Hello, how are you?"),
      ("That is great exclamation mark", "That is great!"),
      ("this is the end full stop", "this is the end."),
      ("see the notes open bracket page two close bracket", "see the notes (page two)"),
      ("he said open quote hello close quote", "he said “hello”"),
      ("state hyphen of hyphen the hyphen art", "state-of-the-art"),
      ("wait ellipsis", "wait…")],
     "Also: colon, semicolon, open and close square bracket, open and close curly bracket, em dash. "
     "“Period” and “dash” are left as words, because people say them in normal sentences; say "
     "“full stop” and “hyphen”."),
    ("Symbols",
     "For tags, file names, sums and code.",
     [("use hashtag launch", "use #launch"),
      ("thanks at the rate Priya", "thanks @Priya"),
      ("my file underscore name", "my file_name"),
      ("either slash or", "either/or"),
      ("tom ampersand jerry", "tom & jerry"),
      ("two plus sign two equals sign four", "two + two = four")],
     "Also: at sign, percent sign, dollar sign, asterisk, backslash."),
    ("Numbers, money, times and dates",
     "Say numbers the way you normally would.",
     [("it costs forty eight", "it costs 48"),
      ("pay fifty rupees", "pay ₹50"),
      ("pay twenty dollars", "pay $20"),
      ("meet at three thirty pm", "meet at 3:30 PM"),
      ("on the twenty first", "on the 21st"),
      ("on november seventeenth", "on November 17th"),
      ("growth was twelve percent", "growth was 12%"),
      ("it is three point five", "it is 3.5"),
      ("two lakh fifty thousand", "250000")],
     "A single “one” to “ten” stays a word, as in “I have two ideas”. This works for English only."),
    ("Email and web addresses",
     "Say the address as you would read it out.",
     [("mail john at gmail dot com", "mail john@gmail.com"),
      ("visit example dot com", "visit example.com")],
     ""),
    ("Automatic clean-up",
     "Bolikh tidies what you say without being asked.",
     [("I I think the the plan is um good", "I think the plan is good")],
     "Have your own habit words, such as “like” or “you know”? Add them under “Words to leave out” in "
     "Dictionary and Bolikh removes those too. For names and special words Bolikh gets wrong, add what "
     "it wrote and what it should write, also in Dictionary. Every feature in this guide can be "
     "switched off in Settings."),
]

esc = html.escape


def replace_between(path, new):
    text = path.read_text()
    a, b = text.index("<!--GUIDE-START-->"), text.index("<!--GUIDE-END-->")
    path.write_text(text[:a] + "<!--GUIDE-START-->\n" + new + "\n" + text[b:])
    print("wrote guide into", path.relative_to(root))


def app():
    out = []
    for title, intro, rows, note in GROUPS:
        out.append(f'        <h2>{esc(title)}</h2>\n        <p class="ghint">{esc(intro)}</p>')
        out.append('        <div class="card gtable"><div class="grow ghead"><span>You say</span><span>Bolikh types</span></div>')
        for said, typed in rows:
            out.append(f'          <div class="grow"><span class="gsay">{esc(said)}</span><span class="gout selectable">{esc(typed)}</span></div>')
        out.append("        </div>")
        if note:
            out.append(f'        <p class="gnote">{esc(note)}</p>')
    replace_between(root / "ui" / "index.html", "\n".join(out))


def website():
    out = []
    for title, intro, rows, note in GROUPS:
        out.append(f'      <article class="gcard"><h3>{esc(title)}</h3><p class="gintro">{esc(intro)}</p>')
        out.append('        <div class="gt"><div class="gr gh"><span>You say</span><span>Bolikh types</span></div>')
        for said, typed in rows:
            out.append(f'          <div class="gr"><span class="gs">{esc(said)}</span><span class="go">{esc(typed)}</span></div>')
        out.append("        </div>")
        if note:
            out.append(f'        <p class="gn">{esc(note)}</p>')
        out.append("      </article>")
    replace_between(root / "website" / "index.html", "\n".join(out))


app()
website()
