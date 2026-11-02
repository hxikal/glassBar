//! One shared HTTP client for all widget fetches (weather, prayer times, IPU, location).
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

fn client() -> &'static reqwest::blocking::Client {
    static C: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    C.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(concat!("GlassBar/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("http client")
    })
}

/// Bytes downloaded through this client since start (Settings › Performance).
static RX: AtomicU64 = AtomicU64::new(0);
pub fn bytes_received() -> u64 { RX.load(Ordering::Relaxed) }

/// Read a whole body, counting it toward `bytes_received`.
fn read_body(r: reqwest::blocking::Response) -> reqwest::Result<impl AsRef<[u8]>> {
    let b = r.bytes()?;
    RX.fetch_add(b.len() as u64, Ordering::Relaxed);
    Ok(b)
}

/// GET a URL and parse JSON. Blocking: call from the blocking pool or a worker thread.
pub fn get_json(url: &str) -> Result<serde_json::Value, String> {
    let r = client().get(url).send().map_err(|e| format!("network: {e}"))?;
    if !r.status().is_success() { return Err(format!("HTTP {}", r.status())); }
    let b = read_body(r).map_err(|e| format!("bad response: {e}"))?;
    serde_json::from_slice(b.as_ref()).map_err(|e| format!("bad response: {e}"))
}

/// GET a URL as text (e.g. an .ics calendar). Blocking, like `get_json`.
pub fn get_text(url: &str) -> Result<String, String> {
    let r = client().get(url).timeout(Duration::from_secs(30)).send().map_err(|e| format!("network: {e}"))?;
    if !r.status().is_success() { return Err(format!("HTTP {}", r.status())); }
    let b = read_body(r).map_err(|e| format!("bad response: {e}"))?;
    Ok(String::from_utf8_lossy(b.as_ref()).into_owned())
}

/// GET JSON with extra headers: (HTTP status, body). The body is Null when it isn't JSON.
pub fn get_json_h(url: &str, headers: &[(&str, &str)]) -> Result<(u16, serde_json::Value), String> {
    let mut rq = client().get(url).timeout(Duration::from_secs(30));
    for (k, v) in headers { rq = rq.header(*k, *v); }
    let r = rq.send().map_err(|e| format!("network: {}", e.without_url()))?;
    let status = r.status().as_u16();
    Ok((status, read_body(r).ok().and_then(|b| serde_json::from_slice(b.as_ref()).ok()).unwrap_or(serde_json::Value::Null)))
}

/// POST JSON with extra headers; returns the JSON body even for HTTP errors (APIs put the
/// reason there). `timeout_s` overrides the default 10 s for slow endpoints such as AI.
pub fn post_json(url: &str, headers: &[(&str, &str)], body: &serde_json::Value, timeout_s: u64) -> Result<serde_json::Value, String> {
    let mut rq = client().post(url).timeout(Duration::from_secs(timeout_s)).json(body);
    for (k, v) in headers { rq = rq.header(*k, *v); }
    let r = rq.send().map_err(|e| format!("network: {e}"))?;
    let status = r.status();
    read_body(r).ok().and_then(|b| serde_json::from_slice(b.as_ref()).ok()).ok_or(format!("HTTP {status}"))
}

/// Percent-encode a query value.
pub fn enc(s: &str) -> String {
    s.bytes().map(|b| match b {
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
        _ => format!("%{b:02X}"),
    }).collect()
}
