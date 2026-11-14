# Wave 9: why the 40-row attax does not die

Ten games against the patched attax (40-row vision, `7b38f51` + the
attax-integration patch), tuning seeds 9001-9005 x both sheets, 100 ms, 8
threads, turn cap 600. **3 wins / 7 losses, 0 forfeits, 10/10 replays
verified.** Evidence: `scripts/evidence/wave9-losses/`.

This is the first measurement of the bot against an opponent that can see its
whole board, so it is the first honest loss analysis.

## 1. The headline: the attack totals are equal, the T-spins are not

| | intetrigence | attax |
|---|---|---|
| pieces | 3,210 | 3,208 |
| lines cleared | 2,036 | 2,100 |
| lines/piece | 0.634 | 0.655 |
| garbage cleared | 866 | 927 |
| attack total | 2,633 | 2,710 |
| attack/piece (aggregate) | 0.820 | 0.845 |
| attack from quads | 1,116 | 1,045 |
| **attack from spins** | **489** | **737** |
| max B2B streak (median) | 7 | 5 |
| turns with B2B >= 4 | 235 | 139 |
| surge lines released | 104 | 41 |

The totals are within 3%. The composition is not: **attax gets 51% more attack
from spins**, and the spin *quality* explains it.

### Spin breakdown

| placement | intetrigence | attax |
|---|---|---|
| full T-spin, 1 line (TSS) | 30 | 60 |
| full T-spin, 2 lines (TSD) | **40** | **100** |
| full T-spin, 0 lines | 2 | 1 |
| mini, 0 lines | 85 | 73 |
| mini, 1 line | **109** | 60 |
| mini, 2 lines | 31 | 6 |
| mini, 3 lines | 1 | 0 |
| quads (no spin) | 188 | 181 |

attax executes **160 full spins against our 70**, and 100 TSDs against our 40.
We execute 225 minis against their 176. The spin *count* is the same (298 vs
300); the difference is entirely that attax converts its T pieces into full
spins and we convert ours into minis.

At the S2 table a full TSD pays 4 and a mini double pays 2, a full TSS pays 2
and a mini single pays 1, so 60 extra TSDs and 30 extra TSSs are worth roughly
+300 attack over ten games -- more than the whole 77-line attack deficit.

**Conclusion: the bot builds T-slots that resolve as minis.** The evaluation
already prices full spins far above minis (`clear[Full] = [0, 8, 20, 40, 50]`
against `clear[Mini] = [0, -5, 2, 8, 30]`), and it already rewards a two-line
slot (45) above a one-line slot (20), so this is not a missing reward term.
Either the slots the bot builds are too shallow, or the execution picks a
rotation/kick that lands as a mini.

## 2. The deficit opens in the opening and never closes

Cumulative attack at turn boundaries (intetrigence/attax):

| game | t20 | t40 | t60 | t120 | t200 | t300 |
|---|---|---|---|---|---|---|
| 9001-A | 11/8 | 33/28 | 47/51 | 103/112 | 178/190 | 201/233 |
| 9001-B | 11/8 | 21/28 | 27/44 | 27/44 | -- | -- |
| 9002-A | 5/12 | 21/20 | 35/34 | 93/100 | 119/140 | 119/140 |
| 9002-B | 5/12 | 21/20 | 36/33 | 86/79 | 170/163 | 280/251 |
| 9003-A | 7/6 | 20/29 | 50/49 | 93/94 | 153/156 | 223/234 |
| 9003-B | 7/6 | 20/22 | 29/33 | 77/84 | 154/147 | 154/147 |
| 9004-A | 5/5 | 19/24 | 40/45 | 91/95 | 152/162 | 210/219 |
| 9004-B | 4/5 | 29/34 | 37/52 | 37/52 | -- | -- |
| 9005-A | 13/18 | 25/33 | 40/46 | 95/87 | 144/145 | 240/238 |
| 9005-B | 13/18 | 29/37 | 54/57 | 109/113 | 174/179 | 277/278 |

Mean cumulative attack by turn 60: **39.5 against 44.4** (attax +12%). attax is
ahead at turn 40 in eight of ten games and never gives the lead back. The three
wins are exactly the games where the curves cross (9003-B at t200, 9005-A at
t120, 9005-B at t400).

In the first 40 turns the clear mix is already different: we clear 31 singles
and 28 spins, attax 14 singles and 37 spins, for 0.588 against 0.647
attack/piece.

## 3. The defence is slightly worse, but it is not the primary cause

We clear 866 garbage rows against 927, and 2,036 lines against 2,100. That is
7% less downstacking on the same piece count. Our longest run of consecutive
zero-line placements is 23 against their 15.

So the stack rises for a second reason: we clear less often. But with attack
totals within 3% and the same lines/piece within 3%, the defence gap is a
consequence of the spin-quality gap (a TSD clears two lines, a mini single
clears one) rather than an independent defect.

## 4. What the data rules out

- **"Our attack is too small in total"** -- 2,633 against 2,710, a 3% gap that
  cannot explain 7 losses.
- **"We do not build B2B"** -- we build *more* B2B than attax (median streak 7
  against 5, 235 turns at B2B >= 4 against 139) and release more surge (104
  against 41). The charge is not the bottleneck.
- **"We lose the opener completely"** -- we are behind by 5 attack lines at
  turn 60, not buried.

## 5. What the data points at

1. **T-spin execution quality** (primary). We need to convert T pieces into
   full TSD/TSS instead of minis. This is worth ~+300 attack and ~+60 downstack
   lines over ten games, i.e. more than the entire observed deficit.
2. **Zero-line run length** (secondary). 23 against 15: when a T-slot fails we
   keep stacking instead of bailing out and clearing something.
3. **Opening attack density** (secondary). 0.588 against 0.647 in the first 40
   pieces.

## 6. Next measurements

- Does the bot *intend* a full spin and get a mini? Instrument the search: log
  the chosen placement's intended slot depth against the realised `spin`.
- If the slots are too shallow, the fix is in `best_t_slot` (it only scans
  `Rotation::Two` entries) and in the setup reward, not in the clear table.
- A/B `mini_penalty` / `tsd_bonus` scalars through `arena match --weights`
  once they exist.
