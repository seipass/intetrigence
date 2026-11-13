//! S2 attack scoring, combo, B2B charging and surge interfaces.
//!
//! **Status: interface only.** The arithmetic waits for `docs/s2-spec.md` to be
//! frozen. The types here are the contract the bot and referee will share; the
//! `TODO(s2-spec)` methods deliberately panic rather than return a plausible
//! but wrong number.
//!
//! PLAN.md §3.1 fixes the shape of the rules (base table, combo multiplier,
//! B2B charging, surge, All-Mini+, perfect clear and garbage bonus), but the
//! exact constants and the PC B2B treatment still need the spec lead's source
//! reconciliation.

use crate::piece::PieceKind;

/// Spin classification for a clear.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpinType {
    None,
    Mini,
    Full,
}

/// Everything the scoring layer needs to know about one locked piece.
///
/// Fields that depend on the frozen spec (combo/B2B/surge state) are supplied
/// explicitly so the scoring function stays pure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClearEvent {
    pub piece: PieceKind,
    pub lines: u8,
    pub spin: SpinType,
    pub perfect_clear: bool,
    pub garbage_in_clear: bool,
    pub combo: u32,
    pub b2b_streak: u32,
    pub opener: bool,
    pub surge_charge: u32,
}

/// Attack decomposition. The exact split between fields is provisional until
/// the spec is frozen; `total` is the number the referee applies.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Attack {
    pub base: u32,
    pub combo: u32,
    pub b2b: u32,
    pub surge: u32,
    pub total: u32,
}

/// Running scoring state for one player.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScoreState {
    pub combo: u32,
    pub b2b_streak: u32,
    pub surge_charge: u32,
    pub pieces_placed: u32,
}

/// B2B charging / surge state.
///
/// TODO(s2-spec): B2B Charging adds +1 per difficult clear, surge charges at
/// streak >= 4, and a non-B2B clear releases the charge in three parts. The
/// constants and the release schedule are not implemented.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SurgeState {
    pub charge: u32,
}

impl SurgeState {
    pub const fn new() -> Self {
        Self { charge: 0 }
    }

    /// TODO(s2-spec): update B2B/surge state for `event` and return the surge
    /// attack released by this clear.
    pub fn on_clear(&mut self, _event: &ClearEvent) -> u32 {
        todo!("TODO(s2-spec): B2B charging / surge release")
    }
}

/// Scoring engine contract.
pub trait ScoringEngine {
    fn score(&mut self, event: &ClearEvent) -> Attack;
}

/// S2 scoring engine placeholder.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct S2Scoring {
    pub state: ScoreState,
    pub surge: SurgeState,
}

impl ScoringEngine for S2Scoring {
    fn score(&mut self, _event: &ClearEvent) -> Attack {
        todo!("TODO(s2-spec): implement S2 attack scoring after docs/s2-spec.md freeze")
    }
}
