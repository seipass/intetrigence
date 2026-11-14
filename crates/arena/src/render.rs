//! Replay -> MP4 renderer.
//!
//! The renderer re-simulates a replay with the same [`Engine`] the referee uses,
//! checks every recorded board hash while doing so, and only then draws frames.
//! The video is therefore a rendering of the verified state, not a guess from
//! the recorded placements.
//!
//! Frames are raw RGB24 written to an `ffmpeg` pipe (`libx264`, yuv420p), so no
//! image or video crate is needed. Layout: two 10x20 boards, hold, five preview
//! pieces, pending garbage, combo/B2B/attack/lines/think counters, and an end
//! card with the winner and the reason.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use intetrigence_core::{Board, Cell, PieceKind, Rotation};

use crate::engine::{Engine, PlacementTarget, board_hash};
use crate::font;
use crate::replay::{self, ForfeitCause, Header, ReplayRecord, ResultRecord};

/// Rendering parameters.
#[derive(Clone, Debug)]
pub struct RenderOptions {
    pub fps: u32,
    pub width: u32,
    pub height: u32,
    /// Seconds of end card after the last turn.
    pub end_card_seconds: f32,
    /// Stop after this many turns (for quick previews).
    pub max_turns: Option<u32>,
    /// Print progress every this many frames (0 disables).
    pub progress_every: u64,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            fps: 60,
            width: 1280,
            height: 720,
            end_card_seconds: 3.0,
            max_turns: None,
            progress_every: 600,
        }
    }
}

/// What was rendered.
#[derive(Clone, Debug)]
pub struct RenderReport {
    pub seed: u64,
    pub sheet: String,
    pub turns: u32,
    pub frames: u64,
    pub duration_seconds: f64,
    pub output: PathBuf,
    pub winner: Option<u8>,
    pub reason: String,
    pub forfeit_cause: Option<String>,
    pub verified: bool,
}

/// Renders `replay_path` to `out_path`.
///
/// Fails when the replay does not verify: a video that disagrees with the
/// replay is worse than no video.
pub fn render_replay(
    replay_path: &Path,
    out_path: &Path,
    options: &RenderOptions,
) -> Result<RenderReport, String> {
    let verify = replay::verify_replay(replay_path).map_err(|error| format!("verify: {error}"))?;
    if !verify.ok {
        return Err(format!(
            "replay verification failed ({} mismatches): {:?}",
            verify.mismatches.len(),
            &verify.mismatches[..verify.mismatches.len().min(5)]
        ));
    }
    let records = replay::read_replay(replay_path).map_err(|error| format!("read: {error}"))?;
    let header = match records.first() {
        Some(ReplayRecord::Header(header)) => header.clone(),
        _ => return Err("first record is not the header".into()),
    };
    let simulation = simulate(&records, &header, options.max_turns)?;

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("create dir: {error}"))?;
    }
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
            &format!("{}x{}", options.width, options.height),
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
            "-movflags",
            "+faststart",
        ])
        .arg(out_path)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|error| format!("spawn ffmpeg: {error}"))?;
    let mut stdin = std::io::BufWriter::new(
        child
            .stdin
            .take()
            .ok_or_else(|| "ffmpeg stdin unavailable".to_string())?,
    );

    let mut canvas = Canvas::new(options.width as usize, options.height as usize);
    let frames_per_turn = header.frames_per_turn.max(1);
    let mut frames = 0u64;
    let draw = DrawContext {
        header: &header,
        options,
        result: simulation.result.as_ref(),
    };
    for (index, snapshot) in simulation.turns.iter().enumerate() {
        let next = simulation.turns.get(index + 1);
        for frame in 0..frames_per_turn {
            canvas.clear(BACKGROUND);
            draw.draw_turn(&mut canvas, snapshot, next, frame, frames_per_turn);
            stdin
                .write_all(&canvas.data)
                .map_err(|error| format!("ffmpeg pipe: {error}"))?;
            frames += 1;
            if options.progress_every > 0 && frames % options.progress_every == 0 {
                eprintln!(
                    "render: {} frames ({:.1}s)",
                    frames,
                    frames as f64 / options.fps as f64
                );
            }
        }
    }
    // End card.
    let end_frames = (options.end_card_seconds * options.fps as f32).round() as u64;
    for frame in 0..end_frames {
        canvas.clear(BACKGROUND);
        draw.draw_end_card(&mut canvas, frame, end_frames);
        stdin
            .write_all(&canvas.data)
            .map_err(|error| format!("ffmpeg pipe: {error}"))?;
        frames += 1;
    }
    drop(stdin);
    let status = child
        .wait()
        .map_err(|error| format!("wait ffmpeg: {error}"))?;
    if !status.success() {
        return Err(format!("ffmpeg exited with {status}"));
    }
    let turns = simulation
        .turns
        .last()
        .map(|turn| turn.turn + 1)
        .unwrap_or(0);
    let duration_seconds = frames as f64 / options.fps as f64;
    Ok(RenderReport {
        seed: header.seed,
        sheet: header.sheet.clone(),
        turns,
        frames,
        duration_seconds,
        output: out_path.to_path_buf(),
        winner: simulation.result.as_ref().and_then(|result| result.winner),
        reason: simulation
            .result
            .as_ref()
            .map(|result| result.reason.clone())
            .unwrap_or_default(),
        forfeit_cause: simulation
            .result
            .as_ref()
            .and_then(|result| result.forfeit_cause)
            .map(|cause| cause.as_str().to_string()),
        verified: true,
    })
}

// ---------------------------------------------------------------------------
// Simulation snapshots
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct ColorLayer {
    cells: [[Option<PieceKind>; 40]; 10],
}

impl Default for ColorLayer {
    fn default() -> Self {
        Self {
            cells: [[None; 40]; 10],
        }
    }
}

impl ColorLayer {
    fn place(&mut self, kind: PieceKind, cells: &[(i32, i32); 4]) {
        for &(x, y) in cells {
            if (0..10).contains(&x) && (0..40).contains(&y) {
                self.cells[x as usize][y as usize] = Some(kind);
            }
        }
    }

    /// Removes `full` rows and shifts everything above them down.
    fn clear_rows(&mut self, full: &[usize]) {
        if full.is_empty() {
            return;
        }
        let mut next = [[None; 40]; 10];
        let mut destination = 0usize;
        for y in 0..40 {
            if full.contains(&y) {
                continue;
            }
            for (next_column, column) in next.iter_mut().zip(self.cells.iter()) {
                next_column[destination] = column[y];
            }
            destination += 1;
        }
        self.cells = next;
    }

    /// Garbage inserted at the bottom pushes the stack up by `rows`.
    fn shift_up(&mut self, rows: usize) {
        if rows == 0 {
            return;
        }
        let mut next = [[None; 40]; 10];
        for (next_column, column) in next.iter_mut().zip(self.cells.iter()) {
            next_column[rows..].copy_from_slice(&column[..40 - rows]);
        }
        self.cells = next;
    }
}

#[derive(Clone)]
struct PlayerState {
    board: Board,
    colors: ColorLayer,
    hold: Option<PieceKind>,
    preview: Vec<PieceKind>,
    pending: u32,
    combo: u32,
    b2b: u32,
    sent_total: u32,
    received_total: u32,
    pieces: u32,
    lines: u32,
    attack_total: u32,
    think_ms: f64,
}

#[derive(Clone)]
struct PlacementView {
    piece: PieceKind,
    cells: [(i32, i32); 4],
    spin: String,
    lines: u8,
    attack: u32,
    perfect_clear: bool,
    topped_out: bool,
    cleared_rows: Vec<usize>,
}

struct TurnSnapshot {
    turn: u32,
    before: [PlayerState; 2],
    after: [PlayerState; 2],
    placements: [Option<PlacementView>; 2],
}

struct Simulation {
    turns: Vec<TurnSnapshot>,
    result: Option<ResultRecord>,
}

fn player_state(
    engine: &mut Engine,
    slot: usize,
    colors: &ColorLayer,
    think_ms: f64,
) -> PlayerState {
    let player = &mut engine.players[slot];
    let board = player.game.board().clone();
    let hold = player.game.held_piece();
    let preview = player.game.preview(5);
    PlayerState {
        board,
        colors: colors.clone(),
        hold,
        preview,
        pending: player.garbage.queue_total(),
        combo: player.scoring.state.combo,
        b2b: player.scoring.state.b2b_streak,
        sent_total: player.sent_total(),
        received_total: player.received_total(),
        pieces: player.game.stats().pieces_placed,
        lines: player.game.stats().lines_cleared,
        attack_total: player.attack_total,
        think_ms,
    }
}

fn full_rows(board: &Board, cells: &[(i32, i32); 4]) -> Vec<usize> {
    let mut masks = [0u16; 40];
    for (y, mask) in masks.iter_mut().enumerate() {
        *mask = board.row(y);
    }
    for &(x, y) in cells {
        if (0..10).contains(&x) && (0..40).contains(&y) {
            masks[y as usize] |= 1u16 << x;
        }
    }
    (0..40)
        .filter(|&y| masks[y] == intetrigence_core::FULL_ROW)
        .collect()
}

fn simulate(
    records: &[ReplayRecord],
    header: &Header,
    max_turns: Option<u32>,
) -> Result<Simulation, String> {
    let mut engine = replay::engine_from_header(header);
    let mut colors: [ColorLayer; 2] = [ColorLayer::default(), ColorLayer::default()];
    let mut turns = Vec::new();
    let mut result = None;
    let mut index = 1;
    while index < records.len() {
        match &records[index] {
            ReplayRecord::Garbage(garbage) if garbage.from == 2 => {
                if garbage.to <= 1 {
                    engine.players[garbage.to as usize]
                        .garbage
                        .receive(garbage.amount, garbage.arrival_frame);
                }
                index += 1;
            }
            ReplayRecord::Turn(turn) => {
                if let Some(limit) = max_turns {
                    if turn.t >= limit {
                        break;
                    }
                }
                engine.turn = turn.t;
                engine.begin_turn();
                let think = [
                    turn.placements
                        .iter()
                        .find(|placement| placement.slot == 0)
                        .map(|placement| placement.think_ms)
                        .unwrap_or(0.0),
                    turn.placements
                        .iter()
                        .find(|placement| placement.slot == 1)
                        .map(|placement| placement.think_ms)
                        .unwrap_or(0.0),
                ];
                let before = [
                    player_state(&mut engine, 0, &colors[0], think[0]),
                    player_state(&mut engine, 1, &colors[1], think[1]),
                ];
                let mut placements: [Option<PlacementView>; 2] = [None, None];
                for placement in &turn.placements {
                    let slot = placement.slot as usize;
                    if slot >= 2 {
                        return Err(format!("t{}: slot {slot} out of range", turn.t));
                    }
                    let kind = replay::parse_piece(&placement.piece)
                        .ok_or_else(|| format!("t{}: bad piece {:?}", turn.t, placement.piece))?;
                    let rotation = Rotation::from_u8(placement.rotation).ok_or_else(|| {
                        format!("t{}: bad rotation {}", turn.t, placement.rotation)
                    })?;
                    let target =
                        PlacementTarget::from_core(kind, rotation, placement.x, placement.y);
                    let claimed = placement
                        .claimed_spin
                        .as_deref()
                        .and_then(replay::parse_spin);
                    let cleared = full_rows(&before[slot].board, &target.cells);
                    let applied = engine
                        .apply(slot, &target, claimed)
                        .map_err(|error| format!("t{} slot{slot}: {error}", turn.t))?;
                    colors[slot].place(kind, &target.cells);
                    colors[slot].clear_rows(&cleared);
                    colors[slot].shift_up(applied.inserted.len());
                    placements[slot] = Some(PlacementView {
                        piece: kind,
                        cells: target.cells,
                        spin: replay::spin_name(applied.spin).to_string(),
                        lines: applied.lines,
                        attack: applied.attack_total,
                        perfect_clear: applied.perfect_clear,
                        topped_out: applied.topped_out,
                        cleared_rows: cleared,
                    });
                }
                index += 1;
                // Garbage packets sent by this turn.
                while let Some(ReplayRecord::Garbage(_)) = records.get(index) {
                    index += 1;
                }
                // Hash check: this is the renderer's own guarantee.
                match records.get(index) {
                    Some(ReplayRecord::Hash(hash)) => {
                        let actual = [
                            board_hash(engine.players[0].game.board()),
                            board_hash(engine.players[1].game.board()),
                        ];
                        if hash.boards != actual {
                            return Err(format!(
                                "t{} hash mismatch: recorded {:?}, recomputed {:?}",
                                turn.t, hash.boards, actual
                            ));
                        }
                        index += 1;
                    }
                    _ => return Err(format!("t{}: missing hash record", turn.t)),
                }
                let after = [
                    player_state(&mut engine, 0, &colors[0], think[0]),
                    player_state(&mut engine, 1, &colors[1], think[1]),
                ];
                turns.push(TurnSnapshot {
                    turn: turn.t,
                    before,
                    after,
                    placements,
                });
            }
            ReplayRecord::Result(record) => {
                result = Some(record.clone());
                index += 1;
            }
            _ => index += 1,
        }
    }
    Ok(Simulation { turns, result })
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

const BACKGROUND: [u8; 3] = [14, 16, 24];
const PANEL: [u8; 3] = [24, 27, 38];
const GRID: [u8; 3] = [38, 42, 56];
const TEXT: [u8; 3] = [226, 232, 240];
const DIM: [u8; 3] = [132, 142, 160];
const ACCENT: [u8; 3] = [90, 200, 250];
const WARN: [u8; 3] = [240, 120, 90];
const GARBAGE: [u8; 3] = [120, 122, 132];

fn piece_color(kind: PieceKind) -> [u8; 3] {
    match kind {
        PieceKind::I => [40, 210, 220],
        PieceKind::O => [230, 220, 60],
        PieceKind::T => [170, 90, 230],
        PieceKind::S => [70, 200, 90],
        PieceKind::Z => [230, 80, 80],
        PieceKind::J => [70, 120, 235],
        PieceKind::L => [240, 150, 50],
    }
}

fn dim(color: [u8; 3], factor: f32) -> [u8; 3] {
    [
        (color[0] as f32 * factor) as u8,
        (color[1] as f32 * factor) as u8,
        (color[2] as f32 * factor) as u8,
    ]
}

struct Canvas {
    width: usize,
    height: usize,
    data: Vec<u8>,
}

impl Canvas {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![0; width * height * 3],
        }
    }

    fn clear(&mut self, color: [u8; 3]) {
        for pixel in self.data.chunks_exact_mut(3) {
            pixel.copy_from_slice(&color);
        }
    }

    fn px(&mut self, x: i32, y: i32, color: [u8; 3]) {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return;
        }
        let index = (y as usize * self.width + x as usize) * 3;
        self.data[index..index + 3].copy_from_slice(&color);
    }

    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: [u8; 3]) {
        for dy in 0..h {
            for dx in 0..w {
                self.px(x + dx, y + dy, color);
            }
        }
    }

    fn outline(&mut self, x: i32, y: i32, w: i32, h: i32, thickness: i32, color: [u8; 3]) {
        self.rect(x, y, w, thickness, color);
        self.rect(x, y + h - thickness, w, thickness, color);
        self.rect(x, y, thickness, h, color);
        self.rect(x + w - thickness, y, thickness, h, color);
    }

    fn blend(&mut self, x: i32, y: i32, w: i32, h: i32, color: [u8; 3], alpha: f32) {
        let alpha = alpha.clamp(0.0, 1.0);
        for dy in 0..h {
            for dx in 0..w {
                let px = x + dx;
                let py = y + dy;
                if px < 0 || py < 0 || px as usize >= self.width || py as usize >= self.height {
                    continue;
                }
                let index = (py as usize * self.width + px as usize) * 3;
                for (channel, &target) in color.iter().enumerate() {
                    let old = self.data[index + channel] as f32;
                    let new = old * (1.0 - alpha) + target as f32 * alpha;
                    self.data[index + channel] = new as u8;
                }
            }
        }
    }

    fn text(&mut self, x: i32, y: i32, scale: i32, text: &str, color: [u8; 3]) {
        let mut cursor = x;
        for c in text.chars() {
            if let Some(glyph) = font::glyph(c) {
                for (row, bits) in glyph.iter().enumerate() {
                    for col in 0..5 {
                        if bits & (1 << (4 - col)) != 0 {
                            self.rect(
                                cursor + col * scale,
                                y + row as i32 * scale,
                                scale,
                                scale,
                                color,
                            );
                        }
                    }
                }
            }
            cursor += 6 * scale;
        }
    }

    fn text_right(&mut self, right: i32, y: i32, scale: i32, text: &str, color: [u8; 3]) {
        let width = font::text_width(text, scale as u32) as i32;
        self.text(right - width, y, scale, text, color);
    }
}

/// Draws a piece in a 4x4 mini grid at `(x, y)` with `cell` pixels per cell.
fn mini_piece(canvas: &mut Canvas, x: i32, y: i32, cell: i32, kind: PieceKind, color: [u8; 3]) {
    let cells = kind.cells(Rotation::Zero);
    let min_x = cells.iter().map(|cell| cell.0).min().unwrap_or(0);
    let max_x = cells.iter().map(|cell| cell.0).max().unwrap_or(0);
    let min_y = cells.iter().map(|cell| cell.1).min().unwrap_or(0);
    let max_y = cells.iter().map(|cell| cell.1).max().unwrap_or(0);
    let width = max_x - min_x + 1;
    let height = max_y - min_y + 1;
    let offset_x = (4 - width) / 2;
    let offset_y = (4 - height) / 2;
    for &(cx, cy) in &cells {
        let px = x + (cx - min_x + offset_x) * cell;
        let py = y + (3 - (cy - min_y + offset_y)) * cell;
        canvas.rect(px, py, cell - 1, cell - 1, color);
    }
}

struct DrawContext<'a> {
    header: &'a Header,
    options: &'a RenderOptions,
    result: Option<&'a ResultRecord>,
}

impl DrawContext<'_> {
    fn draw_turn(
        &self,
        canvas: &mut Canvas,
        snapshot: &TurnSnapshot,
        next: Option<&TurnSnapshot>,
        frame: u64,
        frames_per_turn: u64,
    ) {
        canvas.clear(BACKGROUND);
        self.draw_header(canvas, snapshot.turn);
        let cell = 24;
        let board_y = 96;
        let left_x = 56;
        let right_x = 984;
        for slot in 0..2 {
            let x = if slot == 0 { left_x } else { right_x };
            self.draw_player(
                canvas,
                slot,
                x,
                board_y,
                cell,
                snapshot,
                next,
                frame,
                frames_per_turn,
            );
        }
        self.draw_footer(canvas, snapshot, frame, frames_per_turn);
    }

    fn draw_header(&self, canvas: &mut Canvas, turn: u32) {
        let title = format!(
            "SEED {}   SHEET {}   {}",
            self.header.seed, self.header.sheet, self.header.profile
        )
        .to_uppercase();
        canvas.text(24, 20, 2, &title, TEXT);
        canvas.text_right(1256, 20, 2, "INTETRIGENCE VS ATTAX", ACCENT);
        canvas.rect(24, 52, 1232, 2, GRID);
        let _ = turn;
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_player(
        &self,
        canvas: &mut Canvas,
        slot: usize,
        x: i32,
        y: i32,
        cell: i32,
        snapshot: &TurnSnapshot,
        next: Option<&TurnSnapshot>,
        frame: u64,
        frames_per_turn: u64,
    ) {
        let name = self
            .header
            .slots
            .get(&slot.to_string())
            .cloned()
            .unwrap_or_else(|| format!("slot {slot}"));
        let placement = snapshot.placements[slot].as_ref();
        let fall_frames = (frames_per_turn * 2 / 3).max(1);
        let lock_frames = (frames_per_turn / 6).max(1);
        let lock_end = fall_frames + lock_frames;
        let (state, phase) = if frame >= lock_end {
            (&snapshot.after[slot], 2)
        } else {
            (
                &snapshot.before[slot],
                if frame < fall_frames { 0 } else { 1 },
            )
        };
        canvas.text(
            x,
            y - 28,
            2,
            &name.to_uppercase(),
            if slot == 0 { ACCENT } else { WARN },
        );
        // Board frame and cells.
        canvas.rect(x - 4, y - 4, cell * 10 + 8, cell * 20 + 8, PANEL);
        for by in 0..20 {
            for bx in 0..10 {
                let px = x + bx * cell;
                let py = y + (19 - by) * cell;
                let color = match state.board.cell(bx as usize, by as usize) {
                    Cell::Empty => BACKGROUND,
                    Cell::Garbage => GARBAGE,
                    Cell::Block => state.colors.cells[bx as usize][by as usize]
                        .map(piece_color)
                        .unwrap_or([90, 96, 112]),
                };
                canvas.rect(px, py, cell - 1, cell - 1, color);
            }
        }
        canvas.outline(x - 4, y - 4, cell * 10 + 8, cell * 20 + 8, 2, GRID);
        // Falling / locked piece and ghost.
        if let Some(placement) = placement {
            if phase < 2 {
                let ghost = dim(piece_color(placement.piece), 0.45);
                for &(cx, cy) in &placement.cells {
                    if (0..10).contains(&cx) && (0..20).contains(&cy) {
                        let px = x + cx * cell;
                        let py = y + (19 - cy) * cell;
                        canvas.outline(px + 2, py + 2, cell - 5, cell - 5, 2, ghost);
                    }
                }
            }
            let (draw_cells, bright) = match phase {
                0 => {
                    let start_y =
                        (placement.cells.iter().map(|cell| cell.1).max().unwrap_or(0) + 8).min(20);
                    let final_y = placement.cells.iter().map(|cell| cell.1).max().unwrap_or(0);
                    let progress = frame as f32 / fall_frames as f32;
                    let shift = ((start_y - final_y) as f32 * (1.0 - progress)).round() as i32;
                    let mut shifted = placement.cells;
                    for cell in &mut shifted {
                        cell.1 += shift;
                    }
                    (shifted, false)
                }
                1 => (placement.cells, true),
                _ => (placement.cells, false),
            };
            if phase < 2 {
                let color = piece_color(placement.piece);
                for &(cx, cy) in &draw_cells {
                    if (0..10).contains(&cx) && (0..20).contains(&cy) {
                        let px = x + cx * cell;
                        let py = y + (19 - cy) * cell;
                        canvas.rect(px, py, cell - 1, cell - 1, color);
                        if bright {
                            canvas.blend(px, py, cell - 1, cell - 1, [255, 255, 255], 0.35);
                        }
                    }
                }
            }
            // Flash the rows this placement cleared.
            if phase == 1 {
                for &row in &placement.cleared_rows {
                    if row < 20 {
                        let py = y + (19 - row as i32) * cell;
                        canvas.blend(x, py, cell * 10, cell - 1, [255, 255, 255], 0.45);
                    }
                }
            }
        } else if phase == 2 {
            // Next piece waiting at spawn.
            if let Some(next) = next {
                if let Some(next_placement) = next.placements[slot].as_ref() {
                    let color = dim(piece_color(next_placement.piece), 0.8);
                    let spawn = crate::movegen::spawn_state(next_placement.piece);
                    for spawn_cell in spawn.cells() {
                        if (0..10).contains(&spawn_cell.0) && (0..20).contains(&spawn_cell.1) {
                            let px = x + spawn_cell.0 * cell;
                            let py = y + (19 - spawn_cell.1) * cell;
                            canvas.rect(px, py, cell - 1, cell - 1, color);
                        }
                    }
                }
            }
        }
        // Pending garbage bar on the inner side of the board.
        let bar_x = if slot == 0 {
            x + cell * 10 + 10
        } else {
            x - 20
        };
        canvas.rect(bar_x, y, 10, cell * 20, PANEL);
        let pending = state.pending.min(20);
        for index in 0..pending {
            let py = y + (19 - index as i32) * cell;
            canvas.rect(bar_x, py, 10, cell - 1, WARN);
        }
        canvas.text(bar_x - 2, y + cell * 20 + 6, 1, "GARBAGE", DIM);
        // Center panel for this player.
        let panel_x = if slot == 0 { 340 } else { 660 };
        let panel_width = 280;
        canvas.rect(panel_x, y - 4, panel_width, cell * 20 + 8, PANEL);
        canvas.text(panel_x + 10, y + 6, 1, "HOLD", DIM);
        if let Some(hold) = state.hold {
            mini_piece(canvas, panel_x + 10, y + 20, 12, hold, piece_color(hold));
        }
        canvas.text(panel_x + 70, y + 6, 1, "NEXT", DIM);
        for (index, kind) in state.preview.iter().take(5).enumerate() {
            mini_piece(
                canvas,
                panel_x + 70 + index as i32 * 42,
                y + 20,
                10,
                *kind,
                piece_color(*kind),
            );
        }
        let stats = [
            ("B2B", state.b2b.to_string()),
            ("COMBO", state.combo.to_string()),
            ("ATTACK", state.attack_total.to_string()),
            ("SENT", state.sent_total.to_string()),
            ("RECV", state.received_total.to_string()),
            ("PENDING", state.pending.to_string()),
            ("LINES", state.lines.to_string()),
            ("PIECES", state.pieces.to_string()),
            ("THINK", format!("{:.0} MS", state.think_ms)),
        ];
        let mut sy = y + 78;
        for (label, value) in stats {
            canvas.text(panel_x + 10, sy, 1, label, DIM);
            canvas.text(panel_x + 10, sy + 10, 2, &value, TEXT);
            sy += 42;
        }
        // Last placement summary.
        if let Some(placement) = placement {
            let mut summary = String::new();
            if placement.perfect_clear {
                summary.push_str("PERFECT CLEAR");
            } else if placement.spin == "full" {
                summary.push_str("T-SPIN ");
                summary.push_str(line_name(placement.lines));
            } else if placement.spin == "mini" {
                summary.push_str("T-SPIN MINI ");
                summary.push_str(line_name(placement.lines));
            } else if placement.lines > 0 {
                summary.push_str(line_name(placement.lines));
            }
            if placement.attack > 0 {
                summary.push_str(&format!(" +{}", placement.attack));
            }
            if placement.topped_out {
                summary.push_str(" TOP OUT");
            }
            if !summary.is_empty() {
                canvas.text(panel_x + 10, y + cell * 20 - 22, 1, &summary, ACCENT);
            }
        }
    }

    fn draw_footer(
        &self,
        canvas: &mut Canvas,
        snapshot: &TurnSnapshot,
        frame: u64,
        frames_per_turn: u64,
    ) {
        let total = self
            .result
            .map(|result| result.turns)
            .unwrap_or(snapshot.turn + 1);
        canvas.rect(24, 606, 1232, 2, GRID);
        canvas.text(
            24,
            620,
            2,
            &format!("TURN {} / {}", snapshot.turn + 1, total),
            TEXT,
        );
        let seconds = (snapshot.turn as f64 * frames_per_turn as f64 + frame as f64)
            / self.options.fps as f64;
        canvas.text(24, 654, 1, &format!("{:.1} S", seconds), DIM);
        canvas.text_right(1256, 620, 2, &format!("{} FPS", self.options.fps), DIM);
        canvas.text_right(1256, 654, 1, "VERIFIED REPLAY", DIM);
    }

    fn draw_end_card(&self, canvas: &mut Canvas, frame: u64, total: u64) {
        canvas.clear(BACKGROUND);
        let width = self.options.width as i32;
        let height = self.options.height as i32;
        let fade = (frame as f32 / 30.0).clamp(0.0, 1.0);
        canvas.blend(0, 0, width, height, [0, 0, 0], 0.55 + 0.25 * fade);
        let (winner_text, reason_text) = match self.result {
            Some(result) => {
                let winner = match result.winner {
                    Some(0) => self
                        .header
                        .slots
                        .get("0")
                        .cloned()
                        .unwrap_or_else(|| "slot 0".into()),
                    Some(1) => self
                        .header
                        .slots
                        .get("1")
                        .cloned()
                        .unwrap_or_else(|| "slot 1".into()),
                    _ => "DRAW".into(),
                };
                let reason = if result.reason == "Forfeit" {
                    match result.forfeit_cause {
                        Some(ForfeitCause::Vision) => "FORFEIT (VISION)".to_string(),
                        Some(ForfeitCause::Other) => "FORFEIT (OTHER)".to_string(),
                        Some(ForfeitCause::Unknown) => "FORFEIT (UNKNOWN)".to_string(),
                        None => "FORFEIT".to_string(),
                    }
                } else if result.reason == "TurnCap" {
                    "TURN CAP".to_string()
                } else {
                    result.reason.to_uppercase()
                };
                (format!("WINNER: {}", winner.to_uppercase()), reason)
            }
            None => ("NO RESULT RECORD".to_string(), "-".to_string()),
        };
        let center = width / 2;
        canvas.text(center - 120, 170, 4, "GAME OVER", WARN);
        canvas.text(center - 220, 250, 3, &winner_text, TEXT);
        canvas.text(
            center - 140,
            320,
            2,
            &format!("REASON: {reason_text}"),
            ACCENT,
        );
        if let Some(result) = self.result {
            canvas.text(
                center - 100,
                370,
                2,
                &format!("TURNS: {}", result.turns),
                TEXT,
            );
            if result.per_player.len() >= 2 {
                let left = &result.per_player[0];
                let right = &result.per_player[1];
                let line = format!(
                    "ATTACK {} VS {}   RECV {} VS {}   LINES {} VS {}",
                    left.attack_total,
                    right.attack_total,
                    left.received_total,
                    right.received_total,
                    left.lines_cleared,
                    right.lines_cleared
                );
                canvas.text(center - 300, 420, 1, &line, DIM);
            }
        }
        canvas.text(
            center - 240,
            470,
            2,
            &format!(
                "SEED {}  SHEET {}  {}",
                self.header.seed, self.header.sheet, self.header.profile
            )
            .to_uppercase(),
            DIM,
        );
        let _ = total;
    }
}

fn line_name(lines: u8) -> &'static str {
    match lines {
        0 => "",
        1 => "SINGLE",
        2 => "DOUBLE",
        3 => "TRIPLE",
        4 => "QUAD",
        _ => "CLEAR",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_layer_clears_and_shifts() {
        let mut layer = ColorLayer::default();
        layer.place(PieceKind::T, &[(0, 0), (1, 0), (2, 0), (1, 1)]);
        layer.clear_rows(&[0]);
        assert_eq!(layer.cells[1][0], Some(PieceKind::T));
        assert_eq!(layer.cells[0][0], None);
        layer.shift_up(1);
        assert_eq!(layer.cells[1][1], Some(PieceKind::T));
    }

    #[test]
    fn full_rows_detects_a_completed_row() {
        let mut board = Board::new();
        board.set_row(0, 0b1111111110);
        let rows = full_rows(&board, &[(0, 0), (0, 1), (0, 2), (0, 3)]);
        assert_eq!(rows, vec![0]);
    }
}
