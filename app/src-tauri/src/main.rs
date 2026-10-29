// =====================================================================
// Chokh - চোখ | Rust ব্যাকএন্ড (ধাপ ২)
//   • সিস্টেম ট্রে + মেনু
//   • ব্যাকগ্রাউন্ড টাইমার (ওয়েবভিউ লুকানো থাকলেও ঠিকমতো চলে)
//   • ফুল-স্ক্রিন ব্রেক ওভারলে খোলা/বন্ধ করা
//   • সেটিংস (ইন্টারভাল) JSON ফাইলে সংরক্ষণ
// =====================================================================

// রিলিজ বিল্ডে কালো কনসোল জানালা দেখাবে না
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    fs,
    path::PathBuf,
    sync::Mutex,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    window::{Effect, EffectsBuilder},
    AppHandle, Emitter, Manager, State, WindowEvent,
};

/// সেটিংসে যে ইন্টারভালগুলো অনুমোদিত (মিনিট)
const ALLOWED_INTERVALS: [u64; 5] = [15, 20, 30, 45, 60];
/// ব্রেক স্ক্রিন ৫ মিনিটের বেশি আটকে থাকলে নিজে বন্ধ হবে (ইউজার যেন আটকে না যায়)
const BREAK_FAILSAFE_SECS: u64 = 300;
/// টাইমার থ্রেড এর চেয়ে বেশি সময় "ঘুমালে" ধরা হয় পিসি স্লিপে ছিল
const SLEEP_GAP_SECS: u64 = 120;

// ---------------------------------------------------------------------
// সেটিংস (settings.json)
// ---------------------------------------------------------------------
#[derive(Serialize, Deserialize, Clone)]
struct Settings {
    interval_min: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { interval_min: 20 }
    }
}

fn load_settings(path: &PathBuf) -> Settings {
    let s: Settings = fs::read_to_string(path)
        .ok()
        .and_then(|txt| serde_json::from_str(&txt).ok())
        .unwrap_or_default();
    // ফাইল হাতে বদলানো থাকলে অবৈধ মান ঠেকাতে
    if ALLOWED_INTERVALS.contains(&s.interval_min) {
        s
    } else {
        Settings::default()
    }
}

fn save_settings(path: &PathBuf, s: &Settings) {
    if let Ok(txt) = serde_json::to_string_pretty(s) {
        let _ = fs::write(path, txt);
    }
}

// ---------------------------------------------------------------------
// টাইমারের অবস্থা (সব সময় epoch সেকেন্ডে: ঘড়ি দিয়ে মাপা)
// ---------------------------------------------------------------------
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

struct Timer {
    interval_min: u64,
    next_break: u64,
    paused_until: Option<u64>,
    break_started: Option<u64>,
}

struct AppState {
    timer: Mutex<Timer>,
    blur_ok: bool,
    settings_path: PathBuf,
}

impl AppState {
    fn pause(&self, hours: u64) {
        let mut t = self.timer.lock().unwrap();
        t.paused_until = Some(now() + hours * 3600);
    }

    fn resume(&self) {
        let mut t = self.timer.lock().unwrap();
        t.paused_until = None;
        t.next_break = now() + t.interval_min * 60;
    }
}

#[derive(Serialize)]
struct Status {
    interval_min: u64,
    secs_until_break: u64,
    pause_secs_left: u64,
    break_active: bool,
}

#[derive(Serialize, Clone)]
struct BreakPayload {
    /// Windows-এর acrylic blur কাজ করলে true; নইলে CSS বেশি গাঢ় করবে
    blur: bool,
}

// ---------------------------------------------------------------------
// ব্রেক শুরু ও শেষ
// ---------------------------------------------------------------------
fn trigger_break(app: &AppHandle) {
    let state = app.state::<AppState>();
    {
        let mut t = state.timer.lock().unwrap();
        if t.break_started.is_some() {
            return; // আগের ব্রেক এখনও চলছে
        }
        t.break_started = Some(now());
    }
    if let Some(win) = app.get_webview_window("break") {
        let _ = win.show();
        let _ = win.set_always_on_top(true);
        let _ = win.set_focus();
        let _ = app.emit_to("break", "break-start", BreakPayload { blur: state.blur_ok });
    }
}

fn end_break(app: &AppHandle, taken: bool) {
    let state = app.state::<AppState>();
    {
        let mut t = state.timer.lock().unwrap();
        if t.break_started.take().is_none() {
            return; // কোনো ব্রেক চলছিল না
        }
        t.next_break = now() + t.interval_min * 60;
    }
    if let Some(win) = app.get_webview_window("break") {
        let _ = win.hide();
    }
    // 🔜 ধাপ ৩/৪: এখানে `taken` দিয়ে দৈনিক স্ট্যাটস ও অ্যানালিটিক্স ইভেন্ট (break_taken / break_skipped) যাবে।
    let _ = taken;
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

// ---------------------------------------------------------------------
// ব্যাকগ্রাউন্ড টাইমার থ্রেড: প্রতি সেকেন্ডে একবার ঘড়ি দেখে
// ---------------------------------------------------------------------
fn spawn_timer(app: AppHandle) {
    thread::spawn(move || {
        let mut last_tick = now();
        loop {
            thread::sleep(Duration::from_secs(1));
            let n = now();
            let gap = n.saturating_sub(last_tick);
            last_tick = n;

            // লক ধরে রেখে সিদ্ধান্ত নেই, লক ছেড়ে তারপর কাজ করি (ডেডলক এড়াতে)
            let mut start_break = false;
            let mut force_end = false;
            {
                let state = app.state::<AppState>();
                let mut t = state.timer.lock().unwrap();

                // পিসি স্লিপ থেকে উঠলে সঙ্গে সঙ্গে ব্রেক না দেখিয়ে কাউন্টডাউন নতুন করে শুরু
                if gap > SLEEP_GAP_SECS && t.break_started.is_none() {
                    t.next_break = n + t.interval_min * 60;
                }

                if let Some(started) = t.break_started {
                    if n.saturating_sub(started) > BREAK_FAILSAFE_SECS {
                        force_end = true;
                    }
                } else if let Some(p) = t.paused_until {
                    if n >= p {
                        t.paused_until = None;
                        t.next_break = n + t.interval_min * 60;
                    }
                } else if n >= t.next_break {
                    start_break = true;
                }
            }

            if force_end {
                end_break(&app, false);
            } else if start_break {
                trigger_break(&app);
            }
        }
    });
}

// ---------------------------------------------------------------------
// কমান্ড: ফ্রন্টএন্ড (JS) থেকে invoke() দিয়ে ডাকা হয়
// ---------------------------------------------------------------------
#[tauri::command]
fn get_status(state: State<'_, AppState>) -> Status {
    let t = state.timer.lock().unwrap();
    let n = now();
    Status {
        interval_min: t.interval_min,
        secs_until_break: t.next_break.saturating_sub(n),
        pause_secs_left: t.paused_until.map(|p| p.saturating_sub(n)).unwrap_or(0),
        break_active: t.break_started.is_some(),
    }
}

#[tauri::command]
fn set_interval(state: State<'_, AppState>, minutes: u64) -> Result<(), String> {
    if !ALLOWED_INTERVALS.contains(&minutes) {
        return Err("অনুমোদিত নয়: শুধু 15, 20, 30, 45, 60".into());
    }
    {
        let mut t = state.timer.lock().unwrap();
        t.interval_min = minutes;
        t.next_break = now() + minutes * 60;
    }
    save_settings(&state.settings_path, &Settings { interval_min: minutes });
    Ok(())
}

#[tauri::command]
fn pause(state: State<'_, AppState>, hours: u64) -> Result<(), String> {
    if hours != 1 && hours != 2 {
        return Err("শুধু ১ বা ২ ঘণ্টা".into());
    }
    state.pause(hours);
    Ok(())
}

#[tauri::command]
fn resume(state: State<'_, AppState>) {
    state.resume();
}

#[tauri::command]
fn start_break_now(app: AppHandle) {
    trigger_break(&app);
}

#[tauri::command]
fn finish_break(app: AppHandle, taken: bool) {
    end_break(&app, taken);
}

// ---------------------------------------------------------------------
// অ্যাপ চালু
// ---------------------------------------------------------------------
fn main() {
    tauri::Builder::default()
        // একই অ্যাপ দুবার চালালে নতুনটা না খুলে পুরনোর জানালা দেখায় (এই প্লাগইন সবার আগে)
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main(app);
        }))
        // ✕ চাপলে অ্যাপ বন্ধ না হয়ে ট্রে-তে লুকাবে। ব্রেক জানালা Alt+F4 দিলেও "Skip" ধরা হবে।
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if window.label() == "break" {
                    end_break(&window.app_handle(), false);
                } else {
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            set_interval,
            pause,
            resume,
            start_break_now,
            finish_break
        ])
        .setup(|app| {
            // ১) সেটিংস লোড
            let dir = app.path().app_config_dir()?;
            let _ = fs::create_dir_all(&dir);
            let settings_path = dir.join("settings.json");
            let settings = load_settings(&settings_path);

            // ২) ব্রেক জানালার পেছনে Windows acrylic blur (না হলে CSS দিয়ে গাঢ় করা হবে)
            let blur_ok = app
                .get_webview_window("break")
                .map(|w| w.set_effects(EffectsBuilder::new().effect(Effect::Acrylic).build()).is_ok())
                .unwrap_or(false);

            // ৩) টাইমারের অবস্থা
            app.manage(AppState {
                timer: Mutex::new(Timer {
                    interval_min: settings.interval_min,
                    next_break: now() + settings.interval_min * 60,
                    paused_until: None,
                    break_started: None,
                }),
                blur_ok,
                settings_path,
            });

            // ৪) ট্রে আইকন ও মেনু
            let m_open = MenuItem::with_id(app, "open", "চোখ খুলুন", true, None::<&str>)?;
            let m_now = MenuItem::with_id(app, "break_now", "এখনই ব্রেক নিন", true, None::<&str>)?;
            let m_p1 = MenuItem::with_id(app, "pause1", "১ ঘণ্টা বিরতি", true, None::<&str>)?;
            let m_p2 = MenuItem::with_id(app, "pause2", "২ ঘণ্টা বিরতি", true, None::<&str>)?;
            let m_resume = MenuItem::with_id(app, "resume", "রিমাইন্ডার চালু করুন", true, None::<&str>)?;
            let m_quit = MenuItem::with_id(app, "quit", "বন্ধ করুন", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&m_open, &m_now, &m_p1, &m_p2, &m_resume, &m_quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Chokh - চোখ")
                .menu(&menu)
                .show_menu_on_left_click(false) // বাম ক্লিকে জানালা, ডান ক্লিকে মেনু
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_main(app),
                    "break_now" => trigger_break(app),
                    "pause1" => app.state::<AppState>().pause(1),
                    "pause2" => app.state::<AppState>().pause(2),
                    "resume" => app.state::<AppState>().resume(),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;

            // ৫) টাইমার চালু
            spawn_timer(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Chokh চালু করতে সমস্যা হয়েছে");
}
