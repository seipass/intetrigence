//! Round engine: per-player state, placement application, garbage flow.
//!
//! The engine owns everything the referee must track (`docs/s2-spec.md` §9) and
//! is the single code path used by live play and `verify-replay`. It never
//! reads the clock: `frame` is passed in by the caller, and the only time-like
//! input is `frame = turn * 12` (`docs/s2-spec.md` §11-U9).

use intetrigence_core::{
    ActivePiece, Board, ClearInput, Game, GarbageMultiplier, GarbageQueue, OpenerPhase, PieceKind,
    Rotation, S2Scoring, SpinType, compute_attack, derive_seeds,
};

use crate::movegen;

/// Board hash recorded per turn (`docs/s2-referee-protocol.md` §8.2).
///
/// FNV-1a over the 40 occupancy rows and the 40 garbage rows, so a hole-column
/// difference is visible even when the occupancy mask is identical.
pub fn board_hash(board: &Board) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for y in 0..Board::HEIGHT {
        hash ^= board.row(y) as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        hash ^= board.garbage_row(y) as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// One player's complete referee-side state.
#[derive(Clone, Debug)]
pub struct Player {
    pub slot: usize,
    pub game: Game,
    pub scoring: S2Scoring,
    pub opener: OpenerPhase,
    pub garbage: GarbageQueue,
    /// Number of attack events emitted (surge chunks count separately).
    pub attack_events: u32,
    pub attack_total: u32,
    pub max_combo: u32,
    pub max_b2b: u32,
    pub surge_released: u32,
}

impl Player {
    /// `piece_seed` is shared by both players; `garbage_seed` is per slot
    /// (`docs/s2-spec.md` §11-U7).
    pub fn new(slot: usize, piece_seed: u64, garbage_seed: u64) -> Self {
        Self {
            slot,
            game: Game::new(piece_seed),
            scoring: S2Scoring::new(),
            opener: OpenerPhase::new(),
            garbage: GarbageQueue::new(garbage_seed),
            attack_events: 0,
            attack_total: 0,
            max_combo: 0,
            max_b2b: 0,
            surge_released: 0,
        }
    }

    pub fn sent_total(&self) -> u32 {
        self.opener.sent_total
    }

    pub fn received_total(&self) -> u32 {
        self.game.stats().garbage_lines_received
    }

    pub fn topped_out(&self) -> bool {
        self.game.is_over()
    }
}

/// Read-only state handed to a bot for one placement.
#[derive(Clone, Debug)]
pub struct View {
    pub turn: u32,
    pub frame: u64,
    pub slot: usize,
    /// 1 or 2 (`docs/s2-referee-protocol.md` §1.2).
    pub seat: u8,
    pub board: Board,
    pub active: ActivePiece,
    pub hold: Option<PieceKind>,
    pub can_hold: bool,
    /// Five preview pieces (`docs/s2-spec.md` §2.6).
    pub preview: Vec<PieceKind>,
    /// Pieces left in the current 7-bag (TBP `start.randomizer`).
    pub bag_state: Vec<PieceKind>,
    /// Combo before this placement (0 = no chain).
    pub combo: u32,
    /// Consecutive difficult clears `N` before this placement.
    pub b2b_streak: u32,
    /// Rows in this player's own garbage queue (arrived or not).
    pub pending_garbage: u32,
    pub pieces_placed: u32,
    pub sent_total: u32,
    pub received_total: u32,
    /// Frame-derived garbage multiplier (`docs/s2-spec.md` §7.8).
    pub multiplier: f64,
}

impl View {
    pub fn active_kind(&self) -> PieceKind {
        self.active.kind
    }

    /// TBP `combo` (0 = no chain) equals the core combo value.
    pub fn tbp_combo(&self) -> u32 {
        self.combo
    }

    /// TBP `back_to_back`: the previous clear was difficult.
    pub fn tbp_b2b(&self) -> bool {
        self.b2b_streak >= 1
    }

    /// The piece a hold would bring in, if hold is available.
    pub fn hold_piece(&self) -> Option<PieceKind> {
        if !self.can_hold {
            return None;
        }
        match self.hold {
            Some(kind) => Some(kind),
            None => self.preview.first().copied(),
        }
    }
}

/// A placement the referee is asked to apply, identified by its cell set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacementTarget {
    pub piece: PieceKind,
    pub cells: [(i32, i32); 4],
}

impl PlacementTarget {
    pub fn new(piece: PieceKind, cells: [(i32, i32); 4]) -> Self {
        let mut cells = cells;
        cells.sort_unstable();
        Self { piece, cells }
    }

    pub fn from_core(piece: PieceKind, rotation: Rotation, x: i32, y: i32) -> Self {
        Self::new(piece, crate::movegen::core_cells(piece, rotation, x, y))
    }
}

/// Outcome of one resolved placement.
#[derive(Clone, Debug)]
pub struct AppliedPlacement {
    pub slot: usize,
    pub piece: PieceKind,
    pub rotation: Rotation,
    pub x: i32,
    pub y: i32,
    pub hold: bool,
    pub spin: SpinType,
    pub lines: u8,
    pub garbage_cleared: u32,
    pub perfect_clear: bool,
    pub events: Vec<u32>,
    /// Rows that actually went to the opponent (cancelled rows excluded).
    pub sent: u32,
    pub cancelled: u32,
    pub attack_total: u32,
    /// Attack the bot would have produced with its claimed spin label.
    pub declared_attack: Option<u32>,
    pub combo_before: u32,
    pub b2b_before: u32,
    pub multiplier: f64,
    /// `(amount, arrival_frame)` packets sent to the opponent, in order.
    pub packets: Vec<(u32, u64)>,
    /// Garbage rows inserted into this player's board this placement.
    pub inserted: Vec<u8>,
    pub combo_after: u32,
    pub b2b_after: u32,
    pub topped_out: bool,
    pub board_hash: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefereeError {
    NoActivePiece,
    HoldUnavailable,
    HoldMismatch {
        expected: Option<PieceKind>,
        got: PieceKind,
    },
    HoldFailed,
    PieceMismatch {
        expected: PieceKind,
        got: PieceKind,
    },
    IllegalPlacement {
        piece: PieceKind,
        cells: [(i32, i32); 4],
    },
    PathRejected {
        action: intetrigence_core::Action,
    },
    PathMismatch {
        expected: ActivePiece,
        got: Option<ActivePiece>,
    },
    LockFailed,
    Garbage(intetrigence_core::GarbageError),
}

impl std::fmt::Display for RefereeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefereeError::NoActivePiece => write!(f, "player has no active piece"),
            RefereeError::HoldUnavailable => write!(f, "hold is not available this piece"),
            RefereeError::HoldMismatch { expected, got } => {
                write!(f, "hold mismatch: expected {expected:?}, got {got:?}")
            }
            RefereeError::HoldFailed => write!(f, "hold failed (block-out)"),
            RefereeError::PieceMismatch { expected, got } => {
                write!(f, "piece mismatch: expected {expected:?}, got {got:?}")
            }
            RefereeError::IllegalPlacement { piece, cells } => {
                write!(f, "unreachable or ungrounded {piece:?} placement {cells:?}")
            }
            RefereeError::PathRejected { action } => {
                write!(f, "replayed path action {action:?} was rejected")
            }
            RefereeError::PathMismatch { expected, got } => {
                write!(f, "path ended at {got:?}, expected {expected:?}")
            }
            RefereeError::LockFailed => write!(f, "lock failed (no active piece)"),
            RefereeError::Garbage(error) => write!(f, "garbage insertion failed: {error:?}"),
        }
    }
}

impl std::error::Error for RefereeError {}

/// Sheet: which slot sits in seat 1 (`docs/s2-referee-protocol.md` §1.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sheet {
    A,
    B,
}

impl Sheet {
    pub const ALL: [Sheet; 2] = [Sheet::A, Sheet::B];

    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_uppercase().as_str() {
            "A" => Some(Sheet::A),
            "B" => Some(Sheet::B),
            _ => None,
        }
    }

    /// Seat order as slot indices: seat 1 first.
    pub fn order(self) -> [usize; 2] {
        match self {
            // A: seat 1 = intetrigence (slot 0), seat 2 = attax (slot 1).
            Sheet::A => [0, 1],
            Sheet::B => [1, 0],
        }
    }

    pub fn seat_of(self, slot: usize) -> u8 {
        if self.order()[0] == slot { 1 } else { 2 }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Sheet::A => "A",
            Sheet::B => "B",
        }
    }
}

/// A candidate adopted by the referee.
#[derive(Clone, Copy, Debug)]
pub struct Adopted {
    pub candidate_index: usize,
    pub target: PlacementTarget,
    pub claimed_spin: Option<SpinType>,
    pub claimed_attack: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdoptError {
    NoActivePiece,
    /// The bot returned no reachable candidate, but the referee's movegen has
    /// at least one legal placement: the bot's movegen is broken (forfeit).
    NoLegalCandidate {
        candidates: usize,
    },
    /// The referee's movegen has no legal placement at all: the board is stuck
    /// (top-out). This is defensive; a live active piece is itself a reachable
    /// grounded state, so it should not fire in practice.
    NoMoves,
}

impl std::fmt::Display for AdoptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdoptError::NoActivePiece => write!(f, "player has no active piece"),
            AdoptError::NoLegalCandidate { candidates } => {
                write!(f, "none of the {candidates} candidates is reachable")
            }
            AdoptError::NoMoves => write!(f, "the referee has no legal placement"),
        }
    }
}

impl std::error::Error for AdoptError {}

/// Match profile (protocol v1.11 §1.3).
///
/// `Official` is the empty start board and the only profile usable for final
/// claims. `DefenseStress` starts both players with deterministic garbage rows;
/// `GarbagePressure` injects garbage packets every `interval` turns. Both are
/// diagnostic only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Official,
    DefenseStress {
        rows: u8,
    },
    GarbagePressure {
        interval: u32,
        rows: u32,
    },
    /// Start-board stress plus a delayed spike schedule (diagnostic).
    Composite {
        start_rows: u8,
        interval: u32,
        rows: u32,
        from_turn: u32,
    },
}

impl Profile {
    pub const fn official() -> Self {
        Profile::Official
    }

    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let mut parts = text.split(':');
        match parts.next()? {
            "official" => Some(Profile::Official),
            "defense-stress" => {
                let rows = match parts.next() {
                    Some(value) => value.parse().ok()?,
                    None => 12,
                };
                Some(Profile::DefenseStress { rows })
            }
            "garbage-pressure" => {
                let interval = parts.next()?.parse().ok()?;
                let rows = parts.next()?.parse().ok()?;
                Some(Profile::GarbagePressure { interval, rows })
            }
            "composite" => {
                let start_rows = match parts.next() {
                    Some(value) => value.parse().ok()?,
                    None => 8,
                };
                let interval = match parts.next() {
                    Some(value) => value.parse().ok()?,
                    None => 20,
                };
                let rows = match parts.next() {
                    Some(value) => value.parse().ok()?,
                    None => 8,
                };
                let from_turn = match parts.next() {
                    Some(value) => value.parse().ok()?,
                    None => 200,
                };
                Some(Profile::Composite {
                    start_rows,
                    interval,
                    rows,
                    from_turn,
                })
            }
            _ => None,
        }
    }

    pub fn as_str(self) -> String {
        match self {
            Profile::Official => "official".into(),
            Profile::DefenseStress { rows } => format!("defense-stress:{rows}"),
            Profile::GarbagePressure { interval, rows } => {
                format!("garbage-pressure:{interval}:{rows}")
            }
            Profile::Composite {
                start_rows,
                interval,
                rows,
                from_turn,
            } => format!("composite:{start_rows}:{interval}:{rows}:{from_turn}"),
        }
    }

    /// Deterministic start garbage, in insertion order (`holes[0]` ends up on
    /// top). Only `DefenseStress` has any.
    pub fn start_garbage(self, seed: u64) -> Vec<u8> {
        match self {
            Profile::DefenseStress { rows }
            | Profile::Composite {
                start_rows: rows, ..
            } => {
                let mut rng = intetrigence_core::SplitMix64::new(seed ^ 0x5EED_5EED_5EED_5EED);
                (0..rows).map(|_| rng.below(10) as u8).collect()
            }
            Profile::Official | Profile::GarbagePressure { .. } => Vec::new(),
        }
    }

    /// Rows to inject into both players at the start of `turn`, if any.
    pub fn injection_amount(self, turn: u32) -> Option<u32> {
        match self {
            Profile::GarbagePressure { interval, rows }
                if interval > 0 && turn > 0 && turn % interval == 0 =>
            {
                Some(rows)
            }
            Profile::Composite {
                interval,
                rows,
                from_turn,
                ..
            } if interval > 0 && turn >= from_turn && (turn - from_turn) % interval == 0 => {
                Some(rows)
            }
            _ => None,
        }
    }
}

/// The referee's round state for both players.
#[derive(Clone, Debug)]
pub struct Engine {
    pub players: [Player; 2],
    pub seed: u64,
    pub sheet: Sheet,
    pub order: [usize; 2],
    pub turn: u32,
    pub frame: u64,
    pub frames_per_turn: u64,
    pub multiplier: GarbageMultiplier,
}

impl Engine {
    pub fn new(seed: u64, sheet: Sheet, frames_per_turn: u64) -> Self {
        Self::with_start_garbage(seed, sheet, frames_per_turn, &[])
    }

    /// Builds an engine for `profile`, applying its deterministic start garbage.
    ///
    /// Periodic injections are not applied here: the caller (or replay
    /// verifier) applies them at the recorded turns.
    pub fn with_profile(seed: u64, sheet: Sheet, frames_per_turn: u64, profile: Profile) -> Self {
        let start_garbage = profile.start_garbage(seed);
        Self::with_start_garbage(seed, sheet, frames_per_turn, &start_garbage)
    }

    /// Builds an engine and inserts `start_garbage` into both players' boards.
    ///
    /// The rows are ordinary garbage rows (one hole each), so the active piece
    /// is pushed up and `received_total` includes them. Both players get the
    /// same rows, so the turn-cap tiebreak stays symmetric.
    pub fn with_start_garbage(
        seed: u64,
        sheet: Sheet,
        frames_per_turn: u64,
        start_garbage: &[u8],
    ) -> Self {
        let derivation = derive_seeds(seed);
        let mut players = [
            Player::new(0, derivation.piece_seed, derivation.garbage_seeds[0]),
            Player::new(1, derivation.piece_seed, derivation.garbage_seeds[1]),
        ];
        if !start_garbage.is_empty() {
            for player in &mut players {
                player
                    .game
                    .insert_garbage(start_garbage)
                    .expect("start garbage must be valid");
            }
        }
        Self {
            players,
            seed,
            sheet,
            order: sheet.order(),
            turn: 0,
            frame: 0,
            frames_per_turn,
            multiplier: GarbageMultiplier::new(),
        }
    }

    /// Advances to the next paired turn and updates the frame-derived state.
    pub fn begin_turn(&mut self) {
        self.frame = self.turn as u64 * self.frames_per_turn;
        self.multiplier.advance_to(self.frame);
    }

    pub fn view(&mut self, slot: usize) -> View {
        let seat = self.sheet.seat_of(slot);
        let player = &mut self.players[slot];
        View {
            turn: self.turn,
            frame: self.frame,
            slot,
            seat,
            board: player.game.board().clone(),
            active: player.game.active().expect("active piece during a turn"),
            hold: player.game.held_piece(),
            can_hold: player.game.can_hold(),
            preview: player.game.preview(5),
            bag_state: player.game.bag_state().remaining,
            combo: player.scoring.state.combo,
            b2b_streak: player.scoring.state.b2b_streak,
            pending_garbage: player.garbage.queue_total(),
            pieces_placed: player.opener.pieces_placed,
            sent_total: player.opener.sent_total,
            received_total: player.game.stats().garbage_lines_received,
            multiplier: self.multiplier.value(),
        }
    }

    /// Adopts the first candidate that is in the referee's own reachable move
    /// set (`docs/s2-referee-protocol.md` §4.3).
    ///
    /// Hold is inferred from the candidate's piece: a candidate whose piece is
    /// not the active piece is a hold. The hold slot must contain that piece
    /// (or, when the slot is empty, it must be the next bag piece).
    pub fn adopt(
        &mut self,
        slot: usize,
        suggestion: &crate::agent::Suggestion,
    ) -> Result<Adopted, AdoptError> {
        let active = self.players[slot]
            .game
            .active()
            .ok_or(AdoptError::NoActivePiece)?;
        let current_kind = active.kind;
        let can_hold = self.players[slot].game.can_hold();
        let hold_kind = self.players[slot]
            .game
            .held_piece()
            .or_else(|| self.players[slot].game.preview(1).first().copied());
        let board = self.players[slot].game.board().clone();

        let mut current_moves: Option<movegen::MoveSet> = None;
        let mut hold_moves: Option<movegen::MoveSet> = None;
        for (index, candidate) in suggestion.candidates.iter().enumerate() {
            let is_hold = candidate.piece != current_kind;
            if is_hold && (!can_hold || hold_kind != Some(candidate.piece)) {
                continue;
            }
            let moves = if is_hold {
                hold_moves.get_or_insert_with(|| {
                    movegen::generate(&board, movegen::spawn_state(candidate.piece), true)
                })
            } else {
                current_moves.get_or_insert_with(|| movegen::generate(&board, active, false))
            };
            if moves.get(&candidate.cells).is_some() {
                return Ok(Adopted {
                    candidate_index: index,
                    target: PlacementTarget::new(candidate.piece, candidate.cells),
                    claimed_spin: candidate.claimed_spin,
                    claimed_attack: candidate.claimed_attack,
                });
            }
        }
        // Two-stage decision (protocol v1.6 §7): if the referee has any legal
        // placement, the bot's empty/unreachable suggestion is a forfeit;
        // otherwise the board is stuck and the round ends as a top-out.
        if self.has_legal_move(slot) {
            Err(AdoptError::NoLegalCandidate {
                candidates: suggestion.candidates.len(),
            })
        } else {
            Err(AdoptError::NoMoves)
        }
    }

    /// True when the player has at least one reachable grounded placement with
    /// the current piece or, if hold is available, with the hold piece.
    pub fn has_legal_move(&mut self, slot: usize) -> bool {
        let player = &mut self.players[slot];
        let Some(active) = player.game.active() else {
            return false;
        };
        let board = player.game.board().clone();
        if !movegen::generate(&board, active, false).is_empty() {
            return true;
        }
        if player.game.can_hold() {
            if let Some(kind) = player
                .game
                .held_piece()
                .or_else(|| player.game.preview(1).first().copied())
            {
                if !movegen::generate(&board, movegen::spawn_state(kind), true).is_empty() {
                    return true;
                }
            }
        }
        false
    }

    /// Applies one placement: hold, path replay, lock, scoring, cancellation,
    /// sending and insertion (`docs/s2-referee-protocol.md` §5).
    pub fn apply(
        &mut self,
        slot: usize,
        target: &PlacementTarget,
        claimed_spin: Option<SpinType>,
    ) -> Result<AppliedPlacement, RefereeError> {
        let frame = self.frame;

        let current = self.players[slot]
            .game
            .active()
            .ok_or(RefereeError::NoActivePiece)?;
        let hold = target.piece != current.kind;

        if hold {
            if !self.players[slot].game.can_hold() {
                return Err(RefereeError::HoldUnavailable);
            }
            let expected = self.players[slot]
                .game
                .held_piece()
                .or_else(|| self.players[slot].game.preview(1).first().copied());
            if expected != Some(target.piece) {
                return Err(RefereeError::HoldMismatch {
                    expected,
                    got: target.piece,
                });
            }
            if !self.players[slot].game.hold() {
                return Err(RefereeError::HoldFailed);
            }
        }

        let active = self.players[slot]
            .game
            .active()
            .ok_or(RefereeError::NoActivePiece)?;
        if active.kind != target.piece {
            return Err(RefereeError::PieceMismatch {
                expected: target.piece,
                got: active.kind,
            });
        }

        let moves = movegen::generate(self.players[slot].game.board(), active, hold);
        let placement = moves
            .get(&target.cells)
            .ok_or(RefereeError::IllegalPlacement {
                piece: target.piece,
                cells: target.cells,
            })?
            .clone();

        for action in &placement.path {
            let result = self.players[slot].game.apply(*action);
            if !result.accepted {
                return Err(RefereeError::PathRejected { action: *action });
            }
        }

        let expected_piece = ActivePiece {
            kind: placement.kind,
            rotation: placement.rotation,
            x: placement.x,
            y: placement.y,
        };
        if self.players[slot].game.active() != Some(expected_piece) {
            return Err(RefereeError::PathMismatch {
                expected: expected_piece,
                got: self.players[slot].game.active(),
            });
        }

        let combo_before = self.players[slot].scoring.state.combo;
        let b2b_before = self.players[slot].scoring.state.b2b_streak;
        let multiplier = self.multiplier.value();

        let lock = self.players[slot]
            .game
            .lock()
            .ok_or(RefereeError::LockFailed)?;

        let input = ClearInput {
            lines: lock.clear.lines,
            spin: placement.spin,
            combo: combo_before,
            b2b_streak: b2b_before,
            perfect_clear: lock.clear.perfect_clear,
            garbage_cleared: lock.clear.garbage_lines as u32,
            multiplier,
        };
        let attack = self.players[slot].scoring.score(input);
        let declared_attack =
            claimed_spin.map(|spin| compute_attack(&ClearInput { spin, ..input }).total);

        let (player, opponent_player) = split_players(&mut self.players, slot);
        let mut sent = 0;
        let mut cancelled = 0;
        let mut packets = Vec::new();
        for &event in &attack.events {
            let result = player.garbage.cancel_with_opener(event, &mut player.opener);
            sent += result.sent;
            cancelled += result.cancelled;
            if result.sent > 0 {
                opponent_player.garbage.receive(result.sent, frame);
                packets.push((result.sent, frame));
            }
        }
        player.opener.on_piece_placed();

        player.attack_events += attack.events.len() as u32;
        player.attack_total += attack.total;
        player.max_combo = player.max_combo.max(attack.combo_after);
        player.max_b2b = player.max_b2b.max(attack.b2b_streak_after);
        player.surge_released += attack.surge_amount;

        let mut inserted = Vec::new();
        if lock.clear.lines == 0 && !player.game.is_over() {
            let holes = player.garbage.take_insertable(0, frame);
            if !holes.is_empty() {
                player
                    .game
                    .insert_garbage(&holes)
                    .map_err(RefereeError::Garbage)?;
                inserted = holes;
            }
        }

        Ok(AppliedPlacement {
            slot,
            piece: placement.kind,
            rotation: placement.rotation,
            x: placement.x,
            y: placement.y,
            hold,
            spin: placement.spin,
            lines: lock.clear.lines,
            garbage_cleared: lock.clear.garbage_lines as u32,
            perfect_clear: lock.clear.perfect_clear,
            events: attack.events.clone(),
            sent,
            cancelled,
            attack_total: attack.total,
            declared_attack,
            combo_before,
            b2b_before,
            multiplier,
            packets,
            inserted,
            combo_after: attack.combo_after,
            b2b_after: attack.b2b_streak_after,
            topped_out: player.game.is_over(),
            board_hash: board_hash(player.game.board()),
        })
    }
}

fn split_players(players: &mut [Player; 2], slot: usize) -> (&mut Player, &mut Player) {
    let (first, second) = players.split_at_mut(1);
    if slot == 0 {
        (&mut first[0], &mut second[0])
    } else {
        (&mut second[0], &mut first[0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sheet_orders_are_opposite() {
        assert_eq!(Sheet::A.order(), [0, 1]);
        assert_eq!(Sheet::B.order(), [1, 0]);
        assert_eq!(Sheet::A.seat_of(0), 1);
        assert_eq!(Sheet::A.seat_of(1), 2);
        assert_eq!(Sheet::B.seat_of(0), 2);
        assert_eq!(Sheet::B.seat_of(1), 1);
    }

    #[test]
    fn engine_derives_the_same_piece_stream_for_both_players() {
        let mut engine = Engine::new(12_345, Sheet::A, 12);
        engine.begin_turn();
        let a = engine.players[0].game.active().unwrap();
        let b = engine.players[1].game.active().unwrap();
        assert_eq!(a.kind, b.kind);
        assert_eq!(a, b);
        assert_eq!(
            engine.players[0].game.preview(5),
            engine.players[1].game.preview(5)
        );
    }

    #[test]
    fn garbage_seeds_are_per_slot_and_sheet_independent() {
        let a = Engine::new(7, Sheet::A, 12);
        let b = Engine::new(7, Sheet::B, 12);
        assert_eq!(a.players[0].garbage, b.players[0].garbage);
        assert_eq!(a.players[1].garbage, b.players[1].garbage);
        assert_ne!(a.players[0].garbage, a.players[1].garbage);
    }

    #[test]
    fn empty_suggestion_is_a_forfeit_when_the_referee_has_moves() {
        let mut engine = Engine::new(1, Sheet::A, 12);
        engine.begin_turn();
        assert!(engine.has_legal_move(0));
        let suggestion = crate::agent::Suggestion::default();
        let error = engine.adopt(0, &suggestion).unwrap_err();
        assert!(matches!(
            error,
            AdoptError::NoLegalCandidate { candidates: 0 }
        ));
    }

    #[test]
    fn applying_a_quad_scores_and_sends() {
        // Rows 0..3 full except column 0: a vertical I in column 0 is a quad.
        let mut board = Board::new();
        for y in 0..4 {
            board.set_row(y, Board::FULL_ROW & !0b1);
        }
        let derivation = derive_seeds(1);
        let mut engine = Engine::new(1, Sheet::A, 12);
        let active = ActivePiece {
            kind: PieceKind::I,
            rotation: Rotation::Zero,
            x: 3,
            y: 19,
        };
        engine.players[0].game =
            Game::from_position(derivation.piece_seed, board.clone(), active).unwrap();
        engine.begin_turn();
        let moves = movegen::generate(engine.players[0].game.board(), active, false);
        let placement = moves
            .placements()
            .iter()
            .find(|placement| placement.lines_after(&board) == 4)
            .expect("a quad must be reachable");
        let applied = engine
            .apply(
                0,
                &PlacementTarget::new(placement.kind, placement.cells),
                None,
            )
            .unwrap();
        assert_eq!(applied.lines, 4);
        // The board becomes empty, so this is also a perfect clear: quad (4)
        // plus PC (+5), sent as two packets (docs/s2-spec.md §6.5 / §7.2).
        assert!(applied.perfect_clear);
        assert_eq!(applied.events, vec![4, 5]);
        assert_eq!(applied.attack_total, 9);
        assert_eq!(applied.sent, applied.attack_total);
        assert_eq!(engine.players[1].garbage.queue_total(), applied.sent);
        assert_eq!(applied.packets.len(), 2);
    }
}
