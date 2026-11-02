//! Waktu Solat: JAKIM e-Solat for Malaysian zones (zone auto-detected from
//! location via waktusolat.app), Aladhan by coordinates elsewhere.
use crate::db::{kv_get, kv_set, now_ms, Db};
use crate::net::get_json;
use crate::scheduler::Due;
use crate::widgets::location;
use base64::Engine;
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveTime, TimeZone};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager};

const ZONES: &str = include_str!("jakim_zones.json");
/// Prayers that get an alert (Imsak and Syuruk are shown but not alerted).
const PRAYERS: [&str; 5] = ["Subuh", "Zohor", "Asar", "Maghrib", "Isyak"];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PrayerDay {
    /// YYYY-MM-DD, local to the zone.
    pub date: String,
    /// JAKIM zone code, or "" for Aladhan.
    pub zone: String,
    /// "jakim" | "aladhan"
    pub source: String,
    pub hijri: Option<String>,
    /// (Imsak|Subuh|Syuruk|Zohor|Asar|Maghrib|Isyak, Unix ms)
    pub times: Vec<(String, i64)>,
}

/// JAKIM dates look like "23-Okt-2026" (Malay month abbreviations).
pub fn parse_malay_date(s: &str) -> Option<NaiveDate> {
    let mut it = s.split('-');
    let (d, m, y) = (it.next()?.parse().ok()?, it.next()?, it.next()?.parse().ok()?);
    const MONTHS: [&[&str]; 12] = [&["jan"], &["feb"], &["mac", "mar"], &["apr"], &["mei", "may"], &["jun"],
        &["jul"], &["ogos", "aug"], &["sep"], &["okt", "oct"], &["nov"], &["dis", "dec"]];
    let m = m.to_lowercase();
    let month = MONTHS.iter().position(|names| names.contains(&m.as_str()))? as u32 + 1;
    NaiveDate::from_ymd_opt(y, month, d)
}

fn myt() -> FixedOffset { FixedOffset::east_opt(8 * 3600).unwrap() }

pub fn parse_jakim(v: &Value, zone: &str) -> Vec<PrayerDay> {
    const KEYS: [(&str, &str); 7] = [("imsak", "Imsak"), ("fajr", "Subuh"), ("syuruk", "Syuruk"), ("dhuhr", "Zohor"),
        ("asr", "Asar"), ("maghrib", "Maghrib"), ("isha", "Isyak")];
    v["prayerTime"].as_array().map(|days| days.iter().filter_map(|d| {
        let date = parse_malay_date(d["date"].as_str()?)?;
        let times = KEYS.iter().map(|(k, name)| {
            let t = NaiveTime::parse_from_str(d[*k].as_str()?, "%H:%M:%S").ok()?;
            Some((name.to_string(), myt().from_local_datetime(&date.and_time(t)).single()?.timestamp_millis()))
        }).collect::<Option<Vec<_>>>()?;
        Some(PrayerDay { date: date.to_string(), zone: zone.into(), source: "jakim".into(), hijri: d["hijri"].as_str().map(Into::into), times })
    }).collect()).unwrap_or_default()
}

/// Aladhan with `iso8601=true`: times carry their UTC offset.
pub fn parse_aladhan(v: &Value) -> Option<PrayerDay> {
    const KEYS: [(&str, &str); 7] = [("Imsak", "Imsak"), ("Fajr", "Subuh"), ("Sunrise", "Syuruk"), ("Dhuhr", "Zohor"),
        ("Asr", "Asar"), ("Maghrib", "Maghrib"), ("Isha", "Isyak")];
    let t = &v["data"]["timings"];
    let parsed = KEYS.iter().map(|(k, name)| Some((name.to_string(), DateTime::parse_from_rfc3339(t[*k].as_str()?).ok()?)))
        .collect::<Option<Vec<_>>>()?;
    Some(PrayerDay {
        date: parsed[1].1.date_naive().to_string(),
        zone: String::new(),
        source: "aladhan".into(),
        hijri: v["data"]["date"]["hijri"]["date"].as_str().map(Into::into),
        times: parsed.into_iter().map(|(n, d)| (n, d.timestamp_millis())).collect(),
    })
}

/// The next prayer after `now` today (Syuruk never counts; Imsak only if asked).
pub fn next_prayer(day: &PrayerDay, now: i64, include_imsak: bool) -> Option<(String, i64)> {
    day.times.iter()
        .filter(|(n, _)| n != "Syuruk" && (include_imsak || n != "Imsak"))
        .find(|(_, t)| *t > now)
        .cloned()
}

/// Which source/zone `days()` last resolved; only it produces alerts.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Active { pub source: String, pub zone: String }

pub fn active_days(cached: Vec<PrayerDay>, a: &Active) -> Vec<PrayerDay> {
    cached.into_iter().filter(|d| d.source == a.source && d.zone == a.zone).collect()
}

fn set_active(app: &AppHandle, source: &str, zone: &str) {
    kv_put(app, "solat:active", &serde_json::to_string(&Active { source: source.into(), zone: zone.into() }).unwrap());
}

fn settings(app: &AppHandle) -> Value { app.state::<crate::AppState>().config.lock().unwrap().widget_settings("solat") }

fn kv(app: &AppHandle, key: &str) -> Option<String> { kv_get(&app.state::<Db>().lock(), key) }
fn kv_put(app: &AppHandle, key: &str, v: &str) { kv_set(&app.state::<Db>().lock(), key, v) }

/// JAKIM zone: the user's pick, else looked up from location ("" = outside Malaysia).
fn zone_for(app: &AppHandle, place: Option<&location::Place>) -> Option<String> {
    if let Some(z) = settings(app)["zone"].as_str().filter(|z| !z.is_empty() && *z != "auto") { return Some(z.into()); }
    let p = place?;
    let key = format!("solat:zone:{:.2},{:.2}", p.lat, p.lon);
    if let Some(z) = kv(app, &key) { return Some(z); }
    let v = get_json(&format!("https://api.waktusolat.app/zones/{}/{}", p.lat, p.lon)).ok()?;
    let z = v["zone"].as_str().unwrap_or("").to_string(); // "error" response = not in Malaysia
    kv_put(app, &key, &z);
    Some(z)
}

fn today_myt() -> NaiveDate { chrono::Utc::now().with_timezone(&myt()).date_naive() }

/// Today's and tomorrow's times (fetches when the cache doesn't cover them, or `force`;
/// offline, the cache is kept).
pub fn days(app: &AppHandle, force: bool) -> Result<Vec<PrayerDay>, String> {
    let place = location::current(app).ok();
    let zone = zone_for(app, place.as_ref());
    match zone.as_deref() {
        Some(z) if !z.is_empty() => {
            set_active(app, "jakim", z);
            let key = format!("solat:week:{z}");
            let today = today_myt();
            let want = [today.to_string(), today.succ_opt().unwrap().to_string()];
            let cached: Vec<PrayerDay> = kv(app, &key).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
            let covered = want.iter().all(|d| cached.iter().any(|p| &p.date == d));
            let all = if covered && !force { cached } else {
                match get_json(&format!("https://www.e-solat.gov.my/index.php?r=esolatApi/takwimsolat&period=week&zone={z}")) {
                    Ok(v) => {
                        let fresh = parse_jakim(&v, z);
                        if fresh.is_empty() { cached } else { kv_put(app, &key, &serde_json::to_string(&fresh).unwrap()); fresh }
                    }
                    Err(_) => cached, // offline: whatever we have
                }
            };
            Ok(all.into_iter().filter(|p| p.date >= want[0]).take(2).collect())
        }
        _ => {
            let p = place.ok_or("Location unavailable: set it in Settings → Location, or pick a zone in the Waktu Solat options")?;
            set_active(app, "aladhan", "");
            let mut out = vec![];
            for offset in 0..2 {
                let date = (chrono::Local::now() + chrono::Duration::days(offset)).date_naive();
                let key = format!("solat:aladhan:{date}:{:.2},{:.2}", p.lat, p.lon);
                let cached = kv(app, &key).and_then(|s| serde_json::from_str::<PrayerDay>(&s).ok());
                let day = match cached.clone().filter(|_| !force) {
                    Some(d) => Some(d),
                    None => get_json(&format!("https://api.aladhan.com/v1/timings/{}?latitude={}&longitude={}&method=3&iso8601=true",
                                              date.format("%d-%m-%Y"), p.lat, p.lon)).ok().as_ref().and_then(parse_aladhan)
                        .inspect(|d| kv_put(app, &key, &serde_json::to_string(d).unwrap())).or(cached),
                };
                out.extend(day);
            }
            Ok(out)
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolatInfo { pub days: Vec<PrayerDay>, pub zone_name: Option<String>, pub place: Option<location::Place> }

fn zone_name(code: &str) -> Option<String> {
    let zones: Vec<(String, String, String)> = serde_json::from_str(ZONES).ok()?;
    zones.into_iter().find(|z| z.0 == code).map(|z| format!("{} ({}, {})", z.0, z.2, z.1))
}

#[tauri::command]
pub async fn solat_days(app: AppHandle, force: Option<bool>) -> Result<SolatInfo, String> {
    crate::widgets::blocking(move || {
        let days = days(&app, force.unwrap_or(false))?;
        if days.is_empty() { return Err("No prayer times yet: check your internet connection.".to_string()); }
        let zone_name = days.first().filter(|d| !d.zone.is_empty()).and_then(|d| zone_name(&d.zone));
        crate::scheduler::poke(); // new times may be due soon
        Ok(SolatInfo { days, zone_name, place: location::current(&app).ok() })
    }).await?
}

#[tauri::command]
pub fn solat_zones() -> Vec<(String, String, String)> { serde_json::from_str(ZONES).unwrap_or_default() }

/// Scheduler items from the cache only (no network here). Prayers that passed
/// more than 15 minutes ago are skipped rather than logged as missed.
pub fn due_items(app: &AppHandle) -> Vec<Due> {
    let cfg = app.state::<crate::AppState>().config.lock().unwrap().clone();
    let s = cfg.widget_settings("solat");
    if !cfg.uses("solat") || s["alerts"] == false { return vec![]; }
    let sound = solat_sound(&s);
    let db = app.state::<Db>();
    let c = db.lock();
    let mut cached: Vec<PrayerDay> = vec![];
    let mut q = c.prepare("SELECT value FROM kv WHERE key LIKE 'solat:week:%' OR key LIKE 'solat:aladhan:%'").unwrap();
    for v in q.query_map([], |r| r.get::<_, String>(0)).unwrap().flatten() {
        if let Ok(list) = serde_json::from_str::<Vec<PrayerDay>>(&v) { cached.extend(list); }
        else if let Ok(d) = serde_json::from_str::<PrayerDay>(&v) { cached.push(d); }
    }
    let Some(active) = kv_get(&c, "solat:active").and_then(|s| serde_json::from_str::<Active>(&s).ok()) else { return vec![] };
    drop(q);
    drop(c);
    let cached = active_days(cached, &active);
    let now = now_ms();
    cached.iter()
        .flat_map(|d| d.times.iter().map(move |(n, t)| (d, n, *t)))
        .filter(|(_, n, t)| PRAYERS.contains(&n.as_str()) && *t > now - crate::scheduler::MISSED_AFTER_MS && *t < now + 2 * 86_400_000)
        .map(|(d, n, t)| Due {
            key: format!("solat:{}:{}:{n}", d.date, d.zone),
            at: t,
            source: "solat",
            title: format!("Waktu {n}"),
            body: zone_name(&d.zone).unwrap_or_else(|| "Prayer time".into()),
            sound: sound.clone(),
        })
        .collect()
}

/// The dock alert for a prayer that has just begun: its name comes from the occurrence key
/// ("solat:<date>:<zone>:<name>"). None for a missed one (GlassBar was off or asleep) and
/// under Do Not Disturb, which also keeps banners and sounds quiet.
pub fn prayer_alert(key: &str, at: i64, missed: bool, dnd: bool) -> Option<Value> {
    if missed || dnd { return None; }
    let name = key.strip_prefix("solat:")?.rsplit(':').next().filter(|n| !n.is_empty())?;
    Some(serde_json::json!({ "kind": "prayer", "name": name, "at": at }))
}

/// The prayer-time sound from the widget settings: "short" (default) or "full" adzan,
/// or "chime" (None = the normal alert chime). `azanVoice` picks the built-in recording (the
/// dock knows them; unset = its default). The user's own file replaces it and follows short/full too.
pub fn solat_sound(s: &Value) -> Option<Value> {
    let mode = s["azanSound"].as_str().unwrap_or("short");
    if mode == "chime" { return None; }
    let short = mode != "full";
    // Pause other music while the adzan plays (on unless turned off).
    let pause = s["pauseMusic"].as_bool().unwrap_or(true);
    // Adzan volume, 0-100 % (full when unset).
    let volume = (s["azanVolume"].as_f64().unwrap_or(100.0) / 100.0).clamp(0.0, 1.0);
    Some(match s["azanFile"].as_str().filter(|f| !f.is_empty()) {
        Some(f) => serde_json::json!({ "kind": "azan", "file": f, "short": short, "pauseMusic": pause, "volume": volume }),
        None => serde_json::json!({ "kind": "adhan", "voice": s["azanVoice"].as_str().unwrap_or("madina"), "short": short, "pauseMusic": pause, "volume": volume }),
    })
}

/// Turn {"kind":"azan","file":path} into a playable data: URL (falls back to the chime).
pub fn sound_payload(v: &Value) -> Value {
    let Some(path) = v["file"].as_str() else { return v.clone() };
    let short = v["short"].as_bool().unwrap_or(false);
    let pause = v["pauseMusic"].as_bool().unwrap_or(false);
    let volume = v["volume"].as_f64().unwrap_or(1.0);
    // Check the size before reading: a mistakenly chosen huge file must not be loaded.
    let small = std::fs::metadata(path).map(|m| m.len() < 30 * 1024 * 1024).unwrap_or(false);
    match if small { std::fs::read(path) } else { Err(std::io::ErrorKind::InvalidData.into()) } {
        Ok(bytes) => {
            let mime = if path.to_lowercase().ends_with(".wav") { "audio/wav" } else { "audio/mpeg" };
            serde_json::json!({ "kind": "azan", "short": short, "pauseMusic": pause, "volume": volume, "src": format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)) })
        }
        _ => serde_json::json!({ "kind": "chime" }),
    }
}

#[tauri::command]
pub async fn solat_test_sound(app: AppHandle, settings: Value) {
    use tauri::Emitter;
    let p = crate::widgets::blocking(move || solat_sound(&settings).map(|v| sound_payload(&v)).unwrap_or(serde_json::json!({ "kind": "chime" }))).await;
    if let Ok(p) = p { let _ = app.emit_to("bar", "sound:play", p); }
}

/// Daily: refresh the cache so alerts work even if the widget popup is never opened,
/// and drop cached days for zones/dates no longer in use.
pub fn refresh(app: &AppHandle) {
    if !app.state::<crate::AppState>().config.lock().unwrap().uses("solat") { return; }
    let _ = days(app, false);
    let active = kv(app, "solat:active").and_then(|s| serde_json::from_str::<Active>(&s).ok());
    let keep = active.map(|a| format!("solat:week:{}", a.zone)).unwrap_or_default();
    let today = chrono::Local::now().date_naive().to_string();
    let _ = app.state::<Db>().lock().execute(
        "DELETE FROM kv WHERE (key LIKE 'solat:week:%' AND key <> ?1) OR (key LIKE 'solat:aladhan:%' AND substr(key, 15, 10) < ?2)",
        rusqlite::params![keep, today]);
}

#[cfg(test)]
mod tests {
    use super::*;
    const WEEK: &str = include_str!("../../tests/fixtures/jakim-week.json");
    const ALADHAN: &str = include_str!("../../tests/fixtures/aladhan.json");

    fn ms(s: &str) -> i64 { chrono::DateTime::parse_from_rfc3339(s).unwrap().timestamp_millis() }

    #[test]
    fn prayer_sound_defaults_to_short_adhan() {
        use serde_json::json;
        assert_eq!(solat_sound(&json!({})), Some(json!({ "kind": "adhan", "voice": "madina", "short": true, "pauseMusic": true, "volume": 1.0 })), "old configs get the short Madina adzan, pausing music, full volume");
        assert_eq!(solat_sound(&json!({ "azanVolume": 40 })).unwrap()["volume"], json!(0.4));
        assert_eq!(solat_sound(&json!({ "azanSound": "full", "azanVoice": "classic", "pauseMusic": false })), Some(json!({ "kind": "adhan", "voice": "classic", "short": false, "pauseMusic": false, "volume": 1.0 })));
        assert_eq!(solat_sound(&json!({ "azanSound": "chime", "azanFile": "C:/a.mp3" })), None, "normal sound ignores the file");
        assert_eq!(solat_sound(&json!({ "azanFile": "C:/a.mp3" })), Some(json!({ "kind": "azan", "file": "C:/a.mp3", "short": true, "pauseMusic": true, "volume": 1.0 })));
        assert_eq!(sound_payload(&json!({ "kind": "azan", "file": "C:/missing.mp3", "pauseMusic": true })), json!({ "kind": "chime" }), "unreadable file falls back to the chime, which pauses nothing");
    }
    fn at(day: &PrayerDay, name: &str) -> i64 { day.times.iter().find(|(n, _)| n == name).unwrap().1 }

    #[test]
    fn only_the_active_zone_alerts() {
        let mut days = parse_jakim(&serde_json::from_str(WEEK).unwrap(), "WLY01");
        let mut other = days[0].clone();
        other.zone = "JHR02".into();
        let mut aladhan = days[0].clone();
        aladhan.zone = String::new();
        aladhan.source = "aladhan".into();
        days.push(other);
        days.push(aladhan);
        let active = Active { source: "jakim".into(), zone: "WLY01".into() };
        let kept = active_days(days, &active);
        assert_eq!(kept.len(), 8);
        assert!(kept.iter().all(|d| d.zone == "WLY01" && d.source == "jakim"));
    }

    #[test]
    fn parse_malay_dates() {
        assert_eq!(parse_malay_date("23-Okt-2026"), chrono::NaiveDate::from_ymd_opt(2026, 10, 23));
        assert_eq!(parse_malay_date("05-Ogos-2026"), chrono::NaiveDate::from_ymd_opt(2026, 8, 5));
        assert_eq!(parse_malay_date("garbage"), None);
    }

    #[test]
    fn parse_jakim_reads_all_times() {
        let days = parse_jakim(&serde_json::from_str(WEEK).unwrap(), "WLY01");
        assert_eq!(days.len(), 8);
        let d = &days[0];
        assert_eq!((d.date.as_str(), d.zone.as_str(), d.source.as_str()), ("2026-10-23", "WLY01", "jakim"));
        assert_eq!(at(d, "Zohor"), ms("2026-10-23T13:00:00+08:00"));
        assert_eq!(at(d, "Subuh"), ms("2026-10-23T05:49:00+08:00"));
        assert_eq!(d.times.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
                   ["Imsak", "Subuh", "Syuruk", "Zohor", "Asar", "Maghrib", "Isyak"]);
        assert_eq!(d.hijri.as_deref(), Some("1448-05-12"));
    }

    #[test]
    fn prayer_alert_names_the_prayer_and_stays_quiet_when_it_should() {
        assert_eq!(prayer_alert("solat:2026-10-27:WLY01:Zohor", 5, false, false), Some(serde_json::json!({ "kind": "prayer", "name": "Zohor", "at": 5 })));
        assert_eq!(prayer_alert("solat:2026-10-27:WLY01:Zohor", 5, true, false), None, "missed while off");
        assert_eq!(prayer_alert("solat:2026-10-27:WLY01:Zohor", 5, false, true), None, "Do Not Disturb");
        assert_eq!(prayer_alert("todo:1:5", 5, false, false), None);
    }

    #[test]
    fn next_prayer_skips_syuruk() {
        let d = &parse_jakim(&serde_json::from_str(WEEK).unwrap(), "WLY01")[0];
        let (name, _) = next_prayer(d, ms("2026-10-23T06:00:00+08:00"), false).unwrap();
        assert_eq!(name, "Zohor");
        let (name, _) = next_prayer(d, ms("2026-10-23T05:00:00+08:00"), true).unwrap();
        assert_eq!(name, "Imsak");
        let (name, _) = next_prayer(d, ms("2026-10-23T05:00:00+08:00"), false).unwrap();
        assert_eq!(name, "Subuh");
    }

    #[test]
    fn after_isyak_is_none() {
        let d = &parse_jakim(&serde_json::from_str(WEEK).unwrap(), "WLY01")[0];
        assert!(next_prayer(d, ms("2026-10-23T21:00:00+08:00"), false).is_none());
    }

    #[test]
    fn parse_aladhan_reads_times() {
        let d = parse_aladhan(&serde_json::from_str(ALADHAN).unwrap()).unwrap();
        assert_eq!(d.source, "aladhan");
        assert_eq!(d.date, "2026-10-23");
        assert_eq!(at(&d, "Subuh"), ms("2026-10-23T05:46:00+01:00"));
        assert_eq!(at(&d, "Isyak"), ms("2026-10-23T19:36:00+01:00"));
    }
}
