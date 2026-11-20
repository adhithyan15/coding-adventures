/**
 * # The conformance kit harness: `warrant-corpus-conformance-v1`, the six `v2-*` categories
 *
 * Every lane runner must pass the shared conformance fixture family.  Version 2
 * adds categories to it (`v2-value`, `v2-arith`, `v2-ir`, `v2-domain`,
 * `v2-chunk`, `v2-fault`, and later `v2-binding`), each case carrying an
 * **envelope**:
 *
 *     { "id": "<category>/<slug>", "summary": "...", "requires": [levels], "expect": {...} }
 *
 * and a lane runs a case **if and only if its `requires` is a subset of the
 * levels the runner implements**, skipping it otherwise.  Per category, cases
 * run plus cases skipped must equal the index row's `cases`, so a case is never
 * skipped silently and a kit that grows by a level does not break a runner that
 * lacks it.  A category this runner does not know (S5d's `v2-binding`, until
 * S7b) is digest-checked against the index and skipped whole.
 *
 * ## What each category runs
 *
 * | category    | the lane runs                                    | module here   |
 * |-------------|--------------------------------------------------|---------------|
 * | `v2-value`  | strict reader, section 2.3 decoder, section 2.4 encoder | `values.ts`   |
 * | `v2-arith`  | one evaluator node over `lit` operands           | `ir.ts`       |
 * | `v2-ir`     | an IR tree with `levels`, `env`, `models`, `ops`, `subjects` | `ir.ts`  |
 * | `v2-domain` | `size()` and `nth()` of a descriptor             | `domains.ts`  |
 * | `v2-chunk`  | validate a chunk file, then enumerate one chunk by unranking | `chunks.ts` |
 * | `v2-fault`  | the pre/post/replace wrapper                     | `faults.ts`   |
 *
 * The codes are the kit's own vocabulary (section 11, R25): each case triggers
 * exactly one error, so a case never depends on which of two errors an
 * implementation meets first.
 *
 * ## "Chunk `c` alone equals sequential" (R21)
 *
 * A property of an implementation, not of the data: a `v2-chunk` case marked
 * `also_sequential` is also walked from chunk 0 up to chunk `c` in sequence, and
 * that walk must give the same `cases`, `calls` and digests -- and a
 * `first_admitted` that equals the running sum -- as the one-chunk walk.
 */

import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { KitError } from "./errors.ts";
import { CorpusError } from "../errors.ts";
import { parseJson, readBytes } from "../jsonio.ts";
import { buildDomain } from "./domains.ts";
import { compileFault } from "./faults.ts";
import { enumerateChunk, enumerateChunks, readChunkFile, readCorpusPart, loadSuite, checkSlimChunks } from "./chunks.ts";
import { IMPLEMENTED_LEVELS } from "./levels.ts";
import { type Param, Program, readOp, typeText } from "./ir.ts";
import { EMPTY_TABLE, type Ty, checkNames, eraseNamedWidths, parseType, readTypeTable } from "./types.ts";
import { type Value, decodeJson, encodeJson, fromHex, streamBytes, toHex } from "./values.ts";

type Json = unknown;
type Case = Record<string, Json>;

/** The categories this runner knows how to run. */
export const KNOWN_CATEGORIES = ["v2-value", "v2-arith", "v2-ir", "v2-domain", "v2-chunk", "v2-fault"] as const;

/** One failed case: its id and a note that never echoes input. */
export interface Failure {
  readonly id: string;
  readonly note: string;
}

/** The result of one category. */
export interface CategoryReport {
  readonly category: string;
  /** The index row's `cases`. */
  readonly indexed: number;
  readonly run: number;
  readonly skipped: number;
  readonly passed: number;
  readonly failed: Failure[];
  /**
   * Cases of a category this runner does not know whose `requires` the runner
   * *does* implement: cases section 9.1 calls "run" that nothing ran.  A runner
   * writes a manifest only if this is zero (S7b), which is why it is counted.
   */
  readonly unrunnable: number;
}

/** The result of a kit run. */
export interface KitReport {
  readonly categories: CategoryReport[];
  /** Total run, skipped, failed over the v2 categories. */
  readonly run: number;
  readonly skipped: number;
  readonly failed: number;
  /** The sum of the categories' `unrunnable`. */
  readonly unrunnable: number;
}

/** Options of {@link runKit}. */
export interface KitOptions {
  /** The levels the runner implements (default: this lane's, `ir.core`). */
  readonly levels?: readonly string[];
  /** Also walk `also_sequential` chunk cases from chunk 0 (default true; the slow part of the kit). */
  readonly sequential?: boolean;
  /** Run only these categories (the others are counted as skipped); default all. */
  readonly only?: readonly string[];
}

// ---------------------------------------------------------------------------
// JSON helpers
// ---------------------------------------------------------------------------

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Deep equality of two parsed JSON values (objects by key set, arrays by position). */
export function jsonEqual(a: Json, b: Json): boolean {
  if (a === b) return true;
  if (Array.isArray(a)) return Array.isArray(b) && a.length === b.length && a.every((item, i) => jsonEqual(item, b[i]));
  if (isRecord(a)) {
    if (!isRecord(b)) return false;
    const keys = Object.keys(a);
    return keys.length === Object.keys(b).length && keys.every((key) => Object.hasOwn(b, key) && jsonEqual(a[key], b[key]));
  }
  return false;
}

/** Map a thrown refusal to the kit's error object (or rethrow a bug). */
function errorObject(error: unknown): Case {
  if (error instanceof KitError) {
    return error.overflowClass === undefined ? { error: error.code } : { error: error.code, class: error.overflowClass };
  }
  throw error;
}

// ---------------------------------------------------------------------------
// The six categories
// ---------------------------------------------------------------------------

function runValue(c: Case): Case {
  // Order of codes (section 11): the strict reader first, then the type, then the value.
  const input = fromHex(String(c["input_hex"]));
  if (input === undefined) throw new Error("a kit case has bad input_hex");
  let parsed: Json;
  try {
    parsed = parseJson(input);
  } catch (error) {
    if (error instanceof CorpusError) return { outcome: "reject", error: "malformed-json" };
    throw error;
  }
  let ty: Ty;
  let table;
  try {
    table = readTypeTable(c["types"] ?? {}, "type");
    ty = parseType(String(c["type"]));
    checkNames(ty, table, "type");
  } catch (error) {
    if (error instanceof KitError && error.code === "type") return { outcome: "reject", error: "type" };
    throw error;
  }
  try {
    const value = decodeJson(parsed, ty, table);
    const bytes = streamBytes(value, ty);
    const hex = toHex(bytes);
    const out: Case = { outcome: "accept", stream_sha256: createHash("sha256").update(bytes).digest("hex") };
    if (hex.length <= 2048) out["stream_hex"] = hex;
    return out;
  } catch (error) {
    if (error instanceof KitError && error.code === "value") return { outcome: "reject", error: "value" };
    throw error;
  }
}

function runArith(c: Case): Case {
  const ty = String(c["type"]);
  const literal = (value: unknown): unknown => ["lit", { type: ty, value }];
  const operands = c["op"] === "neg" ? [literal(c["lhs"])] : [literal(c["lhs"]), literal(c["rhs"])];
  const program = new Program({ types: {}, ops: {}, models: {}, modelOrder: [], levels: new Set(IMPLEMENTED_LEVELS) });
  try {
    const compiled = program.compileExpr([String(c["op"]), ...operands], [], "pure");
    const value = compiled.run([]);
    return { type: typeText(compiled.ty), value: encodeJson(value, compiled.ty) };
  } catch (error) {
    // The operands are literals, so the overflow carries no class (section 11).
    return error instanceof KitError ? { error: error.code } : errorObject(error);
  }
}

function runIr(c: Case): Case {
  const levels = new Set((c["levels"] as string[]).filter((level) => IMPLEMENTED_LEVELS.includes(level)));
  const subjects = (c["subjects"] ?? {}) as Record<string, string>;
  try {
    const program = new Program({
      types: c["types"],
      ops: c["ops"],
      models: c["models"],
      modelOrder: c["model_order"],
      levels,
      ...(c["max_ir_cost"] === undefined ? {} : { maxIrCost: c["max_ir_cost"] as number }),
      structuralCode: "ill-typed",
    });
    const env = c["env"] as [string, { type: string; value: unknown }][];
    const vars: Param[] = env.map(([name, entry]) => ({ name, ty: parseType(entry.type, { code: "ill-typed" }) }));
    const compiled = program.compileExpr(c["ir"], vars, (c["role"] as "body" | "admit" | "fault" | undefined) ?? "body");
    const values: Value[] = env.map(([, entry], i) => decodeJson(entry.value, (vars[i] as Param).ty, program.table));
    program.state.calls = 0;
    program.state.subject = (op, args) => program.callModel(subjects[op] as string, args);
    const result = compiled.run(values);
    return { calls: program.state.calls, type: typeText(compiled.ty), value: encodeJson(result, compiled.ty) };
  } catch (error) {
    return errorObject(error);
  }
}

function runDomain(c: Case): Case {
  try {
    // The domain checker is lenient about a type's depth: a descriptor that is too deep is a `descriptor` refusal, not a `type` one.
    const ty = parseType(String(c["type"]), { maxDepth: 64, code: "descriptor" });
    const domain = buildDomain(c["descriptor"], ty, EMPTY_TABLE, false);
    let low: number;
    let high: number;
    if (c["index"] !== undefined) {
      low = c["index"] as number;
      high = low + 1;
      if (low >= domain.size) throw new KitError("index", "index is not below size");
    } else {
      [low, high] = c["range"] as [number, number];
      if (low > high || high > domain.size) throw new KitError("index", "the range is outside the domain");
    }
    const values: string[] = [];
    for (let i = low; i < high; i += 1) values.push(toHex(streamBytes(domain.nth(i), ty)));
    return { size: domain.size, values_hex: values };
  } catch (error) {
    return errorObject(error);
  }
}

function runChunk(c: Case, options: { sequential: boolean; levels: ReadonlySet<string> }): Case {
  try {
    const part = readCorpusPart(c["corpus"], options.levels);
    const suite = loadSuite(part, c["suite"]);
    const chunk = c["chunk"];
    if (typeof chunk !== "number" || !Number.isInteger(chunk) || chunk < 0) throw new Error("a kit case has a bad chunk index");
    if (chunk >= suite.chunks) throw new KitError("corpus", "chunk is not below m");
    checkSlimChunks((c["suite"] as Record<string, unknown>)["slim_chunks"], suite.chunks);
    const file = fromHex(String(c["chunk_file_hex"]));
    if (file === undefined) throw new KitError("chunk-file", "the chunk file is not hex");
    const rows = readChunkFile(file, suite);
    const row = rows[chunk];
    if (row === undefined) throw new KitError("chunk-file", "no such line");
    const alone = enumerateChunk(suite, chunk);
    if (alone.cases !== row.cases || alone.calls !== row.calls || alone.inputs !== row.inputs || alone.expected !== row.expected) {
      // The file is internally consistent but disagrees with the enumeration it describes.
      throw new KitError("chunk-file", "the chunk file's row is not what the enumeration gives");
    }
    if (c["also_sequential"] === true && options.sequential) {
      const walked = enumerateChunks(suite, 0, chunk).results;
      const last = walked[chunk];
      const prefix = walked.slice(0, chunk).reduce((sum, r) => sum + r.cases, 0);
      if (last === undefined || last.cases !== alone.cases || last.calls !== alone.calls || last.inputs !== alone.inputs ||
          last.expected !== alone.expected || prefix !== row.firstAdmitted) {
        throw new Error("sequential walk disagrees with the chunk enumerated alone");
      }
    }
    const out: Case = { cases: alone.cases, first_admitted: row.firstAdmitted, calls: alone.calls, inputs_sha256: alone.inputs };
    if (alone.expected !== undefined) out["expected_sha256"] = alone.expected;
    return out;
  } catch (error) {
    return errorObject(error);
  }
}

function runFault(c: Case, levels: ReadonlySet<string>): Case {
  try {
    const fault = c["fault"] as Record<string, unknown>;
    const program = new Program({ types: c["types"], ops: {}, models: {}, modelOrder: [], levels, structuralCode: "corpus" });
    const op = readOp(String(fault["op"]), c["op"]);
    const compiled = compileFault(program, fault, op);
    // The wrapper has no call site and no widths: a named width is left unstated, so args and result decode as plain Bytes.
    const paramTypes = op.params.map((p) => eraseNamedWidths(p.ty));
    const retType = eraseNamedWidths(op.ret);
    const args = (c["args"] as unknown[]).map((raw, i) => decodeJson(raw, paramTypes[i] as Ty, program.table));
    if (compiled.kind === "pre") {
      const rewritten = compiled.rewriteArgs(args);
      return { args: rewritten.map((value, i) => encodeJson(value, paramTypes[i] as Ty)) };
    }
    const result = decodeJson(c["result"], retType, program.table);
    return { result: encodeJson(compiled.rewriteResult(args, result), retType) };
  } catch (error) {
    return errorObject(error);
  }
}

// ---------------------------------------------------------------------------
// The harness
// ---------------------------------------------------------------------------

/** The case envelope's `requires` as a list of strings (anything else is a broken fixture). */
function requiresOf(c: Case): string[] {
  const list = c["requires"];
  if (!Array.isArray(list) || list.some((level) => typeof level !== "string")) throw new Error("a kit case has a bad requires");
  return list as string[];
}

/**
 * Run the v2 categories of the conformance kit in `dir`.
 *
 * The index is held to the family's rules first: every row's digest and count
 * against its file, `total_cases` the sum of the rows.  Then each category this
 * runner knows is run case by case, and an unknown category is skipped whole.
 */
export function runKit(dir: string, options: KitOptions = {}): KitReport {
  const levels = new Set(options.levels ?? IMPLEMENTED_LEVELS);
  const sequential = options.sequential ?? true;
  const index = parseJson(readBytes(join(dir, "index.json"), 1 << 20)) as { files: Case[]; total_cases: number };
  const reports: CategoryReport[] = [];
  let total = 0;
  for (const row of index.files) {
    total += row["cases"] as number;
    const category = String(row["category"]);
    const path = join(dir, String(row["path"]));
    if (!existsSync(path)) throw new Error(`${category}: file missing`);
    const bytes = readBytes(path, 64 * 1024 * 1024);
    if (createHash("sha256").update(bytes).digest("hex") !== row["sha256"]) throw new Error(`${category}: sha256 moved`);
    if (!category.startsWith("v2-")) continue;
    const document = parseJson(bytes) as { cases: Case[] };
    if (document.cases.length !== row["cases"]) throw new Error(`${category}: case count moved`);
    const known = (KNOWN_CATEGORIES as readonly string[]).includes(category) && (options.only === undefined || options.only.includes(category));
    const report: { run: number; skipped: number; passed: number; failed: Failure[]; unrunnable: number } = { run: 0, skipped: 0, passed: 0, failed: [], unrunnable: 0 };
    const isKnownCategory = (KNOWN_CATEGORIES as readonly string[]).includes(category);
    for (const c of document.cases) {
      const wanted = requiresOf(c).every((level) => levels.has(level));
      if (!known || !wanted) {
        report.skipped += 1;
        if (!isKnownCategory && wanted) report.unrunnable += 1;
        continue;
      }
      report.run += 1;
      const id = String(c["id"]);
      let actual: Case;
      try {
        actual = runOne(category, c, { sequential, levels });
      } catch (error) {
        report.failed.push({ id, note: `harness error: ${error instanceof Error ? error.message.slice(0, 120) : "unknown"}` });
        continue;
      }
      if (jsonEqual(actual, c["expect"])) report.passed += 1;
      else report.failed.push({ id, note: `outcome differs (${describe(actual)})` });
    }
    reports.push({ category, indexed: row["cases"] as number, ...report });
  }
  if (index.total_cases !== total) throw new Error("total_cases is not the sum of the rows");
  for (const r of reports) if (r.run + r.skipped !== r.indexed) throw new Error(`${r.category}: run + skipped is not the indexed count`);
  return {
    categories: reports,
    run: reports.reduce((sum, r) => sum + r.run, 0),
    skipped: reports.reduce((sum, r) => sum + r.skipped, 0),
    failed: reports.reduce((sum, r) => sum + r.failed.length, 0),
    unrunnable: reports.reduce((sum, r) => sum + r.unrunnable, 0),
  };
}

function describe(actual: Case): string {
  return typeof actual["error"] === "string" ? `error ${actual["error"]}` : typeof actual["outcome"] === "string" ? String(actual["outcome"]) : "a value";
}

function runOne(category: string, c: Case, context: { sequential: boolean; levels: ReadonlySet<string> }): Case {
  switch (category) {
    case "v2-value":
      return runValue(c);
    case "v2-arith":
      return runArith(c);
    case "v2-ir":
      return runIr(c);
    case "v2-domain":
      return runDomain(c);
    case "v2-chunk":
      return runChunk(c, context);
    default:
      return runFault(c, context.levels);
  }
}

