# intetrigence

TETR.IO Season 2(TETRA LEAGUE デフォルト相当)ルールで動作する versus Tetris AI。
attax との公式マッチ(held-out seeds 50 × 両シート = 100 ゲーム)で 90 勝以上を達成することを目標とする。

- 計画・確定事項: [PLAN.md](PLAN.md)
- S2 ルール仕様: [docs/s2-spec.md](docs/s2-spec.md)(v1 凍結済み。攻撃採点・surge・opener・ゴミキューは次の実装フェーズ)

## 構成

```
crates/
  core/   # 純粋 S2 ルールエンジン(bitboard, SRS kick, 7-bag, hold, 基本操作)
  bot/    # exact movegen / anytime 探索 / S2 評価関数(後続担当)
  arena/  # S2 レフェリー / TBP host / replay 検証 / 監査ログ(後続担当)
  tbp/    # TBP メッセージ I/F(後続担当)
```

`crates/core` は決定論的・純粋(seed + 入力列 → 状態)であり、replay・ベンチ・実戦で同一コードを共有する。

## ビルドとテスト

```sh
cargo build --release
cargo test --workspace
```

## 現在の状態

- Rust workspace と `crates/core` の基盤を実装済み
  - 7 種 × 4 回転の SRS true rotation 定義(回転中心を明示)
  - 10 列 × 40 行 bitboard(可視 20 行、ゴミセル管理)
  - SRS+ kick テーブル(JLSTZ / I)と 180° kick。`crates/core/src/kick.rs` に隔離し、`docs/s2-spec.md` §3.1/§3.2 に基づく。SRS+ I のテスト順(U1)は差し替え可能
  - 7-bag(seed 指定で決定論的)、hold、左右移動・ソフトドロップ・回転(CW/CCW/180)・ハードドロップ・ロック
  - ゴミ行挿入(穴位置指定)とライン消去
- 攻撃採点・surge・opener phase・ゴミキューは型と TODO のみ(次の実装フェーズ)

## ライセンス

AGPL-3.0-only。[LICENSE](LICENSE) を参照。
