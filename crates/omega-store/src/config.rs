use std::io;
use std::path::Path;

pub const CONFIG_DIR: &str = "Config";

const MOVED: [&str; 5] = [
    "omega-zero.state",
    "selected_core.ini",
    "video_mode.ini",
    "cart_shell.ini",
    "theme.txt",
];

/// The state's name before the frontend was Emellie Slot, which a card from then still has.
const OMEGA_STATE: &str = "omega-zero.state";

pub fn move_config(root: &Path) -> io::Result<()> {
    let config = root.join(CONFIG_DIR);
    std::fs::create_dir_all(&config)?;
    for name in MOVED {
        let old = root.join("System").join(name);
        let new = config.join(name);
        if old.is_file() && !new.exists() {
            std::fs::rename(&old, &new)?;
        }
    }
    // the state kept under omega-zero's name goes on under Emellie Slot's
    let old = config.join(OMEGA_STATE);
    let new = config.join(crate::app_state::STATE_FILE);
    if old.is_file() && !new.exists() {
        std::fs::rename(&old, &new)?;
    }
    Ok(())
}
