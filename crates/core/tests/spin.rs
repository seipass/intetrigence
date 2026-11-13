//! All-Mini+ spin classification (`docs/s2-spec.md` §4, v1.3.1).

use intetrigence_core::{Board, Cell, PieceKind, Rotation, SpinType};

#[test]
fn t_front_corners_follow_the_nub_for_all_rotations() {
    // T at (2, 0); corners are (2,0), (4,0), (4,2), (2,2).
    // (rotation, front pair, back pair)
    let cases = [
        (Rotation::Zero, [(4, 2), (2, 2)], [(2, 0), (4, 0)]), // nub up
        (Rotation::R, [(4, 0), (4, 2)], [(2, 0), (2, 2)]),    // nub right
        (Rotation::Two, [(2, 0), (4, 0)], [(2, 2), (4, 2)]),  // nub down
        (Rotation::L, [(2, 0), (2, 2)], [(4, 0), (4, 2)]),    // nub left
    ];

    for (rotation, front, back) in cases {
        // Full: both front corners + one back corner.
        let mut board = Board::new();
        for &(x, y) in &front {
            board.set_cell(x, y, Cell::Block);
        }
        board.set_cell(back[0].0, back[0].1, Cell::Block);

        assert_eq!(
            board.t_spin_corners(rotation, 2, 0),
            (3, 2),
            "{rotation:?} full corners"
        );
        assert_eq!(
            board.spin_type(PieceKind::T, rotation, 2, 0, true, false),
            SpinType::Full,
            "{rotation:?} full"
        );
        assert_eq!(
            board.spin_type(PieceKind::T, rotation, 2, 0, false, false),
            SpinType::None,
            "{rotation:?} last action was not a rotation"
        );

        // Mini: one front corner + both back corners.
        let mut board = Board::new();
        board.set_cell(front[0].0, front[0].1, Cell::Block);
        for &(x, y) in &back {
            board.set_cell(x, y, Cell::Block);
        }

        assert_eq!(
            board.t_spin_corners(rotation, 2, 0),
            (3, 1),
            "{rotation:?} mini corners"
        );
        assert_eq!(
            board.spin_type(PieceKind::T, rotation, 2, 0, true, false),
            SpinType::Mini,
            "{rotation:?} mini"
        );
        assert_eq!(
            board.spin_type(PieceKind::T, rotation, 2, 0, true, true),
            SpinType::Full,
            "{rotation:?} fin/tst promotion"
        );
    }
}

#[test]
fn t_corners_below_three_are_not_a_spin() {
    let mut board = Board::new();
    board.set_cell(2, 0, Cell::Block);
    board.set_cell(4, 0, Cell::Block);

    assert_eq!(board.t_spin_corners(Rotation::Two, 2, 0), (2, 2));
    assert_eq!(
        board.spin_type(PieceKind::T, Rotation::Two, 2, 0, true, false),
        SpinType::None
    );
    // fin_or_tst does not bypass the 3-corner requirement.
    assert_eq!(
        board.spin_type(PieceKind::T, Rotation::Two, 2, 0, true, true),
        SpinType::None
    );
}

#[test]
fn walls_count_as_filled_but_the_top_is_open() {
    // T R at x = -1: the two left corners are the wall. Fill both right
    // corners to get 4 filled corners and 2 front corners.
    let mut board = Board::new();
    board.set_cell(1, 0, Cell::Block);
    board.set_cell(1, 2, Cell::Block);
    assert_eq!(board.t_spin_corners(Rotation::R, -1, 0), (4, 2));
    assert_eq!(
        board.spin_type(PieceKind::T, Rotation::R, -1, 0, true, false),
        SpinType::Full
    );

    // T Two at the top: the two upper corners are above row 39 and open.
    let mut board = Board::new();
    board.set_cell(2, 38, Cell::Block);
    board.set_cell(4, 38, Cell::Block);
    assert_eq!(board.t_spin_corners(Rotation::Two, 2, 38), (2, 2));
    assert_eq!(
        board.spin_type(PieceKind::T, Rotation::Two, 2, 38, true, false),
        SpinType::None
    );
}

#[test]
fn non_t_immobile_spin_is_mini() {
    let mut board = Board::new();
    board.set_cell(3, 0, Cell::Block);
    board.set_cell(6, 0, Cell::Block);
    board.set_cell(4, 2, Cell::Block);

    assert!(board.is_immobile(PieceKind::O, Rotation::Zero, 4, 0));
    assert_eq!(
        board.spin_type(PieceKind::O, Rotation::Zero, 4, 0, true, false),
        SpinType::Mini
    );
    assert_eq!(
        board.spin_type(PieceKind::O, Rotation::Zero, 4, 0, false, false),
        SpinType::None
    );
}

#[test]
fn non_t_piece_in_the_open_is_not_a_spin() {
    let board = Board::new();
    assert!(!board.is_immobile(PieceKind::O, Rotation::Zero, 4, 0));
    assert_eq!(
        board.spin_type(PieceKind::O, Rotation::Zero, 4, 0, true, false),
        SpinType::None
    );
}
