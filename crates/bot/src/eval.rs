//! S2 evaluation: board features, placement rewards and the T-slot cutout.
//!
//! The search scores a placement in two parts:
//!
//! 1. an immediate **reward** from the S2 scoring engine
//!    (`intetrigence_core::compute_attack`): attack lines, clear shape,
//!    perfect clear, B2B break, surge charge and combo;
//! 2. a **leaf value** from the board after the lock: Dellacherie-style shape
//!    features plus S2 state terms (surge charge, combo) and a T-slot cutout.
//!
//! The cutout follows the public cold-clear-2 idea: instead of only scoring
//! the shape of the stack, take the T pieces that are still available in the
//! queue, hypothetically drop them into the best T-spin slot, and credit the
//! lines they would clear. It is an *evaluation* term, not a search branch, so
//! it costs one small local scan per leaf.
//!
//! Weights are a first hand-set (M3 tunes them with SPSA). They are ours: no
//! value here is copied from another bot.

use intetrigence_core::{
    Board, ClearInput, PieceKind, Rotation, SpinType, compute_attack, surge_charge_from_streak,
};

use crate::movegen::Placement;

/// Columns of the playfield.
pub const COLUMNS: usize = 10;

/// Surge size (lines) at or above which a release is worth taking. Smaller
/// bursts are absorbed by a bot sitting at height 3-5.
pub const SURGE_LETHAL: u32 = 8;

/// Most garbage rows one appearance can insert (`GARBAGE_CAP`).
pub const GARBAGE_CAP: u32 = 8;

/// Pieces in the S2 opener phase.
pub const OPENER_PIECES: u32 = 14;

/// Combo value above which the state term stops growing. Attack/piece peaks at
/// combo 3-4 and declines after that.
pub const COMBO_CAP: u32 = 4;

/// Column height above which the non-linear danger term starts to bite.
pub const DANGER_THRESHOLD: u32 = 12;

/// Projected height (`max_height + pending`) at which the evaluation switches
/// to survival mode: the spawn's lowest cell is row 21, so this is the row at
/// which the next zero-line placement kills.
pub const SURVIVAL_PROJECTED: u32 = 21;

/// Projected height at which zero-line placements start to be discouraged.
pub const WARNING_PROJECTED: u32 = 18;

/// Height above which the stack starts to be punished quadratically.
pub const DANGER_BASE: i32 = 15;

/// Board shape features, all in "count" units so a weight is a plain scale.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Features {
    /// Height of the tallest column.
    pub max_height: f64,
    /// `max(0, max_height - DANGER_BASE)^2`.
    pub height_sq: f64,
    /// Value of a ready 4-deep well (`Features.quad_ready`).
    ///
    /// This is the "preparation" the depth-4 search cannot see: building a quad
    /// takes four pieces of holding a well open, and at four plies clearing a
    /// single always looks better.
    pub quad_ready: f64,
    /// Rows that are one cell short of full, with the hole at the top of its
    /// column (so one piece clears it), capped at 2. This is the "combo
    /// groove": a combo only continues if a clear is available every piece.
    pub combo_potential: f64,
    /// Column-wise `sum(max(0, h - DANGER_THRESHOLD)^2)`.
    ///
    /// `height_sq` only looks at the tallest column and is always on, which
    /// makes the evaluation want a flat board at every depth -- measured
    /// against CC2, a depth-4.4 search with it active becomes a downstacker
    /// that stops attacking (attack/piece 0.811 -> 0.722, quads 170 -> 113).
    /// This term is flat below the threshold and steep above it, so "attack
    /// normally, dig when it gets dangerous" is expressible.
    pub danger_height: f64,
    /// Empty cells with an occupied cell above them in the same column.
    pub holes: f64,
    /// Sum over holes of the occupied cells above them, capped at 4.
    pub hole_cover: f64,
    /// Rows that contain at least one hole.
    pub hole_rows: f64,
    /// Sum of absolute height differences between adjacent columns.
    pub bumpiness: f64,
    /// Sum of squared height differences between adjacent columns.
    pub bumpiness_sq: f64,
    /// Filled/empty changes per row, walls counting as filled.
    pub row_transitions: f64,
    /// Depth of the deepest well (capped at 4).
    pub well_depth: f64,
    /// Columns at least 3 lower than both neighbours, excluding the main well.
    pub extra_wells: f64,
    /// Rows that still contain garbage. Versus-only: 0 in single play.
    pub garbage_rows: f64,
}

/// Column heights, bottom-up.
pub fn heights(board: &Board) -> [i32; COLUMNS] {
    let mut heights = [0i32; COLUMNS];
    for (x, height) in heights.iter_mut().enumerate() {
        *height = board.column_height(x) as i32;
    }
    heights
}

/// Column heights plus the shape features, computed in one top-down pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoardAnalysis {
    pub heights: [i32; COLUMNS],
    pub features: Features,
}

/// Computes [`BoardAnalysis`] for `board`.
///
/// One pass over the rows gives the heights, the holes and their cover; the
/// bumpiness, well and transition terms then come from the heights. The search
/// calls this for every child, so it avoids the per-column `column_height`
/// scan that [`heights`] does.
pub fn analyse(board: &Board) -> BoardAnalysis {
    let mut heights = [0i32; COLUMNS];
    let mut features = Features::default();
    let mut seen = 0u16;
    let mut cover = [0u8; COLUMNS];

    for y in (0..Board::HEIGHT).rev() {
        let row = board.row(y);

        // Holes in this row: empty cells with a filled cell somewhere above.
        let mut holes = !row & seen & Board::FULL_ROW;
        if holes != 0 {
            features.hole_rows += 1.0;
            while holes != 0 {
                let x = holes.trailing_zeros() as usize;
                holes &= holes - 1;
                features.holes += 1.0;
                features.hole_cover += f64::from(cover[x].min(4));
            }
        }

        let mut filled = row;
        while filled != 0 {
            let x = filled.trailing_zeros() as usize;
            filled &= filled - 1;
            if heights[x] == 0 {
                heights[x] = y as i32 + 1;
            }
            cover[x] = (cover[x] + 1).min(4);
        }
        seen |= row;
    }

    let max_height = heights.iter().copied().max().unwrap_or(0);
    let over = (max_height - DANGER_BASE).max(0);
    features.max_height = max_height as f64;
    features.height_sq = (over * over) as f64;
    features.quad_ready = (1..COLUMNS - 1)
        .filter(|&x| {
            heights[x] + 4 <= heights[x - 1]
                && heights[x] + 4 <= heights[x + 1]
                && (0..heights[x] as usize).all(|row| board.row(row) & (1 << x) != 0)
        })
        .count()
        .min(1) as f64;
    features.combo_potential = (0..Board::HEIGHT)
        .filter(|&row| {
            let mask = board.row(row);
            mask.count_ones() == 9
                && (0..COLUMNS).any(|x| mask & (1 << x) == 0 && heights[x] as usize == row + 1)
        })
        .count()
        .min(2) as f64;
    features.danger_height = heights
        .iter()
        .map(|&h| {
            let over = (h - DANGER_THRESHOLD as i32).max(0);
            f64::from(over * over)
        })
        .sum();

    // Main well: the lowest column, leftmost on a tie.
    let mut well = 0usize;
    for x in 1..COLUMNS {
        if heights[x] < heights[well] {
            well = x;
        }
    }
    let left = if well == 0 {
        Board::HEIGHT as i32
    } else {
        heights[well - 1]
    };
    let right = if well + 1 == COLUMNS {
        Board::HEIGHT as i32
    } else {
        heights[well + 1]
    };
    let depth = left.min(right) - heights[well];
    features.well_depth = depth.clamp(0, 4) as f64;

    for x in 0..COLUMNS {
        let height = heights[x];
        let left = if x == 0 { 0 } else { heights[x - 1] };
        let right = if x + 1 == COLUMNS { 0 } else { heights[x + 1] };
        // Bumpiness ignores the pairs that form the main well, so building a
        // well is not punished as bumpiness.
        let touches_well = x == well || x + 1 == well;
        if !touches_well {
            let delta = (height - right).abs();
            features.bumpiness += f64::from(delta);
            features.bumpiness_sq += f64::from(delta * delta);
        }
        if x != well && height + 3 <= left && height + 3 <= right {
            features.extra_wells += 1.0;
        }
    }

    // Row transitions and the garbage rows: walls count as filled, empty rows
    // are skipped.
    for y in 0..Board::HEIGHT {
        let row = board.row(y);
        if board.garbage_row(y) != 0 {
            features.garbage_rows += 1.0;
        }
        if row == 0 {
            continue;
        }
        let mask = (row << 1) | 0x801;
        features.row_transitions += f64::from((mask ^ (mask >> 1)).count_ones());
    }

    BoardAnalysis { heights, features }
}

/// Computes the shape features of `board`.
pub fn features(board: &Board) -> Features {
    analyse(board).features
}

/// Weight vector for [`Features`] and the placement rewards.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weights {
    // Board shape (negative is bad).
    pub max_height: f64,
    pub height_sq: f64,
    /// Column-wise non-linear danger (`Features.danger_height`).
    pub danger_height: f64,
    /// Value of a ready 4-deep well (`Features.quad_ready`).
    pub quad_ready: f64,
    /// Value per row of garbage a clear stops from appearing.
    ///
    /// `GarbageQueue::take_insertable` returns nothing when the lock cleared at
    /// least one line, and one appearance is at most `GARBAGE_CAP = 8` rows, so
    /// clearing a single while `pending` is high is worth up to 8 rows of
    /// defence -- more than a quad's 5 attack. Without this term a single is
    /// worth only "one row less height", so the bot stacks instead (the 9004-A
    /// collapse: 26 attack sent, final y 21).
    pub defense: f64,
    /// Bonus for clearing at least one line during the opener phase.
    /// S2 doubles the cancellation while the opener is active, but the extra
    /// budget equals the *attack*, not the lines, so a single (attack 0)
    /// cancels nothing. Measured on the 9007-B collapse: 23 lines for 15
    /// attack (attack/line 0.65) against the opponent's 16 lines for 28
    /// (1.75), with a nine-turn zero-line run at the start.
    pub opener_clear: f64,
    /// Value of the combo state itself: `combo_state * ln(1.25*combo + 1)`.
    /// S2 pays `base * (1 + 0.25*combo)` and lifts `base` to
    /// `ln(1.25*combo + 1)` once combo >= 2, so a live combo is a *state* with
    /// value: holding it is what makes the next clear worth 2-3 lines.
    pub combo_state: f64,
    /// Value of a board that can keep a combo going (`Features.combo_potential`).
    pub combo_potential: f64,
    /// Value per line of race lead (`sent_total - received_total`). Nothing in
    /// the TBP stream exposes the opponent's board, so the score difference is
    /// the only signal that says "we are winning the race"; without it the
    /// evaluation can only see our own board and our own attack.
    pub race: f64,
    pub holes: f64,
    pub hole_cover: f64,
    pub hole_rows: f64,
    pub bumpiness: f64,
    pub bumpiness_sq: f64,
    pub row_transitions: f64,
    pub well_depth: f64,
    pub extra_wells: f64,
    /// Value of a T-slot that would clear 0, 1 or 2 lines.
    pub t_slot: [f64; 3],
    /// Danger of the *projected* height once pending garbage lands:
    /// `max(0, max_height + incoming - DANGER_BASE)^2`. This is the versus
    /// term that makes a spike visible to the search.
    pub danger_projected: f64,
    /// Penalty per row that still contains garbage (uncleared garbage).
    pub garbage_rows: f64,

    // Placement rewards.
    /// Value per line of attack sent (before cancelling).
    pub attack: f64,
    /// Extra value by `[spin][lines]`.
    pub clear: [[f64; 5]; 3],
    pub perfect_clear: f64,
    pub b2b_break: f64,
    /// Penalty for spending a T without a T-spin while a slot was ready.
    pub wasted_t: f64,
    /// Value per line of surge currently charged, scaled by
    /// `charge^surge_exponent`. Exponent 1 is linear (the old behaviour);
    /// above 1 makes a large charge worth disproportionately more, which is
    /// the incentive to hold it for a 9-13 line burst instead of cashing a
    /// 4-7 line one. Measured on 372 games: 80% of releases were 4-7 lines.
    pub surge_charge: f64,
    /// Exponent applied to the charged amount in the leaf value.
    pub surge_exponent: f64,
    /// Extra value for a full T-spin double. A TSD pays 4 attack and clears
    /// two lines; against the 40-row attax we executed 40 of them to their
    /// 100, which is the single largest attack-composition gap measured.
    pub tsd_bonus: f64,
    /// Penalty for a mini spin. 109 of our 300 spins were mini singles
    /// against attax's 60; a mini single pays 1 attack where a full TSS pays 2.
    pub mini_penalty: f64,
    /// Penalty when a placement releases surge below `SURGE_LETHAL` lines.
    /// A small burst is absorbed by a bot sitting at height 3-5; the charge
    /// is worth more held.
    pub surge_hold_penalty: f64,
    /// Value per combo step after the placement.
    pub combo: f64,
    /// Extra value per attack line while the opener phase is active: the
    /// cancel budget is doubled for the first 14 placements
    /// (`docs/s2-spec.md` §7.4), so early attack is worth more.
    pub opener_attack: f64,
    /// Reward per garbage row cleared (downstack efficiency).
    pub garbage_clear: f64,
    /// Enable survival mode: when the queue is deep or the stack is high the
    /// shape terms are **removed** (not scaled) and every cleared line is
    /// worth `survival_line`, so the bot digs instead of building. The
    /// provisional losses all came from playing 7-8 zero-line placements
    /// while the queue grew to 10-12.
    pub survival: bool,
    /// Value of one cleared line in survival mode.
    pub survival_line: f64,
    /// Penalty per row of pending garbage (linear, on top of the projected
    /// danger).
    pub incoming: f64,
    /// Penalty per soft-dropped row (keeps the bot from over-dropping).
    pub softdrop: f64,

    /// Per-ply discount.
    pub discount: f64,
}

impl Default for Weights {
    /// First hand-set weights. M3 replaces these with SPSA-tuned values.
    fn default() -> Self {
        Self {
            max_height: -3.0,
            height_sq: 0.0,
            danger_height: -12.0,
            race: 1.0,
            defense: 28.0,
            quad_ready: 30.0,
            opener_clear: 20.0,
            combo_state: 40.0,
            combo_potential: 6.0,
            holes: -30.0,
            hole_cover: -6.0,
            hole_rows: -16.0,
            bumpiness: -3.0,
            bumpiness_sq: -1.0,
            row_transitions: -3.0,
            well_depth: 8.0,
            extra_wells: -14.0,
            t_slot: [0.0, 20.0, 80.0],
            danger_projected: -12.0,
            garbage_rows: -6.0,

            attack: 28.0,
            // `attack/line` is the only lever on attack/piece (lines/piece is
            // pinned by the cell budget), and at combo 0 it is: single 0.00,
            // double 0.50, triple 0.67, quad 1.25, TSS 3.00, TSD 2.50. A TSD
            // is therefore worth twice a quad, and the measured spin line
            // ratio is 16% against the 23% that attack/line 1.73 needs.
            // No bonus for a plain clear: `weights.attack * attack.total`
            // already prices it exactly, and `compute_attack` returns 0 for a
            // single, 1 for a double, 5 for a quad or a TSD, 11 for a mini
            // quad. The old `-14` for a single made clearing *worse* than
            // stacking, which is what produced the nine- and ten-turn
            // zero-line runs in the 9007-B collapse.
            clear: [
                [0.0, 0.0, 0.0, 0.0, 0.0],
                [0.0, -5.0, 2.0, 8.0, 30.0],
                [0.0, 20.0, 40.0, 40.0, 50.0],
            ],
            perfect_clear: 120.0,
            b2b_break: -20.0,
            wasted_t: -18.0,
            surge_charge: 1.5,
            surge_exponent: 1.0,
            surge_hold_penalty: 0.0,
            tsd_bonus: 0.0,
            mini_penalty: 0.0,
            combo: 12.0,
            opener_attack: 6.0,
            garbage_clear: 10.0,
            survival: true,
            survival_line: 12.0,
            incoming: -1.0,
            softdrop: -0.05,

            discount: 0.97,
        }
    }
}

/// Scoring state carried through the search.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EvalContext {
    /// Consecutive difficult clears before the next placement.
    pub b2b_streak: u32,
    /// Combo before the next placement (0 = the previous placement missed).
    pub combo: u32,
    /// Pieces placed before the next placement.
    pub pieces_placed: u32,
    /// Pending garbage rows, including packets still in flight.
    pub incoming: u32,
    /// Arrived garbage rows: the rows a clear on this placement would actually
    /// block (`docs/s2-burst-defense.md` §6.7).
    pub arrived: u32,
    /// Attack lines sent so far in this game.
    pub sent_total: u32,
    /// Garbage rows received so far in this game.
    pub received_total: u32,
    /// Lines a T could clear in the current board (0/1/2), for `wasted_t`.
    pub t_slot: u8,
}

impl EvalContext {
    /// Opener phase is active for the first 14 placements (`docs/s2-spec.md`
    /// §7.4).
    pub fn opener(&self) -> bool {
        self.pieces_placed < 14
    }
}

/// What one placement did, in S2 scoring terms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reward {
    /// Attack lines produced (surge chunks + main + perfect clear).
    pub attack: u32,
    /// Lines cleared.
    pub lines: u8,
    pub spin: SpinType,
    pub perfect_clear: bool,
    /// The placement broke a live B2B streak with an ordinary clear.
    pub broke_b2b: bool,
    /// Surge lines released by this placement.
    pub surge_released: u32,
    /// Surge lines still charged afterwards.
    pub surge_charged: u32,
    /// Garbage rows cleared by this placement.
    pub garbage_cleared: u8,
    pub combo_after: u32,
    pub b2b_after: u32,
    /// Weighted immediate reward.
    pub value: f64,
}

impl Default for Reward {
    fn default() -> Self {
        Self {
            attack: 0,
            lines: 0,
            spin: SpinType::None,
            perfect_clear: false,
            broke_b2b: false,
            surge_released: 0,
            surge_charged: 0,
            garbage_cleared: 0,
            combo_after: 0,
            b2b_after: 0,
            value: 0.0,
        }
    }
}

/// Applies `placement` to `board`, scores it with the S2 engine and returns
/// the resulting board, reward and context.
pub fn apply_placement(
    board: &Board,
    placement: &Placement,
    context: &EvalContext,
    weights: &Weights,
) -> (Board, Reward, EvalContext) {
    let mut next = board.clone();
    next.place(
        placement.piece.kind,
        placement.piece.rotation,
        placement.piece.x,
        placement.piece.y,
    )
    .expect("a generated placement is always legal");
    let clear = next.clear_lines();

    let difficult = placement.spin != SpinType::None || clear.lines >= 4;
    let broke_b2b =
        clear.lines > 0 && !difficult && !clear.perfect_clear && context.b2b_streak >= 1;

    let attack = compute_attack(&ClearInput {
        lines: clear.lines,
        spin: placement.spin,
        combo: context.combo,
        b2b_streak: context.b2b_streak,
        perfect_clear: clear.perfect_clear,
        garbage_cleared: u32::from(clear.garbage_lines),
        multiplier: 1.0,
    });

    let surge_charged = surge_charge_from_streak(attack.b2b_streak_after);
    let wasted_t = placement.piece.kind == PieceKind::T
        && context.t_slot > 0
        && placement.spin == SpinType::None;

    // Survival is decided by the *projected* height, not by either term alone.
    // Every spawn's lowest cell is row 21 and `insert_garbage` re-spawns the
    // active piece, topping out on the spot if it cannot be placed, so the
    // bot dies exactly when `height + pending >= 21`. The old condition was an
    // OR (`incoming >= 8 || height >= 15`), which switched to defence at
    // `height 15 / pending 0` (safe: 15 <= 20) and at `height 5 / pending 8`
    // (safe: 13 <= 20) -- it threw away attack while still alive.
    // One appearance inserts at most `GARBAGE_CAP` rows, so the height that
    // matters is `height + min(pending, 8)`, not `height + pending`.
    let projected = board.height() as u32 + context.arrived.min(GARBAGE_CAP);
    let survival = weights.survival && projected >= SURVIVAL_PROJECTED;
    let mut value = weights.attack * f64::from(attack.total);
    value += if survival {
        weights.survival_line * f64::from(clear.lines)
    } else {
        weights.clear[placement.spin.index()][clear.lines as usize]
    };
    if clear.lines > 0 && context.arrived >= 1 {
        // `risk` keeps the term quiet while the stack is low: stopping an
        // appearance only matters when one would kill us.
        let risk = ((board.height() as f64 - 12.0) / 6.0).clamp(0.0, 1.0);
        value += weights.defense * f64::from(context.arrived.min(GARBAGE_CAP)) * risk;
    }
    if context.pieces_placed < OPENER_PIECES && clear.lines > 0 {
        value += weights.opener_clear;
    }
    if placement.spin == SpinType::Full && clear.lines == 2 {
        value += weights.tsd_bonus;
    }
    if placement.spin == SpinType::Mini {
        value += weights.mini_penalty;
    }
    if clear.perfect_clear {
        value += weights.perfect_clear;
    }
    if broke_b2b {
        value += weights.b2b_break;
    }
    if wasted_t {
        value += weights.wasted_t;
    }
    value += weights.surge_charge * f64::from(surge_charged).powf(weights.surge_exponent);
    if attack.surge_amount > 0 && attack.surge_amount < SURGE_LETHAL {
        value += weights.surge_hold_penalty;
    }
    value += weights.combo * f64::from(attack.combo_after);
    value += weights.garbage_clear * f64::from(clear.garbage_lines);
    if context.opener() {
        value += weights.opener_attack * f64::from(attack.total);
    }

    let reward = Reward {
        attack: attack.total,
        lines: clear.lines,
        spin: placement.spin,
        perfect_clear: clear.perfect_clear,
        broke_b2b,
        surge_released: attack.surge_amount,
        surge_charged,
        garbage_cleared: clear.garbage_lines,
        combo_after: attack.combo_after,
        b2b_after: attack.b2b_streak_after,
        value,
    };
    // Attack cancels queued garbage before anything is sent
    // (`docs/s2-spec.md` §7.3), and the opener phase doubles the budget
    // (§7.4). Propagating this through the search is what lets it see that a
    // quad now is worth more than a quad later when a spike is queued.
    let cancel_budget = if context.opener() {
        2 * attack.total
    } else {
        attack.total
    };
    let next_context = EvalContext {
        sent_total: context.sent_total,
        received_total: context.received_total,
        b2b_streak: attack.b2b_streak_after,
        combo: attack.combo_after,
        pieces_placed: context.pieces_placed + 1,
        incoming: context.incoming.saturating_sub(cancel_budget),
        arrived: context.arrived.saturating_sub(cancel_budget),
        t_slot: 0,
    };
    (next, reward, next_context)
}

/// Value of the board after a lock, including the T-slot cutout.
///
/// `t_available` is how many T pieces are still in the queue (including hold);
/// the cutout credits at most two of them.
pub fn leaf_value(
    board: &Board,
    context: &EvalContext,
    weights: &Weights,
    t_available: usize,
) -> f64 {
    let BoardAnalysis { heights, features } = analyse(board);
    // Survival is decided by the *projected* height, not by either term alone.
    // Every spawn's lowest cell is row 21 and `insert_garbage` re-spawns the
    // active piece, topping out on the spot if it cannot be placed, so the
    // bot dies exactly when `height + pending >= 21`. The old condition was an
    // OR (`incoming >= 8 || height >= 15`), which switched to defence at
    // `height 15 / pending 0` (safe: 15 <= 20) and at `height 5 / pending 8`
    // (safe: 13 <= 20) -- it threw away attack while still alive.
    // One appearance inserts at most `GARBAGE_CAP` rows, so the height that
    // matters is `height + min(pending, 8)`, not `height + pending`.
    let projected = board.height() as u32 + context.arrived.min(GARBAGE_CAP);
    let survival = weights.survival && projected >= SURVIVAL_PROJECTED;
    // `wait_pieces = 21 - height - pending`: how many pieces can be spent on
    // preparation before a zero-line placement kills. No budget, no
    // preparation.
    let wait_pieces =
        (SURVIVAL_PROJECTED as i32 - board.height() as i32 - context.incoming as i32).max(0);
    let readiness_scale = (f64::from(wait_pieces) / 4.0).min(1.0);
    // Height is a *risk*, not a cost: at `height 5` a single is the most
    // height-efficient clear there is (1 row per piece against a quad's 0.4),
    // so pricing height at every level makes the bot farm singles forever and
    // never build a burst. Measured: the TurnCap game sent 373 lines against
    // the opponent's 715 at `attack/line` 0.53, with 53% of clears singles.
    //
    // The floor is 0.1, not 0.0: with zero the bot stops maintaining a low
    // stack at all and dies to its own height. Measured with 0.0: 17W-3L and
    // **all three losses had positive net damage** (+48, +7, +19), i.e. the
    // opponent was out-damaged and the bot still topped out -- and the median
    // think time went from ~30 ms to 72 ms because the taller board has more
    // candidates. A floor of 0.2 was too high (TurnCap 1/20, `attack/line`
    // 1.12); 0.1 is the middle.
    let height_risk = ((features.max_height - 12.0) / 6.0).clamp(0.1, 1.0);
    let mut value = height_risk
        * (weights.max_height * features.max_height
            + weights.height_sq * features.height_sq
            + weights.danger_height * features.danger_height)
        + readiness_scale * weights.quad_ready * features.quad_ready
        + weights.holes * features.holes
        + weights.hole_cover * features.hole_cover
        + weights.hole_rows * features.hole_rows
        + weights.bumpiness * features.bumpiness
        + weights.bumpiness_sq * features.bumpiness_sq
        + weights.row_transitions * features.row_transitions
        + weights.well_depth * features.well_depth
        + if survival {
            // Survival mode removes the shape terms instead of scaling them:
            // under a deep queue a hole is worth less than a cleared row.
            0.0
        } else {
            weights.holes * features.holes
                + weights.hole_cover * features.hole_cover
                + weights.hole_rows * features.hole_rows
                + weights.bumpiness * features.bumpiness
                + weights.bumpiness_sq * features.bumpiness_sq
                + weights.row_transitions * features.row_transitions
                + weights.well_depth * features.well_depth
                + weights.extra_wells * features.extra_wells
        };

    // Versus: the height this stack will have once the pending garbage lands.
    let projected = features.max_height + f64::from(context.incoming);
    let over = (projected - f64::from(DANGER_BASE)).max(0.0);
    value += weights.danger_projected * over * over;
    value += weights.garbage_rows * features.garbage_rows;

    let charge = f64::from(surge_charge_from_streak(context.b2b_streak));
    value += weights.surge_charge * charge.powf(weights.surge_exponent);
    value += weights.combo * f64::from(context.combo.min(COMBO_CAP));
    // Capped at COMBO_CAP. Measured over 406 games (s2-spec-lead's
    // decomposition): attack/piece peaks at combo 3-4 (1.196) and gets *worse*
    // at combo 5+ (1.084) and 10+ (1.110), because holding a long combo means
    // clearing singles, whose base is 0 -- only the ln lift applies. Rewarding
    // combo 5+ therefore pushes the bot to trade TSDs and quads for singles.
    let combo = f64::from(context.combo.min(COMBO_CAP));
    value += weights.combo_state * (1.25 * combo + 1.0).ln();
    // Only worth something when the next clear can stop a garbage packet from
    // appearing: a "one cell short" row is also a single, so pricing it
    // unconditionally invites singles.
    // `19` rather than `21`: at exactly 21 there is no guaranteed turn left to
    // clear, so two rows of slack are needed to build the clear.
    let capped = features.max_height + f64::from(context.arrived.min(GARBAGE_CAP));
    if capped >= f64::from(SURVIVAL_PROJECTED - 2) {
        value += weights.combo_potential * features.combo_potential;
    }
    value -= weights.incoming * f64::from(context.incoming);
    let race = i64::from(context.sent_total) - i64::from(context.received_total);
    value += weights.race * race as f64;
    value += readiness_scale * t_slot_cutout_with_heights(board, &heights, t_available, weights);
    value
}

/// Lines a T could clear in the best slot of `board` (0, 1 or 2).
///
/// This is the cheap shape scan behind [`t_slot_cutout`]: only columns that
/// form a nub-down T slot are considered, so it is a handful of integer
/// comparisons per board.
pub fn t_slot_lines(board: &Board) -> u8 {
    best_t_slot(board, &heights(board)).map_or(0, |(_, _, lines)| lines)
}

/// The T-slot cutout: hypothetically spend up to `available` T pieces on the
/// best slots and credit the lines they would clear.
pub fn t_slot_cutout(board: &Board, available: usize, weights: &Weights) -> f64 {
    t_slot_cutout_with_heights(board, &heights(board), available, weights)
}

/// [`t_slot_cutout`] with the heights already computed.
fn t_slot_cutout_with_heights(
    board: &Board,
    heights: &[i32; COLUMNS],
    available: usize,
    weights: &Weights,
) -> f64 {
    if available == 0 {
        return 0.0;
    }
    // The common case is one T available, and `best_t_slot` already reports the
    // lines that slot would clear, so no board copy is needed at all. Only the
    // two-T case has to place the first one to find the second, and that is
    // rare enough to keep the clone.
    let Some((_, _, first)) = best_t_slot(board, heights) else {
        return 0.0;
    };
    let mut value = weights.t_slot[first as usize];
    if available < 2 || first == 0 {
        return value;
    }
    let mut board = board.clone();
    let Some((x, y, _)) = best_t_slot(&board, heights) else {
        return value;
    };
    board
        .place(PieceKind::T, Rotation::Two, x, y)
        .expect("the slot scan only returns legal placements");
    let cleared = board.clear_lines();
    let after = crate::eval::heights(&board);
    if let Some((_, _, second)) = best_t_slot(&board, &after) {
        value += weights.t_slot[second as usize];
    }
    debug_assert_eq!(cleared.lines, first);
    value
}

/// Exact T-slot scan: the most lines any **reachable** T placement that the
/// engine classifies as a spin would clear.
///
/// The cheap [`best_t_slot`] behind the leaf value only looks at nub-down
/// entries and only checks geometry, so it can miss slots. This version asks
/// the movegen for the placements the bot can actually reach and classifies
/// them with the engine, so it is the reference the heuristic is measured
/// against -- not something to call per leaf (a T movegen is ~10 us).
pub fn t_slot_exact(board: &Board) -> u8 {
    let mut best = 0;
    for placement in crate::movegen::generate(board, PieceKind::T) {
        if placement.spin == SpinType::None {
            continue;
        }
        let mut probe = board.clone();
        probe
            .place(
                placement.piece.kind,
                placement.piece.rotation,
                placement.piece.x,
                placement.piece.y,
            )
            .expect("a generated placement is legal");
        let lines = probe.clear_lines().lines;
        if lines > best {
            best = lines;
        }
    }
    best
}

/// Best nub-down T slot: `(x, y, lines cleared)` for a rotation-2 T.
fn best_t_slot(board: &Board, heights: &[i32; COLUMNS]) -> Option<(i32, i32, u8)> {
    let mut best: Option<(i32, i32, u8)> = None;
    for x in 1..COLUMNS - 1 {
        let stem = heights[x];
        // Cheap filter: the nub column must be at least one row below both
        // shoulders. The exact geometry is verified with `can_place` below,
        // because height alone cannot see an overhang hole.
        if stem + 1 > heights[x - 1] || stem + 1 > heights[x + 1] {
            continue;
        }
        let y = stem;
        let bx = x as i32 - 1;
        if !board.can_place(PieceKind::T, Rotation::Two, bx, y) {
            continue;
        }
        if board.can_place(PieceKind::T, Rotation::Two, bx, y - 1) {
            continue;
        }
        // Count the lines the T would complete without cloning the board: a
        // row clears when the T's cells fill its remaining holes.
        //
        // `best_t_slot` only ever scans `Rotation::Two`, whose cells have `cy`
        // in {0, 1}, so two masks suffice. `Rotation::R` and `Rotation::L`
        // span three rows and would index out of bounds here -- widen the mask
        // array before extending the scan to them.
        let mut rows = [0u16; 2];
        for (cx, cy) in PieceKind::T.cells(Rotation::Two) {
            rows[cy as usize] |= 1 << (bx + cx);
        }
        let mut lines = 0u8;
        for (offset, mask) in rows.iter().enumerate() {
            if *mask == 0 {
                continue;
            }
            let row = y as usize + offset;
            if row < Board::HEIGHT && board.row(row) | *mask == Board::FULL_ROW {
                lines += 1;
            }
        }
        if lines == 0 {
            continue;
        }
        if best.is_none_or(|(_, _, best_lines)| lines > best_lines) {
            best = Some((bx, y, lines));
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use intetrigence_core::Cell;

    fn tsd_board() -> Board {
        let mut board = Board::new();
        board.set_row(0, 0x1FF);
        board.set_row(1, 0x3EF);
        board.set_row(2, 0x3C7);
        board.set_row(3, 0x1CF);
        board
    }

    #[test]
    fn empty_board_has_no_holes_and_no_height() {
        let features = features(&Board::new());
        assert_eq!(features.max_height, 0.0);
        assert_eq!(features.holes, 0.0);
        assert_eq!(features.hole_rows, 0.0);
        assert_eq!(features.bumpiness, 0.0);
        assert_eq!(features.row_transitions, 0.0);
    }

    /// Golden values for the single-pass analysis. They were recorded before
    /// the heights were folded into the same pass, so they pin that the
    /// optimisation did not change a single feature.
    #[test]
    fn analysis_matches_the_recorded_feature_values() {
        let mut board = Board::new();
        for y in 0..19 {
            let mut row = Board::FULL_ROW & !(1u16 << 9);
            if y % 3 == 0 {
                row &= !(1u16 << ((y * 7 + 3) % 9));
            }
            board.set_row(y, row);
        }
        board.set_row(19, 0b0000_0001_1100_0000);
        let analysis = analyse(&board);
        assert_eq!(
            analysis.features,
            Features {
                max_height: 20.0,
                // max_height 20 against DANGER_BASE 15 -> 5^2.
                height_sq: 25.0,
                // Column-wise sum of max(0, h - 12)^2 on the fixture: the
                // tallest column contributes 64 and the rest 409.
                danger_height: 473.0,
                combo_potential: 0.0,
                quad_ready: 0.0,
                holes: 6.0,
                hole_cover: 23.0,
                hole_rows: 6.0,
                bumpiness: 3.0,
                bumpiness_sq: 3.0,
                row_transitions: 76.0,
                well_depth: 4.0,
                extra_wells: 0.0,
                garbage_rows: 0.0,
            }
        );
        assert_eq!(analysis.heights, heights(&board));
    }

    #[test]
    fn holes_are_counted_with_their_cover() {
        let mut board = Board::new();
        board.set_cell(3, 0, Cell::Block);
        board.set_cell(3, 1, Cell::Empty);
        board.set_cell(3, 2, Cell::Block);
        let features = features(&board);
        assert_eq!(features.holes, 1.0);
        assert_eq!(features.hole_rows, 1.0);
        assert_eq!(features.hole_cover, 1.0);
        assert_eq!(features.max_height, 3.0);
    }

    #[test]
    fn well_depth_is_capped_and_main_well_is_leftmost() {
        let mut board = Board::new();
        for x in 0..COLUMNS {
            if x == 0 {
                continue;
            }
            for y in 0..4 {
                board.set_cell(x, y, Cell::Block);
            }
        }
        let features = features(&board);
        assert_eq!(features.well_depth, 4.0);
        assert_eq!(features.extra_wells, 0.0);
    }

    #[test]
    fn tsd_board_has_a_two_line_t_slot() {
        let board = tsd_board();
        assert_eq!(t_slot_lines(&board), 2);
        let weights = Weights::default();
        assert!(t_slot_cutout(&board, 1, &weights) > 0.0);
        assert_eq!(t_slot_cutout(&board, 0, &weights), 0.0);
    }

    /// The cheap nub-down scan is a leaf heuristic; `t_slot_exact` is the
    /// reachability-checked reference. This pins where they disagree so the
    /// gap is visible instead of assumed.
    #[test]
    fn tsd_bonus_and_mini_penalty_hit_only_their_cases() {
        let plain = Weights::default();
        let tuned = Weights {
            tsd_bonus: 40.0,
            mini_penalty: -30.0,
            ..Weights::default()
        };
        // A full TSD: the bonus applies.
        let tsd = tsd_board();
        let tsd_placement = crate::movegen::generate(&tsd, PieceKind::T)
            .into_iter()
            .find(|placement| {
                placement.spin == SpinType::Full && {
                    let mut probe = tsd.clone();
                    probe
                        .place(
                            placement.piece.kind,
                            placement.piece.rotation,
                            placement.piece.x,
                            placement.piece.y,
                        )
                        .unwrap();
                    probe.clear_lines().lines == 2
                }
            })
            .expect("the fixture has a TSD");
        let context = EvalContext::default();
        let (_, base, _) = apply_placement(&tsd, &tsd_placement, &context, &plain);
        let (_, bonus, _) = apply_placement(&tsd, &tsd_placement, &context, &tuned);
        assert_eq!(base.spin, SpinType::Full);
        assert_eq!(base.lines, 2);
        assert!((bonus.value - base.value - 40.0).abs() < 1e-9);

        // A plain quad: neither term applies.
        let flat = Board::new();
        let quad = crate::movegen::generate(&flat, PieceKind::I)
            .into_iter()
            .find(|placement| placement.piece.rotation == Rotation::Zero)
            .expect("an I can lie flat");
        let (_, base, _) = apply_placement(&flat, &quad, &context, &plain);
        let (_, tuned_value, _) = apply_placement(&flat, &quad, &context, &tuned);
        assert_eq!(base.spin, SpinType::None);
        assert!((tuned_value.value - base.value).abs() < 1e-9);
    }

    #[test]
    fn the_cheap_slot_scan_is_measured_against_the_exact_one() {
        // A 2-wide, 2-deep well with an overhang on the left: the nub-down
        // entry is blocked, so only a sideways entry reaches the slot.
        let mut sideways = Board::new();
        sideways.set_row(0, 0x1FF);
        sideways.set_row(1, 0x1FF);
        sideways.set_row(2, 0x1FF & !(0b11 << 3));
        sideways.set_row(3, 0x1FF & !(0b11 << 3));
        sideways.set_row(4, 0x1FF & !(0b1 << 3));

        let mut cases = vec![
            ("empty", Board::new()),
            ("tsd", tsd_board()),
            ("sideways", sideways),
        ];
        let mut one_wide = Board::new();
        for y in 0..4 {
            one_wide.set_row(y, Board::FULL_ROW & !(1u16 << 9));
        }
        cases.push(("one-wide well", one_wide));

        for (name, board) in cases {
            let cheap = t_slot_lines(&board);
            let exact = t_slot_exact(&board);
            println!("{name:14} cheap {cheap} exact {exact}");
            assert!(
                cheap <= exact,
                "{name}: the heuristic must not claim a slot the exact scan cannot reach"
            );
        }
    }

    /// The clone-free line count must agree with placing and clearing.
    ///
    /// `best_t_slot` counts the rows a T would fill with two bitmasks instead
    /// of cloning the board; this pins the two against each other on the
    /// boards where an off-by-one would show: a 1-line slot, a 2-line slot, no
    /// slot, the left and right edges, and a stack near the top of the board.
    #[test]
    fn the_clone_free_slot_scan_matches_placing_the_piece() {
        let mut boards = vec![("tsd", tsd_board())];
        let mut one_line = Board::new();
        one_line.set_row(0, 0x1FF);
        one_line.set_row(1, 0x1FF & !(0b111 << 3));
        one_line.set_row(2, 0x1FF & !(0b010 << 3));
        boards.push(("one-line", one_line));
        boards.push(("empty", Board::new()));
        // A real board never holds a full row: `clear_lines` runs after every
        // lock, so the fixture must leave a hole or the comparison is against
        // rows that were already complete.
        let mut flat = Board::new();
        for row in 0..6 {
            flat.set_row(row, Board::FULL_ROW & !(1 << 9));
        }
        boards.push(("flat stack", flat));
        let mut high = Board::new();
        for row in 0..36 {
            high.set_row(row, Board::FULL_ROW & !0x0F);
        }
        high.set_row(36, 0x1FF);
        high.set_row(37, 0x3EF);
        boards.push(("high", high));

        for (name, board) in boards {
            let heights = heights(&board);
            // Every legal resting nub-down T placement, counted both ways.
            // Only `Rotation::Two` is scanned by `best_t_slot`; its cells have
            // `cy` in {0, 1}, which is what lets the mask count use two rows.
            for x in 0..COLUMNS as i32 {
                for rotation in [Rotation::Two] {
                    // The resting row: the highest y that still sits on the
                    // stack, then confirmed against the board.
                    let Some(y) =
                        PieceKind::T
                            .cells(rotation)
                            .iter()
                            .try_fold(i32::MIN, |acc, (cx, cy)| {
                                let column = x + cx;
                                (column >= 0 && column < COLUMNS as i32)
                                    .then(|| acc.max(heights[column as usize] - cy))
                            })
                    else {
                        continue;
                    };
                    if !board.can_place(PieceKind::T, rotation, x, y) {
                        continue;
                    }
                    let mut probe = board.clone();
                    probe
                        .place(PieceKind::T, rotation, x, y)
                        .expect("checked above");
                    let placed = probe.clear_lines().lines;

                    let mut rows = [0u16; 2];
                    for (cx, cy) in PieceKind::T.cells(rotation) {
                        rows[cy as usize] |= 1 << (x + cx);
                    }
                    let mut counted = 0u8;
                    for (offset, mask) in rows.iter().enumerate() {
                        if *mask == 0 {
                            continue;
                        }
                        let row = y as usize + offset;
                        if row < Board::HEIGHT && board.row(row) | *mask == Board::FULL_ROW {
                            counted += 1;
                        }
                    }
                    assert_eq!(
                        counted, placed,
                        "{name} x{x} {rotation:?} y{y}: mask count {counted} vs placed {placed}"
                    );
                }
            }
        }
    }

    /// The slot scan only looks at nub-down T placements.
    ///
    /// The clone-free line count uses two row masks, which is only valid for
    /// `Rotation::Two` (`cy` in {0, 1}). R and L span three rows, so extending
    /// the scan to them without widening the masks would panic.
    #[test]
    fn the_slot_scan_only_considers_nub_down_t() {
        let boards = [("tsd", tsd_board()), ("empty", Board::new())];
        for (name, board) in boards {
            let columns = heights(&board);
            let Some((x, y, _)) = best_t_slot(&board, &columns) else {
                continue;
            };
            // The scan places its probe at rotation two; if that ever stops
            // being legal the scan and the two-row masks have diverged.
            let mut probe = board.clone();
            probe
                .place(PieceKind::T, Rotation::Two, x, y)
                .unwrap_or_else(|_| panic!("{name}: the slot must be a rotation-2 placement"));
        }
    }

    #[test]
    fn flat_board_has_no_t_slot() {
        let mut board = Board::new();
        for y in 0..4 {
            board.set_row(y, Board::FULL_ROW & !(1u16 << 9));
        }
        assert_eq!(t_slot_lines(&board), 0);
    }

    #[test]
    fn reward_uses_the_s2_attack_table() {
        let board = tsd_board();
        let weights = Weights::default();
        let mut movegen = crate::movegen::Movegen::new();
        let placements = movegen.generate(&board, PieceKind::T);
        let tsd = placements
            .iter()
            .find(|placement| placement.key() == (Rotation::Two, 3, 1))
            .expect("the TSD slot is reachable");
        let context = EvalContext::default();
        let (next, reward, next_context) = apply_placement(&board, tsd, &context, &weights);
        assert_eq!(reward.lines, 2);
        assert_eq!(reward.spin, SpinType::Full);
        assert_eq!(reward.attack, 4);
        assert_eq!(reward.b2b_after, 1);
        assert_eq!(next_context.combo, 1);
        assert_eq!(next_context.pieces_placed, 1);
        // Two rows were removed, so the stack is two rows shorter.
        assert_eq!(next.height() + 2, board.height());
    }

    /// Pending garbage must make a tall stack more expensive: the projected
    /// danger is the versus term the M2 losses needed.
    #[test]
    fn pending_garbage_raises_the_danger_of_a_tall_stack() {
        let mut board = Board::new();
        for y in 0..10 {
            board.set_row(y, Board::FULL_ROW & !(1u16 << 9));
        }
        let weights = Weights::default();
        let calm = EvalContext::default();
        let pressured = EvalContext {
            incoming: 14,
            arrived: 8,
            ..EvalContext::default()
        };
        let calm_value = leaf_value(&board, &calm, &weights, 0);
        let pressured_value = leaf_value(&board, &pressured, &weights, 0);
        assert!(
            pressured_value < calm_value - 100.0,
            "14 pending rows must dominate the leaf value: {calm_value} vs {pressured_value}"
        );
    }

    /// Uncleared garbage rows are penalised and clearing them is rewarded.
    #[test]
    fn garbage_rows_and_garbage_clears_are_priced() {
        let mut board = Board::new();
        board.set_row(0, Board::FULL_ROW & !(1u16 << 4));
        board.set_row(1, Board::FULL_ROW & !(1u16 << 4));
        // Mark the two rows as garbage.
        for y in 0..2 {
            let mask = board.row(y);
            board.set_garbage_row(y, mask);
        }
        let analysis = analyse(&board);
        assert_eq!(analysis.features.garbage_rows, 2.0);

        let weights = Weights::default();
        let context = EvalContext::default();
        let clean = Board::new();
        assert!(
            leaf_value(&board, &context, &weights, 0) < leaf_value(&clean, &context, &weights, 0)
        );

        // Clearing a garbage row pays the garbage-clear reward.
        let mut movegen = crate::movegen::Movegen::new();
        let placements = movegen.generate(&board, PieceKind::I);
        let clear = placements
            .iter()
            .find(|placement| placement.piece.rotation == Rotation::R && placement.piece.x == 2)
            .expect("the vertical I fills the hole");
        let (_, reward, _) = apply_placement(&board, clear, &context, &weights);
        assert_eq!(reward.lines, 2);
        assert_eq!(reward.garbage_cleared, 2);
        assert!(reward.value >= weights.garbage_clear * 2.0);
    }

    #[test]
    fn opener_phase_makes_attack_worth_more() {
        let board = tsd_board();
        let weights = Weights::default();
        let mut movegen = crate::movegen::Movegen::new();
        let placements = movegen.generate(&board, PieceKind::T);
        let tsd = placements
            .iter()
            .find(|placement| placement.key() == (Rotation::Two, 3, 1))
            .expect("the TSD slot is reachable");

        let opener = EvalContext {
            pieces_placed: 0,
            ..EvalContext::default()
        };
        let late = EvalContext {
            pieces_placed: 20,
            ..EvalContext::default()
        };
        assert!(opener.opener());
        assert!(!late.opener());
        let (_, opener_reward, _) = apply_placement(&board, tsd, &opener, &weights);
        let (_, late_reward, _) = apply_placement(&board, tsd, &late, &weights);
        assert_eq!(opener_reward.attack, late_reward.attack);
        let expected =
            weights.opener_attack * f64::from(opener_reward.attack) + weights.opener_clear;
        assert!((opener_reward.value - late_reward.value - expected).abs() < 1e-9);
    }

    #[test]
    fn b2b_break_and_surge_charge_are_rewarded() {
        let mut board = Board::new();
        // A single line ready to clear with an I piece. Row 1 keeps the clear
        // from being a perfect clear, which would not break B2B.
        board.set_row(0, Board::FULL_ROW & !0x0F);
        board.set_row(1, 0x200);
        let weights = Weights::default();
        let mut movegen = crate::movegen::Movegen::new();
        let placements = movegen.generate(&board, PieceKind::I);
        let clear = placements
            .iter()
            .find(|placement| placement.piece.rotation == Rotation::Zero && placement.piece.x == 0)
            .expect("the I can finish the row");
        let context = EvalContext {
            b2b_streak: 6,
            combo: 0,
            pieces_placed: 20,
            incoming: 0,
            arrived: 0,
            sent_total: 0,
            received_total: 0,
            t_slot: 0,
        };
        let (_, reward, _) = apply_placement(&board, clear, &context, &weights);
        assert_eq!(reward.lines, 1);
        assert!(reward.broke_b2b);
        // Surge charge from a 6-streak is 5 lines, released in three chunks.
        assert_eq!(reward.surge_released, 5);
        assert_eq!(reward.b2b_after, 0);
    }

    /// A single line ready to clear with an I piece, and not a perfect clear.
    fn single_ready() -> Board {
        let mut board = Board::new();
        board.set_row(0, Board::FULL_ROW & !0x0F);
        board.set_row(1, 0x200);
        board
    }

    fn single_placement(board: &Board) -> crate::movegen::Placement {
        crate::movegen::generate(board, PieceKind::I)
            .into_iter()
            .find(|placement| placement.piece.rotation == Rotation::Zero && placement.piece.x == 0)
            .expect("the I can finish the row")
    }

    #[test]
    fn surge_exponent_makes_a_large_charge_worth_more() {
        let board = Board::new();
        let linear = Weights::default();
        let convex = Weights {
            surge_exponent: 1.5,
            ..Weights::default()
        };
        let value = |weights: &Weights, b2b: u32| {
            leaf_value(
                &board,
                &EvalContext {
                    b2b_streak: b2b,
                    ..EvalContext::default()
                },
                weights,
                1,
            )
        };
        // charge 4 at b2b 5, charge 10 at b2b 11.
        let small_gain = value(&convex, 5) - value(&linear, 5);
        let big_gain = value(&convex, 11) - value(&linear, 11);
        assert!(
            small_gain > 0.0,
            "a convex charge is worth more than a linear one"
        );
        assert!(
            big_gain > 2.5 * small_gain,
            "the surplus has to grow with the charge: {big_gain} vs {small_gain}"
        );
    }

    #[test]
    fn surge_hold_penalty_only_bites_below_lethal() {
        let board = single_ready();
        let placement = single_placement(&board);
        let plain = Weights::default();
        let holding = Weights {
            surge_hold_penalty: -60.0,
            ..Weights::default()
        };
        let reward = |weights: &Weights, b2b: u32| {
            apply_placement(
                &board,
                &placement,
                &EvalContext {
                    b2b_streak: b2b,
                    ..EvalContext::default()
                },
                weights,
            )
            .1
        };
        let small = reward(&holding, 5);
        let small_plain = reward(&plain, 5);
        assert_eq!(small.surge_released, 4);
        assert!(
            (small.value - small_plain.value - holding.surge_hold_penalty).abs() < 1e-9,
            "the penalty applies to a 4-line release"
        );
        let large = reward(&holding, 11);
        let large_plain = reward(&plain, 11);
        assert_eq!(large.surge_released, 10);
        assert!(
            (large.value - large_plain.value).abs() < 1e-9,
            "a 10-line release is lethal and pays no penalty"
        );
    }
}
