import { lstatSync, mkdirSync, readdirSync, realpathSync } from "node:fs";
import { basename, dirname, join, relative, resolve, sep } from "node:path";
import { assertRelativeManifestPath } from "./manifest-path.js";
import { isAbsentErrno, readLedgerFile } from "./shard.js";

export const GENERATED_BOOK_HASH_DIR = "core/generated-book-hashes";
export const GENERATED_NARRATION_HASH_DIR = "core/generated-narration-hashes";

const LANGUAGE = /^[a-z][a-z0-9-]*$/;
const CHAPTER_OWNER = /^(\d{4})\.json$/;
const HASH = /^fnv1a64:[0-9a-f]{16}$/;

export interface GeneratedBookHashChapter {
  language: string;
  chapter: number;
  sourceHash: string;
  lessonIds: string[];
  tex: string;
}

export interface GeneratedBookHashManifest {
  version: 1;
  algorithm: "fnv1a64";
  chapters: GeneratedBookHashChapter[];
}

export interface GeneratedNarrationFinding {
  code: string;
  lessonId: string;
  language: string;
  message: string;
}

export interface GeneratedNarrationHashChapter {
  language: string;
  chapter: number;
  sourceHash: string;
  lessonIds: string[];
  voiceLessons: number;
  drivablePrefix: number;
  text: string;
  json: string;
  textHash: string;
  jsonHash: string;
}

export interface GeneratedNarrationChapterOwner extends GeneratedNarrationHashChapter {
  findings: GeneratedNarrationFinding[];
}

export interface GeneratedNarrationHashManifest {
  version: 1;
  algorithm: "fnv1a64";
  maxLinearisableTableColumns: number;
  chapters: GeneratedNarrationHashChapter[];
  findings: GeneratedNarrationFinding[];
}

export interface LoadedGeneratedHashManifest<T> {
  manifest: T;
  /** Exact files used for this read, suitable for a build tool's watch list. */
  sourcePaths: string[];
  sharded: boolean;
}

export interface GeneratedHashReadOptions {
  /** Temporary migration/testing escape hatch. Canonical consumers must omit this. */
  allowLegacyMonolith?: boolean;
}

interface BookMeta {
  version: 1;
  language: string;
  algorithm: "fnv1a64";
}

interface NarrationMeta extends BookMeta {
  maxLinearisableTableColumns: number;
}

function statIfPresent(path: string): ReturnType<typeof lstatSync> | undefined {
  try {
    return lstatSync(path);
  } catch (cause) {
    const code = (cause as NodeJS.ErrnoException).code;
    if (isAbsentErrno(code)) return undefined;
    throw new Error(
      `generated hash owner '${path}': cannot be inspected (${code ?? "unknown error"})`,
      {
        cause,
      },
    );
  }
}

function assertRealDescendantComponents(root: string, target: string): void {
  const absoluteRoot = resolve(root);
  const absoluteTarget = resolve(target);
  const route = relative(absoluteRoot, absoluteTarget);
  if (
    route === ".." ||
    route.startsWith(`..${sep}`) ||
    resolve(absoluteRoot, route) !== absoluteTarget
  ) {
    throw new Error(`generated hash path '${target}' is outside '${root}'`);
  }
  const realRoot = realpathSync(absoluteRoot);
  let current = absoluteRoot;
  for (const component of route.split(sep).filter(Boolean)) {
    current = join(current, component);
    const stat = lstatSync(current);
    if (stat.isSymbolicLink()) {
      throw new Error(
        `generated hash path component '${current}' must not be a symbolic link`,
      );
    }
    const real = realpathSync(current);
    if (real !== realRoot && !real.startsWith(realRoot + sep)) {
      throw new Error(
        `generated hash path component '${current}' resolves outside '${root}'`,
      );
    }
  }
}

function curriculumRootForManifest(monolithPath: string): string {
  return dirname(dirname(dirname(resolve(monolithPath))));
}

function languageFromMonolith(monolithPath: string): string {
  const name = basename(monolithPath);
  if (!name.endsWith(".json")) {
    throw new Error(
      `generated hash manifest '${monolithPath}' must end in .json`,
    );
  }
  const language = name.slice(0, -".json".length);
  if (!LANGUAGE.test(language)) {
    throw new Error(`unsafe generated hash manifest language '${language}'`);
  }
  return language;
}

export function generatedHashShardDirectory(monolithPath: string): string {
  languageFromMonolith(monolithPath);
  return `${monolithPath.slice(0, -".json".length)}.d`;
}

export function generatedHashChapterFilename(chapter: number): string {
  if (!Number.isInteger(chapter) || chapter < 1 || chapter > 9999) {
    throw new Error(
      `generated hash chapter must be an integer from 1 through 9999`,
    );
  }
  return `${String(chapter).padStart(4, "0")}.json`;
}

/**
 * Resolve one generated-hash write without following a symlinked owner directory
 * or owner file. Missing real directories are created one component at a time.
 */
export function prepareGeneratedHashOwnerWrite(
  root: string,
  relativeDirectory: string,
  relative: string,
): string {
  assertRelativeManifestPath(
    relative,
    `unsafe generated hash owner '${relative}'`,
  );
  const escapedDirectory = relativeDirectory.replace(
    /[.*+?^${}()|[\]\\]/g,
    "\\$&",
  );
  const pattern = new RegExp(
    `^${escapedDirectory}/([a-z][a-z0-9-]*)\\.d/(_meta|\\d{4})\\.json$`,
  );
  if (pattern.exec(relative) === null) {
    throw new Error(`unsafe generated hash owner '${relative}'`);
  }
  const realRoot = realpathSync(root);
  const components = relative.split("/");
  let parent = resolve(root);
  for (const component of components.slice(0, -1)) {
    parent = join(parent, component);
    const stat = statIfPresent(parent);
    if (stat === undefined) mkdirSync(parent);
    else if (stat.isSymbolicLink() || !stat.isDirectory()) {
      throw new Error(
        `generated hash owner directory '${parent}' must be a real directory`,
      );
    }
    const realParent = realpathSync(parent);
    if (realParent !== realRoot && !realParent.startsWith(realRoot + sep)) {
      throw new Error(
        `generated hash owner directory '${parent}' resolves outside the curriculum root`,
      );
    }
  }
  const output = join(parent, components.at(-1)!);
  const outputStat = statIfPresent(output);
  if (outputStat && (outputStat.isSymbolicLink() || !outputStat.isFile())) {
    throw new Error(
      `generated hash owner '${output}' must be a real regular file`,
    );
  }
  return output;
}

function object(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`${label} must contain one JSON object`);
  }
  return value as Record<string, unknown>;
}

function exactKeys(
  value: Record<string, unknown>,
  expected: readonly string[],
  label: string,
): void {
  const actual = Object.keys(value).sort();
  const wanted = [...expected].sort();
  if (
    actual.length !== wanted.length ||
    actual.some((key, index) => key !== wanted[index])
  ) {
    throw new Error(`${label} must contain exactly: ${wanted.join(", ")}`);
  }
}

function string(value: unknown, field: string, label: string): string {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`${label}.${field} must be a non-empty string`);
  }
  return value;
}

function integer(
  value: unknown,
  field: string,
  label: string,
  minimum = 0,
): number {
  if (
    typeof value !== "number" ||
    !Number.isInteger(value) ||
    value < minimum
  ) {
    throw new Error(`${label}.${field} must be an integer >= ${minimum}`);
  }
  return value;
}

function tableWidth(value: unknown, field: string, label: string): number {
  const width = integer(value, field, label);
  if (width > 16) {
    throw new Error(`${label}.${field} must be an integer from 0 through 16`);
  }
  return width;
}

function strings(value: unknown, field: string, label: string): string[] {
  if (
    !Array.isArray(value) ||
    value.some((entry) => typeof entry !== "string" || entry.length === 0)
  ) {
    throw new Error(`${label}.${field} must be an array of non-empty strings`);
  }
  if (new Set(value).size !== value.length) {
    throw new Error(`${label}.${field} must not contain duplicates`);
  }
  return value as string[];
}

function hash(value: unknown, field: string, label: string): string {
  const result = string(value, field, label);
  if (!HASH.test(result))
    throw new Error(`${label}.${field} must be an fnv1a64 hash`);
  return result;
}

function relativePath(
  value: unknown,
  field: string,
  label: string,
  suffix: string,
): string {
  const result = string(value, field, label);
  assertRelativeManifestPath(result, `${label}.${field} is unsafe`);
  if (
    result
      .split(/[\\/]/)
      .some((part) => part === ".." || part === "." || part === "")
  ) {
    throw new Error(`${label}.${field} is unsafe`);
  }
  if (!result.endsWith(suffix))
    throw new Error(`${label}.${field} must end in ${suffix}`);
  return result;
}

function validateMeta(
  value: unknown,
  language: string,
  narration: false,
): BookMeta;
function validateMeta(
  value: unknown,
  language: string,
  narration: true,
): NarrationMeta;
function validateMeta(
  value: unknown,
  language: string,
  narration: boolean,
): BookMeta | NarrationMeta {
  const label = `${language}.d/_meta.json`;
  const record = object(value, label);
  exactKeys(
    record,
    narration
      ? ["version", "language", "algorithm", "maxLinearisableTableColumns"]
      : ["version", "language", "algorithm"],
    label,
  );
  if (
    record.version !== 1 ||
    record.algorithm !== "fnv1a64" ||
    record.language !== language
  ) {
    throw new Error(
      `${label} must declare version 1, language '${language}', and fnv1a64`,
    );
  }
  if (narration) {
    return {
      version: 1,
      language,
      algorithm: "fnv1a64",
      maxLinearisableTableColumns: tableWidth(
        record.maxLinearisableTableColumns,
        "maxLinearisableTableColumns",
        label,
      ),
    };
  }
  return { version: 1, language, algorithm: "fnv1a64" };
}

const BOOK_CHAPTER_KEYS = [
  "language",
  "chapter",
  "sourceHash",
  "lessonIds",
  "tex",
] as const;
const NARRATION_CHAPTER_KEYS = [
  "language",
  "chapter",
  "sourceHash",
  "lessonIds",
  "voiceLessons",
  "drivablePrefix",
  "text",
  "json",
  "textHash",
  "jsonHash",
] as const;
const FINDING_KEYS = ["code", "lessonId", "language", "message"] as const;

function compareFindings(
  left: GeneratedNarrationFinding,
  right: GeneratedNarrationFinding,
): number {
  return (
    left.lessonId.localeCompare(right.lessonId) ||
    left.code.localeCompare(right.code)
  );
}

function validateBookChapter(
  value: unknown,
  language: string,
  ownerChapter: number,
  label: string,
): GeneratedBookHashChapter {
  const record = object(value, label);
  exactKeys(record, BOOK_CHAPTER_KEYS, label);
  const chapter = integer(record.chapter, "chapter", label, 1);
  if (chapter !== ownerChapter || record.language !== language) {
    throw new Error(`${label} must own ${language} chapter ${ownerChapter}`);
  }
  return {
    language,
    chapter,
    sourceHash: hash(record.sourceHash, "sourceHash", label),
    lessonIds: strings(record.lessonIds, "lessonIds", label),
    tex: relativePath(record.tex, "tex", label, ".tex"),
  };
}

function validateFinding(
  value: unknown,
  language: string,
  label: string,
): GeneratedNarrationFinding {
  const record = object(value, label);
  exactKeys(record, FINDING_KEYS, label);
  if (record.language !== language)
    throw new Error(`${label}.language must be '${language}'`);
  return {
    code: string(record.code, "code", label),
    lessonId: string(record.lessonId, "lessonId", label),
    language,
    message: string(record.message, "message", label),
  };
}

function validateNarrationOwner(
  value: unknown,
  language: string,
  ownerChapter: number,
  label: string,
): GeneratedNarrationChapterOwner {
  const record = object(value, label);
  exactKeys(record, [...NARRATION_CHAPTER_KEYS, "findings"], label);
  const chapter = integer(record.chapter, "chapter", label, 1);
  if (chapter !== ownerChapter || record.language !== language) {
    throw new Error(`${label} must own ${language} chapter ${ownerChapter}`);
  }
  if (!Array.isArray(record.findings))
    throw new Error(`${label}.findings must be an array`);
  const lessonIds = strings(record.lessonIds, "lessonIds", label);
  const findings = record.findings.map((finding, index) =>
    validateFinding(finding, language, `${label}.findings[${index}]`),
  );
  const sortedFindings = [...findings].sort(compareFindings);
  if (findings.some((finding, index) => finding !== sortedFindings[index])) {
    throw new Error(`${label}.findings must be sorted by lessonId then code`);
  }
  for (const finding of findings) {
    if (!lessonIds.includes(finding.lessonId)) {
      throw new Error(
        `${label}: finding for '${finding.lessonId}' is not owned by this chapter`,
      );
    }
  }
  return {
    language,
    chapter,
    sourceHash: hash(record.sourceHash, "sourceHash", label),
    lessonIds,
    voiceLessons: integer(record.voiceLessons, "voiceLessons", label),
    drivablePrefix: integer(record.drivablePrefix, "drivablePrefix", label),
    text: relativePath(record.text, "text", label, ".txt"),
    json: relativePath(record.json, "json", label, ".json"),
    textHash: hash(record.textHash, "textHash", label),
    jsonHash: hash(record.jsonHash, "jsonHash", label),
    findings,
  };
}

interface ShardedFiles {
  language: string;
  directory: string;
  names: string[];
  sourcePaths: string[];
}

function shardedFiles(monolithPath: string): ShardedFiles | undefined {
  const language = languageFromMonolith(monolithPath);
  const directory = generatedHashShardDirectory(monolithPath);
  assertRealDescendantComponents(
    curriculumRootForManifest(monolithPath),
    dirname(monolithPath),
  );
  const directoryStat = statIfPresent(directory);
  if (directoryStat === undefined) return undefined;
  if (directoryStat.isSymbolicLink() || !directoryStat.isDirectory()) {
    throw new Error(
      `generated hash owner '${directory}' must be a real directory`,
    );
  }
  if (statIfPresent(monolithPath) !== undefined) {
    throw new Error(
      `generated hash manifest '${monolithPath}' is a resurrected monolith beside '${basename(directory)}'`,
    );
  }
  const names = readdirSync(directory).sort((left, right) =>
    left < right ? -1 : left > right ? 1 : 0,
  );
  if (!names.includes("_meta.json"))
    throw new Error(`${directory}/_meta.json is missing`);
  const sourcePaths: string[] = [];
  const ordinals = new Set<number>();
  for (const name of names) {
    if (name !== "_meta.json" && CHAPTER_OWNER.exec(name) === null) {
      throw new Error(
        `generated hash owner '${join(directory, name)}' has a malformed name`,
      );
    }
    const path = join(directory, name);
    const stat = lstatSync(path);
    if (stat.isSymbolicLink() || !stat.isFile()) {
      throw new Error(
        `generated hash owner '${path}' must be a real regular file`,
      );
    }
    if (name !== "_meta.json") {
      const chapter = Number(CHAPTER_OWNER.exec(name)![1]);
      if (chapter < 1)
        throw new Error(`generated hash owner '${path}' has invalid chapter 0`);
      if (ordinals.has(chapter))
        throw new Error(
          `generated hash owner '${path}' duplicates chapter ${chapter}`,
        );
      ordinals.add(chapter);
    }
    sourcePaths.push(path);
  }
  if (sourcePaths.length === 1)
    throw new Error(`${directory} has metadata but no chapter owners`);
  return { language, directory, names, sourcePaths };
}

function validateBookManifest(
  value: unknown,
  language: string,
  label: string,
): GeneratedBookHashManifest {
  const record = object(value, label);
  exactKeys(record, ["version", "algorithm", "chapters"], label);
  if (
    record.version !== 1 ||
    record.algorithm !== "fnv1a64" ||
    !Array.isArray(record.chapters)
  ) {
    throw new Error(`${label} must declare version 1, fnv1a64, and chapters[]`);
  }
  const seen = new Set<number>();
  const chapters = record.chapters.map((chapter, index) => {
    const raw = object(chapter, `${label}.chapters[${index}]`);
    const number = raw.chapter;
    if (typeof number !== "number")
      throw new Error(`${label}.chapters[${index}].chapter must be a number`);
    const parsed = validateBookChapter(
      chapter,
      language,
      number,
      `${label}.chapters[${index}]`,
    );
    if (seen.has(parsed.chapter))
      throw new Error(`${label} duplicates chapter ${parsed.chapter}`);
    seen.add(parsed.chapter);
    return parsed;
  });
  return { version: 1, algorithm: "fnv1a64", chapters };
}

function validateNarrationManifest(
  value: unknown,
  language: string,
  label: string,
): GeneratedNarrationHashManifest {
  const record = object(value, label);
  exactKeys(
    record,
    [
      "version",
      "algorithm",
      "maxLinearisableTableColumns",
      "chapters",
      "findings",
    ],
    label,
  );
  if (
    record.version !== 1 ||
    record.algorithm !== "fnv1a64" ||
    !Array.isArray(record.chapters) ||
    !Array.isArray(record.findings)
  ) {
    throw new Error(`${label} must declare the narration v1 manifest shape`);
  }
  const width = tableWidth(
    record.maxLinearisableTableColumns,
    "maxLinearisableTableColumns",
    label,
  );
  const seenChapters = new Set<number>();
  const lessonOwners = new Map<string, number>();
  const chapters = record.chapters.map((chapter, index) => {
    const chapterLabel = `${label}.chapters[${index}]`;
    const raw = object(chapter, chapterLabel);
    const number = raw.chapter;
    if (typeof number !== "number") {
      throw new Error(`${chapterLabel}.chapter must be a number`);
    }
    const owner = validateNarrationOwner(
      { ...raw, findings: [] },
      language,
      number,
      chapterLabel,
    );
    if (seenChapters.has(owner.chapter)) {
      throw new Error(`${label} duplicates chapter ${owner.chapter}`);
    }
    seenChapters.add(owner.chapter);
    for (const lessonId of owner.lessonIds) {
      const previous = lessonOwners.get(lessonId);
      if (previous !== undefined) {
        throw new Error(
          `${label}: lesson '${lessonId}' is owned by both chapter ${previous} and chapter ${owner.chapter}`,
        );
      }
      lessonOwners.set(lessonId, owner.chapter);
    }
    const { findings: _findings, ...chapterRecord } = owner;
    return chapterRecord;
  });
  const findings = record.findings.map((finding, index) =>
    validateFinding(finding, language, `${label}.findings[${index}]`),
  );
  for (const finding of findings) {
    if (!lessonOwners.has(finding.lessonId)) {
      throw new Error(
        `${label}: finding for '${finding.lessonId}' has no chapter owner`,
      );
    }
  }
  return {
    version: 1,
    algorithm: "fnv1a64",
    maxLinearisableTableColumns: width,
    chapters,
    findings,
  };
}

export function readGeneratedBookHashManifest(
  monolithPath: string,
  options: GeneratedHashReadOptions = {},
): LoadedGeneratedHashManifest<GeneratedBookHashManifest> {
  const files = shardedFiles(monolithPath);
  if (files === undefined) {
    if (!options.allowLegacyMonolith) {
      throw new Error(
        `generated book hash manifest '${monolithPath}' must use a chapter-owned .d directory`,
      );
    }
    return {
      manifest: validateBookManifest(
        readLedgerFile<unknown>(monolithPath),
        languageFromMonolith(monolithPath),
        monolithPath,
      ),
      sourcePaths: [monolithPath],
      sharded: false,
    };
  }
  const meta = validateMeta(
    readLedgerFile<unknown>(join(files.directory, "_meta.json")),
    files.language,
    false,
  );
  const chapters = files.names
    .filter((name) => name !== "_meta.json")
    .map((name) =>
      validateBookChapter(
        readLedgerFile<unknown>(join(files.directory, name)),
        files.language,
        Number(CHAPTER_OWNER.exec(name)![1]),
        `${files.language}.d/${name}`,
      ),
    );
  return {
    manifest: { version: meta.version, algorithm: meta.algorithm, chapters },
    sourcePaths: files.sourcePaths,
    sharded: true,
  };
}

export function readGeneratedNarrationHashManifest(
  monolithPath: string,
  options: GeneratedHashReadOptions = {},
): LoadedGeneratedHashManifest<GeneratedNarrationHashManifest> {
  const files = shardedFiles(monolithPath);
  if (files === undefined) {
    if (!options.allowLegacyMonolith) {
      throw new Error(
        `generated narration hash manifest '${monolithPath}' must use a chapter-owned .d directory`,
      );
    }
    const language = languageFromMonolith(monolithPath);
    return {
      manifest: validateNarrationManifest(
        readLedgerFile<unknown>(monolithPath),
        language,
        monolithPath,
      ),
      sourcePaths: [monolithPath],
      sharded: false,
    };
  }
  const meta = validateMeta(
    readLedgerFile<unknown>(join(files.directory, "_meta.json")),
    files.language,
    true,
  );
  const owners = files.names
    .filter((name) => name !== "_meta.json")
    .map((name) =>
      validateNarrationOwner(
        readLedgerFile<unknown>(join(files.directory, name)),
        files.language,
        Number(CHAPTER_OWNER.exec(name)![1]),
        `${files.language}.d/${name}`,
      ),
    );
  return {
    manifest: {
      version: meta.version,
      algorithm: meta.algorithm,
      maxLinearisableTableColumns: meta.maxLinearisableTableColumns,
      chapters: owners.map(({ findings: _findings, ...chapter }) => chapter),
      findings: owners.flatMap((owner) => owner.findings).sort(compareFindings),
    },
    sourcePaths: files.sourcePaths,
    sharded: true,
  };
}

function listLanguages(directory: string): string[] {
  assertRealDescendantComponents(
    dirname(dirname(resolve(directory))),
    directory,
  );
  const stat = lstatSync(directory);
  if (stat.isSymbolicLink() || !stat.isDirectory()) {
    throw new Error(
      `generated hash directory '${directory}' must be a real directory`,
    );
  }
  const owners = new Map<string, string>();
  for (const name of readdirSync(directory).sort()) {
    if (name === "README.md") continue;
    const path = join(directory, name);
    const entry = lstatSync(path);
    if (entry.isSymbolicLink())
      throw new Error(
        `generated hash owner '${path}' must not be a symbolic link`,
      );
    if (name.endsWith(".json")) {
      throw new Error(
        `generated hash owner '${path}' is a forbidden flat monolith; use a chapter-owned .d directory`,
      );
    }
    const match = /^([a-z][a-z0-9-]*)\.d$/.exec(name);
    if (match === null || !entry.isDirectory()) {
      throw new Error(
        `generated hash owner '${path}' has a malformed name or type`,
      );
    }
    const language = match[1];
    const previous = owners.get(language);
    if (previous !== undefined) {
      throw new Error(
        `generated hash language '${language}' has duplicate owners '${previous}' and '${name}'`,
      );
    }
    owners.set(language, name);
  }
  return [...owners.keys()].sort();
}

/** Strict, name-only generated-book ownership discovery for eager consumers. */
export function listGeneratedBookHashOwnerLanguages(root: string): string[] {
  return listLanguages(join(root, GENERATED_BOOK_HASH_DIR));
}

export function listGeneratedBookHashManifests(
  root: string,
): Array<
  { language: string } & LoadedGeneratedHashManifest<GeneratedBookHashManifest>
> {
  const directory = join(root, GENERATED_BOOK_HASH_DIR);
  return listGeneratedBookHashOwnerLanguages(root).map((language) => ({
    language,
    ...readGeneratedBookHashManifest(join(directory, `${language}.json`)),
  }));
}

export function generatedBookHashOwnerContents(
  language: string,
  manifest: GeneratedBookHashManifest,
): Map<string, string> {
  if (!LANGUAGE.test(language))
    throw new Error(`unsafe generated book hash language '${language}'`);
  const validated = validateBookManifest(
    manifest,
    language,
    `${language} generated book manifest`,
  );
  const prefix = `${GENERATED_BOOK_HASH_DIR}/${language}.d`;
  const outputs = new Map<string, string>([
    [
      `${prefix}/_meta.json`,
      `${JSON.stringify({ version: 1, language, algorithm: "fnv1a64" }, null, 2)}\n`,
    ],
  ]);
  for (const chapter of validated.chapters) {
    outputs.set(
      `${prefix}/${generatedHashChapterFilename(chapter.chapter)}`,
      `${JSON.stringify(chapter, null, 2)}\n`,
    );
  }
  return outputs;
}

export function generatedNarrationHashOwnerContents(
  language: string,
  manifest: GeneratedNarrationHashManifest,
): Map<string, string> {
  if (!LANGUAGE.test(language))
    throw new Error(`unsafe generated narration hash language '${language}'`);
  const validated = validateNarrationManifest(
    manifest,
    language,
    `${language} generated narration manifest`,
  );
  const prefix = `${GENERATED_NARRATION_HASH_DIR}/${language}.d`;
  const outputs = new Map<string, string>([
    [
      `${prefix}/_meta.json`,
      `${JSON.stringify(
        {
          version: 1,
          language,
          algorithm: "fnv1a64",
          maxLinearisableTableColumns: validated.maxLinearisableTableColumns,
        },
        null,
        2,
      )}\n`,
    ],
  ]);
  for (const chapter of validated.chapters) {
    const findings = validated.findings
      .filter(
        (finding) =>
          finding.language === language &&
          chapter.lessonIds.includes(finding.lessonId),
      )
      .sort(compareFindings);
    const owner = validateNarrationOwner(
      { ...chapter, findings },
      language,
      chapter.chapter,
      `${language} chapter ${chapter.chapter}`,
    );
    outputs.set(
      `${prefix}/${generatedHashChapterFilename(chapter.chapter)}`,
      `${JSON.stringify(owner, null, 2)}\n`,
    );
  }
  return outputs;
}

/** Exact expected owner names for one language directory, used by generator --check modes. */
export function assertGeneratedHashOwnerNames(
  root: string,
  relativeDirectory: string,
  expectedRelativePaths: Iterable<string>,
): void {
  const expectedByDirectory = new Map<string, Set<string>>();
  for (const relative of expectedRelativePaths) {
    assertRelativeManifestPath(
      relative,
      `unsafe generated hash owner '${relative}'`,
    );
    if (!relative.startsWith(`${relativeDirectory}/`)) continue;
    const ownerDirectory = dirname(relative);
    const names = expectedByDirectory.get(ownerDirectory) ?? new Set<string>();
    names.add(basename(relative));
    expectedByDirectory.set(ownerDirectory, names);
  }
  const base = join(root, relativeDirectory);
  assertRealDescendantComponents(root, base);
  const expectedLanguages = new Set(
    [...expectedByDirectory.keys()].map((path) => basename(path, ".d")),
  );
  for (const name of readdirSync(base)) {
    if (name === "README.md") continue;
    const path = join(base, name);
    const stat = lstatSync(path);
    if (stat.isSymbolicLink())
      throw new Error(
        `generated hash owner '${path}' must not be a symbolic link`,
      );
    if (name.endsWith(".json")) {
      throw new Error(
        `generated hash owner '${path}' is an unexpected resurrected monolith`,
      );
    }
    const match = /^([a-z][a-z0-9-]*)\.d$/.exec(name);
    if (
      match === null ||
      !stat.isDirectory() ||
      !expectedLanguages.has(match[1])
    ) {
      throw new Error(
        `generated hash owner '${path}' is unexpected or malformed`,
      );
    }
  }
  for (const [relativeOwnerDirectory, expectedNames] of expectedByDirectory) {
    const ownerDirectory = join(root, relativeOwnerDirectory);
    const stat = lstatSync(ownerDirectory);
    if (stat.isSymbolicLink() || !stat.isDirectory()) {
      throw new Error(
        `generated hash owner '${ownerDirectory}' must be a real directory`,
      );
    }
    const actualNames = readdirSync(ownerDirectory).sort();
    if (
      actualNames.length !== expectedNames.size ||
      actualNames.some((name) => !expectedNames.has(name))
    ) {
      throw new Error(
        `${relativeOwnerDirectory}: generated hash owner set is missing, stale, or malformed`,
      );
    }
    for (const name of actualNames) {
      const owner = join(ownerDirectory, name);
      const ownerStat = lstatSync(owner);
      if (ownerStat.isSymbolicLink() || !ownerStat.isFile()) {
        throw new Error(
          `generated hash owner '${owner}' must be a real regular file`,
        );
      }
    }
  }
}
