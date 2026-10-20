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
}

impl Platform {
    pub const ALL: [Platform; 6] = [
        Platform::Gba,
        Platform::Gb,
        Platform::Gbc,
        Platform::Snes,
        Platform::Ps1,
        Platform::Nds,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Platform::Gba => "Game Boy Advance",
            Platform::Gb => "Game Boy",
            Platform::Gbc => "Game Boy Color",
            Platform::Snes => "Super Nintendo",
            Platform::Ps1 => "PlayStation",
            Platform::Nds => "Nintendo DS",
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
        }
    }

    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Platform::Gba => &["gba"],
            Platform::Gb | Platform::Gbc => &["gb", "gbc"],
            Platform::Snes => &["sfc", "smc"],
            Platform::Ps1 => &["cue", "chd", "pbp", "m3u"],
            Platform::Nds => &["nds"],
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
        }
    }

    /// Whether the frontend keeps L1/R1 while a game of this system plays, for its picture
    /// size: only the Game Boys, whose games have no shoulder buttons.
    pub fn owns_shoulders(self) -> bool {
        matches!(self, Platform::Gb | Platform::Gbc)
    }

    /// Whether held X/Y are turbo A/B: on the handhelds, whose games have no X or Y.
    pub fn turbo_xy(self) -> bool {
        matches!(self, Platform::Gba | Platform::Gb | Platform::Gbc)
    }

    /// Whether the core's colour correction for a handheld's screen applies.
    pub fn colour_correction(self) -> bool {
        matches!(self, Platform::Gba | Platform::Gb | Platform::Gbc)
    }

    /// Whether held L2 rewinds: not on the PlayStation, whose games use L2 and whose states
    /// are too big for the rewind buffer.
    pub fn rewinds(self) -> bool {
        !matches!(self, Platform::Ps1 | Platform::Nds)
    }

    /// Whether L2/R2 belong to the game rather than to rewind and fast-forward.
    pub fn game_owns_l2_r2(self) -> bool {
        matches!(self, Platform::Ps1 | Platform::Nds)
    }

    /// Whether its games run in an emulator of their own, which omega-zero hands the
    /// handheld over to and is started again after, rather than in a libretro core.
    pub fn external(self) -> bool {
        self == Platform::Nds
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
