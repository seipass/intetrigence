//! Per-game audit records and match summary (`docs/s2-referee-protocol.md` §8.4,
//! PLAN.md §3.3).
//!
//! Recorded every game:
//! - both bots' think time (median / p95 / max / mean);
//! - first-choice acceptance rate and the non-adopted index histogram;
//! - scoring audit: the bot's claimed spin and the attack implied by it vs the
//!   referee's recomputed spin and attack;
//! - reachability rejections (must be zero);
//! - attax's own work (`nodes`, `extra` ms, `ATTAX_TBP_LOG` decisions,
//!   `cellsAboveRow20`) and the hold-inference cross-check;
//! - `start` resends with reason and board hash;
//! - bot descriptors, command line and Bun version.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::agent::AgentAudit;

/// 5 PPS budget per placement (200 ms). Placements above it are counted so a
/// report can state whether the nominal pace was met (protocol v1.3 §2.1).
pub const PPS_BUDGET_MS: f64 = 200.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ThinkStats {
    pub count: usize,
    pub median_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
    pub mean_ms: f64,
    /// Placements whose wall-clock think time exceeded [`PPS_BUDGET_MS`].
    pub over_budget: u32,
}

impl ThinkStats {
    pub fn from_samples(samples: &[f64]) -> Self {
        if samples.is_empty() {
            return Self::default();
        }
        let mut sorted = samples.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let count = sorted.len();
        let median = if count % 2 == 1 {
            sorted[count / 2]
        } else {
            (sorted[count / 2 - 1] + sorted[count / 2]) / 2.0
        };
        let rank = ((0.95 * count as f64).ceil() as usize).clamp(1, count);
        let rank99 = ((0.99 * count as f64).ceil() as usize).clamp(1, count);
        Self {
            count,
            median_ms: round3(median),
            p95_ms: round3(sorted[rank - 1]),
            p99_ms: round3(sorted[rank99 - 1]),
            max_ms: round3(sorted[count - 1]),
            mean_ms: round3(sorted.iter().sum::<f64>() / count as f64),
            over_budget: samples
                .iter()
                .filter(|&&sample| sample > PPS_BUDGET_MS)
                .count() as u32,
        }
    }
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FirstChoiceAudit {
    pub total: u32,
    pub accepted_first: u32,
    pub rate: f64,
    /// candidate index -> count, for every non-zero index.
    pub rejected_index_histogram: BTreeMap<u32, u32>,
}

impl FirstChoiceAudit {
    pub fn from_indices(indices: &[u32]) -> Self {
        let total = indices.len() as u32;
        let accepted_first = indices.iter().filter(|&&index| index == 0).count() as u32;
        let mut histogram = BTreeMap::new();
        for &index in indices.iter().filter(|&&index| index != 0) {
            *histogram.entry(index).or_insert(0) += 1;
        }
        Self {
            total,
            accepted_first,
            rate: if total == 0 {
                0.0
            } else {
                round3(accepted_first as f64 / total as f64)
            },
            rejected_index_histogram: histogram,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ScoringAudit {
    pub placements: u32,
    pub spin_checked: u32,
    pub spin_matches: u32,
    pub spin_mismatches: u32,
    pub attack_checked: u32,
    pub attack_matches: u32,
    pub attack_mismatches: u32,
    /// Attack mismatches where the claimed spin label agreed with the referee.
    /// These are scoring-table differences, not spin-classification differences.
    pub attack_mismatches_same_spin: u32,
    /// Attack mismatches where the spin label itself differed.
    pub attack_mismatches_spin_diff: u32,
    pub declared_total: u32,
    pub recomputed_total: u32,
    pub examples: Vec<ScoringMismatch>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoringMismatch {
    pub turn: u32,
    pub slot: u8,
    pub claimed_spin: String,
    pub recomputed_spin: String,
    pub declared_attack: u32,
    pub recomputed_attack: u32,
}

impl ScoringAudit {
    pub fn record(
        &mut self,
        turn: u32,
        slot: u8,
        claimed_spin: Option<intetrigence_core::SpinType>,
        recomputed_spin: intetrigence_core::SpinType,
        declared_attack: Option<u32>,
        recomputed_attack: u32,
    ) {
        self.placements += 1;
        self.recomputed_total += recomputed_attack;
        if let Some(claimed) = claimed_spin {
            self.spin_checked += 1;
            if claimed == recomputed_spin {
                self.spin_matches += 1;
            } else {
                self.spin_mismatches += 1;
            }
        }
        if let Some(declared) = declared_attack {
            self.attack_checked += 1;
            self.declared_total += declared;
            if declared == recomputed_attack {
                self.attack_matches += 1;
            } else {
                self.attack_mismatches += 1;
                if claimed_spin == Some(recomputed_spin) {
                    self.attack_mismatches_same_spin += 1;
                } else {
                    self.attack_mismatches_spin_diff += 1;
                }
            }
            if (declared != recomputed_attack || claimed_spin != Some(recomputed_spin))
                && self.examples.len() < 20
            {
                self.examples.push(ScoringMismatch {
                    turn,
                    slot,
                    claimed_spin: claimed_spin
                        .map(|spin| crate::replay::spin_name(spin).to_string())
                        .unwrap_or_else(|| "none".into()),
                    recomputed_spin: crate::replay::spin_name(recomputed_spin).to_string(),
                    declared_attack: declared,
                    recomputed_attack,
                });
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartResendAudit {
    pub reason: String,
    pub board_hash: u64,
    pub turn: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttaxAudit {
    pub info_name: String,
    pub info_version: String,
    pub info_author: String,
    pub command_line: String,
    pub bun_version: Option<String>,
    pub commit: Option<String>,
    pub decision_log: Option<String>,
    pub decisions_logged: u64,
    pub nodes_total: u64,
    pub extra_ms_total: f64,
    pub suggestions_with_stats: u64,
    pub cells_above_row20: bool,
    pub hold_matches: u64,
    pub hold_mismatches: u64,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct PlayerTotals {
    pub sent_total: u32,
    pub received_total: u32,
    pub attack_total: u32,
    pub pieces_placed: u32,
    pub lines_cleared: u32,
}

/// Board-height samples for one player (one sample per turn, taken at the start
/// of the turn). Used by the diagnostic profiles to report the stack profile.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct HeightStats {
    pub max: u32,
    pub mean: f64,
    pub final_height: u32,
}

impl HeightStats {
    pub fn from_samples(samples: &[u32]) -> Self {
        if samples.is_empty() {
            return Self::default();
        }
        let max = *samples.iter().max().unwrap_or(&0);
        let mean = (samples.iter().map(|&value| f64::from(value)).sum::<f64>()
            / samples.len() as f64
            * 100.0)
            .round()
            / 100.0;
        Self {
            max,
            mean,
            final_height: *samples.last().unwrap_or(&0),
        }
    }
}

/// Per-turn search statistics (protocol v1.31 §8.4.1).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SearchTurn {
    pub turn: u32,
    #[serde(default)]
    pub depth: Option<u32>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub nodes: Option<u64>,
    #[serde(default)]
    pub fallback: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameAudit {
    pub seed: u64,
    pub sheet: String,
    pub started_at: String,
    /// Match profile (protocol v1.11 §1.3).
    pub profile: String,
    pub winner: Option<u8>,
    pub reason: String,
    pub detail: Option<String>,
    /// Forfeit cause (protocol v1.8 §7.1); `None` for non-forfeit results.
    pub forfeit_cause: Option<crate::replay::ForfeitCause>,
    /// Slot that forfeited (no-contest); `None` otherwise.
    #[serde(default)]
    pub forfeit_slot: Option<u8>,
    pub turns: u32,
    /// Surge releases per player (S2 B2B charging payoff).
    #[serde(default)]
    pub surge_released: [u32; 2],
    /// Maximum B2B streak reached per player.
    #[serde(default)]
    pub max_b2b_streak: [u32; 2],
    /// Turns at or above the surge threshold (B2B >= 4) per player.
    #[serde(default)]
    pub turns_with_b2b_ge_4: [u32; 2],
    pub totals: [PlayerTotals; 2],
    /// Per-turn board height samples (diagnostic profiles).
    pub heights: [HeightStats; 2],
    pub think: [ThinkStats; 2],
    pub first_choice: [FirstChoiceAudit; 2],
    pub scoring: [ScoringAudit; 2],
    pub reachability_rejections: u32,
    pub start_resends: Vec<StartResendAudit>,
    pub attax: Option<AttaxAudit>,
    pub bots: [crate::replay::BotRecord; 2],
    pub replay: String,
    pub verify: crate::replay::VerifyReport,
    /// Host-side queue invariant violations (must be zero).
    pub queue_invariant_violations: u64,
    /// intetrigence search calls that used the direct-placement fallback.
    pub bot_fallbacks: u64,
    /// intetrigence search nodes summed over the round.
    pub bot_nodes_total: u64,
    /// Per-turn search statistics per slot (protocol v1.31 §8.4.1). Empty for
    /// TBP opponents and for the baseline.
    #[serde(default)]
    pub search: [Vec<SearchTurn>; 2],
    /// Lines cleared in the opener phase (the first 14 placements per slot).
    #[serde(default)]
    pub opener_lines: [u32; 2],
    /// Placements with a non-none spin label per slot.
    #[serde(default)]
    pub spin_placements: [u32; 2],
}

impl GameAudit {
    pub fn write(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        std::fs::write(path, json)
    }
}

/// Converts a [`AgentAudit`] plus the host info into the attax audit block.
pub fn attax_audit(agent: &AgentAudit, commit: Option<String>) -> AttaxAudit {
    AttaxAudit {
        info_name: agent.info_name.clone(),
        info_version: agent.info_version.clone(),
        info_author: agent.info_author.clone(),
        command_line: agent.command_line.clone().unwrap_or_default(),
        bun_version: agent.bun_version.clone(),
        commit,
        decision_log: agent.decision_log.clone(),
        decisions_logged: agent.decisions_logged,
        nodes_total: agent.nodes_total,
        extra_ms_total: round3(agent.extra_ms_total),
        suggestions_with_stats: agent.suggestions_with_stats,
        cells_above_row20: agent.cells_above_row20,
        hold_matches: agent.hold_matches,
        hold_mismatches: agent.hold_mismatches,
    }
}

/// Wilson score interval for a binomial proportion (95% with `z = 1.959964`).
pub fn wilson_interval(wins: u32, games: u32, z: f64) -> (f64, f64) {
    if games == 0 {
        return (0.0, 1.0);
    }
    let n = games as f64;
    let p = wins as f64 / n;
    let z2 = z * z;
    let denominator = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denominator;
    let margin = z * ((p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt()) / denominator;
    ((center - margin).max(0.0), (center + margin).min(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn think_stats_are_nearest_rank() {
        let samples: Vec<f64> = (1..=20).map(|value| value as f64).collect();
        let stats = ThinkStats::from_samples(&samples);
        assert_eq!(stats.count, 20);
        assert_eq!(stats.median_ms, 10.5);
        assert_eq!(stats.p95_ms, 19.0);
        assert_eq!(stats.p99_ms, 20.0);
        assert_eq!(stats.max_ms, 20.0);
    }

    #[test]
    fn first_choice_histogram_counts_non_zero_indices() {
        let audit = FirstChoiceAudit::from_indices(&[0, 0, 1, 2, 1]);
        assert_eq!(audit.total, 5);
        assert_eq!(audit.accepted_first, 2);
        assert_eq!(audit.rate, 0.4);
        assert_eq!(audit.rejected_index_histogram.get(&1), Some(&2));
        assert_eq!(audit.rejected_index_histogram.get(&2), Some(&1));
    }

    #[test]
    fn wilson_interval_matches_known_values() {
        let (low, high) = wilson_interval(90, 100, 1.959_964);
        assert!((low - 0.8246).abs() < 0.002, "low = {low}");
        assert!((high - 0.9463).abs() < 0.002, "high = {high}");
        assert_eq!(wilson_interval(0, 0, 1.96), (0.0, 1.0));
    }
}
