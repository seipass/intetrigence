//! Replay records and `verify-replay` (`docs/s2-referee-protocol.md` §8).
//!
//! One game is one JSON Lines file. The header carries everything needed to
//! rebuild the round (`seed`, `sheet`, `seats`, `slots`, `frames_per_turn`), and
//! every turn lists the placements in **seat order**. Verification re-runs the
//! recorded placements through the same [`Engine`] used for live play and
//! compares the recomputed spin, lines, attack events, sent/cancelled rows,
//! board hashes and the final result.
//!
//! The protocol's example record has one `piece` per turn. Both players' bags
//! are seeded from the same `piece_seed`, so the sequences are identical until
//! one player uses hold; after that only that player's bag advances. The referee
//! therefore records `piece` on **each placement** as well, and verification
//! uses the per-placement value.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use intetrigence_core::{PieceKind, Rotation, SpinType};
use serde::{Deserialize, Serialize};

use crate::agent::BotDescriptor;
use crate::engine::{Engine, PlacementTarget, Profile, Sheet, board_hash};

pub const SCHEMA: &str = "intetrigence/s2-replay/1";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReplayRecord {
    Header(Header),
    Turn(TurnRecord),
    Garbage(GarbageRecord),
    Hash(HashRecord),
    Result(ResultRecord),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Header {
    pub schema: String,
    pub seed: u64,
    pub sheet: String,
    /// `{"0": "intetrigence", "1": "attax"}` (`docs/s2-referee-protocol.md` §1.2).
    pub slots: BTreeMap<String, String>,
    /// `{"1": 0, "2": 1}`: seat -> slot.
    pub seats: BTreeMap<String, u8>,
    pub rules: String,
    pub protocol: String,
    pub bots: BTreeMap<String, BotRecord>,
    pub started_at: String,
    pub turn_cap: u32,
    pub frames_per_turn: u64,
    pub preview: usize,
    /// Match profile (protocol v1.11 §1.3): `official` or `defense-stress:N`.
    pub profile: String,
    /// Start garbage holes in insertion order; empty for `official`. The
    /// verifier rebuilds the same initial boards from this.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub start_garbage: Vec<u8>,
}

/// Why a bot forfeited (protocol v1.8 §7.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ForfeitCause {
    /// The bot's visible rows are full: every legal placement uses the hidden
    /// buffer, which the bot cannot see.
    Vision,
    /// The referee had a legal placement inside the bot's visible rows, so the
    /// bot's own movegen is broken. The game is invalid and must be rerun.
    Other,
    /// The bot's `visible_rows` was not declared (or the round never started).
    Unknown,
}

impl ForfeitCause {
    pub const fn as_str(self) -> &'static str {
        match self {
            ForfeitCause::Vision => "vision",
            ForfeitCause::Other => "other",
            ForfeitCause::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BotRecord {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    pub args: Vec<String>,
    pub visible_rows: u32,
}

impl From<&BotDescriptor> for BotRecord {
    fn from(descriptor: &BotDescriptor) -> Self {
        Self {
            name: descriptor.name.clone(),
            visible_rows: descriptor.visible_rows,
            version: descriptor.version.clone(),
            commit: descriptor.commit.clone(),
            args: descriptor.args.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TurnRecord {
    pub t: u32,
    pub frame: u64,
    /// Placements in seat order. Each placement carries its own `piece`
    /// (protocol v1.1 §8.1: per-player bags make a turn-level piece undefined).
    pub placements: Vec<PlacementRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlacementRecord {
    pub slot: u8,
    pub piece: String,
    pub rotation: u8,
    pub x: i32,
    pub y: i32,
    pub hold: bool,
    /// Referee's recomputed spin (`docs/s2-referee-protocol.md` §4.2-5).
    pub spin: String,
    pub lines: u8,
    pub garbage_cleared: u32,
    pub perfect_clear: bool,
    pub events: Vec<u32>,
    pub sent: u32,
    pub cancelled: u32,
    pub attack: u32,
    pub combo_before: u32,
    pub b2b_before: u32,
    pub combo_after: u32,
    pub b2b_after: u32,
    pub multiplier: f64,
    pub think_ms: f64,
    pub candidate_index: u32,
    /// Spin label the bot claimed (audit only, never used for scoring).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claimed_spin: Option<String>,
    /// Attack implied by the claimed spin (audit only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declared_attack: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GarbageRecord {
    pub t: u32,
    pub from: u8,
    pub to: u8,
    pub amount: u32,
    pub arrival_frame: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HashRecord {
    pub t: u32,
    pub boards: [u64; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResultRecord {
    /// Slot of the winner; `null` is a draw.
    pub winner: Option<u8>,
    /// `TopOut` | `TurnCap` | `Forfeit`.
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Set only for `Forfeit` (protocol v1.8 §7.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forfeit_cause: Option<ForfeitCause>,
    pub turns: u32,
    pub final_frame: u64,
    pub per_player: Vec<PlayerRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerRecord {
    pub slot: u8,
    pub seat: u8,
    pub sent_total: u32,
    pub received_total: u32,
    pub attack_events: u32,
    pub attack_total: u32,
    pub max_combo: u32,
    pub max_b2b: u32,
    pub surge_released: u32,
    pub pieces_placed: u32,
    pub lines_cleared: u32,
    pub holds: u32,
    pub topped_out: bool,
}

pub fn piece_name(kind: PieceKind) -> &'static str {
    match kind {
        PieceKind::I => "I",
        PieceKind::O => "O",
        PieceKind::T => "T",
        PieceKind::S => "S",
        PieceKind::Z => "Z",
        PieceKind::J => "J",
        PieceKind::L => "L",
    }
}

pub fn parse_piece(name: &str) -> Option<PieceKind> {
    match name {
        "I" => Some(PieceKind::I),
        "O" => Some(PieceKind::O),
        "T" => Some(PieceKind::T),
        "S" => Some(PieceKind::S),
        "Z" => Some(PieceKind::Z),
        "J" => Some(PieceKind::J),
        "L" => Some(PieceKind::L),
        _ => None,
    }
}

pub fn spin_name(spin: SpinType) -> &'static str {
    match spin {
        SpinType::None => "none",
        SpinType::Mini => "mini",
        SpinType::Full => "full",
    }
}

pub fn parse_spin(name: &str) -> Option<SpinType> {
    match name {
        "none" => Some(SpinType::None),
        "mini" => Some(SpinType::Mini),
        "full" => Some(SpinType::Full),
        _ => None,
    }
}

/// Writes one JSON object per line.
pub fn write_replay(path: &Path, records: &[ReplayRecord]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::File::create(path)?;
    for record in records {
        serde_json::to_writer(&mut file, record)?;
        file.write_all(b"\n")?;
    }
    file.flush()
}

/// Rebuilds the initial engine for a replay header (profile start garbage
/// included). Periodic harness injections are in the record stream; apply the
/// `garbage` records with `from == 2` at the same turns.
pub fn engine_from_header(header: &Header) -> Engine {
    let sheet = Sheet::parse(&header.sheet).unwrap_or(Sheet::A);
    Engine::with_start_garbage(
        header.seed,
        sheet,
        header.frames_per_turn,
        &header.start_garbage,
    )
}

pub fn read_replay(path: &Path) -> Result<Vec<ReplayRecord>, VerifyError> {
    let file = std::fs::File::open(path).map_err(|error| VerifyError::Io(error.to_string()))?;
    let mut records = Vec::new();
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|error| VerifyError::Io(error.to_string()))?;
        if line.trim().is_empty() {
            continue;
        }
        let record: ReplayRecord = serde_json::from_str(&line)
            .map_err(|error| VerifyError::Parse(format!("line {}: {error}", index + 1)))?;
        records.push(record);
    }
    Ok(records)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct VerifyReport {
    pub ok: bool,
    pub seed: u64,
    pub sheet: String,
    pub turns: u32,
    pub placements: u32,
    pub mismatches: Vec<String>,
    /// Forfeit cause recomputed by the current classifier (reclassification
    /// support; `None` for non-forfeit games).
    #[serde(default)]
    pub forfeit_cause: Option<ForfeitCause>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerifyError {
    Io(String),
    Parse(String),
    Structure(String),
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerifyError::Io(message) => write!(f, "i/o: {message}"),
            VerifyError::Parse(message) => write!(f, "parse: {message}"),
            VerifyError::Structure(message) => write!(f, "structure: {message}"),
        }
    }
}

impl std::error::Error for VerifyError {}

/// Re-runs a replay file and compares every recomputed value with the record.
pub fn verify_replay(path: &Path) -> Result<VerifyReport, VerifyError> {
    let records = read_replay(path)?;
    let header = match records.first() {
        Some(ReplayRecord::Header(header)) => header.clone(),
        _ => {
            return Err(VerifyError::Structure(
                "first record must be the header".into(),
            ));
        }
    };
    Sheet::parse(&header.sheet)
        .ok_or_else(|| VerifyError::Structure(format!("bad sheet {:?}", header.sheet)))?;
    let visible_rows = [
        header
            .bots
            .get("0")
            .map(|bot| bot.visible_rows)
            .unwrap_or(0),
        header
            .bots
            .get("1")
            .map(|bot| bot.visible_rows)
            .unwrap_or(0),
    ];
    let profile = Profile::parse(&header.profile).unwrap_or(Profile::Official);
    let mut engine = engine_from_header(&header);
    let mut report = VerifyReport {
        seed: header.seed,
        sheet: header.sheet.clone(),
        ..VerifyReport::default()
    };

    let mut index = 1;
    while index < records.len() {
        match &records[index] {
            ReplayRecord::Garbage(garbage) if garbage.from == 2 => {
                // Harness injection (garbage-pressure profile). It is recorded
                // before the turn it applies to, so the verifier applies it at
                // the same point.
                if profile.injection_amount(garbage.t) != Some(garbage.amount) {
                    report.mismatches.push(format!(
                        "t{} harness garbage: profile {:?} does not inject {} rows",
                        garbage.t, header.profile, garbage.amount
                    ));
                }
                if garbage.to > 1 {
                    report.mismatches.push(format!(
                        "t{} harness garbage: bad target slot {}",
                        garbage.t, garbage.to
                    ));
                }
                if garbage.arrival_frame != u64::from(garbage.t) * header.frames_per_turn {
                    report.mismatches.push(format!(
                        "t{} harness garbage: arrival_frame {} != {}",
                        garbage.t,
                        garbage.arrival_frame,
                        u64::from(garbage.t) * header.frames_per_turn
                    ));
                }
                if garbage.to <= 1 {
                    engine.players[garbage.to as usize]
                        .garbage
                        .receive(garbage.amount, garbage.arrival_frame);
                }
                index += 1;
            }
            ReplayRecord::Turn(turn) => {
                engine.turn = turn.t;
                engine.begin_turn();
                report.turns += 1;
                let mut packets: Vec<(u8, u32, u64)> = Vec::new();
                let mut seat_index = 0;
                for placement in &turn.placements {
                    let slot = placement.slot as usize;
                    if slot >= 2 {
                        report
                            .mismatches
                            .push(format!("t{}: slot {slot} out of range", turn.t));
                        continue;
                    }
                    if engine.order.get(seat_index).copied() != Some(slot) {
                        report.mismatches.push(format!(
                            "t{}: placement {} is slot {slot}, expected seat order {:?}",
                            turn.t, seat_index, engine.order
                        ));
                    }
                    seat_index += 1;

                    let kind = parse_piece(&placement.piece).ok_or_else(|| {
                        VerifyError::Structure(format!("bad piece {:?}", placement.piece))
                    })?;
                    let rotation = Rotation::from_u8(placement.rotation).ok_or_else(|| {
                        VerifyError::Structure(format!("bad rotation {}", placement.rotation))
                    })?;
                    let target =
                        PlacementTarget::from_core(kind, rotation, placement.x, placement.y);
                    let claimed = placement.claimed_spin.as_deref().and_then(parse_spin);
                    match engine.apply(slot, &target, claimed) {
                        Ok(applied) => {
                            report.placements += 1;
                            let mut check = |name: &str, expected: String, actual: String| {
                                if expected != actual {
                                    report.mismatches.push(format!(
                                        "t{} slot{} {name}: recorded {expected}, recomputed {actual}",
                                        turn.t, slot
                                    ));
                                }
                            };
                            check(
                                "piece",
                                placement.piece.clone(),
                                piece_name(applied.piece).to_string(),
                            );
                            check(
                                "rotation",
                                placement.rotation.to_string(),
                                (applied.rotation as u8).to_string(),
                            );
                            check("x", placement.x.to_string(), applied.x.to_string());
                            check("y", placement.y.to_string(), applied.y.to_string());
                            check("hold", placement.hold.to_string(), applied.hold.to_string());
                            check(
                                "spin",
                                placement.spin.clone(),
                                spin_name(applied.spin).to_string(),
                            );
                            check(
                                "lines",
                                placement.lines.to_string(),
                                applied.lines.to_string(),
                            );
                            check(
                                "garbage_cleared",
                                placement.garbage_cleared.to_string(),
                                applied.garbage_cleared.to_string(),
                            );
                            check(
                                "perfect_clear",
                                placement.perfect_clear.to_string(),
                                applied.perfect_clear.to_string(),
                            );
                            check(
                                "events",
                                format!("{:?}", placement.events),
                                format!("{:?}", applied.events),
                            );
                            check("sent", placement.sent.to_string(), applied.sent.to_string());
                            check(
                                "cancelled",
                                placement.cancelled.to_string(),
                                applied.cancelled.to_string(),
                            );
                            check(
                                "attack",
                                placement.attack.to_string(),
                                applied.attack_total.to_string(),
                            );
                            check(
                                "combo_after",
                                placement.combo_after.to_string(),
                                applied.combo_after.to_string(),
                            );
                            check(
                                "b2b_after",
                                placement.b2b_after.to_string(),
                                applied.b2b_after.to_string(),
                            );
                            for (amount, arrival) in &applied.packets {
                                packets.push((slot as u8, *amount, *arrival));
                            }
                        }
                        Err(error) => report.mismatches.push(format!(
                            "t{} slot{}: replay apply failed: {error}",
                            turn.t, slot
                        )),
                    }
                }
                index += 1;

                let mut packet_index = 0;
                while let Some(ReplayRecord::Garbage(garbage)) = records.get(index) {
                    if let Some(&(from, amount, arrival)) = packets.get(packet_index) {
                        if garbage.from != from
                            || garbage.amount != amount
                            || garbage.arrival_frame != arrival
                            || garbage.to != 1 - from
                            || garbage.t != turn.t
                        {
                            report.mismatches.push(format!(
                                "t{} garbage #{packet_index}: recorded {:?}, recomputed {:?}",
                                turn.t,
                                (
                                    garbage.from,
                                    garbage.to,
                                    garbage.amount,
                                    garbage.arrival_frame
                                ),
                                (from, 1 - from, amount, arrival)
                            ));
                        }
                    } else {
                        report.mismatches.push(format!(
                            "t{} garbage #{packet_index}: recorded but no packet was sent",
                            turn.t
                        ));
                    }
                    packet_index += 1;
                    index += 1;
                }
                if packet_index != packets.len() {
                    report.mismatches.push(format!(
                        "t{}: {} garbage packets missing from the replay",
                        turn.t,
                        packets.len() - packet_index
                    ));
                }

                match records.get(index) {
                    Some(ReplayRecord::Hash(hash)) => {
                        let actual = [
                            board_hash(engine.players[0].game.board()),
                            board_hash(engine.players[1].game.board()),
                        ];
                        if hash.boards != actual {
                            report.mismatches.push(format!(
                                "t{} hash: recorded {:?}, recomputed {:?}",
                                turn.t, hash.boards, actual
                            ));
                        }
                        index += 1;
                    }
                    _ => report
                        .mismatches
                        .push(format!("t{}: missing hash record", turn.t)),
                }
            }
            ReplayRecord::Result(result) => {
                verify_result(&mut engine, result, visible_rows, &mut report);
                index += 1;
            }
            other => {
                report
                    .mismatches
                    .push(format!("unexpected record: {other:?}"));
                index += 1;
            }
        }
    }

    report.ok = report.mismatches.is_empty();
    Ok(report)
}

fn verify_result(
    engine: &mut Engine,
    result: &ResultRecord,
    visible_rows: [u32; 2],
    report: &mut VerifyReport,
) {
    let over: Vec<usize> = (0..2)
        .filter(|&slot| engine.players[slot].game.is_over())
        .collect();
    let (winner, reason) = if result.reason == "Forfeit" {
        // A forfeit ends the round early; the engine state is not the result.
        (result.winner, result.reason.clone())
    } else if let Some(&loser) = over.first() {
        (Some(1 - loser as u8), "TopOut".to_string())
    } else if result.reason == "TopOut" {
        // Stuck top-out (protocol v1.6 §7): the recorded loser has no legal
        // placement even though the game is not block-out.
        match result.winner.map(|winner| 1 - winner as usize) {
            Some(loser) if !engine.has_legal_move(loser) => (result.winner, "TopOut".to_string()),
            _ => (None, "TurnCap".to_string()),
        }
    } else {
        let a = &engine.players[0];
        let b = &engine.players[1];
        let winner = if a.received_total() != b.received_total() {
            Some(if a.received_total() < b.received_total() {
                0
            } else {
                1
            })
        } else if a.sent_total() != b.sent_total() {
            Some(if a.sent_total() > b.sent_total() {
                0
            } else {
                1
            })
        } else {
            None
        };
        (winner, "TurnCap".to_string())
    };
    if winner != result.winner {
        report.mismatches.push(format!(
            "result winner: recorded {:?}, recomputed {winner:?}",
            result.winner
        ));
    }
    if reason != result.reason {
        report.mismatches.push(format!(
            "result reason: recorded {}, recomputed {reason}",
            result.reason
        ));
    }
    if result.turns != report.turns {
        report.mismatches.push(format!(
            "result turns: recorded {}, recomputed {}",
            result.turns, report.turns
        ));
    }
    if result.reason == "Forfeit" {
        let candidate_failure = result
            .detail
            .as_deref()
            .unwrap_or("")
            .contains("candidates is reachable");
        if candidate_failure {
            if let Some(loser) = result.winner.map(|winner| 1 - winner as usize) {
                let actual = crate::referee::classify_forfeit(engine, loser, visible_rows[loser]);
                report.forfeit_cause = Some(actual);
                if let Some(recorded) = result.forfeit_cause {
                    if recorded != ForfeitCause::Unknown && actual != recorded {
                        report.mismatches.push(format!(
                            "result forfeit_cause: recorded {}, recomputed {}",
                            recorded.as_str(),
                            actual.as_str()
                        ));
                    }
                }
            }
        }
    }
    if result.reason != "Forfeit" && result.final_frame != engine.frame {
        report.mismatches.push(format!(
            "result final_frame: recorded {}, recomputed {}",
            result.final_frame, engine.frame
        ));
    }
    for player in &result.per_player {
        let slot = player.slot as usize;
        if slot >= 2 {
            continue;
        }
        let state = &engine.players[slot];
        let stats = state.game.stats();
        let actual = [
            ("sent_total", state.sent_total(), player.sent_total),
            (
                "received_total",
                state.received_total(),
                player.received_total,
            ),
            ("attack_events", state.attack_events, player.attack_events),
            ("attack_total", state.attack_total, player.attack_total),
            ("max_combo", state.max_combo, player.max_combo),
            ("max_b2b", state.max_b2b, player.max_b2b),
            (
                "surge_released",
                state.surge_released,
                player.surge_released,
            ),
            ("pieces_placed", stats.pieces_placed, player.pieces_placed),
            ("lines_cleared", stats.lines_cleared, player.lines_cleared),
            ("holds", stats.holds, player.holds),
        ];
        for (name, recomputed, recorded) in actual {
            if recomputed != recorded {
                report.mismatches.push(format!(
                    "result slot{slot} {name}: recorded {recorded}, recomputed {recomputed}"
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn piece_and_spin_names_roundtrip() {
        for kind in intetrigence_core::ALL_PIECES {
            assert_eq!(parse_piece(piece_name(kind)), Some(kind));
        }
        for spin in [SpinType::None, SpinType::Mini, SpinType::Full] {
            assert_eq!(parse_spin(spin_name(spin)), Some(spin));
        }
        assert_eq!(parse_piece("X"), None);
        assert_eq!(parse_spin("quad"), None);
    }

    #[test]
    fn record_tags_are_snake_case() {
        let record = ReplayRecord::Hash(HashRecord {
            t: 0,
            boards: [1, 2],
        });
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.starts_with(r#"{"type":"hash""#), "{json}");
    }
}
