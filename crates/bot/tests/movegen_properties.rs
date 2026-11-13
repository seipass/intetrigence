//! Property tests for the exact move generator.
//!
//! Three independent oracles are used, so a bug has to survive all of them:
//!
//! 1. **Brute force** — enumerate every `(rotation, x, y)` whose cells are legal
//!    and which cannot fall one more row. No search at all. On boards without
//!    sealed cavities this is exactly the reachable set, so equality is the
//!    completeness statement.
//! 2. **Core `Game` reference** — a second BFS that drives
//!    [`intetrigence_core::Game`] with real [`Action`] inputs instead of
//!    re-implementing collision and kick logic. This checks the fast path
//!    against the engine's own rotation/kick code, including the arrival flags.
//! 3. **Replay** — every placement's recorded action sequence is executed on a
//!    fresh `Game`; the piece must end in exactly that position, every input
//!    must be accepted, and the hard drop must lock with distance zero.
//!
//! The spin classification is checked against a re-derivation that computes the
//! T's nub direction from its occupied cells (dot-product front-corner test)
//! instead of the hard-coded corner table.

use std::collections::BTreeSet;

use intetrigence_bot::{Placement, generate, reachable_states, spawn_piece};
use intetrigence_core::{
    ALL_PIECES, Action, ActivePiece, Board, Game, PieceKind, RotateDirection, Rotation, SpinType,
    SplitMix64,
};

type Key = (Rotation, i32, i32);
type StateKey = (Rotation, u8, i32, i32);

// ---------------------------------------------------------------------------
// Board fixtures
// ---------------------------------------------------------------------------

/// Empty board: the baseline for the hand-derived placement counts.
fn empty_board() -> Board {
    Board::new()
}

/// Garbage rows with staggered holes plus a short stack with a T slot and a
/// 1-wide well, i.e. the shapes a versus game actually produces.
fn garbage_board() -> Board {
    let mut board = Board::new();
    board.insert_garbage(&[0, 4, 9, 2]).expect("four rows fit");
    board.set_row(4, 0b11_1111_1110);
    board.set_row(5, 0b11_1111_0110);
    board.set_row(6, 0b11_1111_0110);
    board.set_row(7, 0b00_0001_0110);
    board
}

/// A 20-row stack: a 19-deep well at column 9, scattered single holes and a
/// small overhang on the top row. The spawn rows stay free.
fn high_board() -> Board {
    let mut board = Board::new();
    for y in 0..19 {
        let mut row = Board::FULL_ROW & !(1u16 << 9);
        if y % 3 == 0 {
            row &= !(1u16 << ((y * 7 + 3) % 9));
        }
        board.set_row(y, row);
    }
    board.set_row(19, 0b0000_0001_1100_0000);
    board
}

/// A T-spin double slot (`docs/s2-spec.md` §4.3): the T can only arrive by
/// rotating from `R` at the same box position.
fn tsd_board() -> Board {
    let mut board = Board::new();
    board.set_row(0, 0x1FF);
    board.set_row(1, 0x1EF);
    board.set_row(2, 0x1C7);
    board.set_row(3, 0x1CF);
    board
}

/// A 2x2 cavity sealed on all four sides: the O fits legally but nothing can
/// enter, so it must be legal in the brute-force set and absent from movegen.
fn sealed_cavity_board() -> Board {
    let mut board = Board::new();
    board.set_row(0, 0x3FC); // columns 2-9
    board.set_row(1, 0x3FC);
    board.set_row(2, 0x007); // ceiling over columns 0-1
    board
}

// ---------------------------------------------------------------------------
// Oracle 1: brute-force enumeration of legal resting positions
// ---------------------------------------------------------------------------

fn brute_force_resting(board: &Board, kind: PieceKind) -> BTreeSet<Key> {
    let mut out = BTreeSet::new();
    for rotation in Rotation::ALL {
        for x in -4..=10 {
            for y in -4..=40 {
                if board.can_place(kind, rotation, x, y)
                    && !board.can_place(kind, rotation, x, y - 1)
                {
                    out.insert((rotation, x, y));
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Oracle 2: BFS driven through the core engine
// ---------------------------------------------------------------------------

fn reference_states(board: &Board, kind: PieceKind) -> BTreeSet<StateKey> {
    let Ok(start) = Game::from_position(0, board.clone(), spawn_piece(kind)) else {
        return BTreeSet::new();
    };
    let piece = start.active().expect("a fresh game has a piece");
    let mut seen = BTreeSet::from([(piece.rotation, 0u8, piece.x, piece.y)]);
    let mut queue = vec![start];

    while let Some(game) = queue.pop() {
        for action in [Action::MoveLeft, Action::MoveRight, Action::SoftDrop] {
            let mut next = game.clone();
            if next.apply(action).accepted {
                let piece = next.active().expect("accepted input keeps the piece");
                if seen.insert((piece.rotation, 0, piece.x, piece.y)) {
                    queue.push(next);
                }
            }
        }
        for direction in [
            RotateDirection::Cw,
            RotateDirection::Ccw,
            RotateDirection::Half,
        ] {
            let mut next = game.clone();
            if let Some(result) = next.rotate_with_info(direction) {
                let arrival = if result.fin_or_tst { 2 } else { 1 };
                let piece = result.piece;
                if seen.insert((piece.rotation, arrival, piece.x, piece.y)) {
                    queue.push(next);
                }
            }
        }
    }
    seen
}

// ---------------------------------------------------------------------------
// Oracle 3 helpers: replay, independent spin derivation
// ---------------------------------------------------------------------------

/// Rows that become full when `piece` is placed on `board`.
fn expected_clears(board: &Board, piece: ActivePiece) -> u8 {
    let mut added = [0u16; Board::HEIGHT];
    for (x, y) in piece.cells() {
        added[y as usize] |= 1u16 << x as usize;
    }
    (0..Board::HEIGHT)
        .filter(|&y| board.row(y) | added[y] == Board::FULL_ROW)
        .count() as u8
}

fn assert_replays(board: &Board, kind: PieceKind, placement: &Placement) {
    let mut game =
        Game::from_position(0, board.clone(), spawn_piece(kind)).expect("the spawn must be legal");
    let (last, rest) = placement
        .actions
        .split_last()
        .expect("actions are non-empty");
    assert_eq!(*last, Action::HardDrop);
    for action in rest {
        assert!(
            game.apply(*action).accepted,
            "{kind:?} {:?}: recorded input {action:?} was rejected",
            placement.key()
        );
    }
    assert_eq!(
        game.active(),
        Some(placement.piece),
        "{kind:?} {:?}: replay ended elsewhere",
        placement.key()
    );
    let result = game.apply(*last);
    let drop = result.hard_drop.expect("the hard drop must be accepted");
    assert_eq!(drop.distance, 0, "{kind:?} {:?}", placement.key());
    assert_eq!(drop.lock.piece, placement.piece, "{kind:?}");
    assert_eq!(
        drop.lock.clear.lines,
        expected_clears(board, placement.piece),
        "{kind:?} {:?}: clear count",
        placement.key()
    );
}

/// Cell-level collision test that does not use `Board::can_place`.
fn cells_fit(board: &Board, cells: [(i32, i32); 4]) -> bool {
    cells.iter().all(|&(x, y)| {
        (0..Board::WIDTH as i32).contains(&x)
            && (0..Board::HEIGHT as i32).contains(&y)
            && !board.occupied_at(x as usize, y as usize)
    })
}

/// Independent immobility test: shift the occupied cells and check collision.
fn immobile_independent(board: &Board, piece: ActivePiece) -> bool {
    let cells = piece.cells();
    [(-1, 0), (1, 0), (0, 1), (0, -1)]
        .iter()
        .all(|&(dx, dy)| !cells_fit(board, cells.map(|(x, y)| (x + dx, y + dy))))
}

/// The T's nub direction, derived from its occupied cells: the nub is the cell
/// that is not part of the three-cell bar.
fn t_nub_direction(piece: ActivePiece) -> (i32, i32) {
    let cells = piece.cells();
    let bar = (0..4)
        .find(|&skip| {
            let rest: Vec<_> = cells
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != skip)
                .map(|(_, cell)| *cell)
                .collect();
            let same_row = rest.iter().all(|cell| cell.1 == rest[0].1);
            let same_column = rest.iter().all(|cell| cell.0 == rest[0].0);
            same_row || same_column
        })
        .expect("a T always has three collinear cells");
    let nub = cells[bar];
    // Relative to the rotation centre `(x + 1, y + 1)`.
    (nub.0 - piece.x - 1, nub.1 - piece.y - 1)
}

fn corner_filled_independent(board: &Board, x: i32, y: i32) -> bool {
    if x < 0 || x >= Board::WIDTH as i32 || y < 0 {
        return true;
    }
    if y >= Board::HEIGHT as i32 {
        return false;
    }
    board.occupied_at(x as usize, y as usize)
}

/// Independent All-Mini+ classification: the front corners are the box corners
/// on the nub side, selected by a dot product with the nub direction.
fn classify_independent(
    board: &Board,
    piece: ActivePiece,
    spin_entry: bool,
    fin_or_tst: bool,
) -> SpinType {
    if !spin_entry {
        return SpinType::None;
    }
    if piece.kind != PieceKind::T {
        return if immobile_independent(board, piece) {
            SpinType::Mini
        } else {
            SpinType::None
        };
    }

    let corners = [(0, 0), (2, 0), (2, 2), (0, 2)];
    let filled: Vec<bool> = corners
        .iter()
        .map(|&(dx, dy)| corner_filled_independent(board, piece.x + dx, piece.y + dy))
        .collect();
    if filled.iter().filter(|&&f| f).count() < 3 {
        return SpinType::None;
    }
    let (nx, ny) = t_nub_direction(piece);
    let front = corners
        .iter()
        .zip(&filled)
        .filter(|&(&(dx, dy), &filled)| {
            let (cx, cy) = (dx - 1, dy - 1);
            filled && cx * nx + cy * ny > 0
        })
        .count();
    if front == 2 || fin_or_tst {
        SpinType::Full
    } else {
        SpinType::Mini
    }
}

fn assert_spin_flags(board: &Board, kind: PieceKind, placement: &Placement) {
    let last = *placement
        .pre_lock_actions()
        .last()
        .expect("a placement has at least one input");
    let rotation_last = matches!(
        last,
        Action::RotateCw | Action::RotateCcw | Action::Rotate180
    );
    assert_eq!(
        placement.spin_entry,
        rotation_last,
        "{kind:?} {:?}: spin_entry disagrees with the recorded sequence",
        placement.key()
    );
    assert_eq!(
        placement.immobile,
        immobile_independent(board, placement.piece),
        "{kind:?} {:?}: immobile",
        placement.key()
    );
    assert_eq!(
        placement.spin,
        classify_independent(
            board,
            placement.piece,
            placement.spin_entry,
            placement.fin_or_tst
        ),
        "{kind:?} {:?}: spin classification",
        placement.key()
    );
}

// ---------------------------------------------------------------------------
// Representative boards: exact completeness (acceptance criterion)
// ---------------------------------------------------------------------------

#[test]
fn representative_boards_enumerate_every_legal_resting_position() {
    let boards = [
        ("empty", empty_board()),
        ("garbage", garbage_board()),
        ("high", high_board()),
        ("tsd", tsd_board()),
    ];
    for (name, board) in &boards {
        for kind in ALL_PIECES {
            let expected = brute_force_resting(board, kind);
            let placements = generate(board, kind);
            let actual: BTreeSet<Key> = placements.iter().map(Placement::key).collect();
            assert_eq!(
                actual, expected,
                "{name}/{kind:?}: movegen and brute force disagree"
            );
            assert_eq!(
                placements.len(),
                actual.len(),
                "{name}/{kind:?}: duplicates"
            );
            for placement in &placements {
                assert_replays(board, kind, placement);
                assert_spin_flags(board, kind, placement);
            }
        }
    }
}

#[test]
fn representative_boards_match_the_core_game_reference() {
    for board in [empty_board(), garbage_board(), high_board(), tsd_board()] {
        for kind in ALL_PIECES {
            let expected = reference_states(&board, kind);
            let actual: BTreeSet<StateKey> = reachable_states(&board, kind)
                .into_iter()
                .map(|state| (state.rotation, state.arrival as u8, state.x, state.y))
                .collect();
            assert_eq!(actual, expected, "{kind:?}: state space mismatch");
        }
    }
}

#[test]
fn spin_flags_agree_with_the_core_game_reference() {
    for board in [empty_board(), garbage_board(), high_board(), tsd_board()] {
        for kind in ALL_PIECES {
            let reference = reference_states(&board, kind);
            for placement in generate(&board, kind) {
                let (rotation, x, y) = placement.key();
                let rotation_arrival = reference.contains(&(rotation, 1, x, y))
                    || reference.contains(&(rotation, 2, x, y));
                assert_eq!(
                    placement.spin_entry,
                    rotation_arrival,
                    "{kind:?} {:?}: spin entry",
                    placement.key()
                );
                assert_eq!(
                    placement.fin_or_tst,
                    reference.contains(&(rotation, 2, x, y)),
                    "{kind:?} {:?}: FIN/TST",
                    placement.key()
                );
            }
        }
    }
}

#[test]
fn tsd_is_enumerated_as_a_full_spin() {
    let board = tsd_board();
    let placements = generate(&board, PieceKind::T);
    let tsd = placements
        .iter()
        .find(|placement| placement.key() == (Rotation::Two, 3, 1))
        .expect("the TSD slot must be reachable");
    assert!(tsd.spin_entry);
    assert_eq!(tsd.spin, SpinType::Full);
    assert!(
        placements
            .iter()
            .any(|placement| placement.spin == SpinType::Full)
    );
}

#[test]
fn sealed_cavity_is_legal_but_unreachable() {
    let board = sealed_cavity_board();
    let inside = (Rotation::Zero, 0, 0);
    assert!(board.can_place(PieceKind::O, Rotation::Zero, 0, 0));
    assert!(!board.can_place(PieceKind::O, Rotation::Zero, 0, -1));
    assert!(brute_force_resting(&board, PieceKind::O).contains(&inside));
    assert!(
        !generate(&board, PieceKind::O)
            .iter()
            .any(|placement| placement.key() == inside)
    );
    assert!(!reference_states(&board, PieceKind::O).contains(&(Rotation::Zero, 0, 0, 0)));
}

#[test]
fn a_full_stack_blocks_the_spawn() {
    let mut board = Board::new();
    for y in 0..=22 {
        board.set_row(y, Board::FULL_ROW & !(1u16 << 9));
    }
    // The spawn cells of every piece overlap the stack, so nothing is reachable.
    for kind in ALL_PIECES {
        assert!(generate(&board, kind).is_empty(), "{kind:?}");
    }
}

// ---------------------------------------------------------------------------
// Random boards
// ---------------------------------------------------------------------------

/// A board whose columns are filled from the floor up to a random height.
///
/// Normalising the minimum height to zero keeps every row from being full. On
/// such a board the free cells of each column are upward-closed, so a piece can
/// always descend vertically from the spawn to any legal resting position:
/// brute force and movegen must agree exactly.
fn skyline_board(rng: &mut SplitMix64) -> Board {
    let mut heights = [0usize; Board::WIDTH];
    for height in &mut heights {
        *height = rng.below(20) as usize;
    }
    let lowest = *heights.iter().min().expect("ten columns");
    let mut board = Board::new();
    for (x, &height) in heights.iter().enumerate() {
        for y in 0..(height - lowest) {
            board.set_row(y, board.row(y) | (1u16 << x));
        }
    }
    board
}

/// A messy board with holes, overhangs and cavities.
fn messy_board(rng: &mut SplitMix64) -> Board {
    let mut board = Board::new();
    let height = rng.below(20) as usize;
    for y in 0..height {
        let mut row = 0u16;
        for x in 0..Board::WIDTH {
            if rng.below(100) < 62 {
                row |= 1u16 << x;
            }
        }
        if row == Board::FULL_ROW {
            row &= !(1u16 << rng.below(Board::WIDTH as u64));
        }
        board.set_row(y, row);
    }
    board
}

const RANDOM_BOARDS: usize = 120;

#[test]
fn random_skyline_boards_enumerate_exactly() {
    let mut rng = SplitMix64::new(0x5EED_0001);
    for case in 0..RANDOM_BOARDS {
        let board = skyline_board(&mut rng);
        for kind in ALL_PIECES {
            let expected = brute_force_resting(&board, kind);
            let placements = generate(&board, kind);
            let actual: BTreeSet<Key> = placements.iter().map(Placement::key).collect();
            assert_eq!(actual, expected, "case {case}/{kind:?}");
            for placement in &placements {
                assert_replays(&board, kind, placement);
                assert_spin_flags(&board, kind, placement);
            }
        }
    }
}

#[test]
fn random_messy_boards_are_sound_and_complete() {
    let mut rng = SplitMix64::new(0x5EED_0002);
    for case in 0..RANDOM_BOARDS {
        let board = messy_board(&mut rng);
        for kind in ALL_PIECES {
            let legal = brute_force_resting(&board, kind);
            let placements = generate(&board, kind);
            let actual: BTreeSet<Key> = placements.iter().map(Placement::key).collect();

            // Soundness: never invent a placement that is not legal and resting.
            assert!(
                actual.is_subset(&legal),
                "case {case}/{kind:?}: illegal placement"
            );

            // Completeness: the whole reachable state space, airborne states
            // included, matches the core engine's own BFS.
            let reference = reference_states(&board, kind);
            let reachable: BTreeSet<StateKey> = reachable_states(&board, kind)
                .into_iter()
                .map(|state| (state.rotation, state.arrival as u8, state.x, state.y))
                .collect();
            assert_eq!(reachable, reference, "case {case}/{kind:?}: state space");

            // The placements are exactly the reachable resting states.
            let expected: BTreeSet<Key> = reference
                .iter()
                .filter(|&&(rotation, _, x, y)| !board.can_place(kind, rotation, x, y - 1))
                .map(|&(rotation, _, x, y)| (rotation, x, y))
                .collect();
            assert_eq!(actual, expected, "case {case}/{kind:?}: placements");

            for placement in &placements {
                assert_replays(&board, kind, placement);
                assert_spin_flags(&board, kind, placement);
            }
        }
    }
}
