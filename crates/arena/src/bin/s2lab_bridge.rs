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

use intetrigence_bot::{EvalContext, RootState, SearchConfig, Searcher};
use intetrigence_core::{Board, PieceKind, Rotation};

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
    if cells.chars().count() != 400 {
        return Err(format!("expected 400 board cells, got {}", cells.chars().count()));
    }
    let chars: Vec<char> = cells.chars().collect();
    let mut board = Board::new();
    for y in 0..40usize {
        let mut row = 0u16;
        for x in 0..10usize {
            if chars[y * 10 + x] != '_' {
                row |= 1 << x;
            }
        }
        board.set_row(y, row);
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
                let _ = writeln!(stdout, "{}", serde_json::json!({ "error": error.to_string() }));
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

    let context = EvalContext {
        combo: request["chain"]["combo"].as_u64().unwrap_or(0) as u32,
        // The lab carries the exact chain length; TBP only carries a bool.
        b2b_streak: request["chain"]["b2b"].as_u64().unwrap_or(0) as u32,
        incoming: request["incoming"]["pendingRows"].as_u64().unwrap_or(0) as u32,
        arrived: request["incoming"]["dueThisLockRows"]
            .as_u64()
            .unwrap_or(0) as u32,
        pieces_placed: 0,
        sent_total: 0,
        received_total: 0,
        t_slot: 0,
    };

    let root = RootState {
        board,
        current,
        preview,
        hold: if hold_available { hold } else { None },
        context,
    };

    let searcher = searcher.get_or_insert_with(|| {
        Searcher::new(SearchConfig {
            deterministic: true,
            depth: 8,
            width: 88,
            ..SearchConfig::default()
        })
    });
    let result = searcher
        .search(&root)
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
