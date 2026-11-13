//! Hold slot state.
//!
//! The hold slot can be used once per locked piece. `Game::lock` resets the
//! flag; `Game::hold` marks it used.

use crate::piece::PieceKind;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hold {
    held: Option<PieceKind>,
    used: bool,
}

impl Hold {
    pub const fn new() -> Self {
        Self {
            held: None,
            used: false,
        }
    }

    pub const fn held(&self) -> Option<PieceKind> {
        self.held
    }

    pub const fn can_hold(&self) -> bool {
        !self.used
    }

    /// Puts `current` into the hold slot and returns the previously held piece,
    /// if any. The caller is responsible for checking [`Self::can_hold`] and
    /// for spawning the returned piece (or a new bag piece when `None`).
    pub fn swap(&mut self, current: PieceKind) -> Option<PieceKind> {
        let previous = self.held;
        self.held = Some(current);
        previous
    }

    pub fn mark_used(&mut self) {
        self.used = true;
    }

    pub fn reset_after_lock(&mut self) {
        self.used = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_empty_and_available() {
        let hold = Hold::new();
        assert_eq!(hold.held(), None);
        assert!(hold.can_hold());
    }

    #[test]
    fn swap_returns_previous_piece() {
        let mut hold = Hold::new();
        assert_eq!(hold.swap(PieceKind::T), None);
        assert_eq!(hold.held(), Some(PieceKind::T));
        assert_eq!(hold.swap(PieceKind::I), Some(PieceKind::T));
        assert_eq!(hold.held(), Some(PieceKind::I));
    }

    #[test]
    fn used_flag_blocks_until_reset() {
        let mut hold = Hold::new();
        hold.swap(PieceKind::J);
        hold.mark_used();
        assert!(!hold.can_hold());
        hold.reset_after_lock();
        assert!(hold.can_hold());
        assert_eq!(hold.held(), Some(PieceKind::J));
    }
}
