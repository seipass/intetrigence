#!/usr/bin/env python3
"""Render battle JSONL v3 movement traces into a diagnostic MP4."""
from __future__ import annotations

import argparse
import json
import math
import subprocess
from pathlib import Path
from typing import Any, Iterator

W, H = 1280, 720
CELL = 26
BOARD_Y = 145
BOARD_X = (110, 770)
BG = (11, 15, 24)
PANEL = (19, 25, 38)
GRID = (42, 52, 70)
EMPTY = (12, 17, 28)
BLOCK = (60, 145, 226)
GARBAGE = (126, 132, 145)
WHITE = (232, 238, 248)
MUTED = (155, 169, 190)
GREEN = (87, 220, 148)
RED = (245, 105, 108)
YELLOW = (246, 205, 90)
PIECE_COLORS = {
    "I": (35, 200, 225),
    "O": (235, 205, 45),
    "T": (165, 85, 205),
    "S": (85, 190, 105),
    "Z": (220, 70, 80),
    "J": (55, 105, 215),
    "L": (235, 145, 50),
}
PIECE_CELLS = {
    "I": [(-1, 0), (0, 0), (1, 0), (2, 0)],
    "O": [(0, 0), (1, 0), (0, 1), (1, 1)],
    "T": [(-1, 0), (0, 0), (1, 0), (0, 1)],
    "L": [(-1, 0), (0, 0), (1, 0), (1, 1)],
    "J": [(-1, 0), (0, 0), (1, 0), (-1, 1)],
    "S": [(-1, 0), (0, 0), (0, 1), (1, 1)],
    "Z": [(-1, 1), (0, 1), (0, 0), (1, 0)],
}

FONT = {
    "A": ["01110", "10001", "10001", "11111", "10001", "10001", "10001"],
    "B": ["11110", "10001", "10001", "11110", "10001", "10001", "11110"],
    "C": ["01111", "10000", "10000", "10000", "10000", "10000", "01111"],
    "D": ["11110", "10001", "10001", "10001", "10001", "10001", "11110"],
    "E": ["11111", "10000", "10000", "11110", "10000", "10000", "11111"],
    "F": ["11111", "10000", "10000", "11110", "10000", "10000", "10000"],
    "G": ["01111", "10000", "10000", "10111", "10001", "10001", "01111"],
    "H": ["10001", "10001", "10001", "11111", "10001", "10001", "10001"],
    "I": ["11111", "00100", "00100", "00100", "00100", "00100", "11111"],
    "J": ["00111", "00010", "00010", "00010", "10010", "10010", "01100"],
    "K": ["10001", "10010", "10100", "11000", "10100", "10010", "10001"],
    "L": ["10000", "10000", "10000", "10000", "10000", "10000", "11111"],
    "M": ["10001", "11011", "10101", "10101", "10001", "10001", "10001"],
    "N": ["10001", "11001", "10101", "10011", "10001", "10001", "10001"],
    "O": ["01110", "10001", "10001", "10001", "10001", "10001", "01110"],
    "P": ["11110", "10001", "10001", "11110", "10000", "10000", "10000"],
    "R": ["11110", "10001", "10001", "11110", "10100", "10010", "10001"],
    "S": ["01111", "10000", "10000", "01110", "00001", "00001", "11110"],
    "T": ["11111", "00100", "00100", "00100", "00100", "00100", "00100"],
    "U": ["10001", "10001", "10001", "10001", "10001", "10001", "01110"],
    "V": ["10001", "10001", "10001", "10001", "10001", "01010", "00100"],
    "W": ["10001", "10001", "10001", "10101", "10101", "11011", "10001"],
    "X": ["10001", "10001", "01010", "00100", "01010", "10001", "10001"],
    "Y": ["10001", "10001", "01010", "00100", "00100", "00100", "00100"],
    "Z": ["11111", "00001", "00010", "00100", "01000", "10000", "11111"],
    "0": ["01110", "10001", "10011", "10101", "11001", "10001", "01110"],
    "1": ["00100", "01100", "00100", "00100", "00100", "00100", "01110"],
    "2": ["01110", "10001", "00001", "00010", "00100", "01000", "11111"],
    "3": ["11110", "00001", "00001", "01110", "00001", "00001", "11110"],
    "4": ["00010", "00110", "01010", "10010", "11111", "00010", "00010"],
    "5": ["11111", "10000", "10000", "11110", "00001", "00001", "11110"],
    "6": ["01110", "10000", "10000", "11110", "10001", "10001", "01110"],
    "7": ["11111", "00001", "00010", "00100", "01000", "01000", "01000"],
    "8": ["01110", "10001", "10001", "01110", "10001", "10001", "01110"],
    "9": ["01110", "10001", "10001", "01111", "00001", "00001", "01110"],
    "-": ["00000", "00000", "00000", "11111", "00000", "00000", "00000"],
    ":": ["00000", "00100", "00000", "00000", "00100", "00000", "00000"],
    ".": ["00000", "00000", "00000", "00000", "00000", "00110", "00110"],
    "/": ["00001", "00010", "00010", "00100", "01000", "01000", "10000"],
    "|": ["00100", "00100", "00100", "00100", "00100", "00100", "00100"],
}


class Canvas:
    def __init__(self) -> None:
        self.buf = bytearray(BG * (W * H))

    def rect(self, x0: int, y0: int, x1: int, y1: int, color: tuple[int, int, int]) -> None:
        x0, y0 = max(0, x0), max(0, y0)
        x1, y1 = min(W, x1), min(H, y1)
        if x0 >= x1 or y0 >= y1:
            return
        row = bytes(color) * (x1 - x0)
        for y in range(y0, y1):
            start = (y * W + x0) * 3
            self.buf[start : start + len(row)] = row

    def text(self, x: int, y: int, value: Any, scale: int = 2, color: tuple[int, int, int] = WHITE) -> None:
        for char in str(value).upper():
            if char == " ":
                x += 4 * scale
                continue
            glyph = FONT.get(char, FONT["."])
            for gy, line in enumerate(glyph):
                for gx, bit in enumerate(line):
                    if bit == "1":
                        self.rect(
                            x + gx * scale,
                            y + gy * scale,
                            x + (gx + 1) * scale,
                            y + (gy + 1) * scale,
                            color,
                        )
            x += 6 * scale

    def ppm(self) -> bytes:
        return f"P6\n{W} {H}\n255\n".encode() + self.buf


def empty_board() -> list[list[str | None]]:
    return [[None] * 10 for _ in range(40)]


def spawn_placement(piece: Any) -> dict[str, Any] | None:
    if not piece or piece == "-":
        return None
    return {
        "location": {
            "type": str(piece).upper(),
            "orientation": "north",
            "x": 4,
            "y": 19,
        },
        "spin": "none",
    }


def orientation_cells(piece: Any, orientation: Any) -> list[tuple[int, int]]:
    base = PIECE_CELLS.get(str(piece).upper(), [])
    name = str(orientation).lower()
    if name == "east":
        return [(y, -x) for x, y in base]
    if name == "south":
        return [(-x, -y) for x, y in base]
    if name == "west":
        return [(-y, x) for x, y in base]
    return list(base)


def placement_cells(placement: dict[str, Any] | None) -> list[tuple[int, int]]:
    if not placement:
        return []
    location = placement.get("location") or {}
    piece = location.get("type")
    try:
        x, y = int(location["x"]), int(location["y"])
    except (KeyError, TypeError, ValueError):
        return []
    return [(x + dx, y + dy) for dx, dy in orientation_cells(piece, location.get("orientation"))]


def ghost_placement(board: Any, placement: dict[str, Any] | None) -> dict[str, Any] | None:
    if not placement:
        return None
    cells = placement_cells(placement)
    if not cells:
        return None
    rows = board if isinstance(board, list) else []

    def occupied(x: int, y: int) -> bool:
        if x < 0 or x >= 10 or y < 0 or y >= 40:
            return True
        row = rows[y] if y < len(rows) and isinstance(rows[y], list) else []
        return x < len(row) and row[x] is not None

    dropped = json.loads(json.dumps(placement))
    while all(not occupied(x, y - 1) for x, y in placement_cells(dropped)):
        dropped["location"]["y"] -= 1
    return dropped


def draw_piece_cells(
    canvas: Canvas,
    x: int,
    placement: dict[str, Any] | None,
    color: tuple[int, int, int],
    alpha: bool = False,
) -> None:
    for col, row in placement_cells(placement):
        if not (0 <= col < 10 and 0 <= row < 20):
            continue
        left = x + col * CELL
        top = BOARD_Y + (19 - row) * CELL
        fill = tuple(max(0, channel // 3) for channel in color) if alpha else color
        canvas.rect(left + 1, top + 1, left + CELL - 1, top + CELL - 1, fill)
        canvas.rect(left, top + CELL - 1, left + CELL, top + CELL, GRID)
        canvas.rect(left + CELL - 1, top, left + CELL, top + CELL, GRID)


def draw_board(
    canvas: Canvas,
    x: int,
    cells: Any,
    active: dict[str, Any] | None = None,
) -> None:
    canvas.rect(x - 8, BOARD_Y - 8, x + CELL * 10 + 8, BOARD_Y + CELL * 20 + 8, PANEL)
    rows = cells if isinstance(cells, list) else []
    for row in range(20):
        values = rows[row] if row < len(rows) and isinstance(rows[row], list) else []
        y = BOARD_Y + (19 - row) * CELL
        for col in range(10):
            value = values[col] if col < len(values) else None
            if value == "garbage":
                color = GARBAGE
            elif value == "block":
                color = BLOCK
            else:
                color = PIECE_COLORS.get(str(value).upper(), EMPTY) if value else EMPTY
            canvas.rect(x + col * CELL + 1, y + 1, x + (col + 1) * CELL - 1, y + CELL - 1, color)
            canvas.rect(x + col * CELL, y + CELL - 1, x + (col + 1) * CELL, y + CELL, GRID)
            canvas.rect(x + (col + 1) * CELL - 1, y, x + (col + 1) * CELL, y + CELL, GRID)
    ghost = ghost_placement(rows, active)
    if ghost and ghost != active:
        draw_piece_cells(canvas, x, ghost, piece_color((active.get("location") or {}).get("type")), alpha=True)
    draw_piece_cells(canvas, x, active, piece_color((active or {}).get("location", {}).get("type")))


def new_player(initial: dict[str, Any]) -> dict[str, Any]:
    active = initial.get("active", "-")
    return {
        "agent": initial.get("agent", "?"),
        "active": active,
        "active_placement": spawn_placement(active),
        "hold": initial.get("hold"),
        "next": initial.get("next", []),
        "board_cells": empty_board(),
        "pieces": 0,
        "lines": 0,
        "attack": 0,
    }


def count_games(path: Path) -> int:
    total = 0
    with path.open(encoding="utf-8") as source:
        for line_number, line in enumerate(source, 1):
            try:
                event = json.loads(line)
            except json.JSONDecodeError as error:
                raise ValueError(f"{path}:{line_number}: invalid JSON: {error}") from error
            if event.get("type") == "battle_start":
                if event.get("replay_version") != 3:
                    raise ValueError(f"{path}:{line_number}: expected replay_version 3")
                total += 1
    if total == 0:
        raise ValueError(f"{path}: no battle_start records")
    return total



def piece_color(piece: Any) -> tuple[int, int, int]:
    return PIECE_COLORS.get(str(piece).upper(), WHITE)


def draw_mini_piece(canvas: Canvas, x: int, y: int, piece: Any, scale: int) -> None:
    if not piece:
        canvas.text(x, y, "-", 3, MUTED)
        return
    name = str(piece).upper()
    cells = PIECE_CELLS.get(name)
    if cells is None:
        canvas.text(x, y, name, 3, WHITE)
        return
    min_x = min(cell[0] for cell in cells)
    min_y = min(cell[1] for cell in cells)
    color = piece_color(name)
    for cell_x, cell_y in cells:
        left = x + (cell_x - min_x) * scale
        top = y + (1 - cell_y + min_y) * scale
        canvas.rect(left + 1, top + 1, left + scale - 1, top + scale - 1, color)
        canvas.rect(left, top + scale - 1, left + scale, top + scale, GRID)
        canvas.rect(left + scale - 1, top, left + scale, top + scale, GRID)


def draw_player(canvas: Canvas, player: dict[str, Any], x: int) -> None:
    name_color = GREEN if player["agent"] == "intetrigence" else YELLOW
    canvas.text(x, 96, player["agent"], 3, name_color)
    draw_board(canvas, x, player["board_cells"], player.get("active_placement"))
    side = x + CELL * 10 + 28
    canvas.text(side, BOARD_Y + 5, "ACTIVE", 2, MUTED)
    draw_mini_piece(canvas, side, BOARD_Y + 25, player.get("active"), 15)
    canvas.text(side, BOARD_Y + 100, "HOLD", 2, MUTED)
    draw_mini_piece(canvas, side, BOARD_Y + 120, player.get("hold"), 15)
    canvas.text(side, BOARD_Y + 195, "NEXT", 2, MUTED)
    for index, piece in enumerate((player.get("next") or [])[:5]):
        draw_mini_piece(canvas, side, BOARD_Y + 220 + index * 31, piece, 12)
    for index, label in enumerate(("PIECES", "LINES", "ATTACK")):
        y = BOARD_Y + 400 + index * 55
        canvas.text(side, y, label, 2, MUTED)
        canvas.text(side, y + 20, player[label.lower()], 2, WHITE)


def render_frame(
    game: int,
    total: int,
    frame: int,
    players: list[dict[str, Any]],
    status: str,
    rules: str,
    mode: str = "MOVEMENT",
) -> bytes:
    canvas = Canvas()
    rules_label = "GUIDELINE" if rules == "guideline" else rules.upper()
    canvas.text(42, 24, f"{players[0]['agent']} VS {players[1]['agent']}", 3, WHITE)
    canvas.text(42, 61, f"NO SIDE SWAP  |  {rules_label} RULES  |  BATTLE REPLAY V3 {mode}", 2, MUTED)
    canvas.text(900, 30, f"GAME {game}/{total}", 2, WHITE)
    canvas.text(900, 55, f"FRAME {frame}", 2, MUTED)
    canvas.text(42, 680, status, 2, GREEN if "WIN" in status else RED if "LOSS" in status else MUTED)
    draw_player(canvas, players[0], BOARD_X[0])
    draw_player(canvas, players[1], BOARD_X[1])
    return canvas.ppm()


def update_player(player: dict[str, Any], event: dict[str, Any]) -> None:
    next_after = event.get("next_after") or []
    player["active"] = next_after[0] if next_after else "-"
    player["active_placement"] = None
    player["hold"] = event.get("hold_after")
    player["next"] = next_after[1:]
    player["board_cells"] = event.get("board_colors") or event.get("board_cells") or player["board_cells"]
    player["pieces"] += 1
    outcome = event.get("outcome") or {}
    player["lines"] += int(outcome.get("lines", 0))
    player["attack"] += int(outcome.get("sent", 0))


def iter_games(input_path: Path) -> Iterator[dict[str, Any]]:
    current: dict[str, Any] | None = None
    with input_path.open(encoding="utf-8") as source:
        for line_number, line in enumerate(source, 1):
            try:
                event = json.loads(line)
            except json.JSONDecodeError as error:
                raise ValueError(f"{input_path}:{line_number}: invalid JSON: {error}") from error
            kind = event.get("type")
            if kind == "battle_start":
                if current is not None:
                    raise ValueError(f"{input_path}:{line_number}: match started before result")
                current = {"start": event, "moves": []}
            elif kind == "battle_move":
                if current is None:
                    raise ValueError(f"{input_path}:{line_number}: move outside match")
                current["moves"].append(event)
            elif kind == "battle_result":
                if current is None:
                    raise ValueError(f"{input_path}:{line_number}: result outside match")
                current["result"] = event
                yield current
                current = None
            else:
                raise ValueError(f"{input_path}:{line_number}: unsupported record type {kind!r}")
    if current is not None:
        raise ValueError(f"{input_path}: match is missing battle_result")


def movement_segments(game: dict[str, Any], player_index: int) -> list[dict[str, Any]] | None:
    moves = sorted(
        (event for event in game["moves"] if int(event.get("player", -1)) == player_index),
        key=lambda event: int(event["frame"]),
    )
    previous_frame = 0
    board = empty_board()
    segments = []
    for event in moves:
        end_frame = int(event["frame"])
        trace = event.get("movement_trace")
        if (
            not isinstance(trace, list)
            or len(trace) != end_frame - previous_frame
            or int(event.get("movement_frames", -1)) != len(trace)
        ):
            return None
        segments.append(
            {
                "start": previous_frame,
                "end": end_frame,
                "event": event,
                "trace": trace,
                "board": board,
                "hold_index": next(
                    (index for index, item in enumerate(event.get("inputs") or []) if item == "hold"),
                    None,
                ),
            }
        )
        board = event.get("board_colors") or event.get("board_cells") or board
        previous_frame = end_frame
    return segments


def movement_frame_players(
    states: list[dict[str, Any]],
    segments: list[list[dict[str, Any]]],
    indices: list[int],
    frame: int,
) -> list[dict[str, Any]]:
    for player_index in range(2):
        player_segments = segments[player_index]
        while indices[player_index] < len(player_segments) and frame >= player_segments[indices[player_index]]["end"]:
            update_player(states[player_index], player_segments[indices[player_index]]["event"])
            indices[player_index] += 1
        if indices[player_index] >= len(player_segments):
            continue
        segment = player_segments[indices[player_index]]
        if frame <= segment["start"]:
            continue
        offset = frame - segment["start"] - 1
        if not 0 <= offset < len(segment["trace"]):
            continue
        event = segment["event"]
        trace_frame = segment["trace"][offset]
        states[player_index]["board_cells"] = segment["board"]
        states[player_index]["hold"] = event.get("hold_before")
        states[player_index]["next"] = event.get("next_before") or []
        if event.get("hold_used") and segment["hold_index"] is not None and offset < segment["hold_index"]:
            states[player_index]["active"] = event.get("active") or "-"
            states[player_index]["active_placement"] = spawn_placement(event.get("active"))
        else:
            placement = trace_frame.get("active")
            states[player_index]["active_placement"] = placement
            states[player_index]["active"] = (placement.get("location") or {}).get("type", "-")
    return [dict(player) for player in states]


def render_snapshot_game(ffmpeg: Any, game: int, total: int, record: dict[str, Any], fps: int) -> None:
    start = record["start"]
    rules = start.get("rules", "arena")
    initial = start.get("initial") or [{}, {}]
    if len(initial) != 2:
        raise ValueError("initial must have two players")
    players = [new_player(initial[0]), new_player(initial[1])]
    snapshot = render_frame(game, total, 0, players, "FIGHT", rules, "SNAPSHOT")
    for _ in range(max(1, fps // 2)):
        ffmpeg.stdin.write(snapshot)
    frame = 0
    for event in record["moves"]:
        player = int(event.get("player", -1))
        if player not in (0, 1):
            raise ValueError(f"invalid player {player}")
        update_player(players[player], event)
        frame = int(event.get("frame", frame))
        ffmpeg.stdin.write(render_frame(game, total, frame, players, "FIGHT", rules, "SNAPSHOT"))
    result = record["result"].get("result") or {}
    winner = result.get("winner")
    status = "DRAW" if winner is None else f"WIN {players[int(winner)]['agent']}"
    snapshot = render_frame(game, total, int(result.get("frames", frame)), players, status, rules, "SNAPSHOT")
    for _ in range(fps * 2):
        ffmpeg.stdin.write(snapshot)


def render_movement_game(ffmpeg: Any, game: int, total: int, record: dict[str, Any], fps: int) -> None:
    start = record["start"]
    rules = start.get("rules", "arena")
    initial = start.get("initial") or [{}, {}]
    if len(initial) != 2:
        raise ValueError("initial must have two players")
    segments = [movement_segments(record, player) for player in range(2)]
    if any(segment is None for segment in segments):
        render_snapshot_game(ffmpeg, game, total, record, fps)
        return
    typed_segments = [segment for segment in segments if segment is not None]
    states = [new_player(initial[0]), new_player(initial[1])]
    indices = [0, 0]
    start_snapshot = render_frame(game, total, 0, states, "FIGHT", rules)
    for _ in range(max(1, fps // 2)):
        ffmpeg.stdin.write(start_snapshot)
    result = record["result"].get("result") or {}
    max_frame = max(
        [int(result.get("frames", 0))]
        + [segment[-1]["end"] for segment in typed_segments if segment]
    )
    output_frames = max(1, math.ceil(max_frame * fps / 60))
    samples = [
        min(max_frame, (output_index * 60 + fps // 2) // fps)
        for output_index in range(output_frames + 1)
    ]
    if samples[-1] != max_frame:
        samples.append(max_frame)
    for simulation_frame in samples:
        players = movement_frame_players(states, typed_segments, indices, simulation_frame)
        ffmpeg.stdin.write(render_frame(game, total, simulation_frame, players, "FIGHT", rules))
    winner = result.get("winner")
    status = "DRAW" if winner is None else f"WIN {states[int(winner)]['agent']}"
    final_players = movement_frame_players(states, typed_segments, indices, max_frame)
    final_snapshot = render_frame(game, total, max_frame, final_players, status, rules)
    for _ in range(fps * 2):
        ffmpeg.stdin.write(final_snapshot)


def render(input_path: Path, output_path: Path, fps: int) -> None:
    total = count_games(input_path)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    command = [
        "ffmpeg", "-y", "-loglevel", "error", "-f", "image2pipe", "-vcodec", "ppm",
        "-r", str(fps), "-i", "-", "-an", "-c:v", "mpeg4", "-q:v", "5",
        "-pix_fmt", "yuv420p", str(output_path),
    ]
    try:
        ffmpeg = subprocess.Popen(command, stdin=subprocess.PIPE)
    except OSError as error:
        raise RuntimeError("ffmpeg is required to render replay video") from error
    rendered = 0
    try:
        for rendered, record in enumerate(iter_games(input_path), 1):
            has_trace = all(
                isinstance(event.get("movement_trace"), list)
                for event in record["moves"]
            )
            if has_trace:
                render_movement_game(ffmpeg, rendered, total, record, fps)
            else:
                render_snapshot_game(ffmpeg, rendered, total, record, fps)
        ffmpeg.stdin.close()
        return_code = ffmpeg.wait()
    except BrokenPipeError as error:
        ffmpeg.kill()
        raise RuntimeError("ffmpeg could not encode the replay video") from error
    except Exception:
        ffmpeg.kill()
        ffmpeg.wait()
        raise
    if return_code != 0:
        raise RuntimeError(f"ffmpeg exited with status {return_code}")
    if rendered != total:
        raise ValueError(f"rendered {rendered} games after counting {total}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path, help="battle JSONL v3 replay")
    parser.add_argument("--output", required=True, type=Path, help="MP4 output path")
    parser.add_argument("--fps", type=int, default=20, help="output frame rate (default: 20)")
    args = parser.parse_args()
    if not 1 <= args.fps <= 60:
        parser.error("--fps must be between 1 and 60")
    render(args.input, args.output, args.fps)


if __name__ == "__main__":
    main()
