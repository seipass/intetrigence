# S2 match summary

- games: 20
- intetrigence wins / losses / draws / forfeits: 19 / 1 / 0 / 16
- win rate: 95.0% (Wilson 95% CI 76.4%..99.1%)
- replay verified: 20/20 (failures: 0)
- win breakdown: TopOut 0, forfeit-vision 16, forfeit-other 0; losses: TopOut 1, forfeit 0; turn-cap games 3
- attax: Attax overhaul/full commit 7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b bun 1.4.0
- attax command: `bun C:/Users/Hasilan/Projects/attax\tbp\attax-tbp.ts`

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | Forfeit | vision | 198 | yes | 97.2 / 35.0 | 99.2 / 42.5 | 1.00 / 1.00 | 1.00 / 0.97 | 30 |
| 9001 | B | intetrigence | Forfeit | vision | 170 | yes | 97.0 / 34.9 | 99.1 / 41.5 | 1.00 / 1.00 | 1.00 / 0.98 | 20 |
| 9002 | A | intetrigence | Forfeit | vision | 28 | yes | 97.3 / 29.5 | 98.8 / 36.0 | 1.00 / 1.00 | 1.00 / 0.96 | 6 |
| 9002 | B | intetrigence | Forfeit | vision | 28 | yes | 96.5 / 29.4 | 99.0 / 37.9 | 1.00 / 1.00 | 1.00 / 0.96 | 7 |
| 9003 | A | intetrigence | Forfeit | vision | 215 | yes | 97.1 / 36.8 | 99.1 / 45.6 | 1.00 / 1.00 | 1.00 / 0.98 | 32 |
| 9003 | B | intetrigence | Forfeit | vision | 311 | yes | 97.2 / 35.7 | 99.2 / 44.3 | 1.00 / 1.00 | 1.00 / 0.96 | 37 |
| 9004 | A | intetrigence | TurnCap | - | 1000 | yes | 97.1 / 38.1 | 98.9 / 46.8 | 1.00 / 1.00 | 1.00 / 0.99 | 131 |
| 9004 | B | intetrigence | Forfeit | vision | 846 | yes | 97.1 / 35.6 | 98.9 / 44.0 | 1.00 / 1.00 | 1.00 / 0.98 | 118 |
| 9005 | A | intetrigence | Forfeit | vision | 188 | yes | 97.2 / 34.9 | 99.2 / 43.1 | 1.00 / 1.00 | 1.00 / 0.98 | 26 |
| 9005 | B | intetrigence | TurnCap | - | 1000 | yes | 97.1 / 36.3 | 99.1 / 45.0 | 1.00 / 1.00 | 1.00 / 0.99 | 124 |
| 9006 | A | intetrigence | Forfeit | vision | 209 | yes | 97.1 / 36.3 | 99.0 / 44.8 | 1.00 / 1.00 | 1.00 / 0.98 | 27 |
| 9006 | B | intetrigence | Forfeit | vision | 466 | yes | 97.1 / 36.1 | 98.9 / 44.4 | 1.00 / 1.00 | 1.00 / 0.99 | 65 |
| 9007 | A | intetrigence | TurnCap | - | 1000 | yes | 97.2 / 36.2 | 99.1 / 44.0 | 1.00 / 1.00 | 1.00 / 0.99 | 132 |
| 9007 | B | intetrigence | Forfeit | vision | 116 | yes | 97.2 / 35.9 | 98.9 / 41.5 | 1.00 / 1.00 | 1.00 / 0.99 | 8 |
| 9008 | A | intetrigence | Forfeit | vision | 138 | yes | 96.9 / 36.5 | 98.9 / 45.2 | 1.00 / 1.00 | 1.00 / 0.99 | 19 |
| 9008 | B | intetrigence | Forfeit | vision | 978 | yes | 97.1 / 36.1 | 99.0 / 44.9 | 1.00 / 1.00 | 1.00 / 0.99 | 147 |
| 9009 | A | intetrigence | Forfeit | vision | 945 | yes | 97.0 / 33.1 | 99.0 / 42.4 | 1.00 / 1.00 | 1.00 / 0.98 | 142 |
| 9009 | B | attax | TopOut | - | 48 | yes | 96.5 / 28.8 | 98.4 / 37.8 | 1.00 / 1.00 | 1.00 / 0.92 | 3 |
| 9010 | A | intetrigence | Forfeit | vision | 184 | yes | 97.0 / 31.3 | 98.8 / 38.8 | 1.00 / 1.00 | 1.00 / 0.98 | 29 |
| 9010 | B | intetrigence | Forfeit | vision | 505 | yes | 97.4 / 35.4 | 99.2 / 43.3 | 1.00 / 1.00 | 1.00 / 0.98 | 71 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | think > 200ms |
|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 8565 | 0 / 8565 | 0 / 8565 | 0.972 | 124.5 | 0 |
| 1 | Attax | 20 | 8557 | 135 / 8557 | 0 / 8557 | 0.873 | 164.0 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- M3 defense-stress diagnostic baseline (16 start garbage rows, --ms 100, 8 threads). Not usable for final claims.
