import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, normalize, relative as pathRelative, resolve } from "node:path";
import { assertRelativeManifestPath } from "./manifest-path.js";
import { pathToFileURL } from "node:url";
import { buildCurriculumGapReport } from "./report.js";
import {
  defaultCurriculumRoot,
  loadChapterPolicy,
  loadEverything,
  loadTrackChapters,
} from "./loader.js";
import type { GentleRampReport, TrackGentleRamp } from "./gentle-ramp.js";

export const GENTLE_RAMP_SNAPSHOT_DIR = "core/gentle-ramp-snapshots";

function safeSnapshotOutput(root: string, relative: string): string {
  assertRelativeManifestPath(relative, `unsafe gentle-ramp snapshot output '${relative}'`);
  const output = resolve(root, relative);
  const fromRoot = normalize(pathRelative(resolve(root), output)).replaceAll("\\", "/");
  if (
    fromRoot === "" ||
    fromRoot === ".." ||
    fromRoot.startsWith("../") ||
    !fromRoot.startsWith(`${GENTLE_RAMP_SNAPSHOT_DIR}/`) ||
    !fromRoot.endsWith(".json")
  ) {
    throw new Error(`unsafe gentle-ramp snapshot output '${relative}'`);
  }
  return output;
}

export function serializeGentleRampTrack(track: TrackGentleRamp): string {
  return `${JSON.stringify(track, null, 2)}\n`;
}

export function generatedGentleRampSnapshotOutputsFromReport(
  report: GentleRampReport,
): Map<string, string> {
  return new Map(
    [...report.tracks]
      .sort((a, b) => a.language.localeCompare(b.language))
      .map((track) => [
        `${GENTLE_RAMP_SNAPSHOT_DIR}/${track.language}.json`,
        serializeGentleRampTrack(track),
      ]),
  );
}

export function generatedGentleRampSnapshotOutputs(
  root = defaultCurriculumRoot(),
): Map<string, string> {
  const { registry, lessons, books, curricula, spine } = loadEverything(root);
  const report = buildCurriculumGapReport({
    registry,
    lessons,
    books,
    curricula,
    spine,
    chapterPolicy: loadChapterPolicy(root),
    trackChapters: loadTrackChapters(root),
  }).gentleRamp;
  if (!report) throw new Error("chapter policy was not loaded; cannot snapshot the gentle ramp");
  return generatedGentleRampSnapshotOutputsFromReport(report);
}

export function runGentleRampSnapshots(
  args = process.argv.slice(2),
  root = defaultCurriculumRoot(),
): number {
  const mode = args.length === 1 ? args[0] : undefined;
  if (mode !== "--check" && mode !== "--write") {
    process.stderr.write("usage: gentle-ramp-snapshot-cli (--check | --write)\n");
    return 2;
  }

  const outputs = generatedGentleRampSnapshotOutputs(root);
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
    const snapshotDir = resolve(root, GENTLE_RAMP_SNAPSHOT_DIR);
    const expectedNames = new Set([...outputs.keys()].map((path) => path.split("/").at(-1)!));
    const actualNames = existsSync(snapshotDir)
      ? readdirSync(snapshotDir).filter((name) => name.endsWith(".json"))
      : [];
    for (const name of actualNames) {
      if (!expectedNames.has(name)) {
        process.stderr.write(`${GENTLE_RAMP_SNAPSHOT_DIR}/${name}: unexpected stale snapshot\n`);
        mismatch = true;
      }
    }
  }
  return mismatch ? 1 : 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exit(runGentleRampSnapshots());
}
