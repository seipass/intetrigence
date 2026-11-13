//! Deterministic 7-bag randomizer.
//!
//! # Seed protocol
//!
//! `Bag::new(seed)` seeds a [`MinStd`] with the **piece seed** from
//! [`crate::rng::derive_seeds`] and shuffles [`ALL_PIECES`] with a descending
//! Fisher-Yates pass using `floor(next_float() * (i + 1))`. Each bag is a
//! permutation of all seven pieces. The algorithm, piece order and RNG are
//! fixed: changing any of them changes every seeded replay.
//!
//! `peek` may generate whole bags ahead of time, but it never consumes pieces,
//! so previewing does not alter the sequence.

use std::collections::VecDeque;

use crate::piece::{ALL_PIECES, PieceKind};
use crate::rng::MinStd;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bag {
    rng: MinStd,
    queue: VecDeque<PieceKind>,
    bags_generated: u64,
}

impl Bag {
    pub fn new(seed: u64) -> Self {
        let mut bag = Self {
            rng: MinStd::new(seed),
            queue: VecDeque::with_capacity(14),
            bags_generated: 0,
        };
        bag.refill();
        bag
    }

    fn refill(&mut self) {
        let mut pieces = ALL_PIECES;
        for i in (1..pieces.len()).rev() {
            let j = (self.rng.next_float() * (i + 1) as f64).floor() as usize;
            pieces.swap(i, j);
        }
        self.queue.extend(pieces);
        self.bags_generated += 1;
    }

    /// Returns the next piece, generating a new bag when the queue is empty.
    pub fn next_piece(&mut self) -> PieceKind {
        if self.queue.is_empty() {
            self.refill();
        }
        self.queue.pop_front().expect("bag queue is non-empty")
    }

    /// Returns the next `count` pieces without consuming them.
    pub fn peek(&mut self, count: usize) -> Vec<PieceKind> {
        while self.queue.len() < count {
            self.refill();
        }
        self.queue.iter().take(count).copied().collect()
    }

    pub fn bags_generated(&self) -> u64 {
        self.bags_generated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn known_seed_zero_sequence() {
        // Locks the seed protocol: MINSTD + descending Fisher-Yates.
        let mut bag = Bag::new(0);
        let sequence: Vec<PieceKind> = (0..14).map(|_| bag.next_piece()).collect();
        assert_eq!(
            sequence,
            vec![
                PieceKind::I,
                PieceKind::S,
                PieceKind::Z,
                PieceKind::T,
                PieceKind::O,
                PieceKind::J,
                PieceKind::L,
                PieceKind::T,
                PieceKind::S,
                PieceKind::Z,
                PieceKind::I,
                PieceKind::J,
                PieceKind::O,
                PieceKind::L,
            ]
        );
    }

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Bag::new(0xDEAD_BEEF);
        let mut b = Bag::new(0xDEAD_BEEF);
        for _ in 0..1000 {
            assert_eq!(a.next_piece(), b.next_piece());
        }
    }

    #[test]
    fn different_seeds_differ() {
        let mut a = Bag::new(1);
        let mut b = Bag::new(2);
        let a_seq: Vec<_> = (0..100).map(|_| a.next_piece()).collect();
        let b_seq: Vec<_> = (0..100).map(|_| b.next_piece()).collect();
        assert_ne!(a_seq, b_seq);
    }

    #[test]
    fn every_bag_contains_each_piece_once() {
        let mut bag = Bag::new(42);
        for bag_index in 0..100 {
            let mut seen = HashSet::new();
            for _ in 0..7 {
                let piece = bag.next_piece();
                assert!(seen.insert(piece), "duplicate {piece:?} in bag {bag_index}");
            }
            assert_eq!(seen.len(), 7);
            for piece in ALL_PIECES {
                assert!(
                    seen.contains(&piece),
                    "missing {piece:?} in bag {bag_index}"
                );
            }
        }
    }

    #[test]
    fn preview_does_not_consume() {
        let mut bag = Bag::new(7);
        let preview = bag.peek(5);
        assert_eq!(preview.len(), 5);
        assert_eq!(bag.next_piece(), preview[0]);
        let next_preview = bag.peek(5);
        assert_eq!(next_preview[..4], preview[1..]);
    }

    #[test]
    fn preview_across_bag_boundary_contains_two_bags() {
        let mut bag = Bag::new(9);
        let preview = bag.peek(10);
        assert_eq!(preview.len(), 10);
        let first: HashSet<_> = preview[..7].iter().copied().collect();
        let second: HashSet<_> = preview[7..].iter().copied().collect();
        assert_eq!(first.len(), 7);
        assert_eq!(second.len(), 3);
        // The first seven are a complete bag; the next three are the start of
        // the following bag (which necessarily reuses the same seven kinds).
        for piece in ALL_PIECES {
            assert!(first.contains(&piece));
        }
    }

    #[test]
    fn peek_zero_is_empty_and_does_not_generate() {
        let mut bag = Bag::new(0);
        assert!(bag.peek(0).is_empty());
        assert_eq!(bag.bags_generated(), 1);
    }
}
