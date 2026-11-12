//! Bolikh: private, offline dictation for macOS, Windows and Linux.
//!
//! Flow: global hotkey → microphone capture → Parakeet v3 (local ONNX) →
//! clean-up (fillers, dictionary, voice commands) → paste into the focused app.

mod audio;
mod engine;
mod history;
mod hotkey;
mod insert;
mod permissions;
mod settings;
mod text;

use engine::{Engine, ModelState};
use serde::Serialize;
use settings::{Mode, Settings, Trigger};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_autostart::ManagerExt;

const HUD_W: f64 = 380.0;
const HUD_H: f64 = 84.0;
/// macOS keeps the transparent HUD permanently ordered-in (click-through) so it
/// never activates the app; elsewhere it is shown/hidden on demand.
const HUD_ALWAYS_VISIBLE: bool = cfg!(target_os = "macos");

#[derive(Default)]
struct Session {
    recording: bool,
    hands_free: bool,
    started: Option<Instant>,
}

struct Tray {
    status: MenuItem<tauri::Wry>,
    toggle: MenuItem<tauri::Wry>,
}

pub struct Core {
    app: AppHandle,
    settings_path: PathBuf,
    settings: RwLock<Settings>,
    model: Mutex<ModelState>,
    engine: Engine,
    recorder: audio::Recorder,
    session: Mutex<Session>,
    history: Mutex<history::History>,
    tray: Mutex<Option<Tray>>,
    hud_gen: AtomicU64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct HudMsg {
    phase: &'static str,
    text: Option<String>,
    sound: Option<&'static str>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TriggerOption {
    id: Trigger,
    label: &'static str,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AppStatus {
    settings: Settings,
    model: ModelState,
    recording: bool,
    permissions: permissions::Permissions,
    triggers: Vec<TriggerOption>,
    trigger_label: &'static str,
    autostart: bool,
    version: String,
    model_size_mb: u64,
}

impl Core {
    fn settings(&self) -> Settings {
        self.settings.read().unwrap().clone()
    }

    fn status(&self) -> AppStatus {
        let settings = self.settings();
        let mut triggers = vec![Trigger::OptRight, Trigger::CtrlRight, Trigger::CmdRight, Trigger::ShiftRight, Trigger::OptLeft];
        if cfg!(target_os = "macos") {
            triggers.push(Trigger::Fn);
        }
        AppStatus {
            trigger_label: settings.trigger.label(),
            settings,
            model: self.model.lock().unwrap().clone(),
            recording: self.session.lock().unwrap().recording,
            permissions: permissions::check(),
            triggers: triggers.into_iter().map(|t| TriggerOption { id: t, label: t.label() }).collect(),
            autostart: self.app.autolaunch().is_enabled().unwrap_or(false),
            version: self.app.package_info().version.to_string(),
            model_size_mb: engine::total_download_bytes() / 1_000_000,
        }
    }

    fn emit_status(&self) {
        let _ = self.app.emit("status", self.status());
        self.refresh_tray();
    }

    fn set_model_state(&self, s: ModelState) {
        let changed_kind = {
            let mut m = self.model.lock().unwrap();
            let changed = std::mem::discriminant(&*m) != std::mem::discriminant(&s);
            *m = s.clone();
            changed
        };
        let _ = self.app.emit("model", s);
        if changed_kind {
            self.emit_status();
        }
    }

    fn prepare_model(self: &Arc<Self>) {
        let core = self.clone();
        std::thread::spawn(move || {
            let c = core.clone();
            if let Err(e) = core.engine.prepare(&move |s| c.set_model_state(s)) {
                log::error!("model: {e}");
                core.set_model_state(ModelState::Failed { error: e });
            }
        });
    }

    fn model_message(&self) -> String {
        match &*self.model.lock().unwrap() {
            ModelState::NotLoaded => "Speech model not loaded".into(),
            ModelState::Downloading { progress } => format!("Downloading speech model… {}%", (progress * 100.0) as u32),
            ModelState::Loading => "Preparing speech model…".into(),
            ModelState::Ready => "Ready".into(),
            ModelState::Failed { .. } => "Speech model failed to load. Open Bolikh to retry.".into(),
        }
    }

    // ---------- HUD ----------

    fn hud(&self, phase: &'static str, text: Option<String>, sound: Option<&'static str>) {
        let sound = if self.settings().play_sounds { sound } else { None };
        let generation = self.hud_gen.fetch_add(1, Ordering::SeqCst) + 1;
        if !HUD_ALWAYS_VISIBLE {
            if let Some(w) = self.app.get_webview_window("hud") {
                if phase != "hidden" {
                    position_hud(&self.app);
                    let _ = w.show();
                    // Re-applied after every show: hiding a GTK window can drop its input shape.
                    let _ = w.set_ignore_cursor_events(true);
                }
                // Hide after the fade-out (or after a message has been read), unless a
                // newer HUD state arrived in the meantime.
                let delay = match phase {
                    "hidden" => Some(260),
                    "message" => Some(2500),
                    _ => None,
                };
                if let Some(ms) = delay {
                    let app = self.app.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_millis(ms));
                        let core = app.state::<Arc<Core>>();
                        if core.hud_gen.load(Ordering::SeqCst) == generation {
                            let _ = w.hide();
                        }
                    });
                }
            }
        } else if phase != "hidden" {
            position_hud(&self.app);
        }
        let _ = self.app.emit_to("hud", "hud", HudMsg { phase, text, sound });
    }

    fn hud_message(&self, text: impl Into<String>) {
        self.hud("message", Some(text.into()), None);
    }

    // ---------- Dictation ----------

    fn start_recording(&self, hands_free: bool) {
        {
            let s = self.session.lock().unwrap();
            if s.recording {
                return;
            }
        }
        if !self.engine.is_ready() {
            self.hud_message(self.model_message());
            return;
        }
        if let Err(e) = self.recorder.start() {
            self.hud_message(e);
            return;
        }
        {
            let mut s = self.session.lock().unwrap();
            s.recording = true;
            s.hands_free = hands_free;
            s.started = Some(Instant::now());
        }
        self.hud(if hands_free { "handsFree" } else { "listening" }, None, Some("start"));
        self.emit_status();
    }

    fn stop_recording(self: &Arc<Self>) {
        let started = {
            let mut s = self.session.lock().unwrap();
            if !s.recording {
                return;
            }
            s.recording = false;
            s.started.take()
        };
        let (samples, rate) = self.recorder.stop();
        let duration = started.map(|t| t.elapsed().as_secs_f32()).unwrap_or(0.0);
        self.emit_status();

        if duration < 0.25 || samples.len() < (rate as usize / 4) {
            self.hud("hidden", None, None);
            return;
        }
        self.hud("transcribing", None, Some("stop"));

        let core = self.clone();
        std::thread::spawn(move || {
            let audio16k = audio::resample_to_16k(&samples, rate);
            let raw = match core.engine.transcribe_16k(&audio16k) {
                Ok(t) => t,
                Err(e) => {
                    core.hud_message(e);
                    return;
                }
            };
            let settings = core.settings();
            let text = text::process(&raw, &settings);
            if text.trim().is_empty() {
                core.hud_message("Didn't catch that");
                return;
            }
            core.hud("hidden", None, None);
            core.insert(text.clone(), settings.restore_clipboard);
            if settings.keep_history {
                let ts = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
                core.history.lock().unwrap().add(history::Item { text: text.trim().to_string(), ts, duration });
                let _ = core.app.emit("history", ());
            }
        });
    }

    fn cancel_recording(&self) {
        {
            let mut s = self.session.lock().unwrap();
            if !s.recording {
                return;
            }
            s.recording = false;
        }
        let _ = self.recorder.stop();
        self.hud_message("Cancelled");
        self.emit_status();
    }

    fn insert(self: &Arc<Self>, text: String, restore: bool) {
        let core = self.clone();
        // Keyboard simulation must run on the main thread on macOS.
        let _ = self.app.run_on_main_thread(move || {
            let saved = if restore { Some(insert::snapshot()) } else { None };
            match insert::paste(&text) {
                insert::Outcome::Pasted => {
                    if let Some(saved) = saved {
                        let app = core.app.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(Duration::from_millis(700));
                            let _ = app.run_on_main_thread(move || insert::restore(saved, &text));
                        });
                    }
                }
                insert::Outcome::CopiedOnly(reason) => {
                    log::warn!("auto-paste unavailable: {reason}");
                    core.hud_message("Copied: press paste to insert");
                }
            }
        });
    }

    // ---------- Tray ----------

    fn refresh_tray(&self) {
        let recording = self.session.lock().unwrap().recording;
        let status = if recording {
            "Listening…".to_string()
        } else if self.engine.is_ready() {
            format!("Ready: hold {} to dictate", self.settings().trigger.label())
        } else {
            self.model_message()
        };
        if let Some(t) = &*self.tray.lock().unwrap() {
            let _ = t.status.set_text(status);
            let _ = t.toggle.set_text(if recording { "Stop dictation" } else { "Start dictation (hands-free)" });
        }
    }
}

impl hotkey::Handler for Arc<Core> {
    fn trigger(&self) -> (Trigger, Mode) {
        let s = self.settings.read().unwrap();
        (s.trigger, s.mode)
    }
    fn is_recording(&self) -> bool {
        self.session.lock().unwrap().recording
    }
    fn start(&self, hands_free: bool) {
        self.start_recording(hands_free)
    }
    fn stop(&self) {
        self.stop_recording()
    }
    fn cancel(&self) {
        self.cancel_recording()
    }
    fn set_hands_free(&self) {
        let mut s = self.session.lock().unwrap();
        if s.recording && !s.hands_free {
            s.hands_free = true;
            drop(s);
            self.hud("handsFree", None, None);
        }
    }
}

fn position_hud(app: &AppHandle) {
    let Some(w) = app.get_webview_window("hud") else { return };
    // Prefer the monitor under the cursor, fall back to the primary one.
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| w.primary_monitor().ok().flatten());
    if let Some(m) = monitor {
        let scale = m.scale_factor();
        let area = m.work_area();
        let x = area.position.x as f64 + (area.size.width as f64 - HUD_W * scale) / 2.0;
        let y = area.position.y as f64 + area.size.height as f64 - (HUD_H + 24.0) * scale;
        let _ = w.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
    }
}

fn create_hud(app: &tauri::App) -> tauri::Result<()> {
    // Transparent, click-through, never focusable.
    let hud = WebviewWindowBuilder::new(app, "hud", WebviewUrl::App("hud.html".into()))
        .title("Bolikh HUD")
        .inner_size(HUD_W, HUD_H)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible_on_all_workspaces(true)
        .focused(false)
        .focusable(false)
        .visible(HUD_ALWAYS_VISIBLE)
        .build()?;
    // Only on a visible window: on Linux, GTK has no native window until the window is
    // shown, and tao panics (aborting the app) if click-through is set before that.
    if HUD_ALWAYS_VISIBLE {
        let _ = hud.set_ignore_cursor_events(true);
    }
    position_hud(app.handle());
    Ok(())
}

fn create_tray(app: &tauri::App, core: Arc<Core>) -> tauri::Result<Tray> {
    let status = MenuItem::with_id(app, "status", "Starting…", false, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "Start dictation (hands-free)", true, None::<&str>)?;
    let open = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Bolikh", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&status, &PredefinedMenuItem::separator(app)?, &toggle, &open, &PredefinedMenuItem::separator(app)?, &quit],
    )?;
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;
    TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("Bolikh")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, e| match e.id().as_ref() {
            "toggle" => {
                let recording = core.session.lock().unwrap().recording;
                if recording { core.stop_recording() } else { core.start_recording(true) }
            }
            "settings" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(Tray { status, toggle })
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

// ---------- Commands (called from the UI) ----------

type CoreState<'a> = tauri::State<'a, Arc<Core>>;

#[tauri::command]
fn get_status(core: CoreState) -> AppStatus {
    core.status()
}

#[tauri::command]
fn save_settings(core: CoreState, settings: Settings) {
    *core.settings.write().unwrap() = settings.clone();
    settings.save(&core.settings_path);
    core.emit_status();
}

#[tauri::command]
fn get_history(core: CoreState) -> Vec<history::Item> {
    core.history.lock().unwrap().items.clone()
}

#[tauri::command]
fn clear_history(core: CoreState) {
    core.history.lock().unwrap().clear();
}

#[tauri::command]
fn copy_text(text: String) -> bool {
    insert::copy(&text)
}

#[tauri::command]
fn retry_model(core: CoreState) {
    core.prepare_model();
}

#[tauri::command]
fn open_permission_settings() {
    permissions::open_settings();
}

#[tauri::command]
fn toggle_dictation(core: CoreState) {
    let recording = core.session.lock().unwrap().recording;
    if recording {
        core.stop_recording();
    } else {
        core.start_recording(true);
    }
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> bool {
    let al = app.autolaunch();
    let _ = if enabled { al.enable() } else { al.disable() };
    al.is_enabled().unwrap_or(false)
}

/// Records ~1.5 s and reports the peak level, so onboarding can confirm the mic works
/// (and triggers the OS microphone permission prompt).
#[tauri::command]
async fn test_microphone(core: tauri::State<'_, Arc<Core>>) -> Result<f32, String> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        if core.session.lock().unwrap().recording {
            return Err("Dictation in progress".to_string());
        }
        core.recorder.start()?;
        std::thread::sleep(Duration::from_millis(1500));
        let (samples, _) = core.recorder.stop();
        if samples.is_empty() {
            return Err("No audio received. Check microphone permission.".into());
        }
        Ok(samples.iter().fold(0f32, |m, s| m.max(s.abs())))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Linux display-stack workarounds; must run before GTK/WebKit initialise.
#[cfg(target_os = "linux")]
fn linux_env_fixes() {
    let unset = |k: &str| std::env::var_os(k).is_none();
    // WebKitGTK's DMA-BUF renderer crashes ("Error 71 (Protocol error)") or renders a
    // blank window on many Wayland / NVIDIA setups.
    if unset("WEBKIT_DISABLE_DMABUF_RENDERER") {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    // Wayland forbids apps from positioning windows or keeping them on top, which the
    // floating HUD needs; XWayland keeps it bottom-centre, click-through and unfocused.
    if std::env::var_os("WAYLAND_DISPLAY").is_some() && std::env::var_os("DISPLAY").is_some() && unset("GDK_BACKEND") {
        std::env::set_var("GDK_BACKEND", "x11");
    }
}

pub fn run() {
    #[cfg(target_os = "linux")]
    linux_env_fixes();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info,ort=warn")).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_status,
            save_settings,
            get_history,
            clear_history,
            copy_text,
            retry_model,
            open_permission_settings,
            toggle_dictation,
            set_autostart,
            test_microphone
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            let config_dir = app.path().app_config_dir()?;
            let data_dir = app.path().app_data_dir()?;
            let settings_path = config_dir.join("settings.json");
            let settings = Settings::load(&settings_path);

            // Throttled mic level → HUD meter (~30 fps).
            let last_level = Arc::new(AtomicU64::new(0));
            let level_app = handle.clone();
            let epoch = Instant::now();
            let recorder = audio::Recorder::new(Arc::new(move |level: f32| {
                let now = epoch.elapsed().as_millis() as u64;
                if now.saturating_sub(last_level.load(Ordering::Relaxed)) >= 33 {
                    last_level.store(now, Ordering::Relaxed);
                    let _ = level_app.emit_to("hud", "level", level);
                }
            }));

            let core = Arc::new(Core {
                app: handle.clone(),
                settings_path,
                settings: RwLock::new(settings.clone()),
                model: Mutex::new(ModelState::NotLoaded),
                engine: Engine::new(data_dir.join("models")),
                recorder,
                session: Mutex::new(Session::default()),
                history: Mutex::new(history::History::load(data_dir.join("history.json"))),
                tray: Mutex::new(None),
                hud_gen: AtomicU64::new(0),
            });
            app.manage(core.clone());

            // The HUD and tray are conveniences: if the desktop can't provide them
            // (e.g. no system-tray support on Linux), log it and keep the app running.
            if let Err(e) = create_hud(app) {
                log::error!("could not create HUD window: {e}");
            }
            // libappindicator panics (rather than erroring) when the tray library is
            // missing on Linux, so guard it.
            let tray = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| create_tray(app, core.clone())));
            match tray {
                Ok(Ok(t)) => *core.tray.lock().unwrap() = Some(t),
                Ok(Err(e)) => log::error!("could not create tray icon: {e}"),
                Err(_) => log::error!("system tray unavailable (install libayatana-appindicator3-1)"),
            }
            let has_tray = core.tray.lock().unwrap().is_some();

            // Closing the settings window just hides it; the app lives in the tray.
            if let Some(main) = app.get_webview_window("main") {
                let w = main.clone();
                main.on_window_event(move |e| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                        api.prevent_close();
                        let _ = w.hide();
                    }
                });
            }

            hotkey::spawn(Arc::new(core.clone()));
            core.prepare_model();
            core.refresh_tray();

            // Without a tray icon the window is the only way in, so always show it.
            if !has_tray || !settings.onboarded || !permissions::check().all_ok() {
                show_main(&handle);
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Bolikh")
        .run(|_app, event| {
            // Keep running in the tray when all windows are closed.
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
