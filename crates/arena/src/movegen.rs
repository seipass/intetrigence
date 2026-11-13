//! Reachability movegen adapter over [`intetrigence_bot::movegen`].
//!
//! The referee must verify that a bot's placement is reachable from the current
//! spawn with shift / soft drop / SRS+ rotation (including 180°) and that it is
//! grounded (`docs/s2-referee-protocol.md` §4.2). It also has to *apply* the
//! placement through `crates/core` without a "teleport" API, so the movegen
//! returns a concrete action path that is replayed with [`Game::apply`].
//!
//! The BFS itself lives in `crates/bot` (bot-dev): one implementation, shared
//! by the search and the referee, so the two cannot drift. This module only
//! adapts the bot's `Placement` (per `(rotation, x, y)`) into the referee's
//! cell-set-keyed [`MoveSet`] and keeps the pre-lock action path.
//!
//! Spin classification is the bot's (which delegates to
//! `Board::spin_type`): a spin requires the last action to be a rotation, and
//! the 5th-kick FIN/TST predicate depends on the kick used. Cell-identical
//! placements are collapsed to the strongest label (`unique_by_cells`), per
//! protocol v1.7 §4.2.1.

use std::cell::RefCell;
use std::collections::HashMap;

use intetrigence_bot::{Movegen, unique_by_cells};
use intetrigence_core::{Action, ActivePiece, Board, PieceKind, Rotation, SpinType};

thread_local! {
    /// The referee is single-threaded; one reusable generator avoids the
    /// per-call buffer allocation.
    static MOVEGEN: RefCell<Movegen> = RefCell::new(Movegen::new());
}

/// One reachable, grounded placement, keyed by its cell set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    pub kind: PieceKind,
    pub rotation: Rotation,
    pub x: i32,
    pub y: i32,
    /// Absolute occupied cells, sorted lexicographically.
    pub cells: [(i32, i32); 4],
    /// Best spin achievable for this placement (max over paths).
    pub spin: SpinType,
    /// Pre-lock action path from the start state (no hard drop).
    pub path: Vec<Action>,
    /// True when the placement requires a hold first.
    pub hold: bool,
}

impl Placement {
    /// Lines this placement would clear on `board` (pre-lock).
    pub fn lines_after(&self, board: &Board) -> u8 {
        let mut copy = board.clone();
        copy.place(self.kind, self.rotation, self.x, self.y)
            .expect("a reachable placement is legal");
        copy.clear_lines().lines
    }
}

/// All reachable grounded placements for one piece, indexed by cell set.
#[derive(Clone, Debug)]
pub struct MoveSet {
    kind: PieceKind,
    hold: bool,
    placements: Vec<Placement>,
    by_cells: HashMap<[(i32, i32); 4], usize>,
}

impl MoveSet {
    pub fn kind(&self) -> PieceKind {
        self.kind
    }

    pub fn hold(&self) -> bool {
        self.hold
    }

    pub fn placements(&self) -> &[Placement] {
        &self.placements
    }

    pub fn len(&self) -> usize {
        self.placements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.placements.is_empty()
    }

    /// Looks a placement up by absolute cell set. This is the referee's legal
    /// move set (`docs/s2-referee-protocol.md` §4.3).
    pub fn get(&self, cells: &[(i32, i32); 4]) -> Option<&Placement> {
        self.by_cells
            .get(cells)
            .map(|&index| &self.placements[index])
    }
}

/// Canonical spawn state of `kind`.
pub fn spawn_state(kind: PieceKind) -> ActivePiece {
    intetrigence_bot::spawn_piece(kind)
}

/// Absolute cells of a core placement, sorted lexicographically.
pub fn core_cells(kind: PieceKind, rotation: Rotation, x: i32, y: i32) -> [(i32, i32); 4] {
    let mut cells = kind.cells(rotation).map(|(cx, cy)| (cx + x, cy + y));
    cells.sort_unstable();
    cells
}

fn convert(placement: &intetrigence_bot::Placement, hold: bool) -> Placement {
    let mut cells = placement.cells();
    cells.sort_unstable();
    Placement {
        kind: placement.piece.kind,
        rotation: placement.piece.rotation,
        x: placement.piece.x,
        y: placement.piece.y,
        cells,
        spin: placement.spin,
        path: placement.pre_lock_actions().to_vec(),
        hold,
    }
}

/// Every reachable grounded placement, **without** cell-set deduplication.
///
/// This is the raw `(rotation, x, y)` result. It is used by the cell-set
/// invariant tests and by the cross-check against the bot's movegen.
pub fn generate_all(board: &Board, start: ActivePiece, hold: bool) -> Vec<Placement> {
    MOVEGEN.with(|cell| {
        let mut movegen = cell.borrow_mut();
        movegen
            .generate_from(board, start)
            .iter()
            .map(|placement| convert(placement, hold))
            .collect()
    })
}

/// Enumerates every grounded placement reachable from `start`, one per cell
/// set (the strongest spin label wins).
pub fn generate(board: &Board, start: ActivePiece, hold: bool) -> MoveSet {
    let kind = start.kind;
    let unique = MOVEGEN.with(|cell| {
        let mut movegen = cell.borrow_mut();
        unique_by_cells(movegen.generate_from(board, start))
    });
    let mut placements = Vec::with_capacity(unique.len());
    let mut by_cells = HashMap::with_capacity(unique.len());
    for placement in &unique {
        let converted = convert(placement, hold);
        by_cells.insert(converted.cells, placements.len());
        placements.push(converted);
    }
    MoveSet {
        kind,
        hold,
        placements,
        by_cells,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_board() -> Board {
        Board::new()
    }

    #[test]
    fn empty_board_has_grounded_placements_for_every_piece() {
        for kind in intetrigence_core::ALL_PIECES {
            let moves = generate(&empty_board(), spawn_state(kind), false);
            assert!(!moves.is_empty(), "{kind:?} must have placements");
            for placement in moves.placements() {
                assert!(
                    !empty_board().can_place(
                        kind,
                        placement.rotation,
                        placement.x,
                        placement.y - 1
                    ),
                    "{kind:?} placement {:?} is not grounded",
                    placement.cells
                );
            }
        }
    }

    #[test]
    fn lookup_by_cells_roundtrips() {
        let moves = generate(&empty_board(), spawn_state(PieceKind::I), false);
        for placement in moves.placements() {
            let found = moves.get(&placement.cells).expect("cell key must resolve");
            assert_eq!(found.cells, placement.cells);
        }
    }

    #[test]
    fn blocked_spawn_has_no_placements() {
        let mut board = Board::new();
        for y in 20..=23 {
            board.set_row(y, Board::FULL_ROW);
        }
        for kind in intetrigence_core::ALL_PIECES {
            let moves = generate(&board, spawn_state(kind), false);
            assert!(moves.is_empty(), "{kind:?} must have no placement");
        }
    }
}
