//! Exact reachability movegen over the input space.
//!
//! The referee must verify that a bot's placement is reachable from the current
//! spawn with shift / soft drop / SRS+ rotation (including 180°) and that it is
//! grounded (`docs/s2-referee-protocol.md` §4.2). It also has to *apply* the
//! placement through `crates/core` without a "teleport" API, so the same BFS
//! returns a concrete action path that is replayed with [`Game::apply`].
//!
//! Spin classification is a function of the placement **and** the path: a spin
//! requires the last action to be a rotation (`docs/s2-spec.md` §4.1), and the
//! 5th-kick FIN/TST predicate depends on the kick used. The BFS therefore
//! records, for every reachable state, whether some path reaches it with a
//! rotation as the last action, and takes the maximum spin over those paths.
//! This is deterministic: live play and `verify-replay` compute the same value
//! from the same board.

use std::collections::{HashMap, VecDeque};

use intetrigence_core::{
    Action, ActivePiece, Board, PieceKind, RotateDirection, Rotation, SpinType, kick,
};

/// Action order used by the BFS. Fixed so paths are reproducible.
const ACTIONS: [Action; 6] = [
    Action::MoveLeft,
    Action::MoveRight,
    Action::SoftDrop,
    Action::RotateCw,
    Action::RotateCcw,
    Action::Rotate180,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct State {
    rotation: u8,
    x: i32,
    y: i32,
}

impl State {
    fn new(piece: ActivePiece) -> Self {
        Self {
            rotation: piece.rotation as u8,
            x: piece.x,
            y: piece.y,
        }
    }

    fn rotation(self) -> Rotation {
        Rotation::from_u8(self.rotation).expect("rotation is always 0..=3")
    }

    fn piece(self, kind: PieceKind) -> ActivePiece {
        ActivePiece {
            kind,
            rotation: self.rotation(),
            x: self.x,
            y: self.y,
        }
    }
}

/// One reachable, grounded placement.
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
    /// Action path from the start state that reproduces the placement.
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
    let (x, y) = kind.spawn_position();
    ActivePiece {
        kind,
        rotation: Rotation::Zero,
        x,
        y,
    }
}

/// Enumerates every grounded placement reachable from `start`.
pub fn generate(board: &Board, start: ActivePiece, hold: bool) -> MoveSet {
    let kind = start.kind;
    let mut placements: Vec<Placement> = Vec::new();
    let mut by_cells: HashMap<[(i32, i32); 4], usize> = HashMap::new();
    for placement in generate_all(board, start, hold) {
        match by_cells.get(&placement.cells) {
            Some(&index) => {
                // The same cells can be reached with a different last action
                // (I/O/S/Z are 180-degree symmetric). A non-T spin is
                // `spin_entry && immobile`, so keeping the first state could
                // drop a Mini that a rotation entry would earn. Keep the
                // strongest label; ties keep the first BFS-order placement.
                if placement.spin.index() > placements[index].spin.index() {
                    placements[index] = placement;
                }
            }
            None => {
                by_cells.insert(placement.cells, placements.len());
                placements.push(placement);
            }
        }
    }
    MoveSet {
        kind,
        hold,
        placements,
        by_cells,
    }
}

/// Every reachable grounded placement, **without** cell-set deduplication.
///
/// This is the raw BFS result: one entry per `(rotation, x, y)` state. It is
/// used by the cell-set invariant tests and by the post-integration cross-check
/// against `intetrigence_bot::Movegen` (which exposes `unique_by_cells`).
pub fn generate_all(board: &Board, start: ActivePiece, hold: bool) -> Vec<Placement> {
    let kind = start.kind;
    if !board.can_place(kind, start.rotation, start.x, start.y) {
        // A blocked spawn is a block-out (docs/s2-spec.md §2.3): no placement
        // exists, and a hypothetical hold must not be offered.
        return Vec::new();
    }
    let first = State::new(start);

    let mut parent: HashMap<State, (Option<State>, Action)> = HashMap::new();
    let mut order: Vec<State> = Vec::new();
    let mut queue: VecDeque<State> = VecDeque::new();
    parent.insert(first, (None, Action::SoftDrop));
    order.push(first);
    queue.push_back(first);

    while let Some(state) = queue.pop_front() {
        for action in ACTIONS {
            let Some((next, _fin)) = step(board, kind, state, action) else {
                continue;
            };
            if parent.contains_key(&next) {
                continue;
            }
            parent.insert(next, (Some(state), action));
            order.push(next);
            queue.push_back(next);
        }
    }

    // Precompute every rotation edge once: `next -> [(fin_or_tst, previous,
    // action)]`. The order follows the BFS order, so the first maximum wins
    // deterministically.
    let mut rotation_edges: HashMap<State, Vec<(bool, State, Action)>> = HashMap::new();
    for &previous in &order {
        for action in [Action::RotateCw, Action::RotateCcw, Action::Rotate180] {
            if let Some((next, fin)) = step(board, kind, previous, action) {
                rotation_edges.entry(next).or_default().push((
                    fin.unwrap_or(false),
                    previous,
                    action,
                ));
            }
        }
    }

    let mut placements = Vec::new();
    for &state in &order {
        if board.can_place(kind, state.rotation(), state.x, state.y - 1) {
            continue; // not grounded
        }
        let piece = state.piece(kind);
        let cells = sorted_cells(piece.cells());
        let (spin, path) = best_spin(board, kind, state, &rotation_edges, &parent);
        placements.push(Placement {
            kind,
            rotation: state.rotation(),
            x: state.x,
            y: state.y,
            cells,
            spin,
            path,
            hold,
        });
    }
    placements
}

fn sorted_cells(mut cells: [(i32, i32); 4]) -> [(i32, i32); 4] {
    cells.sort_unstable();
    cells
}

/// Absolute cells of a core placement, sorted lexicographically.
pub fn core_cells(kind: PieceKind, rotation: Rotation, x: i32, y: i32) -> [(i32, i32); 4] {
    sorted_cells(kind.cells(rotation).map(|(cx, cy)| (cx + x, cy + y)))
}

fn step(
    board: &Board,
    kind: PieceKind,
    state: State,
    action: Action,
) -> Option<(State, Option<bool>)> {
    let rotation = state.rotation();
    match action {
        Action::MoveLeft => board
            .can_place(kind, rotation, state.x - 1, state.y)
            .then_some((
                State {
                    x: state.x - 1,
                    ..state
                },
                None,
            )),
        Action::MoveRight => board
            .can_place(kind, rotation, state.x + 1, state.y)
            .then_some((
                State {
                    x: state.x + 1,
                    ..state
                },
                None,
            )),
        Action::SoftDrop => board
            .can_place(kind, rotation, state.x, state.y - 1)
            .then_some((
                State {
                    y: state.y - 1,
                    ..state
                },
                None,
            )),
        Action::RotateCw => rotate(board, kind, state, RotateDirection::Cw),
        Action::RotateCcw => rotate(board, kind, state, RotateDirection::Ccw),
        Action::Rotate180 => rotate(board, kind, state, RotateDirection::Half),
        Action::HardDrop | Action::Hold => None,
    }
}

fn rotate(
    board: &Board,
    kind: PieceKind,
    state: State,
    direction: RotateDirection,
) -> Option<(State, Option<bool>)> {
    let from = state.rotation();
    let to = from.step(direction);
    for &(dx, dy) in kick::kicks(kind, from, to) {
        let candidate = State {
            rotation: to as u8,
            x: state.x + dx as i32,
            y: state.y + dy as i32,
        };
        if board.can_place(kind, to, candidate.x, candidate.y) {
            let fin = kick::is_fin_or_tst_kick(from, to, (dx, dy));
            return Some((candidate, Some(fin)));
        }
    }
    None
}

/// Maximum spin over all paths into `state`, plus a path that achieves it.
fn best_spin(
    board: &Board,
    kind: PieceKind,
    state: State,
    rotation_edges: &HashMap<State, Vec<(bool, State, Action)>>,
    parent: &HashMap<State, (Option<State>, Action)>,
) -> (SpinType, Vec<Action>) {
    let mut best = (SpinType::None, path_to(parent, state));
    let rotation = state.rotation();
    let Some(edges) = rotation_edges.get(&state) else {
        return best;
    };
    for &(fin, previous, action) in edges {
        let spin = board.spin_type(kind, rotation, state.x, state.y, true, fin);
        if spin.index() > best.0.index() {
            let mut path = path_to(parent, previous);
            path.push(action);
            best = (spin, path);
        }
    }
    best
}

fn path_to(parent: &HashMap<State, (Option<State>, Action)>, mut state: State) -> Vec<Action> {
    let mut path = Vec::new();
    while let Some(&(previous, action)) = parent.get(&state) {
        let Some(previous) = previous else {
            break;
        };
        path.push(action);
        state = previous;
    }
    path.reverse();
    path
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
                // Grounded: one row lower must collide or leave the board.
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
                // Every path replays to the same placement.
                let mut piece = spawn_state(kind);
                for action in &placement.path {
                    piece = apply_action(&empty_board(), piece, *action);
                }
                assert_eq!(
                    piece,
                    ActivePiece {
                        kind,
                        rotation: placement.rotation,
                        x: placement.x,
                        y: placement.y
                    }
                );
            }
        }
    }

    fn apply_action(board: &Board, piece: ActivePiece, action: Action) -> ActivePiece {
        match action {
            Action::MoveLeft => ActivePiece {
                x: piece.x - 1,
                ..piece
            },
            Action::MoveRight => ActivePiece {
                x: piece.x + 1,
                ..piece
            },
            Action::SoftDrop => ActivePiece {
                y: piece.y - 1,
                ..piece
            },
            Action::RotateCw | Action::RotateCcw | Action::Rotate180 => {
                let direction = match action {
                    Action::RotateCw => RotateDirection::Cw,
                    Action::RotateCcw => RotateDirection::Ccw,
                    _ => RotateDirection::Half,
                };
                let (state, _) = rotate(board, piece.kind, State::new(piece), direction)
                    .expect("path rotation must be legal");
                state.piece(piece.kind)
            }
            Action::HardDrop | Action::Hold => unreachable!("not a movegen action"),
        }
    }

    #[test]
    fn wall_t_rotation_is_classified_as_a_spin() {
        // A single block at (1, 0) makes the left wall a 3-corner T-spin slot:
        // the T rotates from north into R with the (-1, 0) kick, and the last
        // action is a rotation, so the placement must be a mini T-spin
        // (docs/s2-spec.md §4.3).
        let mut board = Board::new();
        board.set_row(0, 1 << 1);
        let moves = generate(&board, spawn_state(PieceKind::T), false);
        let spins = moves
            .placements()
            .iter()
            .filter(|placement| placement.spin != SpinType::None)
            .collect::<Vec<_>>();
        assert!(
            !spins.is_empty(),
            "expected at least one spin placement, got {:?}",
            moves.placements()
        );
        assert!(
            spins
                .iter()
                .any(|placement| placement.spin == SpinType::Mini),
            "expected a mini T-spin"
        );
        for placement in &spins {
            assert!(
                placement.path.last().is_some_and(|action| matches!(
                    action,
                    Action::RotateCw | Action::RotateCcw | Action::Rotate180
                )),
                "a spin path must end with a rotation"
            );
        }
    }

    #[test]
    fn blocked_spawn_has_no_placements() {
        let mut board = Board::new();
        // Fill the whole spawn area (row 21) so every piece blocks out.
        for y in 20..=23 {
            board.set_row(y, Board::FULL_ROW);
        }
        for kind in intetrigence_core::ALL_PIECES {
            let moves = generate(&board, spawn_state(kind), false);
            assert!(moves.is_empty(), "{kind:?} must have no placement");
        }
    }

    #[test]
    fn lookup_by_cells_roundtrips() {
        // A cell set can be described by more than one rotation (I north/south,
        // O, S/Z); the move set keeps one canonical placement per cell set, so
        // only the cells are guaranteed to roundtrip.
        let moves = generate(&empty_board(), spawn_state(PieceKind::I), false);
        for placement in moves.placements() {
            let found = moves.get(&placement.cells).expect("cell key must resolve");
            assert_eq!(found.cells, placement.cells);
        }
    }
}
