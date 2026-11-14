# S2 match summary

- games: 10
- intetrigence wins / losses / draws / forfeits: 9 / 1 / 0 / 6
- win rate: 90.0% (Wilson 95% CI 59.6%..98.2%)
- replay verified: 10/10 (failures: 0)
- win breakdown: TopOut 1, forfeit-vision 0, forfeit-other 6; losses: TopOut 1, forfeit 0; turn-cap games 2
- attax: Attax overhaul/full commit 7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b bun 1.4.0
- attax command: `bun C:/Users/Hasilan/Projects/attax\tbp\attax-tbp.ts`

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | TopOut | - | 152 | yes | 98.3 / 50.7 | 101.4 / 80.6 | 1.00 / 1.00 | 1.00 / 0.99 | 18 |
| 9002 | A | intetrigence | Forfeit | other | 615 | yes | 97.5 / 45.5 | 100.4 / 72.5 | 1.00 / 1.00 | 1.00 / 0.98 | 82 |
| 9003 | A | intetrigence | TurnCap | - | 1000 | yes | 97.2 / 44.5 | 100.1 / 77.1 | 1.00 / 1.00 | 1.00 / 0.99 | 114 |
| 9004 | A | attax | TopOut | - | 150 | yes | 97.3 / 44.7 | 101.7 / 92.6 | 1.00 / 1.00 | 1.00 / 0.99 | 18 |
| 9005 | A | intetrigence | TurnCap | - | 1000 | yes | 97.5 / 45.9 | 99.5 / 61.3 | 1.00 / 1.00 | 1.00 / 0.98 | 141 |
| 9006 | A | intetrigence | Forfeit | unknown | 0 | yes | 0.0 / 0.0 | 0.0 / 0.0 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9007 | A | intetrigence | Forfeit | unknown | 0 | yes | 0.0 / 0.0 | 0.0 / 0.0 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9008 | A | intetrigence | Forfeit | unknown | 0 | yes | 0.0 / 0.0 | 0.0 / 0.0 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9009 | A | intetrigence | Forfeit | unknown | 0 | yes | 0.0 / 0.0 | 0.0 / 0.0 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |
| 9010 | A | intetrigence | Forfeit | unknown | 0 | yes | 0.0 / 0.0 | 0.0 / 0.0 | 0.00 / 0.00 | 0.00 / 0.00 | 0 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 2917 | 0 / 2917 | 0 / 2917 | 0.908 | 81.9 | 2.8 / 8.7 | 1 |
| 1 | Attax | 20 | 2915 | 40 / 2915 | 0 / 2915 | 0.872 | 92.5 | 2.8 / 9.2 | 12 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
