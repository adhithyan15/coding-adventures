import { lstatSync, readFileSync, readdirSync, realpathSync } from "node:fs";
import { basename, dirname, join, relative, resolve, sep } from "node:path";
import type { ExamInventory, ExamPoint } from "./exam-inventory.js";
import { isAbsentErrno, readLedgerFile } from "./shard.js";

export const EXAM_INVENTORY_META_OWNER = "_meta.json";

const SAFE_POINT_ID = /^[A-Z][A-Z0-9-]*$/;
const OWNER_NAME = /^(\d{4})-([A-Z][A-Z0-9-]*)\.json$/;

interface ExamInventoryMeta extends Record<string, unknown> {
  pointIds: string[];
}

export interface ExamInventoryOwnerReadOptions {
  expectedLanguage: string;
  expectedLevel: string;
  rejectMonolith?: boolean;
  requireCanonicalBytes?: boolean;
}

function canonical(value: unknown): string {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function object(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`exam-inventory owner '${label}' must contain one JSON object`);
  }
  return value as Record<string, unknown>;
}

function statIfPresent(path: string): ReturnType<typeof lstatSync> | undefined {
  try {
    return lstatSync(path);
  } catch (cause) {
    const code = (cause as NodeJS.ErrnoException).code;
    if (isAbsentErrno(code)) return undefined;
    throw new Error(
      `exam-inventory owner '${path}': cannot be inspected (${code ?? "unknown error"})`,
      { cause },
    );
  }
}

function ownerDirectoryFor(aggregate: string): string {
  if (!aggregate.endsWith(".json")) {
    throw new Error(`exam-inventory aggregate '${aggregate}' must end in .json`);
  }
  return aggregate.slice(0, -".json".length) + ".d";
}

function assertRealDescendantComponents(root: string, target: string): void {
  const absoluteRoot = resolve(root);
  const absoluteTarget = resolve(target);
  const route = relative(absoluteRoot, absoluteTarget);
  if (route === ".." || route.startsWith(`..${sep}`) || resolve(absoluteRoot, route) !== absoluteTarget) {
    throw new Error(`exam-inventory owner path '${target}' is outside '${root}'`);
  }
  const realRoot = realpathSync(absoluteRoot);
  let current = absoluteRoot;
  for (const component of route.split(sep).filter(Boolean)) {
    current = join(current, component);
    const stat = lstatSync(current);
    if (stat.isSymbolicLink()) {
      throw new Error(`exam-inventory owner path component '${current}' must not be a symbolic link`);
    }
    const real = realpathSync(current);
    if (real !== realRoot && !real.startsWith(realRoot + sep)) {
      throw new Error(`exam-inventory owner path component '${current}' resolves outside '${root}'`);
    }
  }
}

function pointIds(meta: Record<string, unknown>): string[] {
  if (!Array.isArray(meta.pointIds) || meta.pointIds.length === 0) {
    throw new Error(`exam-inventory owner '${EXAM_INVENTORY_META_OWNER}'.pointIds must be non-empty`);
  }
  const ids: string[] = [];
  const exact = new Set<string>();
  const folded = new Map<string, string>();
  for (const value of meta.pointIds) {
    if (typeof value !== "string" || !SAFE_POINT_ID.test(value)) {
      throw new Error(`exam-inventory owner '${EXAM_INVENTORY_META_OWNER}' has unsafe point id '${String(value)}'`);
    }
    if (exact.has(value)) {
      throw new Error(`exam-inventory owner '${EXAM_INVENTORY_META_OWNER}' repeats point id '${value}'`);
    }
    exact.add(value);
    const lower = value.toLowerCase();
    const prior = folded.get(lower);
    if (prior !== undefined) {
      throw new Error(`exam-inventory point ids have a case-fold collision: '${prior}'/'${value}'`);
    }
    folded.set(lower, value);
    ids.push(value);
  }
  return ids;
}

function parsePoint(value: unknown, filename: string, filenameId: string): ExamPoint {
  const point = object(value, filename);
  if (point.id !== filenameId) {
    throw new Error(
      `exam-inventory owner '${filename}' carries point id '${String(point.id)}', expected '${filenameId}'`,
    );
  }
  return point as unknown as ExamPoint;
}

/**
 * Canonical migration bytes: stable metadata plus one self-binding point owner.
 * The private pointIds list is a completeness manifest, not a data aggregate;
 * ordinary probe/note edits touch only their point file.
 */
export function examInventoryOwnerContents(input: ExamInventory): Map<string, string> {
  if (!Array.isArray(input.points) || input.points.length === 0) {
    throw new Error("exam inventory: cannot shard an empty points array");
  }
  const record = input as unknown as Record<string, unknown>;
  const metadata: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(record)) {
    if (key !== "points") metadata[key] = value;
  }

  const ids: string[] = [];
  const exact = new Set<string>();
  const folded = new Map<string, string>();
  for (const point of input.points) {
    if (!SAFE_POINT_ID.test(point.id)) {
      throw new Error(`exam inventory: point id '${point.id}' is unsafe for direct ownership`);
    }
    if (exact.has(point.id)) throw new Error(`exam inventory: duplicate point id '${point.id}'`);
    exact.add(point.id);
    const lower = point.id.toLowerCase();
    const prior = folded.get(lower);
    if (prior !== undefined) {
      throw new Error(`exam inventory: point ids have a case-fold collision: '${prior}'/'${point.id}'`);
    }
    folded.set(lower, point.id);
    ids.push(point.id);
  }
  metadata.pointIds = ids;

  const out = new Map<string, string>();
  out.set(EXAM_INVENTORY_META_OWNER, canonical(metadata));
  input.points.forEach((point, index) => {
    const ordinal = String((index + 1) * 10).padStart(4, "0");
    out.set(`${ordinal}-${point.id}.json`, canonical(point));
  });
  return out;
}

/**
 * Load direct point owners when X.d exists; return null for an unmigrated X.json.
 * A directory, once present, is canonical and forbids aggregate resurrection.
 */
export function readExamInventoryOwnersIfPresent(
  aggregate: string,
  options: ExamInventoryOwnerReadOptions,
): ExamInventory | null {
  const directory = ownerDirectoryFor(aggregate);
  const directoryStat = statIfPresent(directory);
  if (directoryStat === undefined) return null;
  const aggregateStat = statIfPresent(aggregate);
  if (options.rejectMonolith !== false && aggregateStat !== undefined) {
    throw new Error(
      `${basename(aggregate)} is present beside canonical ${basename(directory)}; ` +
        `move its edits into direct point owners and remove the aggregate`,
    );
  }
  if (directoryStat.isSymbolicLink() || !directoryStat.isDirectory()) {
    throw new Error(`exam-inventory owner directory '${directory}' must be a real directory`);
  }
  const root = dirname(dirname(aggregate));
  assertRealDescendantComponents(root, directory);

  const entries = readdirSync(directory, { withFileTypes: true }).sort((a, b) =>
    a.name < b.name ? -1 : a.name > b.name ? 1 : 0,
  );
  const names = entries.map((entry) => entry.name);
  const foldedNames = new Map<string, string>();
  for (const entry of entries) {
    if (entry.isSymbolicLink() || !entry.isFile()) {
      throw new Error(`exam-inventory owner '${entry.name}' must be a real direct-child regular file`);
    }
    const lower = entry.name.toLowerCase();
    const prior = foldedNames.get(lower);
    if (prior !== undefined) {
      throw new Error(`exam-inventory owners have a case-fold collision: '${prior}'/'${entry.name}'`);
    }
    foldedNames.set(lower, entry.name);
  }
  if (!names.includes(EXAM_INVENTORY_META_OWNER)) {
    throw new Error(`exam-inventory owner '${EXAM_INVENTORY_META_OWNER}' is missing`);
  }

  const metaPath = join(directory, EXAM_INVENTORY_META_OWNER);
  const meta = object(readLedgerFile(metaPath), EXAM_INVENTORY_META_OWNER) as ExamInventoryMeta;
  const ids = pointIds(meta);
  if (meta.language !== options.expectedLanguage || meta.level !== options.expectedLevel) {
    throw new Error(
      `exam-inventory owners declare ${String(meta.language)}/${String(meta.level)}, expected ` +
        `${options.expectedLanguage}/${options.expectedLevel}`,
    );
  }
  if (options.requireCanonicalBytes !== false && readFileSync(metaPath, "utf8") !== canonical(meta)) {
    throw new Error(`exam-inventory owner '${EXAM_INVENTORY_META_OWNER}' is not canonical`);
  }

  const pointNames = names.filter((name) => name !== EXAM_INVENTORY_META_OWNER);
  const actualIds: string[] = [];
  const points: ExamPoint[] = [];
  let previousOrdinal = -1;
  for (const name of pointNames) {
    const match = OWNER_NAME.exec(name);
    if (match === null) throw new Error(`unexpected exam-inventory owner '${name}'`);
    const ordinal = Number(match[1]);
    const id = match[2]!;
    if (ordinal <= 0 || ordinal <= previousOrdinal) {
      throw new Error(`exam-inventory owner '${name}' has an invalid or repeated ordinal`);
    }
    previousOrdinal = ordinal;
    actualIds.push(id);
    const path = join(directory, name);
    const point = parsePoint(readLedgerFile(path), name, id);
    if (options.requireCanonicalBytes !== false && readFileSync(path, "utf8") !== canonical(point)) {
      throw new Error(`exam-inventory owner '${name}' is not canonical`);
    }
    points.push(point);
  }

  const missing = ids.filter((id) => !actualIds.includes(id));
  const extra = actualIds.filter((id) => !ids.includes(id));
  if (missing.length > 0 || extra.length > 0) {
    throw new Error(
      `exam-inventory point owners do not match ${EXAM_INVENTORY_META_OWNER}` +
        `${missing.length > 0 ? `; missing: ${missing.join(", ")}` : ""}` +
        `${extra.length > 0 ? `; extra: ${extra.join(", ")}` : ""}`,
    );
  }
  if (actualIds.some((id, index) => id !== ids[index])) {
    throw new Error("exam-inventory point owner order does not match _meta.json.pointIds");
  }

  const document: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(meta)) {
    if (key !== "pointIds") document[key] = value;
  }
  document.points = points;
  return document as unknown as ExamInventory;
}
