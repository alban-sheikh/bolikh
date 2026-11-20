#!/usr/bin/env python3
"""Builds website/mac.html, windows.html and linux.html.

Each page reuses the setup guide and the fixes for one system from website/index.html and the page shell
(head, header, footer, script) from website/privacy.html, so those stay the single source of truth.
Run again whenever the setup guide or the fixes on the home page change:  python3 scripts/build-os-pages.py
"""
import json, re, pathlib

SITE = "https://bolikh.aazadsoftwaresolutions.in"
web = pathlib.Path(__file__).resolve().parent.parent / "website"
home = (web / "index.html").read_text()
shell = (web / "privacy.html").read_text()

PAGES = {
    "mac": dict(
        file="mac.html", name="Mac", system="macOS", icon="i-apple",
        title="Dictation app for Mac: free, offline voice typing | Bolikh",
        desc="Bolikh is a free dictation app for Mac. Hold Right Option, speak, and your words are typed in any app. "
             "Works offline on Apple Silicon Macs; your voice never leaves your Mac.",
        h1="Dictation for Mac that works in every app, offline.",
        lead="Bolikh is a free dictation app for macOS. Hold the Right Option key, speak, and let go: your words are "
             "typed wherever your cursor is, in Mail, Slack, Notes, Word, your browser or a terminal. Speech is "
             "turned into text on your Mac, so it works without the internet and your voice is never uploaded.",
        req="Macs with Apple Silicon (M1 or newer), macOS 12 Monterey or later. Intel Macs aren’t supported yet.",
        buttons=[("mac", "Download for Mac", "p")],
        after="Open the .dmg and drag Bolikh to Applications.",
        points=[
            ("One key, any app", "Hold Right Option to talk and release to type. It works in any app where you can "
             "type, and you can change the key or switch to press-once-to-start in Settings."),
            ("Runs on your Mac’s own chip", "The speech model runs locally on Apple Silicon. After the one-time "
             "model download (about 670 MB) you can dictate on a plane or with Wi-Fi off."),
            ("Text that reads as written", "Bolikh removes “um” and “uh”, adds punctuation, writes numbers as "
             "digits and understands commands such as “new line” and “bullet point”."),
            ("Lives in the menu bar", "No window to manage. Bolikh waits in the menu bar and shows a small "
             "indicator while it listens."),
        ],
        perms="Bolikh needs two macOS permissions: Microphone, to hear you, and Accessibility, so the key works "
              "while you are in other apps and Bolikh can paste the text for you.",
    ),
    "win": dict(
        file="windows.html", name="Windows", system="Windows", icon="i-win",
        title="Voice typing for Windows 10 and 11: free, offline dictation | Bolikh",
        desc="Bolikh is a free voice typing app for Windows 10 and 11. Hold Right Ctrl, speak, and your words are "
             "typed in any app. Works offline; your voice never leaves your PC.",
        h1="Voice typing for Windows that works offline, in every app.",
        lead="Bolikh is a free dictation app for Windows 10 and 11. Hold the Right Ctrl key, speak, and let go: "
             "your words are typed wherever your cursor is, in Outlook, Word, Teams, Slack, your browser or a "
             "terminal. Speech is turned into text on your PC, so it works without the internet and your voice "
             "is never uploaded.",
        req="Windows 10 or 11, 64-bit.",
        buttons=[("win", "Download for Windows", "p")],
        after="Run the installer. No admin rights needed.",
        points=[
            ("One key, any app", "Hold Right Ctrl to talk and release to type. It works in any app where you can "
             "type, and you can change the key or switch to press-once-to-start in Settings."),
            ("No account, no internet", "There is nothing to sign in to. After the one-time speech model download "
             "(about 670 MB), dictation keeps working with the network switched off."),
            ("Text that reads as written", "Bolikh removes “um” and “uh”, adds punctuation, writes numbers as "
             "digits and understands commands such as “new line” and “bullet point”."),
            ("Installs for you only", "The installer doesn’t ask for administrator rights, so it also works on a "
             "work PC where you can’t install system-wide software."),
        ],
        perms="Windows needs no special permissions. Bolikh only needs to be able to use your microphone, which Windows "
              "allows for desktop apps by default.",
    ),
    "linux": dict(
        file="linux.html", name="Linux", system="Linux", icon="i-linux",
        title="Dictation for Linux: offline speech to text for Ubuntu, Debian and Fedora | Bolikh",
        desc="Bolikh is a free, open-source dictation app for Linux. Hold Right Ctrl, speak, and your words are "
             "typed in any app, on X11 and Wayland. Works offline on Ubuntu, Debian and Fedora.",
        h1="Dictation for Linux, offline, on X11 and Wayland.",
        lead="Bolikh is a free, open-source dictation app for Linux. Hold the Right Ctrl key, speak, and let go: "
             "your words are typed wherever your cursor is, in your browser, editor, chat app or terminal. Speech "
             "is turned into text on your own machine, so it works without the internet and your voice is never "
             "uploaded.",
        req="Ubuntu 24.04 or later, Debian 13, Fedora 40 or later. 64-bit Intel or AMD PCs, X11 and Wayland.",
        buttons=[("deb", "Download .deb (Ubuntu, Debian)", "p"), ("rpm", "Download .rpm (Fedora)", "s")],
        after="Install the package, then join the input group.",
        points=[
            ("X11 and Wayland", "Bolikh works on both. On Wayland it types through wtype or ydotool; on X11 it "
             "needs nothing extra."),
            ("Works in terminals", "Bolikh notices when the focused window is a terminal and pastes with "
             "Ctrl+Shift+V. You can also set the paste shortcut yourself in Settings."),
            ("Open source", "The full source is on GitHub under GPL-3.0, so you can read exactly what it does "
             "with your microphone and build it yourself."),
            ("Text that reads as written", "Bolikh removes “um” and “uh”, adds punctuation, writes numbers as "
             "digits and understands commands such as “new line” and “bullet point”."),
        ],
        perms="Bolikh reads the dictation key directly from the keyboard, so your user must be in the "
              "<code>input</code> group. The setup below shows the one command for that.",
    ),
}
ORDER = ["mac", "win", "linux"]


def between(text, start, end):
    a = text.index(start)
    return text[a:text.index(end, a)]


def guide(key):
    g = between(home, f'<ol class="guide" id="sp-{key}"', "</ol>") + "</ol>"
    g = re.sub(r'<ol class="guide"[^>]*>', '<ol class="guide">', g, count=1)
    return g.replace('href="#', 'href="index.html#')


def fixes(key):
    seg = between(home, 'id="fixes"', 'id="faq"')
    out = []
    for m in re.finditer(r'<details data-os="(\w+)">(.*?)</details>', seg, re.S):
        if m.group(1) == key:
            body = re.sub(r'<span class="ostag">.*?</span>', "", m.group(2))
            out.append("<details>" + body.replace('href="#', 'href="index.html#') + "</details>")
    assert out, key
    return "\n      ".join(out)


def page(key):
    p = PAGES[key]
    url = f"{SITE}/{p['file']}"
    s = shell

    # head
    a, b = s.index("<title>"), s.index('<meta name="theme-color"')
    ld = {"@context": "https://schema.org", "@graph": [
        {"@type": "SoftwareApplication", "name": "Bolikh", "url": url, "description": p["desc"],
         "applicationCategory": "UtilitiesApplication", "operatingSystem": p["req"],
         "image": f"{SITE}/og.png", "isAccessibleForFree": True,
         "offers": {"@type": "Offer", "price": "0", "priceCurrency": "USD"},
         "publisher": {"@type": "Organization", "name": "Aazad Software Solutions",
                       "url": "https://aazadsoftwaresolutions.in"}},
        {"@type": "BreadcrumbList", "itemListElement": [
            {"@type": "ListItem", "position": 1, "name": "Bolikh", "item": SITE + "/"},
            {"@type": "ListItem", "position": 2, "name": f"Bolikh for {p['name']}", "item": url}]}]}
    ld_json = json.dumps(ld, ensure_ascii=False, separators=(",", ":")).replace("</", "<\\/")
    og_title = p["title"].split(" | ")[0]
    head = f'''<title>{p["title"]}</title>
<meta name="claude:viewport-fit-sides" content="cover">
<meta name="description" content="{p["desc"]}">
<link rel="canonical" href="{url}">
<meta property="og:site_name" content="Bolikh">
<meta property="og:title" content="{og_title}">
<meta property="og:description" content="{p["desc"]}">
<meta property="og:type" content="website">
<meta property="og:url" content="{url}">
<meta property="og:image" content="{SITE}/og.png">
<meta property="og:image:width" content="1200">
<meta property="og:image:height" content="630">
<meta name="twitter:card" content="summary_large_image">
<script type="application/ld+json">{ld_json}</script>
'''
    s = s[:a] + head + s[b:]

    # body
    btns = "".join(f'<a class="btn {c}" rel="nofollow" href="/api/download/{f}">{label}</a>' for f, label, c in p["buttons"])
    points = "".join(f'<div style="display:grid;gap:6px"><h3 style="font-weight:600;font-size:1.08rem">{t}</h3>'
                     f'<p class="muted">{d}</p></div>' for t, d in p["points"])
    others = " · ".join(f'<a href="{PAGES[k]["file"]}">Bolikh for {PAGES[k]["name"]}</a>' for k in ORDER if k != key)
    main = f'''<main id="top">
<section class="g" style="padding-block:48px 96px">
  <div class="in" style="max-width:860px;display:grid;grid-template-columns:minmax(0,1fr);gap:44px">
    <div style="display:grid;gap:14px">
      <p class="mono muted"><a href="index.html" style="color:inherit;text-decoration:none">Bolikh</a> / {p["name"]}</p>
      <span class="eb">Bolikh for {p["system"]} · free</span>
      <h1 class="h2">{p["h1"]}</h1>
      <p class="lead">{p["lead"]}</p>
    </div>
    <article class="os" style="max-width:520px">
      <div class="os-h"><svg aria-hidden="true"><use href="#{p["icon"]}"/></svg><h3>{p["system"]}</h3></div>
      <p class="req">{p["req"]}</p>
      <div class="btns">{btns}</div>
      <p class="req">{p["after"]}</p>
    </article>
    <div style="display:grid;gap:20px">
      <h2 class="sfh">Why use Bolikh on {p["name"]}</h2>
      <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,300px),1fr));gap:22px 32px">{points}</div>
    </div>
    <div style="display:grid;gap:20px">
      <h2 class="sfh">Set up Bolikh on {p["name"]}</h2>
      <p class="muted" style="max-width:68ch">{p["perms"]}</p>
      {guide(key)}
    </div>
    <div class="sfaq" style="margin-top:0">
      <h2 class="sfh">Problems and fixes on {p["name"]}</h2>
      <div class="faq">
      {fixes(key)}
      </div>
      <p class="muted">More answers, including microphone, key and update questions, are in the <a href="index.html#fixes">full list of fixes</a>.</p>
    </div>
    <div style="display:grid;gap:14px">
      <div style="display:flex;gap:10px;flex-wrap:wrap"><a class="btn p" href="index.html">See everything Bolikh does</a><a class="btn s" href="index.html#faq">Questions and answers</a></div>
      <p class="muted">Other systems: {others}</p>
    </div>
  </div>
</section>
</main>'''
    a, b = s.index('<main id="top">'), s.index("</main>") + len("</main>")
    s = s[:a] + main + s[b:]
    (web / p["file"]).write_text(s)
    print("wrote", p["file"], len(s))


for k in ORDER:
    page(k)
