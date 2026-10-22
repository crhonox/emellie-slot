use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Platform {
    #[default]
    Gba,
    Gb,
    Gbc,
    Snes,
    Ps1,
    Nds,
    /// Ports from PortMaster: their scripts, run by `System/ports.sh`.
    Ports,
}

impl Platform {
    pub const ALL: [Platform; 7] = [
        Platform::Gba,
        Platform::Gb,
        Platform::Gbc,
        Platform::Snes,
        Platform::Ps1,
        Platform::Nds,
        Platform::Ports,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Platform::Gba => "Game Boy Advance",
            Platform::Gb => "Game Boy",
            Platform::Gbc => "Game Boy Color",
            Platform::Snes => "Super Nintendo",
            Platform::Ps1 => "PlayStation",
            Platform::Nds => "Nintendo DS",
            Platform::Ports => "Ports",
        }
    }

    pub fn dir_name(self) -> &'static str {
        match self {
            Platform::Gba => "GBA",
            Platform::Gb => "GB",
            Platform::Gbc => "GBC",
            Platform::Snes => "SNES",
            Platform::Ps1 => "PS1",
            Platform::Nds => "NDS",
            Platform::Ports => "PORTS",
        }
    }

    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Platform::Gba => &["gba"],
            Platform::Gb | Platform::Gbc => &["gb", "gbc"],
            Platform::Snes => &["sfc", "smc"],
            Platform::Ps1 => &["cue", "chd", "pbp", "m3u"],
            Platform::Nds => &["nds"],
            Platform::Ports => &["sh"],
        }
    }

    pub fn picture(self) -> (u32, u32) {
        match self {
            Platform::Gba => (240, 160),
            Platform::Gb | Platform::Gbc => (160, 144),
            Platform::Snes => (256, 224),
            Platform::Ps1 => (320, 240),
            // both screens, stacked
            Platform::Nds => (256, 384),
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
        matches!(self, Platform::Ps1 | Platform::Nds | Platform::Ports)
    }

    /// Whether its games run in an emulator of their own, which omega-zero hands the
    /// handheld over to and is started again after, rather than in a libretro core.
    pub fn external(self) -> bool {
        matches!(self, Platform::Nds | Platform::Ports)
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
