//! intetrigence arena: S2 referee, TBP host, benchmark harness, replay
//! verification and audit logging.
//!
//! Skeleton only. The implementation lands after the core S2 interfaces are
//! frozen against `docs/s2-spec.md`.
//!
//! Planned responsibilities (PLAN.md §3.3, §4):
//! - S2 referee (paired turns, 5 PPS, 1000 turn cap, both seats per seed)
//! - TBP host / agreed attax connection interface
//! - think-time recording (median / p95), first-choice acceptance log
//! - verify-replay from recorded actions and scoring audit

#![forbid(unsafe_code)]
