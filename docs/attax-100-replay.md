# Attax との100局リプレイ

2026-11-17、s2-bot-lab 上で Attax `c83d339` と100局対戦し、30fps/480pの1本の動画にした。

## 条件

- seed は **30001–30100** の連番。各局で席を交互にした。
- Attax は Rust `c83d3397d458f4af9cfd6fe3ba3468df48bb2877`、幅96、Φオン、solvencyオフ。
- Intetrigence は決定論 d8 w88、8スレッド。
- Lab は 1 PPS、公開キュー14個（current を含む）、ゴミ配達の追加遅延0。
- 実行した runner、decider、bridge、Lab のハッシュは
  [`out/attax-100-30001/frozen-manifest.json`](../out/attax-100-30001/frozen-manifest.json)
  に固定した。51局シリーズと同じハッシュで、環境は同一。

## 結果

**Intetrigence 0勝、Attax 100勝**。100局すべて TopOut で決着し、エラーと上限到達は0件。
合計 10,648 ターン（各側）を 29.1 分で消化した。

| 指標 | Intetrigence | Attax |
| --- | ---: | ---: |
| ターン数 | 10,648 | 10,648 |
| 相殺前の攻撃行 | 7,146 | 10,468 |
| 送信したゴミ行 | 3,314 | 9,129 |
| 受信したゴミ行 | 9,129 | 3,314 |
| 相殺した行 | 3,902 | 1,352 |
| 盤面に挿入されたゴミ行 | 5,017 | 1,962 |
| スピンなしのシングル | 3,417 | 420 |
| T-spin double | 148 | 473 |
| Surge の攻撃行 | 104 | 1,596 |

各局の `contestants` を使い、左右の席をボット名に変換して集計した。集計 JSON の `left` と `right` は席ごとの値。相殺前の攻撃量は100局すべてでAttaxが上回り、最初の20手でも合計897行対1,265行と差があった。全体の攻撃差3,322行のうち、Surgeの差は1,492行（45%）。自機は最終手でゴミが挿入された局が93局あった。

自機の探索深度は、滞留ゴミがある6,004手のうち3,663手で8未満だった。滞留ゴミがゼロの4,644手では3手。自機は仮にゴミが挿入された枝をそこで評価し、Attaxは仮の穴を置いた盤面から先読みを続ける。この違いが勝率に与える影響の確認には、同じ局面を使った比較が必要。 [自機の探索](../crates/bot/src/search.rs#L951) · [Attaxの探索](https://github.com/souta-lab/attax/blob/c83d3397d458f4af9cfd6fe3ba3468df48bb2877/rust/attax-search/src/search.rs#L502)

集計は [`out/attax-100-30001/summary.json`](../out/attax-100-30001/summary.json)、
対局記録は [`out/attax-100-30001/games.jsonl`](../out/attax-100-30001/games.jsonl) に置く。
JSONL の SHA-256 は `2ef17b03f6629f7b73dd1f701738623d1a70a02f0655eddb117260bf1e5900cc`。

## 検証

`--verify` で100局の設置を Lab レフェリーに再適用し、勝敗と trace が一致することを確認した。
検証結果は **100局すべて一致、スコアも 0–100 で一致**。検証はボットに判断をやり直させない。
ログは [`out/attax-100-30001/verification.log`](../out/attax-100-30001/verification.log) に保存する。

## 動画

`arena render-s2lab` で各局を 854x480・30fps のセグメントにし、最後に連結した。
1局の長さはその局のターン数（1 PPS なので1ターン=1秒）に一致する。各局の頭に1秒の
タイトル、末尾に3秒の結果カードを付けた。

- 出力: [`out/attax-100-30001/videos/attax-100-30001-30fps-480p.mp4`](../out/attax-100-30001/videos/attax-100-30001-30fps-480p.mp4)
- 解像度: 854x480、30fps、H.264、音声なし
- 長さ: **11,048秒（3時間4分8秒）** = 10,648秒の対局 + 100秒のタイトル + 300秒の結果カード
- サイズ: 123,164,920 バイト（約118 MiB）
- SHA-256: `09cc3700ebf95b70055f8ea0ad3c0afa2b70bc52ae5b282e816eec38288b5d6b`

再生成:

```sh
target/release/arena render-s2lab \
  --games out/attax-100-30001/games.jsonl \
  --out out/attax-100-30001/videos/attax-100-30001-30fps-480p.mp4 \
  --fps 30 --width 854 --height 480 --title 1 --end-card 3
```

レンダラーは trace の各ターンで記録された公開盤面を描き、設置とライン消去を適用する。
ゲームの正しさは `--verify` が担い、動画はその記録の表示である。`arena render` の
TBP replay のような盤面ハッシュ再計算は行わない。
