# M3 garbage-pressure self-play rate sweep (diagnostic)

**Profile**: `garbage-pressure:I:R` (protocol v1.11 §1.3). Both slots run the
in-process search (self-play), so both players are full-vision and both receive
the same periodic injection. **Diagnostic only** — these numbers are not usable
for final claims.

- self-play: `--ms 100`, 4 threads per bot (8 total), tuning seeds only
  (held-out 20001+ not used)
- turn cap **300** (survival is a lower bound for turn-cap games)
- every replay verified: 16:4 10/10, 8:4 20/20, 4:4 10/10

| injection | rows/turn | games | mean turns | turn-cap | TopOut | slot 0 record | slot 0 max / mean height | slot 0 lines/game |
|---|---|---|---|---|---|---|---|---|
| `garbage-pressure:16:4` | 0.25 | 10 | **280.0** | 9 | 1 | 5–5 | 18.6 / 6.5 | 199.5 |
| `garbage-pressure:8:4` | 0.50 | 20 | **220.4** | 9 | 11 | 13–7 | 19.1 / 7.0 | 165.2 |
| `garbage-pressure:4:4` | 1.00 | 10 | **165.5** | 3 | 7 | 4–6 | 21.1 / 8.7 | 139.7 |

- evidence: `scripts/evidence/m3-garbage-pressure-selfplay-16x4/`,
  `.../m3-garbage-pressure-selfplay/`, `.../m3-garbage-pressure-selfplay-4x4/`
- per-rate reports: `docs/m3-garbage-pressure-selfplay-16x4.md`,
  `docs/m3-garbage-pressure-selfplay.md`,
  `docs/m3-garbage-pressure-selfplay-4x4.md`

## Interpretation

- **0.25 rows/turn is sustainable** in self-play: 9/10 games reach the 300-turn
  cap, mean height stays at 6.5, and the bots clear ~200 lines/game.
- **0.5 rows/turn is the boundary**: only 9/20 games reach the cap; the rest top
  out at a mean of 220 turns, and the final height rises to 8.7.
- **1.0 rows/turn is over the limit**: 7/10 top out, mean survival 165 turns,
  final height 14.7.
- The total incoming is the injection **plus the opponent's attack** (~1.0
  rows/turn each in self-play), so the injection-only limit is lower than the
  total. The sweep measures the combined pressure, which is what the final
  match will look like.

## Caveats

- **The mirror win rate is not 50/50** (13–7 at 8:4). The two slots differ in
  their garbage-hole RNG seeds (slot identity) and in search timing; the seat
  swap cancels the seat order but not the hole RNG. The win rate is **not** a
  metric here — survival, height and cleared lines are.
- The turn cap of 300 truncates the survival distribution; a 1000-turn cap
  would be needed to call 0.25 rows/turn "indefinitely sustainable".
- Diagnostic profile; do not mix with `official` numbers.
