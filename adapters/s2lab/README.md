# intetrigence in 61bi's s2-bot-lab

The lab is the only interface that carries the full S2 state to both players:
the exact `b2b` chain length, the arrived and total garbage rows, and the
logical frame. TBP carries none of that, so a TBP-connected S2 attax plays with
a handicap — which is what `souta-lab/attax` issue *"s2-bot-lab 対応しろ"* says.

## Pieces

| File | Role |
|---|---|
| `../../crates/arena/src/bin/s2lab_bridge.rs` | one decision JSON in, one placement JSON out; one process serves a whole match |
| `intetrigence-decider.mjs` | spawns the bridge and exposes the lab's `decide(decision)` shape |
| `run-match.mjs` | the match loop, adapted from attax's `tools/s2-lab/run-match.mjs` (AGPL-3.0) |

## Running

```sh
cargo build --release -p intetrigence-arena --bin s2lab_bridge
ATTAX_DIR=/path/to/attax@1f41806 \
S2_LAB_ROOT=/path/to/s2-bot-lab-ref \
  bun adapters/s2lab/run-match.mjs --games 5 --seed 9001 --out out.jsonl
```

`ATTAX_DIR` must be the S2-capable attax (`1f41806`), whose `tools/s2-lab/`
exports the decider and the lab bindings. `S2_LAB_ROOT` is 61bi's
`s2-bot-lab` checkout (`npm install` once).

## What the lab is, and is not

It is a **placement arena** with a synthetic paced clock, legacy-LCG queues and
zero garbage delivery delay. It does not run realtime INPUT. attax's own S2
result (16.75% of decided games against Sold Slear) is a lab result, so an attax
comparison has to be a lab result too.

**Seats replay identically.** `--seat-offset 0` and `1` produce the same game,
so N seeds give N independent games, not 2N. Use distinct seeds.
