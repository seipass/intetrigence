# Post-search-fix tuning smoke (CC2 20 + clys 10)

> **Diagnostic, tuning seeds only.** Seeds 9001–9010 are never used for a
> claim. This run measures the build after the search fix (completed iterations
> kept, deepen-first) and the harness fixes (content-based measurement
> completeness, fixed build commit, truncated per-game logs).

## Setup

- build: **`121a8cf`** (release binary). Bot commit **`2799058`**
  (non-linear danger, race term, combo weight 12.0, attack up); the later
  combo-state commit (`53896a4`, `ln(1.25·combo+1)` with `COMBO_CAP = 4`) is
  **not** in this build.
- weights: bot defaults; no `--weights` overrides (the replay header records
  the build commit, not the weight vector; the bot commit pins the defaults).
- `--bot search --ms 100 --threads 8`, 1000 turn cap, official profile.
- CC2: `ed8b193...` + `adapters/cc2-tbp-compat.patch`, `--tbp-*` path.
- clys: `ccae45f...` + `adapters/clys-tbp-spawn.patch` + `adapters/clys/tbp/adapter.ts`.
- evidence: `scripts/evidence/tuning-cc2/` (20 games), `scripts/evidence/tuning-clys/` (10 games).

## Result

| opponent | games | wins_topout | losses_topout | unresolved | no_contest | stuck | verified | measurement_complete |
|---|---|---|---|---|---|---|---|---|
| Cold Clear 2 | 20 | **17** | 3 | 0 | 0 | 0 | 20/20 | **true** |
| clys | 10 | **1** | 8 | 0 | 1 | 0 | 10/10 | **false** (the forfeit game's log is short) |

- CC2: **17/20 = 85.0%** (Wilson 95% CI 64.0–94.8%). The 13 games reused from
  the interrupted first attempt are verified and included.
- clys: 1/10 = 10.0% raw (Wilson 1.8–40.4%); **TopOut-only 1/9 = 11.1%**. The
  one no-contest is a clys process exit (below).
- attack/piece: CC2 **0.719**, clys **0.751**.
- received garbage/game: CC2 **69.05**, clys **75.8**.
- combo bands (slot 0, `combo_after`): CC2 0 **59.9%** / 1–2 26.7% / 3–4
  **10.4%** / 5–9 3.0% / 10+ 0%; clys 0 **57.3%** / 1–2 27.7% / 3–4 **11.3%** /
  5–9 3.7% / 10+ 0%.
- search (intetrigence): CC2 depth median/p95/max **4 / 8 / 8**, width
  144 / 144 / 216, fallback **19/4432 = 0.4%**; clys **5 / 8 / 8**, width
  144 / 144 / 216, fallback **4/1816 = 0.2%**.
- think median: intetrigence 97.4–98.0 ms; CC2 ~100.1 ms; clys ~110 ms.

### CC2 per game

| seed | sheet | result | turns | attack/piece | received | depth median | fallback |
|---|---|---|---|---|---|---|---|
| 9001 | A | loss | 70 | 0.557 | 28 | 5 | 1 |
| 9001 | B | win | 172 | 0.801 | 63 | 3 | 1 |
| 9002 | A | win | 299 | 0.592 | 112 | 3 | 2 |
| 9002 | B | win | 313 | 0.821 | 114 | 5 | 0 |
| 9003 | A | win | 224 | 0.688 | 65 | 5 | 0 |
| 9003 | B | win | 377 | 0.699 | 103 | 5 | 0 |
| 9004 | A | loss | 173 | 0.486 | 70 | 4 | 1 |
| 9004 | B | win | 96 | 0.758 | 15 | 5 | 0 |
| 9005 | A | win | 36 | 0.694 | 6 | 3 | 1 |
| 9005 | B | win | 113 | 0.705 | 32 | 4 | 1 |
| 9006 | A | win | 175 | 0.869 | 52 | 5 | 0 |
| 9006 | B | win | 112 | 0.838 | 36 | 4 | 1 |
| 9007 | A | win | 102 | 0.814 | 32 | 4 | 2 |
| 9007 | B | win | 159 | 0.829 | 43 | 5 | 2 |
| 9008 | A | win | 124 | 0.726 | 39 | 4 | 0 |
| 9008 | B | loss | 336 | 0.640 | 110 | 4 | 2 |
| 9009 | A | win | 210 | 0.867 | 46 | 5 | 0 |
| 9009 | B | win | 102 | 0.733 | 41 | 5 | 1 |
| 9010 | A | win | 439 | 0.670 | 122 | 4 | 1 |
| 9010 | B | win | 809 | 0.724 | 252 | 4 | 3 |

### clys per game

| seed | sheet | result | turns | attack/piece | received | depth median | fallback |
|---|---|---|---|---|---|---|---|
| 9001 | A | loss | 212 | 0.802 | 88 | 4 | 0 |
| 9002 | A | **no contest** | 78 | 0.885 | 37 | 4 | 0 |
| 9003 | A | loss | 127 | 0.614 | 59 | 5 | 0 |
| 9004 | A | win | 592 | 0.792 | 217 | 4 | 3 |
| 9005 | A | loss | 329 | 0.787 | 142 | 5 | 0 |
| 9006 | A | loss | 93 | 0.742 | 44 | 4 | 0 |
| 9007 | A | loss | 157 | 0.669 | 63 | 5 | 0 |
| 9008 | A | loss | 53 | 0.472 | 26 | 5 | 0 |
| 9009 | A | loss | 75 | 0.653 | 41 | 5 | 1 |
| 9010 | A | loss | 100 | 0.700 | 41 | 5 | 0 |

## The clys no-contest (9002-A)

`audit/seed-9002-sheet-A.json`: `winner = None`, `reason = "Forfeit"`,
`detail = "agent i/o: bot closed stdout"`, `forfeit_cause = "other"`,
`forfeit_slot = 1` (clys), 78 turns. The clys child process exited mid-game;
the adapter forwards the child's exit (`child.on("exit")` → `process.exit`),
and the referee sees EOF. The clys stderr file is **0 bytes**, so the child
did not print a panic; the adapter's `clys-exit` code/signal is only logged
when `CLYS_TBP_LOG` is set (it was not). The transcript has 77 suggestions for
78 expected turns, so `measurement_complete` is false for this game.

This is **not** a board desync (the attax board-dump work does not apply):
the clys process died. The clys stage requires `no_contest == 0`, so this is a
blocker for the clys stage independent of the win rate. A harness-level
restart-and-resend (`start` with the current board/queue) is the candidate
recovery; it needs a protocol decision because a bot's process death is
normally a forfeit.

## Interpretation

- **CC2 is close to the target**: 17/20 (85%) with 0 forfeits, 0 turn-caps,
  and a working search (depth 4–8, fallback 0.4%). The stage gate is 50/50,
  so a 20-game 85% is not yet the gate; the remaining gaps are the attack
  metrics.
- **The attack targets are not met**: attack/piece 0.719 (target ≥1.10),
  combo 0 59.9% (target ≤45%), combo 3–4 10.4% (target ≥15%), received/game
  69.05 (target ≤55). The search is working; the evaluation is the gap.
- **clys is the hard stage**: 1/9 TopOut (11.1%), and the process exit is a
  separate blocker. The clys bot is deeper (its think ~110 ms with a native
  search); the current evaluation loses the attack race.
- The bot-dev's combo-state build (`53896a4`) measured CC2 14/20 with combo 0
  38.8% and combo 3–4 14.8% (a/p 0.745): the combo targets move, the win rate
  drops. The two builds are not directly comparable (different bot commits,
  same tuning seeds); both are diagnostic.
- The 1-ply baseline was CC2 10/10 (my 10-game smoke) and clys 2/10; the
  search fix changed the CC2 sample to 17/20 and the clys sample to 1/9. The
  samples and the non-determinism differ; no claim is made from tuning seeds.

## Caveats

- 20 + 10 games, tuning seeds; the CIs are wide (CC2 64–95%).
- The intetrigence search is non-deterministic across runs; the 13 resumed
  games are the original verified replays, not re-runs.
- The clys 9002-A game has `measurement_complete = false`; it is reported as a
  no-contest, not a win or a loss.
- The `nodes` in the search statistics are expanded beam nodes, not leaves
  (≈34 placements per node).
