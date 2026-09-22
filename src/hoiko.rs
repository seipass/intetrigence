//! A Rust reconstruction of Hoiko's evaluator and beam search.
//!
//! The Windows release calls the weight files `*.dll`, but they are UTF-8
//! comma-separated text files.  The release's metadata exposes the four
//! profiles (`we`, `wo`, `wd`, `wr`), a 14-ply search limit, and a default
//! beam of 50.  This module keeps those data formats and search boundaries,
//! while using the local Rust SRS move generator and an explicit empty-HOLD
//! state.  It therefore does not depend on the battle.tet protocol or on a
//! particular versus ruleset.

use crate::{
    game::{Game, OpponentState},
    search::Budget,
};
use enumset::EnumSet;
use intetrigence_engine::{
    data::{Board, Piece, PieceLocation, Placement, Rotation, Spin},
    movegen::find_moves,
};
use serde::{Deserialize, Serialize};
use std::{
    cell::Cell,
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const WIDTH: usize = 10;
const MAX_DEPTH: usize = 14;
const NATIVE_BOARD_MASK: u64 = u32::MAX as u64;

// `NextMino::Bag::Pick` in the public Hoiko source uses this fixed order for
// the hidden part of a seven-bag.  Visible queue entries still take
// precedence; this order is only used when the search has to speculate past
// the advertised NEXT window.
const HOIKO_BAG_ORDER: [Piece; 7] = [
    Piece::Z,
    Piece::S,
    Piece::T,
    Piece::O,
    Piece::J,
    Piece::L,
    Piece::I,
];

#[derive(Clone, Copy, Debug)]
#[repr(usize)]
#[allow(dead_code)]
enum W {
    MaxHeight,
    Over10,
    Over15,
    UpDown2,
    DonateCover,
    Cover,
    Roof,
    BadRoof,
    Pierce,
    Anabara,
    WellHint,
    WellDepth,
    WellPeak,
    Column,
    PcStack,
    PcChance,
    Resource,
    ResourceMax,
    TsdHole,
    TsdSpinable,
    TsdClearable,
    TsdOffensive,
    TstHole,
    TstSpinable,
    TstClearable,
    TstHint,
    TstOffensive,
    TdHole,
    TdHint,
    MoveDelay,
    HoldI,
    HoldT,
    WasteI,
    WasteT,
    Single,
    Double,
    Triple,
    Quad,
    Tss,
    Tsd,
    Tst,
    Btb,
    Pc,
    ComboBtb,
    ComboAtk,
    ComboDebt,
    Nexus,
}

impl W {
    const COUNT: usize = 50;
}

#[derive(Clone, Debug, PartialEq)]
pub struct HoikoWeights {
    values: [i32; W::COUNT],
    column: [i32; WIDTH],
    combo_table: [i32; 5],
    nexus: f64,
}

impl HoikoWeights {
    /// Parse the exact row-oriented format used by `EvaluateAI::ReadWeight`.
    /// Unknown rows are ignored so a newer Hoiko weight file remains usable.
    pub fn parse_csv(text: &str) -> Result<Self, String> {
        let mut out = Self {
            values: [0; W::COUNT],
            column: [0; WIDTH],
            combo_table: [0; 5],
            nexus: 0.0,
        };
        for (line_no, raw) in text.lines().enumerate() {
            let mut fields = raw.split(',');
            let Some(name) = fields.next() else { continue };
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            let mut numbers = Vec::new();
            for field in fields {
                let field = field.trim();
                if field.is_empty() {
                    continue;
                }
                numbers.push(field.parse::<i32>().map_err(|e| {
                    format!(
                        "weight row {} ({name}): invalid integer {field:?}: {e}",
                        line_no + 1
                    )
                })?);
            }
            if name == "column" {
                for (dst, value) in out.column.iter_mut().zip(numbers.iter().copied()) {
                    *dst = value;
                }
                continue;
            }
            if name == "comboAtk" || name == "comboTable" {
                for (dst, value) in out.combo_table.iter_mut().zip(numbers.iter().copied()) {
                    *dst = value;
                }
                if numbers.len() == 1 {
                    for i in 1..5 {
                        out.combo_table[i] = out.combo_table[0] * (i as i32 + 1);
                    }
                }
                continue;
            }
            let Some(index) = weight_name(name) else {
                continue;
            };
            if let Some(value) = numbers.first() {
                out.values[index] = *value;
                if index == W::Nexus as usize {
                    out.nexus = (*value).clamp(0, 100) as f64 * 0.01;
                }
            }
        }
        Ok(out)
    }

    pub fn from_file(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)?;
        Self::parse_csv(&text)
            .map_err(|message| io::Error::new(io::ErrorKind::InvalidData, message))
    }

    fn get(&self, weight: W) -> f64 {
        self.values[weight as usize] as f64
    }
}

impl Default for HoikoWeights {
    fn default() -> Self {
        Self::parse_csv(include_str!("../assets/hoiko/we.dll")).expect("embedded Hoiko weights")
    }
}

fn weight_name(name: &str) -> Option<usize> {
    Some(match name {
        "maxHeight" => W::MaxHeight,
        "over10" => W::Over10,
        "over15" => W::Over15,
        "updown2" => W::UpDown2,
        "donateCover" => W::DonateCover,
        "cover" => W::Cover,
        "roof" => W::Roof,
        "badRoof" => W::BadRoof,
        "pierce" => W::Pierce,
        "anabara" => W::Anabara,
        "wellHole" | "wellHint" => W::WellHint,
        "wellDepth" => W::WellDepth,
        "wellPeak" => W::WellPeak,
        "pcStack" => W::PcStack,
        "pcChance" => W::PcChance,
        "resource" => W::Resource,
        "resourceMax" => W::ResourceMax,
        "tsdHole" => W::TsdHole,
        "tsdSpinable" => W::TsdSpinable,
        "tsdClearable" => W::TsdClearable,
        "tsdOffensive" => W::TsdOffensive,
        "tstHole" => W::TstHole,
        "tstSpinable" => W::TstSpinable,
        "tstClearable" => W::TstClearable,
        "tstHint" => W::TstHint,
        "tstOffensive" => W::TstOffensive,
        "tdHole" => W::TdHole,
        "tdHint" => W::TdHint,
        "moveDelay" => W::MoveDelay,
        "holdI" => W::HoldI,
        "holdT" => W::HoldT,
        "wasteI" => W::WasteI,
        "wasteT" => W::WasteT,
        "single" => W::Single,
        "double" => W::Double,
        "triple" => W::Triple,
        "quad" => W::Quad,
        "tss" => W::Tss,
        "tsd" => W::Tsd,
        "tst" => W::Tst,
        "btb" => W::Btb,
        "pc" => W::Pc,
        "comboBtb" => W::ComboBtb,
        "comboDebt" => W::ComboDebt,
        "nexus" => W::Nexus,
        _ => return None,
    } as usize)
}

#[derive(Clone, Debug)]
pub struct HoikoConfig {
    pub beam_size: usize,
    pub min_depth: usize,
    pub max_depth: usize,
    pub view_next: usize,
    pub thread_count: usize,
    pub auto_select: bool,
    pub play_style: usize,
    pub use_hold: bool,
    pub use_s4w: bool,
    pub use_pcstack: bool,
    pub use_td_openers: bool,
    pub use_offset_off: bool,
    pub loop_template: bool,
    pub profiles: [HoikoWeights; 4],
    pub solo_profiles: [HoikoWeights; 2],
}

impl Default for HoikoConfig {
    fn default() -> Self {
        Self {
            beam_size: 50,
            min_depth: 10,
            max_depth: MAX_DEPTH,
            // The binary clamps the config value 2 to the minimum 3.
            view_next: 3,
            thread_count: 1,
            auto_select: true,
            play_style: 0,
            use_hold: true,
            use_s4w: false,
            use_pcstack: false,
            use_td_openers: false,
            use_offset_off: false,
            loop_template: false,
            profiles: [
                HoikoWeights::default(),
                HoikoWeights::parse_csv(include_str!("../assets/hoiko/wo.dll")).unwrap(),
                HoikoWeights::parse_csv(include_str!("../assets/hoiko/wd.dll")).unwrap(),
                HoikoWeights::parse_csv(include_str!("../assets/hoiko/wr.dll")).unwrap(),
            ],
            solo_profiles: [
                HoikoWeights::parse_csv(include_str!("../assets/hoiko/ultra1.dll")).unwrap(),
                HoikoWeights::parse_csv(include_str!("../assets/hoiko/sprint.dll")).unwrap(),
            ],
        }
    }
}

impl HoikoConfig {
    /// Load a directory extracted from `Hoiko_PPT_v0-beta1.zip`.
    ///
    /// The files deliberately retain their original `.dll` names.  This
    /// loader is independent of the Windows executable and is also useful
    /// for testing modified weights.
    pub fn from_dir(dir: impl AsRef<Path>) -> io::Result<Self> {
        let dir = dir.as_ref();
        let mut config = Self::default();
        let config_text = fs::read_to_string(dir.join("config.dll"))?;
        for line in config_text.lines() {
            let Some((key, value)) = line.split_once(',') else {
                continue;
            };
            match key.trim() {
                "viewNext" => {
                    config.view_next = value.trim().parse::<usize>().unwrap_or(3).clamp(3, 12)
                }
                "useHold" => config.use_hold = value.trim().eq_ignore_ascii_case("true"),
                "beamSize" => {
                    config.beam_size = value.trim().parse::<usize>().unwrap_or(50).clamp(50, 500)
                }
                "minDepth" => {
                    config.min_depth = value
                        .trim()
                        .parse::<usize>()
                        .unwrap_or(10)
                        .clamp(1, MAX_DEPTH)
                }
                "threadCount" => {
                    config.thread_count = value.trim().parse::<usize>().unwrap_or(1).clamp(1, 64)
                }
                "autoSelect" => config.auto_select = value.trim().eq_ignore_ascii_case("true"),
                "playStyle" => {
                    config.play_style = value.trim().parse::<usize>().unwrap_or(0).min(2)
                }
                "useS4W" => config.use_s4w = value.trim().eq_ignore_ascii_case("true"),
                "usePCstack" => config.use_pcstack = value.trim().eq_ignore_ascii_case("true"),
                "useTDOpeners" => config.use_td_openers = value.trim().eq_ignore_ascii_case("true"),
                "useOffsetOff" => config.use_offset_off = value.trim().eq_ignore_ascii_case("true"),
                "loopTemplate" => config.loop_template = value.trim().eq_ignore_ascii_case("true"),
                _ => {}
            }
        }
        config.profiles = [
            HoikoWeights::from_file(dir.join("we.dll"))?,
            HoikoWeights::from_file(dir.join("wo.dll"))?,
            HoikoWeights::from_file(dir.join("wd.dll"))?,
            HoikoWeights::from_file(dir.join("wr.dll"))?,
        ];
        config.solo_profiles = [
            HoikoWeights::from_file(dir.join("ultra1.dll"))?,
            HoikoWeights::from_file(dir.join("sprint.dll"))?,
        ];
        config.max_depth = MAX_DEPTH;
        Ok(config)
    }

    pub fn load_or_default(dir: impl AsRef<Path>) -> Self {
        Self::from_dir(dir).unwrap_or_default()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HoikoStats {
    pub nodes: u64,
    pub depth: usize,
    pub root_candidates: usize,
    pub elapsed_us: u128,
    pub profile: usize,
}

#[derive(Clone)]
struct BeamNode {
    board: Board,
    garbage: Board,
    opponent: Option<OpponentState>,
    hold: Option<Piece>,
    active: Option<Piece>,
    future: Vec<Piece>,
    bag: EnumSet<Piece>,
    combo: i32,
    b2b: i32,
    /// Hoiko selects a weight profile once for the root search.  The native
    /// evaluator keeps that pointer while expanding every descendant; it is
    /// not recomputed for each speculative board.
    profile: usize,
    score: f64,
    family_score: f64,
    pc_score: f64,
    first: Option<Placement>,
    first_commands: Option<CommandPath>,
}

#[derive(Clone, Copy, Debug, Default)]
struct Evaluation {
    board_score: f64,
    action_score: f64,
    pc_score: f64,
}

/// Abstract input sequence used by Hoiko while expanding a placement.  It is
/// kept even when the caller only needs the final placement because the
/// evaluator scores the operation time and the native result applies a
/// second, frame-based correction to the chosen first move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HoikoCommand {
    Left,
    Right,
    RotateCw,
    RotateCcw,
    SoftDrop(u8),
    Hold,
    FirstHold,
    HardDrop,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct CommandPath {
    commands: Vec<HoikoCommand>,
    raw_delay: u32,
}

impl CommandPath {
    fn pushed(&self, command: HoikoCommand, delay: u32) -> Self {
        let mut result = self.clone();
        result.commands.push(command);
        result.raw_delay += delay;
        result
    }

    /// Port of `Command::CorrectDelay`. This value is used for planning the
    /// next search window; evaluation itself uses `raw_delay` like Hoiko.
    fn corrected_delay(&self) -> u32 {
        const FRAME: f64 = 1000.0 / 60.0;
        let mut frames = 0.0;
        let mut previous = None;
        for command in &self.commands {
            match *command {
                HoikoCommand::Left | HoikoCommand::Right => {
                    frames += 1.0;
                    if matches!(previous, Some(HoikoCommand::Left | HoikoCommand::Right)) {
                        frames += 1.0;
                    }
                }
                HoikoCommand::RotateCw | HoikoCommand::RotateCcw => {
                    frames += 1.0;
                    if matches!(
                        previous,
                        Some(HoikoCommand::RotateCw | HoikoCommand::RotateCcw)
                    ) {
                        frames += 1.0;
                    }
                }
                HoikoCommand::SoftDrop(cells) => frames += 2.0 * f64::from(cells),
                HoikoCommand::Hold => frames += 1.0,
                HoikoCommand::FirstHold => frames += 8.0,
                HoikoCommand::HardDrop => {
                    frames += if matches!(
                        previous,
                        Some(HoikoCommand::RotateCw | HoikoCommand::RotateCcw)
                    ) {
                        6.0
                    } else {
                        7.0
                    };
                    break;
                }
            }
            previous = Some(*command);
        }
        (frames * FRAME) as u32
    }
}

#[derive(Clone, Debug)]
struct MoveCandidate {
    placement: Placement,
    commands: CommandPath,
    order: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TemplateState {
    drops: i16,
    opener: u8,
    failed: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct OffsetState {
    enabled: bool,
    center_height: u8,
    planned_attack: u8,
    opponent_attack: u8,
}

#[derive(Clone)]
struct LockResult {
    board: Board,
    garbage: Board,
    lines: u32,
    combo: i32,
    b2b: i32,
    back_to_back: bool,
    perfect_clear: bool,
}

fn trust_rate(view_next: usize, depth: usize) -> f64 {
    if depth > view_next + 2 {
        0.9
    } else {
        1.0
    }
}

pub struct HoikoSearcher {
    pub config: HoikoConfig,
    profile: Cell<usize>,
    template: Cell<TemplateState>,
    offset: Cell<OffsetState>,
    beam_limit: Cell<usize>,
}

impl Default for HoikoSearcher {
    fn default() -> Self {
        Self::new(HoikoConfig::default())
    }
}

impl HoikoSearcher {
    pub fn new(config: HoikoConfig) -> Self {
        let beam_limit = corrected_beam_size(&config);
        Self {
            config,
            profile: Cell::new(0),
            template: Cell::new(TemplateState::default()),
            offset: Cell::new(OffsetState::default()),
            beam_limit: Cell::new(beam_limit),
        }
    }

    pub fn choose(&self, game: &Game, budget: Budget) -> (Option<Placement>, HoikoStats) {
        let started = Instant::now();
        let mut stats = HoikoStats::default();
        let Some(&active) = game.queue.front() else {
            return (None, stats);
        };
        if game.dead {
            return (None, stats);
        }

        self.prepare_tactical_state(game);

        let future = predict_future(
            &game.queue,
            self.config.view_next,
            self.config.max_depth + 2,
            game.speculate,
        );
        // The original `NextMino::Compare` reconstructs the remaining
        // seven-bag from the visible NEXT queue.  Do not read the referee's
        // hidden RNG state here: that would give the Rust backend information
        // the PPT bot did not have.
        let bag = infer_remaining_bag(&game.queue, self.config.view_next);
        let profile = self.profile_index(
            &game.board,
            &game.garbage_board(),
            game.pending.len(),
            game.opponent,
        );
        self.profile.set(profile);
        let root = BeamNode {
            board: game.board,
            garbage: game.garbage_board(),
            opponent: game.opponent,
            hold: game.hold,
            active: Some(active),
            future,
            bag,
            combo: i32::from(game.combo),
            b2b: i32::from(game.b2b),
            profile,
            score: 0.0,
            family_score: 0.0,
            pc_score: 0.0,
            first: None,
            first_commands: None,
        };
        let deadline =
            (budget.milliseconds > 0).then(|| started + Duration::from_millis(budget.milliseconds));
        let quick_search = (1..=58).contains(&budget.milliseconds);
        let mut beam = vec![root.clone()];
        let mut depth = 0usize;
        let mut iterations = 0u64;
        stats.profile = profile;

        while depth < self.config.max_depth && !beam.is_empty() {
            // SearchAI::Search only observes its timer after lowerDepthLimit.
            // The shipped profile sets that limit to ten, so a short wall
            // clock budget may overrun while Hoiko first builds the minimum
            // lookahead required by its family-score evaluation.
            if depth >= self.config.min_depth {
                if let Some(deadline) = deadline {
                    if Instant::now() >= deadline {
                        break;
                    }
                }
            }
            let mut next = Vec::new();
            if self.config.thread_count > 1 && budget.iterations == 0 && beam.len() > 1 {
                let worker_count = self.config.thread_count.min(beam.len());
                let chunk_size = beam.len().div_ceil(worker_count);
                let workers: Vec<_> = (0..worker_count).map(|_| self.worker_snapshot()).collect();
                let speculate = game.speculate;
                let batches = std::thread::scope(|scope| {
                    let mut handles = Vec::new();
                    for (chunk, worker) in beam.chunks(chunk_size).zip(workers) {
                        handles.push(scope.spawn(move || {
                            let mut output = Vec::new();
                            let mut nodes = 0;
                            let mut root_candidates = 0;
                            for node in chunk {
                                let added = worker.expand_node(
                                    node,
                                    speculate,
                                    depth,
                                    &mut output,
                                    &mut nodes,
                                );
                                if depth == 0 {
                                    root_candidates += added;
                                }
                            }
                            (output, nodes, root_candidates, chunk.len() as u64)
                        }));
                    }
                    handles
                        .into_iter()
                        .map(|handle| handle.join().expect("Hoiko beam worker panicked"))
                        .collect::<Vec<_>>()
                });
                for (mut output, nodes, root_candidates, expanded) in batches {
                    next.append(&mut output);
                    stats.nodes += nodes;
                    stats.root_candidates += root_candidates;
                    iterations += expanded;
                }
            } else {
                for node in &beam {
                    if node.active.is_none() {
                        continue;
                    }
                    let added =
                        self.expand_node(node, game.speculate, depth, &mut next, &mut stats.nodes);
                    stats.root_candidates += if depth == 0 { added } else { 0 };
                    iterations += 1;
                    if budget.iterations > 0 && iterations >= budget.iterations {
                        break;
                    }
                    if depth >= self.config.min_depth {
                        if let Some(deadline) = deadline {
                            if Instant::now() >= deadline {
                                break;
                            }
                        }
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            let weights = self.weights(profile);
            let corrected_score = |node: &BeamNode| {
                node.score - node.combo.max(0) as f64 * weights.get(W::ComboDebt) - node.pc_score
            };
            // The native corrected beam calls `CorrectScore` before every
            // generation cut, not only when selecting the final result.
            next.sort_by(|a, b| corrected_score(b).total_cmp(&corrected_score(a)));
            let mut beam_limit = if quick_search {
                // Native `QuickSearch` uses half of the basic beam.
                (self.config.beam_size / 2).max(1)
            } else {
                self.beam_limit.get().max(1)
            };
            if depth > 7 {
                beam_limit = (beam_limit / 2).max(1);
            }
            next.truncate(beam_limit);
            beam = next;
            depth += 1;
            if budget.iterations > 0 && iterations >= budget.iterations {
                break;
            }
        }
        stats.depth = depth;
        stats.elapsed_us = started.elapsed().as_micros();
        let weights = self.weights(profile);
        let corrected_score = |node: &&BeamNode| {
            node.score - node.combo.max(0) as f64 * weights.get(W::ComboDebt) - node.pc_score
        };
        let chosen_node = beam
            .iter()
            .filter(|node| node.first.is_some_and(|mv| game.legal(mv)))
            .max_by(|a, b| corrected_score(&a).total_cmp(&corrected_score(&b)));
        let chosen = chosen_node.and_then(|node| node.first).or_else(|| {
            self.quick_fallback(&root, game.speculate, game)
                .or_else(|| {
                    hoiko_moves(&game.board, active, None)
                        .into_iter()
                        .map(|candidate| candidate.placement)
                        .find(|mv| game.legal(*mv))
                })
        });
        if let Some(node) = chosen_node {
            self.finish_tactical_state(game, node);
        }
        (chosen, stats)
    }

    fn weights(&self, profile: usize) -> &HoikoWeights {
        match self.config.play_style {
            1 => &self.config.solo_profiles[0],
            2 => &self.config.solo_profiles[1],
            _ => &self.config.profiles[profile.min(3)],
        }
    }

    fn quick_fallback(&self, root: &BeamNode, speculate: bool, game: &Game) -> Option<Placement> {
        let mut children = Vec::new();
        let mut nodes = 0;
        self.expand_node(root, speculate, 0, &mut children, &mut nodes);
        let weights = self.weights(root.profile);
        children
            .into_iter()
            .filter(|node| node.first.is_some_and(|mv| game.legal(mv)))
            .max_by(|a, b| {
                let a = a.score - a.combo.max(0) as f64 * weights.get(W::ComboDebt) - a.pc_score;
                let b = b.score - b.combo.max(0) as f64 * weights.get(W::ComboDebt) - b.pc_score;
                a.total_cmp(&b)
            })
            .and_then(|node| node.first)
    }

    fn worker_snapshot(&self) -> Self {
        let worker = Self::new(self.config.clone());
        worker.profile.set(self.profile.get());
        worker.template.set(self.template.get());
        worker.offset.set(self.offset.get());
        worker.beam_limit.set(self.beam_limit.get());
        worker
    }

    fn prepare_tactical_state(&self, game: &Game) {
        if self.config.use_td_openers {
            let mut template = self.template.get();
            let empty = game.board.cols.iter().all(|&column| column == 0);
            if game.pieces == 0 || (self.config.loop_template && empty && template.drops < 0) {
                template = TemplateState {
                    drops: 0,
                    opener: 1,
                    failed: false,
                };
            } else if template.drops > 14 || game.pending.len() >= 8 {
                template.opener = 0;
            }
            self.template.set(template);
        }
        if self.config.use_offset_off {
            let heights = column_heights(&game.board);
            let center = heights[3..=6].iter().copied().max().unwrap_or(0) as u8;
            let opponent_attack = game.opponent.map_or(0, |opponent| {
                estimate_opponent_attack(opponent).min(255) as u8
            });
            self.offset.set(OffsetState {
                enabled: opponent_attack > 0 || !game.pending.is_empty(),
                center_height: center,
                planned_attack: 0,
                opponent_attack,
            });
        }
    }

    fn finish_tactical_state(&self, game: &Game, node: &BeamNode) {
        if self.config.use_td_openers {
            let mut template = self.template.get();
            template.drops += 1;
            if template.opener != 0 && opener_template_score(&node.board, node.active) <= -100_000 {
                template.failed = true;
                template.opener = 0;
            }
            self.template.set(template);
        }
        if self.config.use_offset_off {
            let mut offset = self.offset.get();
            if let Some(first) = node.first {
                if let Some(lock) = lock(
                    &game.board,
                    &game.garbage_board(),
                    first,
                    i32::from(game.combo),
                    i32::from(game.b2b),
                ) {
                    offset.planned_attack = hoiko_attack(&lock, first).min(255) as u8;
                }
            }
            self.offset.set(offset);
        }
        if let Some(commands) = &node.first_commands {
            let delay = commands.corrected_delay().saturating_sub(5);
            let limit = self
                .config
                .beam_size
                .saturating_mul(delay as usize)
                .checked_div(112)
                .unwrap_or(1)
                .clamp(1, self.config.beam_size.saturating_mul(3).min(1500));
            self.beam_limit.set(limit);
        }
    }

    fn expand_node(
        &self,
        node: &BeamNode,
        speculate: bool,
        depth: usize,
        output: &mut Vec<BeamNode>,
        nodes: &mut u64,
    ) -> usize {
        let Some(active) = node.active else { return 0 };
        let mut added = 0;
        let active_moves = hoiko_moves(&node.board, active, None);
        for candidate in active_moves {
            let placement = candidate.placement;
            if let Some(lock) = lock(&node.board, &node.garbage, placement, node.combo, node.b2b) {
                let hold = node.hold;
                let cursors = self.after_piece(node, node.future.clone(), node.bag, speculate);
                for (next_active, next_future, next_bag) in cursors {
                    output.push(self.child(
                        node,
                        candidate.clone(),
                        lock.clone(),
                        depth,
                        hold,
                        next_active,
                        next_future,
                        next_bag,
                    ));
                    added += 1;
                    *nodes += 1;
                    if added >= 100 {
                        return added;
                    }
                }
            }
        }
        if self.config.use_hold {
            match node.hold {
                Some(held) => {
                    if held != active {
                        for candidate in hoiko_moves(&node.board, held, Some(HoikoCommand::Hold)) {
                            let placement = candidate.placement;
                            if let Some(lock) =
                                lock(&node.board, &node.garbage, placement, node.combo, node.b2b)
                            {
                                let cursors = self.after_piece(
                                    node,
                                    node.future.clone(),
                                    node.bag,
                                    speculate,
                                );
                                for (next_active, next_future, next_bag) in cursors {
                                    output.push(self.child(
                                        node,
                                        candidate.clone(),
                                        lock.clone(),
                                        depth,
                                        Some(active),
                                        next_active,
                                        next_future,
                                        next_bag,
                                    ));
                                    added += 1;
                                    *nodes += 1;
                                    if added >= 100 {
                                        return added;
                                    }
                                }
                            }
                        }
                    }
                }
                None => {
                    // An empty hold consumes one preview immediately.  The
                    // original binary's Command::SwapHold does exactly this.
                    let draws = if let Some(&piece) = node.future.first() {
                        vec![(piece, node.future[1..].to_vec(), node.bag)]
                    } else {
                        Vec::new()
                    };
                    for (piece, remaining, bag) in draws {
                        for candidate in
                            hoiko_moves(&node.board, piece, Some(HoikoCommand::FirstHold))
                        {
                            let placement = candidate.placement;
                            if let Some(lock) =
                                lock(&node.board, &node.garbage, placement, node.combo, node.b2b)
                            {
                                for (next_active, next_future, next_bag) in
                                    self.after_piece(node, remaining.clone(), bag, speculate)
                                {
                                    output.push(self.child(
                                        node,
                                        candidate.clone(),
                                        lock.clone(),
                                        depth,
                                        Some(active),
                                        next_active,
                                        next_future,
                                        next_bag,
                                    ));
                                    added += 1;
                                    *nodes += 1;
                                    if added >= 100 {
                                        return added;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        added
    }

    fn child(
        &self,
        parent: &BeamNode,
        candidate: MoveCandidate,
        lock: LockResult,
        depth: usize,
        hold: Option<Piece>,
        active: Option<Piece>,
        future: Vec<Piece>,
        bag: EnumSet<Piece>,
    ) -> BeamNode {
        let weights = self.weights(parent.profile);
        let placement = candidate.placement;
        let evaluation = self.evaluate(
            parent,
            &lock,
            placement,
            candidate.commands.raw_delay,
            hold,
            active,
            weights,
        );
        let trust = trust_rate(self.config.view_next, depth);
        // Native Evaluate keeps two related scores. `familyScore` uses the
        // nexus blend and becomes the baseline for descendants; `score`
        // ranks the current node using the full board score.  Mixing these
        // into one running total makes height and roughness penalties too
        // weak and is the source of the characteristic terrain collapse.
        let family_local =
            ((evaluation.action_score + (evaluation.board_score * weights.nexus).trunc()) * trust)
                .trunc();
        let score_local = ((evaluation.action_score + evaluation.board_score) * trust).trunc();
        let template_override = if self.config.use_td_openers && self.template.get().opener != 0 {
            opener_template_score(&lock.board, active)
        } else {
            0
        };
        let (score, family_score) = if template_override != 0 {
            let score = (f64::from(template_override) * trust).trunc();
            (score, score)
        } else {
            (
                parent.family_score + score_local,
                parent.family_score + family_local,
            )
        };
        BeamNode {
            board: lock.board,
            garbage: lock.garbage,
            opponent: parent.opponent,
            hold,
            active,
            future,
            bag,
            combo: lock.combo,
            b2b: lock.b2b,
            profile: parent.profile,
            score,
            family_score,
            pc_score: evaluation.pc_score,
            first: parent.first.or(Some(placement)),
            first_commands: parent.first_commands.clone().or(Some(candidate.commands)),
        }
    }

    fn after_piece(
        &self,
        _node: &BeamNode,
        future: Vec<Piece>,
        bag: EnumSet<Piece>,
        speculate: bool,
    ) -> Vec<(Option<Piece>, Vec<Piece>, EnumSet<Piece>)> {
        if let Some((&active, rest)) = future.split_first() {
            return vec![(Some(active), rest.to_vec(), bag)];
        }
        let _ = speculate;
        vec![(None, Vec::new(), bag)]
    }

    fn profile_index(
        &self,
        board: &Board,
        garbage: &Board,
        pending: usize,
        opponent: Option<OpponentState>,
    ) -> usize {
        if self.config.play_style != 0 {
            let stack_plus_one = source_column_heights(board)
                .0
                .into_iter()
                .max()
                .unwrap_or(-1)
                + 1;
            return usize::from(stack_plus_one > 10);
        }
        // This is the source `TransitionWeight` decision. The Windows binary
        // retains the selected weight pointer across searches, producing a
        // two-row hysteresis around its defensive profiles. An incoming queue
        // remains a conservative recovery signal when no opponent snapshot
        // is available.
        let (heights, source_garbage_height) = source_column_heights(board);
        let garbage_height = if source_garbage_height < 0 {
            marker_garbage_height(garbage).unwrap_or(source_garbage_height)
        } else {
            source_garbage_height
        };
        let center = heights[3..=6].iter().copied().max().unwrap_or(-1);
        let stack_plus_one = heights.iter().copied().max().unwrap_or(-1) + 1;
        let transition = if self.profile.get() >= 2 { 13 } else { 15 };
        let opponent_combo = opponent.map_or(0, |state| state.combo);
        let multi_evaluation = opponent.is_some_and(|state| state.multi_evaluation);
        let opponent_pc = opponent.is_some_and(|state| opponent_is_pc(&state.board));
        let profile = if center + 1 <= transition {
            if multi_evaluation
                && opponent_combo > 8
                && stack_plus_one >= if garbage_height < 3 { 11 } else { 6 }
            {
                return 3;
            }
            if stack_plus_one >= 11 {
                return 0;
            }
            let even = board_popcount(board) % 2 == 0;
            if even {
                if multi_evaluation
                    && (opponent_pc || opponent_combo >= 3)
                    && garbage_height < 0
                    && stack_plus_one <= 5
                {
                    return 1;
                }
            } else if garbage_height < 0 && stack_plus_one <= 5 {
                return 1;
            }
            if pending >= 8 && opponent.is_none() {
                return 3;
            }
            0
        } else if multi_evaluation && opponent_combo > 8 {
            3
        } else {
            2
        };
        // Keep the explicit garbage argument part of the transition API. It
        // is intentionally read here so a marker-only row is never optimized
        // away when a caller supplies a board assembled from replay cells.
        let _ = garbage;
        profile
    }

    fn evaluate(
        &self,
        parent: &BeamNode,
        lock: &LockResult,
        placement: Placement,
        move_delay: u32,
        resulting_hold: Option<Piece>,
        next_active: Option<Piece>,
        weight: &HoikoWeights,
    ) -> Evaluation {
        let board = &lock.board;
        let (heights, source_garbage_height) = source_column_heights(board);
        let garbage_height = if source_garbage_height < 0 {
            marker_garbage_height(&lock.garbage).unwrap_or(source_garbage_height)
        } else {
            source_garbage_height
        };
        let stack_height = heights.iter().copied().max().unwrap_or(-1);
        let stack_plus_one = (stack_height + 1).max(0) as f64;
        let mut board_score = if board
            .cols
            .iter()
            .any(|column| column & !NATIVE_BOARD_MASK != 0)
        {
            -1_000_000_000.0
        } else {
            stack_plus_one * weight.get(W::MaxHeight)
        };
        if stack_height > 9 {
            let center_height = heights[3..=6].iter().copied().max().unwrap_or(-1).max(10);
            board_score += (center_height + 1 - 10) as f64 * weight.get(W::Over10);
            if center_height > 14 {
                board_score += (center_height + 1 - 15) as f64 * weight.get(W::Over15);
            }
        }

        let (well_column, well_depth, well_hint) = well(board, garbage_height);
        let well_depth_peak = well_peak(well_depth, weight.get(W::WellPeak));
        let well_hint_peak = well_peak(well_hint, weight.get(W::WellPeak));
        board_score += well_hint_peak as f64 * weight.get(W::WellHint);
        board_score += well_depth_peak as f64 * weight.get(W::WellDepth);
        if let Some(column) = well_column {
            board_score += weight.column[column] as f64;
        }
        board_score += eval_updown(
            &mut heights.clone(),
            well_column,
            well_depth,
            weight.get(W::WellPeak),
        ) * weight.get(W::UpDown2);
        board_score += eval_underground(board, &heights, garbage_height, weight);
        board_score += eval_ground(board, &heights, garbage_height + well_depth as i32, weight);
        board_score += eval_resource(
            board,
            &heights,
            garbage_height,
            weight,
            self.config.use_s4w,
            parent.combo,
            parent.opponent,
        );
        board_score += eval_tsd(
            board,
            &heights,
            garbage_height + well_depth as i32,
            resulting_hold,
            next_active,
            &parent.future,
            weight,
        );
        board_score += eval_tst_and_dt(
            board,
            &heights,
            garbage_height + well_depth as i32,
            garbage_height,
            resulting_hold,
            next_active,
            &parent.future,
            weight,
        );
        // `EvalContBonus` is a board term in the native evaluator.  The
        // operation-time component (MoveDelay) is deliberately omitted: the
        // requested Rust port evaluates placements from state, without PPT's
        // real-time input telemetry.
        if resulting_hold == Some(Piece::I) {
            board_score += weight.get(W::HoldI);
        }
        if resulting_hold == Some(Piece::T) {
            board_score += weight.get(W::HoldT);
        }
        if lock.b2b > 0 {
            board_score += weight.get(W::Btb);
        }
        if self.config.use_pcstack && board_popcount(board) % 2 == 0 {
            board_score += source_pc_stack_score(board) as f64 * weight.get(W::PcStack);
        }

        let mut action = 0.0;
        action += f64::from(move_delay >> 4) * weight.get(W::MoveDelay);
        if lock.perfect_clear {
            action += weight.get(W::Pc);
            // The release refunds half of the movement penalty for a PC.
            action += f64::from(move_delay >> 5) * -weight.get(W::MoveDelay);
        }
        let line_weight = if lock.perfect_clear {
            0.0
        } else {
            match (placement.spin, lock.lines) {
                (Spin::None, 0) => 0.0,
                (Spin::None, 1) => weight.get(W::Single),
                (Spin::None, 2) => weight.get(W::Double),
                (Spin::None, 3) => weight.get(W::Triple),
                (Spin::None, 4) => weight.get(W::Quad),
                (Spin::Mini, 1) => {
                    weight.get(W::Single) + weight.get(W::WasteT) - weight.get(W::Btb)
                }
                (Spin::Full, 1) => weight.get(W::Tss),
                (Spin::Full, 2) => weight.get(W::Tsd),
                (Spin::Full, 3) => weight.get(W::Tst),
                (_, _) => 0.0,
            }
        };
        action += line_weight;
        if self.config.use_offset_off {
            let offset = self.offset.get();
            if offset.enabled {
                let outgoing = hoiko_attack(lock, placement);
                if can_offset_off(offset, outgoing) {
                    action += f64::from(outgoing.min(8) * 25);
                }
            }
        }
        if lock.lines == 0 {
            action += match placement.location.piece {
                Piece::T => weight.get(W::WasteT),
                Piece::I => weight.get(W::WasteI),
                _ => 0.0,
            };
            action += lock.combo as f64 * weight.get(W::ComboDebt);
        } else {
            if lock.b2b == 0 {
                match placement.location.piece {
                    Piece::T => action += weight.get(W::WasteT),
                    Piece::I => action += weight.get(W::WasteI),
                    _ => {}
                }
            } else if lock.back_to_back {
                action += weight.get(W::ComboBtb);
                if lock.combo > 1 {
                    action += weight.get(W::ComboBtb);
                }
            }
            action += weight.get(W::ComboDebt);
            if lock.combo >= 3 && !lock.perfect_clear {
                action += weight.combo_table[combo_rank(lock.combo)] as f64;
            }
            if self.config.play_style == 0 && parent.profile >= 2 {
                let mut attack = match (placement.spin, lock.lines) {
                    (Spin::None, 2) => 1,
                    (Spin::None, 3) => 2,
                    (Spin::None, 4) => 4,
                    (Spin::Full, 1) => 2,
                    (Spin::Full, 2) => 4,
                    (Spin::Full, 3) => 6,
                    _ => 0,
                };
                if lock.back_to_back {
                    attack += 1;
                }
                if attack > 0 {
                    action += weight.combo_table[(attack.min(5) - 1) as usize] as f64;
                }
            }
        }

        // `pcChance` is the active one-piece PC look-ahead in the native
        // evaluator. Its templates only inspect stacks up to height three;
        // restricting the exact move check to that range keeps the beam
        // search bounded while preserving the observable early-PC bonus.
        let mut pc_score = 0.0;
        if garbage_height < 0 {
            let mut candidates = Vec::with_capacity(2);
            if let Some(piece) = next_active {
                candidates.push(piece);
            }
            if let Some(piece) = resulting_hold {
                if !candidates.contains(&piece) {
                    candidates.push(piece);
                }
            }
            if board_popcount(board) % 2 == 0
                && candidates
                    .into_iter()
                    .any(|piece| source_pc_chance(board, piece))
            {
                pc_score = weight.get(W::PcChance);
            }
        }
        // Eval subtracts the parent's pre-PC bonus before adding the current
        // one. This prevents a one-piece PC opportunity from being counted at
        // every depth of the same family.
        action += pc_score - parent.pc_score;
        Evaluation {
            board_score,
            action_score: action,
            pc_score,
        }
    }
}

fn corrected_beam_size(config: &HoikoConfig) -> usize {
    let threads = config.thread_count.max(1);
    let max_threads = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(threads)
        .max(threads);
    let scale = if threads == 1 {
        1.0
    } else {
        threads as f64 * (1.0 - threads as f64 * (0.4 / max_threads as f64))
    };
    ((config.beam_size as f64 * scale * 3.0) as usize).clamp(1, 1_500)
}

fn hoiko_moves(
    board: &Board,
    piece: Piece,
    hold_command: Option<HoikoCommand>,
) -> Vec<MoveCandidate> {
    let mut spawn = PieceLocation {
        piece,
        rotation: Rotation::North,
        x: 4,
        y: 19,
    };
    if spawn.obstructed(board) {
        spawn.y += 1;
        if spawn.obstructed(board) {
            return Vec::new();
        }
    }
    let mut initial = CommandPath::default();
    if let Some(command) = hold_command {
        initial = initial.pushed(command, 16);
    }
    let first = MoveCandidate {
        placement: Placement {
            location: spawn,
            spin: Spin::None,
        },
        commands: initial,
        order: 0,
    };
    let mut babies = vec![first];
    let mut positions = HashMap::<PieceLocation, usize>::new();
    positions.insert(spawn, 0);
    let mut order = 1usize;

    // ExpandBaseNode: L, R, B, A until the spawn-height state graph closes.
    let mut cursor = 0;
    while cursor < babies.len() && babies.len() < 300 {
        let source = babies[cursor].clone();
        let mut targets = Vec::with_capacity(4);
        for (dx, command) in [(-1, HoikoCommand::Left), (1, HoikoCommand::Right)] {
            let mut location = source.placement.location;
            location.x += dx;
            if !location.obstructed(board) {
                targets.push((
                    Placement {
                        location,
                        spin: Spin::None,
                    },
                    source.commands.pushed(command, 30),
                ));
            }
        }
        if let Some(placement) = hoiko_rotate(source.placement.location, board, false) {
            targets.push((
                placement,
                source.commands.pushed(HoikoCommand::RotateCcw, 16),
            ));
        }
        if let Some(placement) = hoiko_rotate(source.placement.location, board, true) {
            targets.push((
                placement,
                source.commands.pushed(HoikoCommand::RotateCw, 16),
            ));
        }
        for (placement, commands) in targets {
            register_baby(&mut babies, &mut positions, placement, commands, &mut order);
        }
        cursor += 1;
    }

    // ExpandDeriveNode: drop to the current surface, then derive only moves
    // that travel below an existing column or rotations that enter a notch.
    let heights = column_heights(board);
    let mut generation_start = 0;
    let mut generation_end = babies.len();
    while generation_start < generation_end && babies.len() < 300 {
        let snapshot_end = generation_end;
        for index in generation_start..snapshot_end {
            let source = babies[index].clone();
            let distance = source.placement.location.drop_distance(board);
            let mut dropped = source.clone();
            if distance > 0 {
                dropped.placement.location.y -= distance;
                dropped.placement.spin = Spin::None;
                dropped.commands = dropped
                    .commands
                    .pushed(HoikoCommand::SoftDrop(distance as u8), distance as u32 * 33);
            }

            for (dx, command) in [(-1, HoikoCommand::Left), (1, HoikoCommand::Right)] {
                let mut shifted = dropped.clone();
                let mut moved = 0u32;
                let mut valid = true;
                loop {
                    let mut location = shifted.placement.location;
                    location.x += dx;
                    if location.obstructed(board) {
                        break;
                    }
                    shifted.placement = Placement {
                        location,
                        spin: Spin::None,
                    };
                    moved += 1;
                    if !above_cell(location, &heights) {
                        valid = false;
                        break;
                    }
                }
                if moved > 0 && valid {
                    for _ in 0..moved {
                        shifted.commands = shifted.commands.pushed(command, 30);
                    }
                    register_baby(
                        &mut babies,
                        &mut positions,
                        shifted.placement,
                        shifted.commands,
                        &mut order,
                    );
                }
            }
            for (clockwise, command) in [
                (false, HoikoCommand::RotateCcw),
                (true, HoikoCommand::RotateCw),
            ] {
                if let Some(placement) = hoiko_rotate(dropped.placement.location, board, clockwise)
                {
                    if above_cell(placement.location, &heights) || placement.spin != Spin::None {
                        register_baby(
                            &mut babies,
                            &mut positions,
                            placement,
                            dropped.commands.pushed(command, 16),
                            &mut order,
                        );
                    }
                }
            }
        }
        if babies.len() == generation_end {
            break;
        }
        generation_start = generation_end;
        generation_end = babies.len();
    }

    let mut locks = HashMap::<Placement, MoveCandidate>::new();
    for baby in babies {
        let distance = baby.placement.location.drop_distance(board);
        let placement = Placement {
            location: PieceLocation {
                y: baby.placement.location.y - distance,
                ..baby.placement.location
            }
            .canonical_form(),
            spin: if distance == 0 {
                baby.placement.spin
            } else {
                Spin::None
            },
        };
        let commands = baby.commands.pushed(HoikoCommand::HardDrop, 116);
        if locks
            .get(&placement)
            .is_none_or(|old| commands.raw_delay < old.commands.raw_delay)
        {
            locks.insert(
                placement,
                MoveCandidate {
                    placement,
                    commands,
                    order: baby.order,
                },
            );
        }
    }
    let mut result: Vec<_> = locks.into_values().collect();
    result.sort_by_key(|candidate| candidate.order);
    result
}

fn register_baby(
    babies: &mut Vec<MoveCandidate>,
    positions: &mut HashMap<PieceLocation, usize>,
    placement: Placement,
    commands: CommandPath,
    order: &mut usize,
) {
    if let Some(&index) = positions.get(&placement.location) {
        if commands.raw_delay < babies[index].commands.raw_delay {
            babies[index].placement = placement;
            babies[index].commands = commands;
        }
        return;
    }
    positions.insert(placement.location, babies.len());
    babies.push(MoveCandidate {
        placement,
        commands,
        order: *order,
    });
    *order += 1;
}

fn above_cell(location: PieceLocation, heights: &[u32; WIDTH]) -> bool {
    location.cells().into_iter().any(|(x, y)| {
        (0..WIDTH as i8).contains(&x) && heights[x as usize].saturating_sub(1) as i8 > y
    })
}

fn hoiko_rotate(from: PieceLocation, board: &Board, clockwise: bool) -> Option<Placement> {
    if from.piece == Piece::O {
        return None;
    }
    let target_rotation = if clockwise {
        from.rotation.cw()
    } else {
        from.rotation.ccw()
    };
    let unkicked = PieceLocation {
        rotation: target_rotation,
        ..from
    };
    for (kick_index, (dx, dy)) in hoiko_kicks(from.piece, from.rotation, target_rotation)
        .into_iter()
        .enumerate()
    {
        let target = PieceLocation {
            x: unkicked.x + dx,
            y: unkicked.y + dy,
            ..unkicked
        };
        if target.obstructed(board) {
            continue;
        }
        let spin = if target.piece != Piece::T {
            Spin::None
        } else {
            let corners = [(-1, -1), (1, -1), (-1, 1), (1, 1)]
                .into_iter()
                .filter(|&(x, y)| board.occupied((target.x + x, target.y + y)))
                .count();
            let fronts = [(-1, 1), (1, 1)]
                .into_iter()
                .map(|cell| target.rotation.rotate_cell(cell))
                .filter(|&(x, y)| board.occupied((target.x + x, target.y + y)))
                .count();
            if corners < 3 {
                Spin::None
            } else if fronts == 2 || kick_index == 4 {
                Spin::Full
            } else {
                Spin::Mini
            }
        };
        return Some(Placement {
            location: target,
            spin,
        });
    }
    None
}

const fn hoiko_offsets(piece: Piece, rotation: Rotation) -> [(i8, i8); 5] {
    match piece {
        Piece::O => [(0, 0); 5],
        Piece::I => match rotation {
            Rotation::North => [(0, 0), (-1, 0), (2, 0), (-1, 0), (2, 0)],
            Rotation::East => [(-1, 0), (0, 0), (0, 0), (0, 1), (0, -2)],
            Rotation::South => [(-1, 1), (1, 1), (-2, 1), (1, 0), (-2, 0)],
            Rotation::West => [(0, 1), (0, 1), (0, 1), (0, -1), (0, 2)],
        },
        _ => match rotation {
            Rotation::North | Rotation::South => [(0, 0); 5],
            Rotation::East => [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
            Rotation::West => [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
        },
    }
}

const fn hoiko_kicks(piece: Piece, from: Rotation, to: Rotation) -> [(i8, i8); 5] {
    let from = hoiko_offsets(piece, from);
    let to = hoiko_offsets(piece, to);
    let mut result = [(0, 0); 5];
    let mut i = 0;
    while i < 5 {
        result[i] = (from[i].0 - to[i].0, from[i].1 - to[i].1);
        i += 1;
    }
    result
}

fn hoiko_attack(lock: &LockResult, placement: Placement) -> u32 {
    let mut attack = match (placement.spin, lock.lines) {
        (Spin::None, 2) => 1,
        (Spin::None, 3) => 2,
        (Spin::None, 4) => 4,
        (Spin::Mini, 1) => 1,
        (Spin::Full, 1) => 2,
        (Spin::Full, 2) => 4,
        (Spin::Full, 3) => 6,
        _ => 0,
    };
    if lock.back_to_back && attack > 0 {
        attack += 1;
    }
    if lock.combo > 2 && attack > 0 {
        attack += 1;
    }
    if lock.perfect_clear {
        attack += 10;
    }
    attack
}

fn estimate_opponent_attack(opponent: OpponentState) -> usize {
    let combo = usize::from(opponent.combo);
    opponent.pending
        + usize::from(opponent.b2b)
        + match combo {
            0..=2 => 0,
            3..=4 => 1,
            5..=6 => 2,
            7..=8 => 3,
            9..=11 => 4,
            _ => 5,
        }
}

fn can_offset_off(state: OffsetState, outgoing: u32) -> bool {
    if !state.enabled || usize::from(state.center_height) + outgoing as usize >= 15 {
        return false;
    }
    if state.opponent_attack >= 4 {
        return true;
    }
    outgoing < u32::from(state.opponent_attack) || state.opponent_attack > 2
}

fn opener_template_score(board: &Board, active: Option<Piece>) -> i32 {
    const TARGETS: [([u16; 4], Piece, i32); 12] = [
        ([1007, 1015, 135, 6], Piece::L, 30_000),
        ([879, 823, 775, 518], Piece::T, 30_000),
        ([987, 947, 899, 385], Piece::T, 30_000),
        ([911, 967, 903, 518], Piece::S, 30_000),
        ([991, 959, 135, 1], Piece::S, 30_000),
        ([1007, 1015, 900, 512], Piece::Z, 30_000),
        ([967, 911, 903, 385], Piece::Z, 30_000),
        ([991, 959, 900, 384], Piece::J, 30_000),
        ([1007, 1015, 932, 512], Piece::Z, 20_000),
        ([991, 959, 151, 1], Piece::S, 20_000),
        ([991, 959, 900, 768], Piece::L, 10_000),
        ([1007, 1015, 135, 3], Piece::J, 10_000),
    ];
    let rows = [
        native_row(board, 0),
        native_row(board, 1),
        native_row(board, 2),
        native_row(board, 3),
    ];
    if (4..32).any(|y| native_row(board, y) != 0) {
        return -1_000_000;
    }
    let occupied: u32 = rows.iter().map(|row| row.count_ones()).sum();
    let mut best = None;
    for (target, expected, terminal_score) in TARGETS {
        // Every intermediate node in a valid TD branch is a subset of at
        // least one six-piece checkpoint. This turns the extracted terminal
        // predicates into the complete prefix decision tree instead of
        // rewarding arbitrary low stacks between checkpoints.
        if rows
            .iter()
            .zip(target)
            .any(|(&actual, target)| actual & !target != 0)
        {
            continue;
        }
        let complete = rows == target;
        let score = if complete {
            if active == Some(expected) {
                terminal_score
            } else {
                -1_000_000
            }
        } else {
            1_000 + occupied as i32 * 100
        };
        best = Some(best.map_or(score, |old: i32| old.max(score)));
    }
    best.unwrap_or(-1_000_000)
}

fn predict_future(
    queue: &std::collections::VecDeque<Piece>,
    view_next: usize,
    required: usize,
    speculate: bool,
) -> Vec<Piece> {
    let visible = if speculate {
        queue.len().min(view_next.saturating_add(1))
    } else {
        queue.len()
    };
    let size = required.max(visible).max(28);
    let mut predicted: Vec<_> = (0..size)
        .map(|index| HOIKO_BAG_ORDER[index % HOIKO_BAG_ORDER.len()])
        .collect();
    for (index, &observed) in queue.iter().take(visible).enumerate() {
        if predicted[index] == observed {
            continue;
        }
        if let Some(found) = predicted[index + 1..]
            .iter()
            .position(|&piece| piece == observed)
            .map(|offset| index + 1 + offset)
        {
            for cursor in (index + 1..=found).rev() {
                predicted[cursor] = predicted[cursor - 1];
            }
        }
        predicted[index] = observed;
    }
    predicted
        .into_iter()
        .skip(1)
        .take(if speculate {
            required
        } else {
            visible.saturating_sub(1)
        })
        .collect()
}

fn infer_remaining_bag(
    queue: &std::collections::VecDeque<Piece>,
    view_next: usize,
) -> EnumSet<Piece> {
    let mut bag = EnumSet::all();
    for piece in queue.iter().skip(1).take(view_next.max(1)) {
        if !bag.contains(*piece) {
            // A repeated visible piece is the first reliable indication that
            // the previous seven-bag ended.  The source predictor resets its
            // internal Bag at the same boundary.
            bag = EnumSet::all();
        }
        bag.remove(*piece);
    }
    bag
}

fn lock(
    board: &Board,
    garbage: &Board,
    placement: Placement,
    combo: i32,
    b2b: i32,
) -> Option<LockResult> {
    let cells = placement.location.cells();
    if cells
        .iter()
        .any(|&(x, y)| x < 0 || x >= WIDTH as i8 || y < 0 || y >= 40 || board.occupied((x, y)))
    {
        return None;
    }
    // The referee's visible field is 20 rows.  A lock wholly above it is a
    // top-out even though the engine keeps a 40-row buffer.
    if cells.iter().all(|&(_, y)| y >= 20) {
        return None;
    }
    let mut board = *board;
    let mut garbage = *garbage;
    board.place(placement.location);
    let mask = board.line_clears();
    let lines = mask.count_ones();
    if mask != 0 {
        board.remove_lines(mask);
        garbage.remove_lines(mask);
    }
    let difficult = lines > 0 && (lines == 4 || placement.spin != Spin::None);
    let next_b2b = if lines == 0 {
        b2b
    } else if difficult {
        b2b.max(0).saturating_add(1)
    } else {
        0
    };
    let back_to_back = next_b2b > 1;
    let next_combo = if lines > 0 {
        combo.max(0).saturating_add(1)
    } else if combo > 0 {
        -combo
    } else {
        0
    };
    Some(LockResult {
        perfect_clear: lines > 0 && board.cols.iter().all(|column| *column == 0),
        board,
        garbage,
        lines,
        combo: next_combo,
        b2b: next_b2b,
        back_to_back,
    })
}

fn one_piece_pc(board: &Board, piece: Piece) -> bool {
    find_moves(board, piece).into_iter().any(|(mv, _)| {
        let mut test = *board;
        if mv
            .location
            .cells()
            .iter()
            .any(|&(x, y)| x < 0 || x >= WIDTH as i8 || y < 0 || y >= 40 || test.occupied((x, y)))
        {
            return false;
        }
        test.place(mv.location);
        let mask = test.line_clears();
        if mask == 0 {
            return false;
        }
        test.remove_lines(mask);
        test.cols.iter().all(|column| *column == 0)
    })
}

fn column_heights(board: &Board) -> [u32; WIDTH] {
    let mut heights = [0; WIDTH];
    for (height, column) in heights.iter_mut().zip(board.cols) {
        let native_column = column & NATIVE_BOARD_MASK;
        *height = 64 - native_column.leading_zeros();
    }
    heights
}

fn row_mask(board: &Board, y: u32) -> u16 {
    if y >= 32 {
        return 0;
    }
    let mut mask = 0u16;
    for (x, column) in board.cols.iter().enumerate() {
        if column & (1u64 << y) != 0 {
            mask |= 1 << x;
        }
    }
    mask
}

/// Reproduce `Board::GetColumnHeight` from the public source.  Heights are
/// top row indices (`-1` means empty), and the second value is the row at
/// which the accumulated mask first becomes a complete ten-wide line.  Hoiko
/// calls that row `garbageHeight` and evaluates the rows below it as
/// underground support.
fn source_column_heights(board: &Board) -> ([i32; WIDTH], i32) {
    let mut heights = [-1i32; WIDTH];
    let stack_height = column_heights(board)
        .into_iter()
        .max()
        .map(|height| height as i32 - 1)
        .unwrap_or(-1);
    if stack_height < 0 {
        return (heights, -1);
    }
    let mut seen = 0u16;
    let mut garbage_height = -1;
    for y in (0..=stack_height as u32).rev() {
        seen |= row_mask(board, y);
        if seen == 0x03ff {
            garbage_height = y as i32;
            break;
        }
        for x in 0..WIDTH {
            if seen & (1 << x) != 0 {
                heights[x] += 1;
            }
        }
    }
    if garbage_height >= 0 {
        for height in &mut heights {
            *height += garbage_height + 1;
        }
    }
    (heights, garbage_height)
}

fn marker_garbage_height(garbage: &Board) -> Option<i32> {
    let mut highest = None;
    for y in 0..40u32 {
        let row = row_mask(garbage, y);
        if row.count_ones() == 9 {
            highest = Some(y as i32);
        } else if highest.is_some() {
            break;
        }
    }
    highest
}

fn well(board: &Board, garbage_height: i32) -> (Option<usize>, u32, u32) {
    let max_height = column_heights(board)
        .into_iter()
        .max()
        .unwrap_or(0)
        .saturating_sub(1);
    let start = (garbage_height + 1).max(0) as u32;
    if start > max_height {
        return (None, 0, 0);
    }
    let row = row_mask(board, start);
    if row.count_ones() != 9 {
        return (None, 0, 0);
    }
    let column = (!row & 0x03ff).trailing_zeros() as usize;
    let mut depth = 1;
    while start + depth <= max_height && row_mask(board, start + depth) == row {
        depth += 1;
    }

    // The source stores the row with the leftmost cell in the most
    // significant bit.  Reverse its five-cell hint into this engine's
    // least-significant-bit-left representation, then shift by the well
    // column just as the C++ implementation does.
    let hint_mask = reverse10((0b1_01000_00000u16 >> column) & 0x03ff);
    let mut hint = 1;
    let mut y = start + 1;
    while y <= max_height && (row_mask(board, y) & hint_mask) == hint_mask {
        hint += 1;
        y += 1;
    }
    (Some(column), depth, hint)
}

fn well_peak(depth: u32, weight: f64) -> u32 {
    if depth == 2 {
        1
    } else {
        depth.min(weight.max(0.0) as u32)
    }
}

fn eval_updown(
    heights: &mut [i32; WIDTH],
    well_column: Option<usize>,
    well_depth: u32,
    well_peak_weight: f64,
) -> f64 {
    if let Some(column) = well_column {
        heights[column] += well_depth.min(well_peak_weight.max(0.0) as u32) as i32;
    }
    let mut score = 0i64;
    let diff = heights[0] - heights[1];
    score += i64::from(diff * diff) * if diff < 0 { 2 } else { 1 };
    for x in 1..WIDTH - 2 {
        let diff = heights[x] - heights[x + 1];
        score += i64::from(diff * diff);
    }
    let diff = heights[8] - heights[9];
    score += i64::from(diff * diff) * if diff > 0 { 2 } else { 1 };
    score as f64
}

fn eval_underground(
    board: &Board,
    heights: &[i32; WIDTH],
    garbage_height: i32,
    weight: &HoikoWeights,
) -> f64 {
    if garbage_height < 0 {
        return 0.0;
    }
    let mut cover = 0i32;
    let mut donate_cover = 0i32;
    let mut prev = row_mask(board, garbage_height as u32) ^ 0x03ff;
    let mut bit = prev;
    for x in 0..WIDTH {
        if bit & (1 << x) != 0 {
            donate_cover += (heights[x] - garbage_height).min(5);
        }
    }
    for y in (0..garbage_height).rev() {
        bit = row_mask(board, y as u32) ^ 0x03ff;
        if bit & !prev != 0 {
            if bit.count_ones() == 1 {
                let x = bit.trailing_zeros() as usize;
                cover += (heights[x] - y).min(5);
            } else {
                for x in 0..WIDTH {
                    if bit & (1 << x) != 0 {
                        donate_cover += heights[x] - y;
                    }
                }
            }
        }
        prev = bit;
    }
    donate_cover as f64 * weight.get(W::DonateCover) + cover as f64 * weight.get(W::Cover)
}

fn eval_ground(board: &Board, heights: &[i32; WIDTH], y0: i32, weight: &HoikoWeights) -> f64 {
    let stack = heights.iter().copied().max().unwrap_or(-1);
    if stack <= y0 + 1 {
        return 0.0;
    }
    let mut roof = 0i32;
    let mut bad_roof = 0i32;
    let mut pierce = 0i32;
    let mut single_roofs = 0i32;
    let mut covered = row_mask(board, stack.max(0) as u32);
    for y in (y0 + 1..stack).rev() {
        let holes = (!row_mask(board, y as u32)) & covered & 0x03ff;
        let mut worst = 0;
        for x in 0..WIDTH {
            if holes & (1 << x) == 0 {
                continue;
            }
            let depth = heights[x] - y;
            let open_right = x + 2 < WIDTH && heights[x + 1] < y && heights[x + 2] < y;
            let open_left = x >= 2 && heights[x - 1] < y && heights[x - 2] < y;
            if open_left || open_right {
                roof += depth;
                single_roofs += 1;
            } else if (x + 1 < WIDTH && heights[x + 1] < y) || (x > 0 && heights[x - 1] < y) {
                bad_roof += depth;
            } else {
                worst = worst.max(depth);
            }
        }
        pierce += worst;
        covered |= row_mask(board, y as u32);
    }
    if single_roofs == 1 {
        roof = 1;
    }
    roof as f64 * weight.get(W::Roof)
        + bad_roof as f64 * weight.get(W::BadRoof)
        + pierce as f64 * weight.get(W::Pierce)
}

fn native_row(board: &Board, y: i32) -> u16 {
    if !(0..40).contains(&y) {
        0
    } else {
        reverse10(row_mask(board, y as u32))
    }
}

fn native_heights(heights: &[i32; WIDTH]) -> [i32; WIDTH] {
    std::array::from_fn(|index| heights[WIDTH - 1 - index])
}

fn t_piece_rank(hold: Option<Piece>, active: Option<Piece>, future: &[Piece]) -> i32 {
    if hold == Some(Piece::T) {
        return 2;
    }
    let mut rank = 0;
    for piece in active.into_iter().chain(future.iter().copied()).take(7) {
        if piece == Piece::T {
            rank += 1;
            if rank == 2 {
                break;
            }
        }
    }
    rank
}

fn eval_tsd(
    board: &Board,
    heights: &[i32; WIDTH],
    y0: i32,
    hold: Option<Piece>,
    active: Option<Piece>,
    future: &[Piece],
    weight: &HoikoWeights,
) -> f64 {
    let h = native_heights(heights);
    let mut score = 0i32;
    let mut inspect = |x: usize, y: i32, mask: u16, center: u16, side: u16| {
        if y < y0 || y >= h[x].min(h[x + 2]) + 1 {
            return;
        }
        let lower = native_row(board, y);
        let middle = native_row(board, y + 1);
        if lower & mask != side || middle & mask != 0 {
            return;
        }
        score += weight.get(W::TsdHole) as i32;
        if native_row(board, y + 2) & mask == center {
            score += weight.get(W::TsdSpinable) as i32;
        }
        if y > 0 && native_row(board, y - 1) & (mask ^ center) == side {
            score += weight.get(W::TsdOffensive) as i32;
        }
        let mut clearable = 0;
        if lower | center == 0x03ff {
            clearable += 1;
        }
        if middle | mask == 0x03ff {
            clearable += 1;
        }
        score += clearable * weight.get(W::TsdClearable) as i32;
    };
    // Edge slots.
    for y in y0.max(h[1] + 1)..h[0].min(h[2]) + 1 {
        inspect(0, y, 0x380, 0x100, 0x280);
    }
    for y in y0.max(h[8] + 1)..h[7].min(h[9]) + 1 {
        inspect(7, y, 0x007, 0x002, 0x005);
    }
    // Interior slots, matching the six sliding masks in the release.
    for x in 1..=6 {
        let mask = 0x1c0u16 >> (x - 1);
        let side = 0x140u16 >> (x - 1);
        let center = 0x080u16 >> (x - 1);
        for y in y0.max(h[x + 1] + 1)..h[x].min(h[x + 2]) + 1 {
            inspect(x, y, mask, center, side);
        }
    }
    if score != 0 && t_piece_rank(hold, active, future) == 0 {
        score >>= 1;
    }
    score as f64
}

fn eval_tst_and_dt(
    board: &Board,
    heights: &[i32; WIDTH],
    _y0: i32,
    min_height: i32,
    hold: Option<Piece>,
    active: Option<Piece>,
    future: &[Piece],
    weight: &HoikoWeights,
) -> f64 {
    let h = native_heights(heights);
    let trank = t_piece_rank(hold, active, future);
    let mut score = 0i32;

    // DT-cannon signatures from SearchDTCannon, in the release's native
    // left-most-bit orientation. Both mirrored forms are checked.
    let left_y = h[8] - 4;
    if left_y >= 0
        && native_row(board, left_y) | 0x004 == 0x03ff
        && native_row(board, left_y + 1) | 0x004 == 0x03ff
        && native_row(board, left_y + 2) & 0x01f == 0x011
        && native_row(board, left_y + 3) & 0x00f == 0x009
        && native_row(board, left_y + 4) & 0x007 == 0x003
        && native_row(board, left_y + 5) & 0x007 == 0
    {
        score = weight.get(W::TstHole) as i32 + weight.get(W::TdHole) as i32;
    }
    let right_y = h[1] - 4;
    if score == 0
        && right_y >= 0
        && native_row(board, right_y) | 0x080 == 0x03ff
        && native_row(board, right_y + 1) | 0x080 == 0x03ff
        && native_row(board, right_y + 2) & 0x3e0 == 0x220
        && native_row(board, right_y + 3) & 0x3c0 == 0x240
        && native_row(board, right_y + 4) & 0x380 == 0x300
        && native_row(board, right_y + 5) & 0x380 == 0
    {
        score = weight.get(W::TstHole) as i32 + weight.get(W::TdHole) as i32;
    }

    // TST cavity signatures. These are the two mirrored three-row cores and
    // the release's overhang form at either wall.
    if score == 0 {
        for x in 0..=6 {
            let y = h[x + 2] - 2;
            if y < 0 {
                continue;
            }
            let core = ((native_row(board, y) >> (7 - x)) & 7) == 5;
            let left = native_row(board, y + 1) & (0x380 >> x) == (0x200 >> x)
                && native_row(board, y + 2) & (0x180 >> x) == (0x080 >> x);
            if core && left {
                score = weight.get(W::TstHole) as i32;
                if native_row(board, y + 3) & (0x1e0 >> x) == 0 {
                    score += weight.get(W::TstSpinable) as i32;
                }
                break;
            }
        }
    }
    if score == 0 && min_height + 2 < 40 {
        let row = native_row(board, min_height + 2);
        for x in 0..=6 {
            if ((row >> (6 - x)) & 0x0f) == 9 {
                score = weight.get(W::TdHint) as i32;
                break;
            }
        }
    }
    if score != 0 && trank == 0 {
        score >>= 1;
    }
    score as f64
}

fn eval_resource(
    board: &Board,
    heights: &[i32; WIDTH],
    garbage_height: i32,
    weight: &HoikoWeights,
    use_s4w: bool,
    combo: i32,
    opponent: Option<OpponentState>,
) -> f64 {
    let stack_height = heights.iter().copied().max().unwrap_or(-1);
    let mut anabara = 0i32;
    let mut resource = 0i32;
    let mut line = row_mask(board, 0);
    if garbage_height >= 0 {
        for y in 1..=garbage_height {
            let row = row_mask(board, y as u32);
            if (line | row) != line {
                anabara += 1;
            }
            line = row;
        }
    }
    for y in (garbage_height + 1).max(0)..=stack_height {
        if y < 0 {
            continue;
        }
        resource += row_mask(board, y as u32).count_ones() as i32;
        let max_resource = weight.get(W::ResourceMax).max(0.0) as i32;
        if resource >= max_resource {
            resource = max_resource;
            break;
        }
    }
    let mut score = 0.0;
    if garbage_height >= 0 {
        let row = row_mask(board, garbage_height as u32);
        if row.count_ones() == 9 {
            let column = (!row & 0x03ff).trailing_zeros() as usize;
            score += weight.column[column] as f64 * 0.5;
        }
    }
    if use_s4w {
        score += eval_s4w(board, garbage_height, stack_height, weight, combo, opponent);
    }
    score + anabara as f64 * weight.get(W::Anabara) + resource as f64 * weight.get(W::Resource)
}

fn eval_s4w(
    board: &Board,
    garbage_height: i32,
    stack_height: i32,
    weight: &HoikoWeights,
    combo: i32,
    opponent: Option<OpponentState>,
) -> f64 {
    if stack_height <= garbage_height {
        return 0.0;
    }
    let left = reverse10(0b11111_10000);
    let right = reverse10(0b00001_11111);
    let mut fill = 0u16;
    let mut wide = None;
    let mut h = 0i32;
    for y in (garbage_height + 1..=stack_height).rev() {
        let row = row_mask(board, y as u32);
        fill |= row;
        if row == left {
            if fill & (0x03ff ^ left) != 0 {
                return 0.0;
            }
            wide = Some(left);
            h = y;
            break;
        }
        if row == right {
            if fill & (0x03ff ^ right) != 0 {
                return 0.0;
            }
            wide = Some(right);
            h = y;
            break;
        }
    }
    let Some(wide) = wide else { return 0.0 };
    if stack_height + 1 > 13 {
        return 0.0;
    }
    let mut y = h - 1;
    let mut run = 1i32;
    while y > garbage_height && row_mask(board, y as u32) == wide {
        run += 1;
        y -= 1;
    }
    let mut seed = 0i32;
    while y > garbage_height {
        seed += (row_mask(board, y as u32) & (0x03ff ^ wide)).count_ones() as i32;
        y -= 1;
    }
    if seed < 2 {
        return 0.0;
    }
    if seed == 5 {
        run -= 1;
    }
    if run >= 4 && combo > 2 {
        run += combo;
    }
    let powerful = opponent
        .is_some_and(|state| opponent_is_powerful(&state.board, state.combo, state.pending));
    let mut score = 0i32;
    if garbage_height <= run && run >= 4 {
        if run >= 6 {
            if run >= 8 {
                if run >= 11 {
                    if run >= 12 {
                        score += weight.combo_table[4];
                    }
                    score += weight.combo_table[3];
                } else if powerful {
                    return 0.0;
                }
                score += weight.combo_table[2];
            } else if powerful {
                return 0.0;
            }
            score += weight.combo_table[1];
        } else if powerful {
            return 0.0;
        }
        score += weight.combo_table[0];
    }
    score as f64
}

fn reverse10(value: u16) -> u16 {
    value.reverse_bits() >> 6
}

fn board_popcount(board: &Board) -> u32 {
    board
        .cols
        .iter()
        .map(|column| (column & NATIVE_BOARD_MASK).count_ones())
        .sum()
}

fn source_pc_chance(board: &Board, piece: Piece) -> bool {
    let (heights, garbage_height) = source_column_heights(board);
    if garbage_height >= 0 || heights.iter().copied().max().unwrap_or(-1) > 3 {
        return false;
    }
    // The public `EvalPrePcBoard` spells out one-row templates for all seven
    // pieces.  A legal one-piece placement is the same predicate expressed
    // through the Rust SRS generator, and also handles the empty-board I case
    // without relying on the native coordinate orientation.
    one_piece_pc(board, piece)
}

fn source_pc_stack_score(board: &Board) -> i32 {
    let stack = source_column_heights(board)
        .0
        .into_iter()
        .max()
        .unwrap_or(-1);
    if stack > 3 {
        return 0;
    }
    let goal = if board_popcount(board) & 3 == 0 { 3 } else { 2 };
    if goal > stack {
        return 1;
    }
    let mut common = 0x03ffu16;
    for y in 0..=stack.max(0) {
        common &= row_mask(board, y as u32);
    }
    let mut segment = 0u16;
    for x in 0..WIDTH {
        let bit = 1u16 << x;
        if common & bit != 0 {
            if segment != 0 {
                let holes: u32 = (0..=goal)
                    .map(|y| ((!row_mask(board, y as u32)) & segment).count_ones())
                    .sum();
                if holes & 3 != 0 {
                    return 0;
                }
                segment = 0;
            }
        } else {
            segment |= bit;
        }
    }
    if segment != 0 {
        let holes: u32 = (0..=goal)
            .map(|y| ((!row_mask(board, y as u32)) & segment).count_ones())
            .sum();
        if holes & 3 != 0 {
            return 0;
        }
    }
    common.count_ones() as i32 + 1
}

fn opponent_is_pc(board: &Board) -> bool {
    let height = column_heights(board).into_iter().max().unwrap_or(0);
    height < 10 && board_popcount(board) % 2 == 0
}

fn opponent_is_powerful(board: &Board, combo: u8, garbage: usize) -> bool {
    if combo >= 5 || garbage >= 6 {
        return true;
    }
    let (heights, _) = source_column_heights(board);
    let stack = heights.iter().copied().max().unwrap_or(-1);
    let mut previous = 0u16;
    let mut bara = 0i32;
    let mut well = 0i32;
    let mut same = 0i32;
    for y in (0..=stack).rev() {
        let row = row_mask(board, y as u32);
        if (row | previous) != row {
            bara += 1;
            if same >= 4 {
                well += same;
            }
            same = 0;
        } else if row == previous {
            same += 1;
        }
        previous = row;
    }
    if same >= 4 {
        well += same;
    }
    if board_popcount(board) % 2 == 0 {
        !(stack + 1 >= 10 && bara >= 4 && well <= 6)
    } else {
        well > 6
    }
}

fn combo_rank(combo: i32) -> usize {
    match combo {
        0..=2 => 0,
        3..=4 => 0,
        5..=6 => 1,
        7..=8 => 2,
        9..=11 => 3,
        _ => 4,
    }
}

/// A small helper for callers that keep the original package under a home
/// directory.  It first tries the extracted package and otherwise uses the
/// reverse-engineered embedded profiles.
pub fn config_from_home() -> HoikoConfig {
    let candidates: [PathBuf; 3] = [
        PathBuf::from("/home/server/Hoiko_PPT_v0-beta1"),
        PathBuf::from("/home/server/hoiko"),
        PathBuf::from("assets/hoiko"),
    ];
    for path in candidates {
        if let Ok(config) = HoikoConfig::from_dir(path) {
            return config;
        }
    }
    HoikoConfig::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;
    use intetrigence_engine::data::{PieceLocation, Rotation};
    use std::collections::HashSet;

    fn test_node(combo: i32, b2b: bool) -> BeamNode {
        BeamNode {
            board: Board::default(),
            garbage: Board::default(),
            opponent: None,
            hold: None,
            active: Some(Piece::I),
            future: Vec::new(),
            bag: EnumSet::all(),
            combo,
            b2b: i32::from(b2b),
            profile: 0,
            score: 0.0,
            family_score: 0.0,
            pc_score: 0.0,
            first: None,
            first_commands: None,
        }
    }

    fn test_lock(lines: u32, combo: i32, b2b: bool, back_to_back: bool) -> LockResult {
        LockResult {
            board: Board::default(),
            garbage: Board::default(),
            lines,
            combo,
            b2b: i32::from(b2b),
            back_to_back,
            perfect_clear: false,
        }
    }

    fn i_placement() -> Placement {
        Placement {
            location: PieceLocation {
                piece: Piece::I,
                rotation: Rotation::North,
                x: 0,
                y: 0,
            },
            spin: Spin::None,
        }
    }

    #[test]
    fn parses_release_weight_rows() {
        let weights =
            HoikoWeights::parse_csv("maxHeight,-15\ncolumn,-1,2\ncomboAtk,3\nnexus,50\n").unwrap();
        assert_eq!(weights.values[W::MaxHeight as usize], -15);
        assert_eq!(weights.column[0], -1);
        assert_eq!(weights.combo_table, [3, 6, 9, 12, 15]);
        assert_eq!(weights.nexus, 0.5);
    }

    #[test]
    fn embedded_profile_is_usable() {
        let config = HoikoConfig::default();
        assert_eq!(config.beam_size, 50);
        assert_eq!(config.max_depth, 14);
        assert!(config.use_hold);
        assert_eq!(config.profiles[0].values[W::Quad as usize], 200);
    }

    #[test]
    fn empty_board_expansion_has_native_operation_costs() {
        let board = Board::default();
        let moves = hoiko_moves(&board, Piece::T, None);
        assert_eq!(moves.len(), 34);
        assert!(moves.iter().all(|candidate| {
            candidate.commands.raw_delay >= 116
                && candidate.commands.commands.last() == Some(&HoikoCommand::HardDrop)
        }));
        let center = moves
            .iter()
            .find(|candidate| {
                candidate.placement.location.rotation == Rotation::North
                    && candidate.placement.location.x == 4
            })
            .unwrap();
        assert_eq!(center.commands.raw_delay, 116);
        assert_eq!(center.commands.corrected_delay(), 116);
    }

    #[test]
    fn abstract_hold_delays_match_release_frame_correction() {
        let normal = CommandPath::default()
            .pushed(HoikoCommand::Hold, 16)
            .pushed(HoikoCommand::HardDrop, 116);
        let first = CommandPath::default()
            .pushed(HoikoCommand::FirstHold, 16)
            .pushed(HoikoCommand::HardDrop, 116);
        assert_eq!(normal.corrected_delay(), 133);
        assert_eq!(first.corrected_delay(), 250);
    }

    #[test]
    fn derived_expansion_only_returns_referee_legal_locations() {
        let mut board = Board::default();
        board.cols[0] = 0b1111;
        board.cols[1] = 0b1001;
        board.cols[2] = 0b1111;
        board.cols[5] = 0b11;
        board.cols[6] = 0b1;
        for piece in HOIKO_BAG_ORDER {
            let legal: HashSet<_> = find_moves(&board, piece)
                .into_iter()
                .map(|(placement, _)| placement)
                .collect();
            for candidate in hoiko_moves(&board, piece, None) {
                assert!(
                    legal.contains(&candidate.placement),
                    "{piece:?}: {:?}",
                    candidate.placement
                );
            }
        }
    }

    #[test]
    fn next_prediction_reorders_fixed_bag_without_hidden_rng() {
        let queue = [Piece::I, Piece::O, Piece::T, Piece::Z, Piece::S]
            .into_iter()
            .collect();
        let future = predict_future(&queue, 3, 10, true);
        assert_eq!(&future[..4], &[Piece::O, Piece::T, Piece::Z, Piece::S]);
        assert_eq!(future.len(), 10);
    }

    #[test]
    fn release_template_checkpoint_is_recognized() {
        let mut board = Board::default();
        for (y, native) in [1007u16, 1015, 135, 6].into_iter().enumerate() {
            let local = reverse10(native);
            for x in 0..WIDTH {
                if local & (1 << x) != 0 {
                    board.cols[x] |= 1u64 << y;
                }
            }
        }
        assert_eq!(opener_template_score(&board, Some(Piece::L)), 30_000);
    }

    #[test]
    fn chooses_legal_move_and_handles_empty_hold() {
        let game = Game::new_guideline(7);
        let (mv, stats) = HoikoSearcher::default().choose(
            &game,
            Budget {
                milliseconds: 0,
                iterations: 200,
            },
        );
        assert!(mv.is_some());
        assert!(game.legal(mv.unwrap()));
        assert!(stats.nodes > 0);
    }

    #[test]
    fn time_budget_does_not_cut_search_before_configured_minimum_depth() {
        let game = Game::new_guideline(11);
        let mut config = HoikoConfig::default();
        config.beam_size = 1;
        config.min_depth = 2;
        config.max_depth = 3;
        let (_, stats) = HoikoSearcher::new(config).choose(
            &game,
            Budget {
                milliseconds: 1,
                iterations: 0,
            },
        );
        assert!(stats.depth >= 2, "search stopped at depth {}", stats.depth);
    }

    #[test]
    fn final_action_uses_post_lock_b2b_state() {
        let weights = HoikoWeights::parse_csv("wasteI,-100\ncomboBtb,50\n").unwrap();
        let searcher = HoikoSearcher::default();

        let first_tetris = searcher.evaluate(
            &test_node(0, false),
            &test_lock(4, 1, true, false),
            i_placement(),
            0,
            None,
            None,
            &weights,
        );
        assert_eq!(first_tetris.action_score, 0.0);

        let continued = searcher.evaluate(
            &test_node(0, true),
            &test_lock(4, 1, true, true),
            i_placement(),
            0,
            None,
            None,
            &weights,
        );
        assert_eq!(continued.action_score, 50.0);

        let broken = searcher.evaluate(
            &test_node(0, true),
            &test_lock(2, 1, false, false),
            i_placement(),
            0,
            None,
            None,
            &weights,
        );
        assert_eq!(broken.action_score, -100.0);
    }

    #[test]
    fn native_combo_debt_survives_one_no_clear_as_negative_combo() {
        let mut placement = i_placement();
        placement.location.x = 1;
        let result = lock(&Board::default(), &Board::default(), placement, 4, 0).unwrap();
        assert_eq!(result.lines, 0);
        assert_eq!(result.combo, -4);

        let weights = HoikoWeights::parse_csv("comboDebt,250\n").unwrap();
        let evaluation = HoikoSearcher::default().evaluate(
            &test_node(4, false),
            &result,
            placement,
            116,
            None,
            None,
            &weights,
        );
        assert_eq!(evaluation.action_score, -1000.0);
    }

    #[test]
    fn source_garbage_marker_and_well_geometry_are_bottom_up() {
        let mut garbage = Board::default();
        let mut board = Board::default();
        for x in 0..WIDTH {
            if x != 4 {
                garbage.cols[x] |= 1;
                board.cols[x] |= 1;
            }
        }
        assert_eq!(marker_garbage_height(&garbage), Some(0));
        let (column, depth, hint) = well(&board, -1);
        assert_eq!(column, Some(4));
        assert_eq!(depth, 1);
        assert!(hint >= 1);
    }

    #[test]
    fn opponent_snapshot_changes_profile_transition() {
        let searcher = HoikoSearcher::default();
        let board = Board::default();
        let opponent = OpponentState {
            board,
            active: None,
            reserve: None,
            hold: None,
            queue: [None; 6],
            combo: 9,
            b2b: false,
            pending: 0,
            multi_evaluation: true,
        };
        assert_eq!(
            searcher.profile_index(&board, &Board::default(), 0, None),
            0
        );
        let mut raised = board;
        for x in 0..WIDTH {
            if x != 4 {
                raised.cols[x] = (1u64 << 13) - 1;
            }
        }
        assert_eq!(
            searcher.profile_index(&raised, &Board::default(), 0, Some(opponent)),
            3
        );
    }

    #[test]
    fn defensive_profile_keeps_the_source_height_hysteresis() {
        let searcher = HoikoSearcher::default();
        let garbage = Board::default();
        let mut board = Board::default();

        board.cols[3] = (1u64 << 15) - 1;
        assert_eq!(searcher.profile_index(&board, &garbage, 0, None), 0);

        board.cols[3] = (1u64 << 16) - 1;
        let defensive = searcher.profile_index(&board, &garbage, 0, None);
        assert_eq!(defensive, 2);
        searcher.profile.set(defensive);

        board.cols[3] = (1u64 << 14) - 1;
        assert_eq!(searcher.profile_index(&board, &garbage, 0, None), 2);

        board.cols[3] = (1u64 << 13) - 1;
        assert_eq!(searcher.profile_index(&board, &garbage, 0, None), 0);
    }

    #[test]
    fn native_board_projection_ignores_rows_above_31() {
        let mut board = Board::default();
        board.cols[0] = 1 | (1u64 << 40);
        assert_eq!(column_heights(&board)[0], 1);
        assert_eq!(board_popcount(&board), 1);
        assert_eq!(row_mask(&board, 40), 0);
    }

    #[test]
    fn native_pc_stack_scores_only_low_four_row_structures() {
        assert_eq!(source_pc_stack_score(&Board::default()), 1);
        let mut high = Board::default();
        high.cols[0] = 1 << 4;
        assert_eq!(source_pc_stack_score(&high), 0);
    }

    #[test]
    fn play_style_selects_the_release_solo_weights() {
        let mut config = HoikoConfig::default();
        config.play_style = 1;
        let ultra = HoikoSearcher::new(config);
        assert_eq!(ultra.weights(0).values[W::Quad as usize], 10_000);

        let mut config = HoikoConfig::default();
        config.play_style = 2;
        let sprint = HoikoSearcher::new(config);
        assert_eq!(sprint.weights(0).values[W::Single as usize], -9_999);
    }

    #[test]
    fn thread_count_applies_native_beam_scaling() {
        let single = HoikoConfig::default();
        let mut parallel = single.clone();
        parallel.thread_count = 2;
        assert!(corrected_beam_size(&parallel) > corrected_beam_size(&single));
    }
}
