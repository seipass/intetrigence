//! Garbage queue tests (`docs/s2-spec.md` §7).

use intetrigence_core::{GarbageQueue, OpenerPhase};

#[test]
fn delay_is_20_frames() {
    let mut queue = GarbageQueue::new(1);
    queue.receive(2, 100);

    assert!(queue.take_insertable(0, 119).is_empty());
    let holes = queue.take_insertable(0, 120);
    assert_eq!(holes.len(), 2);
    assert_eq!(queue.queue_total(), 0);
}

#[test]
fn cap_is_8_rows_per_lock() {
    let mut queue = GarbageQueue::new(1);
    queue.receive(20, 0);

    let holes = queue.take_insertable(0, 20);
    assert_eq!(holes.len(), 8);
    assert_eq!(queue.queue_total(), 12);
}

#[test]
fn combo_blocking_skips_insertion_after_a_clear() {
    let mut queue = GarbageQueue::new(1);
    queue.receive(3, 0);

    assert!(queue.take_insertable(1, 20).is_empty());
    assert_eq!(queue.queue_total(), 3);
}

#[test]
fn cancellation_consumes_send_before_the_opener_budget() {
    let mut queue = GarbageQueue::new(1);
    queue.receive(5, 0);

    let result = queue.cancel(3, 0);
    assert_eq!(result.sent, 0);
    assert_eq!(result.cancelled, 0);
    assert_eq!(result.queue_total, 2);

    let result = queue.cancel(10, 0);
    assert_eq!(result.sent, 8);
    assert_eq!(result.queue_total, 0);
}

#[test]
fn opener_doubles_cancellation_but_not_send() {
    let mut queue = GarbageQueue::new(1);
    queue.receive(4, 0);
    let mut opener = OpenerPhase::new();

    let result = queue.cancel_with_opener(1, &mut opener);
    assert_eq!(result.sent, 0);
    assert_eq!(result.cancelled, 1);
    assert_eq!(result.queue_total, 2);
    assert_eq!(opener.sent_total, 0);
}

#[test]
fn opener_is_inactive_from_the_15th_placement() {
    let mut queue = GarbageQueue::new(1);
    queue.receive(4, 0);
    let mut opener = OpenerPhase::new();
    opener.pieces_placed = 14;

    let result = queue.cancel_with_opener(1, &mut opener);
    assert_eq!(result.sent, 0);
    assert_eq!(result.cancelled, 0);
    assert_eq!(result.queue_total, 3);
}

#[test]
fn opener_uses_queue_total_ge_sent_total() {
    let mut queue = GarbageQueue::new(1);
    queue.receive(4, 0);
    let mut opener = OpenerPhase::new();
    opener.on_sent(4);

    let result = queue.cancel_with_opener(1, &mut opener);
    assert_eq!(result.cancelled, 1);
    assert_eq!(result.queue_total, 2);
}

#[test]
fn opener_does_not_trigger_when_queue_is_smaller_than_sent() {
    let mut queue = GarbageQueue::new(1);
    queue.receive(4, 0);
    let mut opener = OpenerPhase::new();
    opener.on_sent(5);

    let result = queue.cancel_with_opener(1, &mut opener);
    assert_eq!(result.sent, 0);
    assert_eq!(result.cancelled, 0);
    assert_eq!(result.queue_total, 3);
}

#[test]
fn all_rows_in_one_packet_use_the_same_column() {
    let mut queue = GarbageQueue::new(42);
    queue.receive(4, 0);

    let holes = queue.take_insertable(0, 20);
    assert_eq!(holes.len(), 4);
    assert!(holes.windows(2).all(|pair| pair[0] == pair[1]), "{holes:?}");
}

#[test]
fn packets_are_sorted_by_arrival_and_stable_for_equal_frames() {
    let mut queue = GarbageQueue::new(1);
    queue.receive(1, 10);
    queue.receive(2, 10);
    queue.receive(3, 5);

    let packets = queue.packets();
    assert_eq!(
        packets
            .iter()
            .map(|packet| packet.arrival_frame)
            .collect::<Vec<_>>(),
        vec![5, 10, 10]
    );
    assert_eq!(
        packets
            .iter()
            .map(|packet| packet.amount)
            .collect::<Vec<_>>(),
        vec![3, 1, 2]
    );
}

#[test]
fn same_seed_produces_the_same_holes() {
    let mut a = GarbageQueue::new(7);
    let mut b = GarbageQueue::new(7);
    for queue in [&mut a, &mut b] {
        queue.receive(3, 0);
        queue.receive(2, 0);
    }

    assert_eq!(a.take_insertable(0, 20), b.take_insertable(0, 20));
    assert_eq!(a.take_insertable(0, 20), b.take_insertable(0, 20));
}

#[test]
fn columns_stay_in_range() {
    let mut queue = GarbageQueue::new(99);
    queue.receive(100, 0);

    let mut holes = Vec::new();
    while queue.queue_total() > 0 {
        holes.extend(queue.take_insertable(0, 20));
    }
    assert_eq!(holes.len(), 100);
    assert!(holes.iter().all(|&hole| hole < 10), "{holes:?}");
}

#[test]
fn packet_completion_rerolls_the_column() {
    let mut queue = GarbageQueue::new(42);
    queue.receive(2, 0);
    queue.receive(2, 0);

    let holes = queue.take_insertable(0, 20);
    assert_eq!(holes.len(), 4);
    assert_eq!(holes[0], holes[1]);
    assert_eq!(holes[2], holes[3]);
}

#[test]
fn zero_amount_packets_are_ignored() {
    let mut queue = GarbageQueue::new(1);
    queue.receive(0, 0);
    assert_eq!(queue.queue_total(), 0);
    assert!(queue.packets().is_empty());
}
