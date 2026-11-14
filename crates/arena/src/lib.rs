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
pub mod engine;
pub mod font;
pub mod harness;
pub mod movegen;
pub mod referee;
pub mod render;
pub mod replay;
pub mod round;
pub mod tbp_agent;

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
