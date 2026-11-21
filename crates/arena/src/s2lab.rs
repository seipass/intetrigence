//! Reader for the `intetrigence-s2lab/2` JSONL written by
//! `adapters/s2lab/run-match.mjs`.
//!
//! The arena's own replay format is produced by the TBP referee. The Attax
//! series runs inside 61bi's s2-bot-lab instead, which records the public state
//! before every lock and the placement both sides chose. This module only
//! parses that record; `s2lab_render` turns it into the same `TurnSnapshot`
//! drawing model the arena renderer uses.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

pub const SCHEMA: &str = "intetrigence-s2lab/2";

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameRecord {
    pub schema: String,
    pub seed: u64,
    pub our_side: String,
    pub contestants: BTreeMap<String, String>,
    pub status: String,
    pub winner: Option<String>,
    pub winner_name: Option<String>,
    pub reason: String,
    pub locks: BTreeMap<String, u32>,
    pub attack: BTreeMap<String, u32>,
    pub stats: BTreeMap<String, Stats>,
    pub trace: Vec<TraceEntry>,
    pub wall_ms: f64,
    pub logical_frames: u64,
    pub error: Option<String>,
    pub settings: Settings,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub mode: String,
    pub pps: u32,
    pub garbage_delay_frames: u32,
    pub queue_model: String,
    pub public_queue_including_current: usize,
    pub max_locks: u32,
    pub first_to: Option<u32>,
    pub start_seed: u64,
    pub seat_offset: u32,
    pub intetrigence_search: SearchSettings,
    pub attax_engine: AttaxEngine,
    pub attax_width: u32,
    pub attax_stock_potential: bool,
    pub attax_solvency: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchSettings {
    pub depth: u32,
    pub width: u32,
    pub threads: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttaxEngine {
    pub id: String,
    #[serde(default)]
    pub build: String,
    #[serde(default)]
    pub binary: String,
    #[serde(default)]
    pub version: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub turns: u32,
    pub attack: u32,
    pub garbage_cancelled: u32,
    pub garbage_cleared: u32,
    pub garbage_sent: u32,
    pub garbage_received: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceEntry {
    pub side: String,
    pub decision: Decision,
    pub reply: Reply,

    pub think_ms: f64,
    pub lock: Lock,
    pub attack: Attack,
    pub tanked_rows: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    pub board: BoardState,
    pub pieces: Pieces,
    pub chain: Chain,
    pub lock_time: LockTime,
    pub incoming: Incoming,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardState {
    pub width: usize,
    pub height: usize,
    pub visible_height: usize,
    pub cells: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pieces {
    pub current: String,
    pub hold: Option<String>,
    pub known: Vec<String>,
    pub hold_available: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chain {
    pub combo: u32,
    pub b2b: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockTime {
    pub logical_frame: u64,
    pub pieces_placed: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Incoming {
    pub pending_rows: u32,
    pub due_this_lock_rows: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reply {
    pub cells: Vec<[i32; 2]>,
    pub kind: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lock {
    pub cleared_rows: Vec<usize>,
    pub lines: u8,
    pub spin: String,
    pub perfect_clear: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attack {
    pub raw: f64,
    pub after_multiplier: f64,
    pub after_rounding: u32,
    pub special_bonus: u32,
    pub perfect_clear_bonus: u32,
    pub outgoing_before_cancel: u32,
}

/// Reads every non-empty line of a Lab JSONL file.
pub fn read_games(path: &Path) -> Result<Vec<GameRecord>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    let mut games = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let game: GameRecord = serde_json::from_str(line)
            .map_err(|error| format!("{}:{}: {error}", path.display(), index + 1))?;
        if game.schema != SCHEMA {
            return Err(format!(
                "{}:{}: schema {:?}, expected {SCHEMA}",
                path.display(),
                index + 1,
                game.schema
            ));
        }
        games.push(game);
    }
    if games.is_empty() {
        return Err(format!("{}: no games", path.display()));
    }
    Ok(games)
}

/// Parses the 40x10 Lab board string. Row 0 is the bottom row.
pub fn parse_board(cells: &str, width: usize, height: usize) -> Result<Vec<Vec<char>>, String> {
    if width != 10 || height != 40 {
        return Err(format!("unsupported Lab board {width}x{height}"));
    }
    let chars: Vec<char> = cells.chars().collect();
    if chars.len() != width * height {
        return Err(format!(
            "Lab board has {} cells, expected {}",
            chars.len(),
            width * height
        ));
    }
    let mut rows = Vec::with_capacity(height);
    for y in 0..height {
        rows.push(chars[y * width..(y + 1) * width].to_vec());
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_rows_are_bottom_up() {
        let mut cells = "_".repeat(400);
        cells.replace_range(0..1, "T");
        cells.replace_range(399..400, "G");
        let rows = parse_board(&cells, 10, 40).unwrap();
        assert_eq!(rows[0][0], 'T');
        assert_eq!(rows[39][9], 'G');
    }

    #[test]
    fn board_rejects_wrong_size() {
        assert!(parse_board("___", 10, 40).is_err());
        assert!(parse_board(&"_".repeat(400), 9, 40).is_err());
    }
}
