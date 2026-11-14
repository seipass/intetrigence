//! Garbage queue per `docs/s2-spec.md` v1.3.1 §7.
//!
//! The queue is a pure versus-policy object: it owns packet arrival, opener
//! cancellation, the 8-row cap, the 20-frame delay and the change-on-attack
//! hole RNG. The caller applies the returned hole columns to a [`crate::Board`]
//! (or [`crate::Game`]) and decides top-out from the resulting position.
//!
//! Change-on-attack TL behavior: every row in one packet uses the same hole
//! column; the column is rerolled (always, `messiness_change = 1.0`) when the
//! packet is consumed. The MINSTD draw order is part of the replay contract.

use crate::constants::{GARBAGE_CAP, GARBAGE_SPEED_FRAMES, MESSINESS_CHANGE, MESSINESS_WITHIN};
use crate::opener::OpenerPhase;
use crate::rng::MinStd;

/// One queued attack packet. `arrival_frame` is the frame at send time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GarbagePacket {
    pub amount: u32,
    pub arrival_frame: u64,
}

/// Result of cancelling one attack event against the queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CancelResult {
    /// Rows that still go to the opponent.
    pub sent: u32,
    /// Rows removed by the opener's extra cancellation budget.
    pub cancelled: u32,
    /// Queue total after cancellation.
    pub queue_total: u32,
}

/// Per-player incoming garbage queue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GarbageQueue {
    packets: Vec<GarbagePacket>,
    rng: MinStd,
    last_column: Option<u8>,
    has_changed_column: bool,
}

impl GarbageQueue {
    pub fn new(seed: u64) -> Self {
        Self {
            packets: Vec::new(),
            rng: MinStd::new(seed),
            last_column: None,
            has_changed_column: false,
        }
    }

    pub fn packets(&self) -> &[GarbagePacket] {
        &self.packets
    }

    /// Total pending rows, including packets that have not arrived yet.
    pub fn queue_total(&self) -> u32 {
        self.packets.iter().map(|packet| packet.amount).sum()
    }

    pub fn pending(&self) -> u32 {
        self.queue_total()
    }

    /// Pending rows that have already arrived, i.e. the rows that
    /// [`Self::take_insertable`] could insert on the next lock.
    ///
    /// `queue_total` counts packets still in flight, which cannot be inserted
    /// yet; using it to price defence would over-value a clear while a burst is
    /// still travelling.
    pub fn arrived_total(&self, current_frame: u64) -> u32 {
        self.packets
            .iter()
            .filter(|packet| {
                packet.arrival_frame.saturating_add(GARBAGE_SPEED_FRAMES) <= current_frame
            })
            .map(|packet| packet.amount)
            .sum()
    }

    pub fn last_column(&self) -> Option<u8> {
        self.last_column
    }

    /// Enqueues one attack packet. Packets are kept sorted by arrival frame;
    /// equal frames preserve insertion order (stable sort).
    pub fn receive(&mut self, amount: u32, arrival_frame: u64) {
        if amount == 0 {
            return;
        }
        self.packets.push(GarbagePacket {
            amount,
            arrival_frame,
        });
        self.packets.sort_by_key(|packet| packet.arrival_frame);
    }

    /// Cancels `attack` against the front of the queue.
    ///
    /// `cancel_budget` is the opener's extra budget (see
    /// [`OpenerPhase::cancel_budget`]). The send side is consumed first, then
    /// the extra budget. Returns the rows that still go to the opponent.
    pub fn cancel(&mut self, attack: u32, cancel_budget: u32) -> CancelResult {
        let mut send = attack;
        let mut cancel = cancel_budget;
        let mut cancelled = 0;

        while (send > 0 || cancel > 0) && !self.packets.is_empty() {
            let packet = &mut self.packets[0];
            packet.amount -= 1;

            if send > 0 {
                send -= 1;
            } else {
                cancel -= 1;
                cancelled += 1;
            }

            if packet.amount == 0 {
                self.packets.remove(0);
                self.on_packet_consumed();
            }
        }

        CancelResult {
            sent: send,
            cancelled,
            queue_total: self.queue_total(),
        }
    }

    /// Cancels one attack event with the opener condition applied and updates
    /// the opener's `sent_total`.
    pub fn cancel_with_opener(&mut self, attack: u32, opener: &mut OpenerPhase) -> CancelResult {
        let budget = opener.cancel_budget(attack, self.queue_total());
        let result = self.cancel(attack, budget);
        opener.on_sent(result.sent);
        result
    }

    /// Returns the hole columns for garbage that may be inserted after a lock.
    ///
    /// `lines_cleared` implements combo blocking: a lock that cleared at least
    /// one line inserts nothing. At most [`GARBAGE_CAP`] rows are returned.
    /// A packet is eligible only when `arrival_frame + 20 <= current_frame`.
    pub fn take_insertable(&mut self, lines_cleared: u8, current_frame: u64) -> Vec<u8> {
        if lines_cleared > 0 {
            return Vec::new();
        }

        self.packets.sort_by_key(|packet| packet.arrival_frame);

        let mut holes = Vec::new();
        while holes.len() < GARBAGE_CAP as usize && !self.packets.is_empty() {
            let packet = self.packets[0];
            if packet.arrival_frame.saturating_add(GARBAGE_SPEED_FRAMES) > current_frame {
                break;
            }

            holes.push(self.next_hole());

            let packet = &mut self.packets[0];
            packet.amount -= 1;
            if packet.amount == 0 {
                self.packets.remove(0);
                self.on_packet_consumed();
            }
        }
        holes
    }

    /// One row insertion's column decision (`docs/s2-spec.md` §7.6).
    fn next_hole(&mut self) -> u8 {
        let mut column = self.last_column;
        let should_reroll = match column {
            None => true,
            // within = 0.0: consume the draw, always false.
            Some(_) => self.rng.next_float() < MESSINESS_WITHIN,
        };

        if should_reroll && !self.has_changed_column {
            column = Some(self.reroll_column());
            self.has_changed_column = true;
        }

        let column = column.expect("a hole column must be available");
        self.has_changed_column = false;
        column
    }

    /// Packet completion: always reroll in TL (`messiness_change = 1.0`).
    fn on_packet_consumed(&mut self) {
        if self.rng.next_float() < MESSINESS_CHANGE {
            self.reroll_column();
            self.has_changed_column = true;
        }
    }

    fn reroll_column(&mut self) -> u8 {
        let column = (self.rng.next_float() * 10.0).floor() as u8;
        self.last_column = Some(column);
        column
    }
}
