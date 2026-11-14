//! Match harness: fixed seeds × both sheets, replay verification and summary.
//!
//! M0 acceptance (PLAN.md §5): ten games against attax with fixed seeds, both
//! sheets, every game replay-verified, and an audit log per game. The opponent
//! on our side is configurable: the M0 baseline stand-in (`crate::baseline`) or
//! the real search (`crate::bot_agent`). The summary records which one ran.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use intetrigence_bot::SearchConfig;
use intetrigence_tbp::HostConfig;
use serde::{Deserialize, Serialize};

use crate::agent::Agent;
use crate::audit::{GameAudit, wilson_interval};
use crate::baseline::BaselineBot;
use crate::engine::{Profile, Sheet};
use crate::referee::{RoundConfig, run_round};
use crate::replay::{VerifyReport, verify_replay, write_replay};
use crate::tbp_agent::{TbpAgent, TbpAgentConfig};

#[derive(Clone, Debug)]
pub struct AttaxConfig {
    pub dir: PathBuf,
    pub bun: String,
    pub commit: Option<String>,
    pub bun_version: Option<String>,
    /// attax's TBP grid rows. `0` auto-detects from
    /// `<dir>/src/ai/bot/movegen.ts` (`ROWS = N`); upstream `7b38f51` is 20,
    /// the vision patch sets 40.
    pub visible_rows: u32,
    /// SHA-256 of the compatibility patch applied to attax (if any).
    pub patch_sha256: Option<String>,
    /// Attax's movegen has no 180-degree rotation; the referee must not
    /// treat a 180-only placement as a legal move for it.
    pub supports_180: bool,
    /// Extra environment variables for the attax process (for example
    /// `ATTAX_TBP_LOG` or `ATTAX_TBP_BOARD_LOG`).
    pub env: Vec<(String, String)>,
    pub timeout: Duration,
}

impl AttaxConfig {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            bun: "bun".into(),
            commit: None,
            bun_version: None,
            visible_rows: 0,
            patch_sha256: None,
            supports_180: false,
            env: Vec::new(),
            timeout: Duration::from_secs(30),
        }
    }

    pub fn entry_point(&self) -> PathBuf {
        self.dir.join("tbp").join("attax-tbp.ts")
    }
}

/// A generic TBP opponent (CC2, clys through a TBP adapter, ...).
#[derive(Clone, Debug)]
pub struct TbpOpponentConfig {
    /// Display name recorded in the replay header and audit.
    pub name: String,
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    pub commit: Option<String>,
    /// Rows the bot can see (20 for attax, 40 for full-board bots).
    pub visible_rows: u32,
    /// Whether the bot's input set includes 180-degree rotations.
    pub supports_180: bool,
    /// Delay before each `suggest` (budget for continuously-searching bots).
    pub suggest_delay_ms: u64,
    /// Retry an empty `suggest` for this long (CC2 race).
    pub retry_empty_suggest_ms: u64,
    pub timeout: Duration,
}

impl TbpOpponentConfig {
    pub fn new(name: impl Into<String>, program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            name: name.into(),
            program: program.into(),
            args,
            cwd: None,
            env: Vec::new(),
            commit: None,
            visible_rows: 40,
            supports_180: true,
            suggest_delay_ms: 0,
            retry_empty_suggest_ms: 0,
            timeout: Duration::from_secs(30),
        }
    }
}

/// Which bot plays slot 0 (intetrigence).
#[derive(Clone, Debug)]
pub enum BotChoice {
    /// The M0 in-process stand-in (plumbing tests).
    Baseline,
    /// The real anytime search from `crates/bot`.
    Search(Box<SearchConfig>),
}

#[derive(Clone, Debug)]
pub struct MatchConfig {
    pub seeds: Vec<u64>,
    pub sheets: Vec<Sheet>,
    pub turn_cap: u32,
    pub frames_per_turn: u64,
    pub out_dir: PathBuf,
    pub attax: Option<AttaxConfig>,
    /// Generic TBP opponent (takes precedence over `attax` when set).
    pub tbp_opponent: Option<TbpOpponentConfig>,
    pub started_at: String,
    /// Provenance string recorded in every replay header.
    pub rules: String,
    pub protocol: String,
    /// Slot 0 bot.
    /// Slot 0 bot.
    pub bot: BotChoice,
    /// Match profile (protocol v1.11 §1.3).
    pub profile: Profile,
    /// Run both slots with the in-process search (self-play diagnostic).
    pub self_play: bool,
    /// Reuse already verified `(seed, sheet)` games in `out_dir` (protocol
    /// v1.18 §1.4). Only valid with the same build and seeds.
    pub resume: bool,
    /// Notes written into the summary. Set per match so the M0 plumbing run and
    /// the M2 real-bot run cannot inherit each other's wording.
    pub notes: Vec<String>,
    /// Weight overrides applied to the search, recorded in the replay header.
    pub weights_note: Vec<String>,
}

impl MatchConfig {
    pub fn new(out_dir: impl Into<PathBuf>) -> Self {
        Self {
            seeds: vec![1001, 1002, 1003, 1004, 1005],
            sheets: Sheet::ALL.to_vec(),
            turn_cap: 1000,
            frames_per_turn: 12,
            out_dir: out_dir.into(),
            attax: None,
            tbp_opponent: None,
            started_at: now_stamp(),
            rules: version_string("docs/s2-spec.md", "v1.3.11"),
            protocol: version_string("docs/s2-referee-protocol.md", "v1.9"),
            bot: BotChoice::Baseline,
            profile: Profile::Official,
            self_play: false,
            resume: false,
            notes: vec!["The intetrigence side is the bot recorded in each replay header.".into()],
            weights_note: Vec::new(),
        }
    }
}

/// Parses `key=value[,key=value]` weight overrides.
pub fn parse_weight_overrides(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut overrides = Vec::new();
    for part in text.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (key, value) = part
            .split_once('=')
            .ok_or_else(|| format!("weight override `{part}` needs key=value"))?;
        overrides.push((key.trim().to_string(), value.trim().to_string()));
    }
    Ok(overrides)
}

/// Applies weight overrides to `Weights`. Unknown keys are rejected; the
/// defaults are untouched when the list is empty (bit-identical behaviour).
pub fn apply_weight_overrides(
    weights: &mut intetrigence_bot::Weights,
    overrides: &[(String, String)],
) -> Result<(), String> {
    for (key, value) in overrides {
        let number = |value: &str| -> Result<f64, String> {
            value
                .parse::<f64>()
                .map_err(|error| format!("weight `{key}`: {error}"))
        };
        match key.as_str() {
            "max_height" => weights.max_height = number(value)?,
            "height_sq" => weights.height_sq = number(value)?,
            "holes" => weights.holes = number(value)?,
            "hole_cover" => weights.hole_cover = number(value)?,
            "hole_rows" => weights.hole_rows = number(value)?,
            "bumpiness" => weights.bumpiness = number(value)?,
            "bumpiness_sq" => weights.bumpiness_sq = number(value)?,
            "row_transitions" => weights.row_transitions = number(value)?,
            "well_depth" => weights.well_depth = number(value)?,
            "extra_wells" => weights.extra_wells = number(value)?,
            "danger_projected" => weights.danger_projected = number(value)?,
            "garbage_rows" => weights.garbage_rows = number(value)?,
            "attack" => weights.attack = number(value)?,
            "perfect_clear" => weights.perfect_clear = number(value)?,
            "b2b_break" => weights.b2b_break = number(value)?,
            "wasted_t" => weights.wasted_t = number(value)?,
            "surge_charge" => weights.surge_charge = number(value)?,
            "combo" => weights.combo = number(value)?,
            "opener_attack" => weights.opener_attack = number(value)?,
            "garbage_clear" => weights.garbage_clear = number(value)?,
            "incoming" => weights.incoming = number(value)?,
            "softdrop" => weights.softdrop = number(value)?,
            "discount" => weights.discount = number(value)?,
            "surge_exponent" => weights.surge_exponent = number(value)?,
            "surge_hold_penalty" => weights.surge_hold_penalty = number(value)?,
            "survival_line" => weights.survival_line = number(value)?,
            "survival" => {
                weights.survival = match value.as_str() {
                    "1" | "true" | "yes" => true,
                    "0" | "false" | "no" => false,
                    other => return Err(format!("weight `survival`: bad bool `{other}`")),
                }
            }
            other => return Err(format!("unknown weight `{other}`")),
        }
    }
    Ok(())
}

/// Reads attax's board rows from its movegen source. Falls back to 20 (the
/// upstream `7b38f51` value) when the file is missing or unparseable.
pub fn detect_attax_visible_rows(dir: &Path) -> u32 {
    let path = dir.join("src").join("ai").join("bot").join("movegen.ts");
    if let Ok(text) = std::fs::read_to_string(path) {
        for line in text.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("export const ROWS =") {
                let value = rest.trim().trim_end_matches(';').trim();
                if let Ok(rows) = value.parse::<u32>() {
                    return rows;
                }
            }
        }
    }
    20
}

/// Repository root (the arena crate lives at `<root>/crates/arena`).
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn git_output(root: &Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// `docs/...@vX.Y.Z+<short head>[-dirty]`.
///
/// The commit suffix is the snapshot the run used, so
/// `git show <commit>:<path>` restores the exact document. `-dirty` marks an
/// uncommitted edit of that document.
pub fn version_string(path: &str, version: &str) -> String {
    let root = repo_root();
    let head = git_output(&root, &["rev-parse", "--short", "HEAD"]);
    let dirty = git_output(&root, &["status", "--porcelain", "--", path])
        .map(|status| !status.is_empty())
        .unwrap_or(false);
    match head {
        Some(head) if !head.is_empty() => {
            let suffix = if dirty { "-dirty" } else { "" };
            format!("{path}@{version}+{head}{suffix}")
        }
        _ => format!("{path}@{version}"),
    }
}

/// Short HEAD of this checkout, recorded as the bot's commit.
pub fn git_short_head() -> Option<String> {
    git_output(&repo_root(), &["rev-parse", "--short", "HEAD"])
}

pub fn now_stamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("unix:{seconds}")
}

/// Per-game search statistics (protocol v1.31 §8.4.1).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SearchGameSummary {
    pub turns: u32,
    pub fallback_turns: u32,
    #[serde(default)]
    pub depth_median: Option<f64>,
    #[serde(default)]
    pub depth_p95: Option<f64>,
    #[serde(default)]
    pub depth_max: Option<u32>,
    #[serde(default)]
    pub width_median: Option<f64>,
    #[serde(default)]
    pub width_p95: Option<f64>,
    #[serde(default)]
    pub width_max: Option<u32>,
    #[serde(default)]
    pub nodes_median: Option<f64>,
    #[serde(default)]
    pub nodes_p95: Option<f64>,
    #[serde(default)]
    pub nodes_max: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameSummary {
    pub seed: u64,
    pub sheet: String,
    pub winner: Option<u8>,
    pub reason: String,
    /// `vision` | `other` | `unknown` for Forfeit results.
    pub forfeit_cause: Option<String>,
    pub turns: u32,
    #[serde(default)]
    pub surge_released: [u32; 2],
    #[serde(default)]
    pub max_b2b_streak: [u32; 2],
    #[serde(default)]
    pub turns_with_b2b_ge_4: [u32; 2],
    pub verified: bool,
    pub verify_mismatches: usize,
    pub think_median_ms: [f64; 2],
    pub think_p95_ms: [f64; 2],
    #[serde(default)]
    pub think_p99_ms: [f64; 2],
    #[serde(default)]
    pub think_max_ms: [f64; 2],
    pub first_choice_rate: [f64; 2],
    pub spin_match_rate: [f64; 2],
    pub attack_match_rate: [f64; 2],
    pub attack_per_piece: [f64; 2],
    pub spin_mismatches: [u32; 2],
    pub attack_mismatches_same_spin: [u32; 2],
    pub think_over_budget: [u32; 2],
    pub sent_total: [u32; 2],
    pub received_total: [u32; 2],
    pub attack_total: [u32; 2],
    pub pieces_placed: [u32; 2],
    pub start_resends: usize,
    pub cells_above_row20: bool,
    pub queue_invariant_violations: u64,
    pub replay: String,
    /// Per-slot search statistics (protocol v1.31 §8.4.1).
    #[serde(default)]
    pub search: [SearchGameSummary; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttaxSummary {
    pub info_name: String,
    pub info_version: String,
    pub commit: Option<String>,
    pub bun_version: Option<String>,
    pub command_line: String,
}

/// Match-level audit totals for one bot. The final report must separate spin
/// label mismatches from same-spin attack mismatches (protocol v1.3 §8.4).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BotAggregate {
    pub slot: u8,
    pub name: String,
    pub placements: u32,
    pub spin_checked: u32,
    pub spin_mismatches: u32,
    pub attack_checked: u32,
    pub attack_mismatches: u32,
    pub attack_mismatches_same_spin: u32,
    pub think_over_budget: u32,
    pub attack_per_piece: f64,
    pub received_per_game: f64,
    pub visible_rows: u32,
    /// Mean over games of the per-game mean board height.
    pub mean_height: f64,
    /// Mean over games of the per-game max board height.
    pub max_height: f64,
    /// Search turns with a reported depth/width (protocol v1.31 §8.4.1).
    #[serde(default)]
    pub search_turns: u32,
    #[serde(default)]
    pub fallback_turns: u32,
    #[serde(default)]
    pub fallback_rate: f64,
    #[serde(default)]
    pub depth_median: Option<f64>,
    #[serde(default)]
    pub depth_p95: Option<f64>,
    #[serde(default)]
    pub depth_max: Option<u32>,
    #[serde(default)]
    pub width_median: Option<f64>,
    #[serde(default)]
    pub width_p95: Option<f64>,
    #[serde(default)]
    pub width_max: Option<u32>,
    #[serde(default)]
    pub nodes_median: Option<f64>,
    #[serde(default)]
    pub nodes_p95: Option<f64>,
    #[serde(default)]
    pub nodes_max: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchSummary {
    pub schema: String,
    pub started_at: String,
    /// Match profile (protocol v1.11 §1.3); only `official` is usable for
    /// final claims.
    pub profile: String,
    pub games: Vec<GameSummary>,
    pub wins: u32,
    pub losses: u32,
    /// TurnCap games (no winner decided).
    #[serde(default)]
    pub unresolved: u32,
    /// Forfeit games (no contest; no winner recorded).
    #[serde(default)]
    pub no_contest: u32,
    /// Stuck top-outs (no legal placement, not block-out).
    #[serde(default)]
    pub stuck: u32,
    pub forfeits: u32,
    /// Win/loss breakdown (protocol v1.8 §7.1).
    pub wins_topout: u32,
    #[serde(default)]
    pub wins_turncap: u32,
    #[serde(default)]
    pub losses_turncap: u32,
    pub wins_forfeit_vision: u32,
    pub wins_forfeit_other: u32,
    #[serde(default)]
    pub forfeit_vision: u32,
    #[serde(default)]
    pub forfeit_other: u32,
    #[serde(default)]
    pub forfeit_unknown: u32,
    pub losses_topout: u32,
    pub losses_forfeit: u32,
    pub turncap: u32,
    #[serde(default)]
    pub surge_released: [u32; 2],
    #[serde(default)]
    pub max_b2b_streak: [u32; 2],
    #[serde(default)]
    pub turns_with_b2b_ge_4: [u32; 2],
    pub win_rate: f64,
    pub wilson_low: f64,
    pub wilson_high: f64,
    pub verified: u32,
    pub verify_failures: u32,
    /// Games reused from a previous interrupted run (protocol v1.18 §1.4).
    #[serde(default)]
    pub resumed_games: u32,
    /// Every game has replay + audit + opponent decision log (v1.21 §8.3.1).
    #[serde(default)]
    pub measurement_complete: bool,
    /// SHA-256 of the attax compatibility patch applied for this match.
    #[serde(default)]
    pub attax_patch_sha256: Option<String>,
    pub attax: Option<AttaxSummary>,
    pub bots: [BotAggregate; 2],
    pub notes: Vec<String>,
}

fn ratio(numerator: u32, denominator: u32) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        (f64::from(numerator) / f64::from(denominator) * 1000.0).round() / 1000.0
    }
}

fn rate(matches: u32, checked: u32) -> f64 {
    if checked == 0 {
        0.0
    } else {
        (matches as f64 / checked as f64 * 1000.0).round() / 1000.0
    }
}

fn spin_rate(audit: &GameAudit, slot: usize) -> f64 {
    let scoring = &audit.scoring[slot];
    rate(scoring.spin_matches, scoring.spin_checked)
}

fn attack_rate(audit: &GameAudit, slot: usize) -> f64 {
    let scoring = &audit.scoring[slot];
    rate(scoring.attack_matches, scoring.attack_checked)
}

fn percentile(values: &[f64], p: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let index = ((sorted.len() - 1) as f64 * p / 100.0).round() as usize;
    Some(sorted[index])
}

fn median(values: &[f64]) -> Option<f64> {
    percentile(values, 50.0)
}

fn max_f64(values: &[f64]) -> Option<f64> {
    values.iter().cloned().fold(None, |acc, value| {
        Some(acc.map_or(value, |a: f64| a.max(value)))
    })
}

fn search_game_summary(turns: &[crate::audit::SearchTurn]) -> SearchGameSummary {
    let depths: Vec<f64> = turns
        .iter()
        .filter_map(|turn| turn.depth.map(f64::from))
        .collect();
    let widths: Vec<f64> = turns
        .iter()
        .filter_map(|turn| turn.width.map(f64::from))
        .collect();
    let nodes: Vec<f64> = turns
        .iter()
        .filter_map(|turn| turn.nodes.map(|nodes| nodes as f64))
        .collect();
    SearchGameSummary {
        turns: turns.len() as u32,
        fallback_turns: turns.iter().filter(|turn| turn.fallback).count() as u32,
        depth_median: median(&depths),
        depth_p95: percentile(&depths, 95.0),
        depth_max: max_f64(&depths).map(|value| value as u32),
        width_median: median(&widths),
        width_p95: percentile(&widths, 95.0),
        width_max: max_f64(&widths).map(|value| value as u32),
        nodes_median: median(&nodes),
        nodes_p95: percentile(&nodes, 95.0),
        nodes_max: max_f64(&nodes).map(|value| value as u64),
    }
}

struct SearchAggregate {
    turns: u32,
    fallback_turns: u32,
    fallback_rate: f64,
    depth_median: Option<f64>,
    depth_p95: Option<f64>,
    depth_max: Option<u32>,
    width_median: Option<f64>,
    width_p95: Option<f64>,
    width_max: Option<u32>,
    nodes_median: Option<f64>,
    nodes_p95: Option<f64>,
    nodes_max: Option<u64>,
}

fn search_aggregate(audits: &[GameAudit], slot: usize) -> SearchAggregate {
    let turns: Vec<&crate::audit::SearchTurn> = audits
        .iter()
        .flat_map(|audit| audit.search[slot].iter())
        .collect();
    let depths: Vec<f64> = turns
        .iter()
        .filter_map(|turn| turn.depth.map(f64::from))
        .collect();
    let widths: Vec<f64> = turns
        .iter()
        .filter_map(|turn| turn.width.map(f64::from))
        .collect();
    let nodes: Vec<f64> = turns
        .iter()
        .filter_map(|turn| turn.nodes.map(|nodes| nodes as f64))
        .collect();
    let fallback_turns = turns.iter().filter(|turn| turn.fallback).count() as u32;
    SearchAggregate {
        turns: turns.len() as u32,
        fallback_turns,
        fallback_rate: ratio(fallback_turns, turns.len() as u32),
        depth_median: median(&depths),
        depth_p95: percentile(&depths, 95.0),
        depth_max: max_f64(&depths).map(|value| value as u32),
        width_median: median(&widths),
        width_p95: percentile(&widths, 95.0),
        width_max: max_f64(&widths).map(|value| value as u32),
        nodes_median: median(&nodes),
        nodes_p95: percentile(&nodes, 95.0),
        nodes_max: max_f64(&nodes).map(|value| value as u64),
    }
}

fn game_summary(audit: &GameAudit) -> GameSummary {
    GameSummary {
        seed: audit.seed,
        sheet: audit.sheet.clone(),
        winner: audit.winner,
        reason: audit.reason.clone(),
        forfeit_cause: audit.forfeit_cause.map(|cause| cause.as_str().to_string()),
        turns: audit.turns,
        surge_released: audit.surge_released,
        max_b2b_streak: audit.max_b2b_streak,
        turns_with_b2b_ge_4: audit.turns_with_b2b_ge_4,
        verified: audit.verify.ok,
        verify_mismatches: audit.verify.mismatches.len(),
        think_median_ms: [audit.think[0].median_ms, audit.think[1].median_ms],
        think_p95_ms: [audit.think[0].p95_ms, audit.think[1].p95_ms],
        think_p99_ms: [audit.think[0].p99_ms, audit.think[1].p99_ms],
        think_max_ms: [audit.think[0].max_ms, audit.think[1].max_ms],
        first_choice_rate: [audit.first_choice[0].rate, audit.first_choice[1].rate],
        spin_match_rate: [spin_rate(audit, 0), spin_rate(audit, 1)],
        attack_match_rate: [attack_rate(audit, 0), attack_rate(audit, 1)],
        attack_per_piece: [
            ratio(audit.totals[0].attack_total, audit.totals[0].pieces_placed),
            ratio(audit.totals[1].attack_total, audit.totals[1].pieces_placed),
        ],
        spin_mismatches: [
            audit.scoring[0].spin_mismatches,
            audit.scoring[1].spin_mismatches,
        ],
        attack_mismatches_same_spin: [
            audit.scoring[0].attack_mismatches_same_spin,
            audit.scoring[1].attack_mismatches_same_spin,
        ],
        think_over_budget: [audit.think[0].over_budget, audit.think[1].over_budget],
        sent_total: [audit.totals[0].sent_total, audit.totals[1].sent_total],
        received_total: [
            audit.totals[0].received_total,
            audit.totals[1].received_total,
        ],
        attack_total: [audit.totals[0].attack_total, audit.totals[1].attack_total],
        pieces_placed: [audit.totals[0].pieces_placed, audit.totals[1].pieces_placed],
        start_resends: audit.start_resends.len(),
        cells_above_row20: audit
            .attax
            .as_ref()
            .map(|attax| attax.cells_above_row20)
            .unwrap_or(false),
        queue_invariant_violations: audit.queue_invariant_violations,
        replay: audit.replay.clone(),
        search: [
            search_game_summary(&audit.search[0]),
            search_game_summary(&audit.search[1]),
        ],
    }
}

/// Aggregates audit records into the match summary (used by `arena report`).
pub fn summarize(audits: &[GameAudit], notes: Vec<String>) -> MatchSummary {
    let games: Vec<GameSummary> = audits.iter().map(game_summary).collect();
    let wins = games.iter().filter(|game| game.winner == Some(0)).count() as u32;
    let losses = games.iter().filter(|game| game.winner == Some(1)).count() as u32;
    let unresolved = games.iter().filter(|game| game.reason == "TurnCap").count() as u32;
    let no_contest = games.iter().filter(|game| game.reason == "Forfeit").count() as u32;
    let stuck = games.iter().filter(|game| game.reason == "Stuck").count() as u32;
    let forfeits = games.iter().filter(|game| game.reason == "Forfeit").count() as u32;
    let wins_topout = games
        .iter()
        .filter(|game| game.winner == Some(0) && game.reason == "TopOut")
        .count() as u32;
    let wins_turncap = games
        .iter()
        .filter(|game| game.winner == Some(0) && game.reason == "TurnCap")
        .count() as u32;
    let losses_turncap = games
        .iter()
        .filter(|game| game.winner == Some(1) && game.reason == "TurnCap")
        .count() as u32;
    let wins_forfeit_vision = games
        .iter()
        .filter(|game| {
            game.winner == Some(0)
                && game.reason == "Forfeit"
                && game.forfeit_cause.as_deref() == Some("vision")
        })
        .count() as u32;
    let wins_forfeit_other = games
        .iter()
        .filter(|game| {
            game.winner == Some(0)
                && game.reason == "Forfeit"
                && game.forfeit_cause.as_deref() == Some("other")
        })
        .count() as u32;
    let forfeit_vision = games
        .iter()
        .filter(|game| game.reason == "Forfeit" && game.forfeit_cause.as_deref() == Some("vision"))
        .count() as u32;
    let forfeit_other = games
        .iter()
        .filter(|game| game.reason == "Forfeit" && game.forfeit_cause.as_deref() == Some("other"))
        .count() as u32;
    let forfeit_unknown = games
        .iter()
        .filter(|game| {
            game.reason == "Forfeit"
                && !matches!(
                    game.forfeit_cause.as_deref(),
                    Some("vision") | Some("other")
                )
        })
        .count() as u32;
    let losses_topout = games
        .iter()
        .filter(|game| game.winner == Some(1) && game.reason == "TopOut")
        .count() as u32;
    let losses_forfeit = games
        .iter()
        .filter(|game| game.winner == Some(1) && game.reason == "Forfeit")
        .count() as u32;
    let turncap = games.iter().filter(|game| game.reason == "TurnCap").count() as u32;
    let verified = games.iter().filter(|game| game.verified).count() as u32;
    let (wilson_low, wilson_high) = wilson_interval(wins, games.len() as u32, 1.959_964);
    let attax = audits
        .iter()
        .find_map(|audit| audit.attax.as_ref())
        .map(|attax| AttaxSummary {
            info_name: attax.info_name.clone(),
            info_version: attax.info_version.clone(),
            commit: attax.commit.clone(),
            bun_version: attax.bun_version.clone(),
            command_line: attax.command_line.clone(),
        });
    let bots = [0usize, 1].map(|slot| {
        let search = search_aggregate(audits, slot);
        BotAggregate {
            slot: slot as u8,
            name: audits
                .first()
                .map(|audit| audit.bots[slot].name.clone())
                .unwrap_or_default(),
            placements: audits
                .iter()
                .map(|audit| audit.scoring[slot].placements)
                .sum(),
            spin_checked: audits
                .iter()
                .map(|audit| audit.scoring[slot].spin_checked)
                .sum(),
            spin_mismatches: audits
                .iter()
                .map(|audit| audit.scoring[slot].spin_mismatches)
                .sum(),
            attack_checked: audits
                .iter()
                .map(|audit| audit.scoring[slot].attack_checked)
                .sum(),
            attack_mismatches: audits
                .iter()
                .map(|audit| audit.scoring[slot].attack_mismatches)
                .sum(),
            attack_mismatches_same_spin: audits
                .iter()
                .map(|audit| audit.scoring[slot].attack_mismatches_same_spin)
                .sum(),
            think_over_budget: audits
                .iter()
                .map(|audit| audit.think[slot].over_budget)
                .sum(),
            attack_per_piece: ratio(
                audits
                    .iter()
                    .map(|audit| audit.totals[slot].attack_total)
                    .sum(),
                audits
                    .iter()
                    .map(|audit| audit.totals[slot].pieces_placed)
                    .sum(),
            ),
            received_per_game: if audits.is_empty() {
                0.0
            } else {
                (audits
                    .iter()
                    .map(|audit| f64::from(audit.totals[slot].received_total))
                    .sum::<f64>()
                    / audits.len() as f64
                    * 1000.0)
                    .round()
                    / 1000.0
            },
            visible_rows: audits
                .first()
                .map(|audit| audit.bots[slot].visible_rows)
                .unwrap_or(0),
            mean_height: if audits.is_empty() {
                0.0
            } else {
                (audits
                    .iter()
                    .map(|audit| audit.heights[slot].mean)
                    .sum::<f64>()
                    / audits.len() as f64
                    * 100.0)
                    .round()
                    / 100.0
            },
            max_height: if audits.is_empty() {
                0.0
            } else {
                (audits
                    .iter()
                    .map(|audit| f64::from(audit.heights[slot].max))
                    .sum::<f64>()
                    / audits.len() as f64
                    * 100.0)
                    .round()
                    / 100.0
            },
            search_turns: search.turns,
            fallback_turns: search.fallback_turns,
            fallback_rate: search.fallback_rate,
            depth_median: search.depth_median,
            depth_p95: search.depth_p95,
            depth_max: search.depth_max,
            width_median: search.width_median,
            width_p95: search.width_p95,
            width_max: search.width_max,
            nodes_median: search.nodes_median,
            nodes_p95: search.nodes_p95,
            nodes_max: search.nodes_max,
        }
    });
    MatchSummary {
        schema: "intetrigence/s2-match-summary/1".into(),
        profile: audits
            .first()
            .map(|audit| audit.profile.clone())
            .unwrap_or_else(|| "official".into()),
        started_at: audits
            .first()
            .map(|audit| audit.started_at.clone())
            .unwrap_or_else(now_stamp),
        games: games.clone(),
        wins,
        losses,
        unresolved,
        no_contest,
        stuck,
        forfeits,
        wins_topout,
        wins_turncap,
        losses_turncap,
        wins_forfeit_vision,
        wins_forfeit_other,
        forfeit_vision,
        forfeit_other,
        forfeit_unknown,
        losses_topout,
        losses_forfeit,
        turncap,
        surge_released: [
            games.iter().map(|game| game.surge_released[0]).sum(),
            games.iter().map(|game| game.surge_released[1]).sum(),
        ],
        max_b2b_streak: [
            games
                .iter()
                .map(|game| game.max_b2b_streak[0])
                .max()
                .unwrap_or(0),
            games
                .iter()
                .map(|game| game.max_b2b_streak[1])
                .max()
                .unwrap_or(0),
        ],
        turns_with_b2b_ge_4: [
            games.iter().map(|game| game.turns_with_b2b_ge_4[0]).sum(),
            games.iter().map(|game| game.turns_with_b2b_ge_4[1]).sum(),
        ],
        win_rate: rate(wins, games.len() as u32),
        wilson_low: (wilson_low * 10000.0).round() / 10000.0,
        wilson_high: (wilson_high * 10000.0).round() / 10000.0,
        verified,
        verify_failures: games.len() as u32 - verified,
        resumed_games: 0,
        measurement_complete: false,
        attax_patch_sha256: None,
        attax,
        bots,
        notes,
    }
}

impl MatchSummary {
    pub fn write(&self, out_dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(out_dir)?;
        let json = serde_json::to_string_pretty(self)
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        std::fs::write(out_dir.join("summary.json"), json)?;
        std::fs::write(out_dir.join("summary.md"), self.to_markdown())?;
        Ok(())
    }

    pub fn to_markdown(&self) -> String {
        let mut text = String::new();
        text.push_str("# S2 match summary\n\n");
        text.push_str(&format!("- games: {}\n", self.games.len()));
        text.push_str(&format!(
            "- intetrigence wins / losses / unresolved / no-contest: {} / {} / {} / {}\n",
            self.wins, self.losses, self.unresolved, self.no_contest
        ));
        text.push_str(&format!(
            "- win rate: {:.1}% (Wilson 95% CI {:.1}%..{:.1}%)\n",
            self.win_rate * 100.0,
            self.wilson_low * 100.0,
            self.wilson_high * 100.0
        ));
        text.push_str(&format!(
            "- replay verified: {}/{} (failures: {})\n",
            self.verified,
            self.games.len(),
            self.verify_failures
        ));
        text.push_str(&format!(
            "- win breakdown: TopOut {}, forfeit-vision {}, forfeit-other {}; losses: TopOut {}, forfeit {}; turn-cap games {}\n",
            self.wins_topout,
            self.wins_forfeit_vision,
            self.wins_forfeit_other,
            self.losses_topout,
            self.losses_forfeit,
            self.turncap,
        ));
        if let Some(attax) = &self.attax {
            text.push_str(&format!(
                "- attax: {} {} commit {} bun {}\n",
                attax.info_name,
                attax.info_version,
                attax.commit.as_deref().unwrap_or("unknown"),
                attax.bun_version.as_deref().unwrap_or("unknown")
            ));
            text.push_str(&format!("- attax command: `{}`\n", attax.command_line));
        }
        text.push_str("\n## Games\n\n");
        text.push_str(
            "| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |\n",
        );
        text.push_str("|---|---|---|---|---|---|---|---|---|---|---|---|\n");
        for game in &self.games {
            let winner = match game.winner {
                Some(0) => "intetrigence".to_string(),
                Some(1) => "attax".to_string(),
                Some(slot) => format!("slot {slot}"),
                None => "draw".to_string(),
            };
            text.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {:.1} / {:.1} | {:.1} / {:.1} | {:.2} / {:.2} | {:.2} / {:.2} | {} |\n",
                game.seed,
                game.sheet,
                winner,
                game.reason,
                game.forfeit_cause.as_deref().unwrap_or("-"),
                game.turns,
                if game.verified { "yes" } else { "NO" },
                game.think_median_ms[0],
                game.think_median_ms[1],
                game.think_p95_ms[0],
                game.think_p95_ms[1],
                game.first_choice_rate[0],
                game.first_choice_rate[1],
                game.spin_match_rate[0],
                game.spin_match_rate[1],
                game.start_resends,
            ));
        }
        text.push_str("\n## Per-bot audit\n\n");
        text.push_str(
            "| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |\n",
        );
        text.push_str("|---|---|---|---|---|---|---|---|---|---|\n");
        for bot in &self.bots {
            text.push_str(&format!(
                "| {} | {} | {} | {} | {} / {} | {} / {} | {:.3} | {:.1} | {:.1} / {:.1} | {} |\n",
                bot.slot,
                bot.name,
                bot.visible_rows,
                bot.placements,
                bot.spin_mismatches,
                bot.spin_checked,
                bot.attack_mismatches_same_spin,
                bot.attack_checked,
                bot.attack_per_piece,
                bot.received_per_game,
                bot.mean_height,
                bot.max_height,
                bot.think_over_budget,
            ));
        }
        text.push_str("\n## Notes\n\n");
        for note in &self.notes {
            text.push_str(&format!("- {note}\n"));
        }
        text
    }
}

/// Runs every (seed, sheet) pair, writes replays + audits and returns the summary.
/// Content check for the opponent decision log (protocol v1.32 §8.3.1.2).
///
/// A file that merely exists is not evidence: it can be 0 bytes, end in a
/// partial JSON line, or stop before the game did. For a TBP transcript we
/// require one `suggestion` per **opponent turn**, one `play` per opponent
/// turn (minus one when the opponent forfeited on its last turn), exactly one
/// initial `start` (garbage resends are allowed), and non-empty suggestions
/// unless the opponent forfeited. For an attax decision log we require one
/// decision line per opponent turn.
///
/// The opponent does not play the last turn when it wins (the game ends on the
/// loser's turn), so the caller passes the opponent's turn count, not the
/// replay's total turns.
fn check_opponent_log(
    path: &std::path::Path,
    expected_suggestions: u32,
    expected_plays: u32,
    forfeit_by_opponent: bool,
    tbp: bool,
) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|error| format!("read: {error}"))?;
    if text.trim().is_empty() {
        return Err("empty log".into());
    }
    let mut lines = 0u32;
    let mut starts = 0u32;
    let mut initial_starts = 0u32;
    let mut suggestions = 0u32;
    let mut nonempty_suggestions = 0u32;
    let mut plays = 0u32;
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)
            .map_err(|error| format!("line {} is not complete JSON: {error}", index + 1))?;
        lines += 1;
        if tbp {
            match value.get("type").and_then(|value| value.as_str()) {
                Some("start") => {
                    starts += 1;
                    // The initial start has no reason; garbage resends carry
                    // `reason: "garbage"` (protocol §5.4).
                    if value.get("reason").is_none() {
                        initial_starts += 1;
                    }
                }
                Some("suggestion") => {
                    suggestions += 1;
                    let has_candidate = value
                        .get("payload")
                        .and_then(|payload| payload.get("moves"))
                        .and_then(|moves| moves.as_array())
                        .map(|moves| !moves.is_empty())
                        .unwrap_or(false);
                    if has_candidate {
                        nonempty_suggestions += 1;
                    }
                }
                Some("play") => plays += 1,
                _ => {}
            }
        }
    }
    if tbp {
        if starts == 0 {
            return Err("no start message".into());
        }
        if initial_starts != 1 {
            return Err(format!("initial start count {initial_starts} != 1"));
        }
        if suggestions != expected_suggestions {
            return Err(format!(
                "suggestion count {suggestions} != expected {expected_suggestions}"
            ));
        }
        if !forfeit_by_opponent && nonempty_suggestions != suggestions {
            return Err(format!(
                "{} of {suggestions} suggestions are empty",
                suggestions - nonempty_suggestions
            ));
        }
        if nonempty_suggestions == 0 {
            return Err("no suggestion with a candidate".into());
        }
        if plays != expected_plays {
            return Err(format!("play count {plays} != expected {expected_plays}"));
        }
    } else if lines != expected_suggestions {
        return Err(format!(
            "decision count {lines} != expected {expected_suggestions}"
        ));
    }
    Ok(())
}

pub fn run_match(config: &MatchConfig) -> Result<MatchSummary, String> {
    let replays = config.out_dir.join("replays");
    let audits_dir = config.out_dir.join("audit");
    let logs = config.out_dir.join("logs");
    for dir in [&replays, &audits_dir, &logs] {
        std::fs::create_dir_all(dir)
            .map_err(|error| format!("create {}: {error}", dir.display()))?;
    }

    let mut audits = Vec::new();
    let mut resumed_games = 0u32;
    let mut measurement_incomplete = 0u32;
    for &seed in &config.seeds {
        for &sheet in &config.sheets {
            let name = format!("seed-{seed}-sheet-{}", sheet.as_str());
            let replay_path = replays.join(format!("{name}.jsonl"));
            let audit_path = audits_dir.join(format!("{name}.json"));
            let log_path = logs.join(format!("{name}-attax.jsonl"));
            let stderr_path = logs.join(format!("{name}-attax-stderr.log"));

            if config.resume && replay_path.exists() && audit_path.exists() {
                if let Ok(verify) = verify_replay(&replay_path) {
                    if verify.ok {
                        if let Ok(text) = std::fs::read_to_string(&audit_path) {
                            if let Ok(mut audit) = serde_json::from_str::<GameAudit>(&text) {
                                audit.verify = verify;
                                println!("{name}: resumed (verified)");
                                audits.push(audit);
                                resumed_games += 1;
                                continue;
                            }
                        }
                    }
                }
            }

            // A fresh game must not append to a previous run's logs: the TBP
            // transcript and the attax decision log are opened in append mode.
            // Resumed games are handled above and keep their logs.
            let transcript_path = logs.join(format!("{name}-opponent.jsonl"));
            for path in [&log_path, &stderr_path, &transcript_path] {
                let _ = std::fs::remove_file(path);
            }

            let mut bot: Box<dyn Agent> = match &config.bot {
                BotChoice::Baseline => Box::new(BaselineBot::new()),
                BotChoice::Search(search) => {
                    let mut bot_config =
                        crate::bot_agent::BotAgentConfig::new(search.as_ref().clone());
                    bot_config.commit = git_short_head();
                    bot_config.args.extend(config.weights_note.clone());
                    Box::new(crate::bot_agent::BotAgent::new(bot_config))
                }
            };
            let mut opponent: Box<dyn Agent> = if config.self_play {
                let BotChoice::Search(search) = &config.bot else {
                    return Err("--self-play requires --bot search".into());
                };
                let mut bot_config = crate::bot_agent::BotAgentConfig::new(search.as_ref().clone());
                bot_config.commit = git_short_head();
                bot_config.args.extend(config.weights_note.clone());
                Box::new(crate::bot_agent::BotAgent::new(bot_config))
            } else if let Some(tbp) = &config.tbp_opponent {
                let mut host = HostConfig::new(tbp.program.clone(), tbp.args.clone());
                host.cwd = tbp.cwd.clone();
                host.env = tbp.env.clone();
                host.timeout = tbp.timeout;
                host.stderr_path = Some(stderr_path.clone());
                let mut tbp_config = TbpAgentConfig::new(host);
                tbp_config.commit = tbp.commit.clone();
                tbp_config.name = Some(tbp.name.clone());
                tbp_config.visible_rows = tbp.visible_rows;
                tbp_config.supports_180 = tbp.supports_180;
                tbp_config.suggest_delay_ms = tbp.suggest_delay_ms;
                tbp_config.retry_empty_suggest_ms = tbp.retry_empty_suggest_ms;
                tbp_config.transcript = Some(logs.join(format!("{name}-opponent.jsonl")));
                Box::new(TbpAgent::new(tbp_config))
            } else {
                match &config.attax {
                    Some(attax) => {
                        let mut host = HostConfig::new(
                            attax.bun.clone(),
                            vec![attax.entry_point().to_string_lossy().to_string()],
                        );
                        host.timeout = attax.timeout;
                        host.stderr_path = Some(stderr_path.clone());
                        host.env = attax.env.clone();
                        let mut tbp_config = TbpAgentConfig::new(host);
                        tbp_config.commit = attax.commit.clone();
                        tbp_config.bun_version = attax.bun_version.clone();
                        tbp_config.visible_rows = if attax.visible_rows == 0 {
                            detect_attax_visible_rows(&attax.dir)
                        } else {
                            attax.visible_rows
                        };
                        tbp_config.supports_180 = attax.supports_180;
                        tbp_config.decision_log = Some(log_path.clone());
                        tbp_config.transcript = Some(logs.join(format!("{name}-opponent.jsonl")));
                        Box::new(TbpAgent::new(tbp_config))
                    }
                    // No external bot: run the harness against itself offline.
                    None => Box::new(BaselineBot::with_name("baseline-opponent")),
                }
            };

            let round_config = RoundConfig {
                seed,
                sheet,
                turn_cap: config.turn_cap,
                frames_per_turn: config.frames_per_turn,
                rules: config.rules.clone(),
                protocol: config.protocol.clone(),
                started_at: config.started_at.clone(),
                profile: config.profile,
            };

            let mut agents: [&mut dyn Agent; 2] = [bot.as_mut(), opponent.as_mut()];
            let report = run_round(&round_config, &mut agents)
                .map_err(|error| format!("{name}: {error}"))?;

            write_replay(&replay_path, &report.records)
                .map_err(|error| format!("write {}: {error}", replay_path.display()))?;
            let verify = verify_replay(&replay_path)
                .map_err(|error| format!("verify {}: {error}", replay_path.display()))?;

            let mut audit = report.audit;
            if config.self_play || config.attax.is_none() {
                audit.attax = None;
            }
            audit.replay = format!("replays/{name}.jsonl");
            audit.verify = verify;
            audit.queue_invariant_violations = report.agent_audits[1].queue_invariant_violations;
            audit
                .write(&audit_path)
                .map_err(|error| format!("write {}: {error}", audit_path.display()))?;

            let tbp_opponent = config.tbp_opponent.is_some();
            let opponent_log = if tbp_opponent {
                logs.join(format!("{name}-opponent.jsonl"))
            } else {
                log_path.clone()
            };
            // `exists()` is not proof of completeness: a 0-byte file, a
            // truncated final line, or a log that stops mid-game all exist.
            // Count the opponent's actual placements from the replay; do not
            // derive the count from the winner (the seat that dies first
            // decides whether the opponent gets the last turn; protocol v1.33
            // §8.3.1.2).
            let opponent_plays = report
                .records
                .iter()
                .filter(|record| {
                    matches!(
                        record,
                        crate::replay::ReplayRecord::Turn(turn)
                            if turn.placements.iter().any(|placement| placement.slot == 1)
                    )
                })
                .count() as u32;
            let forfeit_by_opponent = audit.reason == "Forfeit" && audit.forfeit_slot == Some(1);
            // A forfeit by the opponent logs the forfeit turn's suggestion
            // but no play; every other game has one suggestion per play.
            let expected_suggestions = opponent_plays + u32::from(forfeit_by_opponent);
            let log_check = check_opponent_log(
                &opponent_log,
                expected_suggestions,
                opponent_plays,
                forfeit_by_opponent,
                tbp_opponent,
            );
            if !replay_path.exists() || !audit_path.exists() || log_check.is_err() {
                measurement_incomplete += 1;
                eprintln!(
                    "{name}: measurement incomplete (replay={} audit={} opponent_log={})",
                    replay_path.exists(),
                    audit_path.exists(),
                    log_check.as_ref().err().map(String::as_str).unwrap_or("ok")
                );
            }
            println!(
                "{name}: winner={:?} reason={} turns={} verified={} mismatches={}",
                audit.winner,
                audit.reason,
                audit.turns,
                audit.verify.ok,
                audit.verify.mismatches.len()
            );
            audits.push(audit);
        }
    }

    let mut summary = summarize(&audits, config.notes.clone());
    summary.resumed_games = resumed_games;
    summary.measurement_complete = measurement_incomplete == 0;
    summary.attax_patch_sha256 = config
        .attax
        .as_ref()
        .and_then(|attax| attax.patch_sha256.clone());
    if resumed_games > 0 {
        println!("resume: {resumed_games} verified game(s) reused");
    }
    summary
        .write(&config.out_dir)
        .map_err(|error| format!("write summary: {error}"))?;
    Ok(summary)
}

/// Loads every audit JSON in `<dir>/audit`.
pub fn load_audits(dir: &Path) -> Result<Vec<GameAudit>, String> {
    let audit_dir = dir.join("audit");
    let mut audits = Vec::new();
    let entries = std::fs::read_dir(&audit_dir)
        .map_err(|error| format!("read {}: {error}", audit_dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        let audit: GameAudit = serde_json::from_str(&text)
            .map_err(|error| format!("parse {}: {error}", path.display()))?;
        audits.push(audit);
    }
    audits.sort_by_key(|audit| (audit.seed, audit.sheet.clone()));
    Ok(audits)
}

/// Re-verifies every replay under `<dir>/replays`.
pub fn verify_all(dir: &Path) -> Result<Vec<(String, VerifyReport)>, String> {
    let replay_dir = dir.join("replays");
    let mut reports = Vec::new();
    let entries = std::fs::read_dir(&replay_dir)
        .map_err(|error| format!("read {}: {error}", replay_dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
            continue;
        }
        let report =
            verify_replay(&path).map_err(|error| format!("verify {}: {error}", path.display()))?;
        reports.push((path.display().to_string(), report));
    }
    reports.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(reports)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opponent_log_content_check_rejects_truncated_transcripts() {
        let dir = std::env::temp_dir().join(format!("arena-log-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("transcript.jsonl");
        let mut text = String::from(
            "{\"type\":\"start\",\"payload\":{}}
",
        );
        for _ in 0..3 {
            text.push_str(
                "{\"type\":\"suggestion\",\"payload\":{\"moves\":[{\"location\":{}}]}}
",
            );
            text.push_str(
                "{\"type\":\"play\",\"move\":{}}
",
            );
        }
        std::fs::write(&path, &text).unwrap();
        assert!(check_opponent_log(&path, 3, 3, false, true).is_ok());

        // A log that stops after 2 of 3 turns exists but is not complete.
        let truncated = text.lines().take(5).collect::<Vec<_>>().join(
            "
",
        );
        std::fs::write(&path, truncated).unwrap();
        assert!(check_opponent_log(&path, 3, 3, false, true).is_err());

        // A 0-byte file exists but carries no evidence.
        std::fs::write(&path, "").unwrap();
        assert!(check_opponent_log(&path, 3, 3, false, true).is_err());

        // The log has one more suggestion than expected: the old
        // "winner does not play the last turn" rule produced exactly this
        // mismatch (s2-spec-lead's failure 8).
        std::fs::write(&path, &text).unwrap();
        assert!(check_opponent_log(&path, 2, 3, false, true).is_err());

        // A partial final line is not valid JSON.
        std::fs::write(
            &path,
            "{\"type\":\"start\"}
{\"type\":\"suggestion\"",
        )
        .unwrap();
        assert!(check_opponent_log(&path, 1, 1, false, true).is_err());

        // A forfeit has one fewer play than suggestions.
        let mut forfeit = String::from(
            "{\"type\":\"start\",\"payload\":{}}
",
        );
        forfeit.push_str(
            "{\"type\":\"suggestion\",\"payload\":{\"moves\":[{}]}}
",
        );
        forfeit.push_str(
            "{\"type\":\"suggestion\",\"payload\":{\"moves\":[]}}
",
        );
        forfeit.push_str(
            "{\"type\":\"play\",\"move\":{}}
",
        );
        std::fs::write(&path, forfeit).unwrap();
        assert!(check_opponent_log(&path, 2, 1, true, true).is_ok());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn wilson_interval_is_sane() {
        let (low, high) = wilson_interval(10, 10, 1.959_964);
        assert!(low < 1.0 && high > 0.99);
        let (low, high) = wilson_interval(0, 10, 1.959_964);
        assert_eq!(low, 0.0);
        assert!(high > 0.0);
    }

    #[test]
    fn summary_markdown_contains_the_gate_evidence() {
        let summary = MatchSummary {
            schema: "x".into(),
            started_at: "unix:0".into(),
            profile: "official".into(),
            games: vec![],
            wins: 0,
            losses: 0,
            unresolved: 0,
            no_contest: 0,
            stuck: 0,
            forfeits: 0,
            wins_topout: 0,
            wins_turncap: 0,
            losses_turncap: 0,
            wins_forfeit_vision: 0,
            wins_forfeit_other: 0,
            forfeit_vision: 0,
            forfeit_other: 0,
            forfeit_unknown: 0,
            losses_topout: 0,
            losses_forfeit: 0,
            turncap: 0,
            surge_released: [0, 0],
            max_b2b_streak: [0, 0],
            turns_with_b2b_ge_4: [0, 0],
            win_rate: 0.0,
            wilson_low: 0.0,
            wilson_high: 1.0,
            verified: 0,
            verify_failures: 0,
            resumed_games: 0,
            measurement_complete: true,
            attax_patch_sha256: None,
            attax: None,
            bots: [
                BotAggregate {
                    slot: 0,
                    name: "a".into(),
                    placements: 0,
                    spin_checked: 0,
                    spin_mismatches: 0,
                    attack_checked: 0,
                    attack_mismatches: 0,
                    attack_mismatches_same_spin: 0,
                    think_over_budget: 0,
                    attack_per_piece: 0.0,
                    received_per_game: 0.0,
                    visible_rows: 0,
                    mean_height: 0.0,
                    max_height: 0.0,
                    ..Default::default()
                },
                BotAggregate {
                    slot: 1,
                    name: "b".into(),
                    placements: 0,
                    spin_checked: 0,
                    spin_mismatches: 0,
                    attack_checked: 0,
                    attack_mismatches: 0,
                    attack_mismatches_same_spin: 0,
                    think_over_budget: 0,
                    attack_per_piece: 0.0,
                    received_per_game: 0.0,
                    visible_rows: 0,
                    mean_height: 0.0,
                    max_height: 0.0,
                    ..Default::default()
                },
            ],
            notes: vec!["note".into()],
        };
        let markdown = summary.to_markdown();
        assert!(markdown.contains("replay verified: 0/0"));
        assert!(markdown.contains("note"));
    }
}
