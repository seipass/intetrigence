//! Tetromino definitions using SRS "true rotation".
//!
//! # Coordinate system
//!
//! Board coordinates are `x` right, `y` up, with `(0, 0)` at the bottom-left
//! cell. A piece is positioned by the bottom-left corner of its bounding box:
//!
//! - I: 4x4 box
//! - O: 2x2 box
//! - J, L, S, T, Z: 3x3 box
//!
//! Local cell coordinates are cell indices inside that box (`0..size-1`), with
//! `y` pointing up. The SRS true-rotation center is the center of the bounding
//! box:
//!
//! - 3x3 (JLSTZ): `(1.0, 1.0)` in cell-index coordinates
//! - 4x4 (I): `(1.5, 1.5)` in cell-index coordinates (a grid intersection)
//! - 2x2 (O): `(0.5, 0.5)` in cell-index coordinates
//!
//! Equivalently, in corner coordinates where the box spans `[0, size]^2`, the
//! center is `(size/2, size/2)`. Rotating a cell `(x, y)` 90 degrees clockwise
//! about that center is the exact integer map `(x, y) -> (y, size-1-x)`.
//!
//! # Spawn
//!
//! Every piece spawns with its lowest occupied cell in row 21, the second
//! hidden row above the 20 visible rows (TETR.IO S2, `docs/s2-spec.md` §2.2.1).
//! The box corner is therefore `(3, 19)` for I, `(4, 21)` for O, and `(3, 20)`
//! for JLSTZ.

/// The seven tetromino kinds.
///
/// The discriminants and the order of [`ALL_PIECES`] are part of the seeded
/// 7-bag protocol. Do not reorder them without versioning the seed format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum PieceKind {
    I = 0,
    O = 1,
    T = 2,
    S = 3,
    Z = 4,
    J = 5,
    L = 6,
}

/// Canonical piece order used by the 7-bag.
pub const ALL_PIECES: [PieceKind; 7] = [
    PieceKind::I,
    PieceKind::O,
    PieceKind::T,
    PieceKind::S,
    PieceKind::Z,
    PieceKind::J,
    PieceKind::L,
];

impl PieceKind {
    pub const fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0 => PieceKind::I,
            1 => PieceKind::O,
            2 => PieceKind::T,
            3 => PieceKind::S,
            4 => PieceKind::Z,
            5 => PieceKind::J,
            6 => PieceKind::L,
            _ => return None,
        })
    }

    pub const fn index(self) -> usize {
        self as usize
    }

    /// Side length of the SRS bounding box.
    pub const fn box_size(self) -> i32 {
        match self {
            PieceKind::I => 4,
            PieceKind::O => 2,
            _ => 3,
        }
    }

    /// SRS true-rotation center in cell-index coordinates.
    pub const fn rotation_center(self) -> (f64, f64) {
        let c = (self.box_size() as f64 - 1.0) / 2.0;
        (c, c)
    }

    /// Bottom-left corner of the bounding box at spawn.
    ///
    /// The lowest occupied cell is row 21 for every piece
    /// (`docs/s2-spec.md` §2.2.1).
    pub const fn spawn_position(self) -> (i32, i32) {
        match self {
            PieceKind::I => (3, 19),
            PieceKind::O => (4, 21),
            _ => (3, 20),
        }
    }

    /// Spawn-orientation cells in box-local coordinates.
    const fn spawn_cells(self) -> [(i32, i32); 4] {
        match self {
            PieceKind::I => [(0, 2), (1, 2), (2, 2), (3, 2)],
            PieceKind::O => [(0, 0), (1, 0), (0, 1), (1, 1)],
            PieceKind::T => [(1, 2), (0, 1), (1, 1), (2, 1)],
            PieceKind::S => [(1, 2), (2, 2), (0, 1), (1, 1)],
            PieceKind::Z => [(0, 2), (1, 2), (1, 1), (2, 1)],
            PieceKind::J => [(0, 2), (0, 1), (1, 1), (2, 1)],
            PieceKind::L => [(2, 2), (0, 1), (1, 1), (2, 1)],
        }
    }

    /// Cells for `rotation`, in box-local coordinates (y up).
    ///
    /// Rotations are generated from the spawn cells by exact 90-degree
    /// clockwise true-rotation steps, so the table and the documented center
    /// cannot drift apart.
    pub const fn cells(self, rotation: Rotation) -> [(i32, i32); 4] {
        let mut cells = self.spawn_cells();
        let max = self.box_size() - 1;
        let mut step = 0;
        while step < rotation as u8 {
            let mut i = 0;
            while i < cells.len() {
                let (x, y) = cells[i];
                cells[i] = (y, max - x);
                i += 1;
            }
            step += 1;
        }
        cells
    }
}

/// SRS rotation state. `Zero` is spawn, then one 90-degree clockwise step per
/// discriminant increment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Rotation {
    Zero = 0,
    R = 1,
    Two = 2,
    L = 3,
}

impl Rotation {
    pub const ALL: [Rotation; 4] = [Rotation::Zero, Rotation::R, Rotation::Two, Rotation::L];

    pub const fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0 => Rotation::Zero,
            1 => Rotation::R,
            2 => Rotation::Two,
            3 => Rotation::L,
            _ => return None,
        })
    }

    pub const fn cw(self) -> Self {
        match self {
            Rotation::Zero => Rotation::R,
            Rotation::R => Rotation::Two,
            Rotation::Two => Rotation::L,
            Rotation::L => Rotation::Zero,
        }
    }

    pub const fn ccw(self) -> Self {
        match self {
            Rotation::Zero => Rotation::L,
            Rotation::R => Rotation::Zero,
            Rotation::Two => Rotation::R,
            Rotation::L => Rotation::Two,
        }
    }

    pub const fn flip(self) -> Self {
        match self {
            Rotation::Zero => Rotation::Two,
            Rotation::R => Rotation::L,
            Rotation::Two => Rotation::Zero,
            Rotation::L => Rotation::R,
        }
    }

    pub const fn step(self, direction: RotateDirection) -> Self {
        match direction {
            RotateDirection::Cw => self.cw(),
            RotateDirection::Ccw => self.ccw(),
            RotateDirection::Half => self.flip(),
        }
    }
}

/// Requested rotation direction for a single input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RotateDirection {
    Cw,
    Ccw,
    Half,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_centers_are_explicit() {
        assert_eq!(PieceKind::T.rotation_center(), (1.0, 1.0));
        assert_eq!(PieceKind::J.rotation_center(), (1.0, 1.0));
        assert_eq!(PieceKind::I.rotation_center(), (1.5, 1.5));
        assert_eq!(PieceKind::O.rotation_center(), (0.5, 0.5));
    }

    #[test]
    fn four_rotations_return_to_spawn() {
        for kind in ALL_PIECES {
            let spawn = kind.cells(Rotation::Zero);
            let mut rotation = Rotation::Zero;
            for _ in 0..4 {
                rotation = rotation.cw();
            }
            assert_eq!(rotation, Rotation::Zero);
            assert_eq!(kind.cells(rotation), spawn);
        }
    }

    #[test]
    fn two_half_rotations_return_to_spawn() {
        for kind in ALL_PIECES {
            let spawn = kind.cells(Rotation::Zero);
            assert_eq!(kind.cells(Rotation::Zero.flip().flip()), spawn, "{kind:?}");
        }
    }

    #[test]
    fn spawn_lowest_cell_is_row_twenty_one() {
        for kind in ALL_PIECES {
            let (x, y) = kind.spawn_position();
            let lowest = kind
                .cells(Rotation::Zero)
                .iter()
                .map(|&(_, cy)| y + cy)
                .min()
                .unwrap();
            assert_eq!(lowest, 21, "{kind:?} spawn row");
            assert!((0..10).contains(&x));
        }
    }
}
