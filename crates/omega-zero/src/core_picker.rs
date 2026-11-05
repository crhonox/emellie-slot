//! The core picker: the cart's lid lifted off its board, and the chip hopping between the
//! board's sockets, one for each core that runs the cart's system, in the order the system lists
//! them (`Core::for_platform`). Left and Right move the chip a socket that way; A keeps the core
//! it is in, or heading for, and B leaves the cart as it was.
//!
//! A board has two sockets. A system with more cores (the GBA's mGBA, gpSP and armGBA) shows two
//! of them at a time: Right from the second socket turns the cores along a place, so the one that
//! was in the second is in the first, and the chip hops from it into the next core's socket; Left
//! from the first turns them back.

use omega_store::Core;
use omega_ui::{ease, Millis, Refusal};

pub const SLIDE_MS: Millis = 160;
pub const LIFT_MS: Millis = 260;
pub const OPEN_MS: Millis = SLIDE_MS + LIFT_MS;
pub const CLOSE_MS: Millis = 320;
pub const HOP_MS: Millis = 180;
/// The sockets on a board.
pub const SOCKETS: usize = 2;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Press {
    Left,
    Right,
    Keep,
    Back,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Outcome {
    Nothing,
    Refused,
    Write(Core),
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Chip {
    /// How far across the sockets the chip is, in sockets from the first on the board: a whole
    /// number when it is seated in one.
    pub across: f32,
    pub lift: f32,
    pub seated: Option<Core>,
    pub shake: f32,
}

#[derive(Copy, Clone)]
struct Hop {
    from: usize,
    started: Millis,
}

#[derive(Copy, Clone)]
struct Close {
    started: Millis,
    from: f32,
}

#[derive(Copy, Clone)]
pub struct CorePicker {
    /// The cores the board has a socket for, in socket order.
    cores: &'static [Core],
    /// The core the chip is in, or heading for, as its place in `cores`.
    slot: usize,
    /// The place in `cores` of the core in the board's first socket.
    first: usize,
    opened: Millis,
    started: Option<Millis>,
    hop: Option<Hop>,
    close: Option<Close>,
    refusal: Option<Refusal>,
}

impl CorePicker {
    /// A picker over a socket for each of `cores`, the chip in `seat`'s; a seat that is none of
    /// them puts it in the first.
    pub fn open(cores: &'static [Core], seat: Core, now: Millis) -> Self {
        let slot = cores.iter().position(|c| *c == seat).unwrap_or(0);
        CorePicker {
            cores,
            slot,
            // the cores as they stand at first, unless the seat is past the second socket
            first: (slot + 1)
                .saturating_sub(SOCKETS)
                .min(cores.len().saturating_sub(SOCKETS)),
            opened: now,
            started: None,
            hop: None,
            close: None,
            refusal: None,
        }
    }

    pub fn start(&mut self, now: Millis) {
        self.started.get_or_insert(now);
    }

    pub fn waiting(&self) -> bool {
        self.started.is_none()
    }

    pub fn waited(&self, now: Millis) -> Millis {
        now.saturating_sub(self.opened)
    }

    /// The core whose socket the chip is in, or heading for.
    pub fn seat(&self) -> Core {
        self.cores[self.slot]
    }

    /// The cores the board has a socket for, in socket order.
    pub fn cores(&self) -> &'static [Core] {
        self.cores
    }

    /// The cores in the board's sockets now, in socket order.
    pub fn shown(&self) -> &'static [Core] {
        let end = (self.first + SOCKETS).min(self.cores.len());
        &self.cores[self.first..end]
    }

    pub fn closing(&self) -> bool {
        self.close.is_some()
    }

    pub fn openness(&self, now: Millis) -> f32 {
        match (self.close, self.started) {
            (Some(Close { started, from }), _) => {
                (from - now.saturating_sub(started) as f32 / CLOSE_MS as f32).max(0.0)
            }
            (None, Some(started)) => (now.saturating_sub(started) as f32 / OPEN_MS as f32).min(1.0),
            (None, None) => 0.0,
        }
    }

    pub fn finished(&self, now: Millis) -> bool {
        self.close.is_some() && self.openness(now) <= 0.0
    }

    pub fn press(&mut self, press: Press, now: Millis) -> Outcome {
        if self.close.is_some() {
            return Outcome::Nothing;
        }
        let last = self.cores.len().saturating_sub(1);
        let target = match press {
            Press::Keep => {
                self.begin_close(now);
                return Outcome::Write(self.seat());
            }
            Press::Back => {
                self.begin_close(now);
                return Outcome::Nothing;
            }
            Press::Left => self.slot.saturating_sub(1),
            Press::Right => (self.slot + 1).min(last),
        };
        if self.waiting() {
            return Outcome::Nothing;
        }
        if let Some(progress) = self.hop_progress(now) {
            // mid-hop the chip only turns back to where it came from
            if self.hop.map(|hop| hop.from) != Some(target) {
                return Outcome::Nothing;
            }
            let done = ((1.0 - progress) * HOP_MS as f32) as Millis;
            self.hop = Some(Hop {
                from: self.slot,
                started: now.saturating_sub(done),
            });
            self.slot = target;
            self.refusal = None;
            return Outcome::Nothing;
        }
        if target == self.slot {
            self.refusal = Some(Refusal::started(now));
            return Outcome::Refused;
        }
        // a core past either socket turns the cores along to bring it onto the board; the hop
        // then starts from the socket the chip's core has turned into
        if target < self.first {
            self.first = target;
        } else if target >= self.first + SOCKETS {
            self.first = target + 1 - SOCKETS;
        }
        self.hop = Some(Hop {
            from: self.slot,
            started: now,
        });
        self.slot = target;
        self.refusal = None;
        Outcome::Nothing
    }

    pub fn chip(&self, now: Millis) -> Chip {
        let shake = self.refusal.map_or(0.0, |r| r.offset(now));
        match (self.hop, self.hop_progress(now)) {
            (Some(hop), Some(q)) => {
                let first = self.first as f32;
                let (from, to) = (hop.from as f32 - first, self.slot as f32 - first);
                let arc = (std::f32::consts::PI * q).sin();
                Chip {
                    across: from + (to - from) * ease(q),
                    lift: arc,
                    seated: None,
                    shake,
                }
            }
            _ => Chip {
                across: (self.slot - self.first) as f32,
                lift: 0.0,
                seated: Some(self.seat()),
                shake,
            },
        }
    }

    fn begin_close(&mut self, now: Millis) {
        self.close = Some(Close {
            started: now,
            from: self.openness(now),
        });
    }

    fn hop_progress(&self, now: Millis) -> Option<f32> {
        let hop = self.hop?;
        let q = now.saturating_sub(hop.started) as f32 / HOP_MS as f32;
        (q < 1.0).then_some(q)
    }
}
