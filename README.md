# Intetrigence

Rust製の対戦テトリスAIと評価環境。Cold Clear 2を基盤とする派生実装です。**名前付き設定のない無制限な「最強」への優位性は未検証です。**

```sh
cargo test
cargo test --manifest-path engine/Cargo.toml --lib
cargo build --release
cargo build --release --target-dir target/strict
cargo build --release --manifest-path vendor/cold-clear-2/Cargo.toml
./target/release/intetrigence arena --games 20 --seed 101 --ms 20 --pps 40 --turns 1000 \
  --output results/development/arena-development.json --replay results/development/arena-development.jsonl
./target/release/intetrigence arena --rules guideline --games 20 --seed 101 --ms 20 --pps 40 --turns 1000 \
  --output results/development/guideline-development.json --replay results/development/guideline-development.jsonl
./target/release/intetrigence external-arena \
  --opponent vendor/cold-clear-2/target/release/cold-clear-2 \
  --games 2 --seed 101 --ms 20 --pps 40 --turns 1000
./target/strict/release/intetrigence external-arena --strict-external-time \
  --opponent vendor/cold-clear-2/target/release/cold-clear-2 \
  --games 2 --seed 101 --ms 20 --pps 40 --turns 1000
./target/release/intetrigence verify-replay --input results/development/arena-development.jsonl
./target/release/intetrigence tbp --ms 20
# Hoiko reverse-engineered backend (embedded profiles; optional extracted package)
./target/release/intetrigence hoiko-selfplay --pieces 100 --ms 20
./target/release/intetrigence hoiko-arena --games 20 --ms 20 --pps 40 --turns 1000
./target/release/intetrigence hoiko-tbp --ms 400 --hoiko-config /path/to/Hoiko_PPT_v0-beta1
# Quick paired Hoiko vs Intetrigence development match with fixed work.
./target/release/intetrigence match --p1 hoiko --p2 intetrigence \
  --games 2 --ms 0 --iterations 200 --turns 1000 \
  --output results/battles/hoiko-vs-intetrigence.json \
  --replay results/battles/hoiko-vs-intetrigence.jsonl
# Frame-driven battle: both active pieces move independently at 60 Hz.
./target/release/intetrigence battle --p1 hoiko --p2 intetrigence \
  --games 2 --ms 0 --iterations 200 --turns 1000 \
  --output results/battles/hoiko-vs-intetrigence-battle.json \
  --replay results/battles/hoiko-vs-intetrigence-battle.jsonl
# Any pair can be selected; cold_clear_2 is the external TBP executable.
./target/release/intetrigence match \
  --p1 hoiko --p2 cold_clear_2 \
  --opponent vendor/cold-clear-2/target/release/cold-clear-2 \
  --games 20 --ms 20 --pps 40 --turns 1000
# Swap the named first participant, or use intetrigence instead of Hoiko.
./target/release/intetrigence match --p1 cold_clear_2 --p2 hoiko \
  --opponent vendor/cold-clear-2/target/release/cold-clear-2 --games 20 --ms 20
# Optional battle.tet Bot Protocol v1 adapter (the server supplies the authoritative state)
cargo build --release --features battle-wss
./target/release/intetrigence battle-bot \
  --url 'wss://example.com/ws' --match-id '<match-id>' --token '<ai-token>' --role ai \
  --ms 400
# 固定バイナリ・設定のSHA-256と主要リプレイを検証
./scripts/reproduce_validation.sh
```

`--iterations N` は探索の試行回数を固定する開発用オプションです。探索内部には乱数があるため、同一seed・試行数でも対局は完全一致しません。`verify-replay` は記録済みの着手を適用し、盤面・攻撃・相殺・終局結果の一致を検査します。movement battleでは、記録された各60 Hz入力もGeneration Phase、重力、SRS、ロック遅延に対して再実行します。配置レベルのSRS参照検証と、公開Guidelineのレベル別落下速度・20倍ソフトドロップ・0.2秒Generation Phase・DAS/ARR入力変換を含むフレーム検証器のテストは`cargo test`で実行できます。

探索は実時間予算では利用可能なCPUから最大8ワーカーを既定で使います。`INTETRIGENCE_SEARCH_WORKERS=1`のように指定するとワーカー数を固定できます。固定試行数（`--iterations`）では探索結果を再現しやすくするため1ワーカーに固定します。

`match` は `--p1` と `--p2` に `hoiko`、`intetrigence`、`cold_clear_2` を指定する共通ランチャーです。各seedは左右を入れ替えた2試合として実行されるため、`match --p1 hoiko --p2 cold_clear_2` と `match --p1 cold_clear_2 --p2 hoiko` は同じ対戦を視点ごとに記録できます。`cold_clear_2` は `--opponent` で指定したTBP実行ファイルを使い、1試合につき外部プロセス1つを起動します。`--strict-external-time`、`--rules guideline`、`--replay`、`--output`、`--hoiko-config`も利用できます。同じ主体同士、またはCold Clear 2を2体同時に指定する対戦はまだ受け付けません。

`battle` は瞬間配置を使わない非同期対戦です。各AIの最終配置を、60 HzのGeneration Phase、左右入力、CW/CCW回転、SRSキック、20倍Soft Drop、重力、Hard Drop、500msロック遅延、最大15回のリセットを通る入力列へ変換します。離散入力間には解放フレームを要求し、両プレイヤーのロック予定時刻を独立して進めるため、操作量が少ない側は相手を待たず次のミノへ進みます。同一フレームのロックは両方を確定してから攻撃を交換します。探索の実測CPU時間はゲーム時計に含めません。`--pps`は受け付けず、`--no-swap`で左右交換を無効化できます。三つのAIの任意の組み合わせと同じAI同士を利用でき、Cold Clear 2を含む場合は通常のTBP制約どおり正の`--ms`が必要です。

Hoikoバックエンドは、`/home/server/Hoiko_PPT_v0-beta1.zip`の配布実行ファイル・CSV設定と公開[HoikoCode20230120](https://github.com/ultimacrown/HoikoCode20230120)を照合した独立Rust実装です。Hoiko固有の配置展開、14手ビーム探索、4プロファイル評価、固定NEXT予測、符号付きCombo/B2B遷移、offset-off判断、抽象コマンドの`MoveDelay`/`CorrectDelay`、PC-stack・S4W・TSD/TST/DT・TD openerを実装しています。PPTプロセス読出しと実コントローラ出力は含まず、サーバーやローカル審判から受けた状態だけで合法な最終配置を返します。詳細な根拠と対応範囲は[docs/hoiko-reverse-engineering.md](docs/hoiko-reverse-engineering.md)に記録しています。

配布設定の`minDepth=10`は原版と同様に時間期限より優先されます。Rust版の直接SRS展開は原版の専用展開器より遅いため、`--ms`を小さくしてもHoiko側は最小深度を完了するまで大きく超過する場合があります。短い動作確認や比較には`--ms 0 --iterations N`を使えます。この固定回数モードは配布設定と同等の探索深度を意味しません。

通常のターン制リプレイはJSONL v2、`battle`のフレーム入力リプレイはJSONL v3です。`start`/`battle_start`の`agents`がplayer番号と実行主体を対応付け、`initial`に開始時のActive/Hold/Nextを記録します。v2の各`move`とv3の各`battle_move`には`active`、`hold_before`/`hold_after`、`next_before`/`next_after`、`hold_used`が入り、v3はさらに絶対`frame`、`movement_frames`、全フレームの`inputs`を持ちます。従来の数値`board`は互換用に残し、`board_cells`には40×10の底から上への配列で`null`、`block`、`garbage`を記録します。`verify-replay`はv2の盤面遷移とv3の入力経路を検証し、旧JSONLも受理します。固定スキーマと動画生成手順は[docs/replay-format.md](docs/replay-format.md)に記録しています。

`scripts/reproduce_validation.sh` は記録済みの固定バイナリ・公式実行ファイル・設定のSHA-256と主要5本のリプレイを検証します。長時間の100試合を再実行する場合だけ `RUN_BENCHMARKS=1 ./scripts/reproduce_validation.sh` を指定します。

比較対象は [MinusKelvin/cold-clear-2](https://github.com/MinusKelvin/cold-clear-2) の `ed8b19327b6bd1410ddd873d8611485bd45d8fae`。pinned sourceは`spawn_workers`を1ワーカーに固定し、公開設定は既定JSONだけなので、マニフェストではこれを公開ソース上の最大プロファイルとして固定しています。`vendor/cold-clear-2` は比較用で、公開API化とビルド互換性の変更のみです。`engine` は派生版で、コンボ更新、探索評価の伝播、SRS配置生成のソフトドロップ経路を修正しています。探索アダプターはArenaでは既知のお邪魔量に応じて評価重みを変更し、Guidelineでは上位64候補を実際の攻撃・相殺・ロック後盤面で再評価します。各ディレクトリに元のMIT/Apache-2.0ライセンスを保存しています。

対戦は同じマシン上で同じ時間予算を使います。Cold Clear 2の比較プロセスは公開版の1ワーカー、Intetrigenceは実時間予算では最大8ワーカーを使います（`INTETRIGENCE_SEARCH_WORKERS=1`で同じ1ワーカーに固定できます）。両者とも探索木を継続し、盤面変更などで状態が一致しなくなったときに再構築します。左右を交換した同一seedの2試合を組にします。`--pps N` を指定すると、両者が1手ずつ進むターン境界を毎秒N回に固定します（探索が境界を超えた場合は遅延します）。`--ms` は探索ループの時間上限であり、初期化・後処理と最後の展開による超過があり、ログに実測時間を残します。PPSを指定しないターン制は開発用です。`--rules guideline` は公開マルチプレイヤー表に合わせ、コンボ攻撃とPC攻撃を使わず、ロック後に受信キューの残りを全投入します。省略時の`arena`は過去ログと比較する固定開発プロファイルです。

公開仕様との対応と未完了項目は [docs/validation.md](docs/validation.md) を参照してください。

`battle-bot` は `battle-tet.bot/v1` の任意featureによるWebSocketトランスポートアダプターです。`decision.request` の `boardRows`（底から上）、Active、HOLD、NEXT 5個、Garbage量を既存の探索へ渡し、`decision.response` の `x`（4×4ローカルボックス左端）、時計回りの`rotation`（0〜3）、`useHold`を返します。`--ms`はサーバーの`deadlineMs`を超えないよう自動的に短縮されます。通常の`cargo build/test`ではこのアダプターとTLS依存を有効にせず、`arena`・`tbp`などのローカル実行はプロトコルなしで動作します。`ws://`だけなら`--features battle-bot`、`wss://`は標準CA検証付きRustlsを含む`--features battle-wss`でビルドします。

固定条件の開発評価は`benchmarks/`、外部Cold Clear 2比較は`comparisons/`に整理しています。集計JSONと対応するJSONLは、それぞれ`results/benchmarks/`と`results/comparisons/cold-clear2/`に保存し、履歴の条件と数値は[docs/validation.md](docs/validation.md)に記録しています。

固定40 PPS・10msでは83/100勝（seedペア単位の片側95%下限65.69%）、高予算5 PPS・100msでは独立seed 50組の100試合で86/100勝（下限68.69%）でした。公開Guidelineルールで公式実行ファイルとstrictに比較した最新のv2リプレイ100試合では77/100勝、seedペア単位の片側95%下限は59.69%でした。旧形式の同条件ログは69/100勝です。旧Arenaルールの公式実行ファイル比較では、10ms/40 PPSで73/100勝（下限55.69%）、高予算100試合では77/100勝（下限59.69%）でした。さらに現行releaseのstrictモードでArenaルール・100ms窓を与えた100試合では82/100勝（下限64.69%）でした。内部比較とstrict比較は固定したpinned source・既定設定・1スレッド・PPSを使う開発評価です。strictモードはUnixのプロセス停止信号を使い、TBPのgarbage更新時には再同期します。Cold Clear 2のソースに名前付きの無制限「最強設定」はないため、マニフェストの固定プロファイルを超える主張はしていません。
