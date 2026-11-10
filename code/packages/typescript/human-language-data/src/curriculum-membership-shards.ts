import {
  lstatSync,
  readdirSync,
  realpathSync,
} from "node:fs";
import { spawn } from "node:child_process";
import { basename, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import type {
  CurriculumExtensionMembership,
  CurriculumLessonMembership,
} from "./curriculum-membership.js";
import { isAbsentErrno, parseLedgerText, readLedgerFileWithSource } from "./shard.js";

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
  const metaSource = readLedgerFileWithSource(metaPath);
  const meta = object(metaSource.value, CURRICULUM_MEMBERSHIP_META_OWNER);
  exactKeys(meta, ["version", "language"], CURRICULUM_MEMBERSHIP_META_OWNER);
  if (meta.version !== 1 || meta.language !== language) {
    throw new Error(
      `curriculum membership owner '${CURRICULUM_MEMBERSHIP_META_OWNER}' must declare ` +
        `version 1 and language '${language}'`,
    );
  }
  if (options.requireCanonicalBytes !== false && metaSource.text !== canonical(meta)) {
    throw new Error(
      `curriculum membership owner '${CURRICULUM_MEMBERSHIP_META_OWNER}' is not canonical`,
    );
  }

  const actualNames = names.filter((name) => name !== CURRICULUM_MEMBERSHIP_META_OWNER);
  const expectedNames = identities.ids.map((id) => `${id}.json`);
  const actualNameSet = new Set(actualNames);
  const expectedNameSet = new Set(expectedNames);
  const missing = expectedNames.filter((name) => !actualNameSet.has(name));
  const extra = actualNames.filter((name) => !expectedNameSet.has(name));
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
    const source = readLedgerFileWithSource(path);
    const raw = source.value;
    const owner = parseOwner(raw, name, id);
    if (options.requireCanonicalBytes !== false && source.text !== canonical(raw)) {
      throw new Error(`curriculum membership owner '${name}' is not canonical`);
    }
    return owner;
  });
  return {
    owners,
    sourcePaths: [...identities.sourcePaths, metaPath, ...expectedNames.map((name) => join(directory, name))],
  };
}

/** Read exact direct-owner sets through pinned parents with bounded opens. */
export async function readCurriculumMembershipOwnersBatchAsync(
  root: string,
  languages: readonly string[],
  options: { requireCanonicalBytes?: boolean } = {},
): Promise<Map<string, LoadedCurriculumMembershipOwners>> {
  if (languages.length === 0 || languages.length > 64 ||
      languages.some((language) => !SAFE_LANGUAGE.test(language)) ||
      new Set(languages).size !== languages.length) {
    throw new Error("curriculum membership batch requires distinct safe languages");
  }
  const identities = languages.map((language) => curriculumLessonIdentitySources(root, language));
  const tasks = languages.map((language, index) => ({
    language,
    names: [CURRICULUM_MEMBERSHIP_META_OWNER,
      ...identities[index]!.ids.map((id) => `${id}.json`)],
  }));
  const script = fileURLToPath(new URL(
    "../../../../learning/human-languages/data/scripts/curriculum_membership_owner_read.py",
    import.meta.url,
  ));
  const python = process.platform === "win32" ? "python" : "python3";
  const output = await new Promise<Buffer>((resolve, reject) => {
    const child = spawn(python, [script], { stdio: ["pipe", "pipe", "pipe"], windowsHide: true });
    const chunks: Buffer[] = [];
    let bytes = 0;
    let exceeded = false;
    child.stdout.on("data", (chunk: Buffer) => {
      bytes += chunk.length;
      if (bytes > 128 * 1024 * 1024) {
        exceeded = true;
        child.kill();
      } else {
        chunks.push(chunk);
      }
    });
    // Python exception context may include author-controlled bytes. Never log it.
    child.stderr.resume();
    child.stdin.on("error", () => { /* A failed child is handled by close/error. */ });
    child.on("error", (cause) => reject(new Error("curriculum owner reader could not start", { cause })));
    child.on("close", (code) => {
      if (exceeded || code !== 0) reject(new Error("anchored curriculum owner batch failed"));
      else resolve(Buffer.concat(chunks));
    });
    child.stdin.end(JSON.stringify({ root, tasks }));
  });
  let response: unknown;
  try {
    response = JSON.parse(output.toString("utf8"));
  } catch (cause) {
    throw new Error("anchored curriculum owner batch returned invalid JSON", { cause });
  }
  const results = typeof response === "object" && response !== null && !Array.isArray(response)
    ? (response as Record<string, unknown>).results : undefined;
  if (!Array.isArray(results) || results.length !== tasks.length) {
    throw new Error("anchored curriculum owner batch returned the wrong task count");
  }
  const out = new Map<string, LoadedCurriculumMembershipOwners>();
  for (const [index, task] of tasks.entries()) {
    const files = results[index];
    if (typeof files !== "object" || files === null || Array.isArray(files) ||
        Object.keys(files).length !== task.names.length) {
      throw new Error("anchored curriculum owner batch returned an invalid owner set");
    }
    const texts = files as Record<string, unknown>;
    const directory = join(root, task.language, CURRICULUM_MEMBERSHIP_DIRECTORY);
    const metaPath = join(directory, CURRICULUM_MEMBERSHIP_META_OWNER);
    const metaText = texts[CURRICULUM_MEMBERSHIP_META_OWNER];
    if (typeof metaText !== "string") throw new Error("curriculum membership metadata is missing");
    const meta = object(parseLedgerText(metaText, metaPath), CURRICULUM_MEMBERSHIP_META_OWNER);
    exactKeys(meta, ["version", "language"], CURRICULUM_MEMBERSHIP_META_OWNER);
    if (meta.version !== 1 || meta.language !== task.language) {
      throw new Error(`curriculum membership owner '${CURRICULUM_MEMBERSHIP_META_OWNER}' must declare version 1 and language '${task.language}'`);
    }
    if (options.requireCanonicalBytes !== false && metaText !== canonical(meta)) {
      throw new Error(`curriculum membership owner '${CURRICULUM_MEMBERSHIP_META_OWNER}' is not canonical`);
    }
    const owners = task.names.slice(1).map((name) => {
      const text = texts[name];
      if (typeof text !== "string") throw new Error(`curriculum membership owner '${name}' is missing`);
      const raw = parseLedgerText(text, join(directory, name));
      const owner = parseOwner(raw, name, basename(name, ".json"));
      if (options.requireCanonicalBytes !== false && text !== canonical(raw)) {
        throw new Error(`curriculum membership owner '${name}' is not canonical`);
      }
      return owner;
    });
    out.set(task.language, {
      owners,
      sourcePaths: [...identities[index]!.sourcePaths, metaPath,
        ...task.names.slice(1).map((name) => join(directory, name))],
    });
  }
  return out;
}
