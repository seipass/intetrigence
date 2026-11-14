//! 7-bag state accessors for the referee / TBP `start.randomizer`.

use std::collections::HashSet;

use intetrigence_core::{ALL_PIECES, BAG_SIZE, Bag, BagState, Game};

#[test]
fn fresh_bag_has_all_seven_pieces() {
    let bag = Bag::new(0);
    let state = bag.state();

    assert_eq!(state.len(), BAG_SIZE);
    assert_eq!(state.mask(), 0b111_1111);
    assert!(!state.is_empty());

    let set: HashSet<_> = state.remaining.iter().copied().collect();
    assert_eq!(set.len(), 7);
    for piece in ALL_PIECES {
        assert!(state.contains(piece), "{piece:?}");
    }
}

#[test]
fn remaining_shrinks_one_per_draw() {
    let mut bag = Bag::new(0);
    let mut seen = HashSet::new();

    for expected in (0..BAG_SIZE).rev() {
        assert_eq!(bag.remaining().len(), expected + 1);
        let piece = bag.next_piece();
        assert!(seen.insert(piece), "duplicate {piece:?}");
        assert!(!bag.remaining().contains(&piece));
    }

    assert!(bag.remaining().is_empty());
    assert_eq!(bag.state().mask(), 0);
    assert_eq!(seen.len(), 7);
}

#[test]
fn next_bag_starts_after_seven_draws() {
    let mut bag = Bag::new(0);
    for _ in 0..BAG_SIZE {
        bag.next_piece();
    }
    assert!(bag.remaining().is_empty());

    let first = bag.next_piece();
    assert_eq!(bag.remaining().len(), BAG_SIZE - 1);
    assert!(!bag.remaining().contains(&first));
    assert_eq!(bag.bags_generated(), 2);
}

#[test]
fn peek_does_not_change_the_current_bag_state() {
    let mut bag = Bag::new(0);
    let before = bag.state();

    let preview = bag.peek(10);
    assert_eq!(preview.len(), 10);
    assert_eq!(bag.state(), before);
    assert_eq!(bag.remaining().len(), BAG_SIZE);
}

#[test]
fn peek_ahead_does_not_change_current_bag_remaining() {
    let mut bag = Bag::new(0);
    for _ in 0..3 {
        bag.next_piece();
    }
    let before = bag.state();
    assert_eq!(before.len(), 4);

    let _ = bag.peek(10);
    assert_eq!(bag.state(), before);
    assert_eq!(bag.remaining().len(), 4);
}

#[test]
fn mask_tracks_remaining_pieces() {
    let mut bag = Bag::new(0);
    for _ in 0..BAG_SIZE {
        let state = bag.state();
        let expected = state
            .remaining
            .iter()
            .fold(0u8, |mask, &piece| mask | (1u8 << piece.index()));
        assert_eq!(state.mask(), expected);
        bag.next_piece();
    }
}

#[test]
fn same_seed_same_bag_states() {
    let mut a = Bag::new(1234);
    let mut b = Bag::new(1234);
    for _ in 0..100 {
        assert_eq!(a.state(), b.state());
        assert_eq!(a.next_piece(), b.next_piece());
    }
}

#[test]
fn game_bag_state_excludes_the_active_piece() {
    let game = Game::new(0);
    let active = game.active().unwrap().kind;
    let state = game.bag_state();

    assert_eq!(state.len(), BAG_SIZE - 1);
    assert!(!state.contains(active));
}

#[test]
fn game_lock_advances_the_bag_state() {
    let mut game = Game::new(0);
    let before = game.bag_state();

    game.hard_drop().unwrap();

    assert_eq!(game.bag_state().len(), before.len() - 1);
}

#[test]
fn game_preview_does_not_change_bag_state() {
    let mut game = Game::new(0);
    let before = game.bag_state();

    let _ = game.preview(5);

    assert_eq!(game.bag_state(), before);
}

#[test]
fn game_hold_with_empty_slot_draws_one_piece() {
    let mut game = Game::new(0);
    let before = game.bag_state();

    assert!(game.hold());

    assert_eq!(game.bag_state().len(), before.len() - 1);
}

#[test]
fn game_hold_swap_does_not_draw_a_new_piece() {
    let mut game = Game::new(0);
    assert!(game.hold()); // held = first, active = second
    game.hard_drop().unwrap(); // lock second, active = third
    let before = game.bag_state();

    assert!(game.hold()); // swap third with held first

    assert_eq!(game.bag_state(), before);
}

#[test]
fn game_bag_state_after_seven_locks_is_six() {
    let mut game = Game::new(0);
    for _ in 0..BAG_SIZE {
        assert!(!game.is_over());
        game.hard_drop().unwrap();
    }

    assert!(!game.is_over());
    assert_eq!(game.bag_state().len(), BAG_SIZE - 1);
}

#[test]
fn remaining_never_contains_duplicates() {
    let mut bag = Bag::new(42);
    for _ in 0..100 {
        let state = bag.state();
        let set: HashSet<_> = state.remaining.iter().copied().collect();
        assert_eq!(set.len(), state.len());
        bag.next_piece();
    }
}

#[test]
fn empty_state_is_empty() {
    let state = BagState {
        remaining: Vec::new(),
    };
    assert!(state.is_empty());
    assert_eq!(state.len(), 0);
    assert_eq!(state.mask(), 0);
}
