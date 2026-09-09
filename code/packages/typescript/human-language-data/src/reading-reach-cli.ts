// Prints the reading-reach table. Deliberately read-only: this report is NOT a
// committed artifact.
//
// Three derived ledgers already regenerate on every lesson-prose edit --- books,
// the modality manifest, the narration export --- and each one is a file every
// parallel author must rebuild and can conflict on. A fourth would cost every
// future authoring branch a regeneration step to tell a reader something they
// can print on demand. The invariants that actually need enforcing live in
// `reading-reach.test.ts` and in `core/reading-reach-floor.json`, which is
// touched only when a track's reading genuinely grows.

import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import {
  defaultCurriculumRoot as defaultRoot,
  listTaskShapeInventories,
  loadEverything,
  loadTaskShapeInventory,
} from "./loader.js";
import { formatReadingReach, measureReadingReach } from "./reading-reach.js";

interface ReadingReachOptions {
  root?: string;
  format?: "text" | "json";
}

export function runReadingReachReport(
  argv: readonly string[] = process.argv.slice(2),
  options: ReadingReachOptions = {},
): number {
  const root = options.root ?? defaultRoot();
  const format = argv.includes("--json") ? "json" : (options.format ?? "text");
  const inventories = listTaskShapeInventories(root).map(({ language, level }) =>
    loadTaskShapeInventory(language, level, root),
  );
  const report = measureReadingReach(loadEverything(root).lessons, inventories);
  process.stdout.write(
    format === "json"
      ? `${JSON.stringify(report, null, 2)}\n`
      : `${formatReadingReach(report)}\n`,
  );
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exit(runReadingReachReport());
}
