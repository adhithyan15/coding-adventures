import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, normalize, relative as pathRelative, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { assertRelativeManifestPath } from "./manifest-path.js";
import { defaultCurriculumRoot, loadEverything } from "./loader.js";
import {
  CEFR_LEVELS,
  summarizeLevels,
  type CefrLevel,
  type LevelSummary,
  type TrackLevelCoverage,
} from "./levels.js";

export const LEVEL_SNAPSHOT_DIR = "core/level-snapshots";

function safeSnapshotOutput(root: string, relative: string): string {
  assertRelativeManifestPath(relative, `unsafe level snapshot output '${relative}'`);
  const output = resolve(root, relative);
  const fromRoot = normalize(pathRelative(resolve(root), output)).replaceAll("\\", "/");
  if (
    fromRoot === "" ||
    fromRoot === ".." ||
    fromRoot.startsWith("../") ||
    !fromRoot.startsWith(`${LEVEL_SNAPSHOT_DIR}/`) ||
    !fromRoot.endsWith(".json")
  ) {
    throw new Error(`unsafe level snapshot output '${relative}'`);
  }
  return output;
}

export function serializeLevelTrack(track: TrackLevelCoverage): string {
  return `${JSON.stringify(track, null, 2)}\n`;
}

export function generatedLevelSnapshotOutputsFromSummary(
  summary: LevelSummary,
): Map<string, string> {
  return new Map(
    [...summary.tracks]
      .sort((a, b) => a.language.localeCompare(b.language))
      .map((track) => [
        `${LEVEL_SNAPSHOT_DIR}/${track.language}.json`,
        serializeLevelTrack(track),
      ]),
  );
}

export function generatedLevelSnapshotOutputs(
  root = defaultCurriculumRoot(),
): Map<string, string> {
  const { lessons, curricula, spine } = loadEverything(root);
  return generatedLevelSnapshotOutputsFromSummary(summarizeLevels(lessons, curricula, spine));
}

/** Reconstruct the exact corpus totals from independently mergeable track snapshots. */
export function summarizeLevelTracks(tracks: TrackLevelCoverage[]): Omit<LevelSummary, "tracks"> {
  const byLevel = Object.fromEntries(CEFR_LEVELS.map((level) => [level, 0])) as Record<
    CefrLevel,
    number
  >;
  let totalLessons = 0;
  let unmapped = 0;
  for (const track of tracks) {
    totalLessons += track.lessonCount;
    unmapped += track.unmapped;
    for (const level of CEFR_LEVELS) byLevel[level] += track.byLevel[level];
  }
  const mapped = totalLessons - unmapped;
  return {
    totalLessons,
    byLevel,
    unmapped,
    mappedPercent: totalLessons === 0 ? 0 : Math.round((mapped / totalLessons) * 100),
  };
}

export function runLevelSnapshots(
  args = process.argv.slice(2),
  root = defaultCurriculumRoot(),
): number {
  const mode = args.length === 1 ? args[0] : undefined;
  if (mode !== "--check" && mode !== "--write") {
    process.stderr.write("usage: level-snapshot-cli (--check | --write)\n");
    return 2;
  }

  const outputs = generatedLevelSnapshotOutputs(root);
  let mismatch = false;
  for (const [relative, expected] of outputs) {
    const output = safeSnapshotOutput(root, relative);
    if (mode === "--write") {
      mkdirSync(dirname(output), { recursive: true });
      writeFileSync(output, expected, "utf8");
      process.stdout.write(`generated ${relative}\n`);
      continue;
    }
    const actual = existsSync(output) ? readFileSync(output, "utf8") : undefined;
    if (actual !== expected) {
      process.stderr.write(`${relative}: generated output is missing or stale\n`);
      mismatch = true;
    }
  }

  if (mode === "--check") {
    const snapshotDir = resolve(root, LEVEL_SNAPSHOT_DIR);
    const expectedNames = new Set([...outputs.keys()].map((path) => path.split("/").at(-1)!));
    const actualNames = existsSync(snapshotDir)
      ? readdirSync(snapshotDir).filter((name) => name.endsWith(".json"))
      : [];
    for (const name of actualNames) {
      if (!expectedNames.has(name)) {
        process.stderr.write(`${LEVEL_SNAPSHOT_DIR}/${name}: unexpected stale snapshot\n`);
        mismatch = true;
      }
    }
  }
  return mismatch ? 1 : 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exit(runLevelSnapshots());
}
