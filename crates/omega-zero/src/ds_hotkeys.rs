//! DSperate's hotkeys put back as they came: in each pad's settings, Config/dsperate/nds-<pad>.ini,
//! the [hotkeys] and [padhotkeys] sections are taken again from the copy the card ships,
//! System/dsperate/nds.ini, and every other section is left as the player set it.

use std::path::Path;

/// The sections that are DSperate's hotkeys.
const SECTIONS: [&str; 2] = ["hotkeys", "padhotkeys"];

/// Puts the hotkeys of every pad's settings on the card at `root` back as System/dsperate/nds.ini
/// has them, and says how many files it wrote.
pub fn reset(root: &Path) -> std::io::Result<usize> {
    let shipped = std::fs::read_to_string(root.join("System/dsperate/nds.ini"))?;
    let dir = root.join("Config/dsperate");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(0);
    };
    let mut written = 0;
    for path in entries.flatten().map(|e| e.path()) {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if !(name.starts_with("nds-") && name.ends_with(".ini")) {
            continue;
        }
        let own = std::fs::read_to_string(&path)?;
        let tmp = path.with_extension("ini.tmp");
        std::fs::write(&tmp, with_hotkeys_of(&own, &shipped))?;
        std::fs::rename(&tmp, &path)?;
        written += 1;
    }
    Ok(written)
}

/// `own` with its hotkey sections replaced by `shipped`'s, the rest as it was.
pub fn with_hotkeys_of(own: &str, shipped: &str) -> String {
    let mut out: String = lines(own, false).concat();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.extend(lines(shipped, true));
    out
}

/// The lines of `ini`, each with its newline, inside the hotkey sections if `hotkeys`, else
/// outside them.
fn lines(ini: &str, hotkeys: bool) -> Vec<&str> {
    let mut inside = false;
    ini.split_inclusive('\n')
        .filter(|line| {
            let t = line.trim();
            if let Some(name) = t.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
                inside = SECTIONS.contains(&name.trim());
            }
            inside == hotkeys
        })
        .collect()
}
