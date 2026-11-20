// Shared helpers for the download counter. Counts live in Upstash Redis (free tier, added to the
// Vercel project through the Marketplace, which provides the REST URL and token as env vars).
// Only plain numbers are stored: no IP addresses, user agents or anything else about the visitor.

const FILES = {
  mac: { os: "macOS", file: "Bolikh-macos-apple-silicon.dmg" },
  win: { os: "Windows", file: "Bolikh-windows-x64-setup.exe" },
  deb: { os: "Linux (.deb)", file: "Bolikh-linux-amd64.deb" },
  rpm: { os: "Linux (.rpm)", file: "Bolikh-linux-x86_64.rpm" },
};
const GITHUB = "https://github.com/alban-sheikh/bolikh/releases/latest/download/";

// On this website the installers carry the version in their name. The deploy workflow renames
// them and writes the version to _release.json; without that file the plain names are used.
let VERSION = "";
try {
  VERSION = String(require("./_release.json").version || "");
} catch {}
for (const f of Object.values(FILES)) f.local = VERSION ? f.file.replace(/^Bolikh-/, `Bolikh-${VERSION}-`) : f.file;

function redis() {
  const url = process.env.KV_REST_API_URL || process.env.UPSTASH_REDIS_REST_URL;
  const token = process.env.KV_REST_API_TOKEN || process.env.UPSTASH_REDIS_REST_TOKEN;
  return url && token ? { url: url.replace(/\/$/, ""), token } : null;
}

// Runs several Redis commands in one request. Returns the results, or null if Redis isn't set up.
async function pipeline(commands, timeoutMs = 2000) {
  const r = redis();
  if (!r) return null;
  const ctrl = new AbortController();
  const t = setTimeout(() => ctrl.abort(), timeoutMs);
  try {
    const res = await fetch(`${r.url}/pipeline`, {
      method: "POST",
      headers: { Authorization: `Bearer ${r.token}`, "Content-Type": "application/json" },
      body: JSON.stringify(commands),
      signal: ctrl.signal,
    });
    if (!res.ok) throw new Error(`redis ${res.status}`);
    return (await res.json()).map((x) => x.result);
  } finally {
    clearTimeout(t);
  }
}

// Date in India time, so "today" matches the team's day.
function day(d = new Date()) {
  return new Date(d.getTime() + 5.5 * 3600 * 1000).toISOString().slice(0, 10);
}

const BOT = /bot|crawl|spider|slurp|preview|facebookexternalhit|whatsapp|telegram|discord|slack|curl|wget|python|headless|monitor|uptime|lighthouse|inspection|google|bing|yandex|baidu|duckduck|archiver|scan|scrapy|phantom|selenium|puppeteer|playwright|http-client|httpclient|axios|okhttp|node-fetch|undici|java\/|go-http|libwww|postman|insomnia|ahrefs|semrush|vercel/i;

// Which kind of browser a request came from, as a coarse label. Anything that isn't one of the
// common browsers is "other", which is how scripts and unknown crawlers show up in the stats.
function browser(ua) {
  if (!/^Mozilla\/5\.0 /.test(ua)) return "other";
  if (/Edg\//.test(ua)) return "edge";
  if (/Firefox\//.test(ua)) return "firefox";
  if (/OPR\/|Opera/.test(ua)) return "other";
  if (/Chrome\/|CriOS\//.test(ua)) return "chrome";
  if (/Safari\//.test(ua)) return "safari";
  return "other";
}
const BROWSERS = ["chrome", "safari", "firefox", "edge", "other"];

// A real person in a browser: not a known bot or tool, and one of the common browsers.
function isPerson(ua) {
  return !BOT.test(ua) && browser(ua) !== "other";
}

module.exports = { FILES, GITHUB, pipeline, day, BOT, redis, browser, BROWSERS, isPerson };
