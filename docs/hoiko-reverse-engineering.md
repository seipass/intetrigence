# Hoiko reverse-engineering record

This is the provenance and compatibility record for the Rust Hoiko backend.
It is an independent implementation of observed behavior; it does not link to
the Windows executable at runtime and it does not require the battle.tet
protocol.

## Evidence collected

The local package was `/home/server/Hoiko_PPT_v0-beta1.zip` (SHA-256
`8877a307a38a57d6ad9d9cb53c6829c1e911f57278f138f306290bcd20ba21bf`).
`Hoiko.exe` is a PE32+ x86-64 C++/CLI image.  The accompanying files named
`config.dll`, `we.dll`, `wo.dll`, `wd.dll`, and `wr.dll` are plain CSV text,
despite the extension.  The extracted release hashes were recorded during
the analysis; the Rust tree includes the four evaluator profiles under
[`assets/hoiko`](../assets/hoiko).

Metadata inspection with `monodis`, native section inspection with `objdump`,
and IL/C++/CLI decompilation with ILSpy 9.1 established the following class
and method bodies:

* `ExpandAI` contains `ExpandBaseNode`, `ExpandDeriveNode`,
  `RegisterChildNode`, `TryDeriveRotateA/B`, `TryDeriveMoveL/R`, and
  `TryDeriveFinA/B`.  These methods are present in the binary even though the
  public source mirror only contains the body of `ExpandNode`.
* `SearchAI` owns `QuickSearch`, `Search`, `MaxElement`, and `WriteResult`.
  The public source constants are `MAX_DEPTH=14`, `BEAMSIZE_MIN=50`, and
  `BEAMSIZE_MAX_PERTHREAD=500`; the release config sets `beamSize=50` and
  `minDepth=10`.
* `EvaluateAI` owns the board feature functions (`GetHeight`,
  `GetWellColumn`, `GetUpDown`, `GetUnderground`, `GetResource`) and the
  action functions (`GetFinalAction`, `GetContBonus`).  It selects among four
  profiles and uses a `nexus` fraction to blend action and board scores.
* The binary layout reports `Board` as 136 bytes, `Node` as 328 bytes,
  `NextMino` as 120 bytes, and `RootNode` as 968 bytes.  This is consistent
  with a 32-row internal board, 28-piece next buffer, and a fixed beam tree.
  The Rust engine retains its 40-row board because that is the host
  simulator's representation; the evaluator only uses occupied cells.
* Strings and imports identify `PPTMain`/`PPT2Main`, `MemoryReader`,
  `OpenProcess`, `ReadProcessMemory`, `FindWindowW`, and the controller
  classes.  Those are the Windows/PPT integration layer and are intentionally
  outside the Rust AI backend.

The MethodDef table also provides stable RVA anchors for the release analyzed
here: `EvaluateAI.GetHeight` is method 431 at RVA `0x1173c`,
`EvaluateAI.GetFinalAction` is method 455 at `0x12924`,
`ExpandAI.ExpandBaseNode` is method 501 at `0x1560c`,
`ExpandAI.ExpandDeriveNode` is method 502 at `0x15890`, and
`SearchAI.Search` is method 526 at `0x17268`.  These are metadata/native
entry-point anchors, not calls made by the Rust program; they make it possible
to re-check the reconstruction against the same executable without treating
the executable as a runtime dependency.

The public source used for cross-checking was
[HoikoCode20230120](https://github.com/ultimacrown/HoikoCode20230120).  The
audited revision is `eb0d112812c1dba6b3a45755468dba52cc36ecb1` (the
repository's 2023-01-20 upload).  The source mirror has no build manifest and leaves several expansion methods
undefined, so it was treated as a behavioral reference rather than a library
dependency.

## Reconstructed behavior

`src/hoiko.rs` implements all nine AI-side groups identified during the
audit:

1. `ExpandBaseNode`/`ExpandDeriveNode`-style placement expansion, including
   spawn motion, surface drops, under-stack shifts, SRS kicks, HOLD, duplicate
   suppression, and the 100-child boundary.
2. The 14-ply corrected beam, depth-ten minimum, trust reduction, family
   scores, end correction, state deduplication, and operation-time beam
   adjustment.
3. Height, well, roughness, underground, ground/roof/pierce, resource,
   TSD/TST/DT, PC, action, B2B, combo, and `nexus` evaluation terms.
4. Persistent `we`/`wo`/`wd`/`wr` selection with the release's 15/13-row
   hysteresis and opponent-state transitions.
5. HOLD and the 28-piece `NextMino` predictor. The fixed Z/S/T/O/J/L/I
   prediction is reordered by visible NEXT values and never reads the host's
   hidden random state.
6. Hoiko's signed combo debt, counted B2B chain, lock, clear, PC, and action
   transitions inside speculative nodes.
7. The optional offset-off state and opponent attack estimate used when
   `useOffsetOff` is enabled.
8. Abstract command paths and both native timings: raw `MoveDelay` for node
   evaluation and frame-corrected delay for the next beam limit. This does not
   emit physical controller input.
9. Optional PC-stack, side 4-wide, TSD/TST/DT recognition, and TD-opener
   template state/checkpoints. These paths are controlled by the original
   config flags.

Intermediate nodes carry garbage markers so underground terms continue to
work after a line clear.

The embedded CSV files are canonical LF copies of the release rows.  The ZIP
uses CRLF and writes one value as `-0`, so byte hashes differ even though the
parsed configuration and four versus profiles are equal.  The search retains
the selected profile between decisions, including the source's defensive
height hysteresis.  It also applies I/T waste and B2B bonuses from the
post-lock state, matching `EvalFinalAction`.

The local arena now supplies the opponent snapshot at each decision boundary;
that drives the source `wo`/`wd`/`wr` transitions and the powerful-board checks
without exposing opponent data to the network Bot Protocol. Public source
stubs that return zero (`EvalGround`, TSD/TST/DT and PC-stack) are implemented
from the distributed executable's IL rather than left disabled.

The PPT process/memory/controller layer remains outside this backend. The Rust
search receives a state snapshot and returns a legal final placement. It
models Hoiko's operation cost and corrected delay, but does not press keys or
read Windows process memory.

The release sets `minDepth=10`.  Like `SearchAI::Search`, the Rust backend does
not honor its wall-clock deadline until that depth has been completed.  The
direct Rust move generator is much slower than the native expansion routines,
so source-profile runs can exceed a small `--ms` value substantially.  Use a
fixed `--iterations` value for quick deterministic development matches; that
is an explicit reduced-search mode rather than the release search depth.

Use the embedded profiles with:

```sh
cargo run --release -- hoiko-selfplay --pieces 100 --ms 20
cargo run --release -- hoiko-tbp --ms 400
cargo run --release -- match --p1 hoiko --p2 intetrigence \
  --games 2 --ms 0 --iterations 200
cargo run --release -- match --p1 hoiko --p2 cold_clear_2 \
  --opponent vendor/cold-clear-2/target/release/cold-clear-2 \
  --games 20 --ms 20 --pps 40
```

An extracted package can override them with
`--hoiko-config /path/to/extracted/package`.  The code falls back to the
embedded profiles if that directory is absent.  The `match` launcher can
swap Hoiko and the external opponent by changing `--p1`/`--p2`; it also
accepts `intetrigence` for the in-process searcher.  It intentionally starts
at most one external TBP participant per match.
