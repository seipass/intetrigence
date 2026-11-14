#!/usr/bin/env python3
"""Compare two match summaries on the M4 gate metrics.

Usage:
    python scripts/compare-m3-m4.py <baseline-summary.json> <reserve-summary.json>

Prints a Markdown table (the M3 tuning run vs the M4 reserve run) so the M4
report can show that the reserve metrics are consistent with tuning.
"""
import json
import sys


def load(path):
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def mean(values):
    return sum(values) / len(values) if values else 0.0


def metrics(summary):
    games = summary["games"]
    bot = summary["bots"][0]
    return {
        "games": len(games),
        "wins": summary["wins"],
        "losses": summary["losses"],
        "draws": summary["draws"],
        "win_rate": summary["win_rate"] * 100.0,
        "wilson": (summary["wilson_low"] * 100.0, summary["wilson_high"] * 100.0),
        "topout_wins": summary.get("wins_topout", 0),
        "vision_wins": summary.get("wins_forfeit_vision", 0),
        "other_wins": summary.get("wins_forfeit_other", 0),
        "topout_losses": summary.get("losses_topout", 0),
        "forfeit_losses": summary.get("losses_forfeit", 0),
        "turncap": summary.get("turncap", 0),
        "attack_per_piece": bot["attack_per_piece"],
        "received_per_game": bot["received_per_game"],
        "think_median": mean([game["think_median_ms"][0] for game in games]),
        "think_p95": mean([game["think_p95_ms"][0] for game in games]),
        "mean_turns": mean([game["turns"] for game in games]),
        "spin_mismatches": bot["spin_mismatches"],
        "spin_checked": bot["spin_checked"],
        "same_spin_attack": bot["attack_mismatches_same_spin"],
        "attack_checked": bot["attack_checked"],
    }


def main():
    baseline = metrics(load(sys.argv[1]))
    reserve = metrics(load(sys.argv[2]))
    rows = [
        ("games", "games", "{:d}"),
        ("wins / losses / draws", "wins", "{:d}"),
        ("win rate", "win_rate", "{:.1f}%"),
        ("win breakdown", "topout_wins", "{:d}"),
        ("attack/piece", "attack_per_piece", "{:.3f}"),
        ("received garbage/game", "received_per_game", "{:.1f}"),
        ("think median / p95", "think_median", "{:.1f} ms"),
        ("mean turns", "mean_turns", "{:.1f}"),
        ("spin mismatches", "spin_mismatches", "{:d}"),
        ("same-spin attack mismatches", "same_spin_attack", "{:d}"),
    ]
    print("| metric | M3 tuning | M4 reserve |")
    print("|---|---|---|")
    for label, key, fmt in rows:
        if key == "wins":
            left = f"{baseline['wins']} / {baseline['losses']} / {baseline['draws']}"
            right = f"{reserve['wins']} / {reserve['losses']} / {reserve['draws']}"
        elif key == "win_rate":
            left = (
                f"{baseline['win_rate']:.1f}% "
                f"(Wilson {baseline['wilson'][0]:.1f}..{baseline['wilson'][1]:.1f})"
            )
            right = (
                f"{reserve['win_rate']:.1f}% "
                f"(Wilson {reserve['wilson'][0]:.1f}..{reserve['wilson'][1]:.1f})"
            )
        elif key == "topout_wins":
            left = (
                f"TopOut {baseline['topout_wins']} / vision {baseline['vision_wins']} / "
                f"other {baseline['other_wins']} / turn-cap {baseline['turncap']}"
            )
            right = (
                f"TopOut {reserve['topout_wins']} / vision {reserve['vision_wins']} / "
                f"other {reserve['other_wins']} / turn-cap {reserve['turncap']}"
            )
        elif key == "think_median":
            left = f"{baseline['think_median']:.1f} / {baseline['think_p95']:.1f} ms"
            right = f"{reserve['think_median']:.1f} / {reserve['think_p95']:.1f} ms"
        elif key == "spin_mismatches":
            left = f"{baseline['spin_mismatches']} / {baseline['spin_checked']}"
            right = f"{reserve['spin_mismatches']} / {reserve['spin_checked']}"
        elif key == "same_spin_attack":
            left = f"{baseline['same_spin_attack']} / {baseline['attack_checked']}"
            right = f"{reserve['same_spin_attack']} / {reserve['attack_checked']}"
        else:
            left = fmt.format(baseline[key])
            right = fmt.format(reserve[key])
        print(f"| {label} | {left} | {right} |")


if __name__ == "__main__":
    main()
