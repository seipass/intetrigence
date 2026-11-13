//! intetrigence core: pure, deterministic TETR.IO Season 2 rules engine.
//!
//! # Determinism
//!
//! `Game` is a pure function of `(seed, action sequence)`: the 7-bag uses a
//! fixed SplitMix64/Fisher-Yates protocol, and no operation reads wall-clock
//! time or OS randomness. Replays therefore do not need to reproduce search;
//! they only need the seed and the recorded actions.
//!
//! # Coordinate system
//!
//! Board coordinates are `x` right, `y` up, with row 0 at the bottom. The
//! board is 10 columns x 40 rows; rows 0..19 are visible and rows 20..39 are the
//! hidden buffer. A piece is positioned by the bottom-left corner of its SRS
//! bounding box. See [`piece`] for the true-rotation center of each piece.
//!
//! # Status
//!
//! Implemented: piece states, bitboard, SRS+ and 180-degree kicks
//! (`docs/s2-spec.md` §3.1/§3.2, isolated in [`kick`]), 7-bag, hold,
//! movement/rotation/drop/lock, garbage-row insertion, line clearing.
//!
//! Pending `docs/s2-spec.md`: attack scoring, combo, B2B charging/surge,
//! All-Mini+ spin classification, opener phase, and the garbage queue policy.
//! The pending pieces are present as types with `TODO(s2-spec)` methods, not as
//! guessed arithmetic.

#![forbid(unsafe_code)]

pub mod bag;
pub mod board;
pub mod game;
pub mod garbage;
pub mod hold;
pub mod kick;
pub mod opener;
pub mod piece;
pub mod rng;
pub mod scoring;

pub use bag::Bag;
pub use board::{
    Board, BoardInvariantError, Cell, ClearInfo, FULL_ROW, GarbageError, HEIGHT, PlacementError,
    VISIBLE_HEIGHT, WIDTH,
};
pub use game::{
    Action, ActionResult, ActivePiece, Game, GameInvariantError, GameStateError, HardDropResult,
    LockResult, RotationResult, Stats,
};
pub use garbage::{GarbageEntry, GarbageQueue};
pub use hold::Hold;
pub use kick::{Kick, is_fin_or_tst_kick, kicks};
pub use opener::{OPENER_PIECES, OpenerPhase};
pub use piece::{ALL_PIECES, PieceKind, RotateDirection, Rotation};
pub use rng::SplitMix64;
pub use scoring::{Attack, ClearEvent, S2Scoring, ScoreState, ScoringEngine, SpinType, SurgeState};
