# S2 attax 2 腕(holes=-20 / holes=-30)

条件: tuning seeds 9001–9010、sheet A、1000 turn、`--deterministic --depth 8 --width 88`、
`--tbp-supports-180 true`、attax pin `1f41806` + `adapters/attax-s2-tbp.patch` + `adapters/attax-s2-180.patch`。

## 結果

| 腕 | 勝 | 負 | TurnCap | Forfeit | 未決着 | 合計 | 検証 |
|---|---|---|---|---|---|---|---|
| **holes=-20** | 1 (9003) | 8 | 0 | 1 (9002) | 0 | 10 | 10/10 |
| **holes=-30** | 1 (9001) | 7 | 0 | 2 (9003, 9007) | 0 | 10 | 10/10 |

**holes=-30 の方が S2 attax に良い**(1-7 対 1-8、相手の攻撃指標も低い)。

## 指標

| 指標 | holes=-20 我々 | holes=-20 attax | holes=-30 我々 | holes=-30 attax |
|---|---|---|---|---|
| attack/piece | 0.771 | 0.935 | 0.734 | 0.891 |
| attack/line | 1.086 | 1.545 | 1.067 | 1.523 |
| spin rate | 0.095 | 0.150 | 0.089 | 0.144 |
| net damage / game | +31.4 | +144.6 | +26.2 | +115.2 |
| received / game | 133.4 | 80.9 | 103.4 | 61.8 |
| mean / max height | 5.96 / 19.5 | 5.30 / 16.4 | 6.33 / 19.6 | 5.34 / 17.2 |
| collapse(最終高さ ≥ 20) | 1 | 0 | 1 | 0 |
| collapse 負け / race 負け | 7 / 1 | — | 6 / 1 | — |
| think median(ms) | 62.8 | 43.2 | 63.5 | 45.3 |
| think over 100ms | 0 | 0 | 0 | 0 |
| load_note | null | | null | |

## Forfeit の原因(確定): S2 I の 90° kick の **試行順** の不一致

- レフェリー(`crates/core/src/kick.rs`)の I 0→R は **SRS+ ミラー順**
  `[(0,0),(1,0),(-2,0),(-2,-1),(1,2)]`。
- attax(`src/core/srs.ts`)の I 0>1 は **標準 SRS 順**
  `[(0,0),(-2,0),(1,0),(-2,-1),(1,2)]`。
- SRS は「最初に fit した kick で確定」するので、**順序が違うと到達集合が変わる**。
- 9003 turn 268 の実例: 盤面は列 3 の rows 0-3 が空、row 4 が塞がれた「蓋付きの縦穴」。
  attax は `rot0 (3,35) --kick#3 (-2,-1)--> rot1 (1,36)` で縦 I を穴に入れる。
  レフェリーは同じ遷移を kick#3 `(0,-2)` として試すため衝突し、この配置を持たない
  → `none of the 6 candidates is reachable` → forfeit。
- 実測: 同じ盤面で attax の movegen は `rotate180` 有効時に 19 配置(問題の配置を含む)、
  無効時に 18 配置(含まない)。レフェリーの movegen は 18 配置のみ。
- **盤面・hold の desync は 10 ゲーム 2,743 suggest で 0**。接続のバグではない。

### 修正候補

1. **U1 の確定**(s2-spec-lead): TETR.IO S2 の I 90° kick の試行順を一次ソースで確定し、
   レフェリーと attax の両方を同じ順序にする。
2. レフェリーを **順序非依存**にする(全 kick を探索して到達判定)。曖昧さを吸収でき forfeit は消えるが、
   実際には到達できない配置も許す可能性がある(相手に有利 = 強くする方向)。
3. attax 側の I kick 順をレフェリーに合わせるパッチ(強弱が混ざるため要判断)。

## エビデンス

- `scripts/evidence/attax-holes20-10/`、`scripts/evidence/attax-h30-10/`
  (`summary.json` / `audit/` / `replays/` / `logs/`)
- 決定ログ: `C:/tmp/attax-h30-decisions.jsonl`(10 ゲーム 2,743 決定)
- 盤面ログ: `C:/tmp/attax-h30-{ref,adapter}-board.jsonl`(desync 0)
