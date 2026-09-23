# Intetrigence

対戦テトリスAIと、その評価・対戦・リプレイ検証を行うためのRust製ツール群です。

Cold Clear 2を基盤とした派生エンジンに加え、Hoiko互換バックエンド、外部AIとの対戦、60 Hz入力を使う対戦処理、リプレイ検証を含みます。

比較条件、測定結果、公開仕様との対応状況は [`docs/validation.md`](docs/validation.md) にまとめています。

## ビルド

```sh
cargo build --release
```

テスト:

```sh
cargo test
cargo test --manifest-path engine/Cargo.toml --lib
```

Cold Clear 2もビルドする場合:

```sh
cargo build --release --manifest-path vendor/cold-clear-2/Cargo.toml
```

## 主なコマンド

### arena

Intetrigence同士を固定条件で対戦させます。

```sh
./target/release/intetrigence arena \
  --games 20 \
  --seed 101 \
  --ms 20 \
  --pps 40 \
  --turns 1000
```

公開マルチプレイヤールールに寄せた設定を使う場合:

```sh
./target/release/intetrigence arena \
  --rules guideline \
  --games 20 \
  --seed 101 \
  --ms 20 \
  --pps 40 \
  --turns 1000
```

### external-arena

外部のTBP対応AIと対戦します。

```sh
./target/release/intetrigence external-arena \
  --opponent vendor/cold-clear-2/target/release/cold-clear-2 \
  --games 20 \
  --seed 101 \
  --ms 20 \
  --pps 40 \
  --turns 1000
```

### match

`intetrigence`、`hoiko`、`cold_clear_2` から対戦相手を指定します。

```sh
./target/release/intetrigence match \
  --p1 hoiko \
  --p2 intetrigence \
  --games 20 \
  --ms 20 \
  --pps 40 \
  --turns 1000
```

`cold_clear_2` を指定する場合は `--opponent` で実行ファイルを渡します。

```sh
./target/release/intetrigence match \
  --p1 hoiko \
  --p2 cold_clear_2 \
  --opponent vendor/cold-clear-2/target/release/cold-clear-2 \
  --games 20 \
  --ms 20 \
  --pps 40 \
  --turns 1000
```

同じseedについて左右を交換した試合も実行します。`--no-swap` を指定すると左右交換を無効にできます。

### battle

60 Hzの入力処理を使って対戦します。

AIが選んだ最終配置までの左右移動、回転、落下、ロックを入力列として実行し、両プレイヤーのゲーム時間を個別に進めます。

```sh
./target/release/intetrigence battle \
  --p1 hoiko \
  --p2 intetrigence \
  --games 20 \
  --ms 20 \
  --turns 1000
```

このモードでは `--pps` は使いません。

### tbp

TBP対応AIとして起動します。

```sh
./target/release/intetrigence tbp --ms 20
```

HoikoバックエンドをTBPで使う場合:

```sh
./target/release/intetrigence hoiko-tbp \
  --ms 400 \
  --hoiko-config /path/to/Hoiko_PPT_v0-beta1
```

### verify-replay

保存した対局を再実行して、盤面、攻撃、相殺、終局結果を検証します。

```sh
./target/release/intetrigence verify-replay \
  --input results/development/arena-development.jsonl
```

`battle` のリプレイでは、各フレームの入力経路も再実行します。

## 探索条件

`--ms` は探索時間の目安です。初期化や最後の展開処理などにより、実測時間が指定値を超える場合があります。

実時間指定では、Intetrigenceは既定で最大8ワーカーを使用します。ワーカー数を固定する場合:

```sh
INTETRIGENCE_SEARCH_WORKERS=1 ./target/release/intetrigence arena ...
```

開発時に探索量を固定したい場合は `--iterations` を使えます。

```sh
./target/release/intetrigence match \
  --p1 hoiko \
  --p2 intetrigence \
  --games 2 \
  --ms 0 \
  --iterations 200
```

探索内部に乱数を含むため、同じseedと試行回数を指定しても対局が完全に一致するとは限りません。

## Hoikoバックエンド

Hoikoバックエンドは、配布実行ファイルと設定、および公開されている [HoikoCode20230120](https://github.com/ultimacrown/HoikoCode20230120) を参照して作成した独立実装です。

配置候補の生成、探索、評価設定、NEXT予測、Combo/B2B状態、各種定型手順などを実装しています。PPTプロセスの読出しや実コントローラへの入力は含みません。

実装範囲と調査内容は [`docs/hoiko-reverse-engineering.md`](docs/hoiko-reverse-engineering.md) を参照してください。

## リプレイ

通常のターン制対戦と、`battle` のフレーム入力対戦では記録形式が異なります。

リプレイには開始盤面、Active、Hold、Next、各手の配置、攻撃、盤面遷移などを保存します。`battle` ではフレーム番号と入力列も保存します。

形式の詳細と動画生成手順は [`docs/replay-format.md`](docs/replay-format.md) にあります。

## 検証

記録済みの実行ファイル、設定、主要リプレイを確認するスクリプトがあります。

```sh
./scripts/reproduce_validation.sh
```

長時間の比較試験も再実行する場合:

```sh
RUN_BENCHMARKS=1 ./scripts/reproduce_validation.sh
```

比較条件、結果、既知の差異、公開仕様との対応状況は [`docs/validation.md`](docs/validation.md) を参照してください。

## battle.tet接続

`battle.tet Bot Protocol v1` 用の接続機能は任意機能として分離しています。

TLS接続を使う場合:

```sh
cargo build --release --features battle-wss
```

起動例:

```sh
./target/release/intetrigence battle-bot \
  --url 'wss://example.com/ws' \
  --match-id '<match-id>' \
  --token '<ai-token>' \
  --role ai \
  --ms 400
```

通常の `cargo build` と `cargo test` では、この接続機能は有効になりません。

## ディレクトリ構成

- `src/` — コマンド、対戦処理、接続処理
- `engine/` — Intetrigenceの探索エンジン
- `vendor/cold-clear-2/` — 比較用に固定したCold Clear 2
- `benchmarks/` — 固定条件の評価設定
- `results/` — 対戦結果とリプレイ
- `docs/` — 検証、リプレイ形式、Hoiko調査資料
- `scripts/` — 再現確認用スクリプト
- `tests/` — 統合テスト

## Cold Clear 2との関係

比較対象は [MinusKelvin/cold-clear-2](https://github.com/MinusKelvin/cold-clear-2) の固定リビジョンです。

`vendor/cold-clear-2` は比較用のソースを保持し、公開API化とビルド互換性に必要な変更を含みます。`engine` はそこから派生したIntetrigence側の実装です。

元プロジェクト由来のコードには、それぞれのMIT / Apache-2.0ライセンスを保持しています。

## 資料

- [`docs/validation.md`](docs/validation.md) — 比較条件、結果、仕様対応、既知の制約
- [`docs/replay-format.md`](docs/replay-format.md) — リプレイ形式と再生手順
- [`docs/hoiko-reverse-engineering.md`](docs/hoiko-reverse-engineering.md) — Hoikoバックエンドの調査内容と実装範囲
