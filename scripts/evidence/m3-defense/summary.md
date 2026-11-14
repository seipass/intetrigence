# S2 match summary

- games: 20
- intetrigence wins / losses / draws / forfeits: 20 / 0 / 0 / 20
- win rate: 100.0% (Wilson 95% CI 83.9%..100.0%)
- replay verified: 20/20 (failures: 0)
- win breakdown: TopOut 0, forfeit-vision 20, forfeit-other 0; losses: TopOut 0, forfeit 0; turn-cap games 0
- attax: Attax overhaul/full commit 7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b bun 1.4.0
- attax command: `bun C:/Users/Hasilan/Projects/attax\tbp\attax-tbp.ts`

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | Forfeit | vision | 474 | yes | 97.4 / 36.5 | 99.1 / 46.4 | 1.00 / 1.00 | 1.00 / 0.99 | 60 |
| 9001 | B | intetrigence | Forfeit | vision | 191 | yes | 97.2 / 36.4 | 99.2 / 44.2 | 1.00 / 1.00 | 1.00 / 0.99 | 21 |
| 9002 | A | intetrigence | Forfeit | vision | 298 | yes | 97.3 / 36.1 | 99.2 / 45.9 | 1.00 / 1.00 | 1.00 / 0.98 | 42 |
| 9002 | B | intetrigence | Forfeit | vision | 91 | yes | 96.9 / 34.5 | 98.7 / 42.8 | 1.00 / 1.00 | 1.00 / 0.99 | 14 |
| 9003 | A | intetrigence | Forfeit | vision | 214 | yes | 97.3 / 36.8 | 99.1 / 46.5 | 1.00 / 1.00 | 1.00 / 0.96 | 27 |
| 9003 | B | intetrigence | Forfeit | vision | 134 | yes | 97.2 / 36.4 | 99.0 / 44.0 | 1.00 / 1.00 | 1.00 / 0.96 | 15 |
| 9004 | A | intetrigence | Forfeit | vision | 311 | yes | 97.1 / 36.8 | 99.1 / 44.8 | 1.00 / 1.00 | 1.00 / 0.98 | 39 |
| 9004 | B | intetrigence | Forfeit | vision | 94 | yes | 97.3 / 34.1 | 98.9 / 43.8 | 1.00 / 1.00 | 1.00 / 0.99 | 13 |
| 9005 | A | intetrigence | Forfeit | vision | 194 | yes | 97.4 / 36.3 | 99.2 / 44.3 | 1.00 / 1.00 | 1.00 / 0.99 | 22 |
| 9005 | B | intetrigence | Forfeit | vision | 354 | yes | 97.2 / 35.9 | 99.2 / 44.5 | 1.00 / 1.00 | 1.00 / 0.98 | 41 |
| 9006 | A | intetrigence | Forfeit | vision | 224 | yes | 97.1 / 36.1 | 98.9 / 43.6 | 1.00 / 1.00 | 1.00 / 0.98 | 37 |
| 9006 | B | intetrigence | Forfeit | vision | 99 | yes | 97.2 / 35.7 | 98.9 / 47.0 | 1.00 / 1.00 | 1.00 / 0.99 | 17 |
| 9007 | A | intetrigence | Forfeit | vision | 615 | yes | 97.3 / 37.0 | 99.1 / 44.7 | 1.00 / 1.00 | 1.00 / 0.98 | 80 |
| 9007 | B | intetrigence | Forfeit | vision | 179 | yes | 97.4 / 36.7 | 99.1 / 44.3 | 1.00 / 1.00 | 1.00 / 0.98 | 17 |
| 9008 | A | intetrigence | Forfeit | vision | 26 | yes | 96.7 / 31.1 | 98.3 / 38.4 | 1.00 / 1.00 | 1.00 / 1.00 | 2 |
| 9008 | B | intetrigence | Forfeit | vision | 866 | yes | 97.3 / 36.2 | 99.0 / 44.3 | 1.00 / 1.00 | 1.00 / 0.98 | 123 |
| 9009 | A | intetrigence | Forfeit | vision | 121 | yes | 97.4 / 37.8 | 99.0 / 46.4 | 1.00 / 1.00 | 1.00 / 0.97 | 13 |
| 9009 | B | intetrigence | Forfeit | vision | 437 | yes | 97.2 / 37.1 | 98.9 / 45.7 | 1.00 / 1.00 | 1.00 / 0.99 | 68 |
| 9010 | A | intetrigence | Forfeit | vision | 184 | yes | 97.2 / 34.9 | 98.7 / 44.5 | 1.00 / 1.00 | 1.00 / 0.97 | 23 |
| 9010 | B | intetrigence | Forfeit | vision | 81 | yes | 97.2 / 33.0 | 98.9 / 42.8 | 1.00 / 1.00 | 1.00 / 1.00 | 14 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | think > 200ms |
|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 5177 | 0 / 5177 | 0 / 5177 | 1.002 | 76.2 | 0 |
| 1 | Attax | 20 | 5167 | 85 / 5167 | 0 / 5167 | 0.871 | 108.7 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- M3 defense-stress diagnostic baseline (12 start garbage rows, --ms 100, 8 threads). Not usable for final claims.
