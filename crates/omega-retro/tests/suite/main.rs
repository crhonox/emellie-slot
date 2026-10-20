//! omega-retro's tests, built as one binary. Built one per file, linking and starting them took
//! longer than running them. A test that needs a process to itself stays a file of its own in
//! tests/, beside this folder.

use std::sync::Mutex;

/// A process holds one live libretro core at a time, so a test takes this before it loads one.
pub static CORE_LOCK: Mutex<()> = Mutex::new(());

mod console_cores;
mod libretro;
mod mock;
mod netpacket;
mod options;
mod remap;
mod rumble;

/// A file in this folder that the list above leaves out is never compiled, let alone run.
#[test]
fn every_file_in_the_folder_is_in_the_suite() {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/suite");
    let listed = include_str!("main.rs");
    for entry in std::fs::read_dir(&folder).expect("the suite's folder") {
        let path = entry.expect("a file in the suite's folder").path();
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if path.extension().is_some_and(|e| e == "rs") && stem != "main" {
            assert!(
                listed.contains(&format!("\nmod {stem};")),
                "tests/suite/{stem}.rs is missing from tests/suite/main.rs"
            );
        }
    }
}
