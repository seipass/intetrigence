# intetrigence

TETR.IO Season 2(TETRA LEAGUE デフォルト相当)ルールで動く versus Tetris AI と、その計測基盤。

- **ルールエンジン** — S2 の攻撃テーブル、B2B Charging / surge、All-Mini+、Opener Phase、Multiplier コンボ、ゴミキュー(遅延・cap・相殺)
- **レフェリー** — ペアドターン、両シート、replay 検証、思考時間記録、採点オーディット
- **ボット** — exact movegen(180° 含む)+ anytime beam search + S2 評価関数
- **TBP host** — attax / Cold Clear 2 を外部ボットとして接続、clys は専用アダプタ経由

## 目標(段階制)

弱い相手から順に、通過したら次へ進む。

| 段階 | 相手 | 条件 |
|---|---|---|
| 1 | **Cold Clear 2** | **50 勝 0 敗** |
| 2 | **clys** | **50 勝 0 敗** |
| 3 | **attax**(40 行パッチ適用) | **48 勝以上 / 50** |

**勝ちは TopOut 決着のみ。** TurnCap(1000 turn 到達)は**勝者を決めない未決着**として扱い、1 件でも出ればゲート不通過。
forfeit も勝ちに含めない。時間切れで攻撃量が多い方を勝ちにする規則は存在しないので、実装もしない。
評価 seeds は実行前にコミットで確定し、チューニングには使わない。

50 勝 0 敗は「平均勝率」ではなく **per-game 99% 以上**を要求する(0.95^50 = 7.7%、0.99^50 = 60.5%)。詳細は [PLAN.md](PLAN.md)。

## 現在の測定値

**チューニング用 seeds(9001–9010)での測定。評価用 seeds は未使用。**

| 相手 | ゲーム | TopOut 勝ち | 備考 |
|---|---|---|---|
| **Cold Clear 2** | 20 | **17–19** | 構成により変動。`docs/` の各測定を参照 |
| **clys** | 10 | **1–2** | 未改善 |
| **attax**(40 行パッチ) | 18 | **2** | 11.1% |

### この数字をどう読むか(重要)

**「TBP 経由で接続した相手に勝った数」であり、「フル強度の相手に勝った数」ではありません。** 次の 3 点で条件が対称ではありません(`docs/fairness-audit.md`)。

1. **情報**: intetrigence は自分の S2 状態(`pending_garbage` / 正確な `b2b_streak` / `sent_total` / `received_total`)を受け取ります。TBP は `back_to_back` の bool しか運びません。**TETR.IO の実プレイヤーは自分のゴミキューを見られる**ので、我々の側が現実に近く、外部ボットがハーネスの制約で不利という構造です
2. **計算資源**: intetrigence は 8 スレッド、外部ボットは実装上のシングルスレッド
3. **S2 への対応**: 相手は 3 者とも S2 非対応(B2B Charging / surge / All-Mini+ / Opener Phase を知らない)。**これは意図した比較**(S2 を実装したボットが S2 非対応のボットに勝てるか)です

**同一条件での比較ではありません。** 条件を揃えた測定は行っていません。

### 探索の重大な欠陥を修正した(2026-11-15)

それ以前の全測定は、**anytime 探索が期限超過で反復を捨てて 1-ply greedy にフォールバックしていた**状態の数字だった。修正後、実戦の探索深度は depth 2 → **4.4** になり、greedy フォールバックは 0 になった。

**修正前の重みは「探索が動かない前提」で校正されていた**ため、探索を直した直後は勝率が下がった(CC2 16-4 → 7-13)。評価関数を再構成(非線形 danger、レース項、コンボ項、attack 増)して **14-6** まで戻した。

### 参考: attax スモーク(40 行パッチ適用、seeds 11001–11005 × 両シート)

| 内訳 | 件数 |
|---|---|
| **TopOut 勝ち** | **0** |
| **TopOut 負け** | **9** |
| **未決着(ターン上限)** | **1** |
| forfeit(no contest) | 0 |

- replay 検証 10/10
- 盤面の最大高さ: intetrigence 20–23 / attax 10–19(attax はほぼ崩れない)
- attack/piece: intetrigence 0.50–0.98 / attax 0.71–0.96
- 思考時間 median: intetrigence 97.1–97.4 ms / attax 28.1–38.5 ms

**つまり、まともな 40 行 attax に対して TopOut で 1 本も勝てていません。** 現在のボットは目標(50 勝 0 敗)から遠く、強化が必要です。

### 過去の数字について

以前に報告した「attax に 94%」「M4 ゲート 95%」「スモーク 8 勝 2 敗」は **すべて無効** です。原因は 5 つの計測バグでした:

1. `classify_forfeit` の判定が tautology で、forfeit が常に「vision」に分類されていた
2. attax アダプタが active ピースをグリッド最上段に置き、レフェリーの S2 spawn と 18 行ずれていた
3. upstream attax の視野は 20 行で、S2 の spawn(row 21)が可視領域の外だった
4. **ターン上限で「攻撃量が多い方の勝ち」にしていた**(TETR.IO にそんなルールはない)
5. **0 ライン設置に直前の combo 乗数が乗り、攻撃が発生していた**

旧 220 ゲームの真の内訳は **TopOut 勝ち 0 / TurnCap 勝ち 13 / TopOut 負け 13 / forfeit 194** で、実質勝率 5.9% でした。
詳細と無効化の記録は [docs/seed-registry.md](docs/seed-registry.md) §5、監査は [docs/s2-rule-audit.md](docs/s2-rule-audit.md) と [docs/core-rule-audit.md](docs/core-rule-audit.md)。

## 構成

```
crates/
  core/   # 純粋 S2 ルールエンジン(bitboard, SRS+, 180°, 7-bag, 採点, ゴミキュー)
  bot/    # exact movegen / anytime 探索 / S2 評価関数 / 重み
  arena/  # S2 レフェリー / replay 検証 / 監査 / ラウンドランナー / 動画レンダラ
  tbp/    # Tetris Bot Protocol メッセージ I/F と host
adapters/ # 外部ボット(attax / CC2 / clys)の互換パッチとアダプタ
docs/     # S2 仕様、レフェリープロトコル、統合手順、計測レポート
scripts/  # ベンチ、スモーク、ラウンド実行、動画生成
```

`crates/core` は決定論的・純粋(seed + 入力列 → 状態)で、replay・ベンチ・実戦が同一コードを共有します。

## ビルドとテスト

```sh
cargo build --release
cargo test --workspace
```

## 対戦の実行

```sh
# 単発マッチ(replay 検証・監査ログ付き)
cargo run --release -p intetrigence-arena -- match \
  --seeds 9001-9010 --sides both --out scripts/evidence/my-run \
  --attax-dir ../attax

# 段階ゲート付きラウンド(attax 50 → CC2 50 → clys 50)
bash scripts/run-round.sh

# リプレイを動画にする
cargo run --release -p intetrigence-arena -- render \
  --replay scripts/evidence/my-run/replays/seed-9001-sheet-A.jsonl \
  --out videos/seed-9001-sheet-A.mp4
```

外部ボットは TBP(標準入出力の JSON 行)で接続します。attax は `adapters/attax-vision-40.patch`、
CC2 は `adapters/cc2-tbp-compat.patch`、clys は `adapters/clys-tbp-spawn.patch` + `adapters/clys/tbp/adapter.ts` を適用します。
いずれも**視野・spawn 位置のプロトコル互換修正のみ**で、探索・評価・重みには触れません。

## ドキュメント

| 文書 | 内容 |
|---|---|
| [PLAN.md](PLAN.md) | 計画、確定事項、進捗ログ |
| [docs/s2-spec.md](docs/s2-spec.md) | S2 ルール仕様(凍結版) |
| [docs/s2-referee-protocol.md](docs/s2-referee-protocol.md) | マッチ運用プロトコル |
| [docs/seed-registry.md](docs/seed-registry.md) | seed の使用履歴と無効化された測定 |
| [docs/attax-integration.md](docs/attax-integration.md) | attax の接続仕様とパッチ |
| [docs/cc2-integration.md](docs/cc2-integration.md) | Cold Clear 2 の接続 |
| [docs/clys-integration.md](docs/clys-integration.md) | clys の接続とアダプタ |
| [docs/attax-analysis.md](docs/attax-analysis.md) | attax の探索・評価の分析 |

## ライセンス

**AGPL-3.0-only**。[LICENSE](LICENSE) を参照。

AGPL-3.0 は GPL 系のコピーレフトライセンスです(GPL-3.0 との互換性を保つため、AGPL を選択しています)。
attax は AGPL-3.0 で公開されており、本リポジトリは作者の許可を得て、その接続仕様の調査と
互換パッチの作成を行っています。attax 本体のコードは含みません。
