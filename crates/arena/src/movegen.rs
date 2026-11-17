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
use intetrigence_core::{Action, ActivePiece, Board, KickOrder, PieceKind, Rotation, SpinType};

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
    /// The I 90-degree test order used to build `path` (S2 spec U1).
    pub kick_order: KickOrder,
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
    /// Cell sets reachable only under the alternate I 90-degree kick order
    /// (S2 spec U1). The referee accepts them; the count is audited.
    alternate_only: std::collections::HashSet<[(i32, i32); 4]>,
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

    /// True when this placement is reachable only under the alternate I
    /// 90-degree kick order (S2 spec U1).
    pub fn is_alternate_only(&self, cells: &[(i32, i32); 4]) -> bool {
        self.alternate_only.contains(cells)
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
        kick_order: placement.kick_order,
        hold,
    }
}

/// Every reachable grounded placement, **without** cell-set deduplication.
///
/// This is the raw `(rotation, x, y)` result. It is used by the cell-set
/// invariant tests and by the cross-check against the bot's movegen.
pub fn generate_all(board: &Board, start: ActivePiece, hold: bool) -> Vec<Placement> {
    generate_all_with_180(board, start, hold, true)
}

/// Like [`generate_all`], but excludes 180-degree rotations when
/// `supports_180` is false (capability-aware forfeit classification).
pub fn generate_all_with_180(
    board: &Board,
    start: ActivePiece,
    hold: bool,
    supports_180: bool,
) -> Vec<Placement> {
    MOVEGEN.with(|cell| {
        let mut movegen = cell.borrow_mut();
        movegen
            .generate_from_with_180(board, start, supports_180)
            .iter()
            .map(|placement| convert(placement, hold))
            .collect()
    })
}

/// Enumerates every grounded placement reachable from `start`, one per cell
/// set (the strongest spin label wins).
pub fn generate(board: &Board, start: ActivePiece, hold: bool) -> MoveSet {
    generate_with_180(board, start, hold, true)
}

/// Like [`generate`], but excludes 180-degree rotations when `supports_180`
/// is false (capability-aware forfeit classification).
pub fn generate_with_180(
    board: &Board,
    start: ActivePiece,
    hold: bool,
    supports_180: bool,
) -> MoveSet {
    generate_with_spin_rule(board, start, hold, supports_180, true)
}

pub(crate) fn generate_with_spin_rule(
    board: &Board,
    start: ActivePiece,
    hold: bool,
    supports_180: bool,
    t_immobility_mini: bool,
) -> MoveSet {
    let kind = start.kind;
    // S2 spec U1: the I 90-degree offset set is fixed but the test order is
    // not. SRS stops at the first fitting offset, so the orders reach
    // different placements. The referee accepts the union; the bot keeps
    // SRS+ only.
    let (unique, alternate) = MOVEGEN.with(|cell| {
        let mut movegen = cell.borrow_mut();
        movegen.set_t_immobility_mini(t_immobility_mini);
        movegen.set_kick_order(KickOrder::SrsPlus);
        let primary = unique_by_cells(movegen.generate_from_with_180(board, start, supports_180));
        movegen.set_kick_order(KickOrder::Srs);
        let alternate = unique_by_cells(movegen.generate_from_with_180(board, start, supports_180));
        movegen.set_kick_order(KickOrder::SrsPlus);
        movegen.set_t_immobility_mini(true);
        (primary, alternate)
    });
    let mut placements = Vec::with_capacity(unique.len());
    let mut by_cells = HashMap::with_capacity(unique.len());
    for placement in &unique {
        let converted = convert(placement, hold);
        by_cells.insert(converted.cells, placements.len());
        placements.push(converted);
    }
    let mut alternate_only = std::collections::HashSet::new();
    for placement in &alternate {
        let converted = convert(placement, hold);
        if by_cells.contains_key(&converted.cells) {
            continue;
        }
        alternate_only.insert(converted.cells);
        by_cells.insert(converted.cells, placements.len());
        placements.push(converted);
    }
    MoveSet {
        kind,
        hold,
        placements,
        by_cells,
        alternate_only,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_board() -> Board {
        Board::new()
    }

    #[test]
    fn supports_180_false_drops_the_180_only_placement() {
        // The bot-dev fixture: 36 placements with 180, 35 without; the
        // difference is exactly one 180-only placement.
        let mut board = Board::new();
        board.set_row(0, 0b1101110111);
        board.set_row(1, 0b1111110011);
        board.set_row(2, 0b1111111001);
        let start = spawn_state(PieceKind::Z);
        // Compare the raw (rotation, x, y) lists; `generate` deduplicates by
        // cell set, which would hide the one-placement difference.
        let with = generate_all_with_180(&board, start, false, true);
        let without = generate_all_with_180(&board, start, false, false);
        assert_eq!(with.len(), 36);
        assert_eq!(without.len(), 35);
        let with_cells: std::collections::HashSet<_> =
            with.iter().map(|placement| placement.cells).collect();
        for placement in &without {
            assert!(
                with_cells.contains(&placement.cells),
                "the 180-less set must be a subset"
            );
        }
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
    fn i_90_kick_order_changes_reachability() {
        // Board from the S2 attax 10-game run, seed 9003, turn 268
        // (row 0 = bottom). Column 3 has a four-deep covered well: rows 0-3
        // are free, row 4 is blocked.
        //
        // TETR.IO S2 pins the I 90-degree offset *set* but not the test order
        // (docs/s2-spec.md U1). Our SRS+ `L -> 2` order is
        // [(0,0),(1,0),(-2,0),(1,2),(-2,-1)]; the attax engine uses the
        // standard SRS order [(0,0),(-2,0),(1,0),(-2,-1),(1,2)]. SRS stops at
        // the first fitting offset, so ours lands at (1, 2) and the attax
        // lands at (-2, -1) inside the cavity.
        //
        // The referee accepts the union of both orders (spec v1.3.24 §3.1.2) and
        // audits the count; the bot keeps SRS+ only.
        let mut board = Board::new();
        board.set_row(0, 0x3F7);
        board.set_row(1, 0x3F7);
        board.set_row(2, 0x3F7);
        board.set_row(3, 0x386);
        board.set_row(4, 0x10E);
        board.set_row(5, 0x008);
        let moves = generate(&board, spawn_state(PieceKind::I), true);
        let covered_well = [(3, 0), (3, 1), (3, 2), (3, 3)];
        assert!(
            moves.get(&covered_well).is_some(),
            "the referee must accept the alternate-order placement"
        );
        assert!(
            moves.is_alternate_only(&covered_well),
            "the placement is reachable only under the standard SRS order"
        );
        // The placement above the overhang is reachable in both orders.
        let above = [(3, 6), (3, 7), (3, 8), (3, 9)];
        assert!(moves.get(&above).is_some());
        assert!(!moves.is_alternate_only(&above));
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
