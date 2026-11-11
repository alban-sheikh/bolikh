//! Inserts text into whichever text field has keyboard focus, in any app.
//! Clipboard + paste shortcut is the most compatible approach across apps and OSes;
//! the user's previous clipboard is restored afterwards.
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

/// Put `text` on the clipboard and press the platform paste shortcut.
/// On macOS this must run on the main thread (keyboard layout APIs).
pub fn paste(text: &str) -> Outcome {
    if !copy(text) {
        return Outcome::CopiedOnly("Could not access the clipboard.".into());
    }
    // Give the clipboard a moment to propagate (notably on Linux/Windows).
    std::thread::sleep(Duration::from_millis(40));

    #[cfg(target_os = "linux")]
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        if run("wtype", &["-M", "ctrl", "v", "-m", "ctrl"]) {
            return Outcome::Pasted;
        }
        // ydotool uses Linux input event codes: 29 = Left Ctrl, 47 = V.
        if run("ydotool", &["key", "29:1", "47:1", "47:0", "29:0"]) {
            return Outcome::Pasted;
        }
        // Fall through to enigo (works for XWayland apps).
    }

    match press_paste() {
        Ok(()) => Outcome::Pasted,
        Err(e) => Outcome::CopiedOnly(e),
    }
}

fn press_paste() -> Result<(), String> {
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
        enigo.key(v, Direction::Click)?;
        enigo.key(modifier, Direction::Release)
    })();
    r.map_err(|e| format!("Could not send paste shortcut: {e}"))
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
