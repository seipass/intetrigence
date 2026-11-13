//! Opener phase interface.
//!
//! **Status: interface only.** PLAN.md §3.1 fixes the window (the first 14
//! pieces) and the effect (attack cancellation ×2), but the exact accounting
//! waits for `docs/s2-spec.md`.

/// Number of pieces in the opener phase (PLAN.md §3.1).
pub const OPENER_PIECES: u8 = 14;

/// Tracks the opener window for one player.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OpenerPhase {
    pub pieces_placed: u8,
}

impl OpenerPhase {
    pub const fn new() -> Self {
        Self { pieces_placed: 0 }
    }

    /// True while the player is still inside the opener window.
    pub const fn is_active(&self) -> bool {
        self.pieces_placed < OPENER_PIECES
    }

    /// TODO(s2-spec): attack multiplier during the opener phase.
    pub fn attack_multiplier(&self) -> u32 {
        todo!("TODO(s2-spec): opener phase attack multiplier")
    }
}
