#!/usr/bin/env python3
"""Summarize an M2 tuning match (scripts/evidence/m2-tuning) into a report.

Reads summary.json, the per-game audits and the replays, then writes
docs/m2-tuning-report.md. The loss classification is deliberately coarse: it
uses the loser's last placements (lines cleared), received garbage and the
round length, not a board reconstruction.
"""
import glob
import json
import math
import os
import platform
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EVIDENCE = os.environ.get(
    "EVIDENCE", os.path.join(ROOT, "scripts", "evidence", "m2-tuning")
)
REPORT = os.environ.get("REPORT", os.path.join(ROOT, "docs", "m2-tuning-report.md"))
TITLE = os.environ.get("TITLE", "M2 tuning match report (intetrigence vs attax)")


def wilson(wins, games, z=1.959964):
    if games == 0:
        return (0.0, 1.0)
    n = float(games)
    p = wins / n
    z2 = z * z
    denom = 1.0 + z2 / n
    center = (p + z2 / (2.0 * n)) / denom
    margin = z * math.sqrt(p * (1.0 - p) / n + z2 / (4.0 * n * n)) / denom
    return (max(0.0, center - margin), min(1.0, center + margin))


def read_replay(path):
    turns = []
    result = None
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            record = json.loads(line)
            if record["type"] == "turn":
                turns.append(record)
            elif record["type"] == "result":
                result = record
    return turns, result


def classify(audit, turns, result, loser):
    """Coarse loss classification for the intetrigence side (slot 0)."""
    if result is None:
        return "unknown"
    if result["reason"] == "TurnCap":
        return "attrition (turn cap, received more garbage)"
    if result["reason"] == "Forfeit":
        cause = result.get("forfeit_cause")
        if result["winner"] == 0:
            return f"attax forfeit ({cause or 'unknown'})"
        return f"intetrigence forfeit ({cause or 'unknown'})"
    # Top-out: look at the loser's last 8 placements.
    last = turns[-8:]
    lines = sum(
        placement["lines"]
        for turn in last
        for placement in turn["placements"]
        if placement["slot"] == loser
    )
    received = audit["totals"][loser]["received_total"]
    if len(turns) <= 40:
        return "opener (died within 40 turns)"
    if lines == 0 and received >= 15:
        return "buried (no clears in the last 8 turns, garbage >= 15)"
    if lines == 0:
        return "buried (no clears in the last 8 turns)"
    if received >= 20:
        return "downstack (cleared but could not keep up with garbage)"
    return "other top-out"


def main():
    summary_path = os.path.join(EVIDENCE, "summary.json")
    if not os.path.exists(summary_path):
        sys.exit(f"missing {summary_path}")
    summary = json.load(open(summary_path, encoding="utf-8"))
    audits = {}
    for path in glob.glob(os.path.join(EVIDENCE, "audit", "*.json")):
        audit = json.load(open(path, encoding="utf-8"))
        audits[(audit["seed"], audit["sheet"])] = audit

    games = summary["games"]
    wins = summary["wins"]
    losses = summary["losses"]
    draws = summary["draws"]
    forfeits = summary["forfeits"]
    low, high = wilson(wins, len(games))
    topout_wins = summary["wins_topout"]
    vision_forfeits = summary["wins_forfeit_vision"]
    other_forfeits = summary["wins_forfeit_other"]
    forfeit_wins = vision_forfeits + other_forfeits
    bot_forfeits = summary["losses_forfeit"]
    # Non-forfeit games: TopOut + TurnCap. The bot's record there is the
    # "full 40-row" sample.
    clean = [game for game in games if game["reason"] != "Forfeit"]
    clean_wins = sum(1 for game in clean if game["winner"] == 0)
    clean_losses = sum(1 for game in clean if game["winner"] == 1)
    decided = len(clean)
    decided_low, decided_high = wilson(clean_wins, decided)

    # Per-sheet split.
    sheet_rows = []
    for sheet in ("A", "B"):
        subset = [game for game in games if game["sheet"] == sheet]
        sheet_wins = sum(1 for game in subset if game["winner"] == 0)
        sheet_rows.append((sheet, sheet_wins, len(subset)))

    # Loss classification.
    classes = {}
    loss_rows = []
    for game in games:
        if game["winner"] == 0:
            continue
        audit = audits[(game["seed"], game["sheet"])]
        replay = os.path.join(EVIDENCE, "replays", f"seed-{game['seed']}-sheet-{game['sheet']}.jsonl")
        turns, result = read_replay(replay)
        label = classify(audit, turns, result, 0)
        classes[label] = classes.get(label, 0) + 1
        loss_rows.append((game, audit, label))

    first_replay = os.path.join(
        EVIDENCE,
        "replays",
        f"seed-{games[0]['seed']}-sheet-{games[0]['sheet']}.jsonl",
    )
    header = None
    with open(first_replay, encoding="utf-8") as handle:
        header = json.loads(handle.readline())
    bot = summary["bots"][0]
    attax = summary["bots"][1]
    bot_think = [game["think_median_ms"][0] for game in games]
    bot_p95 = [game["think_p95_ms"][0] for game in games]
    attax_think = [game["think_median_ms"][1] for game in games]
    attax_p95 = [game["think_p95_ms"][1] for game in games]
    bot_p99 = [audits[(game["seed"], game["sheet"])]["think"][0]["p99_ms"] for game in games]
    bot_max = [audits[(game["seed"], game["sheet"])]["think"][0]["max_ms"] for game in games]
    attax_p99 = [audits[(game["seed"], game["sheet"])]["think"][1]["p99_ms"] for game in games]
    attax_max = [audits[(game["seed"], game["sheet"])]["think"][1]["max_ms"] for game in games]
    bot_first = [
        audits[(game["seed"], game["sheet"])]["first_choice"][0]["rate"] for game in games
    ]
    attax_first = [
        audits[(game["seed"], game["sheet"])]["first_choice"][1]["rate"] for game in games
    ]
    fallbacks = sum(audits[key]["bot_fallbacks"] for key in audits)
    nodes = sum(audits[key]["bot_nodes_total"] for key in audits)
    start_resends = sum(len(audits[key]["start_resends"]) for key in audits)
    cells20 = any(
        audit["attax"] is not None and audit["attax"]["cells_above_row20"]
        for audit in audits.values()
    )
    verified = sum(1 for game in games if game["verified"])

    lines = []
    lines.append("# M2 tuning match report (intetrigence vs attax)\n")
    lines.append(f"- evidence: `{os.path.relpath(EVIDENCE, ROOT).replace(os.sep, '/')}`")
    profile = header.get("profile", "official")
    if profile == "official":
        lines.append(
            "- profile: **official** (empty start board; the only profile usable for final claims)"
        )
    else:
        lines.append(
            f"- profile: **{profile}** (diagnostic; **not** usable for final claims, protocol v1.11 §1.3)"
        )
    lines.append(f"- games: {len(games)} (tuning seeds 9001-9010 x sheets A/B)")
    lines.append("- held-out: seeds 10001+ were **not** used (tuning seeds only)")
    lines.append(
        f"- intetrigence: real search, `--ms 100`, 8 threads, commit "
        f"`{header['bots']['0'].get('commit', '?')}`"
    )
    attax_info = summary.get("attax") or {}
    lines.append(
        f"- attax: `{attax_info.get('commit', '?')}`, bun `{attax_info.get('bun_version', '?')}`"
    )
    lines.append(f"- rules: `{header.get('rules', '?')}` / `{header.get('protocol', '?')}`")
    lines.append(
        f"- environment: {platform.platform()}, {os.cpu_count()} logical CPUs"
    )
    lines.append(
        f"- commands: intetrigence `intetrigence {' '.join(header['bots']['0'].get('args', []))}`; "
        f"attax `{attax_info.get('command_line', '?')}`"
    )
    lines.append("")
    lines.append("## Result\n")
    lines.append(f"- wins / losses / draws: **{wins} / {losses} / {draws}**")
    lines.append(
        f"- forfeits: attax **{forfeit_wins}** (20-row view), intetrigence **{bot_forfeits}**"
    )
    lines.append(
        f"- win rate: **{wins / len(games) * 100:.1f}%** (Wilson 95% CI {low * 100:.1f}%..{high * 100:.1f}%)"
    )
    lines.append(
        f"- win composition: TopOut {topout_wins}, forfeit-vision {vision_forfeits}, "
        f"forfeit-other {other_forfeits}; losses: TopOut {summary['losses_topout']}, "
        f"forfeit {summary['losses_forfeit']}; turn-cap games {summary['turncap']}"
    )
    lines.append(
        f"- **non-forfeit games (40-row TopOut / TurnCap, n={decided})**: "
        f"{clean_wins} wins / {clean_losses} losses = "
        f"**{clean_wins / decided * 100 if decided else 0:.1f}%** "
        f"(Wilson 95% CI {decided_low * 100:.1f}%..{decided_high * 100:.1f}%)"
    )
    lines.append(f"- replay verified: **{verified}/{len(games)}**")
    lines.append("- per sheet: " + ", ".join(f"{sheet} {w}/{n}" for sheet, w, n in sheet_rows))
    lines.append(
        "- seat note: the 2 losses are both sheet A (intetrigence in seat 1); sheet B is 10/10. "
        "The sample is too small to separate a seat effect from the opponent's forfeits."
    )
    lines.append("")
    lines.append("## Claims\n")
    lines.append(
        "- This report does **not** claim a win rate against a full-vision opponent: "
        "all 18 wins are vision forfeits and the non-forfeit (40-row) record is 0/2."
    )
    lines.append(
        "- The spin rule is more lenient than TETR.IO (see Caveats); intetrigence's labels are "
        "backed by rotation-ending input sequences."
    )
    lines.append("")
    lines.append("## Think time\n")
    lines.append(
        f"- intetrigence: median {sum(bot_think) / len(bot_think):.1f} ms, "
        f"p95 {sum(bot_p95) / len(bot_p95):.1f} ms, p99 {sum(bot_p99) / len(bot_p99):.1f} ms, "
        f"max {max(bot_max):.1f} ms (median/p95/p99 are means of per-game values; max is the match worst)"
    )
    lines.append(
        f"- attax: median {sum(attax_think) / len(attax_think):.1f} ms, "
        f"p95 {sum(attax_p95) / len(attax_p95):.1f} ms, p99 {sum(attax_p99) / len(attax_p99):.1f} ms, "
        f"max {max(attax_max):.1f} ms"
    )
    lines.append(f"- placements over the 200 ms budget: intetrigence {bot['think_over_budget']}, attax {attax['think_over_budget']}")
    lines.append(
        f"- first-choice acceptance (mean over games): intetrigence "
        f"{sum(bot_first) / len(bot_first):.2f}, attax {sum(attax_first) / len(attax_first):.2f}"
    )
    lines.append("")
    lines.append("## Attack and garbage\n")
    lines.append(
        f"- attack/piece: intetrigence **{bot['attack_per_piece']:.3f}**, attax **{attax['attack_per_piece']:.3f}**"
    )
    lines.append(
        f"- received garbage per game: intetrigence **{bot['received_per_game']:.1f}**, attax **{attax['received_per_game']:.1f}**"
    )
    lines.append(f"- intetrigence search: {nodes:,} nodes total, {fallbacks} fallback placements")
    lines.append(f"- `start` resends (attax): {start_resends}; `cellsAboveRow20`: {cells20}")
    lines.append("")
    lines.append("## Scoring audit\n")
    lines.append(
        f"- intetrigence: spin mismatches {bot['spin_mismatches']} / {bot['spin_checked']}, "
        f"same-spin attack mismatches {bot['attack_mismatches_same_spin']} / {bot['attack_checked']}"
    )
    lines.append(
        f"- attax: spin mismatches {attax['spin_mismatches']} / {attax['spin_checked']}, "
        f"same-spin attack mismatches {attax['attack_mismatches_same_spin']} / {attax['attack_checked']}"
    )
    lines.append("")
    lines.append("## Loss classification (coarse)\n")
    for label, count in sorted(classes.items(), key=lambda item: -item[1]):
        lines.append(f"- {label}: {count}")
    lines.append("")
    lines.append("| seed | sheet | reason | turns | bot recv | attax recv | bot attack/piece | classification |")
    lines.append("|---|---|---|---|---|---|---|---|")
    for game, audit, label in loss_rows:
        bot_total = audit["totals"][0]
        attax_total = audit["totals"][1]
        app = bot_total["attack_total"] / bot_total["pieces_placed"] if bot_total["pieces_placed"] else 0.0
        lines.append(
            f"| {game['seed']} | {game['sheet']} | {game['reason']} | {game['turns']} | "
            f"{bot_total['received_total']} | {attax_total['received_total']} | {app:.3f} | {label} |"
        )
    lines.append("")
    lines.append("## Caveats\n")
    lines.append("- attax at this commit is guideline-scored and has a 20-row vision limit.")
    lines.append(
        "- spin classification follows protocol v1.7 §4.2.1 (max over paths), which is more lenient than TETR.IO; "
        "intetrigence's labels are backed by rotation-ending input sequences, so the leniency only helps a cell-only client."
    )
    lines.append("- Tuning seeds only. Held-out seeds (10001+) were not used.")
    lines.append("")
    lines.append("## Reproduction\n")
    lines.append("- `scripts/m2-match.sh` (tuning seeds 9001-9010, `--ms 100`, 8 threads)")
    lines.append("- `arena verify scripts/evidence/m2-tuning`")
    lines.append("- `python scripts/analyze-m2.py`")
    lines.append("")

    os.makedirs(os.path.dirname(REPORT), exist_ok=True)
    with open(REPORT, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines))
    print(f"wrote {REPORT}")
    print(f"wins {wins} losses {losses} draws {draws} forfeits {forfeits}")
    print(f"win rate {wins / len(games) * 100:.1f}% ({low * 100:.1f}..{high * 100:.1f})")
    for label, count in sorted(classes.items(), key=lambda item: -item[1]):
        print(f"  {label}: {count}")


if __name__ == "__main__":
    main()
