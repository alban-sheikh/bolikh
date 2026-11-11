//! User preferences, persisted as JSON in the app config directory.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The key that triggers dictation. Modifier-only, side-specific so it never
/// collides with normal shortcuts.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Trigger {
    OptRight,
    OptLeft,
    CtrlRight,
    CmdRight,
    ShiftRight,
    Fn,
}

impl Trigger {
    pub fn modifier(self) -> handy_keys::Modifiers {
        use handy_keys::Modifiers as M;
        match self {
            Trigger::OptRight => M::OPT_RIGHT,
            Trigger::OptLeft => M::OPT_LEFT,
            Trigger::CtrlRight => M::CTRL_RIGHT,
            Trigger::CmdRight => M::CMD_RIGHT,
            Trigger::ShiftRight => M::SHIFT_RIGHT,
            Trigger::Fn => M::FN,
        }
    }

    pub fn platform_default() -> Self {
        if cfg!(target_os = "macos") { Trigger::OptRight } else { Trigger::CtrlRight }
    }

    pub fn label(self) -> &'static str {
        let mac = cfg!(target_os = "macos");
        match self {
            Trigger::OptRight => if mac { "Right ⌥" } else { "Right Alt" },
            Trigger::OptLeft => if mac { "Left ⌥" } else { "Left Alt" },
            Trigger::CtrlRight => if mac { "Right ⌃" } else { "Right Ctrl" },
            Trigger::CmdRight => if mac { "Right ⌘" } else { "Right Win" },
            Trigger::ShiftRight => if mac { "Right ⇧" } else { "Right Shift" },
            Trigger::Fn => "Fn",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    /// Hold to talk; a quick tap switches to hands-free until the next tap.
    Smart,
    /// Records only while the key is held.
    Hold,
    /// Tap to start, tap again to stop.
    Toggle,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Replacement {
    pub from: String,
    pub to: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub trigger: Trigger,
    pub mode: Mode,
    pub play_sounds: bool,
    pub add_trailing_space: bool,
    pub restore_clipboard: bool,
    pub voice_commands: bool,
    pub remove_fillers: bool,
    pub keep_history: bool,
    pub replacements: Vec<Replacement>,
    pub onboarded: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            trigger: Trigger::platform_default(),
            mode: Mode::Smart,
            play_sounds: true,
            add_trailing_space: true,
            restore_clipboard: true,
            voice_commands: true,
            remove_fillers: true,
            keep_history: true,
            replacements: Vec::new(),
            onboarded: false,
        }
    }
}

impl Settings {
    pub fn load(path: &PathBuf) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &PathBuf) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }
}
