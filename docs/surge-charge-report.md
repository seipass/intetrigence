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

## 5. A/B result (harness-dev's `--weights`, commit 12196ec)

Self-play, `--profile garbage-pressure` (interval 8, 4 rows), 100 ms, 4 threads
each, tuning seeds 9001-9004 x both sheets. Run 1 is 2 seeds x 2 sheets, run 2
is 4 seeds x 2 sheets; the seeds overlap between runs, so treat the two runs as
one sample per setting rather than independent measurements.

| setting | games | turns | attack/piece | releases | surge lines | >=8 | >=8 % | >=12 | surge % of attack |
|---|---|---|---|---|---|---|---|---|---|
| default (1.0 / 0) | 8 | 612 | 0.84 | 8 | 44 | **0** | 0.0 | 0 | 4.3 |
| 1.5 / 0 | 4 | 504 | 0.90 | 6 | 34 | 0 | 0.0 | 0 | 3.8 |
| 1.5 / -60 | 8 | 844 | 0.87 | 7 | 41 | 1 | 14.3 | 0 | 2.8 |
| 0 / -60 | 4 | 399 | 0.93 | 3 | 14 | 0 | 0.0 | 0 | 1.9 |
| **1.8 / -100** | 8 | 720 | 0.91 | 6 | 48 | **4** | **66.7** | 0 | 3.7 |

Histograms: default `{4:2, 5:1, 6:4, 7:1}`; 1.5/-60 `{4:2, 5:1, 6:2, 7:1, 9:1}`;
1.8/-100 `{6:1, 7:1, 8:2, 9:1, 10:1}`.

What this says:

- the response is **monotone in the strength of the two terms**: 0/8, then 1/7,
  then 4/6 releases at or above 8 lines;
- the exponent alone does nothing (1.5/0 is 0/6), exactly as the arithmetic in
  section 2 predicted -- the hold penalty is the term that moves the decision;
- the hold penalty alone *reduces* the number of releases (3 in 399 turns)
  without making them bigger, so the two terms are complementary;
- **no release ever reaches 12 lines**, so the "9-13 line burst" target is only
  half met even at the extreme setting.

Caveats that stop this from being a strength claim:

- 6-8 releases per setting: the sample is tiny and the seeds overlap;
- self-play is a mirror, so the win rate is ~50% by construction and says
  nothing about strength;
- the extreme setting also lengthened games (720 turns against 612), which
  confounds "more bursts" with "survived longer to have them".

The committed defaults stay at 1.0 / 0.0, i.e. bit-identical to `7aa0c73`.
Adopting 1.8 / -100 needs a real-match measurement, which the patched attax
(40 rows, forfeit 0) now makes possible.

## 6. What is blocked

Nothing is blocked any more: harness-dev added `arena match --weights
key=value` (12196ec) and the A/B above ran on it.

The open question is whether a bigger burst actually converts into top-outs.
The only instrument for that is the real match against the patched attax
(40-row vision, forfeit 0), and the honest statement today is: the terms are
implemented, unit-tested and shown to raise the burst size at extreme settings,
but **no measurement yet shows that they win games**.
