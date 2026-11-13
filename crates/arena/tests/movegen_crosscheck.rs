//! Cross-check the referee's movegen adapter against `intetrigence_bot::Movegen`.
//!
//! Both sides are compared as **`(sorted cells, spin)` sets**: the bot emits one
//! placement per `(rotation, x, y)` and the referee collapses cell-identical
//! placements to the strongest spin label (I/O/S/Z are 180-degree symmetric).
//! Comparing `(rotation, x, y)` would produce false mismatches.
//!
//! Kept permanently in CI: it is the drift detector for the one movegen that
//! both the search and the referee now share.

use std::collections::BTreeSet;

use intetrigence_arena::movegen;
use intetrigence_bot::Movegen;
use intetrigence_core::{Board, SplitMix64};

fn board(rows: &[u16]) -> Board {
    Board::from_rows(rows)
}

fn fixtures() -> Vec<(&'static str, Board)> {
    vec![
        ("empty", Board::new()),
        (
            "garbage",
            board(&[0x3FB, 0x1FF, 0x3EF, 0x3FE, 0x3FE, 0x3F6, 0x3F6, 0x016]),
        ),
        (
            "high",
            board(&[
                0x1F7, 0x1FF, 0x1FF, 0x1BF, 0x1FF, 0x1FF, 0x1FE, 0x1FF, 0x1FF, 0x1F7, 0x1FF, 0x1FF,
                0x1BF, 0x1FF, 0x1FF, 0x1FE, 0x1FF, 0x1FF, 0x1F7, 0x1C0,
            ]),
        ),
        ("tsd", board(&[0x1FF, 0x3EF, 0x3C7, 0x1CF])),
        ("overhang", board(&[0x3FC, 0x3FC, 0x007])),
    ]
}

fn bot_key(placement: &intetrigence_bot::Placement) -> ([(i32, i32); 4], u8) {
    let mut cells = placement.cells();
    cells.sort_unstable();
    (cells, placement.spin as u8)
}

fn arena_key(placement: &movegen::Placement) -> ([(i32, i32); 4], u8) {
    (placement.cells, placement.spin as u8)
}

fn compare(board: &Board, name: &str) {
    let mut movegen = Movegen::new();
    for kind in intetrigence_core::ALL_PIECES {
        let bot: BTreeSet<_> = intetrigence_bot::unique_by_cells(movegen.generate(board, kind))
            .iter()
            .map(bot_key)
            .collect();
        let arena: BTreeSet<_> = movegen::generate(board, movegen::spawn_state(kind), false)
            .placements()
            .iter()
            .map(arena_key)
            .collect();
        assert_eq!(bot, arena, "{name}/{kind:?}: movegen drift");
    }
}

#[test]
fn fixtures_match_the_bot_movegen() {
    for (name, board) in fixtures() {
        compare(&board, name);
    }
}

#[test]
fn random_boards_match_the_bot_movegen() {
    let mut rng = SplitMix64::new(0xC0FF_EE12_3456_789A);
    for case in 0..32 {
        let mut rows = [0u16; 40];
        let height = 2 + rng.below(20) as usize;
        for row in rows.iter_mut().take(height.min(20)) {
            let density = 25 + rng.below(65);
            let mut mask = 0u16;
            for x in 0..10 {
                if rng.below(100) < density {
                    mask |= 1 << x;
                }
            }
            if mask == Board::FULL_ROW {
                mask &= !(1 << rng.below(10));
            }
            *row = mask;
        }
        let board = Board::from_rows(&rows);
        compare(&board, &format!("random-{case}"));
    }
}

#[test]
fn referee_and_bot_agree_on_the_strongest_spin_label() {
    // `unique_by_cells` keeps the strongest label on both sides; this pins the
    // property that the adapter does not re-collapse in a different order.
    for (name, board) in fixtures() {
        for kind in intetrigence_core::ALL_PIECES {
            let raw = movegen::generate_all(&board, movegen::spawn_state(kind), false);
            let mut best: std::collections::HashMap<[(i32, i32); 4], u8> =
                std::collections::HashMap::new();
            for placement in &raw {
                let entry = best.entry(placement.cells).or_insert(placement.spin as u8);
                *entry = (*entry).max(placement.spin as u8);
            }
            let moves = movegen::generate(&board, movegen::spawn_state(kind), false);
            for placement in moves.placements() {
                assert_eq!(
                    placement.spin as u8, best[&placement.cells],
                    "{name}/{kind:?}: {:?} lost its strongest spin",
                    placement.cells
                );
            }
        }
    }
}
