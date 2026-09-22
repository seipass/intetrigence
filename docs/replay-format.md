# リプレイ形式と動画生成

## 結論

対戦成果物は次のカテゴリに分ける。

- `results/battles/<match-name>/`は新規の対戦セット。`report.json`、正本`replay.jsonl`、任意の`battle.mp4`を同じディレクトリに置く。
- `results/benchmarks/`と`results/comparisons/cold-clear2/`は既存の評価マニフェストをbasename単位で保管する。`foo.json`と`foo.jsonl`が同一収録の組である。
- `results/development/`は短時間の開発ログ、`results/archive/`は旧形式・終了済み成果物である。

新規のbattle成果物は次の形にする。

```text
results/battles/<match-name>/
  report.json    # 集計結果・条件
  replay.jsonl  # 正本。UTF-8、改行区切りJSON
  battle.mp4    # replay.jsonlから生成した派生動画（任意）
```

`replay.jsonl`が正本であり、JSONレポートとMP4は再生成できる派生物とする。ファイル名やディレクトリ名に日時を埋め込まず、条件を名前に含める。既存ログは移動しても内容を変更しない。

## JSONL共通規則

- UTF-8、LF改行、1行1オブジェクト。空行は禁止。
- すべてのレコードに文字列の`type`を持たせる。
- 数値の盤面座標・フレーム番号は整数。`null`は未設定を表す。
- `verify-replay --input FILE`を通過することを保存条件とする。
- 1ファイルに複数試合を連結できる。各試合は開始レコードと終了レコードを必ず持つ。

## 形式 v2: 配置リプレイ

`arena`、`match`、`external-arena`の配置単位のログ。レコード順は次の通り。

```text
start
move (player 0, player 1, ...)
result
```

### `start`

必須フィールド:

```json
{
  "type": "start",
  "seed": 101,
  "swapped": false,
  "turn_limit": 1000,
  "rules": "arena",
  "agents": ["intetrigence", "hoiko"]
}
```

`agents[0]`と`agents[1]`がプレイヤー番号と主体名の対応を固定する。`rules`を省略した旧ログは`arena`として検証される。

### `move`

`move`、`outcome`、`board`、`pending`を必須とする。現行ログでは次も記録する。

- `turn`, `player`, `agent`
- `active`, `hold_before`, `hold_after`, `next_before`, `next_after`
- `hold_used`
- `board_cells`: 40行×10列、下から上。セル値は`null`、`block`、`garbage`

`board`は後方互換用の列ビットマスクであり、表示・動画生成では`board_cells`を優先する。

### `result`

`result`には`seed`、`swapped`、`winner`、`turns`、`pieces`、`attack`、`lines`を含める。`winner`はプレイヤー番号または`null`。

## 形式 v3: フレーム入力リプレイ

`battle`の正本形式。SRS・HOLD・DAS/ARR・重力・ロック遅延を通った入力列を保存し、思考を再実行せずに経路を検証できる。

### `battle_start`

```json
{
  "type": "battle_start",
  "replay_version": 3,
  "seed": 1,
  "swapped": false,
  "piece_limit": 120,
  "ms": 0,
  "iterations": 30,
  "rules": "arena",
  "agents": ["intetrigence", "hoiko"],
  "clock_hz": 60,
  "movement": "frame_inputs",
  "movement_trace": "active_placement_after_each_input",
  "board_cells": "row_major_bottom_up; null|block|garbage",
  "board_colors": "row_major_bottom_up; null|I|O|T|L|J|S|Z|garbage",
  "initial": [
    {"agent":"intetrigence","active":"T","hold":null,"next":["I","O","S","Z","J"]},
    {"agent":"hoiko","active":"L","hold":null,"next":["O","T","S","Z","I"]}
  ]
}
```

`piece_limit`は各試合の終了上限、`ms`/`iterations`はAI探索条件、`rules`はrefereeのルールプロファイルである。`swapped`は左右入れ替えの有無で、入れ替えなしの評価では常に`false`にする。

### `battle_move`

1プレイヤーの1ミノがロックした時点のレコード。`inputs`と`movement_trace`は、そのロックまでの60Hz移動を記録する。

- `frame`: 60Hz絶対ロックフレーム。プレイヤーごとに単調増加。
- `player`, `agent`: 実行主体。
- `move`: 最終`Placement`（回転、位置、ミノ）。
- `inputs`: その配置へ到達する`BattleInput`列。
- `movement_frames`: `inputs`の要素数。`frame`差分と一致する。
- `movement_trace`: `inputs`各フレーム後のActive `Placement`と`phase`（`generation`/`active`/`locked`）。動画 rendererはこれを使って移動、回転、落下、ゴーストを描画する。
- `active`, `hold_used`, `hold_before`, `next_before`: 入力前の公開状態。
- `hold_after`, `next_after`: 固定後の公開状態。
- `outcome`: 消去、攻撃、相殺、適用Garbage、Perfect Clear、死亡。
- `board`: 後方互換用の列ビットマスク。
- `board_cells`: 固定後の40×10セル。行0が最下段、行39が最上段。
- `board_colors`: 任意の表示用40×10セル。`block`セルの実際のミノ種別（`I`/`O`/`T`/`L`/`J`/`S`/`Z`）を保持し、`garbage`は`garbage`、空セルは`null`。`board_cells`と同じ行・列順で、対戦判定の正本は`board_cells`とする。
- `stats`: 探索統計。検証には使わず、診断表示専用。

同じ`frame`の2レコードは同時ロックである。検証器は両方の着手を適用してから攻撃を交換する。`inputs`は検証器が再生し、`movement_trace`は同じ検証器から生成される。`move`だけを直接適用してはならない。

### `battle_result`

```json
{
  "type": "battle_result",
  "result": {
    "seed": 1,
    "swapped": false,
    "winner": 0,
    "frames": 2048,
    "pieces": [94,106],
    "attack": [48,35],
    "lines": [54,52]
  },
  "dead": [false,true],
  "winner_agent": "intetrigence"
}
```

`winner`と`winner_agent`は引き分け時に`null`。試合が途中で切れているファイルは保存済みリプレイとみなさない。

## 動画生成

動画はv3 JSONLのフレーム入力を正本から生成する。`movement_trace`がある場合、AIを再実行せず、60HzのActive位置、SRS回転、重力、ゴーストを再生する。ロック時だけ`board_colors`へ切り替えるため、盤面の線消去・Garbage投入も正本のスナップショットと一致する。古いv3レコードに`movement_trace`がない場合は、後方互換のロック後スナップショットへフォールバックする。

`battle_start.rules`を画面に表示し、`guideline`の場合はGuidelineプロファイルとして表示する。Active/HOLD/NEXTと`board_colors`のロック済みミノにはI=cyan、O=yellow、T=purple、S=green、Z=red、J=blue、L=orangeの標準的な7種色を使い、盤面の`garbage`は別色で描画する。移動中はActiveとゴーストを同じミノ色の濃淡で描画する。公式ロゴ、音楽、商標素材は含めない。見た目の色は表示規約であり、ゲームルールの認証を意味しない。

必要条件:

- Python 3.10以降（標準ライブラリのみ）
- `ffmpeg`（`mpeg4`映像エンコーダー）

コマンド:

```sh
python3 scripts/render_battle_replay.py \
  --input results/development/replay-video-test/guideline-movement-game-001.jsonl \
  --output results/development/replay-video-test/guideline-movement-game-001.mp4
```

100試合の動画を作る場合は、同じスクリプトに正本の`replay.jsonl`を指定する。同じ入力、同じ`--fps`、同じffmpeg実装なら同じイベント列から同じ順序の動画を生成する。デフォルトは20fps・1280×720・音声なし。動画は必須成果物ではなく、`replay.jsonl`と`report.json`を先に保存する。

## 検証手順

```sh
./target/release/intetrigence verify-replay \
  --input results/battles/intetrigence-vs-hoiko-100-no-swap/replay.jsonl

python3 scripts/render_battle_replay.py \
  --input results/battles/intetrigence-vs-hoiko-100-no-swap/replay.jsonl \
  --output /tmp/battle.mp4
```

検証が成功した後にのみ動画を配布する。v2とv3を同じ`verify-replay`入口で受理するが、動画生成スクリプトはフレーム入力を持つv3だけを対象にする。
