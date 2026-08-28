import { lstatSync, readFileSync, readdirSync, realpathSync } from "node:fs";
import { join, relative, resolve, sep } from "node:path";
import { isAbsentErrno, readLedgerFile } from "./shard.js";
import {
  MODALITY_MANIFEST_DIR,
  MODALITY_MANIFEST_VERSION,
  buildModalityManifestFromRows,
  type ModalityManifest,
  type ModalityManifestHeader,
  type ModalityManifestLesson,
} from "./modality-manifest.js";
import {
  modalityRank,
  type Modality,
  type ModalityFinding,
  type ModalityReasonCode,
} from "./modality.js";
import {
  GENERATED_NARRATION_HASH_DIR,
  readGeneratedNarrationHashManifest,
} from "./generated-hash-shards.js";

export const MODALITY_META_OWNER = "_meta.json";

const LANGUAGE = /^[a-z][a-z0-9-]*$/;
const LESSON_ID = /^[A-Za-z0-9][A-Za-z0-9-]*$/;
const HASH = /^fnv1a64:[0-9a-f]{16}$/;
const WINDOWS_RESERVED = /^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])$/i;
const DANGEROUS_IDENTITIES = new Set(["__proto__", "constructor", "prototype"]);
const MODALITIES = new Set<Modality>(["voice", "sight", "pen"]);
const REASONS = new Set<ModalityReasonCode>([
  "writing-type",
  "writing-block",
  "script-block",
  "sight-cue",
  "wide-table",
  "no-visual-dependency",
]);
const FINDING_CODES = new Set<ModalityFinding["code"]>([
  "modality-unknown-value",
  "modality-unexplained-override",
  "modality-writing-segment-not-separable",
]);

interface ModalityMetaOwner extends ModalityManifestHeader {
  language: string;
}

interface ModalityLessonOwner {
  lesson: ModalityManifestLesson;
  findings: ModalityFinding[];
}

export interface ModalityOwnerReadOptions {
  /** Independent language registry identities. */
  expectedLanguages: readonly string[];
  /** Exact lesson identities derived from parsed lesson sources. */
  expectedLessonIds: ReadonlyMap<string, readonly string[]>;
  /** Optional second independent projection from narration chapter owners. */
  expectedNarrationLessonIds?: ReadonlyMap<string, readonly string[]>;
  /** Migration validation may temporarily retain canonical legacy aggregates. */
  rejectAggregates?: boolean;
  /** Direct owners are canonical generated data, so formatting drift is drift. */
  requireCanonicalBytes?: boolean;
}

function canonical(value: unknown): string {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function statIfPresent(path: string): ReturnType<typeof lstatSync> | undefined {
  try {
    return lstatSync(path);
  } catch (cause) {
    const code = (cause as NodeJS.ErrnoException).code;
    if (isAbsentErrno(code)) return undefined;
    throw new Error(
      `modality owner '${path}': cannot be inspected (${code ?? "unknown error"})`,
      { cause },
    );
  }
}

function object(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`modality owner '${label}' must contain one JSON object`);
  }
  return value as Record<string, unknown>;
}

function exactKeys(
  value: Record<string, unknown>,
  required: readonly string[],
  optional: readonly string[],
  label: string,
): void {
  const actual = Object.keys(value).sort();
  const allowed = new Set([...required, ...optional]);
  const missing = required.filter((key) => !Object.hasOwn(value, key));
  const extra = actual.filter((key) => !allowed.has(key));
  if (missing.length > 0 || extra.length > 0) {
    throw new Error(
      `modality owner '${label}' has invalid keys` +
        `${missing.length > 0 ? `; missing: ${missing.join(", ")}` : ""}` +
        `${extra.length > 0 ? `; extra: ${extra.join(", ")}` : ""}`,
    );
  }
}

function safeLanguage(value: string): boolean {
  return LANGUAGE.test(value) && !WINDOWS_RESERVED.test(value);
}

function safeLessonId(value: string): boolean {
  return LESSON_ID.test(value) &&
    !WINDOWS_RESERVED.test(value) &&
    !DANGEROUS_IDENTITIES.has(value.toLowerCase());
}

function assertNoCaseFoldCollisions(values: readonly string[], label: string): void {
  const seen = new Map<string, string>();
  for (const value of values) {
    const folded = value.toLowerCase();
    const prior = seen.get(folded);
    if (prior !== undefined && prior !== value) {
      throw new Error(`${label} has a case-fold collision between '${prior}' and '${value}'`);
    }
    if (prior !== undefined) throw new Error(`${label} repeats '${value}'`);
    seen.set(folded, value);
  }
}

function assertIdentitySet(
  actual: readonly string[],
  expected: readonly string[],
  label: string,
): void {
  assertNoCaseFoldCollisions(expected, `${label} expected identities`);
  assertNoCaseFoldCollisions(actual, `${label} owner identities`);
  const found = [...actual].sort();
  const wanted = [...expected].sort();
  const missing = wanted.filter((identity) => !found.includes(identity));
  const extra = found.filter((identity) => !wanted.includes(identity));
  if (missing.length > 0 || extra.length > 0) {
    throw new Error(
      `${label} do not match` +
        `${missing.length > 0 ? `; missing: ${missing.join(", ")}` : ""}` +
        `${extra.length > 0 ? `; extra: ${extra.join(", ")}` : ""}`,
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
    throw new Error(`modality owner path '${target}' is outside '${root}'`);
  }
  const realRoot = realpathSync(absoluteRoot);
  let current = absoluteRoot;
  for (const component of route.split(sep).filter(Boolean)) {
    current = join(current, component);
    const stat = lstatSync(current);
    if (stat.isSymbolicLink()) {
      throw new Error(`modality owner path component '${current}' must not be a symbolic link`);
    }
    const real = realpathSync(current);
    if (real !== realRoot && !real.startsWith(realRoot + sep)) {
      throw new Error(`modality owner path component '${current}' resolves outside '${root}'`);
    }
  }
}

function string(value: unknown, field: string, label: string): string {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`modality owner '${label}'.${field} must be a non-empty string`);
  }
  return value;
}

function nullableInteger(value: unknown, field: string, label: string): number | null {
  if (value === null) return null;
  if (typeof value !== "number" || !Number.isInteger(value) || value < 0) {
    throw new Error(`modality owner '${label}'.${field} must be null or a non-negative integer`);
  }
  return value;
}

function modality(value: unknown, field: string, label: string): Modality {
  if (typeof value !== "string" || !MODALITIES.has(value as Modality)) {
    throw new Error(`modality owner '${label}'.${field} must be voice, sight, or pen`);
  }
  return value as Modality;
}

function reasons(value: unknown, field: string, label: string): ModalityReasonCode[] {
  if (!Array.isArray(value)) {
    throw new Error(`modality owner '${label}'.${field} must be an array`);
  }
  return value.map((entry, index) => {
    if (typeof entry !== "string" || !REASONS.has(entry as ModalityReasonCode)) {
      throw new Error(`modality owner '${label}'.${field}[${index}] is unknown`);
    }
    return entry as ModalityReasonCode;
  });
}

function stringArray(value: unknown, field: string, label: string): string[] {
  if (!Array.isArray(value)) {
    throw new Error(`modality owner '${label}'.${field} must be an array`);
  }
  return value.map((entry, index) =>
    string(entry, `${field}[${index}]`, label));
}

function parseMeta(value: unknown, language: string): ModalityMetaOwner {
  const label = `${language}.d/${MODALITY_META_OWNER}`;
  const meta = object(value, label);
  exactKeys(
    meta,
    ["version", "language", "algorithm", "features", "policy"],
    [],
    label,
  );
  if (meta.version !== MODALITY_MANIFEST_VERSION) {
    throw new Error(`modality metadata '${language}' version must be ${MODALITY_MANIFEST_VERSION}`);
  }
  if (meta.language !== language) {
    throw new Error(
      `modality metadata language '${String(meta.language)}' does not match '${language}'`,
    );
  }
  if (meta.algorithm !== "fnv1a64") {
    throw new Error(`modality metadata '${language}' algorithm must be fnv1a64`);
  }
  const features = object(meta.features, `${label}.features`);
  exactKeys(features, ["blockModality"], [], `${label}.features`);
  if (typeof features.blockModality !== "boolean") {
    throw new Error(`modality metadata '${language}' blockModality must be boolean`);
  }
  const policy = object(meta.policy, `${label}.policy`);
  exactKeys(policy, ["maxLinearisableTableColumns"], [], `${label}.policy`);
  const width = policy.maxLinearisableTableColumns;
  if (typeof width !== "number" || !Number.isInteger(width) || width < 0 || width > 16) {
    throw new Error(`modality metadata '${language}' table width must be an integer from 0 to 16`);
  }
  return {
    version: MODALITY_MANIFEST_VERSION,
    language,
    algorithm: "fnv1a64",
    features: { blockModality: features.blockModality },
    policy: { maxLinearisableTableColumns: width },
  };
}

function parseLesson(value: unknown, language: string, filename: string): ModalityManifestLesson {
  const label = `${language}.d/${filename}`;
  const row = object(value, `${label}.lesson`);
  exactKeys(
    row,
    [
      "id",
      "language",
      "chapter",
      "sequence",
      "modality",
      "derived",
      "drivable",
      "reasons",
      "coreModality",
      "coreReasons",
      "coreDrivable",
      "sourceHash",
    ],
    [
      "detachableSegments",
      "delivery",
      "authored",
      "authoredReason",
      "overridden",
    ],
    `${label}.lesson`,
  );
  const id = string(row.id, "id", label);
  if (!safeLessonId(id)) throw new Error(`modality lesson id '${id}' is unsafe or reserved`);
  if (filename !== `${id}.json`) {
    throw new Error(`modality owner filename '${filename}' does not match lesson '${id}'`);
  }
  if (row.language !== language) {
    throw new Error(
      `modality lesson '${id}' language '${String(row.language)}' does not match '${language}'`,
    );
  }
  const full = modality(row.modality, "modality", label);
  const derived = modality(row.derived, "derived", label);
  const core = modality(row.coreModality, "coreModality", label);
  if (modalityRank(core) > modalityRank(full)) {
    throw new Error(`modality lesson '${id}' coreModality is stronger than modality`);
  }
  if (row.drivable !== (full === "voice")) {
    throw new Error(`modality lesson '${id}' drivable disagrees with modality`);
  }
  if (row.coreDrivable !== (core === "voice")) {
    throw new Error(`modality lesson '${id}' coreDrivable disagrees with coreModality`);
  }
  const sourceHash = string(row.sourceHash, "sourceHash", label);
  if (!HASH.test(sourceHash)) {
    throw new Error(`modality lesson '${id}' sourceHash is invalid`);
  }
  const lesson: ModalityManifestLesson = {
    id,
    language,
    chapter: nullableInteger(row.chapter, "chapter", label),
    sequence: nullableInteger(row.sequence, "sequence", label),
    modality: full,
    derived,
    drivable: row.drivable,
    reasons: reasons(row.reasons, "reasons", label),
    coreModality: core,
    coreReasons: reasons(row.coreReasons, "coreReasons", label),
    coreDrivable: row.coreDrivable,
    sourceHash,
  };
  if (Object.hasOwn(row, "detachableSegments")) {
    lesson.detachableSegments = stringArray(row.detachableSegments, "detachableSegments", label);
  }
  for (const key of ["delivery", "authored", "authoredReason"] as const) {
    if (Object.hasOwn(row, key)) lesson[key] = string(row[key], key, label);
  }
  if (Object.hasOwn(row, "overridden")) {
    if (row.overridden !== true) {
      throw new Error(`modality lesson '${id}' overridden may only be emitted as true`);
    }
    lesson.overridden = true;
  }
  return lesson;
}

function parseFindings(
  value: unknown,
  language: string,
  lessonId: string,
  label: string,
): ModalityFinding[] {
  if (!Array.isArray(value)) {
    throw new Error(`modality owner '${label}'.findings must be an array`);
  }
  const seenCodes = new Set<string>();
  let previousCode: string | undefined;
  return value.map((entry, index) => {
    const finding = object(entry, `${label}.findings[${index}]`);
    exactKeys(
      finding,
      ["code", "lessonId", "language", "message"],
      [],
      `${label}.findings[${index}]`,
    );
    const code = string(finding.code, "code", label);
    if (!FINDING_CODES.has(code as ModalityFinding["code"])) {
      throw new Error(`modality finding '${label}' has unknown code '${code}'`);
    }
    if (seenCodes.has(code)) {
      throw new Error(`modality finding '${label}' repeats identity '${code}'`);
    }
    if (previousCode !== undefined && previousCode.localeCompare(code) > 0) {
      throw new Error(`modality findings '${label}' are not in canonical code order`);
    }
    seenCodes.add(code);
    previousCode = code;
    if (finding.lessonId !== lessonId || finding.language !== language) {
      throw new Error(`modality finding '${label}' does not belong to its lesson owner`);
    }
    return {
      code: code as ModalityFinding["code"],
      lessonId,
      language,
      message: string(finding.message, "message", label),
    };
  });
}

function parseLessonOwner(value: unknown, language: string, filename: string): ModalityLessonOwner {
  const label = `${language}.d/${filename}`;
  const owner = object(value, label);
  exactKeys(owner, ["lesson", "findings"], [], label);
  const lesson = parseLesson(owner.lesson, language, filename);
  return { lesson, findings: parseFindings(owner.findings, language, lesson.id, label) };
}

function expectedLanguagesFromMap(
  identities: ReadonlyMap<string, readonly string[]>,
  languages: readonly string[],
  label: string,
): void {
  assertIdentitySet([...identities.keys()], languages, `${label} languages`);
  const globalIds: string[] = [];
  for (const [language, ids] of identities) {
    if (!safeLanguage(language)) throw new Error(`${label} language '${language}' is unsafe`);
    for (const id of ids) {
      if (!safeLessonId(id)) throw new Error(`${label} lesson id '${id}' is unsafe or reserved`);
    }
    assertNoCaseFoldCollisions(ids, `${label} ${language} lesson ids`);
    globalIds.push(...ids);
  }
  assertNoCaseFoldCollisions(globalIds, `${label} global lesson ids`);
}

/** Stable direct-owner bytes for one complete public modality manifest. */
export function modalityOwnerContents(manifest: ModalityManifest): Map<string, string> {
  const languages = manifest.tracks.map((track) => track.language);
  assertModalityManifestLanguages(manifest, languages);
  assertIdentitySet(
    [...new Set(manifest.lessons.map((lesson) => lesson.language))],
    languages,
    "modality lesson languages",
  );
  const lessonLanguages = new Map(
    manifest.lessons.map((lesson) => [lesson.id, lesson.language] as const),
  );
  const findingsByLesson = new Map<string, ModalityFinding[]>();
  for (const finding of manifest.findings) {
    const ownerLanguage = lessonLanguages.get(finding.lessonId);
    if (ownerLanguage === undefined || ownerLanguage !== finding.language) {
      throw new Error(
        `modality finding '${finding.code}' does not belong to lesson '${finding.lessonId}'`,
      );
    }
    const bucket = findingsByLesson.get(finding.lessonId);
    if (bucket) bucket.push(finding);
    else findingsByLesson.set(finding.lessonId, [finding]);
  }
  const out = new Map<string, string>();
  const seenIds = new Map<string, string>();
  for (const language of languages) {
    if (!safeLanguage(language)) throw new Error(`modality language '${language}' is unsafe`);
    out.set(
      `${language}.d/${MODALITY_META_OWNER}`,
      canonical({
        version: manifest.version,
        language,
        algorithm: manifest.algorithm,
        features: manifest.features,
        policy: manifest.policy,
      }),
    );
    for (const lesson of manifest.lessons.filter((row) => row.language === language)) {
      if (!safeLessonId(lesson.id)) {
        throw new Error(`modality lesson id '${lesson.id}' is unsafe or reserved`);
      }
      const folded = lesson.id.toLowerCase();
      const prior = seenIds.get(folded);
      if (prior !== undefined) {
        throw new Error(`modality lesson ids have a case-fold collision: '${prior}'/'${lesson.id}'`);
      }
      seenIds.set(folded, lesson.id);
      out.set(
        `${language}.d/${lesson.id}.json`,
        canonical({ lesson, findings: findingsByLesson.get(lesson.id) ?? [] }),
      );
      findingsByLesson.delete(lesson.id);
    }
  }
  if (findingsByLesson.size > 0) {
    throw new Error(
      `modality findings name missing lessons: ${[...findingsByLesson.keys()].sort().join(", ")}`,
    );
  }
  return out;
}

/** Prove public track identities against the independent language registry. */
export function assertModalityManifestLanguages(
  manifest: ModalityManifest,
  expectedLanguages: readonly string[],
): void {
  for (const language of expectedLanguages) {
    if (!safeLanguage(language)) throw new Error(`modality expected language '${language}' is unsafe`);
  }
  assertIdentitySet(
    manifest.tracks.map((track) => track.language),
    expectedLanguages,
    "modality manifest languages",
  );
}

/** Independent lesson identities declared by chapter-owned narration manifests. */
export function modalityNarrationLessonIds(
  root: string,
  languages: readonly string[],
): ReadonlyMap<string, readonly string[]> {
  const out = new Map<string, readonly string[]>();
  const global = new Map<string, string>();
  for (const language of languages) {
    if (!safeLanguage(language)) {
      throw new Error(`modality narration language '${language}' is unsafe`);
    }
    const loaded = readGeneratedNarrationHashManifest(
      join(root, GENERATED_NARRATION_HASH_DIR, `${language}.json`),
    );
    const ids = loaded.manifest.chapters.flatMap((chapter) => chapter.lessonIds);
    assertNoCaseFoldCollisions(ids, `narration ${language} lesson ids`);
    for (const id of ids) {
      if (!safeLessonId(id)) {
        throw new Error(`narration lesson id '${id}' is unsafe or reserved`);
      }
      const folded = id.toLowerCase();
      const prior = global.get(folded);
      if (prior !== undefined) {
        throw new Error(`narration lesson id '${id}' duplicates '${prior}' across languages`);
      }
      global.set(folded, id);
    }
    out.set(language, [...ids].sort());
  }
  return out;
}

/**
 * Strictly fold direct lesson owners into the historical public manifest shape.
 *
 * Completeness is established from independent identities before any owner bytes are
 * opened. There is deliberately no flat-aggregate fallback.
 */
export function readModalityManifestOwners(
  root: string,
  options: ModalityOwnerReadOptions,
): ModalityManifest {
  for (const language of options.expectedLanguages) {
    if (!safeLanguage(language)) throw new Error(`modality expected language '${language}' is unsafe`);
  }
  assertNoCaseFoldCollisions(options.expectedLanguages, "modality expected languages");
  expectedLanguagesFromMap(options.expectedLessonIds, options.expectedLanguages, "source");
  if (options.expectedNarrationLessonIds !== undefined) {
    expectedLanguagesFromMap(
      options.expectedNarrationLessonIds,
      options.expectedLanguages,
      "narration",
    );
    for (const language of options.expectedLanguages) {
      assertIdentitySet(
        options.expectedNarrationLessonIds.get(language) ?? [],
        options.expectedLessonIds.get(language) ?? [],
        `narration lesson identities for ${language}`,
      );
    }
  }

  const directory = join(root, MODALITY_MANIFEST_DIR);
  const directoryStat = statIfPresent(directory);
  if (directoryStat === undefined) throw new Error(`modality owner directory '${directory}' is missing`);
  if (directoryStat.isSymbolicLink() || !directoryStat.isDirectory()) {
    throw new Error(`modality owner directory '${directory}' must be a real directory`);
  }
  assertRealDescendantComponents(root, directory);

  const entries = readdirSync(directory, { withFileTypes: true });
  const ownerLanguages: string[] = [];
  for (const entry of entries) {
    const path = join(directory, entry.name);
    if (entry.name.endsWith(".json")) {
      if (options.rejectAggregates !== false) {
        throw new Error(`modality aggregate '${entry.name}' is resurrected beside direct owners`);
      }
      const aggregateLanguage = entry.name.slice(0, -".json".length);
      if (!safeLanguage(aggregateLanguage) || entry.isSymbolicLink() || !entry.isFile()) {
        throw new Error(`unexpected modality aggregate '${entry.name}'`);
      }
      continue;
    }
    if (!entry.name.endsWith(".d") || entry.isSymbolicLink() || !entry.isDirectory()) {
      throw new Error(`unexpected modality owner entry '${entry.name}'`);
    }
    const language = entry.name.slice(0, -2);
    if (!safeLanguage(language)) throw new Error(`modality owner language '${language}' is unsafe`);
    assertRealDescendantComponents(root, path);
    ownerLanguages.push(language);
  }
  assertIdentitySet(ownerLanguages, options.expectedLanguages, "modality owner languages");

  const lessons: ModalityManifestLesson[] = [];
  const findings: ModalityFinding[] = [];
  let firstMeta: ModalityMetaOwner | undefined;
  for (const language of options.expectedLanguages) {
    const languageDirectory = join(directory, `${language}.d`);
    const languageEntries = readdirSync(languageDirectory, { withFileTypes: true });
    const actualNames = languageEntries.map((entry) => entry.name);
    for (const entry of languageEntries) {
      if (entry.isSymbolicLink() || !entry.isFile()) {
        throw new Error(
          `modality owner '${language}.d/${entry.name}' must be a real direct-child regular file`,
        );
      }
      if (entry.name !== MODALITY_META_OWNER) {
        const id = entry.name.endsWith(".json")
          ? entry.name.slice(0, -".json".length)
          : entry.name;
        if (!entry.name.endsWith(".json") || !safeLessonId(id)) {
          throw new Error(`unexpected or reserved modality owner '${language}.d/${entry.name}'`);
        }
      }
    }
    const expectedIds = options.expectedLessonIds.get(language) ?? [];
    assertIdentitySet(
      actualNames
        .filter((name) => name !== MODALITY_META_OWNER)
        .map((name) => name.slice(0, -".json".length)),
      expectedIds,
      `modality lesson owners for ${language}`,
    );
    if (!actualNames.includes(MODALITY_META_OWNER)) {
      throw new Error(`modality owner '${language}.d/${MODALITY_META_OWNER}' is missing`);
    }

    const metaPath = join(languageDirectory, MODALITY_META_OWNER);
    const meta = parseMeta(readLedgerFile(metaPath), language);
    if (
      firstMeta !== undefined &&
      (meta.version !== firstMeta.version ||
        meta.algorithm !== firstMeta.algorithm ||
        meta.features.blockModality !== firstMeta.features.blockModality ||
        meta.policy.maxLinearisableTableColumns !==
          firstMeta.policy.maxLinearisableTableColumns)
    ) {
      throw new Error(`incompatible modality metadata for '${language}'`);
    }
    firstMeta ??= meta;
    if (options.requireCanonicalBytes !== false && readFileSync(metaPath, "utf8") !== canonical(meta)) {
      throw new Error(`modality owner '${language}.d/${MODALITY_META_OWNER}' is not canonical`);
    }

    for (const id of expectedIds) {
      const filename = `${id}.json`;
      const path = join(languageDirectory, filename);
      const parsed = parseLessonOwner(readLedgerFile(path), language, filename);
      if (options.requireCanonicalBytes !== false) {
        const expected = canonical(parsed);
        if (readFileSync(path, "utf8") !== expected) {
          throw new Error(`modality owner '${language}.d/${filename}' is not canonical`);
        }
      }
      lessons.push(parsed.lesson);
      findings.push(...parsed.findings);
    }
  }
  if (firstMeta === undefined) throw new Error("no modality owner metadata found");
  const manifest = buildModalityManifestFromRows(firstMeta, lessons, findings);
  assertModalityManifestLanguages(manifest, options.expectedLanguages);
  return manifest;
}
