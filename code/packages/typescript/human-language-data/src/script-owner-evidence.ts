import { createHash } from "node:crypto";
import {
  lstatSync,
  readFileSync,
  readdirSync,
  realpathSync,
  writeFileSync,
} from "node:fs";
import { join, relative, resolve, sep } from "node:path";
import { isAbsentErrno, readLedgerFile } from "./shard.js";
import { scriptEntryId } from "./script-shards.js";

export const SCRIPT_OWNER_EVIDENCE_DIRECTORY = "data/script-owner-evidence";

const SAFE_SLUG = /^[a-z][a-z0-9-]*$/;
const WINDOWS_RESERVED = /^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])$/;
const DANGEROUS_IDENTITIES = new Set(["__proto__", "constructor", "prototype"]);
const IDENTITY = "U-[0-9A-F]{1,6}(?:-U-[0-9A-F]{1,6})*";
const EVIDENCE_NAME = new RegExp(`^(${IDENTITY})\\.json$`);
const OWNER_NAME = new RegExp(`^\\d{4}-(${IDENTITY})\\.json$`);
const SHA256 = /^[0-9a-f]{64}$/;

export type ScriptOwnerEvidenceKind = "letter" | "mark";

export interface ScriptOwnerEvidenceOptions {
  readonly language: string;
  readonly script: string;
  readonly requireCanonicalBytes?: boolean;
}

export const SCRIPT_OWNER_EVIDENCE_CONFIGS = [
  { language: "japanese", script: "japanese" },
  { language: "persian", script: "perso-arabic" },
  { language: "tamil", script: "tamil" },
  { language: "urdu", script: "urdu-nastaliq" },
] as const;

export function scriptOwnerEvidenceRelativePath(
  script: string,
  kind: ScriptOwnerEvidenceKind,
  value: string,
): string {
  if (!safeSlug(script)) throw new Error(`script owner evidence script '${script}' is unsafe or reserved`);
  return `${SCRIPT_OWNER_EVIDENCE_DIRECTORY}/${script}/${sectionName(kind)}/${scriptEntryId(value)}.json`;
}

interface Owner {
  readonly identity: string;
  readonly value: string;
  readonly sha256: string;
}

function canonical(value: unknown): string {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function safeSlug(value: string): boolean {
  return SAFE_SLUG.test(value) && !WINDOWS_RESERVED.test(value) &&
    !DANGEROUS_IDENTITIES.has(value);
}

function statIfPresent(path: string): ReturnType<typeof lstatSync> | undefined {
  try {
    return lstatSync(path);
  } catch (cause) {
    const code = (cause as NodeJS.ErrnoException).code;
    if (isAbsentErrno(code)) return undefined;
    throw new Error(
      `script owner evidence '${path}': cannot be inspected (${code ?? "unknown error"})`,
      { cause },
    );
  }
}

function assertRealDescendantComponents(root: string, target: string): void {
  const absoluteRoot = resolve(root);
  const absoluteTarget = resolve(target);
  const route = relative(absoluteRoot, absoluteTarget);
  if (route === ".." || route.startsWith(`..${sep}`) ||
      resolve(absoluteRoot, route) !== absoluteTarget) {
    throw new Error(`script owner evidence path '${target}' is outside '${root}'`);
  }
  const realRoot = realpathSync(absoluteRoot);
  let current = absoluteRoot;
  for (const component of route.split(sep).filter(Boolean)) {
    current = join(current, component);
    const stat = lstatSync(current);
    if (stat.isSymbolicLink()) {
      throw new Error(`script owner evidence path component '${current}' must not be a symbolic link`);
    }
    const real = realpathSync(current);
    if (real !== realRoot && !real.startsWith(realRoot + sep)) {
      throw new Error(`script owner evidence path component '${current}' resolves outside '${root}'`);
    }
  }
}

function object(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`script owner evidence '${label}' must contain one JSON object`);
  }
  return value as Record<string, unknown>;
}

function exactKeys(value: Record<string, unknown>, expected: readonly string[], label: string): void {
  const actual = Object.keys(value).sort();
  const wanted = [...expected].sort();
  if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) {
    throw new Error(`script owner evidence '${label}' must contain exactly: ${wanted.join(", ")}`);
  }
}

function assertNoCaseFoldCollisions(names: readonly string[], label: string): void {
  const seen = new Map<string, string>();
  for (const name of names) {
    const folded = name.toLowerCase();
    const prior = seen.get(folded);
    if (prior !== undefined) {
      throw new Error(prior === name
        ? `${label} repeats '${name}'`
        : `${label} has a case-fold collision between '${prior}' and '${name}'`);
    }
    seen.set(folded, name);
  }
}

function sectionName(kind: ScriptOwnerEvidenceKind): "letters" | "marks" {
  return kind === "letter" ? "letters" : "marks";
}

function identityField(kind: ScriptOwnerEvidenceKind): "glyph" | "mark" {
  return kind === "letter" ? "glyph" : "mark";
}

function listRealFiles(root: string, directory: string, label: string): readonly string[] {
  const stat = statIfPresent(directory);
  if (stat === undefined) throw new Error(`${label} '${directory}' is missing`);
  if (stat.isSymbolicLink() || !stat.isDirectory()) {
    throw new Error(`${label} '${directory}' must be a real directory`);
  }
  assertRealDescendantComponents(root, directory);
  const entries = readdirSync(directory, { withFileTypes: true }).sort((a, b) =>
    a.name < b.name ? -1 : a.name > b.name ? 1 : 0);
  if (entries.length === 0) throw new Error(`${label} '${directory}' must not be empty`);
  assertNoCaseFoldCollisions(entries.map((entry) => entry.name), label);
  for (const entry of entries) {
    const path = join(directory, entry.name);
    const entryStat = lstatSync(path);
    if (entry.isSymbolicLink() || entryStat.isSymbolicLink() || !entryStat.isFile()) {
      throw new Error(`${label} '${path}' must be a real direct-child regular file`);
    }
  }
  return entries.map((entry) => entry.name);
}

function assertEvidenceRoot(root: string, script: string): void {
  const directory = join(root, SCRIPT_OWNER_EVIDENCE_DIRECTORY, script);
  const stat = statIfPresent(directory);
  if (stat === undefined || stat.isSymbolicLink() || !stat.isDirectory()) {
    throw new Error(`script owner evidence directory '${directory}' must be a real directory`);
  }
  assertRealDescendantComponents(root, directory);
  const entries = readdirSync(directory, { withFileTypes: true }).sort((a, b) =>
    a.name < b.name ? -1 : a.name > b.name ? 1 : 0);
  assertNoCaseFoldCollisions(entries.map((entry) => entry.name), `script owner evidence directory '${directory}'`);
  if (entries.length !== 2 || entries[0]?.name !== "letters" || entries[1]?.name !== "marks") {
    throw new Error(`script owner evidence directory '${directory}' must contain exactly: letters, marks`);
  }
  for (const entry of entries) {
    const path = join(directory, entry.name);
    const entryStat = lstatSync(path);
    if (entry.isSymbolicLink() || entryStat.isSymbolicLink() || !entryStat.isDirectory()) {
      throw new Error(`script owner evidence section '${path}' must be a real directory`);
    }
  }
}

function inventoryOwners(root: string, options: ScriptOwnerEvidenceOptions, kind: ScriptOwnerEvidenceKind): readonly Owner[] {
  const section = sectionName(kind);
  const field = identityField(kind);
  const directory = join(root, "data", "scripts", `${options.script}.d`, section);
  return listRealFiles(root, directory, "script inventory owner").map((name) => {
    const match = OWNER_NAME.exec(name);
    if (match === null) throw new Error(`unexpected script inventory owner '${section}/${name}'`);
    const path = join(directory, name);
    const value = object(readLedgerFile<unknown>(path), path);
    const glyph = value[field];
    if (typeof glyph !== "string" || glyph.length === 0) {
      throw new Error(`script inventory owner '${path}'.${field} must be a non-empty string`);
    }
    const identity = scriptEntryId(glyph);
    if (identity !== match[1]) {
      throw new Error(`script inventory owner '${path}' claims '${match[1]}' but its ${field} is '${identity}'`);
    }
    return {
      identity,
      value: glyph,
      sha256: createHash("sha256").update(readFileSync(path)).digest("hex"),
    };
  });
}

function evidenceOwners(root: string, options: ScriptOwnerEvidenceOptions, kind: ScriptOwnerEvidenceKind): readonly Owner[] {
  const section = sectionName(kind);
  const field = identityField(kind);
  const directory = join(root, SCRIPT_OWNER_EVIDENCE_DIRECTORY, options.script, section);
  return listRealFiles(root, directory, "script owner evidence section").map((name) => {
    const match = EVIDENCE_NAME.exec(name);
    if (match === null) throw new Error(`unexpected script owner evidence '${section}/${name}'`);
    const path = join(directory, name);
    const value = object(readLedgerFile<unknown>(path), path);
    exactKeys(value, ["language", "script", "kind", field, "sha256"], path);
    if (value.language !== options.language || value.script !== options.script || value.kind !== kind) {
      throw new Error(`script owner evidence '${path}' has the wrong language, script, or kind`);
    }
    const glyph = value[field];
    if (typeof glyph !== "string" || glyph.length === 0) {
      throw new Error(`script owner evidence '${path}'.${field} must be a non-empty string`);
    }
    const identity = scriptEntryId(glyph);
    if (identity !== match[1]) {
      throw new Error(`script owner evidence '${path}' claims '${match[1]}' but its ${field} is '${identity}'`);
    }
    if (typeof value.sha256 !== "string" || !SHA256.test(value.sha256)) {
      throw new Error(`script owner evidence '${path}'.sha256 must be a lowercase SHA-256 digest`);
    }
    const parsed = {
      language: options.language,
      script: options.script,
      kind,
      [field]: glyph,
      sha256: value.sha256,
    };
    if (options.requireCanonicalBytes !== false && readFileSync(path, "utf8") !== canonical(parsed)) {
      throw new Error(`script owner evidence '${path}' is not canonical`);
    }
    return { identity, value: glyph, sha256: value.sha256 };
  });
}

function assertExactOwners(label: string, inventory: readonly Owner[], evidence: readonly Owner[]): void {
  const inventoryById = new Map(inventory.map((owner) => [owner.identity, owner]));
  const evidenceById = new Map(evidence.map((owner) => [owner.identity, owner]));
  if (inventoryById.size !== inventory.length || evidenceById.size !== evidence.length) {
    throw new Error(`${label} repeats an owner identity`);
  }
  const missing = [...evidenceById.keys()].filter((id) => !inventoryById.has(id)).sort();
  const unexpected = [...inventoryById.keys()].filter((id) => !evidenceById.has(id)).sort();
  if (missing.length > 0 || unexpected.length > 0) {
    throw new Error(`${label} identity set differs: missing [${missing.join(", ")}], unexpected [${unexpected.join(", ")}]`);
  }
  for (const [identity, owner] of inventoryById) {
    const proof = evidenceById.get(identity)!;
    if (proof.value !== owner.value) throw new Error(`${label} '${identity}' embeds a different glyph or mark`);
    if (proof.sha256 !== owner.sha256) throw new Error(`${label} '${identity}' owner bytes differ from its evidence digest`);
  }
}

export function checkScriptOwnerEvidence(root: string, options: ScriptOwnerEvidenceOptions): void {
  if (!safeSlug(options.language) || !safeSlug(options.script)) {
    throw new Error("script owner evidence language and script must be safe slugs");
  }
  assertEvidenceRoot(root, options.script);
  const allInventory: Owner[] = [];
  const allEvidence: Owner[] = [];
  for (const kind of ["letter", "mark"] as const) {
    const inventory = inventoryOwners(root, options, kind);
    const evidence = evidenceOwners(root, options, kind);
    allInventory.push(...inventory);
    allEvidence.push(...evidence);
    assertExactOwners(
      `${options.script} ${sectionName(kind)}`,
      inventory,
      evidence,
    );
  }
  assertNoCaseFoldCollisions(allInventory.map((owner) => owner.identity), `${options.script} inventory owners`);
  assertNoCaseFoldCollisions(allEvidence.map((owner) => owner.identity), `${options.script} evidence owners`);
}

export function writeScriptOwnerEvidence(root: string, options: ScriptOwnerEvidenceOptions): void {
  if (!safeSlug(options.language) || !safeSlug(options.script)) {
    throw new Error("script owner evidence language and script must be safe slugs");
  }
  assertEvidenceRoot(root, options.script);
  for (const kind of ["letter", "mark"] as const) {
    const section = sectionName(kind);
    const field = identityField(kind);
    const directory = join(root, SCRIPT_OWNER_EVIDENCE_DIRECTORY, options.script, section);
    const directoryStat = statIfPresent(directory);
    if (directoryStat === undefined || directoryStat.isSymbolicLink() || !directoryStat.isDirectory()) {
      throw new Error(`script owner evidence section '${directory}' must already be a real directory`);
    }
    assertRealDescendantComponents(root, directory);
    for (const owner of inventoryOwners(root, options, kind)) {
      const path = join(directory, `${owner.identity}.json`);
      const prior = statIfPresent(path);
      if (prior !== undefined && (prior.isSymbolicLink() || !prior.isFile())) {
        throw new Error(`script owner evidence '${path}' must be a real regular file`);
      }
      writeFileSync(path, canonical({
        language: options.language,
        script: options.script,
        kind,
        [field]: owner.value,
        sha256: owner.sha256,
      }), "utf8");
    }
  }
}
