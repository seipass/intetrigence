//! Bridge between 61bi's s2-bot-lab and this bot.
//!
//! The lab hands a decision to a JavaScript `propose` callback; this binary is
//! what that callback shells out to. One JSON request per line on stdin, one
//! JSON placement per line on stdout, so a single process can serve a whole
//! match.
//!
//! The lab's `PublicDecision` carries the S2 state that TBP does not: the exact
//! `b2b` chain length, the arrived and total garbage rows, and the logical
//! frame. That is why a TBP-connected S2 attax plays with a handicap
//! (`souta-lab/attax#`); here both sides get the same state.

#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::io::{BufRead, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use intetrigence_bot::eval::{GARBAGE_CAP, apply_placement, leaf_value_with_hold, t_slot_lines};
use intetrigence_bot::search::GarbageProjection;
use intetrigence_bot::{
    EvalContext, Movegen, Placement, RootState, SearchConfig, SearchTiming, Searcher, Weights,
    spawn_piece,
};
use intetrigence_core::constants::{
    GARBAGE_INCREASE_PER_FRAME, GARBAGE_MARGIN_FRAMES, GARBAGE_SPEED_FRAMES,
};
use intetrigence_core::{Board, Cell, GarbageMultiplier, PieceKind, Rotation};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Witness {
    kind: String,
    used_hold: bool,
    spin_entry: bool,
    cells: [(i32, i32); 4],
}

fn piece_kind(name: &str) -> Result<PieceKind, String> {
    Ok(match name {
        "I" => PieceKind::I,
        "O" => PieceKind::O,
        "T" => PieceKind::T,
        "S" => PieceKind::S,
        "Z" => PieceKind::Z,
        "J" => PieceKind::J,
        "L" => PieceKind::L,
        other => return Err(format!("unknown piece `{other}`")),
    })
}

fn rotation_name(rotation: Rotation) -> &'static str {
    match rotation {
        Rotation::Zero => "0",
        Rotation::R => "R",
        Rotation::Two => "2",
        Rotation::L => "L",
    }
}

/// `cells` is 400 characters, row-major with row 0 at the **bottom** — the lab
/// counts rows the same way we do, and its top-out check reads the top half as
/// `cells.slice(200)`. No flip.
fn board_from_cells(cells: &str) -> Result<Board, String> {
    if cells.len() != Board::WIDTH * Board::HEIGHT {
        return Err(format!("expected 400 board cells, got {}", cells.len()));
    }
    let mut board = Board::new();
    for (index, value) in cells.bytes().enumerate() {
        let cell = match value {
            b'_' => Cell::Empty,
            b'G' => Cell::Garbage,
            b'I' | b'O' | b'T' | b'S' | b'Z' | b'J' | b'L' => Cell::Block,
            _ => return Err(format!("unknown board cell at index {index}")),
        };
        board.set_cell(index % Board::WIDTH, index / Board::WIDTH, cell);
    }
    Ok(board)
}

fn main() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut searcher: Option<Searcher> = None;
    let mut gpu: Option<GpuClient> = None;

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let request: serde_json::Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(error) => {
                let _ = writeln!(
                    stdout,
                    "{}",
                    serde_json::json!({ "error": error.to_string() })
                );
                let _ = stdout.flush();
                continue;
            }
        };
        let reply = match decide(&request, &mut searcher, &mut gpu) {
            Ok(value) => value,
            Err(error) => serde_json::json!({ "error": error }),
        };
        let _ = writeln!(stdout, "{reply}");
        let _ = stdout.flush();
    }
    if let Some(mut client) = gpu {
        let _ = client.child.kill();
        let _ = client.child.wait();
    }
}

fn decide(
    request: &serde_json::Value,
    searcher: &mut Option<Searcher>,
    gpu: &mut Option<GpuClient>,
) -> Result<serde_json::Value, String> {
    let cells = request["board"]["cells"]
        .as_str()
        .ok_or("missing board.cells")?;
    let board = board_from_cells(cells)?;

    let current = piece_kind(
        request["pieces"]["current"]
            .as_str()
            .ok_or("missing pieces.current")?,
    )?;
    let mut preview = Vec::new();
    if let Some(known) = request["pieces"]["known"].as_array() {
        for name in known {
            preview.push(piece_kind(name.as_str().ok_or("bad pieces.known entry")?)?);
        }
    }
    let hold = match request["pieces"]["hold"].as_str() {
        Some(name) => Some(piece_kind(name)?),
        None => None,
    };
    let hold_available = request["pieces"]["holdAvailable"].as_bool().unwrap_or(true);

    let counter = |value: &serde_json::Value, field: &str| {
        value
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| format!("invalid {field}"))
    };
    let frame = counter(
        &request["lockTime"]["logicalFrame"],
        "lockTime.logicalFrame",
    )?;
    let context = EvalContext {
        multiplier: GarbageMultiplier::at_frame(u64::from(frame)).value(),
        combo: counter(&request["chain"]["combo"], "chain.combo")?,
        // The lab carries the exact chain length; TBP only carries a bool.
        b2b_streak: counter(&request["chain"]["b2b"], "chain.b2b")?,
        incoming: counter(&request["incoming"]["pendingRows"], "incoming.pendingRows")?,
        arrived: counter(
            &request["incoming"]["dueThisLockRows"],
            "incoming.dueThisLockRows",
        )?,
        pieces_placed: counter(
            &request["lockTime"]["piecesPlaced"],
            "lockTime.piecesPlaced",
        )?,
        sent_total: counter(&request["ownTotals"]["sent"], "ownTotals.sent")?,
        received_total: counter(&request["ownTotals"]["received"], "ownTotals.received")?,
        t_slot: 0,
    };

    let root = RootState {
        board,
        current,
        preview,
        hold,
        context,
    };
    let env_usize = |name: &str, fallback: usize| {
        std::env::var(name)
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(fallback)
    };
    let attack_profile = env_usize("S2LAB_ATTACK_PROFILE", 6);
    let timing = request
        .get("framesPerLock")
        .map(|value| {
            let frames = counter(value, "framesPerLock")?;
            if frames == 0 {
                return Err("framesPerLock must be positive".to_string());
            }
            Ok(SearchTiming::from_lab(
                u64::from(frame),
                u64::from(frames),
                context.incoming,
                context.arrived,
            )
            .with_forecast_pressure(attack_profile == 5))
        })
        .transpose()?;

    // The lab runs on a synthetic paced clock rather than a realtime budget, so
    // the search size is a free parameter here. That makes the lab the place to
    // ask how much of the gap to attax is search and how much is evaluation.
    let searcher = searcher.get_or_insert_with(|| {
        let mut weights = Weights::default();
        if attack_profile >= 1 {
            // Test whether fewer low-attack clears leave enough room for
            // difficult clears before the first heavy garbage arrives.
            weights.opener_clear = 0.0;
            weights.combo = 3.0;
            weights.combo_state = 10.0;
            weights.tsd_bonus = 24.0;
            weights.b2b_break = -48.0;
            weights.surge_charge = 8.0;
        }
        if attack_profile == 2 {
            // Reward preparing the first charge, and prefer a concrete send
            // at the root over attack which exists only later in the beam.
            weights.b2b_stock = 24.0;
            weights.root_sent = 20.0;
            weights.incoming = 1.0;
        }
        if attack_profile >= 3 {
            // Keep the offensive weights while reducing the incentive to
            // delay the same packet with repeated low-attack clears.
            weights.defense = 8.0;
            weights.stack_exposure = 10.0;
        }
        if attack_profile == 4 {
            weights.margin_stock = 36.0;
        }
        if attack_profile == 6 {
            weights.opener_clear = 0.0;
            weights.opener_attack = 10.0;
            weights.combo = 3.0;
            weights.combo_state = 14.0;
            weights.tsd_bonus = 20.0;
            weights.b2b_break = -48.0;
            weights.b2b_stock = 14.0;
            weights.margin_stock = 0.0;
            weights.surge_charge = 22.0;
            weights.defense = 12.0;
            weights.stack_exposure = 8.0;
            weights.incoming = 6.0;
            weights.garbage_clear = 18.0;
            weights.wasted_t = -24.0;
            weights.clear = [
                [0.0, -18.0, -12.0, -8.0, 18.0],
                [0.0, 4.0, 14.0, 24.0, 36.0],
                [0.0, 20.0, 40.0, 50.0, 60.0],
            ];
            weights.garbage_cover = -10.0;
            weights.center_height = -2.5;
            weights.col_transitions = -2.0;
            weights.t_slot_open_ratio = 0.35;
            weights.cancel_bonus = 12.0;
            weights.hold_t_bonus = 12.0;
            weights.hold_i_bonus = 8.0;
            weights.height_mult_scale = 0.45;
            weights.surge_cashout_bonus = 6.0;
            weights.well_center_penalty = -6.0;
            weights.downstack_urgency = -1.5;
            weights.b2b_keep_mult = 0.35;
        }
        if let Ok(raw) = std::env::var("S2LAB_WEIGHTS_JSON") {
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                if let Ok(serde_json::Value::Object(map)) =
                    serde_json::from_str::<serde_json::Value>(trimmed)
                {
                    let get = |k: &str| map.get(k).and_then(|v| v.as_f64());
                    if let Some(v) = get("max_height") { weights.max_height = v; }
                    if let Some(v) = get("danger_height") { weights.danger_height = v; }
                    if let Some(v) = get("danger_projected") { weights.danger_projected = v; }
                    if let Some(v) = get("risk_base") { weights.risk_base = v; }
                    if let Some(v) = get("risk_span") { weights.risk_span = v; }
                    if let Some(v) = get("risk_floor") { weights.risk_floor = v; }
                    if let Some(v) = get("holes") { weights.holes = v; }
                    if let Some(v) = get("hole_cover") { weights.hole_cover = v; }
                    if let Some(v) = get("hole_rows") { weights.hole_rows = v; }
                    if let Some(v) = get("bumpiness") { weights.bumpiness = v; }
                    if let Some(v) = get("bumpiness_sq") { weights.bumpiness_sq = v; }
                    if let Some(v) = get("row_transitions") { weights.row_transitions = v; }
                    if let Some(v) = get("well_depth") { weights.well_depth = v; }
                    if let Some(v) = get("extra_wells") { weights.extra_wells = v; }
                    if let Some(v) = get("quad_ready") { weights.quad_ready = v; }
                    if let Some(v) = get("garbage_rows") { weights.garbage_rows = v; }
                    if let Some(v) = get("garbage_clear") { weights.garbage_clear = v; }
                    if let Some(v) = get("attack") { weights.attack = v; }
                    if let Some(v) = get("root_sent") { weights.root_sent = v; }
                    if let Some(v) = get("defense") { weights.defense = v; }
                    if let Some(v) = get("incoming") { weights.incoming = v; }
                    if let Some(v) = get("stack_exposure") { weights.stack_exposure = v; }
                    if let Some(v) = get("b2b_break") { weights.b2b_break = v; }
                    if let Some(v) = get("b2b_stock") { weights.b2b_stock = v; }
                    if let Some(v) = get("margin_stock") { weights.margin_stock = v; }
                    if let Some(v) = get("surge_charge") { weights.surge_charge = v; }
                    if let Some(v) = get("surge_exponent") { weights.surge_exponent = v; }
                    if let Some(v) = get("tsd_bonus") { weights.tsd_bonus = v; }
                    if let Some(v) = get("wasted_t") { weights.wasted_t = v; }
                    if let Some(v) = get("combo") { weights.combo = v; }
                    if let Some(v) = get("combo_state") { weights.combo_state = v; }
                    if let Some(v) = get("opener_attack") { weights.opener_attack = v; }
                    if let Some(v) = get("clear_single") { weights.clear[0][1] = v; }
                    if let Some(v) = get("clear_double") { weights.clear[0][2] = v; }
                    if let Some(v) = get("clear_triple") { weights.clear[0][3] = v; }
                    if let Some(v) = get("clear_quad") { weights.clear[0][4] = v; }
                    if let Some(v) = get("clear_mini1") { weights.clear[1][1] = v; }
                    if let Some(v) = get("clear_mini2") { weights.clear[1][2] = v; }
                    if let Some(v) = get("clear_tss") { weights.clear[2][1] = v; }
                    if let Some(v) = get("clear_tsd") { weights.clear[2][2] = v; }
                    if let Some(v) = get("t_slot1") { weights.t_slot[1] = v; }
                    if let Some(v) = get("t_slot2") { weights.t_slot[2] = v; }
                    if let Some(v) = get("garbage_cover") { weights.garbage_cover = v; }
                    if let Some(v) = get("center_height") { weights.center_height = v; }
                    if let Some(v) = get("col_transitions") { weights.col_transitions = v; }
                    if let Some(v) = get("t_slot_open_ratio") { weights.t_slot_open_ratio = v; }
                    if let Some(v) = get("cancel_bonus") { weights.cancel_bonus = v; }
                    if let Some(v) = get("hold_t_bonus") { weights.hold_t_bonus = v; }
                    if let Some(v) = get("hold_i_bonus") { weights.hold_i_bonus = v; }
                    if let Some(v) = get("height_mult_scale") { weights.height_mult_scale = v; }
                    if let Some(v) = get("surge_cashout_bonus") { weights.surge_cashout_bonus = v; }
                    if let Some(v) = get("well_center_penalty") { weights.well_center_penalty = v; }
                    if let Some(v) = get("downstack_urgency") { weights.downstack_urgency = v; }
                    if let Some(v) = get("b2b_keep_mult") { weights.b2b_keep_mult = v; }
                    if let Some(v) = get("discount") { weights.discount = v; }
                    if let Some(v) = get("perfect_clear") { weights.perfect_clear = v; }
                    if let Some(v) = get("survival_line") { weights.survival_line = v; }
                    if let Some(v) = get("surge_hold_penalty") { weights.surge_hold_penalty = v; }
                }
            }
        }
        Searcher::new(SearchConfig {
            deterministic: true,
            depth: env_usize("S2LAB_DEPTH", 12),
            width: env_usize("S2LAB_WIDTH", 96),
            threads: env_usize("S2LAB_THREADS", 8),
            weights,
            ..SearchConfig::default()
        })
        .with_garbage_projection(match env_usize("S2LAB_PROJECT_GARBAGE", 2) {
            1 => GarbageProjection::Straight,
            2 => GarbageProjection::SplitBurst,
            _ => GarbageProjection::Stop,
        })
    });
    let witnesses: Vec<Witness> = serde_json::from_value(request["legalRoots"].clone())
        .map_err(|error| format!("invalid legalRoots: {error}"))?;
    let roots = witnesses
        .into_iter()
        .filter(|witness| hold_available || !witness.used_hold)
        .map(|mut witness| {
            witness.cells.sort_unstable();
            Ok((
                piece_kind(&witness.kind)?,
                witness.used_hold,
                witness.spin_entry,
                witness.cells,
            ))
        })
        .collect::<Result<_, String>>()?;
    let use_gpu = env_usize("S2LAB_GPU", 0) != 0;
    if use_gpu {
        let cfg = searcher.config();
        let frames_per_lock = request
            .get("framesPerLock")
            .and_then(|v| v.as_u64())
            .unwrap_or(60);
        let proj_mode = env_usize("S2LAB_PROJECT_GARBAGE", 2) as u32;
        let client = match gpu {
            Some(c) => c,
            None => gpu.insert(GpuClient::spawn()?),
        };
        return gpu_decide(
            client,
            &root,
            hold_available,
            &roots,
            cfg.depth,
            cfg.width,
            proj_mode,
            u64::from(frame),
            frames_per_lock,
            attack_profile == 5,
            &cfg.weights,
        );
    }

    let result = searcher
        .search_with_roots(&root, roots, timing.as_ref())
        .ok_or("the search found no placement")?;
    let best = result.best;
    // The lab's piece origins differ from ours, so the occupied cell set is the
    // only unambiguous way for the runner to find its own placement object.
    // Both sides use board space with row 0 at the bottom, so no flip is needed.
    let cells: Vec<[i32; 2]> = best
        .piece
        .cells(best.placement.piece.rotation)
        .iter()
        .map(|(cx, cy)| [best.placement.piece.x + cx, best.placement.piece.y + cy])
        .collect();
    Ok(serde_json::json!({
        "cells": cells,
        "kind": format!("{:?}", best.piece),
        "rotation": rotation_name(best.placement.piece.rotation),
        "x": best.placement.piece.x,
        "y": best.placement.piece.y,
        "usedHold": best.hold,
        "spinEntry": best.placement.spin_entry,
        "depth": result.depth,
        "width": result.width,
        "nodes": result.nodes,
    }))
}

const GPU_PROTO_MAGIC: u32 = 0x4947_5055;
const GPU_MAX_ROOTS: usize = 256;

struct GpuClient {
    child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
    movegen: Movegen,
}

impl GpuClient {
    fn spawn() -> Result<Self, String> {
        let bin = std::env::var("INTETRIGENCE_GPU_BIN")
            .unwrap_or_else(|_| "gpu/build/intetrigence_gpu".to_string());
        let expand = std::env::var("INTETRIGENCE_GPU_EXPAND")
            .unwrap_or_else(|_| "gpu/build/expand.spv".to_string());
        let select = std::env::var("INTETRIGENCE_GPU_SELECT")
            .unwrap_or_else(|_| "gpu/build/select.spv".to_string());
        let mut child = Command::new(&bin)
            .args([&expand, &select, "serve"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("failed to spawn GPU server `{bin}`: {e}"))?;
        let stdin = child.stdin.take().ok_or("GPU server stdin unavailable")?;
        let stdout = child.stdout.take().ok_or("GPU server stdout unavailable")?;
        Ok(Self {
            child,
            stdin,
            stdout,
            movegen: Movegen::new(),
        })
    }
}

fn push_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn push_f32(buf: &mut Vec<u8>, v: f64) {
    buf.extend_from_slice(&(v as f32).to_le_bytes());
}

fn push_weights(buf: &mut Vec<u8>, w: &Weights) {
    push_f32(buf, w.max_height);
    push_f32(buf, w.height_sq);
    push_f32(buf, w.danger_height);
    push_f32(buf, w.quad_ready);
    push_f32(buf, w.defense);
    push_f32(buf, w.risk_base);
    push_f32(buf, w.risk_span);
    push_f32(buf, w.risk_floor);
    push_f32(buf, w.opener_clear);
    push_f32(buf, w.combo_state);
    push_f32(buf, w.combo_potential);
    push_f32(buf, w.race);
    push_f32(buf, w.holes);
    push_f32(buf, w.hole_cover);
    push_f32(buf, w.hole_rows);
    push_f32(buf, w.bumpiness);
    push_f32(buf, w.bumpiness_sq);
    push_f32(buf, w.row_transitions);
    push_f32(buf, w.well_depth);
    push_f32(buf, w.extra_wells);
    push_f32(buf, w.t_slot[0]);
    push_f32(buf, w.t_slot[1]);
    push_f32(buf, w.t_slot[2]);
    push_f32(buf, w.danger_projected);
    push_f32(buf, w.stack_exposure);
    push_f32(buf, w.garbage_rows);
    push_f32(buf, w.attack);
    push_f32(buf, w.root_sent);
    for row in &w.clear {
        for &val in row {
            push_f32(buf, val);
        }
    }
    push_f32(buf, w.perfect_clear);
    push_f32(buf, w.b2b_break);
    push_f32(buf, w.b2b_stock);
    push_f32(buf, w.margin_stock);
    push_f32(buf, w.wasted_t);
    push_f32(buf, w.surge_charge);
    push_f32(buf, w.surge_exponent);
    push_f32(buf, w.surge_hold_penalty);
    push_f32(buf, w.tsd_bonus);
    push_f32(buf, w.mini_penalty);
    push_f32(buf, w.combo);
    push_f32(buf, w.opener_attack);
    push_f32(buf, w.garbage_clear);
    push_f32(buf, if w.survival { 1.0 } else { 0.0 });
    push_f32(buf, w.survival_line);
    push_f32(buf, w.incoming);
    push_f32(buf, w.discount);
    push_f32(buf, w.garbage_cover);
    push_f32(buf, w.center_height);
    push_f32(buf, w.col_transitions);
    push_f32(buf, w.t_slot_open_ratio);
    push_f32(buf, w.cancel_bonus);
    push_f32(buf, w.hold_t_bonus);
    push_f32(buf, w.hold_i_bonus);
    push_f32(buf, w.height_mult_scale);
    push_f32(buf, w.surge_cashout_bonus);
    push_f32(buf, w.well_center_penalty);
    push_f32(buf, w.downstack_urgency);
    push_f32(buf, w.b2b_keep_mult);
}

#[allow(clippy::too_many_arguments)]
fn push_gpu_node(
    buf: &mut Vec<u8>,
    board: &Board,
    next: usize,
    hold: Option<PieceKind>,
    ctx: &EvalContext,
    root_idx: usize,
    terminal: bool,
    reward: f64,
    value: f64,
) {
    for i in 0..20 {
        let r0 = u32::from(board.row(i * 2));
        let r1 = u32::from(board.row(i * 2 + 1));
        push_u32(buf, r0 | (r1 << 16));
    }
    let mut glo = 0u32;
    let mut ghi = 0u32;
    for y in 0..32 {
        if board.garbage_row(y) != 0 {
            glo |= 1u32 << y;
        }
    }
    for y in 32..40 {
        if board.garbage_row(y) != 0 {
            ghi |= 1u32 << (y - 32);
        }
    }
    push_u32(buf, glo);
    push_u32(buf, ghi);
    let hold_u8 = hold.map_or(255u32, |k| k as u32);
    let meta0 = (next as u32 & 0xFF)
        | ((hold_u8 & 0xFF) << 8)
        | (ctx.b2b_streak.min(255) << 16)
        | (ctx.combo.min(255) << 24);
    let flags = if terminal { 1u32 } else { 2u32 }; // bit 1 = can_hold
    let meta1 = (ctx.incoming.min(255))
        | (ctx.arrived.min(255) << 8)
        | ((root_idx as u32 & 0xFF) << 16)
        | (flags << 24);
    push_u32(buf, meta0);
    push_u32(buf, meta1);
    push_u32(buf, ctx.sent_total);
    push_u32(buf, ctx.received_total);
    push_u32(buf, ctx.pieces_placed);
    push_f32(buf, reward);
    push_f32(buf, value);
    push_u32(buf, 0); // key_lo (not needed for ply-1 init beam)
    push_u32(buf, 0); // key_hi
    push_u32(buf, 0); // pad0
}

fn with_garbage_gpu(board: &Board, inserted: usize, hole: usize, proj_mode: u32) -> Board {
    let mut holes = [hole as u8; GARBAGE_CAP as usize];
    if proj_mode == 2 && inserted >= 6 {
        holes[inserted / 2..inserted].fill(((hole + Board::WIDTH / 2) % Board::WIDTH) as u8);
    }
    let mut raised = board.clone();
    raised
        .insert_garbage(&holes[..inserted])
        .expect("legal garbage rows");
    raised
}

type AllowedRootKey = (PieceKind, bool, bool, [(i32, i32); 4]);

#[allow(clippy::too_many_arguments)]
fn gpu_decide(
    client: &mut GpuClient,
    root: &RootState,
    hold_available: bool,
    allowed_roots: &HashSet<AllowedRootKey>,
    depth: usize,
    width: usize,
    proj_mode: u32,
    start_frame: u64,
    frames_per_lock: u64,
    forecast_pressure: bool,
    weights: &Weights,
) -> Result<serde_json::Value, String> {
    let mut seq = Vec::with_capacity(1 + root.preview.len());
    seq.push(root.current);
    seq.extend_from_slice(&root.preview);
    let seq_len = seq.len().min(24);
    let max_horizon = (seq_len + usize::from(root.hold.is_some())).min(24);
    let eff_depth = depth.min(max_horizon).max(1);

    // Build exact 24-level LockForecast matching SearchTiming::from_lab
    let arrivals = [
        (start_frame, root.context.arrived),
        (
            start_frame.saturating_add(GARBAGE_SPEED_FRAMES),
            root.context.incoming.saturating_sub(root.context.arrived),
        ),
    ];
    let mut locks = [(0u32, 0u32, 0u32, 1.0f64); 24];
    let mut frame = start_frame;
    let mut multiplier = root.context.multiplier.max(1.0);
    let mut hypothetical = 0u32;
    for (level, slot) in locks.iter_mut().enumerate() {
        let extra = if forecast_pressure && frame >= GARBAGE_MARGIN_FRAMES && level % 4 == 3 {
            ((3.0 * multiplier).ceil() as u32).min(GARBAGE_CAP)
        } else {
            0
        };
        hypothetical += extra;
        let eligible = arrivals
            .iter()
            .filter(|(due, _)| *due <= frame)
            .map(|(_, rows)| *rows)
            .sum::<u32>()
            + hypothetical;
        *slot = (eligible, hypothetical, extra, multiplier);
        let next_frame = frame.saturating_add(frames_per_lock);
        for _ in frame.max(GARBAGE_MARGIN_FRAMES)..next_frame {
            multiplier += GARBAGE_INCREASE_PER_FRAME;
        }
        frame = next_frame;
    }

    // Expand ply 0 on CPU with exact witness filter
    let mut base_ctx = root.context;
    base_ctx.t_slot = t_slot_lines(&root.board);

    struct RootCandidate {
        piece: PieceKind,
        hold_used: bool,
        placement: Placement,
        board: Board,
        next: usize,
        hold: Option<PieceKind>,
        ctx: EvalContext,
        reward: f64,
        value: f64,
        terminal: bool,
        topped_out: bool,
    }

    let mut cands: Vec<RootCandidate> = Vec::with_capacity(96);
    let mut expand_root_piece = |piece: PieceKind, hold_used: bool| {
        let next = if hold_used && root.hold.is_none() {
            2
        } else {
            1
        };
        let hold = if hold_used {
            Some(root.current)
        } else {
            root.hold
        };
        let t_avail = usize::from(hold == Some(PieceKind::T))
            + seq[next.min(seq_len)..seq_len]
                .iter()
                .filter(|&&k| k == PieceKind::T)
                .count();
        let spawn = spawn_piece(piece);
        for placement in client.movegen.generate_from(&root.board, spawn) {
            let mut cells = placement.cells();
            cells.sort_unstable();
            if !allowed_roots.contains(&(piece, hold_used, placement.spin_entry, cells)) {
                continue;
            }
            let (mut board, reward, mut next_ctx) =
                apply_placement(&root.board, placement, &base_ctx, weights);
            let mut acc = reward.value
                + weights.root_sent * f64::from(next_ctx.sent_total - base_ctx.sent_total);
            let mut topped_out = board.height() > Board::VISIBLE_HEIGHT;
            let inserted = if reward.lines == 0 && !topped_out {
                next_ctx.arrived.min(next_ctx.incoming).min(GARBAGE_CAP)
            } else {
                0
            };
            next_ctx.incoming -= inserted;
            next_ctx.arrived -= inserted;
            next_ctx.received_total += inserted;
            // context_at(1)
            let (elig1, hyp1, extra1, mult1) = locks[1];
            next_ctx.incoming += extra1;
            let removed = (root.context.incoming + hyp1).saturating_sub(next_ctx.incoming);
            next_ctx.arrived = elig1.saturating_sub(removed).min(next_ctx.incoming);
            next_ctx.multiplier = mult1;

            let leaf = if inserted > 0 {
                let ins = inserted as usize;
                if board.height() + ins > Board::VISIBLE_HEIGHT {
                    topped_out = true;
                    f64::NEG_INFINITY
                } else {
                    let mut vals = [0.0f64; Board::WIDTH];
                    for (hole, v) in vals.iter_mut().enumerate() {
                        let raised = with_garbage_gpu(&board, ins, hole, proj_mode);
                        *v = leaf_value_with_hold(&raised, &next_ctx, weights, t_avail, hold);
                    }
                    if proj_mode == 0 {
                        vals.iter().sum::<f64>() / Board::WIDTH as f64
                    } else {
                        let mut cols = std::array::from_fn::<_, { Board::WIDTH }, _>(|h| h);
                        cols.sort_by(|&a, &b| vals[a].total_cmp(&vals[b]));
                        let q_idx = if board.height() + ins >= 18 {
                            1
                        } else if board.height() + ins >= 15 {
                            2
                        } else {
                            4
                        };
                        let med = cols[q_idx];
                        board = with_garbage_gpu(&board, ins, med, proj_mode);
                        vals[med]
                    }
                }
            } else if topped_out {
                f64::NEG_INFINITY
            } else {
                leaf_value_with_hold(&board, &next_ctx, weights, t_avail, hold)
            };
            const SAFE_HEIGHT: usize = Board::VISIBLE_HEIGHT - GARBAGE_CAP as usize;
            acc -= weights.stack_exposure * board.height().saturating_sub(SAFE_HEIGHT) as f64;
            let val = if leaf == f64::NEG_INFINITY {
                f64::NEG_INFINITY
            } else {
                acc + weights.discount * leaf
            };
            let terminal =
                topped_out || (inserted > 0 && proj_mode == 0) || (next >= seq_len && hold.is_none());
            cands.push(RootCandidate {
                piece,
                hold_used,
                placement: *placement,
                board,
                next,
                hold,
                ctx: next_ctx,
                reward: acc,
                value: val,
                terminal,
                topped_out,
            });
        }
    };

    expand_root_piece(root.current, false);
    if hold_available {
        match root.hold {
            Some(held) => expand_root_piece(held, true),
            None => {
                if let Some(&nxt) = seq.get(1) {
                    expand_root_piece(nxt, true);
                }
            }
        }
    }
    if cands.is_empty() {
        return Err("the search found no placement".to_string());
    }
    if cands.iter().any(|c| !c.topped_out) {
        cands.retain(|c| !c.topped_out);
    }
    if cands.len() > GPU_MAX_ROOTS {
        cands.sort_unstable_by(|a, b| b.value.total_cmp(&a.value));
        cands.truncate(GPU_MAX_ROOTS);
    }

    let root_count = cands.len();
    let mut best_scores: Vec<f64> = cands
        .iter()
        .map(|c| {
            if c.terminal || eff_depth <= 1 {
                c.value
            } else {
                f64::NEG_INFINITY
            }
        })
        .collect();

    let live_indices: Vec<usize> = cands
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.terminal)
        .map(|(i, _)| i)
        .collect();

    let mut gpu_overflow = 0u32;
    if eff_depth > 1 && !live_indices.is_empty() {
        let mut req = Vec::with_capacity(4 + 800 + 4 + live_indices.len() * 128);
        push_u32(&mut req, GPU_PROTO_MAGIC);
        // SearchParams (800 bytes)
        for i in 0..24 {
            push_u32(&mut req, seq.get(i).map_or(0u32, |&k| k as u32));
        }
        push_u32(&mut req, seq_len as u32);
        push_u32(&mut req, eff_depth as u32);
        push_u32(&mut req, width.min(4096) as u32);
        push_u32(&mut req, root.context.incoming);
        push_u32(&mut req, proj_mode);
        push_u32(&mut req, root_count as u32);
        push_u32(&mut req, 0);
        push_u32(&mut req, 0);
        for &(elig, hyp, extra, mult) in &locks {
            push_u32(&mut req, elig);
            push_u32(&mut req, hyp);
            push_u32(&mut req, extra);
            push_f32(&mut req, mult);
        }
        push_weights(&mut req, weights);
        push_u32(&mut req, live_indices.len() as u32);
        for &ridx in &live_indices {
            let c = &cands[ridx];
            push_gpu_node(
                &mut req,
                &c.board,
                c.next,
                c.hold,
                &c.ctx,
                ridx,
                c.terminal,
                c.reward,
                c.value,
            );
        }

        client
            .stdin
            .write_all(&req)
            .map_err(|e| format!("GPU write failed: {e}"))?;
        client
            .stdin
            .flush()
            .map_err(|e| format!("GPU flush failed: {e}"))?;

        let mut hdr = [0u8; 16];
        client
            .stdout
            .read_exact(&mut hdr)
            .map_err(|e| format!("GPU read header failed: {e}"))?;
        let magic = u32::from_le_bytes(hdr[0..4].try_into().unwrap());
        if magic != GPU_PROTO_MAGIC {
            return Err(format!("GPU bad response magic 0x{magic:08x}"));
        }
        let rc = u32::from_le_bytes(hdr[4..8].try_into().unwrap()) as usize;
        gpu_overflow = u32::from_le_bytes(hdr[8..12].try_into().unwrap());
        let mut scores_bytes = vec![0u8; rc * 4];
        client
            .stdout
            .read_exact(&mut scores_bytes)
            .map_err(|e| format!("GPU read scores failed: {e}"))?;
        for (i, chunk) in scores_bytes.chunks_exact(4).enumerate() {
            let v = f64::from(f32::from_le_bytes(chunk.try_into().unwrap()));
            if v > -1e29 && v > best_scores[i] {
                best_scores[i] = v;
            }
        }
    }

    // Fallback to ply-1 score if all GPU lines topped out
    if best_scores.iter().all(|&v| v == f64::NEG_INFINITY) {
        for (i, c) in cands.iter().enumerate() {
            best_scores[i] = c.value;
        }
    }
    let best_i = (0..root_count)
        .max_by(|&a, &b| best_scores[a].total_cmp(&best_scores[b]))
        .ok_or("no root candidate")?;
    let best = &cands[best_i];
    let cells: Vec<[i32; 2]> = best
        .piece
        .cells(best.placement.piece.rotation)
        .iter()
        .map(|(cx, cy)| [best.placement.piece.x + cx, best.placement.piece.y + cy])
        .collect();
    Ok(serde_json::json!({
        "cells": cells,
        "kind": format!("{:?}", best.piece),
        "rotation": rotation_name(best.placement.piece.rotation),
        "x": best.placement.piece.x,
        "y": best.placement.piece.y,
        "usedHold": best.hold_used,
        "spinEntry": best.placement.spin_entry,
        "depth": eff_depth,
        "width": width.min(4096),
        "nodes": live_indices.len(),
        "gpu": true,
        "gpuOverflow": gpu_overflow,
    }))
}
