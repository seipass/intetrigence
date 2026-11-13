//! Movegen fixtures shared with `crates/bot` (bot-dev, 2026-11-14).
//!
//! These pin the referee's reachable placement set on five boards:
//! empty, garbage/stack, a 20-row high board with a 1-wide well, a TSD slot, and
//! a sealed 2x2 cavity. The cavity is the negative test: the O placement is
//! legal and grounded but unreachable, so it must not appear.
//!
//! The same fixtures are the basis for the post-integration cross-check against
//! `intetrigence_bot::Movegen`; both sides must be compared as **cell sets**
//! (I/O/S/Z describe the same cells with more than one rotation).

use std::collections::HashMap;

use intetrigence_arena::movegen::{self, spawn_state};
use intetrigence_core::{Board, Game, PieceKind, Rotation, SpinType};

fn board(rows: &[u16]) -> Board {
    Board::from_rows(rows)
}

fn empty() -> Board {
    Board::new()
}

fn garbage() -> Board {
    board(&[0x3FB, 0x1FF, 0x3EF, 0x3FE, 0x3FE, 0x3F6, 0x3F6, 0x016])
}

fn high() -> Board {
    board(&[
        0x1F7, 0x1FF, 0x1FF, 0x1BF, 0x1FF, 0x1FF, 0x1FE, 0x1FF, 0x1FF, 0x1F7, 0x1FF, 0x1FF, 0x1BF,
        0x1FF, 0x1FF, 0x1FE, 0x1FF, 0x1FF, 0x1F7, 0x1C0,
    ])
}

fn tsd() -> Board {
    board(&[0x1FF, 0x3EF, 0x3C7, 0x1CF])
}

fn overhang() -> Board {
    board(&[0x3FC, 0x3FC, 0x007])
}

fn fixtures() -> Vec<(&'static str, Board)> {
    vec![
        ("empty", empty()),
        ("garbage", garbage()),
        ("high", high()),
        ("tsd", tsd()),
        ("overhang", overhang()),
    ]
}

fn cells_key(mut cells: [(i32, i32); 4]) -> [(i32, i32); 4] {
    cells.sort_unstable();
    cells
}

#[test]
fn fixtures_have_no_full_rows_and_accept_the_standard_spawn() {
    for (name, board) in fixtures() {
        for y in 0..Board::HEIGHT {
            assert_ne!(board.row(y), Board::FULL_ROW, "{name}: full row {y}");
        }
        for kind in intetrigence_core::ALL_PIECES {
            Game::from_position(0, board.clone(), spawn_state(kind))
                .unwrap_or_else(|error| panic!("{name}/{kind:?}: {error:?}"));
        }
    }
}

#[test]
fn every_placement_is_legal_and_grounded() {
    for (name, board) in fixtures() {
        for kind in intetrigence_core::ALL_PIECES {
            let moves = movegen::generate(&board, spawn_state(kind), false);
            assert!(!moves.is_empty(), "{name}/{kind:?}: no placements");
            for placement in moves.placements() {
                for (x, y) in placement.cells {
                    assert!(
                        (0..Board::WIDTH as i32).contains(&x)
                            && (0..Board::HEIGHT as i32).contains(&y),
                        "{name}/{kind:?}: cell ({x}, {y}) out of bounds"
                    );
                    assert!(
                        !board.occupied_at(x as usize, y as usize),
                        "{name}/{kind:?}: cell ({x}, {y}) overlaps"
                    );
                }
                assert!(
                    !board.can_place(kind, placement.rotation, placement.x, placement.y - 1),
                    "{name}/{kind:?}: {:?} is not grounded",
                    placement.cells
                );
                replay_path(&board, kind, placement);
            }
        }
    }
}

#[test]
fn empty_board_cell_set_counts_match_the_bot() {
    // bot-dev's measurement: I/O/S/Z have duplicate rotations that describe the
    // same cells, so the cell-set unique counts are lower than the placement
    // counts.
    let expected = [
        (PieceKind::I, 17),
        (PieceKind::O, 9),
        (PieceKind::S, 17),
        (PieceKind::Z, 17),
        (PieceKind::T, 34),
        (PieceKind::J, 34),
        (PieceKind::L, 34),
    ];
    let board = empty();
    for (kind, count) in expected {
        let moves = movegen::generate(&board, spawn_state(kind), false);
        let unique: std::collections::BTreeSet<_> = moves
            .placements()
            .iter()
            .map(|placement| cells_key(placement.cells))
            .collect();
        assert_eq!(
            unique.len(),
            count,
            "{kind:?}: cell-set unique count must match crates/bot"
        );
        assert_eq!(
            moves.placements().len(),
            count,
            "{kind:?}: the move set must already be deduplicated by cells"
        );
    }
}

#[test]
fn tsd_slot_is_a_full_spin_double() {
    let board = tsd();
    let moves = movegen::generate(&board, spawn_state(PieceKind::T), false);
    // T at Rotation::Two, bbox (3, 1): cells (4, 1), (3, 2), (4, 2), (5, 2).
    let target = cells_key([(4, 1), (3, 2), (4, 2), (5, 2)]);
    let placement = moves
        .get(&target)
        .unwrap_or_else(|| panic!("TSD placement must be reachable: {target:?}"));
    assert_eq!(placement.spin, SpinType::Full);
    assert_eq!(placement.lines_after(&board), 2);
    assert_eq!(placement.rotation, Rotation::Two);
    assert_eq!((placement.x, placement.y), (3, 1));
}

#[test]
fn sealed_cavity_is_legal_but_unreachable() {
    let board = overhang();
    // The 2x2 cavity at columns 0..1, rows 0..1 is empty, but the O piece
    // cannot get in: it is sealed by row 2 (columns 0..1) and column 2.
    let cavity = cells_key([(0, 0), (1, 0), (0, 1), (1, 1)]);
    assert!(
        board.can_place(PieceKind::O, Rotation::Zero, 0, 0),
        "the cavity itself must be legal"
    );
    let moves = movegen::generate(&board, spawn_state(PieceKind::O), false);
    assert!(
        moves.get(&cavity).is_none(),
        "movegen must not offer the sealed cavity"
    );
}

#[test]
fn cell_set_keeps_the_strongest_spin() {
    // I/O/S/Z describe the same cells with more than one rotation, and the
    // rotations can differ in whether the last action was a rotation. A non-T
    // spin is `spin_entry && immobile`, so the collapsed placement must keep
    // the strongest label (bot-dev, 27edb70).
    for (name, board) in fixtures() {
        for kind in intetrigence_core::ALL_PIECES {
            let all = movegen::generate_all(&board, spawn_state(kind), false);
            let mut best: HashMap<[(i32, i32); 4], SpinType> = HashMap::new();
            for placement in &all {
                let entry = best.entry(placement.cells).or_insert(placement.spin);
                if placement.spin.index() > entry.index() {
                    *entry = placement.spin;
                }
            }
            let moves = movegen::generate(&board, spawn_state(kind), false);
            assert_eq!(
                moves.placements().len(),
                best.len(),
                "{name}/{kind:?}: exactly one placement per cell set"
            );
            for placement in moves.placements() {
                assert_eq!(
                    placement.spin, best[&placement.cells],
                    "{name}/{kind:?}: {:?} lost its strongest spin",
                    placement.cells
                );
            }
        }
    }
}

#[test]
fn high_board_well_accepts_a_vertical_i() {
    let board = high();
    let moves = movegen::generate(&board, spawn_state(PieceKind::I), false);
    // Column 9 is a 1-wide well. Both vertical I rotations (R at bbox x = 7,
    // L at bbox x = 8) describe the same cells; the canonical one is whichever
    // the BFS reaches first, so only the cells are asserted.
    let target = cells_key([(9, 0), (9, 1), (9, 2), (9, 3)]);
    let placement = moves
        .get(&target)
        .unwrap_or_else(|| panic!("vertical I in the well must be reachable: {target:?}"));
    assert!(matches!(placement.rotation, Rotation::R | Rotation::L));
    replay_path(&board, PieceKind::I, placement);
}

/// Replays a placement's action path through `Game` and checks the final cells.
fn replay_path(board: &Board, kind: PieceKind, placement: &movegen::Placement) {
    let mut game = Game::from_position(0, board.clone(), spawn_state(kind))
        .expect("the fixture must accept the spawn");
    for action in &placement.path {
        let result = game.apply(*action);
        assert!(result.accepted, "path action {action:?} was rejected");
    }
    let active = game.active().expect("the piece must still be active");
    assert_eq!(
        cells_key(active.cells()),
        placement.cells,
        "path replay must end at the placement"
    );
}
