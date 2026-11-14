/**
 * # The `warrant-lane-evidence-v1` manifest: build, digest, validate
 *
 * A manifest is the lane's *receipt*: what ran, against which corpus, binding
 * and source, and what happened. Every number in a report is meant to be read
 * from it, never typed by hand (VER04i section 0).
 *
 * ## `warrant-src-v1` -- digesting a source tree
 *
 * `source_sha256` must change whenever code that can change the library's
 * behaviour changes, and never because of anything else. The framing is the same
 * one Rust and Python use, so every lane's producers and the checker agree:
 *
 *     "warrant-src-v1\0"
 *     u64-LE file count
 *     for each file, sorted by path bytes:
 *         u64-LE path length,    path bytes   (relative to the source root, "/")
 *         u64-LE content length, content bytes
 *
 * Every field is length-prefixed, so moving bytes between a path and its
 * content, or between two files, changes a length and therefore the digest. For
 * TypeScript the files are `**` `/*.ts` except `*.test.ts` (VER04i 8.1), and a
 * `dist` or `node_modules` directory is skipped as derived or foreign. A
 * symlink, a non-UTF-8 name, an oversized file or an over-deep tree is an error:
 * the digest must cover exactly what the runtime would read, so anything it
 * cannot account for fails closed.
 */

import { readdirSync, readFileSync, statSync, lstatSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import type { Binding } from "./binding.ts";
import type { Corpus } from "./corpus.ts";
import { CorpusError, need } from "./errors.ts";
import { isObject, parseJson, sha256Hex } from "./jsonio.ts";

export const MAX_SOURCE_FILE_BYTES = 8 * 1024 * 1024;
export const MAX_SOURCE_DEPTH = 32;
export const SCHEMA = "warrant-lane-evidence-v1";
export const RUNNER_NAME = "warrant-corpus";
const HEX64 = /^[0-9a-f]{64}$/;
const IDENT = /^[a-z][a-z0-9_]*$/;
const PACKAGE_NAME = /^[a-z0-9][a-z0-9_-]*$/;
const SKIPPED_DIRECTORIES = new Set(["dist", "node_modules"]);

/** A source file: its path relative to the root, and its bytes. */
export type SourceFile = readonly [string, Uint8Array];

function isSource(name: string): boolean {
  return name.endsWith(".ts") && !name.endsWith(".test.ts");
}

/** Compare two strings by their UTF-8 bytes (the order `frameSources` sorts by). */
function byUtf8(left: string, right: string): number {
  return Buffer.compare(Buffer.from(left, "utf-8"), Buffer.from(right, "utf-8"));
}

/** Every production source file under `root` as `[relative path, bytes]`. */
export function readSourceTree(root: string): SourceFile[] {
  const files: SourceFile[] = [];
  const walk = (directory: string, relative: string, depth: number): void => {
    need(depth <= MAX_SOURCE_DEPTH, "source-tree", "source tree is nested too deeply");
    let names: string[];
    try {
      names = readdirSync(directory);
    } catch {
      throw new CorpusError("source-tree", "cannot list the source tree");
    }
    for (const name of names.sort(byUtf8)) {
      need(!name.includes("�"), "source-tree", "non-UTF-8 file name");
      const rel = relative === "" ? name : `${relative}/${name}`;
      const full = join(directory, name);
      const info = lstatSync(full);
      need(!info.isSymbolicLink(), "source-tree", `${rel} is a symlink`);
      if (info.isDirectory()) {
        if (!SKIPPED_DIRECTORIES.has(name)) walk(full, rel, depth + 1);
      } else if (info.isFile() && isSource(name)) {
        need(info.size <= MAX_SOURCE_FILE_BYTES, "source-tree", `${rel} is larger than the limit`);
        files.push([rel, new Uint8Array(readFileSync(full))]);
      }
    }
  };
  walk(root, "", 0);
  return files.sort((a, b) => byUtf8(a[0], b[0]));
}

function u64le(number: number): Uint8Array {
  const out = new Uint8Array(8);
  new DataView(out.buffer).setBigUint64(0, BigInt(number), true);
  return out;
}

/** The `warrant-src-v1` byte framing of `files` (see the module comment). */
export function frameSources(files: readonly SourceFile[]): Uint8Array {
  const ordered = [...files].sort((a, b) => byUtf8(a[0], b[0]));
  const parts: Uint8Array[] = [new TextEncoder().encode("warrant-src-v1\u0000"), u64le(ordered.length)];
  for (const [path, data] of ordered) {
    const encoded = new TextEncoder().encode(path);
    parts.push(u64le(encoded.length), encoded, u64le(data.length), data);
  }
  const out = new Uint8Array(parts.reduce((sum, part) => sum + part.length, 0));
  let at = 0;
  for (const part of parts) {
    out.set(part, at);
    at += part.length;
  }
  return out;
}

/** `source_sha256` of the source tree at `root`. */
export function sourceDigest(root: string): string {
  let isDirectory = false;
  try {
    isDirectory = statSync(root).isDirectory();
  } catch {
    isDirectory = false;
  }
  need(isDirectory, "source-tree", "source root is not a directory");
  return sha256Hex(frameSources(readSourceTree(root)));
}

/** `runner.src_sha256`: the digest of this runner package's own `src` tree. */
export function runnerDigest(): string {
  return sourceDigest(dirname(fileURLToPath(import.meta.url)));
}

/** Assemble the manifest for a clean, passing run. */
export function buildManifest(
  corpus: Corpus,
  binding: Binding,
  corpusRelative: string,
  sourceRoot: string,
  rows: Record<string, unknown>[],
): Record<string, unknown> {
  return {
    schema: SCHEMA,
    lane: binding.lane,
    package: binding.package,
    module: binding.module,
    spec: corpus.spec,
    spec_sha256: corpus.specSha256,
    corpus: corpusRelative,
    corpus_sha256: corpus.corpusSha256,
    cases_sha256: corpus.casesSha256,
    runner: { name: RUNNER_NAME, lane: binding.lane, src_sha256: runnerDigest() },
    binding_sha256: sha256Hex(binding.raw),
    source_sha256: sourceDigest(sourceRoot),
    toolchain: { name: "node", version: process.versions.node },
    evidence_seed: binding.seed.toString(),
    faulted: false,
    tests: [...rows].sort((a, b) => (String(a["id"]) < String(b["id"]) ? -1 : 1)),
  };
}

// ---------------------------------------------------------------------------
// Structural validation (a TypeScript port of WD-004a's closed ct-compare-v0 rules)
// ---------------------------------------------------------------------------
const MANIFEST_KEYS = [
  "schema", "lane", "package", "module", "spec", "spec_sha256", "corpus",
  "corpus_sha256", "cases_sha256", "runner", "binding_sha256", "source_sha256",
  "toolchain", "evidence_seed", "faulted", "tests",
]; // prettier-ignore
const ROW_KEYS: Readonly<Record<string, readonly string[]>> = {
  example: ["id", "layer", "kind", "op", "cases", "verdict"],
  exhaustive: [
    "id", "layer", "kind", "op", "claim", "cases", "discarded", "calls",
    "inputs_sha256", "expected_sha256", "domain", "verdict",
  ],
  negctl: ["id", "layer", "kind", "fault", "op", "target", "caught_by", "verdict"],
}; // prettier-ignore

function obj(value: unknown, keys: readonly string[], where: string): Record<string, unknown> {
  need(
    isObject(value) && Object.keys(value).length === keys.length && keys.every((key) => Object.hasOwn(value, key)),
    "manifest",
    `${where} has unexpected fields`,
  );
  return value;
}

function digest(value: unknown, where: string): void {
  need(typeof value === "string" && HEX64.test(value), "manifest", `${where} must be a lowercase SHA-256`);
}

/**
 * Validate a lane manifest against the corpus it claims to have run.
 *
 * This mirrors `ca-assurance-report`'s `validate_lane_evidence` (WD-004a) and
 * the Python lane's port of it: closed key sets, exact ids and counts, sorted
 * rows, and joins on digests and controls. It checks *structure and joins only*.
 * It does not run anything, and it cannot tell a real run from a hand-written
 * file that happens to be consistent -- that is what regenerating (`check`) is
 * for.
 */
export function validateManifest(
  manifestBytes: Uint8Array,
  corpus: Corpus,
  corpusRelative: string,
  registryBytes: Uint8Array | null = null,
  requireActive = false,
): Record<string, unknown> {
  const root = obj(parseJson(manifestBytes), MANIFEST_KEYS, "manifest");
  need(root["schema"] === SCHEMA, "manifest", "unsupported schema");
  const lane = root["lane"];
  const packageName = root["package"];
  const module = root["module"];
  need(typeof lane === "string" && IDENT.test(lane), "manifest", "bad lane");
  const parts = String(packageName).split("/");
  need(
    parts.length === 2 && parts[0] === lane && PACKAGE_NAME.test(parts[1] as string),
    "manifest",
    "package must be <lane>/<name> and match the lane",
  );
  need(module === corpus.module, "manifest", "module differs from the corpus");
  need(
    root["spec"] === corpus.spec && root["spec_sha256"] === corpus.specSha256,
    "manifest",
    "spec path or digest differs from the corpus",
  );
  need(root["corpus"] === corpusRelative, "manifest", "corpus path differs");
  need(
    root["corpus_sha256"] === corpus.corpusSha256 && root["cases_sha256"] === corpus.casesSha256,
    "manifest",
    "corpus or cases digest differs from the files",
  );
  const runner = obj(root["runner"], ["name", "lane", "src_sha256"], "runner");
  need(runner["name"] === RUNNER_NAME && runner["lane"] === lane, "manifest", "bad runner");
  digest(runner["src_sha256"], "runner.src_sha256");
  digest(root["binding_sha256"], "binding_sha256");
  digest(root["source_sha256"], "source_sha256");
  const toolchain = obj(root["toolchain"], ["name", "version"], "toolchain");
  need(typeof toolchain["name"] === "string" && typeof toolchain["version"] === "string", "manifest", "bad toolchain");
  const seed = root["evidence_seed"];
  need(
    typeof seed === "string" && /^(?:0|[1-9][0-9]*)$/.test(seed) && BigInt(seed) < 1n << 64n,
    "manifest",
    "evidence_seed must be a canonical u64 decimal string",
  );
  need(root["faulted"] === false, "manifest", "faulted manifests are not accepted");

  const tests = root["tests"];
  const expectedRows = corpus.examples.length + corpus.suites.length + corpus.controls.length;
  need(Array.isArray(tests) && tests.length === expectedRows, "manifest", "wrong number of rows");
  const rows = tests as unknown[];
  const ids = rows.map((row) => (isObject(row) ? row["id"] : undefined));
  need(ids.every((id) => typeof id === "string"), "manifest", "row without an id");
  need(new Set(ids).size === ids.length, "manifest", "duplicate test id");
  need(
    ids.every((id, index) => index === 0 || (ids[index - 1] as string) < (id as string)),
    "manifest",
    "rows must be sorted by id",
  );
  joinRows(new Map(rows.map((row) => [(row as Record<string, unknown>)["id"] as string, row])), corpus);

  if (registryBytes !== null) {
    joinRegistry(registryBytes, lane as string, String(packageName), String(module), requireActive);
  }
  return root;
}

/** Every corpus example, suite and control appears once, with matching data. */
function joinRows(rows: ReadonlyMap<string, unknown>, corpus: Corpus): void {
  const expectedIds = new Set([
    ...corpus.examples.map((e) => e.id),
    ...corpus.suites.map((s) => s.id),
    ...corpus.controls.map((c) => c.id),
  ]);
  need(rows.size === expectedIds.size && [...rows.keys()].every((id) => expectedIds.has(id)), "manifest", "row ids differ from the corpus");
  for (const example of corpus.examples) {
    const row = obj(rows.get(example.id), ROW_KEYS["example"] as string[], "example row");
    agrees(row, "example", { op: example.op, cases: 1 });
  }
  for (const suite of corpus.suites) {
    const row = obj(rows.get(suite.id), ROW_KEYS["exhaustive"] as string[], "exhaustive row");
    need(suite.ops.length === 1, "manifest", "profile allows one subject op per suite");
    agrees(row, "exhaustive", {
      op: suite.ops[0],
      claim: suite.claim,
      cases: suite.cases,
      discarded: suite.discarded,
      calls: suite.expectedCalls,
      domain: suite.domainText,
      inputs_sha256: suite.inputsSha256,
      expected_sha256: suite.expectedSha256,
    });
  }
  for (const control of corpus.controls) {
    const row = obj(rows.get(control.id), ROW_KEYS["negctl"] as string[], "negctl row");
    const caught = row["caught_by"];
    need(
      Array.isArray(caught) &&
        caught.length > 0 &&
        new Set(caught).size === caught.length &&
        caught.every((claim) => control.requiredClaims.includes(claim as string)),
      "manifest",
      "caught_by must be a non-empty subset of the required claims",
    );
    agrees(row, "negctl", { fault: control.fault, op: control.op, target: control.targetKind });
  }
}

/** A row is a portable, passing `kind` row whose fields equal `expected`. */
function agrees(row: Record<string, unknown>, kind: string, expected: Record<string, unknown>): void {
  const all = { kind, layer: "portable", verdict: "pass", ...expected };
  need(
    Object.entries(all).every(([key, value]) => row[key] === value),
    "manifest",
    `${kind} row ${String(row["id"])} disagrees with the corpus`,
  );
}

function joinRegistry(
  registryBytes: Uint8Array,
  lane: string,
  packageName: string,
  module: string,
  requireActive: boolean,
): void {
  const registry = parseJson(registryBytes);
  need(isObject(registry) && Array.isArray(registry["consumers"]), "registry", "bad registry");
  const rows = (registry["consumers"] as unknown[]).filter(
    (row) => isObject(row) && row["lane"] === lane && row["package"] === packageName,
  ) as Record<string, unknown>[];
  need(rows.length === 1, "registry", "lane/package is not registered exactly once");
  const [row] = rows as [Record<string, unknown>];
  need(row["module"] === module, "registry", "module differs from the registered consumer");
  need(!requireActive || row["status"] === "active", "registry", "the registered consumer is not active");
}
