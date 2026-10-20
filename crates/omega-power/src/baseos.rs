use std::path::Path;

pub const BASEOS_RELEASE: &str = "/etc/baseos-release";

/// What BaseOS says about the handheld it was built for, from /etc/baseos-release. Each
/// BaseOS image is built for one model, so this names it exactly.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BaseosRelease {
    /// The build target, such as `rg35xxsp` or `rgsp`.
    pub target: Option<String>,
    /// The model's name, such as `RG35XX SP`.
    pub model: Option<String>,
    pub version: Option<String>,
    /// How far the picture is turned counter-clockwise to stand upright on the panel.
    pub rotation_ccw: u32,
    pub wifi: Option<bool>,
    pub bluetooth: Option<bool>,
}

pub fn parse_baseos_release(text: &str) -> BaseosRelease {
    let mut r = BaseosRelease::default();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"').trim();
        let text = (!value.is_empty()).then(|| value.to_string());
        let flag = || match value.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" => Some(true),
            "0" | "false" | "no" => Some(false),
            _ => None,
        };
        match key.trim() {
            "BASEOS_TARGET" => r.target = text,
            "BASEOS_MODEL" => r.model = text,
            "BASEOS_VERSION" => r.version = text,
            "BASEOS_PANEL_ROTATION_CCW" => r.rotation_ccw = value.parse().unwrap_or(0) % 360,
            "BASEOS_WIFI" => r.wifi = flag(),
            "BASEOS_BLUETOOTH" => r.bluetooth = flag(),
            _ => {}
        }
    }
    r
}

/// The sticks a handheld's pad has, which decide how a DS game's stylus is worked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PadLayout {
    /// The RG35XX SP and Plus, RG34XX, RG SP and RG28XX.
    NoStick,
    /// The RG40XX V, a left stick alone.
    OneStick,
    /// Everything else, the RG CubeXX among them, and a handheld BaseOS does not name.
    #[default]
    TwoSticks,
}

impl PadLayout {
    /// The layout of BaseOS's build target, as spruceOS sorts its H700 models.
    /// card/System/nds.sh holds the same list, for the pad map and the settings it picks.
    pub fn of_target(target: Option<&str>) -> PadLayout {
        match target.map(str::trim) {
            Some("rg35xxsp" | "rg35xxplus" | "rg34xx" | "rgsp" | "rg28xx") => PadLayout::NoStick,
            Some("rg40xxv") => PadLayout::OneStick,
            _ => PadLayout::TwoSticks,
        }
    }

    /// The running handheld's, read once.
    pub fn current() -> PadLayout {
        static LAYOUT: std::sync::OnceLock<PadLayout> = std::sync::OnceLock::new();
        *LAYOUT.get_or_init(|| {
            PadLayout::of_target(baseos_release().as_ref().and_then(|r| r.target.as_deref()))
        })
    }
}

/// The running BaseOS's description of its handheld, if this is BaseOS.
pub fn baseos_release() -> Option<BaseosRelease> {
    read_baseos_release(Path::new(BASEOS_RELEASE))
}

pub fn read_baseos_release(path: &Path) -> Option<BaseosRelease> {
    std::fs::read_to_string(path)
        .ok()
        .map(|t| parse_baseos_release(&t))
}

#[cfg(test)]
mod tests {
    use super::PadLayout;

    #[test]
    fn each_model_has_the_sticks_it_is_built_with() {
        for target in ["rg35xxsp", "rg35xxplus", "rg34xx", "rgsp", "rg28xx"] {
            assert_eq!(
                PadLayout::of_target(Some(target)),
                PadLayout::NoStick,
                "{target}"
            );
        }
        assert_eq!(PadLayout::of_target(Some("rg40xxv")), PadLayout::OneStick);
        for target in ["rgcubexx", "rg34xxsp", "rg35xxh", "rg40xxh"] {
            assert_eq!(
                PadLayout::of_target(Some(target)),
                PadLayout::TwoSticks,
                "{target}"
            );
        }
        assert_eq!(PadLayout::of_target(None), PadLayout::TwoSticks);
    }
}
