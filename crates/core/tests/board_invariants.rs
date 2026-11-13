//! Bitboard invariants and row operations.

use intetrigence_core::{Board, Cell, SplitMix64};

#[test]
fn new_board_is_empty_and_consistent() {
    let board = Board::new();
    assert!(board.is_empty());
    assert_eq!(board.height(), 0);
    assert_eq!(board.count_blocks(), 0);
    assert_eq!(board.garbage_count(), 0);
    assert_eq!(board.check_invariants(), Ok(()));
}

#[test]
fn from_rows_is_bottom_up_and_masks_extra_bits() {
    let board = Board::from_rows(&[0b1, 0b10, 0xFFFF]);
    assert_eq!(board.row(0), 1);
    assert_eq!(board.row(1), 2);
    assert_eq!(board.row(2), Board::FULL_ROW);
    assert_eq!(board.height(), 3);
    assert_eq!(board.check_invariants(), Ok(()));
}

#[test]
fn set_row_masks_bits_outside_the_width() {
    let mut board = Board::new();
    board.set_row(0, 0xFFFF);
    assert_eq!(board.row(0), Board::FULL_ROW);
    assert_eq!(board.garbage_row(0), 0);
    assert_eq!(board.check_invariants(), Ok(()));
}

#[test]
fn set_garbage_row_marks_all_cells_as_garbage() {
    let mut board = Board::new();
    board.set_garbage_row(0, 0xFFFF);
    assert_eq!(board.row(0), Board::FULL_ROW);
    assert_eq!(board.garbage_row(0), Board::FULL_ROW);
    assert_eq!(board.cell(0, 0), Cell::Garbage);
    assert_eq!(board.cell(9, 0), Cell::Garbage);
    assert_eq!(board.check_invariants(), Ok(()));
}

#[test]
fn height_and_column_height_track_the_highest_cell() {
    let mut board = Board::new();
    board.set_cell(3, 5, Cell::Block);
    board.set_cell(7, 2, Cell::Block);

    assert_eq!(board.height(), 6);
    assert_eq!(board.column_height(3), 6);
    assert_eq!(board.column_height(7), 3);
    assert_eq!(board.column_height(0), 0);
}

#[test]
fn garbage_is_always_a_subset_of_occupancy() {
    let mut board = Board::new();
    board.set_cell(1, 0, Cell::Garbage);
    board.set_cell(2, 0, Cell::Block);
    board.set_cell(3, 0, Cell::Empty);

    for y in 0..Board::HEIGHT {
        assert_eq!(board.garbage_row(y) & !board.row(y), 0, "row {y}");
    }
    assert_eq!(board.check_invariants(), Ok(()));
}

#[test]
fn line_clear_shifts_garbage_with_its_row() {
    let mut board = Board::new();
    board.set_row(0, Board::FULL_ROW);
    board.set_cell(5, 2, Cell::Garbage);

    let clear = board.clear_lines();
    assert_eq!(clear.lines, 1);
    assert_eq!(board.cell(5, 1), Cell::Garbage);
    assert_eq!(board.cell(5, 2), Cell::Empty);
    assert_eq!(board.check_invariants(), Ok(()));
}

#[test]
fn line_clear_counts_garbage_rows() {
    let mut board = Board::new();
    board.set_garbage_row(0, Board::FULL_ROW);
    board.set_garbage_row(1, Board::FULL_ROW);

    let clear = board.clear_lines();
    assert_eq!(clear.lines, 2);
    assert_eq!(clear.garbage_lines, 2);
    assert!(clear.perfect_clear);
    assert!(board.is_empty());
}

#[test]
fn line_clear_leaves_no_full_rows() {
    let mut board = Board::new();
    board.set_row(0, Board::FULL_ROW);
    board.set_row(1, Board::FULL_ROW);
    board.set_row(3, Board::FULL_ROW);

    let clear = board.clear_lines();
    assert_eq!(clear.lines, 3);
    for y in 0..Board::HEIGHT {
        assert_ne!(board.row(y), Board::FULL_ROW, "row {y}");
    }
    assert_eq!(board.check_invariants(), Ok(()));
}

#[test]
fn random_row_operations_keep_invariants() {
    let mut rng = SplitMix64::new(0x0BAD_F00D_1234_5678);

    for _ in 0..200 {
        let mut board = Board::new();
        for _ in 0..100 {
            match rng.below(6) {
                0 => {
                    let y = rng.below(40) as usize;
                    let mask = rng.below(1 << 10) as u16;
                    board.set_row(y, mask);
                }
                1 => {
                    let y = rng.below(40) as usize;
                    let mask = rng.below(1 << 10) as u16;
                    board.set_garbage_row(y, mask);
                }
                2 => {
                    let x = rng.below(10) as usize;
                    let y = rng.below(40) as usize;
                    board.set_cell(x, y, Cell::Block);
                }
                3 => {
                    let x = rng.below(10) as usize;
                    let y = rng.below(40) as usize;
                    board.set_cell(x, y, Cell::Garbage);
                }
                4 => {
                    let x = rng.below(10) as usize;
                    let y = rng.below(40) as usize;
                    board.set_cell(x, y, Cell::Empty);
                }
                _ => {
                    let count = rng.below(3) as usize + 1;
                    let holes: Vec<u8> = (0..count).map(|_| rng.below(10) as u8).collect();
                    let _ = board.insert_garbage(&holes);
                }
            }
            board.clear_lines();
            assert_eq!(board.check_invariants(), Ok(()));
        }
    }
}
