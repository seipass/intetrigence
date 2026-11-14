# S2 match summary

- games: 20
- intetrigence wins / losses / draws / forfeits: 20 / 0 / 0 / 19
- win rate: 100.0% (Wilson 95% CI 83.9%..100.0%)
- replay verified: 20/20 (failures: 0)
- win breakdown: TopOut 0, forfeit-vision 19, forfeit-other 0; losses: TopOut 0, forfeit 0; turn-cap games 1
- attax: Attax overhaul/full commit 7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b bun 1.4.0
- attax command: `bun C:/Users/Hasilan/Projects/attax\tbp\attax-tbp.ts`

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | Forfeit | vision | 209 | yes | 96.8 / 29.4 | 98.6 / 36.6 | 1.00 / 1.00 | 1.00 / 0.99 | 25 |
| 9001 | B | intetrigence | Forfeit | vision | 404 | yes | 97.2 / 32.7 | 99.0 / 45.2 | 1.00 / 1.00 | 1.00 / 0.98 | 52 |
| 9002 | A | intetrigence | Forfeit | vision | 78 | yes | 97.0 / 36.1 | 99.1 / 44.6 | 1.00 / 1.00 | 1.00 / 1.00 | 10 |
| 9002 | B | intetrigence | Forfeit | vision | 158 | yes | 97.1 / 36.1 | 98.7 / 43.6 | 1.00 / 1.00 | 1.00 / 0.99 | 21 |
| 9003 | A | intetrigence | Forfeit | vision | 614 | yes | 97.2 / 36.2 | 99.0 / 44.9 | 1.00 / 1.00 | 1.00 / 0.99 | 72 |
| 9003 | B | intetrigence | Forfeit | vision | 493 | yes | 97.2 / 36.3 | 99.0 / 44.0 | 1.00 / 1.00 | 1.00 / 0.99 | 59 |
| 9004 | A | intetrigence | Forfeit | vision | 156 | yes | 97.1 / 35.1 | 98.9 / 42.5 | 1.00 / 1.00 | 1.00 / 0.99 | 19 |
| 9004 | B | intetrigence | Forfeit | vision | 51 | yes | 96.9 / 33.3 | 98.4 / 41.6 | 1.00 / 1.00 | 1.00 / 0.98 | 7 |
| 9005 | A | intetrigence | Forfeit | vision | 98 | yes | 97.1 / 35.1 | 99.0 / 44.5 | 1.00 / 1.00 | 1.00 / 0.98 | 14 |
| 9005 | B | intetrigence | Forfeit | vision | 619 | yes | 97.2 / 35.8 | 99.0 / 44.0 | 1.00 / 1.00 | 1.00 / 0.98 | 80 |
| 9006 | A | intetrigence | Forfeit | vision | 859 | yes | 97.1 / 35.8 | 98.9 / 45.1 | 1.00 / 1.00 | 1.00 / 0.98 | 117 |
| 9006 | B | intetrigence | Forfeit | vision | 311 | yes | 97.2 / 36.8 | 99.0 / 45.4 | 1.00 / 1.00 | 1.00 / 0.98 | 47 |
| 9007 | A | intetrigence | Forfeit | vision | 999 | yes | 97.0 / 36.4 | 99.1 / 44.1 | 1.00 / 1.00 | 1.00 / 0.99 | 143 |
| 9007 | B | intetrigence | Forfeit | vision | 207 | yes | 97.4 / 35.8 | 99.2 / 43.7 | 1.00 / 1.00 | 1.00 / 0.99 | 31 |
| 9008 | A | intetrigence | Forfeit | vision | 162 | yes | 97.1 / 37.9 | 99.4 / 46.3 | 1.00 / 1.00 | 1.00 / 0.98 | 20 |
| 9008 | B | intetrigence | Forfeit | vision | 470 | yes | 97.1 / 39.6 | 99.1 / 48.1 | 1.00 / 1.00 | 1.00 / 0.99 | 63 |
| 9009 | A | intetrigence | Forfeit | vision | 279 | yes | 97.1 / 36.1 | 99.0 / 45.0 | 1.00 / 1.00 | 1.00 / 1.00 | 34 |
| 9009 | B | intetrigence | Forfeit | vision | 594 | yes | 97.2 / 37.3 | 99.0 / 45.9 | 1.00 / 1.00 | 1.00 / 0.99 | 88 |
| 9010 | A | intetrigence | Forfeit | vision | 140 | yes | 97.3 / 38.4 | 99.1 / 47.4 | 1.00 / 1.00 | 1.00 / 1.00 | 19 |
| 9010 | B | intetrigence | TurnCap | - | 1000 | yes | 97.1 / 35.5 | 99.0 / 43.8 | 1.00 / 1.00 | 1.00 / 0.99 | 130 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | think > 200ms |
|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 7892 | 0 / 7892 | 0 / 7892 | 0.962 | 109.3 | 0 |
| 1 | Attax | 20 | 7882 | 95 / 7882 | 0 / 7882 | 0.878 | 139.7 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- M3 tuning gate re-run with the current binary (official profile; forfeit causes recorded).
