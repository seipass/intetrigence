# Surge charge: why the bursts are small, and what was built

Diagnostic report. **None of these numbers are strength claims**; they are
measurements of the bot's own behaviour under the diagnostic profiles and under
real attack pressure.

## 1. The question

Across 372 intetrigence-vs-attax replays the bot released surge 1,602 times
(9,679 lines, 7.9% of all attack). The size distribution is the problem:

| lines | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15+ |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| count | 503 | 352 | 251 | 171 | 103 | 83 | 56 | 30 | 16 | 10 | 11 | 15 |

80% of releases are 4-7 lines. A bot sitting at height 3-5 absorbs that in one
or two turns. What would hurt is a 9-13 line burst, which needs the streak held
to 10-14 before cashing.

## 2. Why holding loses today

The release is an *immediate* reward (`attack.total`, which includes the surge
chunks) and the charge is a *leaf* term priced linearly:

- release at charge 4: `attack 4 -> +48`, plus the clear table (-14) = **+34**,
  charge resets to 0;
- hold instead: no reward this turn, leaf keeps `1.5 * 4 = 6`.

Holding is worth 6 against 34 for cashing, so the bot always cashes. The
arithmetic does not change with a convex exponent at charge 4 (`1.5 * 4^1.5 =
12`), so the exponent alone cannot stop an early release; it only makes a
*already large* charge worth protecting. That is why both terms below exist and
why the hold penalty is the one that bites.

## 3. Instruments: what can and cannot measure this

| instrument | verdict |
|---|---|
| composite proxy (`play_game_with_setup`, 16 start rows) | **unusable**: the bot is in survival mode from turn 0 and never builds a streak. 0 releases in 6 games, 6/6 top-outs, identical results for all four weight settings. |
| trickle proxy (0 start rows, 2 rows every 8 turns) | **unusable**: with no opponent there is no reason to attack, so `attack/piece` is 0.24 against 1.65 in real games and the streak never forms. 0 releases in 6 games. |
| self-play referee (`--self-play --profile garbage-pressure`) | **works**: real attack pressure, both slots our search. 4 games, 744 pieces, `attack/piece` 0.812, 4 releases, all 4-7 lines, **0 at 8+**, surge 4.0% of attack. |
| attax referee | works for the distribution (a forfeit ends the game but does not invalidate the placements already played); it is the source of the 372-game table above. |

The self-play run reproduces the real-game shape, so it is the instrument to
A/B on -- but `crates/arena` has no flag to inject weights, so the A/B cannot
be run from the harness as it stands.

## 4. What was implemented

`crates/bot/src/eval.rs`:

- `Weights.surge_exponent` (default `1.0`, i.e. the old linear behaviour): the
  leaf charge value becomes `surge_charge * charge^exponent`. At 1.5 a charge
  of 10 is worth 47 against 15 linear.
- `Weights.surge_hold_penalty` (default `0.0`, off): a placement that releases
  surge below `SURGE_LETHAL = 8` lines pays this penalty. This is the term that
  makes cashing a 4-7 line burst unattractive.
- `Weights.survival` (default on) from the previous commit: above
  `pending >= 8` or height 15 the shape terms are removed and every cleared
  line pays `survival_line` instead of the calm-board clear table.

Both new defaults reproduce the previous behaviour exactly, so the committed
state is bit-identical to `7aa0c73` unless a weight is changed. Unit tests pin
the arithmetic: `surge_exponent_makes_a_large_charge_worth_more` and
`surge_hold_penalty_only_bites_below_lethal`.

`crates/bot/src/play.rs`: `GameReport.surge_releases` records every burst size,
so any harness that reads a `GameReport` can compute the distribution.

## 5. What is blocked

The A/B the manager asked for (exponent 1.3 / 1.5 / 1.8, then the hold penalty)
needs the weights to reach the referee. `crates/arena` is owned by
harness-dev and has no `--weights` / `--surge-exponent` / `--surge-hold` flag,
and I must not edit it. Requesting one flag pair is the unblocking step;
alternatively the A/B can be run as an in-crate versus loop, which would
duplicate the referee's cancellation semantics and risks divergence.

Until then the honest statement is: the terms are implemented and unit-tested,
their direction is argued from the reward arithmetic and the measured release
distribution, and **no measurement yet shows that they increase the size of the
bursts**.
