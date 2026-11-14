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
//! - [`eval`]: board features, S2 placement rewards (attack, B2B, combo,
//!   surge, opener) and the T-slot cutout;
//! - [`search`]: anytime beam search with hold, bag prediction, a
//!   transposition graph and a worker pool;
//! - [`play`]: a self-contained single-player driver that plays a legal game
//!   through the engine and records think times.
//!
//! Still to come: S2 versus terms (garbage queue danger) and SPSA-tuned
//! weights (M3).

#![forbid(unsafe_code)]

pub mod eval;
pub mod movegen;
pub mod play;
pub mod search;

pub use eval::{
    EvalContext, Features, Reward, Weights, analyse, apply_placement, features, leaf_value,
    t_slot_cutout,
};
pub use movegen::{
    ActionSeq, Arrival, Movegen, Placement, State, generate, generate_from, reachable_states,
    spawn_piece, unique_by_cells,
};
pub use play::{GameReport, SearchStats, play_game, play_game_with_garbage, play_game_with_setup};
pub use search::{
    Candidate, MAX_THREADS, Move, PieceSequence, RootState, SearchConfig, SearchResult, Searcher,
};
