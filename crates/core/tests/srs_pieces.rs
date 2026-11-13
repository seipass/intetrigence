//! Explicit SRS piece-state table.
//!
//! These assertions are intentionally verbose: they pin every one of the 28
//! states so a change to the spawn cells or the true-rotation center cannot
//! silently alter the piece geometry.

use std::collections::HashSet;

use intetrigence_core::{ALL_PIECES, PieceKind, Rotation};

fn render(kind: PieceKind, rotation: Rotation) -> String {
    let size = kind.box_size() as usize;
    let mut grid = vec![vec!['.'; size]; size];
    for (x, y) in kind.cells(rotation) {
        grid[size - 1 - y as usize][x as usize] = 'X';
    }
    grid.into_iter()
        .map(|row| row.into_iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn all_28_srs_states_match_the_expected_table() {
    let expected: &[(PieceKind, Rotation, &str)] = &[
        // I (4x4 box)
        (PieceKind::I, Rotation::Zero, "....\nXXXX\n....\n...."),
        (PieceKind::I, Rotation::R, "..X.\n..X.\n..X.\n..X."),
        (PieceKind::I, Rotation::Two, "....\n....\nXXXX\n...."),
        (PieceKind::I, Rotation::L, ".X..\n.X..\n.X..\n.X.."),
        // O (2x2 box; all four states are identical)
        (PieceKind::O, Rotation::Zero, "XX\nXX"),
        (PieceKind::O, Rotation::R, "XX\nXX"),
        (PieceKind::O, Rotation::Two, "XX\nXX"),
        (PieceKind::O, Rotation::L, "XX\nXX"),
        // T
        (PieceKind::T, Rotation::Zero, ".X.\nXXX\n..."),
        (PieceKind::T, Rotation::R, ".X.\n.XX\n.X."),
        (PieceKind::T, Rotation::Two, "...\nXXX\n.X."),
        (PieceKind::T, Rotation::L, ".X.\nXX.\n.X."),
        // S
        (PieceKind::S, Rotation::Zero, ".XX\nXX.\n..."),
        (PieceKind::S, Rotation::R, ".X.\n.XX\n..X"),
        (PieceKind::S, Rotation::Two, "...\n.XX\nXX."),
        (PieceKind::S, Rotation::L, "X..\nXX.\n.X."),
        // Z
        (PieceKind::Z, Rotation::Zero, "XX.\n.XX\n..."),
        (PieceKind::Z, Rotation::R, "..X\n.XX\n.X."),
        (PieceKind::Z, Rotation::Two, "...\nXX.\n.XX"),
        (PieceKind::Z, Rotation::L, ".X.\nXX.\nX.."),
        // J
        (PieceKind::J, Rotation::Zero, "X..\nXXX\n..."),
        (PieceKind::J, Rotation::R, ".XX\n.X.\n.X."),
        (PieceKind::J, Rotation::Two, "...\nXXX\n..X"),
        (PieceKind::J, Rotation::L, ".X.\n.X.\nXX."),
        // L
        (PieceKind::L, Rotation::Zero, "..X\nXXX\n..."),
        (PieceKind::L, Rotation::R, ".X.\n.X.\n.XX"),
        (PieceKind::L, Rotation::Two, "...\nXXX\nX.."),
        (PieceKind::L, Rotation::L, "XX.\n.X.\n.X."),
    ];

    assert_eq!(expected.len(), 28);
    for &(kind, rotation, expected) in expected {
        assert_eq!(
            render(kind, rotation),
            expected,
            "unexpected {kind:?} state {rotation:?}"
        );
    }
}

#[test]
fn o_has_one_unique_state_and_other_pieces_have_four() {
    for kind in ALL_PIECES {
        let unique: HashSet<String> = Rotation::ALL
            .iter()
            .map(|&rotation| render(kind, rotation))
            .collect();
        let expected = if kind == PieceKind::O { 1 } else { 4 };
        assert_eq!(unique.len(), expected, "{kind:?}");
    }
}

#[test]
fn rotation_centers_are_the_true_srs_centers() {
    // Cell-index coordinates; the 4x4 I box center is a grid intersection.
    assert_eq!(PieceKind::I.rotation_center(), (1.5, 1.5));
    assert_eq!(PieceKind::O.rotation_center(), (0.5, 0.5));
    for kind in [
        PieceKind::T,
        PieceKind::S,
        PieceKind::Z,
        PieceKind::J,
        PieceKind::L,
    ] {
        assert_eq!(kind.rotation_center(), (1.0, 1.0), "{kind:?}");
    }
}

#[test]
fn spawn_lowest_occupied_row_is_hidden_row_20() {
    for kind in ALL_PIECES {
        let (_, y) = kind.spawn_position();
        let lowest = kind
            .cells(Rotation::Zero)
            .iter()
            .map(|&(_, cy)| y + cy)
            .min()
            .unwrap();
        assert_eq!(lowest, 20, "{kind:?}");
    }
}
