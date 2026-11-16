//! Inserts text into whichever text field has keyboard focus, in any app.
//! Clipboard + paste shortcut is the most compatible approach across apps and OSes;
//! the user's previous clipboard is restored afterwards.
use crate::settings::LinuxPaste;
use arboard::{Clipboard, ImageData};
use enigo::{Direction, Enigo, Key, Keyboard};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::time::Duration;

/// Kept alive for the whole process: on Linux the clipboard owner must stay
/// alive for the pasted content to remain available.
static CLIPBOARD: Lazy<Mutex<Option<Clipboard>>> = Lazy::new(|| Mutex::new(Clipboard::new().ok()));

pub enum Saved {
    Text(String),
    Image(ImageData<'static>),
    Nothing,
}

pub enum Outcome {
    Pasted,
    /// No way to simulate the paste keystroke; the text is on the clipboard.
    CopiedOnly(String),
}

fn with_clipboard<T>(f: impl FnOnce(&mut Clipboard) -> T) -> Option<T> {
    let mut guard = CLIPBOARD.lock().ok()?;
    if guard.is_none() {
        *guard = Clipboard::new().ok();
    }
    guard.as_mut().map(f)
}

pub fn copy(text: &str) -> bool {
    with_clipboard(|c| c.set_text(text.to_string()).is_ok()).unwrap_or(false)
}

pub fn snapshot() -> Saved {
    with_clipboard(|c| {
        if let Ok(t) = c.get_text() {
            return Saved::Text(t);
        }
        if let Ok(img) = c.get_image() {
            return Saved::Image(img.to_owned_img());
        }
        Saved::Nothing
    })
    .unwrap_or(Saved::Nothing)
}

/// Restore the previous clipboard, but only if it still holds what we put there.
pub fn restore(saved: Saved, ours: &str) {
    with_clipboard(|c| {
        if c.get_text().ok().as_deref() != Some(ours) {
            return;
        }
        let _ = match saved {
            Saved::Text(t) => c.set_text(t),
            Saved::Image(i) => c.set_image(i),
            Saved::Nothing => c.clear(),
        };
    });
}

/// Window classes and app ids of terminal emulators. Terminals paste with Ctrl+Shift+V,
/// because Ctrl+V is a control character there.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn is_terminal(class: &str) -> bool {
    const NAMES: [&str; 31] = [
        "terminal", "konsole", "kitty", "alacritty", "wezterm", "xterm", "rxvt", "foot", "tilix", "terminator",
        "ghostty", "ptyxis", "kgx", "guake", "yakuake", "tilda", "warp", "hyper", "tabby", "contour",
        "cool-retro-term", "blackbox", "termite", "sakura", "terminology", "termius", "extraterm", "st-256color",
        "eterm", "kermit", "tmux",
    ];
    let class = class.trim().to_lowercase();
    // "st" (suckless) and "rio" are too short to search for inside other names.
    matches!(class.as_str(), "st" | "rio") || NAMES.iter().any(|n| class.contains(n))
}

/// The class or app id of the focused window, from whichever desktop tool is installed.
/// GNOME on Wayland offers no way to ask; the setting covers that case.
#[cfg(target_os = "linux")]
fn focused_window_class() -> Option<String> {
    fn output(cmd: &str, args: &[&str]) -> Option<String> {
        let out = std::process::Command::new(cmd).args(args).stderr(std::process::Stdio::null()).output().ok()?;
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (out.status.success() && !text.is_empty()).then_some(text)
    }
    /// The focused node of a sway window tree.
    fn sway_focused(node: &serde_json::Value) -> Option<String> {
        if node["focused"].as_bool() == Some(true) {
            let class = node["app_id"].as_str().or_else(|| node["window_properties"]["class"].as_str());
            return class.map(str::to_string);
        }
        ["nodes", "floating_nodes"]
            .iter()
            .filter_map(|k| node[*k].as_array())
            .flatten()
            .find_map(sway_focused)
    }

    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
        if let Some(json) = output("hyprctl", &["activewindow", "-j"]) {
            let v: serde_json::Value = serde_json::from_str(&json).ok()?;
            return v["class"].as_str().map(str::to_string);
        }
    }
    if std::env::var_os("SWAYSOCK").is_some() {
        if let Some(json) = output("swaymsg", &["-t", "get_tree"]) {
            return sway_focused(&serde_json::from_str(&json).ok()?);
        }
    }
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        // KDE Plasma on Wayland.
        if let Some(class) = output("kdotool", &["getactivewindow", "getwindowclassname"]) {
            return Some(class);
        }
    }
    if let Some(class) = output("xdotool", &["getactivewindow", "getwindowclassname"]) {
        return Some(class);
    }
    // xprop ships with almost every X11 desktop.
    let active = output("xprop", &["-root", "_NET_ACTIVE_WINDOW"])?;
    let id = active.rsplit(' ').next()?.to_string();
    let class = output("xprop", &["-id", &id, "WM_CLASS"])?;
    class.split('=').nth(1).map(str::to_string)
}

/// Whether to press Ctrl+Shift+V instead of Ctrl+V.
#[cfg(target_os = "linux")]
fn terminal_paste(mode: LinuxPaste) -> bool {
    match mode {
        LinuxPaste::CtrlV => false,
        LinuxPaste::CtrlShiftV => true,
        LinuxPaste::Auto => focused_window_class().is_some_and(|c| is_terminal(&c)),
    }
}

/// Put `text` on the clipboard and press the platform paste shortcut.
/// On macOS this must run on the main thread (keyboard layout APIs).
#[cfg_attr(not(target_os = "linux"), allow(unused_variables))]
pub fn paste(text: &str, linux_paste: LinuxPaste) -> Outcome {
    if !copy(text) {
        return Outcome::CopiedOnly("Could not access the clipboard.".into());
    }
    // Give the clipboard a moment to propagate (notably on Linux/Windows).
    std::thread::sleep(Duration::from_millis(40));

    #[cfg(target_os = "linux")]
    let shift = terminal_paste(linux_paste);
    #[cfg(not(target_os = "linux"))]
    let shift = false;

    #[cfg(target_os = "linux")]
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        let wtype: &[&str] = if shift {
            &["-M", "ctrl", "-M", "shift", "v", "-m", "shift", "-m", "ctrl"]
        } else {
            &["-M", "ctrl", "v", "-m", "ctrl"]
        };
        if run("wtype", wtype) {
            return Outcome::Pasted;
        }
        // ydotool uses Linux input event codes: 29 = Left Ctrl, 42 = Left Shift, 47 = V.
        let ydotool: &[&str] = if shift {
            &["key", "29:1", "42:1", "47:1", "47:0", "42:0", "29:0"]
        } else {
            &["key", "29:1", "47:1", "47:0", "29:0"]
        };
        if run("ydotool", ydotool) {
            return Outcome::Pasted;
        }
        // Fall through to enigo (works for XWayland apps).
    }

    match press_paste(shift) {
        Ok(()) => Outcome::Pasted,
        Err(e) => Outcome::CopiedOnly(e),
    }
}

/// `shift` adds Shift to the shortcut (Linux terminals); it is never set on macOS or Windows.
fn press_paste(shift: bool) -> Result<(), String> {
    let mut enigo = Enigo::new(&enigo::Settings::default())
        .map_err(|e| format!("Keyboard control unavailable: {e}"))?;

    // Physical "V" key so it works with any keyboard layout (Urdu, Arabic, Cyrillic…).
    #[cfg(target_os = "macos")]
    let (modifier, v) = (Key::Meta, Key::Other(9));
    #[cfg(target_os = "windows")]
    let (modifier, v) = (Key::Control, Key::V);
    #[cfg(all(unix, not(target_os = "macos")))]
    let (modifier, v) = (Key::Control, Key::Unicode('v'));

    let r = (|| {
        enigo.key(modifier, Direction::Press)?;
        if shift {
            enigo.key(Key::Shift, Direction::Press)?;
        }
        enigo.key(v, Direction::Click)?;
        if shift {
            enigo.key(Key::Shift, Direction::Release)?;
        }
        enigo.key(modifier, Direction::Release)
    })();
    r.map_err(|e| format!("Could not send paste shortcut: {e}"))
}

/// Presses Backspace `count` times to remove the text typed by the last dictation.
/// On macOS this must run on the main thread.
pub fn backspace(count: usize) -> Result<(), String> {
    let mut enigo = Enigo::new(&enigo::Settings::default())
        .map_err(|e| format!("Keyboard control unavailable: {e}"))?;
    // A safety cap: never hold the keyboard for more than a few thousand presses.
    for _ in 0..count.min(4000) {
        enigo.key(Key::Backspace, Direction::Click).map_err(|e| format!("Could not send Backspace: {e}"))?;
        std::thread::sleep(Duration::from_millis(2));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn run(cmd: &str, args: &[&str]) -> bool {
    std::process::Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::is_terminal;

    #[test]
    fn recognises_terminals() {
        for class in [
            "gnome-terminal-server", "org.gnome.Terminal", "org.gnome.Ptyxis", "kgx", "konsole", "org.kde.konsole",
            "kitty", "Alacritty", "org.wezfurlong.wezterm", "XTerm", "URxvt", "foot", "footclient", "st",
            "com.mitchellh.ghostty", "xfce4-terminal", "\"gnome-terminal-server\", \"Gnome-terminal\"", "tilix",
        ] {
            assert!(is_terminal(class), "{class}");
        }
        for class in ["firefox", "Code", "Slack", "org.gnome.Nautilus", "libreoffice-writer", "google-chrome", "obsidian"] {
            assert!(!is_terminal(class), "{class}");
        }
    }
}
