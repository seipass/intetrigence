//! intetrigence bot: exact movegen, anytime search and S2 evaluation.
//!
//! Implemented so far (PLAN.md §4):
//! - [`movegen`]: exact enumeration of every lockable placement over the input
//!   space (shift / soft drop / SRS+ rotation+kick, including 180°), with the
//!   replayable [`movegen::Placement::actions`] sequence and the spin entry
//!   flags. The All-Mini+ spin classification and the all-spin test are the
//!   core engine's (`Board::spin_type` / `Board::is_immobile`); the bot keeps
//!   no second copy of the corner table.
//!
//! Still to come: multithreaded anytime search, Dellacherie-style board
//! features plus S2 versus terms, and SPSA-tuned weights.

#![forbid(unsafe_code)]

pub mod movegen;

pub use movegen::{
    Arrival, Placement, State, generate, generate_from, reachable_states, spawn_piece,
};
