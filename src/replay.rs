//! Replay the recorded moves without rerunning the stochastic search.
use crate::{
    arena::MatchResult,
    battle::BattleResult,
    game::{Game, Outcome, RulesProfile},
    timing::{trace_movement, validate_movement_plan, BattleInput, TimingProfile},
};
use intetrigence_engine::data::Placement;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::Serialize;
use serde_json::Value;
use std::io::BufRead;

pub fn verify(reader: impl BufRead) -> Result<usize, String> {
    let mut session: Option<Session> = None;
    let mut battle_session: Option<BattleSession> = None;
    let mut verified = 0;
    for (line_number, line) in reader.lines().enumerate() {
        let result = (|| {
            let v: Value = serde_json::from_str(&line.map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            match v["type"].as_str() {
                Some("start") => {
                    if session.is_some() || battle_session.is_some() {
                        return Err("new match before previous result".into());
                    }
                    let seed = v["seed"].as_u64().ok_or("missing seed")?;
                    let swapped = v["swapped"].as_bool().ok_or("missing swapped")?;
                    let limit = v["turn_limit"].as_u64().ok_or("missing turn limit")?;
                    if limit == 0 || limit > u32::MAX as u64 {
                        return Err("invalid turn limit".into());
                    }
                    let rules = match v["rules"].as_str() {
                        None => RulesProfile::Arena,
                        Some(name) => RulesProfile::parse(name)
                            .ok_or_else(|| format!("unsupported rules profile: {name}"))?,
                    };
                    let agents = match v.get("agents") {
                        None => None,
                        Some(value) => {
                            let entries = value.as_array().ok_or("agents must be an array")?;
                            if entries.len() != 2 {
                                return Err("agents must contain two entries".into());
                            }
                            Some([
                                entries[0]
                                    .as_str()
                                    .ok_or("agent name must be a string")?
                                    .to_owned(),
                                entries[1]
                                    .as_str()
                                    .ok_or("agent name must be a string")?
                                    .to_owned(),
                            ])
                        }
                    };
                    session = Some(Session {
                        seed,
                        swapped,
                        limit: limit as u32,
                        games: [
                            Game::new_with_rules(seed, rules),
                            Game::new_with_rules(seed ^ 0x9e3779b97f4a7c15, rules),
                        ],
                        garbage: [
                            ChaCha8Rng::seed_from_u64(seed ^ 12345),
                            ChaCha8Rng::seed_from_u64(seed ^ 67890),
                        ],
                        turn: 0,
                        player: 0,
                        sent: [0; 2],
                        agents,
                    });
                }
                Some("battle_start") => {
                    if session.is_some() || battle_session.is_some() {
                        return Err("new match before previous result".into());
                    }
                    if v["replay_version"].as_u64() != Some(3) {
                        return Err("unsupported movement replay version".into());
                    }
                    let seed = v["seed"].as_u64().ok_or("missing seed")?;
                    let swapped = v["swapped"].as_bool().ok_or("missing swapped")?;
                    let piece_limit = v["piece_limit"].as_u64().ok_or("missing piece limit")?;
                    if piece_limit == 0 || piece_limit > u32::MAX as u64 {
                        return Err("invalid piece limit".into());
                    }
                    let rules = v["rules"]
                        .as_str()
                        .and_then(RulesProfile::parse)
                        .ok_or("unsupported rules profile")?;
                    let entries = v["agents"].as_array().ok_or("agents must be an array")?;
                    if entries.len() != 2 {
                        return Err("agents must contain two entries".into());
                    }
                    let agents = [
                        entries[0]
                            .as_str()
                            .ok_or("agent name must be a string")?
                            .to_owned(),
                        entries[1]
                            .as_str()
                            .ok_or("agent name must be a string")?
                            .to_owned(),
                    ];
                    battle_session = Some(BattleSession {
                        seed,
                        swapped,
                        piece_limit: piece_limit as u32,
                        games: [
                            Game::new_with_rules(seed, rules),
                            Game::new_with_rules(seed ^ 0x9e3779b97f4a7c15, rules),
                        ],
                        garbage: [
                            ChaCha8Rng::seed_from_u64(seed ^ 12345),
                            ChaCha8Rng::seed_from_u64(seed ^ 67890),
                        ],
                        batch_frame: None,
                        batch_players: [false; 2],
                        sent: [0; 2],
                        last_lock_frame: [0; 2],
                        agents,
                    });
                }
                Some("battle_move") => {
                    let s = battle_session.as_mut().ok_or("battle move outside match")?;
                    let frame = v["frame"].as_u64().ok_or("missing battle frame")?;
                    if s.batch_frame.is_some_and(|current| frame < current) {
                        return Err("battle frame moved backwards".into());
                    }
                    if s.batch_frame.is_some_and(|current| frame > current) {
                        flush_battle_batch(s);
                    }
                    if s.batch_frame.is_none() {
                        s.batch_frame = Some(frame);
                    }
                    let player = v["player"].as_u64().ok_or("missing player")? as usize;
                    if player >= 2 || s.batch_players[player] {
                        return Err("duplicate or invalid battle player".into());
                    }
                    if v["agent"].as_str() != Some(s.agents[player].as_str()) {
                        return Err("agent/player mismatch".into());
                    }
                    let placement: Placement =
                        serde_json::from_value(v["move"].clone()).map_err(|e| e.to_string())?;
                    let inputs: Vec<BattleInput> =
                        serde_json::from_value(v["inputs"].clone()).map_err(|e| e.to_string())?;
                    if v["movement_frames"].as_u64() != Some(inputs.len() as u64)
                        || frame != s.last_lock_frame[player] + inputs.len() as u64
                    {
                        return Err("movement frame mismatch".into());
                    }
                    let game = &mut s.games[player];
                    let active = game.queue.front().copied();
                    let hold_before = game.hold;
                    let next_before: Vec<_> = game.queue.iter().skip(1).take(5).copied().collect();
                    let hold_used = Some(placement.location.piece) != game.queue.front().copied();
                    validate_movement_plan(
                        game.board,
                        placement,
                        hold_used,
                        TimingProfile::guideline_level(1),
                        &inputs,
                    )
                    .map_err(|error| format!("invalid movement input path: {error:?}"))?;
                    if let Some(trace) = v.get("movement_trace") {
                        let expected = trace_movement(
                            game.board,
                            placement,
                            hold_used,
                            TimingProfile::guideline_level(1),
                            &inputs,
                        )
                        .map_err(|error| format!("invalid movement trace: {error:?}"))?;
                        if serde_json::to_value(expected).unwrap() != *trace {
                            return Err("movement trace mismatch".into());
                        }
                    }
                    let expected: Outcome =
                        serde_json::from_value(v["outcome"].clone()).map_err(|e| e.to_string())?;
                    let actual = game.play(placement)?;
                    if actual != expected {
                        return Err("outcome mismatch".into());
                    }
                    if serde_json::to_value(game.board.cols).unwrap() != v["board"] {
                        return Err("board mismatch".into());
                    }
                    if serde_json::to_value(&game.pending).unwrap() != v["pending"] {
                        return Err("pending garbage mismatch".into());
                    }
                    check_optional(&v, "active", active, "active piece")?;
                    check_optional(&v, "hold_used", hold_used, "hold usage")?;
                    check_optional(&v, "hold_before", hold_before, "hold before")?;
                    check_optional(&v, "next_before", next_before, "next before")?;
                    check_optional(&v, "hold_after", game.hold, "hold after")?;
                    check_optional(
                        &v,
                        "next_after",
                        game.queue.iter().take(5).copied().collect::<Vec<_>>(),
                        "next after",
                    )?;
                    if v.get("board_cells").is_some() && game.board_cells_json() != v["board_cells"]
                    {
                        return Err("board cell kinds mismatch".into());
                    }
                    if let Some(colors) = v.get("board_colors") {
                        check_board_colors(colors, &game.board_cells_json())?;
                    }
                    s.sent[player] = actual.sent;
                    s.batch_players[player] = true;
                    s.last_lock_frame[player] = frame;
                }
                Some("battle_result") => {
                    let mut s = battle_session.take().ok_or("battle result outside match")?;
                    flush_battle_batch(&mut s);
                    let frames = s.last_lock_frame.into_iter().max().unwrap_or(0);
                    let expected: BattleResult =
                        serde_json::from_value(v["result"].clone()).map_err(|e| e.to_string())?;
                    let recorded_dead: [bool; 2] = match v.get("dead") {
                        Some(dead) => {
                            serde_json::from_value(dead.clone()).map_err(|e| e.to_string())?
                        }
                        None => match expected.winner {
                            Some(0) => [false, true],
                            Some(1) => [true, false],
                            None => [false, false],
                            Some(_) => return Err("invalid battle winner".into()),
                        },
                    };
                    for (game, dead) in s.games.iter_mut().zip(recorded_dead) {
                        if game.dead && !dead {
                            return Err("terminal state mismatch".into());
                        }
                        game.dead = dead;
                    }
                    if frames == 0
                        || (!s.games.iter().any(|game| game.dead)
                            && s.games.iter().map(|game| game.pieces).max().unwrap_or(0)
                                < s.piece_limit)
                    {
                        return Err("premature battle result".into());
                    }
                    let actual = BattleResult {
                        seed: s.seed,
                        swapped: s.swapped,
                        winner: match (s.games[0].dead, s.games[1].dead) {
                            (true, false) => Some(1),
                            (false, true) => Some(0),
                            _ => None,
                        },
                        frames,
                        pieces: [s.games[0].pieces, s.games[1].pieces],
                        attack: [s.games[0].attack, s.games[1].attack],
                        lines: [s.games[0].lines, s.games[1].lines],
                    };
                    if actual != expected {
                        return Err("battle result mismatch".into());
                    }
                    if let Some(winner_agent) = v.get("winner_agent") {
                        let expected_agent = actual.winner.map(|player| s.agents[player].clone());
                        if winner_agent != &serde_json::to_value(expected_agent).unwrap() {
                            return Err("winner agent mismatch".into());
                        }
                    }
                    verified += 1;
                }
                Some("move") => {
                    let s = session.as_mut().ok_or("move outside match")?;
                    if s.turn >= s.limit || (s.player == 0 && s.games.iter().any(|g| g.dead)) {
                        return Err("move after match ended".into());
                    }
                    if v["turn"].as_u64() != Some(s.turn as u64)
                        || v["player"].as_u64() != Some(s.player as u64)
                    {
                        return Err("wrong turn/player order".into());
                    }
                    let mv: Option<Placement> =
                        serde_json::from_value(v["move"].clone()).map_err(|e| e.to_string())?;
                    if let Some(agent) = v["agent"].as_str() {
                        if let Some(agents) = &s.agents {
                            if agent != agents[s.player] {
                                return Err("agent/player mismatch".into());
                            }
                        }
                    }
                    let g = &mut s.games[s.player];
                    let active = g.queue.front().copied();
                    let hold_before = g.hold;
                    let next_before: Vec<_> = g.queue.iter().skip(1).take(5).copied().collect();
                    let hold_used =
                        mv.is_some_and(|placement| Some(placement.location.piece) != active);
                    let expected: Option<Outcome> =
                        serde_json::from_value(v["outcome"].clone()).map_err(|e| e.to_string())?;
                    let mut actual = if let Some(mv) = mv {
                        Some(g.play(mv)?)
                    } else {
                        g.dead = true;
                        None
                    };
                    // Pre-profile logs did not record whether a pending line
                    // was applied. Their board and pending fields still give
                    // a complete compatibility check, so compare the legacy
                    // value as zero while preserving the stricter field for
                    // new profile logs.
                    if v["outcome"].get("garbage_applied").is_none() {
                        if let Some(outcome) = actual.as_mut() {
                            outcome.garbage_applied = 0;
                        }
                    }
                    if actual != expected {
                        return Err("outcome mismatch".into());
                    }
                    s.sent[s.player] = actual.as_ref().map_or(0, |o| o.sent);
                    if serde_json::to_value(g.board.cols).unwrap() != v["board"] {
                        return Err("board mismatch".into());
                    }
                    if serde_json::to_value(&g.pending).unwrap() != v["pending"] {
                        return Err("pending garbage mismatch".into());
                    }
                    check_optional(&v, "active", active, "active piece")?;
                    check_optional(&v, "hold_used", hold_used, "hold usage")?;
                    check_optional(&v, "hold_before", hold_before, "hold before")?;
                    check_optional(&v, "next_before", next_before, "next before")?;
                    check_optional(&v, "hold_after", g.hold, "hold after")?;
                    check_optional(
                        &v,
                        "next_after",
                        g.queue.iter().take(5).copied().collect::<Vec<_>>(),
                        "next after",
                    )?;
                    if v.get("board_cells").is_some() && g.board_cells_json() != v["board_cells"] {
                        return Err("board cell kinds mismatch".into());
                    }
                    s.player += 1;
                    if s.player == 2 {
                        s.games[0].receive(s.sent[1], &mut s.garbage[0]);
                        s.games[1].receive(s.sent[0], &mut s.garbage[1]);
                        s.turn += 1;
                        s.player = 0;
                    }
                }
                Some("result") => {
                    let s = session.take().ok_or("result outside match")?;
                    if s.player != 0
                        || s.turn == 0
                        || (s.turn < s.limit && !s.games.iter().any(|g| g.dead))
                    {
                        return Err("premature result".into());
                    }
                    let actual = MatchResult {
                        seed: s.seed,
                        swapped: s.swapped,
                        winner: match (s.games[0].dead, s.games[1].dead) {
                            (true, false) => Some(1),
                            (false, true) => Some(0),
                            _ => None,
                        },
                        turns: s.turn,
                        pieces: [s.games[0].pieces, s.games[1].pieces],
                        attack: [s.games[0].attack, s.games[1].attack],
                        lines: [s.games[0].lines, s.games[1].lines],
                    };
                    let expected: MatchResult =
                        serde_json::from_value(v["result"].clone()).map_err(|e| e.to_string())?;
                    if actual != expected {
                        return Err("result mismatch".into());
                    }
                    if let Some(winner_agent) = v.get("winner_agent") {
                        let expected_agent = s
                            .agents
                            .as_ref()
                            .and_then(|agents| actual.winner.map(|player| agents[player].clone()));
                        if winner_agent != &serde_json::to_value(expected_agent).unwrap() {
                            return Err("winner agent mismatch".into());
                        }
                    }
                    verified += 1;
                }
                _ => return Err("unknown record".into()),
            }
            Ok(())
        })();
        result.map_err(|e: String| format!("line {}: {e}", line_number + 1))?;
    }
    if session.is_some() || battle_session.is_some() {
        return Err("truncated match: missing result".into());
    }
    if verified == 0 {
        return Err("no complete matches".into());
    }
    Ok(verified)
}

fn check_optional<T: Serialize>(
    record: &Value,
    key: &str,
    actual: T,
    label: &str,
) -> Result<(), String> {
    if let Some(expected) = record.get(key) {
        if serde_json::to_value(actual).map_err(|e| e.to_string())? != *expected {
            return Err(format!("{label} mismatch"));
        }
    }
    Ok(())
}
fn check_board_colors(colors: &Value, cells: &Value) -> Result<(), String> {
    let colors = colors.as_array().ok_or("board_colors must be an array")?;
    let cells = cells.as_array().ok_or("board_cells must be an array")?;
    if colors.len() != 40 || cells.len() != 40 {
        return Err("board_colors must contain 40 rows".into());
    }
    for (row, (color_row, cell_row)) in colors.iter().zip(cells).enumerate() {
        let color_row = color_row
            .as_array()
            .ok_or_else(|| format!("board_colors row {row} must be an array"))?;
        let cell_row = cell_row
            .as_array()
            .ok_or_else(|| format!("board_cells row {row} must be an array"))?;
        if color_row.len() != 10 || cell_row.len() != 10 {
            return Err(format!("board_colors row {row} must contain 10 cells"));
        }
        for (column, (color, cell)) in color_row.iter().zip(cell_row).enumerate() {
            match cell.as_str() {
                None if !color.is_null() => {
                    return Err(format!("colored empty cell at {row},{column}"))
                }
                Some("garbage") if color.as_str() != Some("garbage") => {
                    return Err(format!("garbage color mismatch at {row},{column}"))
                }
                Some("block")
                    if !matches!(
                        color.as_str(),
                        Some("I" | "O" | "T" | "L" | "J" | "S" | "Z")
                    ) =>
                {
                    return Err(format!("invalid block color at {row},{column}"))
                }
                _ => {}
            }
        }
    }
    Ok(())
}
struct Session {
    seed: u64,
    swapped: bool,
    limit: u32,
    games: [Game; 2],
    garbage: [ChaCha8Rng; 2],
    turn: u32,
    player: usize,
    sent: [u32; 2],
    agents: Option<[String; 2]>,
}

struct BattleSession {
    seed: u64,
    swapped: bool,
    piece_limit: u32,
    games: [Game; 2],
    garbage: [ChaCha8Rng; 2],
    batch_frame: Option<u64>,
    batch_players: [bool; 2],
    sent: [u32; 2],
    last_lock_frame: [u64; 2],
    agents: [String; 2],
}

fn flush_battle_batch(session: &mut BattleSession) {
    if session.batch_frame.take().is_none() {
        return;
    }
    session.games[0].receive(session.sent[1], &mut session.garbage[0]);
    session.games[1].receive(session.sent[0], &mut session.garbage[1]);
    session.batch_players = [false; 2];
    session.sent = [0; 2];
}
