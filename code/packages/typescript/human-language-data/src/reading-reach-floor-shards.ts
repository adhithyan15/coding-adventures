import { lstatSync, readFileSync, readdirSync, realpathSync } from "node:fs";
import { join, relative, resolve, sep } from "node:path";
import { CEFR_LEVELS, type CefrLevel } from "./levels.js";
import { isAbsentErrno, readLedgerFile } from "./shard.js";

export const READING_REACH_FLOOR_PATH = "core/reading-reach-floor.json";
export const READING_REACH_FLOOR_OWNER_DIRECTORY = "core/reading-reach-floor.d";
export const READING_REACH_FLOOR_META_OWNER = "_meta.json";

export const READING_REACH_FLOOR_ABOUT =
  "A ratchet, not a snapshot. Each entry is the number of published reading parts a track's longest passage ALREADY reaches; `reading-reach.test.ts` fails if the measured value drops below it. Absent entries floor at zero, so a track with no passage costs nothing here and only a track that has actually gained reading is ever edited. Raise an entry when a track's reading grows; never lower one to make a red test green -- a falling number means a passage was shortened or deleted, which is the regression this file exists to catch.";

const LANGUAGE = /^[a-z][a-z0-9-]*$/;
const WINDOWS_RESERVED = /^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])$/i;

export interface ReadingReachFloorIdentity {
  language: string;
  level: CefrLevel;
}

export interface ReadingReachFloorRegistry {
  version: 1;
  about: string;
  floors: Record<string, number>;
}

interface ReadingReachFloorOwner extends ReadingReachFloorIdentity {
  floor: number;
}

export interface ReadingReachFloorReadOptions {
  /** Task-shape identities are independent of these owners and catch clean deletion. */
  expectedInventories: readonly ReadingReachFloorIdentity[];
  /** Migration fixtures may retain the aggregate until staged owners validate. */
  rejectMonolith?: boolean;
  /** Ratchet owners are reviewed policy, so whitespace/key drift is drift too. */
  requireCanonicalBytes?: boolean;
}

function canonical(value: unknown): string {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function object(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`reading-reach floor owner '${label}' must contain one JSON object`);
  }
  return value as Record<string, unknown>;
}

function exactKeys(value: Record<string, unknown>, expected: readonly string[], label: string): void {
  const actual = Object.keys(value).sort();
  const wanted = [...expected].sort();
  if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) {
    throw new Error(
      `reading-reach floor owner '${label}' must contain exactly: ${wanted.join(", ")}`,
    );
  }
}

function safeLanguage(value: string): boolean {
  return LANGUAGE.test(value) && !WINDOWS_RESERVED.test(value);
}

function levelSlug(level: CefrLevel): string {
  return level.toLowerCase();
}

export function readingReachFloorOwnerFilename(identity: ReadingReachFloorIdentity): string {
  if (!safeLanguage(identity.language)) {
    throw new Error(`reading-reach floor language '${identity.language}' is unsafe`);
  }
  if (!CEFR_LEVELS.includes(identity.level)) {
    throw new Error(`reading-reach floor level '${identity.level}' is invalid`);
  }
  return `${identity.language}--${levelSlug(identity.level)}.json`;
}

function identityKey(identity: ReadingReachFloorIdentity): string {
  return `${identity.language}/${identity.level}`;
}

function statIfPresent(path: string): ReturnType<typeof lstatSync> | undefined {
  try {
    return lstatSync(path);
  } catch (cause) {
    const code = (cause as NodeJS.ErrnoException).code;
    if (isAbsentErrno(code)) return undefined;
    throw new Error(
      `reading-reach floor owner '${path}': cannot be inspected (${code ?? "unknown error"})`,
      { cause },
    );
  }
}

function assertRealDescendantComponents(root: string, target: string): void {
  const absoluteRoot = resolve(root);
  const absoluteTarget = resolve(target);
  const route = relative(absoluteRoot, absoluteTarget);
  if (route === ".." || route.startsWith(`..${sep}`) || resolve(absoluteRoot, route) !== absoluteTarget) {
    throw new Error(`reading-reach floor owner path '${target}' is outside '${root}'`);
  }
  const realRoot = realpathSync(absoluteRoot);
  let current = absoluteRoot;
  for (const component of route.split(sep).filter(Boolean)) {
    current = join(current, component);
    const stat = lstatSync(current);
    if (stat.isSymbolicLink()) {
      throw new Error(
        `reading-reach floor owner path component '${current}' must not be a symbolic link`,
      );
    }
    const real = realpathSync(current);
    if (real !== realRoot && !real.startsWith(realRoot + sep)) {
      throw new Error(
        `reading-reach floor owner path component '${current}' resolves outside '${root}'`,
      );
    }
  }
}

function parseMeta(value: unknown): { version: 1; about: string } {
  const meta = object(value, READING_REACH_FLOOR_META_OWNER);
  exactKeys(meta, ["version", "about"], READING_REACH_FLOOR_META_OWNER);
  if (meta.version !== 1) {
    throw new Error(`reading-reach floor owner '${READING_REACH_FLOOR_META_OWNER}'.version must be 1`);
  }
  if (meta.about !== READING_REACH_FLOOR_ABOUT) {
    throw new Error(`reading-reach floor owner '${READING_REACH_FLOOR_META_OWNER}'.about is not canonical`);
  }
  return { version: 1, about: READING_REACH_FLOOR_ABOUT };
}

function parseOwner(value: unknown, filename: string): ReadingReachFloorOwner {
  const owner = object(value, filename);
  exactKeys(owner, ["language", "level", "floor"], filename);
  if (typeof owner.language !== "string" || !safeLanguage(owner.language)) {
    throw new Error(`reading-reach floor owner '${filename}'.language is unsafe`);
  }
  const level = CEFR_LEVELS.find((candidate) => candidate === owner.level);
  if (level === undefined) {
    throw new Error(`reading-reach floor owner '${filename}'.level must be pre-A1 through C2`);
  }
  if (!Number.isInteger(owner.floor) || (owner.floor as number) < 0) {
    throw new Error(`reading-reach floor owner '${filename}'.floor must be a non-negative integer`);
  }
  const parsed = { language: owner.language, level, floor: owner.floor as number };
  const expected = readingReachFloorOwnerFilename(parsed);
  if (filename !== expected) {
    throw new Error(
      `reading-reach floor owner '${filename}' carries '${identityKey(parsed)}', expected '${expected}'`,
    );
  }
  return parsed;
}

function expectedOwnerNames(expectedInventories: readonly ReadingReachFloorIdentity[]): string[] {
  const names: string[] = [];
  const identities = new Set<string>();
  const folded = new Map<string, string>();
  for (const inventory of expectedInventories) {
    const key = identityKey(inventory);
    if (identities.has(key)) {
      throw new Error(`reading-reach floor expected inventories repeat '${key}'`);
    }
    identities.add(key);
    const name = readingReachFloorOwnerFilename(inventory);
    const lower = name.toLowerCase();
    const prior = folded.get(lower);
    if (prior !== undefined) {
      throw new Error(`reading-reach floor expected owners have a case-fold collision: '${prior}'/'${name}'`);
    }
    folded.set(lower, name);
    names.push(name);
  }
  return names.sort();
}

function assertExactOwnerNames(actual: readonly string[], expected: readonly string[]): void {
  const found = [...actual].sort();
  const wanted = [...expected].sort();
  const missing = wanted.filter((name) => !found.includes(name));
  const extra = found.filter((name) => !wanted.includes(name));
  if (missing.length > 0 || extra.length > 0) {
    throw new Error(
      `reading-reach floor owners do not match task-shape inventories` +
        `${missing.length > 0 ? `; missing: ${missing.join(", ")}` : ""}` +
        `${extra.length > 0 ? `; extra: ${extra.join(", ")}` : ""}`,
    );
  }
}

/** Build canonical direct-owner bytes, including explicit zero owners. */
export function readingReachFloorOwnerContents(
  input: ReadingReachFloorRegistry,
  inventories: readonly ReadingReachFloorIdentity[],
): Map<string, string> {
  if (input.version !== 1) throw new Error("reading-reach floor registry.version must be 1");
  const names = expectedOwnerNames(inventories);
  const byName = new Map(
    inventories.map((inventory) => [readingReachFloorOwnerFilename(inventory), inventory]),
  );
  const known = new Set(inventories.map(identityKey));
  for (const [key, floor] of Object.entries(input.floors)) {
    if (!known.has(key)) throw new Error(`reading-reach floor '${key}' has no task-shape inventory`);
    if (!Number.isInteger(floor) || floor < 0) {
      throw new Error(`reading-reach floor '${key}' must be a non-negative integer`);
    }
  }
  const out = new Map<string, string>();
  out.set(
    READING_REACH_FLOOR_META_OWNER,
    canonical({ version: 1, about: READING_REACH_FLOOR_ABOUT }),
  );
  for (const name of names) {
    const inventory = byName.get(name)!;
    out.set(
      name,
      canonical({
        language: inventory.language,
        level: inventory.level,
        floor: input.floors[identityKey(inventory)] ?? 0,
      }),
    );
  }
  return out;
}

/** Strictly reconstruct the historical positive-floor registry from direct owners. */
export function readReadingReachFloorOwners(
  root: string,
  options: ReadingReachFloorReadOptions,
): ReadingReachFloorRegistry {
  const directory = join(root, READING_REACH_FLOOR_OWNER_DIRECTORY);
  const aggregate = join(root, READING_REACH_FLOOR_PATH);
  const directoryStat = statIfPresent(directory);
  const aggregateStat = statIfPresent(aggregate);
  if (directoryStat === undefined) {
    if (aggregateStat !== undefined) {
      throw new Error(
        `${READING_REACH_FLOOR_PATH} is a legacy aggregate; migrate it to canonical ` +
          `${READING_REACH_FLOOR_OWNER_DIRECTORY} direct owners`,
      );
    }
    throw new Error(`reading-reach floor owner directory '${directory}' is missing`);
  }
  if (options.rejectMonolith !== false && aggregateStat !== undefined) {
    throw new Error(
      `${READING_REACH_FLOOR_PATH} is present beside canonical ` +
        `${READING_REACH_FLOOR_OWNER_DIRECTORY}; move its edits into direct owners and remove the aggregate`,
    );
  }
  if (directoryStat.isSymbolicLink() || !directoryStat.isDirectory()) {
    throw new Error(`reading-reach floor owner directory '${directory}' must be a real directory`);
  }
  assertRealDescendantComponents(root, directory);

  const entries = readdirSync(directory, { withFileTypes: true }).sort((a, b) =>
    a.name < b.name ? -1 : a.name > b.name ? 1 : 0,
  );
  const folded = new Map<string, string>();
  for (const entry of entries) {
    if (entry.isSymbolicLink() || !entry.isFile()) {
      throw new Error(
        `reading-reach floor owner '${entry.name}' must be a real direct-child regular file`,
      );
    }
    const lower = entry.name.toLowerCase();
    const prior = folded.get(lower);
    if (prior !== undefined) {
      throw new Error(`reading-reach floor owners have a case-fold collision: '${prior}'/'${entry.name}'`);
    }
    folded.set(lower, entry.name);
  }

  const names = entries.map((entry) => entry.name);
  if (!names.includes(READING_REACH_FLOOR_META_OWNER)) {
    throw new Error(`reading-reach floor owner '${READING_REACH_FLOOR_META_OWNER}' is missing`);
  }
  const ownerNames = names.filter((name) => name !== READING_REACH_FLOOR_META_OWNER);
  assertExactOwnerNames(ownerNames, expectedOwnerNames(options.expectedInventories));

  const metaPath = join(directory, READING_REACH_FLOOR_META_OWNER);
  const meta = parseMeta(readLedgerFile(metaPath));
  if (options.requireCanonicalBytes !== false && readFileSync(metaPath, "utf8") !== canonical(meta)) {
    throw new Error(`reading-reach floor owner '${READING_REACH_FLOOR_META_OWNER}' is not canonical`);
  }

  const floors: Record<string, number> = Object.create(null);
  for (const name of ownerNames) {
    const path = join(directory, name);
    const owner = parseOwner(readLedgerFile(path), name);
    if (options.requireCanonicalBytes !== false && readFileSync(path, "utf8") !== canonical(owner)) {
      throw new Error(`reading-reach floor owner '${name}' is not canonical`);
    }
    if (owner.floor > 0) floors[identityKey(owner)] = owner.floor;
  }
  return { version: 1, about: meta.about, floors };
}
