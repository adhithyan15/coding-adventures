import { lstatSync, readdirSync } from "node:fs";

/** The exact debt ratchet and its track-owned audit trail. */
export interface EtymologyGapPinOwner {
  readonly language: string;
  readonly withoutHook: number;
  readonly withoutVisible: number;
  readonly rationale: readonly string[];
}

export type EtymologyGapPinOwners = Readonly<Record<string, EtymologyGapPinOwner>>;

const OWNER_NAME = /^[a-z][a-z0-9]*(?:-[a-z0-9]+)*\.owner\.ts$/;
const TRACK_SLUG = /^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$/;
const RESERVED_NAMES = new Set([
  "index.owner.ts",
  "manifest.owner.ts",
  "readme.owner.ts",
  "registry.owner.ts",
]);
const OWNER_EXPORT = "etymologyGapPinOwner";
const OWNER_FIELDS = ["language", "rationale", "withoutHook", "withoutVisible"];
const MODULE_PREFIX = "./etymology-gap-pin-owners/";

const compareAscii = (left: string, right: string): number =>
  left < right ? -1 : left > right ? 1 : 0;

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

/**
 * Validate names separately so case-folded collisions can be tested on a
 * case-insensitive development filesystem without pretending two such files
 * can coexist there.
 */
export function validateEtymologyGapPinOwnerNames(names: readonly string[]): void {
  const folded = new Map<string, string>();
  for (const name of names) {
    const key = name.toLocaleLowerCase("en-US");
    const prior = folded.get(key);
    if (prior !== undefined) {
      throw new Error(
        `etymology-gap pin owners collide when case-folded: '${prior}' and '${name}'`,
      );
    }
    folded.set(key, name);
  }

  for (const name of names) {
    if (!OWNER_NAME.test(name)) {
      throw new Error(
        `unsafe etymology-gap pin owner '${name}': use a lowercase track slug plus .owner.ts`,
      );
    }
    if (RESERVED_NAMES.has(name)) {
      throw new Error(
        `reserved etymology-gap pin owner '${name}' would create a shared registry`,
      );
    }
  }
}

const validateExpectedTracks = (languages: readonly string[]): Set<string> => {
  const expected = new Set<string>();
  const folded = new Set<string>();
  for (const language of languages) {
    if (!TRACK_SLUG.test(language)) {
      throw new Error(`unsafe expected track '${language}': use a lowercase ASCII slug`);
    }
    const key = language.toLocaleLowerCase("en-US");
    if (folded.has(key)) {
      throw new Error(`duplicate expected track '${language}'`);
    }
    folded.add(key);
    expected.add(language);
  }
  if (expected.size === 0) {
    throw new Error("etymology-gap pin expected track set must not be empty");
  }
  return expected;
};

const validatePinValue = (filename: string, field: string, value: unknown): number => {
  if (!Number.isSafeInteger(value) || (value as number) < 0) {
    throw new Error(`${filename}: ${field} must be a non-negative safe integer`);
  }
  return value as number;
};

const validateRationale = (filename: string, value: unknown): readonly string[] => {
  if (!Array.isArray(value) || value.length === 0) {
    throw new Error(`${filename}: rationale must be a non-empty string array`);
  }
  const paragraphs = value.map((paragraph, index) => {
    if (
      typeof paragraph !== "string" ||
      paragraph.length === 0 ||
      paragraph.trim() !== paragraph ||
      /[\u0000-\u001f\u007f]/.test(paragraph)
    ) {
      throw new Error(`${filename}: rationale paragraph ${index + 1} is malformed`);
    }
    return paragraph;
  });
  return Object.freeze(paragraphs);
};

const validateOwner = (
  filename: string,
  moduleValue: unknown,
  expected: ReadonlySet<string>,
): EtymologyGapPinOwner => {
  if (
    !isRecord(moduleValue) ||
    Object.keys(moduleValue).length !== 1 ||
    !Object.hasOwn(moduleValue, OWNER_EXPORT)
  ) {
    throw new Error(`${filename}: expected exactly the ${OWNER_EXPORT} export`);
  }

  const value = moduleValue[OWNER_EXPORT];
  if (!isRecord(value)) {
    throw new Error(`${filename}: ${OWNER_EXPORT} must be an object`);
  }
  const fields = Object.keys(value).sort(compareAscii);
  if (
    fields.length !== OWNER_FIELDS.length ||
    fields.some((field, index) => field !== OWNER_FIELDS[index])
  ) {
    throw new Error(
      `${filename}: expected exactly language, rationale, withoutHook, and withoutVisible`,
    );
  }

  const filenameLanguage = filename.slice(0, -".owner.ts".length);
  if (value.language !== filenameLanguage) {
    throw new Error(
      `${filename}: declares language '${String(value.language)}' instead of '${filenameLanguage}'`,
    );
  }
  if (!expected.has(filenameLanguage)) {
    throw new Error(`unknown track owner '${filenameLanguage}' in ${filename}`);
  }

  return Object.freeze({
    language: filenameLanguage,
    withoutHook: validatePinValue(filename, "withoutHook", value.withoutHook),
    withoutVisible: validatePinValue(filename, "withoutVisible", value.withoutVisible),
    rationale: validateRationale(filename, value.rationale),
  });
};

/**
 * Compose the bounded Vite module set only after validating its real files.
 * The corpus-derived expected track set is the completeness authority; this
 * loader owns no parallel track registry or count pin.
 */
export function loadEtymologyGapPinOwners(
  directory: string,
  modules: Readonly<Record<string, unknown>>,
  expectedLanguages: readonly string[],
): EtymologyGapPinOwners {
  const root = lstatSync(directory);
  if (root.isSymbolicLink()) {
    throw new Error("etymology-gap pin owner directory must not be a symbolic link");
  }
  if (!root.isDirectory()) {
    throw new Error("etymology-gap pin owner root must be a directory");
  }

  const entries = readdirSync(directory, { withFileTypes: true }).sort((left, right) =>
    compareAscii(left.name, right.name),
  );
  const names = entries.map((entry) => entry.name);
  validateEtymologyGapPinOwnerNames(names);
  for (const entry of entries) {
    const stat = lstatSync(`${directory}/${entry.name}`);
    if (stat.isSymbolicLink()) {
      throw new Error(`${entry.name}: etymology-gap pin owner must not be a symbolic link`);
    }
    if (!stat.isFile()) {
      throw new Error(`${entry.name}: etymology-gap pin owner must be a regular file`);
    }
  }
  if (entries.length === 0) {
    throw new Error("etymology-gap pin owner set must not be empty");
  }

  const expected = validateExpectedTracks(expectedLanguages);
  const filenames = new Set(names);
  for (const modulePath of Object.keys(modules)) {
    if (!modulePath.startsWith(MODULE_PREFIX)) {
      throw new Error(`unexpected bounded-glob module '${modulePath}'`);
    }
    const filename = modulePath.slice(MODULE_PREFIX.length);
    if (!filenames.has(filename)) {
      throw new Error(`unexpected bounded-glob module '${modulePath}'`);
    }
  }

  const owners: Record<string, EtymologyGapPinOwner> = {};
  for (const entry of entries) {
    const modulePath = `${MODULE_PREFIX}${entry.name}`;
    if (!Object.hasOwn(modules, modulePath)) {
      throw new Error(`${entry.name}: has no bounded-glob module`);
    }
    const owner = validateOwner(entry.name, modules[modulePath], expected);
    if (Object.hasOwn(owners, owner.language)) {
      throw new Error(`duplicate etymology-gap pin owner '${owner.language}'`);
    }
    owners[owner.language] = owner;
  }

  for (const language of [...expected].sort(compareAscii)) {
    if (!Object.hasOwn(owners, language)) {
      throw new Error(`missing track owner '${language}'`);
    }
  }
  return Object.freeze(owners);
}
