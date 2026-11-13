//! intetrigence bot: exact movegen, anytime search and S2 evaluation.
//!
//! Implemented so far (PLAN.md §4):
//! - [`movegen`]: exact enumeration of every lockable placement over the input
//!   space (shift / soft drop / SRS+ rotation+kick, including 180°), with the
//!   replayable [`movegen::Placement::actions`] sequence and the spin entry
//!   flags;
//! - [`spin`]: the All-Mini+ classification (`docs/s2-spec.md` §4).
//!
//! Still to come: multithreaded anytime search, Dellacherie-style board
//! features plus S2 versus terms, and SPSA-tuned weights.

#![forbid(unsafe_code)]

pub mod movegen;
pub mod spin;

pub use movegen::{
    Arrival, Placement, State, generate, generate_from, reachable_states, spawn_piece,
};
pub use spin::{classify_spin, is_immobile};
