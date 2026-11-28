//! Screen prompts: for a game that keeps its progress in passwords, a box that offers to save a
//! state when its password screen comes up, and one that offers to load that state again when
//! its title screen does.
//!
//! Which game and which screen is read off the game's memory, as a RetroAchievements trigger
//! reads it, from rules on the card: `Config/screen-prompts.ini` first (the player's own), then
//! `System/screen-prompts.ini` (shipped). A rule is a section:
//!
//! ```ini
//! [Castlevania: Bloodlines]
//! system = MD
//! match = castlevania bloodlines | vampire killer
//! save = 0xH9002=7
//! load = 0xH9002=2
//! ```
//!
//! `match` is a part of the game's file name (case, spaces and punctuation aside), several
//! parted by `|`. `save` and `load` are conditions in RetroAchievements' memory syntax joined by
//! `_`, all of them to hold: `0xH` reads 8 bits, `0x` 16 and `0xX` 32 (little end first, as
//! RetroAchievements reads them), compared with `=`, `!=`, `<`, `<=`, `>` or `>=` to a number in
//! decimal or in hex (`0x1f` or `h1f`). A key written again is another way for that screen to
//! show. The addresses are RetroAchievements', the same its code notes give.
//!
//! `trace` lists addresses (`0xH9002 0xH9004`) whose every change is written to the log with
//! the frame it came on, to find the values a screen shows with.
//!
//! The emulator thread reads them after every frame; a screen counts once it has held for
//! [`HOLD`] frames, and asks once each time it comes up. A state loaded or the game resumed
//! does not ask about the screen it lands on.

use std::path::{Path, PathBuf};

use omega_cheevos::memory::Ram;
use omega_retro::RetroCore;
use omega_store::Platform;

/// Where the player's rules are, and where the shipped ones are.
pub const OWN_RULES: &str = "Config/screen-prompts.ini";
pub const SHIPPED_RULES: &str = "System/screen-prompts.ini";

/// How many frames in a row a screen is to show before it asks: a moment, so a value the game
/// passes through on its way elsewhere does not.
pub const HOLD: u32 = 20;

/// What a screen asks the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    /// The password screen: save a state here?
    Save = 1,
    /// The title screen: load the state saved at the password screen?
    Load = 2,
}

impl Ask {
    pub fn from_u8(v: u8) -> Option<Ask> {
        match v {
            1 => Some(Ask::Save),
            2 => Some(Ask::Load),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    Bits8,
    Bits16,
    Bits32,
}

impl Size {
    fn bytes(self) -> usize {
        match self {
            Size::Bits8 => 1,
            Size::Bits16 => 2,
            Size::Bits32 => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// One comparison of the game's memory with a number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cond {
    address: u32,
    size: Size,
    op: Op,
    value: u32,
}

impl Cond {
    /// `text` as a condition, as `0xH9002=7`; None for anything it cannot read.
    pub fn parse(text: &str) -> Option<Cond> {
        let text = text.trim();
        let (left, op, right) = [
            ("!=", Op::Ne),
            ("<=", Op::Le),
            (">=", Op::Ge),
            ("=", Op::Eq),
            ("<", Op::Lt),
            (">", Op::Gt),
        ]
        .into_iter()
        .find_map(|(s, op)| text.split_once(s).map(|(l, r)| (l, op, r)))?;
        let (address, size) = memory_ref(left)?;
        Some(Cond {
            address,
            size,
            op,
            value: number(right.trim())?,
        })
    }

    fn holds(&self, read: &dyn Fn(u32, &mut [u8]) -> u32) -> bool {
        let Some(v) = read_value(read, self.address, self.size) else {
            return false;
        };
        match self.op {
            Op::Eq => v == self.value,
            Op::Ne => v != self.value,
            Op::Lt => v < self.value,
            Op::Le => v <= self.value,
            Op::Gt => v > self.value,
            Op::Ge => v >= self.value,
        }
    }
}

/// A memory reference as RetroAchievements writes it, `0xH9002`: its address and size.
fn memory_ref(text: &str) -> Option<(u32, Size)> {
    let text = text.trim();
    let rest = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))?;
    let (size, digits) = match rest.chars().next()? {
        'H' | 'h' => (Size::Bits8, &rest[1..]),
        'X' | 'x' => (Size::Bits32, &rest[1..]),
        ' ' => (Size::Bits16, &rest[1..]),
        c if c.is_ascii_hexdigit() => (Size::Bits16, rest),
        _ => return None,
    };
    Some((u32::from_str_radix(digits.trim(), 16).ok()?, size))
}

/// The value at `address`, `size` wide, little end first; None where it cannot be read.
fn read_value(read: &dyn Fn(u32, &mut [u8]) -> u32, address: u32, size: Size) -> Option<u32> {
    let mut buf = [0u8; 4];
    let n = size.bytes();
    (read(address, &mut buf[..n]) as usize == n).then(|| u32::from_le_bytes(buf))
}

fn number(text: &str) -> Option<u32> {
    if let Some(hex) = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .or_else(|| text.strip_prefix('h'))
        .or_else(|| text.strip_prefix('H'))
    {
        return u32::from_str_radix(hex, 16).ok();
    }
    text.parse().ok()
}

/// Conditions that are all to hold, written as one line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct All(Vec<Cond>);

impl All {
    pub fn parse(text: &str) -> Option<All> {
        let conds: Option<Vec<Cond>> = text.split('_').map(Cond::parse).collect();
        conds.filter(|c| !c.is_empty()).map(All)
    }

    fn holds(&self, read: &dyn Fn(u32, &mut [u8]) -> u32) -> bool {
        self.0.iter().all(|c| c.holds(read))
    }
}

/// One game's rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    pub name: String,
    pub system: Option<Platform>,
    pub names: Vec<String>,
    pub save: Vec<All>,
    pub load: Vec<All>,
    /// Addresses whose every change is written to the log, to find a screen's values.
    pub trace: Vec<(u32, Size)>,
}

impl Rule {
    fn new(name: &str) -> Rule {
        Rule {
            name: name.to_string(),
            system: None,
            names: Vec::new(),
            save: Vec::new(),
            load: Vec::new(),
            trace: Vec::new(),
        }
    }

    /// Whether the rule is for the game `stem` of `platform`.
    pub fn fits(&self, platform: Platform, stem: &str) -> bool {
        if self.system.is_some_and(|s| s != platform) {
            return false;
        }
        let stem = plain(stem);
        self.names.iter().any(|n| stem.contains(n.as_str()))
    }
}

/// `text` in small letters, every run of anything else than a letter or digit one space.
fn plain(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() {
            out.push(c);
        } else if !out.ends_with(' ') {
            out.push(' ');
        }
    }
    format!(" {} ", out.trim())
}

/// The rules in `text`; a line it cannot read is said and left out.
pub fn parse_rules(text: &str) -> Vec<Rule> {
    let mut rules: Vec<Rule> = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            rules.push(Rule::new(name.trim()));
            continue;
        }
        let (Some(rule), Some((key, value))) = (rules.last_mut(), line.split_once('=')) else {
            eprintln!(
                "omega-zero: screen prompts: line {} left out: {line}",
                i + 1
            );
            continue;
        };
        let value = value.trim();
        let ok = match key.trim().to_ascii_lowercase().as_str() {
            "system" => Platform::ALL
                .into_iter()
                .find(|p| p.dir_name().eq_ignore_ascii_case(value))
                .map(|p| rule.system = Some(p))
                .is_some(),
            "match" => {
                rule.names
                    .extend(value.split('|').map(plain).filter(|n| !n.trim().is_empty()));
                true
            }
            "save" => All::parse(value).map(|a| rule.save.push(a)).is_some(),
            "load" => All::parse(value).map(|a| rule.load.push(a)).is_some(),
            "trace" => {
                let refs: Option<Vec<_>> = value
                    .split([' ', ',', '_'])
                    .filter(|t| !t.is_empty())
                    .map(memory_ref)
                    .collect();
                refs.filter(|r| !r.is_empty())
                    .map(|r| rule.trace.extend(r))
                    .is_some()
            }
            _ => false,
        };
        if !ok {
            eprintln!(
                "omega-zero: screen prompts: line {} left out: {line}",
                i + 1
            );
        }
    }
    rules.retain(|r| {
        !r.names.is_empty() && (!r.save.is_empty() || !r.load.is_empty() || !r.trace.is_empty())
    });
    rules
}

/// The rule for the game `stem` of `platform`, from the player's rules first, then the shipped.
pub fn rule_for(root: &Path, platform: Platform, stem: &str) -> Option<Rule> {
    [OWN_RULES, SHIPPED_RULES].iter().find_map(|file| {
        let text = std::fs::read_to_string(root.join(file)).ok()?;
        parse_rules(&text)
            .into_iter()
            .find(|r| r.fits(platform, stem))
    })
}

/// Where the stamp of the state the password screen saved is noted, in the game's state folder.
pub fn note_path(states: &Path) -> PathBuf {
    states.join("screen-prompt.txt")
}

/// One screen's watch: how long it has shown, and whether it has asked this time it is up.
#[derive(Clone, Copy, Debug, Default)]
struct Edge {
    run: u32,
    asked: bool,
}

impl Edge {
    /// The screen shows (`on`) or not this frame; true the frame it has held for [`HOLD`].
    fn step(&mut self, on: bool, quiet: bool) -> bool {
        if !on {
            *self = Edge::default();
            return false;
        }
        self.run = self.run.saturating_add(1);
        if quiet {
            self.asked = true;
        }
        if self.asked || self.run < HOLD {
            return false;
        }
        self.asked = true;
        true
    }
}

/// A game's screens watched on the emulator thread.
pub struct Watch {
    rule: Rule,
    console: u32,
    ram: Option<Ram>,
    save: Edge,
    load: Edge,
    /// The next frame read sees where the game is without asking: just loaded or resumed.
    quiet: bool,
    /// The frames read, and the traced addresses' values last read.
    frames: u64,
    traced: Vec<Option<u32>>,
}

impl Watch {
    pub fn new(rule: Rule, console: u32) -> Watch {
        Watch {
            rule,
            console,
            ram: None,
            save: Edge::default(),
            load: Edge::default(),
            quiet: true,
            frames: 0,
            traced: Vec::new(),
        }
    }

    /// A state was loaded: the screen it lands on does not ask.
    pub fn quiet(&mut self) {
        self.quiet = true;
        self.save = Edge::default();
        self.load = Edge::default();
    }

    /// Reads the game after the frame it ran, and what a screen asks, if one has just come up.
    pub fn frame(&mut self, core: &dyn RetroCore) -> Option<Ask> {
        let console = self.console;
        let mut ram = self.ram.take().unwrap_or_else(|| Ram::new(console));
        let asked = ram
            .refresh(core.memory_map(), core.core_memory())
            .then(|| self.step(&|address: u32, buf: &mut [u8]| ram.read(address, buf)))
            .flatten();
        self.ram = Some(ram);
        asked
    }

    fn step(&mut self, read: &dyn Fn(u32, &mut [u8]) -> u32) -> Option<Ask> {
        self.frames += 1;
        for line in self.trace(read) {
            eprintln!("omega-zero: screen trace: {line}");
        }
        let quiet = std::mem::take(&mut self.quiet);
        let save = self.rule.save.iter().any(|a| a.holds(read));
        let load = self.rule.load.iter().any(|a| a.holds(read));
        let asked_save = self.save.step(save, quiet);
        let asked_load = self.load.step(load, quiet);
        if asked_save || asked_load {
            eprintln!(
                "omega-zero: screen prompts: frame {} asks {}",
                self.frames,
                if asked_save { "save" } else { "load" }
            );
        }
        if asked_save {
            Some(Ask::Save)
        } else if asked_load {
            Some(Ask::Load)
        } else {
            None
        }
    }

    /// A line for each traced address whose value changed since the frame before, and for
    /// each on the first frame read.
    fn trace(&mut self, read: &dyn Fn(u32, &mut [u8]) -> u32) -> Vec<String> {
        self.traced.resize(self.rule.trace.len(), None);
        let mut lines = Vec::new();
        for (i, &(address, size)) in self.rule.trace.iter().enumerate() {
            let now = read_value(read, address, size);
            let was = self.traced[i];
            if now == was {
                continue;
            }
            self.traced[i] = now;
            let width = size.bytes() * 2;
            let show = |v: Option<u32>| match v {
                Some(v) => format!("0x{v:0width$x} ({v})"),
                None => "unreadable".to_string(),
            };
            let line = match was {
                Some(_) => format!(
                    "frame {} 0x{address:04x} = {} (was {})",
                    self.frames,
                    show(now),
                    show(was)
                ),
                None => format!("frame {} 0x{address:04x} = {}", self.frames, show(now)),
            };
            lines.push(line);
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    const BLOODLINES: &str = "\
# Castlevania: Bloodlines
[Castlevania: Bloodlines]
system = MD
match = castlevania bloodlines | Vampire Killer
save = 0xH9002=7
load = 0xH9002=2
";

    #[test]
    fn conditions_read_as_retroachievements_writes_them() {
        let c = Cond::parse("0xH9002=7").unwrap();
        assert_eq!(
            (c.address, c.size, c.op, c.value),
            (0x9002, Size::Bits8, Op::Eq, 7)
        );
        let c = Cond::parse("0x 00fb2e!=h1F").unwrap();
        assert_eq!(
            (c.address, c.size, c.op, c.value),
            (0xfb2e, Size::Bits16, Op::Ne, 0x1f)
        );
        let c = Cond::parse("0xXfb06>=0x100").unwrap();
        assert_eq!((c.size, c.op, c.value), (Size::Bits32, Op::Ge, 0x100));
        let c = Cond::parse("0x9002<3").unwrap();
        assert_eq!((c.size, c.op), (Size::Bits16, Op::Lt));
        assert!(Cond::parse("0xZ9002=7").is_none());
        assert!(Cond::parse("9002=7").is_none());
        assert!(Cond::parse("0xH9002").is_none());
        assert_eq!(All::parse("0xH9002=9_0xHfb2e=0").unwrap().0.len(), 2);
    }

    #[test]
    fn a_rule_fits_its_game_by_name_and_system() {
        let rules = parse_rules(BLOODLINES);
        assert_eq!(rules.len(), 1);
        let r = &rules[0];
        assert!(r.fits(Platform::Md, "Castlevania - Bloodlines (USA)"));
        assert!(r.fits(Platform::Md, "Castlevania_Bloodlines"));
        assert!(r.fits(
            Platform::Md,
            "Castlevania - The New Generation [Vampire Killer] (Europe)"
        ));
        assert!(!r.fits(Platform::Snes, "Castlevania - Bloodlines (USA)"));
        assert!(!r.fits(Platform::Md, "Castlevania Bloodlinesx"));
        assert!(!r.fits(Platform::Md, "Sonic the Hedgehog (USA)"));
    }

    #[test]
    fn a_rule_with_no_name_or_no_screen_is_left_out() {
        assert!(parse_rules("[A]\nsave = 0xH1=1\n").is_empty());
        assert!(parse_rules("[A]\nmatch = a\n").is_empty());
        assert!(parse_rules("save = 0xH1=1\n").is_empty());
    }

    fn watch() -> Watch {
        Watch::new(
            parse_rules(BLOODLINES).remove(0),
            omega_cheevos::game::MEGA_DRIVE,
        )
    }

    fn run(w: &mut Watch, screen: &Cell<u8>, value: u8, frames: u32) -> Vec<Ask> {
        screen.set(value);
        let read = |address: u32, buf: &mut [u8]| {
            buf.fill(0);
            if address == 0x9002 {
                buf[0] = screen.get();
            }
            buf.len() as u32
        };
        (0..frames).filter_map(|_| w.step(&read)).collect()
    }

    #[test]
    fn a_screen_asks_once_it_has_held_and_once_each_time_it_comes_up() {
        let mut w = watch();
        let screen = Cell::new(0);
        assert!(run(&mut w, &screen, 0, 5).is_empty());
        assert!(run(&mut w, &screen, 2, HOLD - 1).is_empty());
        assert_eq!(run(&mut w, &screen, 2, 100), vec![Ask::Load]);
        assert!(run(&mut w, &screen, 9, 100).is_empty());
        assert_eq!(run(&mut w, &screen, 7, 100), vec![Ask::Save]);
        assert!(run(&mut w, &screen, 9, 3).is_empty());
        assert_eq!(run(&mut w, &screen, 7, 100), vec![Ask::Save]);
    }

    #[test]
    fn a_screen_passed_through_does_not_ask() {
        let mut w = watch();
        let screen = Cell::new(0);
        for _ in 0..10 {
            assert!(run(&mut w, &screen, 7, HOLD - 1).is_empty());
            assert!(run(&mut w, &screen, 9, 1).is_empty());
        }
    }

    #[test]
    fn the_screen_a_load_or_a_resume_lands_on_does_not_ask() {
        let mut w = watch();
        let screen = Cell::new(0);
        assert!(run(&mut w, &screen, 7, 100).is_empty());
        assert!(run(&mut w, &screen, 9, 10).is_empty());
        assert_eq!(run(&mut w, &screen, 7, 100), vec![Ask::Save]);
        w.quiet();
        assert!(run(&mut w, &screen, 7, 100).is_empty());
        assert!(run(&mut w, &screen, 2, 100) == vec![Ask::Load]);
    }

    #[test]
    fn a_traced_address_says_each_change_once() {
        let mut rule = parse_rules("[T]\nmatch = t\ntrace = 0xH9002, 0x9004\n").remove(0);
        assert_eq!(
            rule.trace,
            vec![(0x9002, Size::Bits8), (0x9004, Size::Bits16)]
        );
        rule.save.clear();
        let mut w = Watch::new(rule, omega_cheevos::game::MEGA_DRIVE);
        let screen = Cell::new(9u8);
        let read = |address: u32, buf: &mut [u8]| {
            buf.fill(0);
            if address == 0x9002 {
                buf[0] = screen.get();
            }
            buf.len() as u32
        };
        w.frames = 1;
        let first = w.trace(&read);
        assert_eq!(first.len(), 2);
        assert!(first[0].contains("0x9002 = 0x09 (9)"), "{first:?}");
        assert!(w.trace(&read).is_empty());
        screen.set(8);
        assert_eq!(
            w.trace(&read),
            vec!["frame 1 0x9002 = 0x08 (8) (was 0x09 (9))"]
        );
    }

    #[test]
    fn the_players_rules_come_before_the_shipped() {
        let dir = std::env::temp_dir().join(format!("screen-prompts-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("Config")).unwrap();
        std::fs::create_dir_all(dir.join("System")).unwrap();
        std::fs::write(dir.join(SHIPPED_RULES), BLOODLINES).unwrap();
        let stem = "Castlevania - Bloodlines (USA)";
        let shipped = rule_for(&dir, Platform::Md, stem).unwrap();
        assert_eq!(shipped.save, vec![All::parse("0xH9002=7").unwrap()]);
        std::fs::write(
            dir.join(OWN_RULES),
            "[Mine]\nmatch = bloodlines\nsave = 0xH9002=8\n",
        )
        .unwrap();
        let own = rule_for(&dir, Platform::Md, stem).unwrap();
        assert_eq!(own.name, "Mine");
        assert!(rule_for(&dir, Platform::Md, "Sonic").is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
