// GET /api/download/mac  (or win, deb, rpm)          → counts one download, then sends the file from this site
// GET /api/download/mac?from=github                   → counts it under "GitHub", then sends the file from GitHub
// HEAD requests, link previews and bots are not counted. If counting fails the download still starts.
const { FILES, GITHUB, pipeline, day, BOT } = require("../_counter");

module.exports = async (req, res) => {
  const key = String((req.query && req.query.os) || "").toLowerCase();
  const entry = FILES[key];
  if (!entry) {
    res.statusCode = 404;
    res.setHeader("Content-Type", "text/plain; charset=utf-8");
    return res.end("Unknown download. Use mac, win, deb or rpm.");
  }
  const fromGithub = req.query && req.query.from === "github";
  const target = fromGithub ? GITHUB + entry.file : "/downloads/" + entry.file;

  const ua = req.headers["user-agent"] || "";
  if (req.method === "GET" && !BOT.test(ua)) {
    const d = day();
    const src = fromGithub ? "github" : "site";
    try {
      await pipeline([
        ["INCR", "dl:total"],
        ["INCR", `dl:os:${key}`],
        ["INCR", `dl:src:${src}`],
        ["INCR", `dl:day:${d}`],
        ["INCR", `dl:day:${d}:${key}`],
      ]);
    } catch (e) {
      console.error("download count failed:", e.message);
    }
  }

  res.statusCode = 302;
  res.setHeader("Location", target);
  res.setHeader("Cache-Control", "no-store");
  res.end();
};
