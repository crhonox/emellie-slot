use std::collections::HashMap;
use std::path::Path;

use crate::Platform;

pub const SELECTED_CORE_FILE: &str = "Config/selected_core.ini";

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Core {
    /// The GBA's default, in the place gpSP had.
    #[default]
    Armgba,
    Mgba,
    Snes9x,
    Supafaust,
    PcsxRearmed,
    Dsperate,
    Gambatte,
    Picodrive,
    GenesisPlusGx,
    Fceumm,
    Nestopia,
    /// The player's own PICO-8, handed over to `System/pico8.sh`.
    Pico8,
    /// What runs ports: PortMaster's scripts, handed over to `System/ports.sh`.
    PortMaster,
}

impl Core {
    pub const ALL: [Core; 13] = [
        Core::Armgba,
        Core::Mgba,
        Core::Snes9x,
        Core::Supafaust,
        Core::PcsxRearmed,
        Core::Dsperate,
        Core::Gambatte,
        Core::Picodrive,
        Core::GenesisPlusGx,
        Core::Fceumm,
        Core::Nestopia,
        Core::Pico8,
        Core::PortMaster,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Core::Mgba => "mgba",
            Core::Armgba => "armgba",
            Core::Snes9x => "snes9x",
            Core::Supafaust => "supafaust",
            Core::PcsxRearmed => "pcsx_rearmed",
            Core::Dsperate => "dsperate",
            Core::Gambatte => "gambatte",
            Core::Picodrive => "picodrive",
            Core::GenesisPlusGx => "genesis_plus_gx",
            Core::Fceumm => "fceumm",
            Core::Nestopia => "nestopia",
            Core::Pico8 => "pico8",
            Core::PortMaster => "portmaster",
        }
    }

    /// The libretro name its library is built under, `<name>_libretro.so`.
    pub fn library(&self) -> &'static str {
        match self {
            Core::Supafaust => "mednafen_supafaust",
            other => other.as_str(),
        }
    }

    /// Whether it is an emulator of its own that the handheld is handed over to, not a
    /// libretro core omega-zero loads.
    pub fn external(self) -> bool {
        matches!(self, Core::Dsperate | Core::Pico8 | Core::PortMaster)
    }

    pub fn index(self) -> usize {
        self as usize
    }

    /// The core's name as its chip in the core picker wears it, in its own case: as long as the
    /// chip has room for, six letters at most, so Supafaust is Faust.
    pub fn text(self) -> &'static str {
        match self {
            Core::Mgba => "mGBA",
            Core::Armgba => "arm",
            Core::Snes9x => "Snes9x",
            Core::Supafaust => "Faust",
            Core::PcsxRearmed => "PCSX",
            Core::Dsperate => "DSpr",
            Core::Gambatte => "Gamb",
            Core::Picodrive => "PicoDr",
            Core::GenesisPlusGx => "GPGX",
            Core::Fceumm => "FCEUmm",
            Core::Nestopia => "Nesto",
            Core::Pico8 => "PICO",
            Core::PortMaster => "Port",
        }
    }

    /// The core's whole name, in its own case, as About credits it.
    pub fn name(self) -> &'static str {
        match self {
            Core::Mgba => "mGBA",
            Core::Armgba => "armGBA",
            Core::Snes9x => "Snes9x",
            Core::Supafaust => "Supafaust",
            Core::PcsxRearmed => "PCSX-ReARMed",
            Core::Dsperate => "DSperate",
            Core::Gambatte => "Gambatte",
            Core::Picodrive => "PicoDrive",
            Core::GenesisPlusGx => "Genesis Plus GX",
            Core::Fceumm => "FCEUmm",
            Core::Nestopia => "Nestopia UE",
            Core::Pico8 => "PICO-8",
            Core::PortMaster => "PortMaster",
        }
    }

    pub fn parse(s: &str) -> Option<Core> {
        match s.trim().to_ascii_lowercase().as_str() {
            "mgba" => Some(Core::Mgba),
            // gpSP was the GBA's other core before armGBA took its place
            "armgba" | "gpsp" => Some(Core::Armgba),
            "snes9x" => Some(Core::Snes9x),
            "supafaust" => Some(Core::Supafaust),
            "pcsx_rearmed" => Some(Core::PcsxRearmed),
            "dsperate" => Some(Core::Dsperate),
            "gambatte" => Some(Core::Gambatte),
            "picodrive" => Some(Core::Picodrive),
            "genesis_plus_gx" => Some(Core::GenesisPlusGx),
            "fceumm" => Some(Core::Fceumm),
            "nestopia" => Some(Core::Nestopia),
            "pico8" => Some(Core::Pico8),
            "portmaster" => Some(Core::PortMaster),
            _ => None,
        }
    }

    pub fn runs(self, platform: Platform) -> bool {
        Core::for_platform(platform).contains(&self)
    }

    /// The cores that run `platform`, the default first, in the order the core picker's
    /// sockets show them.
    pub fn for_platform(platform: Platform) -> &'static [Core] {
        match platform {
            Platform::Gba => &[Core::Armgba, Core::Mgba],
            // Gambatte, in the palette it is set to; armGBA, which runs them on Gambatte too
            Platform::Gb | Platform::Gbc => &[Core::Gambatte, Core::Armgba],
            Platform::Nes => &[Core::Fceumm, Core::Nestopia],
            Platform::Snes => &[Core::Snes9x, Core::Supafaust],
            Platform::Md => &[Core::Picodrive, Core::GenesisPlusGx],
            Platform::Ps1 => &[Core::PcsxRearmed],
            Platform::Nds => &[Core::Dsperate],
            Platform::Pico8 => &[Core::Pico8],
            Platform::Ports => &[Core::PortMaster],
        }
    }

    pub fn default_for(platform: Platform) -> Core {
        Core::for_platform(platform)[0]
    }
}

pub fn read_selected_cores(root: &Path) -> HashMap<String, Core> {
    crate::ini::read(root, SELECTED_CORE_FILE)
        .into_iter()
        .filter_map(|(stem, name)| Core::parse(&name).map(|core| (stem, core)))
        .collect()
}

pub fn core_for(root: &Path, stem: &str) -> Core {
    crate::ini::value(root, SELECTED_CORE_FILE, stem)
        .as_deref()
        .and_then(Core::parse)
        .unwrap_or_default()
}

pub fn core_for_platform(root: &Path, stem: &str, platform: Platform) -> Core {
    crate::ini::value(root, SELECTED_CORE_FILE, stem)
        .as_deref()
        .and_then(Core::parse)
        .filter(|core| core.runs(platform))
        .unwrap_or_else(|| Core::default_for(platform))
}

pub fn write_selected_core(root: &Path, stem: &str, core: Core) -> std::io::Result<()> {
    crate::ini::write(root, SELECTED_CORE_FILE, stem, core.as_str())
}
