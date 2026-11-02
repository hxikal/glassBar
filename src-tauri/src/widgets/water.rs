//! Water reminder: today's drinks (SQLite, so they survive a restart), the day's goal, and
//! reminders through the scheduler, shown as a Dock alert with "+ cup" and "Later".
//! The day starts at local midnight. Settings live in the widget's settings (Water widget).
use crate::db::{kv_get, kv_set, now_ms, Db};
use crate::scheduler::Due;
use chrono::{Local, TimeZone};
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

const DAY: i64 = 86_400_000;
const HOUR: i64 = 3_600_000;

/// The widget's settings that Rust needs, with their defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct Cfg { pub dynamic: bool, pub goal: u32, pub weight: f64, pub hot_bonus: bool, pub cup: u32,
                 pub remind: bool, pub every_min: i64, pub from_h: i64, pub to_h: i64 }

pub fn cfg(s: &Value) -> Cfg {
    // Choice buttons save text ("250"), sliders numbers: read both.
    let n = |k: &str, d: f64| s[k].as_f64().or_else(|| s[k].as_str()?.parse().ok()).unwrap_or(d);
    Cfg {
        dynamic: s["goalMode"].as_str() == Some("dynamic"),
        goal: n("goal", 2000.0).clamp(500.0, 6000.0) as u32,
        weight: n("weight", 60.0).clamp(20.0, 250.0),
        hot_bonus: s["hotBonus"].as_bool().unwrap_or(true),
        cup: n("cup", 250.0).clamp(50.0, 1500.0) as u32,
        remind: s["remind"].as_bool().unwrap_or(true),
        every_min: n("every", 60.0).clamp(15.0, 240.0) as i64,
        from_h: n("fromHour", 8.0).clamp(0.0, 23.0) as i64,
        to_h: n("toHour", 22.0).clamp(1.0, 24.0) as i64,
    }
}

/// Pure: today's goal in ml. Dynamic = 35 ml per kg, plus 10 % on a hot day (30 °C+) when the
/// Weather widget has a current temperature -- never a guess when it doesn't.
pub fn goal_ml(c: &Cfg, temp_c: Option<f64>) -> u32 {
    if !c.dynamic { return c.goal; }
    let base = c.weight * 35.0;
    let hot = c.hot_bonus && temp_c.is_some_and(|t| t >= 30.0);
    ((if hot { base * 1.1 } else { base } / 50.0).round() * 50.0).clamp(500.0, 6000.0) as u32
}

/// Pure: the reminder that is due (latest one at or before `now`) and the next one, as epoch
/// ms. Reminders count `every` from the later of the last drink today, the start of the
/// active hours, and a snooze; none fall after the active hours end (then: tomorrow).
/// ponytail: a day is 24 h from local midnight, so a DST change day is an hour off at its end.
pub fn reminder_times(day0: i64, now: i64, last: Option<i64>, snooze: Option<i64>, every: i64, from: i64, to: i64) -> (Option<i64>, i64) {
    let (start, end) = (day0 + from, day0 + to);
    let mut anchor = last.filter(|&l| l >= day0).unwrap_or(start).max(start);
    if let Some(s) = snooze { anchor = anchor.max(s - every); }
    let first = anchor + every;
    let k = if now >= first { (now - first) / every } else { -1 };
    let past = (k >= 0).then(|| first + k * every).filter(|&p| p <= end);
    let next = first + (k + 1) * every;
    (past, if next <= end { next } else { day0 + DAY + from + every })
}

fn day0(now: i64) -> i64 {
    let d = Local.timestamp_millis_opt(now).single().map(|t| t.date_naive()).unwrap_or_default();
    d.and_hms_opt(0, 0, 0).and_then(|t| Local.from_local_datetime(&t).earliest()).map(|t| t.timestamp_millis()).unwrap_or(now - now % DAY)
}

/// The current temperature in °C from the Weather widget's cache, if it has one.
fn temp_c(app: &AppHandle) -> Option<f64> {
    let db = app.state::<Db>();
    let w: Value = serde_json::from_str(&kv_get(&db.lock(), "weather")?).ok()?;
    let t = w["current"]["temp"].as_f64()?;
    Some(if w["unit"].as_str() == Some("f") { (t - 32.0) * 5.0 / 9.0 } else { t })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry { at: i64, ml: u32 }
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Day { day0: i64, ml: u32 }
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Today { total: u32, goal: u32, cup: u32, entries: Vec<Entry>, week: Vec<Day>, hot: bool,
                   /// When the next reminder shows (epoch ms); None when reminders are off or the goal is met.
                   next_reminder: Option<i64> }

/// "ml" or "oz": how the dock alert writes amounts.
fn unit(app: &AppHandle) -> String {
    app.state::<crate::AppState>().config.lock().unwrap().widget_settings("water")["unit"].as_str().unwrap_or("ml").to_string()
}

fn settings(app: &AppHandle) -> Cfg { cfg(&app.state::<crate::AppState>().config.lock().unwrap().widget_settings("water")) }

fn today(app: &AppHandle) -> Today {
    let c = settings(app);
    let t = temp_c(app);
    let d0 = day0(now_ms());
    let db = app.state::<Db>();
    let conn = db.lock();
    let rows: Vec<(i64, u32)> = conn.prepare("SELECT at, ml FROM water WHERE at >= ?1 ORDER BY at")
        .and_then(|mut q| q.query_map([d0 - 6 * DAY], |r| Ok((r.get(0)?, r.get(1)?)))?.collect())
        .unwrap_or_default();
    let week = (0..7).rev().map(|i| {
        let s = d0 - i * DAY;
        Day { day0: s, ml: rows.iter().filter(|(at, _)| *at >= s && *at < s + DAY).map(|r| r.1).sum() }
    }).collect();
    let entries: Vec<Entry> = rows.iter().filter(|(at, _)| *at >= d0).map(|&(at, ml)| Entry { at, ml }).collect();
    let (total, goal) = (entries.iter().map(|e| e.ml).sum(), goal_ml(&c, t));
    let next_reminder = (c.remind && total < goal).then(|| {
        let snooze = kv_get(&conn, "water:snooze").and_then(|s| s.parse().ok());
        reminder_times(d0, now_ms(), entries.last().map(|e| e.at), snooze, c.every_min * 60_000, c.from_h * HOUR, c.to_h * HOUR).1
    });
    Today { total, goal, cup: c.cup, entries, week, hot: c.dynamic && c.hot_bonus && t.is_some_and(|t| t >= 30.0), next_reminder }
}

fn changed(app: &AppHandle) -> Today {
    crate::scheduler::poke(); // the next reminder counts from this drink
    let t = today(app);
    let _ = app.emit("water:changed", &t);
    t
}

#[tauri::command]
pub async fn water_today(app: AppHandle) -> Result<Today, String> { crate::widgets::blocking(move || today(&app)).await }

/// Log a drink. Crossing the day's goal shows one "goal reached" alert, once a day.
#[tauri::command]
pub async fn water_add(app: AppHandle, ml: u32) -> Result<Today, String> {
    crate::widgets::blocking(move || {
        let before = today(&app);
        {
            let db = app.state::<Db>();
            let _ = db.lock().execute("INSERT INTO water(at, ml) VALUES(?1, ?2)", rusqlite::params![now_ms(), ml.clamp(1, 3000)]);
        }
        let t = changed(&app);
        let key = format!("water:goal:{}", day0(now_ms()));
        if before.total < t.goal && t.total >= t.goal {
            let db = app.state::<Db>();
            let first = { let c = db.lock(); let f = kv_get(&c, &key).is_none(); kv_set(&c, &key, "1"); f };
            if first { let _ = app.emit_to("bar", "dock:alert", serde_json::json!({ "kind": "water", "total": t.total, "goal": t.goal, "cup": t.cup, "unit": unit(&app), "reached": true })); }
        }
        t
    }).await
}

#[tauri::command]
pub async fn water_remove(app: AppHandle, at: i64) -> Result<Today, String> {
    crate::widgets::blocking(move || {
        { let db = app.state::<Db>(); let _ = db.lock().execute("DELETE FROM water WHERE at = ?1", [at]); }
        changed(&app)
    }).await
}

/// "Later": no reminder for `minutes`.
#[tauri::command]
pub async fn water_snooze(app: AppHandle, minutes: i64) -> Result<(), String> {
    crate::widgets::blocking(move || {
        { let db = app.state::<Db>(); kv_set(&db.lock(), "water:snooze", &(now_ms() + minutes.clamp(1, 240) * 60_000).to_string()); }
        changed(&app); // the countdown on the widget moves too
    }).await
}

/// Scheduler items: the reminder due now (if not yet fired) and the next one.
pub fn due_items(app: &AppHandle) -> Vec<Due> {
    let cfgs = app.state::<crate::AppState>().config.lock().unwrap().clone();
    if !cfgs.uses("water") { return vec![]; }
    let c = cfg(&cfgs.widget_settings("water"));
    if !c.remind { return vec![]; }
    let t = today(app);
    if t.total >= t.goal { return vec![]; } // done for today
    let now = now_ms();
    let snooze = { let db = app.state::<Db>(); let c = db.lock(); kv_get(&c, "water:snooze").and_then(|s| s.parse().ok()) };
    let (past, next) = reminder_times(day0(now), now, t.entries.last().map(|e| e.at), snooze, c.every_min * 60_000, c.from_h * HOUR, c.to_h * HOUR);
    past.into_iter().chain([next]).map(|at| Due {
        key: format!("water:{at}"), at, source: "water", title: String::new(), body: String::new(), sound: None,
    }).collect()
}

/// A reminder is due: the dock says so (nothing missed while away, nothing in Do Not Disturb).
pub fn remind(app: &AppHandle, missed: bool) {
    let t = today(app);
    let _ = app.emit("water:changed", &t); // the widget's countdown moves on to the next one
    if missed || app.state::<crate::AppState>().config.lock().unwrap().dnd { return; }
    let _ = app.emit_to("bar", "dock:alert", serde_json::json!({ "kind": "water", "total": t.total, "goal": t.goal, "cup": t.cup, "unit": unit(app) }));
}

#[cfg(test)]
mod tests {
    use super::*;
    const D0: i64 = 1_000 * DAY; // a local midnight
    const M: i64 = 60_000;
    const EVERY: i64 = 60 * M;
    /// Active 08:00 to 22:00.
    fn at(now: i64, last: Option<i64>, snooze: Option<i64>) -> (Option<i64>, i64) {
        reminder_times(D0, now, last, snooze, EVERY, 8 * HOUR, 22 * HOUR)
    }

    #[test]
    fn first_reminder_comes_an_interval_after_the_day_starts() {
        assert_eq!(at(D0 + 7 * HOUR, None, None), (None, D0 + 9 * HOUR));
    }

    #[test]
    fn counts_from_the_last_drink() {
        let drink = D0 + 10 * HOUR + 20 * M;
        assert_eq!(at(drink + M, Some(drink), None), (None, drink + EVERY));
    }

    #[test]
    fn keeps_reminding_every_interval_while_nothing_is_logged() {
        let drink = D0 + 9 * HOUR;
        // 10:30: the 10:00 one is due, 11:00 is next.
        assert_eq!(at(D0 + 10 * HOUR + 30 * M, Some(drink), None), (Some(D0 + 10 * HOUR), D0 + 11 * HOUR));
    }

    #[test]
    fn later_pushes_the_next_one_out() {
        let snooze = D0 + 10 * HOUR + 15 * M;
        assert_eq!(at(D0 + 10 * HOUR, Some(D0 + 9 * HOUR), Some(snooze)).1, snooze);
    }

    #[test]
    fn nothing_after_the_active_hours_then_tomorrow_morning() {
        let (past, next) = at(D0 + 22 * HOUR + 30 * M, Some(D0 + 21 * HOUR + 30 * M), None);
        assert_eq!(past, None, "22:30 is past the end");
        assert_eq!(next, D0 + DAY + 9 * HOUR);
    }

    #[test]
    fn yesterdays_drink_does_not_count_today() {
        assert_eq!(at(D0 + 8 * HOUR, Some(D0 - HOUR), None).1, D0 + 9 * HOUR);
    }

    #[test]
    fn dynamic_goal_follows_weight_and_hot_days_only_with_real_weather() {
        let c = Cfg { dynamic: true, weight: 60.0, ..cfg(&Value::Null) };
        assert_eq!(goal_ml(&c, None), 2100);
        assert_eq!(goal_ml(&c, Some(25.0)), 2100);
        assert_eq!(goal_ml(&c, Some(32.0)), 2300, "+10 %, to the nearest 50 ml");
        assert_eq!(goal_ml(&Cfg { dynamic: false, ..c.clone() }, Some(40.0)), 2000);
    }

    #[test]
    fn settings_have_safe_defaults() {
        let c = cfg(&Value::Null);
        assert_eq!((c.goal, c.cup, c.every_min, c.from_h, c.to_h, c.remind), (2000, 250, 60, 8, 22, true));
        assert_eq!(cfg(&serde_json::json!({ "cup": "330" })).cup, 330, "a choice button saves text");
    }
}
