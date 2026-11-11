//! Global push-to-talk. Watches a single modifier key system-wide and turns
//! presses into start / stop / cancel intents.
use crate::settings::{Mode, Trigger};
use handy_keys::{Key, KeyEvent, KeyboardListener};
use std::time::{Duration, Instant};

/// Holding longer than this = push-to-talk; shorter = a tap.
const TAP_THRESHOLD: Duration = Duration::from_millis(300);

pub trait Handler: Send + Sync + 'static {
    fn trigger(&self) -> (Trigger, Mode);
    fn is_recording(&self) -> bool;
    fn start(&self, hands_free: bool);
    fn stop(&self);
    fn cancel(&self);
    fn set_hands_free(&self);
}

#[derive(Default)]
struct PressState {
    down: bool,
    pressed_at: Option<Instant>,
    started_this_press: bool,
    other_key: bool,
}

pub fn spawn<H: Handler>(handler: std::sync::Arc<H>) {
    std::thread::Builder::new()
        .name("hotkey".into())
        .spawn(move || loop {
            // Waits for OS permission (macOS Accessibility / Linux input group) and
            // retries, so granting access takes effect without restarting the app.
            if !crate::permissions::check().accessibility {
                std::thread::sleep(Duration::from_secs(2));
                continue;
            }
            match KeyboardListener::new() {
                Ok(listener) => {
                    log::info!("hotkey listener started");
                    run(&listener, handler.as_ref());
                    log::warn!("hotkey listener stopped; restarting");
                }
                Err(e) => log::warn!("hotkey listener unavailable: {e}"),
            }
            std::thread::sleep(Duration::from_secs(2));
        })
        .expect("spawn hotkey thread");
}

fn run<H: Handler>(listener: &KeyboardListener, h: &H) {
    let mut st = PressState::default();
    while let Ok(ev) = listener.recv() {
        handle(&ev, &mut st, h);
    }
}

fn handle<H: Handler>(ev: &KeyEvent, st: &mut PressState, h: &H) {
    let (trigger, mode) = h.trigger();
    let flag = trigger.modifier();

    if ev.changed_modifier == Some(flag) {
        if ev.is_key_down && !st.down {
            st.down = true;
            st.pressed_at = Some(Instant::now());
            st.other_key = false;
            on_press(st, mode, h);
        } else if !ev.is_key_down && st.down {
            st.down = false;
            let held = st.pressed_at.map(|t| t.elapsed()).unwrap_or_default();
            on_release(st, mode, held, h);
        }
        return;
    }

    if ev.is_key_down {
        if let Some(key) = ev.key {
            if key == Key::Escape && h.is_recording() {
                h.cancel();
                return;
            }
            // Another key while the trigger is held = the user is typing a shortcut. Back off.
            if st.down {
                st.other_key = true;
                if st.started_this_press && h.is_recording() {
                    h.cancel();
                }
            }
        }
    }
}

fn on_press<H: Handler>(st: &mut PressState, mode: Mode, h: &H) {
    st.started_this_press = false;
    if h.is_recording() {
        h.stop();
        return;
    }
    st.started_this_press = true;
    h.start(mode == Mode::Toggle);
}

fn on_release<H: Handler>(st: &mut PressState, mode: Mode, held: Duration, h: &H) {
    if !st.started_this_press || st.other_key || !h.is_recording() {
        return;
    }
    match mode {
        Mode::Hold => h.stop(),
        Mode::Smart => {
            if held >= TAP_THRESHOLD {
                h.stop();
            } else {
                h.set_hands_free();
            }
        }
        Mode::Toggle => {}
    }
}
