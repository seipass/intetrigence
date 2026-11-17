# Cold Clear 2 統合仕様 — TBP 接続とレフェリー要件

対象: [MinusKelvin/cold-clear-2](https://github.com/MinusKelvin/cold-clear-2) の TBP 実装(`src/main.rs` / `src/tbp.rs`)。
本書は「我々の Rust レフェリー(`crates/arena` + `crates/tbp`)が CC2 を正しくホストする」ための事実と要件をまとめる。
clys は [clys-integration.md](clys-integration.md)、attax は [attax-integration.md](attax-integration.md) を参照。

- 検証日: 2026-11-14
- CC2 commit: **`ed8b19327b6bd1410ddd873d8611485bd45d8fae`**(`main`, 2022-06-29, "improve move generation performance")
- ビルド: Rust edition 2021 / `cargo build --release`(25.4s、警告のみ)
- バイナリ: `target/release/cold-clear-2.exe`(1,143,296 bytes, SHA-256 `097cb142…e3ecee`)
- 既定設定: `src/default.json` 埋め込み(SHA-256 `39770179…c1178e`)。外部設定なしで起動可
- 検証環境: Windows 11 / AMD Ryzen 7 3700X / Bun 1.4.0(attax 用)/ Rust 1.98.1
- ライセンス: MIT OR Apache-2.0。**本体 bot へのコード・重みのコピーは方針禁止**(§10)
- S2 対応: **非対応**(§11)。採点・監査はレフェリー側で行う

---

## 1. セットアップ

### 1.1 clone とビルド

```powershell
cd C:\Users\Hasilan\Projects
git clone https://github.com/MinusKelvin/cold-clear-2
cd cold-clear-2
git rev-parse HEAD          # ed8b19327b6bd1410ddd873d8611485bd45d8fae
cargo build --release       # 検証時 25.4s
```

- 追加の依存(ネットワーク)は cargo が取得する。`bun` は不要。
- 既定設定は `src/default.json` が `include_str!` で埋め込まれる(`BotConfig::default()`)。
  外部設定を使う場合は `-c <config.json>` を渡す。**公式マッチでは既定設定を使う**(§7)。

### 1.2 TBP 互換パッチ(空の hold + スポーン位置)

上流 `ed8b193` の `create_bot` は **`hold=None` のとき `queue[0]`(current)を hold スロットに移す**:

```rust
let reserve = start.hold.unwrap_or_else(|| start.queue.remove(0));
```

`GameState.reserve` は hold スロットで、CC2 のモデルは **空の hold を表現できない**(`Piece` に NONE がない)。
このままだと CC2 は current を 1 ピース先と誤認し、最初の `play` を hold と解釈する
(実測: `queue=[I,T,L,S,Z,O]` で 1 手目 I、2 手目が T ではなく **L**)。

互換パッチ `adapters/cc2-tbp-compat.patch` は空 hold、スポーン位置、`suggest` ごとの探索枠を扱う。

**(a) 空の hold の表現**

- `start` では current を維持し、空 hold を明示する。
- 探索グラフでは、空 hold を使うと current だけを消費し、盤面・得点を変えずに次のピースの層へ進む。次の設置まで再 hold を禁止し、通常の設置と同じ評価で候補を比較する。数手先の初回 hold もこの遷移で探索する。
- TBP の `play` では、空 hold で次のピースを置いたときだけ2枚進める。current と次のピースが同種のときは TBP が hold の有無を運ばないため、通常設置として扱う。

**(b) スポーン位置を S2 に合わせる**

CC2 の `movegen.rs` は全ピースを `y: 19`(可視最上段)からスポーンさせる。S2 のスポーンは
**最下段セルが row 21**(バッファ内)なので、2 行ずれている。高スタック(高さ 21)では CC2 の active が
スタックに埋まり、movegen が 0 candidates を返して forfeit する。`y: 19` を **`y: 21`** に変更する
(2 箇所: fast mode と slow mode)。

```powershell
cd C:\Users\Hasilan\Projects\cold-clear-2
git apply C:\Users\Hasilan\Projects\intetrigence\adapters\cc2-tbp-compat.patch
cargo build --bin cold-clear-2 --release --locked
```

動作確認は Intetrigence のルートで `node adapters/cc2-hold-smoke.mjs <CC2実行ファイル>` を実行する。初手・数手後・プレビューが後で届く場合の空 hold、2枚補充、同種ピースの扱いを TBP 経由で検証する。公式マッチのレポートにはパッチと CC2 バイナリの識別情報を記録する。

**探索枠**。Rust の公式ラウンドは `INTETRIGENCE_TBP_THINK_MS=100` を渡して起動し、CC2 の `info.features` に `intetrigence-request-window-v1:100` があることを確認する。探索木は手の間で引き継ぐ。探索ワーカーは `suggest` に応じた枠内で新しい展開を開始し、開始済みの展開と入出力の時間も審判が計測する。単発検証では arena の `--tbp-request-window-ms 100` または `node adapters/tbp-request-window-smoke.mjs <CC2実行ファイル>` を使う。

### 1.3 起動

```powershell
# 単体で TBP を話す(標準入出力)
C:\Users\Hasilan\Projects\cold-clear-2\target\release\cold-clear-2.exe
# → {"type":"info","name":"Cold Clear 2","version":"0.1.0 ed8b193","author":"MinusKelvin","features":[]}
```

レフェリーからは `program = ...\cold-clear-2.exe`, `args = []`, `cwd = cold-clear-2 ルート` で spawn する。
`cwd` は必須ではないが、ログや設定の相対パスを避けるためルートを推奨。

---

## 2. プロトコル

CC2 は **TBP ネイティブ**。`src/main.rs` が stdin の行を `FrontendMessage` として読み、
`src/lib.rs::run` が `BotMessage` を stdout に書く。標準 TBP の順序(`play` → `new_piece`)で動く。

| 方向 | `type` | CC2 の挙動 |
|---|---|---|
| bot→host | `info` | 起動直後に 1 回。`version` は `"0.1.0 <GIT_HASH>"` |
| host→bot | `rules` | 即 `ready`。payload は読まない |
| bot→host | `ready` | `rules` への応答 |
| host→bot | `start` | 盤面・queue・hold・combo・b2b を設定。`randomizer` は省略可 |
| host→bot | `suggest` | 現在の探索結果を `suggestion` で返す |
| bot→host | `suggestion` | `moves[]` + `move_info {nodes, nps, extra}` |
| host→bot | `play` | `bot.advance(mv)` で状態を進める |
| host→bot | `new_piece` | queue に push |
| host→bot | `stop` | 無視(応答なし) |
| host→bot | `quit` | 終了 |

### 2.1 `start` のフィールド

```json
{
  "type": "start",
  "board": [[null, ...], ...],          // 40 行 × 10 列、y=0 が最下段
  "queue": ["I","T","L","S","Z","O"],   // 現在 + preview
  "hold": null,
  "combo": 0,                            // TBP 規約: 0 = チェインなし
  "back_to_back": false,
  "randomizer": {"type":"seven_bag","bag_state":["J"]}   // 任意(推奨)
}
```

- `board`: `Vec<[Option<char>; 10]>` として 40 行を読む。**セルは `null` またはピース文字列**(`"I"`, `"G"` 等)。
  `8` などの整数を送ると `invalid type: integer 8, expected a character` で panic する(実測)。
- `randomizer` は `#[serde(default)]` で省略可。省略すると `Randomizer::Unknown` になり、
  `speculate = false`(未来の袋を仮定しない)になる。**公平性のため送る**。
- `start` で `hold.is_none() && queue.is_empty()` の場合、CC2 は最初の `new_piece` まで待つ
  (`waiting_on_first_piece`)。通常は queue を 6 個入れるので即開始する。

### 2.2 `suggestion` の形

```json
{
  "type": "suggestion",
  "moves": [
    {"location":{"type":"I","orientation":"north","x":1,"y":0},"spin":"none"}
  ],
  "move_info": {
    "nodes": 22142,
    "nps": 80557.99014690865,
    "extra": "100.0% of selections expanded, overall speed: 0.1 Mnps"
  }
}
```

- `moves` は通常 1 件(最良手のみ)。attax のような代替候補は返さない。
- **`nps` は浮動小数**(`f64`)。我々の `MoveInfo` は `nps: f64` で受ける必要がある
  (`u64` だと `invalid type: floating point ..., expected u64` で forfeit する。実測)。
- `extra` は人間向けの統計文字列。ms は含まれない。

### 2.3 空 suggestion の race

CC2 の探索は `start` / `play` でワーカースレッドが再開し、`suggest` はその時点の最良手を返す。
`start` 直後に `suggest` を送ると、root 展開前で **`moves: []`** を返すことがある(実測)。

- ホストは空 `moves` を受けたら **5ms 間隔で `suggest` を再送**する(最大 2s)。
- `think_ms` は全リトライ分を合算する。
- attax は空 = 本物の forfeit だが、CC2 はリトライで回復する。

実測(空盤面、`suggest` を送るまでの待ち時間と nodes):

| 待ち | moves | nodes |
|---|---|---|
| 0ms(直後) | 0(空) | 0 |
| 5ms | 1 | 214,642 |
| 20ms | 1 | 405,501 |
| 100ms | 1 | 915,919 |
| 200ms | 1 | 1,298,006 |

---

## 3. 座標系

CC2 は TBP 標準の座標系を使う。attax と同じ Cold Clear 規約。

- 盤面: **40 行 × 10 列**、`board[y][x]`、**`y=0` が最下段**。
- `location = {type, orientation, x, y}` は **SRS true rotation 中心**(4×4 ボックスの中心)。
  `orientation ∈ north/east/south/west`。
- 占有セルは `CC_CELLS[type]` を orientation で回転し `(x, y)` を足したもの
  (表は [attax-integration.md](attax-integration.md) §4.2 と同じ)。
- CC2 は **40 行すべてを見る**(`visible_rows = 40`)。attax の 20 行制限はない。

---

## 4. 状態追跡

CC2 は `play` で `bot.advance(mv)` を呼び、内部の `GameState` を進める。
ホストは以下の順序を守る:

1. `suggest` → `suggestion` を受け取り、レフェリーの合法手リストで検証する。
2. 採用した手を `play` で送る。
3. 次に引いたピースを `new_piece` で送る(毎ロック 1 個。初回 hold は 2 個)。

- `play` を送り忘れると CC2 の盤面がずれる。attax と同様、毎手必ず送る。
- `new_piece` は `play` の**後**でよい(標準順)。clys とは逆なので注意。
- `combo` は TBP 規約(0 = チェインなし)。CC2 は `start.combo` をそのまま使う。
- `back_to_back` は bool。

---

## 5. `start` 再送

CC2 は `play` から盤面を追跡するため、**ゴミ挿入など `play` 以外で盤面が変わったら `start` を再送**する。
再送時は `board` / `queue` / `hold` / `combo` / `back_to_back` / `randomizer` を現在値で送る。
CC2 の `start` は `waiting_on_first_piece` をリセットし、`bot.start(create_bot(...))` で新しい探索を始める。

---

## 6. 環境変数・設定

| 項目 | 値 |
|---|---|
| 環境変数 | なし |
| CLI | `-c/--config <path>`(任意)、`--profile`(puffin profiler、ビルド時 feature) |
| 既定設定 | `src/default.json` 埋め込み。`freestyle_weights` + `freestyle_exploitation` |
| 思考時間オプション | **なし**。ワーカーは常時探索し、`suggest` 時点の最良手を返す |

既定設定の主要値(`src/default.json`):

```json
{
  "freestyle_weights": {
    "cell_coveredness": -0.2, "max_cell_covered_height": 6,
    "holes": -1.5, "row_transitions": -0.2,
    "height": -0.4, "height_upper_half": -1.5, "height_upper_quarter": -5.0,
    "tetris_well_depth": 0.3,
    "tslot": [0.1, 1.5, 2.0, 4.0],
    "has_back_to_back": 0.5, "wasted_t": -1.5, "softdrop": -0.2,
    "normal_clears": [0.0, -2.0, -1.5, -1.0, 3.5],
    "mini_spin_clears": [0.0, -1.5, -1.0],
    "spin_clears": [0.0, 1.0, 4.0, 6.0],
    "back_to_back_clear": 1.0, "combo_attack": 1.5,
    "perfect_clear": 15.0, "perfect_clear_override": true
  },
  "freestyle_exploitation": 0.6931471805599453
}
```

- これは **guideline 用の重み**。S2 の surge / opener / all-spin / ゴミキューは考慮しない。
- 公式マッチでは既定設定を固定し、コミットと SHA-256 をレポートに記録する。

---

## 7. レフェリー(Rust)実装要件チェックリスト

### A. プロセス

- [ ] **A1** `program = <cc2>/target/release/cold-clear-2.exe`, `args = []`, `cwd = <cc2>` で spawn。
- [ ] **A2** stdin/stdout は行区切り JSON。stderr をキャプチャ。
- [ ] **A3** 起動直後の `info` をタイムアウト付きで読む。
- [ ] **A4** 異常終了・タイムアウトを forfeit として記録。
- [ ] **A5** 終了時に `quit` を送り、exit 0 を確認。`stop` は応答なし。

### B. ハンドシェイク

- [ ] **B1** `info` を読み、`name` / `version`(GIT_HASH 含む)を記録。
- [ ] **B2** `rules` を送る。
- [ ] **B3** `ready` を待つ。
- [ ] **B4** `start` を送る。

### C. `start`

- [ ] **C1** `board` は 40 行 × 10 列、`y=0` が最下段。
- [ ] **C2** `queue` は現在 + preview 5(6 個)。空にしない。
- [ ] **C3** `hold` はピース種 or `null`。
- [ ] **C4** `combo` は TBP 規約(0 = チェインなし)。
- [ ] **C5** `back_to_back` は bool。
- [ ] **C6** `randomizer: {"type":"seven_bag","bag_state":[...]}` を送る(任意だが推奨)。

### D. ラウンド進行

- [ ] **D1** `suggest` → `suggestion` の往復時間を記録。
- [ ] **D2** 合法性検証(セル範囲・重なり・接地・自前 movegen の到達可能性)。
- [ ] **D3** `moves` を先頭から走査し、自前の合法手リストに入る最初のものを採用。
- [ ] **D4** 採用手を `play` で送る(毎手必須)。
- [ ] **D5** 次ピースを `new_piece` で送る(毎ロック 1 個、初回 hold は 2 個)。
- [ ] **D6** **空 `moves` のリトライ**(5ms 間隔、最大 2s)。
- [ ] **D7** `move_info.nps` を **f64** で受ける。

### E. `start` 再送

- [ ] **E1** ゴミ挿入など `play` 以外で盤面が変わったら再送。
- [ ] **E2** 再送時は全フィールドを現在値で送る。

### F. 監査

- [ ] **F1** think time(median / p95 / p99 / max)。
- [ ] **F2** `move_info.nodes` / `nps` / `extra`。
- [ ] **F3** 第 1 候補採用率(CC2 は 1 件なので 0 か 1)。
- [ ] **F4** replay 検証(seed + 着手列)。
- [ ] **F5** 採点一致オーディット(スピンラベル・攻撃)。
- [ ] **F6** commit / 設定 / 起動コマンドをレポートに固定。

### G. フェアネス

- [ ] **G1** 応答を待つ(飢死させない)。
- [ ] **G2** 時間制限を課さない。`suggest_delay_ms = 100` で 100ms の予算を与える。
- [ ] **G3** 両者に同じ seed・5 preview・hold。
- [ ] **G4** `visible_rows = 40`(CC2 は全 40 行を見る)。

---

## 8. 起動・再現手順

```powershell
# 1) ビルド
cd C:\Users\Hasilan\Projects\cold-clear-2
git rev-parse HEAD          # ed8b19327b6bd1410ddd873d8611485bd45d8fae
cargo build --release

# 2) ハンドシェイク(手動)
printf '{"type":"rules"}\n{"type":"quit"}\n' | .\target\release\cold-clear-2.exe
# → {"type":"info",...}
# → {"type":"ready"}

# 3) レフェリーのスモーク(harness-dev の round ランナー)
cd C:\Users\Hasilan\Projects\intetrigence-m2
.\target\release\round.exe run --round 0 --seeds 9001 --out .\scripts\evidence-smoke `
  --dry-run --max-stages 2 --gate-attax 2 --gate-cc2 2 --turns 300 `
  --attax-dir C:\Users\Hasilan\Projects\attax `
  --attax-commit 7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b `
  --cc2-program C:\Users\Hasilan\Projects\cold-clear-2\target\release\cold-clear-2.exe `
  --cc2-cwd C:\Users\Hasilan\Projects\cold-clear-2 `
  --cc2-commit ed8b19327b6bd1410ddd873d8611485bd45d8fae `
  --cc2-visible-rows 40 --cc2-suggest-delay-ms 100 --cc2-retry-empty-ms 2000
```

---

## 9. 既知の落とし穴

| # | 事象 | 対策 |
|---|---|---|
| 1 | `move_info.nps` が float で JSON パースに失敗 | `MoveInfo.nps` を `f64` にする |
| 2 | `start` 直後の `suggest` が空 `moves` | 5ms 間隔で再送(最大 2s) |
| 3 | `randomizer` を送らないと speculate 無効 | `bag_state` を送る |
| 4 | `stop` は応答なし | 待たない |
| 5 | 40 行すべてを見る | `visible_rows = 40` |
| 6 | 既定設定は guideline 用 | S2 非対応を明記(§11) |

---

## 10. 方針(ライセンス)

- CC2 は **MIT OR Apache-2.0**。attax より緩いが、**本体 bot へのコード・重みのコピーは方針禁止**。
- ハーネスでの利用(subprocess 実行、検証スクリプト、仕様分析)は可。
- CC2 の重み(`src/default.json`)を intetrigence の評価に移植しない。
- チューニングは tuning seeds のみ。held-out seeds では最適化しない。

---

## 11. S2 非対応の明記

CC2 は **TETR.IO Season 2 非対応**。以下は guideline のまま:

- 攻撃テーブル: Single 0 / Double 1 / Triple 2 / Quad 4 / T-spin 系(guideline)。
- B2B: bool の +1(charging/surge なし)。
- コンボ: guideline の加算(乗算なし)。
- パーフェクトクリア: guideline の +10(S2 は +5)。
- All-Spin / opener phase / ゴミキュー / 180°: なし。
- ゴミ: `start` に受信ゴミの情報がない(TBP の制約)。CC2 はゴミを見ない。

したがって CC2 とのマッチは **「S2 非対応ボット相手の汎化検証」** であり、
勝率の解釈にはこの旨を明記する(PLAN §1 の goal 条件 2)。

---

## 12. スモーク結果(2026-11-14)

レフェリー: `intetrigence-m2` の `round` ランナー(`c5d4b2d` + `bag_after_queue` + core の
`insert_garbage` active 再スポーン修正)。CC2 は **hold パッチ適用版**(§1.2)。

```text
seed-9001-sheet-A: winner=0 reason=TopOut turns=97 verified=true mismatches=0
seed-9001-sheet-B: winner=0 reason=TopOut turns=79 verified=true mismatches=0
round 0: cc2 2-0-0 PASSED
```

| 項目 | sheet A | sheet B |
|---|---|---|
| 結果 | TopOut(CC2 が負け) | TopOut(CC2 が負け) |
| turns | 97 | 79 |
| replay 検証 | OK(mismatches 0) | OK(mismatches 0) |
| CC2 の第 1 候補採用 | 97/97 | 79/79 |
| CC2 の think median / p95 | 100.1 / 100.2 ms | 100.1 / 100.2 ms |
| intetrigence の think median | 97.5 ms | 97.0 ms |

集計(2 ゲーム): attack/piece は intetrigence **0.897** vs CC2 **0.790**、
受信ゴミ/ゲームは intetrigence 28.5 vs CC2 36.5、勝ちの種類は TopOut 2 / forfeit 0 / turn-cap 0。

- `start` 再送は garbage 挿入時に発生(§5)。再送後も forfeit せず、TopOut まで完走。
- 以前の forfeit(turn 30〜46)は、`bag_state` のずれ + `insert_garbage` の active 位置上書き + CC2 の hold バグの
  3 つが重なったもの。3 つとも修正済み。
- エビデンス: `scripts/evidence/cc2-smoke/{summary.json,audit/}`

## 13. 180° capability

- CC2 の movegen は 4 回転の**休息配置**を生成するが、kick は **SRS 90° のみ**(`src/movegen.rs` の
  `Rotation::North/East/South/West` + 90° kick table)。**S2 180° の kick 遷移は無い**。
- したがって **`supports_180 = false`**。レフェリーの 180° 限定合法手は CC2 には出せない。
- スモーク(seed 9001–9010、sheet A、1000 turn)では **forfeit 0 / no-contest 0 / unresolved 0**、verified 10/10。
  3 敗は TopOut(CC2 勝ち)で、接続の問題ではない。

### 12.1 10 ゲームスモーク(2026-11-15)

seed 9001–9010、sheet A、1000 turn、`--ms 100`/8t。

```text
summary: 10 games, wins 7 losses 3 unresolved 0 no-contest 0, verified 10/10
```

- **forfeit 0 / no-contest 0 / unresolved 0**。3 敗はすべて TopOut(CC2 勝ち)。
- 接続は透明 TBP シム(`--attax DIR` の `tbp/attax-tbp.ts` が CC2 バイナリを exec)経由。
- エビデンス: `scripts/evidence/cc2-10/{summary.json,audit/}`
- CC2 はネイティブの decision log を持たないため `measurement incomplete (opponent_log=false)` と表示される。
  replay / audit は完全。
