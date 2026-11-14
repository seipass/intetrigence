# Evidence files

Each directory holds the raw output of one measurement run: `summary.json`,
`audit/*.json` (per-game scoring audit), `replays/*.jsonl` (verified replays)
and `logs/*.jsonl` (opponent transcripts / decision logs).

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
