//! RetroAchievements for the games omega-zero runs itself, every system but the DS, whose
//! emulator keeps its own: the account signed in once Wi-Fi is up, each game identified by
//! its hash, its memory read through the core every frame, and a banner over the picture for
//! what happens. Softcore only: save states, rewind and fast-forward are left as they are.

pub mod boot;
mod live;
pub mod overlay;

pub use live::*;
