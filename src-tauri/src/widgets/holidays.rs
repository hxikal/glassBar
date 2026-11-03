//! Malaysian public holidays from Google's public "Holidays in Malaysia" calendar (no key).
//! Each holiday lists the states it applies to (none = every state); observances such as
//! Valentine's Day are dropped. Cached for a week; when a fetch fails the cache is used.
use crate::db::{kv_get, kv_set, now_ms, Db};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const URL: &str = "https://calendar.google.com/calendar/ical/en.malaysia%23holiday%40group.v.calendar.google.com/public/basic.ics";
const TTL_MS: i64 = 7 * 24 * 3600_000;
const KEY: &str = "holidays:my";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Holiday {
    /// YYYY-MM-DD
    pub date: String,
    pub name: String,
    /// States as Google names them ("Selangor", "Kuala Lumpur"…); empty = national.
    pub states: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Holidays {
    pub items: Vec<Holiday>,
    pub fetched_at: i64,
    #[serde(default)]
    pub cached: bool,
}

pub fn parse_ics(text: &str) -> Vec<Holiday> {
    // Long lines are folded onto the next line, which then starts with a space.
    let text = text.replace("\r\n", "\n").replace("\n ", "");
    let mut out: Vec<Holiday> = text.split("BEGIN:VEVENT").skip(1).filter_map(|ev| {
        let field = |k: &str| ev.lines().find_map(|l| l.strip_prefix(k)).map(|v| v.trim().replace(r"\,", ","));
        let d = field("DTSTART;VALUE=DATE:")?;
        let desc = field("DESCRIPTION:")?;
        let desc = desc.split(r"\n").next()?.trim();
        let states = if desc == "Public holiday" { vec![] } else {
            desc.strip_prefix("Public holiday in ")?.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
        };
        if d.len() != 8 || !d.bytes().all(|b| b.is_ascii_digit()) { return None; }
        let name = field("SUMMARY:")?.replace(" (regional holiday)", "");
        Some(Holiday { date: format!("{}-{}-{}", &d[..4], &d[4..6], &d[6..]), name, states })
    }).collect();
    out.sort_by(|a, b| a.date.cmp(&b.date).then(a.name.cmp(&b.name)));
    out
}

fn get(app: &AppHandle, force: bool) -> Result<Holidays, String> {
    let cached: Option<Holidays> = kv_get(&app.state::<Db>().lock(), KEY).and_then(|s| serde_json::from_str(&s).ok());
    if let Some(c) = &cached { if !force && now_ms() - c.fetched_at < TTL_MS { return Ok(c.clone()); } }
    let fresh = crate::net::get_text(URL).map(|t| parse_ics(&t)).and_then(|items|
        if items.is_empty() { Err("the holiday calendar was empty".into()) } else { Ok(items) });
    match fresh {
        Ok(items) => {
            let h = Holidays { items, fetched_at: now_ms(), cached: false };
            kv_set(&app.state::<Db>().lock(), KEY, &serde_json::to_string(&h).unwrap());
            Ok(h)
        }
        Err(e) => cached.map(|c| Holidays { cached: true, ..c }).ok_or(e),
    }
}

#[tauri::command]
pub async fn holidays_get(app: AppHandle, force: bool) -> Result<Holidays, String> {
    crate::widgets::blocking(move || get(&app, force)).await?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_national_regional_and_skips_observances() {
        let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nDTSTART;VALUE=DATE:20260201\r\nDESCRIPTION:Public holiday in Kuala Lumpur\\, Labuan\\, Putr\r\n ajaya\r\nSUMMARY:Federal Territory Day (regional holiday)\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nDTSTART;VALUE=DATE:20260831\r\nDESCRIPTION:Public holiday\r\nSUMMARY:Malaysia's National Day\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nDTSTART;VALUE=DATE:20260214\r\nDESCRIPTION:Observance\\nTo hide observances\r\nSUMMARY:Valentine's Day\r\nEND:VEVENT\r\nEND:VCALENDAR";
        let h = parse_ics(ics);
        assert_eq!(h.len(), 2);
        assert_eq!(h[0], Holiday { date: "2026-02-01".into(), name: "Federal Territory Day".into(), states: vec!["Kuala Lumpur".into(), "Labuan".into(), "Putrajaya".into()] });
        assert_eq!(h[1].states, Vec::<String>::new());
    }
}
