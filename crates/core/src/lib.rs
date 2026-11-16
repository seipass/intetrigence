//! intetrigence core: pure, deterministic TETR.IO Season 2 rules engine.
//!
//! # Determinism
//!
//! `Game` is a pure function of `(seed, action sequence)`: the 7-bag uses a
//! fixed MINSTD/Fisher-Yates protocol, and no operation reads wall-clock time
//! or OS randomness. Replays therefore do not need to reproduce search; they
//! only need the seed and the recorded actions. [`rng::derive_seeds`] splits a
//! round seed into the shared piece seed and two per-bot garbage seeds
//! (`docs/s2-spec.md` v1.3.1 §11-U7).
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
//! movement/rotation/drop/lock, garbage-row insertion, line clearing,
//! All-Mini+ spin classification, attack scoring/combo/B2B/surge, opener
//! phase, and the garbage queue.
//!
//! The `docs/s2-attack-examples.json` fixtures are the reference test for
//! [`scoring::compute_attack`].

#![forbid(unsafe_code)]

pub mod bag;
pub mod board;
pub mod constants;
pub mod game;
pub mod garbage;
pub mod hold;
pub mod kick;
pub mod opener;
pub mod piece;
pub mod rng;
pub mod scoring;

pub use bag::{BAG_SIZE, Bag, BagState};
pub use board::{
    Board, BoardInvariantError, Cell, ClearInfo, FULL_ROW, GarbageError, HEIGHT, PlacementError,
    VISIBLE_HEIGHT, WIDTH,
};
pub use game::{
    Action, ActionResult, ActivePiece, Game, GameInvariantError, GameStateError, HardDropResult,
    LockResult, RotationResult, Stats,
};
pub use garbage::{CancelResult, GarbagePacket, GarbageQueue};
pub use hold::Hold;
pub use kick::{
    I_KICKS_SRS, I_KICKS_SRS_PLUS, Kick, KickOrder, is_fin_or_tst_kick, kicks, kicks_with_order,
};
pub use opener::{OPENER_PIECES, OpenerPhase};
pub use piece::{ALL_PIECES, PieceKind, RotateDirection, Rotation};
pub use rng::{MinStd, SeedDerivation, SplitMix64, derive_seeds};
pub use scoring::{
    Attack, ClearInput, GarbageMultiplier, S2Scoring, ScoreState, ScoringEngine, SpinType,
    SurgeState, base_attack, compute_attack, split_surge, surge_charge_from_streak,
};
