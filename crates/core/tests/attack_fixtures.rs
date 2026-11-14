//! Mechanical verification against `docs/s2-attack-examples.json` (18 cases).

use std::fs;
use std::path::Path;

use intetrigence_core::{ClearInput, SpinType, compute_attack};
use serde_json::Value;

fn fixture(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs")
        .join(name);
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

#[test]
fn attack_fixtures_match_18_of_18() {
    let root = fixture("s2-attack-examples.json");
    assert_eq!(
        root["schema"].as_str(),
        Some("intetrigence/s2-attack-examples/1")
    );

    let cases = root["cases"].as_array().expect("cases array");
    // The spec lead may append cases (e.g. the 0-line combo regression).
    // Run every case; only require the original 18 to be present.
    assert!(
        cases.len() >= 18,
        "expected at least the 18 reference cases, got {}",
        cases.len()
    );

    for case in cases {
        let id = case["id"].as_str().expect("case id");
        let input = &case["input"];
        let expected = &case["expected"];

        let spin_name = input["spin"].as_str().expect("spin");
        let spin = SpinType::from_name(spin_name)
            .unwrap_or_else(|| panic!("{id}: unknown spin {spin_name}"));

        let clear = ClearInput {
            lines: input["lines"].as_u64().unwrap() as u8,
            spin,
            combo: input["combo"].as_u64().unwrap() as u32,
            b2b_streak: input["b2bStreak"].as_u64().unwrap() as u32,
            perfect_clear: input["perfectClear"].as_bool().unwrap(),
            garbage_cleared: input["garbageCleared"].as_u64().unwrap() as u32,
            multiplier: input["multiplier"].as_f64().unwrap(),
        };

        let actual = compute_attack(&clear);
        let expected_events: Vec<u32> = expected["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_u64().unwrap() as u32)
            .collect();

        assert_eq!(actual.events, expected_events, "{id}: events");
        assert_eq!(
            actual.total,
            expected["total"].as_u64().unwrap() as u32,
            "{id}: total"
        );
        assert_eq!(
            actual.combo_after,
            expected["comboAfter"].as_u64().unwrap() as u32,
            "{id}: comboAfter"
        );
        assert_eq!(
            actual.b2b_streak_after,
            expected["b2bStreakAfter"].as_u64().unwrap() as u32,
            "{id}: b2bStreakAfter"
        );

        let expected_base = expected["base"].as_f64().unwrap();
        assert!(
            (actual.base - expected_base).abs() < 1e-12,
            "{id}: base {} vs {expected_base}",
            actual.base
        );
        assert_eq!(
            actual.gsb,
            expected["gsb"].as_u64().unwrap() as u32,
            "{id}: gsb"
        );
        assert_eq!(
            actual.surge_amount,
            expected["surgeAmount"].as_u64().unwrap() as u32,
            "{id}: surgeAmount"
        );
    }
}
