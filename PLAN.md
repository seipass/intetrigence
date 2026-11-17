# intetrigence の現行計画

## 対戦

- Attax の強度測定と50先は [s2-bot-lab の手順](adapters/s2lab/README.md)を使う。相手は Rust `c83d339`、幅96、Φオン。両者が選んだ合法手と各設置の採点を保存し、完走後に再検証する。
- CC2 と clys は [Rust レフェリー](docs/arena.md) の TBP 経路で測る。互換パッチ、公開情報、思考予算とルールセットを記録する。
- 50先では TopOut 勝ちのみを得点にする。未決着と引き分けを別に記録し、接続・検証エラーが出た場合はその場で停止する。
- 修正版の50先は **Intetrigence 1勝、Attax 50勝**。全51局を再検証済み。対戦条件と初回の成績は [結果報告](docs/attax-first-to-50.md)を参照。
- 勝率を測る前に、相手と自機のビルド、探索設定、シードを固定する。調整用シードは成績の評価に使わない。現行のシード使用状況は [seed registry](docs/seed-registry.md) に置く。

## 調査と仕様

- [リポジトリ監査](docs/repository-audit.md)に検出した不具合と再現方法がある。
- [S2 仕様](docs/s2-spec.md)はルール定数と参照資料を収録する。[Rust レフェリーのプロトコル](docs/s2-referee-protocol.md)には旧実験の決定履歴も含む。Attax の Lab 手順は上記の専用手順に集約する。
- [2026-11-16 までの計画と実験履歴](docs/history/plan-2026-11-16.md)は当時の番号、数値、撤回経緯を保管する。旧 Attax の38.9%は撤回済み。
