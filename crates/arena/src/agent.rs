//! Bot interface used by the referee.
//!
//! Two implementations exist:
//! - [`crate::tbp_agent::TbpAgent`] hosts an external TBP bot (attax).
//! - [`crate::baseline::BaselineBot`] is an in-process stand-in for
//!   intetrigence until `crates/bot` lands. It is **not** the real bot; M0 only
//!   proves the plumbing, audit and replay path (PLAN.md §5 M0).
//!
//! A bot returns candidates in priority order. The referee adopts the first
//! candidate that is in its own reachable move set (`docs/s2-referee-protocol.md`
//! §4.3) and never trusts a claimed spin label for scoring.

use std::time::Instant;

use intetrigence_core::{PieceKind, SpinType};

use crate::engine::{PlacementTarget, View};

/// Identity recorded in the replay header and the audit log.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BotDescriptor {
    pub name: String,
    pub version: Option<String>,
    pub commit: Option<String>,
    pub args: Vec<String>,
    /// Rows the bot can actually see (attax = 20, intetrigence = 40). Used to
    /// classify a forfeit as `vision` when the empty suggestion is explained by
    /// the hidden buffer (protocol v1.8 §7.1).
    pub visible_rows: u32,
    /// Whether the bot's input set includes 180-degree rotations. Attax does
    /// not; the referee's capability-aware forfeit check uses this.
    pub supports_180: bool,
    /// Content hash of the sources that produced this bot (`crate::BUILD_ID`).
    /// `commit` records the checkout, not the binary; this records the code.
    pub build_id: Option<String>,
}

/// One candidate placement, identified by its absolute cell set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub piece: PieceKind,
    pub cells: [(i32, i32); 4],
    /// Spin label claimed by the bot (audit only; never used for scoring).
    pub claimed_spin: Option<SpinType>,
    /// Attack the bot claims for this placement (audit only).
    pub claimed_attack: Option<u32>,
}

impl Candidate {
    pub fn target(&self) -> PlacementTarget {
        PlacementTarget::new(self.piece, self.cells)
    }
}

/// A bot's answer for one placement.
#[derive(Clone, Debug, Default)]
pub struct Suggestion {
    pub candidates: Vec<Candidate>,
    /// Wall-clock time spent producing the suggestion.
    pub think_ms: f64,
    /// Search statistics reported by the bot (attax `move_info`).
    pub nodes: Option<u64>,
    pub extra_ms: Option<f64>,
    /// Completed search depth of the winning iteration (protocol v1.31
    /// §8.4.1). `None` when the bot does not report it (TBP opponents).
    pub depth: Option<u32>,
    /// Beam width of the winning iteration (protocol v1.31 §8.4.1).
    pub width: Option<u32>,
    /// True when the move came from the 1-ply greedy fallback.
    pub fallback: bool,
    /// True when the search hit its hard deadline and returned the best
    /// completed iteration (the bot's `aborted` flag).
    pub aborted: bool,
}

/// Initial state passed to [`Agent::begin_round`].
#[derive(Clone, Debug)]
pub struct RoundContext {
    pub seed: u64,
    pub sheet: crate::engine::Sheet,
    pub slot: usize,
    pub seat: u8,
    pub initial: View,
}

/// Authoritative outcome of one placement, sent back to the bot.
#[derive(Clone, Debug)]
pub struct Commit {
    pub turn: u32,
    pub frame: u64,
    pub piece: PieceKind,
    pub rotation: intetrigence_core::Rotation,
    pub x: i32,
    pub y: i32,
    pub hold: bool,
    pub spin: SpinType,
    pub lines: u8,
    pub events: Vec<u32>,
    pub sent: u32,
    pub cancelled: u32,
    pub board: intetrigence_core::Board,
    pub board_hash: u64,
    /// Current piece + five preview pieces (six entries).
    pub queue: Vec<PieceKind>,
    /// Pieces left in the current 7-bag at this point (TBP start.randomizer).
    pub bag_state: Vec<PieceKind>,
    pub hold_piece: Option<PieceKind>,
    pub combo: u32,
    pub b2b_streak: u32,
    pub garbage_inserted: bool,
    pub topped_out: bool,
}

impl Commit {
    pub fn tbp_b2b(&self) -> bool {
        self.b2b_streak >= 1
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentError {
    Io(String),
    Protocol(String),
    Referee(String),
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentError::Io(message) => write!(f, "agent i/o: {message}"),
            AgentError::Protocol(message) => write!(f, "agent protocol: {message}"),
            AgentError::Referee(message) => write!(f, "agent referee: {message}"),
        }
    }
}

impl std::error::Error for AgentError {}

/// Extra audit data an agent may expose after a round.
#[derive(Clone, Debug, Default)]
pub struct AgentAudit {
    /// `info` payload (TBP bots only).
    pub info_name: String,
    pub info_version: String,
    pub info_author: String,
    /// `nodes` summed over suggestions (attax `move_info.nodes`).
    pub nodes_total: u64,
    /// `extra` milliseconds summed over suggestions.
    pub extra_ms_total: f64,
    /// Number of suggestions that carried search statistics.
    pub suggestions_with_stats: u64,
    /// attax `ATTAX_TBP_LOG` path, if one was written.
    pub decision_log: Option<String>,
    /// Decisions parsed from the log.
    pub decisions_logged: u64,
    /// True if any logged decision had cells above TBP row 19.
    pub cells_above_row20: bool,
    /// Host-inferred hold vs attax `primary.hold` cross-check.
    pub hold_matches: u64,
    pub hold_mismatches: u64,
    /// `start` resends performed this round.
    pub start_resends: Vec<(String, u64, u32)>,
    /// Search calls that had to fall back to a direct placement (Clutch or a
    /// pushed-up spawn, where the search's standard-spawn assumption fails).
    pub clutch_fallbacks: u64,
    /// Host-side queue invariant violations (must be zero).
    pub queue_invariant_violations: u64,
    /// Host command line (reproducibility).
    pub command_line: Option<String>,
    /// `bun --version` output.
    pub bun_version: Option<String>,
}

pub trait Agent {
    fn descriptor(&self) -> BotDescriptor;

    fn begin_round(&mut self, context: &RoundContext) -> Result<(), AgentError>;

    fn suggest(&mut self, view: &View) -> Result<Suggestion, AgentError>;

    fn commit(&mut self, commit: &Commit) -> Result<(), AgentError>;

    fn end_round(&mut self) -> Result<(), AgentError>;

    fn audit(&self) -> AgentAudit {
        AgentAudit::default()
    }
}

/// Times `f` and returns its result with the elapsed milliseconds.
pub fn timed<T>(f: impl FnOnce() -> T) -> (T, f64) {
    let started = Instant::now();
    let value = f();
    (value, started.elapsed().as_secs_f64() * 1000.0)
}
