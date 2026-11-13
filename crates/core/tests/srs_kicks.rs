//! SRS kick integration tests: walls, floor, other pieces, 180s and failure.

use intetrigence_core::{
    ALL_PIECES, ActivePiece, Board, Cell, Game, PieceKind, RotateDirection, Rotation,
    is_fin_or_tst_kick, kicks,
};

fn game_at(board: Board, piece: ActivePiece) -> Game {
    Game::from_position(0, board, piece).expect("test position must be legal")
}

#[test]
fn j_wall_kick_right() {
    // J in R state with its box hanging one column past the left wall.
    let active = ActivePiece {
        kind: PieceKind::J,
        rotation: Rotation::R,
        x: -1,
        y: 0,
    };
    let mut game = game_at(Board::new(), active);

    assert!(game.rotate(RotateDirection::Cw));
    assert_eq!(
        game.active(),
        Some(ActivePiece {
            kind: PieceKind::J,
            rotation: Rotation::Two,
            x: 0,
            y: 0,
        })
    );
}

#[test]
fn i_wall_kick_right_two_cells() {
    // Vertical I in column 0. Rotating to horizontal needs the +2 kick.
    let active = ActivePiece {
        kind: PieceKind::I,
        rotation: Rotation::R,
        x: -2,
        y: 0,
    };
    let mut game = game_at(Board::new(), active);

    assert!(game.rotate(RotateDirection::Ccw));
    assert_eq!(
        game.active(),
        Some(ActivePiece {
            kind: PieceKind::I,
            rotation: Rotation::Zero,
            x: 0,
            y: 0,
        })
    );
}

#[test]
fn i_wall_kick_left_one_cell() {
    // Vertical I in column 9. The SRS+ R->0 second candidate is (-1, 0).
    let active = ActivePiece {
        kind: PieceKind::I,
        rotation: Rotation::R,
        x: 7,
        y: 0,
    };
    let mut game = game_at(Board::new(), active);

    assert!(game.rotate(RotateDirection::Ccw));
    assert_eq!(
        game.active(),
        Some(ActivePiece {
            kind: PieceKind::I,
            rotation: Rotation::Zero,
            x: 6,
            y: 0,
        })
    );
}

#[test]
fn i_floor_kick_up_two() {
    // Flat I resting on row 0. Rotating CW would put cells below the floor;
    // the fifth 0->R candidate is (+1, +2), which lifts it to row 0.
    let active = ActivePiece {
        kind: PieceKind::I,
        rotation: Rotation::Zero,
        x: 3,
        y: -2,
    };
    let mut game = game_at(Board::new(), active);

    assert!(game.rotate(RotateDirection::Cw));
    assert_eq!(
        game.active(),
        Some(ActivePiece {
            kind: PieceKind::I,
            rotation: Rotation::R,
            x: 4,
            y: 0,
        })
    );
}

#[test]
fn t_floor_kick_up_one() {
    // T resting on the floor with its box one row below. CW rotation would
    // put the stem at row -1; the (-1, +1) candidate lifts it.
    let active = ActivePiece {
        kind: PieceKind::T,
        rotation: Rotation::Zero,
        x: 3,
        y: -1,
    };
    let mut game = game_at(Board::new(), active);

    assert!(game.rotate(RotateDirection::Cw));
    assert_eq!(
        game.active(),
        Some(ActivePiece {
            kind: PieceKind::T,
            rotation: Rotation::R,
            x: 2,
            y: 0,
        })
    );
}

#[test]
fn t_kick_against_another_piece() {
    // A block at (1, 0) blocks the first CW candidate; the (-1, 0) kick wins.
    let mut board = Board::new();
    board.set_cell(1, 0, Cell::Block);

    let active = ActivePiece {
        kind: PieceKind::T,
        rotation: Rotation::Zero,
        x: 0,
        y: 0,
    };
    let mut game = game_at(board, active);

    assert!(game.rotate(RotateDirection::Cw));
    assert_eq!(
        game.active(),
        Some(ActivePiece {
            kind: PieceKind::T,
            rotation: Rotation::R,
            x: -1,
            y: 0,
        })
    );
}

#[test]
fn t_third_kick_against_two_blocks() {
    // The first two candidates are blocked; (-1, +1) is the first free one.
    let mut board = Board::new();
    board.set_cell(1, 0, Cell::Block);
    board.set_cell(0, 0, Cell::Block);

    let active = ActivePiece {
        kind: PieceKind::T,
        rotation: Rotation::Zero,
        x: 0,
        y: 0,
    };
    let mut game = game_at(board, active);

    assert!(game.rotate(RotateDirection::Cw));
    assert_eq!(
        game.active(),
        Some(ActivePiece {
            kind: PieceKind::T,
            rotation: Rotation::R,
            x: -1,
            y: 1,
        })
    );
}

#[test]
fn rotation_fails_when_all_kicks_collide() {
    // Blocks cover every in-bounds candidate of T's 0->R table without
    // overlapping the spawn cells.
    let mut board = Board::new();
    for (x, y) in [(1, 0), (0, 0), (0, 2), (0, 3)] {
        board.set_cell(x, y, Cell::Block);
    }

    let active = ActivePiece {
        kind: PieceKind::T,
        rotation: Rotation::Zero,
        x: 0,
        y: 0,
    };
    let mut game = game_at(board, active);
    let before = game.active();

    assert!(!game.rotate(RotateDirection::Cw));
    assert_eq!(game.active(), before);
}

#[test]
fn half_rotation_in_open_space() {
    let active = ActivePiece {
        kind: PieceKind::T,
        rotation: Rotation::Zero,
        x: 3,
        y: 20,
    };
    let mut game = game_at(Board::new(), active);

    assert!(game.rotate(RotateDirection::Half));
    assert_eq!(
        game.active(),
        Some(ActivePiece {
            kind: PieceKind::T,
            rotation: Rotation::Two,
            x: 3,
            y: 20,
        })
    );
}

#[test]
fn t_180_kick_against_block() {
    // The (0,0) 180 target would overlap (4,20); (0,+1) is the first free
    // candidate.
    let mut board = Board::new();
    board.set_cell(4, 20, Cell::Block);

    let active = ActivePiece {
        kind: PieceKind::T,
        rotation: Rotation::Zero,
        x: 3,
        y: 20,
    };
    let mut game = game_at(board, active);

    assert!(game.rotate(RotateDirection::Half));
    assert_eq!(
        game.active(),
        Some(ActivePiece {
            kind: PieceKind::T,
            rotation: Rotation::Two,
            x: 3,
            y: 21,
        })
    );
}

#[test]
fn i_180_kick_against_block() {
    // The (0,0) 180 target would overlap (3,21); (0,+1) is the first free
    // candidate.
    let mut board = Board::new();
    board.set_cell(3, 21, Cell::Block);

    let active = ActivePiece {
        kind: PieceKind::I,
        rotation: Rotation::Zero,
        x: 3,
        y: 20,
    };
    let mut game = game_at(board, active);

    assert!(game.rotate(RotateDirection::Half));
    assert_eq!(
        game.active(),
        Some(ActivePiece {
            kind: PieceKind::I,
            rotation: Rotation::Two,
            x: 3,
            y: 21,
        })
    );
}

#[test]
fn o_rotation_is_identity() {
    let active = ActivePiece {
        kind: PieceKind::O,
        rotation: Rotation::Zero,
        x: 4,
        y: 20,
    };
    let mut game = game_at(Board::new(), active);

    assert!(game.rotate(RotateDirection::Cw));
    assert_eq!(
        game.active(),
        Some(ActivePiece {
            kind: PieceKind::O,
            rotation: Rotation::R,
            x: 4,
            y: 20,
        })
    );
}

#[test]
fn public_kick_tables_start_with_identity() {
    for kind in ALL_PIECES {
        for from in Rotation::ALL {
            for to in Rotation::ALL {
                if from == to {
                    continue;
                }
                assert_eq!(
                    kicks(kind, from, to)[0],
                    (0, 0),
                    "{kind:?} {from:?}->{to:?}"
                );
            }
        }
    }
}

#[test]
fn quarter_turn_kick_tables_are_symmetric() {
    for kind in ALL_PIECES {
        for from in Rotation::ALL {
            for to in Rotation::ALL {
                if from.cw() != to && from.ccw() != to {
                    continue;
                }
                let forward = kicks(kind, from, to);
                let reverse = kicks(kind, to, from);
                for &(dx, dy) in forward {
                    assert!(
                        reverse.contains(&(-dx, -dy)),
                        "{kind:?} {from:?}->{to:?}: ({dx}, {dy}) has no reverse"
                    );
                }
            }
        }
    }
}

#[test]
fn srs_plus_tables_are_exposed_verbatim() {
    assert_eq!(
        kicks(PieceKind::T, Rotation::Zero, Rotation::R),
        &[(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)]
    );
    assert_eq!(
        kicks(PieceKind::I, Rotation::Zero, Rotation::R),
        &[(0, 0), (1, 0), (-2, 0), (-2, -1), (1, 2)]
    );
}

#[test]
fn jlstz_180_tables_are_exposed_verbatim() {
    assert_eq!(
        kicks(PieceKind::T, Rotation::Zero, Rotation::Two),
        &[(0, 0), (0, 1), (1, 1), (-1, 1), (1, 0)]
    );
    assert_eq!(
        kicks(PieceKind::T, Rotation::Two, Rotation::Zero),
        &[(0, 0), (0, -1), (-1, -1), (1, -1), (-1, 0)]
    );
    assert_eq!(
        kicks(PieceKind::T, Rotation::R, Rotation::L),
        &[(0, 0), (1, 0), (1, 2), (1, 1), (0, 2)]
    );
    assert_eq!(
        kicks(PieceKind::T, Rotation::L, Rotation::R),
        &[(0, 0), (-1, 0), (-1, 2), (-1, 1), (0, 2)]
    );
}

#[test]
fn i_180_tables_are_exposed_verbatim() {
    assert_eq!(
        kicks(PieceKind::I, Rotation::Zero, Rotation::Two),
        &[(0, 0), (0, 1)]
    );
    assert_eq!(
        kicks(PieceKind::I, Rotation::Two, Rotation::Zero),
        &[(0, 0), (0, -1)]
    );
    assert_eq!(
        kicks(PieceKind::I, Rotation::R, Rotation::L),
        &[(0, 0), (1, 0)]
    );
    assert_eq!(
        kicks(PieceKind::I, Rotation::L, Rotation::R),
        &[(0, 0), (-1, 0)]
    );
}

#[test]
fn fin_or_tst_kick_predicate_matches_spec() {
    assert!(is_fin_or_tst_kick(Rotation::Zero, Rotation::R, (-1, -2)));
    assert!(is_fin_or_tst_kick(Rotation::Two, Rotation::R, (-1, -2)));
    assert!(is_fin_or_tst_kick(Rotation::Zero, Rotation::L, (1, -2)));
    assert!(is_fin_or_tst_kick(Rotation::Two, Rotation::L, (1, -2)));
    assert!(!is_fin_or_tst_kick(Rotation::Zero, Rotation::R, (1, -2)));
    assert!(!is_fin_or_tst_kick(Rotation::R, Rotation::Zero, (-1, -2)));
}
