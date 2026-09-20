import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { assertGentleRampSnapshotsRetired } from "./gentle-ramp-retirement.js";
import {
  defaultCurriculumRoot,
  loadChapterPolicy,
  loadEverything,
  loadTrackChapters,
} from "./loader.js";
import { buildCurriculumGapReport } from "./report.js";

/**
 * Compatibility entry point for the old CI command. It is now a retirement
 * guard plus an on-demand derivation check and never writes generated state.
 */
export function runGentleRampSnapshots(
  args = process.argv.slice(2),
  root = defaultCurriculumRoot(),
): number {
  if (args.length !== 1 || args[0] !== "--check") {
    process.stderr.write("usage: gentle-ramp-snapshot-cli --check\n");
    return 2;
  }
  try {
    assertGentleRampSnapshotsRetired(root);
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
    if (report === undefined) {
      throw new Error("chapter policy was not loaded; cannot derive the gentle ramp");
    }
    const expected = registry.languages.map((language) => language.id).sort();
    const actual = report.tracks.map((track) => track.language).sort();
    if (
      actual.length !== expected.length ||
      actual.some((language, index) => language !== expected[index])
    ) {
      throw new Error("derived gentle-ramp track identities do not match the language registry");
    }
    return 0;
  } catch (cause) {
    process.stderr.write(`${cause instanceof Error ? cause.message : String(cause)}\n`);
    return 1;
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exit(runGentleRampSnapshots());
}
