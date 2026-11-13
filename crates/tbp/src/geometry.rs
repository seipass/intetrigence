//! Coordinate conversion between the TBP / Cold Clear location convention and
//! `crates/core` placements.
//!
//! TBP (`docs/attax-integration.md` §4.2): `location = {type, orientation, x, y}`
//! where `(x, y)` is the **SRS true rotation centre** and the occupied cells are
//! `CC_CELLS[type]` rotated by `orientation` and translated by `(x, y)`.
//!
//! `crates/core`: a placement is `(rotation, x, y)` where `(x, y)` is the
//! **bottom-left corner of the SRS bounding box** (`docs/s2-referee-protocol.md`
//! §4.1). Both describe the same cell set; this module converts by matching
//! cell sets, so it cannot drift if a piece table changes.

use intetrigence_core::{PieceKind, Rotation};

use crate::message::{Location, Orientation, Piece};

/// Cold Clear north-orientation cells, relative to the true rotation centre
/// (`docs/attax-integration.md` §4.2).
pub const fn cc_cells(piece: Piece) -> [(i32, i32); 4] {
    match piece {
        Piece::I => [(-1, 0), (0, 0), (1, 0), (2, 0)],
        Piece::O => [(0, 0), (1, 0), (0, 1), (1, 1)],
        Piece::T => [(-1, 0), (0, 0), (1, 0), (0, 1)],
        Piece::L => [(-1, 0), (0, 0), (1, 0), (1, 1)],
        Piece::J => [(-1, 0), (0, 0), (1, 0), (-1, 1)],
        Piece::S => [(-1, 0), (0, 0), (0, 1), (1, 1)],
        Piece::Z => [(-1, 1), (0, 1), (0, 0), (1, 0)],
    }
}

/// Rotates a centre-relative cell into `orientation` (y up, clockwise).
pub const fn rotate_cell(orientation: Orientation, (x, y): (i32, i32)) -> (i32, i32) {
    match orientation {
        Orientation::North => (x, y),
        Orientation::East => (y, -x),
        Orientation::South => (-x, -y),
        Orientation::West => (-y, x),
    }
}

/// Absolute cells of a TBP location, sorted lexicographically.
pub fn location_cells(location: &Location) -> [(i32, i32); 4] {
    let mut cells = cc_cells(location.kind)
        .map(|cell| rotate_cell(location.orientation, cell))
        .map(|(x, y)| (x + location.x, y + location.y));
    cells.sort_unstable();
    cells
}

/// Sorts a cell set into the canonical order used as a map key.
pub fn cells_key(cells: [(i32, i32); 4]) -> [(i32, i32); 4] {
    let mut key = cells;
    key.sort_unstable();
    key
}

/// Absolute cells of a core placement, sorted lexicographically.
pub fn core_cells(kind: PieceKind, rotation: Rotation, x: i32, y: i32) -> [(i32, i32); 4] {
    cells_key(kind.cells(rotation).map(|(cx, cy)| (cx + x, cy + y)))
}

/// Core rotation corresponding to a TBP orientation (same SRS state order).
pub const fn rotation_of(orientation: Orientation) -> Rotation {
    match orientation {
        Orientation::North => Rotation::Zero,
        Orientation::East => Rotation::R,
        Orientation::South => Rotation::Two,
        Orientation::West => Rotation::L,
    }
}

/// TBP orientation corresponding to a core rotation.
pub const fn orientation_of(rotation: Rotation) -> Orientation {
    match rotation {
        Rotation::Zero => Orientation::North,
        Rotation::R => Orientation::East,
        Rotation::Two => Orientation::South,
        Rotation::L => Orientation::West,
    }
}

/// Resolves a TBP location to a core `(rotation, x, y)` placement.
///
/// `preferred` is tried first so a cell set that is a translation of another
/// orientation (I, O, S, Z) still resolves to the orientation the bot meant.
pub fn core_placement(
    kind: PieceKind,
    cells: [(i32, i32); 4],
    preferred: Option<Orientation>,
) -> Option<(Rotation, i32, i32)> {
    let target = cells_key(cells);
    let mut rotations = Vec::with_capacity(4);
    if let Some(orientation) = preferred {
        rotations.push(rotation_of(orientation));
    }
    for rotation in Rotation::ALL {
        if !rotations.contains(&rotation) {
            rotations.push(rotation);
        }
    }
    for rotation in rotations {
        let local = kind.cells(rotation);
        let min_local_x = local.iter().map(|cell| cell.0).min()?;
        let min_local_y = local.iter().map(|cell| cell.1).min()?;
        let min_x = target.iter().map(|cell| cell.0).min()?;
        let min_y = target.iter().map(|cell| cell.1).min()?;
        let x = min_x - min_local_x;
        let y = min_y - min_local_y;
        if core_cells(kind, rotation, x, y) == target {
            return Some((rotation, x, y));
        }
    }
    None
}

/// Converts a core placement into the TBP location with the matching
/// orientation.
pub fn location_of_core(kind: PieceKind, rotation: Rotation, x: i32, y: i32) -> Location {
    let target = core_cells(kind, rotation, x, y);
    let piece = Piece::from_core(kind);
    let mut orientations = Vec::with_capacity(4);
    orientations.push(orientation_of(rotation));
    for orientation in Orientation::ALL {
        if !orientations.contains(&orientation) {
            orientations.push(orientation);
        }
    }
    for orientation in orientations {
        let mut cc = cc_cells(piece)
            .map(|cell| rotate_cell(orientation, cell))
            .to_vec();
        cc.sort_unstable();
        let (dx, dy) = (target[0].0 - cc[0].0, target[0].1 - cc[0].1);
        let mut translated = [(0, 0); 4];
        for (index, cell) in cc.iter().enumerate() {
            translated[index] = (cell.0 + dx, cell.1 + dy);
        }
        translated.sort_unstable();
        if translated == target {
            return Location {
                kind: piece,
                orientation,
                x: dx,
                y: dy,
            };
        }
    }
    unreachable!(
        "every core placement has a TBP location: {kind:?} {rotation:?} ({x}, {y}) -> {target:?}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_cells_match_the_spec_table() {
        // docs/s2-spec.md §2.2.1: every piece's lowest cell is row 21.
        for piece in Piece::ALL {
            let kind = piece.to_core();
            let (x, y) = kind.spawn_position();
            let cells = core_cells(kind, Rotation::Zero, x, y);
            assert_eq!(
                cells.iter().map(|cell| cell.1).min(),
                Some(21),
                "{piece:?} spawn must be row 21"
            );
        }
    }

    #[test]
    fn every_core_placement_roundtrips_through_tbp() {
        for kind in intetrigence_core::ALL_PIECES {
            for rotation in Rotation::ALL {
                for x in -3..=10 {
                    for y in -2..=40 {
                        // Only placements whose cells are inside the board can
                        // be represented by both sides.
                        let cells = core_cells(kind, rotation, x, y);
                        if cells
                            .iter()
                            .any(|cell| cell.0 < 0 || cell.0 > 9 || cell.1 < 0 || cell.1 > 39)
                        {
                            continue;
                        }
                        let location = location_of_core(kind, rotation, x, y);
                        assert_eq!(location_cells(&location), cells);
                        let resolved =
                            core_placement(kind, cells, Some(location.orientation)).unwrap();
                        assert_eq!(resolved, (rotation, x, y), "{kind:?} {cells:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn tbp_primary_orientation_is_preferred() {
        // S north and S south describe the same cell set with different
        // centres; the preferred orientation must win.
        let north = Location {
            kind: Piece::S,
            orientation: Orientation::North,
            x: 4,
            y: 1,
        };
        let cells = location_cells(&north);
        let (rotation, _, _) = core_placement(Piece::S.to_core(), cells, Some(north.orientation))
            .expect("S must resolve");
        assert_eq!(rotation, Rotation::Zero);
        let south = Location {
            kind: Piece::S,
            orientation: Orientation::South,
            x: 4,
            y: 1,
        };
        let cells = location_cells(&south);
        let (rotation, _, _) = core_placement(Piece::S.to_core(), cells, Some(south.orientation))
            .expect("S must resolve");
        assert_eq!(rotation, Rotation::Two);
    }
}
