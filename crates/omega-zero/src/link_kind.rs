#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LinkKind {
    Cable,
    Wireless,
}

impl LinkKind {
    pub fn other(self) -> LinkKind {
        match self {
            LinkKind::Cable => LinkKind::Wireless,
            LinkKind::Wireless => LinkKind::Cable,
        }
    }
}

const WIRELESS: [&str; 43] = [
    "B2WE", "B3AE", "B4UE", "B4UP", "B85A", "B85P", "BDGE", "BDGP", "BG3E", "BKRJ", "BMGD", "BMGE",
    "BMGF", "BMGI", "BMGJ", "BMGP", "BMGS", "BMGU", "BPED", "BPEE", "BPEF", "BPEI", "BPEJ", "BPES",
    "BPGD", "BPGE", "BPGF", "BPGI", "BPGJ", "BPGS", "BPRD", "BPRE", "BPRF", "BPRI", "BPRJ", "BPRS",
    "BR5E", "BR6E", "BRBE", "BRKE", "BTME", "BTMJ", "BTMP",
];

pub fn link_kind(code: &str, title: &str, clean: bool) -> LinkKind {
    if pokemon(code, title) {
        let retail = clean
            && WIRELESS.contains(&code)
            && ["POKEMON FIRE", "POKEMON LEAF", "POKEMON EMER"].contains(&title);
        return if retail {
            LinkKind::Wireless
        } else {
            LinkKind::Cable
        };
    }
    if WIRELESS.contains(&code) {
        LinkKind::Wireless
    } else {
        LinkKind::Cable
    }
}

/// What armGBA's GBA Link Port is set to for `chosen`: the Link Cable, or the Wireless Adapter.
pub fn armgba_port(chosen: LinkKind) -> &'static str {
    match chosen {
        LinkKind::Cable => "cable",
        LinkKind::Wireless => "wireless",
    }
}

/// Whether a game links both ways, on the cable and on the Wireless Adapter, so the link screen
/// lets the player pick: Pokemon's, whose newer games have both. Any other links one way only.
pub fn both_ports(code: &str, title: &str) -> bool {
    pokemon(code, title)
}

fn pokemon(code: &str, title: &str) -> bool {
    title.starts_with("POKEMON")
        || ["AXV", "AXP", "BPE", "BPR", "BPG"]
            .iter()
            .any(|p| code.starts_with(p))
}
