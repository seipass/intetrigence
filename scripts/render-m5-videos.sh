#!/usr/bin/env bash
# Renders the ten M5-provisional highlight games to MP4 in videos/.
#
# Selection (docs/m5-provisional-report.md): the nine non-forfeit games
# (three turn-cap wins and six TopOut losses) plus one short forfeit-vision win.
# Every replay is verified before rendering; `arena render` refuses to draw a
# replay whose hashes do not match.
#
# Output: videos/seed-<seed>-sheet-<sheet>.mp4 and videos/manifest.json.
# The MP4 files are git-ignored; the manifest is committed.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="${OUT_DIR:-$ROOT/videos}"
EVIDENCE="${EVIDENCE:-$ROOT/scripts/evidence/m5-provisional}"
LOG="$OUT_DIR/.render-log.jsonl"

GAMES=(
  "11007-A:non-forfeit turn-cap win"
  "11015-B:non-forfeit turn-cap win"
  "11031-A:non-forfeit turn-cap win"
  "11004-A:non-forfeit TopOut loss"
  "11011-A:non-forfeit TopOut loss"
  "11020-B:non-forfeit TopOut loss"
  "11025-A:non-forfeit TopOut loss"
  "11027-A:non-forfeit TopOut loss (opener)"
  "11030-B:non-forfeit TopOut loss"
  "11012-A:forfeit-vision win (short)"
)

mkdir -p "$OUT_DIR"
: > "$LOG"

cargo build --offline --release -p intetrigence-arena --bin arena

for entry in "${GAMES[@]}"; do
  game="${entry%%:*}"
  label="${entry#*:}"
  seed="${game%-*}"
  sheet="${game#*-}"
  replay="$EVIDENCE/replays/seed-${seed}-sheet-${sheet}.jsonl"
  out="$OUT_DIR/seed-${seed}-sheet-${sheet}.mp4"
  if [ ! -f "$replay" ]; then
    echo "missing replay: $replay" >&2
    exit 1
  fi
  echo "render: $game ($label)"
  "$ROOT/target/release/arena.exe" render \
    --replay "$replay" --out "$out" | tee -a "$LOG"
done

python - "$OUT_DIR" "$LOG" "$EVIDENCE" <<'PYEOF'
import json
import os
import sys
import time

out_dir, log_path, evidence = sys.argv[1], sys.argv[2], sys.argv[3]
labels = {
    "11007-A": "non-forfeit turn-cap win",
    "11015-B": "non-forfeit turn-cap win",
    "11031-A": "non-forfeit turn-cap win",
    "11004-A": "non-forfeit TopOut loss",
    "11011-A": "non-forfeit TopOut loss",
    "11020-B": "non-forfeit TopOut loss",
    "11025-A": "non-forfeit TopOut loss",
    "11027-A": "non-forfeit TopOut loss (opener)",
    "11030-B": "non-forfeit TopOut loss",
    "11012-A": "forfeit-vision win (short)",
}
names = {0: "intetrigence", 1: "attax"}
videos = []
with open(log_path, encoding="utf-8") as handle:
    for line in handle:
        line = line.strip()
        if not line.startswith("{"):
            continue
        record = json.loads(line)
        key = f"{record['seed']}-{record['sheet']}"
        file_name = os.path.basename(record["output"])
        path = os.path.join(out_dir, file_name)
        videos.append(
            {
                "file": file_name,
                "replay": os.path.relpath(record["replay"], evidence).replace(os.sep, "/"),
                "seed": record["seed"],
                "sheet": record["sheet"],
                "turns": record["turns"],
                "frames": record["frames"],
                "duration_seconds": round(record["duration_seconds"], 2),
                "winner": names.get(record.get("winner"), "draw"),
                "reason": record["reason"],
                "forfeit_cause": record.get("forfeit_cause"),
                "label": labels.get(key, ""),
                "verified": record["verified"],
                "size_bytes": os.path.getsize(path) if os.path.exists(path) else None,
            }
        )
manifest = {
    "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "source": os.path.relpath(evidence, os.path.dirname(out_dir)).replace(os.sep, "/"),
    "profile": "official",
    "attax": "7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b (guideline scoring, 20-row vision)",
    "renderer": "arena render (crates/arena/src/render.rs), 1280x720, 60 fps, libx264",
    "videos": videos,
}
with open(os.path.join(out_dir, "manifest.json"), "w", encoding="utf-8") as handle:
    json.dump(manifest, handle, indent=2)
    handle.write("\n")
print(f"manifest: {len(videos)} videos")
PYEOF
