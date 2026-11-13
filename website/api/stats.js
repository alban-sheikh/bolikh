// GET /api/stats?key=STATS_KEY            → private stats page (HTML)
// GET /api/stats?key=STATS_KEY&format=json → the same numbers as JSON
// STATS_KEY is a password you choose and set in Vercel → Project → Settings → Environment Variables.
const { FILES, pipeline, day, redis } = require("./_counter");

const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
const n = (v) => Number(v || 0);

async function githubCounts() {
  try {
    const r = await fetch("https://api.github.com/repos/alban-sheikh/bolikh/releases?per_page=100", {
      headers: { "User-Agent": "bolikh-stats", Accept: "application/vnd.github+json" },
    });
    if (!r.ok) return null;
    const out = {};
    for (const rel of await r.json()) {
      for (const a of rel.assets || []) out[a.name] = (out[a.name] || 0) + a.download_count;
    }
    return out;
  } catch {
    return null;
  }
}

module.exports = async (req, res) => {
  const want = process.env.STATS_KEY;
  const given = (req.query && req.query.key) || "";
  res.setHeader("Cache-Control", "no-store");
  res.setHeader("X-Robots-Tag", "noindex");
  if (!want || given !== want) {
    res.statusCode = want ? 401 : 503;
    res.setHeader("Content-Type", "text/plain; charset=utf-8");
    return res.end(want ? "Not allowed. Add ?key=<your stats key> to the address." : "STATS_KEY is not set in Vercel yet.");
  }
  if (!redis()) {
    res.statusCode = 503;
    res.setHeader("Content-Type", "text/plain; charset=utf-8");
    return res.end("Upstash Redis is not connected to this Vercel project yet.");
  }

  const keys = Object.keys(FILES);
  const days = [...Array(30)].map((_, i) => day(new Date(Date.now() - i * 86400000)));
  const cmds = [
    ["GET", "dl:total"],
    ...keys.map((k) => ["GET", `dl:os:${k}`]),
    ["GET", "dl:src:site"],
    ["GET", "dl:src:github"],
    ...days.flatMap((d) => [["GET", `dl:day:${d}`], ...keys.map((k) => ["GET", `dl:day:${d}:${k}`])]),
  ];
  const [r, gh] = await Promise.all([pipeline(cmds, 4000), githubCounts()]);
  let i = 0;
  const total = n(r[i++]);
  const byOs = Object.fromEntries(keys.map((k) => [k, n(r[i++])]));
  const bySource = { site: n(r[i++]), github: n(r[i++]) };
  const daily = days.map((d) => ({ day: d, total: n(r[i++]), ...Object.fromEntries(keys.map((k) => [k, n(r[i++])])) }));
  const data = {
    total,
    byOs,
    linuxTotal: byOs.deb + byOs.rpm,
    bySource,
    last30Days: daily,
    githubReleaseDownloads: gh, // GitHub's own counter: includes the GitHub backup link and the copies our deploy workflow downloads (one per deploy)
    note: "Counts downloads started from bolikh.aazadsoftwaresolutions.in since counting began. Bots and link previews are excluded.",
  };

  if (req.query.format === "json") {
    res.setHeader("Content-Type", "application/json; charset=utf-8");
    return res.end(JSON.stringify(data, null, 2));
  }

  const row = (label, v, strong) => `<tr${strong ? ' class="t"' : ""}><td>${esc(label)}</td><td>${v.toLocaleString("en-IN")}</td></tr>`;
  const ghRows = gh
    ? Object.entries(gh).filter(([name]) => name.startsWith("Bolikh-")).map(([name, v]) => row(name, v)).join("")
    : `<tr><td colspan="2">GitHub numbers unavailable right now.</td></tr>`;
  const dayRows = daily
    .map((d) => `<tr><td>${d.day}</td>${["total", ...keys].map((k) => `<td>${d[k]}</td>`).join("")}</tr>`)
    .join("");

  res.setHeader("Content-Type", "text/html; charset=utf-8");
  res.end(`<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Bolikh downloads</title><meta name="robots" content="noindex">
<style>
:root{--bg:#1B1814;--s:#26221C;--r:#3A342C;--i:#F5EEE3;--m:#B6AC9A;--a:#EDB64A;color-scheme:dark}
@media (prefers-color-scheme:light){:root{--bg:#F3EDE3;--s:#FFFBF5;--r:#E2D9CA;--i:#17120C;--m:#6B6152;--a:#8A5A0E;color-scheme:light}}
body{margin:0;background:var(--bg);color:var(--i);font:15px/1.5 ui-sans-serif,system-ui,-apple-system,"Segoe UI",sans-serif;padding:24px max(16px,env(safe-area-inset-left))}
main{max-width:860px;margin:0 auto;display:grid;gap:22px}h1{font-size:1.6rem;margin:0}h2{font-size:1rem;margin:0 0 8px;color:var(--m);font-weight:600}
.big{display:grid;grid-template-columns:repeat(auto-fit,minmax(150px,1fr));gap:10px}.k{background:var(--s);border:1px solid var(--r);border-radius:14px;padding:14px 16px}
.k b{display:block;font-size:1.9rem;font-variant-numeric:tabular-nums;color:var(--a)}.k span{color:var(--m);font-size:.85rem}
table{width:100%;border-collapse:collapse;font-variant-numeric:tabular-nums;background:var(--s);border:1px solid var(--r);border-radius:12px;overflow:hidden}
td,th{padding:7px 12px;border-bottom:1px solid var(--r);text-align:right}td:first-child,th:first-child{text-align:left}th{color:var(--m);font-weight:600;font-size:.8rem}
tr.t td{font-weight:700}.wrap{overflow-x:auto}p{color:var(--m);margin:0;font-size:.9rem}
</style></head><body><main>
<h1>Bolikh downloads</h1>
<div class="big">
<div class="k"><b>${total.toLocaleString("en-IN")}</b><span>Total downloads</span></div>
<div class="k"><b>${byOs.mac.toLocaleString("en-IN")}</b><span>macOS</span></div>
<div class="k"><b>${byOs.win.toLocaleString("en-IN")}</b><span>Windows</span></div>
<div class="k"><b>${data.linuxTotal.toLocaleString("en-IN")}</b><span>Linux (.deb ${byOs.deb} · .rpm ${byOs.rpm})</span></div>
</div>
<section><h2>Where the file came from</h2><table>${row("This website", bySource.site)}${row("GitHub backup link", bySource.github)}${row("Total", total, true)}</table></section>
<section><h2>Last 30 days (India time)</h2><div class="wrap"><table><tr><th>Day</th><th>Total</th><th>macOS</th><th>Windows</th><th>.deb</th><th>.rpm</th></tr>${dayRows}</table></div></section>
<section><h2>GitHub's own download count (all versions; includes the backup link and one copy per website deploy)</h2><table>${ghRows}</table></section>
<p>${esc(data.note)} Visitor numbers: Vercel → project “bolikh” → Analytics. Raw data: add <code>&amp;format=json</code> to this address.</p>
</main></body></html>`);
};
