#!/usr/bin/env python3
"""Builds the comparison pages: website/compare.html and one "<app>-alternative.html" per app in APPS.

The page shell (head, header, footer, script) comes from website/privacy.html, like the per-system pages.
Every fact about another app must come from that app's own site or docs; put the date you checked in CHECKED
and keep the source link in `src`. Say what the other app does better as well: these pages are meant to be fair.
Run again after changing anything here:  python3 scripts/build-compare-pages.py
"""
import json, pathlib

SITE = "https://bolikh.aazadsoftwaresolutions.in"
CHECKED = "23 November 2026"
web = pathlib.Path(__file__).resolve().parent.parent / "website"
shell = (web / "privacy.html").read_text()

ROWS = ["Price", "Works on", "Where speech becomes text", "Works without internet", "Source code", "Rewrites your text with AI"]
BOLIKH = ["Free, with no word limit", "Mac (Apple Silicon), Windows, Linux", "On your computer", "Yes",
          "Open (GPL-3.0)", "No. It tidies with fixed rules: fillers, numbers, punctuation"]

APPS = {
    "wispr-flow": dict(
        name="Wispr Flow", file="wispr-flow-alternative.html", src="https://wisprflow.ai/pricing",
        title="Free Wispr Flow alternative that works offline | Bolikh",
        desc="Bolikh is a free, open-source alternative to Wispr Flow. It turns speech into text on your own "
             "computer, with no weekly word limit, no subscription and no internet needed. Mac, Windows and Linux.",
        h1="A free Wispr Flow alternative that works offline.",
        lead="Wispr Flow and Bolikh do the same job: you speak, and your words are typed in whatever app you are "
             "using. The main difference is where the work happens. Wispr Flow sends your speech to its servers; "
             "Bolikh turns it into text on your own computer, so it is free, has no word limit and keeps working "
             "with the internet off.",
        cells=["Free plan with 2,000 words a week on desktop. Pro is $15 a month, or $12 a month billed yearly",
               "Mac, Windows, iPhone, Android", "On Wispr’s servers", "No, it needs a connection", "Closed",
               "Yes"],
        ours=["You dictate a lot and don’t want a weekly word limit or a subscription.",
              "You want your voice to stay on your computer, or you often work without internet.",
              "You use Linux, which Wispr Flow doesn’t support.",
              "You want to read the source code of the app that listens to your microphone."],
        theirs=["You want dictation on your phone as well. Bolikh is for computers only.",
                "You speak a language outside Bolikh’s 25, such as Hindi. Wispr Flow lists more than 100.",
                "You want AI to rewrite and polish what you said, not only tidy it.",
                "You need team features such as shared dictionaries and admin controls."],
        faq=[("Is there a free alternative to Wispr Flow?",
              "Yes. Bolikh is free with no word limit, and its source code is open. It runs on Mac (Apple Silicon), "
              "Windows and Linux."),
             ("Does Wispr Flow work offline?",
              "Wispr Flow’s own help pages say transcription needs an internet connection, because speech is "
              "processed on its servers. Bolikh works offline after a one-time download of its speech model "
              "(about 670 MB)."),
             ("What will I miss if I switch from Wispr Flow to Bolikh?",
              "Phone apps, AI rewriting, team features and languages outside Bolikh’s 25 European ones. Bolikh "
              "keeps to one job: typing what you say, cleaned up, in any app.")],
    ),
    "superwhisper": dict(
        name="Superwhisper", file="superwhisper-alternative.html", src="https://superwhisper.com/docs/billing/plans",
        title="Free Superwhisper alternative for Mac, Windows, Linux | Bolikh",
        desc="Bolikh is a free, open-source alternative to Superwhisper. Offline dictation in any app on Mac, "
             "Windows and Linux, with the Parakeet speech model included at no cost.",
        h1="A free Superwhisper alternative, with Linux included.",
        lead="Superwhisper and Bolikh can both turn speech into text on your own device. Superwhisper gives you "
             "many models and AI modes, and keeps the more advanced ones for its paid plan. Bolikh does less on "
             "purpose: one good speech model, text clean-up built in, everything free.",
        cells=["Free with local Whisper models. Pro is $8.49 a month, $84.99 a year or $249.99 once",
               "Mac, Windows, iPhone, Android", "On your device with local models; cloud models in Pro",
               "Yes, with local models", "Closed", "Yes, in Pro"],
        ours=["You want the Parakeet speech model without paying. In Superwhisper it is part of Pro; in Bolikh "
              "it is the only model and it is free.",
              "You use Linux, which Superwhisper doesn’t support.",
              "You want something with nothing to configure: install, hold a key, speak.",
              "You prefer open-source software."],
        theirs=["You want dictation on your phone as well.",
                "You want AI modes that turn rough speech into an email, a summary or a different tone.",
                "You want to choose between several speech models, including cloud ones.",
                "You speak a language outside Bolikh’s 25. Whisper models cover more."],
        faq=[("Is there a free alternative to Superwhisper?",
              "Yes. Bolikh is completely free and open source. Superwhisper also has a free plan with local "
              "Whisper models; its Parakeet model, cloud models and AI modes need Pro."),
             ("Does Bolikh run on Linux?",
              "Yes. Bolikh has installers for Ubuntu, Debian and Fedora, and works on X11 and Wayland."),
             ("What will I miss if I switch from Superwhisper to Bolikh?",
              "Phone apps, AI modes, a choice of models and meeting recording. Bolikh only does live dictation.")],
    ),
    "macwhisper": dict(
        name="MacWhisper", file="macwhisper-alternative.html", src="https://goodsnooze.gumroad.com/l/macwhisper",
        title="MacWhisper alternative for live dictation, free | Bolikh",
        desc="Looking for a MacWhisper alternative for typing by voice? Bolikh is a free, offline dictation app "
             "for Mac, Windows and Linux. Hold a key, speak, and your words are typed in any app.",
        h1="A free MacWhisper alternative for live dictation.",
        lead="MacWhisper is best known for transcribing recordings: you give it an audio or video file and get "
             "the text. Bolikh is built for the other job: you speak now, and the words are typed where your "
             "cursor is. If live dictation is what you are after, Bolikh is free and also runs on Windows and "
             "Linux.",
        cells=["Free version; Pro is a one-time purchase (about €64 when we checked)", "Mac only",
               "On your Mac", "Yes", "Closed", "Optional, in Pro"],
        ours=["You want to type by voice in any app, all day, with one key.",
              "You also use Windows or Linux and want the same app everywhere.",
              "You don’t want to pay for dictation."],
        theirs=["You need to transcribe audio or video files, podcasts or meetings. Bolikh doesn’t do that.",
                "You want subtitles or other export formats from a recording.",
                "You want to pick between Whisper and Parakeet models."],
        faq=[("Can Bolikh transcribe an audio file like MacWhisper?",
              "No. Bolikh only types what you say into the microphone. For files, MacWhisper is the right tool."),
             ("Is Bolikh free on Mac?",
              "Yes. It is free and open source, and needs a Mac with Apple Silicon (M1 or newer)."),
             ("Do both work offline?",
              "Yes. Both turn speech into text on your Mac.")],
    ),
    "voiceink": dict(
        name="VoiceInk", file="voiceink-alternative.html", src="https://tryvoiceink.com/",
        title="Free VoiceInk alternative for Mac, Windows and Linux | Bolikh",
        desc="Bolikh is a free, open-source alternative to VoiceInk. Offline dictation in any app, with a "
             "ready-made installer for Mac, Windows and Linux.",
        h1="A free VoiceInk alternative that also runs on Windows and Linux.",
        lead="VoiceInk and Bolikh have a lot in common: both are open source and both can turn speech into text "
             "on your own machine. VoiceInk is a Mac app you buy once, with optional AI features. Bolikh is free, "
             "simpler, and runs on Windows and Linux too.",
        cells=["$25 once for one Mac ($39 for two, $49 for three); free if you build it from the source yourself",
               "Mac (macOS 15 or later), with an iPhone app", "On your Mac with local models; cloud models optional",
               "Yes, with local models", "Open", "Yes, optional"],
        ours=["You want a free, ready-made installer.",
              "You use Windows or Linux, or a Mac on macOS 12 to 14.",
              "You want one app that behaves the same on every computer you use."],
        theirs=["You want AI to rewrite what you said, for example into an email.",
                "You have an Intel Mac. VoiceInk suggests cloud transcription there; Bolikh doesn’t support "
                "Intel Macs yet.",
                "You want an iPhone app from the same maker."],
        faq=[("Is there a free alternative to VoiceInk?",
              "Yes. Bolikh is free to download and use. VoiceInk is also free if you build it from its source "
              "code yourself; the ready-made app is a one-time purchase."),
             ("Are both open source?",
              "Yes. You can read the code of both apps."),
             ("Does Bolikh need macOS 15?",
              "No. Bolikh runs on macOS 12 Monterey or later, on Macs with Apple Silicon.")],
    ),
    "handy": dict(
        name="Handy", file="handy-alternative.html", src="https://github.com/cjpais/Handy",
        title="Handy alternative: Bolikh vs Handy, free offline dictation | Bolikh",
        desc="Bolikh and Handy are both free, open-source, offline dictation apps for Mac, Windows and Linux. "
             "See how they differ: Bolikh has text clean-up and voice commands built in; Handy lets you choose "
             "the speech model.",
        h1="Bolikh and Handy: two free, offline dictation apps.",
        lead="Handy is the closest app to Bolikh. Both are free, open source, run on Mac, Windows and Linux, and "
             "turn speech into text on your own computer. They differ in what happens around the speech model: "
             "Handy gives you a choice of models and stays minimal; Bolikh has one model and puts its effort "
             "into the text you get back.",
        cells=["Free", "Mac, Windows, Linux", "On your computer", "Yes", "Open (MIT)",
               "Optional: you can connect an AI model"],
        ours=["You want clean text without setting anything up: fillers removed, numbers as digits, and spoken "
              "commands such as “new line”, “bullet point” and “scratch that”.",
              "You want your own list of words to fix and words to leave out, inside the app.",
              "You want a built-in guide that shows everything you can say."],
        theirs=["You speak a language outside Bolikh’s 25. Handy can run Whisper models, which cover more "
                "languages, including Hindi.",
                "You want to choose your speech model, or use your graphics card to speed it up.",
                "You want to pass the text through an AI model before it is typed.",
                "You want to control the app from the command line."],
        faq=[("Do Bolikh and Handy use the same speech model?",
              "Both can use NVIDIA’s Parakeet V3. In Bolikh it is the only model. Handy also offers several "
              "Whisper models."),
             ("Which one should I pick?",
              "If you want to choose models or need a language Bolikh doesn’t have, pick Handy. If you want "
              "tidy text and voice commands with nothing to set up, pick Bolikh. Both are free, so you can try "
              "both."),
             ("Are both really free?",
              "Yes. Neither has a paid plan.")],
    ),
}
ORDER = ["wispr-flow", "superwhisper", "macwhisper", "voiceink", "handy"]

HUB_FAQ = [
    ("Is Bolikh built on Whisper?",
     "No. Bolikh uses NVIDIA’s Parakeet TDT 0.6B v3 speech model, which runs on your own computer. Whisper is a "
     "different speech model, made by OpenAI. Several apps with “whisper” in their name are built on it."),
    ("Which of these dictation apps are free?",
     "Bolikh and Handy are free with nothing held back. Superwhisper, Wispr Flow and MacWhisper have free plans "
     "with limits or fewer features. VoiceInk is a one-time purchase, or free if you build it yourself."),
    ("Which ones work without internet?",
     "Bolikh, Handy, MacWhisper, and Superwhisper and VoiceInk with their local models. Wispr Flow needs a "
     "connection."),
    ("Which ones run on Linux?",
     "Of the apps on this page, Bolikh and Handy."),
]


def esc(s):
    return s.replace("&", "&amp;")


def head(title, desc, url, crumbs, faq):
    ld = {"@context": "https://schema.org", "@graph": [
        {"@type": "SoftwareApplication", "name": "Bolikh", "url": SITE + "/", "description": desc,
         "applicationCategory": "UtilitiesApplication", "operatingSystem": "macOS, Windows, Linux",
         "image": f"{SITE}/og.png", "isAccessibleForFree": True,
         "offers": {"@type": "Offer", "price": "0", "priceCurrency": "USD"},
         "publisher": {"@type": "Organization", "name": "Aazad Software Solutions",
                       "url": "https://www.aazadsoftwaresolutions.in"}},
        {"@type": "BreadcrumbList", "itemListElement": [
            {"@type": "ListItem", "position": i + 1, "name": n, "item": u} for i, (n, u) in enumerate(crumbs)]},
        {"@type": "FAQPage", "mainEntity": [
            {"@type": "Question", "name": q, "acceptedAnswer": {"@type": "Answer", "text": a}} for q, a in faq]}]}
    ld_json = json.dumps(ld, ensure_ascii=False, separators=(",", ":")).replace("</", "<\\/")
    og = title.split(" | ")[0]
    return f'''<title>{title}</title>
<meta name="claude:viewport-fit-sides" content="cover">
<meta name="description" content="{desc}">
<link rel="canonical" href="{url}">
<meta property="og:site_name" content="Bolikh">
<meta property="og:title" content="{og}">
<meta property="og:description" content="{desc}">
<meta property="og:type" content="website">
<meta property="og:url" content="{url}">
<meta property="og:image" content="{SITE}/og.png">
<meta property="og:image:width" content="1200">
<meta property="og:image:height" content="630">
<meta name="twitter:card" content="summary_large_image">
<script type="application/ld+json">{ld_json}</script>
'''


def write(file, head_html, main):
    s = shell
    a, b = s.index("<title>"), s.index('<meta name="theme-color"')
    s = s[:a] + head_html + s[b:]
    a, b = s.index('<main id="top">'), s.index("</main>") + len("</main>")
    s = s[:a] + main + s[b:]
    (web / file).write_text(s)
    print("wrote", file, len(s))


def table(cols, rows):
    """cols: header labels after the first (row-name) column; rows: (label, [cells])."""
    th = "".join(f'<th scope="col">{c}</th>' for c in cols)
    tr = "".join(f'<tr><th scope="row">{label}</th>' + "".join(f"<td>{esc(c)}</td>" for c in cells) + "</tr>"
                 for label, cells in rows)
    return f'<div class="vs-wrap"><table class="vs"><thead><tr><td></td>{th}</tr></thead><tbody>{tr}</tbody></table></div>'


def faq_html(faq):
    return "\n      ".join(f'<details><summary>{q}</summary><div class="a"><p>{esc(a)}</p></div></details>' for q, a in faq)


def ul(items):
    return '<ul class="vs-list">' + "".join(f"<li>{esc(i)}</li>" for i in items) + "</ul>"


CTA = ('<div style="display:flex;gap:10px;flex-wrap:wrap"><a class="btn p" href="index.html#download">Download Bolikh, free</a>'
       '<a class="btn s" href="index.html#how">See how it works</a></div>')


def note(links):
    return (f'<p class="muted" style="font-size:.9rem;max-width:72ch">Checked on {CHECKED} against each app’s own website: {links}. '
            'Prices and features change, so look at their site before you decide. If something here is out of date, '
            'tell us at info@aazadsoftwaresolutions.in and we will correct it. All product names belong to their owners; '
            'Bolikh is not connected to any of them.</p>')


def app_page(key):
    p = APPS[key]
    url = f"{SITE}/{p['file']}"
    others = " · ".join(f'<a href="{APPS[k]["file"]}">Bolikh vs {APPS[k]["name"]}</a>' for k in ORDER if k != key)
    rows = [(r, [BOLIKH[i], p["cells"][i]]) for i, r in enumerate(ROWS)]
    main = f'''<main id="top">
<section class="g" style="padding-block:48px 96px">
  <div class="in" style="max-width:860px;display:grid;grid-template-columns:minmax(0,1fr);gap:44px">
    <div style="display:grid;gap:14px">
      <p class="mono muted"><a href="index.html" style="color:inherit;text-decoration:none">Bolikh</a> / <a href="compare.html" style="color:inherit;text-decoration:none">Compare</a> / {p["name"]}</p>
      <span class="eb">{p["name"]} alternative · free</span>
      <h1 class="h2">{p["h1"]}</h1>
      <p class="lead" style="max-width:68ch">{p["lead"]}</p>
      {CTA}
    </div>
    <div style="display:grid;gap:20px">
      <h2 class="sfh">Bolikh and {p["name"]}, side by side</h2>
      {table(["Bolikh", p["name"]], rows)}
    </div>
    <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,320px),1fr));gap:32px">
      <div style="display:grid;gap:14px;align-content:start"><h2 class="sfh">Choose Bolikh if</h2>{ul(p["ours"])}</div>
      <div style="display:grid;gap:14px;align-content:start"><h2 class="sfh">Choose {p["name"]} if</h2>{ul(p["theirs"])}</div>
    </div>
    <div class="sfaq" style="margin-top:0">
      <h2 class="sfh">Questions</h2>
      <div class="faq">
      {faq_html(p["faq"])}
      </div>
    </div>
    <div style="display:grid;gap:14px">
      {CTA}
      <p class="muted">More comparisons: <a href="compare.html">all apps in one table</a> · {others}</p>
      {note(f'<a href="{p["src"]}" target="_blank" rel="nofollow noopener">{p["name"]}</a>')}
    </div>
  </div>
</section>
</main>'''
    crumbs = [("Bolikh", SITE + "/"), ("Compare", f"{SITE}/compare.html"), (f"Bolikh vs {p['name']}", url)]
    write(p["file"], head(p["title"], p["desc"], url, crumbs, p["faq"]), main)


def hub():
    url = f"{SITE}/compare.html"
    title = "Bolikh vs Wispr Flow, Superwhisper, MacWhisper, VoiceInk, Handy"
    desc = ("How Bolikh compares with Wispr Flow, Superwhisper, MacWhisper, VoiceInk and Handy: price, systems, "
            "offline use and open source, in one table. Bolikh is a free offline dictation app for Mac, Windows "
            "and Linux.")
    names = [APPS[k]["name"] for k in ORDER]
    rows = [(r, [BOLIKH[i]] + [APPS[k]["cells"][i] for k in ORDER]) for i, r in enumerate(ROWS)]
    cards = "".join(
        f'<a class="vs-card" href="{APPS[k]["file"]}"><b>Bolikh vs {APPS[k]["name"]}</b>'
        f'<span>{APPS[k]["h1"]}</span></a>' for k in ORDER)
    links = ", ".join(f'<a href="{APPS[k]["src"]}" target="_blank" rel="nofollow noopener">{APPS[k]["name"]}</a>' for k in ORDER)
    main = f'''<main id="top">
<section class="g" style="padding-block:48px 96px">
  <div class="in" style="max-width:1080px;display:grid;grid-template-columns:minmax(0,1fr);gap:44px">
    <div style="display:grid;gap:14px">
      <p class="mono muted"><a href="index.html" style="color:inherit;text-decoration:none">Bolikh</a> / Compare</p>
      <span class="eb">Dictation apps compared</span>
      <h1 class="h2">How Bolikh compares with other dictation apps.</h1>
      <p class="lead" style="max-width:68ch">There are several good apps that type what you say. This page puts Bolikh next to {", ".join(names[:-1])} and {names[-1]}, using facts from each app’s own website, and says plainly where another app is the better choice.</p>
      {CTA}
    </div>
    <div style="display:grid;gap:20px">
      <h2 class="sfh">All six in one table</h2>
      {table(["Bolikh"] + names, rows)}
    </div>
    <div style="display:grid;gap:20px">
      <h2 class="sfh">Where Bolikh stands</h2>
      <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,300px),1fr));gap:22px 32px">
        <div style="display:grid;gap:6px"><h3 style="font-weight:600;font-size:1.08rem">What it does well</h3><p class="muted">It is free with no limits, open source, works offline, and is the same app on Mac, Windows and Linux. The text comes out tidy: no “um”, numbers as digits, and commands such as “new line” and “scratch that”.</p></div>
        <div style="display:grid;gap:6px"><h3 style="font-weight:600;font-size:1.08rem">What it doesn’t do</h3><p class="muted">No phone app, no AI rewriting, no transcribing of audio files, and no Hindi or other languages outside its 25 European ones. Intel Macs aren’t supported yet. If you need one of these, an app in the table will suit you better.</p></div>
      </div>
    </div>
    <div style="display:grid;gap:20px">
      <h2 class="sfh">One app at a time</h2>
      <div class="vs-cards">{cards}</div>
    </div>
    <div style="display:grid;gap:14px">
      <h2 class="sfh">And the dictation built into your computer</h2>
      <p class="muted" style="max-width:72ch">Macs have dictation built in, and it is fine for a quick sentence. Windows has voice typing (Windows key + H), which uses Microsoft’s online speech service. Most Linux desktops have nothing built in. Bolikh is for people who write by voice often: you hold a key for as long as you talk, fillers are removed, you can teach it your own words, and it keeps a history of what you said.</p>
    </div>
    <div class="sfaq" style="margin-top:0">
      <h2 class="sfh">Questions</h2>
      <div class="faq">
      {faq_html(HUB_FAQ)}
      </div>
    </div>
    <div style="display:grid;gap:14px">
      {CTA}
      {note(links)}
    </div>
  </div>
</section>
</main>'''
    write("compare.html", head(title, desc, url, [("Bolikh", SITE + "/"), ("Compare", url)], HUB_FAQ), main)


hub()
for k in ORDER:
    app_page(k)
