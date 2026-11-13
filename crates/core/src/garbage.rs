//! Garbage queue interface.
//!
//! **Status: interface only.** PLAN.md §3.1 fixes the high-level behaviour
//! (change-on-attack, queue + delayed insertion, outgoing attack cancels
//! queued garbage first, cap/delay constants), but the constants and the
//! change-on-attack schedule wait for `docs/s2-spec.md`.
//!
//! Direct garbage-row insertion is already implemented on
//! [`crate::board::Board`] and [`crate::game::Game`]; this module is only the
//! versus queue policy.

/// One queued garbage delivery.
///
/// Fields are provisional until the spec is frozen; `id` is intended for
/// change-on-attack grouping and replay auditing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GarbageEntry {
    pub amount: u32,
    pub hole: u8,
    pub id: u64,
}

/// Per-player incoming garbage queue.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GarbageQueue {
    entries: Vec<GarbageEntry>,
    pending: u32,
}

impl GarbageQueue {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            pending: 0,
        }
    }

    pub fn pending(&self) -> u32 {
        self.pending
    }

    pub fn entries(&self) -> &[GarbageEntry] {
        &self.entries
    }

    /// TODO(s2-spec): enqueue incoming attack with change-on-attack hole
    /// assignment and the cap.
    pub fn receive(&mut self, _attack: u32, _holes: &[u8]) {
        todo!("TODO(s2-spec): garbage queue receive / change-on-attack")
    }

    /// TODO(s2-spec): release garbage whose delay has elapsed.
    pub fn tick(&mut self, _elapsed_ms: u64) -> Vec<GarbageEntry> {
        todo!("TODO(s2-spec): garbage queue delay / release")
    }

    /// TODO(s2-spec): cancel queued garbage with outgoing attack and return
    /// the uncancelled remainder.
    pub fn cancel(&mut self, _incoming: u32) -> u32 {
        todo!("TODO(s2-spec): garbage queue cancellation")
    }
}
