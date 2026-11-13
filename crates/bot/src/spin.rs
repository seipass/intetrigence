//! All-Mini+ spin classification (`docs/s2-spec.md` §4).
//!
//! The classification is a pure function of
//!
//! - the board **before** the lock,
//! - the placement (kind, rotation, x, y),
//! - whether the last input before the lock was a rotation, and
//! - whether that rotation used the FIN/TST 5th kick (`docs/s2-spec.md` §3.3).
//!
//! Keeping it out of the BFS makes it directly testable and lets it be
//! replaced by the core engine's implementation once `crates/core` exposes
//! one; [`intetrigence_core::SpinType`] is already the shared vocabulary.

use intetrigence_core::{ActivePiece, Board, PieceKind, SpinType};

/// T corner offsets in box-local coordinates, i.e. the spec's four corners
/// relative to the T's rotation center `(1, 1)`:
/// `(-1,-1), (+1,-1), (+1,+1), (-1,+1)`.
const T_CORNERS: [(i32, i32); 4] = [(0, 0), (2, 0), (2, 2), (0, 2)];

/// Corner indices that face the direction the T points: **front = nub side**.
///
/// Transcription warning: S7's `cornerTable.t` is y-down
/// (`occupied(x + dx + 1, y - dy - 1)`), so its `dy` must be negated before it
/// is read in this crate's y-up board. `docs/s2-spec.md` v1.3.1 §4.3 carries
/// the corrected table; v1.3 and earlier had the un-negated version, which
/// pointed "front" at the cells *behind* the T and turned every reachable TSD
/// into a mini.
///
/// | rotation | front corners (relative to the centre) |
/// |---|---|
/// | 0 (North) | `(+1,+1)`, `(-1,+1)` |
/// | R (East)  | `(+1,+1)`, `(+1,-1)` |
/// | 2 (South) | `(+1,-1)`, `(-1,-1)` |
/// | L (West)  | `(-1,-1)`, `(-1,+1)` |
///
/// A T-spin double has both nub-side corners filled, so it is a full spin.
const T_FRONT_CORNERS: [[usize; 2]; 4] = [[2, 3], [1, 2], [0, 1], [3, 0]];

/// Whether a corner cell counts as filled.
///
/// The left/right walls and the floor count as filled (`docs/s2-spec.md`
/// §4.3). Above the 40-row board nothing is filled; a legal T placement can
/// never put a corner there, but the rule is spelled out rather than assumed.
#[inline]
fn corner_filled(board: &Board, x: i32, y: i32) -> bool {
    if x < 0 || x >= Board::WIDTH as i32 || y < 0 {
        return true;
    }
    if y >= Board::HEIGHT as i32 {
        return false;
    }
    board.occupied_at(x as usize, y as usize)
}

/// All-spin immobility test (`docs/s2-spec.md` §4.2).
///
/// The piece is immobile when it has no legal placement one cell left, right,
/// up or down. A resting piece can never move down, so in practice this is the
/// overhang (up) test plus the two side tests.
pub fn is_immobile(board: &Board, piece: ActivePiece) -> bool {
    let (kind, rotation, x, y) = (piece.kind, piece.rotation, piece.x, piece.y);
    !board.can_place(kind, rotation, x - 1, y)
        && !board.can_place(kind, rotation, x + 1, y)
        && !board.can_place(kind, rotation, x, y + 1)
        && !board.can_place(kind, rotation, x, y - 1)
}

/// Classifies a placement under All-Mini+ (`docs/s2-spec.md` §4).
///
/// `spin_entry` is the movegen's "the last input before the lock was a
/// rotation" flag; `fin_or_tst` is the §3.3 5th-kick flag of that rotation.
/// Both are ignored when `spin_entry` is false, because a spin needs a
/// rotation as its last input (§4.1).
pub fn classify_spin(
    board: &Board,
    piece: ActivePiece,
    spin_entry: bool,
    fin_or_tst: bool,
) -> SpinType {
    if !spin_entry {
        return SpinType::None;
    }

    if piece.kind != PieceKind::T {
        // Non-T pieces never get a full spin; immobility is the whole test.
        return if is_immobile(board, piece) {
            SpinType::Mini
        } else {
            SpinType::None
        };
    }

    let front = T_FRONT_CORNERS[piece.rotation as usize];
    let mut corners = 0u8;
    let mut front_corners = 0u8;
    for (index, &(dx, dy)) in T_CORNERS.iter().enumerate() {
        if corner_filled(board, piece.x + dx, piece.y + dy) {
            corners += 1;
            if front.contains(&index) {
                front_corners += 1;
            }
        }
    }

    if corners < 3 {
        return SpinType::None;
    }
    if front_corners == 2 || fin_or_tst {
        SpinType::Full
    } else {
        SpinType::Mini
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use intetrigence_core::Rotation;

    fn piece(kind: PieceKind, rotation: Rotation, x: i32, y: i32) -> ActivePiece {
        ActivePiece {
            kind,
            rotation,
            x,
            y,
        }
    }

    /// A T-spin double slot: a 3-wide, 2-deep slot with a 1-wide nub hole and
    /// one taller side so the 3-corner rule is met.
    ///
    /// ```text
    /// row 3: X X X X . . X X X .   <- (3,3) filled, (5,3) empty
    /// row 2: X X X . . . X X X .   <- the T's flat top row
    /// row 1: X X X X . X X X X .   <- the nub hole at column 4
    /// row 0: X X X X X X X X X .
    /// ```
    fn tsd_board() -> Board {
        let mut board = Board::new();
        board.set_row(0, 0x1FF);
        board.set_row(1, 0x1EF);
        board.set_row(2, 0x1C7);
        board.set_row(3, 0x1CF);
        board
    }

    #[test]
    fn tsd_slot_is_a_full_spin() {
        let board = tsd_board();
        let t = piece(PieceKind::T, Rotation::Two, 3, 1);
        assert!(board.can_place(t.kind, t.rotation, t.x, t.y));
        assert!(!board.can_place(t.kind, t.rotation, t.x, t.y - 1));
        // Three corners, both nub-side (front) corners filled.
        assert_eq!(classify_spin(&board, t, true, false), SpinType::Full);
        // Without a rotation as the last input there is no spin at all.
        assert_eq!(classify_spin(&board, t, false, false), SpinType::None);
    }

    #[test]
    fn one_front_corner_is_a_mini_unless_the_fifth_kick_was_used() {
        let mut board = tsd_board();
        // Clear (5,1): the right nub-side corner. (5,3) is filled instead, so
        // the corner count stays at three with one front corner.
        board.set_row(1, 0x1EF & !0x20);
        board.set_row(3, 0x1EF);
        let t = piece(PieceKind::T, Rotation::Two, 3, 1);
        assert_eq!(classify_spin(&board, t, true, false), SpinType::Mini);
        assert_eq!(classify_spin(&board, t, true, true), SpinType::Full);
    }

    #[test]
    fn t_spin_needs_three_corners() {
        let mut board = tsd_board();
        board.set_row(3, 0x1C7); // no upper corners
        let t = piece(PieceKind::T, Rotation::Two, 3, 1);
        assert_eq!(classify_spin(&board, t, true, false), SpinType::None);
        assert_eq!(classify_spin(&board, t, true, true), SpinType::None);
    }

    #[test]
    fn non_t_spin_is_immobile_only() {
        // A 1-wide, 4-deep well with a ceiling: the vertical I cannot move in
        // any direction, so it is a mini spin.
        let mut board = Board::new();
        for y in 0..4 {
            board.set_row(y, 0x1FE); // columns 1-8 filled, 0 and 9 empty
        }
        board.set_row(4, 0x1FF); // ceiling over column 0
        let vertical_i = piece(PieceKind::I, Rotation::R, -2, 0);
        assert!(board.can_place(vertical_i.kind, vertical_i.rotation, -2, 0));
        assert!(is_immobile(&board, vertical_i));
        assert_eq!(
            classify_spin(&board, vertical_i, true, false),
            SpinType::Mini
        );
        assert_eq!(
            classify_spin(&board, vertical_i, false, false),
            SpinType::None
        );

        // The same well without a ceiling lets the piece move up: no spin.
        let mut open = Board::new();
        for y in 0..4 {
            open.set_row(y, 0x1FE);
        }
        assert!(!is_immobile(&open, vertical_i));
        assert_eq!(
            classify_spin(&open, vertical_i, true, false),
            SpinType::None
        );
    }

    #[test]
    fn walls_and_floor_count_as_filled_corners() {
        // T in the left wall at the floor: the (-1, y) corners are walls and
        // the (x, -1) corners are the floor.
        let board = Board::new();
        let t = piece(PieceKind::T, Rotation::R, -1, 0);
        assert!(board.can_place(t.kind, t.rotation, t.x, t.y));
        assert!(corner_filled(&board, -1, 0));
        assert!(corner_filled(&board, 1, -1));
        assert!(!corner_filled(&board, 1, 2));
    }
}
