# clys 統合仕様 — 専用アダプタとレフェリー要件

対象: [citrus610/clys](https://github.com/citrus610/clys) の TBP 実装(`tbp/main.cpp` / `tbp/tbp.h`)と、
我々の専用アダプタ `adapters/clys/tbp/adapter.ts`。
CC2 は [cc2-integration.md](cc2-integration.md)、attax は [attax-integration.md](attax-integration.md) を参照。

- 検証日: 2026-11-14
- clys commit: **`ccae45fc3e30c3d3432af2b06d5b4f7f2e8dfbcc`**(`main`, 2025-08-16, "update config")
- ビルド: C++20 / MinGW-w64 GCC 16.2.0(WinLibs UCRT)/ 静的リンク
- バイナリ: `bin/tbp/tbp.exe`(3,100,043 bytes, SHA-256 `21d0a57d…4631eb2`)
- 設定: `config.json`(1,009 bytes, SHA-256 `198c6f1d…e48637`)。cwd から読む
- 検証環境: Windows 11 / AMD Ryzen 7 3700X / Bun 1.4.0(アダプタ)
- ライセンス: リポジトリの `LICENSE` を参照。**本体 bot へのコード・重みのコピーは方針禁止**(§10)
- S2 対応: **非対応**(§11)。採点・監査はレフェリー側で行う

---

## 1. セットアップ

### 1.1 ツールチェーン

clys は C++20。上流の `makefile` は MinGW 前提(`-msse4 -mbmi2 -march=native -flto`)だが、
**HEAD ではそのままではビルドできない**(§1.3)。我々は MinGW-w64 GCC 16.2.0 を使う。

```powershell
# WinLibs (UCRT) を winget で導入(検証時 16.2.0)
winget install --id BrechtSanders.WinLibs.POSIX.UCRT --accept-package-agreements --accept-source-agreements
# g++ は %LOCALAPPDATA%\Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.UCRT_*\mingw64\bin\g++.exe
```

### 1.2 ビルド(安定版)

```powershell
cd C:\Users\Hasilan\Projects\clys
git rev-parse HEAD          # ccae45fc3e30c3d3432af2b06d5b4f7f2e8dfbcc
mkdir bin\tbp

g++ -DUNICODE -DNDEBUG -std=c++20 -O3 -msse4 -mbmi2 `
    -static -static-libgcc -static-libstdc++ `
    core\bag.cpp core\board.cpp core\move.cpp core\state.cpp `
    ai\ai.cpp ai\beam.cpp ai\eval.cpp ai\gaze.cpp ai\layer.cpp ai\path.cpp `
    tbp\main.cpp -o bin\tbp\tbp.exe
```

**上流との差分(重要)**:

| 項目 | 上流 makefile | 我々のビルド | 理由 |
|---|---|---|---|
| `core/piece.cpp` | 含む(`core/*.cpp`) | **除外** | HEAD では古い死コード(`Piece::Data` が `piece.h` に存在せずコンパイル不能) |
| `-march=native` | あり | **なし** | この環境で探索スレッドが SIGSEGV(3/3 再現) |
| `-flto` | あり | **なし** | 同上(LTO でも再現) |
| `-static` | `tbp` ターゲットには無し | **あり** | Git の `libstdc++-6.dll` を拾って SIGSEGV するため |
| `lib/ppt_sync` のコピー | `makedir` が実行 | 不要 | `tbp` ターゲットには不要。リポジトリにも存在しない |

- `-O3 -msse4 -mbmi2`(native/LTO なし)は 3/3 安定。`-O2` でも可。
- デバッグビルド(`-O0 -g`)は常に安定。クラッシュ時の切り分けに使う。

### 1.3 TBP 互換パッチ(スポーン位置)

clys の `core/move.cpp` は **全ピースを `(x=4, y=19)` からスポーン**させる。S2 のスポーンは
**最下段セルが row 21** なので、2 行ずれている。高スタック(高さ 21 以上)では clys の movegen が
スポーン位置で衝突して 0 candidates を返し、`suggest` が空 plan → `return -1`(exit 255)になる。

`adapters/clys-tbp-spawn.patch` をビルド前に適用する。

```powershell
cd C:\Users\Hasilan\Projects\clys
git apply C:\Users\Hasilan\Projects\intetrigence\adapters\clys-tbp-spawn.patch
```

変更内容:
- `Placement init = Placement(4, 19, ...)` → `(4, 21, ...)`
- スポーン衝突チェックの `1ULL << 19` → `1ULL << 21`(2 箇所)、`1ULL << 20` → `1ULL << 22`
- 公式ラウンドでは `INTETRIGENCE_TBP_THINK_MS=100` に応じ、`suggest` を受けてから探索を開始し、同じ100 ms枠で停止を要求する。join と入出力の時間も計測に含む。アダプター自身の従来の待機はこのモードで無効になる。

回帰テストは Intetrigence のルートで `node adapters/tbp-request-window-smoke.mjs bun adapters/clys/tbp/adapter.ts --clys-dir <clysルート>` を実行する。相手の機能申告、探索結果、待機時間、正常終了を確認する。

パッチ後、seed 9002 の 2 ゲームは **両方 TopOut で完走**(forfeit 0)。

### 1.4 起動

```powershell
cd C:\Users\Hasilan\Projects\clys      # cwd 必須(config.json を読む)
.\bin\tbp\tbp.exe
# → {"author":"citrus610","features":[],"name":"clys","type":"info","version":"v1.5"}
```

- **cwd = clys リポジトリルート**。`config.json` が無いと既定値を書き出す(`save()`)。
- 公式マッチではリポジトリの `config.json` を固定し、SHA-256 をレポートに記録する。

---

## 2. プロトコル(clys の方言)

clys は TBP を話すが、**標準ホストでは動かない癖**がある。我々のアダプタが吸収する。

| 方向 | `type` | clys の挙動 | アダプタの扱い |
|---|---|---|---|
| bot→host | `info` | 起動直後。`name="clys"`, `version="v1.5"` | そのまま転送 |
| host→bot | `rules` | 即 `ready` | そのまま転送 |
| bot→host | `ready` | `rules` への応答 | そのまま転送 |
| host→bot | `start` | **`randomizer.bag_state` 必須**。無いと例外で落ちる | 無ければ 7-bag からフォールバックを補う |
| host→bot | `suggest` | `ai.request(0)` で探索を join し、3 moves を返す | **100ms 待ってから**転送 |
| bot→host | `suggestion` | 同一 placement の `spin: full/none/mini` の 3 件 | そのまま転送 |
| host→bot | `play` | 前回 play 以降の `new_piece` vector で `ai.advance` | **`new_piece` を先に**送ってから転送 |
| host→bot | `new_piece` | vector に push(即時には使わない) | バッファし、次の `play` の前に送る |
| host→bot | `stop` | `ai.request(0); ai.clear();`(**破壊的**) | **握りつぶす** |
| host→bot | `quit` | 終了 | そのまま転送 |

### 2.1 `start` の必須フィールド

```json
{
  "type": "start",
  "board": [[null, ...], ...],          // 40 行 × 10 列
  "queue": ["I","T","L","S","Z","O"],
  "hold": null,
  "combo": 0,
  "back_to_back": false,
  "randomizer": {"type":"seven_bag","bag_state":["J"]}
}
```

- clys の `Randomizer` は `bag_state` だけを読み、`type` は無視する(nlohmann は未知フィールドを許容)。
- `bag_state` は **queue 末尾時点の現在の袋の残り**(TBP spec 0001)。空だと `Bag::deupdate` の再構築が狂う。
- `board` のセルは **`null` またはピース文字列**(`"I"`, `"G"` 等)。clys の `Cell` enum は文字列を要求し、
  整数を送ると nlohmann のパースで落ちる。
- アダプタは `randomizer` が無い/空の場合、queue から 7-bag を推定して補う(フォールバック)。
  ただし**レフェリーは正しい `bag_state` を送ること**(`Game::bag_state().remaining`)。

### 2.2 `suggestion` の形

```json
{
  "type": "suggestion",
  "moves": [
    {"location":{"type":"I","orientation":"north","x":6,"y":0},"spin":"full"},
    {"location":{"type":"I","orientation":"north","x":6,"y":0},"spin":"none"},
    {"location":{"type":"I","orientation":"north","x":6,"y":0},"spin":"mini"}
  ],
  "move_info": {"nodes": 1008819, "nps": 0, "depth": 20, "extra": "9013"}
}
```

- **3 moves はすべて同じ placement** で、スピンラベルだけが違う。代替 placement は無い。
- `move_info.nps` は常に 0。`depth` は探索深さ、`extra` は評価値。
- レフェリーは 3 件を先頭から検証し、自前のスピン判定で採用手を決める。

### 2.3 `suggest` の race と think budget

clys は `start` / `play` で探索スレッドを開始し、`suggest` の `ai.request(0)` が
`running` フラグを落として現在の beam iteration を join する。

- **`start` 直後に `suggest` を送ると、root 展開前で空 plan → clys は `return -1` で即終了**する。
- アダプタは `suggest` を **100ms 遅延**してから送る。これで探索に 100ms の予算を与え、空 plan を避ける。
- 実測(空盤面、100ms 遅延): depth 10〜20、nodes 368k〜1.0M、応答 25ms 前後。
- 深い層では 1 iteration の join に時間がかかるため、応答が 100ms を超えることがある。
  `think_ms` は実測値として記録する(打ち切りはしない)。

### 2.4 `new_piece` の順序(最重要)

標準 TBP は `play` → `new_piece`。clys は `play` 時に「前回 play 以降に受けた `new_piece` の vector」を
`ai.advance(piece, new_piece)` に渡すため、**`new_piece` が先**でないと queue が 1 個ずつ縮む。

アダプタの動作:

1. `play` を受けたら **保留**する(まだ clys に送らない)。
2. `new_piece` を受けたらバッファし、5ms の settle timer を張る。
3. timer 発火 or 次の `suggest`/`quit` で、**バッファした `new_piece` をすべて送ってから `play`** を送る。
4. 初回 hold で `new_piece` が 2 個来る場合は、両方を `play` の前に送る。

これで clys の `advance` が正しい queue を得る。

---

## 3. 座標系

clys は TBP 標準の座標系を使う(attax / CC2 と同じ Cold Clear 規約)。

- 盤面: **40 行 × 10 列**、`board[y][x]`、**`y=0` が最下段**。
- `location = {type, orientation, x, y}` は **SRS true rotation 中心**。
- 占有セルは `CC_CELLS[type]` を orientation で回転し `(x, y)` を足したもの
  ([attax-integration.md](attax-integration.md) §4.2 の表と同じ)。
- clys は **40 行すべてを見る**(`visible_rows = 40`)。

> アダプタは座標を**書き換えない**。clys の TBP 座標をそのまま転送し、
> レフェリーがセル集合一致で検証する(`crates/tbp/src/geometry.rs`)。

---

## 4. アダプタの起動形

```
program = bun
args    = ["C:/Users/Hasilan/Projects/intetrigence/adapters/clys/tbp/adapter.ts",
           "--clys-dir", "C:/Users/Hasilan/Projects/clys",
           "--think-ms", "100"]
cwd     = "C:/Users/Hasilan/Projects/intetrigence"
```

- アダプタは内部で `cwd = C:/Users/Hasilan/Projects/clys` の `bin/tbp/tbp.exe` を spawn する。
- `--log <jsonl>` または環境変数 `CLYS_TBP_LOG` で全メッセージの監査ログを出す。
- `--think-ms` または `CLYS_THINK_MS` で think budget を変更できる(既定 100ms)。
- `--clys-dir` または `CLYS_DIR` で clys の場所を指定する。

### 4.1 アダプタの責務

| 責務 | 実装 |
|---|---|
| `new_piece` → `play` の順序変換 | `pendingPlay` + `pendingNewPieces` + 5ms settle |
| `randomizer.bag_state` の補完 | queue から 7-bag を推定(フォールバック) |
| think budget | `suggest` を `--think-ms` 遅延 |
| `stop` の握りつぶし | clys の `stop` は engine を clear するため |
| 監査ログ | `--log` / `CLYS_TBP_LOG` に JSONL |
| 異常終了の伝播 | clys の exit code をそのまま返す |

---

## 5. `start` 再送

clys は `play` から盤面を追跡する。ゴミ挿入など `play` 以外で盤面が変わったら `start` を再送する。
アダプタは `start` を受けると保留中の `play`/`new_piece` を破棄し、新しい `start` を clys に送る
(clys の `start` は `ai.clear()` + `ai.init()` + `ai.search()` で完全リセット)。

---

## 6. 設定

| 項目 | 値 |
|---|---|
| 環境変数 | なし(clys 本体) |
| CLI | なし |
| 設定ファイル | `config.json`(cwd)。無ければ既定値を書き出す |
| 思考時間 | なし。`suggest` のタイミングで探索を join |
| 探索 | beam width **500**, depth **32**, branch **3**(`beam::Configs` 既定) |
| 探索の特徴 | Monte Carlo で未来の queue を 3 本生成し、入力 queue の先まで読む |

`config.json` の主要値(guideline 用):

```json
{
  "b2b": 100, "bonus_b2b": 200, "bonus_ren": 200,
  "bump": -25, "clear": [-400, -300, -200, 250],
  "cover": -20, "delay": -25, "feed": -100, "height": -50,
  "height_map": [10,10,0,-5,-10,-10,-5,0,10,10],
  "hole_a": -400, "hole_b": -200, "order": 100,
  "pc": 2000, "pc_next": 1000,
  "ren": [180, 450, 1000, 1500, 2000],
  "transition": -20, "tslot": [100, 200, 250, 500],
  "tspin": [50, 400, 750],
  "waste_I": -100, "waste_T": -150,
  "well": 50, "well_map": [-100,-150,0,50,100,100,50,0,-150,-100],
  "scale": 1024
}
```

- S2 の surge / opener / all-spin / ゴミキューは考慮しない。
- 公式マッチでは `config.json` を固定し、SHA-256 をレポートに記録する。

---

## 7. レフェリー(Rust)実装要件チェックリスト

### A. プロセス

- [ ] **A1** `program = bun`, `args = [adapter.ts, --clys-dir, <clys>, --think-ms, 100]`, `cwd = <repo>`。
- [ ] **A2** アダプタの stdout は行区切り JSON。stderr をキャプチャ。
- [ ] **A3** 起動直後の `info` をタイムアウト付きで読む。
- [ ] **A4** 異常終了・タイムアウトを forfeit として記録。
- [ ] **A5** 終了時に `quit` を送り、exit 0 を確認。`stop` は送らない(アダプタが握りつぶす)。

### B. ハンドシェイク

- [ ] **B1** `info` を読み、`name` / `version` を記録。
- [ ] **B2** `rules` を送る。
- [ ] **B3** `ready` を待つ。
- [ ] **B4** `start` を送る。

### C. `start`

- [ ] **C1** `board` は 40 行 × 10 列、`y=0` が最下段。
- [ ] **C2** `queue` は現在 + preview 5(6 個)。
- [ ] **C3** `hold` はピース種 or `null`。
- [ ] **C4** `combo` は TBP 規約(0 = チェインなし)。
- [ ] **C5** `back_to_back` は bool。
- [ ] **C6** `randomizer: {"type":"seven_bag","bag_state":[...]}` を**必ず**送る。

### D. ラウンド進行

- [ ] **D1** `suggest` → `suggestion` の往復時間を記録(アダプタの 100ms delay を含む)。
- [ ] **D2** 合法性検証(セル範囲・重なり・接地・自前 movegen の到達可能性)。
- [ ] **D3** 3 moves を先頭から走査し、自前の合法手リストに入る最初のものを採用。
- [ ] **D4** 採用手を `play` で送る(毎手必須)。
- [ ] **D5** 次ピースを `new_piece` で送る(アダプタが順序を変換)。
- [ ] **D6** 空 `moves` は clys の exit -1 につながるため、アダプタの delay で回避。

### E. `start` 再送

- [ ] **E1** ゴミ挿入など `play` 以外で盤面が変わったら再送。
- [ ] **E2** 再送時は全フィールドを現在値で送る。

### F. 監査

- [ ] **F1** think time(median / p95 / p99 / max)。
- [ ] **F2** `move_info.nodes` / `depth` / `extra`。
- [ ] **F3** 第 1 候補採用率(3 moves のうち何番目を採用したか)。
- [ ] **F4** replay 検証(seed + 着手列)。
- [ ] **F5** 採点一致オーディット(スピンラベル・攻撃)。
- [ ] **F6** commit / config.json の SHA-256 / 起動コマンドをレポートに固定。

### G. フェアネス

- [ ] **G1** 応答を待つ(飢死させない)。
- [ ] **G2** 時間制限を課さない。アダプタの 100ms delay で予算を与える。
- [ ] **G3** 両者に同じ seed・5 preview・hold。
- [ ] **G4** `visible_rows = 40`。

---

## 8. 起動・再現手順

```powershell
# 1) ビルド(§1.2)
cd C:\Users\Hasilan\Projects\clys
g++ -DUNICODE -DNDEBUG -std=c++20 -O3 -msse4 -mbmi2 -static -static-libgcc -static-libstdc++ `
  core\bag.cpp core\board.cpp core\move.cpp core\state.cpp `
  ai\ai.cpp ai\beam.cpp ai\eval.cpp ai\gaze.cpp ai\layer.cpp ai\path.cpp `
  tbp\main.cpp -o bin\tbp\tbp.exe

# 2) ハンドシェイク(手動)
printf '{"type":"rules"}\n{"type":"quit"}\n' | .\bin\tbp\tbp.exe
# → {"author":"citrus610",...}
# → {"type":"ready"}

# 3) アダプタ単体(モックレフェリー)
cd C:\Users\Hasilan\Projects\intetrigence
bun adapters/clys/tbp/adapter.ts --clys-dir C:\Users\Hasilan\Projects\clys --think-ms 100

# 4) レフェリーのスモーク(harness-dev の round ランナー)
cd C:\Users\Hasilan\Projects\intetrigence-m2
.\target\release\round.exe run --round 0 --seeds 9001 --out .\scripts\evidence-smoke `
  --dry-run --max-stages 3 --gate-attax 2 --gate-cc2 2 --gate-clys 2 --turns 300 `
  --attax-dir C:\Users\Hasilan\Projects\attax `
  --cc2-program C:\Users\Hasilan\Projects\cold-clear-2\target\release\cold-clear-2.exe `
  --cc2-cwd C:\Users\Hasilan\Projects\cold-clear-2 `
  --cc2-commit ed8b19327b6bd1410ddd873d8611485bd45d8fae `
  --clys-program bun `
  --clys-args "C:/Users/Hasilan/Projects/intetrigence/adapters/clys/tbp/adapter.ts,--clys-dir,C:/Users/Hasilan/Projects/clys,--think-ms,100" `
  --clys-cwd C:\Users\Hasilan\Projects\intetrigence `
  --clys-commit ccae45fc3e30c3d3432af2b06d5b4f7f2e8dfbcc `
  --clys-visible-rows 40 --clys-suggest-delay-ms 0
```

---

## 9. 既知の落とし穴

| # | 事象 | 対策 |
|---|---|---|
| 1 | `new_piece` を `play` の後に送ると queue が縮む | アダプタが `new_piece` を先に送る |
| 2 | `randomizer.bag_state` が無いと例外で落ちる | レフェリーが必ず送る。アダプタもフォールバック |
| 3 | `start` 直後の `suggest` で空 plan → exit -1 | アダプタが 100ms 遅延 |
| 4 | `stop` が engine を clear する | アダプタが握りつぶす |
| 5 | `-march=native` / `-flto` で SIGSEGV | 外す(§1.2) |
| 6 | 動的 libstdc++ が Git の DLL を拾う | `-static` でリンク |
| 7 | `core/piece.cpp` がコンパイル不能 | ビルドから除外 |
| 8 | 3 moves は同一 placement | レフェリーは先頭から検証し、自前スピン判定 |
| 9 | 応答が 100ms を超えることがある | 打ち切らず `think_ms` を記録 |

---

## 10. 方針(ライセンス)

- clys のライセンスはリポジトリの `LICENSE` に従う。**本体 bot へのコード・重みのコピーは方針禁止**。
- ハーネスでの利用(subprocess 実行、アダプタ、検証スクリプト、仕様分析)は可。
- clys の `config.json` の重みを intetrigence の評価に移植しない。
- チューニングは tuning seeds のみ。held-out seeds では最適化しない。

---

## 11. S2 非対応の明記

clys は **TETR.IO Season 2 非対応**。以下は guideline のまま:

- 攻撃テーブル: `core/state.cpp` の guideline 実装(Single 0 / Double 1 / Triple 2 / Quad 4、
  T-spin は `clear * 2`、B2B +1、ren 加算)。
- コンボ: ren の加算テーブル(乗算なし)。
- パーフェクトクリア: guideline の +10。
- All-Spin / opener phase / ゴミキュー / 180°: なし。
- ゴミ: `start` に受信ゴミの情報がない(TBP の制約)。clys はゴミを見ない。

したがって clys とのマッチは **「S2 非対応ボット相手の汎化検証」** であり、
勝率の解釈にはこの旨を明記する(PLAN §1 の goal 条件 2)。

---

## 12. スモーク結果(2026-11-14)

レフェリー: `intetrigence-m2` の `round` ランナー(`c5d4b2d` + `bag_after_queue` + core の
`insert_garbage` 再スポーン修正 + clys のスポーン位置パッチ)。seed 9002、300 turn。

```text
seed-9002-sheet-A: winner=0 reason=TopOut turns=55 verified=true mismatches=0
seed-9002-sheet-B: winner=1 reason=TopOut turns=75 verified=true mismatches=0
round 0: clys 1-1-0 (gate 2) FAILED — gate failed: 1 wins / 2 games
```

| 項目 | sheet A | sheet B |
|---|---|---|
| 結果 | TopOut(intetrigence 勝ち) | TopOut(clys 勝ち) |
| turns | 55 | 75 |
| replay 検証 | OK(mismatches 0) | OK(mismatches 0) |
| clys の第 1 候補採用 | 55/55 | 75/75 |
| clys の think median / p95 | 113.1 / 134.2 ms | 116.3 / 131.1 ms |
| intetrigence の think median | 97.5 ms | 97.6 ms |

集計(2 ゲーム): attack/piece は intetrigence **0.969** vs clys **0.985**、
受信ゴミ/ゲームは intetrigence 25.5 vs clys 24.5、勝ちの種類は TopOut 2 / forfeit 0 / turn-cap 0。

- **forfeit 0、両ゲーム合法に完走**。ゲート(2 勝)は clys が 1 勝したため未達 = 強度の課題であり、
  接続・合法性の問題ではない。
- 以前の forfeit(exit 255 / `bot closed stdout`)は、clys のスポーン y=19 と S2 の y=21 のずれ、
  および `bag_state` が袋境界で空になる問題の 2 つ。両方修正済み。
- clys はフル 40 行視野で強く、intetrigence に対して 1-1。48/50 の達成は bot-dev の課題。
- エビデンス: `scripts/evidence/clys-smoke/{summary.json,audit/}`

## 13. 180° capability

- clys の movegen は `SRS_LUT` の **90° kick のみ**(`core/move.cpp` の `expand`)。**S2 180° の kick 遷移は無い**。
- したがって **`supports_180 = false`**。
- スモーク(seed 9001–9010、sheet A、1000 turn)では **forfeit 0 / no-contest 0 / unresolved 0**、verified 10/10。
  8 敗は TopOut(clys 勝ち)で、接続の問題ではない。clys はフル 40 行視野で強い。

### 12.1 10 ゲームスモーク(2026-11-15)

seed 9001–9010、sheet A、1000 turn、`--ms 100`/8t。

```text
summary: 10 games, wins 2 losses 8 unresolved 0 no-contest 0, verified 10/10
```

- **forfeit 0 / no-contest 0 / unresolved 0**。8 敗はすべて TopOut(clys 勝ち)。
- clys はフル 40 行視野で強く、現状の intetrigence では 2-8。強度の課題。
- エビデンス: `scripts/evidence/clys-10/{summary.json,audit/}`
- clys はネイティブの decision log を持たないため `measurement incomplete (opponent_log=false)` と表示される。
