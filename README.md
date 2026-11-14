# intetrigence

TETR.IO Season 2(TETRA LEAGUE デフォルト相当)ルールで動く versus Tetris AI と、その計測基盤。

- **ルールエンジン** — S2 の攻撃テーブル、B2B Charging / surge、All-Mini+、Opener Phase、Multiplier コンボ、ゴミキュー(遅延・cap・相殺)
- **レフェリー** — ペアドターン、両シート、replay 検証、思考時間記録、採点オーディット
- **ボット** — exact movegen(180° 含む)+ anytime beam search + S2 評価関数
- **TBP host** — attax / Cold Clear 2 を外部ボットとして接続、clys は専用アダプタ経由

## 目標

attax(現行公開版)に **50 勝 0 敗**、Cold Clear 2 と clys に **48 勝以上 / 50 ゲーム**。
勝数は **TopOut と TurnCap の決着のみ**を数える(forfeit は勝ちに含めない)。
評価 seeds は実行前にコミットで確定し、チューニングには使わない。

詳細は [PLAN.md](PLAN.md)。

## 現在の測定値

**パッチ適用 attax(40 行視野、SHA-256 `e22aab7b…`)とのスモーク 10 ゲーム: 8 勝 2 敗、forfeit 0、replay 10/10 検証。**

- 敗因は 2 件とも TopOut(スタック管理)。forfeit ではない
- attax の探索は正常(nodes ≈ 10,000、depth 6、35 ms/手)

### 過去の数字について

以前に報告した「attax に 94%」「M4 ゲート 95%」は **無効** です。原因は 3 つの計測バグでした:

1. `classify_forfeit` の判定が tautology で、forfeit が常に「vision」に分類されていた
2. attax アダプタが active ピースをグリッド最上段に置き、レフェリーの S2 spawn と 18 行ずれていた
3. upstream attax の視野は 20 行で、S2 の spawn(row 21)が可視領域の外だった

旧 220 ゲームの真の内訳は **TopOut 勝ち 0 / TurnCap 勝ち 13 / TopOut 負け 13 / forfeit 194** で、実質勝率 5.9% でした。
詳細と無効化の記録は [docs/seed-registry.md](docs/seed-registry.md) §5。

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
