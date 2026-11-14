# S2 match summary

- games: 20
- intetrigence wins / losses / draws / forfeits: 18 / 2 / 0 / 18
- win rate: 90.0% (Wilson 95% CI 69.9%..97.2%)
- replay verified: 20/20 (failures: 0)
- win breakdown: TopOut 0, forfeit-vision 18, forfeit-other 0; losses: TopOut 2, forfeit 0; turn-cap games 0
- attax: Attax overhaul/full commit 7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b bun 1.4.0
- attax command: `bun C:/Users/Hasilan/Projects/attax\tbp\attax-tbp.ts`

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | Forfeit | vision | 81 | yes | 97.2 / 32.0 | 99.1 / 41.3 | 1.00 / 1.00 | 1.00 / 0.99 | 9 |
| 9001 | B | intetrigence | Forfeit | vision | 437 | yes | 97.1 / 30.9 | 98.6 / 38.3 | 1.00 / 1.00 | 1.00 / 0.99 | 59 |
| 9002 | A | intetrigence | Forfeit | vision | 28 | yes | 97.2 / 27.0 | 98.4 / 34.1 | 1.00 / 1.00 | 1.00 / 0.96 | 6 |
| 9002 | B | intetrigence | Forfeit | vision | 28 | yes | 96.9 / 27.4 | 98.3 / 35.8 | 1.00 / 1.00 | 1.00 / 0.96 | 7 |
| 9003 | A | intetrigence | Forfeit | vision | 134 | yes | 96.7 / 29.6 | 98.6 / 36.4 | 1.00 / 1.00 | 1.00 / 0.96 | 18 |
| 9003 | B | intetrigence | Forfeit | vision | 674 | yes | 96.9 / 30.4 | 98.4 / 38.9 | 1.00 / 1.00 | 1.00 / 0.99 | 91 |
| 9004 | A | intetrigence | Forfeit | vision | 808 | yes | 97.0 / 31.5 | 98.4 / 39.0 | 1.00 / 1.00 | 1.00 / 0.99 | 104 |
| 9004 | B | intetrigence | Forfeit | vision | 68 | yes | 96.9 / 29.7 | 98.5 / 37.2 | 1.00 / 1.00 | 1.00 / 1.00 | 7 |
| 9005 | A | intetrigence | Forfeit | vision | 72 | yes | 96.8 / 30.3 | 98.4 / 38.7 | 1.00 / 1.00 | 1.00 / 0.99 | 7 |
| 9005 | B | intetrigence | Forfeit | vision | 770 | yes | 96.8 / 29.3 | 98.3 / 36.4 | 1.00 / 1.00 | 1.00 / 0.98 | 100 |
| 9006 | A | intetrigence | Forfeit | vision | 347 | yes | 96.9 / 30.2 | 98.4 / 38.6 | 1.00 / 1.00 | 1.00 / 0.99 | 46 |
| 9006 | B | attax | TopOut | - | 892 | yes | 96.9 / 29.9 | 98.3 / 37.3 | 1.00 / 1.00 | 1.00 / 0.99 | 120 |
| 9007 | A | intetrigence | Forfeit | vision | 100 | yes | 96.8 / 30.2 | 98.4 / 37.0 | 1.00 / 1.00 | 1.00 / 1.00 | 7 |
| 9007 | B | intetrigence | Forfeit | vision | 230 | yes | 96.7 / 30.1 | 98.2 / 36.8 | 1.00 / 1.00 | 1.00 / 0.97 | 27 |
| 9008 | A | intetrigence | Forfeit | vision | 895 | yes | 96.8 / 29.8 | 98.4 / 37.1 | 1.00 / 1.00 | 1.00 / 0.98 | 126 |
| 9008 | B | intetrigence | Forfeit | vision | 84 | yes | 96.7 / 29.0 | 98.3 / 35.4 | 1.00 / 1.00 | 1.00 / 1.00 | 8 |
| 9009 | A | intetrigence | Forfeit | vision | 327 | yes | 96.9 / 29.0 | 98.5 / 36.9 | 1.00 / 1.00 | 1.00 / 0.98 | 47 |
| 9009 | B | attax | TopOut | - | 569 | yes | 96.8 / 29.2 | 98.2 / 36.1 | 1.00 / 1.00 | 1.00 / 0.99 | 68 |
| 9010 | A | intetrigence | Forfeit | vision | 813 | yes | 96.9 / 28.8 | 98.3 / 36.3 | 1.00 / 1.00 | 1.00 / 0.98 | 112 |
| 9010 | B | intetrigence | Forfeit | vision | 662 | yes | 96.9 / 29.9 | 98.3 / 36.3 | 1.00 / 1.00 | 1.00 / 0.98 | 87 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | think > 200ms |
|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 8011 | 0 / 8011 | 0 / 8011 | 1.003 | 118.2 | 0 |
| 1 | Attax | 20 | 8001 | 117 / 8001 | 0 / 8001 | 0.895 | 160.8 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- M3 defense-stress diagnostic baseline (16 start garbage rows, --ms 100, 8 threads). Not usable for final claims.
