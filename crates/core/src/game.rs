//! Deterministic game state and basic operations.
//!
//! The engine models one player's board. It is deliberately free of wall-clock
//! time, lock delay and input timing: the referee/harness decides when to call
//! [`Game::lock`] or [`Game::hard_drop`]. This keeps `(seed, action sequence)`
//! fully reproducible.
//!
//! Attack scoring, surge, opener phase and the garbage queue are **not**
//! implemented yet; their interfaces live in `scoring`, `opener` and `garbage`
//! and are marked `TODO(s2-spec)`. Direct garbage-row insertion is implemented
//! here because it is a board operation, not a queue policy.

use crate::bag::{Bag, BagState};
use crate::board::{Board, BoardInvariantError, ClearInfo, GarbageError};
use crate::hold::Hold;
use crate::kick::{self, Kick};
use crate::piece::{PieceKind, RotateDirection, Rotation};

/// A piece at a concrete board position.
///
/// `x, y` is the bottom-left corner of the SRS bounding box. It may be negative
/// as long as every occupied cell is inside the board (for example, a flat
/// T-piece resting on the floor has `y = -1` because its 3x3 box has an empty
/// bottom row).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ActivePiece {
    pub kind: PieceKind,
    pub rotation: Rotation,
    pub x: i32,
    pub y: i32,
}

impl ActivePiece {
    /// Occupied cells in board coordinates.
    pub fn cells(self) -> [(i32, i32); 4] {
        let mut cells = self.kind.cells(self.rotation);
        for cell in &mut cells {
            cell.0 += self.x;
            cell.1 += self.y;
        }
        cells
    }
}

/// Per-game counters. Attack/surge counters are added once scoring is frozen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub pieces_placed: u32,
    pub lines_cleared: u32,
    pub garbage_lines_received: u32,
    pub garbage_lines_cleared: u32,
    pub holds: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LockResult {
    pub piece: ActivePiece,
    pub clear: ClearInfo,
    pub topped_out: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HardDropResult {
    /// Rows travelled before locking.
    pub distance: u32,
    pub lock: LockResult,
}

/// Result of a successful rotation, including the kick that was applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RotationResult {
    pub piece: ActivePiece,
    pub kick: Kick,
    /// Index into the kick table (0 means no kick offset was needed).
    pub kick_index: usize,
    /// True when the 5th-kick FIN/TST candidate was used
    /// (`docs/s2-spec.md` §3.3).
    pub fin_or_tst: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionResult {
    /// Whether the input changed the game state (or locked a piece).
    pub accepted: bool,
    /// Set only for hard drops.
    pub hard_drop: Option<HardDropResult>,
}

/// Replayable input. Discriminants are stable for replay encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Action {
    MoveLeft = 0,
    MoveRight = 1,
    SoftDrop = 2,
    RotateCw = 3,
    RotateCcw = 4,
    Rotate180 = 5,
    HardDrop = 6,
    Hold = 7,
}

impl Action {
    pub const ALL: [Action; 8] = [
        Action::MoveLeft,
        Action::MoveRight,
        Action::SoftDrop,
        Action::RotateCw,
        Action::RotateCcw,
        Action::Rotate180,
        Action::HardDrop,
        Action::Hold,
    ];

    pub const fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0 => Action::MoveLeft,
            1 => Action::MoveRight,
            2 => Action::SoftDrop,
            3 => Action::RotateCw,
            4 => Action::RotateCcw,
            5 => Action::Rotate180,
            6 => Action::HardDrop,
            7 => Action::Hold,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameStateError {
    Board(BoardInvariantError),
    FullRow(usize),
    ActivePieceCollides,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameInvariantError {
    Board(BoardInvariantError),
    FullRow(usize),
    ActivePieceCollides,
    /// `topped_out` and `active` disagree (one is set, the other is not).
    ActiveStateMismatch,
}

/// One player's deterministic game state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Game {
    board: Board,
    active: Option<ActivePiece>,
    hold: Hold,
    bag: Bag,
    stats: Stats,
    topped_out: bool,
    last_clear: Option<ClearInfo>,
}

impl Game {
    /// Starts a new game with `seed` and spawns the first bag piece.
    pub fn new(seed: u64) -> Self {
        let mut game = Self {
            board: Board::new(),
            active: None,
            hold: Hold::new(),
            bag: Bag::new(seed),
            stats: Stats::default(),
            topped_out: false,
            last_clear: None,
        };
        let first = game.bag.next_piece();
        let spawned = game.spawn(first, false);
        debug_assert!(spawned, "an empty board must accept the first spawn");
        game
    }

    /// Builds a game at an explicit position.
    ///
    /// This is a position-level helper for tests and replay bootstrapping. The
    /// bag starts fresh from `seed`, the hold is empty, and stats are zero; it
    /// does not reconstruct a mid-game bag/hold history.
    pub fn from_position(
        seed: u64,
        board: Board,
        active: ActivePiece,
    ) -> Result<Self, GameStateError> {
        board.check_invariants().map_err(GameStateError::Board)?;
        for y in 0..Board::HEIGHT {
            if board.row(y) == Board::FULL_ROW {
                return Err(GameStateError::FullRow(y));
            }
        }
        if !board.can_place(active.kind, active.rotation, active.x, active.y) {
            return Err(GameStateError::ActivePieceCollides);
        }
        Ok(Self {
            board,
            active: Some(active),
            hold: Hold::new(),
            bag: Bag::new(seed),
            stats: Stats::default(),
            topped_out: false,
            last_clear: None,
        })
    }

    /// Spawns `kind` at the standard position.
    ///
    /// When `clutch` is true (the previous lock cleared at least one line) and
    /// the standard position is blocked, the piece is pushed up one row at a
    /// time until a legal position is found (`docs/s2-spec.md` §2.3). If none
    /// exists, the game tops out.
    fn spawn(&mut self, kind: PieceKind, clutch: bool) -> bool {
        let (x, y) = kind.spawn_position();
        let mut piece = ActivePiece {
            kind,
            rotation: Rotation::Zero,
            x,
            y,
        };

        if self
            .board
            .can_place(piece.kind, piece.rotation, piece.x, piece.y)
        {
            self.active = Some(piece);
            return true;
        }

        if clutch {
            let mut y = piece.y + 1;
            while y < Board::HEIGHT as i32 {
                piece.y = y;
                if self
                    .board
                    .can_place(piece.kind, piece.rotation, piece.x, piece.y)
                {
                    self.active = Some(piece);
                    return true;
                }
                y += 1;
            }
        }

        self.active = None;
        self.topped_out = true;
        false
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn active(&self) -> Option<ActivePiece> {
        self.active
    }

    pub fn stats(&self) -> Stats {
        self.stats
    }

    pub fn last_clear(&self) -> Option<ClearInfo> {
        self.last_clear
    }

    pub fn is_over(&self) -> bool {
        self.topped_out || self.active.is_none()
    }

    pub fn held_piece(&self) -> Option<PieceKind> {
        self.hold.held()
    }

    pub fn can_hold(&self) -> bool {
        self.hold.can_hold()
    }

    /// Next `count` pieces without consuming them.
    pub fn preview(&mut self, count: usize) -> Vec<PieceKind> {
        self.bag.peek(count)
    }

    /// Read-only snapshot of the current 7-bag for the referee / TBP
    /// `start.randomizer`. The active piece is already drawn and excluded.
    pub fn bag_state(&self) -> BagState {
        self.bag.state()
    }

    /// Lowest legal `y` for the active piece (ghost position).
    pub fn ghost_y(&self) -> Option<i32> {
        let piece = self.active?;
        let mut y = piece.y;
        while self
            .board
            .can_place(piece.kind, piece.rotation, piece.x, y - 1)
        {
            y -= 1;
        }
        Some(y)
    }

    /// Moves the active piece horizontally. Returns whether it moved.
    pub fn try_move(&mut self, dx: i32) -> bool {
        let Some(mut piece) = self.active else {
            return false;
        };
        if dx == 0 {
            return false;
        }
        if self
            .board
            .can_place(piece.kind, piece.rotation, piece.x + dx, piece.y)
        {
            piece.x += dx;
            self.active = Some(piece);
            true
        } else {
            false
        }
    }

    pub fn move_left(&mut self) -> bool {
        self.try_move(-1)
    }

    pub fn move_right(&mut self) -> bool {
        self.try_move(1)
    }

    /// Moves the active piece down one row. Returns whether it moved.
    ///
    /// This does not lock the piece; the caller decides when to lock.
    pub fn soft_drop(&mut self) -> bool {
        let Some(mut piece) = self.active else {
            return false;
        };
        if self
            .board
            .can_place(piece.kind, piece.rotation, piece.x, piece.y - 1)
        {
            piece.y -= 1;
            self.active = Some(piece);
            true
        } else {
            false
        }
    }

    /// Rotates the active piece, applying SRS+ kicks in table order.
    ///
    /// Returns the applied piece and kick, or `None` when every candidate
    /// collides. [`Self::rotate`] is the boolean convenience wrapper.
    pub fn rotate_with_info(&mut self, direction: RotateDirection) -> Option<RotationResult> {
        let piece = self.active?;
        let to = piece.rotation.step(direction);
        for (kick_index, &(dx, dy)) in kick::kicks(piece.kind, piece.rotation, to)
            .iter()
            .enumerate()
        {
            let candidate = ActivePiece {
                kind: piece.kind,
                rotation: to,
                x: piece.x + dx as i32,
                y: piece.y + dy as i32,
            };
            if self
                .board
                .can_place(candidate.kind, candidate.rotation, candidate.x, candidate.y)
            {
                self.active = Some(candidate);
                return Some(RotationResult {
                    piece: candidate,
                    kick: (dx, dy),
                    kick_index,
                    fin_or_tst: kick::is_fin_or_tst_kick(piece.rotation, to, (dx, dy)),
                });
            }
        }
        None
    }

    /// Rotates the active piece. Returns whether the rotation succeeded.
    pub fn rotate(&mut self, direction: RotateDirection) -> bool {
        self.rotate_with_info(direction).is_some()
    }

    /// Drops the active piece to its lowest legal row and locks it.
    pub fn hard_drop(&mut self) -> Option<HardDropResult> {
        let mut piece = self.active?;
        let start_y = piece.y;
        while self
            .board
            .can_place(piece.kind, piece.rotation, piece.x, piece.y - 1)
        {
            piece.y -= 1;
        }
        self.active = Some(piece);
        let distance = (start_y - piece.y) as u32;
        let lock = self.lock()?;
        Some(HardDropResult { distance, lock })
    }

    /// Locks the active piece, clears lines, and spawns the next bag piece.
    ///
    /// S2 TETRA LEAGUE has lock-out disabled: locking entirely above the
    /// visible field does not end the game. A block-out (the next spawn has no
    /// legal position, including the Clutch push-up search after a clear) does.
    /// The returned result records the piece and clear even when the game ends.
    pub fn lock(&mut self) -> Option<LockResult> {
        let piece = self.active.take()?;
        self.board
            .place(piece.kind, piece.rotation, piece.x, piece.y)
            .expect("active piece must be legal");

        let clear = self.board.clear_lines();
        self.stats.pieces_placed += 1;
        self.stats.lines_cleared += clear.lines as u32;
        self.stats.garbage_lines_cleared += clear.garbage_lines as u32;
        self.last_clear = Some(clear);
        self.hold.reset_after_lock();

        let next = self.bag.next_piece();
        let spawned = self.spawn(next, clear.lines > 0);
        Some(LockResult {
            piece,
            clear,
            topped_out: !spawned,
        })
    }

    /// Swaps the active piece with the hold slot.
    ///
    /// Returns `false` when hold was already used for this piece, or when the
    /// game is over. A failed respawn (block-out) ends the game.
    pub fn hold(&mut self) -> bool {
        if !self.hold.can_hold() {
            return false;
        }
        let Some(piece) = self.active else {
            return false;
        };

        let swapped = self.hold.swap(piece.kind);
        self.hold.mark_used();
        self.stats.holds += 1;

        let kind = match swapped {
            Some(kind) => kind,
            None => self.bag.next_piece(),
        };
        self.spawn(kind, false)
    }

    /// Inserts garbage rows at the bottom and pushes the stack (and the active
    /// piece) up by the same amount.
    /// Rows pushed past row 39 are discarded (`docs/s2-spec.md` §11-U6). If the
    /// active piece is pushed out of the board, the game tops out and `Ok(())`
    /// is returned with `is_over() == true`.
    pub fn insert_garbage(&mut self, holes: &[u8]) -> Result<(), GarbageError> {
        let count = holes.len();
        self.board.insert_garbage(holes)?;
        if count == 0 {
            return Ok(());
        }

        self.stats.garbage_lines_received += count as u32;
        if let Some(mut piece) = self.active {
            piece.y += count as i32;
            if self
                .board
                .can_place(piece.kind, piece.rotation, piece.x, piece.y)
            {
                self.active = Some(piece);
            } else {
                self.active = None;
                self.topped_out = true;
            }
        }
        Ok(())
    }

    /// Applies one replayable input.
    pub fn apply(&mut self, action: Action) -> ActionResult {
        if self.topped_out {
            return ActionResult {
                accepted: false,
                hard_drop: None,
            };
        }

        match action {
            Action::MoveLeft => ActionResult {
                accepted: self.move_left(),
                hard_drop: None,
            },
            Action::MoveRight => ActionResult {
                accepted: self.move_right(),
                hard_drop: None,
            },
            Action::SoftDrop => ActionResult {
                accepted: self.soft_drop(),
                hard_drop: None,
            },
            Action::RotateCw => ActionResult {
                accepted: self.rotate(RotateDirection::Cw),
                hard_drop: None,
            },
            Action::RotateCcw => ActionResult {
                accepted: self.rotate(RotateDirection::Ccw),
                hard_drop: None,
            },
            Action::Rotate180 => ActionResult {
                accepted: self.rotate(RotateDirection::Half),
                hard_drop: None,
            },
            Action::HardDrop => match self.hard_drop() {
                Some(result) => ActionResult {
                    accepted: true,
                    hard_drop: Some(result),
                },
                None => ActionResult {
                    accepted: false,
                    hard_drop: None,
                },
            },
            Action::Hold => ActionResult {
                accepted: self.hold(),
                hard_drop: None,
            },
        }
    }

    /// Checks state invariants. Used by tests and debug assertions.
    pub fn check_invariants(&self) -> Result<(), GameInvariantError> {
        self.board
            .check_invariants()
            .map_err(GameInvariantError::Board)?;

        for y in 0..Board::HEIGHT {
            if self.board.row(y) == Board::FULL_ROW {
                return Err(GameInvariantError::FullRow(y));
            }
        }

        if self.topped_out == self.active.is_some() {
            return Err(GameInvariantError::ActiveStateMismatch);
        }

        if let Some(piece) = self.active {
            if !self
                .board
                .can_place(piece.kind, piece.rotation, piece.x, piece.y)
            {
                return Err(GameInvariantError::ActivePieceCollides);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_roundtrip() {
        for action in Action::ALL {
            assert_eq!(Action::from_u8(action as u8), Some(action));
        }
        assert_eq!(Action::from_u8(8), None);
    }

    #[test]
    fn active_piece_cells_are_translated() {
        let piece = ActivePiece {
            kind: PieceKind::T,
            rotation: Rotation::Zero,
            x: 3,
            y: 19,
        };
        assert_eq!(piece.cells(), [(4, 21), (3, 20), (4, 20), (5, 20)]);
    }

    #[test]
    fn new_game_has_active_piece_and_clean_board() {
        let game = Game::new(0);
        assert!(game.active().is_some());
        assert!(!game.is_over());
        assert!(game.board().is_empty());
        assert_eq!(game.check_invariants(), Ok(()));
    }

    #[test]
    fn rotate_with_info_reports_the_applied_kick() {
        let mut game = Game::from_position(
            0,
            Board::new(),
            ActivePiece {
                kind: PieceKind::J,
                rotation: Rotation::R,
                x: -1,
                y: 0,
            },
        )
        .unwrap();

        let result = game.rotate_with_info(RotateDirection::Cw).unwrap();
        assert_eq!(
            result.piece,
            ActivePiece {
                kind: PieceKind::J,
                rotation: Rotation::Two,
                x: 0,
                y: 0,
            }
        );
        assert_eq!(result.kick, (1, 0));
        assert_eq!(result.kick_index, 1);
        assert!(!result.fin_or_tst);
    }

    #[test]
    fn rotate_with_info_reports_a_late_fifth_kick() {
        // Flat I on the floor: the SRS+ 0->R table's 5th candidate (+1, +2)
        // is the only one that fits.
        let mut game = Game::from_position(
            0,
            Board::new(),
            ActivePiece {
                kind: PieceKind::I,
                rotation: Rotation::Zero,
                x: 3,
                y: -2,
            },
        )
        .unwrap();

        let result = game.rotate_with_info(RotateDirection::Cw).unwrap();
        assert_eq!(
            result.piece,
            ActivePiece {
                kind: PieceKind::I,
                rotation: Rotation::R,
                x: 4,
                y: 0,
            }
        );
        assert_eq!(result.kick, (1, 2));
        assert_eq!(result.kick_index, 4);
        assert!(!result.fin_or_tst);
    }
}
