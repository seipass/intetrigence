# intetrigence

TETR.IO Season 2(TETRA LEAGUE デフォルト相当)ルールで動作する versus Tetris AI。
attax との公式マッチ(held-out seeds 50 × 両シート = 100 ゲーム)で 90 勝以上を達成することを目標とする。

- 計画・確定事項: [PLAN.md](PLAN.md)
- S2 ルール仕様: [docs/s2-spec.md](docs/s2-spec.md)(v1.3.1 凍結済み。§11 の U 項目は決定済み)

## 構成

```
crates/
  core/   # 純粋 S2 ルールエンジン(bitboard, SRS+, 7-bag, hold, 採点, ゴミキュー)
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

- Rust workspace と `crates/core` の S2 エンジンを実装済み
  - 7 種 × 4 回転の SRS true rotation 定義(回転中心を明示)
  - 10 列 × 40 行 bitboard(可視 20 行、ゴミセル管理)
  - SRS+ kick(JLSTZ / I)と 180° kick。`crates/core/src/kick.rs` に隔離し、`docs/s2-spec.md` §3.1/§3.2 に基づく。U1 は v1.3 で S7 順に決定
  - All-Mini+ スピン分類(`Board::spin_type` / `t_spin_corners` / `is_immobile`)
  - 7-bag(MINSTD + Fisher-Yates)、U7 の `derive_seeds`、hold、左右移動・ソフトドロップ・回転(CW/CCW/180)・ハードドロップ・ロック
  - ゴミ行挿入(穴位置指定、上端破棄)とライン消去
  - 攻撃採点(base / combo / B2B Charging / surge / PC / GSB)、opener phase、ゴミキュー(cap 8・遅延 20 frame・相殺・change-on-attack)
- `docs/s2-attack-examples.json` の 18 fixture と `docs/s2-rng-derivation.json` の 4 fixture をテストで照合済み

## ライセンス

AGPL-3.0-only。[LICENSE](LICENSE) を参照。
