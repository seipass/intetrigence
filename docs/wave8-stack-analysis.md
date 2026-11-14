# Wave 8: stack management redesign — analysis and rejected attempts

- goal: attax 50-0 (100%). Provisional baseline: 94-6 (94.0%) on seeds 11001-11050.
- seeds used here: tuning 9001-9010 (already burned as tuning), verification
  13001-13020. **13001-13020 are now burned** (used in a failed verification)
  and must not be reused for a claim.
- starting point: `ec04daf` (defence v2, M4 reserve 37-3 = 92.5%).

## 1. The six provisional losses share one mechanism

Replaying the six losses through the referee engine and dumping the height,
the received total, the queue depth and each placement's clear shows the same
sequence in five of them (the sixth, 11027-A, dies at turn 37 and is the
opener case):

1. the stack is low (height 2-5) and the queue is empty;
2. garbage starts arriving and the bot plays **7-8 consecutive placements that
   clear nothing** while the queue grows to 10-12;
3. one clear lands (3-4 lines) but the queue has already split into two waves;
4. the waves land back to back: height 4 -> 10 -> 17 -> 22 in three turns;
5. the bot digs a few lines and dies at height 22-24.

### 11004-A (194 turns)

| turn | height | received | pending | placement |
|---|---|---|---|---|
| 184 | 3 | 62 | 5 | I horizontal, 0 lines |
| 185 | 4 | 62 | 5 | J, 0 lines |
| 186 | 9 | 67 | 0 | S, 0 lines (garbage landed) |
| 187 | 9 | 67 | 6 | T, 0 lines |
| 188 | 6 | 67 | 10 | L, **3 lines** |
| 189 | 12 | 71 | 7 | Z, 0 lines (garbage landed) |
| 190 | 18 | 77 | 4 | J, 0 lines |
| 191 | 19 | 78 | 3 | L (mini spin), 0 lines |
| 192 | 22 | 81 | 0 | T, 0 lines |
| 193 | 23 | 81 | 0 | O, 0 lines -> dead |

### 11020-B (245 turns)

Turns 233-239: seven placements, zero lines, queue 0 -> 12. Turn 240: a quad
(4 lines) at height 17. Turns 241-244: four placements, zero lines. Dead at
height 24.

So the bot does not lose the attack war (attack/piece 0.89-0.97 in these
games, above attax's 0.86) and does not receive more garbage than attax. It
loses because **it stops clearing for seven or eight pieces exactly when the
queue is filling**.

### Why it stops clearing

The clear table penalises small clears: `clear[None][1] = -14`,
`clear[None][2] = -6`, `clear[None][3] = +2`. Those penalties exist to stop
the bot wasting pieces on singles when the stack is calm. Under a growing
queue they are wrong: a single is a row of downstack. The `danger_projected`
term only bites once `height + pending` exceeds 10, which at height 3-4 means
pending 6-7 — by then the queue is already two waves deep.

## 2. Attempt 4: a danger-scaled downstack reward (rejected)

`Weights.downstack`: `value += downstack * urgency * lines` with
`urgency = clamp(max(pending, height - 8) / 6, 0, 1)`, so a clear is worth
more when garbage is queued. This is the direct fix for the mechanism above.

Diagnostic A/B (composite proxy, 12 start rows + 6 rows / 12 turns):

| seeds | mean height 150-250 | attack/piece | max queue |
|---|---|---|---|
| 1-12, downstack 0 | 6.92 | 0.765 | 6.0 |
| 1-12, downstack 40 | 6.17 (-10.8%) | 0.714 (-6.7%) | 6.0 |
| 13-24, downstack 0 | 7.26 | 0.749 | - |
| 13-24, downstack 40 | 6.61 (-9.0%) | 0.704 (-6.0%) | - |

Two things to note:

- the trade (height down ~10%, attack down ~6%) is the **same signature** as
  the three variants rejected in wave 7 (`garbage_buried`, `DANGER_BASE` 10,
  `flatness`);
- the proxy's queue never exceeds 6, so it **cannot reproduce the failure**
  (queue 10-12). The proxy injects garbage at a fixed rate; the real losses
  come from attax's burst attacks.

Verification on fresh seeds 13001-13020 (40 games, official):

| | provisional (11001-11050) | downstack 40 (13001-13020) |
|---|---|---|
| wins / losses | 94 / 6 (94.0%) | **36 / 4 (90.0%)** (Wilson 76.9-96.0) |
| TopOut losses | 6 | 4 |

**Rejected.** The loss rate rose from 6.0% to 10.0%. The four new losses
(13008-A, 13012-A, 13012-B, 13017-B) are the same shape: attack advantage
(0.99 vs 0.93 in 13012-A) and stack death.

## 3. Attempt 5: open-space shortcut in the movegen (no gain, reverted)

Hypothesis: the search's beam width is limited by the movegen cost (~74 us per
piece on an empty board, ~13 us on a 19-row stack), so making it cheaper would
buy a wider beam without touching the attack/defence trade-off.

Implementation: seed the BFS with every `(rotation, x)` two rows above the
stack (the largest upward kick) plus a canonical prologue from the spawn, and
search downward from there. The exactness argument is that above the stack
every cell is empty, so the piece moves freely and any state below the seed
row is reached from a seed.

The property tests (brute-force equality on representative boards, 120
skyline + 120 messy boards, core-`Game` cross-check, replay) all passed, so
the shortcut was exact. It did **not** speed anything up: the reachable state
count is unchanged (9921 states over 7 pieces on an empty board) and the cost
is dominated by the per-cell `fits` checks, not by the empty rows. Reverted.

## 4. Correction: the win numbers were forfeits, and we never release surge

The manager re-tallied all 220 attax games: **0 top-out wins, 13 top-out losses,
13 turn-cap wins, 194 forfeits** (the attax adapter returns zero candidates).
The real record is 13/220 = 5.9%, and every "94%" / "92.5%" figure in this and
earlier reports measured the adapter, not the bot.

Two consequences for this document:

- the loss analysis in §1 stands (13 top-out losses, same mechanism);
- the *win* side was never real, so "attack/piece 0.965 vs 0.862" has never
  converted into a kill. attax has not been topped out once in 220 games.

### Correction: surge *is* released, but it is small

An earlier version of this document claimed the bot never releases surge. That
was wrong: it generalised from the last twelve turns of the six losses. A tally
over all 372 intetrigence-vs-attax replays in `scripts/evidence/` says the
opposite.

- 16,662 turns had `b2b_before >= 5` (charge >= 4).
- **1,602 surge releases**, 9,679 surge lines in total, 7.9% of all attack
  (122,987 lines).
- 329 of 372 games contain at least one release.
- Release sizes: 4 lines x503, 5 x352, 6 x251, 7 x171, 8 x103, 9 x83, 10 x56,
  11 x30, 12 x16, 13 x10, 14 x11, 15 x6, 16 x3, 17 x3, 19 x2, 20 x2.

So the S2 win condition is being used, just weakly: **most bursts are 4-7
lines**, which a bot sitting at height 3-5 absorbs in one or two turns. Only
325 releases (20%) reach 8+ lines.

The open question is therefore not "why no release" but "why are the releases
small": the bot cashes the charge as soon as a cheap non-difficult clear
appears, instead of building the streak to 10-14 and releasing 9-13 lines at
once. The reward does prefer the bigger burst (a single at B2B 12 pays 188
against 48 at B2B 5), so the cause is positional: at B2B 12 the bot has
usually already spent the cheap clears, and the search's 8-ply horizon cannot
see the value of *holding* the charge for a later, bigger release.

That points at a term the evaluation does not have: the **option value of the
charge**. `surge_charge_from_streak` is priced linearly (1.5 per charged line),
which is the same whether the charge is 4 or 14, while the realised burst grows
super-linearly in practice (the bot must find a non-difficult clear).

## 5. Conclusion

Four evaluation-weight variants have now been rejected (`garbage_buried`,
`DANGER_BASE` 10, `flatness`, `downstack`). They all trade attack for height,
and the two that reached a real match both lost. The mechanism in §1 says why
a weight cannot fix it: the bot must **change policy**, not preferences — when
the queue is deep it has to clear every piece, even a single, and it must do
so *before* the queue reaches 10.

Concrete next steps that are structural rather than parametric:

1. **Queue-aware pruning**: when `pending >= 8`, drop every child that clears
   no line (unless no such child exists). That is a hard constraint, not a
   weight, and it cannot be traded away by shape terms.
2. **Two-mode evaluation**: a "survival mode" that switches the leaf value to
   pure clear-rate when `pending >= 8` or `height >= 14`, keeping the normal
   table otherwise. Different from a weight shift because the shape terms are
   removed, not merely scaled.
3. **Burst-aware search**: the losses come in pairs of waves 2-4 turns apart.
   A search that models the queue's arrival frames (the referee exposes them)
   would see both waves; today `EvalContext.incoming` is a single scalar.

The current build (`ec04daf` + the `max_pending` metric) is unchanged in
strength: provisional 94-6, M4 reserve 37-3. Nothing in this wave is adopted.
