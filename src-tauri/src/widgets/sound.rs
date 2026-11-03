use super::blocking;
use crate::platform::{audio, AudioSession, AudioState, Res};

#[tauri::command]
pub async fn audio_state() -> Res<AudioState> { blocking(move || audio().state()).await? }
#[tauri::command]
pub async fn audio_set_volume(volume: f32) -> Res<()> { blocking(move || audio().set_volume(volume)).await? }
#[tauri::command]
pub async fn audio_set_mute(muted: bool) -> Res<()> { blocking(move || audio().set_mute(muted)).await? }
#[tauri::command]
pub async fn audio_set_default(id: String) -> Res<()> { blocking(move || audio().set_default(&id)).await? }
#[tauri::command]
pub async fn audio_sessions() -> Res<Vec<AudioSession>> { blocking(move || audio().sessions()).await? }
#[tauri::command]
pub async fn audio_set_session_volume(pid: u32, volume: f32) -> Res<()> { blocking(move || audio().set_session_volume(pid, volume)).await? }
#[tauri::command]
pub async fn audio_set_session_mute(pid: u32, muted: bool) -> Res<()> { blocking(move || audio().set_session_mute(pid, muted)).await? }

// ---- Device connection sounds ----

/// Play at most one sound per burst: a hub or a flaky cable can fire several events.
/// Same kind within 1.5 s, or anything within 0.7 s of the last sound, is skipped.
pub fn should_play(last: Option<(bool, i64)>, connected: bool, now: i64) -> bool {
    match last {
        Some((k, at)) if now - at < 700 || (k == connected && now - at < 1500) => false,
        _ => true,
    }
}

static LAST_DEVICE: std::sync::Mutex<Option<(bool, i64)>> = std::sync::Mutex::new(None);

pub fn device_event(app: &tauri::AppHandle, connected: bool) {
    use tauri::{Emitter, Manager};
    let s = {
        let st = app.state::<crate::AppState>();
        let c = st.config.lock().unwrap();
        if c.dnd { return; }
        c.device_sounds.clone()
    };
    if !(if connected { s.connect } else { s.disconnect }) { return; }
    let now = crate::db::now_ms();
    let mut last = LAST_DEVICE.lock().unwrap();
    if !should_play(*last, connected, now) { return; }
    *last = Some((connected, now));
    let kind = if connected { "connect" } else { "disconnect" };
    let _ = app.emit_to("bar", "sound:play", serde_json::json!({ "kind": kind, "volume": s.volume }));
}

#[cfg(test)]
mod tests {
    use super::should_play;
    #[test]
    fn device_sounds_debounce_bursts() {
        assert!(should_play(None, true, 0));
        assert!(!should_play(Some((true, 0)), true, 1000), "same kind within 1.5 s");
        assert!(should_play(Some((true, 0)), true, 1600));
        assert!(!should_play(Some((true, 0)), false, 500), "anything within 0.7 s");
        assert!(should_play(Some((true, 0)), false, 800), "unplug shortly after plug-in still sounds");
    }
}

// ---- Charger sounds and the dock flash ----

/// Pure: returns Some(plugged_in) only when the charger state really flipped.
/// Windows sends power events for battery % changes too; those must stay silent.
pub fn charger_flip(last: Option<bool>, now: Option<bool>) -> Option<bool> {
    match (last, now) { (Some(a), Some(b)) if a != b => Some(b), _ => None }
}

static LAST_AC: std::sync::Mutex<Option<bool>> = std::sync::Mutex::new(None);

/// Remember the starting state so launching GlassBar never plays a sound.
pub fn charger_init() { *LAST_AC.lock().unwrap() = crate::platform::power().battery().map(|b| b.on_ac); }

/// Pure: whether a real charger flip should make a sound. Do Not Disturb silences it, because
/// a sound is an interruption.
///
/// There is no matching gate for the dock flash. The flip event goes to the bar either way:
/// the flash is chrome reporting what the user just did with a cable, Do Not Disturb has no
/// business hiding it, and its own switch is checked in the page -- gating the *event* here
/// would also have switched off everything else that reacts to the charger.
pub fn charger_sound(charger_sound_on: bool, dnd: bool) -> bool {
    charger_sound_on && !dnd
}

pub fn power_event(app: &tauri::AppHandle) {
    use tauri::{Emitter, Manager};
    let bat = crate::platform::power().battery();
    let flip = {
        let now = bat.map(|b| b.on_ac);
        let mut l = LAST_AC.lock().unwrap();
        let f = charger_flip(*l, now);
        if now.is_some() { *l = now; }
        f
    };
    let Some(plugged) = flip else { return };
    let (sound, volume) = {
        let st = app.state::<crate::AppState>();
        let c = st.config.lock().unwrap();
        (charger_sound(c.device_sounds.charger, c.dnd), c.device_sounds.volume)
    };
    // Unconditional: the dock flash rides this (its own switch is checked in the page). Only the bar gets it --
    // a broadcast would wake the hidden panels for nothing.
    let _ = app.emit_to("bar", "charger:flip", serde_json::json!({
        "plugged": plugged,
        "percent": bat.map(|b| b.percent),
        "secsLeft": bat.and_then(|b| b.secs_left),
    }));
    if sound {
        let _ = app.emit_to("bar", "sound:play", serde_json::json!({ "kind": if plugged { "plug" } else { "unplug" }, "volume": volume }));
    }
}

#[cfg(test)]
mod charger_tests {
    use super::charger_flip;
    #[test]
    fn only_real_plug_changes_make_a_sound() {
        assert_eq!(charger_flip(Some(false), Some(true)), Some(true));
        assert_eq!(charger_flip(Some(true), Some(false)), Some(false));
        assert_eq!(charger_flip(Some(true), Some(true)), None, "battery % updates while plugged in");
        assert_eq!(charger_flip(None, Some(true)), None, "first reading");
        assert_eq!(charger_flip(Some(true), None), None, "no battery info");
    }

    #[test]
    fn dnd_silences_the_charger_sound() {
        use super::charger_sound;
        assert!(charger_sound(true, false));
        assert!(!charger_sound(true, true), "Do Not Disturb silences it");
        assert!(!charger_sound(false, false), "switched off");
        assert!(!charger_sound(false, true));
    }
}

// ---- Live level for the music visualizer ----
// The page calls `audio_meter` every ~2 s while a visualizer is on screen and music plays.
// The meter thread emits "audio:peak" ~25 times a second and goes idle 3 s after the
// last call, so a closed panel or reloaded page can never leave it running.

static METER_LEASE: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
static METER_STARTED: std::sync::Once = std::sync::Once::new();

#[tauri::command]
pub fn audio_meter(app: tauri::AppHandle) {
    METER_LEASE.store(crate::db::now_ms(), std::sync::atomic::Ordering::Relaxed);
    METER_STARTED.call_once(|| {
        std::thread::spawn(move || {
            use tauri::Emitter;
            loop {
                let live = crate::db::now_ms() - METER_LEASE.load(std::sync::atomic::Ordering::Relaxed) < 3000;
                if !live { std::thread::sleep(std::time::Duration::from_millis(300)); continue; }
                // ponytail: re-activates the meter each tick (follows device switches for free; ~µs cost).
                let p = crate::platform::audio().peak().unwrap_or(0.0);
                let _ = app.emit("audio:peak", p);
                std::thread::sleep(std::time::Duration::from_millis(40));
            }
        });
    });
}
