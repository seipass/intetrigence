//! End-to-end referee tests: a short round with two in-process bots, a JSONL
//! replay, and `verify-replay` (including tamper detection).

use intetrigence_arena::agent::Agent;
use intetrigence_arena::baseline::BaselineBot;
use intetrigence_arena::engine::{Profile, Sheet};
use intetrigence_arena::referee::{RoundConfig, run_round};
use intetrigence_arena::replay::{ReplayRecord, verify_replay, write_replay};

fn config(seed: u64, sheet: Sheet, turn_cap: u32) -> RoundConfig {
    RoundConfig {
        seed,
        sheet,
        turn_cap,
        frames_per_turn: 12,
        rules: "docs/s2-spec.md@v1.3.11".into(),
        protocol: "docs/s2-referee-protocol.md@v1.9".into(),
        started_at: "test".into(),
        profile: Profile::Official,
        deterministic: false,
    }
}

#[test]
fn short_round_is_replay_verifiable_on_both_sheets() {
    for sheet in Sheet::ALL {
        let mut first = BaselineBot::new();
        let mut second = BaselineBot::with_name("opponent");
        let mut agents: [&mut dyn Agent; 2] = [&mut first, &mut second];
        let report = run_round(&config(42, sheet, 40), &mut agents).expect("round must run");
        assert!(report.outcome.turns > 0);
        assert!(!report.records.is_empty());

        let path = std::env::temp_dir().join(format!(
            "intetrigence-arena-round-{}-{}.jsonl",
            sheet.as_str(),
            std::process::id()
        ));
        write_replay(&path, &report.records).expect("write replay");
        let verify = verify_replay(&path).expect("verify replay");
        assert!(
            verify.ok,
            "sheet {} replay mismatches: {:?}",
            sheet.as_str(),
            verify.mismatches
        );
        assert_eq!(verify.turns, report.outcome.turns);
        assert!(verify.placements >= report.outcome.turns as u32);
        let _ = std::fs::remove_file(&path);
    }
}

#[test]
fn tampered_replay_is_rejected() {
    let mut first = BaselineBot::new();
    let mut second = BaselineBot::with_name("opponent");
    let mut agents: [&mut dyn Agent; 2] = [&mut first, &mut second];
    let report = run_round(&config(7, Sheet::A, 20), &mut agents).expect("round must run");

    let mut records = report.records.clone();
    let mut tampered = false;
    for record in &mut records {
        if let ReplayRecord::Turn(turn) = record {
            if let Some(placement) = turn.placements.first_mut() {
                placement.sent = placement.sent.wrapping_add(1);
                tampered = true;
                break;
            }
        }
    }
    assert!(tampered, "the round must contain a placement");

    let path = std::env::temp_dir().join(format!(
        "intetrigence-arena-tampered-{}.jsonl",
        std::process::id()
    ));
    write_replay(&path, &records).expect("write replay");
    let verify = verify_replay(&path).expect("verify replay");
    assert!(!verify.ok, "tampering must be detected");
    assert!(
        verify
            .mismatches
            .iter()
            .any(|mismatch| mismatch.contains("sent")),
        "expected a sent mismatch, got {:?}",
        verify.mismatches
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn audit_records_think_time_and_first_choice() {
    let mut first = BaselineBot::new();
    let mut second = BaselineBot::with_name("opponent");
    let mut agents: [&mut dyn Agent; 2] = [&mut first, &mut second];
    let report = run_round(&config(11, Sheet::B, 12), &mut agents).expect("round must run");
    for slot in 0..2 {
        assert!(report.audit.think[slot].count > 0);
        assert!(report.audit.think[slot].median_ms >= 0.0);
        assert_eq!(report.audit.first_choice[slot].total, report.outcome.turns);
        assert_eq!(report.audit.reachability_rejections, 0);
        assert_eq!(report.audit.scoring[slot].placements, report.outcome.turns);
    }
}
