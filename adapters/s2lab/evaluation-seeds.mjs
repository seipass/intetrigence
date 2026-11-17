import { execFileSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import { link, mkdir, open, readFile, readdir, realpath, unlink } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { isDeepStrictEqual } from "node:util";
import { lockPath } from "./measurement-lock.mjs";

const repo = fileURLToPath(new URL("../../", import.meta.url));

function parseRange(value, source) {
  const match = /^(\d+)\s*-\s*(\d+)$/.exec(value);
  if (!match || BigInt(match[1]) > BigInt(match[2])) throw new Error(`invalid seed range in ${source}`);
  return [BigInt(match[1]), BigInt(match[2]), source];
}

async function reservedRanges() {
  const exclusions = "docs/evaluation-seed-exclusions.txt";
  const text = await readFile(join(repo, exclusions), "utf8");
  const ranges = text.split(/\r?\n/).map(line => line.split("#", 1)[0].trim()).filter(Boolean)
    .map(value => parseRange(value, exclusions));
  const files = execFileSync("git", ["-C", repo, "ls-files", "-z", "--", "docs/seeds-round-*.md"], { encoding: "utf8" });
  for (const file of files.split("\0").filter(Boolean)) {
    const lines = (await readFile(join(repo, file), "utf8")).split(/\r?\n/);
    for (const line of lines) {
      const field = /^\s*(?:-\s+)?(range|seeds)\s*:\s*(.+?)\s*$/i.exec(line);
      if (!field) continue;
      if (field[1].toLowerCase() === "range") {
        ranges.push(parseRange(field[2], file));
      } else {
        for (const seed of field[2].split(",").map(value => value.trim()).filter(Boolean)) {
          if (!/^\d+$/.test(seed)) throw new Error(`invalid seed in ${file}`);
          ranges.push([BigInt(seed), BigInt(seed), file]);
        }
      }
    }
  }
  return ranges;
}

// Called while the measurement lock is held, so the ledger cannot change
// between overlap checks and publication of the next game claim.
export async function labSeedClaims(out, settings) {
  const ledger = join(dirname(lockPath()), "evaluation-seeds");
  await mkdir(ledger, { recursive: true });
  const path = resolve(out);
  const identity = { output: join(await realpath(dirname(path)), basename(path)), settings };
  const ranges = await reservedRanges();
  const claimed = new Map();
  for (const name of await readdir(ledger)) {
    if (!name.endsWith(".json")) continue;
    const file = join(ledger, name);
    const entry = JSON.parse(await readFile(file, "utf8"));
    if (!Array.isArray(entry.seeds) || !entry.seeds.length || !entry.kind || !entry.identity) {
      throw new Error(`invalid evaluation seed claim at ${file}`);
    }
    for (const seed of entry.seeds) {
      if (claimed.has(seed)) throw new Error(`duplicate evaluation seed ${seed} in the ledger`);
      claimed.set(seed, { file, entry });
    }
  }

  function check(seed, resume) {
    const existing = claimed.get(seed);
    if (existing) {
      if (resume && existing.entry.kind === "lab" && isDeepStrictEqual(existing.entry.identity, identity)) return true;
      throw new Error(`evaluation seed ${seed} has a claim at ${existing.file}`);
    }
    const value = BigInt(seed);
    const reserved = ranges.find(([min, max]) => value >= min && value <= max);
    if (reserved) throw new Error(`evaluation seed ${seed} is reserved in ${reserved[2]}`);
    return false;
  }

  async function claim(seed, resume) {
    if (check(seed, resume)) return;
    const entry = { seeds: [seed], kind: "lab", identity };
    const file = join(ledger, `lab-${seed}.json`);
    const temp = join(ledger, `${randomUUID()}.tmp`);
    const handle = await open(temp, "wx");
    try {
      try {
        await handle.writeFile(JSON.stringify(entry));
        await handle.sync();
      } finally {
        await handle.close();
      }
      await link(temp, file);
      claimed.set(seed, { file, entry });
    } finally {
      await unlink(temp);
    }
  }
  return { check, claim };
}
