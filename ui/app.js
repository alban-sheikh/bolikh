const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let status = null;
let history = [];

const $ = (s) => document.querySelector(s);
const el = (tag, props = {}, ...kids) => {
  const n = Object.assign(document.createElement(tag), props);
  for (const k of kids) n.append(k);
  return n;
};
const icon = (id) => {
  const s = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  const u = document.createElementNS("http://www.w3.org/2000/svg", "use");
  u.setAttribute("href", "#" + id);
  s.append(u);
  return s;
};

const MODE_HELP = {
  smart: "Hold to talk, release to type. Quick-tap for hands-free.",
  hold: "Recording lasts only while the key is held.",
  toggle: "Tap to start, tap again to stop.",
};

// ---------- Navigation ----------
document.querySelectorAll(".nav button").forEach((b) =>
  b.addEventListener("click", () => showPage(b.dataset.page))
);
function showPage(name) {
  document.querySelectorAll(".nav button").forEach((b) => b.classList.toggle("active", b.dataset.page === name));
  document.querySelectorAll(".page").forEach((p) => p.classList.toggle("active", p.id === "page-" + name));
  if (name === "history") loadHistory();
}

// ---------- Settings ----------
async function save(patch) {
  status.settings = { ...status.settings, ...patch };
  await invoke("save_settings", { settings: status.settings });
}

document.querySelectorAll("input[data-key]").forEach((input) =>
  input.addEventListener("change", () => save({ [input.dataset.key]: input.checked }))
);
$("#trigger").addEventListener("change", (e) => save({ trigger: e.target.value }));
document.querySelectorAll("#mode button").forEach((b) =>
  b.addEventListener("click", () => save({ mode: b.dataset.v }))
);
$("#autostart").addEventListener("change", async (e) => {
  e.target.checked = await invoke("set_autostart", { enabled: e.target.checked });
});

// ---------- Dictionary ----------
function addReplacement() {
  const from = $("#dict-from").value.trim();
  const to = $("#dict-to").value;
  if (!from) return;
  save({ replacements: [...status.settings.replacements, { from, to }] });
  $("#dict-from").value = "";
  $("#dict-to").value = "";
  $("#dict-from").focus();
}
$("#dict-add").addEventListener("click", addReplacement);
$("#dict-to").addEventListener("keydown", (e) => e.key === "Enter" && addReplacement());
$("#dict-from").addEventListener("keydown", (e) => e.key === "Enter" && $("#dict-to").focus());

function renderDictionary() {
  const list = $("#dict-list");
  list.replaceChildren();
  const items = status.settings.replacements;
  if (!items.length) {
    list.append(el("div", { className: "empty", textContent: 'No entries yet. Example: "azad" → "Aazad"' }));
    return;
  }
  items.forEach((r, i) => {
    const del = el("button", { className: "icon-btn", title: "Remove" }, icon("i-trash"));
    del.onclick = () => save({ replacements: items.filter((_, j) => j !== i) });
    list.append(
      el("div", { className: "item" },
        el("div", { className: "grow" },
          el("span", { textContent: r.from }),
          el("span", { className: "arrow", textContent: "  →  " }),
          el("b", { textContent: r.to || "(remove)" })),
        del)
    );
  });
}

// ---------- History ----------
async function loadHistory() {
  history = await invoke("get_history");
  renderHistory();
  renderStats();
}
function timeAgo(ts) {
  const s = Math.max(1, Math.round((Date.now() - ts) / 1000));
  if (s < 60) return "just now";
  const m = Math.round(s / 60);
  if (m < 60) return `${m} min ago`;
  const h = Math.round(m / 60);
  if (h < 24) return `${h} h ago`;
  return new Date(ts).toLocaleDateString();
}
function renderHistory() {
  const list = $("#hist-list");
  list.replaceChildren();
  const words = history.reduce((n, h) => n + h.text.split(/\s+/).filter(Boolean).length, 0);
  $("#hist-summary").textContent = `${history.length} dictations · ${words.toLocaleString()} words`;
  $("#hist-clear").disabled = !history.length;
  if (!history.length) {
    list.append(el("div", { className: "empty", textContent: "Nothing dictated yet." }));
    return;
  }
  for (const h of history) {
    const copy = el("button", { className: "icon-btn", title: "Copy" }, icon("i-copy"));
    copy.onclick = () => invoke("copy_text", { text: h.text });
    list.append(
      el("div", { className: "item" },
        el("div", { className: "grow" },
          el("div", { className: "txt selectable", textContent: h.text }),
          el("div", { className: "meta", textContent: `${timeAgo(h.ts)} · ${h.duration.toFixed(1)}s` })),
        copy)
    );
  }
}
$("#hist-clear").addEventListener("click", async () => {
  await invoke("clear_history");
  loadHistory();
});
function renderStats() {
  const words = history.reduce((n, h) => n + h.text.split(/\s+/).filter(Boolean).length, 0);
  const spoken = history.reduce((n, h) => n + h.duration, 0) / 60;
  const typed = words / 40; // average typing speed ~40 wpm
  $("#st-words").textContent = words.toLocaleString();
  $("#st-sessions").textContent = history.length.toLocaleString();
  $("#st-saved").textContent = `${Math.max(0, Math.round(typed - spoken))} min`;
}

// ---------- Status rendering ----------
function modelText(m) {
  switch (m.kind) {
    case "ready": return ["good", "Ready"];
    case "downloading": return ["warn", `Downloading ${Math.round(m.progress * 100)}%`];
    case "loading": return ["warn", "Preparing…"];
    case "failed": return ["bad", "Failed"];
    default: return ["warn", "Not loaded"];
  }
}

function render() {
  if (!status) return;
  const s = status.settings;
  document.body.classList.add("os-" + status.permissions.os);
  document.querySelectorAll("[data-trigger]").forEach((n) => (n.textContent = status.triggerLabel));

  // Settings controls
  const trig = $("#trigger");
  if (trig.options.length !== status.triggers.length) {
    trig.replaceChildren(...status.triggers.map((t) => el("option", { value: t.id, textContent: t.label })));
  }
  if (document.activeElement !== trig) trig.value = s.trigger;
  document.querySelectorAll("#mode button").forEach((b) => b.classList.toggle("on", b.dataset.v === s.mode));
  $("#mode-help").textContent = MODE_HELP[s.mode];
  document.querySelectorAll("input[data-key]").forEach((i) => (i.checked = !!s[i.dataset.key]));
  $("#autostart").checked = status.autostart;
  $("#about-version").textContent = `Version ${status.version}`;
  renderDictionary();

  // Status rows
  const rows = $("#status-rows");
  rows.replaceChildren();
  const [mk, mt] = modelText(status.model);
  const addRow = (title, sub, kind, label, action) => {
    const right = el("div", { style: "display:flex;gap:10px;align-items:center" }, el("span", { className: "badge " + kind, textContent: label }));
    if (action) right.prepend(action);
    rows.append(el("div", { className: "row" }, el("div", { className: "label" }, el("b", { textContent: title }), el("span", { textContent: sub })), right));
  };
  addRow("Speech model", "NVIDIA Parakeet TDT 0.6B v3 · runs locally", mk, mt,
    status.model.kind === "failed" ? retryButton() : null);
  const p = status.permissions;
  if (p.os === "macos") addRow("Accessibility", "Needed for the hotkey and to type into apps", p.accessibility ? "good" : "bad", p.accessibility ? "Granted" : "Not granted");
  if (p.os === "linux") {
    addRow("Keyboard access", "Membership of the 'input' group", p.inputDevices ? "good" : "bad", p.inputDevices ? "OK" : "Missing");
    if (p.wayland) addRow("Wayland typing tool", "wtype or ydotool", p.waylandPasteTool ? "good" : "bad", p.waylandPasteTool ? "Found" : "Missing");
  }

  renderHero();
  renderOnboarding();
}

function retryButton() {
  const b = el("button", { className: "btn small", textContent: "Retry" });
  b.onclick = () => invoke("retry_model");
  return b;
}

function renderHero() {
  const m = status.model;
  const dot = $("#hero-dot");
  const prog = $("#hero-progress");
  const actions = $("#hero-actions");
  actions.style.display = "none";
  prog.style.display = "none";
  if (status.recording) {
    dot.className = "dot live";
    $("#hero-state").textContent = "Listening…";
  } else if (m.kind === "ready" && status.permissions.accessibility && status.permissions.inputDevices) {
    dot.className = "dot good";
    $("#hero-state").textContent = "Ready";
  } else if (m.kind === "downloading") {
    dot.className = "dot warn";
    $("#hero-state").textContent = `Downloading speech model (${status.modelSizeMb} MB, one time)… ${Math.round(m.progress * 100)}%`;
    prog.style.display = "block";
    $("#hero-bar").style.width = `${m.progress * 100}%`;
  } else if (m.kind === "failed") {
    dot.className = "dot bad";
    $("#hero-state").textContent = "Speech model failed: " + m.error;
    actions.replaceChildren(retryButton());
    actions.style.display = "block";
  } else if (m.kind === "ready") {
    dot.className = "dot warn";
    $("#hero-state").textContent = "Almost ready: finish setup";
    const b = el("button", { className: "btn small", textContent: "Finish setup" });
    b.onclick = () => openOnboarding(true);
    actions.replaceChildren(b);
    actions.style.display = "block";
  } else {
    dot.className = "dot warn";
    $("#hero-state").textContent = "Preparing speech model…";
  }
}

// ---------- Onboarding ----------
let onboardingForced = false;
let micState = null; // null | "testing" | number (peak) | string (error)

function openOnboarding(force) {
  onboardingForced = force;
  renderOnboarding();
}

let lastOnboardingKey = "";

function renderOnboarding() {
  const p = status.permissions;
  const show = onboardingForced || !status.settings.onboarded;
  $("#onboarding").classList.toggle("show", show);
  if (!show) return;
  // Only rebuild when something visible changed, so typing in the test box isn't disturbed.
  const m0 = status.model;
  const key = JSON.stringify([micState, p, m0.kind, m0.kind === "downloading" ? Math.round(m0.progress * 100) : 0, status.triggerLabel]);
  if (key === lastOnboardingKey) return;
  lastOnboardingKey = key;

  const steps = [];
  const step = (n, done, title, body, action) => {
    const card = el("div", { className: "card step" + (done ? " done" : "") },
      el("div", { className: "num", textContent: done ? "✓" : n }),
      el("div", {}, el("b", { textContent: title }), ...[].concat(body)),
      action || el("span"));
    steps.push(card);
  };

  // 1. Microphone
  let micBody = el("p", { textContent: "Click Test and say something. Audio stays in memory and is never saved or sent." });
  if (micState === "testing") micBody = el("p", { textContent: "Listening… say a few words." });
  else if (typeof micState === "number") micBody = el("p", { textContent: micState > 0.02 ? "Microphone works." : "Very quiet. Check your input device, then test again." });
  else if (typeof micState === "string") micBody = el("p", { textContent: micState });
  const micBtn = el("button", { className: "btn small", textContent: micState === "testing" ? "Listening…" : "Test", disabled: micState === "testing" });
  micBtn.onclick = async () => {
    micState = "testing"; renderOnboarding();
    try { micState = await invoke("test_microphone"); } catch (e) { micState = String(e); }
    renderOnboarding();
  };
  step(1, typeof micState === "number" && micState > 0.02, "Microphone", micBody, micBtn);

  // 2. OS permissions
  if (p.os === "macos") {
    const b = el("button", { className: "btn small", textContent: "Open Settings" });
    b.onclick = () => invoke("open_permission_settings");
    step(2, p.accessibility, "Accessibility",
      el("p", { textContent: "Lets Aazad Voice hear the shortcut key and type into the app you're using. Turn on Aazad Voice in System Settings → Privacy & Security → Accessibility." }),
      p.accessibility ? null : b);
  } else if (p.os === "linux") {
    const body = [el("p", { textContent: "The global shortcut reads your keyboard directly, which needs the 'input' group. Run this, then log out and back in:" }),
      el("span", { className: "code", textContent: "sudo usermod -aG input $USER" })];
    if (p.wayland) {
      body.push(el("p", { style: "margin-top:10px", textContent: "On Wayland, a helper is needed to type the text. Use wtype on KDE, Sway and Hyprland, or ydotool on GNOME:" }),
        el("span", { className: "code", textContent: "sudo apt install wtype    # or: sudo apt install ydotool" }));
    }
    step(2, p.inputDevices && (!p.wayland || p.waylandPasteTool), "Keyboard access", body);
  } else {
    step(2, true, "No extra permissions needed", el("p", { textContent: "Windows lets Aazad Voice use the shortcut and type right away." }));
  }

  // 3. Model
  const m = status.model;
  const modelBody = [el("p", { textContent: `Downloads NVIDIA Parakeet v3 (${status.modelSizeMb} MB) once. After that everything works offline.` })];
  if (m.kind === "downloading") {
    const bar = el("div", { className: "progress" }, el("i", { style: `width:${m.progress * 100}%` }));
    modelBody.push(bar, el("p", { textContent: `${Math.round(m.progress * 100)}%` }));
  } else if (m.kind === "loading") modelBody.push(el("p", { textContent: "Preparing…" }));
  else if (m.kind === "failed") modelBody.push(el("p", { style: "color:var(--bad)", textContent: m.error }));
  step(3, m.kind === "ready", "Speech model", modelBody, m.kind === "failed" ? retryButton() : null);

  // 4. Try it
  const allReady = m.kind === "ready" && p.accessibility && p.inputDevices;
  const area = el("textarea", { className: "try", placeholder: `Click here, hold ${status.triggerLabel}, say "Hello world", then release.` });
  const prev = document.querySelector("#ob-steps textarea");
  if (prev) area.value = prev.value;
  step(4, false, "Try it", [el("p", { textContent: allReady ? "Everything's set. Give it a go:" : "Available once the steps above are done." }), allReady ? area : el("span")]);

  const holder = $("#ob-steps");
  const focused = document.activeElement === prev;
  holder.replaceChildren(...steps);
  if (focused) area.focus();

  $("#ob-hint").replaceChildren("Then hold ", el("span", { className: "kbd", textContent: status.triggerLabel }), " anywhere to dictate.");
  $("#ob-done").textContent = allReady ? "Start using Aazad Voice" : "Finish later";
}

$("#ob-done").addEventListener("click", async () => {
  onboardingForced = false;
  await save({ onboarded: true });
  render();
});

// ---------- Live updates ----------
async function refresh() {
  status = await invoke("get_status");
  render();
}
listen("status", (e) => { status = e.payload; render(); });
listen("model", (e) => { if (status) { status.model = e.payload; renderHero(); renderOnboarding(); } });
listen("history", () => loadHistory());
// Permissions can change outside the app (System Settings, group membership).
setInterval(() => { if (document.visibilityState === "visible") refresh(); }, 2000);

refresh().then(loadHistory);
