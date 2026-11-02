//! AI Usage: Claude plan limits (5-hour / weekly) and daily token counts.
//! Limits come from Anthropic's usage endpoint with the login Claude Code saved in
//! ~/.claude/.credentials.json. When that token has expired we refresh it the same way the
//! `claude` CLI does and write the new tokens back, so the CLI stays signed in.
//! Daily counts are read from Claude Code's local transcripts (~/.claude/projects/**/*.jsonl).
//! ponytail: both endpoints are unofficial; if they change, limits show an error and the
//! daily graph keeps working.
use crate::db::{kv_get, kv_set, now_ms, Db};
use chrono::{DateTime, Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

const TTL_MS: i64 = 5 * 60_000;
const KEY: &str = "ai:limits";
const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const TOKEN_URL: &str = "https://platform.claude.com/v1/oauth/token";
const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const RELOGIN: &str = "Claude login expired. Run `claude` in a terminal and sign in again.";

/// One refresh at a time: a refresh token works once, so two racing refreshes would sign us out.
static TOKEN_LOCK: Mutex<()> = Mutex::new(());

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    /// five_hour, seven_day, or e.g. "seven_day:Fable" for a per-model weekly limit.
    pub key: String,
    pub name: String,
    pub used_pct: f64,
    pub resets_at: Option<i64>,
}

/// Share of this week's usage by surface (Claude Code, Chats, Cowork…).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Share { pub name: String, pub pct: f64 }

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    pub windows: Vec<Window>,
    #[serde(default)]
    pub breakdown: Vec<Share>,
    pub plan: String,
    pub fetched_at: i64,
    /// Old data because the latest fetch failed.
    #[serde(default)]
    pub cached: bool,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Day {
    pub date: String,
    pub input: u64,
    pub output: u64,
    pub cache_write: u64,
    pub cache_read: u64,
    /// Model -> all tokens that day.
    pub by_model: BTreeMap<String, u64>,
}

fn home() -> PathBuf {
    std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_default()
}

fn ts(x: &Value) -> Option<i64> { x.as_str().and_then(|s| DateTime::parse_from_rfc3339(s).ok()).map(|d| d.timestamp_millis()) }

/// Limit windows from the usage response. The newer `limits` list is preferred; the older
/// named fields fill in what it lacks. Null fields are limits the plan doesn't have right
/// now, and code-named fields (credits, promos) are skipped.
pub fn parse_usage(v: &Value) -> Vec<Window> {
    let mut w: Vec<Window> = vec![];
    let mut add = |key: String, name: String, used: Option<f64>, resets: &Value| {
        if let Some(used_pct) = used { if !w.iter().any(|x| x.key == key) { w.push(Window { key, name, used_pct, resets_at: ts(resets) }); } }
    };
    for l in v["limits"].as_array().into_iter().flatten() {
        let g = l["group"].as_str().unwrap_or("");
        let (base, name) = if g == "weekly" { ("seven_day", "Weekly") }
            else if g.contains("five") || g.contains("hour") || g == "session" { ("five_hour", "5-hour") }
            else { (g, g) };
        let (key, name) = match l["scope"]["model"]["display_name"].as_str() {
            Some(m) => (format!("{base}:{m}"), format!("{name} · {m}")),
            None => (base.to_string(), name.to_string()),
        };
        add(key, name, l["percent"].as_f64(), &l["resets_at"]);
    }
    for (k, name) in [("five_hour", "5-hour"), ("seven_day", "Weekly"), ("seven_day_opus", "Weekly · Opus"), ("seven_day_sonnet", "Weekly · Sonnet")] {
        add(k.to_string(), name.to_string(), v[k]["utilization"].as_f64(), &v[k]["resets_at"]);
    }
    // 5-hour first, then the overall weekly, then per-model ones.
    w.sort_by_key(|x| (x.key != "five_hour", x.key != "seven_day", x.key.clone()));
    w
}

pub fn parse_breakdown(v: &Value) -> Vec<Share> {
    v["seven_day_breakdown"]["rows"].as_array().into_iter().flatten()
        .filter_map(|r| Some(Share { name: r["display_name"].as_str()?.into(), pct: r["percent"].as_f64()? }))
        .filter(|s| s.pct > 0.0).collect()
}

fn read_creds(p: &Path) -> Result<Value, String> {
    let s = std::fs::read_to_string(p).map_err(|_| "No Claude Code login on this PC. Run `claude` in a terminal and sign in.".to_string())?;
    let v: Value = serde_json::from_str(&s).map_err(|_| "Claude login file is unreadable".to_string())?;
    if !v["claudeAiOauth"]["accessToken"].is_string() { return Err("No Claude login found. Run `claude` and sign in.".into()); }
    Ok(v)
}

/// Swap the refresh token for a new pair and write it back in place (other fields kept).
fn refresh(p: &Path, v: &mut Value) -> Result<(), String> {
    let o = &v["claudeAiOauth"];
    let rt = o["refreshToken"].as_str().ok_or(RELOGIN)?;
    let scope = o["scopes"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" ")).unwrap_or_default();
    let body = json!({ "grant_type": "refresh_token", "refresh_token": rt, "client_id": CLIENT_ID, "scope": scope });
    let r = crate::net::post_json(TOKEN_URL, &[], &body, 30)?;
    let at = r["access_token"].as_str().ok_or(RELOGIN)?.to_string();
    let now = now_ms();
    let o = &mut v["claudeAiOauth"];
    o["accessToken"] = json!(at);
    if let Some(n) = r["refresh_token"].as_str() { o["refreshToken"] = json!(n); }
    o["expiresAt"] = json!(now + r["expires_in"].as_i64().unwrap_or(3600) * 1000);
    if let Some(s) = r["refresh_token_expires_in"].as_i64() { o["refreshTokenExpiresAt"] = json!(now + s * 1000); }
    // Write to a temp file then rename, so a crash never leaves a half-written login.
    let tmp = p.with_extension("json.glassbar-tmp");
    std::fs::write(&tmp, serde_json::to_string(v).unwrap()).map_err(|e| format!("couldn't save login: {e}"))?;
    std::fs::rename(&tmp, p).map_err(|e| format!("couldn't save login: {e}"))
}

pub fn needs_refresh(expires_at: Option<i64>, now: i64) -> bool { expires_at.map_or(true, |e| e - 60_000 < now) }

fn fetch() -> Result<Limits, String> {
    let _g = TOKEN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let p = home().join(".claude").join(".credentials.json");
    let mut v = read_creds(&p)?;
    let mut refreshed = false;
    if needs_refresh(v["claudeAiOauth"]["expiresAt"].as_i64(), now_ms()) { refresh(&p, &mut v)?; refreshed = true; }
    loop {
        let tok = format!("Bearer {}", v["claudeAiOauth"]["accessToken"].as_str().unwrap_or(""));
        let (status, body) = crate::net::get_json_h(USAGE_URL, &[("Authorization", &tok), ("anthropic-beta", "oauth-2025-04-20")])?;
        match status {
            200 => return Ok(Limits {
                windows: parse_usage(&body),
                breakdown: parse_breakdown(&body),
                plan: v["claudeAiOauth"]["subscriptionType"].as_str().unwrap_or("").into(),
                fetched_at: now_ms(),
                cached: false,
            }),
            401 if !refreshed => { refresh(&p, &mut v)?; refreshed = true; }
            401 => return Err(RELOGIN.into()),
            429 => return Err("Anthropic is rate limiting usage checks; try again in a few minutes".into()),
            s => return Err(format!("usage check failed (HTTP {s})")),
        }
    }
}

fn get(app: &AppHandle, force: bool) -> Result<Limits, String> {
    let cached: Option<Limits> = kv_get(&app.state::<Db>().lock(), KEY).and_then(|s| serde_json::from_str(&s).ok());
    if let Some(l) = &cached { if !force && now_ms() - l.fetched_at < TTL_MS { return Ok(l.clone()); } }
    match fetch() {
        Ok(l) => { kv_set(&app.state::<Db>().lock(), KEY, &serde_json::to_string(&l).unwrap()); Ok(l) }
        // Keep showing the last numbers on a network blip, but not past their reset.
        Err(e) => cached.filter(|l| l.windows.iter().all(|w| w.resets_at.map_or(true, |r| r > now_ms())))
            .map(|l| Limits { cached: true, ..l }).ok_or(e),
    }
}

#[tauri::command]
pub async fn ai_limits(app: AppHandle, force: bool) -> Result<Limits, String> {
    crate::widgets::blocking(move || get(&app, force)).await?
}

#[derive(Default)]
pub struct Agg { seen: HashSet<String>, days: BTreeMap<NaiveDate, Day> }

/// Add one transcript line. The same API response is logged more than once (one line per
/// content block, and again in resumed sessions), so lines are deduped on message id + request id.
pub fn fold_line(line: &str, since: NaiveDate, agg: &mut Agg) {
    if !line.contains("\"usage\"") { return; }
    let Ok(v) = serde_json::from_str::<Value>(line) else { return };
    let m = &v["message"];
    let u = &m["usage"];
    let n = |k: &str| u[k].as_u64().unwrap_or(0);
    let (i, o, cw, cr) = (n("input_tokens"), n("output_tokens"), n("cache_creation_input_tokens"), n("cache_read_input_tokens"));
    if i + o + cw + cr == 0 { return; }
    let Some(date) = v["timestamp"].as_str().and_then(|s| DateTime::parse_from_rfc3339(s).ok()).map(|d| d.with_timezone(&Local).date_naive()) else { return };
    if date < since { return; }
    if let (Some(id), Some(rq)) = (m["id"].as_str(), v["requestId"].as_str()) {
        if !agg.seen.insert(format!("{id}:{rq}")) { return; }
    }
    let d = agg.days.entry(date).or_default();
    d.input += i; d.output += o; d.cache_write += cw; d.cache_read += cr;
    *d.by_model.entry(m["model"].as_str().unwrap_or("unknown").to_string()).or_default() += i + o + cw + cr;
}

fn walk(dir: &Path, since_ms: i64, out: &mut Vec<PathBuf>) {
    // Folders can vanish mid-walk (temp sessions); skip anything unreadable.
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let Ok(md) = e.metadata() else { continue };
        if md.is_dir() { walk(&p, since_ms, out); continue; }
        let fresh = md.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(true, |d| d.as_millis() as i64 >= since_ms);
        if fresh && p.extension().is_some_and(|x| x == "jsonl") { out.push(p); }
    }
}

/// Last `days` days (today included), oldest first, empty days as zeros.
pub fn daily(root: &Path, days: u32, today: NaiveDate) -> Vec<Day> {
    let days = days.clamp(1, 90) as i64;
    let since = today - Duration::days(days - 1);
    let since_ms = since.and_hms_opt(0, 0, 0).and_then(|t| t.and_local_timezone(Local).earliest()).map_or(0, |t| t.timestamp_millis());
    let mut files = vec![];
    walk(root, since_ms, &mut files);
    let mut agg = Agg::default();
    for f in files {
        let Ok(fh) = std::fs::File::open(&f) else { continue };
        for line in BufReader::new(fh).lines().map_while(Result::ok) { fold_line(&line, since, &mut agg); }
    }
    (0..days).map(|k| {
        let d = since + Duration::days(k);
        Day { date: d.format("%Y-%m-%d").to_string(), ..agg.days.remove(&d).unwrap_or_default() }
    }).collect()
}

#[tauri::command]
pub async fn ai_daily(days: u32) -> Result<Vec<Day>, String> {
    crate::widgets::blocking(move || daily(&home().join(".claude").join("projects"), days, Local::now().date_naive())).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_legacy_usage_fields() {
        let v = json!({
            "seven_day": {"utilization": 40.0, "resets_at": "2026-11-05T10:00:00+00:00"},
            "five_hour": {"utilization": 12.5, "resets_at": "2026-11-02T18:00:00.123+00:00"},
            "seven_day_opus": {"utilization": 3.0, "resets_at": null},
            "seven_day_oauth_apps": null,
            "iguana_necktie": {"utilization": 11.0, "resets_at": null},
            "extra_usage": {"utilization": 0.0}
        });
        let w = parse_usage(&v);
        assert_eq!(w.iter().map(|x| x.key.as_str()).collect::<Vec<_>>(), ["five_hour", "seven_day", "seven_day_opus"]);
        assert_eq!(w[0].used_pct, 12.5);
        assert_eq!(w[1].resets_at, Some(1793872800000));
        assert_eq!(w[2].resets_at, None);
    }

    #[test]
    fn prefers_limits_list_and_reads_breakdown() {
        // Trimmed from a real 2026-11-02 response (Max plan, no 5-hour window).
        let v = json!({
            "five_hour": null,
            "seven_day": {"utilization": 86.0, "resets_at": "2026-11-02T23:00:00.014405+00:00"},
            "limits": [
                {"kind": "weekly_all", "group": "weekly", "percent": 86, "resets_at": "2026-11-02T23:00:00.014405+00:00", "scope": null},
                {"kind": "weekly_scoped", "group": "weekly", "percent": 45, "resets_at": "2026-11-02T23:00:00.014577+00:00",
                 "scope": {"model": {"id": null, "display_name": "Fable"}, "surface": null}}
            ],
            "seven_day_breakdown": {"rows": [
                {"key": "claude_code", "display_name": "Claude Code", "percent": 72},
                {"key": "other", "display_name": "Other", "percent": 0}
            ]}
        });
        let w = parse_usage(&v);
        assert_eq!(w.iter().map(|x| (x.key.as_str(), x.name.as_str())).collect::<Vec<_>>(),
            [("seven_day", "Weekly"), ("seven_day:Fable", "Weekly · Fable")]);
        assert_eq!(w[1].used_pct, 45.0);
        assert_eq!(parse_breakdown(&v), [Share { name: "Claude Code".into(), pct: 72.0 }]);
    }

    #[test]
    fn refresh_when_missing_or_near_expiry() {
        assert!(needs_refresh(None, 1000));
        assert!(needs_refresh(Some(1_000_000), 1_000_000 - 30_000));
        assert!(!needs_refresh(Some(1_000_000), 1_000_000 - 120_000));
    }

    #[test]
    fn folds_and_dedupes_lines() {
        let line = |id: &str, ts: &str| json!({"timestamp": ts, "requestId": "req_1", "message": {"id": id, "model": "claude-opus-5-5",
            "usage": {"input_tokens": 2, "output_tokens": 10, "cache_creation_input_tokens": 100, "cache_read_input_tokens": 1000}}}).to_string();
        // Midday UTC lands on the same date in any timezone within ±11h.
        let ts = "2026-11-02T12:00:00Z";
        let day = DateTime::parse_from_rfc3339(ts).unwrap().with_timezone(&Local).date_naive();
        let mut a = Agg::default();
        fold_line(&line("m1", ts), day, &mut a);
        fold_line(&line("m1", ts), day, &mut a); // duplicate
        fold_line(&line("m2", ts), day, &mut a);
        fold_line(&line("m3", "2026-10-01T12:00:00Z"), day, &mut a); // before range
        fold_line("{\"type\":\"user\"}", day, &mut a);
        fold_line("not json \"usage\"", day, &mut a);
        let d = &a.days[&day];
        assert_eq!((d.input, d.output, d.cache_write, d.cache_read), (4, 20, 200, 2000));
        assert_eq!(d.by_model["claude-opus-5-5"], 2224);
        assert_eq!(a.days.len(), 1);
    }

    /// Real login + real logs: `cargo test live_ai -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn live_ai() {
        let l = fetch().unwrap();
        println!("plan={} windows={:?}", l.plan, l.windows);
        let d = daily(&home().join(".claude").join("projects"), 7, Local::now().date_naive());
        for x in &d { println!("{} in={} out={} cw={} cr={} {:?}", x.date, x.input, x.output, x.cache_write, x.cache_read, x.by_model); }
    }

    #[test]
    fn daily_fills_empty_days() {
        let dir = std::env::temp_dir().join(format!("glassbar-ai-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("proj")).unwrap();
        let today = Local::now().date_naive();
        let ts = today.and_hms_opt(12, 0, 0).unwrap().and_local_timezone(Local).unwrap().to_rfc3339();
        let l = json!({"timestamp": ts, "requestId": "r", "message": {"id": "m", "model": "x", "usage": {"output_tokens": 5}}});
        std::fs::write(dir.join("proj").join("s.jsonl"), format!("{l}\n")).unwrap();
        let d = daily(&dir, 3, today);
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(d.len(), 3);
        assert_eq!(d[2].output, 5);
        assert_eq!(d[0].output, 0);
        assert_eq!(d[2].date, today.format("%Y-%m-%d").to_string());
    }
}
