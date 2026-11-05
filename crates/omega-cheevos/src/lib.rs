mod client;
mod ffi;

pub mod badge;
pub mod config;
pub mod game;
pub mod memory;
pub mod notice;
pub mod offline;
pub mod transport;

pub use client::{Cheevos, Login};
pub use config::{Credentials, Secret, CHEEVOS_FILE};
pub use notice::Notice;
pub use offline::{dorequest_url, Cached, Offline, Sent};
pub use transport::{Http, Request, Response, Transport};

/// The stack each RetroAchievements thread is given, room for a TLS handshake and a disc's
/// hash: the default 8 MB each, reserved near a core loaded after them, left gpsp no room
/// for its JIT cache within the 128 MB its branches reach.
pub const THREAD_STACK: usize = 512 * 1024;
