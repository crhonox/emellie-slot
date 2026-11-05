//! Updates over the air: Emellie Slot's releases on GitHub, tagged `es-x.y.z`, each carrying the
//! card tree as a zip and that zip's SHA-256.
//!
//! The check asks GitHub for the latest release and compares its tag with this build's version.
//! An install fetches the zip into `.update/` on the card, checks it against its SHA-256, and
//! unpacks the tree's `System/` beside the one in use, as `.update/System`, with `.update/ready`
//! naming the release. Emellie Slot then leaves, and `launch_frontend.sh` swaps the two folders
//! and starts the new one, going back to the old where the new will not start. Nothing outside
//! `System/` is touched: the games, saves, states and settings stay as they are.
//!
//! All of it runs on a thread of its own, through the curl BaseOS ships; what it learns comes
//! back as news the app takes each tick.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

use crate::json::Json;
use crate::label_sync::{Curl, Http, SOFTNAME};

/// The repository whose releases are Emellie Slot's, `owner/name`; a build can name another.
pub fn repo() -> &'static str {
    option_env!("EMELLIE_UPDATE_REPO").unwrap_or("crhonox/emellie-slot")
}

/// What every release's tag starts with, the version after it.
pub const TAG_PREFIX: &str = "es-";

/// Where on the card an update is fetched and unpacked, `launch_frontend.sh` looking there.
pub const DIR: &str = ".update";

/// The file that says an unpacked update is whole and waiting, holding its tag.
pub const READY: &str = "ready";

/// The end of the card tree's zip among a release's files, and of its SHA-256's.
const ZIP_END: &str = "-h700.zip";
const SHA_END: &str = "-h700.zip.sha256";

/// How often a download in progress says how far it has got.
const PROGRESS_MS: u64 = 250;

/// A version as a release tag carries it, `es-1.2.3`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// `1.2.3`, with or without the `es-` before it.
    pub fn parse(text: &str) -> Option<Version> {
        let text = text.trim();
        let text = text.strip_prefix(TAG_PREFIX).unwrap_or(text);
        let mut parts = text.split('.').map(|p| p.parse::<u32>().ok());
        let version = Version {
            major: parts.next()??,
            minor: parts.next()??,
            patch: parts.next()??,
        };
        parts.next().is_none().then_some(version)
    }

    /// This build's, from its crate's version.
    pub fn current() -> Version {
        Version::parse(env!("CARGO_PKG_VERSION")).expect("the crate's version is x.y.z")
    }

    /// The release tag it goes by, `es-1.2.3`.
    pub fn tag(self) -> String {
        format!("{TAG_PREFIX}{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A release, as GitHub lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    /// What the release says of itself, its notes' first line.
    pub notes: String,
    pub zip_url: String,
    /// The zip's size in bytes, as GitHub gives it, for the download's progress.
    pub zip_size: u64,
    pub sha_url: String,
}

/// The URL GitHub answers with the latest release on: never a draft nor a pre-release.
pub fn latest_url() -> String {
    format!("https://api.github.com/repos/{}/releases/latest", repo())
}

/// The release in GitHub's reply about the latest one: none where its tag is not `es-x.y.z`
/// or it lacks the card tree's zip or its SHA-256.
pub fn parse_release(body: &str) -> Option<Release> {
    let json = Json::parse(body)?;
    let version = Version::parse(json.get("tag_name")?.text()?)?;
    if !json.get("tag_name")?.text()?.starts_with(TAG_PREFIX) {
        return None;
    }
    let assets = json.get("assets")?.list();
    let asset = |end: &str| {
        assets.iter().find(|a| {
            a.get("name")
                .and_then(Json::text)
                .is_some_and(|n| n.ends_with(end))
        })
    };
    let zip = asset(ZIP_END)?;
    let sha = asset(SHA_END)?;
    let url = |a: &Json| a.get("browser_download_url")?.text().map(str::to_string);
    let notes = json
        .get("body")
        .and_then(Json::text)
        .unwrap_or("")
        .lines()
        .map(|l| l.trim().trim_start_matches('#').trim())
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .to_string();
    Some(Release {
        version,
        notes,
        zip_url: url(zip)?,
        zip_size: zip.get("size").and_then(Json::count).unwrap_or(0),
        sha_url: url(sha)?,
    })
}

/// The SHA-256 a `.sha256` file names first, in lower-case hex, as `sha256sum` writes it.
pub fn parse_sha(text: &str) -> Option<String> {
    let hex = text.split_whitespace().next()?.to_ascii_lowercase();
    (hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit())).then_some(hex)
}

/// SHA-256 (FIPS 180-4), fed a piece at a time.
pub struct Sha256 {
    state: [u32; 8],
    block: [u8; 64],
    filled: usize,
    length: u64,
}

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

impl Default for Sha256 {
    fn default() -> Self {
        Sha256 {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            block: [0; 64],
            filled: 0,
            length: 0,
        }
    }
}

impl Sha256 {
    pub fn update(&mut self, mut bytes: &[u8]) {
        self.length += bytes.len() as u64;
        while !bytes.is_empty() {
            let take = (64 - self.filled).min(bytes.len());
            self.block[self.filled..self.filled + take].copy_from_slice(&bytes[..take]);
            self.filled += take;
            bytes = &bytes[take..];
            if self.filled == 64 {
                let block = self.block;
                self.compress(&block);
                self.filled = 0;
            }
        }
    }

    /// The digest, in lower-case hex.
    pub fn hex(mut self) -> String {
        let bits = self.length.wrapping_mul(8);
        let mut pad = vec![0x80u8];
        let room = (64 + 56 - (self.filled + 1) % 64) % 64;
        pad.extend(std::iter::repeat_n(0, room));
        pad.extend(bits.to_be_bytes());
        let length = self.length;
        self.update(&pad);
        self.length = length;
        self.state.iter().map(|w| format!("{w:08x}")).collect()
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for (i, word) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (s, v) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *s = s.wrapping_add(v);
        }
    }
}

/// The SHA-256 of the file at `path`, in lower-case hex.
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut sha = Sha256::default();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            return Ok(sha.hex());
        }
        sha.update(&buf[..n]);
    }
}

/// Why a check or an install came to nothing.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Fail {
    /// GitHub could not be reached, or answered with something other than a release.
    Unreachable,
    /// The zip did not come down whole.
    Download,
    /// The zip is not the one its SHA-256 names.
    Corrupt,
    /// The zip would not unpack, or held no Emellie Slot.
    Unpack,
    /// Stopped before it was done.
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateNews {
    /// The latest release, whatever its version.
    Checked(Release),
    /// So many bytes of so many have come down.
    Progress(u64, u64),
    Verifying,
    Unpacking,
    /// The release is unpacked in `.update/System` and waiting for Emellie Slot to leave.
    Ready(Version),
    Failed(Fail),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateJob {
    Check,
    Install(Release),
    Stop,
}

pub trait UpdateJobs: Send {
    fn ask(&mut self, job: UpdateJob);
    fn news(&mut self) -> Vec<UpdateNews>;
}

/// The latest release, through `http`.
pub fn check(http: &mut dyn Http) -> Result<Release, Fail> {
    let (status, body) = http.get(&latest_url()).ok_or(Fail::Unreachable)?;
    if status != 200 {
        return Err(Fail::Unreachable);
    }
    let body = String::from_utf8_lossy(&body);
    parse_release(&body).ok_or(Fail::Unreachable)
}

/// How a download and an unpack are run: by curl and 7-Zip on the handheld, by stand-ins in
/// the tests.
pub trait Tools {
    /// Starts fetching `url` into `to`.
    fn download(&mut self, url: &str, to: &Path) -> std::io::Result<Child>;
    /// Unpacks the zip at `zip` into the folder `into`.
    fn unpack(&mut self, zip: &Path, into: &Path) -> bool;
}

/// curl for the download, and the static 7-Zip PortMaster's bundle brings (System/ports/bin),
/// else BusyBox's unzip, for the unpack.
pub struct CardTools {
    pub root: PathBuf,
}

impl Tools for CardTools {
    fn download(&mut self, url: &str, to: &Path) -> std::io::Result<Child> {
        Command::new("curl")
            .args(["-s", "-f", "-L", "-m", "3600", "-A", SOFTNAME, "-o"])
            .arg(to)
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    }

    fn unpack(&mut self, zip: &Path, into: &Path) -> bool {
        let seven = self.root.join("System/ports/bin/7zzs");
        let ran = if seven.exists() {
            Command::new(&seven)
                .arg("x")
                .arg("-y")
                .arg(format!("-o{}", into.display()))
                .arg(zip)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
        } else {
            Command::new("unzip")
                .arg("-q")
                .arg("-o")
                .arg(zip)
                .arg("-d")
                .arg(into)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
        };
        ran.is_ok_and(|s| s.success())
    }
}

/// The `System/` inside an unpacked tree: the one folder down from `into` that holds an
/// `emellie-slot`, or `into`'s own.
pub fn unpacked_system(into: &Path) -> Option<PathBuf> {
    let holds = |dir: &Path| dir.join("System/emellie-slot").is_file();
    if holds(into) {
        return Some(into.join("System"));
    }
    std::fs::read_dir(into)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_dir() && holds(p))
        .map(|p| p.join("System"))
}

/// Fetches, checks and unpacks `release` into `root`'s `.update/`, telling how it goes.
pub fn install(
    root: &Path,
    release: &Release,
    http: &mut dyn Http,
    tools: &mut dyn Tools,
    stop: &AtomicBool,
    tell: &mut dyn FnMut(UpdateNews),
) -> Result<(), Fail> {
    let dir = root.join(DIR);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|_| Fail::Download)?;

    let want = match http.get(&release.sha_url) {
        Some((200, body)) => parse_sha(&String::from_utf8_lossy(&body)).ok_or(Fail::Download)?,
        _ => return Err(Fail::Unreachable),
    };

    let zip = dir.join("update.zip");
    let mut child = tools
        .download(&release.zip_url, &zip)
        .map_err(|_| Fail::Download)?;
    let total = release.zip_size;
    let fetched = loop {
        if stop.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_dir_all(&dir);
            return Err(Fail::Stopped);
        }
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) => {}
            Err(_) => break false,
        }
        let got = std::fs::metadata(&zip).map(|m| m.len()).unwrap_or(0);
        tell(UpdateNews::Progress(got, total));
        std::thread::sleep(Duration::from_millis(PROGRESS_MS));
    };
    if !fetched {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(Fail::Download);
    }
    let got = std::fs::metadata(&zip).map(|m| m.len()).unwrap_or(0);
    tell(UpdateNews::Progress(got, total.max(got)));

    tell(UpdateNews::Verifying);
    if sha256_file(&zip).ok().as_deref() != Some(want.as_str()) {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(Fail::Corrupt);
    }

    tell(UpdateNews::Unpacking);
    let into = dir.join("unpacked");
    let system = tools
        .unpack(&zip, &into)
        .then(|| unpacked_system(&into))
        .flatten();
    let _ = std::fs::remove_file(&zip);
    let Some(system) = system else {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(Fail::Unpack);
    };
    if std::fs::rename(&system, dir.join("System")).is_err() {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(Fail::Unpack);
    }
    let _ = std::fs::remove_dir_all(&into);
    omega_store::atomic_write(
        &dir.join(READY),
        format!("{}\n", release.version.tag()).as_bytes(),
    )
    .map_err(|_| Fail::Unpack)?;
    Ok(())
}

/// Checks and installs on a thread of its own, one job at a time.
pub struct Worker {
    jobs: Sender<UpdateJob>,
    news: Receiver<UpdateNews>,
    stop: Arc<AtomicBool>,
}

impl Worker {
    pub fn spawn(root: PathBuf) -> Worker {
        let (jobs, rx) = channel::<UpdateJob>();
        let (tx, news) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let _ = std::thread::Builder::new()
            .name("omega-update".into())
            .spawn(move || {
                for job in rx {
                    stopping.store(false, Ordering::Relaxed);
                    let said = match job {
                        UpdateJob::Check => match check(&mut Curl) {
                            Ok(release) => UpdateNews::Checked(release),
                            Err(fail) => UpdateNews::Failed(fail),
                        },
                        UpdateJob::Install(release) => {
                            let mut tools = CardTools { root: root.clone() };
                            let mut tell = |n| {
                                let _ = tx.send(n);
                            };
                            match install(
                                &root, &release, &mut Curl, &mut tools, &stopping, &mut tell,
                            ) {
                                Ok(()) => UpdateNews::Ready(release.version),
                                Err(fail) => UpdateNews::Failed(fail),
                            }
                        }
                        UpdateJob::Stop => continue,
                    };
                    let _ = tx.send(said);
                }
            });
        Worker { jobs, news, stop }
    }
}

impl UpdateJobs for Worker {
    fn ask(&mut self, job: UpdateJob) {
        match job {
            UpdateJob::Stop => self.stop.store(true, Ordering::Relaxed),
            job => {
                let _ = self.jobs.send(job);
            }
        }
    }

    fn news(&mut self) -> Vec<UpdateNews> {
        self.news.try_iter().collect()
    }
}

/// No updates: a build off the handheld, or the tests, unless they hand the app their own.
#[derive(Default)]
pub struct NoUpdates;

impl UpdateJobs for NoUpdates {
    fn ask(&mut self, _job: UpdateJob) {}

    fn news(&mut self) -> Vec<UpdateNews> {
        Vec::new()
    }
}

/// Where an update stands, as the Update screen and the Settings row show it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateState {
    /// Not asked yet this boot.
    Unknown,
    Checking,
    /// The latest release is this build's, or older.
    UpToDate,
    Available(Release),
    Downloading {
        got: u64,
        total: u64,
        release: Release,
    },
    Verifying(Release),
    Unpacking(Release),
    /// Unpacked and waiting: Emellie Slot leaves for `launch_frontend.sh` to put it in place.
    Ready(Version),
    Failed {
        fail: Fail,
        /// The release a failed install was of, to try again.
        release: Option<Release>,
    },
}

pub struct Updates {
    jobs: Box<dyn UpdateJobs>,
    state: UpdateState,
    installed: Version,
    /// A release newer than this build was found, and not yet told of.
    news_of: Option<Version>,
}

impl Updates {
    pub fn new(jobs: Box<dyn UpdateJobs>) -> Self {
        Updates::with_installed(jobs, Version::current())
    }

    pub fn with_installed(jobs: Box<dyn UpdateJobs>, installed: Version) -> Self {
        Updates {
            jobs,
            state: UpdateState::Unknown,
            installed,
            news_of: None,
        }
    }

    pub fn state(&self) -> &UpdateState {
        &self.state
    }

    pub fn installed(&self) -> Version {
        self.installed
    }

    /// The newer release found, while it is not yet being installed.
    pub fn available(&self) -> Option<&Release> {
        match &self.state {
            UpdateState::Available(r) => Some(r),
            _ => None,
        }
    }

    /// A check or an install is under way.
    pub fn busy(&self) -> bool {
        matches!(
            self.state,
            UpdateState::Checking
                | UpdateState::Downloading { .. }
                | UpdateState::Verifying(_)
                | UpdateState::Unpacking(_)
        )
    }

    pub fn installing(&self) -> bool {
        self.busy() && !matches!(self.state, UpdateState::Checking)
    }

    pub fn ready(&self) -> bool {
        matches!(self.state, UpdateState::Ready(_))
    }

    pub fn check(&mut self) {
        if self.busy() || self.ready() {
            return;
        }
        self.state = UpdateState::Checking;
        self.jobs.ask(UpdateJob::Check);
    }

    /// Installs the release found, or the one whose install failed.
    pub fn install(&mut self) {
        let release = match &self.state {
            UpdateState::Available(r) => r.clone(),
            UpdateState::Failed {
                release: Some(r), ..
            } => r.clone(),
            _ => return,
        };
        self.state = UpdateState::Downloading {
            got: 0,
            total: release.zip_size,
            release: release.clone(),
        };
        self.jobs.ask(UpdateJob::Install(release));
    }

    pub fn stop(&mut self) {
        if self.installing() {
            self.jobs.ask(UpdateJob::Stop);
        }
    }

    /// The release whose news is due, once: a newer one than this build, found by a check.
    pub fn take_news(&mut self) -> Option<Version> {
        self.news_of.take()
    }

    fn release(&self) -> Option<Release> {
        match &self.state {
            UpdateState::Available(r)
            | UpdateState::Verifying(r)
            | UpdateState::Unpacking(r)
            | UpdateState::Downloading { release: r, .. } => Some(r.clone()),
            UpdateState::Failed { release, .. } => release.clone(),
            _ => None,
        }
    }

    pub fn tick(&mut self) {
        for news in self.jobs.news() {
            self.state = match news {
                UpdateNews::Checked(release) if release.version > self.installed => {
                    self.news_of = Some(release.version);
                    UpdateState::Available(release)
                }
                UpdateNews::Checked(_) => UpdateState::UpToDate,
                UpdateNews::Progress(got, total) => match self.release() {
                    Some(release) => UpdateState::Downloading {
                        got,
                        total,
                        release,
                    },
                    None => continue,
                },
                UpdateNews::Verifying => match self.release() {
                    Some(r) => UpdateState::Verifying(r),
                    None => continue,
                },
                UpdateNews::Unpacking => match self.release() {
                    Some(r) => UpdateState::Unpacking(r),
                    None => continue,
                },
                UpdateNews::Ready(version) => UpdateState::Ready(version),
                UpdateNews::Failed(Fail::Stopped) => match self.release() {
                    Some(r) => UpdateState::Available(r),
                    None => UpdateState::Unknown,
                },
                UpdateNews::Failed(fail) => UpdateState::Failed {
                    fail,
                    release: self.release(),
                },
            };
        }
    }
}

/// The updates for the handheld: a worker on a card, none in a host build's window.
pub fn update_jobs(root: &Path) -> Box<dyn UpdateJobs> {
    if cfg!(feature = "device") {
        Box::new(Worker::spawn(root.to_path_buf()))
    } else {
        Box::new(NoUpdates)
    }
}
