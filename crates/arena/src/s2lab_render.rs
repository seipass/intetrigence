//! Renders `intetrigence-s2lab/2` games to MP4.
//!
//! The arena renderer re-simulates its own replay format. The Attax series is
//! recorded by s2-bot-lab instead: every trace entry carries the public board
//! before the lock, the placement both sides chose, and the lock result. This
//! module rebuilds the same `TurnSnapshot` model from those recorded states and
//! reuses the arena drawing code. It does not re-run the referee; the Lab
//! runner's `--verify` is the authority for the game, and this renderer is a
//! view of the verified record.
//!
//! Frames are written at 1280x720 and scaled by ffmpeg to the requested output
//! size (the default is 854x480). Each game is rendered to a segment first so a
//! long series can resume; the segments are concatenated with `-c copy`.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use intetrigence_core::{Board, Cell};

use crate::render::{
    ACCENT, BACKGROUND, Canvas, ColorLayer, DIM, DrawContext, PlacementView, PlayerState,
    RenderOptions, TEXT, TurnSnapshot, full_rows,
};
use crate::replay::{self, Header, PlayerRecord, ResultRecord};
use crate::s2lab::{self, GameRecord, TraceEntry};

/// Options for the s2lab series renderer.
#[derive(Clone, Debug)]
pub struct S2LabRenderOptions {
    pub fps: u32,
    /// Output width after ffmpeg's scale filter.
    pub width: u32,
    /// Output height after ffmpeg's scale filter.
    pub height: u32,
    /// Seconds of title card before each game.
    pub title_seconds: f32,
    /// Seconds of end card after each game.
    pub end_card_seconds: f32,
    /// Stop after this many games.
    pub max_games: Option<usize>,
    /// Zero-based index of the first game.
    pub start_game: usize,
    /// Reuse an existing segment instead of rendering it again.
    pub resume: bool,
    /// Keep the per-game segments after concatenation.
    pub keep_segments: bool,
    /// Directory for segments. Defaults to `<out parent>/segments`.
    pub segment_dir: Option<PathBuf>,
}

impl Default for S2LabRenderOptions {
    fn default() -> Self {
        Self {
            fps: 30,
            width: 854,
            height: 480,
            title_seconds: 1.0,
            end_card_seconds: 3.0,
            max_games: None,
            start_game: 0,
            resume: false,
            keep_segments: false,
            segment_dir: None,
        }
    }
}

/// What was rendered.
#[derive(Clone, Debug)]
pub struct S2LabRenderReport {
    pub games: usize,
    pub frames: u64,
    pub duration_seconds: f64,
    pub output: PathBuf,
    pub segments: Vec<PathBuf>,
}

struct S2LabGame {
    header: Header,
    result: ResultRecord,
    snapshots: Vec<TurnSnapshot>,
    left: String,
    right: String,
}

/// Renders every selected game in `games_path` and concatenates the segments.
pub fn render_s2lab_games(
    games_path: &Path,
    out_path: &Path,
    options: &S2LabRenderOptions,
) -> Result<S2LabRenderReport, String> {
    if options.fps == 0 || options.width == 0 || options.height == 0 {
        return Err("fps, width and height must be positive".into());
    }
    let records = s2lab::read_games(games_path)?;
    let total = records.len();
    let start = options.start_game.min(total);
    let end = options
        .max_games
        .map(|count| (start + count).min(total))
        .unwrap_or(total);
    if start >= end {
        return Err(format!("no games selected (start {start}, total {total})"));
    }
    let segment_dir = options.segment_dir.clone().unwrap_or_else(|| {
        out_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("segments")
    });
    std::fs::create_dir_all(&segment_dir)
        .map_err(|error| format!("create {}: {error}", segment_dir.display()))?;

    let mut segments = Vec::new();
    let mut rendered_frames = 0u64;
    for (offset, record) in records[start..end].iter().enumerate() {
        let game_index = start + offset;
        let segment = segment_dir.join(format!("seed-{:09}.mp4", record.seed));
        if options.resume
            && segment
                .metadata()
                .map(|metadata| metadata.len() > 0)
                .unwrap_or(false)
        {
            eprintln!(
                "render-s2lab: skip game {}/{} seed {} (existing segment)",
                game_index + 1,
                total,
                record.seed
            );
        } else {
            eprintln!(
                "render-s2lab: game {}/{} seed {} ({} locks)",
                game_index + 1,
                total,
                record.seed,
                record.trace.len()
            );
            let game = build_game(record, options.fps)?;
            let frames = render_game(&game, &segment, options, game_index, total)?;
            rendered_frames += frames;
            eprintln!(
                "render-s2lab: game {}/{} seed {} -> {} frames ({:.1}s)",
                game_index + 1,
                total,
                record.seed,
                frames,
                frames as f64 / options.fps as f64
            );
        }
        segments.push(segment);
    }

    let list_path = segment_dir.join("concat.txt");
    let mut list = String::new();
    for segment in &segments {
        let path = segment
            .canonicalize()
            .unwrap_or_else(|_| segment.to_path_buf());
        let escaped = path
            .display()
            .to_string()
            .replace('\\', "/")
            .replace('\'', "'\\''");
        list.push_str(&format!("file '{escaped}'\n"));
    }
    std::fs::write(&list_path, list)
        .map_err(|error| format!("write {}: {error}", list_path.display()))?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    concat_segments(&list_path, out_path)?;

    let duration_seconds =
        probe_duration(out_path).unwrap_or_else(|| rendered_frames as f64 / options.fps as f64);
    let games = segments.len();
    if !options.keep_segments {
        for segment in &segments {
            let _ = std::fs::remove_file(segment);
        }
        let _ = std::fs::remove_file(&list_path);
        let _ = std::fs::remove_dir(&segment_dir);
    }
    Ok(S2LabRenderReport {
        games,
        frames: rendered_frames,
        duration_seconds,
        output: out_path.to_path_buf(),
        segments: if options.keep_segments {
            segments
        } else {
            Vec::new()
        },
    })
}

fn concat_segments(list_path: &Path, out_path: &Path) -> Result<(), String> {
    let copy = Command::new("ffmpeg")
        .args([
            "-y",
            "-loglevel",
            "error",
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
        ])
        .arg(list_path)
        .args(["-c", "copy", "-movflags", "+faststart"])
        .arg(out_path)
        .status()
        .map_err(|error| format!("spawn ffmpeg concat: {error}"))?;
    if copy.success() {
        return Ok(());
    }
    // Segments from the same encoder should copy cleanly. If one does not,
    // re-encode the concatenation rather than leave a broken final file.
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-loglevel",
            "error",
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
        ])
        .arg(list_path)
        .args([
            "-an",
            "-c:v",
            "libx264",
            "-preset",
            "medium",
            "-crf",
            "20",
            "-pix_fmt",
            "yuv420p",
            "-movflags",
            "+faststart",
        ])
        .arg(out_path)
        .status()
        .map_err(|error| format!("spawn ffmpeg concat re-encode: {error}"))?;
    if !status.success() {
        return Err("ffmpeg concat failed".into());
    }
    Ok(())
}

fn probe_duration(path: &Path) -> Option<f64> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

fn render_game(
    game: &S2LabGame,
    out_path: &Path,
    options: &S2LabRenderOptions,
    game_index: usize,
    total: usize,
) -> Result<u64, String> {
    let temporary = out_path.with_extension("tmp.mp4");
    let scale = format!("scale={}:{}", options.width, options.height);
    let mut child = Command::new("ffmpeg")
        .args([
            "-y",
            "-loglevel",
            "error",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-s",
            "1280x720",
            "-r",
            &options.fps.to_string(),
            "-i",
            "-",
            "-an",
            "-c:v",
            "libx264",
            "-preset",
            "medium",
            "-crf",
            "20",
            "-pix_fmt",
            "yuv420p",
            "-vf",
            &scale,
            "-movflags",
            "+faststart",
        ])
        .arg(&temporary)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|error| format!("spawn ffmpeg: {error}"))?;
    let mut stdin = std::io::BufWriter::new(
        child
            .stdin
            .take()
            .ok_or_else(|| "ffmpeg stdin unavailable".to_string())?,
    );
    let draw_options = RenderOptions {
        fps: options.fps,
        width: 1280,
        height: 720,
        end_card_seconds: options.end_card_seconds,
        max_turns: None,
        progress_every: 0,
    };
    let draw = DrawContext {
        header: &game.header,
        options: &draw_options,
        result: Some(&game.result),
    };
    let mut canvas = Canvas::new(1280, 720);
    let mut frames = 0u64;
    let title_frames = (options.title_seconds * options.fps as f32).round() as u64;
    for _ in 0..title_frames {
        draw_title(&mut canvas, game, game_index, total);
        stdin
            .write_all(canvas.bytes())
            .map_err(|error| format!("ffmpeg pipe: {error}"))?;
        frames += 1;
    }
    let frames_per_turn = game.header.frames_per_turn.max(1);
    for (index, snapshot) in game.snapshots.iter().enumerate() {
        let next = game.snapshots.get(index + 1);
        for frame in 0..frames_per_turn {
            draw.draw_turn(&mut canvas, snapshot, next, frame, frames_per_turn);
            stdin
                .write_all(canvas.bytes())
                .map_err(|error| format!("ffmpeg pipe: {error}"))?;
            frames += 1;
        }
    }
    let end_frames = (options.end_card_seconds * options.fps as f32).round() as u64;
    for frame in 0..end_frames {
        draw.draw_end_card(&mut canvas, frame, end_frames);
        stdin
            .write_all(canvas.bytes())
            .map_err(|error| format!("ffmpeg pipe: {error}"))?;
        frames += 1;
    }
    drop(stdin);
    let status = child
        .wait()
        .map_err(|error| format!("wait ffmpeg: {error}"))?;
    if !status.success() {
        return Err(format!(
            "ffmpeg exited with {status} for {}",
            out_path.display()
        ));
    }
    std::fs::rename(&temporary, out_path)
        .map_err(|error| format!("rename {}: {error}", out_path.display()))?;
    Ok(frames)
}

fn draw_title(canvas: &mut Canvas, game: &S2LabGame, game_index: usize, total: usize) {
    canvas.clear(BACKGROUND);
    canvas.text(
        80,
        230,
        5,
        &format!("GAME {}/{}", game_index + 1, total),
        TEXT,
    );
    canvas.text(80, 330, 2, &format!("SEED {}", game.header.seed), ACCENT);
    canvas.text(
        80,
        370,
        2,
        &format!(
            "{} VS {}",
            game.left.to_uppercase(),
            game.right.to_uppercase()
        ),
        DIM,
    );
    canvas.text(80, 440, 1, "INTETRIGENCE S2 BOT LAB", DIM);
    canvas.text(80, 460, 1, &game.header.profile.to_uppercase(), DIM);
}

fn build_game(record: &GameRecord, fps: u32) -> Result<S2LabGame, String> {
    let left = record
        .contestants
        .get("left")
        .cloned()
        .unwrap_or_else(|| "left".into());
    let right = record
        .contestants
        .get("right")
        .cloned()
        .unwrap_or_else(|| "right".into());
    let pps = record.settings.pps.max(1);
    let frames_per_turn = (fps / pps).max(1) as u64;
    let mut states = [empty_state(), empty_state()];
    let mut snapshots = Vec::with_capacity(record.trace.len());
    let mut index = 0usize;
    let mut turn = 0u32;
    while index < record.trace.len() {
        let frame = record.trace[index].decision.lock_time.logical_frame;
        let group_start = index;
        while index < record.trace.len()
            && record.trace[index].decision.lock_time.logical_frame == frame
        {
            index += 1;
        }
        let group = &record.trace[group_start..index];
        for entry in group {
            let side = side_index(&entry.side)?;
            update_from_decision(&mut states[side], entry)?;
        }
        let before = states.clone();
        let mut after = before.clone();
        let mut placements = [None, None];
        for entry in group {
            let side = side_index(&entry.side)?;
            let cells = cells4(&entry.reply.cells)?;
            let kind = replay::parse_piece(&entry.reply.kind).ok_or_else(|| {
                format!(
                    "t{}: bad piece {:?}",
                    entry.decision.lock_time.logical_frame, entry.reply.kind
                )
            })?;
            let cleared = full_rows(&after[side].board, &cells);
            for &(x, y) in &cells {
                if (0..10).contains(&x) && (0..40).contains(&y) {
                    after[side]
                        .board
                        .set_cell(x as usize, y as usize, Cell::Block);
                }
            }
            after[side].colors.place(kind, &cells);
            after[side].board.clear_lines();
            after[side].colors.clear_rows(&cleared);
            after[side].lines += entry.lock.lines as u32;
            after[side].pieces += 1;
            after[side].attack_total += entry.attack.outgoing_before_cancel;
            after[side].sent_total += entry.attack.outgoing_before_cancel;
            after[side].received_total += entry.tanked_rows;
            after[side].combo = entry.decision.chain.combo;
            after[side].b2b = entry.decision.chain.b2b;
            after[side].pending = entry.decision.incoming.pending_rows;
            after[side].think_ms = entry.think_ms;
            states[side] = after[side].clone();
            placements[side] = Some(PlacementView {
                piece: kind,
                cells,
                spin: entry.lock.spin.clone(),
                lines: entry.lock.lines,
                attack: entry.attack.outgoing_before_cancel,
                perfect_clear: entry.lock.perfect_clear,
                topped_out: cells.iter().all(|&(_, y)| y >= 20),
                cleared_rows: entry.lock.cleared_rows.clone(),
            });
        }
        snapshots.push(TurnSnapshot {
            turn,
            before,
            after,
            placements,
        });
        turn += 1;
    }
    if snapshots.is_empty() {
        return Err(format!("seed {}: empty trace", record.seed));
    }

    let winner: Option<u8> = match record.winner.as_deref() {
        Some("left") => Some(0),
        Some("right") => Some(1),
        _ => None,
    };
    let reason = match record.reason.as_str() {
        "top-out" => "TOP OUT".to_string(),
        "max-locks" => "TurnCap".to_string(),
        "harness-error" => "Error".to_string(),
        other => other.to_string(),
    };
    let mut per_player = Vec::with_capacity(2);
    for (slot, side) in ["left", "right"].iter().enumerate() {
        let stats = record.stats.get(*side);
        let lines = record
            .trace
            .iter()
            .filter(|entry| entry.side == *side)
            .map(|entry| entry.lock.lines as u32)
            .sum();
        let attack_events = record
            .trace
            .iter()
            .filter(|entry| entry.side == *side && entry.attack.outgoing_before_cancel > 0)
            .count() as u32;
        per_player.push(PlayerRecord {
            slot: slot as u8,
            seat: slot as u8,
            sent_total: stats.map(|value| value.garbage_sent).unwrap_or(0),
            received_total: stats.map(|value| value.garbage_received).unwrap_or(0),
            attack_events,
            attack_total: stats.map(|value| value.attack).unwrap_or(0),
            max_combo: 0,
            max_b2b: 0,
            surge_released: 0,
            pieces_placed: stats.map(|value| value.turns).unwrap_or(0),
            lines_cleared: lines,
            holds: 0,
            topped_out: winner == Some(1 - slot as u8) && reason == "TOP OUT",
        });
    }
    let result = ResultRecord {
        winner,
        reason,
        detail: record.error.clone(),
        forfeit_cause: None,
        forfeit_slot: None,
        turns: snapshots.len() as u32,
        final_frame: snapshots.len() as u64 * frames_per_turn,
        per_player,
    };

    let mut slots = BTreeMap::new();
    slots.insert("0".to_string(), left.clone());
    slots.insert("1".to_string(), right.clone());
    let mut seats = BTreeMap::new();
    seats.insert("1".to_string(), 0);
    seats.insert("2".to_string(), 1);
    let header = Header {
        schema: "intetrigence/s2lab-render/1".into(),
        seed: record.seed,
        sheet: "S2LAB".into(),
        slots,
        seats,
        rules: "S2 Lab".into(),
        t_immobility_mini: true,
        protocol: "s2-bot-lab".into(),
        bots: BTreeMap::new(),
        started_at: String::new(),
        turn_cap: record.settings.max_locks,
        frames_per_turn,
        preview: 5,
        profile: format!("LAB {} PPS", pps),
        start_garbage: Vec::new(),
        deterministic: true,
        threads: record.settings.intetrigence_search.threads,
    };
    Ok(S2LabGame {
        header,
        result,
        snapshots,
        left,
        right,
    })
}

fn empty_state() -> PlayerState {
    PlayerState {
        board: Board::new(),
        colors: ColorLayer::default(),
        hold: None,
        preview: Vec::new(),
        pending: 0,
        combo: 0,
        b2b: 0,
        sent_total: 0,
        received_total: 0,
        pieces: 0,
        lines: 0,
        attack_total: 0,
        think_ms: 0.0,
    }
}

fn update_from_decision(state: &mut PlayerState, entry: &TraceEntry) -> Result<(), String> {
    let rows = s2lab::parse_board(
        &entry.decision.board.cells,
        entry.decision.board.width,
        entry.decision.board.height,
    )?;
    let mut board = Board::new();
    let mut colors = ColorLayer::default();
    for (y, row) in rows.iter().enumerate() {
        for (x, &cell) in row.iter().enumerate() {
            match cell {
                '_' => {}
                'G' => board.set_cell(x, y, Cell::Garbage),
                'I' | 'O' | 'T' | 'S' | 'Z' | 'J' | 'L' => {
                    board.set_cell(x, y, Cell::Block);
                    colors.cells[x][y] = replay::parse_piece(&cell.to_string());
                }
                other => {
                    return Err(format!(
                        "{}: unknown board cell {other:?} at ({x},{y})",
                        entry.side
                    ));
                }
            }
        }
    }
    state.board = board;
    state.colors = colors;
    state.hold = entry
        .decision
        .pieces
        .hold
        .as_deref()
        .and_then(replay::parse_piece);
    state.preview = entry
        .decision
        .pieces
        .known
        .iter()
        .take(5)
        .filter_map(|piece| replay::parse_piece(piece))
        .collect();
    state.pending = entry.decision.incoming.pending_rows;
    state.combo = entry.decision.chain.combo;
    state.b2b = entry.decision.chain.b2b;
    state.pieces = entry.decision.lock_time.pieces_placed;
    state.think_ms = entry.think_ms;
    Ok(())
}

fn side_index(side: &str) -> Result<usize, String> {
    match side {
        "left" => Ok(0),
        "right" => Ok(1),
        other => Err(format!("unknown side {other:?}")),
    }
}

fn cells4(cells: &[[i32; 2]]) -> Result<[(i32, i32); 4], String> {
    if cells.len() != 4 {
        return Err(format!("placement has {} cells, expected 4", cells.len()));
    }
    Ok([
        (cells[0][0], cells[0][1]),
        (cells[1][0], cells[1][1]),
        (cells[2][0], cells[2][1]),
        (cells[3][0], cells[3][1]),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn side_names_are_strict() {
        assert_eq!(side_index("left").unwrap(), 0);
        assert_eq!(side_index("right").unwrap(), 1);
        assert!(side_index("middle").is_err());
    }

    #[test]
    fn cells4_requires_four_cells() {
        assert!(cells4(&[[0, 0], [1, 0], [2, 0], [3, 0]]).is_ok());
        assert!(cells4(&[[0, 0]]).is_err());
    }

    #[test]
    fn empty_state_is_empty() {
        let state = empty_state();
        assert!(state.board.is_empty());
        assert_eq!(state.lines, 0);
    }
}
