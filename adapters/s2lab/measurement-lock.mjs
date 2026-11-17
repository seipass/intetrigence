import { mkdir, readFile, rmdir, unlink, writeFile } from "node:fs/promises";
import { dirname, isAbsolute, join } from "node:path";

export function lockPath() {
  if (process.env.MEASUREMENT_LOCK_DIR) {
    const path = process.env.MEASUREMENT_LOCK_DIR;
    if (!isAbsolute(path)) throw new Error("MEASUREMENT_LOCK_DIR must be an absolute path");
    return path;
  }
  const base = process.platform === "win32"
    ? process.env.LOCALAPPDATA || (process.env.USERPROFILE && join(process.env.USERPROFILE, "AppData", "Local"))
    : process.env.HOME && join(process.env.HOME, ".cache");
  if (!base || !isAbsolute(base)) throw new Error("cannot locate the measurement lock directory; set MEASUREMENT_LOCK_DIR");
  return join(base, "intetrigence", "measurement.lock");
}

export async function acquireMeasurement(label) {
  const path = lockPath();
  const ownerFile = join(path, "owner");
  await mkdir(dirname(path), { recursive: true });
  try {
    await mkdir(path);
  } catch (error) {
    if (error.code !== "EEXIST") throw error;
    const owner = (await readFile(ownerFile, "utf8").catch(() => "unknown owner")).trimEnd();
    if (process.env.INTETRIGENCE_MEASUREMENT_OWNER && process.env.INTETRIGENCE_MEASUREMENT_OWNER === owner) {
      return async () => {};
    }
    throw new Error(`another measurement is running (${owner}) at ${path}`);
  }
  const owner = `${label} pid=${process.pid} started=${new Date().toISOString()}`;
  try {
    await writeFile(ownerFile, owner);
  } catch (error) {
    await unlink(ownerFile).catch(() => {});
    await rmdir(path).catch(() => {});
    throw error;
  }
  return async () => {
    if (await readFile(ownerFile, "utf8").catch(() => null) === owner) {
      await unlink(ownerFile);
      await rmdir(path);
    }
  };
}
