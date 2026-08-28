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
import { pathToFileURL } from "node:url";
import { narrationLessonIdentityIndex } from "./generated-hash-shards.js";
import {
  GENTLE_RAMP_FINDINGS_DIR,
  GENTLE_RAMP_META_OWNER,
  GENTLE_RAMP_METRIC_OWNERS,
  GENTLE_RAMP_METRICS_DIR,
  GENTLE_RAMP_SNAPSHOT_DIR,
  gentleRampOwnerContents,
  readGentleRampOwners,
} from "./gentle-ramp-shards.js";
import type { GentleRampReport, TrackGentleRamp } from "./gentle-ramp.js";
import { GENTLE_RAMP_PRIORITIES } from "./gentle-ramp.js";
import {
  defaultCurriculumRoot,
  loadChapterPolicy,
  loadEverything,
  loadLanguageRegistry,
  loadTrackChapters,
} from "./loader.js";
import { assertRelativeManifestPath } from "./manifest-path.js";
import type { ParsedLesson } from "./parse.js";
import { buildCurriculumGapReport } from "./report.js";

export { GENTLE_RAMP_SNAPSHOT_DIR } from "./gentle-ramp-shards.js";

const LANGUAGE = /^[a-z][a-z0-9-]*$/;
const WINDOWS_RESERVED = /^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])$/i;
const DANGEROUS_IDENTITIES = new Set(["__proto__", "constructor", "prototype"]);

function safeLanguage(value: string): boolean {
  return LANGUAGE.test(value) && !WINDOWS_RESERVED.test(value) &&
    !DANGEROUS_IDENTITIES.has(value.toLowerCase());
}

export interface GentleRampInstallFaultHooks {
  afterBackupMoved?: () => void;
  afterInstalled?: () => void;
  beforeInstalledRemoval?: () => void;
  beforeBackupRestore?: () => void;
}

function lstatIfPresent(path: string): ReturnType<typeof lstatSync> | undefined {
  try {
    return lstatSync(path);
  } catch (cause) {
    if ((cause as NodeJS.ErrnoException).code === "ENOENT") return undefined;
    throw cause;
  }
}

function isExpectedOwnerPath(relative: string): boolean {
  const parts = relative.split("/");
  if (parts.length === 4 && parts[0] === "core" && parts[1] === "gentle-ramp-snapshots") {
    const languageDirectory = parts[2]!;
    const language = languageDirectory.endsWith(".d") ? languageDirectory.slice(0, -2) : "";
    return safeLanguage(language) && parts[3] === GENTLE_RAMP_META_OWNER;
  }
  if (parts.length !== 5 || parts[0] !== "core" || parts[1] !== "gentle-ramp-snapshots") return false;
  const languageDirectory = parts[2]!;
  const language = languageDirectory.endsWith(".d") ? languageDirectory.slice(0, -2) : "";
  if (!safeLanguage(language)) return false;
  const stem = parts[4]!.endsWith(".json") ? parts[4]!.slice(0, -5) : "";
  return (parts[3] === GENTLE_RAMP_METRICS_DIR &&
      (GENTLE_RAMP_METRIC_OWNERS as readonly string[]).includes(stem)) ||
    (parts[3] === GENTLE_RAMP_FINDINGS_DIR && GENTLE_RAMP_PRIORITIES.includes(stem as never));
}

/** Resolve exactly one fixed owner path without permitting a legacy aggregate. */
export function safeGentleRampOwnerOutput(root: string, relative: string): string {
  assertRelativeManifestPath(relative, `unsafe gentle-ramp owner output '${relative}'`);
  if (!isExpectedOwnerPath(relative)) throw new Error(`unsafe gentle-ramp owner output '${relative}'`);
  const output = resolve(root, relative);
  const fromRoot = normalize(pathRelative(resolve(root), output)).replaceAll("\\", "/");
  if (fromRoot === "" || fromRoot === ".." || fromRoot.startsWith("../") || fromRoot !== relative) {
    throw new Error(`unsafe gentle-ramp owner output '${relative}'`);
  }
  return output;
}

export function serializeGentleRampTrack(track: TrackGentleRamp): string {
  return `${JSON.stringify(track, null, 2)}\n`;
}

function sorted(values: readonly string[]): string[] {
  return [...values].sort((left, right) => left < right ? -1 : left > right ? 1 : 0);
}

function assertExactIdentities(actual: readonly string[], expected: readonly string[], label: string): void {
  const found = sorted(actual);
  const wanted = sorted(expected);
  if (found.length !== wanted.length || found.some((value, index) => value !== wanted[index])) {
    const missing = wanted.filter((value) => !found.includes(value));
    const extra = found.filter((value) => !wanted.includes(value));
    throw new Error(
      `${label} do not match` +
      `${missing.length > 0 ? `; missing: ${missing.join(", ")}` : ""}` +
      `${extra.length > 0 ? `; extra: ${extra.join(", ")}` : ""}`,
    );
  }
}

function assertReportLanguages(report: GentleRampReport, languages: readonly string[]): void {
  assertExactIdentities(report.tracks.map((track) => track.language), languages, "gentle-ramp report languages");
  const folded = new Set<string>();
  for (const language of report.tracks.map((track) => track.language)) {
    const identity = language.toLowerCase();
    if (folded.has(identity)) throw new Error(`gentle-ramp report repeats language '${language}'`);
    folded.add(identity);
  }
}

/** The 851 current direct-owner bytes, without touching disk. */
export function generatedGentleRampSnapshotOutputsFromReport(
  report: GentleRampReport,
): Map<string, string> {
  return new Map(
    [...gentleRampOwnerContents(report.tracks)].map(([relative, contents]) => [
      `${GENTLE_RAMP_SNAPSHOT_DIR}/${relative}`,
      contents,
    ]),
  );
}

function lessonIdsByLanguage(
  lessons: readonly ParsedLesson[],
  languages: readonly string[],
): ReadonlyMap<string, readonly string[]> {
  const out = new Map(languages.map((language) => [language, [] as string[]]));
  for (const lesson of lessons) {
    const ids = out.get(lesson.language);
    const id = lesson.frontmatter.id;
    if (typeof id !== "string" || id.length === 0) {
      throw new Error(`lesson in '${lesson.language}' has no usable id`);
    }
    if (ids === undefined) throw new Error(`lesson '${id}' has unregistered language '${lesson.language}'`);
    ids.push(id);
  }
  for (const ids of out.values()) ids.sort();
  return out;
}

export interface GentleRampGeneration {
  report: GentleRampReport;
  languages: string[];
  sourceIds: ReadonlyMap<string, readonly string[]>;
  narrationIds: ReadonlyMap<string, readonly string[]>;
  outputs: Map<string, string>;
}

function generation(root: string): GentleRampGeneration {
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
  const languages = registry.languages.map((language) => language.id);
  assertReportLanguages(report, languages);
  const sourceIds = lessonIdsByLanguage(lessons, languages);
  const narrationIds = narrationLessonIdentityIndex(root, languages);
  return {
    report,
    languages,
    sourceIds,
    narrationIds,
    outputs: generatedGentleRampSnapshotOutputsFromReport(report),
  };
}

export function generatedGentleRampSnapshotOutputs(
  root = defaultCurriculumRoot(),
): Map<string, string> {
  return generation(root).outputs;
}

function assertReconstruction(
  root: string,
  generated: GentleRampGeneration,
): TrackGentleRamp[] {
  const tracks = readGentleRampOwners(root, {
    expectedLanguages: generated.languages,
    expectedLessonIds: generated.sourceIds,
    expectedNarrationLessonIds: generated.narrationIds,
  });
  const expected = new Map(generated.report.tracks.map((track) => [track.language, serializeGentleRampTrack(track)]));
  for (const track of tracks) {
    if (serializeGentleRampTrack(track) !== expected.get(track.language)) {
      throw new Error(`gentle-ramp owners for '${track.language}' do not reconstruct the generated track`);
    }
  }
  return tracks;
}

function assertRealOutputDirectory(root: string): string {
  const absoluteRoot = resolve(root);
  const target = resolve(root, GENTLE_RAMP_SNAPSHOT_DIR);
  const route = pathRelative(absoluteRoot, target);
  if (route === "" || route === ".." || route.startsWith(`..${sep}`) || resolve(absoluteRoot, route) !== target) {
    throw new Error(`unsafe gentle-ramp snapshot directory '${GENTLE_RAMP_SNAPSHOT_DIR}'`);
  }
  const realRoot = realpathSync(absoluteRoot);
  let current = absoluteRoot;
  for (const component of route.split(sep).filter(Boolean)) {
    current = join(current, component);
    const stat = lstatIfPresent(current);
    if (stat === undefined) break;
    if (stat.isSymbolicLink() || !stat.isDirectory()) {
      throw new Error(`gentle-ramp snapshot directory '${current}' must be a real directory`);
    }
    const real = realpathSync(current);
    if (real !== realRoot && !real.startsWith(realRoot + sep)) {
      throw new Error(`gentle-ramp snapshot directory '${current}' resolves outside '${root}'`);
    }
  }
  return target;
}

function validateExistingForMigration(root: string, generated: GentleRampGeneration): void {
  const target = assertRealOutputDirectory(root);
  if (!existsSync(target)) return;
  const entries = readdirSync(target, { withFileTypes: true });
  for (const entry of entries) {
    if (entry.isSymbolicLink()) throw new Error(`gentle-ramp migration entry '${entry.name}' must not be a symbolic link`);
  }
  const aggregates = entries.filter((entry) => entry.name.endsWith(".json"));
  const ownerDirectories = entries.filter((entry) => entry.name.endsWith(".d"));
  if (aggregates.length > 0 && ownerDirectories.length > 0) {
    throw new Error("gentle-ramp migration refuses a mixed aggregate/direct-owner tree");
  }
  if (aggregates.length > 0) {
    if (entries.some((entry) => !entry.name.endsWith(".json") || !entry.isFile())) {
      throw new Error("legacy gentle-ramp snapshot directory has unexpected entries");
    }
    assertExactIdentities(
      aggregates.map((entry) => entry.name.slice(0, -5)),
      generated.languages,
      "legacy gentle-ramp aggregate languages",
    );
    const expected = new Map(generated.report.tracks.map((track) => [track.language, serializeGentleRampTrack(track)]));
    for (const aggregate of aggregates) {
      const language = aggregate.name.slice(0, -5);
      if (readFileSync(join(target, aggregate.name), "utf8") !== expected.get(language)) {
        throw new Error(`legacy gentle-ramp aggregate '${aggregate.name}' is not canonical or current`);
      }
    }
    return;
  }
  if (entries.length > 0) assertReconstruction(root, generated);
}

function recoveryPath(target: string): string {
  return `${target}.backup`;
}

function recoverInterruptedInstall(root: string, generated: GentleRampGeneration): void {
  const target = assertRealOutputDirectory(root);
  const backup = recoveryPath(target);
  const backupStat = lstatIfPresent(backup);
  if (backupStat === undefined) return;
  if (backupStat.isSymbolicLink() || !backupStat.isDirectory()) {
    throw new Error(`gentle-ramp recovery path '${backup}' must be a real directory`);
  }
  if (lstatIfPresent(target) === undefined) {
    renameSync(backup, target);
    try {
      assertReconstruction(root, generated);
    } catch (cause) {
      renameSync(target, backup);
      throw new Error(
        `gentle-ramp recovery backup at '${backup}' is invalid; canonical path remains absent`,
        { cause },
      );
    }
    return;
  }
  try {
    assertReconstruction(root, generated);
  } catch (cause) {
    throw new Error(
      `gentle-ramp recovery is ambiguous; preserving both '${target}' and '${backup}'`,
      { cause },
    );
  }
  rmSync(backup, { recursive: true, force: true });
}

export function installGentleRampOwnerTree(
  root: string,
  generated: GentleRampGeneration,
  faultHooks: GentleRampInstallFaultHooks,
): void {
  const target = assertRealOutputDirectory(root);
  const stagingRoot = mkdtempSync(join(resolve(root), ".gentle-ramp-stage-"));
  const backup = recoveryPath(target);
  let movedPrevious = false;
  let installed = false;
  let verified = false;
  try {
    for (const [relative, contents] of generated.outputs) {
      const output = safeGentleRampOwnerOutput(stagingRoot, relative);
      mkdirSync(dirname(output), { recursive: true });
      writeFileSync(output, contents, { encoding: "utf8", flag: "wx" });
    }
    assertReconstruction(stagingRoot, generated);
    mkdirSync(dirname(target), { recursive: true });
    if (lstatIfPresent(target) !== undefined) {
      renameSync(target, backup);
      movedPrevious = true;
      faultHooks.afterBackupMoved?.();
    }
    renameSync(resolve(stagingRoot, GENTLE_RAMP_SNAPSHOT_DIR), target);
    installed = true;
    faultHooks.afterInstalled?.();
    assertReconstruction(root, generated);
    verified = true;
    if (movedPrevious) rmSync(backup, { recursive: true, force: true });
  } catch (cause) {
    if (verified) throw cause;
    let recoveryFailure: unknown;
    if (installed && lstatIfPresent(target) !== undefined) {
      try {
        faultHooks.beforeInstalledRemoval?.();
        rmSync(target, { recursive: true, force: true });
      } catch (rollbackCause) {
        recoveryFailure = rollbackCause;
      }
    }
    if (recoveryFailure === undefined && movedPrevious && lstatIfPresent(backup) !== undefined) {
      try {
        faultHooks.beforeBackupRestore?.();
        renameSync(backup, target);
      } catch (rollbackCause) {
        recoveryFailure = rollbackCause;
      }
    }
    if (recoveryFailure !== undefined) {
      throw new Error(`gentle-ramp install failed and recovery failed; backup preserved at '${backup}'`, {
        cause: recoveryFailure,
      });
    }
    throw cause;
  } finally {
    rmSync(stagingRoot, { recursive: true, force: true });
  }
}

export function runGentleRampSnapshots(
  args = process.argv.slice(2),
  root = defaultCurriculumRoot(),
  faultHooks: GentleRampInstallFaultHooks = {},
): number {
  const mode = args.length === 1 ? args[0] : undefined;
  if (mode !== "--check" && mode !== "--write") {
    process.stderr.write("usage: gentle-ramp-snapshot-cli (--check | --write)\n");
    return 2;
  }
  try {
    // Keep registry loading explicit at the filesystem boundary: a malformed or
    // duplicate registry must fail before generated owners can define identities.
    loadLanguageRegistry(root);
    const generated = generation(root);
    if (mode === "--write") {
      recoverInterruptedInstall(root, generated);
      validateExistingForMigration(root, generated);
      installGentleRampOwnerTree(root, generated, faultHooks);
      process.stdout.write(
        `generated ${GENTLE_RAMP_SNAPSHOT_DIR} (${generated.outputs.size} direct owners)\n`,
      );
      return 0;
    }
    assertReconstruction(root, generated);
    return 0;
  } catch (cause) {
    process.stderr.write(`${cause instanceof Error ? cause.message : String(cause)}\n`);
    return 1;
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exit(runGentleRampSnapshots());
}
