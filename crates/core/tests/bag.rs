//! 7-bag determinism and composition, exercised through the public API.

use std::collections::HashSet;

use intetrigence_core::{ALL_PIECES, Bag, PieceKind};

#[test]
fn same_seed_produces_the_same_sequence() {
    let mut a = Bag::new(0x1234_5678_9ABC_DEF0);
    let mut b = Bag::new(0x1234_5678_9ABC_DEF0);
    for index in 0..2000 {
        assert_eq!(a.next_piece(), b.next_piece(), "piece {index}");
    }
}

#[test]
fn every_consecutive_seven_pieces_are_one_bag() {
    let mut bag = Bag::new(2024);
    for bag_index in 0..200 {
        let mut seen = HashSet::new();
        for _ in 0..7 {
            let piece = bag.next_piece();
            assert!(seen.insert(piece), "duplicate {piece:?} in bag {bag_index}");
        }
        for piece in ALL_PIECES {
            assert!(
                seen.contains(&piece),
                "missing {piece:?} in bag {bag_index}"
            );
        }
    }
}

#[test]
fn preview_does_not_advance_the_sequence() {
    let mut bag = Bag::new(99);
    let preview = bag.peek(10);
    assert_eq!(preview.len(), 10);
    for &expected in &preview {
        assert_eq!(bag.next_piece(), expected);
    }
}

#[test]
fn known_seed_zero_sequence_is_stable() {
    let mut bag = Bag::new(0);
    let sequence: Vec<PieceKind> = (0..14).map(|_| bag.next_piece()).collect();
    assert_eq!(
        sequence,
        vec![
            PieceKind::L,
            PieceKind::S,
            PieceKind::O,
            PieceKind::J,
            PieceKind::Z,
            PieceKind::I,
            PieceKind::T,
            PieceKind::S,
            PieceKind::I,
            PieceKind::L,
            PieceKind::J,
            PieceKind::Z,
            PieceKind::T,
            PieceKind::O,
        ]
    );
}

#[test]
fn different_seeds_produce_different_sequences() {
    let mut a = Bag::new(1);
    let mut b = Bag::new(2);
    let a_seq: Vec<_> = (0..100).map(|_| a.next_piece()).collect();
    let b_seq: Vec<_> = (0..100).map(|_| b.next_piece()).collect();
    assert_ne!(a_seq, b_seq);
}

#[test]
fn preview_across_a_bag_boundary_has_a_full_first_bag() {
    let mut bag = Bag::new(7);
    let preview = bag.peek(10);
    let first: HashSet<_> = preview[..7].iter().copied().collect();
    assert_eq!(first.len(), 7);
    for piece in ALL_PIECES {
        assert!(first.contains(&piece));
    }
}
