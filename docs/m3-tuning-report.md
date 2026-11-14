# M3 tuning report — defence, downstack and SPSA

- evidence: `scripts/evidence/m3-tuning` (matches), `crates/bot/src/bin/tune.rs` (tuner)
- games: 20 (tuning seeds 9001-9010 x sheets A/B)
- intetrigence: real search, `--ms 100`, 8 threads, commit `a723bf3`
- attax: `7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b`, bun `1.4.0`
- comparison: `docs/m2-tuning-report.md` (same seeds, pre-M3 weights)
- held-out seeds (10001+) were **not** used.

## 1. Failure analysis of the two M2 losses

Both M2 losses were 40-row top-outs after a garbage spike, and both replays
show the same shape: the bot was **low and clean right before the spike**, then
went up 15-20 rows in a handful of turns without clearing.

### 9006-A (M2: top-out at turn 111)

| turn | pending | height | holes | placement |
|---|---|---|---|---|
| 100 | 5 | 5 | 1 | I vertical, 0 lines |
| 101 | **12** | 5 | 0 | T-spin (full), **0 lines** |
| 102 | 8 | 10 | 5 | Z, 0 lines (garbage landed) |
| 103 | 2 | 17 | 12 | S, 0 lines |
| 104-107 | 2-3 | 19-23 | 13-16 | four placements, 0 lines |
| 108 | 2 | 22 | 20 | L, 1 line |
| 109-110 | 0 | 25 | 23 | topped out |

Final board: eight garbage rows at rows 10-17 with holes at columns 1 and 3,
seven rows of player blocks on top with 23 holes. The stack never dug into the
garbage; it stacked *above* it.

### 9010-A (M2: top-out at turn 188)

Same pattern, compressed: turn 180 height 4, pending 2; turns 181-186 pending
2 -> 6 -> 11 -> 8 -> 2 while the height went 6 -> 7 -> 8 -> 10 -> 16 -> 23.
Final board: five garbage rows at rows 10-14 with holes at columns 1, 5 and 8,
eight rows of player blocks above.

### What the evaluation was missing

1. **The danger term only saw the current height.** `height_sq` was
   `max(0, height - 12)^2` on the board as it stood. With 12 rows pending on a
   height-5 board the *projected* height was 17, but the leaf value saw 5.
2. **Pending garbage was a flat -3 per row.** That is linear and small: 12
   rows cost 36, which a T-spin slot bonus (+20) plus shape terms could
   outweigh. The real cost is quadratic, because the projected height is what
   decides whether the next spike kills you.
3. **Uncleared garbage was not priced at all.** The generator rows are full
   rows with one hole; leaving them in place is what actually kills. Only the
   generic `holes` term saw them, at the same price as any other hole.
4. **Clearing garbage was only worth the GSB (+1).** Digging a garbage row is
   worth far more than one line of attack when a spike is queued.

## 2. Versus terms

All in `crates/bot/src/eval.rs`, surfaced through `Weights` and `EvalContext`
(so M4/SPSA can move them):

| term | where | definition | default |
|---|---|---|---|
| `danger_projected` | leaf | `max(0, max_height + incoming - DANGER_BASE)^2` | -12.0 |
| `garbage_rows` | feature | rows that still contain garbage | -6.0 |
| `garbage_clear` | reward | per garbage row cleared by the placement | +10.0 |
| `incoming` | leaf | linear pressure, now secondary | -1.0 (was -3.0) |

`incoming` is the referee's `pending_garbage` (packets that have not arrived
yet, `docs/arena.md` §D1.1). Because it is constant across the children of a
node, `danger_projected` does not add a constant — it changes the *slope* of
the height penalty, which is exactly the "switch to downstack" signal.

A cheap proxy harness was added for tuning: `play_game_with_garbage` plays a
single-player game with scripted garbage and models the referee's cancel flow
(attack cancels the queue first, the opener phase doubles the budget, garbage
is inserted only after a placement that cleared nothing). It also found a real
bug in the clutch fallback: the driver explored from the standard spawn, which
is blocked precisely when Clutch pushed the piece up.

## 3. SPSA

### Design

- **Parameterisation**: multipliers on six weights (`danger_projected`,
  `garbage_rows`, `garbage_clear`, `holes`, `hole_rows`, `attack`), clamped to
  `[0.4, 2.5]`, so the hand-set scale is preserved and the space is bounded.
- **Objective**: the garbage proxy (baseline 2 rows every 12 turns plus a
  12-row spike every 40 turns), `mean(turns survived) + 0.05 * attack` over
  the proxy seeds. The spike schedule is calibrated so the untuned bot dies
  between turns 45 and 208, which leaves room without saturating the turn cap.
- **Determinism**: the proxy runs at a fixed width equal to `max_width` and no
  time budget. The first version used a 10 ms budget and the objective was pure
  machine-load noise; the gradient was meaningless.
- **SPSA**: two-sided perturbation, `c_k = 0.12 / (k+1)^0.101`, normalised step
  (`0.10 / (k+1)^0.25` times the gradient divided by its largest component).
  A plain `a * gradient` pinned every parameter to a bound on the first
  iteration because the objective is in turns (gradient O(100)).
- **Staged verification**: SPSA on proxy seeds 1-6, then a held-back check on
  proxy seeds 7-12, then the real match on tuning seeds 9001-9010.

### Execution record

```
start: objective 274.14 (seeds 1..6)
iter  1: J+ 245.3 J- 169.4 theta 228.4 best 274.1
iter  3: J+ 219.4 J- 190.6 theta 282.7 best 282.7
iter  4: J+ 258.5 J- 130.5 theta 292.5 best 292.5
...
iter 12: J+ 128.7 J- 149.6 theta 234.6 best 292.5
best multipliers: danger_projected x0.669, garbage_rows x0.837,
                  garbage_clear x1.179, holes x1.011, hole_rows x1.179,
                  attack x0.669
```

### Result: rejected (overfitting)

| seed set | default | SPSA-tuned |
|---|---|---|
| proxy 1-6 (tuned on) | 274.14 | **292.50** |
| proxy 7-12 (held back) | **203.29** | 133.52 |

The tuned set improves the seeds it was fitted on by 6.7% and loses 34% on
disjoint seeds. The objective is a step function (survival turns) over six
seeds, so SPSA fits the timing of individual spikes rather than a general
policy. **The default weights therefore stay hand-set**, and the tuner plus
this negative result are kept so M4 can retry with many more seeds and a
smoother objective (for example mean height or garbage rows at fixed
checkpoints rather than survival).

The versus **terms** are principled independently of the weights, and the gate
below measures them.

## 4. Gate re-verification (same protocol as M2)

| metric | M2 | M3 |
|---|---|---|
| wins / losses | 18 / 2 | **20 / 0** |
| win rate | 90.0% | **100.0%** (Wilson 95% CI 83.9%..100.0%) |
| replay verified | 20/20 | **20/20** |
| forfeit-vision wins / turn-cap games | 0 / 0 | **19 / 1** |
| attack/piece | 0.986 | 0.962 |
| received garbage per game | 75.7 | 109.4 |
| think median / p95 | 97.1 / 98.7 ms | 97.1 / 99.0 ms |
| placements over the 200 ms budget | 0 | 0 |
| spin mismatches (ours) | 0 / 5934 | 0 / 7892 |
| same-spin attack mismatches | 0 / 5934 | 0 / 7892 |

### The two M2 loss cases

| seed/sheet | M2 | M3 |
|---|---|---|
| 9006-A | top-out at turn 111 (received 46) | **win**, attax forfeits at turn **859** (received 247) |
| 9010-A | top-out at turn 188 (received 73) | **win**, attax forfeits at turn 140 (received 33) |

9006-A is the clearest evidence: the bot survived 7.7x longer and absorbed
5.4x more garbage in the same seed. 9010-A was decided earlier, before the
late-game spike that used to kill it.

## 5. Caveats

- attax at this commit is guideline-scored with a 20-row vision limit, so 19 of
  the 20 wins are forfeits (attax cannot place a piece) rather than top-outs.
  The win rate is a gate metric, not a strength claim; the honest signal is the
  per-game survival and the two previously fatal seeds.
- The sample is 20 games on tuning seeds. The final match uses held-out seeds.
- The proxy has no opponent, so it cannot price the attack/defence trade-off
  exactly. That is why the SPSA result was validated on the real match rather
  than trusted from the proxy.
