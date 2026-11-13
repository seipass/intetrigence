//! S2 TETRA LEAGUE constants (docs/s2-spec.md v1.3.1).
//!
//! Numeric rules live here so a spec revision changes one file, not the
//! algorithms. The §11 U-items are decided in v1.3.1:
//! - U3: PC's +1 B2B also charges surge (S7 behavior).
//! - U5: Opener condition uses `queue_total >= sent_total`.
//! - U6: garbage insertion uses a plain 40-row board and discards rows above
//!   row 39 (the teto 41-row quirk is unreachable).
//! - U7: `rng::derive_seeds` advances one MINSTD stream; the old xor mix is
//!   removed because it made slot 0 share the bag stream.

/// Base attack table, indexed by `[spin][lines]`.
///
/// Rows: none, mini, full. Columns: 0..=4 lines.
pub const BASE_ATTACK: [[u32; 5]; 3] = [
    // none
    [0, 0, 1, 2, 4],
    // mini (All-Mini+; non-T spins are always mini)
    [0, 0, 1, 2, 10],
    // full (T only)
    [0, 2, 4, 6, 10],
];

/// Flat B2B Charging attack bonus (applied when `N_after >= 2`).
pub const B2B_ATTACK_BONUS: u32 = 1;

/// Display B2B at which surge starts charging (B2B x4).
pub const B2B_CHARGE_AT: u32 = 4;

/// S2 TL surge base (the general formula is `disp - at + base + 1`).
pub const B2B_CHARGE_BASE: u32 = 3;

/// Perfect-clear garbage before the multiplier.
pub const PC_GARBAGE: u32 = 5;

/// Perfect-clear B2B increment (only +1; does not stack with difficult +1).
pub const PC_B2B: u32 = 1;

/// Maximum garbage rows inserted per lock.
pub const GARBAGE_CAP: u32 = 8;

/// Garbage travel delay in frames (20 frames at 60 fps).
pub const GARBAGE_SPEED_FRAMES: u64 = 20;

/// Garbage hole width in columns.
pub const GARBAGE_HOLE_SIZE: u8 = 1;

/// Probability of rerolling the hole column after a packet is consumed.
pub const MESSINESS_CHANGE: f64 = 1.0;

/// Probability of rerolling the hole column within a packet.
pub const MESSINESS_WITHIN: f64 = 0.0;

/// Initial garbage multiplier.
pub const GARBAGE_MULTIPLIER_INIT: f64 = 1.0;

/// Margin time start in frames (180 s at 60 fps).
pub const GARBAGE_MARGIN_FRAMES: u64 = 10_800;

/// Per-frame multiplier increase after margin time (`0.008 / 60`).
///
/// Must be accumulated frame by frame; a closed-form expression differs in the
/// last bits and can change a floor result (spec §7.8).
pub const GARBAGE_INCREASE_PER_FRAME: f64 = 0.008 / 60.0;

/// Number of placements covered by the Opener Phase.
pub const OPENER_PIECES: u32 = 14;

/// U3 (decided): PC's +1 B2B contributes to surge charge.
pub const ALLCLEAR_CHARGES_SURGE: bool = true;

/// U5 (decided): opener uses `queue_total >= sent_total` (not `>`).
pub const OPENER_QUEUE_GE_SENT: bool = true;

/// U6 (decided): plain 40-row insertion; rows above row 39 are discarded.
pub const GARBAGE_DISCARD_TOP_ROWS: bool = true;
