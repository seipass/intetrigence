//! Staged round runner (`docs/s2-referee-protocol.md` v1.13 §1.4/§1.5).
//!
//! A round is: attax 50 games (25 seeds x both sheets) -> CC2 50 -> clys 50,
//! all with the same build and the same pre-committed evaluation seeds. A stage
//! only runs when the previous gate passed; a failed gate stops the round.
//!
//! The runner also owns the seed-discipline checks: the seeds file must be
//! committed, its `build:` must match the running build, and the seeds commit
//! must be an ancestor of the build commit.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::audit::wilson_interval;
use crate::engine::{Profile, Sheet};
use crate::harness::{
    AttaxConfig, BotChoice, MatchConfig, MatchSummary, TbpOpponentConfig, git_short_head,
    now_stamp, run_match,
};

/// The three stages, in order.
pub const STAGE_KEYS: [&str; 3] = ["attax", "cc2", "clys"];

/// How to start an opponent.
#[derive(Clone, Debug)]
pub enum OpponentKind {
    Attax {
        dir: PathBuf,
        bun: String,
        commit: Option<String>,
        visible_rows: u32,
        env: Vec<(String, String)>,
    },
    Tbp(TbpOpponentConfig),
    /// Not configured for this run (the stage fails if it is reached).
    Missing,
}

#[derive(Clone, Debug)]
pub struct OpponentSpec {
    pub key: String,
    pub display: String,
    pub commit: Option<String>,
    /// Protocol-compatibility patch disclosure (v1.14 §7.3): file names,
    /// line count, purpose, and the "no search/eval changes" statement.
    pub patch_note: Option<String>,
    pub patch_sha256: Option<String>,
    pub kind: OpponentKind,
}

#[derive(Clone, Debug)]
pub struct RoundSpec {
    pub round: u32,
    pub seeds: Vec<u64>,
    /// Path of the seeds file, for provenance.
    pub seeds_source: String,
    pub seeds_commit: Option<String>,
    pub build_commit: Option<String>,
    /// Deterministic search (fixed schedule). The A/B uses this; the official
    /// round's mode is recorded in the report.
    pub deterministic: bool,
    pub out_root: PathBuf,
    pub report_path: PathBuf,
    pub bot: BotChoice,
    pub turn_cap: u32,
    pub profile: Profile,
    /// Wins required at each stage (50, 48, 48).
    pub gates: [u32; 3],
    pub opponents: [OpponentSpec; 3],
    pub notes: Vec<String>,
    /// Normally 3; a dry run sets 1.
    pub max_stages: usize,
    /// Reuse verified `(seed, sheet)` games from a previous interrupted run
    /// (protocol v1.18 §1.4). Requires the same build and seeds.
    pub resume: bool,
    pub resume_reason: Option<String>,
    /// Applied weight overrides, recorded in the report and replay headers.
    pub weights_note: Vec<String>,
}

/// One stage's result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StageReport {
    pub key: String,
    pub opponent: String,
    pub commit: Option<String>,
    pub patch_note: Option<String>,
    pub patch_sha256: Option<String>,
    /// Extra environment passed to the opponent process (absolute paths).
    #[serde(default)]
    pub env: Vec<(String, String)>,
    pub games: u32,
    pub wins: u32,
    pub losses: u32,
    pub gate: u32,
    pub passed: bool,
    pub reason: String,
    pub summary_path: String,
    pub verified: u32,
    pub verify_failures: u32,
    #[serde(default)]
    pub topout_wins: u32,
    #[serde(default)]
    pub unresolved: u32,
    #[serde(default)]
    pub no_contest: u32,
    #[serde(default)]
    pub stuck: u32,
    #[serde(default)]
    pub turncap: u32,
    #[serde(default)]
    pub turncap_wins: u32,
    #[serde(default)]
    pub topout_losses: u32,
    #[serde(default)]
    pub forfeit_vision: u32,
    #[serde(default)]
    pub resumed_games: u32,
}

/// The whole round, written to `scripts/evidence/round-N/round.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoundReport {
    pub schema: String,
    pub round: u32,
    pub started_at: String,
    pub build_commit: Option<String>,
    #[serde(default)]
    pub deterministic: bool,
    pub seeds: Vec<u64>,
    pub seeds_source: String,
    pub seeds_commit: Option<String>,
    pub profile: String,
    pub gates: [u32; 3],
    pub stages: Vec<StageReport>,
    #[serde(default)]
    pub resumed: bool,
    #[serde(default)]
    pub resumed_games: u32,
    #[serde(default)]
    pub resume_reason: Option<String>,
    #[serde(default)]
    pub weights: Vec<String>,
    pub stopped_at: Option<String>,
    /// 1-based stage number where the round stopped, when it did not complete.
    pub stopped_at_stage: Option<usize>,
    /// Human-readable reason for the stop (the failed stage's reason).
    pub stop_reason: Option<String>,
    pub completed: bool,
}

// ---------------------------------------------------------------------------
// Seeds file
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct SeedsFile {
    pub round: Option<u32>,
    pub committed: Option<String>,
    pub build: Option<String>,
    pub seeds: Vec<u64>,
    pub count: Option<usize>,
    pub registry: Option<String>,
    pub note: Option<String>,
}

/// Parses the `docs/seeds-round-N.md` format (protocol v1.13 §1.5).
///
/// Accepts `range: a-b` or `seeds: a,b,c` and ignores prose lines.
pub fn read_seeds_file(path: &Path) -> Result<SeedsFile, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    let mut file = SeedsFile::default();
    for line in text.lines() {
        let line = line.trim().trim_start_matches("- ").trim();
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        match key.as_str() {
            "round" => file.round = value.parse().ok(),
            "committed" => file.committed = Some(value.to_string()),
            "build" => file.build = Some(value.trim_matches('`').to_string()),
            "range" => {
                if let Some((from, to)) = value.split_once('-') {
                    let from: u64 = from
                        .trim()
                        .parse()
                        .map_err(|error| format!("bad range start `{from}`: {error}"))?;
                    let to: u64 = to
                        .trim()
                        .parse()
                        .map_err(|error| format!("bad range end `{to}`: {error}"))?;
                    file.seeds.extend(from..=to);
                }
            }
            "seeds" => {
                for part in value.split(',') {
                    let part = part.trim();
                    if part.is_empty() {
                        continue;
                    }
                    file.seeds.push(
                        part.parse()
                            .map_err(|error| format!("bad seed `{part}`: {error}"))?,
                    );
                }
            }
            "count" => file.count = value.parse().ok(),
            "registry" => file.registry = Some(value.to_string()),
            "note" => file.note = Some(value.to_string()),
            _ => {}
        }
    }
    if file.seeds.is_empty() {
        return Err(format!("{}: no seeds found", path.display()));
    }
    Ok(file)
}

/// Git commit that last touched `path`, if it is committed.
pub fn file_commit(path: &Path) -> Option<String> {
    let root = crate::harness::repo_root();
    let relative = path.strip_prefix(&root).unwrap_or(path);
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["log", "-1", "--format=%H", "--"])
        .arg(relative)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if commit.is_empty() {
        None
    } else {
        Some(commit)
    }
}

/// True when `ancestor` is reachable from `descendant`.
pub fn is_ancestor(ancestor: &str, descendant: &str) -> bool {
    let root = crate::harness::repo_root();
    std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["merge-base", "--is-ancestor", ancestor, descendant])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// True when the working tree has uncommitted changes.
pub fn working_tree_dirty() -> Option<bool> {
    let root = crate::harness::repo_root();
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["status", "--porcelain"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

/// Full HEAD commit.
pub fn head_commit() -> Option<String> {
    let root = crate::harness::repo_root();
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Checks the seed-discipline rules before a round starts.
///
/// Returns the seeds commit when everything holds. `strict` is false for dry
/// runs, where the seeds file may be absent and already-burned seeds are used.
pub fn check_seed_discipline(
    seeds_path: Option<&Path>,
    build_commit: Option<&str>,
    strict: bool,
) -> Result<Option<String>, String> {
    if !strict {
        return Ok(None);
    }
    let Some(path) = seeds_path else {
        return Err("a round needs --seeds-file (docs/seeds-round-N.md)".into());
    };
    let file = read_seeds_file(path)?;
    let Some(build) = file.build.as_deref() else {
        return Err(format!(
            "{}: missing `build:` (protocol v1.13 §1.5)",
            path.display()
        ));
    };
    let Some(head) = build_commit else {
        return Err("cannot read the current git HEAD".into());
    };
    if let Some(count) = file.count {
        if count != file.seeds.len() {
            return Err(format!(
                "seeds file count {count} != {} parsed seeds",
                file.seeds.len()
            ));
        }
    }
    if file.seeds.len() != 25 {
        return Err(format!(
            "a round needs 25 evaluation seeds, found {}",
            file.seeds.len()
        ));
    }
    if !is_ancestor(build, head) {
        return Err(format!(
            "seeds file build `{build}` is not an ancestor of HEAD `{head}`"
        ));
    }
    let Some(seeds_commit) = file_commit(path) else {
        return Err(format!(
            "{} is not committed; commit the seeds before running",
            path.display()
        ));
    };
    if !is_ancestor(&seeds_commit, head) {
        return Err(format!(
            "seeds commit {seeds_commit} is not an ancestor of HEAD {head}"
        ));
    }
    if seeds_commit == build {
        return Err(format!(
            "seeds must be committed after the build commit (both are {build})"
        ));
    }
    if !is_ancestor(build, &seeds_commit) {
        return Err(format!(
            "seeds commit {seeds_commit} is not after the build commit {build}"
        ));
    }
    if let Some(dirty) = working_tree_dirty() {
        if dirty {
            return Err(
                "working tree is dirty; commit the build and the seeds before running a round"
                    .into(),
            );
        }
    }
    Ok(Some(seeds_commit))
}

// ---------------------------------------------------------------------------
// Round execution
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn gate_check(
    index: usize,
    topout_wins: u32,
    topout_losses: u32,
    turncap: u32,
    no_contest: u32,
    stuck: u32,
    measurement_complete: bool,
    verified: u32,
    games: u32,
    gate: u32,
) -> bool {
    // A stage is exactly 50 games (25 seeds x 2 sheets).
    if games != 50 {
        return false;
    }
    // Every game needs replay + verify + audit + opponent decision log
    // (protocol v1.22 §8.3.1).
    if !measurement_complete {
        return false;
    }
    // Every replay must verify.
    if verified != games {
        return false;
    }
    // TurnCap is not a win: every game must be decided by TopOut (user rule,
    // protocol v1.26 §1.4).
    if turncap > 0 {
        return false;
    }
    // Every game must be accounted for (five outcomes, including Stuck).
    if topout_wins + topout_losses + turncap + no_contest + stuck != games {
        return false;
    }
    if index < 2 {
        // Stages 1-2 (CC2, clys) are perfect records: 50 TopOut wins, no
        // loss, no turn-cap, no forfeit (new staged goal).
        topout_wins == gate && topout_losses == 0 && no_contest == 0
    } else {
        // Stage 3 (attax): 48+ TopOut wins, no turn-cap; no-contest is
        // allowed only if the 50-game total still holds.
        topout_wins >= gate
    }
}

fn stage_gate(index: usize, summary: &MatchSummary, gate: u32) -> bool {
    gate_check(
        index,
        summary.wins_topout,
        summary.losses_topout,
        summary.turncap,
        summary.no_contest,
        summary.stuck,
        summary.measurement_complete,
        summary.verified,
        summary.games.len() as u32,
        gate,
    )
}

fn stage_reason(index: usize, summary: &MatchSummary, gate: u32, passed: bool) -> String {
    if passed {
        return "passed".into();
    }
    if index < 2 && summary.no_contest > 0 {
        return format!(
            "invalid: {} no-contest (forfeit) game(s) in stage {} (perfect record required)",
            summary.no_contest,
            index + 1
        );
    }
    if !summary.measurement_complete {
        return "measurement incomplete (missing replay, audit or opponent decision log)".into();
    }
    if summary.verify_failures > 0 {
        return format!("{} replay verification failure(s)", summary.verify_failures);
    }
    if summary.turncap > 0 {
        return format!(
            "gate failed: {} turn-cap game(s) (every game must be decided by TopOut)",
            summary.turncap
        );
    }
    if index < 2 {
        format!(
            "gate failed: {} TopOut wins / {} games (needs {}; TopOut losses {})",
            summary.wins_topout,
            summary.games.len(),
            gate,
            summary.losses_topout
        )
    } else {
        format!(
            "gate failed: {} TopOut wins / {} games (needs {})",
            summary.wins_topout,
            summary.games.len(),
            gate
        )
    }
}

/// Runs the stages in order, stopping at the first failed gate.
pub fn run_round(spec: &RoundSpec) -> Result<RoundReport, String> {
    if spec.resume {
        check_resume_compatibility(spec)?;
    }
    let mut stages = Vec::new();
    let mut stopped_at = None;
    let mut stopped_at_stage = None;
    let mut stop_reason = None;
    for (index, opponent) in spec.opponents.iter().enumerate() {
        if index >= spec.max_stages {
            break;
        }
        let gate = spec.gates[index];
        let stage = if matches!(opponent.kind, OpponentKind::Missing) {
            StageReport {
                key: opponent.key.clone(),
                opponent: opponent.display.clone(),
                commit: opponent.commit.clone(),
                patch_note: opponent.patch_note.clone(),
                patch_sha256: opponent.patch_sha256.clone(),
                env: Vec::new(),
                games: 0,
                wins: 0,
                losses: 0,
                gate,
                passed: false,
                reason: "opponent not configured".into(),
                summary_path: String::new(),
                verified: 0,
                verify_failures: 0,
                topout_wins: 0,
                unresolved: 0,
                no_contest: 0,
                stuck: 0,
                turncap: 0,
                turncap_wins: 0,
                topout_losses: 0,
                forfeit_vision: 0,
                resumed_games: 0,
            }
        } else {
            let out_dir = spec
                .out_root
                .join(format!("round-{}-{}", spec.round, opponent.key));
            let mut config = MatchConfig::new(out_dir.clone());
            config.seeds = spec.seeds.clone();
            config.sheets = Sheet::ALL.to_vec();
            config.turn_cap = spec.turn_cap;
            config.bot = spec.bot.clone();
            config.profile = spec.profile;
            config.notes = spec.notes.clone();
            config.resume = spec.resume;
            config.weights_note = spec.weights_note.clone();
            config.build_commit = spec.build_commit.clone();
            config.deterministic = spec.deterministic;
            match &opponent.kind {
                OpponentKind::Attax {
                    dir,
                    bun,
                    commit,
                    visible_rows,
                    env,
                } => {
                    let mut attax = AttaxConfig::new(dir.clone());
                    attax.bun = bun.clone();
                    attax.commit = commit.clone();
                    attax.visible_rows = *visible_rows;
                    attax.patch_sha256 = opponent.patch_sha256.clone();
                    attax.env = env.clone();
                    config.attax = Some(attax);
                }
                OpponentKind::Tbp(tbp) => {
                    config.tbp_opponent = Some(tbp.clone());
                }
                OpponentKind::Missing => unreachable!(),
            }
            let summary = run_match(&config)?;
            let passed = stage_gate(index, &summary, gate);
            StageReport {
                key: opponent.key.clone(),
                opponent: opponent.display.clone(),
                commit: opponent.commit.clone(),
                patch_note: opponent.patch_note.clone(),
                patch_sha256: opponent.patch_sha256.clone(),
                env: match &opponent.kind {
                    OpponentKind::Tbp(config) => config.env.clone(),
                    OpponentKind::Attax { env, .. } => env.clone(),
                    OpponentKind::Missing => Vec::new(),
                },
                games: summary.games.len() as u32,
                wins: summary.wins,
                losses: summary.losses,
                gate,
                passed,
                reason: stage_reason(index, &summary, gate, passed),
                summary_path: out_dir.join("summary.json").display().to_string(),
                verified: summary.verified,
                verify_failures: summary.verify_failures,
                topout_wins: summary.wins_topout,
                unresolved: summary.unresolved,
                no_contest: summary.no_contest,
                stuck: summary.stuck,
                turncap: summary.turncap,
                turncap_wins: summary.wins_turncap,
                topout_losses: summary.losses_topout,
                forfeit_vision: summary.wins_forfeit_vision,
                resumed_games: summary.resumed_games,
            }
        };
        let failed = !stage.passed;
        if failed {
            stopped_at = Some(opponent.key.clone());
            stopped_at_stage = Some(index + 1);
            stop_reason = Some(stage.reason.clone());
        }
        stages.push(stage);
        if failed {
            break;
        }
    }
    let completed = stages.len() == spec.max_stages && stopped_at.is_none();
    let resumed_games: u32 = stages.iter().map(|stage| stage.resumed_games).sum();
    let report = RoundReport {
        schema: "intetrigence/round/1".into(),
        round: spec.round,
        started_at: now_stamp(),
        build_commit: spec.build_commit.clone(),
        deterministic: spec.deterministic,
        seeds: spec.seeds.clone(),
        seeds_source: spec.seeds_source.clone(),
        seeds_commit: spec.seeds_commit.clone(),
        profile: spec.profile.as_str().to_string(),
        gates: spec.gates,
        stages,
        resumed: spec.resume,
        resumed_games,
        resume_reason: spec.resume_reason.clone(),
        weights: spec.weights_note.clone(),
        stopped_at,
        stopped_at_stage,
        stop_reason,
        completed,
    };
    let round_dir = spec.out_root.join(format!("round-{}", spec.round));
    std::fs::create_dir_all(&round_dir)
        .map_err(|error| format!("create {}: {error}", round_dir.display()))?;
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("serialize round report: {error}"))?;
    std::fs::write(round_dir.join("round.json"), json + "\n")
        .map_err(|error| format!("write round.json: {error}"))?;
    write_markdown_report(spec, &report)?;
    Ok(report)
}

// ---------------------------------------------------------------------------
// Markdown report
// ---------------------------------------------------------------------------

fn load_summary(path: &str) -> Option<MatchSummary> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let mut count = 0usize;
    let mut total = 0.0;
    for value in values {
        total += value;
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        total / count as f64
    }
}

fn opt_f64(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.1}"))
        .unwrap_or_else(|| "n/a".into())
}

fn opt_u32(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "n/a".into())
}

fn stage_section(
    out: &mut String,
    index: usize,
    stage: &StageReport,
    summary: Option<&MatchSummary>,
) -> std::fmt::Result {
    writeln!(out, "### Stage {} — {}", index + 1, stage.opponent)?;
    writeln!(out)?;
    writeln!(
        out,
        "- opponent commit: `{}`",
        stage.commit.as_deref().unwrap_or("unknown")
    )?;
    if let Some(note) = &stage.patch_note {
        writeln!(out, "- compatibility patch: {note}")?;
    }
    if let Some(sha) = &stage.patch_sha256 {
        writeln!(out, "- compatibility patch SHA-256: `{sha}`")?;
    }
    for (key, value) in &stage.env {
        writeln!(out, "- environment: `{key}={value}`")?;
    }
    writeln!(
        out,
        "- gate: **{}** (wins needed) — {}",
        stage.gate,
        if stage.passed {
            "**PASSED**"
        } else {
            "**FAILED**"
        }
    )?;
    writeln!(out, "- reason: {}", stage.reason)?;
    if stage.games == 0 {
        writeln!(out, "- games: 0 (not run)")?;
        writeln!(out)?;
        return Ok(());
    }
    let Some(summary) = summary else {
        writeln!(out, "- summary: `{}` (unreadable)", stage.summary_path)?;
        writeln!(out)?;
        return Ok(());
    };
    let games = summary.games.len() as u32;
    let gate_wins = summary.wins_topout;
    let (low, high) = wilson_interval(gate_wins, games, 1.959_964);
    writeln!(
        out,
        "- games: **{}** ({} seed(s) x sheets A/B)",
        games,
        games / 2
    )?;
    if let Some(sha) = &summary.attax_patch_sha256 {
        writeln!(out, "- opponent patch SHA-256: `{sha}`")?;
    }
    writeln!(
        out,
        "- opponent vision: **{} rows**{}",
        summary.bots[1].visible_rows,
        if summary.bots[1].visible_rows < 40 {
            " (vision forfeits are possible)"
        } else {
            " (full board: decisions are TopOut / turn-cap)"
        }
    )?;
    writeln!(
        out,
        "- raw games: {} W / {} L; unresolved (TurnCap): {}; no-contest (forfeit): {}; stuck: {} (this is **not** a win rate)",
        summary.wins, summary.losses, summary.unresolved, summary.no_contest, summary.stuck
    )?;
    writeln!(
        out,
        "- **gate record: {} TopOut wins / {} games** — {:.1}% (Wilson 95% CI {:.1}..{:.1}%)",
        gate_wins,
        games,
        if games == 0 {
            0.0
        } else {
            gate_wins as f64 / games as f64 * 100.0
        },
        low * 100.0,
        high * 100.0
    )?;
    writeln!(
        out,
        "- win breakdown: TopOut **{}**; forfeit-vision **{}** (no contest); forfeit-other **{}** (invalid); turn-cap **{}** (unresolved)",
        summary.wins_topout, summary.forfeit_vision, summary.forfeit_other, summary.turncap
    )?;
    let non_forfeit_games = games - summary.forfeits;
    let non_forfeit_wins = summary.wins_topout + summary.turncap;
    writeln!(
        out,
        "- non-forfeit record: **{}/{}** (TopOut + turn-cap games)",
        non_forfeit_wins, non_forfeit_games
    )?;
    writeln!(
        out,
        "- attack/piece: **{:.3}** vs **{:.3}**",
        summary.bots[0].attack_per_piece, summary.bots[1].attack_per_piece
    )?;
    writeln!(
        out,
        "- received garbage/game: **{:.1}** vs **{:.1}**",
        summary.bots[0].received_per_game, summary.bots[1].received_per_game
    )?;
    let think = |slot: usize, field: fn(&crate::harness::GameSummary) -> [f64; 2]| {
        mean(summary.games.iter().map(|game| field(game)[slot]))
    };
    let think_max = |slot: usize| {
        summary
            .games
            .iter()
            .map(|game| game.think_max_ms[slot])
            .fold(0.0f64, f64::max)
    };
    writeln!(
        out,
        "- think median / p95 / p99 / max: **{:.1} / {:.1} / {:.1} / {:.1} ms** vs **{:.1} / {:.1} / {:.1} / {:.1} ms**",
        think(0, |game| game.think_median_ms),
        think(0, |game| game.think_p95_ms),
        think(0, |game| game.think_p99_ms),
        think_max(0),
        think(1, |game| game.think_median_ms),
        think(1, |game| game.think_p95_ms),
        think(1, |game| game.think_p99_ms),
        think_max(1)
    )?;
    {
        let bot = &summary.bots[0];
        writeln!(
            out,
            "- search (intetrigence): depth median / p95 / max **{} / {} / {}**, width **{} / {} / {}**, fallback **{}/{} ({:.1}%)**",
            opt_f64(bot.depth_median),
            opt_f64(bot.depth_p95),
            opt_u32(bot.depth_max),
            opt_f64(bot.width_median),
            opt_f64(bot.width_p95),
            opt_u32(bot.width_max),
            bot.fallback_turns,
            bot.search_turns,
            bot.fallback_rate * 100.0
        )?;
        writeln!(
            out,
            "- net damage/game: **{:+.1}** (negative games {}/{}); collapses **{}/{}**; opener lines/game **{:.2}**",
            bot.net_damage_per_game,
            bot.negative_net_games,
            games,
            bot.collapses,
            games,
            bot.opener_lines_per_game
        )?;
        writeln!(
            out,
            "- burst (intetrigence received): max 1-turn **{}**, max 5-turn **{}**, over capacity **{}** (overlapping 5-turn windows, not burst events)",
            bot.max_burst_1, bot.max_burst_5, bot.burst_over_capacity
        )?;
    }
    writeln!(
        out,
        "- think over 200 ms: {} vs {}",
        summary.bots[0].think_over_budget, summary.bots[1].think_over_budget
    )?;
    writeln!(
        out,
        "- measurement complete: {} (replay + verify + audit + opponent decision log)",
        if summary.measurement_complete {
            "**yes**"
        } else {
            "**NO**"
        }
    )?;
    writeln!(
        out,
        "- surge releases: {} vs {}; max B2B: {} vs {}; turns at B2B>=4: {} vs {}",
        summary.surge_released[0],
        summary.surge_released[1],
        summary.max_b2b_streak[0],
        summary.max_b2b_streak[1],
        summary.turns_with_b2b_ge_4[0],
        summary.turns_with_b2b_ge_4[1]
    )?;
    writeln!(
        out,
        "- replay verified: **{}/{}** (failures {})",
        summary.verified, games, summary.verify_failures
    )?;
    writeln!(
        out,
        "- audit: spin mismatches {} / {}, same-spin attack mismatches {} / {}",
        summary.bots[0].spin_mismatches,
        summary.bots[1].spin_mismatches,
        summary.bots[0].attack_mismatches_same_spin,
        summary.bots[1].attack_mismatches_same_spin
    )?;
    let per_sheet = |sheet: &str| {
        let wins = summary
            .games
            .iter()
            .filter(|game| game.sheet == sheet && game.winner == Some(0))
            .count();
        let total = summary
            .games
            .iter()
            .filter(|game| game.sheet == sheet)
            .count();
        (wins, total)
    };
    let (a_wins, a_total) = per_sheet("A");
    let (b_wins, b_total) = per_sheet("B");
    writeln!(
        out,
        "- per sheet: A {a_wins}/{a_total}, B {b_wins}/{b_total}"
    )?;
    writeln!(out)?;
    Ok(())
}

fn write_markdown_report(spec: &RoundSpec, report: &RoundReport) -> Result<(), String> {
    let mut out = String::new();
    writeln!(out, "# Round {} — staged round report", report.round).map_err(|e| e.to_string())?;
    writeln!(out).map_err(|e| e.to_string())?;
    writeln!(
        out,
        "- profile: **{}** (protocol v1.11 §1.3)",
        report.profile
    )
    .map_err(|e| e.to_string())?;
    writeln!(
        out,
        "- build: `{}`",
        report.build_commit.as_deref().unwrap_or("unknown")
    )
    .map_err(|e| e.to_string())?;
    writeln!(out, "- seeds: `{}`", report.seeds_source).map_err(|e| e.to_string())?;
    writeln!(
        out,
        "- seeds commit: `{}`",
        report.seeds_commit.as_deref().unwrap_or("uncommitted")
    )
    .map_err(|e| e.to_string())?;
    writeln!(
        out,
        "- seeds: {} values (`{}` .. `{}`)",
        report.seeds.len(),
        report.seeds.first().copied().unwrap_or(0),
        report.seeds.last().copied().unwrap_or(0)
    )
    .map_err(|e| e.to_string())?;
    writeln!(out, "- started: {}", report.started_at).map_err(|e| e.to_string())?;
    if !report.weights.is_empty() {
        writeln!(out, "- search weights: {}", report.weights.join(", "))
            .map_err(|e| e.to_string())?;
    }
    if report.resumed {
        writeln!(
            out,
            "- **resumed from an interrupted run**: {} verified game(s) reused; reason: {}",
            report.resumed_games,
            report.resume_reason.as_deref().unwrap_or("unspecified")
        )
        .map_err(|e| e.to_string())?;
    }
    writeln!(
        out,
        "- environment: {} / {} logical CPUs / bun 1.4.0",
        std::env::consts::OS,
        std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(0)
    )
    .map_err(|e| e.to_string())?;
    writeln!(out).map_err(|e| e.to_string())?;
    writeln!(out, "## Gates").map_err(|e| e.to_string())?;
    writeln!(out).map_err(|e| e.to_string())?;
    writeln!(out, "| stage | opponent | gate | games | W-L-D | result |")
        .map_err(|e| e.to_string())?;
    writeln!(out, "|---|---|---|---|---|---|").map_err(|e| e.to_string())?;
    for (index, stage) in report.stages.iter().enumerate() {
        writeln!(
            out,
            "| {} | {} | {} wins | {} | {}W-{}L (U{}, NC{}) | {} |",
            index + 1,
            stage.opponent,
            stage.gate,
            stage.games,
            stage.wins,
            stage.losses,
            stage.unresolved,
            stage.no_contest,
            if stage.passed { "PASSED" } else { "FAILED" }
        )
        .map_err(|e| e.to_string())?;
    }
    for index in report.stages.len()..spec.max_stages {
        writeln!(
            out,
            "| {} | {} | {} wins | 0 | - | not run |",
            index + 1,
            spec.opponents[index].display,
            spec.gates[index]
        )
        .map_err(|e| e.to_string())?;
    }
    writeln!(out).map_err(|e| e.to_string())?;
    if let Some(stage) = &report.stopped_at {
        writeln!(
            out,
            "**Round stopped at stage {} (`{}`): {}.** The remaining stages were not run; a retry needs a new build and a new pre-committed seed set (protocol v1.13 §1.4).",
            report
                .stopped_at_stage
                .map(|stage| stage.to_string())
                .unwrap_or_else(|| "?".into()),
            stage,
            report.stop_reason.as_deref().unwrap_or("unknown")
        )
        .map_err(|e| e.to_string())?;
        writeln!(out).map_err(|e| e.to_string())?;
    }
    for (index, stage) in report.stages.iter().enumerate() {
        let summary = load_summary(&stage.summary_path);
        stage_section(&mut out, index, stage, summary.as_ref()).map_err(|e| e.to_string())?;
    }
    writeln!(out, "## Claims and caveats").map_err(|e| e.to_string())?;
    writeln!(out).map_err(|e| e.to_string())?;
    writeln!(
        out,
        "- Only the `official` profile is used; diagnostic profiles are never part of a round."
    )
    .map_err(|e| e.to_string())?;
    writeln!(
        out,
        "- Spin classification follows protocol v1.7 §4.2.1 (max over paths), which is more lenient than TETR.IO; intetrigence's labels are backed by rotation-ending input sequences."
    )
    .map_err(|e| e.to_string())?;
    writeln!(
        out,
        "- This report does not claim a win rate against a full-vision opponent: forfeit wins are reported separately and the non-forfeit record is stated per stage."
    )
    .map_err(|e| e.to_string())?;
    writeln!(
        out,
        "- CC2 and clys are not S2-native. The referee applies S2 scoring to both sides and records the bots' declared values as audit only (protocol v1.13 §7.2); their stages are a generalization check, not an S2-vs-S2 claim."
    )
    .map_err(|e| e.to_string())?;
    writeln!(
        out,
        "- A `forfeit-other` win is an invalid game (protocol v1.13 §7.1.3-4) and fails the stage."
    )
    .map_err(|e| e.to_string())?;
    writeln!(
        out,
        "- Protocol-compatibility patches (v1.14 §7.3) only remove handicaps: they align spawn positions and the empty-hold representation with TBP/S2. They do not touch the opponents' search, evaluation or move legality, and they can only make the opponent stronger."
    )
    .map_err(|e| e.to_string())?;
    writeln!(
        out,
        "- Stage 1 (attax, 20-row vision) is largely decided by vision forfeits; stages 2-3 (CC2/clys, 40-row vision) are real TopOut battles. The stage win rates are therefore not directly comparable; the non-forfeit record per stage is the honest strength signal."
    )
    .map_err(|e| e.to_string())?;
    writeln!(out).map_err(|e| e.to_string())?;
    writeln!(out, "## Reproduction").map_err(|e| e.to_string())?;
    writeln!(out).map_err(|e| e.to_string())?;
    writeln!(
        out,
        "- `scripts/run-round.sh` with `ROUND={}` and `SEEDS_FILE={}`",
        report.round, report.seeds_source
    )
    .map_err(|e| e.to_string())?;
    writeln!(
        out,
        "- `arena verify scripts/evidence/round-{}-<opponent>`",
        report.round
    )
    .map_err(|e| e.to_string())?;
    writeln!(out).map_err(|e| e.to_string())?;
    if let Some(parent) = spec.report_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    std::fs::write(&spec.report_path, out)
        .map_err(|error| format!("write {}: {error}", spec.report_path.display()))?;
    Ok(())
}

/// Refuses to resume when the previous `round.json` used a different build or
/// seed set (protocol v1.18 §1.4: resume only continues the same measurement).
fn check_resume_compatibility(spec: &RoundSpec) -> Result<(), String> {
    let path = spec
        .out_root
        .join(format!("round-{}", spec.round))
        .join("round.json");
    if !path.exists() {
        return Ok(());
    }
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    let previous: RoundReport = serde_json::from_str(&text)
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    if previous.build_commit != spec.build_commit {
        return Err(format!(
            "cannot resume: previous build {:?} != current build {:?}",
            previous.build_commit, spec.build_commit
        ));
    }
    if previous.seeds != spec.seeds {
        return Err("cannot resume: previous seeds differ from the current seeds".into());
    }
    if previous.seeds_commit != spec.seeds_commit {
        return Err(format!(
            "cannot resume: previous seeds commit {:?} != current {:?}",
            previous.seeds_commit, spec.seeds_commit
        ));
    }
    Ok(())
}

/// Short helper for the CLI: the current build commit (short form).
pub fn current_build() -> Option<String> {
    git_short_head()
}

/// Default report path for a round.
pub fn default_report_path(round: u32) -> PathBuf {
    crate::harness::repo_root()
        .join("docs")
        .join(format!("round-{round}-report.md"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_file_parses_range_form() {
        let path = std::env::temp_dir().join("intetrigence-seeds-test.md");
        std::fs::write(
            &path,
            "# Round 1\n\n- round: 1\n- build: abc1234\n- range: 30001-30025\n- count: 25\n",
        )
        .unwrap();
        let file = read_seeds_file(&path).unwrap();
        assert_eq!(file.round, Some(1));
        assert_eq!(file.build.as_deref(), Some("abc1234"));
        assert_eq!(file.seeds, (30001..=30025).collect::<Vec<_>>());
        assert_eq!(file.count, Some(25));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn seeds_file_parses_list_form() {
        let path = std::env::temp_dir().join("intetrigence-seeds-list.md");
        std::fs::write(&path, "- seeds: 1, 2, 3\n").unwrap();
        let file = read_seeds_file(&path).unwrap();
        assert_eq!(file.seeds, vec![1, 2, 3]);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn stage_gate_cc2_clys_perfect_attax_48() {
        // Stages 1-2 (CC2, clys): perfect record.
        assert!(gate_check(0, 50, 0, 0, 0, 0, true, 50, 50, 50));
        assert!(gate_check(1, 50, 0, 0, 0, 0, true, 50, 50, 50));
        assert!(!gate_check(0, 49, 0, 0, 0, 0, true, 50, 50, 50));
        assert!(!gate_check(0, 50, 1, 0, 0, 0, true, 50, 50, 50));
        assert!(!gate_check(0, 50, 0, 1, 0, 0, true, 50, 50, 50));
        assert!(!gate_check(0, 50, 0, 0, 1, 0, true, 50, 50, 50));
        // A stuck game is not one of the five outcomes; the total fails.
        assert!(!gate_check(0, 50, 0, 0, 0, 1, true, 50, 50, 50));
        assert!(!gate_check(0, 50, 0, 0, 0, 0, false, 50, 50, 50));
        assert!(!gate_check(0, 50, 0, 0, 0, 0, true, 49, 50, 50));
        // Missing or extra games must fail even when the TopOut count is 50.
        assert!(!gate_check(0, 50, 0, 0, 0, 0, true, 51, 51, 50));
        assert!(!gate_check(0, 50, 0, 0, 0, 0, true, 49, 49, 50));
        // Stage 3 (attax): >= 48 TopOut wins, no turn-cap; no-contest is
        // allowed only when the 50-game total still holds.
        assert!(gate_check(2, 48, 2, 0, 0, 0, true, 50, 50, 48));
        assert!(gate_check(2, 48, 0, 0, 2, 0, true, 50, 50, 48));
        assert!(!gate_check(2, 47, 3, 0, 0, 0, true, 50, 50, 48));
        assert!(!gate_check(2, 48, 2, 1, 0, 0, true, 50, 50, 48));
        assert!(!gate_check(2, 48, 1, 0, 0, 0, true, 50, 50, 48));
    }
}
