import {
  lstatSync,
  readFileSync,
  readdirSync,
  realpathSync,
} from "node:fs";
import { basename, join, relative, resolve, sep } from "node:path";
import type {
  CurriculumExtensionMembership,
  CurriculumLessonMembership,
} from "./curriculum-membership.js";
import { isAbsentErrno, readLedgerFile } from "./shard.js";

export const CURRICULUM_MEMBERSHIP_DIRECTORY = "curriculum-membership.d";
export const CURRICULUM_MEMBERSHIP_META_OWNER = "_meta.json";

const SAFE_LANGUAGE = /^[a-z][a-z0-9-]*$/;
const SAFE_LESSON_ID = /^[A-Za-z0-9][A-Za-z0-9_-]*$/;
const WINDOWS_RESERVED = /^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])$/i;

export interface LoadedCurriculumMembershipOwners {
  owners: CurriculumLessonMembership[];
  /** Every identity and owner source that a build tool must watch. */
  sourcePaths: string[];
}

function canonical(value: unknown): string {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function object(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`curriculum membership owner '${label}' must contain one JSON object`);
  }
  return value as Record<string, unknown>;
}

function exactKeys(value: Record<string, unknown>, expected: readonly string[], label: string): void {
  const actual = Object.keys(value).sort();
  const wanted = [...expected].sort();
  if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) {
    throw new Error(
      `curriculum membership owner '${label}' must contain exactly: ${wanted.join(", ")}`,
    );
  }
}

function statIfPresent(path: string): ReturnType<typeof lstatSync> | undefined {
  try {
    return lstatSync(path);
  } catch (cause) {
    const code = (cause as NodeJS.ErrnoException).code;
    if (isAbsentErrno(code)) return undefined;
    throw new Error(
      `curriculum membership owner '${path}': cannot be inspected (${code ?? "unknown error"})`,
      { cause },
    );
  }
}

function assertRealDescendantComponents(root: string, target: string): void {
  const absoluteRoot = resolve(root);
  const absoluteTarget = resolve(target);
  const route = relative(absoluteRoot, absoluteTarget);
  if (route === ".." || route.startsWith(`..${sep}`) || resolve(absoluteRoot, route) !== absoluteTarget) {
    throw new Error(`curriculum membership owner path '${target}' is outside '${root}'`);
  }
  const realRoot = realpathSync(absoluteRoot);
  let current = absoluteRoot;
  for (const component of route.split(sep).filter(Boolean)) {
    current = join(current, component);
    const stat = lstatSync(current);
    if (stat.isSymbolicLink()) {
      throw new Error(
        `curriculum membership owner path component '${current}' must not be a symbolic link`,
      );
    }
    const real = realpathSync(current);
    if (real !== realRoot && !real.startsWith(realRoot + sep)) {
      throw new Error(
        `curriculum membership owner path component '${current}' resolves outside '${root}'`,
      );
    }
  }
}

function safeLessonId(id: string, label: string): string {
  if (!SAFE_LESSON_ID.test(id) || WINDOWS_RESERVED.test(id)) {
    throw new Error(`curriculum membership ${label} '${id}' is unsafe`);
  }
  return id;
}

function orderedUniqueLessonIds(ids: readonly string[], label: string): string[] {
  const exact = new Set<string>();
  const folded = new Map<string, string>();
  const out: string[] = [];
  for (const raw of ids) {
    const id = safeLessonId(raw, label);
    if (exact.has(id)) throw new Error(`curriculum membership ${label} repeats '${id}'`);
    exact.add(id);
    const lower = id.toLowerCase();
    const prior = folded.get(lower);
    if (prior !== undefined) {
      throw new Error(
        `curriculum membership ${label} has a case-fold collision: '${prior}'/'${id}'`,
      );
    }
    folded.set(lower, id);
    out.push(id);
  }
  return out.sort();
}

function parseExtension(value: unknown, filename: string, index: number): CurriculumExtensionMembership {
  const label = `${filename}.extensions[${index}]`;
  const extension = object(value, label);
  exactKeys(extension, ["id", "order"], label);
  if (typeof extension.id !== "string" || extension.id.length === 0) {
    throw new Error(`curriculum membership owner '${label}'.id must be a non-empty string`);
  }
  if (!Number.isInteger(extension.order) || (extension.order as number) < 0) {
    throw new Error(`curriculum membership owner '${label}'.order must be a non-negative integer`);
  }
  return { id: extension.id, order: extension.order as number };
}

function parseOwner(value: unknown, filename: string, filenameId: string): CurriculumLessonMembership {
  const owner = object(value, filename);
  exactKeys(owner, ["id", "pathSegment", "pathOrder", "extensions"], filename);
  if (owner.id !== filenameId) {
    throw new Error(
      `curriculum membership owner '${filename}' carries lesson id '${String(owner.id)}', ` +
        `expected '${filenameId}'`,
    );
  }
  if (typeof owner.pathSegment !== "string" || owner.pathSegment.length === 0) {
    throw new Error(
      `curriculum membership owner '${filename}'.pathSegment must be a non-empty string`,
    );
  }
  if (!Number.isInteger(owner.pathOrder) || (owner.pathOrder as number) < 0) {
    throw new Error(
      `curriculum membership owner '${filename}'.pathOrder must be a non-negative integer`,
    );
  }
  if (!Array.isArray(owner.extensions)) {
    throw new Error(`curriculum membership owner '${filename}'.extensions must be an array`);
  }
  return {
    id: filenameId,
    pathSegment: owner.pathSegment,
    pathOrder: owner.pathOrder as number,
    extensions: owner.extensions.map((entry, index) => parseExtension(entry, filename, index)),
  };
}

/** Lesson filenames are the independent completeness source for direct owners. */
export function curriculumLessonIdentitySources(
  root: string,
  language: string,
): { ids: string[]; sourcePaths: string[] } {
  if (!SAFE_LANGUAGE.test(language)) {
    throw new Error(`curriculum membership language '${language}' is unsafe`);
  }
  const directory = join(root, language, "lessons");
  const stat = statIfPresent(directory);
  if (stat === undefined || stat.isSymbolicLink() || !stat.isDirectory()) {
    throw new Error(`curriculum membership lesson directory '${directory}' must be a real directory`);
  }
  assertRealDescendantComponents(root, directory);
  const entries = readdirSync(directory, { withFileTypes: true })
    .filter((entry) => entry.name.endsWith(".md"))
    .sort((left, right) => left.name.localeCompare(right.name));
  const ids: string[] = [];
  const sourcePaths: string[] = [];
  for (const entry of entries) {
    if (entry.isSymbolicLink() || !entry.isFile()) {
      throw new Error(`curriculum lesson identity '${entry.name}' must be a real regular file`);
    }
    const id = safeLessonId(entry.name.slice(0, -".md".length), "lesson id");
    ids.push(id);
    sourcePaths.push(join(directory, entry.name));
  }
  return { ids: orderedUniqueLessonIds(ids, "lesson ids"), sourcePaths };
}

/** Strictly read one canonical direct owner for every lesson filename. */
export function readCurriculumMembershipOwners(
  root: string,
  language: string,
  options: { requireCanonicalBytes?: boolean } = {},
): LoadedCurriculumMembershipOwners {
  const identities = curriculumLessonIdentitySources(root, language);
  const directory = join(root, language, CURRICULUM_MEMBERSHIP_DIRECTORY);
  const stat = statIfPresent(directory);
  if (stat === undefined || stat.isSymbolicLink() || !stat.isDirectory()) {
    throw new Error(`curriculum membership owner directory '${directory}' must be a real directory`);
  }
  assertRealDescendantComponents(root, directory);

  const entries = readdirSync(directory, { withFileTypes: true }).sort((left, right) =>
    left.name.localeCompare(right.name),
  );
  const names: string[] = [];
  const folded = new Map<string, string>();
  for (const entry of entries) {
    if (entry.isSymbolicLink() || !entry.isFile()) {
      throw new Error(
        `curriculum membership owner '${entry.name}' must be a real direct-child regular file`,
      );
    }
    const lower = entry.name.toLowerCase();
    const prior = folded.get(lower);
    if (prior !== undefined) {
      throw new Error(
        `curriculum membership owners have a case-fold collision: '${prior}'/'${entry.name}'`,
      );
    }
    folded.set(lower, entry.name);
    names.push(entry.name);
  }
  if (!names.includes(CURRICULUM_MEMBERSHIP_META_OWNER)) {
    throw new Error(`curriculum membership owner '${CURRICULUM_MEMBERSHIP_META_OWNER}' is missing`);
  }

  const metaPath = join(directory, CURRICULUM_MEMBERSHIP_META_OWNER);
  const meta = object(readLedgerFile(metaPath), CURRICULUM_MEMBERSHIP_META_OWNER);
  exactKeys(meta, ["version", "language"], CURRICULUM_MEMBERSHIP_META_OWNER);
  if (meta.version !== 1 || meta.language !== language) {
    throw new Error(
      `curriculum membership owner '${CURRICULUM_MEMBERSHIP_META_OWNER}' must declare ` +
        `version 1 and language '${language}'`,
    );
  }
  if (options.requireCanonicalBytes !== false && readFileSync(metaPath, "utf8") !== canonical(meta)) {
    throw new Error(
      `curriculum membership owner '${CURRICULUM_MEMBERSHIP_META_OWNER}' is not canonical`,
    );
  }

  const actualNames = names.filter((name) => name !== CURRICULUM_MEMBERSHIP_META_OWNER);
  const expectedNames = identities.ids.map((id) => `${id}.json`);
  const missing = expectedNames.filter((name) => !actualNames.includes(name));
  const extra = actualNames.filter((name) => !expectedNames.includes(name));
  if (missing.length > 0 || extra.length > 0) {
    throw new Error(
      `curriculum membership owners do not match lesson files` +
        `${missing.length > 0 ? `; missing: ${missing.join(", ")}` : ""}` +
        `${extra.length > 0 ? `; extra: ${extra.join(", ")}` : ""}`,
    );
  }

  const owners = expectedNames.map((name) => {
    const id = basename(name, ".json");
    const path = join(directory, name);
    const raw = readLedgerFile(path);
    const owner = parseOwner(raw, name, id);
    if (options.requireCanonicalBytes !== false && readFileSync(path, "utf8") !== canonical(raw)) {
      throw new Error(`curriculum membership owner '${name}' is not canonical`);
    }
    return owner;
  });
  return {
    owners,
    sourcePaths: [...identities.sourcePaths, metaPath, ...expectedNames.map((name) => join(directory, name))],
  };
}
