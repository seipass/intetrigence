# Determinism scope: our moves, not the game

Status: measured 2026-11-16. Build `d7742bc`, build id `e24cca552047dc43`.

## Question

`--deterministic --depth 8 --width 96 --threads 8` makes our search a pure
function of the position sequence (tested at threads 1, 2 and 8: same moves,
same node counts, same lines). Does that make a **game** reproducible from its
seed?

## Experiment

Two runs of the same seed and sheet, same build, same flags:

```
arena match --bot search --seeds 9001 --sheets A --deterministic --depth 8 --width 96
```

| run | result | turns |
| --- | --- | --- |
| 1 | intetrigence TopOut win | 182 |
| 2 | intetrigence TopOut win | 171 |

Replays: `scripts/evidence/cc2-9001a-det-1/`, `scripts/evidence/cc2-9001a-det-2/`.

Per-slot comparison of `placements` (piece, rotation, x, y, hold, spin):

- **turn 0: identical for both slots**
- **turn 1, slot 1 (Cold Clear 2): first divergence** — T at `x=7` in run 1,
  `x=5` in run 2
- slot 0 (intetrigence) had played identically up to that point

## Conclusion

- **Our moves are deterministic** given the position sequence.
- **Cold Clear 2 is not.** It runs as a separate process with its own time
  budget, so the same position can produce a different placement.
- **"Same seed → same game" only holds when both sides are deterministic.**
  Against CC2 (and any external engine) it does not.

## What this means for claims

1. A seed fixes the **starting conditions**, not the game. A 50-game rate is a
   sample of 50 games, not a reproducible script; the Wilson interval is the
   right uncertainty statement.
2. A replay is the record of the game that was played. It is never
   re-simulated from the seed.
3. `--resume` reuses the **replay**, not a re-simulation, so a resumed game is
   the same game. That is why the resume check must compare the build id and
   the bot name (see `docs/s2-referee-protocol.md` §1.4).
4. Do not describe a match as "reproducible". Describe it as "our side is
   deterministic; the opponent's side is whatever it played".

## Corollary: the stale-binary incident

The first `cc2-40` attempt recorded `commit: 18b32d2` in its audits while
playing with an older binary. `git HEAD` records the checkout, not the build.
The build id (`crates/arena/build.rs`) now hashes `crates/bot`, `crates/core`
and `crates/arena` into `ARENA_BUILD_ID`; the replay header, the audit and the
summary carry it, together with the binary's path, size and mtime.
