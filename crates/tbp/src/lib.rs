//! intetrigence TBP: Tetris Bot Protocol message interface and host.
//!
//! Scope (`docs/attax-integration.md` §8):
//! - [`message`]: wire types for `info` / `rules` / `ready` / `start` /
//!   `suggest` / `suggestion` / `play` / `new_piece` / `stop` / `quit`.
//! - [`geometry`]: TBP / Cold Clear location <-> `crates/core` placement
//!   conversion by cell-set matching.
//! - [`host`]: process spawn, line-delimited JSON I/O, handshake, think-time
//!   measurement, stderr capture and clean shutdown (checklists A and B).
//! - [`mirror`]: the adapter's own queue/hold transition, used to keep the
//!   host's six-entry queue invariant (checklist D5).
//!
//! The referee owns legality, scoring and garbage; this crate only speaks the
//! protocol and never trusts a bot's spin label.

#![forbid(unsafe_code)]

pub mod geometry;
pub mod host;
pub mod message;
pub mod mirror;

pub use geometry::{
    cc_cells, cells_key, core_cells, core_placement, location_cells, location_of_core,
    orientation_of, rotate_cell, rotation_of,
};
pub use host::{Host, HostConfig, HostError, StartResend, ThinkTimeStats};
pub use message::{
    BotMessage, HostMessage, InfoPayload, Location, MessageError, Move, MoveInfo, NewPiecePayload,
    Orientation, Piece, PlayPayload, SpinLabel, StartPayload, SuggestionPayload,
};
pub use mirror::Mirror;
