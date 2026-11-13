//! Opener Phase tests (`docs/s2-spec.md` §7.4).

use intetrigence_core::{OPENER_PIECES, OpenerPhase};

#[test]
fn opener_window_is_the_first_14_placements() {
    let mut opener = OpenerPhase::new();
    for placed in 0..OPENER_PIECES {
        assert_eq!(opener.pieces_placed, placed);
        assert!(opener.is_active(), "placement {}", placed + 1);
        opener.on_piece_placed();
    }
    assert_eq!(opener.pieces_placed, OPENER_PIECES);
    assert!(!opener.is_active());
}

#[test]
fn cancel_budget_doubles_only_when_active_and_queue_ge_sent() {
    let mut opener = OpenerPhase::new();
    assert_eq!(opener.cancel_budget(3, 5), 3);

    opener.on_sent(5);
    assert_eq!(opener.cancel_budget(3, 5), 3); // U5: >=, not >
    assert_eq!(opener.cancel_budget(3, 4), 0);

    opener.pieces_placed = OPENER_PIECES - 1;
    assert_eq!(opener.cancel_budget(3, 100), 3); // 14th placement
    opener.pieces_placed = OPENER_PIECES;
    assert_eq!(opener.cancel_budget(3, 100), 0); // 15th placement
}

#[test]
fn sent_total_accumulates() {
    let mut opener = OpenerPhase::new();
    opener.on_sent(2);
    opener.on_sent(3);
    assert_eq!(opener.sent_total, 5);
}
