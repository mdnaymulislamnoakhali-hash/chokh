// =====================================================================
// Chokh - চোখ | Rust ব্যাকএন্ড (ধাপ ২ + ৩)
//   • ট্রে, টাইমার, ব্রেক ওভারলে
//   • সেটিংস (settings.json) ও দৈনিক স্ট্যাটস (stats.json)
//   • অটো-স্টার্ট, পানি/পশ্চার নোটিফিকেশন
//   • ক্রেডিট দেখানোর নিয়ম (দিনে সর্বোচ্চ ২ বার)
// লক রাখার নিয়ম (ডেডলক এড়াতে): সবসময় settings → timer → stats ক্রমে, আর একসাথে দুটো ধরে রাখা হয় না।
// =====================================================================

// রিলিজ বিল্ডে কালো কনসোল জানালা দেখাবে না
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;

use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::Mutex,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono::{Local, Timelike};
use serde::{Deserialize, Serialize};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    window::{Effect, EffectsBuilder},
    AppHandle, Emitter, Manager, State, WindowEvent,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as _};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

const ALLOWED_INTERVALS: [u64; 5] = [15, 20, 30, 45, 60];
const WATER_EVERY_SECS: u64 = 60 * 60;
const POSTURE_EVERY_SECS: u64 = 45 * 60;
/// ব্রেক স্ক্রিন ৫ মিনিটের বেশি আটকে থাকলে নিজে বন্ধ হবে
const BREAK_FAILSAFE_SECS: u64 = 300;
/// থ্রেড এর চেয়ে বেশি সময় "ঘুমালে" ধরা হয় পিসি স্লিপে ছিল
const SLEEP_GAP_SECS: u64 = 120;
/// স্ট্যাটস কত দিনের রাখা হবে
const KEEP_DAYS: usize = 90;

// ---------------------------------------------------------------------
// সেটিংস (settings.json)
// ---------------------------------------------------------------------
#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
struct Settings {
    interval_min: u64,
    sound: bool,
    autostart: bool,
    lang: String, // "bn" | "en"
    water: bool,
    posture: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            interval_min: 20,
            sound: true,
            autostart: false,
            lang: "bn".into(),
            water: false,
            posture: false,
        }
    }
}

impl Settings {
    /// ফাইল হাতে বদলানো বা ফ্রন্টএন্ড থেকে ভুল মান এলে ঠিক করে দেয়
    fn sanitize(&mut self) {
        if !ALLOWED_INTERVALS.contains(&self.interval_min) {
            self.interval_min = 20;
        }
        if self.lang != "bn" && self.lang != "en" {
            self.lang = "bn".into();
        }
    }
}

fn load_settings(path: &PathBuf) -> Settings {
    let mut s: Settings = fs::read_to_string(path)
        .ok()
        .and_then(|txt| serde_json::from_str(&txt).ok())
        .unwrap_or_default();
    s.sanitize();
    s
}

fn save_settings(path: &PathBuf, s: &Settings) {
    if let Ok(txt) = serde_json::to_string_pretty(s) {
        let _ = fs::write(path, txt);
    }
}

// ---------------------------------------------------------------------
// দৈনিক স্ট্যাটস ও ক্রেডিটের হিসাব (stats.json), সব ডেটা শুধু এই পিসিতে
// ---------------------------------------------------------------------
#[derive(Serialize, Deserialize, Default, Clone)]
struct DayStats {
    taken: u32,
    skipped: u32,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct StatsFile {
    days: BTreeMap<String, DayStats>, // "2026-10-29" → সংখ্যা
    credit_day: String,               // শেষ যে দিনের জন্য নিচের দুটো ফ্ল্যাগ
    credit_morning: bool,             // আজ সকালের ক্রেডিট দেখানো হয়েছে?
    credit_evening: bool,             // আজ সন্ধ্যার ক্রেডিট দেখানো হয়েছে?
}

fn load_stats(path: &PathBuf) -> StatsFile {
    fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_stats(path: &PathBuf, s: &StatsFile) {
    if let Ok(txt) = serde_json::to_string_pretty(s) {
        let _ = fs::write(path, txt);
    }
}

fn today_key() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn today_counts(s: &StatsFile) -> (u32, u32) {
    s.days
        .get(&today_key())
        .map(|d| (d.taken, d.skipped))
        .unwrap_or((0, 0))
}

fn record_break(state: &AppState, taken: bool) {
    let mut st = state.stats.lock().unwrap();
    {
        let e = st.days.entry(today_key()).or_default();
        if taken {
            e.taken += 1;
        } else {
            e.skipped += 1;
        }
    }
    while st.days.len() > KEEP_DAYS {
        let oldest = st.days.keys().next().cloned();
        match oldest {
            Some(k) => {
                st.days.remove(&k);
            }
            None => break,
        }
    }
    save_stats(&state.stats_path, &st);
}

/// ০–৯৯ এর একটা এলোমেলো সংখ্যা (বাড়তি crate ছাড়া; মাইক্রোসেকেন্ড অংশ ব্যবহার করা হয়েছে)
fn rand_percent() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.subsec_micros() % 100) as u64)
        .unwrap_or(0)
}

/// ক্রেডিট দেখাব কি না: সকাল ১০–১টা ও সন্ধ্যা ৬–১০টার জানালায় একবার করে, দিনে সর্বোচ্চ ২ বার, এলোমেলোভাবে
fn pick_credit(st: &mut StatsFile) -> bool {
    let now = Local::now();
    let day = now.format("%Y-%m-%d").to_string();
    if st.credit_day != day {
        st.credit_day = day;
        st.credit_morning = false;
        st.credit_evening = false;
    }
    let h = now.hour();
    let in_morning = (10..13).contains(&h);
    let in_evening = (18..22).contains(&h);
    let eligible = (in_morning && !st.credit_morning) || (in_evening && !st.credit_evening);
    if !eligible || rand_percent() >= config::CREDIT_CHANCE_PERCENT {
        return false;
    }
    if in_morning {
        st.credit_morning = true;
    } else {
        st.credit_evening = true;
    }
    true
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
    next_break: u64,
    paused_until: Option<u64>,
    break_started: Option<u64>,
    break_preview: bool, // "দেখে নিন" বাটনের ব্রেক: স্ট্যাটসে গোনা হয় না
    next_water: u64,
    next_posture: u64,
}

struct AppState {
    settings: Mutex<Settings>,
    timer: Mutex<Timer>,
    stats: Mutex<StatsFile>,
    blur_ok: bool,
    settings_path: PathBuf,
    stats_path: PathBuf,
}

impl AppState {
    fn pause(&self, hours: u64) {
        let mut t = self.timer.lock().unwrap();
        t.paused_until = Some(now() + hours * 3600);
    }

    fn resume(&self) {
        let iv = self.settings.lock().unwrap().interval_min;
        let n = now();
        let mut t = self.timer.lock().unwrap();
        t.paused_until = None;
        t.next_break = n + iv * 60;
        t.next_water = n + WATER_EVERY_SECS;
        t.next_posture = n + POSTURE_EVERY_SECS;
    }
}

#[derive(Serialize)]
struct Status {
    settings: Settings,
    secs_until_break: u64,
    pause_secs_left: u64,
    break_active: bool,
    today_taken: u32,
    today_skipped: u32,
}

#[derive(Serialize, Clone)]
struct BreakPayload {
    blur: bool,     // Windows acrylic blur কাজ করছে কি না
    sound: bool,    // "টিং" বাজবে কি না
    lang: String,   // "bn" | "en"
    credit: bool,   // ক্রেডিট ফুটার দেখাবে কি না
    fb_ready: bool, // ফেসবুক লিংক বসানো আছে কি না
    taken: u32,     // আজ এখন পর্যন্ত রেস্ট
    skipped: u32,   // আজ এখন পর্যন্ত Skip
    preview: bool,
}

// ---------------------------------------------------------------------
// রিমাইন্ডার নোটিফিকেশন
// ---------------------------------------------------------------------
#[derive(Clone, Copy)]
enum Reminder {
    Water,
    Posture,
}

fn send_reminder(app: &AppHandle, lang: &str, kind: Reminder) {
    let (title, body) = match (lang, kind) {
        ("en", Reminder::Water) => ("💧 Time for water", "Have a glass of water."),
        ("en", Reminder::Posture) => ("🪑 Posture check", "Straighten your back and relax your shoulders."),
        (_, Reminder::Water) => ("💧 পানি খাওয়ার সময়", "এক গ্লাস পানি খেয়ে নিন।"),
        (_, Reminder::Posture) => ("🪑 সোজা হয়ে বসুন", "পিঠ সোজা করুন, কাঁধ নামিয়ে রিল্যাক্স করুন।"),
    };
    let _ = app.notification().builder().title(title).body(body).show();
}

// ---------------------------------------------------------------------
// ব্রেক শুরু ও শেষ
// ---------------------------------------------------------------------
fn trigger_break(app: &AppHandle, preview: bool) {
    let state = app.state::<AppState>();
    let settings = state.settings.lock().unwrap().clone();
    {
        let mut t = state.timer.lock().unwrap();
        if t.break_started.is_some() {
            return; // আগের ব্রেক এখনও চলছে
        }
        t.break_started = Some(now());
        t.break_preview = preview;
    }
    let (credit, taken, skipped) = {
        let mut st = state.stats.lock().unwrap();
        // "দেখে নিন" বাটনে সবসময় ক্রেডিট দেখানো হয় (ডিজাইন যাচাইয়ের জন্য), তবে দিনের হিসাবে গোনা হয় না
        let credit = if preview {
            true
        } else {
            let c = pick_credit(&mut st);
            if c {
                save_stats(&state.stats_path, &st);
            }
            c
        };
        let (a, b) = today_counts(&st);
        (credit, a, b)
    };
    if let Some(win) = app.get_webview_window("break") {
        let _ = win.show();
        let _ = win.set_always_on_top(true);
        let _ = win.set_focus();
        let _ = app.emit_to(
            "break",
            "break-start",
            BreakPayload {
                blur: state.blur_ok,
                sound: settings.sound,
                lang: settings.lang,
                credit,
                fb_ready: config::facebook_ready(),
                taken,
                skipped,
                preview,
            },
        );
    }
}

/// record = Some(true) রেস্ট, Some(false) Skip, None = গোনা হবে না (যেমন ফেসবুক খুলতে গেলে)
fn end_break_inner(app: &AppHandle, record: Option<bool>) {
    let state = app.state::<AppState>();
    let iv = state.settings.lock().unwrap().interval_min;
    let preview;
    {
        let mut t = state.timer.lock().unwrap();
        if t.break_started.take().is_none() {
            return; // কোনো ব্রেক চলছিল না
        }
        preview = t.break_preview;
        t.break_preview = false;
        t.next_break = now() + iv * 60;
    }
    if !preview {
        if let Some(taken) = record {
            record_break(&state, taken);
            // 🔜 ধাপ ৪: এখানে অ্যানালিটিক্স ইভেন্ট (break_taken / break_skipped) পাঠানো হবে।
        }
    }
    if let Some(win) = app.get_webview_window("break") {
        let _ = win.hide();
    }
}

fn end_break(app: &AppHandle, taken: bool) {
    end_break_inner(app, Some(taken));
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn apply_autostart(app: &AppHandle, enable: bool) {
    let m = app.autolaunch();
    let _ = if enable { m.enable() } else { m.disable() };
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

            let state = app.state::<AppState>();
            let s = state.settings.lock().unwrap().clone();

            // লক ধরে সিদ্ধান্ত নেই, লক ছেড়ে তারপর কাজ করি
            let mut start_break = false;
            let mut force_end = false;
            let mut water_due = false;
            let mut posture_due = false;
            {
                let mut t = state.timer.lock().unwrap();

                // পিসি স্লিপ থেকে উঠলে সব কাউন্টডাউন নতুন করে শুরু
                if gap > SLEEP_GAP_SECS && t.break_started.is_none() {
                    t.next_break = n + s.interval_min * 60;
                    t.next_water = n + WATER_EVERY_SECS;
                    t.next_posture = n + POSTURE_EVERY_SECS;
                }

                if let Some(started) = t.break_started {
                    if n.saturating_sub(started) > BREAK_FAILSAFE_SECS {
                        force_end = true;
                    }
                } else if let Some(p) = t.paused_until {
                    if n >= p {
                        t.paused_until = None;
                        t.next_break = n + s.interval_min * 60;
                        t.next_water = n + WATER_EVERY_SECS;
                        t.next_posture = n + POSTURE_EVERY_SECS;
                    }
                } else {
                    if n >= t.next_break {
                        start_break = true;
                    }
                    if s.water && n >= t.next_water {
                        water_due = true;
                        t.next_water = n + WATER_EVERY_SECS;
                    }
                    if s.posture && n >= t.next_posture {
                        posture_due = true;
                        t.next_posture = n + POSTURE_EVERY_SECS;
                    }
                }
            }

            if force_end {
                end_break(&app, false);
            } else if start_break {
                trigger_break(&app, false);
            } else {
                // ব্রেক আর রিমাইন্ডার একই মুহূর্তে এলে ব্রেক অগ্রাধিকার পায়
                if water_due {
                    send_reminder(&app, &s.lang, Reminder::Water);
                }
                if posture_due {
                    send_reminder(&app, &s.lang, Reminder::Posture);
                }
            }
        }
    });
}

// ---------------------------------------------------------------------
// কমান্ড: ফ্রন্টএন্ড (JS) থেকে invoke() দিয়ে ডাকা হয়
// ---------------------------------------------------------------------
#[tauri::command]
fn get_status(state: State<'_, AppState>) -> Status {
    let settings = state.settings.lock().unwrap().clone();
    let n = now();
    let (secs_until_break, pause_secs_left, break_active) = {
        let t = state.timer.lock().unwrap();
        (
            t.next_break.saturating_sub(n),
            t.paused_until.map(|p| p.saturating_sub(n)).unwrap_or(0),
            t.break_started.is_some(),
        )
    };
    let (today_taken, today_skipped) = {
        let st = state.stats.lock().unwrap();
        today_counts(&st)
    };
    Status {
        settings,
        secs_until_break,
        pause_secs_left,
        break_active,
        today_taken,
        today_skipped,
    }
}

#[tauri::command]
fn set_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    let mut new = settings;
    new.sanitize();
    let old = {
        let mut s = state.settings.lock().unwrap();
        let old = s.clone();
        *s = new.clone();
        old
    };
    {
        let n = now();
        let mut t = state.timer.lock().unwrap();
        if old.interval_min != new.interval_min {
            t.next_break = n + new.interval_min * 60;
        }
        if !old.water && new.water {
            t.next_water = n + WATER_EVERY_SECS;
        }
        if !old.posture && new.posture {
            t.next_posture = n + POSTURE_EVERY_SECS;
        }
    }
    if old.autostart != new.autostart {
        apply_autostart(&app, new.autostart);
    }
    save_settings(&state.settings_path, &new);
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

/// মূল জানালার "দেখে নিন" বাটন: স্ট্যাটসে গোনা হয় না, ক্রেডিট সবসময় দেখায়
#[tauri::command]
fn start_break_now(app: AppHandle) {
    trigger_break(&app, true);
}

#[tauri::command]
fn finish_break(app: AppHandle, taken: bool) {
    end_break(&app, taken);
}

/// ফেসবুক লিংক Rust-এ বাঁধা (JS থেকে ইচ্ছেমতো URL খোলা যায় না, এটা নিরাপত্তার জন্য)
/// ওভারলে সব কিছুর ওপরে থাকে, তাই ব্রাউজার দেখাতে ব্রেক শেষ করা হয় (Skip বা রেস্ট কোনোটাই গোনা হয় না)
#[tauri::command]
fn open_facebook(app: AppHandle) {
    if config::facebook_ready() {
        let _ = app.opener().open_url(config::FACEBOOK_PAGE_URL, None::<&str>);
        end_break_inner(&app, None);
    }
}

// ---------------------------------------------------------------------
// অ্যাপ চালু
// ---------------------------------------------------------------------
fn main() {
    tauri::Builder::default()
        // একই অ্যাপ দুবার চালালে পুরনোর জানালা দেখায় (এই প্লাগইন সবার আগে)
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main(app);
        }))
        // অটো-স্টার্টে "--hidden" দিয়ে চালু হয়, তাই লগইনের সময় জানালা লাফিয়ে ওঠে না
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        // ✕ চাপলে ট্রে-তে লুকাবে। ব্রেক জানালা Alt+F4 দিলেও "Skip" ধরা হবে।
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
            set_settings,
            pause,
            resume,
            start_break_now,
            finish_break,
            open_facebook
        ])
        .setup(|app| {
            // ১) সেটিংস ও স্ট্যাটস লোড
            let dir = app.path().app_config_dir()?;
            let _ = fs::create_dir_all(&dir);
            let settings_path = dir.join("settings.json");
            let stats_path = dir.join("stats.json");
            let settings = load_settings(&settings_path);
            let stats = load_stats(&stats_path);

            // ২) ব্রেক জানালার পেছনে Windows acrylic blur (না হলে CSS দিয়ে গাঢ় করা হবে)
            let blur_ok = app
                .get_webview_window("break")
                .map(|w| w.set_effects(EffectsBuilder::new().effect(Effect::Acrylic).build()).is_ok())
                .unwrap_or(false);

            // ৩) টাইমারের অবস্থা
            let n = now();
            app.manage(AppState {
                timer: Mutex::new(Timer {
                    next_break: n + settings.interval_min * 60,
                    paused_until: None,
                    break_started: None,
                    break_preview: false,
                    next_water: n + WATER_EVERY_SECS,
                    next_posture: n + POSTURE_EVERY_SECS,
                }),
                settings: Mutex::new(settings.clone()),
                stats: Mutex::new(stats),
                blur_ok,
                settings_path,
                stats_path,
            });

            // ৪) অটো-স্টার্টের রেজিস্ট্রি এন্ট্রি সেটিংসের সাথে মিলিয়ে নেওয়া (অ্যাপ সরালে/আপডেটে পথ বদলাতে পারে)
            apply_autostart(app.handle(), settings.autostart);

            // ৫) মূল জানালা: অটো-স্টার্টে লুকানো, নিজে চালালে দেখানো
            let hidden = std::env::args().any(|a| a == "--hidden");
            if !hidden {
                show_main(app.handle());
            }

            // ৬) ট্রে আইকন ও মেনু
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
                    "break_now" => trigger_break(app, false),
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

            // ৭) টাইমার চালু
            spawn_timer(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Chokh চালু করতে সমস্যা হয়েছে");
}
