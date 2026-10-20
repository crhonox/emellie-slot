//! Handing the handheld over to an emulator of its own, for a system no libretro core here
//! plays: the DS, in DSperate.
//!
//! omega-zero writes what to run to the handoff file and leaves; `launch_frontend.sh` runs it,
//! then starts omega-zero again with [`RETURNED_ENV`] set, so the boot leaves the game
//! selected on its shelf rather than put it straight back in.

use std::path::{Path, PathBuf};

use omega_store::{atomic_write, Platform};

/// Where `launch_frontend.sh` looks for what to run once omega-zero has left.
pub const HANDOFF_ENV: &str = "OMEGA_HANDOFF";

/// Set by `launch_frontend.sh` on the start that follows a game played in its own emulator.
pub const RETURNED_ENV: &str = "OMEGA_RETURNED";

/// The handoff file under the card when `launch_frontend.sh` names none.
const HANDOFF_FILE: &str = "System/.handoff";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Handoff {
    pub platform: Platform,
    pub stem: String,
    pub rom: PathBuf,
    /// Started afresh, A held: the emulator's own resume is not taken up.
    pub clean: bool,
}

impl Handoff {
    /// What the handoff file says, one `key=value` a line: the system's folder name, the game's
    /// stem, the path of its ROM, and 1 when it starts afresh.
    pub fn text(&self) -> String {
        format!(
            "platform={}\nstem={}\nrom={}\nclean={}\n",
            self.platform.dir_name(),
            self.stem,
            self.rom.display(),
            u8::from(self.clean)
        )
    }

    /// Writes the handoff file for `launch_frontend.sh` to act on once omega-zero is gone.
    pub fn write(&self, root: &Path) -> std::io::Result<PathBuf> {
        let path = handoff_path(root);
        atomic_write(&path, self.text().as_bytes())?;
        Ok(path)
    }
}

pub fn handoff_path(root: &Path) -> PathBuf {
    std::env::var_os(HANDOFF_ENV)
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(HANDOFF_FILE))
}

/// Whether this start follows a game played in its own emulator.
pub fn returned() -> bool {
    std::env::var_os(RETURNED_ENV).is_some_and(|v| !v.is_empty() && v != "0")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_names_the_system_the_game_and_its_rom() {
        let h = Handoff {
            platform: Platform::Nds,
            stem: "Pokemon - Platinum Version (USA)".into(),
            rom: PathBuf::from("/mnt/sdcard/Games/NDS/Pokemon - Platinum Version (USA).nds"),
            clean: false,
        };
        assert_eq!(
            h.text(),
            "platform=NDS\nstem=Pokemon - Platinum Version (USA)\n\
             rom=/mnt/sdcard/Games/NDS/Pokemon - Platinum Version (USA).nds\nclean=0\n"
        );
        let clean = Handoff { clean: true, ..h };
        assert!(clean.text().ends_with("\nclean=1\n"));
    }
}
