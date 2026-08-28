import { lstatSync, readdirSync, realpathSync } from "node:fs";
import { basename, join, relative, resolve, sep } from "node:path";
import { assertRelativeManifestPath } from "./manifest-path.js";
import { isAbsentErrno, readLedgerFile } from "./shard.js";

export const BOOK_GENERATION_DIRECTORY = "core/book-generation.d";

export const BOOK_GENERATION_SECTION_DIRECTORIES = [
  "script-sets.d",
  "reference-appendices.d",
  "glossaries.d",
  "answer-keys.d",
  "indexes.d",
  "targets.d",
  "handwritten.d",
] as const;

const LANGUAGE = /^[a-z][a-z0-9-]*$/;
const SAFE_SLUG = /^[a-z0-9][a-z0-9-]*$/;
const CHAPTER_OWNER = /^([a-z][a-z0-9-]*)-(\d{4})\.json$/;
const SCRIPT_SET_OWNER = /^(\d{4})-([a-z][a-z0-9-]*)\.json$/;
const WINDOWS_RESERVED = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])$/i;
const codeUnit = (left: string, right: string) =>
  left < right ? -1 : left > right ? 1 : 0;
const serialize = (value: unknown) => `${JSON.stringify(value, null, 2)}\n`;

export interface BookGenerationChapterOwner {
  language: string;
  chapter: number;
  output: string;
  [key: string]: unknown;
}

export interface BookGenerationBackmatterOwner {
  language: string;
  output: string;
  [key: string]: unknown;
}

export interface BookGenerationDocument {
  version: 1;
  sourceBaseUrl: string;
  scriptSets?: Record<string, unknown[]>;
  referenceAppendices?: BookGenerationBackmatterOwner[];
  glossaries?: BookGenerationBackmatterOwner[];
  answerKeys?: BookGenerationBackmatterOwner[];
  indexes?: BookGenerationBackmatterOwner[];
  targets: BookGenerationChapterOwner[];
  handwritten?: BookGenerationChapterOwner[];
}

export interface BookGenerationIdentitySets {
  targets: Set<string>;
  handwritten: Set<string>;
  combined: Set<string>;
  languages: Set<string>;
  referenceAppendices: Set<string>;
  glossaries: Set<string>;
  answerKeys: Set<string>;
  indexes: Set<string>;
}

export interface LoadedBookGenerationOwners {
  document: BookGenerationDocument;
  sourcePaths: string[];
}

function statIfPresent(path: string): ReturnType<typeof lstatSync> | undefined {
  try {
    return lstatSync(path);
  } catch (cause) {
    if (isAbsentErrno((cause as NodeJS.ErrnoException).code)) return undefined;
    throw cause;
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
    throw new Error(`book-generation path '${target}' is outside '${root}'`);
  }
  const realRoot = realpathSync(absoluteRoot);
  let current = absoluteRoot;
  for (const component of route.split(sep).filter(Boolean)) {
    current = join(current, component);
    const stat = lstatSync(current);
    if (stat.isSymbolicLink()) {
      throw new Error(
        `book-generation path component '${current}' must not be a symbolic link`,
      );
    }
    const real = realpathSync(current);
    if (real !== realRoot && !real.startsWith(realRoot + sep)) {
      throw new Error(
        `book-generation path component '${current}' resolves outside '${root}'`,
      );
    }
  }
}

function object(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`${label} must contain one JSON object`);
  }
  return value as Record<string, unknown>;
}

function nonEmptyString(value: unknown, label: string): string {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`${label} must be a non-empty string`);
  }
  return value;
}

function safeLanguage(value: unknown, label: string): string {
  const language = nonEmptyString(value, label);
  if (!LANGUAGE.test(language) || WINDOWS_RESERVED.test(language)) {
    throw new Error(`${label} is not a safe language owner`);
  }
  return language;
}

function chapterIdentity(language: string, chapter: number): string {
  return `${language}/${String(chapter).padStart(4, "0")}`;
}

function chapterOwnerName(value: unknown, label: string): string {
  const record = object(value, label);
  const language = safeLanguage(record.language, `${label}.language`);
  if (
    typeof record.chapter !== "number" ||
    !Number.isInteger(record.chapter) ||
    record.chapter < 1 ||
    record.chapter > 9999
  ) {
    throw new Error(`${label}.chapter must be an integer from 1 through 9999`);
  }
  safeOwnedOutput(record.output, language, `${label}.output`);
  return `${language}-${String(record.chapter).padStart(4, "0")}.json`;
}

function safeOutput(value: unknown, label: string): string {
  const output = nonEmptyString(value, label);
  assertRelativeManifestPath(output, `${label} is unsafe`);
  if (
    output.includes("\\") ||
    output.split("/").some((component) =>
      component === "" || component === "." || component === ".."
    )
  ) {
    throw new Error(`${label} is unsafe`);
  }
  if (!output.endsWith(".tex")) throw new Error(`${label} must end in .tex`);
  return output;
}

function safeOwnedOutput(value: unknown, language: string, label: string): string {
  const output = safeOutput(value, label);
  if (output.split("/", 1)[0] !== language) {
    throw new Error(`${label} must stay within the '${language}' track`);
  }
  return output;
}

function backmatterOwnerName(value: unknown, label: string): string {
  const record = object(value, label);
  const language = safeLanguage(record.language, `${label}.language`);
  const output = safeOwnedOutput(record.output, language, `${label}.output`);
  const leaf = basename(output, ".tex");
  if (!SAFE_SLUG.test(leaf) || WINDOWS_RESERVED.test(leaf)) {
    throw new Error(`${label}.output basename '${leaf}' is not a safe owner id`);
  }
  return `${language}-${leaf}.json`;
}

function backmatterIdentity(value: unknown, label: string): string {
  const record = object(value, label);
  const language = safeLanguage(record.language, `${label}.language`);
  const output = safeOwnedOutput(record.output, language, `${label}.output`);
  return `${language}/${basename(output, ".tex")}`;
}

function exactMeta(value: unknown, label: string): {
  version: 1;
  sourceBaseUrl: string;
} {
  const meta = object(value, label);
  const keys = Object.keys(meta);
  if (
    keys.length !== 2 ||
    keys[0] !== "version" ||
    keys[1] !== "sourceBaseUrl" ||
    meta.version !== 1
  ) {
    throw new Error(`${label} must contain exactly version 1 and sourceBaseUrl`);
  }
  const sourceBaseUrl = nonEmptyString(meta.sourceBaseUrl, `${label}.sourceBaseUrl`);
  if (!/^https?:\/\//.test(sourceBaseUrl)) {
    throw new Error(`${label}.sourceBaseUrl must be HTTP(S)`);
  }
  return { version: 1, sourceBaseUrl };
}

function assertSortedOwnerNames(
  entries: readonly unknown[],
  ownerName: (value: unknown, label: string) => string,
  section: string,
): string[] {
  const names = entries.map((entry, index) =>
    ownerName(entry, `${section}[${index}]`),
  );
  const sorted = [...names].sort(codeUnit);
  if (names.some((name, index) => name !== sorted[index])) {
    throw new Error(`${section} must be in raw-code-unit owner order`);
  }
  if (new Set(names).size !== names.length) {
    throw new Error(`${section} contains duplicate owner identities`);
  }
  return names;
}

function arrays(value: unknown, label: string): unknown[] {
  if (!Array.isArray(value)) throw new Error(`${label} must contain one JSON array`);
  return value;
}

function requiredArray(
  document: Record<string, unknown>,
  key: string,
): unknown[] {
  if (!Array.isArray(document[key])) {
    throw new Error(`book-generation.${key} must be an array`);
  }
  return document[key] as unknown[];
}

function optionalArray(
  document: Record<string, unknown>,
  key: string,
): unknown[] {
  const value = document[key];
  if (value === undefined) return [];
  if (!Array.isArray(value)) throw new Error(`book-generation.${key} must be an array`);
  return value;
}

function validateDocument(value: unknown): BookGenerationDocument {
  const record = object(value, "book-generation");
  const expected = [
    "version",
    "sourceBaseUrl",
    "scriptSets",
    "referenceAppendices",
    "glossaries",
    "answerKeys",
    "indexes",
    "targets",
    "handwritten",
  ];
  const keys = Object.keys(record);
  if (
    keys.length !== expected.length ||
    keys.some((key, index) => key !== expected[index])
  ) {
    throw new Error(`book-generation must contain canonical top-level keys in order`);
  }
  exactMeta(
    { version: record.version, sourceBaseUrl: record.sourceBaseUrl },
    "book-generation",
  );
  const scriptSets = object(record.scriptSets, "book-generation.scriptSets");
  for (const [id, entries] of Object.entries(scriptSets)) {
    if (!SAFE_SLUG.test(id) || WINDOWS_RESERVED.test(id)) {
      throw new Error(`book-generation.scriptSets key '${id}' is unsafe`);
    }
    arrays(entries, `book-generation.scriptSets.${id}`);
  }
  const sections: Array<[
    string,
    unknown[],
    (entry: unknown, label: string) => string,
  ]> = [
    ["referenceAppendices", requiredArray(record, "referenceAppendices"), backmatterOwnerName],
    ["glossaries", requiredArray(record, "glossaries"), backmatterOwnerName],
    ["answerKeys", requiredArray(record, "answerKeys"), backmatterOwnerName],
    ["indexes", requiredArray(record, "indexes"), backmatterOwnerName],
    ["targets", requiredArray(record, "targets"), chapterOwnerName],
    ["handwritten", requiredArray(record, "handwritten"), chapterOwnerName],
  ];
  const outputs = new Set<string>();
  const usedScriptSets = new Set<string>();
  for (const [key, entries, ownerName] of sections) {
    assertSortedOwnerNames(entries, ownerName, `book-generation.${key}`);
    for (const [index, entry] of entries.entries()) {
      const row = object(entry, `book-generation.${key}[${index}]`);
      const output = safeOutput(row.output, `book-generation.${key}[${index}].output`);
      if (outputs.has(output)) throw new Error(`book-generation output '${output}' is duplicated`);
      outputs.add(output);
      if (row.scriptSet !== undefined) {
        const scriptSet = nonEmptyString(row.scriptSet, `${key}[${index}].scriptSet`);
        if (!Object.hasOwn(scriptSets, scriptSet)) {
          throw new Error(`${key}[${index}] references unknown scriptSet '${scriptSet}'`);
        }
        usedScriptSets.add(scriptSet);
      }
    }
  }
  for (const scriptSet of Object.keys(scriptSets)) {
    if (!usedScriptSets.has(scriptSet)) {
      throw new Error(`book-generation scriptSet '${scriptSet}' has no owning declaration`);
    }
  }
  const identities = bookGenerationIdentitySets(record as unknown as BookGenerationDocument);
  for (const identity of identities.targets) {
    if (identities.handwritten.has(identity)) {
      throw new Error(`book-generation chapter '${identity}' is both target and handwritten`);
    }
  }
  return record as unknown as BookGenerationDocument;
}

export function bookGenerationIdentitySets(
  value: BookGenerationDocument,
): BookGenerationIdentitySets {
  const targets = new Set<string>();
  const handwritten = new Set<string>();
  const languages = new Set<string>();
  for (const [kind, into, entries] of [
    ["targets", targets, value.targets ?? []],
    ["handwritten", handwritten, value.handwritten ?? []],
  ] as const) {
    for (const [index, raw] of entries.entries()) {
      const record = object(raw, `book-generation.${kind}[${index}]`);
      const language = safeLanguage(record.language, `${kind}[${index}].language`);
      const chapter = record.chapter;
      if (
        typeof chapter !== "number" ||
        !Number.isInteger(chapter) ||
        chapter < 1 ||
        chapter > 9999
      ) {
        throw new Error(`${kind}[${index}].chapter must be an integer from 1 through 9999`);
      }
      const identity = chapterIdentity(language, chapter);
      if (into.has(identity)) throw new Error(`${kind} duplicates '${identity}'`);
      into.add(identity);
      languages.add(language);
    }
  }
  const backmatter = (kind: keyof Pick<
    BookGenerationDocument,
    "referenceAppendices" | "glossaries" | "answerKeys" | "indexes"
  >): Set<string> => new Set(
    (value[kind] ?? []).map((entry, index) =>
      backmatterIdentity(entry, `book-generation.${kind}[${index}]`),
    ),
  );
  return {
    targets,
    handwritten,
    combined: new Set([...targets, ...handwritten]),
    languages,
    referenceAppendices: backmatter("referenceAppendices"),
    glossaries: backmatter("glossaries"),
    answerKeys: backmatter("answerKeys"),
    indexes: backmatter("indexes"),
  };
}

function sameSet(actual: ReadonlySet<string>, expected: ReadonlySet<string>): boolean {
  return actual.size === expected.size && [...actual].every((value) => expected.has(value));
}

export function assertBookGenerationIdentitySets(
  document: BookGenerationDocument,
  expected: {
    targets?: ReadonlySet<string>;
    handwritten?: ReadonlySet<string>;
    combined?: ReadonlySet<string>;
    languages?: ReadonlySet<string>;
    referenceAppendices?: ReadonlySet<string>;
    glossaries?: ReadonlySet<string>;
    answerKeys?: ReadonlySet<string>;
    indexes?: ReadonlySet<string>;
  },
): void {
  const actual = bookGenerationIdentitySets(document);
  for (const key of [
    "targets",
    "handwritten",
    "combined",
    "languages",
    "referenceAppendices",
    "glossaries",
    "answerKeys",
    "indexes",
  ] as const) {
    const wanted = expected[key];
    if (wanted !== undefined && !sameSet(actual[key], wanted)) {
      const missing = [...wanted].filter((value) => !actual[key].has(value)).sort(codeUnit);
      const unexpected = [...actual[key]].filter((value) => !wanted.has(value)).sort(codeUnit);
      throw new Error(
        `book-generation ${key} identity set differs: missing [${missing.join(", ")}], unexpected [${unexpected.join(", ")}]`,
      );
    }
  }
}

export function bookGenerationOwnerContents(
  value: BookGenerationDocument,
): Map<string, string> {
  const document = validateDocument(value);
  const outputs = new Map<string, string>();
  outputs.set(
    "_meta.json",
    serialize({ version: document.version, sourceBaseUrl: document.sourceBaseUrl }),
  );
  let ordinal = 10;
  for (const [id, entries] of Object.entries(document.scriptSets ?? {})) {
    outputs.set(
      `script-sets.d/${String(ordinal).padStart(4, "0")}-${id}.json`,
      serialize(entries),
    );
    ordinal += 10;
  }
  for (const [key, directory, ownerName] of [
    ["referenceAppendices", "reference-appendices.d", backmatterOwnerName],
    ["glossaries", "glossaries.d", backmatterOwnerName],
    ["answerKeys", "answer-keys.d", backmatterOwnerName],
    ["indexes", "indexes.d", backmatterOwnerName],
    ["targets", "targets.d", chapterOwnerName],
    ["handwritten", "handwritten.d", chapterOwnerName],
  ] as const) {
    for (const [index, entry] of (document[key] ?? []).entries()) {
      const name = `${directory}/${ownerName(entry, `${key}[${index}]`)}`;
      if (outputs.has(name)) throw new Error(`book-generation owner '${name}' is duplicated`);
      outputs.set(name, serialize(entry));
    }
  }
  return outputs;
}

function readStrictDirectory(root: string, directory: string): string[] {
  const path = join(root, BOOK_GENERATION_DIRECTORY, directory);
  const stat = lstatSync(path);
  if (stat.isSymbolicLink() || !stat.isDirectory()) {
    throw new Error(`book-generation owner '${path}' must be a real directory`);
  }
  return readdirSync(path).sort(codeUnit);
}

export function readBookGenerationOwners(root: string): LoadedBookGenerationOwners {
  const base = join(root, BOOK_GENERATION_DIRECTORY);
  assertRealDescendantComponents(root, base);
  const stat = statIfPresent(base);
  if (stat === undefined || stat.isSymbolicLink() || !stat.isDirectory()) {
    throw new Error(`book-generation must use the real owner directory '${base}'`);
  }
  const monolith = join(root, "core", "book-generation.json");
  if (statIfPresent(monolith) !== undefined) {
    throw new Error(`book-generation has a resurrected monolith '${monolith}'`);
  }
  const expectedTop = new Set(["_meta.json", ...BOOK_GENERATION_SECTION_DIRECTORIES]);
  const actualTop = readdirSync(base).sort(codeUnit);
  if (
    actualTop.length !== expectedTop.size ||
    actualTop.some((name) => !expectedTop.has(name as (typeof BOOK_GENERATION_SECTION_DIRECTORIES)[number] | "_meta.json"))
  ) {
    throw new Error(`book-generation owner root has missing, legacy, or unexpected entries`);
  }
  const metaPath = join(base, "_meta.json");
  const metaStat = lstatSync(metaPath);
  if (metaStat.isSymbolicLink() || !metaStat.isFile()) {
    throw new Error(`book-generation owner '${metaPath}' must be a real regular file`);
  }
  const meta = exactMeta(readLedgerFile(metaPath), "book-generation.d/_meta.json");
  const sourcePaths = [metaPath];

  const readSection = (
    directory: string,
    expectedName: (value: unknown, label: string) => string,
  ): unknown[] => {
    const names = readStrictDirectory(root, directory);
    const entries: unknown[] = [];
    for (const name of names) {
      if (!name.endsWith(".json")) {
        throw new Error(`book-generation owner '${directory}/${name}' has a malformed name`);
      }
      const path = join(base, directory, name);
      const ownerStat = lstatSync(path);
      if (ownerStat.isSymbolicLink() || !ownerStat.isFile()) {
        throw new Error(`book-generation owner '${path}' must be a real regular file`);
      }
      const entry = readLedgerFile(path);
      if (expectedName(entry, `${directory}/${name}`) !== name) {
        throw new Error(`book-generation owner '${directory}/${name}' does not match its record identity`);
      }
      entries.push(entry);
      sourcePaths.push(path);
    }
    return entries;
  };

  const scriptSets: Record<string, unknown[]> = {};
  let previousOrdinal = 0;
  for (const name of readStrictDirectory(root, "script-sets.d")) {
    const match = SCRIPT_SET_OWNER.exec(name);
    const ordinal = match === null ? 0 : Number(match[1]);
    if (match === null || ordinal <= previousOrdinal) {
      throw new Error(`book-generation script-set owner '${name}' is malformed or out of canonical order`);
    }
    const id = match[2];
    if (Object.hasOwn(scriptSets, id)) throw new Error(`book-generation scriptSet '${id}' is duplicated`);
    const path = join(base, "script-sets.d", name);
    const ownerStat = lstatSync(path);
    if (ownerStat.isSymbolicLink() || !ownerStat.isFile()) {
      throw new Error(`book-generation owner '${path}' must be a real regular file`);
    }
    scriptSets[id] = arrays(readLedgerFile(path), `script-sets.d/${name}`);
    sourcePaths.push(path);
    previousOrdinal = ordinal;
  }

  const document: BookGenerationDocument = {
    ...meta,
    scriptSets,
    referenceAppendices: readSection("reference-appendices.d", backmatterOwnerName) as BookGenerationBackmatterOwner[],
    glossaries: readSection("glossaries.d", backmatterOwnerName) as BookGenerationBackmatterOwner[],
    answerKeys: readSection("answer-keys.d", backmatterOwnerName) as BookGenerationBackmatterOwner[],
    indexes: readSection("indexes.d", backmatterOwnerName) as BookGenerationBackmatterOwner[],
    targets: readSection("targets.d", chapterOwnerName) as BookGenerationChapterOwner[],
    handwritten: readSection("handwritten.d", chapterOwnerName) as BookGenerationChapterOwner[],
  };
  return { document: validateDocument(document), sourcePaths };
}

export function assertBookGenerationOwnerNames(
  root: string,
  expectedRelativeNames: Iterable<string>,
): void {
  const expected = new Set(expectedRelativeNames);
  const loaded = readBookGenerationOwners(root);
  const actual = new Set(
    loaded.sourcePaths.map((path) => relative(join(root, BOOK_GENERATION_DIRECTORY), path).split(sep).join("/")),
  );
  if (!sameSet(actual, expected)) {
    throw new Error(`book-generation owner set is missing, stale, or malformed`);
  }
}
