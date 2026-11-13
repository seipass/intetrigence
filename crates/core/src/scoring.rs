//! S2 attack scoring, combo, B2B Charging and surge.
//!
//! Reference: `docs/s2-spec.md` v1.3.1 §6.8. The pure entry point is
//! [`compute_attack`]; [`S2Scoring`] wraps it with the combo/B2B state.
//!
//! Important ordering rules:
//! - the B2B +1 check uses `N_after` (the streak after this clear);
//! - the surge amount uses `N_before` (the streak before this clear);
//! - the combo multiplier uses the combo before this clear;
//! - rounding happens once, after `base * multiplier + GSB`.

use crate::constants::{
    B2B_ATTACK_BONUS, B2B_CHARGE_AT, B2B_CHARGE_BASE, BASE_ATTACK, GARBAGE_INCREASE_PER_FRAME,
    GARBAGE_MARGIN_FRAMES, GARBAGE_MULTIPLIER_INIT, PC_B2B, PC_GARBAGE,
};

/// Spin classification for a clear (`docs/s2-spec.md` §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SpinType {
    None = 0,
    Mini = 1,
    Full = 2,
}

impl SpinType {
    pub const fn index(self) -> usize {
        self as usize
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "none" => Some(SpinType::None),
            "mini" => Some(SpinType::Mini),
            "full" => Some(SpinType::Full),
            _ => None,
        }
    }
}

/// Input to one placement's attack calculation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClearInput {
    pub lines: u8,
    pub spin: SpinType,
    /// Combo before this placement (0 = previous placement did not clear).
    pub combo: u32,
    /// Consecutive difficult clears before this placement (`N_before`).
    pub b2b_streak: u32,
    pub perfect_clear: bool,
    /// Garbage rows cleared by this placement.
    pub garbage_cleared: u32,
    /// Garbage multiplier at this placement's frame.
    pub multiplier: f64,
}

impl ClearInput {
    pub fn new(lines: u8, spin: SpinType) -> Self {
        Self {
            lines,
            spin,
            combo: 0,
            b2b_streak: 0,
            perfect_clear: false,
            garbage_cleared: 0,
            multiplier: 1.0,
        }
    }
}

/// Result of one placement's attack calculation.
#[derive(Clone, Debug, PartialEq)]
pub struct Attack {
    /// Attack events in send order: surge chunks, main attack, PC bonus.
    pub events: Vec<u32>,
    pub total: u32,
    pub combo_after: u32,
    /// `N_after`, the streak after this placement.
    pub b2b_streak_after: u32,
    /// Base after B2B +1 and the combo multiplier, before floor/GSB.
    pub base: f64,
    pub gsb: u32,
    /// Surge amount before the 3-way split (0 when no release).
    pub surge_amount: u32,
}

/// Base attack table lookup. `lines > 4` is not a TETRA LEAGUE clear.
pub fn base_attack(spin: SpinType, lines: u8) -> u32 {
    assert!(
        lines <= 4,
        "S2 TETRA LEAGUE cannot clear more than 4 lines (got {lines})"
    );
    BASE_ATTACK[spin.index()][lines as usize]
}

/// Surge charge derived from the B2B streak.
///
/// `b2b_disp = max(N - 1, 0)`; charge starts at display B2B x4 and equals the
/// display streak.
pub fn surge_charge_from_streak(b2b_streak: u32) -> u32 {
    let display = b2b_streak.saturating_sub(1);
    if display >= B2B_CHARGE_AT {
        display - B2B_CHARGE_AT + B2B_CHARGE_BASE + 1
    } else {
        0
    }
}

/// Splits a surge amount into `[r, r, S - 2r]` with `r = round_half_up(S/3)`.
///
/// Zero chunks are kept here; [`compute_attack`] filters them from the event
/// list.
pub fn split_surge(amount: u32) -> [u32; 3] {
    if amount == 0 {
        return [0, 0, 0];
    }
    let r = (amount as f64 / 3.0).round() as u32;
    [r, r, amount.saturating_sub(2 * r)]
}

/// Pure S2 attack calculation for one placement.
pub fn compute_attack(input: &ClearInput) -> Attack {
    assert!(
        input.lines <= 4,
        "S2 TETRA LEAGUE cannot clear more than 4 lines (got {})",
        input.lines
    );
    assert!(
        !input.perfect_clear || input.lines > 0,
        "a perfect clear must clear at least one line"
    );

    let n_before = input.b2b_streak;
    let combo = input.combo;
    let difficult = input.spin != SpinType::None || input.lines >= 4;

    // 1. Chain update (§5). PC adds exactly +1 B2B and never stacks with the
    // difficult-clear increment.
    let (combo_after, n_after, broke) = if input.lines > 0 {
        if input.perfect_clear {
            (combo + 1, n_before + PC_B2B, false)
        } else if difficult {
            (combo + 1, n_before + 1, false)
        } else {
            (combo + 1, 0, n_before >= 1)
        }
    } else {
        (0, n_before, false)
    };

    // 2. Base + flat B2B +1 (uses N_after).
    let mut base = base_attack(input.spin, input.lines) as f64;
    if input.lines > 0 && n_after >= 2 {
        base += B2B_ATTACK_BONUS as f64;
    }

    // 3. Combo multiplier. The ln term is a max, not a replacement.
    if combo >= 1 {
        base *= 1.0 + 0.25 * combo as f64;
    }
    if combo >= 2 {
        base = base.max((1.25 * combo as f64).ln_1p());
    }

    // 4. Garbage Special Bonus: flat +1, inside the final floor.
    let gsb = if input.garbage_cleared > 0 && (input.spin != SpinType::None || input.lines >= 4) {
        1
    } else {
        0
    };
    let main = if base > 0.0 || gsb > 0 {
        Some((base * input.multiplier + gsb as f64).floor() as u32)
    } else {
        None
    };

    // 5. Surge release. `broke` is true only for a non-difficult, non-PC line
    // clear; the amount uses N_before, not N_after.
    let mut events = Vec::new();
    let mut surge_amount = 0;
    if broke && n_before > B2B_CHARGE_AT {
        let amount = ((n_before - 1) as f64 * input.multiplier).floor() as u32;
        surge_amount = amount;
        if amount > 0 {
            for chunk in split_surge(amount) {
                if chunk > 0 {
                    events.push(chunk);
                }
            }
        }
    }

    // 6. Main attack, then the PC bonus.
    if let Some(main) = main {
        events.push(main);
    }
    if input.perfect_clear {
        events.push((PC_GARBAGE as f64 * input.multiplier).floor() as u32);
    }

    let total = events.iter().sum();
    Attack {
        events,
        total,
        combo_after,
        b2b_streak_after: n_after,
        base,
        gsb,
        surge_amount,
    }
}

/// Running combo / B2B state for one player.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScoreState {
    pub combo: u32,
    /// Consecutive difficult clears `N` (0 = no B2B).
    pub b2b_streak: u32,
}

impl ScoreState {
    pub const fn new() -> Self {
        Self {
            combo: 0,
            b2b_streak: 0,
        }
    }

    /// Display B2B count = `max(N - 1, 0)`.
    pub fn b2b_display(&self) -> u32 {
        self.b2b_streak.saturating_sub(1)
    }

    pub fn surge_charge(&self) -> u32 {
        surge_charge_from_streak(self.b2b_streak)
    }
}

/// Derived surge charge helper.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SurgeState {
    pub charge: u32,
}

impl SurgeState {
    pub const fn new() -> Self {
        Self { charge: 0 }
    }

    pub fn from_b2b_streak(b2b_streak: u32) -> Self {
        Self {
            charge: surge_charge_from_streak(b2b_streak),
        }
    }

    pub fn chunks(self) -> [u32; 3] {
        split_surge(self.charge)
    }
}

/// Stateful S2 scoring engine.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct S2Scoring {
    pub state: ScoreState,
    pub last: Option<Attack>,
}

impl S2Scoring {
    pub fn new() -> Self {
        Self::default()
    }

    /// Scores one placement using the engine's current combo/B2B state, then
    /// advances that state.
    pub fn score(&mut self, mut input: ClearInput) -> Attack {
        input.combo = self.state.combo;
        input.b2b_streak = self.state.b2b_streak;
        let attack = compute_attack(&input);
        self.state.combo = attack.combo_after;
        self.state.b2b_streak = attack.b2b_streak_after;
        self.last = Some(attack.clone());
        attack
    }
}

/// Scoring engine contract.
pub trait ScoringEngine {
    fn score(&mut self, input: ClearInput) -> Attack;
}

impl ScoringEngine for S2Scoring {
    fn score(&mut self, input: ClearInput) -> Attack {
        S2Scoring::score(self, input)
    }
}

/// Frame-accurate garbage multiplier.
///
/// The spec forbids a closed-form expression: TETR.IO adds
/// `0.008 / 60` once per frame after margin time, and the accumulated f64 can
/// differ from `1.0 + (frame - 10800) * 0.008 / 60` enough to change a floor.
#[derive(Clone, Debug, PartialEq)]
pub struct GarbageMultiplier {
    frame: u64,
    value: f64,
}

impl GarbageMultiplier {
    pub fn new() -> Self {
        Self {
            frame: 0,
            value: GARBAGE_MULTIPLIER_INIT,
        }
    }

    pub fn at_frame(frame: u64) -> Self {
        let mut multiplier = Self::new();
        multiplier.advance_to(frame);
        multiplier
    }

    /// Advances frame by frame, accumulating after margin time.
    pub fn advance_to(&mut self, frame: u64) {
        while self.frame < frame {
            self.frame += 1;
            if self.frame > GARBAGE_MARGIN_FRAMES {
                self.value += GARBAGE_INCREASE_PER_FRAME;
            }
        }
    }

    pub fn advance_by(&mut self, frames: u64) {
        self.advance_to(self.frame.saturating_add(frames));
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }
}

impl Default for GarbageMultiplier {
    fn default() -> Self {
        Self::new()
    }
}
