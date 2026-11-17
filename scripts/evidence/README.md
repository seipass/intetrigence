# Evidence files

Directories here contain historical measurement outputs. Consult the
[seed registry](../../docs/seed-registry.md) for invalidated runs. The verified
result of a current run belongs with its exact setup and raw game log. Write
new runs under `out/` and keep their evidence together when sharing results.
The Lab runner stores a replayable trace and provenance in each game record.

## Measurement lock

`arena match`, `round run`, the Lab runner and the Bash measurement wrappers
share one lock directory per user, across checkouts on the same machine. The
default is `%LOCALAPPDATA%/intetrigence/measurement.lock` on Windows and
`~/.cache/intetrigence/measurement.lock` elsewhere. Use an absolute
`MEASUREMENT_LOCK_DIR` when testing the lock in isolation.

The `owner` file identifies the process and start time. If a forced termination
leaves the lock behind, confirm the owner and its bot processes have stopped,
then remove `owner` and the empty lock directory. Other applications and
processes outside the measurement entry points still affect timing.

## Corrupted evidence (removed 2026-11-15)

The merge commit `2a2fcd1` resolved a conflict in the Rust sources but left
git conflict markers inside 37 evidence files (`attax-40-smoke/`,
`tuning-attax-40/`). Those files were interleaved from two different runs, so
they could not be repaired — `audit/*.json` no longer parsed and the
`summary.json` counters could not be re-aggregated.

Rather than silently reconstructing them, the corrupted files were deleted in
`44d4815`'s follow-up. **The numbers they contained are not cited anywhere**;
they came from the 1-ply-greedy era, which `docs/seed-registry.md` §5.3 already
marks invalid.

Rule for the future: a merge must be checked with

```sh
grep -rn '<<<<<<<' --include='*' . | grep -v '^./target'
```

before it is committed. Evidence is data — a corrupted evidence file must never
be committed, and must never be "repaired" by guessing which side is right.
