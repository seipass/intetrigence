# Attax との50先

## 修正版の再計測

2026-11-17 JST、**Intetrigence 1勝、Attax 50勝**。シード `487001–487051` の全51局が TopOut で決着した。自機は決定論 d8 w88・8スレッド。ゴミの到着と挿入、Clutch、All-Mini+ の修正を含む bridge とランナーを固定した。Attax は Rust `c83d339`、幅96、Φオン、solvencyオフ。 [実行条件とハッシュ](../out/attax-first-to-50-487001/frozen-manifest.json)

| 指標 | Intetrigence | Attax |
| --- | ---: | ---: |
| 設置数 | 5,655 | 5,655 |
| 相殺前の攻撃行 | 3,919 | 5,608 |
| 送信したゴミ行 | 1,878 | 4,842 |

固定したランナーで全51局を再検証し、勝敗が一致した。検証の終了コードは0。 [検証ログ](../out/attax-first-to-50-487001/verification.log) · [集計 JSON](../out/attax-first-to-50-487001/summary.json) · [対局 JSONL](../out/attax-first-to-50-487001/games.jsonl)

試験用の同一シード9001・9002で両ビルドを比較した。初回ビルドでは両局とも各200手で上限に達し、修正版では165手・152手でAttaxがTopOut勝ちした。自機の最初の100手では平均探索深度が8.00・8.00から6.52・6.07へ、相殺前の攻撃行が85・85から76・82へ変化した。4局とも再検証済み。複数の探索変更を含むため、個々の変更の寄与は同一シードでの切り分けが必要。 [初回ビルドの試験](../out/attax-paired-audit/386001.jsonl) · [修正版の試験](../out/attax-paired-audit/487001.jsonl)

修正版の試験2局で、探索深度が8未満だった114手はすべてゴミが滞留していた。到着済みでは122手中87手、未到着のみでは58手中27手、滞留ゼロでは137手中0手。初回ビルドと着手が最初に分かれたのは、9001の17手目（到着済み4行、深度1）と9002の11手目（未到着4行、深度2）。

修正版 bridge を固定し、ランナーから `framesPerLock` だけを省いた診断では、9001は49手でAttax勝ち、9002は200手で上限到達。2局とも再検証済み。この入力は将来到着と将来フレームの倍率を同時に切り替える。対戦結果への効果はこの2局で方向が異なった。 [診断対局](../out/attax-paired-audit/no-future-arrivals.jsonl)

## 初回の結果

2026-11-16〜17 JST、s2-bot-lab の配置対戦。**Intetrigence 5勝、Attax 50勝**。55局すべて TopOut 決着。引き分け、上限到達、接続エラーは0件。

## 条件

- 連番 seed は 386001〜386055。席を各局で交互にした。Intetrigence の勝ちは左2・右3。
- Attax は Rust `c83d3397d458f4af9cfd6fe3ba3468df48bb2877`、幅96、Φオン、solvencyオフ。常駐 Rust プロセスで探索した。
- Intetrigence は決定論 d8 w88、8スレッド。開始前に固定した bridge と runner を全局で使用した。
- Lab は 1 PPS の論理クロック、公開キュー14個（current を含む）、ゴミ配達の追加遅延0。計測した思考時間は診断値。公開 Lab の参照コミットと照合した実行ソース136ファイルは同一で、ローカルと公開 lockfile の197依存エントリは同じバージョンだった。 [参照コミット](https://github.com/61bi-234469/s2-bot-lab/tree/79505ca93bd15f7bcbd58724cb2a6ba312e09434)

| 指標 | Intetrigence | Attax |
| --- | ---: | ---: |
| 設置数 | 8,560 | 8,560 |
| 相殺前の攻撃行 | 8,216 | 9,680 |
| 送信したゴミ行 | 4,827 | 7,504 |
| T-spin double | 179 | 326 |

## 記録

- [全局の JSONL](../out/attax-first-to-50-386001/games.jsonl) と [集計 JSON](../out/attax-first-to-50-386001/summary.json) はローカルに保存した。全局 JSONL の SHA-256 は `7dc2e3fb61756e12a0a6443cb0a2d8ac829cc3775f7df996ab0f7ba753d3029f`。
- 採用手の照合はピース、hold、最終回転、占有セルを使う。候補を合法手フィルターで除外した回数は Attax 側で19件。各試合の公開状態、選択手、採用手、採点を記録した。
- 対戦条件と実行コマンドは [Lab 手順](../adapters/s2lab/README.md)。このシリーズの再検証では `--recorded-runner out/attax-first-to-50-386001/run-match.mjs` を追加し、凍結したランナーのハッシュも照合する。
- 全55局の設置を再適用し、同じ **5勝対50勝** と一致した。検証の終了コードは0。 [検証ログ](../out/attax-first-to-50-386001/verification.log)

## 残件

50先の後に CC2 の空 hold 探索と、CC2・clys の TBP リクエスト探索枠を修正した。短い対戦で各変更を検証した。公式の CC2・clys 評価と、Lab と実 TETR.IO 入力対戦との外部照合は未完了。自機の探索は滞留ゴミの数量を考慮し、未知の穴配置や将来の到着時刻を近似する。 [CC2 の検証](cc2-integration.md) · [監査記録](repository-audit.md)
