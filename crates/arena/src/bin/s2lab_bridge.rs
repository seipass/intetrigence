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

use std::io::{BufRead, Write};

use intetrigence_bot::{EvalContext, RootState, SearchConfig, SearchTiming, Searcher};
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
        let reply = match decide(&request, &mut searcher) {
            Ok(value) => value,
            Err(error) => serde_json::json!({ "error": error }),
        };
        let _ = writeln!(stdout, "{reply}");
        let _ = stdout.flush();
    }
}

fn decide(
    request: &serde_json::Value,
    searcher: &mut Option<Searcher>,
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
            ))
        })
        .transpose()?;

    // The lab runs on a synthetic paced clock rather than a realtime budget, so
    // the search size is a free parameter here. That makes the lab the place to
    // ask how much of the gap to attax is search and how much is evaluation.
    let env_usize = |name: &str, fallback: usize| {
        std::env::var(name)
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(fallback)
    };
    let searcher = searcher.get_or_insert_with(|| {
        Searcher::new(SearchConfig {
            deterministic: true,
            depth: env_usize("S2LAB_DEPTH", 8),
            width: env_usize("S2LAB_WIDTH", 88),
            threads: env_usize("S2LAB_THREADS", 8),
            ..SearchConfig::default()
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
