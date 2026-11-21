# intetrigence

TETR.IO Season 2 を扱う対戦テトリス AI。Rust のルールエンジンと探索、TBP 対戦ハーネス、s2-bot-lab への接続を含みます。

## ビルドとテスト

```sh
cargo build --workspace --release --locked
cargo test --workspace --release --locked
```

## 対戦を実行する

- **Attax** は [s2-bot-lab の手順](adapters/s2lab/README.md)で Rust 版 `c83d339` に接続します。公開 S2 状態・キュー長・Lab の採点規則をそろえ、ロック単位で手と結果を記録します。
- Attax の s2lab 記録は `arena render-s2lab --games <games.jsonl> --out <video.mp4>` で動画にできます。100局を30fps/480pで連結した例は [Attax との100局リプレイ](docs/attax-100-replay.md) にあります。
- **Cold Clear 2** と **clys** は、[CC2](docs/cc2-integration.md)と[clys](docs/clys-integration.md)の統合手順でビルドして TBP で接続します。それぞれの互換パッチと計測条件は対戦結果に併記します。
- **ローカル疎通**は外部ボットのセットアップなしで確認できます。

```sh
cargo run --release -p intetrigence-arena --bin arena -- match --no-attax --seeds 9001,9002 --sheets A,B --turns 40 --out out/local-smoke
cargo run --release -p intetrigence-arena --bin arena -- verify out/local-smoke
cargo run --release -p intetrigence-arena --bin arena -- render --replay out/local-smoke/replays/seed-9001-sheet-A.jsonl --out out/local-smoke/seed-9001-A.mp4
```

動画には FFmpeg が必要です。既存の出力を使う場合は、同一の計測条件で `--resume` を指定します。

## 構成

| ディレクトリ | 内容 |
| --- | --- |
| `crates/core/` | 盤面、回転、7-bag、S2 得点、ゴミキュー |
| `crates/bot/` | 到達可能手の列挙、ビーム探索、盤面評価 |
| `crates/arena/` | レフェリー、対戦、採点監査、リプレイ、動画 |
| `crates/tbp/` | Tetris Bot Protocol のホスト |
| `adapters/` | 外部ボット接続と互換パッチ |

[現行計画](PLAN.md) · [S2 仕様](docs/s2-spec.md) · [Rust レフェリー](docs/arena.md) · [シード台帳](docs/seed-registry.md) · [リポジトリ監査](docs/repository-audit.md)

## ライセンス

**AGPL-3.0-only**。[LICENSE](LICENSE) を参照。

AGPL-3.0 は GPL 系のコピーレフトライセンスです(GPL-3.0 との互換性を保つため、AGPL を選択しています)。
attax は AGPL-3.0 で公開されており、本リポジトリは作者の許可を得て、その接続仕様の調査と
互換パッチの作成を行っています。attax 本体のコードは含みません。
