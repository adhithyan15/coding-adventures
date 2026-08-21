// modality-cli — write and gate the derived modality manifest (spec: HL08).
//
// The filesystem shell around `modality-manifest.ts`, following `book-cli.ts` line for
// line so there is one shape of generated-artifact CLI in this package rather than two:
//
//   generatedModalityOutputs()  one language-owned path -> content entry per track
//                               the bytes without ever touching a disk
//   safeOutput()                fail-closed path containment, applied to every write
//   --write                     put those bytes on disk
//   --check                     compare byte for byte, exit 1 on any difference
//
// ---------------------------------------------------------------------------
// Why `--check` is the whole point
// ---------------------------------------------------------------------------
//
// A derived file that nothing verifies is worse than no file. It looks authoritative,
// it is committed, it is read by the app — and it drifts the first time somebody edits
// a lesson without re-running the generator. For a modality manifest the drift is not
// cosmetic: a lesson that gained a paradigm table still reads `drivable: true`, the
// driving edition ships it, and a learner at 70mph is told to look at a chart.
//
// So CI runs `--check` beside `check:books`. The manifest cannot be stale, because a
// stale manifest fails the build.
//
// ---------------------------------------------------------------------------
// Why the output path is a parameter
// ---------------------------------------------------------------------------
//
// `MODALITY_MANIFEST_DIR` is the only directory anything writes today, and a constant
// cannot escape the curriculum root. The parameter exists because a future edition may emit an
// edition-specific variant beside it, and the moment an output path comes from anywhere
// but this file it must be contained — `../../.github/workflows/x.yml` is a perfectly
// good relative path and a perfectly terrible thing to overwrite. `safeOutput` is
// therefore applied unconditionally rather than "when the path looks configurable",
// which is the mistake that leaves a guard un-run on the day it starts mattering.

import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, normalize, relative as pathRelative, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { defaultCurriculumRoot, loadLessons } from "./loader.js";
import {
  MODALITY_MANIFEST_DIR,
  buildModalityManifest,
  serializeModalityManifest,
} from "./modality-manifest.js";
import type { ModalityOptions } from "./modality.js";
import type { ParsedLesson } from "./parse.js";

/**
 * Resolve a relative output inside the curriculum root, or throw.
 *
 * Three conditions, all of which must hold, and the function throws rather than
 * returning a fallback — a path guard that silently substitutes a "safe" path is a
 * guard that writes the wrong file instead of no file:
 *
 *   1. the resolved path lies strictly under `root` (`..` and absolute paths rejected);
 *   2. it is not the root directory itself;
 *   3. it ends in `.json`, so a mistake cannot land on a `.md` lesson or a `.tex` book.
 *
 * Containment is decided AFTER `resolve`, not by inspecting the input string. Checking
 * the raw string for `..` is the classic hole: `a/b/../../../etc` contains no leading
 * `..` and still escapes, while a symlink-free `resolve` plus a `relative` back to the
 * root answers the question that actually matters — where does this land?
 */
export function safeOutput(root: string, relative: string): string {
  const output = resolve(root, relative);
  const fromRoot = normalize(pathRelative(resolve(root), output)).replaceAll("\\", "/");
  if (
    fromRoot === "" ||
    fromRoot === ".." ||
    fromRoot.startsWith("../") ||
    !fromRoot.endsWith(".json")
  ) {
    throw new Error(`unsafe generated modality output '${relative}'`);
  }
  return output;
}

/**
 * The generated file(s), as a path → content map.
 *
 * A map rather than a write, so `--write` and `--check` consume identical bytes from
 * identical code. Any divergence between "what we would write" and "what we compare
 * against" is a gate that passes while lying, and returning the content is the cheapest
 * way to make that divergence impossible.
 */
export function generatedModalityOutputsFromLessons(
  lessons: readonly ParsedLesson[],
  outputDir: string = MODALITY_MANIFEST_DIR,
  options: ModalityOptions = {},
): Map<string, string> {
  const byLanguage = new Map<string, ParsedLesson[]>();
  for (const lesson of lessons) {
    const bucket = byLanguage.get(lesson.language);
    if (bucket) bucket.push(lesson);
    else byLanguage.set(lesson.language, [lesson]);
  }
  return new Map(
    [...byLanguage]
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([language, lessons]) => {
        if (!/^[a-z0-9-]+$/.test(language)) {
          throw new Error(`unsafe language id '${language}' for modality shard`);
        }
        const output = `${outputDir}/${language}.json`;
        return [output, serializeModalityManifest(buildModalityManifest(lessons, options))];
      }),
  );
}

export function generatedModalityOutputs(
  root = defaultCurriculumRoot(),
  outputDir: string = MODALITY_MANIFEST_DIR,
  options: ModalityOptions = {},
): Map<string, string> {
  // Validate before doing the expensive corpus walk: a bad path should fail in
  // milliseconds, not after parsing the corpus.
  safeOutput(root, `${outputDir}/probe.json`);
  const outputs = generatedModalityOutputsFromLessons(loadLessons(root), outputDir, options);
  for (const output of outputs.keys()) safeOutput(root, output);
  return outputs;
}

export function runModalityManifest(
  args = process.argv.slice(2),
  root = defaultCurriculumRoot(),
): number {
  const mode = args.length === 1 ? args[0] : undefined;
  if (mode !== "--check" && mode !== "--write") {
    process.stderr.write("usage: modality-cli (--check | --write)\n");
    return 2;
  }
  let mismatch = false;
  const outputs = generatedModalityOutputs(root);
  for (const [relative, expected] of outputs) {
    const output = safeOutput(root, relative);
    if (mode === "--write") {
      mkdirSync(dirname(output), { recursive: true });
      writeFileSync(output, expected, "utf8");
      process.stdout.write(`generated ${relative}\n`);
      continue;
    }
    // A missing file and a stale file are the same failure with the same fix — run
    // `--write` and commit the result — so they share one message.
    const actual = existsSync(output) ? readFileSync(output, "utf8") : undefined;
    if (actual !== expected) {
      process.stderr.write(`${relative}: generated output is missing or stale\n`);
      mismatch = true;
    }
  }
  if (mode === "--check") {
    const directory = resolve(root, MODALITY_MANIFEST_DIR);
    const expectedNames = new Set(
      [...outputs.keys()].map((path) => path.split("/").at(-1)!),
    );
    const actualNames = existsSync(directory)
      ? readdirSync(directory).filter((name) => name.endsWith(".json"))
      : [];
    for (const name of actualNames) {
      if (!expectedNames.has(name)) {
        process.stderr.write(`${MODALITY_MANIFEST_DIR}/${name}: unexpected stale shard\n`);
        mismatch = true;
      }
    }
  }
  return mismatch ? 1 : 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exit(runModalityManifest());
}
