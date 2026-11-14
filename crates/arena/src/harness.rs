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
    pub timeout: Duration,
}

impl AttaxConfig {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            bun: "bun".into(),
            commit: None,
            bun_version: None,
            timeout: Duration::from_secs(30),
        }
    }

    pub fn entry_point(&self) -> PathBuf {
        self.dir.join("tbp").join("attax-tbp.ts")
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
    /// Notes written into the summary. Set per match so the M0 plumbing run and
    /// the M2 real-bot run cannot inherit each other's wording.
    pub notes: Vec<String>,
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
            started_at: now_stamp(),
            rules: version_string("docs/s2-spec.md", "v1.3.11"),
            protocol: version_string("docs/s2-referee-protocol.md", "v1.9"),
            bot: BotChoice::Baseline,
            profile: Profile::Official,
            self_play: false,
            notes: vec!["The intetrigence side is the bot recorded in each replay header.".into()],
        }
    }
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameSummary {
    pub seed: u64,
    pub sheet: String,
    pub winner: Option<u8>,
    pub reason: String,
    /// `vision` | `other` | `unknown` for Forfeit results.
    pub forfeit_cause: Option<String>,
    pub turns: u32,
    pub verified: bool,
    pub verify_mismatches: usize,
    pub think_median_ms: [f64; 2],
    pub think_p95_ms: [f64; 2],
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub draws: u32,
    pub forfeits: u32,
    /// Win/loss breakdown (protocol v1.8 §7.1).
    pub wins_topout: u32,
    pub wins_forfeit_vision: u32,
    pub wins_forfeit_other: u32,
    pub losses_topout: u32,
    pub losses_forfeit: u32,
    pub turncap: u32,
    pub win_rate: f64,
    pub wilson_low: f64,
    pub wilson_high: f64,
    pub verified: u32,
    pub verify_failures: u32,
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

fn game_summary(audit: &GameAudit) -> GameSummary {
    GameSummary {
        seed: audit.seed,
        sheet: audit.sheet.clone(),
        winner: audit.winner,
        reason: audit.reason.clone(),
        forfeit_cause: audit.forfeit_cause.map(|cause| cause.as_str().to_string()),
        turns: audit.turns,
        verified: audit.verify.ok,
        verify_mismatches: audit.verify.mismatches.len(),
        think_median_ms: [audit.think[0].median_ms, audit.think[1].median_ms],
        think_p95_ms: [audit.think[0].p95_ms, audit.think[1].p95_ms],
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
    }
}

/// Aggregates audit records into the match summary (used by `arena report`).
pub fn summarize(audits: &[GameAudit], notes: Vec<String>) -> MatchSummary {
    let games: Vec<GameSummary> = audits.iter().map(game_summary).collect();
    let wins = games.iter().filter(|game| game.winner == Some(0)).count() as u32;
    let losses = games.iter().filter(|game| game.winner == Some(1)).count() as u32;
    let draws = games.iter().filter(|game| game.winner.is_none()).count() as u32;
    let forfeits = games.iter().filter(|game| game.reason == "Forfeit").count() as u32;
    let wins_topout = games
        .iter()
        .filter(|game| game.winner == Some(0) && game.reason == "TopOut")
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
                && game.forfeit_cause.as_deref() != Some("vision")
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
    let bots = [0usize, 1].map(|slot| BotAggregate {
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
        draws,
        forfeits,
        wins_topout,
        wins_forfeit_vision,
        wins_forfeit_other,
        losses_topout,
        losses_forfeit,
        turncap,
        win_rate: rate(wins, games.len() as u32),
        wilson_low: (wilson_low * 10000.0).round() / 10000.0,
        wilson_high: (wilson_high * 10000.0).round() / 10000.0,
        verified,
        verify_failures: games.len() as u32 - verified,
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
            "- intetrigence wins / losses / draws / forfeits: {} / {} / {} / {}\n",
            self.wins, self.losses, self.draws, self.forfeits
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
pub fn run_match(config: &MatchConfig) -> Result<MatchSummary, String> {
    let replays = config.out_dir.join("replays");
    let audits_dir = config.out_dir.join("audit");
    let logs = config.out_dir.join("logs");
    for dir in [&replays, &audits_dir, &logs] {
        std::fs::create_dir_all(dir)
            .map_err(|error| format!("create {}: {error}", dir.display()))?;
    }

    let mut audits = Vec::new();
    for &seed in &config.seeds {
        for &sheet in &config.sheets {
            let name = format!("seed-{seed}-sheet-{}", sheet.as_str());
            let replay_path = replays.join(format!("{name}.jsonl"));
            let audit_path = audits_dir.join(format!("{name}.json"));
            let log_path = logs.join(format!("{name}-attax.jsonl"));
            let stderr_path = logs.join(format!("{name}-attax-stderr.log"));

            let mut bot: Box<dyn Agent> = match &config.bot {
                BotChoice::Baseline => Box::new(BaselineBot::new()),
                BotChoice::Search(search) => {
                    let mut bot_config =
                        crate::bot_agent::BotAgentConfig::new(search.as_ref().clone());
                    bot_config.commit = git_short_head();
                    Box::new(crate::bot_agent::BotAgent::new(bot_config))
                }
            };
            let mut opponent: Box<dyn Agent> = if config.self_play {
                let BotChoice::Search(search) = &config.bot else {
                    return Err("--self-play requires --bot search".into());
                };
                let mut bot_config = crate::bot_agent::BotAgentConfig::new(search.as_ref().clone());
                bot_config.commit = git_short_head();
                Box::new(crate::bot_agent::BotAgent::new(bot_config))
            } else {
                match &config.attax {
                    Some(attax) => {
                        let mut host = HostConfig::new(
                            attax.bun.clone(),
                            vec![attax.entry_point().to_string_lossy().to_string()],
                        );
                        host.timeout = attax.timeout;
                        host.stderr_path = Some(stderr_path.clone());
                        let mut tbp_config = TbpAgentConfig::new(host);
                        tbp_config.commit = attax.commit.clone();
                        tbp_config.bun_version = attax.bun_version.clone();
                        tbp_config.decision_log = Some(log_path.clone());
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

    let summary = summarize(&audits, config.notes.clone());
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
            draws: 0,
            forfeits: 0,
            wins_topout: 0,
            wins_forfeit_vision: 0,
            wins_forfeit_other: 0,
            losses_topout: 0,
            losses_forfeit: 0,
            turncap: 0,
            win_rate: 0.0,
            wilson_low: 0.0,
            wilson_high: 1.0,
            verified: 0,
            verify_failures: 0,
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
                },
            ],
            notes: vec!["note".into()],
        };
        let markdown = summary.to_markdown();
        assert!(markdown.contains("replay verified: 0/0"));
        assert!(markdown.contains("note"));
    }
}
