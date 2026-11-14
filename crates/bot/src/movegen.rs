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
//!
//! # Scratch reuse
//!
//! [`Movegen`] owns every buffer the search needs, so a hot loop can generate
//! placements for thousands of boards without allocating. The free functions
//! [`generate`] and [`reachable_states`] are convenience wrappers that build a
//! one-shot `Movegen`.

use intetrigence_core::{Action, ActivePiece, Board, PieceKind, Rotation, SpinType, kick};

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

/// Capacity of [`ActionSeq`].
///
/// A shortest spawn-to-placement path on a 10x40 board is at most 24 soft
/// drops + 12 shifts + 3 rotations + the hard drop, so 64 leaves a wide
/// margin. [`tests::action_sequences_fit_the_inline_buffer`] pins it.
pub const MAX_ACTIONS: usize = 64;

/// A fixed-capacity input sequence, so [`Placement`] stays `Copy`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ActionSeq {
    len: u8,
    buf: [Action; MAX_ACTIONS],
}

impl ActionSeq {
    pub const fn new() -> Self {
        Self {
            len: 0,
            buf: [Action::MoveLeft; MAX_ACTIONS],
        }
    }

    pub fn as_slice(&self) -> &[Action] {
        &self.buf[..self.len as usize]
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn last(&self) -> Option<Action> {
        self.as_slice().last().copied()
    }

    fn push(&mut self, action: Action) {
        assert!(
            (self.len as usize) < MAX_ACTIONS,
            "action path exceeds the {MAX_ACTIONS}-input inline buffer"
        );
        self.buf[self.len as usize] = action;
        self.len += 1;
    }

    fn reverse(&mut self) {
        self.buf[..self.len as usize].reverse();
    }
}

impl Default for ActionSeq {
    fn default() -> Self {
        Self::new()
    }
}

/// A lockable placement: a resting position plus the inputs that reach it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    pub actions: ActionSeq,
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
        let actions = self.actions.as_slice();
        if let Some((last, rest)) = actions.split_last() {
            if *last == Action::HardDrop {
                return rest;
            }
        }
        actions
    }
}

/// Drops placements that occupy exactly the same cells as an earlier one.
///
/// The generator emits one placement per `(rotation, x, y)`, but the
/// 180-degree-symmetric pieces reach the same cell set in two rotations (O in
/// all four). On an empty board the counts are
///
/// | piece | placements | unique cells |
/// |---|---|---|
/// | I | 34 | 17 |
/// | O | 36 | 9 |
/// | S, Z | 34 | 17 |
/// | T, J, L | 34 | 34 |
///
/// A referee that identifies candidates by their cells — the protocol in
/// `docs/s2-referee-protocol.md` §4.1 does — needs this adapter so the same
/// placement is not offered twice.
///
/// The search does not need it: two placements with the same cells also have
/// the same transposition key and collapse during the merge.
pub fn unique_by_cells(placements: &[Placement]) -> Vec<Placement> {
    let mut unique: Vec<Placement> = Vec::with_capacity(placements.len());
    let mut seen: Vec<[(i32, i32); 4]> = Vec::with_capacity(placements.len());
    for placement in placements {
        let mut cells = placement.cells();
        cells.sort_unstable();
        match seen.iter().position(|existing| *existing == cells) {
            Some(index) => {
                // Cell-identical placements can differ in whether the last
                // input was a rotation (the horizontal I reaches the same
                // cells from rotations 0 and 2), and that decides the spin.
                // Keep the strongest label, not the first one.
                if spin_strength(placement) > spin_strength(&unique[index]) {
                    unique[index] = *placement;
                }
            }
            None => {
                seen.push(cells);
                unique.push(*placement);
            }
        }
    }
    unique
}

/// Ordering for [`unique_by_cells`]: a full spin beats a mini beats none, and
/// a spin entry beats a plain arrival at the same label.
fn spin_strength(placement: &Placement) -> (u8, bool, bool) {
    (
        placement.spin as u8,
        placement.spin_entry,
        placement.fin_or_tst,
    )
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
    Movegen::new().generate(board, kind).to_vec()
}

/// Every placement reachable from an explicit spawn state.
pub fn generate_from(board: &Board, spawn: ActivePiece) -> Vec<Placement> {
    Movegen::new().generate_from(board, spawn).to_vec()
}

/// [`generate`] for an engine that may not have a 180-degree rotation.
///
/// The referee needs this to answer "does this opponent have a legal move?"
/// for a bot whose movegen only knows shift, CW and CCW: a placement that
/// only a 180 can reach is not legal for that opponent, and counting it makes
/// the referee call a stuck bot "still alive" (the forfeit-other bug).
pub fn generate_with_180(board: &Board, kind: PieceKind, supports_180: bool) -> Vec<Placement> {
    Movegen::new()
        .generate_with_180(board, kind, supports_180)
        .to_vec()
}

/// Every input-reachable state, including airborne ones, in BFS order.
///
/// This is the raw search result behind [`generate`]; it exists for tests and
/// diagnostics (a completeness check compares it against an independent
/// reference), not for the search hot path.
pub fn reachable_states(board: &Board, kind: PieceKind) -> Vec<State> {
    let mut movegen = Movegen::new();
    if !movegen.explore(board, spawn_piece(kind)) {
        return Vec::new();
    }
    movegen
        .order
        .iter()
        .map(|&index| state_at(index as usize))
        .collect()
}

/// Reusable exact movegen.
///
/// All buffers are allocated once; [`Movegen::generate`] only clears and
/// refills them. The returned slice borrows the generator, so callers must
/// finish with one board's placements before asking for the next board's.
#[derive(Clone, Debug)]
pub struct Movegen {
    root: usize,
    parent: Vec<u16>,
    input: Vec<u8>,
    order: Vec<u16>,
    expanded: Vec<bool>,
    resting: Vec<(Rotation, i32, i32)>,
    placements: Vec<Placement>,
    /// Whether the modelled engine has a 180-degree rotation.
    supports_180: bool,
}

impl Default for Movegen {
    fn default() -> Self {
        Self::new()
    }
}

impl Movegen {
    pub fn new() -> Self {
        Self {
            root: 0,
            parent: vec![UNVISITED; STATE_COUNT],
            input: vec![0u8; STATE_COUNT],
            order: Vec::with_capacity(STATE_COUNT),
            expanded: vec![false; POSITION_COUNT],
            resting: Vec::with_capacity(64),
            placements: Vec::with_capacity(64),
            supports_180: true,
        }
    }

    /// Fills and returns the placements for `kind` from its standard spawn.
    pub fn generate(&mut self, board: &Board, kind: PieceKind) -> &[Placement] {
        self.generate_from(board, spawn_piece(kind))
    }

    /// Fills and returns the placements for an explicit spawn state.
    ///
    /// Returns an empty slice when the spawn itself is blocked (block-out).
    /// The order is `(rotation, x, y)` ascending, which makes results easy to
    /// compare between implementations.
    pub fn generate_from(&mut self, board: &Board, spawn: ActivePiece) -> &[Placement] {
        self.generate_from_with_180(board, spawn, true)
    }

    /// [`Movegen::generate`] with the 180-degree capability made explicit.
    pub fn generate_with_180(
        &mut self,
        board: &Board,
        kind: PieceKind,
        supports_180: bool,
    ) -> &[Placement] {
        self.generate_from_with_180(board, spawn_piece(kind), supports_180)
    }

    /// [`Movegen::generate_from`] with the 180-degree capability made explicit.
    ///
    /// With `supports_180 == false` the 180 transition is dropped from the
    /// BFS, so a placement that only a 180 can reach is not reported.
    pub fn generate_from_with_180(
        &mut self,
        board: &Board,
        spawn: ActivePiece,
        supports_180: bool,
    ) -> &[Placement] {
        self.supports_180 = supports_180;
        self.placements.clear();
        if !self.explore(board, spawn) {
            return &self.placements;
        }
        let kind = spawn.kind;

        for index in 0..self.resting.len() {
            let (rotation, x, y) = self.resting[index];
            let reached = |arrival| {
                state_index(rotation, arrival, x, y)
                    .filter(|&index| self.parent[index] != UNVISITED)
            };
            let moved = reached(Arrival::Moved);
            let rotated = reached(Arrival::Rotated);
            let fin_or_tst = reached(Arrival::RotatedFinOrTst);
            let Some(best) = fin_or_tst.or(rotated).or(moved) else {
                continue;
            };

            let spin_entry = fin_or_tst.is_some() || rotated.is_some();
            let actions = self.path_to(best);
            let piece = ActivePiece {
                kind,
                rotation,
                x,
                y,
            };
            self.placements.push(Placement {
                piece,
                spin_entry,
                fin_or_tst: fin_or_tst.is_some(),
                // The all-spin test and the All-Mini+ classification are the
                // core engine's (`docs/s2-spec.md` §4); the bot must not keep
                // a second copy of the corner table.
                immobile: board.is_immobile(kind, rotation, x, y),
                spin: board.spin_type(kind, rotation, x, y, spin_entry, fin_or_tst.is_some()),
                actions,
            });
        }

        &self.placements
    }

    /// Runs the BFS and records the reachable states and resting positions.
    fn explore(&mut self, board: &Board, spawn: ActivePiece) -> bool {
        let kind = spawn.kind;
        let cells = [
            kind.cells(Rotation::Zero),
            kind.cells(Rotation::R),
            kind.cells(Rotation::Two),
            kind.cells(Rotation::L),
        ];
        let fits_at =
            |rotation: Rotation, x: i32, y: i32| fits(board, &cells[rotation as usize], x, y);
        if !fits_at(spawn.rotation, spawn.x, spawn.y) {
            return false;
        }
        let Some(root) = state_index(spawn.rotation, Arrival::Moved, spawn.x, spawn.y) else {
            return false;
        };

        self.root = root;
        self.parent.fill(UNVISITED);
        self.order.clear();
        self.expanded.fill(false);
        self.resting.clear();
        self.parent[root] = root as u16;
        self.order.push(root as u16);

        // Successors do not depend on how a position was reached, so each
        // `(rotation, x, y)` is expanded once even when it is reachable with
        // several arrival kinds. Resting positions are recorded here rather
        // than re-scanned over the whole board afterwards.
        let mut head = 0;
        while head < self.order.len() {
            let from = self.order[head] as usize;
            head += 1;
            let State { rotation, x, y, .. } = state_at(from);
            let position = position_index(rotation, x, y);
            if self.expanded[position] {
                continue;
            }
            self.expanded[position] = true;
            if !fits_at(rotation, x, y - 1) {
                self.resting.push((rotation, x, y));
            }

            for (action, dx, dy) in MOVE_INPUTS {
                if fits_at(rotation, x + dx, y + dy) {
                    self.push_state(
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
                if action == Action::Rotate180 && !self.supports_180 {
                    continue;
                }
                // SRS tries the kick candidates in table order and stops at
                // the first one that fits.
                for &(dx, dy) in kick::kicks(kind, rotation, to) {
                    let (nx, ny) = (x + dx as i32, y + dy as i32);
                    if fits_at(to, nx, ny) {
                        let arrival = if kick::is_fin_or_tst_kick(rotation, to, (dx, dy)) {
                            Arrival::RotatedFinOrTst
                        } else {
                            Arrival::Rotated
                        };
                        self.push_state(from, state_index(to, arrival, nx, ny), action);
                        break;
                    }
                }
            }
        }

        self.resting.sort_unstable();
        true
    }

    fn push_state(&mut self, from: usize, to: Option<usize>, action: Action) {
        let Some(to) = to else {
            return;
        };
        if self.parent[to] != UNVISITED {
            return;
        }
        self.parent[to] = from as u16;
        self.input[to] = action as u8;
        self.order.push(to as u16);
    }

    /// Inputs from spawn to `index`, in execution order, plus the hard drop.
    fn path_to(&self, mut index: usize) -> ActionSeq {
        let mut actions = ActionSeq::new();
        while index != self.root {
            actions.push(
                Action::from_u8(self.input[index]).expect("recorded inputs are always valid"),
            );
            index = self.parent[index] as usize;
        }
        actions.reverse();
        actions.push(Action::HardDrop);
        actions
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
                assert_eq!(placement.actions.last(), Some(Action::HardDrop));
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

    /// The inline buffer is large enough for every path the BFS can produce on
    /// the test corpus, with room to spare.
    #[test]
    fn action_sequences_fit_the_inline_buffer() {
        let mut boards = vec![Board::new()];
        let mut high = Board::new();
        for y in 0..19 {
            high.set_row(y, Board::FULL_ROW & !(1u16 << 9));
        }
        boards.push(high);
        let mut longest = 0;
        for board in &boards {
            for kind in ALL_PIECES {
                for placement in generate(board, kind) {
                    longest = longest.max(placement.actions.len());
                }
            }
        }
        assert!(longest <= 32, "longest recorded path is {longest} inputs");
        assert!(longest < MAX_ACTIONS);
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
                let (last, rest) = placement
                    .actions
                    .as_slice()
                    .split_last()
                    .expect("non-empty");
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

    /// O reaches the same cells in all four rotations; the referee adapter
    /// must collapse them, and nothing else may change.
    #[test]
    fn unique_by_cells_collapses_only_cell_identical_placements() {
        let board = Board::new();
        for kind in ALL_PIECES {
            let placements = generate(&board, kind);
            let unique = unique_by_cells(&placements);
            assert!(unique.len() <= placements.len());
            let mut keys: Vec<_> = unique.iter().map(Placement::key).collect();
            keys.sort_unstable();
            keys.dedup();
            assert_eq!(keys.len(), unique.len(), "{kind:?}: duplicate placements");
            for placement in &unique {
                assert!(placements.iter().any(|other| other == placement));
            }
        }
        // The 180-degree-symmetric pieces collapse; T, J and L do not.
        for (kind, placements, unique) in [
            (PieceKind::I, 34, 17),
            (PieceKind::O, 36, 9),
            (PieceKind::S, 34, 17),
            (PieceKind::Z, 34, 17),
            (PieceKind::T, 34, 34),
            (PieceKind::J, 34, 34),
            (PieceKind::L, 34, 34),
        ] {
            let all = generate(&board, kind);
            assert_eq!(all.len(), placements, "{kind:?} placements");
            assert_eq!(unique_by_cells(&all).len(), unique, "{kind:?} unique");
        }
    }

    /// Cell-identical placements can differ in whether the last input was a
    /// rotation, so the adapter must keep the strongest label.
    #[test]
    fn unique_by_cells_keeps_the_strongest_spin_label() {
        let board = Board::new();
        for kind in ALL_PIECES {
            let placements = generate(&board, kind);
            for placement in unique_by_cells(&placements) {
                let mut cells = placement.cells();
                cells.sort_unstable();
                let best = placements
                    .iter()
                    .filter(|other| {
                        let mut other_cells = other.cells();
                        other_cells.sort_unstable();
                        other_cells == cells
                    })
                    .map(spin_strength)
                    .max()
                    .expect("the group is non-empty");
                assert_eq!(spin_strength(&placement), best, "{kind:?}");
            }
        }
        // The horizontal I is reachable as a rotation from state 2 and as a
        // plain drop from state 0; the kept placement must be the spin one.
        let kept = unique_by_cells(&generate(&board, PieceKind::I))
            .into_iter()
            .find(|placement| placement.key() == (Rotation::Two, 3, -1))
            .expect("the rotation-2 horizontal I is kept");
        assert!(kept.spin_entry);
    }

    /// Reusing one generator for many boards must give exactly the same
    /// placements as a fresh one.
    #[test]
    fn scratch_reuse_is_transparent() {
        let mut movegen = Movegen::new();
        let boards = [Board::new(), tsd_board()];
        for board in &boards {
            for kind in ALL_PIECES {
                let reused: Vec<Placement> = movegen.generate(board, kind).to_vec();
                let fresh = generate(board, kind);
                assert_eq!(reused, fresh, "{kind:?}");
            }
        }
    }
}

#[cfg(test)]
mod supports_180 {
    use super::*;

    /// A Z stack where exactly one of the 36 placements needs a 180.
    fn one_eighty_only_board() -> Board {
        let mut board = Board::new();
        board.set_row(0, 0b1101110111);
        board.set_row(1, 0b1111110011);
        board.set_row(2, 0b1111111001);
        board
    }

    fn keys(placements: &[Placement]) -> Vec<(Rotation, i32, i32)> {
        placements
            .iter()
            .map(|p| (p.piece.rotation, p.piece.x, p.piece.y))
            .collect()
    }

    #[test]
    fn dropping_the_180_removes_only_180_only_placements() {
        let board = one_eighty_only_board();
        let with = generate_with_180(&board, PieceKind::Z, true);
        let without = generate_with_180(&board, PieceKind::Z, false);
        assert_eq!(with.len(), 36);
        assert_eq!(without.len(), 35);
        // Every placement the 180-less engine can reach is still reachable.
        for key in keys(&without) {
            assert!(
                keys(&with).contains(&key),
                "the 180-less set must be a subset, {key:?} is missing"
            );
        }
        // And the default path still uses the 180.
        assert_eq!(generate(&board, PieceKind::Z).len(), 36);
    }

    #[test]
    fn an_open_board_is_unaffected() {
        let board = Board::new();
        for kind in [PieceKind::T, PieceKind::I, PieceKind::S, PieceKind::L] {
            let with = generate_with_180(&board, kind, true).len();
            let without = generate_with_180(&board, kind, false).len();
            assert_eq!(with, without, "{kind:?} on an open board");
        }
    }
}
