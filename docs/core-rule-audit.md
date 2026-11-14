# crates/core S2 ルール監査

- 監査対象: `crates/core`（main `43765c8` + 本監査の F1 修正）
- 一次ソース: `docs/s2-spec.md` v1.3.1、`docs/s2-referee-protocol.md`、`docs/s2-attack-examples.json`、`docs/s2-rng-derivation.json`
- 目的: S2 に存在しない規則・TETR.IO と異なる規則・我々が発明した取り決めを洗い出し、勝者決定/終了ロジックの穴を列挙する。

## 0. 分類

| 分類 | 意味 |
|---|---|
| **[S2 準拠]** | 一次ソースで確認できる。TETR.IO S2 / 仕様の決定に一致 |
| **[我々の取り決め]** | S2 のルールではないが、TBP 互換・計測・再現性のために決めたもの。レポートで開示する |
| **[誤り / S2 に存在しない]** | 修正が必要。本監査で修正したものは「修正済み」と記す |

ルールではない実装詳細（テスト用コンストラクタ、デバッグ不変条件、API ガード）は §10 にまとめ、上の 3 分類には含めない。

## 1. サマリ

| 領域 | 判定 | 備考 |
|---|---|---|
| 盤面 10×40 / 可視 20 | [S2 準拠] | §2.1 |
| spawn 最下段 row 21 | [S2 準拠] | §2.2.1 |
| block-out | [S2 準拠] | §2.3 |
| lock-out 無効 | [S2 準拠] | §2.3 |
| Clutch Clear | [S2 準拠] | §2.3。ライン消去後のみ |
| `Game::is_over` | [S2 準拠] + 実装詳細 | block-out のみ。`active.is_none()` は内部不変条件 |
| `has_legal_move` | [我々の取り決め]（referee 側） | core には無い。protocol §7 の 2 段判定 |
| `insert_garbage` 後の active 再スポーン | [我々の取り決め] | TBP `start` 互換。S2 のルールではない |
| 採点 base / combo / B2B / surge / PC / GSB | [S2 準拠] | §6。F1 のみ修正 |
| 0 ライン設置の combo ln 項 | [誤り / S2 に存在しない] | **F1。修正済み** |
| All-Mini+ の mini 扱い | [S2 準拠] | §4.2 / §6.1 |
| T-spin 3-corner / 2-corner / 5th kick | [S2 準拠] | §4.3 |
| 非 T immobile | [S2 準拠] | §4.2 |
| max spin label | [我々の取り決め]（referee 側） | protocol §4.2.1。core は 1 配置の分類のみ |
| ゴミ cap 8 / 遅延 20 / 相殺 / change-on-attack | [S2 準拠] | §7 |
| ゴミ穴の縦順 | [S2 準拠] | §7.5。one-by-one と一致 |
| Opener 14 / 相殺×2 / `>=` | [S2 準拠] | §7.4 / §11-U5 |
| SRS+ / 180 / FIN-TST | [S2 準拠] | §3.1 / §3.2 / §3.3 |
| 7-bag MINSTD / Fisher–Yates | [S2 準拠] | §2.4 |
| `derive_seeds` | [我々の取り決め] | §11-U7。TETR.IO の導出式は非公開 |
| bag の初期配列順 | [我々の取り決め] | 仕様に明記なし。既知列テストで固定 |
| `BagState` / `Game::bag_state` | [我々の取り決め] | TBP `start.randomizer` 互換 |
| 勝者決定 | core には無い | referee の責務。TurnCap の攻撃量比較は referee の決定 |
| ターン上限 | core には無い | referee の責務 |

## 2. 盤面・spawn・終了条件

### 2.1 盤面

- 10 列 × 40 行、可視 20 行、row 0 = 最下段。`[S2 準拠]`（§2.1）
- ゴミセルは occupied の部分集合。`[S2 準拠]`（§7.1）
- ライン消去は全 full row を除去し、上を詰める。`[S2 準拠]`（§6 / §7.5）
- ゴミ挿入は 40 行を超えた分を破棄。`[S2 準拠]`（§11-U6 の決定。teto の 41 行 quirk は本プロファイルでは到達不能: spawn 最上段 22 + 最大 8 行 = 30 < 37）
- ゴミ挿入の穴の縦順は one-by-one 挿入と同じ（`holes[0]` が最上段）。`[S2 準拠]`（§7.5）

### 2.2 spawn

- I=(3,19)、O=(4,21)、JLSTZ=(3,20)。最下段セルは row 21。`[S2 準拠]`（§2.2.1）
- spawn 位置は標準スポーン。TBP `start` の前提と一致。`[S2 準拠]`

### 2.3 block-out / lock-out / Clutch

- **block-out**: spawn 位置が既存ブロックまたは盤外 → 死亡。`[S2 準拠]`（§2.3）
- **lock-out**: 無効。可視領域より上でロックしても死なない。`[S2 準拠]`（§2.3）
- **Clutch Clear**: 直前の設置でライン消去があり、spawn が衝突する場合、1 行ずつ上へ押し上げて `y < 40` の合法位置を探す。見つからなければ block-out。`[S2 準拠]`（§2.3）
  - 実装: `Game::lock` が `clear.lines > 0` のときだけ `spawn(next, true)` を呼ぶ。`spawn` は標準位置を先に確認し、衝突時のみ `y+1..40` を試す。
  - ライン消去なしのロックでは Clutch を適用しない。`[S2 準拠]`（§2.3 の条件どおり）

### 2.4 `Game::is_over`

```rust
pub fn is_over(&self) -> bool {
    self.topped_out || self.active.is_none()
}
```

- `topped_out` は block-out で立つ。`[S2 準拠]`
- `active.is_none()` は内部不変条件（`topped_out` と同値になるよう `check_invariants` が検査）。S2 のルールではなく実装詳細。
- core には **lock-out による終了は無い**。`[S2 準拠]`

### 2.5 `has_legal_move`

- core には `has_legal_move` は無い。referee の `crates/arena::Engine::has_legal_move` が movegen で判定する。
- protocol §7 の「空の候補」2 段判定（referee に合法手がある → forfeit、無い → top-out）は **referee の責務**。`[我々の取り決め]`
- core の `is_over` は block-out のみで、「spawn は合法だが置ける場所が無い」を検出しない。これは protocol §7 の 2 段判定で扱う。`[S2 準拠 + referee-level]`

## 3. 採点（`scoring.rs`）

### 3.1 base テーブル

`BASE_ATTACK` は §6.1 の 12 ケース（none/mini/full × 0..4 行）と一致。`[S2 準拠]`

- mini triple = 2、mini quad = 10、full quad = 10 を含む。
- 0 行は base 0。`[S2 準拠]`

### 3.2 combo 乗数

- `combo >= 1`: `base *= 1 + 0.25*combo`。`[S2 準拠]`（§6.3）
- `combo >= 2`: `base = max(base, ln_1p(1.25*combo))`。`[S2 準拠]`（§6.3）
- **F1（修正済み）**: 0 ライン設置では combo 乗数を適用しない。§6.8 の擬似コードは `lines > 0` ガードを省略しており、stale combo > 0 の 0 ライン設置に ln 項が攻撃を作る。S2 では 0 ライン設置は攻撃 0。`[誤り / S2 に存在しない]`
  - 修正: `if input.lines > 0 { ... }` で乗算と ln 項を囲む。
  - テスト: `no_clear_resets_combo_but_keeps_b2b` を combo=5 / lines=0 に変更し、events 空・base 0 を固定。
  - fixture 18/18 は全ケース lines > 0 のため影響なし。

### 3.3 B2B Charging

- `lines > 0 && N_after >= 2` で +1。`[S2 準拠]`（§6.2）
- 1 回目の difficult clear は `N_after = 1` なので加算なし。`[S2 準拠]`
- B2B を切る消去は `N_after = 0` なので加算なし。`[S2 準拠]`
- PC は `N_after = N_before + 1` のみ（difficult と重ねない）。`[S2 準拠]`（§6.5）

### 3.4 surge

- 溜めは `N` から導出。`surge_charge = display`（display >= 4）。`[S2 準拠]`（§6.7.1）
- 放出は `broke && N_before >= 5`。`[S2 準拠]`（§6.7.2）
- 量は `floor((N_before - 1) * mult)`。`[S2 準拠]`（§6.7.3）
- 3 分割は `[r, r, S-2r]`、`r = round_half_up(S/3)`。`[S2 準拠]`（§6.7.3 / §11-U4）
- 送出順は surge → main → PC。`[S2 準拠]`（§6.7.4）
- `N_before` と `N_after` の取り違えなし。`[S2 準拠]`

### 3.5 PC / GSB / 丸め

- PC: +5（multiplier 適用後 floor）、B2B +1 のみ。`[S2 準拠]`（§6.5）
- GSB: `garbage_cleared > 0 && (spin != none || lines >= 4)` で +1、最終 floor の内側。`[S2 準拠]`（§6.6）
- 丸めは `floor(base * mult + gsb)` の 1 回だけ。`[S2 準拠]`（§6.4）
- `GarbageMultiplier` は frame ごとの逐次加算。閉じた式ではない。`[S2 準拠]`（§7.8）

### 3.6 All-Mini+ の mini 扱い

- 非 T は mini 固定、full spin なし。`[S2 準拠]`（§4.2）
- mini single 0 / mini double 1 / mini triple 2 / mini quad 10。`[S2 準拠]`（§6.1）
- mini も difficult clear（B2B 維持・加算）。`[S2 準拠]`（§4.4）
- 0 ライン spin は B2B を維持も破壊もしない。`[S2 準拠]`（§4.4 / §5）

### 3.7 `S2Scoring` の状態

- `combo` は設置前の値、`b2b_streak` は設置前の N。`[S2 準拠]`（§5）
- 消去なしで combo_after = 0、N_after = N_before。`[S2 準拠]`
- 状態更新は `compute_attack` の結果を使用。`[S2 準拠]`

## 4. スピン（`board.rs`）

- T の 3-corner / front 2 / 5th kick 昇格。`[S2 準拠]`（§4.3）
- 非 T の immobile 4 方向 → Mini。`[S2 準拠]`（§4.2）
- 「最後の操作が回転」を前提。`[S2 準拠]`（§4.1）
- 左右の壁・底は filled、上は open。`[S2 準拠]`（§4.3）
- `FRONT` 表は v1.3.1 の y 反転版。`[S2 準拠]`（§4.3）
- T の mini は 2-corner ルールのみ、immobile は非 T 専用（U2 決定）。`[S2 準拠]`（§11-U2）
- **max spin label**: core の `spin_type` は 1 配置の分類のみ。referee が複数経路の最大ラベルを採用する（protocol §4.2.1）。これは **我々の取り決め** であり、core の分類自体は寛容ではない。`[我々の取り決め]`
- `spin_type` は配置の合法性を検査しない（呼び出し側が合法配置を渡す契約）。実装詳細。

## 5. ゴミキュー（`garbage.rs`）

- cap 8 行/ロック。`[S2 準拠]`（§7.1 / §7.5）
- 遅延 20 frame。`[S2 準拠]`（§7.1 / §7.5）
- 相殺は 1 行ずつ、送信優先。`[S2 準拠]`（§7.3）
- Opener は相殺予算 +attack（合計 2×）。`[S2 準拠]`（§7.4）
- `sent_total` は実際に飛んだ行数のみ。`[S2 準拠]`（§7.3）
- change-on-attack: packet 内同列、packet 消化時に reroll。`[S2 準拠]`（§7.6）
- MINSTD の draw 順序。`[S2 準拠]`（§7.6 / §7.7）
- キュー上限なし。`[S2 準拠]`（§7.1）
- combo blocking: ライン消去したロックでは挿入しない。`[S2 準拠]`（§7.5）
- 穴の縦順は one-by-one と一致。`[S2 準拠]`（§7.5）
- `GarbageQueue` は frame を保持せず、呼び出し側が `current_frame` を渡す。実装詳細。
- `receive(0)` は無視。実装詳細。

## 6. Opener（`opener.rs`）

- 最初の 14 設置（`pieces_placed < 14`）。`[S2 準拠]`（§7.4）
- 相殺予算が 2 倍（`attack + cancel_budget`）。`[S2 準拠]`（§7.4）
- `queue_total >= sent_total`（U5 決定）。`[S2 準拠]`（§11-U5）
- hold はカウントを進めない。`[S2 準拠]`（§2.5 / §7.4）
- `sent_total` は残り送信のみ加算。`[S2 準拠]`（§7.3）
- `on_piece_placed` はロックごとに呼ぶ（referee の責務）。実装詳細。

## 7. SRS+ / 180° kick（`kick.rs`）

- JLSTZ の SRS 表。`[S2 準拠]`（§3.1.1）
- I の SRS+ 表（T2/T3 入れ替え、0→L の T4/T5 入れ替え）。`[S2 準拠]`（§3.1.2 / §11-U1）
- 180° 全ピース・全 4 遷移。`[S2 準拠]`（§3.2）
- O の 90° は `(0,0)`、180° は汎用表。`[S2 準拠]`（§3.1.3）
- FIN/TST の 5th kick 判定 `(+1,-2)` / `(-1,-2)`。`[S2 準拠]`（§3.3）
- kick のテスト順は表順。`[S2 準拠]`

## 8. 7-bag / seed（`bag.rs`, `rng.rs`）

- MINSTD + 降順 Fisher–Yates。`[S2 準拠]`（§2.4）
- 7 種が 1 袋に 1 回ずつ。`[S2 準拠]`（§2.4）
- `peek` は消費しない。`[S2 準拠]`（§2.6）
- `derive_seeds`: `piece_seed = base.state`、`garbage_seed(0/1) = base.next()`。`[我々の取り決め]`（§11-U7。TETR.IO の導出式は非公開）
- `Bag::new` は `piece_seed` を受ける。`[我々の取り決め]`（U7 のレフェリープロトコル）
- **bag の初期配列順**: 仕様 §2.4 は初期配列順を明記していない。実装は `ALL_PIECES = [I,O,T,S,Z,J,L]`。既知 seed 0 列テストで固定。`[我々の取り決め]`
  - リスク: attax S2 版が別の初期順を使うと、同じ seed でも袋列が一致しない。仕様に初期順を追記し、attax と共有する必要がある。
- `BagState` / `Game::bag_state` は TBP `start.randomizer` 互換の読み取り専用 API。`[我々の取り決め]`
- `Bag::remaining` の current bag 導出は既存の queue/bag counter から計算。実装詳細。

## 9. 勝者決定・終了ロジックの全列挙

core には **勝者を選ぶロジックは無い**。core が持つのは「そのプレイヤーが死んだか」だけ。勝者は referee が決める（相手の勝ち）。

| # | 場所 | 何をするか | 分類 |
|---|---|---|---|
| 1 | `Game::is_over` | `topped_out || active.is_none()` | [S2 準拠]（block-out）+ 実装詳細 |
| 2 | `Game::spawn` | 標準 spawn が衝突 → `topped_out = true` | [S2 準拠] block-out |
| 3 | `Game::spawn`（clutch=true） | 1 行ずつ上へ探索、見つからなければ `topped_out` | [S2 準拠] Clutch |
| 4 | `Game::lock` | 次ピース spawn 失敗 → `LockResult.topped_out = true` | [S2 準拠] block-out |
| 5 | `Game::hold` | 交換後の spawn 失敗 → `topped_out` | [S2 準拠] block-out |
| 6 | `Game::insert_garbage` | active を標準 spawn に再スポーン、失敗 → `topped_out` | [我々の取り決め] 再スポーン + [S2 準拠] block-out |
| 7 | `Game::apply` | `topped_out` 後の入力を拒否 | 実装詳細 |
| 8 | `Game::check_invariants` | `topped_out` と `active` の不一致を検出 | 実装詳細（デバッグ） |
| 9 | `Game::from_position` | 不正な active / full row を拒否 | 実装詳細（テスト/replay 用） |
| 10 | `Board::insert_garbage` | `TooManyRows` / `InvalidHole` を返す（top-out ではない） | 実装詳細（API ガード） |
| 11 | `GarbageQueue` | top-out を判定しない | 実装詳細 |
| 12 | referee | どちらかが top-out した瞬間にラウンド終了、相手の勝ち | [我々の取り決め]（protocol §6.1） |
| 13 | referee | 空の suggestion の 2 段判定（合法手あり → forfeit、無し → top-out） | [我々の取り決め]（protocol §7） |
| 14 | referee | 1000 turn cap の順位（受信ゴミ少 → 送信ゴミ多 → draw） | [我々の取り決め]（protocol §6.3。S2 のルールではない） |
| 15 | referee | forfeit（非合法手、タイムアウト、プロトコル違反） | [我々の取り決め]（protocol §7） |

- **TurnCap の「攻撃量が多い方を勝ち」は core には存在しない**。referee の決定であり、S2 のルールではない。今回の修正対象は referee 側。
- core の `is_over` は block-out のみ。lock-out は無い。`[S2 準拠]`
- `has_legal_move` は core ではなく referee の movegen。`[我々の取り決め]`
- `insert_garbage` の active 再スポーンは **TBP 互換のための我々の取り決め**。S2 の「garbage 挿入後に active がどうなるか」を規定する一次ソースは無い。公式レフェリー経路はロック後挿入なので、再スポーン後の block-out 判定は S2 の block-out と一致する。
- `insert_garbage` は Clutch を適用しない。garbage はライン消去なしのロックでのみ挿入され、Clutch はライン消去ありのロックでのみ適用されるため、両者は排他。`[我々の取り決め]`
- `Game::lock` は次ピースを spawn してから referee が garbage を挿入する。この順序は S2 の一次ソースに明記が無く、TBP の active 前提に合わせた我々の取り決め。`[我々の取り決め]`

## 10. ルール外の実装詳細

- `Game::from_position` は full row を拒否する（テスト/replay 用の位置構築ヘルパー）。ゲーム中の一時状態には full row が存在しないため、ルール違反ではない。
- `Game::check_invariants` は `topped_out == active.is_some()` の不一致、full row、active の衝突を検出するデバッグ用。
- `Board::insert_garbage` の `TooManyRows` / `InvalidHole` は API ガード。S2 のルールではない。
- `GarbageQueue::receive(0)` は無視。S2 のルールではない。
- `GarbageQueue` は frame を保持せず、`take_insertable(lines, current_frame)` で frame を受け取る。referee のターン管理と分離するための API 設計。
- `Bag::remaining` は既存の queue と `bags_generated` から current bag の残数を導出する読み取り専用アクセサ。RNG/queue の内部表現は変えていない。
- `Attack.base` は診断用の丸め前値。ゲームの攻撃判定には `events`/`total` を使う。

## 11. 発見事項と修正

### F1 [誤り / S2 に存在しない] — 0 ライン設置の combo ln 攻撃（修正済み）

- 症状: `compute_attack` が combo 乗数を `lines > 0` でガードしていなかった。stale combo > 0 の 0 ライン設置で `ln_1p` 項が攻撃を作り、referee が 1 行送っていた。
- 影響: ライン消去に続く 0 ライン設置で、意図しない攻撃が発生。fixture 18 件は全て lines > 0 のため検出できなかった。
- 修正: combo 乗数と ln 項を `if input.lines > 0 { ... }` で囲む。
- テスト: `no_clear_resets_combo_but_keeps_b2b` を combo=5 / lines=0 に変更し、events 空・base 0 を固定。fixture 18/18 は引き続き緑。
- 仕様側の対応依頼: §6.8 の擬似コードに `lines > 0` ガードを追記する（spec lead へ）。

### F2 [我々の取り決め] — `insert_garbage` 後の active 再スポーン

- TBP `start` は active 位置を運ばないため、referee は挿入後に active を標準スポーンへ戻す。S2 のルールではなく、TBP 同期のための取り決め。
- 公式レフェリー経路はロック後挿入。再スポーン後の spawn 衝突は S2 の block-out として扱う。
- レポートで開示する。

### F3 [我々の取り決め] — `derive_seeds`（U7）

- TETR.IO の seed 導出は非公開。§11-U7 のレフェリープロトコルとして `piece_seed` / `garbage_seed(0/1)` を MINSTD 前進で導出する。S2 のルールではない。
- レポートで開示する。

### F4 [我々の取り決め] — bag の初期配列順

- 仕様 §2.4 は初期配列順を明記していない。実装は `[I,O,T,S,Z,J,L]` で固定し、seed 0 の既知列をテストで固定している。
- リスク: attax S2 版が別の初期順を使うと袋列が一致しない。仕様に初期順を追記し、attax と共有する。
- レポートで開示する。

### F5 [我々の取り決め] — max spin label

- core の `spin_type` は 1 配置の分類のみ。referee が複数経路の最大ラベルを採用する（protocol §4.2.1）。TETR.IO の実際の経路ラベルより寛容になり得る。
- core の分類は S2 準拠。referee の max-label 選択をレポートで開示する。

### F6 [我々の取り決め] — `BagState` / `Game::bag_state`

- TBP `start.randomizer` 互換の読み取り専用 API。S2 のルールではない。ゲーム状態を変えない。

### F7 [我々の取り決め] — `has_legal_move` は referee 側

- core には無い。protocol §7 の 2 段判定は referee の movegen が担う。core の `is_over` は block-out のみ。
- レポートで「core の終了条件は block-out、空候補の判定は referee」と明記する。

### F8 [S2 準拠] — U6 の上端破棄

- 40 行を超えた分を破棄する素直な実装。teto の 41 行 quirk は本プロファイルでは到達不能（spawn 最上段 22 + 最大 8 行 = 30 < 37）。§11-U6 の決定どおり。

### F9 [S2 準拠] — U3 / U5

- U3: PC の +1 B2B は surge に寄与する（S7 と同じ）。
- U5: Opener は `queue_total >= sent_total`。
- どちらも §11 の決定。レポートで「仕様の決定」と明記する。

## 12. 検証

- `cargo test --workspace`: 全テスト緑（本チェックアウト。harness-dev の保留中 3 テストは未反映）
- `cargo clippy --workspace --all-targets -- -D warnings`: 緑
- `cargo fmt --all -- --check`: 緑
- fixture: `docs/s2-attack-examples.json` 18/18、`docs/s2-rng-derivation.json` 4/4
- F1 修正後も fixture 18/18 と全 core テストが緑

## 13. 申し送り

1. **spec lead**: §6.8 の擬似コードに `lines > 0` ガードを追記（F1）。§2.4 に bag の初期配列順を追記（F4）。
2. **manager**: レポートで F2（active 再スポーン）、F3（U7 seed 導出）、F4（bag 初期順）、F5（max spin label）、F7（has_legal_move は referee 側）を「我々の取り決め」として開示する。
3. **referee**: TurnCap の攻撃量比較は S2 のルールではない。protocol §6.3 の決定として明記する（既に修正済みと聞いている）。
4. **core**: F1 以外に S2 に存在しない勝者決定/終了ロジックは無い。core は勝者を決めない。
