//! Opener Phase: the first 14 placements cancel twice as much garbage.
//!
//! Reference: `docs/s2-spec.md` v1.3.1 §7.4. The opener does **not** double the
//! sent attack; it adds an extra cancellation budget equal to the attack.

pub use crate::constants::OPENER_PIECES;

use crate::constants::OPENER_QUEUE_GE_SENT;

/// Tracks the opener window and the cumulative sent total for one player.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OpenerPhase {
    /// Placements before the current one. Hold does not increment this.
    pub pieces_placed: u32,
    /// Rows actually sent to the opponent this round (cancelled rows excluded).
    pub sent_total: u32,
}

impl OpenerPhase {
    pub const fn new() -> Self {
        Self {
            pieces_placed: 0,
            sent_total: 0,
        }
    }

    /// True for placements 1 through 14 (the placement about to happen).
    pub const fn is_active(&self) -> bool {
        self.pieces_placed < OPENER_PIECES
    }

    /// Call once per locked piece. Hold does not count as a placement.
    pub fn on_piece_placed(&mut self) {
        self.pieces_placed += 1;
    }

    /// Record rows that were actually sent by one attack event.
    pub fn on_sent(&mut self, lines: u32) {
        self.sent_total += lines;
    }

    /// Additional cancellation budget for one attack event.
    ///
    /// The caller's total cancellation is `attack + cancel_budget(attack, ..)`,
    /// i.e. `2 * attack` while the opener condition holds. U5 (decided):
    /// `queue_total >= sent_total`.
    pub fn cancel_budget(&self, attack: u32, queue_total: u32) -> u32 {
        let queue_ok = if OPENER_QUEUE_GE_SENT {
            queue_total >= self.sent_total
        } else {
            queue_total > self.sent_total
        };
        if self.is_active() && queue_ok {
            attack
        } else {
            0
        }
    }
}
