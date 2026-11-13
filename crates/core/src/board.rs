//! 10-column x 40-row bitboard.
//!
//! Row 0 is the bottom row. Rows 0..19 are the visible playfield; rows 20..39
//! are the hidden buffer above it. Each row is a `u16` mask where bit `x` is
//! column `x`.
//!
//! Garbage cells are tracked in a second bitboard. The invariant
//! `garbage[y] & !occupied[y] == 0` must always hold, so a garbage cell is also
//! an occupied cell. This lets the engine answer "did this clear contain
//! garbage?" without a separate per-cell object.

use crate::piece::{PieceKind, Rotation};

pub const WIDTH: usize = 10;
pub const HEIGHT: usize = 40;
pub const VISIBLE_HEIGHT: usize = 20;
pub const FULL_ROW: u16 = (1u16 << WIDTH) - 1;

/// Contents of a single board cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Empty,
    /// A block placed by a player piece.
    Block,
    /// A garbage block inserted by the referee.
    Garbage,
}

/// Result of removing completed rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClearInfo {
    /// Number of rows removed (0..=4 for a normal lock).
    pub lines: u8,
    /// How many of the removed rows contained at least one garbage cell.
    pub garbage_lines: u8,
    /// True when the clear emptied the board.
    pub perfect_clear: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GarbageError {
    /// More garbage rows than the board can hold.
    TooManyRows(usize),
    /// Hole column outside `0..WIDTH`.
    InvalidHole(u8),
    /// Shifting the stack up would push an occupied cell past row 39.
    Overflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementError {
    /// The piece would overlap a block or leave the board.
    Illegal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoardInvariantError {
    OccupiedBitsOutsideWidth { row: usize },
    GarbageBitsOutsideWidth { row: usize },
    GarbageWithoutBlock { row: usize },
}

/// A 10x40 board with separate occupancy and garbage bitboards.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Board {
    occupied: [u16; HEIGHT],
    garbage: [u16; HEIGHT],
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl Board {
    pub const WIDTH: usize = WIDTH;
    pub const HEIGHT: usize = HEIGHT;
    pub const VISIBLE_HEIGHT: usize = VISIBLE_HEIGHT;
    pub const FULL_ROW: u16 = FULL_ROW;

    pub const fn new() -> Self {
        Self {
            occupied: [0; HEIGHT],
            garbage: [0; HEIGHT],
        }
    }

    /// Builds a board from bottom-up row masks. Garbage bits are not set.
    pub fn from_rows(rows: &[u16]) -> Self {
        assert!(rows.len() <= HEIGHT, "too many rows");
        let mut board = Self::new();
        for (y, &row) in rows.iter().enumerate() {
            board.occupied[y] = row & FULL_ROW;
        }
        board
    }

    pub fn is_empty(&self) -> bool {
        self.occupied.iter().all(|&row| row == 0)
    }

    /// Occupancy mask for row `y` (bit `x` = column `x`).
    pub fn row(&self, y: usize) -> u16 {
        assert!(y < HEIGHT, "row {y} out of bounds");
        self.occupied[y]
    }

    /// Garbage mask for row `y`.
    pub fn garbage_row(&self, y: usize) -> u16 {
        assert!(y < HEIGHT, "row {y} out of bounds");
        self.garbage[y]
    }

    pub fn occupied_at(&self, x: usize, y: usize) -> bool {
        assert!(x < WIDTH && y < HEIGHT, "cell ({x}, {y}) out of bounds");
        self.occupied[y] & (1u16 << x) != 0
    }

    pub fn cell(&self, x: usize, y: usize) -> Cell {
        assert!(x < WIDTH && y < HEIGHT, "cell ({x}, {y}) out of bounds");
        let bit = 1u16 << x;
        if self.garbage[y] & bit != 0 {
            Cell::Garbage
        } else if self.occupied[y] & bit != 0 {
            Cell::Block
        } else {
            Cell::Empty
        }
    }

    /// Sets one cell. Panics outside the board.
    pub fn set_cell(&mut self, x: usize, y: usize, cell: Cell) {
        assert!(x < WIDTH && y < HEIGHT, "cell ({x}, {y}) out of bounds");
        let bit = 1u16 << x;
        match cell {
            Cell::Empty => {
                self.occupied[y] &= !bit;
                self.garbage[y] &= !bit;
            }
            Cell::Block => {
                self.occupied[y] |= bit;
                self.garbage[y] &= !bit;
            }
            Cell::Garbage => {
                self.occupied[y] |= bit;
                self.garbage[y] |= bit;
            }
        }
    }

    /// Replaces a row with normal blocks. Panics outside the board.
    pub fn set_row(&mut self, y: usize, mask: u16) {
        assert!(y < HEIGHT, "row {y} out of bounds");
        self.occupied[y] = mask & FULL_ROW;
        self.garbage[y] = 0;
    }

    /// Replaces a row with garbage cells. Panics outside the board.
    pub fn set_garbage_row(&mut self, y: usize, mask: u16) {
        assert!(y < HEIGHT, "row {y} out of bounds");
        let mask = mask & FULL_ROW;
        self.occupied[y] = mask;
        self.garbage[y] = mask;
    }

    /// True when every cell of the piece is inside the board and unoccupied.
    pub fn can_place(&self, kind: PieceKind, rotation: Rotation, x: i32, y: i32) -> bool {
        for (cx, cy) in kind.cells(rotation) {
            let bx = x + cx;
            let by = y + cy;
            if bx < 0 || bx >= WIDTH as i32 || by < 0 || by >= HEIGHT as i32 {
                return false;
            }
            if self.occupied[by as usize] & (1u16 << bx as usize) != 0 {
                return false;
            }
        }
        true
    }

    /// Places a piece without clearing lines. Fails if the placement is illegal.
    pub fn place(
        &mut self,
        kind: PieceKind,
        rotation: Rotation,
        x: i32,
        y: i32,
    ) -> Result<(), PlacementError> {
        if !self.can_place(kind, rotation, x, y) {
            return Err(PlacementError::Illegal);
        }
        for (cx, cy) in kind.cells(rotation) {
            let bx = (x + cx) as usize;
            let by = (y + cy) as usize;
            self.occupied[by] |= 1u16 << bx;
        }
        Ok(())
    }

    /// Removes all full rows, shifting rows above them down.
    pub fn clear_lines(&mut self) -> ClearInfo {
        let mut cleared = 0u8;
        let mut garbage_lines = 0u8;
        let mut write = 0usize;

        for read in 0..HEIGHT {
            if self.occupied[read] == FULL_ROW {
                cleared += 1;
                if self.garbage[read] != 0 {
                    garbage_lines += 1;
                }
            } else {
                if write != read {
                    self.occupied[write] = self.occupied[read];
                    self.garbage[write] = self.garbage[read];
                }
                write += 1;
            }
        }

        for y in write..HEIGHT {
            self.occupied[y] = 0;
            self.garbage[y] = 0;
        }

        ClearInfo {
            lines: cleared,
            garbage_lines,
            perfect_clear: cleared > 0 && self.is_empty(),
        }
    }

    /// Inserts garbage rows at the bottom, pushing the existing stack up.
    ///
    /// `holes[i]` is the hole column of the i-th inserted row (row `i` from the
    /// bottom). The operation is atomic: on error the board is unchanged.
    pub fn insert_garbage(&mut self, holes: &[u8]) -> Result<(), GarbageError> {
        if holes.is_empty() {
            return Ok(());
        }
        if holes.len() > HEIGHT {
            return Err(GarbageError::TooManyRows(holes.len()));
        }
        for &hole in holes {
            if hole as usize >= WIDTH {
                return Err(GarbageError::InvalidHole(hole));
            }
        }

        let n = holes.len();
        for y in (HEIGHT - n)..HEIGHT {
            if self.occupied[y] != 0 {
                return Err(GarbageError::Overflow);
            }
        }

        for y in (n..HEIGHT).rev() {
            self.occupied[y] = self.occupied[y - n];
            self.garbage[y] = self.garbage[y - n];
        }
        for (i, &hole) in holes.iter().enumerate() {
            let row = FULL_ROW & !(1u16 << hole);
            self.occupied[i] = row;
            self.garbage[i] = row;
        }
        Ok(())
    }

    /// Height of the highest occupied row (0 for an empty board).
    pub fn height(&self) -> usize {
        for y in (0..HEIGHT).rev() {
            if self.occupied[y] != 0 {
                return y + 1;
            }
        }
        0
    }

    /// Height of column `x` (0 for an empty column).
    pub fn column_height(&self, x: usize) -> usize {
        assert!(x < WIDTH, "column {x} out of bounds");
        let bit = 1u16 << x;
        for y in (0..HEIGHT).rev() {
            if self.occupied[y] & bit != 0 {
                return y + 1;
            }
        }
        0
    }

    pub fn count_blocks(&self) -> usize {
        self.occupied
            .iter()
            .map(|row| row.count_ones() as usize)
            .sum()
    }

    pub fn garbage_count(&self) -> usize {
        self.garbage
            .iter()
            .map(|row| row.count_ones() as usize)
            .sum()
    }

    /// Checks the bitboard invariants. Used by tests and debug assertions.
    pub fn check_invariants(&self) -> Result<(), BoardInvariantError> {
        for y in 0..HEIGHT {
            if self.occupied[y] & !FULL_ROW != 0 {
                return Err(BoardInvariantError::OccupiedBitsOutsideWidth { row: y });
            }
            if self.garbage[y] & !FULL_ROW != 0 {
                return Err(BoardInvariantError::GarbageBitsOutsideWidth { row: y });
            }
            if self.garbage[y] & !self.occupied[y] != 0 {
                return Err(BoardInvariantError::GarbageWithoutBlock { row: y });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_board_is_empty() {
        let board = Board::new();
        assert!(board.is_empty());
        assert_eq!(board.height(), 0);
        assert_eq!(board.count_blocks(), 0);
        assert_eq!(board.garbage_count(), 0);
        assert_eq!(board.check_invariants(), Ok(()));
    }

    #[test]
    fn set_cell_roundtrip() {
        let mut board = Board::new();
        board.set_cell(3, 5, Cell::Block);
        assert_eq!(board.cell(3, 5), Cell::Block);
        board.set_cell(3, 5, Cell::Garbage);
        assert_eq!(board.cell(3, 5), Cell::Garbage);
        assert_eq!(board.garbage_count(), 1);
        board.set_cell(3, 5, Cell::Empty);
        assert_eq!(board.cell(3, 5), Cell::Empty);
        assert_eq!(board.garbage_count(), 0);
        assert_eq!(board.check_invariants(), Ok(()));
    }

    #[test]
    fn detects_occupied_bits_outside_width() {
        let mut board = Board::new();
        board.occupied[0] = 1 << 15;
        assert_eq!(
            board.check_invariants(),
            Err(BoardInvariantError::OccupiedBitsOutsideWidth { row: 0 })
        );
    }

    #[test]
    fn detects_garbage_bits_outside_width() {
        let mut board = Board::new();
        board.garbage[2] = 1 << 10;
        assert_eq!(
            board.check_invariants(),
            Err(BoardInvariantError::GarbageBitsOutsideWidth { row: 2 })
        );
    }

    #[test]
    fn detects_garbage_without_block() {
        let mut board = Board::new();
        board.garbage[4] = 1;
        assert_eq!(
            board.check_invariants(),
            Err(BoardInvariantError::GarbageWithoutBlock { row: 4 })
        );
    }

    #[test]
    fn full_row_constant_is_ten_bits() {
        assert_eq!(FULL_ROW, 0b11_1111_1111);
    }
}
