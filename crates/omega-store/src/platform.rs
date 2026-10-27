use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Platform {
    #[default]
    Gba,
    Gb,
    Gbc,
    /// The NES and the Famicom, in Games/FC as spruceOS keeps them.
    Nes,
    Snes,
    /// The Mega Drive and the Genesis, in Games/MD as spruceOS keeps them.
    Md,
    Ps1,
    Nds,
    /// PICO-8 carts, run by the player's own PICO-8 (`System/pico8.sh`), which needs the
    /// Raspberry Pi build's files in BIOS/.
    Pico8,
    /// Ports from PortMaster: their scripts, run by `System/ports.sh`.
    Ports,
}

impl Platform {
    pub const ALL: [Platform; 10] = [
        Platform::Gba,
        Platform::Gb,
        Platform::Gbc,
        Platform::Nes,
        Platform::Snes,
        Platform::Md,
        Platform::Ps1,
        Platform::Nds,
        Platform::Pico8,
        Platform::Ports,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Platform::Gba => "Game Boy Advance",
            Platform::Gb => "Game Boy",
            Platform::Gbc => "Game Boy Color",
            Platform::Nes => "Nintendo",
            Platform::Snes => "Super Nintendo",
            Platform::Md => "Mega Drive",
            Platform::Ps1 => "PlayStation",
            Platform::Nds => "Nintendo DS",
            Platform::Pico8 => "PICO-8",
            Platform::Ports => "Ports",
        }
    }

    pub fn dir_name(self) -> &'static str {
        match self {
            Platform::Gba => "GBA",
            Platform::Gb => "GB",
            Platform::Gbc => "GBC",
            Platform::Nes => "FC",
            Platform::Snes => "SNES",
            Platform::Md => "MD",
            Platform::Ps1 => "PS1",
            Platform::Nds => "NDS",
            Platform::Pico8 => "PICO8",
            Platform::Ports => "PORTS",
        }
    }

    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Platform::Gba => &["gba"],
            Platform::Gb | Platform::Gbc => &["gb", "gbc"],
            Platform::Nes => &["nes", "fds", "unf", "unif"],
            Platform::Snes => &["sfc", "smc"],
            Platform::Md => &["md", "gen", "smd", "bin"],
            Platform::Ps1 => &["cue", "chd", "pbp", "m3u"],
            Platform::Nds => &["nds"],
            // a cart as text or as its picture, cart.p8.png; Splore.splore is PICO-8's own browser
            Platform::Pico8 => &["p8", "png", "splore"],
            Platform::Ports => &["sh"],
        }
    }

    pub fn picture(self) -> (u32, u32) {
        match self {
            Platform::Gba => (240, 160),
            Platform::Gb | Platform::Gbc => (160, 144),
            Platform::Nes => (256, 240),
            Platform::Snes => (256, 224),
            Platform::Md => (320, 224),
            Platform::Ps1 => (320, 240),
            // both screens, stacked
            Platform::Nds => (256, 384),
            Platform::Pico8 => (128, 128),
            // each port draws to the whole panel itself
            Platform::Ports => (640, 480),
        }
    }

    /// Whether the frontend keeps L1/R1 while a game of this system plays, for its picture
    /// size: none does, the Game Boys' L1/R1 are their A and B.
    pub fn owns_shoulders(self) -> bool {
        false
    }

    /// Whether the core's colour correction for a handheld's screen applies.
    pub fn colour_correction(self) -> bool {
        matches!(self, Platform::Gba | Platform::Gb | Platform::Gbc)
    }

    /// Whether held L2 rewinds: on none, Emellie Slot plays without rewind.
    pub fn rewinds(self) -> bool {
        false
    }

    /// Whether L2/R2 belong to the game rather than to rewind and fast-forward.
    pub fn game_owns_l2_r2(self) -> bool {
        matches!(
            self,
            Platform::Ps1 | Platform::Nds | Platform::Pico8 | Platform::Ports
        )
    }

    /// Whether its games run in an emulator of their own, which omega-zero hands the
    /// handheld over to and is started again after, rather than in a libretro core.
    pub fn external(self) -> bool {
        matches!(self, Platform::Nds | Platform::Pico8 | Platform::Ports)
    }

    /// Whether its labels are looked up on ScreenScraper: a port's and a PICO-8 cart's are not.
    pub fn scraped(self) -> bool {
        !matches!(self, Platform::Pico8 | Platform::Ports)
    }

    pub fn accepts(self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| {
                self.extensions()
                    .iter()
                    .any(|k| ext.eq_ignore_ascii_case(k))
            })
    }
}
