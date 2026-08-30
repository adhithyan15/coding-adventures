import { lstatSync, readFileSync, readdirSync, realpathSync } from "node:fs";
import { join, relative, resolve, sep } from "node:path";
import { isAbsentErrno, readLedgerFile } from "./shard.js";
import { scriptEntryId } from "./script-shards.js";

export const SCRIPT_OWNER_DECLARATION_DIRECTORY =
  "data/script-owner-declarations";

const SAFE_SLUG = /^[a-z][a-z0-9-]*$/;
const DECLARATION_NAME =
  /^U-[0-9A-F]{1,6}(?:-U-[0-9A-F]{1,6})*\.json$/;
const WINDOWS_RESERVED = /^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])$/;
const DANGEROUS_IDENTITIES = new Set(["__proto__", "constructor", "prototype"]);

export type ScriptOwnerKind = "letter" | "mark";

export interface ScriptOwnerDeclarationOptions {
  readonly language: string;
  readonly script: string;
  readonly requireCanonicalBytes?: boolean;
}

export interface ScriptOwnerDeclarationSet {
  readonly language: string;
  readonly script: string;
  readonly letters: readonly string[];
  readonly marks: readonly string[];
}

function canonical(value: unknown): string {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function safeSlug(value: string): boolean {
  return (
    SAFE_SLUG.test(value) &&
    !WINDOWS_RESERVED.test(value) &&
    !DANGEROUS_IDENTITIES.has(value)
  );
}

function statIfPresent(path: string): ReturnType<typeof lstatSync> | undefined {
  try {
    return lstatSync(path);
  } catch (cause) {
    const code = (cause as NodeJS.ErrnoException).code;
    if (isAbsentErrno(code)) return undefined;
    throw new Error(
      `script owner declaration '${path}': cannot be inspected (${code ?? "unknown error"})`,
      { cause },
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
    throw new Error(
      `script owner declaration path '${target}' is outside '${root}'`,
    );
  }
  const realRoot = realpathSync(absoluteRoot);
  let current = absoluteRoot;
  for (const component of route.split(sep).filter(Boolean)) {
    current = join(current, component);
    const stat = lstatSync(current);
    if (stat.isSymbolicLink()) {
      throw new Error(
        `script owner declaration path component '${current}' must not be a symbolic link`,
      );
    }
    const real = realpathSync(current);
    if (real !== realRoot && !real.startsWith(realRoot + sep)) {
      throw new Error(
        `script owner declaration path component '${current}' resolves outside '${root}'`,
      );
    }
  }
}

function object(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(
      `script owner declaration '${label}' must contain one JSON object`,
    );
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
      `script owner declaration '${label}' must contain exactly: ${wanted.join(", ")}`,
    );
  }
}

function assertNoCaseFoldCollisions(
  names: readonly string[],
  label: string,
): void {
  const seen = new Map<string, string>();
  for (const name of names) {
    const folded = name.toLowerCase();
    const prior = seen.get(folded);
    if (prior !== undefined) {
      throw new Error(
        prior === name
          ? `${label} repeats '${name}'`
          : `${label} has a case-fold collision between '${prior}' and '${name}'`,
      );
    }
    seen.set(folded, name);
  }
}

function declarationIdentity(filename: string, label: string): string {
  if (!DECLARATION_NAME.test(filename)) {
    throw new Error(
      `unexpected script owner declaration '${label}'; want ${DECLARATION_NAME.source}`,
    );
  }
  return filename.slice(0, -".json".length);
}

export function scriptOwnerDeclarationRelativePath(
  script: string,
  kind: ScriptOwnerKind,
  value: string,
): string {
  if (!safeSlug(script)) {
    throw new Error(`script owner declaration script '${script}' is unsafe or reserved`);
  }
  const identity = scriptEntryId(value);
  const section = kind === "letter" ? "letters" : "marks";
  return `${SCRIPT_OWNER_DECLARATION_DIRECTORY}/${script}/${section}/${identity}.json`;
}

interface DeclarationFile {
  readonly filename: string;
  readonly identity: string;
  readonly path: string;
}

function listSection(
  root: string,
  directory: string,
  section: "letters" | "marks",
): readonly DeclarationFile[] {
  const sectionPath = join(directory, section);
  const stat = statIfPresent(sectionPath);
  if (stat === undefined) {
    throw new Error(
      `script owner declaration section '${sectionPath}' is missing`,
    );
  }
  if (stat.isSymbolicLink() || !stat.isDirectory()) {
    throw new Error(
      `script owner declaration section '${sectionPath}' must be a real directory`,
    );
  }
  assertRealDescendantComponents(root, sectionPath);

  const entries = readdirSync(sectionPath, { withFileTypes: true }).sort((a, b) =>
    a.name < b.name ? -1 : a.name > b.name ? 1 : 0,
  );
  if (entries.length === 0) {
    throw new Error(
      `script owner declaration section '${sectionPath}' must not be empty`,
    );
  }
  assertNoCaseFoldCollisions(
    entries.map((entry) => entry.name),
    `script owner declaration section '${sectionPath}'`,
  );

  return entries.map((entry) => {
    const path = join(sectionPath, entry.name);
    const entryStat = lstatSync(path);
    if (entry.isSymbolicLink() || entryStat.isSymbolicLink() || !entryStat.isFile()) {
      throw new Error(
        `script owner declaration '${path}' must be a real direct-child regular file`,
      );
    }
    const identity = declarationIdentity(entry.name, `${section}/${entry.name}`);
    return { filename: entry.name, identity, path };
  });
}

function parseDeclaration(
  file: DeclarationFile,
  options: ScriptOwnerDeclarationOptions,
  kind: ScriptOwnerKind,
): string {
  const identityField = kind === "letter" ? "glyph" : "mark";
  const value = object(readLedgerFile<unknown>(file.path), file.path);
  exactKeys(value, ["language", "script", "kind", identityField], file.path);
  if (value.language !== options.language) {
    throw new Error(
      `script owner declaration '${file.path}'.language must be '${options.language}'`,
    );
  }
  if (value.script !== options.script) {
    throw new Error(
      `script owner declaration '${file.path}'.script must be '${options.script}'`,
    );
  }
  if (value.kind !== kind) {
    throw new Error(
      `script owner declaration '${file.path}'.kind must be '${kind}'`,
    );
  }
  const glyph = value[identityField];
  if (typeof glyph !== "string" || glyph.length === 0) {
    throw new Error(
      `script owner declaration '${file.path}'.${identityField} must be a non-empty string`,
    );
  }
  const bodyIdentity = scriptEntryId(glyph);
  if (bodyIdentity !== file.identity) {
    throw new Error(
      `script owner declaration '${file.path}' claims '${file.identity}' but its ` +
        `${identityField} is '${bodyIdentity}'`,
    );
  }
  if (options.requireCanonicalBytes !== false) {
    const parsed = {
      language: options.language,
      script: options.script,
      kind,
      [identityField]: glyph,
    };
    if (readFileSync(file.path, "utf8") !== canonical(parsed)) {
      throw new Error(
        `script owner declaration '${file.path}' is not canonical`,
      );
    }
  }
  return bodyIdentity;
}

/**
 * Read one fixed declaration root without consulting its inventory owners.
 *
 * Names and filesystem types are established for both sections before any
 * declaration body is opened, so a malformed Spanish file cannot hide a
 * missing Tamil filename and a missing owner never becomes an empty dataset.
 */
export function readScriptOwnerDeclarations(
  root: string,
  options: ScriptOwnerDeclarationOptions,
): ScriptOwnerDeclarationSet {
  if (!safeSlug(options.language)) {
    throw new Error(
      `script owner declaration language '${options.language}' is unsafe or reserved`,
    );
  }
  if (!safeSlug(options.script)) {
    throw new Error(
      `script owner declaration script '${options.script}' is unsafe or reserved`,
    );
  }

  const directory = join(
    root,
    SCRIPT_OWNER_DECLARATION_DIRECTORY,
    options.script,
  );
  const stat = statIfPresent(directory);
  if (stat === undefined) {
    throw new Error(
      `script owner declaration directory '${directory}' is missing`,
    );
  }
  if (stat.isSymbolicLink() || !stat.isDirectory()) {
    throw new Error(
      `script owner declaration directory '${directory}' must be a real directory`,
    );
  }
  assertRealDescendantComponents(root, directory);

  const rootEntries = readdirSync(directory, { withFileTypes: true }).sort((a, b) =>
    a.name < b.name ? -1 : a.name > b.name ? 1 : 0,
  );
  assertNoCaseFoldCollisions(
    rootEntries.map((entry) => entry.name),
    `script owner declaration directory '${directory}'`,
  );
  if (
    rootEntries.length !== 2 ||
    rootEntries[0]?.name !== "letters" ||
    rootEntries[1]?.name !== "marks"
  ) {
    throw new Error(
      `script owner declaration directory '${directory}' must contain exactly: letters, marks`,
    );
  }
  for (const entry of rootEntries) {
    const path = join(directory, entry.name);
    const entryStat = lstatSync(path);
    if (
      entry.isSymbolicLink() ||
      entryStat.isSymbolicLink() ||
      !entryStat.isDirectory()
    ) {
      throw new Error(
        `script owner declaration section '${path}' must be a real directory`,
      );
    }
  }

  const letterFiles = listSection(root, directory, "letters");
  const markFiles = listSection(root, directory, "marks");
  assertNoCaseFoldCollisions(
    [...letterFiles, ...markFiles].map((file) => file.identity),
    `script owner declarations for '${options.script}'`,
  );

  return {
    language: options.language,
    script: options.script,
    letters: letterFiles.map((file) =>
      parseDeclaration(file, options, "letter"),
    ),
    marks: markFiles.map((file) => parseDeclaration(file, options, "mark")),
  };
}
