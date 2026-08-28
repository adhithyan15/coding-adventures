// modality-cli — write and gate the derived modality manifest (spec: HL08).
//
// The filesystem shell around `modality-manifest.ts`, following `book-cli.ts` line for
// line so there is one shape of generated-artifact CLI in this package rather than two:
//
//   generatedModalityOutputs()  one language metadata owner plus one owner per lesson
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

import {
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  realpathSync,
  renameSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, normalize, relative as pathRelative, resolve, sep } from "node:path";
import { assertRelativeManifestPath } from "./manifest-path.js";
import { pathToFileURL } from "node:url";
import {
  defaultCurriculumRoot,
  loadLanguageRegistry,
  loadLessons,
} from "./loader.js";
import {
  MODALITY_MANIFEST_DIR,
  buildModalityManifest,
  buildModalityManifestFromRows,
  serializeModalityManifest,
  type ModalityManifest,
} from "./modality-manifest.js";
import {
  assertModalityManifestLanguages,
  modalityNarrationLessonIds,
  modalityOwnerContents,
  readModalityManifestOwners,
} from "./modality-shards.js";
import type { ModalityOptions } from "./modality.js";
import type { ParsedLesson } from "./parse.js";

export interface ModalityInstallFaultHooks {
  /** Test-only fault boundary after the previous tree has moved to its recovery path. */
  afterBackupMoved?: () => void;
  /** Test-only fault boundary before removing an installed-but-unverified tree. */
  beforeInstalledRemoval?: () => void;
  /** Test-only fault boundary before restoring the previous tree. */
  beforeBackupRestore?: () => void;
  /** Test-only fault boundary after installation and before verification. */
  afterInstalled?: () => void;
}

function lstatIfPresent(path: string): ReturnType<typeof lstatSync> | undefined {
  try {
    return lstatSync(path);
  } catch (cause) {
    if ((cause as NodeJS.ErrnoException).code === "ENOENT") return undefined;
    throw cause;
  }
}

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
  assertRelativeManifestPath(relative, `unsafe generated modality output '${relative}'`);
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
  const manifest = buildModalityManifest(lessons, options);
  return new Map(
    [...modalityOwnerContents(manifest)].map(([relative, contents]) => [
      `${outputDir}/${relative}`,
      contents,
    ]),
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

function expectedLessonIds(
  manifest: ModalityManifest,
): ReadonlyMap<string, readonly string[]> {
  const out = new Map<string, string[]>();
  for (const track of manifest.tracks) out.set(track.language, []);
  for (const lesson of manifest.lessons) out.get(lesson.language)!.push(lesson.id);
  for (const ids of out.values()) ids.sort();
  return out;
}

function splitLegacyAggregates(manifest: ModalityManifest): Map<string, string> {
  const out = new Map<string, string>();
  for (const track of manifest.tracks) {
    const lessons = manifest.lessons.filter((lesson) => lesson.language === track.language);
    const lessonIds = new Set(lessons.map((lesson) => lesson.id));
    const findings = manifest.findings.filter((finding) => lessonIds.has(finding.lessonId));
    const languageManifest = buildModalityManifestFromRows(
      {
        version: manifest.version,
        algorithm: manifest.algorithm,
        features: manifest.features,
        policy: manifest.policy,
      },
      lessons,
      findings,
    );
    out.set(`${track.language}.json`, serializeModalityManifest(languageManifest));
  }
  return out;
}

function assertRealOutputDirectory(root: string, outputDir: string): string {
  const absoluteRoot = resolve(root);
  const target = resolve(root, outputDir);
  const route = pathRelative(absoluteRoot, target);
  if (
    route === "" ||
    route === ".." ||
    route.startsWith(`..${sep}`) ||
    resolve(absoluteRoot, route) !== target
  ) {
    throw new Error(`unsafe generated modality directory '${outputDir}'`);
  }
  const realRoot = realpathSync(absoluteRoot);
  let current = absoluteRoot;
  for (const component of route.split(sep).filter(Boolean)) {
    current = join(current, component);
    const stat = lstatIfPresent(current);
    if (stat === undefined) break;
    if (stat.isSymbolicLink() || !stat.isDirectory()) {
      throw new Error(`generated modality directory '${current}' must be a real directory`);
    }
    const real = realpathSync(current);
    if (real !== realRoot && !real.startsWith(realRoot + sep)) {
      throw new Error(`generated modality directory '${current}' resolves outside '${root}'`);
    }
  }
  return target;
}

function recoveryPath(target: string): string {
  return `${target}.backup`;
}

function recoverInterruptedInstall(root: string, outputDir: string): void {
  const target = assertRealOutputDirectory(root, outputDir);
  const backup = recoveryPath(target);
  const backupStat = lstatIfPresent(backup);
  if (backupStat === undefined) return;
  if (backupStat.isSymbolicLink() || !backupStat.isDirectory()) {
    throw new Error(`modality recovery path '${backup}' must be a real directory`);
  }
  if (lstatIfPresent(target) === undefined) renameSync(backup, target);
}

function actualModalityOutputs(root: string, outputDir: string): Map<string, string> {
  const target = assertRealOutputDirectory(root, outputDir);
  const out = new Map<string, string>();
  if (!existsSync(target)) return out;
  for (const entry of readdirSync(target, { withFileTypes: true }).sort((left, right) =>
    left.name.localeCompare(right.name))) {
    const path = join(target, entry.name);
    if (entry.isSymbolicLink()) {
      throw new Error(`generated modality entry '${entry.name}' must not be a symbolic link`);
    }
    if (entry.isFile()) {
      if (!entry.name.endsWith(".json")) {
        throw new Error(`unexpected generated modality entry '${entry.name}'`);
      }
      out.set(`${outputDir}/${entry.name}`, readFileSync(path, "utf8"));
      continue;
    }
    if (!entry.isDirectory() || !entry.name.endsWith(".d")) {
      throw new Error(`unexpected generated modality entry '${entry.name}'`);
    }
    for (const owner of readdirSync(path, { withFileTypes: true }).sort((left, right) =>
      left.name.localeCompare(right.name))) {
      if (owner.isSymbolicLink() || !owner.isFile()) {
        throw new Error(
          `generated modality owner '${entry.name}/${owner.name}' must be a real direct-child file`,
        );
      }
      if (!owner.name.endsWith(".json")) {
        throw new Error(`unexpected generated modality owner '${entry.name}/${owner.name}'`);
      }
      out.set(
        `${outputDir}/${entry.name}/${owner.name}`,
        readFileSync(join(path, owner.name), "utf8"),
      );
    }
  }
  return out;
}

function validateLegacyInputs(
  root: string,
  outputDir: string,
  manifest: ModalityManifest,
): void {
  const target = assertRealOutputDirectory(root, outputDir);
  if (!existsSync(target)) return;
  const expected = splitLegacyAggregates(manifest);
  const entries = readdirSync(target, { withFileTypes: true });
  const aggregates = entries.filter((entry) => entry.name.endsWith(".json"));
  const ownerDirectories = entries.filter((entry) => entry.name.endsWith(".d"));
  for (const entry of entries) {
    if (entry.isSymbolicLink()) {
      throw new Error(`generated modality entry '${entry.name}' must not be a symbolic link`);
    }
    if (
      (entry.name.endsWith(".json") && !entry.isFile()) ||
      (entry.name.endsWith(".d") && !entry.isDirectory()) ||
      (!entry.name.endsWith(".json") && !entry.name.endsWith(".d"))
    ) {
      throw new Error(`unexpected generated modality entry '${entry.name}'`);
    }
  }
  for (const ownerDirectory of ownerDirectories) {
    const language = ownerDirectory.name.slice(0, -2);
    if (!expected.has(`${language}.json`)) {
      throw new Error(`unexpected modality owner directory '${ownerDirectory.name}'`);
    }
  }
  if (aggregates.length > 0 && ownerDirectories.length === 0) {
    const found = aggregates.map((entry) => entry.name).sort();
    const wanted = [...expected.keys()].sort();
    if (JSON.stringify(found) !== JSON.stringify(wanted)) {
      throw new Error("legacy modality aggregate languages are incomplete or unexpected");
    }
  }
  for (const aggregate of aggregates) {
    const wanted = expected.get(aggregate.name);
    if (wanted === undefined) throw new Error(`unexpected legacy modality aggregate '${aggregate.name}'`);
    if (readFileSync(join(target, aggregate.name), "utf8") !== wanted) {
      throw new Error(`legacy modality aggregate '${aggregate.name}' is not canonical or current`);
    }
  }
}

function compareOutputMaps(
  expected: ReadonlyMap<string, string>,
  actual: ReadonlyMap<string, string>,
): string[] {
  const diagnostics: string[] = [];
  for (const [path, contents] of expected) {
    if (!actual.has(path)) diagnostics.push(`${path}: generated output is missing`);
    else if (actual.get(path) !== contents) diagnostics.push(`${path}: generated output is stale`);
  }
  for (const path of actual.keys()) {
    if (!expected.has(path)) diagnostics.push(`${path}: unexpected stale output`);
  }
  return diagnostics;
}

function installGeneratedOwners(
  root: string,
  outputDir: string,
  outputs: ReadonlyMap<string, string>,
  manifest: ModalityManifest,
  languages: readonly string[],
  sourceIds: ReadonlyMap<string, readonly string[]>,
  narrationIds: ReadonlyMap<string, readonly string[]>,
  faultHooks: ModalityInstallFaultHooks = {},
): void {
  const target = assertRealOutputDirectory(root, outputDir);
  const stagingRoot = mkdtempSync(join(resolve(root), ".lesson-modality-stage-"));
  const backup = recoveryPath(target);
  let movedPrevious = false;
  let installed = false;
  let verifiedInstalled = false;
  try {
    for (const [relative, contents] of outputs) {
      const output = safeOutput(stagingRoot, relative);
      mkdirSync(dirname(output), { recursive: true });
      writeFileSync(output, contents, "utf8");
    }
    const staged = readModalityManifestOwners(stagingRoot, {
      expectedLanguages: languages,
      expectedLessonIds: sourceIds,
      expectedNarrationLessonIds: narrationIds,
    });
    if (serializeModalityManifest(staged) !== serializeModalityManifest(manifest)) {
      throw new Error("staged modality owners do not reconstruct the generated manifest");
    }
    mkdirSync(dirname(target), { recursive: true });
    if (lstatIfPresent(backup) !== undefined) {
      const current = readModalityManifestOwners(root, {
        expectedLanguages: languages,
        expectedLessonIds: sourceIds,
        expectedNarrationLessonIds: narrationIds,
      });
      if (serializeModalityManifest(current) !== serializeModalityManifest(manifest)) {
        throw new Error(`modality recovery backup is preserved at '${backup}'`);
      }
      rmSync(backup, { recursive: true, force: true });
    }
    if (lstatIfPresent(target) !== undefined) {
      renameSync(target, backup);
      movedPrevious = true;
      faultHooks.afterBackupMoved?.();
    }
    renameSync(resolve(stagingRoot, outputDir), target);
    installed = true;
    faultHooks.afterInstalled?.();
    const reloaded = readModalityManifestOwners(root, {
      expectedLanguages: languages,
      expectedLessonIds: sourceIds,
      expectedNarrationLessonIds: narrationIds,
    });
    if (serializeModalityManifest(reloaded) !== serializeModalityManifest(manifest)) {
      throw new Error("installed modality owners do not reconstruct the generated manifest");
    }
    verifiedInstalled = true;
    if (movedPrevious) rmSync(backup, { recursive: true, force: true });
  } catch (cause) {
    if (verifiedInstalled) throw cause;
    let recoveryFailure: unknown;
    if (installed && lstatIfPresent(target) !== undefined) {
      try {
        faultHooks.beforeInstalledRemoval?.();
        rmSync(target, { recursive: true, force: true });
      } catch (rollbackCause) {
        recoveryFailure = rollbackCause;
      }
    }
    if (
      recoveryFailure === undefined &&
      movedPrevious &&
      lstatIfPresent(backup) !== undefined
    ) {
      try {
        faultHooks.beforeBackupRestore?.();
        renameSync(backup, target);
      } catch (rollbackCause) {
        recoveryFailure = rollbackCause;
      }
    }
    if (recoveryFailure !== undefined) {
      throw new Error(
        `modality install failed and automatic recovery failed; backup preserved at '${backup}'`,
        { cause: recoveryFailure },
      );
    }
    throw cause;
  } finally {
    rmSync(stagingRoot, { recursive: true, force: true });
  }
}

export function runModalityManifest(
  args = process.argv.slice(2),
  root = defaultCurriculumRoot(),
  faultHooks: ModalityInstallFaultHooks = {},
): number {
  const mode = args.length === 1 ? args[0] : undefined;
  if (mode !== "--check" && mode !== "--write") {
    process.stderr.write("usage: modality-cli (--check | --write)\n");
    return 2;
  }
  try {
    const lessons = loadLessons(root);
    const manifest = buildModalityManifest(lessons);
    const languages = loadLanguageRegistry(root).languages.map((language) => language.id);
    assertModalityManifestLanguages(manifest, languages);
    const sourceIds = expectedLessonIds(manifest);
    const narrationIds = modalityNarrationLessonIds(root, languages);
    const outputs = generatedModalityOutputsFromLessons(lessons);

    if (mode === "--write") {
      recoverInterruptedInstall(root, MODALITY_MANIFEST_DIR);
      validateLegacyInputs(root, MODALITY_MANIFEST_DIR, manifest);
      installGeneratedOwners(
        root,
        MODALITY_MANIFEST_DIR,
        outputs,
        manifest,
        languages,
        sourceIds,
        narrationIds,
        faultHooks,
      );
      process.stdout.write(
        `generated ${MODALITY_MANIFEST_DIR} (${outputs.size} direct owners)\n`,
      );
      return 0;
    }

    const diagnostics = compareOutputMaps(
      outputs,
      actualModalityOutputs(root, MODALITY_MANIFEST_DIR),
    );
    if (diagnostics.length === 0) {
      const loaded = readModalityManifestOwners(root, {
        expectedLanguages: languages,
        expectedLessonIds: sourceIds,
        expectedNarrationLessonIds: narrationIds,
      });
      if (serializeModalityManifest(loaded) !== serializeModalityManifest(manifest)) {
        diagnostics.push("modality direct owners do not reconstruct the lesson-derived manifest");
      }
    }
    for (const diagnostic of diagnostics) process.stderr.write(`${diagnostic}\n`);
    return diagnostics.length === 0 ? 0 : 1;
  } catch (cause) {
    process.stderr.write(`${cause instanceof Error ? cause.message : String(cause)}\n`);
    return 1;
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exit(runModalityManifest());
}
