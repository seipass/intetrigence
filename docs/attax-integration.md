# attax 統合仕様 — TBP host 側の読み解きとレフェリー要件

対象: [souta-lab/attax](https://github.com/souta-lab/attax) の TBP アダプタ(`tbp/attax-tbp.ts` / `tbp/attax-tbp.sh`)。
本書は「我々の Rust レフェリーが attax を正しくホストする」ために必要な事実と要件をまとめる。
敵の探索・評価の中身は [attax-analysis.md](attax-analysis.md) を参照。

- 検証日: 2026-11-13(UTC 表記のエビデンスは 2026-11-13T14:3xZ)
- attax commit: **`7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b`**(`main`, 2026-11-13T21:11:16+09:00, "Merge pull request #37 from souta-lab/page/three-opponents")
- 検証環境: Windows 11 (Windows_NT 10.0.26200), AMD Ryzen 7 3700X, Bun **1.4.0**, Node v24.19.0
- 検証結果: `bun install` / `bun run build` / `bun test`(100 pass)成功。TBP ハンドシェイクと実手 9 手が成功し、**第 1 候補採用 9/9**、adapter は exit 0。
- エビデンス: `scripts/evidence/attax-tbp-smoke/{summary.json,transcript.jsonl,attax-decisions.jsonl}`
- 再現スクリプト: `scripts/attax-tbp-smoke.mjs`

> 注: attax は AGPL-3.0。作者から AGPL 範囲内での利用許可済み(ソース参照・引用可)。
> 本体 bot(`crates/bot`)へのコード・重みのコピーは方針として禁止。ハーネス(TBP host・検証スクリプト)での利用は可。
> 本書のコード引用は仕様理解のための最小限にとどめる。

---

## 1. セットアップ(Windows 11)

### 1.1 Bun のインストール

attax の TBP アダプタは **Bun 専用**。`for await (const line of console)`(Bun の stdin 行イテレータ)を使っており、Node では動かない。
未インストールなら PowerShell で:

```powershell
powershell -c "irm bun.sh/install.ps1 | iex"
# 新しいシェルを開いて
bun --version    # 検証時: 1.4.0
```

`bun` が PATH に無い場合、`tbp/attax-tbp.sh` は失敗する(`exec bun ...` のため)。その場合は `bun` の絶対パスで `.ts` を直接起動する。

### 1.2 clone とビルド

```powershell
cd C:\Users\Hasilan\Projects
git clone https://github.com/souta-lab/attax
cd attax
git rev-parse HEAD          # 7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b
bun install                 # 検証時: Checked 17 installs across 75 packages (no changes)
bun run build               # tsc --noEmit && vite build / 検証時: built in 488ms
bun test                    # 検証時: 100 pass, 0 fail, 9481 expect() calls, 13 files
```

注意: `bun run build` は追跡下の `dist/` を書き換える。エビデンスを取る前に `git checkout -- dist` で戻すと、
`summary.json` の `attax.dirty` が空文字列になり「未改変の上流コミット」であることが記録できる。

---

## 2. TBP アダプタの構造

| ファイル | 行数 | 役割 |
|---|---|---|
| `tbp/attax-tbp.ts` | 296 | TBP の JSON を行単位で読み、attax の fighter を呼び、`suggestion` を返す |
| `tbp/attax-tbp.sh` | 3 | `exec bun "$(dirname "$0")/attax-tbp.ts"`。cwd 非依存。Bun が PATH に必要 |
| `tbp/SMOKE.md` | 46 | 作者による intetrigence レフェリー相手のスモーク記録 |
| `tbp/smoke.py` | 38 | 作者による 2 ゲームの実レフェリー検証スクリプト |
| `tbp/intetrigence-match.md` | 32 | 旧 intetrigence(guideline, commit e816913)との 100–0 の記録 |

アダプタは **ゲームエンジンを一切使わない**。盤面・queue・hold・combo・b2b は TBP メッセージから自前で追跡し、
`createBot().findBestMove(snapshot)`(attax 本体の fighter)に渡す。つまり:

- アダプタが持つ状態は「ホストが送った情報」だけ。ホストが送り忘れた情報(ゴミキュー等)は存在しない。
- アダプタは `play` を信頼して盤面を更新する。**合法性検証はしない**。
- アダプタは `suggest` のたびに独立に探索する(探索状態の持ち越しなし)。`start` の `bot.reset()` は `lastSearch` を消すだけ。

### 2.1 Windows での起動

```powershell
# 方法 A: .ts を直接(Bun が PATH にあれば cwd 不問)
bun C:\Users\Hasilan\Projects\attax\tbp\attax-tbp.ts

# 方法 B: .sh 経由(Git Bash / WSL など sh がある環境)
sh C:\Users\Hasilan\Projects\attax\tbp\attax-tbp.sh
```

PowerShell から `.sh` を直接実行することはできない(`.sh` は sh スクリプトのため)。我々のレフェリーは
`bun <attax>/tbp/attax-tbp.ts` を直接 spawn するのが最も移植性が高い。

### 2.2 S2 profile と 40 行視野(新 pin `1f41806`)

**pin は `1f41806cbfe5ed3a5b1df81e6b7aac60f256e9d0`(S2 対応版)に更新済み**。
旧 `7b38f51` の 20 行切り詰めは、新アダプタでは **`ATTAX_TBP_ROWS=40`** で解消されている。

新アダプタ(`tbp/attax-tbp.ts`)の機能:

- `ATTAX_TBP_ROWS=40`: TBP の 40 行を snapshot に載せる(`grid` 40 行、`board.rows = 40`、
  `attaxCellsToTbp` は `FIELD_ROWS - 1 - ...`)。
- **空 queue で落ちない**: `snapshot()` が `null` を返し、`suggest` は空 suggestion で生存する。
- `ATTAX_TBP_PROFILE=s2`: `createBot({ profile: "s2" })` で **S2 profile** を選択する
  (本 repo の `adapters/attax-s2-tbp.patch` で追加。上流は profile 未指定 = guideline)。

### 2.2.1 `adapters/attax-s2-tbp.patch`(28 行、SHA-256 `461e4396de5fe046ec908d256df886f9c497faa8fbd7c27d02f46d197ab3049f`)

- `createBot` に `profile: "s2"` を渡す(`ATTAX_TBP_PROFILE=s2` のときだけ)。
- `snapshot()` の `posY` を **S2 spawn** に修正:
  `FIELD_ROWS > 20 ? FIELD_ROWS - 22 - maxY : -minY`。
  上流の `FIELD_ROWS - 21` は field40 rig の原点で、我々の S2 spawn(最下段セル row 21)とは
  ピースにより 2〜3 行ずれていた。

```powershell
cd C:\Users\Hasilan\Projects\attax
git checkout 1f41806cbfe5ed3a5b1df81e6b7aac60f256e9d0
git apply C:\Users\Hasilan\Projects\intetrigence\adapters\attax-s2-tbp.patch
bun run typecheck
```

起動時の環境変数:

```powershell
$env:ATTAX_TBP_ROWS = "40"
$env:ATTAX_TBP_PROFILE = "s2"
$env:ATTAX_TBP_LOG = "C:\tmp\attax-decisions.jsonl"
```

> **制約**: `createBot` は `GameSnapshot` の guideline フィールドしか S2 の `SearchInput` に写さない
> (`fighter.ts` のコメント)。`b2bChain` / `garbageRows` / `dueThisLock` / `frame` は渡らないため、
> **TBP 経由の S2 profile では surge の「溜め」・GSB・margin multiplier が探索に効かない**。
> 攻撃表・コンボ・All-Mini+・180°・deathHeight 21 は有効。
> 完全な S2 は `tools/s2-lab/` の外部ラボ用(`s2-input.ts` が S2-only フィールドを埋める)。

---

## 3. メッセージフロー

```
レフェリー(Rust)                         attax アダプタ(bun)
      │                                          │
      │ ── spawn ──────────────────────────────▶ │
      │ ◀──────────────────────── info ───────── │ 起動直後、1 行
      │ ── rules ──────────────────────────────▶ │ payload は無視される
      │ ◀──────────────────────── ready ──────── │
      │ ── start {board, queue, hold, ...} ────▶ │ 盤面/queue/hold/combo/b2b 設定
      │                                          │
      │ ── suggest ────────────────────────────▶ │ 探索(固定仕事量)
      │ ◀─────────────────── suggestion {moves} ─ │ 第 1 候補が先頭
      │ ── play {move} ────────────────────────▶ │ 盤面/queue/hold 更新
      │ ── new_piece {piece} ──────────────────▶ │ queue に push
      │            … 以下、1 ピースごとに繰り返し …  │
      │                                          │
      │ ── start (再送) ───────────────────────▶ │ ゴミ挿入など盤面が変わったとき
      │                                          │
      │ ── stop ───────────────────────────────▶ │ 何もしない(応答なし)
      │ ── quit ───────────────────────────────▶ │ exit 0
```

### 3.1 メッセージ仕様(アダプタ実装に基づく)

| 方向 | `type` | フィールド | アダプタの挙動 |
|---|---|---|---|
| bot→host | `info` | `name`, `version`, `author`, `features` | 起動直後に 1 回。`{name:"Attax", version:"overhaul/full", author:"attax", features:[]}` |
| host→bot | `rules` | 任意 | 即 `ready` を返す。payload は読まない |
| bot→host | `ready` | なし | `rules` への応答 |
| host→bot | `start` | `board`, `queue`, `hold?`, `combo?`, `back_to_back?` | 状態を全設定し `bot.reset()` |
| host→bot | `suggest` | なし | 探索して `suggestion` を返す |
| bot→host | `suggestion` | `moves[]`, `move_info` | 下記 §5.3 |
| host→bot | `play` | `move` | 盤面・queue・hold・combo・b2b を更新 |
| host→bot | `new_piece` | `piece` | `queue` に push |
| host→bot | `stop` | なし | 何もしない(応答を返さない) |
| host→bot | `quit` | なし | `process.exit(0)` |

`start` のフィールド:

- `board`: **40 行 × 10 列**の配列。`board[y][x]`、`y=0` が最下段。**非 null はすべて占有**として扱う
  (`(board[y]?.[x] ?? null) !== null`)。ゴミも区別せず非 null にする。
- `queue`: 現在ピース + preview。`queue[0]` が現在ピース、`queue[1..]` が preview。
  **`suggest` 時点で空だとアダプタは未捕捉例外で落ちる**(実測。§10 の落とし穴 1)。
- `hold`: ピース種または `null`。
- `combo`: TBP 規約。**0 = チェインなし**、連続消しで +1。アダプタは attax 規約(`-1` = チェインなし)へ `combo - 1` で変換する。
- `back_to_back`: 現在 B2B 中かどうか。

`play` の `move`:

```json
{ "location": { "type": "T", "orientation": "north", "x": 4, "y": 0 }, "spin": "none" }
```

`spin` は `"none" | "mini" | "full"`。

---

## 4. 座標系

### 4.1 TBP 盤面

- **40 行 × 10 列**、`board[y][x]`、**`y=0` が最下段**(上に向かって y が増える)。
- 非 null = 占有。attax 側は `8` などの値でも「埋まっている」とだけ解釈する。

### 4.2 ピース位置 = SRS true rotation 中心 + orientation

`location = {type, orientation, x, y}` は **Cold Clear 規約**で、`(x, y)` は
**SRS の真の回転中心**(4×4 ボックスの中心)を表す。占有セルは次で求まる:

```
cells = CC_CELLS[type] を orientation で回転し、(x, y) を足す
```

回転(中心原点、y 上向き):

| orientation | 変換 |
|---|---|
| `north` | `(cx, cy)` |
| `east` | `(cy, -cx)` |
| `south` | `(-cx, -cy)` |
| `west` | `(-cy, cx)` |

`CC_CELLS`(north の中心相対座標):

| 種 | cells |
|---|---|
| `I` | `(-1,0) (0,0) (1,0) (2,0)` |
| `O` | `(0,0) (1,0) (0,1) (1,1)` |
| `T` | `(-1,0) (0,0) (1,0) (0,1)` |
| `L` | `(-1,0) (0,0) (1,0) (1,1)` |
| `J` | `(-1,0) (0,0) (1,0) (-1,1)` |
| `S` | `(-1,0) (0,0) (0,1) (1,1)` |
| `Z` | `(-1,1) (0,1) (0,0) (1,0)` |

例: `T` の `north` は「上向きの T」(stem が上)。`south` は下向き。

### 4.3 attax 内部座標との対応

attax の内部盤面は **20 行・index 0 = 最上段**。アダプタの変換は:

- TBP → attax: `grid[r][x] = rows[19 - r][x]`(r は attax の上から index)
- attax → TBP: `tbp_y = 19 - (attax_y + cell_y)`

**帰結**: TBP の row 20〜39 は attax から見えない。row 19 より上のセルを含む配置は表現できず、
アダプタのログでは `cellsAboveRow20: true` になる。我々のレフェリーは「attax の視界は下 20 行」を
前提に、20 行を超えるスタックを許すなら**その旨を監査ログに残す**必要がある(PLAN §2 の制約)。

---

## 5. 状態追跡と `suggestion` の解釈

### 5.1 `applyPlay` の挙動(ホストが合わせるべき仕様)

1. `queue.shift()` で現在ピースを取り出す。
2. 置いたピース種が現在ピースと**違う**なら hold とみなす:
   - `hold = 取り出した現在ピース`
   - 直前の hold が `null` なら、さらに `queue.shift()`(swap で入ってきたピースを消費)
3. `move` のセルを盤面に書く。**範囲外のセルは黙って捨てる**。
4. 全 10 列が埋まった行を消し、上に空行を詰める。
5. `combo`: 消したら +1、消さなければ 0。
6. `b2b`: 消したとき `lines === 4 || spin !== "none"` なら true、それ以外の消しで false。消さなければ前値を維持。

> 重要: アダプタは `play` の内容を検証しない。ホストが不正な手を送ると、アダプタの盤面が静かにずれる。
> ホスト側で必ず合法性を検証してから `play` を送ること。

### 5.2 `snapshot()` が attax に渡す値

| フィールド | 値 |
|---|---|
| `board` | TBP row 0〜19 を 20 行に写像。埋まっていれば `8` |
| `active` | `queue[0]`、rotation 0、`posX=3`、`posY=-minY`(スポーン位置) |
| `hold` | 追跡中の hold |
| `canHold` | **常に `true`**(TBP に「hold 禁止」の概念がないため。毎 suggest の後にロックするので実質正しい) |
| `next` | `queue.slice(1)`(fighter 側で先頭 5 個に切る) |
| `combo` | `combo - 1`(attax 規約へ変換) |
| `b2b` | `back_to_back` |
| `pendingGarbage` | **常に `0`**(TBP に受信ゴミの情報がない) |

### 5.3 `suggestion` の構造

```json
{
  "type": "suggestion",
  "moves": [
    { "location": {"type":"T","orientation":"north","x":4,"y":0}, "spin":"none" },
    { "location": {"type":"T","orientation":"south","x":4,"y":1}, "spin":"none" },
    { "location": {"type":"T","orientation":"north","x":4,"y":0}, "spin":"mini" },
    ...
  ],
  "move_info": { "nodes": 8653, "nps": 0, "extra": "attax 37.0ms" }
}
```

順序(アダプタの `movesFor`):

1. **第 1 候補**を attax のスピンラベルで。
2. 同じセルを残り 2 つのスピンラベルで。
3. 第 2 候補以降も同様(候補ごとに 1〜3 ブロック)。

つまり `moves` は「セルは同じでスピンラベルだけ違う」重複を含む。レフェリーは**先頭から走査し、
自前の合法手リストに入る最初のものを採用**する。これでスピンラベルはレフェリー側の判定に固定される。

- **第 1 候補採用率** = `moves[0]` がそのまま合法だった割合。PLAN §3.3 の監査項目。
- **hold の推論**: `moves[0].location.type !== queue[0]` なら hold 使用。
  アダプタの `ATTAX_TBP_LOG` には `primary.hold` が入るので、これと突き合わせて監査できる。
- `moves` が空になるのは `findBestMove` が `null` を返したとき(合法手なし/ゲームオーバー)。ホストはフォールバック必要。
- `move_info.nps` は**常に 0**(計測していない)。`extra` に実測 ms が入る。

### 5.4 queue 補充の不変条件(重要)

アダプタの queue は「現在ピース + preview」の列。`play` のたびに先頭が消費されるので、ホストは
`new_piece` で補充する。**hold のケースで消費数が変わる**:

| 手 | アダプタが消費する queue | 必要な `new_piece` |
|---|---|---|
| 通常ロック | 1(現在ピース) | 1 |
| hold(hold スロットが空) | 2(現在ピース + swap で入るピース) | **2** |
| hold(hold スロットに既存) | 1(現在ピース。hold と交換) | 1 |

補充が 1 個足りないと preview が 4 枚に減り、探索の深さが 6 → 5 に落ちる(実測)。
ホストは「補充後に queue 長が 6 になるまで `new_piece` を送る」実装が安全。

---

## 6. `start` 再送条件

**`play` 以外で盤面が変わったら、必ず `start` を再送する。** 典型はゴミ挿入。

理由: アダプタは `play` から自前で盤面を追跡しており、ホストがゴミを挿入してもそれを知る手段がない。
再送しないと、アダプタは古い盤面で探索し、**ゴミと重なる手**を返す。

### 6.1 実測(ネガティブテスト)

`scripts/attax-tbp-smoke.mjs` は 6 手打った後に 4 行のゴミ(穴列 5)を挿入し、
**わざと `start` を再送せずに** `suggest` を送ってから、`start` を再送して 3 手打つ。

結果(`summary.json` の `staleBoard`):

```json
{
  "acceptedIndex": -1,
  "firstProblems": ["overlap (6,3)", "overlap (6,2)"],
  "first": { "location": { "type": "Z", "orientation": "east", "x": 5, "y": 2 }, "spin": "none" }
}
```

- 再送なし: 第 1 候補がゴミと重なり、**合法手が 1 つもない**(`acceptedIndex = -1`)。
- 再送あり: 続く 3 手はすべて `acceptedIndex = 0`(第 1 候補採用)。

再送時は `board` だけでなく `queue` / `hold` / `combo` / `back_to_back` も現在値で送る。
`start` は `bot.reset()` を呼ぶが、探索は毎手独立なので実害はない。

---

## 7. 環境変数

| 変数 | 必須 | 意味 |
|---|---|---|
| `ATTAX_TBP_LOG` | 任意 | JSONL の意思決定ログ出力パス。未設定ならログなし |

`ATTAX_TBP_LOG` の 1 行:

```json
{"ms":38.43,"nodes":8653,"depth":5,
 "primary":{"kind":"I","rotation":0,"x":6,"y":18,"tspin":"none","hold":false},
 "first":{"location":{"type":"I","orientation":"north","x":7,"y":0},"spin":"none"},
 "cellsAboveRow20":false}
```

- `ms`: `suggest` 受信から `suggestion` 送信までのアダプタ計測時間
- `nodes`: 探索ノード数
- `depth`: 最良ラインの深さ(hold を root で使うと 1 減る。§attax-analysis.md 参照)
- `primary`: attax が選んだ配置(attax 座標: 20 行・上から index)
- `first`: 実際に返した `moves[0]`
- `cellsAboveRow20`: 20 行視野を超えるセルがあったか

アダプタが読む環境変数はこれだけ。`bun` 自体の設定(`NO_COLOR` 等)は影響しない。

---

## 8. レフェリー(Rust)実装要件チェックリスト

### A. プロセス管理

- [ ] **A1** `bun` を PATH から解決(または絶対パス)。`bun <attax>/tbp/attax-tbp.ts` を spawn する。
      `.sh` は cwd 非依存だが、Windows では sh が必要なので `.ts` 直接が無難。
- [ ] **A2** stdin/stdout は**行区切り JSON**。1 行 1 メッセージ。stdout に JSON 以外を混ぜない。
- [ ] **A3** stderr は診断用にキャプチャして保存(クラッシュ時のスタックトレースがここに出る)。
- [ ] **A4** 起動直後の `info` をタイムアウト付きで読む。来なければ起動失敗として記録。
- [ ] **A5** 異常終了(非 0 exit / JSON パース失敗 / 応答タイムアウト)を検出し、該当ゲームを
      エラー/forfeit として記録。プロセスを kill してパイプを閉じる。
- [ ] **A6** 終了時に `quit` を送り、exit 0 を確認。`stop` は**応答を返さない**ので待たない。

### B. ハンドシェイク

- [ ] **B1** `info` を読み、`name` / `version` を監査ログに記録。
- [ ] **B2** `rules` を送る(payload は無視されるが必須)。
- [ ] **B3** `ready` を待つ。
- [ ] **B4** `start` を送る。

### C. `start` の内容

- [ ] **C1** `board` は 40 行 × 10 列。`y=0` が最下段。非 null = 占有。ゴミも非 null。
- [ ] **C2** `queue` は現在ピース + preview 5。**空にしない**。
- [ ] **C3** `hold` はピース種 or `null`。
- [ ] **C4** `combo` は TBP 規約(0 = チェインなし)。
- [ ] **C5** `back_to_back` は bool。
- [ ] **C6** S2 の surge / opener 残数 / ゴミキューは TBP に無い(§10)。

### D. ラウンド進行

- [ ] **D1** `suggest` → `suggestion` の往復時間を記録(median / p95)。
- [ ] **D2** 合法性検証: セルが 0..9 × 0..39、重なりなし、接地している。
      加えて**自前 movegen の到達可能性**で検証する(PLAN §3.3 の監査)。
- [ ] **D3** 採用は `moves` を先頭から走査し、自前の合法手リストに入る最初のもの。
      スピンラベルは自前の判定を優先(アダプタは代替ラベルを並べてくれる)。
- [ ] **D4** 採用した手を `play` で返す。**毎手必ず送る**(送らないとアダプタの盤面がずれる)。
- [ ] **D5** 毎ロック後に `new_piece` で補充し、queue を**常に「現在 + 5 preview」= 6 個**に保つ。
      **空の hold スロットを使った直後はアダプタが queue を 2 個消費する**ので、`new_piece` を 2 回送る。
      実測: 1 回だけだと preview が 4 枚に減り、探索 depth が 6 → 5 に落ちる。
      実装は「補充後に queue 長が 6 になるまで送る」のが安全。
- [ ] **D6** hold を推論して記録: `moves[0].location.type !== queue[0]`。
      `ATTAX_TBP_LOG` の `primary.hold` と突き合わせる。
- [ ] **D7** `moves` が空のときのフォールバック(合法手なし → ゲームオーバー処理)。

### E. `start` 再送

- [ ] **E1** ゴミ挿入など `play` 以外で盤面が変わったら**必ず** `start` を再送。
- [ ] **E2** 再送時は `board` / `queue` / `hold` / `combo` / `back_to_back` を現在値で送る。
- [ ] **E3** 再送のたびに監査ログへ「再送理由」と盤面ハッシュを記録。

### F. 監査・計測(PLAN §3.3)

- [ ] **F1** 両者の think time(median / p95)を毎ゲーム記録。
- [ ] **F2** attax の `move_info.nodes` と `extra` の ms を記録。
- [ ] **F3** 第 1 候補採用率(`moves[0]` が合法だった割合)と、非採用時のインデックス分布。
- [ ] **F4** `cellsAboveRow20` が常に false か確認(20 行視野の逸脱検出)。
- [ ] **F5** seed + 着手列の replay を保存し、`verify-replay` で再実行。
- [ ] **F6** 採点一致オーディット: suggestion のスピンラベル / 消し行数と、レフェリー実採点の一致。
- [ ] **F7** attax の commit ハッシュ・Bun バージョン・起動コマンドをレポートに固定。

### G. フェアネス

- [ ] **G1** attax を飢死させない(応答を待つ。タイムアウトは十分長く)。
- [ ] **G2** attax に時間制限を課さない(固定仕事量。§attax-analysis.md §7)。
- [ ] **G3** 両者に同じ seed・同じ preview 数・同じ hold ルール。
- [ ] **G4** attax の 20 行視野・ゴミ不可視はハンデとして明記(隠さない)。

---

## 9. 起動・再現手順

```powershell
# 前提: bun 1.4.0 が PATH にあり、../attax が commit 7b38f51 で clean

# 1) attax のビルド確認
cd C:\Users\Hasilan\Projects\attax
bun install
bun run build
bun test
git checkout -- dist      # 追跡下 dist を戻す(任意)

# 2) TBP ハンドシェイク + 実手のスモーク(我々のリポジトリから)
cd C:\Users\Hasilan\Projects\intetrigence
node scripts/attax-tbp-smoke.mjs
# 引数で attax の場所を指定することも可能:
# node scripts/attax-tbp-smoke.mjs C:\Users\Hasilan\Projects\attax

# 3) 生成物
# scripts/evidence/attax-tbp-smoke/summary.json
# scripts/evidence/attax-tbp-smoke/transcript.jsonl
# scripts/evidence/attax-tbp-smoke/attax-decisions.jsonl
```

スモークの合否は `summary.json` の `ok` と `steps[].ok` で判定する。期待値:

- `ok: true`、`steps` が全て `ok: true`(14 ステップ)
- `acceptance.firstChoiceAccepted === acceptance.suggestions`(9/9)
- `childExit.code === 0`
- `attax.commit === "7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b"`、`attax.dirty === ""`

手動でハンドシェイクだけ見る場合:

```powershell
cd C:\Users\Hasilan\Projects\attax
bun tbp/attax-tbp.ts
# → {"type":"info","name":"Attax","version":"overhaul/full","author":"attax","features":[]}
# 以下を 1 行ずつ stdin に貼る:
# {"type":"rules"}
# → {"type":"ready"}
```

---

## 10. 既知の落とし穴

| # | 事象 | 対策 |
|---|---|---|
| 1 | `queue` が空のまま `suggest` → 未捕捉例外で即終了(`error: suggest requires a nonempty queue from start`) | 常に「現在 + 5 preview」を維持。`new_piece` を毎手送る |
| 2 | ゴミ挿入後に `start` を再送しない → ゴミと重なる手を返す(実測) | §6 の再送を徹底 |
| 3 | TBP row 20〜39 は attax に見えない | 20 行超のスタックを許すなら監査ログに記録 |
| 4 | 受信ゴミが見えない(`pendingGarbage: 0`) | S2 では I/F 拡張が必要(§11) |
| 5 | スピンラベルがレフェリーと食い違う可能性 | `moves` を先頭から走査し、自前判定で採用 |
| 6 | `play` を送り忘れるとアダプタの盤面がずれる | 毎手必ず送る。送った手をログに残す |
| 7 | `stop` に応答が返らない | 応答を待たない |
| 8 | Windows で `.sh` が直接実行できない | `bun <path>/tbp/attax-tbp.ts` を直接 spawn |
| 9 | `bun run build` が追跡下 `dist/` を書き換える | エビデンス前に `git checkout -- dist` |
| 10 | `move_info.nps` が常に 0 | NPS はホスト側で `nodes / ms` から計算する |

---

## 11. 未解決事項(S2 拡張)

TBP には以下の S2 状態を運ぶフィールドがない。PLAN §3.3 の「attax S2 版の接続 I/F は作者と調整」に該当する。

| 必要な情報 | 現状 | 候補 |
|---|---|---|
| 受信ゴミキュー(行数・タイマー・穴列) | 無い(`pendingGarbage: 0`) | TBP 拡張 `garbage` メッセージ / 独自プロトコル |
| B2B charging の streak / surge 溜め | 無い(`back_to_back` の bool のみ) | `start` に `b2b_streak`, `surge` を追加 |
| Opener Phase の残りピース数 | 無い | `start` に `opener_remaining` |
| All-spin の判定結果 | 無い(`spin: none/mini/full` は T 前提) | `spin` を全ピースに拡張 |
| 180° 回転 | 無い(`orientation` は 4 方位) | orientation を 8 方位に、または 180° フラグ |
| 40 行の可視化 | 下 20 行のみ | アダプタの 20 行写像を拡張 |

これらは attax 側の実装変更を伴うため、**作者との合意が前提**。我々のレフェリーは
「現行 TBP(guideline 相当)」と「S2 拡張」の 2 モードを切り替えられる構造にしておくのが安全。

S2 側の確定仕様(攻撃テーブル・B2B charging・コンボ乗算・All-Mini+・180° kick)は
[s2-spec.md](s2-spec.md) §3〜§6 を参照。attax の現行実装との差分は
[attax-analysis.md](attax-analysis.md) §9 にまとめた。

---

## 12. スモークテストの実測結果(2026-11-13)

`node scripts/attax-tbp-smoke.mjs` の出力(`summary.json`):

| 項目 | 値 |
|---|---|
| ステップ | 14/14 OK |
| ハンドシェイク | `info` → `rules` → `ready` → `start` OK |
| 実手 | 9 手(通常 6 + ゴミ後 3) |
| 第 1 候補採用 | **9/9**(全手 `acceptedIndex = 0`) |
| think time(ホスト往復) | median **31.51 ms**, p95 41.26 ms, min 27.0 ms, max 41.26 ms |
| 探索ノード | 8,168〜10,809(median 9,124) |
| アダプタ自己申告 ms | 25.5〜38.4 ms |
| 探索 depth | 5 が 3 手(hold 空の序盤)、6 が 6 手(hold 取得後) |
| queue 長 | 全 suggest で 6(現在 + 5 preview) |
| `cellsAboveRow20` | 全 10 決定で `false` |
| アダプタ exit | 0 |
| ゴミ再送テスト | 再送なし: `acceptedIndex = -1`(ゴミと重なる)/ 再送あり: 3/3 採用 |

生ログ:

- `scripts/evidence/attax-tbp-smoke/transcript.jsonl` — host↔adapter の全メッセージ(送受信・タイムスタンプ)
- `scripts/evidence/attax-tbp-smoke/attax-decisions.jsonl` — アダプタの `ATTAX_TBP_LOG`
- `scripts/evidence/attax-tbp-smoke/summary.json` — 上記の集計 + attax commit / Bun / OS / 環境

> 注: このスモークは**ホスト側シミュレーション**であり、実際の versus ゲームではない。
> 盤面モデル・ピース形状・hold/queue 追跡は我々の Rust レフェリーが実装すべき仕様を写したもの。
> 実ゲームでの検証は、Rust レフェリーの TBP host 実装後に行う(M0 の合否条件)。

---

## 13. attax の forfeit 分析(2026-11-15)

### 13.1 事実(m5-provisional、220 ゲーム)

| 結果 | 件数 |
|---|---|
| TopOut 勝ち | **0** |
| TopOut 負け | 13 |
| TurnCap 勝ち | 13 |
| **forfeit-vision** | **194** |
| 実勝率 | 13/220 = **5.9%** |

### 13.2 根本原因: 20 行切り詰め

`tbp/attax-tbp.ts::snapshot()` は TBP の 40 行のうち **下 20 行だけ**を attax の 20 行グリッドに写像する:

```ts
const grid = Array.from({ length: 20 }, (_, r) => (rows[19 - r] ?? []).map(...));
```

S2 のスポーンは **最下段セル = row 21**(可視 20 行の 2 行上)。attax のグリッドでは active が
最上段(row 19)に置かれるため、**row 19 に 1 つでもブロックがあると movegen が 0 候補**になる。
`ATTAX_TBP_LOG` の forfeit 直前の行は:

```json
{"ms":0.12,"nodes":0,"depth":0,"primary":null,"first":null,"cellsAboveRow20":true}
```

- `nodes: 0` = 探索が 1 ノードも展開していない(root の `generatePlacements` が空)
- `primary: null` = `findBestMove` が `null` を返した
- `cellsAboveRow20: true` = 盤面に row 20 以上のセルがある(切り詰めで失われた)

seed 11012 の forfeit は **turn 33**、attax 側のスタック高さ **23**。row 19 まで積まれていたため、
20 行グリッドの最上段で active が衝突し、0 候補になった。盤面が「低い」のではなく、
**attax から見える範囲の最上段が埋まっていた**のが原因。

### 13.3 分類バグ

`crates/arena/src/referee.rs::classify_forfeit` は

```rust
let lowest_cell = active.cells().iter().map(|&(_, y)| y).min().unwrap_or(0);
if lowest_cell as u32 >= visible_rows { return ForfeitCause::Vision; }
```

S2 のスポーンは常に row 21、`visible_rows=20` なので **無条件に Vision** になっていた。
この判定を削除し、`stack_top >= visible_rows`(盤面が隠しバッファに届いている)と
「可視行に収まる合法配置が無い」の 2 条件だけで分類するようにした。

### 13.4 修正と結果

`adapters/attax-vision-40.patch`(68 行、SHA-256 `e22aab7b5a2f9a95efb562e24b3f33da351bb942d6daf6e205475b5871ed3b70`)で
attax を 40 行対応にした(upstream は **20 行**。パッチ適用後のみ 40 行):
`ROWS=40`、BFS の `Y_SPAN=48`、`snapshot()` の grid 40 行、`attaxCellsToTbp` の `39 - ...`、
**`snapshot()` の active を S2 spawn(grid y=18)に**(旧 `posY=-minY` は grid 最上段で 18 行ずれ)。

- ベース: upstream `7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b` + 本パッチ(attax は TypeScript を直接実行するため追加ビルドなし)
- 適用: `cd <attax> && git apply <repo>/adapters/attax-vision-40.patch`
- 変更は**視野のみ**(探索幅・深さ・評価項・重み・思考時間は不変)

- seed 11012 sheet A: **1000 turn 完走、forfeit 0**(以前は turn 33 で forfeit)
- seed 11012 sheet B: **1000 turn 完走、forfeit 0**(以前は turn 38 の low-stack `other`)
- seed 11001〜11012 の 24 ゲーム(300 turn)の再実行結果は §13.5 に追記。

### 13.5 再実行結果(パッチ後)

**代表 10 ゲーム(seed 11001–11005 × 両シート、1000 turn、`--ms 100 --threads 8`)**

```text
summary: 10 games, wins 8 losses 2 draws 0 forfeits 0, verified 10/10
```

| ゲーム | 結果 | turns |
|---|---|---|
| 11001 A | 勝ち(TurnCap) | 1000 |
| 11001 B | 勝ち(TopOut) | 804 |
| 11002 A | 勝ち(TurnCap) | 1000 |
| 11002 B | 勝ち(TopOut) | 411 |
| 11003 A | 勝ち(TopOut) | 351 |
| 11003 B | 勝ち(TopOut) | 723 |
| 11004 A | **負け(TopOut)** | 724 |
| 11004 B | 勝ち(TopOut) | 938 |
| 11005 A | **負け(TopOut)** | 191 |
| 11005 B | 勝ち(TopOut) | 322 |

- **forfeit 0**。旧 194 forfeit(162 other + 32 vision)は、40 行 + posY 修正で消えた。
- `nodes` は 10,000 前後、depth 6、ms median 35(旧 31.5)。探索は走っている。
- 勝率 8/10 = 80%。**パッチ適用 attax は本物の 40 行相手として強い**。50-0 は「ハンディ付き attax」ではなく
  実力勝負になる。敗因は TopOut(スタック管理)で、surge/opener の攻防ではない。
- エビデンス: `scripts/evidence/attax-40-smoke/{summary.json,audit/}`

### 13.5.1 個別確認

| ゲーム | 結果 | 備考 |
|---|---|---|
| seed 11012 sheet A | 1000 turn 完走、forfeit 0 | 旧: turn 33 forfeit(高スタック 23) |
| seed 11012 sheet B | 1000 turn 完走、forfeit 0 | 旧: turn 38 forfeit(低スタック 19) |
| seed 11001 sheet A(別 run) | 834 turn、TopOut(attax 勝ち) | 旧: forfeit。attax nodes 8,036〜9,417、depth 6 |

### 13.6 注意: upstream は 20 行

- **upstream `7b38f51` は 20 行**(`ROWS=20`、grid 20 行、`19 - ...`)。
- パッチ適用後の作業ツリーは 40 行に見えるが、それは `adapters/attax-vision-40.patch` の結果。
- `visible_rows` は **upstream では 20、パッチ適用後は 40**。パッチとセットで宣言を上げる。
- パッチ SHA-256: `e22aab7b5a2f9a95efb562e24b3f33da351bb942d6daf6e205475b5871ed3b70`(68 行)。

## 14. 180° capability

| pin / profile | 180° kick | `supports_180` |
|---|---|---|
| 旧 `7b38f51`(guideline) | なし(`dir < 2`、CW/CCW のみ) | **false** |
| 新 `1f41806` guideline | なし(旧と同じ) | **false** |
| 新 `1f41806` **S2 profile** | **あり**(`src/ai/bot/movegen.ts` の `KICKS_180_JLSTZ` / `KICKS_180_I`、`rotate180: true`) | **true** |

- **S2 profile(`ATTAX_TBP_PROFILE=s2`)では 180° が使える**。`supports_180 = true`。
- guideline profile では従来どおり 180° なし(`supports_180 = false`)。
- レフェリー側は profile に応じて `supports_180` を切り替える必要がある
  (S2 profile 接続時は true、guideline 接続時は false)。

---

## 15. S2 attax との 1 ゲーム(2026-11-16)

条件: seed 9001 sheet A、1000 turn、`--ms 100`/8t、`ATTAX_TBP_ROWS=40` + `ATTAX_TBP_PROFILE=s2`。
attax は `1f41806` + `adapters/attax-s2-tbp.patch`。

```text
seed-9001-sheet-A: winner=1 reason=TopOut turns=169 verified=true mismatches=0
summary: 1 games, wins 0 losses 1 unresolved 0 no-contest 0, verified 1/1
```

| 項目 | intetrigence | attax(S2) |
|---|---|---|
| 結果 | TopOut 負け | **TopOut 勝ち** |
| turns | 169 | 169 |
| attack/piece | 0.882(149/169) | **0.929**(156/168) |
| 受信ゴミ | 69 | 60 |
| think median | 68.5 ms | 41.6 ms |
| 第 1 候補採用 | — | 168/168 |
| spin/attack 監査 | — | **168/168 一致**(mismatch 0) |

- **forfeit / no-contest / unresolved は 0**、replay 検証 1/1。
- attax の decision log: 168 決定、nodes median 9,868、depth 6(167/168)。
- **S2 profile は有効**(攻撃表・コンボ・All-Mini+・180°・deathHeight 21)。
  attack/piece 0.929 は S2 の攻撃表 + コンボ + B2B +1 + PC +5 による。
- **制約**: `createBot` は S2-only フィールド(`b2bChain` / `garbageRows` / `dueThisLock` / `frame`)を
  渡さないため、**surge の溜め・GSB・margin multiplier は attax の探索に効かない**。
  ゲームの採点(レフェリー)は S2 で行われるが、attax の読みはその分弱い。
- エビデンス: `C:/tmp/attax-s2-game1/{summary.json,audit/,replays/,logs/}`(ローカル)。
  `logs/seed-9001-sheet-A-attax.jsonl` が decision log。

---

## 16. S2 attax 10 ゲーム(2026-11-16)

条件: tuning seeds 9001–9010、sheet A、1000 turn、`--ms 100`/8t、
**決定論 `--deterministic --depth 8 --width 88`**、`ATTAX_TBP_ROWS=40` + `ATTAX_TBP_PROFILE=s2`、
`--tbp-supports-180 true`、attax `1f41806` + `adapters/attax-s2-tbp.patch`。

```text
summary: 10 games, wins 1 losses 7 unresolved 0 no-contest 2, verified 10/10
5 分類: TopOut 勝ち 1 / TopOut 負け 7 / TurnCap 0 / Forfeit 2 / 未決着 0 = 10
```

| 指標 | intetrigence | Attax(S2) |
|---|---|---|
| attack/piece | 0.729 | **0.900** |
| attack/line | 1.061 | **1.560** |
| spin rate | 0.086 | **0.151** |
| net damage / game | +23.4 | **+123.9** |
| received / game | 105.8 | 60.8 |
| mean / max height | 6.29 / 19.6 | 5.31 / 17.1 |
| collapse(最終高さ ≥ 20) | 1 | 0 |
| collapse 負け / race 負け | 6 / 1 | — |
| think median(ms) | 63.6 | 44.4 |
| nodes median | 580 | 10,149 |
| spin 監査 | 0 / 2814 | 1 / 2806 |
| attack 監査(same spin) | 0 / 2814 | 0 / 2806 |

- **forfeit 2/10（9003, 9007）の原因は I の 90° kick の試行順（U1）**。180° 6 候補化は別件で、
  この forfeit は直らない。attax の S2 版は I の 90° kick を標準 SRS 順で使い、我々は SRS+ ミラー順。
  オフセット集合は 8 遷移すべて一致するが、順序が 5/8 遷移で違い、SRS は最初に fit した候補で
  確定するため到達可能な配置が変わる。詳細と決定は §17。
- **`load_note` は null**(`--note` 未指定)。
- **attax の decision log は未取得**(`--tbp-*` 経路は `decision_log` を渡さず、`ATTAX_TBP_LOG` も未設定)。
- `createBot` の S2-only フィールド未写像の制約は §2.2.1 / §15 のとおり。
- エビデンス: `scripts/evidence/attax-s2-10/{REPORT.md,summary.json,summary.md,audit/,replays/,logs/}`。

---

## 17. U1: I の 90° kick 順（2026-11-16、manager 決定 C）

### 決定

**C（和集合）**:

- **ボットの自手生成**: **SRS+ のみ**（`I_KICKS_SRS_PLUS`）。生成は部分集合のまま。
- **attax**: **標準 SRS 順のまま**。書き換えない（互換パッチ不要）。
- **レフェリーの到達可能性判定**: **両順序の和集合**。どちらかで到達できれば合法。
- **記録**: **`kick_union_only`**（SRS+ では到達できず SRS では到達できる配置の数）。

### 根拠

- U1 は **[単一実装]**（S7 `halp1/triangle` の `kicks/data.ts`）。TETR.IO の真の順序を
  一次ソースで確定できない。
- SRS は「最初に fit した候補で確定」するため、順序の違いは**到達可能な配置の集合**を変える。
  例: `L→2` の 4 番目が我々 `(1,2)`、attax `(-2,-1)`。attax は空洞内の横 I に到達する。
- 片方の順序で判定すると、他方の実装の合法手を不正扱いする。これは相手のせいではなく、
  **我々の未確定仕様の問題**。
- **A（SRS+ 維持 + attax パッチ）は誤差が無界で検出不能**（U1 が覆れば attax 段階が全滅）。
  **C は件数で有界に測れる**。manager は「誤差を測れること」を優先した。
- 和集合は**判定にだけ**効かせ、生成には効かせない。どちらにも有利にならない。

### 実装

- `crates/core/src/kick.rs`: `I_KICKS_SRS_PLUS`（既定）と `I_KICKS_SRS`、`KickOrder`、
  `kicks_with_order` を公開。`I_KICKS` は SRS+ のまま。
- `crates/bot`: 既定 `KickOrder::SrsPlus`。ボットの生成は SRS+ のみ。
- `crates/arena`: `MoveSet` が両順序を走査して和集合を取り、`alternate_only` を記録。
- `GameAudit` / `GameSummary` / `MatchSummary` / `StageReport` / `RoundReport` に
  `kick_union_only` を追加。
- 180° 6 候補化（`79dfa77`）は S7 のデータに基づく別件の正しい修正であり、維持する。

### しきい値

- 50 ゲームで **0〜2 件**: U1 を確定しない（影響は無視できる）。
- **3〜4 件**: 記録して続行。
- **5 件以上**: TETR.IO のリプレイで分岐局面を探し、U1 を確定する。

### 検証

- `i_90_kick_order_changes_reachability`: 和集合で covered well が到達可能、
  `is_alternate_only` が true。
- seed 9003 turn 268 の forfeit が消える（同じ配置が受理される）。
- `cargo test --workspace` 298 passed / clippy / fmt。
