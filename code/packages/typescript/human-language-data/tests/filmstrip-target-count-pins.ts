import { lstatSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const OWNER_NAME = /^[A-Za-z][A-Za-z0-9-]*\.json$/;

/** Read one exact non-negative filmstrip count per track from real direct files. */
export function loadFilmstripTargetCountPins(directory: string): Readonly<Record<string, number>> {
  const directoryStat = lstatSync(directory);
  if (directoryStat.isSymbolicLink() || !directoryStat.isDirectory()) {
    throw new Error(`filmstrip target count directory must be a real directory: ${directory}`);
  }
  const entries = readdirSync(directory, { withFileTypes: true }).sort((left, right) =>
    left.name < right.name ? -1 : left.name > right.name ? 1 : 0,
  );
  const folded = new Set<string>();
  const pins: Record<string, number> = {};
  for (const entry of entries) {
    if (!OWNER_NAME.test(entry.name) || entry.isSymbolicLink() || !entry.isFile()) {
      throw new Error(`unsafe filmstrip target count owner '${entry.name}'`);
    }
    const track = entry.name.slice(0, -".json".length);
    const key = track.toLocaleLowerCase("en-US");
    if (track !== key || folded.has(key)) {
      throw new Error(`filmstrip target count owner must be unique lowercase: ${entry.name}`);
    }
    folded.add(key);
    const parsed: unknown = JSON.parse(readFileSync(join(directory, entry.name), "utf8"));
    if (
      typeof parsed !== "object" || parsed === null || Array.isArray(parsed) ||
      Object.keys(parsed).length !== 1 || !("count" in parsed) ||
      !Number.isSafeInteger((parsed as { count: unknown }).count) ||
      ((parsed as { count: number }).count < 0)
    ) {
      throw new Error(`${entry.name}: expected exactly one non-negative integer count`);
    }
    pins[track] = (parsed as { count: number }).count;
  }
  if (entries.length === 0) throw new Error("filmstrip target count owner set must not be empty");
  return pins;
}
