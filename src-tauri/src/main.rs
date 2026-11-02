#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod scheduler;
mod updates;
mod alerts;
mod applog;
mod health;
mod autohide;
mod backdrop;
mod bar;
mod magnify;
mod effects;
mod docks;
mod config;
mod db;
mod net;
mod perf;
mod storage;
mod platform;
mod widgets;

use config::{Config, TaskbarMode};
use platform::SystemEvent;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WindowEvent};
use tauri_plugin_autostart::ManagerExt;

pub struct AppState {
    pub config: Mutex<Config>,
    pub path: PathBuf,
    pub popup: Mutex<bar::PopupState>,
    pub edit: AtomicBool,
    /// The user is dragging the bar window (ignore DPI re-layout until they let go).
    pub dragging: AtomicBool,
    /// Auto-hide: is the bar currently slid in?
    pub revealed: AtomicBool,
}

pub fn data_dir() -> PathBuf {
    std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("GlassBar")
}

/// Single write path for config: validate, persist, apply, broadcast.
pub fn commit(app: &AppHandle, cfg: Config) -> Result<(), String> {
    let cfg = cfg.normalized();
    let st = app.state::<AppState>();
    let moved = st.config.lock().unwrap().location != cfg.location;
    config::save(&st.path, &cfg).map_err(|e| e.to_string())?;
    *st.config.lock().unwrap() = cfg.clone();
    bar::apply(app);
    bar::sync_osd(app);
    bar::refresh_tray(app);
    widgets::ocr::register_hotkey(app);
    widgets::quick::sync_tray(app);
    sync_autostart(app, &cfg);
    perf::refresh(app);
    scheduler::poke(); // due items read from settings (countdowns, water) may have changed
    let _ = app.emit("config:changed", &cfg);
    if moved { widgets::location::poke(); }
    Ok(())
}

/// Make Windows' "run at sign-in" entry match the setting (also on start, so a fresh
/// install that never saved its config still starts with Windows).
fn sync_autostart(app: &AppHandle, cfg: &Config) {
    platform::shell().set_starts_with_windows(cfg.startup.launch_at_login);
    // Development runs never touch it: they'd point Windows at the slow debug build, which
    // also needs the dev server (a blank, late dock after a restart).
    if cfg!(debug_assertions) { return; }
    let al = app.autolaunch();
    // Re-enable even when already on: the entry then names this program, not an old copy.
    let _ = if cfg.startup.launch_at_login { al.enable() } else if al.is_enabled().unwrap_or(false) { al.disable() } else { Ok(()) };
}

#[tauri::command]
fn get_config(state: State<AppState>) -> Config { state.config.lock().unwrap().clone() }

#[tauri::command]
fn save_config(app: AppHandle, config: Config) -> Result<(), String> { commit(&app, config) }

#[tauri::command]
fn export_config(state: State<AppState>) -> String { config::export(&state.config.lock().unwrap()) }

#[tauri::command]
fn import_config(app: AppHandle, text: String) -> Result<(), String> { commit(&app, config::import(&text)?) }

#[tauri::command]
fn apply_bar(app: AppHandle) { bar::apply(&app) }

#[tauri::command]
fn restore_taskbar(app: AppHandle) -> Result<(), String> {
    let mut c = app.state::<AppState>().config.lock().unwrap().clone();
    c.profile_mut().bar.native_taskbar = TaskbarMode::Show;
    commit(&app, c)
}

fn main() {
    health::wait_for_previous();
    // A crash must never leave the Windows taskbar hidden or space reserved.
    // Release builds have no console, so the message goes to panic.log.
    std::panic::set_hook(Box::new(|info| {
        platform::shell().appbar_remove();
        platform::shell().set_taskbar(TaskbarMode::Show);
        let msg = format!("{info}
{}
", std::backtrace::Backtrace::force_capture());
        let _ = std::fs::write(data_dir().join("panic.log"), &msg);
        eprintln!("GlassBar panic: {msg}");
    }));

    // Load state before any window exists: the webviews call get_config as soon as
    // they load, and that can happen before `setup` runs.
    let path = data_dir().join("config.json");
    updates::HAD_CONFIG.store(path.exists(), std::sync::atomic::Ordering::Relaxed);
    let state = AppState {
        config: Mutex::new(config::load(&path)),
        path,
        popup: Mutex::new(Default::default()),
        edit: AtomicBool::new(false),
        dragging: AtomicBool::new(false),
        revealed: AtomicBool::new(false),
    };
    let app = tauri::Builder::default()
        .manage(state)
        .plugin(tauri_plugin_single_instance::init(|app, _, _| { let _ = bar::open_settings(app); }))
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, sc, ev| if ev.state == tauri_plugin_global_shortcut::ShortcutState::Pressed { widgets::ocr::on_hotkey(app, sc) })
            .build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let h = app.handle();
            let a = h.clone();
            platform::shell().init(&data_dir(), Box::new(move |e| match e {
                SystemEvent::DisplayChanged => { applog::log("monitors changed"); bar::apply(&a) }
                SystemEvent::ShellRestarted => { applog::log("Explorer (Windows taskbar) restarted"); bar::apply(&a) }
                SystemEvent::PowerChanged => { let _ = a.emit("battery:changed", ()); widgets::sound::power_event(&a); }
                // After sleep, monitors and Explorer settle a little later: place the dock again then.
                SystemEvent::Resumed => {
                    applog::log("woke from sleep");
                    widgets::location::poke();
                    bar::apply_later(&a, &[3, 15]);
                }
                SystemEvent::Device(on) => widgets::sound::device_event(&a, on),
                SystemEvent::Clipboard => widgets::clipboard::on_change(&a),
                SystemEvent::WorkArea => bar::replace(&a),
                // Step back from always-on-top while a game or video is fullscreen.
                // Windows also sends this when the desktop itself is clicked (Explorer): not a
                // real fullscreen app, so the dock must not drop behind other windows then.
                SystemEvent::Fullscreen(on) => {
                    let desktop = on && platform::shell().foreground_exe()
                        .is_some_and(|e| e.to_lowercase().ends_with("\\explorer.exe"));
                    for (l, w) in a.webview_windows() { if l == "bar" || l.starts_with("bar-") { let _ = w.set_always_on_top(!on || desktop); } }
                    bar::set_fullscreen(on && !desktop);
                }
            }));
            sync_autostart(h, &h.state::<AppState>().config.lock().unwrap().clone());
            bar::start_apply_worker(h);
            widgets::ocr::register_hotkey(h);
            bar::watch_autohide(h);
            bar::watch_hover_panel(h);
            bar::sync_osd(h);
            backdrop::watch(h);
            widgets::sound::charger_init();
            widgets::screenshots::prune(h);
            #[cfg(windows)]
            if let Some(p) = app.get_webview_window("popup") { widgets::mirror::allow_camera(&p); }
            widgets::clipboard::prune(h);
            widgets::quick::sync_tray(h);
            let n = h.clone();
            platform::wireless().watch_network(Box::new(move || { let _ = n.emit("wifi:changed", ()); widgets::location::poke(); }));
            bar::build_tray(h)?;
            bar::apply(h);
            if let Some(w) = app.get_webview_window("bar") { w.show()?; }
            // Windows settles the frameless window's frame on its first show, which changes
            // the size placed above; place it again so a fresh start matches every other time.
            bar::apply(h);
            widgets::start_watchers(h);
            scheduler::start(h.clone());
            widgets::location::start(h.clone());
            widgets::ipu::start(h.clone());
            widgets::sysmon::start(h.clone());
            perf::start(h.clone());
            widgets::screentime::start(h.clone());
            widgets::downloads::start(h.clone());
            widgets::trash::start(h.clone());
            widgets::quick::start_bt_battery(h.clone());
            updates::start(h.clone());
            applog::log(format!("GlassBar {} started", env!("CARGO_PKG_VERSION")));
            // Started with Windows: monitors and Explorer may still be settling. Place again soon.
            bar::apply_later(h, &[3, 15]);
            health::start(h.clone());
            // Ready before the first search or app picker opens.
            std::thread::spawn(|| { widgets::apps::installed(); });
            Ok(())
        })
        .on_window_event(|w, e| match (w.label(), e) {
            ("popup", WindowEvent::Focused(false)) => bar::popup_blurred(w.app_handle()),
            ("bar", WindowEvent::ScaleFactorChanged { .. })
                if w.app_handle().state::<AppState>().dragging.load(std::sync::atomic::Ordering::Relaxed) => {}
            ("bar", WindowEvent::ThemeChanged(_)) => bar::apply(w.app_handle()),
            // Windows applies its own DPI resize after this event returns, which would undo
            // a size set here; re-apply a moment later instead.
            (l, WindowEvent::ScaleFactorChanged { .. }) if l == "bar" || l.starts_with("bar-") => {
                let a = w.app_handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(120));
                    bar::apply(&a);
                });
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            get_config, save_config, export_config, import_config, apply_bar, restore_taskbar,
            health::health_pong, updates::whats_new_pending, updates::whats_new_seen, bar::popup_open, bar::popup_reveal, bar::popup_hide, bar::popup_leave_close, bar::popup_width, bar::popup_hover_hide, bar::banner_fit, bar::banner_hide, bar::jump_to, bar::open_settings_cmd, bar::set_edit_mode,
            bar::bar_preview, bar::bar_grow, magnify::magnify_prepare, magnify::magnify_show, magnify::magnify_frame, magnify::magnify_hide, effects::effects_targets, effects::app_restore_animated, bar::bar_drag, bar::mouse_down, bar::menu_open, bar::app_quit, bar::dock_alert_begin, bar::dock_alert_end, bar::dock_alert_test, bar::transparency_effects, bar::appearance_support, backdrop::wallpaper_behind, bar::bar_reveal, bar::popup_leave,
            bar::thumbs_show, bar::thumbs_clear, bar::bar_thumbs,
            widgets::todo::todo_list, widgets::todo::todo_add, widgets::todo::todo_update,
            widgets::todo::todo_delete, widgets::todo::todo_clear_done,
            widgets::notes::notes_list, widgets::notes::note_create, widgets::notes::note_update,
            widgets::notes::note_delete, widgets::notes::note_restore, widgets::notes::note_add_image,
            widgets::notes::note_remove_image, widgets::notes::note_image, widgets::notes::note_drag_image,
            widgets::focus::pomo_get, widgets::focus::pomo_start, widgets::focus::pomo_pause,
            widgets::focus::pomo_skip, widgets::focus::pomo_reset, widgets::focus::pomo_set_phase,
            widgets::focus::timers_list, widgets::focus::timer_add, widgets::focus::timer_pause,
            widgets::focus::timer_resume, widgets::focus::timer_delete, widgets::focus::timer_snooze,
            widgets::downloads::downloads_state, widgets::downloads::downloads_open, widgets::downloads::downloads_reveal,
            widgets::downloads::downloads_open_folder, widgets::downloads::downloads_open_with, widgets::downloads::downloads_rename,
            widgets::downloads::downloads_recycle, widgets::downloads::downloads_drag, widgets::downloads::file_icon,
            widgets::shelf::shelf_drag, widgets::shelf::shelf_pick,
            widgets::downloads::recycle_info, widgets::downloads::recycle_empty,
            widgets::trash::trash_items, widgets::trash::trash_restore, widgets::trash::trash_recycle, widgets::trash::trash_empty_confirm,
            widgets::focus::sw_get, widgets::focus::sw_toggle, widgets::focus::sw_lap, widgets::focus::sw_reset,
            widgets::location::location_status, widgets::location::location_clear, widgets::location::location_open_map, widgets::location::location_search, widgets::weather::weather_get,
            widgets::holidays::holidays_get, widgets::devstack::devstack_status, widgets::devstack::devstack_toggle, widgets::devstack::devstack_open_laragon,
            widgets::solat::solat_days, widgets::solat::solat_zones, widgets::solat::solat_test_sound,
            widgets::tasbih::tasbih_get, widgets::tasbih::tasbih_add, widgets::tasbih::tasbih_reset,
            widgets::ipu::ipu_get,
            perf::perf_history, perf::perf_state, perf::perf_watch, storage::storage_scan, storage::storage_clean, perf::boost::boost_run, perf::boost::boost_idle_apps, perf::boost::boost_close, bar::bar_away,
            widgets::sysmon::sysmon_history, widgets::sysmon::battery_get, widgets::sysmon::disks_list,
            widgets::sysmon::power_get, widgets::sysmon::power_set_mode, widgets::sysmon::power_use_balanced,
            widgets::screentime::screentime_today, widgets::screentime::screentime_week,
            widgets::jumplist::jumplist,
            alerts::alerts_list, alerts::alerts_mark_read, alerts::alerts_dismiss, alerts::alerts_clear,
            widgets::apps::apps_list, widgets::apps::app_icon, widgets::apps::app_activate,
            widgets::apps::start_menu, widgets::apps::windows_search,
            widgets::apps::app_close, widgets::apps::app_launch,
            widgets::sound::audio_state, widgets::sound::audio_set_volume, widgets::sound::audio_set_mute,
            widgets::sound::audio_set_default, widgets::sound::audio_sessions,
            widgets::sound::audio_set_session_volume, widgets::sound::audio_set_session_mute,
            widgets::media::media_info, widgets::media::media_control, widgets::media::media_seek, widgets::media::media_source, widgets::media::media_source_open, widgets::media::media_pause_playing, widgets::media::media_resume,
            widgets::lyrics::lyrics_get, widgets::lyrics::lyrics_folder,
            widgets::quick::radios_list, widgets::quick::radio_set, widgets::quick::airplane_set,
            widgets::quick::display_mode, widgets::quick::display_set_mode, widgets::quick::display_count,
            widgets::quick::brightness_get, widgets::quick::brightness_set,
            widgets::quick::hotspot_get, widgets::quick::hotspot_set,
            widgets::awake::awake_get, widgets::awake::awake_set, widgets::awake::awake_off,
            widgets::mic::mic_state, widgets::mic::mic_set_mute, widgets::mic::mic_set_volume, widgets::mic::mic_set_default, widgets::mic::privacy_in_use,
            widgets::quick::night_light_get, widgets::quick::night_light_set, widgets::quick::nearby_get, widgets::quick::nearby_set, widgets::quick::cast_get,
            widgets::water::water_today, widgets::water::water_add, widgets::water::water_remove, widgets::water::water_snooze,
            widgets::currency::fx_rates,
            widgets::apps::app_pin_target, widgets::apps::app_browse, widgets::apps::apps_installed, widgets::apps::paths_exist,
            widgets::ocr::ocr_hotkey_status, widgets::ocr::color_hotkey_status, widgets::ocr::color_pick_finish, widgets::ocr::ocr_snap_start, widgets::ocr::ocr_shot, widgets::ocr::ocr_snap_cancel, widgets::ocr::ocr_snap_show,
            widgets::ocr::ocr_snap_finish, widgets::screenshots::shots_list, widgets::clipboard::clip_list, widgets::clipboard::clip_copy, widgets::clipboard::clip_delete, widgets::clipboard::clip_pin, widgets::screenshots::shots_copy,
            widgets::screenshots::shots_delete, widgets::screenshots::shots_open, widgets::screenshots::shot_fullscreen, widgets::screenshots::shot_scroll_stop, widgets::quick::record_screen, widgets::mirror::mirror_save, widgets::ocr::ocr_history, widgets::ocr::ocr_history_delete, widgets::ocr::ocr_copy,
            widgets::quick::wifi_list, widgets::quick::wifi_connect, widgets::quick::wifi_disconnect,
            widgets::quick::bt_list, widgets::quick::bt_pair, widgets::quick::bt_unpair, widgets::quick::open_settings_uri,
            bar::popup_fit, widgets::quick::tray_icons, widgets::quick::tray_click, widgets::quick::tray_refresh, widgets::sound::audio_meter, widgets::spectrum::audio_spectrum, widgets::spectrum::audio_spectrum_stream,
            widgets::quick::clock_system_12h, widgets::quick::lang_list, widgets::quick::lang_set, widgets::quick::windows_notifications, widgets::quick::show_desktop, widgets::quick::windows_search_text,
        ])
        .build(tauri::generate_context!())
        .expect("error while building GlassBar");
    // Opened only after build(): a second GlassBar has handed over to the running one and quit
    // inside it (single-instance plugin), so it never opens, checks or backs up the live file.
    // Still before run(), which creates the windows that use it.
    // Never recreated over an existing file; a failure here is logged by the panic hook.
    app.manage(db::open(&data_dir().join("glassbar.db")).expect("could not open glassbar.db"));
    app.run(|app, e| if let RunEvent::Exit = e { bar::cleanup(app) });
}
