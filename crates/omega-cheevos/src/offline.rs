//! RetroAchievements without a network, as spruce's RAOfflineProxy does it: what the server
//! answered for an account, a game and its badges is kept on the card, and answers for them
//! again when it can't be reached; an unlock earned meanwhile is kept in a queue, answered as
//! awarded, and sent, signed for the time since it was earned, once back online.

use std::collections::HashSet;
use std::ffi::{c_char, CString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ffi::RETRYABLE_CLIENT_ERROR;
use crate::transport::{Request, Response, Transport};

/// Where the answers and the queue are kept, under the card's root.
pub const OFFLINE_DIR: &str = "Config/cheevos";
const PENDING: &str = "pending.txt";
const BADGES: &str = "badges";
const URLENCODED: &str = "application/x-www-form-urlencoded";

/// An unlock the server has not been told of yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unlock {
    pub user: String,
    pub achievement: u32,
    pub hash: String,
    /// When it was earned, in seconds since 1970.
    pub when: u64,
    pub url: String,
}

impl Unlock {
    fn line(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\n",
            self.user, self.achievement, self.hash, self.when, self.url
        )
    }

    fn parse(line: &str) -> Option<Self> {
        let mut f = line.split('\t');
        let unlock = Unlock {
            user: f.next()?.to_string(),
            achievement: f.next()?.parse().ok()?,
            hash: f.next()?.to_string(),
            when: f.next()?.parse().ok()?,
            url: f.next()?.trim_end().to_string(),
        };
        (!unlock.user.is_empty() && unlock.achievement != 0).then_some(unlock)
    }

    fn same(&self, other: &Unlock) -> bool {
        self.achievement == other.achievement && self.user.eq_ignore_ascii_case(&other.user)
    }
}

/// What a round of sending the queue did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sent {
    /// Unlocks the server took, or already had.
    pub sent: usize,
    /// Unlocks still waiting: the server could not be reached, or the account's token was
    /// turned down.
    pub left: usize,
}

pub struct Offline {
    dir: PathBuf,
    online: Box<dyn Fn() -> bool + Send + Sync>,
    queue: Mutex<()>,
    queued: AtomicBool,
    fetching: Mutex<HashSet<String>>,
}

impl Offline {
    /// Answers kept in `root`'s Config/cheevos; `online` says whether a network is joined, so
    /// that without one the server isn't waited for.
    pub fn new(root: &Path, online: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        Self::at(root.join(OFFLINE_DIR), online)
    }

    pub fn at(dir: PathBuf, online: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        let this = Offline {
            dir,
            online: Box::new(online),
            queue: Mutex::new(()),
            queued: AtomicBool::new(false),
            fetching: Mutex::new(HashSet::new()),
        };
        this.queued
            .store(!this.read_queue().is_empty(), Ordering::Release);
        this
    }

    fn online(&self) -> bool {
        (self.online)()
    }

    /// Whether `user` could sign in with `token` with no network: the server let them in with
    /// it before.
    pub fn knows(&self, user: &str, token: &str) -> bool {
        self.read(&login_key(user))
            .is_some_and(|body| field_str(&body, "Token").as_deref() == Some(token))
    }

    /// Sends `request` through `inner`, or answers it from the card when the server can't be
    /// reached; `remaining` is how many achievements of the game loaded are still locked.
    pub fn send(
        self: &Arc<Self>,
        inner: &Arc<dyn Transport>,
        request: &Request,
        remaining: &dyn Fn() -> Option<u32>,
    ) -> Response {
        let post = request.post.as_deref().unwrap_or_default();
        let get = |k: &str| param(post, k);
        let api = get("r").unwrap_or_default();
        let user = get("u").unwrap_or_default();
        match api.as_str() {
            "login2" => {
                let key = login_key(&user);
                let cached = || {
                    let body = self.read(&key)?;
                    let token = get("t")?;
                    (field_str(&body, "Token").as_deref() == Some(token.as_str())).then_some(body)
                };
                self.kept(inner, request, &key, cached)
            }
            "achievementsets" | "patch" | "gameid" => {
                let Some(game) = get("m").or_else(|| get("g")) else {
                    return self.pass(inner, request);
                };
                let key = format!("{api}-{}", safe(&game));
                let response = self.kept(inner, request, &key, || self.read(&key));
                if ok(&response) && self.online() {
                    self.fetch_badges(inner, &response.body);
                }
                response
            }
            "startsession" => {
                let key = format!(
                    "session-{}-{}",
                    safe(&user.to_lowercase()),
                    get("g").unwrap_or_default()
                );
                let mut response = self.kept(inner, request, &key, || {
                    Some(self.read(&key).unwrap_or_else(|| {
                        format!(
                            r#"{{"Success":true,"Unlocks":[],"HardcoreUnlocks":[],"ServerNow":{}}}"#,
                            now()
                        )
                    }))
                });
                if ok(&response) {
                    let queued: Vec<_> = self
                        .read_queue()
                        .into_iter()
                        .filter(|u| u.user.eq_ignore_ascii_case(&user))
                        .collect();
                    response.body = with_unlocks(&response.body, &queued);
                }
                response
            }
            "awardachievement" => {
                if self.online() {
                    let response = inner.send(request);
                    if response.status != RETRYABLE_CLIENT_ERROR {
                        return response;
                    }
                }
                let id: u32 = get("a").and_then(|a| a.parse().ok()).unwrap_or(0);
                let ago: u64 = get("o").and_then(|o| o.parse().ok()).unwrap_or(0);
                self.enqueue(Unlock {
                    user: user.clone(),
                    achievement: id,
                    hash: get("m").unwrap_or_default(),
                    when: now().saturating_sub(ago),
                    url: request.url.clone(),
                });
                let login = self.read(&login_key(&user)).unwrap_or_default();
                let mut body = format!(r#"{{"Success":true,"AchievementID":{id}"#);
                for k in ["Score", "SoftcoreScore"] {
                    if let Some(n) = field_num(&login, k) {
                        body += &format!(r#","{k}":{n}"#);
                    }
                }
                if let Some(n) = remaining() {
                    body += &format!(r#","AchievementsRemaining":{n}"#);
                }
                body.push('}');
                reply(body)
            }
            "ping" if !self.online() => reply(r#"{"Success":true}"#.into()),
            _ => self.pass(inner, request),
        }
    }

    /// A badge or game icon, from the card when it was fetched before.
    pub fn fetch(&self, inner: &dyn Transport, url: &str) -> Option<Vec<u8>> {
        let path = self.badge_path(url)?;
        if let Ok(png) = std::fs::read(&path) {
            return Some(png);
        }
        if !self.online() {
            return None;
        }
        let png = inner.fetch(url)?;
        self.write(&path, &png);
        Some(png)
    }

    /// Whether any unlock is waiting to be sent.
    pub fn queued(&self) -> bool {
        self.queued.load(Ordering::Acquire)
    }

    pub fn pending(&self) -> Vec<Unlock> {
        self.read_queue()
    }

    /// Sends the waiting unlocks through `inner`, oldest first, each signed for how long ago it
    /// was earned, with the token the server last gave its account.
    pub fn flush(&self, inner: &dyn Transport) -> Sent {
        let waiting = self.read_queue();
        let mut done = Vec::new();
        let mut result = Sent::default();
        for unlock in &waiting {
            let Some(token) = self
                .read(&login_key(&unlock.user))
                .and_then(|b| field_str(&b, "Token"))
            else {
                continue;
            };
            let seconds = now().saturating_sub(unlock.when).min(u32::MAX as u64) as u32;
            let Some(post) = award_post(
                &unlock.user,
                &token,
                unlock.achievement,
                &unlock.hash,
                seconds,
            ) else {
                done.push(unlock.clone());
                continue;
            };
            let response = inner.send(&Request {
                url: unlock.url.clone(),
                post: Some(post),
                content_type: Some(URLENCODED.into()),
            });
            let body = String::from_utf8_lossy(&response.body);
            if response.status == RETRYABLE_CLIENT_ERROR || response.status >= 500 {
                break;
            }
            if response.status == 401 || response.status == 403 {
                eprintln!(
                    "omega-zero: cheevos: queued unlock {} turned down: {body}",
                    unlock.achievement
                );
                continue;
            }
            if ok(&response) || body.contains("User already has") {
                result.sent += 1;
            } else {
                eprintln!(
                    "omega-zero: cheevos: dropping queued unlock {}: {body}",
                    unlock.achievement
                );
            }
            done.push(unlock.clone());
        }
        let _queue = self.queue.lock().unwrap();
        let left: Vec<_> = self
            .read_queue()
            .into_iter()
            .filter(|u| !done.iter().any(|d| d.same(u)))
            .collect();
        self.write_queue(&left);
        result.left = left.len();
        result
    }

    fn pass(&self, inner: &Arc<dyn Transport>, request: &Request) -> Response {
        if self.online() {
            inner.send(request)
        } else {
            Response::unreachable()
        }
    }

    /// The server's answer, kept under `key` when it is a success; or, when the server can't
    /// be reached, what `cached` finds.
    fn kept(
        &self,
        inner: &Arc<dyn Transport>,
        request: &Request,
        key: &str,
        cached: impl FnOnce() -> Option<String>,
    ) -> Response {
        if self.online() {
            let response = inner.send(request);
            if ok(&response) {
                self.write(&self.dir.join(format!("{key}.json")), &response.body);
            }
            if response.status != RETRYABLE_CLIENT_ERROR {
                return response;
            }
        }
        match cached() {
            Some(body) => reply(body),
            None => Response::unreachable(),
        }
    }

    /// Fetches, one at a time and away from the server's thread, the badges of a game's
    /// achievements not on the card yet, so an unlock earned offline shows its own.
    fn fetch_badges(self: &Arc<Self>, inner: &Arc<dyn Transport>, body: &[u8]) {
        let body = String::from_utf8_lossy(body);
        let mut urls: Vec<String> = all_str(&body, "BadgeURL");
        if urls.is_empty() {
            urls = all_str(&body, "BadgeName")
                .into_iter()
                .map(|n| format!("https://media.retroachievements.org/Badge/{n}.png"))
                .collect();
        }
        urls.extend(all_str(&body, "ImageIconUrl"));
        let mut fetching = self.fetching.lock().unwrap();
        urls.retain(|u| {
            self.badge_path(u).is_some_and(|p| !p.exists()) && fetching.insert(u.clone())
        });
        drop(fetching);
        if urls.is_empty() {
            return;
        }
        let (this, inner) = (self.clone(), inner.clone());
        let thread = std::thread::Builder::new()
            .name("omega-cheevos-badges".into())
            .stack_size(crate::THREAD_STACK)
            .spawn(move || {
                for url in urls {
                    let png = inner.fetch(&url);
                    if let (Some(png), Some(path)) = (png, this.badge_path(&url)) {
                        this.write(&path, &png);
                    }
                    this.fetching.lock().unwrap().remove(&url);
                }
            });
        if let Err(e) = thread {
            eprintln!("omega-zero: cheevos: {e}");
        }
    }

    fn badge_path(&self, url: &str) -> Option<PathBuf> {
        let name = url.rsplit('/').next().filter(|n| !n.is_empty())?;
        Some(self.dir.join(BADGES).join(safe(name)))
    }

    fn read(&self, key: &str) -> Option<String> {
        std::fs::read_to_string(self.dir.join(format!("{key}.json"))).ok()
    }

    fn write(&self, path: &Path, bytes: &[u8]) {
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|_| omega_store::atomic_write(path, bytes));
        if let Err(e) = written {
            eprintln!("omega-zero: cheevos: {}: {e}", path.display());
        }
    }

    fn enqueue(&self, unlock: Unlock) {
        if unlock.achievement == 0 {
            return;
        }
        let _queue = self.queue.lock().unwrap();
        let mut queue = self.read_queue();
        if queue.iter().any(|u| u.same(&unlock)) {
            return;
        }
        eprintln!(
            "omega-zero: cheevos: offline, keeping unlock {} to send later",
            unlock.achievement
        );
        queue.push(unlock);
        self.write_queue(&queue);
    }

    fn read_queue(&self) -> Vec<Unlock> {
        std::fs::read_to_string(self.dir.join(PENDING))
            .unwrap_or_default()
            .lines()
            .filter_map(Unlock::parse)
            .collect()
    }

    fn write_queue(&self, queue: &[Unlock]) {
        let path = self.dir.join(PENDING);
        if queue.is_empty() {
            let _ = std::fs::remove_file(&path);
        } else {
            let text: String = queue.iter().map(Unlock::line).collect();
            self.write(&path, text.as_bytes());
        }
        self.queued.store(!queue.is_empty(), Ordering::Release);
    }
}

fn login_key(user: &str) -> String {
    format!("login-{}", safe(&user.to_lowercase()))
}

fn reply(body: String) -> Response {
    Response {
        status: 200,
        body: body.into_bytes(),
    }
}

fn ok(response: &Response) -> bool {
    response.status == 200
        && field_raw(&String::from_utf8_lossy(&response.body), "Success") == Some("true")
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Only what a file name can hold, so a hash or username never leaves the folder.
fn safe(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => c,
            _ => '_',
        })
        .collect::<String>()
        .trim_start_matches('.')
        .to_string()
}

fn award_post(
    user: &str,
    token: &str,
    achievement: u32,
    hash: &str,
    seconds: u32,
) -> Option<String> {
    let (user, token, hash) = (
        CString::new(user).ok()?,
        CString::new(token).ok()?,
        CString::new(hash).ok()?,
    );
    let mut out = [0 as c_char; 512];
    let len = unsafe {
        crate::ffi::omega_award_post(
            user.as_ptr(),
            token.as_ptr(),
            achievement,
            hash.as_ptr(),
            seconds,
            out.as_mut_ptr(),
            out.len(),
        )
    };
    (len > 0).then(|| unsafe { crate::ffi::text(out.as_ptr()) })
}

/// A form field of `post`, decoded.
pub fn param(post: &str, key: &str) -> Option<String> {
    post.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| decode(v))
    })
}

fn decode(v: &str) -> String {
    let b = v.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                let hex = std::str::from_utf8(&b[i + 1..i + 3]).ok();
                match hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    Some(x) => {
                        out.push(x);
                        i += 2;
                    }
                    None => out.push(b'%'),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Where the value of the first `"key":` in `json` starts, past any spaces.
fn value_at(json: &str, key: &str) -> Option<usize> {
    let needle = format!("\"{key}\"");
    let mut from = 0;
    while let Some(i) = json[from..].find(&needle) {
        let after = from + i + needle.len();
        let rest = json[after..].trim_start();
        if let Some(rest) = rest.strip_prefix(':') {
            return Some(json.len() - rest.trim_start().len());
        }
        from = after;
    }
    None
}

/// The bare value (a number, true, false) of the first `"key":` in `json`.
fn field_raw<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let rest = &json[value_at(json, key)?..];
    let end = rest
        .find(|c: char| c == ',' || c == '}' || c == ']' || c.is_whitespace())
        .unwrap_or(rest.len());
    Some(&rest[..end])
}

fn field_num(json: &str, key: &str) -> Option<i64> {
    field_raw(json, key)?.parse().ok()
}

fn field_str(json: &str, key: &str) -> Option<String> {
    string_at(json, value_at(json, key)?)
}

/// Every string value of `"key":` in `json`, empty ones left out.
fn all_str(json: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{key}\"");
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = json[from..].find(&needle) {
        let here = &json[from + i..];
        from += i + needle.len();
        if let Some(s) = value_at(here, key).and_then(|at| string_at(here, at)) {
            if !s.is_empty() {
                out.push(s);
            }
        }
    }
    out
}

/// The JSON string starting at `at` in `json`, unescaped as far as URLs and names need.
fn string_at(json: &str, at: usize) -> Option<String> {
    let rest = json[at..].strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    out.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?).unwrap_or('?'));
                }
                other => out.push(other),
            },
            c => out.push(c),
        }
    }
    None
}

/// A startsession answer with the queued unlocks added to its softcore ones, so an
/// achievement earned offline isn't earned again before the server hears of it.
fn with_unlocks(body: &[u8], queued: &[Unlock]) -> Vec<u8> {
    let json = String::from_utf8_lossy(body).into_owned();
    if queued.is_empty() {
        return json.into_bytes();
    }
    let entries: Vec<String> = queued
        .iter()
        .map(|u| format!(r#"{{"ID":{},"When":{}}}"#, u.achievement, u.when))
        .collect();
    let entries = entries.join(",");
    let out = match value_at(&json, "Unlocks").filter(|&i| json[i..].starts_with('[')) {
        Some(i) => {
            let rest = json[i + 1..].trim_start();
            let sep = if rest.starts_with(']') { "" } else { "," };
            format!("{}{entries}{sep}{}", &json[..i + 1], &json[i + 1..])
        }
        None => match json.find('{') {
            Some(i) => format!(
                r#"{}"Unlocks":[{entries}],{}"#,
                &json[..i + 1],
                &json[i + 1..]
            ),
            None => json,
        },
    };
    out.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_are_read_off_compact_and_spaced_json() {
        let j =
            r#"{"Success": true,"Token":"abc\/d","Score":15,"HardcoreUnlocks":[],"Unlocks" : []}"#;
        assert_eq!(field_raw(j, "Success"), Some("true"));
        assert_eq!(field_str(j, "Token").as_deref(), Some("abc/d"));
        assert_eq!(field_num(j, "Score"), Some(15));
        assert_eq!(value_at(j, "Unlocks").map(|i| &j[i..i + 1]), Some("["));
    }

    #[test]
    fn queued_unlocks_join_the_softcore_ones_not_the_hardcore_ones() {
        let q = |id| Unlock {
            user: "P".into(),
            achievement: id,
            hash: String::new(),
            when: 7,
            url: String::new(),
        };
        let empty = br#"{"Success":true,"HardcoreUnlocks":[],"Unlocks":[]}"#;
        assert_eq!(
            String::from_utf8(with_unlocks(empty, &[q(5)])).unwrap(),
            r#"{"Success":true,"HardcoreUnlocks":[],"Unlocks":[{"ID":5,"When":7}]}"#
        );
        let some = br#"{"Success":true,"Unlocks":[{"ID":1,"When":2}]}"#;
        assert_eq!(
            String::from_utf8(with_unlocks(some, &[q(5)])).unwrap(),
            r#"{"Success":true,"Unlocks":[{"ID":5,"When":7},{"ID":1,"When":2}]}"#
        );
        let none = br#"{"Success":true}"#;
        assert_eq!(
            String::from_utf8(with_unlocks(none, &[q(5)])).unwrap(),
            r#"{"Unlocks":[{"ID":5,"When":7}],"Success":true}"#
        );
    }

    #[test]
    fn form_fields_are_decoded_and_names_kept_in_the_folder() {
        assert_eq!(param("r=login2&u=A%20b&t=x", "u").as_deref(), Some("A b"));
        assert_eq!(param("r=login2&u=a", "t"), None);
        assert_eq!(safe("../x/y"), "_x_y");
    }

    #[test]
    fn badge_urls_come_from_badge_names_when_none_are_given() {
        let j = r#"{"ImageIconUrl":"http:\/\/s\/Images\/1.png","Achievements":[{"BadgeName":"00234"},{"BadgeName":""}]}"#;
        assert_eq!(all_str(j, "BadgeName"), vec!["00234"]);
        assert_eq!(all_str(j, "ImageIconUrl"), vec!["http://s/Images/1.png"]);
    }
}
