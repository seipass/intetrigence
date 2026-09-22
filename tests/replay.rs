use intetrigence::{
    arena::{self, MatchAgent},
    battle,
    game::RulesProfile,
    hoiko::HoikoConfig,
    replay,
    search::Budget,
};
use std::path::Path;
#[test]
fn replay_reproduces_and_rejects_corruption() {
    let mut data = Vec::new();
    arena::run(
        132,
        false,
        3,
        Budget {
            milliseconds: 0,
            iterations: 1,
        },
        None,
        Some(&mut data),
    )
    .unwrap();
    assert_eq!(replay::verify(&data[..]).unwrap(), 1);
    let text = String::from_utf8(data).unwrap();
    let lines: Vec<_> = text.lines().collect();
    let start: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(start["replay_version"], serde_json::json!(2));
    assert_eq!(start["agents"][0], serde_json::json!("intetrigence"));
    assert_eq!(
        start["initial"][0]["agent"],
        serde_json::json!("intetrigence")
    );
    assert_eq!(start["initial"][0]["next"].as_array().unwrap().len(), 5);
    let first_move: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert!(first_move["board_cells"].is_array());
    assert!(first_move["hold_before"].is_null() || first_move["hold_before"].is_string());
    assert_eq!(first_move["next_before"].as_array().unwrap().len(), 5);
    assert!(
        replay::verify(lines[..lines.len() - 1].join("\n").as_bytes())
            .unwrap_err()
            .contains("truncated")
    );
    let clean_records: Vec<serde_json::Value> = lines
        .iter()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    let mut records = clean_records.clone();
    records[1]["board"][0] = serde_json::json!(9999);
    let corrupted = records
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(replay::verify(corrupted.as_bytes())
        .unwrap_err()
        .contains("board mismatch"));
    let mut cell_corrupted = clean_records;
    cell_corrupted[1]["board_cells"][0][0] = serde_json::json!("garbage");
    let corrupted_cells = cell_corrupted
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(replay::verify(corrupted_cells.as_bytes())
        .unwrap_err()
        .contains("board cell kinds mismatch"));
    assert!(replay::verify(&b""[..]).is_err());
}

#[test]
fn pps_rejects_invalid_values_and_keeps_replay_verifiable() {
    assert!(arena::run_with_pps(
        1,
        false,
        1,
        Budget {
            milliseconds: 0,
            iterations: 1
        },
        None,
        Some(0.0),
        None,
    )
    .is_err());
    let mut log = Vec::new();
    arena::run_with_pps(
        1,
        false,
        1,
        Budget {
            milliseconds: 0,
            iterations: 1,
        },
        None,
        Some(1_000.0),
        Some(&mut log),
    )
    .unwrap();
    assert_eq!(replay::verify(&log[..]).unwrap(), 1);
}

#[test]
fn movement_battle_replay_validates_every_input_frame() {
    let mut log = Vec::new();
    let result = battle::run_matchup(
        77,
        false,
        MatchAgent::Intetrigence,
        MatchAgent::Intetrigence,
        3,
        Budget {
            milliseconds: 0,
            iterations: 1,
        },
        RulesProfile::Arena,
        None,
        &HoikoConfig::default(),
        Path::new("unused"),
        None,
        false,
        Some(&mut log),
    )
    .unwrap();
    assert!(result.frames > 0);
    assert_eq!(result.pieces.into_iter().max(), Some(3));
    assert_eq!(replay::verify(&log[..]).unwrap(), 1);

    let text = String::from_utf8(log).unwrap();
    let mut records: Vec<serde_json::Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let movement = records
        .iter_mut()
        .find(|record| record["type"] == "battle_move")
        .unwrap();
    movement["inputs"][0] = serde_json::json!("left");
    let corrupted = records
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(replay::verify(corrupted.as_bytes())
        .unwrap_err()
        .contains("invalid movement input path"));
}
