//! What the OS requires before global hotkeys and auto-paste can work.
use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Permissions {
    pub os: &'static str,
    /// macOS: Accessibility (global key listening + simulated paste). Always true elsewhere.
    pub accessibility: bool,
    /// Linux: read access to /dev/input (needed for the global hotkey on X11 and Wayland).
    pub input_devices: bool,
    /// Linux Wayland: a tool that can type the paste shortcut (wtype or ydotool).
    pub wayland_paste_tool: bool,
    pub wayland: bool,
}

pub fn os_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
}

pub fn check() -> Permissions {
    #[cfg(target_os = "macos")]
    let accessibility = handy_keys::check_accessibility();
    #[cfg(not(target_os = "macos"))]
    let accessibility = true;

    #[cfg(target_os = "linux")]
    let (input_devices, wayland, wayland_paste_tool) = {
        let readable = std::fs::read_dir("/dev/input")
            .map(|rd| {
                rd.flatten().any(|e| {
                    e.file_name().to_string_lossy().starts_with("event")
                        && std::fs::File::open(e.path()).is_ok()
                })
            })
            .unwrap_or(false);
        let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
        let tool = which("wtype") || which("ydotool");
        (readable, wayland, tool)
    };
    #[cfg(not(target_os = "linux"))]
    let (input_devices, wayland, wayland_paste_tool) = (true, false, true);

    Permissions { os: os_name(), accessibility, input_devices, wayland_paste_tool, wayland }
}

impl Permissions {
    pub fn all_ok(&self) -> bool {
        self.accessibility && self.input_devices && (!self.wayland || self.wayland_paste_tool)
    }
}

#[cfg(target_os = "linux")]
fn which(bin: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join(bin).is_file()))
        .unwrap_or(false)
}

pub fn open_settings() {
    #[cfg(target_os = "macos")]
    {
        // Registers the app in the Accessibility list (and shows the system prompt);
        // merely opening the pane would leave the user to find and add the app by hand.
        macos::prompt_accessibility();
        let _ = handy_keys::open_accessibility_settings();
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::c_void;

    type CFTypeRef = *const c_void;

    #[repr(C)]
    struct CFDictionaryCallBacks([usize; 6]);

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        static kCFBooleanTrue: CFTypeRef;
        static kCFTypeDictionaryKeyCallBacks: CFDictionaryCallBacks;
        static kCFTypeDictionaryValueCallBacks: CFDictionaryCallBacks;
        fn CFDictionaryCreate(
            allocator: CFTypeRef,
            keys: *const CFTypeRef,
            values: *const CFTypeRef,
            count: isize,
            key_callbacks: *const CFDictionaryCallBacks,
            value_callbacks: *const CFDictionaryCallBacks,
        ) -> CFTypeRef;
        fn CFRelease(cf: CFTypeRef);
    }

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        static kAXTrustedCheckOptionPrompt: CFTypeRef;
        fn AXIsProcessTrustedWithOptions(options: CFTypeRef) -> bool;
    }

    pub fn prompt_accessibility() -> bool {
        unsafe {
            let keys = [kAXTrustedCheckOptionPrompt];
            let values = [kCFBooleanTrue];
            let dict = CFDictionaryCreate(
                std::ptr::null(),
                keys.as_ptr(),
                values.as_ptr(),
                1,
                &kCFTypeDictionaryKeyCallBacks,
                &kCFTypeDictionaryValueCallBacks,
            );
            let trusted = AXIsProcessTrustedWithOptions(dict);
            if !dict.is_null() {
                CFRelease(dict);
            }
            trusted
        }
    }
}
