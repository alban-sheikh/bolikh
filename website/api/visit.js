// POST /api/visit?p=<page>&n=<1|0>  → counts one page view; n=1 also counts one visit (the first
// page of a browser tab's session). Only numbers per day and per page are stored: no cookie,
// no IP address, no identifier. Bots and anything that is not a common web browser are ignored.
const { pipeline, day, isPerson } = require("./_counter");

const PAGES = ["home", "mac", "windows", "linux", "privacy", "compare", "wispr-flow-alternative",
  "superwhisper-alternative", "macwhisper-alternative", "voiceink-alternative", "handy-alternative"];

module.exports = async (req, res) => {
  res.setHeader("Cache-Control", "no-store");
  const page = String((req.query && req.query.p) || "");
  const ua = req.headers["user-agent"] || "";
  if (req.method === "POST" && PAGES.includes(page) && isPerson(ua)) {
    const d = day();
    const cmds = [["INCR", "pv:total"], ["INCR", `pv:day:${d}`], ["INCR", `pv:page:${page}`]];
    if (req.query.n === "1") cmds.push(["INCR", "vis:total"], ["INCR", `vis:day:${d}`]);
    try {
      await pipeline(cmds);
    } catch (e) {
      console.error("visit count failed:", e.message);
    }
  }
  res.statusCode = 204;
  res.end();
};
module.exports.PAGES = PAGES;
