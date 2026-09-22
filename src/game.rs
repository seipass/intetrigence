use intetrigence_engine::data::{remove_lines_mask, Board, Piece, Placement, Spin};
use rand::{seq::SliceRandom, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub const PIECES: [Piece; 7] = [
    Piece::I,
    Piece::O,
    Piece::T,
    Piece::L,
    Piece::J,
    Piece::S,
    Piece::Z,
];

/// Public opponent information used by evaluators that can see both boards
/// (the local arena).  Network adapters leave this as `None`, matching the
/// original bot boundary where the opponent's private board is unavailable.
#[derive(Clone, Copy, Debug, Default)]
pub struct OpponentState {
    pub board: Board,
    pub active: Option<Piece>,
    pub reserve: Option<Piece>,
    pub hold: Option<Piece>,
    pub queue: [Option<Piece>; 6],
    pub combo: u8,
    pub b2b: bool,
    pub pending: usize,
    pub multi_evaluation: bool,
}

#[derive(Clone)]
pub struct Game {
    pub board: Board,
    pub hold: Option<Piece>,
    pub queue: VecDeque<Piece>,
    pub combo: u8,
    pub b2b: bool,
    pub pending: VecDeque<u8>,
    pub dead: bool,
    pub pieces: u32,
    pub attack: u32,
    pub lines: u32,
    pub score: u64,
    pub bag: Vec<Piece>,
    /// Whether unseen queue pieces may be sampled from the seven-bag state.
    /// The TBP `unknown` randomizer disables speculative search.
    pub speculate: bool,
    pub rules: RulesProfile,
    pub opponent: Option<OpponentState>,
    /// Guideline garbage rows are queued as 255 until they appear. The hole
    /// state counts only rows that were actually inserted into the matrix.
    pub(crate) garbage_hole: u8,
    pub(crate) garbage_rows_in_block: u8,
    garbage_hole_initialized: bool,
    /// Occupancy mask for cells inserted as incoming garbage. This is kept
    /// separately from `Board` so replay viewers can distinguish garbage from
    /// placed blocks without changing the search engine's board format.
    garbage_cols: [u64; 10],
    rng: ChaCha8Rng,
    garbage_rng: ChaCha8Rng,
}

/// Versus rules used by the referee. `Guideline` follows the public 2009
/// multiplayer table: line attacks are the listed line-clear/T-Spin values,
/// incoming lines are applied after every lock, and there is no combo attack
/// bonus. `Arena` is the historical development profile retained for old
/// replays and benchmark manifests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RulesProfile {
    #[default]
    Arena,
    Guideline,
}

impl RulesProfile {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Arena => "arena",
            Self::Guideline => "guideline",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "arena" => Some(Self::Arena),
            "guideline" => Some(Self::Guideline),
            _ => None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    pub lines: u32,
    pub attack: u32,
    pub sent: u32,
    pub cancelled: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub garbage_applied: u32,
    pub perfect_clear: bool,
    pub dead: bool,
}

const fn is_zero(value: &u32) -> bool {
    *value == 0
}
impl Game {
    pub fn new(seed: u64) -> Self {
        Self::new_with_rules(seed, RulesProfile::Arena)
    }

    pub fn new_guideline(seed: u64) -> Self {
        Self::new_with_rules(seed, RulesProfile::Guideline)
    }

    pub fn new_with_rules(seed: u64, rules: RulesProfile) -> Self {
        let mut s = Self {
            board: Board::default(),
            hold: None,
            queue: VecDeque::new(),
            combo: 0,
            b2b: false,
            pending: VecDeque::new(),
            dead: false,
            pieces: 0,
            attack: 0,
            lines: 0,
            score: 0,
            bag: Vec::new(),
            speculate: true,
            rules,
            opponent: None,
            rng: ChaCha8Rng::seed_from_u64(seed),
            garbage_hole: 0,
            garbage_rows_in_block: 0,
            garbage_hole_initialized: false,
            garbage_cols: [0; 10],
            garbage_rng: ChaCha8Rng::seed_from_u64(seed ^ 0x6a09e667f3bcc909),
        };
        s.refill();
        s
    }

    pub fn opponent_snapshot(&self) -> OpponentState {
        OpponentState {
            board: self.board,
            active: self.queue.front().copied(),
            reserve: self.hold.or_else(|| self.queue.get(1).copied()),
            hold: self.hold,
            queue: std::array::from_fn(|index| self.queue.get(index).copied()),
            combo: self.combo,
            b2b: self.b2b,
            pending: self.pending.len(),
            multi_evaluation: true,
        }
    }

    pub fn set_opponent_snapshot(&mut self, opponent: OpponentState) {
        self.opponent = Some(opponent);
    }

    pub fn clear_opponent_snapshot(&mut self) {
        self.opponent = None;
    }

    /// Occupancy that came from incoming garbage.  The Hoiko evaluator uses
    /// this marker to separate the underground section from the playable
    /// stack; the ordinary board remains a compact occupancy-only bitboard.
    pub fn garbage_board(&self) -> Board {
        Board {
            cols: self.garbage_cols,
        }
    }
    pub fn refill(&mut self) {
        while self.queue.len() < 6 {
            if self.bag.is_empty() {
                self.bag = PIECES.to_vec();
                self.bag.shuffle(&mut self.rng);
            }
            self.queue.push_back(self.bag.pop().unwrap());
        }
    }
    pub fn legal(&self, mv: Placement) -> bool {
        if self.dead || self.queue.is_empty() {
            return false;
        }
        let current = self.queue[0];
        let held = self.hold.or_else(|| self.queue.get(1).copied());
        (mv.location.piece == current || Some(mv.location.piece) == held)
            && intetrigence_engine::movegen::find_moves(&self.board, mv.location.piece)
                .iter()
                .any(|(m, _)| *m == mv)
    }
    pub fn play(&mut self, mv: Placement) -> Result<Outcome, String> {
        if !self.legal(mv) {
            return Err(format!("illegal placement: {mv:?}"));
        }
        let current = self.queue.pop_front().unwrap();
        if mv.location.piece != current {
            match self.hold.replace(current) {
                Some(_) => {}
                None => {
                    self.queue
                        .pop_front()
                        .ok_or("empty hold needs next piece")?;
                }
            }
        }
        let lock_out = mv.location.cells().iter().all(|&(_, y)| y >= 20);
        self.board.place(mv.location);
        let mask = self.board.line_clears();
        let lines = mask.count_ones();
        self.board.remove_lines(mask);
        for column in &mut self.garbage_cols {
            *column = remove_lines_mask(*column, mask);
        }
        let perfect_clear = lines > 0 && self.board.cols.iter().all(|&c| c == 0);
        let difficult = lines > 0 && (lines == 4 || mv.spin != Spin::None);
        let prior_b2b = self.b2b;
        self.combo = if lines > 0 {
            self.combo.saturating_add(1)
        } else {
            0
        };
        if lines > 0 {
            self.b2b = difficult;
        }
        let attack = attack_with_rules(
            lines,
            mv.spin,
            difficult && prior_b2b,
            self.combo,
            perfect_clear,
            self.rules,
        );
        let base_score = match mv.spin {
            Spin::None => [0, 100, 300, 500, 800][lines as usize],
            Spin::Mini => [100, 200, 400][lines as usize],
            Spin::Full => [400, 800, 1200, 1600][lines as usize],
        };
        self.score += if difficult && prior_b2b {
            base_score * 3 / 2
        } else {
            base_score
        };
        if lines > 0 {
            self.score += 50 * self.combo.saturating_sub(1) as u64;
        }
        self.attack += attack;
        self.lines += lines;
        self.pieces += 1;
        let cancelled = attack.min(self.pending.len() as u32);
        self.pending.drain(..cancelled as usize);
        let sent = attack - cancelled;
        self.dead |= lock_out;
        let garbage_limit = match self.rules {
            RulesProfile::Arena => usize::from(lines == 0) * 8,
            RulesProfile::Guideline => self.pending.len(),
        };
        let mut garbage_applied = 0;
        for _ in 0..garbage_limit {
            let Some(queued_hole) = self.pending.pop_front() else {
                break;
            };
            let hole = match self.rules {
                RulesProfile::Arena => queued_hole,
                RulesProfile::Guideline => {
                    if self.garbage_rows_in_block == 0 {
                        let mut hole = self.garbage_rng.gen_range(0..10);
                        if self.garbage_hole_initialized && hole == self.garbage_hole {
                            // The public multiplayer rule changes the gap
                            // location at each eight-row boundary.
                            hole = (hole + 1 + self.garbage_rng.gen_range(0..9)) % 10;
                        }
                        self.garbage_hole = hole;
                        self.garbage_hole_initialized = true;
                    }
                    let hole = self.garbage_hole;
                    self.garbage_rows_in_block += 1;
                    if self.garbage_rows_in_block == 8 {
                        self.garbage_rows_in_block = 0;
                    }
                    hole
                }
            };
            for x in 0..10 {
                if self.board.cols[x] >> 39 != 0 {
                    self.dead = true;
                }
                self.board.cols[x] = ((self.board.cols[x] << 1) | u64::from(x != hole as usize))
                    & ((1u64 << 40) - 1);
                self.garbage_cols[x] = ((self.garbage_cols[x] << 1)
                    | u64::from(x != hole as usize))
                    & ((1u64 << 40) - 1);
            }
            garbage_applied += 1;
        }
        self.refill();
        Ok(Outcome {
            lines,
            attack,
            sent,
            cancelled,
            garbage_applied,
            perfect_clear,
            dead: self.dead,
        })
    }
    pub fn receive(&mut self, count: u32, rng: &mut impl Rng) {
        if matches!(self.rules, RulesProfile::Guideline) {
            // 255 marks a queued line whose hole is selected only when it is
            // actually inserted. This keeps cancelled queue entries from
            // consuming one of the public eight-line hole blocks.
            self.pending
                .extend(std::iter::repeat_n(255, count as usize));
            return;
        }
        let mut hole = rng.gen_range(0..10);
        for _ in 0..count {
            if rng.gen_bool(0.3) {
                hole = rng.gen_range(0..10);
            }
            self.pending.push_back(hole);
        }
    }
    pub fn board_json(&self) -> serde_json::Value {
        serde_json::json!((0..40)
            .map(|y| (0..10)
                .map(|x| if self.board.cols[x] & (1 << y) != 0 {
                    Some('G')
                } else {
                    None
                })
                .collect::<Vec<_>>())
            .collect::<Vec<_>>())
    }

    /// Return a viewer-oriented 40x10 board. `block` is a placed tetromino
    /// cell, `garbage` is an incoming garbage cell, and null is empty.
    pub fn board_cells_json(&self) -> serde_json::Value {
        serde_json::json!((0..40)
            .map(|y| (0..10)
                .map(|x| {
                    if self.garbage_cols[x] & (1 << y) != 0 {
                        serde_json::json!("garbage")
                    } else if self.board.cols[x] & (1 << y) != 0 {
                        serde_json::json!("block")
                    } else {
                        serde_json::Value::Null
                    }
                })
                .collect::<Vec<_>>())
            .collect::<Vec<_>>())
    }
}
/// Explicit versus profile; attack tables are game-specific, not a universal Guideline mandate.
pub fn attack(lines: u32, spin: Spin, b2b: bool, combo: u8, pc: bool) -> u32 {
    attack_with_rules(lines, spin, b2b, combo, pc, RulesProfile::Arena)
}

pub fn guideline_attack(lines: u32, spin: Spin, b2b: bool, pc: bool) -> u32 {
    attack_with_rules(lines, spin, b2b, 0, pc, RulesProfile::Guideline)
}

fn attack_with_rules(
    lines: u32,
    spin: Spin,
    b2b: bool,
    combo: u8,
    pc: bool,
    rules: RulesProfile,
) -> u32 {
    if lines == 0 {
        return 0;
    }
    let base = match spin {
        Spin::None => [0, 0, 1, 2, 4][lines as usize],
        Spin::Mini => [0, 0, 1][lines as usize],
        Spin::Full => [0, 2, 4, 6][lines as usize],
    };
    let combo_table = match rules {
        RulesProfile::Arena => [0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 4, 5],
        RulesProfile::Guideline => [0; 12],
    };
    if pc && matches!(rules, RulesProfile::Arena) {
        10
    } else {
        base + u32::from(b2b) + combo_table[(combo.saturating_sub(1) as usize).min(11)]
    }
}
