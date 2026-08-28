import { lstatSync, readFileSync, readdirSync, realpathSync } from "node:fs";
import { join, relative, resolve, sep } from "node:path";
import { isAbsentErrno, readLedgerFile } from "./shard.js";
import {
  parseSoundTagRegistry,
  type SoundTagRegistry,
} from "./sound-tags.js";

export const SOUND_TAG_REGISTRY_PATH = "core/sound-tags.json";
export const SOUND_TAG_OWNER_DIRECTORY = "core/sound-tags.d";
export const SOUND_TAG_META_OWNER = "_meta.json";

const LANGUAGE = /^[a-z][a-z0-9-]*$/;
const WINDOWS_RESERVED = /^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])$/;

function safeLanguage(value: string): boolean {
  return LANGUAGE.test(value) && !WINDOWS_RESERVED.test(value);
}

interface SoundTagMetaOwner {
  version: 1;
}

interface SoundTagTrackOwner {
  language: string;
  tags: readonly string[];
}

export interface SoundTagOwnerReadOptions {
  /** Independent registered-language identities used to catch clean deletion. */
  expectedLanguages: readonly string[];
  /** Migration fixtures may retain the source aggregate until staged owners validate. */
  rejectMonolith?: boolean;
  /** Canonical owners are reviewed data, so whitespace/key drift is also drift. */
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
      `sound-tag owner '${path}': cannot be inspected (${code ?? "unknown error"})`,
      { cause },
    );
  }
}

function object(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`sound-tag owner '${label}' must contain one JSON object`);
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
    throw new Error(
      `sound-tag owner '${label}' must contain exactly: ${wanted.join(", ")}`,
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
    throw new Error(`sound-tag owner path '${target}' is outside '${root}'`);
  }
  const realRoot = realpathSync(absoluteRoot);
  let current = absoluteRoot;
  for (const component of route.split(sep).filter(Boolean)) {
    current = join(current, component);
    const stat = lstatSync(current);
    if (stat.isSymbolicLink()) {
      throw new Error(
        `sound-tag owner path component '${current}' must not be a symbolic link`,
      );
    }
    const real = realpathSync(current);
    if (real !== realRoot && !real.startsWith(realRoot + sep)) {
      throw new Error(
        `sound-tag owner path component '${current}' resolves outside '${root}'`,
      );
    }
  }
}

function assertIdentitySet(
  actual: readonly string[],
  expected: readonly string[],
): void {
  const found = [...actual].sort();
  const wanted = [...expected].sort();
  const unsafe = wanted.find((language) => !safeLanguage(language));
  if (unsafe !== undefined) {
    throw new Error(`sound-tag expected language '${unsafe}' is unsafe`);
  }
  const duplicate = wanted.find((language, index) => wanted[index - 1] === language);
  if (duplicate !== undefined) {
    throw new Error(`sound-tag expected languages repeat '${duplicate}'`);
  }
  const missing = wanted.filter((language) => !found.includes(language));
  const extra = found.filter((language) => !wanted.includes(language));
  if (missing.length > 0 || extra.length > 0) {
    throw new Error(
      `sound-tag owner languages do not match the registry` +
        `${missing.length > 0 ? `; missing: ${missing.join(", ")}` : ""}` +
        `${extra.length > 0 ? `; extra: ${extra.join(", ")}` : ""}`,
    );
  }
}

function parseMeta(value: unknown): SoundTagMetaOwner {
  const meta = object(value, SOUND_TAG_META_OWNER);
  exactKeys(meta, ["version"], SOUND_TAG_META_OWNER);
  if (meta.version !== 1) {
    throw new Error(`sound-tag owner '${SOUND_TAG_META_OWNER}'.version must be 1`);
  }
  return { version: 1 };
}

function parseTrackOwner(value: unknown, filename: string): SoundTagTrackOwner {
  const owner = object(value, filename);
  exactKeys(owner, ["language", "tags"], filename);
  const language = owner.language;
  if (typeof language !== "string" || !safeLanguage(language)) {
    throw new Error(`sound-tag owner '${filename}'.language is unsafe`);
  }
  const expected = `${language}.json`;
  if (filename !== expected) {
    throw new Error(
      `sound-tag owner '${filename}' carries language '${language}', expected '${expected}'`,
    );
  }
  const parsed = parseSoundTagRegistry({
    version: 1,
    tracks: { [language]: owner.tags },
  });
  return { language, tags: parsed.tracks[language]! };
}

/** Stable direct-owner bytes for one complete registry. */
export function soundTagOwnerContents(
  input: SoundTagRegistry,
): Map<string, string> {
  const registry = parseSoundTagRegistry(input);
  const out = new Map<string, string>();
  out.set(SOUND_TAG_META_OWNER, canonical({ version: registry.version }));
  for (const language of Object.keys(registry.tracks)) {
    if (!safeLanguage(language)) {
      throw new Error(`sound-tag owner language '${language}' is unsafe`);
    }
    out.set(
      `${language}.json`,
      canonical({ language, tags: registry.tracks[language] }),
    );
  }
  return out;
}

/** Prove registry completeness from identities that do not come from its owners. */
export function assertSoundTagRegistryLanguages(
  input: SoundTagRegistry,
  expectedLanguages: readonly string[],
): void {
  const registry = parseSoundTagRegistry(input);
  assertIdentitySet(Object.keys(registry.tracks), expectedLanguages);
}

/**
 * Strictly fold `core/sound-tags.d/` into the historical public registry shape.
 *
 * This deliberately has no legacy fallback. Once any owner directory is the
 * source of truth, accepting the aggregate again would make a resurrected file
 * look live while every canonical consumer silently ignored it.
 */
export function readSoundTagRegistryOwners(
  root: string,
  options: SoundTagOwnerReadOptions,
): SoundTagRegistry {
  const directory = join(root, SOUND_TAG_OWNER_DIRECTORY);
  const monolith = join(root, SOUND_TAG_REGISTRY_PATH);
  const monolithStat = statIfPresent(monolith);
  const directoryStat = statIfPresent(directory);
  if (directoryStat === undefined) {
    if (monolithStat !== undefined) {
      throw new Error(
        `${SOUND_TAG_REGISTRY_PATH} is a legacy aggregate; migrate it to canonical ` +
          `${SOUND_TAG_OWNER_DIRECTORY} direct owners`,
      );
    }
    throw new Error(`sound-tag owner directory '${directory}' is missing`);
  }
  if (options.rejectMonolith !== false && monolithStat !== undefined) {
    throw new Error(
      `${SOUND_TAG_REGISTRY_PATH} is present beside canonical ${SOUND_TAG_OWNER_DIRECTORY}; ` +
        `move its edits into direct owners and remove the aggregate`,
    );
  }
  if (directoryStat.isSymbolicLink() || !directoryStat.isDirectory()) {
    throw new Error(
      `sound-tag owner directory '${directory}' must be a real directory`,
    );
  }
  assertRealDescendantComponents(root, directory);

  const entries = readdirSync(directory, { withFileTypes: true }).sort((a, b) =>
    a.name < b.name ? -1 : a.name > b.name ? 1 : 0,
  );
  const names = entries.map((entry) => entry.name);
  if (!names.includes(SOUND_TAG_META_OWNER)) {
    throw new Error(`sound-tag owner '${SOUND_TAG_META_OWNER}' is missing`);
  }
  for (const entry of entries) {
    if (entry.isSymbolicLink() || !entry.isFile()) {
      throw new Error(
        `sound-tag owner '${entry.name}' must be a real direct-child regular file`,
      );
    }
    if (
      entry.name !== SOUND_TAG_META_OWNER &&
      !safeLanguage(entry.name.replace(/\.json$/, ""))
    ) {
      throw new Error(`unexpected sound-tag owner '${entry.name}'`);
    }
    if (!entry.name.endsWith(".json")) {
      throw new Error(`unexpected sound-tag owner '${entry.name}'`);
    }
  }

  const ownerLanguages = names
    .filter((name) => name !== SOUND_TAG_META_OWNER)
    .map((name) => name.slice(0, -".json".length));
  // Establish completeness from independent registry identities before any
  // owner bytes are opened. A deleted owner must fail as a missing filename,
  // not disappear from the corpus whose bytes happen to survive.
  assertIdentitySet(ownerLanguages, options.expectedLanguages);

  const metaPath = join(directory, SOUND_TAG_META_OWNER);
  const meta = parseMeta(readLedgerFile(metaPath));
  if (options.requireCanonicalBytes !== false) {
    const actual = readFileSync(metaPath, "utf8");
    const expected = canonical(meta);
    if (actual !== expected) {
      throw new Error(`sound-tag owner '${SOUND_TAG_META_OWNER}' is not canonical`);
    }
  }

  const tracks: Record<string, readonly string[]> = {};
  for (const name of names) {
    if (name === SOUND_TAG_META_OWNER) continue;
    const path = join(directory, name);
    const owner = parseTrackOwner(readLedgerFile(path), name);
    if (Object.hasOwn(tracks, owner.language)) {
      throw new Error(`two sound-tag owners claim language '${owner.language}'`);
    }
    tracks[owner.language] = owner.tags;
    if (options.requireCanonicalBytes !== false) {
      const actual = readFileSync(path, "utf8");
      const expected = canonical(owner);
      if (actual !== expected) {
        throw new Error(`sound-tag owner '${name}' is not canonical`);
      }
    }
  }

  return parseSoundTagRegistry({ version: meta.version, tracks });
}

export function serializeSoundTagRegistry(registry: SoundTagRegistry): string {
  return canonical(parseSoundTagRegistry(registry));
}
