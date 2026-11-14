//! Line clearing and garbage-row insertion consistency.

use intetrigence_core::{
    ActivePiece, Board, Cell, Game, GarbageError, PieceKind, Rotation, SplitMix64,
};

#[test]
fn single_line_clear_and_perfect_clear() {
    let mut board = Board::new();
    for x in 4..10 {
        board.set_cell(x, 0, Cell::Block);
    }

    let active = ActivePiece {
        kind: PieceKind::I,
        rotation: Rotation::Zero,
        x: 0,
        y: -2,
    };
    let mut game = Game::from_position(0, board, active).unwrap();

    let result = game.lock().unwrap();
    assert_eq!(result.clear.lines, 1);
    assert_eq!(result.clear.garbage_lines, 0);
    assert!(result.clear.perfect_clear);
    assert!(game.board().is_empty());
    assert_eq!(game.stats().lines_cleared, 1);
}

#[test]
fn quad_clear_and_perfect_clear() {
    let mut board = Board::new();
    for y in 0..4 {
        for x in 1..10 {
            board.set_cell(x, y, Cell::Block);
        }
    }

    let active = ActivePiece {
        kind: PieceKind::I,
        rotation: Rotation::R,
        x: -2,
        y: 0,
    };
    let mut game = Game::from_position(0, board, active).unwrap();

    let result = game.lock().unwrap();
    assert_eq!(result.clear.lines, 4);
    assert!(result.clear.perfect_clear);
    assert!(game.board().is_empty());
    assert_eq!(game.stats().lines_cleared, 4);
}

#[test]
fn clear_with_garbage_sets_the_garbage_line_count() {
    let mut board = Board::new();
    for x in 4..10 {
        board.set_cell(x, 0, Cell::Garbage);
    }

    let active = ActivePiece {
        kind: PieceKind::I,
        rotation: Rotation::Zero,
        x: 0,
        y: -2,
    };
    let mut game = Game::from_position(0, board, active).unwrap();

    let result = game.lock().unwrap();
    assert_eq!(result.clear.lines, 1);
    assert_eq!(result.clear.garbage_lines, 1);
    assert!(result.clear.perfect_clear);
    assert_eq!(game.stats().garbage_lines_cleared, 1);
}

#[test]
fn garbage_insertion_shifts_the_stack_and_sets_holes() {
    let mut board = Board::new();
    board.set_cell(0, 0, Cell::Block);

    board.insert_garbage(&[3, 7]).unwrap();

    // The existing block moved up by two rows.
    assert_eq!(board.cell(0, 2), Cell::Block);

    // Insertion order is bottom-up: the first hole (3) is pushed to the top
    // row, the last hole (7) stays on the bottom row.
    assert_eq!(board.cell(0, 0), Cell::Garbage);
    assert_eq!(board.cell(7, 0), Cell::Empty);
    assert_eq!(board.cell(3, 0), Cell::Garbage);
    assert_eq!(board.cell(3, 1), Cell::Empty);
    assert_eq!(board.cell(7, 1), Cell::Garbage);

    assert_eq!(board.height(), 3);
    assert_eq!(board.count_blocks(), 1 + 2 * 9);
    assert_eq!(board.garbage_count(), 2 * 9);
    assert_eq!(board.check_invariants(), Ok(()));
}

#[test]
fn garbage_batch_hole_order_matches_one_by_one_insertion() {
    let mut board = Board::new();
    board.insert_garbage(&[1, 2, 3]).unwrap();

    // The last inserted row (hole 3) is at the bottom; the first (hole 1) was
    // pushed to the top.
    assert_eq!(board.cell(3, 0), Cell::Empty);
    assert_eq!(board.cell(2, 1), Cell::Empty);
    assert_eq!(board.cell(1, 2), Cell::Empty);
    assert_eq!(board.cell(1, 0), Cell::Garbage);
    assert_eq!(board.cell(2, 0), Cell::Garbage);
    assert_eq!(board.cell(3, 1), Cell::Garbage);
}

#[test]
fn garbage_insertion_preserves_existing_garbage_markers() {
    let mut board = Board::new();
    board.set_cell(5, 0, Cell::Garbage);

    board.insert_garbage(&[2]).unwrap();

    assert_eq!(board.cell(5, 1), Cell::Garbage);
    assert_eq!(board.cell(2, 0), Cell::Empty);
    assert_eq!(board.garbage_count(), 1 + 9);
    assert_eq!(board.check_invariants(), Ok(()));
}

#[test]
fn garbage_overflow_discards_rows_above_39() {
    let mut board = Board::new();
    board.set_cell(0, 39, Cell::Block);

    board.insert_garbage(&[0]).unwrap();

    // The old row 39 is pushed out and discarded; the new row 0 is garbage
    // with its hole at column 0.
    assert_eq!(board.cell(0, 39), Cell::Empty);
    assert_eq!(board.cell(0, 0), Cell::Empty);
    assert_eq!(board.cell(1, 0), Cell::Garbage);
    assert_eq!(board.garbage_count(), 9);
    assert_eq!(board.check_invariants(), Ok(()));
}

#[test]
fn garbage_invalid_hole_is_atomic() {
    let mut board = Board::new();
    board.set_cell(0, 0, Cell::Block);
    let before = board.clone();

    assert_eq!(
        board.insert_garbage(&[10]),
        Err(GarbageError::InvalidHole(10))
    );
    assert_eq!(board, before);
}

#[test]
fn garbage_too_many_rows_is_rejected() {
    let mut board = Board::new();
    assert_eq!(
        board.insert_garbage(&[0; 41]),
        Err(GarbageError::TooManyRows(41))
    );
}

#[test]
fn empty_garbage_insertion_is_a_noop() {
    let mut board = Board::new();
    board.set_cell(0, 0, Cell::Block);
    let before = board.clone();

    assert_eq!(board.insert_garbage(&[]), Ok(()));
    assert_eq!(board, before);
}

#[test]
fn clearing_a_garbage_row_reports_the_bonus() {
    let mut board = Board::new();
    board.insert_garbage(&[0]).unwrap();
    board.set_cell(0, 0, Cell::Block);

    let clear = board.clear_lines();
    assert_eq!(clear.lines, 1);
    assert_eq!(clear.garbage_lines, 1);
    assert!(clear.perfect_clear);
    assert!(board.is_empty());
}

#[test]
fn game_garbage_insertion_respawns_the_active_piece_at_spawn() {
    let mut game = Game::new(0);
    let before = game.active().unwrap();

    game.insert_garbage(&[0, 1]).unwrap();

    let after = game.active().unwrap();
    assert_eq!(after.kind, before.kind);
    assert_eq!((after.x, after.y), after.kind.spawn_position());
    assert_eq!(after.rotation, Rotation::Zero);
    assert_eq!(game.stats().garbage_lines_received, 2);
    assert_eq!(game.check_invariants(), Ok(()));
}

#[test]
fn game_garbage_insertion_after_lock_keeps_next_piece_at_spawn() {
    let mut game = Game::new(0);
    game.hard_drop().unwrap();
    let next = game.active().unwrap();

    game.insert_garbage(&[0, 1, 2]).unwrap();

    let after = game.active().unwrap();
    assert_eq!(after.kind, next.kind);
    assert_eq!((after.x, after.y), after.kind.spawn_position());
    assert_eq!(after.rotation, Rotation::Zero);
    assert_eq!(game.check_invariants(), Ok(()));
}

#[test]
fn game_garbage_insertion_tops_out_when_the_spawn_is_blocked() {
    // The block at (3, 20) is below the T spawn; after one garbage row it
    // shifts to (3, 21), which is a T spawn cell.
    let mut board = Board::new();
    board.set_cell(3, 20, Cell::Block);
    let active = ActivePiece {
        kind: PieceKind::T,
        rotation: Rotation::Zero,
        x: 3,
        y: 20,
    };
    let mut game = Game::from_position(0, board, active).unwrap();

    game.insert_garbage(&[0]).unwrap();

    assert!(game.is_over());
    assert_eq!(game.check_invariants(), Ok(()));
}

#[test]
fn game_garbage_insertion_does_not_apply_clutch() {
    // The standard T spawn is blocked after insertion, but the one-row-up
    // position is free. Clutch only applies after a line clear; garbage
    // insertion only happens after a non-clearing lock, so this must top out.
    let mut board = Board::new();
    board.set_cell(3, 20, Cell::Block);
    let active = ActivePiece {
        kind: PieceKind::T,
        rotation: Rotation::Zero,
        x: 3,
        y: 20,
    };
    let mut game = Game::from_position(0, board, active).unwrap();

    game.insert_garbage(&[0]).unwrap();

    assert!(game.is_over());
    let pushed = ActivePiece {
        kind: PieceKind::T,
        rotation: Rotation::Zero,
        x: 3,
        y: 21,
    };
    assert!(
        game.board()
            .can_place(pushed.kind, pushed.rotation, pushed.x, pushed.y),
        "the push-up position is free; only the no-Clutch rule explains the top-out"
    );
}

#[test]
fn random_garbage_and_clears_keep_board_invariants() {
    let mut rng = SplitMix64::new(0xBEEF_CAFE_1234_5678);

    for _ in 0..100 {
        let mut board = Board::new();
        for _ in 0..50 {
            let count = rng.below(4) as usize + 1;
            let holes: Vec<u8> = (0..count).map(|_| rng.below(10) as u8).collect();
            let _ = board.insert_garbage(&holes);

            for _ in 0..10 {
                let x = rng.below(10) as usize;
                let y = rng.below(40) as usize;
                let cell = match rng.below(3) {
                    0 => Cell::Empty,
                    1 => Cell::Block,
                    _ => Cell::Garbage,
                };
                board.set_cell(x, y, cell);
            }

            board.clear_lines();
            assert_eq!(board.check_invariants(), Ok(()));
        }
    }
}
