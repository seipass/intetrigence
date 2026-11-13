//! Mirror of the state an attax-style TBP adapter tracks from host messages.
//!
//! The adapter (`tbp/attax-tbp.ts`) tracks `queue`, `hold`, `combo` and `b2b`
//! itself and **never validates** `play` (`docs/attax-integration.md` §5.1).
//! The host must therefore reproduce the adapter's transitions exactly; this
//! type is that transition function, used to check the queue invariant after
//! every lock (`docs/attax-integration.md` §5.4 / §8-D5).

use crate::message::{Move, Piece, StartPayload};

/// Protocol-level state of one TBP bot.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Mirror {
    /// Current piece + preview (`queue[0]` is the current piece).
    pub queue: Vec<Piece>,
    pub hold: Option<Piece>,
    /// TBP combo: 0 = no chain, +1 per consecutive clear.
    pub combo: u32,
    /// TBP `back_to_back`: the previous clear was difficult.
    pub b2b: bool,
}

impl Mirror {
    pub fn new(queue: Vec<Piece>, hold: Option<Piece>, combo: u32, b2b: bool) -> Self {
        Self {
            queue,
            hold,
            combo,
            b2b,
        }
    }

    pub fn from_start(payload: &StartPayload) -> Self {
        Self {
            queue: payload.queue.clone(),
            hold: payload.hold,
            combo: payload.combo,
            b2b: payload.back_to_back,
        }
    }

    /// Applies the adapter's `applyPlay` transition (`attax-tbp.ts`).
    ///
    /// Returns `true` when the adapter inferred a hold (the placed piece is not
    /// the queue head). The returned flag is the host's hold inference
    /// (`attax-integration.md` §8-D6).
    pub fn apply_play(&mut self, mv: &Move) -> bool {
        let kind = mv.location.kind;
        let current = if self.queue.is_empty() {
            None
        } else {
            Some(self.queue.remove(0))
        };
        let mut held = false;
        if Some(kind) != current {
            held = true;
            let previous_hold = self.hold;
            self.hold = current;
            if previous_hold.is_none() && !self.queue.is_empty() {
                self.queue.remove(0);
            }
        }
        // The adapter recomputes lines from its board; the host knows the real
        // clear count and overwrites combo/b2b from the referee's authoritative
        // state after the play, so only the queue/hold part is mirrored here.
        held
    }

    pub fn push(&mut self, piece: Piece) {
        self.queue.push(piece);
    }

    /// Number of `new_piece` messages needed to restore a six-entry queue.
    pub fn refill_count(&self) -> usize {
        6usize.saturating_sub(self.queue.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Location, Orientation, SpinLabel};

    fn mv(kind: Piece) -> Move {
        Move {
            location: Location {
                kind,
                orientation: Orientation::North,
                x: 4,
                y: 0,
            },
            spin: SpinLabel::None,
        }
    }

    #[test]
    fn normal_lock_consumes_one() {
        let mut mirror = Mirror::new(vec![Piece::T, Piece::I, Piece::O], None, 0, false);
        assert!(!mirror.apply_play(&mv(Piece::T)));
        assert_eq!(mirror.queue, vec![Piece::I, Piece::O]);
        assert_eq!(mirror.refill_count(), 4);
    }

    #[test]
    fn hold_with_empty_slot_consumes_two() {
        let mut mirror = Mirror::new(vec![Piece::T, Piece::I, Piece::O], None, 0, false);
        assert!(mirror.apply_play(&mv(Piece::I)));
        assert_eq!(mirror.queue, vec![Piece::O]);
        assert_eq!(mirror.hold, Some(Piece::T));
        assert_eq!(mirror.refill_count(), 5);
    }

    #[test]
    fn hold_with_filled_slot_consumes_one() {
        let mut mirror = Mirror::new(vec![Piece::T, Piece::I], Some(Piece::O), 0, false);
        assert!(mirror.apply_play(&mv(Piece::O)));
        assert_eq!(mirror.queue, vec![Piece::I]);
        assert_eq!(mirror.hold, Some(Piece::T));
        assert_eq!(mirror.refill_count(), 5);
    }
}
