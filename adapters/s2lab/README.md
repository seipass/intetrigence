# Attax 対戦を s2-bot-lab で計測する

## 準備

1. [Attax](https://github.com/souta-lab/attax) の GPU 版 (`main` / `b932b82449f97a187042beb015c1413a4e16c0bc`, `bash gpu/build.sh` で `gpu/vkrunner_s26` をビルド) または歴史再現用の Rust CPU 版 (`c83d3397d458f4af9cfd6fe3ba3468df48bb2877`, `cargo build --release --locked --manifest-path rust/Cargo.toml -p attax-search-cli`) をクリーンな checkout に用意する。
2. [s2-bot-lab の参照コミット](https://github.com/61bi-234469/s2-bot-lab/tree/79505ca93bd15f7bcbd58724cb2a6ba312e09434) を checkout し、`npm ci` を実行する。計測は referee のソースハッシュとプロファイルのハッシュを記録する。
3. このリポジトリで `cargo build --release --locked -p intetrigence-arena --bin s2lab_bridge` を実行する。
4. `ATTAX_DIR` と `S2_LAB_ROOT` にそれぞれの checkout を設定する。ビルド出力先やバイナリを変える場合は `ATTAX_GPU_RUNNER` / `ATTAX_GPU_DEVICE` / `ATTAX_RUST_CLI` / `S2LAB_BRIDGE` を設定し、`--attax-engine <auto|gpu|rust>` で選択する（Linux Mesa RADV では `gpu/host/vkrunner_s26.c` の `barrier_rw` に `VK_PIPELINE_STAGE_DRAW_INDIRECT_BIT` と `VK_ACCESS_INDIRECT_COMMAND_READ_BIT` を補ったランナーを `ATTAX_GPU_RUNNER` で指定しつつ、`ATTAX_DIR` 自体はクリーンな checkout を維持する）。

## 実行

```sh
bun adapters/s2lab/run-match.mjs --games 1 --seed 9001 --max-locks 60 --out out/lab-smoke.jsonl
bun adapters/s2lab/run-match.mjs --verify --seed 9001 --max-locks 60 --out out/lab-smoke.jsonl
bun adapters/s2lab/run-match.mjs --first-to 50 --seed "${EVALUATION_SEED:?set an unused seed}" --out out/lab-series.jsonl
bun adapters/s2lab/run-match.mjs --evaluation --games 100 --seed "${EVALUATION_SEED:?set an unused seed}" --out out/lab-evaluation.jsonl
bun adapters/s2lab/run-match.mjs --evaluation --games 100 --verify --seed "${EVALUATION_SEED:?set an unused seed}" --out out/lab-evaluation.jsonl
```

`out` を実行前に作成する。新しいシリーズでは [シード台帳](../../docs/seed-registry.md)を確認して `EVALUATION_SEED` に未使用の開始シードを指定する。各試合でシードを1進め、席を交互にする。`--first-to` は各局の開始前に共有台帳へシードを記録する。席だけを変えた同じ seed は対戦標本を増やさない。

`--evaluation` は開始前に100局分のシードと対戦設定を共有台帳に登録する。再検証には100局すべての記録と、実行時と同じ設定・バイナリが必要。勝率は Intetrigence の勝利数を予定局数100で割る。

対戦時は `arena match` と `round run` と同じ計測ロックを取得する。別の checkout が計測中なら、出力ファイルを作る前に停止する。 [ロックの保存場所と復旧手順](../../scripts/evidence/README.md#measurement-lock)

出力は新規ファイルに1局1行。各局には、両ボットの実行バイナリのSHA-256、探索設定、設置ごとの公開状態・選択手・採用手・採点を保存する。中断後は同じ引数とバイナリで `--resume` を追加する。検証済みの記録から得点を再計算し、次のシードから再開する。`--verify` はボットに判断をやり直させず、記録した手を Lab レフェリーに再適用して勝敗を確かめる。

Attax は幅96、Φオン、solvencyオフ、GPU エンジン（config O, `--attax-engine gpu` / `auto`）または `--attax-engine rust` で `c83d339` の常駐サーバーを使用する。GPU 版では `acceptRoot` が候補への事後フィルタとなるため、全生存ルートが Lab の I キック検証で弾かれた対局（`attax found no move before top-out`）はデータ上の `error` レコードとして記録・検証し、対局列は継続する。自機は決定論の d12 w96、8スレッド、`SplitBurst` (`S2LAB_PROJECT_GARBAGE=2`)、`S2LAB_ATTACK_PROFILE=6` を既定とする。Lab の設定は paced 1 PPS、公開キューは current を含む14個、ゴミ配達の追加遅延は0フレーム。思考時間は診断として記録する。この計測は Lab の配置ルールでの対戦成績を測る。
