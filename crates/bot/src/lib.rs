//! intetrigence bot: exact movegen, anytime search and S2 evaluation.
//!
//! Skeleton only. The implementation lands after the core S2 interfaces are
//! frozen against `docs/s2-spec.md`.
//!
//! Planned responsibilities (PLAN.md §4):
//! - exact movegen over the input space (shift / soft drop / SRS+ rotation+kick,
//!   including 180°)
//! - multithreaded anytime search
//! - Dellacherie-style board features plus S2 versus terms
//! - tunable weights (SPSA)

#![forbid(unsafe_code)]
