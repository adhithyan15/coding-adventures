/**
 * graph-diff.ts -- Pure, bounded graph and diff-selection operations
 * ==================================================================
 *
 * The command-line build tool normally discovers packages and asks Git for a
 * diff.  Those are host-authority operations.  This module is the narrow,
 * process-free seam underneath them: callers supply already-materialized
 * values, and the evaluator returns either one complete result or one stable
 * error code.  It never reads a checkout, invokes Git, inspects an environment,
 * launches a process, consults a clock, uses randomness, or accesses a network.
 *
 * The implementation deliberately validates before evaluating.  In
 * particular, glob work is charged for the complete pattern/path cross product
 * before the first match, and a decoded repository-boundary object graph is
 * bounded before canonical JSON can expand aliases.
 */

import { createHash } from "node:crypto";
import { types as utilTypes } from "node:util";
import {
  compilePatterns,
  matchCompiledPath,
  type CompiledPattern,
} from "./glob-match.js";
import {
  casefold,
  fullUppercase,
  nfc,
} from "./tracked-artifact-unicode17.js";

export const MAX_GRAPH_PACKAGES = 4_096;
export const MAX_GRAPH_EDGES = 16_384;
export const MAX_SOURCE_GLOBS = 256;
export const MAX_DIFF_SELECTION_MATCH_WORK = 50_000_000;

const MAX_PATH_SCALARS = 512;
const MAX_PACKAGE_NAME_SCALARS = 240;
const MAX_BOUNDARY_DEPTH = 16;
const MAX_BOUNDARY_NODES = 65_536;
const MAX_BOUNDARY_CONTAINERS = 4_096;
const MAX_BOUNDARY_STRING_BYTES = 16_777_216;
const BOUNDARY_DOMAIN = new TextEncoder().encode(
  "coding-adventures/build-tool-repository-source-input-boundary/v1\0",
);
const BUILD_FRONTS = new Set([
  "BUILD",
  "BUILD_windows",
  "BUILD_mac",
  "BUILD_linux",
  "BUILD_mac_and_linux",
]);
const BOUNDARY_ORIGINS = new Set([
  "c",
  "cpp",
  "csharp",
  "dart",
  "dotnet",
  "elixir",
  "fsharp",
  "go",
  "haskell",
  "java",
  "kotlin",
  "lua",
  "mosaic",
  "ocaml",
  "perl",
  "python",
  "repository",
  "ruby",
  "rust",
  "starlark",
  "swift",
  "twig",
  "typescript",
  "wasm",
]);
const RESERVED_NAMES = new Set([
  "CON",
  "PRN",
  "AUX",
  "NUL",
  "CONIN$",
  "CONOUT$",
  "CLOCK$",
  ...Array.from({ length: 9 }, (_, index) => `COM${index + 1}`),
  ...Array.from({ length: 9 }, (_, index) => `LPT${index + 1}`),
  "COM¹",
  "COM²",
  "COM³",
  "LPT¹",
  "LPT²",
  "LPT³",
]);
const PACKAGE_NAME = /^[a-z0-9][a-z0-9._-]*(\/[a-z0-9][a-z0-9._-]*)+$/;
const DIGEST = /^[0-9a-f]{64}$/;
const SLUG = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

export type Edge = readonly [prerequisite: string, dependent: string];

export type GraphInput = Readonly<{
  packages: readonly string[];
  edges: readonly (readonly string[])[];
}>;

export type GraphResult = Readonly<{
  edges: readonly Edge[];
  levels: readonly (readonly string[])[];
}>;

export type PackageSpec = Readonly<{
  name: string;
  relPath: string;
  sourceMode: string;
  sourceGlobs: readonly string[];
}>;

export type DiffSelectionInput = Readonly<{
  packages: readonly PackageSpec[];
  edges: readonly (readonly string[])[];
  forcedPackages: readonly string[];
  unknownPathPolicy: string;
  changedPaths: readonly string[];
  boundarySha256?: string;
  boundary?: unknown;
}>;

export type DiffSelectionResult = Readonly<{
  changed_packages: readonly string[];
  affected_packages: readonly string[];
  prerequisite_packages: readonly string[];
}>;

export type Evaluation<T> =
  | Readonly<{ ok: true; value: T }>
  | Readonly<{ ok: false; code: string }>;

type ValidatedGraph = Readonly<{
  names: ReadonlySet<string>;
  edges: readonly Edge[];
  dependents: ReadonlyMap<string, readonly string[]>;
  prerequisites: ReadonlyMap<string, readonly string[]>;
}>;

type ValidatedPackage = Readonly<{
  name: string;
  relPath: string;
  sourceMode: "package_prefix" | "strict_globs";
  sourceGlobs: readonly string[];
  compiledGlobs: readonly CompiledPattern[];
}>;

type SnapshottedDiffInput = Readonly<{
  packages: readonly PackageSpec[];
  edges: readonly (readonly string[])[];
  forcedPackages: readonly string[];
  unknownPathPolicy: string;
  changedPaths: readonly string[];
  boundarySha256?: string;
  boundary?: unknown;
}>;

class ContractError extends Error {
  constructor(readonly code: string) {
    super(code);
    this.name = "ContractError";
  }
}

function success<T>(value: T): Evaluation<T> {
  return Object.freeze({ ok: true, value });
}

function failure<T>(code: string): Evaluation<T> {
  return Object.freeze({ ok: false, code });
}

function requireContract(condition: boolean, code: string): asserts condition {
  if (!condition) throw new ContractError(code);
}

/** Validate and evaluate canonical graph edges and prerequisite-first levels. */
export function evaluateGraph(input: GraphInput): Evaluation<GraphResult> {
  try {
    const snapshot = snapshotGraphInput(input);
    const graph = validateGraph(snapshot.packages, snapshot.edges);
    const levels = graphLevels(graph);
    if (levels === null) return failure("GRAPH_CYCLE");
    return success(
      Object.freeze({
        edges: graph.edges,
        levels,
      }),
    );
  } catch (error) {
    return failure(contractCode(error, "GRAPH_INPUT_INVALID"));
  }
}

/**
 * Validate and evaluate a complete diff-selection snapshot.
 *
 * Boundary data is inert decoded input.  Its presence grants no authority to
 * locate the checked fixture or inspect the caller's repository.
 */
export function evaluateDiffSelection(
  input: DiffSelectionInput,
): Evaluation<DiffSelectionResult> {
  try {
    const snapshot = snapshotDiffInput(input);
    requireContract(
      snapshot.packages.length <= MAX_GRAPH_PACKAGES,
      "DIFF_PACKAGE_INVALID",
    );
    const packageNames = snapshot.packages.map((item) => item.name);
    const graph = validateGraph(packageNames, snapshot.edges);
    requireContract(graphLevels(graph) !== null, "DIFF_EDGE_CYCLE");
    const packages = validateDiffInput(snapshot, graph);
    const boundaryConsumers = boundaryReverseIndex(snapshot, packages);
    preflightMatchWork(packages, snapshot.changedPaths);
    return success(selectDiff(snapshot, packages, graph, boundaryConsumers));
  } catch (error) {
    return failure(contractCode(error, "DIFF_INPUT_INVALID"));
  }
}

/** Copy public input through own data descriptors before validation. */
function snapshotGraphInput(input: GraphInput): GraphInput {
  const record = inertRecord(input, "GRAPH_INPUT_INVALID");
  const packages = snapshotArray(
    dataProperty(record, "packages", "GRAPH_INPUT_INVALID"),
    "GRAPH_INPUT_INVALID",
    (value) => value,
  );
  const edges = snapshotArray(
    dataProperty(record, "edges", "GRAPH_INPUT_INVALID"),
    "GRAPH_INPUT_INVALID",
    (value) =>
      snapshotArray(value, "GRAPH_INPUT_INVALID", (part) => part) as readonly string[],
  );
  return Object.freeze({
    packages: packages as readonly string[],
    edges: edges as readonly (readonly string[])[],
  });
}

function snapshotDiffInput(input: DiffSelectionInput): SnapshottedDiffInput {
  const record = inertRecord(input, "DIFF_INPUT_INVALID");
  const packages = snapshotArray(
    dataProperty(record, "packages", "DIFF_INPUT_INVALID"),
    "DIFF_INPUT_INVALID",
    (value) => {
      const packageRecord = inertRecord(value, "DIFF_INPUT_INVALID");
      return Object.freeze({
        name: dataProperty(packageRecord, "name", "DIFF_INPUT_INVALID"),
        relPath: dataProperty(packageRecord, "relPath", "DIFF_INPUT_INVALID"),
        sourceMode: dataProperty(
          packageRecord,
          "sourceMode",
          "DIFF_INPUT_INVALID",
        ),
        sourceGlobs: snapshotArray(
          dataProperty(packageRecord, "sourceGlobs", "DIFF_INPUT_INVALID"),
          "DIFF_INPUT_INVALID",
          (glob) => glob,
        ),
      }) as PackageSpec;
    },
  );
  const edges = snapshotArray(
    dataProperty(record, "edges", "DIFF_INPUT_INVALID"),
    "DIFF_INPUT_INVALID",
    (value) =>
      snapshotArray(value, "DIFF_INPUT_INVALID", (part) => part) as readonly string[],
  );
  const forcedPackages = snapshotArray(
    dataProperty(record, "forcedPackages", "DIFF_INPUT_INVALID"),
    "DIFF_INPUT_INVALID",
    (value) => value,
  );
  const changedPaths = snapshotArray(
    dataProperty(record, "changedPaths", "DIFF_INPUT_INVALID"),
    "DIFF_INPUT_INVALID",
    (value) => value,
  );
  const boundarySha256 = optionalDataProperty(
    record,
    "boundarySha256",
    "DIFF_INPUT_INVALID",
  );
  requireContract(
    !boundarySha256.present || typeof boundarySha256.value === "string",
    "DIFF_INPUT_INVALID",
  );
  const boundary = optionalDataProperty(
    record,
    "boundary",
    "DIFF_INPUT_INVALID",
  );
  return Object.freeze({
    packages: packages as readonly PackageSpec[],
    edges: edges as readonly (readonly string[])[],
    forcedPackages: forcedPackages as readonly string[],
    unknownPathPolicy: dataProperty(
      record,
      "unknownPathPolicy",
      "DIFF_INPUT_INVALID",
    ) as string,
    changedPaths: changedPaths as readonly string[],
    ...(boundarySha256.present
      ? { boundarySha256: boundarySha256.value as string }
      : {}),
    ...(boundary.present ? { boundary: boundary.value } : {}),
  });
}

function inertRecord(value: unknown, code: string): Record<string, unknown> {
  requireContract(
    typeof value === "object" &&
      value !== null &&
      !utilTypes.isProxy(value) &&
      !Array.isArray(value),
    code,
  );
  const prototype = Object.getPrototypeOf(value);
  requireContract(
    prototype === Object.prototype || prototype === null,
    code,
  );
  return value as Record<string, unknown>;
}

function dataProperty(
  record: Record<string, unknown>,
  key: string,
  code: string,
): unknown {
  const descriptor = Object.getOwnPropertyDescriptor(record, key);
  requireContract(
    descriptor !== undefined &&
      descriptor.enumerable === true &&
      "value" in descriptor,
    code,
  );
  return descriptor.value;
}

function optionalDataProperty(
  record: Record<string, unknown>,
  key: string,
  code: string,
): Readonly<{ present: boolean; value?: unknown }> {
  const descriptor = Object.getOwnPropertyDescriptor(record, key);
  if (descriptor === undefined) return Object.freeze({ present: false });
  requireContract(
    descriptor.enumerable === true && "value" in descriptor,
    code,
  );
  return Object.freeze({ present: true, value: descriptor.value });
}

function snapshotArray<T>(
  value: unknown,
  code: string,
  snapshotElement: (value: unknown) => T,
): readonly T[] {
  requireContract(
    typeof value === "object" &&
      value !== null &&
      !utilTypes.isProxy(value) &&
      Array.isArray(value),
    code,
  );
  const keys = Reflect.ownKeys(value);
  requireContract(
    keys.length === value.length + 1 && keys.at(-1) === "length",
    code,
  );
  const result: T[] = [];
  for (let index = 0; index < value.length; index += 1) {
    const descriptor = Object.getOwnPropertyDescriptor(value, String(index));
    requireContract(
      descriptor !== undefined &&
        descriptor.enumerable === true &&
        "value" in descriptor,
      code,
    );
    result.push(snapshotElement(descriptor.value));
  }
  return Object.freeze(result);
}

function contractCode(error: unknown, fallback: string): string {
  return error instanceof ContractError ? error.code : fallback;
}

function validateGraph(packages: unknown, edges: unknown): ValidatedGraph {
  requireContract(Array.isArray(packages), "GRAPH_PACKAGE_LIMIT_EXCEEDED");
  requireContract(
    packages.length <= MAX_GRAPH_PACKAGES,
    "GRAPH_PACKAGE_LIMIT_EXCEEDED",
  );
  requireContract(Array.isArray(edges), "GRAPH_EDGE_LIMIT_EXCEEDED");
  requireContract(edges.length <= MAX_GRAPH_EDGES, "GRAPH_EDGE_LIMIT_EXCEEDED");

  const names = new Set<string>();
  for (const name of packages) {
    requireContract(validPackageName(name), "GRAPH_PACKAGE_INVALID");
    requireContract(!names.has(name), "GRAPH_PACKAGE_DUPLICATE");
    names.add(name);
  }

  const canonicalEdges: Edge[] = [];
  const seenEdges = new Set<string>();
  const dependents = new Map<string, string[]>();
  const prerequisites = new Map<string, string[]>();
  for (const name of names) {
    dependents.set(name, []);
    prerequisites.set(name, []);
  }

  for (const value of edges) {
    requireContract(
      Array.isArray(value) &&
        value.length === 2 &&
        typeof value[0] === "string" &&
        typeof value[1] === "string",
      "GRAPH_EDGE_UNKNOWN",
    );
    const [prerequisite, dependent] = value;
    requireContract(
      names.has(prerequisite) && names.has(dependent),
      "GRAPH_EDGE_UNKNOWN",
    );
    requireContract(prerequisite !== dependent, "GRAPH_EDGE_SELF");
    const identity = `${prerequisite}\0${dependent}`;
    requireContract(!seenEdges.has(identity), "GRAPH_EDGE_DUPLICATE");
    seenEdges.add(identity);
    canonicalEdges.push(Object.freeze([prerequisite, dependent]));
    dependents.get(prerequisite)!.push(dependent);
    prerequisites.get(dependent)!.push(prerequisite);
  }

  canonicalEdges.sort(compareEdges);
  for (const values of dependents.values()) values.sort(compareOrdinal);
  for (const values of prerequisites.values()) values.sort(compareOrdinal);
  return Object.freeze({
    names,
    edges: Object.freeze(canonicalEdges),
    dependents,
    prerequisites,
  });
}

function graphLevels(graph: ValidatedGraph): readonly (readonly string[])[] | null {
  const indegree = new Map<string, number>();
  for (const name of graph.names) {
    indegree.set(name, graph.prerequisites.get(name)!.length);
  }
  let ready = [...graph.names]
    .filter((name) => indegree.get(name) === 0)
    .sort(compareOrdinal);
  const levels: (readonly string[])[] = [];
  let visited = 0;
  while (ready.length > 0) {
    const level = Object.freeze([...ready]);
    levels.push(level);
    visited += level.length;
    const next: string[] = [];
    for (const name of level) {
      for (const dependent of graph.dependents.get(name)!) {
        const degree = indegree.get(dependent)! - 1;
        indegree.set(dependent, degree);
        if (degree === 0) next.push(dependent);
      }
    }
    ready = next.sort(compareOrdinal);
  }
  return visited === graph.names.size ? Object.freeze(levels) : null;
}

function validateDiffInput(
  input: DiffSelectionInput,
  graph: ValidatedGraph,
): readonly ValidatedPackage[] {
  const packages: ValidatedPackage[] = [];
  const names = new Set<string>();
  for (const value of input.packages) {
    requireContract(isRecord(value), "DIFF_PACKAGE_INVALID");
    requireContract(validPackageName(value.name), "DIFF_PACKAGE_INVALID");
    requireContract(validPortablePath(value.relPath), "DIFF_PATH_INVALID");
    requireContract(
      value.sourceMode === "package_prefix" || value.sourceMode === "strict_globs",
      "DIFF_SOURCE_MODE_INVALID",
    );
    requireContract(
      Array.isArray(value.sourceGlobs) &&
        value.sourceGlobs.length <= MAX_SOURCE_GLOBS,
      "DIFF_GLOB_INVALID",
    );
    const sourceGlobs = [...value.sourceGlobs];
    requireContract(
      sourceGlobs.every(validPortableGlob) &&
        new Set(sourceGlobs).size === sourceGlobs.length,
      "DIFF_GLOB_INVALID",
    );
    requireContract(
      value.sourceMode === "strict_globs" || sourceGlobs.length === 0,
      "DIFF_GLOB_INVALID",
    );
    let compiledGlobs: readonly CompiledPattern[];
    try {
      compiledGlobs = compilePatterns(sourceGlobs);
    } catch {
      throw new ContractError("DIFF_GLOB_INVALID");
    }
    requireContract(!names.has(value.name), "DIFF_PACKAGE_DUPLICATE");
    names.add(value.name);
    packages.push(
      Object.freeze({
        name: value.name,
        relPath: value.relPath,
        sourceMode: value.sourceMode,
        sourceGlobs: Object.freeze(sourceGlobs),
        compiledGlobs,
      }),
    );
  }
  requireContract(sameSet(names, graph.names), "DIFF_PACKAGE_INVALID");
  validateRootIdentities(packages);

  requireContract(
    input.unknownPathPolicy === "all" || input.unknownPathPolicy === "error",
    "DIFF_POLICY_INVALID",
  );
  requireContract(
    Array.isArray(input.forcedPackages) &&
      input.forcedPackages.length <= MAX_GRAPH_PACKAGES &&
      input.forcedPackages.every(validPackageName) &&
      new Set(input.forcedPackages).size === input.forcedPackages.length,
    "DIFF_FORCED_PACKAGE_INVALID",
  );
  requireContract(
    Array.isArray(input.changedPaths) &&
      input.changedPaths.length <= MAX_GRAPH_PACKAGES &&
      input.changedPaths.every(validPortablePath) &&
      new Set(input.changedPaths).size === input.changedPaths.length,
    "DIFF_PATH_INVALID",
  );
  for (const name of input.forcedPackages) {
    requireContract(names.has(name), "DIFF_FORCED_PACKAGE_UNKNOWN");
  }
  return Object.freeze(packages);
}

type RootTrie = { terminal?: true; children: Map<string, RootTrie> };

function validateRootIdentities(packages: readonly ValidatedPackage[]): void {
  const root: RootTrie = { children: new Map() };
  for (const packageValue of packages) {
    const segments = casefold(nfc(packageValue.relPath)).split("/");
    let node = root;
    for (const segment of segments) {
      requireContract(node.terminal !== true, "DIFF_PATH_INVALID");
      let child = node.children.get(segment);
      if (child === undefined) {
        child = { children: new Map() };
        node.children.set(segment, child);
      }
      node = child;
    }
    requireContract(
      node.terminal !== true && node.children.size === 0,
      "DIFF_PATH_INVALID",
    );
    node.terminal = true;
  }
}

function preflightMatchWork(
  packages: readonly ValidatedPackage[],
  paths: readonly string[],
): void {
  let total = 0;
  for (const packageValue of packages) {
    if (packageValue.sourceMode !== "strict_globs") continue;
    const patternFactor = packageValue.sourceGlobs.reduce(
      (sum, pattern) => sum + scalarLength(pattern) + 1,
      0,
    );
    for (const changedPath of paths) {
      if (!inside(changedPath, packageValue.relPath)) continue;
      const relative = relativePath(changedPath, packageValue.relPath);
      if (isBuildFront(relative)) continue;
      const pathFactor = scalarLength(relative) + 1;
      requireContract(
        patternFactor === 0 ||
          patternFactor <=
            Math.floor(
              (MAX_DIFF_SELECTION_MATCH_WORK - total) / pathFactor,
            ),
        "DIFF_MATCH_LIMIT_EXCEEDED",
      );
      total += patternFactor * pathFactor;
    }
  }
}

function selectDiff(
  input: DiffSelectionInput,
  packages: readonly ValidatedPackage[],
  graph: ValidatedGraph,
  boundaryConsumers: ReadonlyMap<string, ReadonlySet<string>>,
): DiffSelectionResult {
  let unknown = false;
  const changed = new Set(input.forcedPackages);
  for (const changedPath of input.changedPaths) {
    const consumers = boundaryConsumers.get(changedPath);
    if (consumers !== undefined) {
      for (const name of consumers) changed.add(name);
    }
    let known = consumers !== undefined && consumers.size > 0;
    for (const packageValue of packages) {
      if (!inside(changedPath, packageValue.relPath)) continue;
      known = true;
      const relative = relativePath(changedPath, packageValue.relPath);
      if (
        packageValue.sourceMode === "package_prefix" ||
        isBuildFront(relative) ||
        packageValue.compiledGlobs.some((pattern) =>
          matchCompiledPath(pattern, relative),
        )
      ) {
        changed.add(packageValue.name);
      }
    }
    if (!known) unknown = true;
  }

  if (unknown && input.unknownPathPolicy === "error") {
    throw new ContractError("DIFF_UNKNOWN_PATH");
  }
  const finalChanged = unknown ? new Set(graph.names) : changed;
  const affected = closure(finalChanged, graph.dependents);
  const prerequisiteClosure = closure(affected, graph.prerequisites);
  for (const name of affected) prerequisiteClosure.delete(name);
  return Object.freeze({
    changed_packages: Object.freeze([...finalChanged].sort(compareOrdinal)),
    affected_packages: Object.freeze([...affected].sort(compareOrdinal)),
    prerequisite_packages: Object.freeze(
      [...prerequisiteClosure].sort(compareOrdinal),
    ),
  });
}

function closure(
  seeds: ReadonlySet<string>,
  adjacency: ReadonlyMap<string, readonly string[]>,
): Set<string> {
  const result = new Set(seeds);
  const pending = [...seeds];
  while (pending.length > 0) {
    const current = pending.pop()!;
    for (const next of adjacency.get(current) ?? []) {
      if (!result.has(next)) {
        result.add(next);
        pending.push(next);
      }
    }
  }
  return result;
}

function boundaryReverseIndex(
  input: DiffSelectionInput,
  packages: readonly ValidatedPackage[],
): ReadonlyMap<string, ReadonlySet<string>> {
  // Snapshot caller properties once.  A structurally compatible JavaScript
  // object can expose getters even though the TypeScript interface is readonly.
  const suppliedDigest = input.boundarySha256;
  const suppliedBoundary = input.boundary;
  const digest = suppliedDigest ?? "";
  if (digest.length === 0) {
    requireContract(
      suppliedBoundary === undefined || suppliedBoundary === null,
      "DIFF_BOUNDARY_DIGEST_MISMATCH",
    );
    return new Map();
  }
  requireContract(DIGEST.test(digest), "DIFF_BOUNDARY_DIGEST_MISMATCH");
  requireContract(isRecord(suppliedBoundary), "DIFF_BOUNDARY_DIGEST_MISMATCH");
  const boundarySnapshot = snapshotBoundaryObjectGraph(suppliedBoundary);
  const encoded = new TextEncoder().encode(canonicalJson(boundarySnapshot));
  const length = new Uint8Array(8);
  new DataView(length.buffer).setBigUint64(0, BigInt(encoded.length));
  const actual = createHash("sha256")
    .update(BOUNDARY_DOMAIN)
    .update(length)
    .update(encoded)
    .digest("hex");
  requireContract(actual === digest, "DIFF_BOUNDARY_DIGEST_MISMATCH");
  const boundary = validateBoundary(boundarySnapshot);

  const result = new Map<string, Set<string>>();
  for (const packageValue of packages) {
    for (const rule of boundary.boundaries) {
      if (!boundaryApplies(rule.applies_to, packageValue.relPath)) continue;
      for (const boundaryInput of rule.inputs) {
        let consumers = result.get(boundaryInput.path);
        if (consumers === undefined) {
          consumers = new Set();
          result.set(boundaryInput.path, consumers);
        }
        consumers.add(packageValue.name);
      }
    }
  }
  return result;
}

type BoundaryInput = Readonly<{
  path: string;
  role: string;
  generated_component?: string;
}>;
type BoundaryApplicability = Readonly<{
  exact_roots: readonly string[];
  descendant_roots: readonly string[];
  excluded_roots: readonly string[];
}>;
type BoundaryRule = Readonly<{
  id: string;
  input_origin: string;
  applies_to: BoundaryApplicability;
  inputs: readonly BoundaryInput[];
  reason: string;
  owner: string;
}>;
type RepositoryBoundary = Readonly<{
  schema_version: 1;
  language_source_input_registry_sha256: string;
  boundaries: readonly BoundaryRule[];
}>;

function validateBoundary(value: Record<string, unknown>): RepositoryBoundary {
  requireContract(
    exactKeys(value, [
      "schema_version",
      "language_source_input_registry_sha256",
      "boundaries",
    ]) &&
      value.schema_version === 1 &&
      typeof value.language_source_input_registry_sha256 === "string" &&
      DIGEST.test(value.language_source_input_registry_sha256) &&
      Array.isArray(value.boundaries) &&
      value.boundaries.length >= 1 &&
      value.boundaries.length <= 256,
    "DIFF_BOUNDARY_INVALID",
  );

  const ids = new Set<string>();
  const boundaries: BoundaryRule[] = [];
  let scopes = 0;
  let authorizations = 0;
  for (const rawRule of value.boundaries) {
    requireContract(isRecord(rawRule), "DIFF_BOUNDARY_INVALID");
    requireContract(
      exactKeys(rawRule, [
        "id",
        "input_origin",
        "applies_to",
        "inputs",
        "reason",
        "owner",
      ]) &&
        validSlug(rawRule.id, 120) &&
        typeof rawRule.input_origin === "string" &&
        BOUNDARY_ORIGINS.has(rawRule.input_origin) &&
        validScalarString(rawRule.reason, 1, 512) &&
        validSlug(rawRule.owner, 120) &&
        isRecord(rawRule.applies_to) &&
        Array.isArray(rawRule.inputs) &&
        rawRule.inputs.length >= 1 &&
        rawRule.inputs.length <= 64 &&
        !ids.has(rawRule.id),
      "DIFF_BOUNDARY_INVALID",
    );
    ids.add(rawRule.id);
    const appliesTo = validateApplicability(rawRule.applies_to);
    const inputs = rawRule.inputs.map(validateBoundaryInput);
    scopes +=
      appliesTo.exact_roots.length + appliesTo.descendant_roots.length;
    authorizations +=
      inputs.length *
      (appliesTo.exact_roots.length + appliesTo.descendant_roots.length);
    requireContract(
      scopes <= 8_192 && authorizations <= 32_768,
      "DIFF_BOUNDARY_INVALID",
    );
    boundaries.push(
      Object.freeze({
        id: rawRule.id,
        input_origin: rawRule.input_origin,
        applies_to: appliesTo,
        inputs: Object.freeze(inputs),
        reason: rawRule.reason,
        owner: rawRule.owner,
      }),
    );
  }
  return Object.freeze({
    schema_version: 1,
    language_source_input_registry_sha256:
      value.language_source_input_registry_sha256,
    boundaries: Object.freeze(boundaries),
  });
}

function validateApplicability(
  value: Record<string, unknown>,
): BoundaryApplicability {
  requireContract(
    exactKeys(value, ["exact_roots", "descendant_roots", "excluded_roots"]),
    "DIFF_BOUNDARY_INVALID",
  );
  const exactRoots = validatePathArray(value.exact_roots, 4_096);
  const descendantRoots = validatePathArray(value.descendant_roots, 64);
  const excludedRoots = validatePathArray(value.excluded_roots, 4_096);
  requireContract(
    exactRoots.length > 0 || descendantRoots.length > 0,
    "DIFF_BOUNDARY_INVALID",
  );
  return Object.freeze({
    exact_roots: exactRoots,
    descendant_roots: descendantRoots,
    excluded_roots: excludedRoots,
  });
}

function validatePathArray(value: unknown, maximum: number): readonly string[] {
  requireContract(
    Array.isArray(value) &&
      value.length <= maximum &&
      value.every(validPortablePath) &&
      new Set(value).size === value.length,
    "DIFF_BOUNDARY_INVALID",
  );
  return Object.freeze([...value]);
}

function validateBoundaryInput(value: unknown): BoundaryInput {
  requireContract(isRecord(value), "DIFF_BOUNDARY_INVALID");
  requireContract(
    typeof value.path === "string" &&
      validPortablePath(value.path) &&
      typeof value.role === "string" &&
      [
        "cross_package_exact",
        "generated_pruning_exception",
        "shared_ancestor",
      ].includes(value.role),
    "DIFF_BOUNDARY_INVALID",
  );
  if (value.role === "generated_pruning_exception") {
    requireContract(
      exactKeys(value, ["path", "role", "generated_component"]) &&
        validGeneratedComponent(value.generated_component),
      "DIFF_BOUNDARY_INVALID",
    );
    return Object.freeze({
      path: value.path,
      role: value.role,
      generated_component: value.generated_component as string,
    });
  }
  requireContract(exactKeys(value, ["path", "role"]), "DIFF_BOUNDARY_INVALID");
  return Object.freeze({ path: value.path, role: value.role });
}

function boundaryApplies(
  applicability: BoundaryApplicability,
  root: string,
): boolean {
  return (
    applicability.exact_roots.includes(root) ||
    (!applicability.excluded_roots.includes(root) &&
      applicability.descendant_roots.some((parent) =>
        root.startsWith(`${parent}/`),
      ))
  );
}

/**
 * Snapshot a decoded JSON-like graph under shallow bounded recursion before
 * canonicalization.
 *
 * A WeakSet rejects cycles and shared containers.  JSON itself cannot express
 * aliases, so accepting one here would let a tiny direct-call DAG expand
 * exponentially when recursively encoded.  Accessor properties and exotic
 * prototypes are also outside the inert decoded-data contract.
 */
function snapshotBoundaryObjectGraph(
  root: Record<string, unknown>,
): Record<string, unknown> {
  const seen = new WeakSet<object>();
  let nodes = 0;
  let containers = 0;
  let stringBytes = 0;
  const encoder = new TextEncoder();

  function snapshot(value: unknown, depth: number): unknown {
    nodes += 1;
    requireContract(
      nodes <= MAX_BOUNDARY_NODES && depth <= MAX_BOUNDARY_DEPTH,
      "DIFF_BOUNDARY_INVALID",
    );
    if (typeof value === "string") {
      requireContract(wellFormed(value), "DIFF_BOUNDARY_INVALID");
      stringBytes += encoder.encode(value).length;
      requireContract(
        stringBytes <= MAX_BOUNDARY_STRING_BYTES,
        "DIFF_BOUNDARY_INVALID",
      );
      return value;
    }
    if (
      value === null ||
      typeof value === "boolean" ||
      (typeof value === "number" && Number.isFinite(value))
    ) {
      return value;
    }
    requireContract(
      typeof value === "object" &&
        value !== null &&
        !utilTypes.isProxy(value),
      "DIFF_BOUNDARY_INVALID",
    );
    requireContract(!seen.has(value), "DIFF_BOUNDARY_INVALID");
    seen.add(value);
    containers += 1;
    requireContract(
      containers <= MAX_BOUNDARY_CONTAINERS,
      "DIFF_BOUNDARY_INVALID",
    );

    if (Array.isArray(value)) {
      const keys = Reflect.ownKeys(value);
      requireContract(
        keys.length === value.length + 1 && keys.at(-1) === "length",
        "DIFF_BOUNDARY_INVALID",
      );
      requireContract(
        value.length <= MAX_BOUNDARY_NODES - nodes,
        "DIFF_BOUNDARY_INVALID",
      );
      if (value.length > 0) {
        requireContract(depth < MAX_BOUNDARY_DEPTH, "DIFF_BOUNDARY_INVALID");
      }
      const result: unknown[] = [];
      for (let index = 0; index < value.length; index += 1) {
        const descriptor = Object.getOwnPropertyDescriptor(value, String(index));
        requireContract(
          descriptor !== undefined &&
            descriptor.enumerable === true &&
            "value" in descriptor,
          "DIFF_BOUNDARY_INVALID",
        );
        result.push(snapshot(descriptor.value, depth + 1));
      }
      return Object.freeze(result);
    }

    const prototype = Object.getPrototypeOf(value);
    requireContract(
      prototype === Object.prototype || prototype === null,
      "DIFF_BOUNDARY_INVALID",
    );
    const keys = Reflect.ownKeys(value);
    requireContract(
      keys.every((key) => typeof key === "string"),
      "DIFF_BOUNDARY_INVALID",
    );
    requireContract(
      keys.length <= MAX_BOUNDARY_NODES - nodes,
      "DIFF_BOUNDARY_INVALID",
    );
    if (keys.length > 0) {
      requireContract(depth < MAX_BOUNDARY_DEPTH, "DIFF_BOUNDARY_INVALID");
    }
    const result = Object.create(null) as Record<string, unknown>;
    for (const key of keys) {
      requireContract(
        typeof key === "string" && wellFormed(key),
        "DIFF_BOUNDARY_INVALID",
      );
      stringBytes += encoder.encode(key).length;
      requireContract(
        stringBytes <= MAX_BOUNDARY_STRING_BYTES,
        "DIFF_BOUNDARY_INVALID",
      );
      const descriptor = Object.getOwnPropertyDescriptor(value, key);
      requireContract(
        descriptor !== undefined &&
          descriptor.enumerable === true &&
          "value" in descriptor,
        "DIFF_BOUNDARY_INVALID",
      );
      Object.defineProperty(result, key, {
        configurable: false,
        enumerable: true,
        writable: false,
        value: snapshot(descriptor.value, depth + 1),
      });
    }
    return Object.freeze(result);
  }

  return snapshot(root, 0) as Record<string, unknown>;
}

function canonicalJson(value: unknown): string {
  if (Array.isArray(value)) {
    return `[${value.map(canonicalJson).join(",")}]`;
  }
  if (isRecord(value)) {
    const encoder = new TextEncoder();
    const keys = Object.keys(value)
      .map((key) => ({ key, bytes: encoder.encode(key) }))
      .sort((left, right) => compareBytes(left.bytes, right.bytes));
    return `{${keys
      .map(({ key }) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`)
      .join(",")}}`;
  }
  const encoded = JSON.stringify(value);
  requireContract(encoded !== undefined, "DIFF_BOUNDARY_DIGEST_MISMATCH");
  return encoded;
}

function validPackageName(value: unknown): value is string {
  return (
    typeof value === "string" &&
    scalarLength(value) <= MAX_PACKAGE_NAME_SCALARS &&
    PACKAGE_NAME.test(value)
  );
}

function validPortablePath(value: unknown): value is string {
  return (
    validPortablePrefix(value, '<>:"|?*') &&
    value.split("/").every(validPathSegment)
  );
}

function validPortableGlob(value: unknown): value is string {
  return (
    validPortablePrefix(value, '<>:"|') &&
    value.split("/").every((segment) => {
      if (
        segment.length === 0 ||
        segment === "." ||
        segment === ".." ||
        segment.endsWith(" ") ||
        segment.endsWith(".")
      ) {
        return false;
      }
      return /[*?[\]{}]/.test(segment) || !reserved(segment);
    })
  );
}

function validPortablePrefix(value: unknown, forbidden: string): value is string {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    wellFormed(value) &&
    scalarLength(value) <= MAX_PATH_SCALARS &&
    nfc(value) === value &&
    !value.startsWith("/") &&
    !/^[A-Za-z]:/.test(value) &&
    !value.includes("\\") &&
    !value.includes("//") &&
    ![...value].some(
      (scalar) => scalar.codePointAt(0)! < 32 || forbidden.includes(scalar),
    )
  );
}

function validPathSegment(segment: string): boolean {
  return (
    segment.length > 0 &&
    segment !== "." &&
    segment !== ".." &&
    !segment.endsWith(" ") &&
    !segment.endsWith(".") &&
    !reserved(segment)
  );
}

function reserved(segment: string): boolean {
  return RESERVED_NAMES.has(fullUppercase(segment.split(".", 1)[0]));
}

function validSlug(value: unknown, maximum: number): value is string {
  return validScalarString(value, 1, maximum) && SLUG.test(value);
}

function validScalarString(
  value: unknown,
  minimum: number,
  maximum: number,
): value is string {
  return (
    typeof value === "string" &&
    wellFormed(value) &&
    scalarLength(value) >= minimum &&
    scalarLength(value) <= maximum
  );
}

function validGeneratedComponent(value: unknown): value is string {
  return (
    validScalarString(value, 1, 128) &&
    !/[\\/<>:"|?*]/.test(value) &&
    [...value].every((scalar) => scalar.codePointAt(0)! >= 32)
  );
}

function scalarLength(value: string): number {
  return Array.from(value).length;
}

function wellFormed(value: string): boolean {
  for (let index = 0; index < value.length; index += 1) {
    const unit = value.charCodeAt(index);
    if (unit >= 0xd800 && unit <= 0xdbff) {
      if (index + 1 >= value.length) return false;
      const next = value.charCodeAt(index + 1);
      if (next < 0xdc00 || next > 0xdfff) return false;
      index += 1;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      return false;
    }
  }
  return true;
}

function inside(candidate: string, root: string): boolean {
  return candidate === root || candidate.startsWith(`${root}/`);
}

function relativePath(candidate: string, root: string): string {
  return candidate === root ? "" : candidate.slice(root.length + 1);
}

function isBuildFront(relative: string): boolean {
  return BUILD_FRONTS.has(relative.split("/").at(-1) ?? "");
}

function sameSet(left: ReadonlySet<string>, right: ReadonlySet<string>): boolean {
  return left.size === right.size && [...left].every((value) => right.has(value));
}

function exactKeys(value: Record<string, unknown>, expected: readonly string[]): boolean {
  const actual = Object.keys(value).sort(compareOrdinal);
  return (
    actual.length === expected.length &&
    [...expected]
      .sort(compareOrdinal)
      .every((key, index) => actual[index] === key)
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function compareEdges(left: Edge, right: Edge): number {
  return compareOrdinal(left[0], right[0]) || compareOrdinal(left[1], right[1]);
}

function compareOrdinal(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

function compareBytes(leftBytes: Uint8Array, rightBytes: Uint8Array): number {
  const common = Math.min(leftBytes.length, rightBytes.length);
  for (let index = 0; index < common; index += 1) {
    if (leftBytes[index] !== rightBytes[index]) {
      return leftBytes[index] - rightBytes[index];
    }
  }
  return leftBytes.length - rightBytes.length;
}
