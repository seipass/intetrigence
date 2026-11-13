//! Exact move generation over the TETR.IO S2 input space.
//!
//! The generator runs a breadth-first search over every input-reachable
//! `(rotation, x, y)` state of one piece and keeps the states where the piece
//! cannot fall any further. That is the exact set of placements a player can
//! lock, not a heuristic sample:
//!
//! - inputs: shift left/right, soft drop, CW/CCW/180 rotation with the SRS+
//!   kick tables (`docs/s2-spec.md` §3.1/§3.2), each rotation stopping at the
//!   first kick that fits;
//! - the search starts at the standard spawn cell (`docs/s2-spec.md` §2.2.1)
//!   and explores *airborne* states too, so rotations in mid-air and kick
//!   push-ups are covered;
//! - a state is a **placement** when `can_place(x, y - 1)` is false. Lock-out
//!   is disabled in S2, so placements entirely inside the hidden buffer are
//!   included.
//!
//! Each placement carries the input sequence that reaches it. The sequence
//! starts at spawn, ends with [`Action::HardDrop`], and drives
//! [`intetrigence_core::Game`] to lock the piece at exactly that placement
//! (the piece is already resting, so the drop distance is zero).
//!
//! # Spin flags
//!
//! Two placements can share `(rotation, x, y)` but differ in whether the last
//! input was a rotation, which is what decides a spin (`docs/s2-spec.md`
//! §4.1). The BFS therefore tracks the arrival kind as part of the state, and
//! a placement prefers the strongest arrival: a FIN/TST 5th kick over a plain
//! rotation over a shift/drop. `spin_entry` reports whether *some* path ends
//! with a rotation, and the recorded action sequence is that spin path when it
//! exists.

use intetrigence_core::{Action, ActivePiece, Board, PieceKind, Rotation, SpinType, kick};

use crate::spin;

/// How the piece arrived at a BFS state.
///
/// Only the last accepted input matters for spin detection, so it is part of
/// the search state rather than a property of the position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Arrival {
    /// The last accepted input was a shift or a soft drop.
    Moved = 0,
    /// The last accepted input was a rotation without the FIN/TST kick.
    Rotated = 1,
    /// The last accepted input was a rotation that used the FIN/TST 5th kick
    /// (`docs/s2-spec.md` §3.3).
    RotatedFinOrTst = 2,
}

const ARRIVALS: [Arrival; ARRIVAL_COUNT] =
    [Arrival::Moved, Arrival::Rotated, Arrival::RotatedFinOrTst];

/// One reachable input-space state, including airborne ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct State {
    pub rotation: Rotation,
    pub arrival: Arrival,
    pub x: i32,
    pub y: i32,
}

/// A lockable placement: a resting position plus the inputs that reach it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    /// The piece at the position it locks at.
    pub piece: ActivePiece,
    /// True when some path ends with a rotation as the last input before the
    /// lock, i.e. the placement is a spin entry (`docs/s2-spec.md` §4.1).
    pub spin_entry: bool,
    /// True when that rotation used the FIN/TST 5th kick (`docs/s2-spec.md`
    /// §3.3). Only meaningful together with `spin_entry`.
    pub fin_or_tst: bool,
    /// All-spin immobility of the placement (`docs/s2-spec.md` §4.2).
    pub immobile: bool,
    /// All-Mini+ classification of the placement (`docs/s2-spec.md` §4).
    pub spin: SpinType,
    /// Inputs that drive `Game` from spawn to this lock, ending in
    /// [`Action::HardDrop`].
    pub actions: Vec<Action>,
}

impl Placement {
    /// `(rotation, x, y)`, the identity of the placement.
    pub fn key(&self) -> (Rotation, i32, i32) {
        (self.piece.rotation, self.piece.x, self.piece.y)
    }

    /// Occupied cells in board coordinates.
    pub fn cells(&self) -> [(i32, i32); 4] {
        self.piece.cells()
    }

    /// The inputs before the final hard drop.
    ///
    /// This is the slice that leaves the piece resting in place; its last
    /// element is the rotation that makes the placement a spin entry when
    /// [`Placement::spin_entry`] is set.
    pub fn pre_lock_actions(&self) -> &[Action] {
        if let Some((last, rest)) = self.actions.split_last() {
            if *last == Action::HardDrop {
                return rest;
            }
        }
        &self.actions
    }
}

/// The standard spawn piece for `kind` (`docs/s2-spec.md` §2.2.1).
pub fn spawn_piece(kind: PieceKind) -> ActivePiece {
    let (x, y) = kind.spawn_position();
    ActivePiece {
        kind,
        rotation: Rotation::Zero,
        x,
        y,
    }
}

/// Every placement reachable from the standard spawn of `kind`.
pub fn generate(board: &Board, kind: PieceKind) -> Vec<Placement> {
    generate_from(board, spawn_piece(kind))
}

/// Every placement reachable from an explicit spawn state.
///
/// Returns an empty vector when the spawn itself is blocked (block-out). The
/// order is `(rotation, x, y)` ascending, which makes results easy to compare
/// between implementations.
pub fn generate_from(board: &Board, spawn: ActivePiece) -> Vec<Placement> {
    let Some(exploration) = explore(board, spawn) else {
        return Vec::new();
    };
    let kind = spawn.kind;
    let mut placements = Vec::with_capacity(exploration.resting.len());

    for &(rotation, x, y) in &exploration.resting {
        let reached = |arrival| {
            state_index(rotation, arrival, x, y)
                .filter(|&index| exploration.parent[index] != UNVISITED)
        };
        let moved = reached(Arrival::Moved);
        let rotated = reached(Arrival::Rotated);
        let fin_or_tst = reached(Arrival::RotatedFinOrTst);
        let Some(best) = fin_or_tst.or(rotated).or(moved) else {
            continue;
        };

        let spin_entry = fin_or_tst.is_some() || rotated.is_some();
        let mut actions = exploration.path_to(best);
        actions.push(Action::HardDrop);

        let piece = ActivePiece {
            kind,
            rotation,
            x,
            y,
        };
        placements.push(Placement {
            piece,
            spin_entry,
            fin_or_tst: fin_or_tst.is_some(),
            immobile: spin::is_immobile(board, piece),
            spin: spin::classify_spin(board, piece, spin_entry, fin_or_tst.is_some()),
            actions,
        });
    }

    placements
}

/// Every input-reachable state, including airborne ones, in BFS order.
///
/// This is the raw search result behind [`generate`]; it exists for tests and
/// diagnostics (a completeness check compares it against an independent
/// reference), not for the search hot path.
pub fn reachable_states(board: &Board, kind: PieceKind) -> Vec<State> {
    match explore(board, spawn_piece(kind)) {
        Some(exploration) => exploration
            .order
            .iter()
            .map(|&index| state_at(index as usize))
            .collect(),
        None => Vec::new(),
    }
}

// The state grid. A legal piece always has every cell inside the 10x40 board,
// so `x` stays within `-3..=9` and `y` within `-3..=39`. The bounds below add
// one cell of slack on each side and are the single place that decides what
// the BFS can represent.
const X_MIN: i32 = -4;
const X_MAX: i32 = 10;
const Y_MIN: i32 = -4;
const Y_MAX: i32 = 40;
const X_COUNT: usize = (X_MAX - X_MIN + 1) as usize;
const Y_COUNT: usize = (Y_MAX - Y_MIN + 1) as usize;
const ARRIVAL_COUNT: usize = 3;
const ROTATION_COUNT: usize = 4;
const POSITION_COUNT: usize = ROTATION_COUNT * Y_COUNT * X_COUNT;
const STATE_COUNT: usize = POSITION_COUNT * ARRIVAL_COUNT;
const UNVISITED: u16 = u16::MAX;

const MOVE_INPUTS: [(Action, i32, i32); 3] = [
    (Action::MoveLeft, -1, 0),
    (Action::MoveRight, 1, 0),
    (Action::SoftDrop, 0, -1),
];

#[inline]
fn state_index(rotation: Rotation, arrival: Arrival, x: i32, y: i32) -> Option<usize> {
    if !(X_MIN..=X_MAX).contains(&x) || !(Y_MIN..=Y_MAX).contains(&y) {
        return None;
    }
    let layer = rotation as usize * ARRIVAL_COUNT + arrival as usize;
    Some((layer * Y_COUNT + (y - Y_MIN) as usize) * X_COUNT + (x - X_MIN) as usize)
}

/// Index of `(rotation, x, y)` ignoring the arrival kind.
#[inline]
fn position_index(rotation: Rotation, x: i32, y: i32) -> usize {
    (rotation as usize * Y_COUNT + (y - Y_MIN) as usize) * X_COUNT + (x - X_MIN) as usize
}

#[inline]
fn state_at(index: usize) -> State {
    let xi = index % X_COUNT;
    let rest = index / X_COUNT;
    let yi = rest % Y_COUNT;
    let layer = rest / Y_COUNT;
    State {
        rotation: Rotation::ALL[layer / ARRIVAL_COUNT],
        arrival: ARRIVALS[layer % ARRIVAL_COUNT],
        x: X_MIN + xi as i32,
        y: Y_MIN + yi as i32,
    }
}

/// The BFS result: a parent pointer and the input that produced each state.
struct Exploration {
    root: usize,
    parent: Vec<u16>,
    input: Vec<u8>,
    order: Vec<u16>,
    /// Reachable resting positions, `(rotation, x, y)` ascending.
    resting: Vec<(Rotation, i32, i32)>,
}

impl Exploration {
    /// Inputs from spawn to `index`, in execution order.
    fn path_to(&self, mut index: usize) -> Vec<Action> {
        let mut actions = Vec::new();
        while index != self.root {
            actions.push(
                Action::from_u8(self.input[index]).expect("recorded inputs are always valid"),
            );
            index = self.parent[index] as usize;
        }
        actions.reverse();
        actions
    }
}

/// Collision test identical to [`Board::can_place`] but with the rotation's
/// cell table hoisted out of the loop. The BFS calls it thousands of times per
/// board, and `PieceKind::cells` re-derives the rotated cells on every call.
/// [`tests::hoisted_collision_check_matches_the_board`] pins the equivalence.
#[inline]
fn fits(board: &Board, cells: &[(i32, i32); 4], x: i32, y: i32) -> bool {
    for &(cx, cy) in cells {
        let (bx, by) = (x + cx, y + cy);
        if !(0..Board::WIDTH as i32).contains(&bx) || !(0..Board::HEIGHT as i32).contains(&by) {
            return false;
        }
        if board.row(by as usize) & (1u16 << bx as usize) != 0 {
            return false;
        }
    }
    true
}

fn explore(board: &Board, spawn: ActivePiece) -> Option<Exploration> {
    let kind = spawn.kind;
    let cells = [
        kind.cells(Rotation::Zero),
        kind.cells(Rotation::R),
        kind.cells(Rotation::Two),
        kind.cells(Rotation::L),
    ];
    let fits_at = |rotation: Rotation, x: i32, y: i32| fits(board, &cells[rotation as usize], x, y);
    if !fits_at(spawn.rotation, spawn.x, spawn.y) {
        return None;
    }
    let root = state_index(spawn.rotation, Arrival::Moved, spawn.x, spawn.y)?;

    let mut parent = vec![UNVISITED; STATE_COUNT];
    let mut input = vec![0u8; STATE_COUNT];
    let mut order = Vec::with_capacity(STATE_COUNT);
    // Successors do not depend on how a position was reached, so each
    // `(rotation, x, y)` is expanded once even when it is reachable with
    // several arrival kinds. Resting positions are recorded here rather than
    // re-scanned over the whole board afterwards.
    let mut expanded = vec![false; POSITION_COUNT];
    let mut resting = Vec::new();
    parent[root] = root as u16;
    order.push(root as u16);

    let mut head = 0;
    while head < order.len() {
        let from = order[head] as usize;
        head += 1;
        let State { rotation, x, y, .. } = state_at(from);
        let position = position_index(rotation, x, y);
        if expanded[position] {
            continue;
        }
        expanded[position] = true;
        if !fits_at(rotation, x, y - 1) {
            resting.push((rotation, x, y));
        }
        for (action, dx, dy) in MOVE_INPUTS {
            if fits_at(rotation, x + dx, y + dy) {
                push_state(
                    &mut parent,
                    &mut input,
                    &mut order,
                    from,
                    state_index(rotation, Arrival::Moved, x + dx, y + dy),
                    action,
                );
            }
        }

        for (action, to) in [
            (Action::RotateCw, rotation.cw()),
            (Action::RotateCcw, rotation.ccw()),
            (Action::Rotate180, rotation.flip()),
        ] {
            // SRS tries the kick candidates in table order and stops at the
            // first one that fits.
            for &(dx, dy) in kick::kicks(kind, rotation, to) {
                let (nx, ny) = (x + dx as i32, y + dy as i32);
                if fits_at(to, nx, ny) {
                    let arrival = if kick::is_fin_or_tst_kick(rotation, to, (dx, dy)) {
                        Arrival::RotatedFinOrTst
                    } else {
                        Arrival::Rotated
                    };
                    push_state(
                        &mut parent,
                        &mut input,
                        &mut order,
                        from,
                        state_index(to, arrival, nx, ny),
                        action,
                    );
                    break;
                }
            }
        }
    }

    resting.sort_unstable();
    Some(Exploration {
        root,
        parent,
        input,
        order,
        resting,
    })
}

fn push_state(
    parent: &mut [u16],
    input: &mut [u8],
    order: &mut Vec<u16>,
    from: usize,
    to: Option<usize>,
    action: Action,
) {
    let Some(to) = to else {
        return;
    };
    if parent[to] != UNVISITED {
        return;
    }
    parent[to] = from as u16;
    input[to] = action as u8;
    order.push(to as u16);
}

#[cfg(test)]
mod tests {
    use super::*;
    use intetrigence_core::{ALL_PIECES, Game};

    /// The hoisted collision test must be the same predicate as
    /// [`Board::can_place`] on every cell of every board shape used in tests.
    #[test]
    fn hoisted_collision_check_matches_the_board() {
        let mut boards = vec![Board::new()];
        let mut garbage = Board::new();
        garbage
            .insert_garbage(&[0, 4, 9, 2])
            .expect("four rows fit");
        boards.push(garbage);
        let mut high = Board::new();
        for y in 0..19 {
            let mut row = Board::FULL_ROW & !(1u16 << 9);
            if y % 3 == 0 {
                row &= !(1u16 << ((y * 7 + 3) % 9));
            }
            high.set_row(y, row);
        }
        boards.push(high);
        let mut cavity = Board::new();
        cavity.set_row(0, 0x3FC);
        cavity.set_row(1, 0x3FC);
        cavity.set_row(2, 0x007);
        boards.push(cavity);

        for board in &boards {
            for kind in ALL_PIECES {
                for rotation in Rotation::ALL {
                    let cells = kind.cells(rotation);
                    for x in X_MIN..=X_MAX {
                        for y in Y_MIN..=Y_MAX {
                            assert_eq!(
                                fits(board, &cells, x, y),
                                board.can_place(kind, rotation, x, y),
                                "{kind:?} {rotation:?} ({x}, {y})"
                            );
                        }
                    }
                }
            }
        }
    }

    /// Hand-derived resting-position counts on an empty board, one entry per
    /// rotation state. Horizontal pieces rest one row lower than vertical ones
    /// because their lowest occupied row is not the box's bottom row.
    const EMPTY_BOARD_COUNTS: [(PieceKind, [usize; 4]); 7] = [
        (PieceKind::I, [7, 10, 7, 10]),
        (PieceKind::O, [9, 9, 9, 9]),
        (PieceKind::T, [8, 9, 8, 9]),
        (PieceKind::S, [8, 9, 8, 9]),
        (PieceKind::Z, [8, 9, 8, 9]),
        (PieceKind::J, [8, 9, 8, 9]),
        (PieceKind::L, [8, 9, 8, 9]),
    ];

    #[test]
    fn empty_board_placement_counts_match_the_hand_derivation() {
        let board = Board::new();
        for (kind, expected) in EMPTY_BOARD_COUNTS {
            let placements = generate(&board, kind);
            let mut counts = [0usize; 4];
            for placement in &placements {
                counts[placement.piece.rotation as usize] += 1;
            }
            assert_eq!(counts, expected, "{kind:?}");
            assert_eq!(placements.len(), expected.iter().sum::<usize>(), "{kind:?}");
        }
    }

    #[test]
    fn placements_are_unique_and_resting() {
        let board = Board::new();
        for kind in ALL_PIECES {
            let placements = generate(&board, kind);
            for (index, placement) in placements.iter().enumerate() {
                assert!(
                    !board.can_place(
                        kind,
                        placement.piece.rotation,
                        placement.piece.x,
                        placement.piece.y - 1
                    ),
                    "{kind:?} {:?} is not resting",
                    placement.key()
                );
                for other in &placements[index + 1..] {
                    assert_ne!(placement.key(), other.key(), "{kind:?} duplicate");
                }
            }
        }
    }

    #[test]
    fn every_sequence_ends_with_a_hard_drop() {
        let board = Board::new();
        for kind in ALL_PIECES {
            for placement in generate(&board, kind) {
                assert_eq!(placement.actions.last(), Some(&Action::HardDrop));
                assert!(!placement.pre_lock_actions().is_empty());
                let last = *placement.pre_lock_actions().last().unwrap();
                let is_rotation = matches!(
                    last,
                    Action::RotateCw | Action::RotateCcw | Action::Rotate180
                );
                assert_eq!(
                    placement.spin_entry,
                    is_rotation,
                    "{kind:?} {:?}: spin_entry disagrees with the recorded sequence",
                    placement.key()
                );
            }
        }
    }

    #[test]
    fn blocked_spawn_yields_no_placements() {
        // Fill row 21 and 22 so the standard spawn collides.
        let mut board = Board::new();
        board.set_row(21, Board::FULL_ROW);
        board.set_row(22, Board::FULL_ROW);
        for kind in ALL_PIECES {
            assert!(generate(&board, kind).is_empty(), "{kind:?}");
            assert!(reachable_states(&board, kind).is_empty(), "{kind:?}");
        }
    }

    /// The TSD slot from `docs/s2-spec.md` §4.3. The T can only arrive by
    /// rotating from `R` at the same box position, so the placement must be a
    /// spin entry and classify as a full T-spin.
    fn tsd_board() -> Board {
        let mut board = Board::new();
        board.set_row(0, 0x1FF);
        board.set_row(1, 0x1EF);
        board.set_row(2, 0x1C7);
        board.set_row(3, 0x1CF);
        board
    }

    #[test]
    fn tsd_slot_is_reached_by_a_rotation() {
        let board = tsd_board();
        let placements = generate(&board, PieceKind::T);
        let tsd = placements
            .iter()
            .find(|placement| placement.key() == (Rotation::Two, 3, 1))
            .expect("the TSD slot must be reachable");
        assert!(tsd.spin_entry);
        assert_eq!(tsd.spin, SpinType::Full);
        assert!(matches!(
            tsd.pre_lock_actions().last(),
            Some(Action::RotateCw | Action::RotateCcw | Action::Rotate180)
        ));
    }

    #[test]
    fn sequences_lock_through_the_core_game() {
        let board = tsd_board();
        for kind in ALL_PIECES {
            for placement in generate(&board, kind) {
                let mut game =
                    Game::from_position(0, board.clone(), spawn_piece(kind)).expect("legal spawn");
                let (last, rest) = placement.actions.split_last().expect("non-empty");
                for action in rest {
                    game.apply(*action);
                }
                assert_eq!(game.active(), Some(placement.piece), "{kind:?}");
                let result = game.apply(*last);
                let drop = result.hard_drop.expect("hard drop is accepted");
                assert_eq!(drop.distance, 0, "{kind:?} {:?}", placement.key());
                assert_eq!(drop.lock.piece, placement.piece, "{kind:?}");
            }
        }
    }
}
