//! intetrigence arena: S2 referee, TBP host integration, benchmark harness,
//! replay verification and audit logging.
//!
//! The protocol of record is `docs/s2-referee-protocol.md` (v1); rule values are
//! `docs/s2-spec.md`. The referee owns turns, seat order, placement validation,
//! garbage flow, replay records and the audit; `crates/core` owns the rules and
//! receives `frame` as an argument.
//!
//! Module map:
//! - [`engine`]: per-player state, placement application, scoring, garbage.
//! - [`movegen`]: exact reachability and the canonical action path used to
//!   apply a placement through `crates/core`.
//! - [`referee`]: paired turns, seat order, adoption, replay records.
//! - [`replay`]: JSONL records and `verify-replay`.
//! - [`audit`]: think time, first-choice acceptance, scoring audit, Wilson CI.
//! - [`agent`] / [`baseline`] / [`tbp_agent`]: bot interface, the M0 in-process
//!   stand-in and the TBP host adapter.
//! - [`match`]: fixed seeds × both sheets, replay verification and summary.

#![forbid(unsafe_code)]

pub mod agent;
pub mod attax_log;
pub mod audit;
pub mod baseline;
pub mod bot_agent;
mod build_fingerprint;
pub mod engine;
mod evaluation_seeds;
pub mod font;
pub mod harness;
pub mod measurement_lock;
pub mod movegen;
pub mod referee;
pub mod render;
pub mod replay;
pub mod round;
pub mod tbp_agent;

/// Content hash of the workspace's Rust sources and dependency manifests,
/// embedded by `build.rs`.
pub const BUILD_ID: &str = env!("ARENA_BUILD_ID");

/// The running binary's path, size and mtime, for the audit trail.
pub fn binary_info() -> String {
    let Ok(path) = std::env::current_exe() else {
        return "unknown".into();
    };
    let Ok(metadata) = std::fs::metadata(&path) else {
        return path.display().to_string();
    };
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|delta| delta.as_secs())
        .unwrap_or(0);
    format!(
        "{} ({} bytes, mtime {modified})",
        path.display(),
        metadata.len()
    )
}
pub use agent::{Agent, AgentAudit, AgentError, BotDescriptor, Candidate, Commit, Suggestion};
pub use audit::{GameAudit, ThinkStats, wilson_interval};
pub use baseline::BaselineBot;
pub use bot_agent::{BotAgent, BotAgentConfig};
pub use engine::{Engine, Sheet, View};
pub use harness::{AttaxConfig, MatchConfig, MatchSummary, run_match};
pub use referee::{RoundConfig, RoundOutcome, RoundReport, run_round};
pub use render::{RenderOptions, RenderReport, render_replay};
pub use replay::{ReplayRecord, VerifyReport, verify_replay, write_replay};
pub use tbp_agent::{TbpAgent, TbpAgentConfig};
