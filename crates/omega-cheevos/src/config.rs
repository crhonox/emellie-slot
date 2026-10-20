use std::path::Path;

pub const CHEEVOS_FILE: &str = "Config/retroachievements.txt";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Secret {
    Password(String),
    Token(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Credentials {
    pub username: String,
    pub secret: Secret,
    pub host: Option<String>,
}

impl Credentials {
    pub fn read(root: &Path) -> Option<Self> {
        Self::parse(&std::fs::read_to_string(root.join(CHEEVOS_FILE)).ok()?)
    }

    /// The account in `text`, unless Settings turned RetroAchievements off.
    pub fn parse(text: &str) -> Option<Self> {
        let mut keys = omega_store::ini::parse(text);
        if keys.remove("enabled").is_some_and(|v| is_off(&v)) {
            return None;
        }
        let mut take = |k: &str| keys.remove(k).filter(|v| !v.is_empty());
        let username = take("username")?;
        let secret = match (take("password"), take("token")) {
            (Some(p), _) => Secret::Password(p),
            (None, Some(t)) => Secret::Token(t),
            (None, None) => return None,
        };
        Some(Self {
            username,
            secret,
            host: take("host"),
        })
    }
}

pub fn remember_token(root: &Path, token: &str) -> std::io::Result<()> {
    let path = root.join(CHEEVOS_FILE);
    let text = std::fs::read_to_string(&path)?;
    let text = omega_store::ini::set(&text, "token", Some(token))?;
    let text = omega_store::ini::set(&text, "password", None)?;
    omega_store::atomic_write(&path, text.as_bytes())
}

fn is_off(v: &str) -> bool {
    matches!(
        v.trim().to_ascii_lowercase().as_str(),
        "false" | "off" | "0" | "no"
    )
}

fn edit(
    root: &Path,
    change: impl FnOnce(String) -> std::io::Result<String>,
) -> std::io::Result<()> {
    let path = root.join(CHEEVOS_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    omega_store::atomic_write(&path, change(text)?.as_bytes())
}

/// What Settings shows of the account: its username, whether one is saved to sign in with, and
/// whether RetroAchievements is on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    pub username: String,
    pub signed: bool,
    pub enabled: bool,
}

impl Default for Account {
    /// A card with no account: RetroAchievements on, waiting for one.
    fn default() -> Self {
        Account {
            username: String::new(),
            signed: false,
            enabled: true,
        }
    }
}

pub fn account(root: &Path) -> Account {
    let keys = omega_store::ini::read(root, CHEEVOS_FILE);
    let get = |k: &str| keys.get(k).filter(|v| !v.is_empty());
    Account {
        username: get("username").cloned().unwrap_or_default(),
        signed: get("password").is_some() || get("token").is_some(),
        enabled: !get("enabled").is_some_and(|v| is_off(v)),
    }
}

/// Saves the username and password typed in Settings: the next sign-in uses the password and
/// keeps a token in its place, as one written by hand does.
pub fn sign_in_with(root: &Path, username: &str, password: &str) -> std::io::Result<()> {
    edit(root, |text| {
        let text = omega_store::ini::set(&text, "username", Some(username))?;
        let text = omega_store::ini::set(&text, "password", Some(password))?;
        omega_store::ini::set(&text, "token", None)
    })
}

/// Forgets the password and token, keeping the username to sign in again with.
pub fn sign_out(root: &Path) -> std::io::Result<()> {
    edit(root, |text| {
        let text = omega_store::ini::set(&text, "password", None)?;
        omega_store::ini::set(&text, "token", None)
    })
}

pub fn set_enabled(root: &Path, on: bool) -> std::io::Result<()> {
    edit(root, |text| {
        omega_store::ini::set(&text, "enabled", Some(if on { "true" } else { "false" }))
    })
}
