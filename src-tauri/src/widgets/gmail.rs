//! Gmail: unread messages in INBOX, read over IMAP (imap.gmail.com:993) with an App Password.
//!
//! Read-only by design: INBOX is opened with EXAMINE and messages are fetched with BODY.PEEK,
//! so nothing is ever marked read in Gmail. A message opened in GlassBar is only hidden here
//! (kv `gmail:read:<email>`); it stays unread in Gmail.
//!
//! Realtime: one IMAP IDLE connection per account (`watch`) waits for Gmail to say the inbox
//! changed, then checks it at once; new mail can raise a GlassBar notification.
//!
//! The App Password lives in Windows Credential Manager (`platform::secrets()`), never in
//! config.json or the database. Message summaries are cached in kv `gmail:inbox:<email>`.
use crate::db::{kv_get, kv_set, now_ms, Db};
use crate::platform::{secrets, shell, Res};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const HOST: &str = "imap.gmail.com";
const PORT: u16 = 993;
/// Newest unread messages listed per category; the counts still cover all of them.
const PRIMARY_LIST: usize = 40;
const OTHER_LIST: usize = 15;
/// Gmail's inbox categories besides Primary, as its search knows them (`category:social`).
pub const CATEGORIES: [&str; 4] = ["social", "promotions", "updates", "forums"];
/// Bytes of each message read for the list (headers and the start of the text).
const PREVIEW_BYTES: usize = 24 * 1024;
/// Bytes read when a message is opened. Attachments past this are never downloaded.
const OPEN_BYTES: usize = 1024 * 1024;
/// A whole IMAP response may not grow past this.
const MAX_RESPONSE: usize = 64 * 1024 * 1024;
/// Dock and panel ask together; one fetch serves both.
const TTL_MS: i64 = 60_000;
/// How long one IDLE waits before it is renewed (well under Gmail's and home routers' limits).
const IDLE_SECS: u64 = 5 * 60;
/// A read that waited past its timeout (not an error while idling).
const TIMED_OUT: &str = "timed out";
const AUTH_FAILED: &str = "Gmail didn't accept this App Password. Check the address, or make a new App Password.";

/// One Gmail account talks to the server at a time.
static BUSY: Mutex<()> = Mutex::new(());

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Mail {
    pub uid: u32,
    pub from_name: String,
    pub from_addr: String,
    pub subject: String,
    pub snippet: String,
    /// Unix ms.
    pub date: i64,
    #[serde(default)]
    pub message_id: String,
    /// Gmail marked it important.
    #[serde(default)]
    pub important: bool,
    /// primary, social, promotions, updates or forums.
    #[serde(default)]
    pub category: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Inbox {
    pub email: String,
    /// Unread in Gmail and not yet opened in GlassBar.
    pub unread: u32,
    /// Of `unread`: marked important by Gmail.
    #[serde(default)]
    pub important: u32,
    /// Of `unread`, per category (primary, social, promotions, updates, forums).
    #[serde(default)]
    pub categories: BTreeMap<String, u32>,
    /// Newest first: up to PRIMARY_LIST from Primary and OTHER_LIST from each other category.
    pub items: Vec<Mail>,
    pub fetched_at: i64,
    /// Old data because the latest check failed.
    #[serde(default)]
    pub cached: bool,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MailBody {
    pub uid: u32,
    pub from_name: String,
    pub from_addr: String,
    pub to: String,
    pub subject: String,
    pub date: i64,
    pub text: String,
    /// The message was longer than what was read.
    pub truncated: bool,
}

/// What is cached per account: the summaries already read, so a check only fetches new ones.
#[derive(Serialize, Deserialize, Default)]
struct Store {
    uidvalidity: u64,
    unseen: Vec<u32>,
    mails: BTreeMap<u32, Mail>,
    fetched_at: i64,
    /// Unread UIDs Gmail marks important.
    #[serde(default)]
    important: Vec<u32>,
    /// Unread UIDs per category other than Primary.
    #[serde(default)]
    cats: BTreeMap<String, Vec<u32>>,
    /// Highest unread UID seen: anything above it next time is new mail.
    #[serde(default)]
    max_uid: u32,
}

// ---------- pure helpers (tested) ----------

/// Google shows App Passwords as "abcd efgh ijkl mnop"; the spaces aren't part of it.
pub fn clean_password(p: &str) -> String { p.chars().filter(|c| !c.is_whitespace()).collect() }

pub fn valid_email(e: &str) -> bool {
    let e = e.trim();
    let Some((user, domain)) = e.split_once('@') else { return false };
    !user.is_empty() && domain.contains('.') && !e.chars().any(|c| c.is_whitespace() || c.is_control() || c == '"')
}

fn norm_email(e: &str) -> Res<String> {
    let e = e.trim().to_lowercase();
    if valid_email(&e) { Ok(e) } else { Err("Enter a full email address".into()) }
}

/// An IMAP quoted string.
pub fn quote(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() { if c == '"' || c == '\\' { o.push('\\'); } o.push(c); }
    o.push('"');
    o
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tok { Atom(String), Str(Vec<u8>), List(Vec<Tok>), Nil }

impl Tok {
    fn atom(&self) -> Option<&str> { if let Tok::Atom(a) = self { Some(a) } else { None } }
    fn num(&self) -> Option<u64> { self.atom()?.parse().ok() }
    fn bytes(&self) -> Option<&[u8]> {
        match self { Tok::Str(b) => Some(b), Tok::Atom(a) => Some(a.as_bytes()), _ => None }
    }
}

/// Split untagged response data into lines of tokens. Literals (`{n}\r\n` + n bytes) and
/// parenthesized lists may span lines.
pub fn parse_lines(b: &[u8]) -> Vec<Vec<Tok>> {
    let mut lines = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let mut line = Vec::new();
        loop {
            match parse_tok(b, &mut i) {
                Some(t) => line.push(t),
                None => break,
            }
        }
        // Past the line end.
        while i < b.len() && (b[i] == b'\r' || b[i] == b'\n') { i += 1; }
        if !line.is_empty() { lines.push(line); }
    }
    lines
}

fn parse_tok(b: &[u8], i: &mut usize) -> Option<Tok> {
    while *i < b.len() && b[*i] == b' ' { *i += 1; }
    if *i >= b.len() || b[*i] == b'\r' || b[*i] == b'\n' || b[*i] == b')' { return None; }
    match b[*i] {
        b'(' => {
            *i += 1;
            let mut v = Vec::new();
            loop {
                // Lists may continue past a line break only inside a literal, which parse_tok
                // consumes whole; a bare CRLF inside a list means the data is cut off.
                while *i < b.len() && b[*i] == b' ' { *i += 1; }
                if *i >= b.len() || b[*i] == b'\r' || b[*i] == b'\n' { break; }
                if b[*i] == b')' { *i += 1; break; }
                match parse_tok(b, i) { Some(t) => v.push(t), None => break }
            }
            Some(Tok::List(v))
        }
        b'"' => {
            *i += 1;
            let mut s = Vec::new();
            while *i < b.len() && b[*i] != b'"' {
                if b[*i] == b'\\' && *i + 1 < b.len() { *i += 1; }
                s.push(b[*i]);
                *i += 1;
            }
            *i += 1;
            Some(Tok::Str(s))
        }
        b'{' => {
            let end = b[*i..].iter().position(|&c| c == b'}')? + *i;
            let n: usize = std::str::from_utf8(&b[*i + 1..end]).ok()?.trim_end_matches('+').parse().ok()?;
            *i = end + 1;
            if b.get(*i) == Some(&b'\r') { *i += 1; }
            if b.get(*i) == Some(&b'\n') { *i += 1; }
            let stop = (*i + n).min(b.len());
            let s = b[*i..stop].to_vec();
            *i = stop;
            Some(Tok::Str(s))
        }
        _ => {
            // An atom; `BODY[...]<0>` keeps its brackets (they may hold spaces and parens).
            let start = *i;
            let mut depth = 0;
            while *i < b.len() {
                let c = b[*i];
                if c == b'[' { depth += 1; } else if c == b']' { depth -= 1; }
                else if depth <= 0 && matches!(c, b' ' | b'(' | b')' | b'\r' | b'\n' | b'"' | b'{') { break; }
                *i += 1;
            }
            let a = String::from_utf8_lossy(&b[start..*i]).into_owned();
            Some(if a.eq_ignore_ascii_case("NIL") { Tok::Nil } else { Tok::Atom(a) })
        }
    }
}

/// UIDs from `* SEARCH 1 2 3` lines.
pub fn search_uids(lines: &[Vec<Tok>]) -> Vec<u32> {
    lines.iter()
        .filter(|l| l.len() >= 2 && l[0].atom() == Some("*") && l[1].atom().is_some_and(|a| a.eq_ignore_ascii_case("SEARCH")))
        .flat_map(|l| l[2..].iter().filter_map(|t| t.num().map(|n| n as u32)))
        .collect()
}

/// (UID, message bytes) from `* n FETCH (UID u BODY[]<0> {..}..)` lines.
pub fn fetched(lines: &[Vec<Tok>]) -> Vec<(u32, Vec<u8>)> {
    let mut out = Vec::new();
    for l in lines {
        if l.len() < 4 || l[0].atom() != Some("*") || !l[2].atom().is_some_and(|a| a.eq_ignore_ascii_case("FETCH")) { continue; }
        let Tok::List(items) = &l[3] else { continue };
        let (mut uid, mut body) = (None, None);
        for kv in items.chunks(2) {
            let [k, v] = kv else { continue };
            let Some(k) = k.atom() else { continue };
            let k = k.to_ascii_uppercase();
            if k == "UID" { uid = v.num().map(|n| n as u32); }
            else if k.starts_with("BODY[") { body = v.bytes().map(<[u8]>::to_vec); }
        }
        if let (Some(u), Some(b)) = (uid, body) { out.push((u, b)); }
    }
    out
}

/// `[UIDVALIDITY n]` from an EXAMINE response.
pub fn uidvalidity(raw: &[u8]) -> u64 {
    let s = String::from_utf8_lossy(raw);
    s.split("UIDVALIDITY ").nth(1).and_then(|r| r.split(|c: char| !c.is_ascii_digit()).next()).and_then(|n| n.parse().ok()).unwrap_or(0)
}

fn squash(s: &str) -> String { s.split_whitespace().collect::<Vec<_>>().join(" ") }

pub fn snippet(text: &str, max: usize) -> String {
    let s = squash(text);
    if s.chars().count() <= max { return s; }
    let mut o: String = s.chars().take(max).collect();
    o.push('…');
    o
}

/// The text of a message, links and layout dropped: HTML-only mail is turned into text.
fn text_of(m: &mail_parser::Message) -> String {
    m.body_text(0).map(|t| t.into_owned()).unwrap_or_default()
}

fn sender(m: &mail_parser::Message) -> (String, String) {
    let a = m.from().and_then(|a| a.first());
    let addr = a.and_then(|a| a.address()).unwrap_or("").to_string();
    let name = a.and_then(|a| a.name()).map(str::to_string).filter(|n| !n.trim().is_empty()).unwrap_or_else(|| addr.clone());
    (name, addr)
}

fn date_of(m: &mail_parser::Message) -> i64 { m.date().map(|d| d.to_timestamp() * 1000).unwrap_or(0) }

pub fn summary(uid: u32, raw: &[u8]) -> Option<Mail> {
    let m = mail_parser::MessageParser::default().parse(raw)?;
    let (from_name, from_addr) = sender(&m);
    Some(Mail {
        uid,
        from_name,
        from_addr,
        subject: m.subject().map(squash).filter(|s| !s.is_empty()).unwrap_or_else(|| "(no subject)".into()),
        snippet: snippet(&text_of(&m), 160),
        date: date_of(&m),
        message_id: m.message_id().unwrap_or("").to_string(),
        // Set per check by inbox_of (Gmail can change both).
        important: false,
        category: String::new(),
    })
}

pub fn body(uid: u32, raw: &[u8], truncated: bool) -> Option<MailBody> {
    let m = mail_parser::MessageParser::default().parse(raw)?;
    let (from_name, from_addr) = sender(&m);
    let to = m.to().map(|a| a.iter().filter_map(|x| x.address()).collect::<Vec<_>>().join(", ")).unwrap_or_default();
    // Long runs of blank lines (common in HTML mail turned into text) become one.
    let mut text = String::new();
    let mut blanks = 0;
    for line in text_of(&m).lines() {
        let line = line.trim_end();
        if line.trim().is_empty() { blanks += 1; if blanks > 1 { continue; } } else { blanks = 0; }
        text.push_str(line);
        text.push('\n');
    }
    Some(MailBody {
        uid, from_name, from_addr, to,
        subject: m.subject().map(squash).filter(|s| !s.is_empty()).unwrap_or_else(|| "(no subject)".into()),
        date: date_of(&m),
        text: text.trim().to_string(),
        truncated,
    })
}

/// Each UID's category; a UID in none of them is Primary.
pub fn cat_map(cats: &BTreeMap<String, Vec<u32>>) -> HashMap<u32, &'static str> {
    let mut m = HashMap::new();
    for c in CATEGORIES { for u in cats.get(c).into_iter().flatten() { m.insert(*u, c); } }
    m
}

fn cat_of(m: &HashMap<u32, &'static str>, uid: u32) -> &'static str { m.get(&uid).copied().unwrap_or("primary") }

/// What to list: unread UIDs not opened in GlassBar, newest first, at most PRIMARY_LIST
/// from Primary and OTHER_LIST from each other category (so promotions can't crowd it out).
pub fn wanted(unseen: &[u32], read_here: &HashSet<u32>, cats: &HashMap<u32, &'static str>) -> Vec<u32> {
    let mut v: Vec<u32> = unseen.iter().copied().filter(|u| !read_here.contains(u)).collect();
    v.sort_unstable_by(|a, b| b.cmp(a));
    let mut taken: HashMap<&str, usize> = HashMap::new();
    v.retain(|u| {
        let c = cat_of(cats, *u);
        let n = taken.entry(c).or_default();
        *n += 1;
        *n <= if c == "primary" { PRIMARY_LIST } else { OTHER_LIST }
    });
    v
}

/// Gmail in the browser, at this message (by its Message-ID), in the right account.
pub fn web_link(email: &str, message_id: &str) -> String {
    let user = crate::net::enc(email);
    if message_id.is_empty() { return format!("https://mail.google.com/mail/u/?authuser={user}#inbox"); }
    format!("https://mail.google.com/mail/u/?authuser={user}#search/rfc822msgid%3A{}", crate::net::enc(message_id))
}

/// Which new mail raises a notification: "all", "smart" (Primary or important; the default)
/// or "off".
pub fn worth_notifying(mode: &str, m: &Mail) -> bool {
    match mode {
        "off" => false,
        "all" => true,
        _ => m.important || m.category.is_empty() || m.category == "primary",
    }
}

/// Unread UIDs that arrived since the last check (none on the first check: that is a baseline).
pub fn arrived(prev_max: u32, had_baseline: bool, unseen: &[u32]) -> Vec<u32> {
    if !had_baseline { return Vec::new(); }
    let mut v: Vec<u32> = unseen.iter().copied().filter(|u| *u > prev_max).collect();
    v.sort_unstable_by(|a, b| b.cmp(a));
    v
}

/// Notification title and body: one per email for up to 3, else one summary.
pub fn notes_for(mails: &[Mail]) -> Vec<(String, String)> {
    if mails.len() <= 3 {
        return mails.iter().map(|m| (m.from_name.clone(), m.subject.clone())).collect();
    }
    let mut who: Vec<&str> = Vec::new();
    for m in mails { if !who.contains(&m.from_name.as_str()) { who.push(&m.from_name); } }
    let more = who.len().saturating_sub(3);
    let mut body = who.iter().take(3).copied().collect::<Vec<_>>().join(", ");
    if more > 0 { body.push_str(&format!(" and {more} more")); }
    vec![(format!("{} new emails", mails.len()), body)]
}

/// Does this widget's settings hold `email` (the account list, or the older single address)?
pub fn has_account(settings: &serde_json::Value, email: &str) -> bool {
    let eq = |v: &serde_json::Value| v.as_str().is_some_and(|e| e.trim().eq_ignore_ascii_case(email));
    settings["accounts"].as_array().is_some_and(|a| a.iter().any(|x| eq(&x["email"]))) || eq(&settings["email"])
}

// ---------- IMAP connection ----------

type Tls = rustls::StreamOwned<rustls::ClientConnection, TcpStream>;

struct Imap { r: BufReader<Tls>, n: u32 }

impl Imap {
    fn connect() -> Res<Imap> {
        let addr = (HOST, PORT).to_socket_addrs().map_err(|_| "No internet connection (couldn't find imap.gmail.com)".to_string())?
            .next().ok_or("couldn't find imap.gmail.com")?;
        let tcp = TcpStream::connect_timeout(&addr, Duration::from_secs(15)).map_err(|e| format!("couldn't reach Gmail: {e}"))?;
        let _ = tcp.set_read_timeout(Some(Duration::from_secs(30)));
        let _ = tcp.set_write_timeout(Some(Duration::from_secs(30)));
        let roots = rustls::RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() };
        let cfg = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions().map_err(|e| e.to_string())?
            .with_root_certificates(roots)
            .with_no_client_auth();
        let name = rustls::pki_types::ServerName::try_from(HOST).map_err(|e| e.to_string())?;
        let conn = rustls::ClientConnection::new(Arc::new(cfg), name).map_err(|e| e.to_string())?;
        let mut s = Imap { r: BufReader::new(rustls::StreamOwned::new(conn, tcp)), n: 0 };
        let greeting = s.line()?;
        if !greeting.starts_with(b"* OK") { return Err("Gmail refused the connection".into()); }
        Ok(s)
    }

    /// One logical line: a physical line plus any literals it announces.
    fn line(&mut self) -> Res<Vec<u8>> {
        let mut out = Vec::new();
        loop {
            let start = out.len();
            let n = self.r.read_until(b'\n', &mut out).map_err(|e| match e.kind() {
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => TIMED_OUT.to_string(),
                _ => format!("Gmail connection lost: {e}"),
            })?;
            if n == 0 { return Err("Gmail closed the connection".into()); }
            if out.len() > MAX_RESPONSE { return Err("Gmail sent too much data".into()); }
            let l = &out[start..];
            let lit = l.strip_suffix(b"}\r\n").and_then(|h| {
                let open = h.iter().rposition(|&c| c == b'{')?;
                std::str::from_utf8(&h[open + 1..]).ok()?.trim_end_matches('+').parse::<usize>().ok()
            });
            let Some(len) = lit else { return Ok(out) };
            if out.len() + len > MAX_RESPONSE { return Err("Gmail sent too much data".into()); }
            let at = out.len();
            out.resize(at + len, 0);
            self.r.read_exact(&mut out[at..]).map_err(|e| format!("Gmail connection lost: {e}"))?;
        }
    }

    /// Send a command; returns its untagged data. A NO/BAD answer is an error.
    fn cmd(&mut self, c: &str) -> Res<Vec<u8>> {
        self.n += 1;
        let tag = format!("g{}", self.n);
        let s = self.r.get_mut();
        s.write_all(format!("{tag} {c}\r\n").as_bytes()).and_then(|_| s.flush()).map_err(|e| format!("Gmail connection lost: {e}"))?;
        let mut data = Vec::new();
        loop {
            let l = self.line()?;
            if l.starts_with(tag.as_bytes()) && l.get(tag.len()) == Some(&b' ') {
                let rest = String::from_utf8_lossy(&l[tag.len() + 1..]).trim().to_string();
                if rest.starts_with("OK") { return Ok(data); }
                return Err(rest);
            }
            data.extend_from_slice(&l);
        }
    }

    fn login(email: &str, password: &str) -> Res<Imap> {
        let mut s = Imap::connect()?;
        s.cmd(&format!("LOGIN {} {}", quote(email), quote(password))).map_err(|e| {
            if e.contains("AUTHENTICATIONFAILED") || e.contains("Invalid credentials") { AUTH_FAILED.to_string() }
            else if e.contains("Web login required") || e.contains("WEBALERT") { "Google wants you to sign in on the web first (security check). Open Gmail in the browser, then try again.".into() }
            else { format!("Gmail sign-in failed: {e}") }
        })?;
        Ok(s)
    }

    /// INBOX, read-only: nothing done here can mark a message read.
    fn examine(&mut self) -> Res<u64> { self.cmd("EXAMINE INBOX").map(|d| uidvalidity(&d)) }

    fn fetch(&mut self, uids: &[u32], bytes: usize) -> Res<Vec<(u32, Vec<u8>)>> {
        if uids.is_empty() { return Ok(Vec::new()); }
        let set = uids.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
        let d = self.cmd(&format!("UID FETCH {set} (UID BODY.PEEK[]<0.{bytes}>)"))?;
        Ok(fetched(&parse_lines(&d)))
    }

    fn logout(mut self) { let _ = self.cmd("LOGOUT"); }

    fn set_timeout(&mut self, secs: u64) { let _ = self.r.get_ref().sock.set_read_timeout(Some(Duration::from_secs(secs))); }

    fn send(&mut self, line: &str) -> Res<()> {
        let s = self.r.get_mut();
        s.write_all(format!("{line}\r\n").as_bytes()).and_then(|_| s.flush()).map_err(|e| format!("Gmail connection lost: {e}"))
    }

    /// One IDLE: true when Gmail says the inbox changed (new, removed or read mail), false
    /// when IDLE_SECS passed quietly.
    fn idle(&mut self) -> Res<bool> {
        self.n += 1;
        let tag = format!("g{}", self.n);
        self.send(&format!("{tag} IDLE"))?;
        if !self.line()?.starts_with(b"+") { return Err("Gmail didn't start IDLE".into()); }
        self.set_timeout(IDLE_SECS);
        let changed = loop {
            match self.line() {
                Ok(l) => {
                    let t = String::from_utf8_lossy(&l).to_ascii_uppercase();
                    if t.contains(" EXISTS") || t.contains(" EXPUNGE") || t.contains(" FETCH") { break true; }
                    if !t.starts_with("* ") { return Err(format!("unexpected while idling: {}", t.trim())); }
                }
                Err(e) if e == TIMED_OUT => break false,
                Err(e) => return Err(e),
            }
        };
        self.set_timeout(30);
        self.send("DONE")?;
        loop {
            let l = self.line()?;
            if l.starts_with(tag.as_bytes()) { return Ok(changed); }
        }
    }
}

// ---------- accounts, cache, commands ----------

/// The notification setting of the Gmail widget that shows `email`; None when no Gmail widget
/// on any dock has it (then nothing is watched or announced).
fn notify_mode(app: &AppHandle, email: &str) -> Option<String> {
    let st = app.state::<crate::AppState>();
    let c = st.config.lock().unwrap_or_else(|e| e.into_inner());
    c.profiles.values().flat_map(|p| p.widgets.iter())
        .filter(|w| w.id == "gmail" && w.enabled)
        .find(|w| has_account(&w.settings, email))
        .map(|w| w.settings["notify"].as_str().unwrap_or("smart").to_string())
}

/// Accounts with a watcher thread running.
static WATCHING: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Keep one IDLE connection per account until it is signed out or no widget shows it.
fn watch(app: AppHandle, email: String) {
    let mut wait = 15;
    loop {
        if notify_mode(&app, &email).is_none() || secrets().get(&secret_key(&email)).is_none() { break; }
        match idle_session(&app, &email) {
            Err(e) if e == AUTH_FAILED => break,
            Err(e) => {
                crate::applog::log(format!("gmail: watch {email}: {e}; retry in {wait}s"));
                std::thread::sleep(Duration::from_secs(wait));
                wait = (wait * 2).min(600);
            }
            Ok(()) => wait = 15,
        }
    }
    WATCHING.lock().unwrap_or_else(|e| e.into_inner()).retain(|e| e != &email);
}

/// One connection's life: IDLE, check on every change, until the connection drops (Err) or
/// the account is no longer shown (Ok).
fn idle_session(app: &AppHandle, email: &str) -> Res<()> {
    let pw = password(email)?;
    let mut s = Imap::login(email, &pw)?;
    s.examine()?;
    // Catch up on anything that came while not connected.
    let _ = get(app, email, true);
    loop {
        if notify_mode(app, email).is_none() { s.logout(); return Ok(()); }
        if s.idle()? { let _ = get(app, email, true); }
    }
}

fn announce(app: &AppHandle, email: &str, mode: &str, mails: Vec<Mail>) {
    let pick: Vec<Mail> = mails.into_iter().filter(|m| worth_notifying(mode, m)).collect();
    for (title, body) in notes_for(&pick) {
        crate::alerts::notify_full(app, "gmail", &title, &body, false, serde_json::json!({ "kind": "chime" }), serde_json::json!({ "email": email }));
    }
}

fn secret_key(email: &str) -> String { format!("GlassBar/gmail/{email}") }
fn inbox_key(email: &str) -> String { format!("gmail:inbox:{email}") }
fn read_key(email: &str) -> String { format!("gmail:read:{email}") }

fn password(email: &str) -> Res<String> {
    secrets().get(&secret_key(email)).ok_or_else(|| "Not signed in. Add your Gmail address and App Password in this widget's settings.".into())
}

fn load<T: for<'a> Deserialize<'a> + Default>(app: &AppHandle, key: &str) -> T {
    let db = app.state::<Db>();
    let c = db.lock();
    kv_get(&c, key).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn save<T: Serialize>(app: &AppHandle, key: &str, v: &T) {
    if let Ok(s) = serde_json::to_string(v) { kv_set(&app.state::<Db>().lock(), key, &s); }
}

fn inbox_of(email: &str, st: &Store, read_here: &HashSet<u32>) -> Inbox {
    let cats = cat_map(&st.cats);
    let important: HashSet<u32> = st.important.iter().copied().collect();
    let want = wanted(&st.unseen, read_here, &cats);
    let visible: Vec<u32> = st.unseen.iter().copied().filter(|u| !read_here.contains(u)).collect();
    let mut categories: BTreeMap<String, u32> = BTreeMap::new();
    for u in &visible { *categories.entry(cat_of(&cats, *u).to_string()).or_default() += 1; }
    Inbox {
        email: email.into(),
        unread: visible.len() as u32,
        important: visible.iter().filter(|u| important.contains(u)).count() as u32,
        categories,
        items: want.iter().filter_map(|u| st.mails.get(u).map(|m| Mail {
            important: important.contains(u),
            category: cat_of(&cats, *u).to_string(),
            ..m.clone()
        })).collect(),
        fetched_at: st.fetched_at,
        cached: false,
        error: None,
    }
}

fn check(email: &str, st: &mut Store, read_here: &HashSet<u32>) -> Res<()> {
    let pw = password(email)?;
    let mut s = Imap::login(email, &pw)?;
    let v = s.examine()?;
    if v != st.uidvalidity { *st = Store { uidvalidity: v, ..Store::default() }; }
    let unseen = search_uids(&parse_lines(&s.cmd("UID SEARCH UNSEEN")?));
    // Gmail's own search (X-GM-RAW) knows its categories and what it marks important.
    let important = search_uids(&parse_lines(&s.cmd(r#"UID SEARCH UNSEEN X-GM-RAW "is:important""#)?));
    let mut cats = BTreeMap::new();
    for c in CATEGORIES {
        let found = search_uids(&parse_lines(&s.cmd(&format!(r#"UID SEARCH UNSEEN X-GM-RAW "category:{c}""#))?));
        cats.insert(c.to_string(), found);
    }
    let want = wanted(&unseen, read_here, &cat_map(&cats));
    let new: Vec<u32> = want.iter().copied().filter(|u| !st.mails.contains_key(u)).collect();
    for chunk in new.chunks(10) {
        for (uid, raw) in s.fetch(chunk, PREVIEW_BYTES)? {
            if let Some(m) = summary(uid, &raw) { st.mails.insert(uid, m); }
        }
    }
    s.logout();
    // Keep only what is still listed.
    let keep: HashSet<u32> = want.into_iter().collect();
    st.mails.retain(|u, _| keep.contains(u));
    st.unseen = unseen;
    st.important = important;
    st.cats = cats;
    st.fetched_at = now_ms();
    Ok(())
}

fn get(app: &AppHandle, email: &str, force: bool) -> Res<Inbox> {
    let _g = BUSY.lock().unwrap_or_else(|e| e.into_inner());
    let mut st: Store = load(app, &inbox_key(email));
    let mut read_here: HashSet<u32> = load::<Vec<u32>>(app, &read_key(email)).into_iter().collect();
    let force = force || crate::widgets::first_since_start(&inbox_key(email));
    if !force && st.fetched_at > 0 && now_ms() - st.fetched_at < TTL_MS { return Ok(inbox_of(email, &st, &read_here)); }
    let (prev_max, prev_validity, had_baseline) = (st.max_uid, st.uidvalidity, st.fetched_at > 0);
    match check(email, &mut st, &read_here) {
        Ok(()) => {
            let new = arrived(prev_max, had_baseline && prev_validity == st.uidvalidity, &st.unseen);
            st.max_uid = st.unseen.iter().copied().max().unwrap_or(0).max(if prev_validity == st.uidvalidity { prev_max } else { 0 });
            // Forget "opened here" for messages no longer unread in Gmail.
            let unseen: HashSet<u32> = st.unseen.iter().copied().collect();
            read_here.retain(|u| unseen.contains(u));
            save(app, &inbox_key(email), &st);
            save(app, &read_key(email), &read_here.iter().collect::<Vec<_>>());
            let ib = inbox_of(email, &st, &read_here);
            let _ = app.emit("gmail:changed", &ib);
            if !new.is_empty() {
                if let Some(mode) = notify_mode(app, email) {
                    let mails: Vec<Mail> = new.iter().filter_map(|u| ib.items.iter().find(|m| m.uid == *u).cloned()).collect();
                    announce(app, email, &mode, mails);
                }
            }
            Ok(ib)
        }
        Err(e) if st.fetched_at > 0 && e != AUTH_FAILED && !e.starts_with("Not signed in") => {
            Ok(Inbox { cached: true, error: Some(e), ..inbox_of(email, &st, &read_here) })
        }
        Err(e) => Err(e),
    }
}

fn open(app: &AppHandle, email: &str, uid: u32) -> Res<MailBody> {
    let raw = {
        let _g = BUSY.lock().unwrap_or_else(|e| e.into_inner());
        let pw = password(email)?;
        let mut s = Imap::login(email, &pw)?;
        s.examine()?;
        let r = s.fetch(&[uid], OPEN_BYTES)?;
        s.logout();
        r.into_iter().find(|(u, _)| *u == uid).map(|(_, b)| b).ok_or("This email is no longer in your inbox")?
    };
    let b = body(uid, &raw, raw.len() >= OPEN_BYTES).ok_or("Couldn't read this email")?;
    // Opened here: hide it from the list (it stays unread in Gmail).
    let mut read_here: Vec<u32> = load(app, &read_key(email));
    if !read_here.contains(&uid) { read_here.push(uid); }
    save(app, &read_key(email), &read_here);
    let st: Store = load(app, &inbox_key(email));
    let _ = app.emit("gmail:changed", inbox_of(email, &st, &read_here.into_iter().collect()));
    Ok(b)
}

#[tauri::command]
pub async fn gmail_signed_in(email: String) -> bool {
    norm_email(&email).map(|e| secrets().get(&secret_key(&e)).is_some()).unwrap_or(false)
}

/// Check the App Password with Gmail, then keep it in Windows Credential Manager.
#[tauri::command]
pub async fn gmail_sign_in(app: AppHandle, email: String, password: String) -> Res<()> {
    let email = norm_email(&email)?;
    let pw = clean_password(&password);
    if pw.is_empty() { return Err("Enter the App Password".into()); }
    crate::widgets::blocking(move || -> Res<()> {
        Imap::login(&email, &pw)?.logout();
        secrets().set(&secret_key(&email), &email, &pw)?;
        save(&app, &inbox_key(&email), &Store::default());
        // Every dock and panel showing this account checks again now.
        let _ = app.emit("gmail:changed", Inbox { email, ..Inbox::default() });
        Ok(())
    }).await?
}

#[tauri::command]
pub async fn gmail_sign_out(app: AppHandle, email: String) -> Res<()> {
    let email = norm_email(&email)?;
    secrets().delete(&secret_key(&email))?;
    save(&app, &inbox_key(&email), &Store::default());
    save(&app, &read_key(&email), &Vec::<u32>::new());
    let _ = app.emit("gmail:changed", Inbox { email, ..Inbox::default() });
    Ok(())
}

#[tauri::command]
pub async fn gmail_inbox(app: AppHandle, email: String, force: bool) -> Res<Inbox> {
    let email = norm_email(&email)?;
    crate::widgets::blocking(move || get(&app, &email, force)).await?
}

/// Read one message (text only). It is then hidden from the list, not marked read in Gmail.
#[tauri::command]
pub async fn gmail_open(app: AppHandle, email: String, uid: u32) -> Res<MailBody> {
    let email = norm_email(&email)?;
    crate::widgets::blocking(move || open(&app, &email, uid)).await?
}

/// Start realtime watching for these accounts (once each; repeat calls are free).
#[tauri::command]
pub fn gmail_watch(app: AppHandle, emails: Vec<String>) {
    for e in emails {
        let Ok(e) = norm_email(&e) else { continue };
        let mut w = WATCHING.lock().unwrap_or_else(|e| e.into_inner());
        if w.contains(&e) { continue; }
        w.push(e.clone());
        let app = app.clone();
        std::thread::spawn(move || watch(app, e));
    }
}

/// The message in Gmail on the web.
#[tauri::command]
pub async fn gmail_open_web(app: AppHandle, email: String, uid: u32) -> Res<()> {
    let email = norm_email(&email)?;
    let st: Store = load(&app, &inbox_key(&email));
    let id = st.mails.get(&uid).map(|m| m.message_id.clone()).unwrap_or_default();
    shell().launch(&web_link(&email, &id), "")
}

/// Google's App Passwords page, to make one.
#[tauri::command]
pub async fn gmail_app_passwords() -> Res<()> { shell().launch("https://myaccount.google.com/apppasswords", "") }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_app_password_spaces() {
        assert_eq!(clean_password(" abcd efgh\tijkl mnop "), "abcdefghijklmnop");
    }

    #[test]
    fn checks_email() {
        assert!(valid_email("ali@gmail.com"));
        assert!(!valid_email("ali"));
        assert!(!valid_email("ali@gmail"));
        assert!(!valid_email("a li@gmail.com"));
        assert!(!valid_email("a\"li@gmail.com"));
        assert_eq!(norm_email(" Ali@Gmail.com ").unwrap(), "ali@gmail.com");
    }

    #[test]
    fn quotes_imap_strings() {
        assert_eq!(quote(r#"a"b\c"#), r#""a\"b\\c""#);
    }

    #[test]
    fn parses_search() {
        let l = parse_lines(b"* SEARCH 3 17 42\r\n");
        assert_eq!(search_uids(&l), vec![3, 17, 42]);
        assert!(search_uids(&parse_lines(b"* SEARCH\r\n")).is_empty());
    }

    #[test]
    fn parses_fetch_with_literal() {
        let msg = b"Subject: Hi\r\nFrom: Ali <ali@x.com>\r\n\r\nHello (there)\r\n";
        let mut raw = format!("* 5 FETCH (UID 99 BODY[]<0> {{{}}}\r\n", msg.len()).into_bytes();
        raw.extend_from_slice(msg);
        raw.extend_from_slice(b")\r\n* 6 FETCH (BODY[]<0> {2}\r\nab UID 100)\r\n");
        let f = fetched(&parse_lines(&raw));
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].0, 99);
        assert_eq!(f[0].1, msg.to_vec());
        assert_eq!(f[1], (100, b"ab".to_vec()));
    }

    #[test]
    fn reads_uidvalidity() {
        assert_eq!(uidvalidity(b"* FLAGS (\\Seen)\r\n* OK [UIDVALIDITY 1234] UIDs valid.\r\n"), 1234);
        assert_eq!(uidvalidity(b"* OK nothing\r\n"), 0);
    }

    #[test]
    fn summarises_a_message() {
        let raw = b"From: \"Ali bin Abu\" <ali@contoh.com>\r\nTo: me@gmail.com\r\nSubject: =?UTF-8?B?TWVzeXVhcmF0IGVzb2s=?=\r\nDate: Tue, 3 Nov 2026 10:30:00 +0800\r\nMessage-ID: <abc@contoh.com>\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nAssalamualaikum,\r\n\r\nMesyuarat   projek esok.\r\n";
        let m = summary(7, raw).unwrap();
        assert_eq!(m.uid, 7);
        assert_eq!(m.from_name, "Ali bin Abu");
        assert_eq!(m.from_addr, "ali@contoh.com");
        assert_eq!(m.subject, "Mesyuarat esok");
        assert_eq!(m.snippet, "Assalamualaikum, Mesyuarat projek esok.");
        assert_eq!(m.message_id, "abc@contoh.com");
        assert!(m.date > 0);
    }

    #[test]
    fn html_only_mail_becomes_text() {
        let raw = b"From: shop@x.com\r\nSubject: Order\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<html><body><p>Your <b>order</b> shipped</p><script>alert(1)</script></body></html>\r\n";
        let b = body(1, raw, false).unwrap();
        assert!(b.text.contains("Your order shipped"), "{}", b.text);
        assert!(!b.text.contains("<b>"));
        assert_eq!(b.from_name, "shop@x.com");
    }

    fn mail(uid: u32) -> Mail {
        Mail { uid, from_name: "a".into(), from_addr: "a".into(), subject: "s".into(), snippet: "".into(), date: 0,
               message_id: "".into(), important: false, category: "".into() }
    }

    #[test]
    fn lists_newest_unread_not_opened_here() {
        let read: HashSet<u32> = [5].into_iter().collect();
        assert_eq!(wanted(&[1, 9, 5, 3], &read, &HashMap::new()), vec![9, 3, 1]);
        let many: Vec<u32> = (1..=80).collect();
        let w = wanted(&many, &HashSet::new(), &HashMap::new());
        assert_eq!(w.len(), PRIMARY_LIST);
        assert_eq!(w[0], 80);
    }

    #[test]
    fn promotions_cant_crowd_out_primary() {
        // The newest 100 are promotions; Primary still gets its full share.
        let cats: BTreeMap<String, Vec<u32>> = [("promotions".to_string(), (101..=200).collect())].into_iter().collect();
        let unseen: Vec<u32> = (1..=200).collect();
        let w = wanted(&unseen, &HashSet::new(), &cat_map(&cats));
        assert_eq!(w.iter().filter(|u| **u > 100).count(), OTHER_LIST);
        assert_eq!(w.iter().filter(|u| **u <= 100).count(), PRIMARY_LIST);
        assert!(w.windows(2).all(|p| p[0] > p[1]), "newest first");
    }

    #[test]
    fn counts_unread_important_and_categories_without_opened_ones() {
        let mut st = Store { unseen: vec![1, 2, 3, 4], important: vec![3, 2], ..Store::default() };
        st.cats.insert("promotions".into(), vec![4]);
        st.mails.insert(3, mail(3));
        st.mails.insert(4, mail(4));
        let ib = inbox_of("me@gmail.com", &st, &[2].into_iter().collect());
        assert_eq!(ib.unread, 3);
        assert_eq!(ib.important, 1);
        assert_eq!(ib.categories.get("primary"), Some(&2));
        assert_eq!(ib.categories.get("promotions"), Some(&1));
        let m3 = ib.items.iter().find(|m| m.uid == 3).unwrap();
        assert!(m3.important);
        assert_eq!(m3.category, "primary");
        assert_eq!(ib.items.iter().find(|m| m.uid == 4).unwrap().category, "promotions");
    }

    /// Talks to the real Gmail server: TLS, greeting, tagged replies and the wrong-password
    /// message. `cargo test -- --ignored gmail_rejects`.
    #[test]
    #[ignore]
    fn gmail_rejects_a_wrong_app_password() {
        let e = Imap::login("glassbar.test.nobody@gmail.com", "aaaabbbbccccdddd").err().expect("login must fail");
        assert_eq!(e, AUTH_FAILED);
    }

    /// Writes, reads and removes a test entry in Windows Credential Manager.
    #[test]
    #[ignore]
    fn keeps_the_password_in_credential_manager() {
        let k = "GlassBar/test/credential-roundtrip";
        secrets().set(k, "me@gmail.com", "abcdéfgh ijkl").unwrap();
        assert_eq!(secrets().get(k).as_deref(), Some("abcdéfgh ijkl"));
        secrets().delete(k).unwrap();
        assert_eq!(secrets().get(k), None);
        secrets().delete(k).unwrap();
    }

    #[test]
    fn notifies_primary_and_important_by_default() {
        let promo = Mail { category: "promotions".into(), ..mail(1) };
        let primary = Mail { category: "primary".into(), ..mail(2) };
        let imp_promo = Mail { important: true, ..promo.clone() };
        assert!(!worth_notifying("smart", &promo));
        assert!(worth_notifying("smart", &primary));
        assert!(worth_notifying("smart", &imp_promo));
        assert!(worth_notifying("all", &promo));
        assert!(!worth_notifying("off", &primary));
    }

    #[test]
    fn new_mail_is_above_the_last_uid_and_never_on_the_first_check() {
        assert_eq!(arrived(10, true, &[3, 11, 12]), vec![12, 11]);
        assert!(arrived(0, false, &[3, 11, 12]).is_empty());
        assert!(arrived(12, true, &[3, 11, 12]).is_empty());
    }

    #[test]
    fn groups_many_new_emails_into_one_note() {
        let m = |n: &str| Mail { from_name: n.into(), subject: format!("from {n}"), ..mail(1) };
        assert_eq!(notes_for(&[m("Ali")]), vec![("Ali".to_string(), "from Ali".to_string())]);
        let many = notes_for(&[m("Ali"), m("Abu"), m("Ali"), m("Siti"), m("Shopee")]);
        assert_eq!(many, vec![("5 new emails".to_string(), "Ali, Abu, Siti and 1 more".to_string())]);
    }

    #[test]
    fn finds_the_account_in_widget_settings() {
        let s = serde_json::json!({ "accounts": [{ "email": "a@gmail.com", "name": "" }] });
        assert!(has_account(&s, "A@gmail.com"));
        assert!(!has_account(&s, "b@gmail.com"));
        assert!(has_account(&serde_json::json!({ "email": " b@gmail.com " }), "b@gmail.com"));
    }

    #[test]
    fn links_to_the_message_in_gmail() {
        assert_eq!(web_link("me@gmail.com", ""), "https://mail.google.com/mail/u/?authuser=me%40gmail.com#inbox");
        assert!(web_link("me@gmail.com", "a+b@x.com").ends_with("#search/rfc822msgid%3Aa%2Bb%40x.com"));
    }
}
