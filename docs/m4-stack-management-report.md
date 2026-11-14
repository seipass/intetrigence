# M4 stack-management iteration — analysis and negative result

- evidence: `scripts/evidence/m4-reserve-v2` (harness-dev, 40 games), this
  iteration's fresh-seed run was rejected and is not kept
- seeds used: 10031-10040 (fresh for this iteration); 20001-20050 untouched
- starting point: defence v2 (cancellation propagation), M4 37-3 = 92.5%

## 1. The five losses have one shape

Every non-forfeit loss in M4 v2 and in this iteration looks the same: the bot
**out-attacks and receives less garbage, yet dies at height 22-24 while attax
sits at 3-5**.

| seed | turns | our attack | their attack | our received | their received | our final height | theirs |
|---|---|---|---|---|---|---|---|
| 10003-A | 267 | 230 | 240 | 90 | 81 | 24 | 4 |
| 10005-B | 551 | 510 | 500 | 174 | 185 | 22 | 3 |
| 10018-A | 821 | 802 | 762 | 245 | 287 | 23 | 5 |
| 10032-B | 514 | 506 | 476 | 150 | 180 | - | - |
| 10038-B | 741 | 707 | 681 | 234 | 265 | - | - |

The two bottom rows are from this iteration's fresh seeds (the run was
rejected, so only the summary numbers are recorded here).

So the failure is **not** attack volume and **not** garbage handling: it is
that our stack grows over a long game while attax's stays flat. Attack/piece
is 0.95-0.98 in those games, i.e. we are efficient at *converting* pieces into
attack; the cost is the stack we build to do it.

10005-B shows the mechanism at the turn level: at turn 377 the bot is at
height 4 with 15 rows pending, the garbage lands and the stack goes 4 -> 20 in
two turns, then it digs 6 lines in three turns and still dies. It can dig, but
not from a stack that was already 4-6 rows higher than attax's.

## 2. Hypothesis tested: the danger term starts too late

`DANGER_BASE` was 12, so the quadratic height penalty only bites above row 12.
If the stack is allowed to drift to 8-10 before the penalty matters, a later
spike has no headroom.

Change: `DANGER_BASE` 12 -> 10.

### Diagnostic A/B (composite proxy: 12 start rows, 6 rows every 12 turns)

| seeds | mean height 150-250 | attack/piece |
|---|---|---|
| 1-12 | 6.92 -> **6.20** (-10.4%) | 0.765 -> 0.730 (-4.6%) |
| 13-24 | 7.26 -> **6.71** (-7.6%) | 0.749 -> 0.727 (-2.9%) |

Height improves, attack drops. That is the same trade the rejected
`garbage_buried` term made, so the fresh-seed run was the decider.

### Fresh seeds (10031-10040 x both sheets, official, 20 games)

| | v2 (10021-10030) | DANGER_BASE 10 (10031-10040) |
|---|---|---|
| wins / losses | 20-0 | **18-2** (90.0%, Wilson 69.9-97.2) |
| TopOut losses | 0 | **2** |
| non-forfeit | 1/1 | **2/4 (50%)** |
| mean turns | 306 | 448 |

**Rejected.** The two losses (10032-B at 514 turns, 10038-B at 741) are the
same shape as above: attack advantage (506 vs 476, 707 vs 681), received
advantage (150 vs 180, 234 vs 265), stack death.

## 3. Conclusion

Lowering the danger threshold trades attack for height and does not fix the
losses. The remaining gap is **stacking style**: attax builds attack from a
flat stack (final height 3-5) while we build it by stacking up, so over 500-800
turn games our stack is the thing that kills us. Closing that needs an
evaluation that prices "keep the stack flat while attacking" directly — for
example a term on the *variance* of column heights or on the height of the
tallest column relative to the median — not a shift of the existing danger
threshold.

That is a larger change than this iteration, and the current state is already
above the M5 target (M4 reserve 37-3 = 92.5% with defence v2, Wilson
80.1-97.4). Recommendation: **run M5 with the current weights** and keep this
analysis as the starting point for a post-M5 redesign if the final match
misses 90%.
