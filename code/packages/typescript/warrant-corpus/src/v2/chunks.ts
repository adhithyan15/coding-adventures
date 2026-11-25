/**
 * # Suites, chunk files and chunk enumeration (VER04r sections 5.2 to 5.6)
 *
 * A suite's product space is cut into **chunks of `chunk_size` product
 * ordinals**.  Chunk `c` covers ordinals `[c*chunk_size, (c+1)*chunk_size)`, and
 * the boundaries are on *product ordinals, not admitted ordinals*, so a lane can
 * unrank `c*chunk_size`, enumerate from there, and never touch an earlier chunk.
 * That is the whole point of sharding: a failure in chunk 200 is reproduced by
 * index, with or without the earlier chunks.
 *
 * ## The product and its unranking
 *
 * Binders `b1..bk` give `P = size(b1) * ... * size(bk)` product ordinals,
 * **row-major, last binder fastest**.  Ordinal `p` unranks by mixed-radix
 * division; walking forward is an odometer (increment the last digit, carry),
 * which is how the hot loop below runs: one `nth` call per *changed* binder,
 * no division, no allocation per case.
 *
 * ## Chunk streams and digests
 *
 * Every chunk has its own pair of SHA-256 streams.  The prefix is the VER04i
 * 2.6 context (header, suite id, binders, result type) followed by the u32-LE
 * chunk index; the body is the typed values of that chunk's admitted cases, in
 * order.  Chunk digests are therefore independent and merge by *listing*:
 *
 *     inputs_sha256 = SHA-256("warrant-chunks-v1\0" || u32-LE m || chunk digests, raw)
 *
 * ## The chunk file (section 5.5)
 *
 * One chunk per line, exactly `m` lines, single spaces, decimal counts, 64-digit
 * lowercase hex, LF:
 *
 *     <c> <cases> <first_admitted> <calls> <inputs_hex> [<expected_hex>]
 *
 * A consumer cross-checks the file against the suite row: the line count, the
 * chunk indices, `first_admitted` as the running prefix sum, `sum(cases)`, the
 * discarded count (`P - sum(cases)`), `chunks_sha256` (the file's own digest) and
 * the two suite digests.  Then it enumerates the one chunk it was asked for,
 * *alone*, and reports `first_admitted` from the validated file (a lane that
 * enumerates one chunk cannot compute it; this is what a slim run does).
 *
 * ## What this module does not check
 *
 * `slim_inputs_sha256` and `slim_expected_sha256` are compared at the end of a
 * *slim run*, not here, and the base set of section 5.6 is recomputed by the
 * oracle, not by a lane (R46).
 */

import { createHash } from "node:crypto";
import { KitError, type KitCode } from "./errors.ts";
import { type Domain, buildDomain } from "./domains.ts";
import { type Compiled, type Param, Program } from "./ir.ts";
import { DEFINED_LEVELS } from "./levels.ts";
import { type Ty, checkNames, flows, parseType, typeLevels } from "./types.ts";
import { type StreamSink, StreamSink as Sink, makeWriter, type Value } from "./values.ts";

/** `MAX_SUITE_PRODUCT`, `MAX_CHUNKS`, `MAX_SLIM_CHUNKS` and the binder bound (section 14, R75). */
export const MAX_SUITE_PRODUCT = 2 ** 28;
export const MAX_CHUNKS = 4096;
export const MAX_SLIM_CHUNKS = 32;
export const MAX_BINDERS = 8;
/** A chunk file is size-capped before parse (section 14). */
export const MAX_CHUNK_FILE_BYTES = 2 * 1024 * 1024;

const HEX64 = /^[0-9a-f]{64}$/;
const COUNT = /^(?:0|[1-9][0-9]{0,14})$/;
const CLAIM = /^[A-Za-z_][A-Za-z0-9_]{0,63}(?:\/[A-Za-z_][A-Za-z0-9_]{0,63})?$/;

function refuse(code: KitCode, why: string): never {
  throw new KitError(code, why);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Deep equality of two parsed JSON trees (objects by key set). */
function sameJson(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (Array.isArray(a)) return Array.isArray(b) && a.length === b.length && a.every((item, i) => sameJson(item, b[i]));
  if (isRecord(a)) {
    if (!isRecord(b)) return false;
    const keys = Object.keys(a);
    return keys.length === Object.keys(b).length && keys.every((key) => Object.hasOwn(b, key) && sameJson(a[key], b[key]));
  }
  return false;
}

/** Two binders may not share a name: the later would shadow the earlier in every expression. */
function distinctBinderNames(rows: readonly unknown[]): void {
  const seen = new Set<string>();
  for (const row of rows) {
    const name = (row as unknown[])[0] as string;
    if (seen.has(name)) refuse("corpus", "a binder name appears twice");
    seen.add(name);
  }
}

function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}

/** The set of keys of `record` must be exactly `names`. */
function exactKeys(record: Record<string, unknown>, names: readonly string[], what: string): void {
  const keys = Object.keys(record);
  if (keys.length !== names.length || !names.every((name) => Object.hasOwn(record, name))) {
    refuse("corpus", `${what} has a missing or unknown key`);
  }
}

// ---------------------------------------------------------------------------
// The corpus part of a v2-chunk case
// ---------------------------------------------------------------------------

/** The parts of a `corpus.json` a suite needs (section 11, `v2-chunk` fields). */
export interface CorpusPart {
  readonly chunkSize: number;
  readonly program: Program;
}

/**
 * Validate the corpus part and build its program.  Every refusal is `corpus`
 * except a missing level, which is the caller's `not-live` ([D64]).
 */
export function readCorpusPart(raw: unknown, levels: ReadonlySet<string>): CorpusPart {
  if (!isRecord(raw)) refuse("corpus", "the corpus part is an object");
  const record = raw as Record<string, unknown>;
  exactKeys(record, ["chunk_size", "max_ir_cost", "types", "ops", "models", "model_order"], "the corpus part");
  const chunkSize = record["chunk_size"];
  if (!isCount(chunkSize) || chunkSize < 1 << 10 || chunkSize > 1 << 16 || (chunkSize & (chunkSize - 1)) !== 0) {
    refuse("corpus", "chunk_size is a power of two in 2^10..=2^16");
  }
  const cost = record["max_ir_cost"];
  if (!isCount(cost)) refuse("corpus", "max_ir_cost is a JSON integer");
  try {
    const program = new Program({
      types: record["types"],
      ops: record["ops"],
      models: record["models"],
      modelOrder: record["model_order"],
      levels,
      maxIrCost: cost as number,
      structuralCode: "corpus",
    });
    return { chunkSize: chunkSize as number, program };
  } catch (error) {
    throw asCorpusError(error);
  }
}

/** Map any refusal of the corpus parts to `corpus`, keeping `level-missing` (a lane records it `not-live`) and `slim-chunks` (the kit's own code for a bad `slim_chunks`). */
export function asCorpusError(error: unknown): unknown {
  if (error instanceof KitError && error.code !== "level-missing" && error.code !== "corpus" && error.code !== "slim-chunks") {
    return new KitError("corpus", error.message.replace(/^[a-z-]+: /, ""));
  }
  return error;
}

// ---------------------------------------------------------------------------
// A suite row
// ---------------------------------------------------------------------------

const ALWAYS = [
  "id", "kind", "claim", "ops", "requires", "binders", "demoted", "domain_label", "admit", "body", "oracle",
  "result_type", "stream", "product", "cases", "discarded", "expected_calls", "wraps", "ir_cost", "chunks",
  "chunks_sha256", "inputs_sha256", "slim_chunks", "slim_inputs_sha256",
]; // prettier-ignore

/** One binder of a suite: its name, type, domain and stream writer. */
export interface Binder {
  readonly name: string;
  readonly ty: Ty;
  readonly typeText: string;
  readonly domain: Domain;
  readonly write: (sink: StreamSink, value: unknown) => void;
}

/** A suite row, validated and compiled. */
export interface Suite {
  readonly id: string;
  readonly claim: string;
  readonly reference: boolean;
  readonly stream: "warrant-stream-v1" | "warrant-stream-v2";
  readonly binders: readonly Binder[];
  readonly resultTypeText: string;
  readonly product: number;
  readonly chunkSize: number;
  readonly chunks: number;
  readonly cases: number;
  readonly discarded: number;
  readonly expectedCalls: number;
  readonly irCost: number;
  readonly slimChunks: readonly number[];
  readonly chunksSha256: string;
  readonly inputsSha256: string;
  readonly expectedSha256: string | undefined;
  readonly v1InputsSha256: string | undefined;
  readonly v1ExpectedSha256: string | undefined;
  readonly admit: Compiled;
  /** Is `admit` the literal `true`?  The enumeration loop then skips evaluating it. */
  readonly admitAlways: boolean;
  readonly body: Compiled;
  readonly expected: Compiled | undefined;
  readonly writeExpected: ((sink: StreamSink, value: unknown) => void) | undefined;
  readonly requires: readonly string[];
  readonly program: Program;
}

/** The v1 type set of section 1.2: a suite whose every type is inside it keeps the v1 header. */
function inV1TypeSet(ty: Ty): boolean {
  switch (ty.k) {
    case "Bool":
    case "Nat":
    case "Bytes":
      return true;
    case "U":
      return ty.w === 8 || ty.w === 64;
    case "List":
    case "Option":
      return ty.of !== null && inV1TypeSet(ty.of);
    case "Result":
      return ty.ok !== null && inV1TypeSet(ty.ok);
    default:
      return false;
  }
}

/** A Windows reserved device name (such a file cannot be created on Windows, section 5.5). */
const DEVICE = /^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])$/i;

/**
 * Section 5.5: a claim any of whose dot-separated file-name segments is a Windows reserved device name (`nul`,
 * `con/post`, `x/com1`) is a `corpus` refusal.  Here and not only in the file-name function, so that a suite row loaded
 * without the corpus reader is refused as well.
 */
export function checkClaimDevice(claim: string): void {
  if (`${claim.replaceAll("/", ".")}.txt`.split(".").some((segment) => DEVICE.test(segment))) {
    throw new KitError("corpus", "a claim is a Windows reserved device name");
  }
}

/**
 * Validate a suite row against the corpus part and compile it.
 *
 * Refusals are `corpus` (a closed key set, a wrong `product`, a wrong descriptor
 * `size`, a row `chunks` that is not `ceil(product / chunk_size)`, more than 8
 * binders, a product over 2^28, an `errs` that differs from the declared cases,
 * an `ir_cost` that is not recomputed, an expression the program refuses), or
 * `level-missing` (the suite reaches a level this lane lacks: not-live, [D64]).
 */
export function loadSuite(part: CorpusPart, raw: unknown): Suite {
  try {
    return loadSuiteUnchecked(part, raw);
  } catch (error) {
    throw asCorpusError(error);
  }
}

function loadSuiteUnchecked(part: CorpusPart, raw: unknown): Suite {
  const { program, chunkSize } = part;
  if (!isRecord(raw)) refuse("corpus", "a suite row is an object");
  const row = raw as Record<string, unknown>;
  const oracle = row["oracle"];
  if (!isRecord(oracle)) refuse("corpus", "oracle is an object");
  const oracleRecord = oracle as Record<string, unknown>;
  const reference = oracleRecord["kind"] === "reference";
  if (!reference && oracleRecord["kind"] !== "predicate") refuse("corpus", "oracle.kind is reference or predicate");
  exactKeys(oracleRecord, reference ? ["kind", "subject", "expected"] : ["kind"], "oracle");
  const stream = row["stream"];
  if (stream !== "warrant-stream-v1" && stream !== "warrant-stream-v2") refuse("corpus", "stream is a warrant-stream header");
  const wanted = [...ALWAYS];
  if (reference) wanted.push("expected_sha256", "slim_expected_sha256");
  if (stream === "warrant-stream-v1") wanted.push("v1_inputs_sha256");
  if (stream === "warrant-stream-v1" && reference) wanted.push("v1_expected_sha256");
  exactKeys(row, wanted, "a suite row");

  const text = (key: string): string => {
    const value = row[key];
    if (typeof value !== "string") return refuse("corpus", `${key} is text`);
    return value;
  };
  const count = (key: string): number => {
    const value = row[key];
    if (!isCount(value)) return refuse("corpus", `${key} is a JSON integer`);
    return value;
  };
  const digest = (key: string): string => {
    const value = text(key);
    if (!HEX64.test(value)) refuse("corpus", `${key} is 64 lowercase hex digits`);
    return value;
  };

  const requiresRaw = row["requires"];
  if (!Array.isArray(requiresRaw) || requiresRaw.some((level) => typeof level !== "string" || !DEFINED_LEVELS.has(level))) {
    refuse("corpus", "requires names a level this document does not define");
  }
  const id = text("id");
  const claim = text("claim");
  if (!CLAIM.test(claim)) refuse("corpus", "claim is one or two names joined by /");
  checkClaimDevice(claim);
  if (row["kind"] !== "exhaustive") refuse("corpus", "kind is exhaustive");

  // Binders: 0..=8, each [name, {type, domain}].
  const bindersRaw = row["binders"];
  if (!Array.isArray(bindersRaw) || bindersRaw.length > MAX_BINDERS) refuse("corpus", "a suite has at most 8 binders");
  const binders: Binder[] = [];
  const levelsNeeded = new Set<string>();
  let product = 1;
  for (const entry of bindersRaw as unknown[]) {
    if (!Array.isArray(entry) || entry.length !== 2 || typeof entry[0] !== "string" || !isRecord(entry[1])) {
      return refuse("corpus", "a binder is [name, {type, domain}]");
    }
    const spec = entry[1] as Record<string, unknown>;
    exactKeys(spec, ["type", "domain"], "a binder");
    if (typeof spec["type"] !== "string") refuse("corpus", "a binder type is text");
    const typeText = spec["type"] as string;
    const ty = parseType(typeText, { code: "corpus" });
    checkNames(ty, program.table, "corpus");
    typeLevels(ty, levelsNeeded);
    // A binder that needs a level this lane lacks is not built (its descriptor is of that level), but the names above
    // were checked and the tag scan below still runs: reach scopes `level-missing` and nothing else ([D64], R69 (c)).
    if (program.lacks(levelsNeeded) !== undefined) continue;
    const domain = buildDomain(spec["domain"], ty, program.table, true);
    product *= domain.size;
    if (product > MAX_SUITE_PRODUCT) refuse("corpus", "the product exceeds 2^28");
    binders.push({ name: entry[0], ty, typeText, domain, write: makeWriter(ty) });
  }
  distinctBinderNames(bindersRaw as unknown[]);

  const resultTypeText = text("result_type");
  const resultTy = parseType(resultTypeText, { code: "corpus" });
  checkNames(resultTy, program.table, "corpus");
  typeLevels(resultTy, levelsNeeded);

  // The checks that read tag names alone run over every expression the suite holds *before* `level-missing` is
  // recorded: an unknown tag, a misplaced `subject_op`, a call out of `model_order` and a tree over 64 deep are corpus
  // errors wherever they stand, in a not-live suite too (section 14; R69 (c) tier 2 and (d)).
  const trees: [unknown, "admit" | "body" | "pure"][] = [[row["admit"], "admit"], [row["body"], "body"]];
  if (reference) trees.push([oracleRecord["expected"], "pure"], [oracleRecord["subject"], "body"]);
  for (const [tree, role] of trees) program.scanTags(tree, role);
  const missing = program.lacks(levelsNeeded);
  if (missing !== undefined) throw new KitError("level-missing", "the suite needs a level this lane lacks", { level: missing });

  if (count("product") !== product) refuse("corpus", "product is not the product of the binder sizes");
  const chunks = Math.ceil(product / chunkSize);
  if (count("chunks") !== chunks || chunks > MAX_CHUNKS) refuse("corpus", "chunks is not ceil(product / chunk_size)");

  const vars: Param[] = binders.map((b) => ({ name: b.name, ty: b.ty }));
  const admit = program.compileExpr(row["admit"], vars, "admit");
  if (admit.ty.k !== "Bool") refuse("corpus", "admit is a Bool");
  const body = program.compileExpr(row["body"], vars, "body");
  if (body.ty.k !== "Bool") refuse("corpus", "body is a Bool");
  let expected: Compiled | undefined;
  let irCost = admit.cost + body.cost;
  if (reference) {
    expected = program.compileExpr(oracleRecord["expected"], vars, "pure");
    if (!flows(expected.ty, resultTy)) refuse("corpus", "expected does not fit result_type");
    irCost += expected.cost;
    // The subject side is a body expression too: it must compile, may call the subject, and has the result type (section 4.3).
    const subject = program.compileExpr(oracleRecord["subject"], vars, "body");
    if (!flows(subject.ty, resultTy)) refuse("corpus", "the oracle's subject does not fit result_type");
    // Section 4.3: a `reference` suite is one whose body *is* `subject == expected`, in either order.
    const sides = Array.isArray(row["body"]) && row["body"].length === 3 && row["body"][0] === "eq" ? [row["body"][1], row["body"][2]] : undefined;
    const agrees =
      sides !== undefined &&
      ((sameJson(sides[0], oracleRecord["subject"]) && sameJson(sides[1], oracleRecord["expected"])) ||
        (sameJson(sides[1], oracleRecord["subject"]) && sameJson(sides[0], oracleRecord["expected"])));
    if (!agrees) refuse("corpus", "a reference suite's body is not its subject compared with its expected");
  } else if (resultTypeText !== "Bool") {
    refuse("corpus", "a predicate suite's result_type is Bool");
  }
  if (count("ir_cost") !== irCost) refuse("corpus", "ir_cost is not the recomputed cost");

  const v1 = [...binders.map((b) => b.ty), resultTy].every(inV1TypeSet);
  if ((stream === "warrant-stream-v1") !== v1) refuse("corpus", "stream does not match the v1 type set rule");

  // Section 14: `slim_chunks` is checked here, on the corpus path as well as the kit's, before anything runs.
  checkSlimChunks(row["slim_chunks"], chunks);
  const slimChunks = row["slim_chunks"] as number[];
  const casesRow = count("cases");
  if (casesRow === 0) refuse("corpus", "a suite admits at least one case");
  return {
    id,
    claim,
    reference,
    stream,
    binders,
    resultTypeText,
    product,
    chunkSize,
    chunks,
    cases: casesRow,
    discarded: count("discarded"),
    expectedCalls: count("expected_calls"),
    irCost,
    slimChunks,
    chunksSha256: digest("chunks_sha256"),
    inputsSha256: digest("inputs_sha256"),
    expectedSha256: reference ? digest("expected_sha256") : undefined,
    v1InputsSha256: stream === "warrant-stream-v1" ? digest("v1_inputs_sha256") : undefined,
    v1ExpectedSha256: stream === "warrant-stream-v1" && reference ? digest("v1_expected_sha256") : undefined,
    admit,
    admitAlways: Array.isArray(row["admit"]) && row["admit"].length === 2 && row["admit"][0] === "lit" && row["admit"][1] === true,
    body,
    expected,
    writeExpected: reference ? makeWriter(resultTy) : undefined,
    requires: requiresRaw as string[],
    program,
  };
}

/**
 * The binder-and-slim checks of section 11's `slim-chunks` code: ascending and
 * duplicate-free, every index below `m`, chunk 0 and chunk `m-1` present, at
 * most 32 entries.
 */
export function checkSlimChunks(raw: unknown, chunks: number): void {
  if (!Array.isArray(raw) || raw.length < 1 || raw.length > MAX_SLIM_CHUNKS) refuse("slim-chunks", "slim_chunks holds 1 to 32 chunk indices");
  let previous = -1;
  for (const item of raw) {
    if (!isCount(item) || item >= chunks || item <= previous) refuse("slim-chunks", "slim_chunks is ascending, duplicate-free and below m");
    previous = item;
  }
  if (raw[0] !== 0 || raw[raw.length - 1] !== chunks - 1) refuse("slim-chunks", "slim_chunks omits the first or the last chunk");
}

// ---------------------------------------------------------------------------
// The chunk file
// ---------------------------------------------------------------------------

/** One line of a chunk file. */
export interface ChunkRow {
  readonly index: number;
  readonly cases: number;
  readonly firstAdmitted: number;
  readonly calls: number;
  readonly inputs: string;
  readonly expected: string | undefined;
}

function sha(...parts: Uint8Array[]): Uint8Array {
  const hash = createHash("sha256");
  for (const part of parts) hash.update(part);
  return hash.digest();
}

function u32(value: number): Uint8Array {
  const out = new Uint8Array(4);
  new DataView(out.buffer).setUint32(0, value, true);
  return out;
}

/** The suite digest over chunk digests: SHA-256("warrant-chunks-v1\0" || u32-LE m || digests). */
export function framedDigest(hexDigests: readonly string[]): string {
  const raw = hexDigests.map((hex) => Buffer.from(hex, "hex"));
  return Buffer.from(sha(new TextEncoder().encode("warrant-chunks-v1\u0000"), u32(hexDigests.length), ...raw)).toString("hex");
}

/**
 * Parse and cross-check a chunk file against its suite (section 5.5).  Every
 * refusal is `chunk-file`: the line grammar, the line count, a chunk index out
 * of sequence, `first_admitted` not the running prefix sum, `sum(cases)`, a row
 * `discarded` that is not `P - sum(cases)`, `chunks_sha256` and the suite
 * digests.  (The slim digests are *not* compared, R46.)
 */
export function readChunkFile(bytes: Uint8Array, suite: Suite): ChunkRow[] {
  const bad = (why: string): never => refuse("chunk-file", why);
  if (bytes.length === 0 || bytes.length > MAX_CHUNK_FILE_BYTES) bad("a chunk file is 1 byte to 2 MiB");
  for (const byte of bytes) if (byte >= 0x80 || byte === 0x0d) bad("a chunk file is ASCII with LF line ends");
  const text = new TextDecoder("ascii").decode(bytes);
  if (!text.endsWith("\n")) bad("a chunk file ends with LF");
  const lines = text.slice(0, -1).split("\n");
  if (lines.length !== suite.chunks) bad("a chunk file has exactly m lines");
  const rows: ChunkRow[] = [];
  let running = 0;
  let casesTotal = 0;
  let callsTotal = 0;
  const want = suite.reference ? 6 : 5;
  lines.forEach((line, i) => {
    const fields = line.split(" ");
    if (fields.length !== want) bad("a chunk line has a wrong number of fields");
    const [c, cases, first, calls, inputs, expected] = fields as [string, string, string, string, string, string | undefined];
    for (const field of [c, cases, first, calls]) if (!COUNT.test(field)) bad("a count is canonical decimal");
    if (!HEX64.test(inputs) || (expected !== undefined && !HEX64.test(expected))) bad("a digest is 64 lowercase hex digits");
    if (Number(c) !== i) bad("a chunk index is out of sequence");
    if (Number(first) !== running) bad("first_admitted is not the running prefix sum");
    running += Number(cases);
    casesTotal += Number(cases);
    callsTotal += Number(calls);
    rows.push({ index: i, cases: Number(cases), firstAdmitted: Number(first), calls: Number(calls), inputs, expected });
  });
  if (casesTotal !== suite.cases) bad("the sum of cases is not the suite's cases");
  if (suite.discarded !== suite.product - casesTotal) bad("discarded is not P - sum(cases)");
  if (callsTotal !== suite.expectedCalls) bad("the sum of calls is not expected_calls");
  if (Buffer.from(sha(bytes)).toString("hex") !== suite.chunksSha256) bad("chunks_sha256 is not the file's digest");
  if (framedDigest(rows.map((r) => r.inputs)) !== suite.inputsSha256) bad("inputs_sha256 is not the digest of the listed chunk digests");
  if (suite.reference && framedDigest(rows.map((r) => r.expected as string)) !== suite.expectedSha256) {
    bad("expected_sha256 is not the digest of the listed chunk digests");
  }
  return rows;
}

// ---------------------------------------------------------------------------
// Enumerating chunks
// ---------------------------------------------------------------------------

/** The result of enumerating one chunk. */
export interface ChunkResult {
  readonly index: number;
  readonly cases: number;
  readonly calls: number;
  readonly inputs: string;
  readonly expected: string | undefined;
}

/** What a walk may additionally record. */
export interface WalkOptions {
  /** Also compute the v1 whole-suite digests over the unchunked stream (a walk from chunk 0 to the last chunk). */
  readonly whole?: boolean;
}

/** The whole-suite v1 digests. */
export interface WholeDigests {
  readonly inputs: string;
  readonly expected: string | undefined;
}

/** The VER04i 2.6 context both streams start with. */
export function streamContext(suite: Suite): Uint8Array {
  const encoder = new TextEncoder();
  const parts: Uint8Array[] = [encoder.encode(`${suite.stream}\u0000`)];
  const field = (value: string): void => {
    const raw = encoder.encode(value);
    parts.push(u32(raw.length), raw);
  };
  field(suite.id);
  parts.push(u32(suite.binders.length));
  for (const binder of suite.binders) field(binder.typeText);
  field(suite.resultTypeText);
  const out = new Uint8Array(parts.reduce((sum, part) => sum + part.length, 0));
  let at = 0;
  for (const part of parts) {
    out.set(part, at);
    at += part.length;
  }
  return out;
}

/**
 * Enumerate chunks `first..=last` **by unranking the first one** and then walking
 * forward as an odometer across chunk boundaries (the hashes restart at each
 * boundary, the values do not).  `enumerateChunk(suite, c)` is the one-chunk
 * walk -- *chunk `c` alone* -- and `enumerateChunks(suite, 0, c)` the sequential
 * one; the two must agree on chunk `c`, and a kit case marked `also_sequential`
 * asserts that they do.
 */
export function enumerateChunks(suite: Suite, first: number, last: number, options: WalkOptions = {}): { results: ChunkResult[]; whole?: WholeDigests } {
  const { binders, product, chunkSize } = suite;
  const count = binders.length;
  const domains = binders.map((b) => b.domain);
  const writers = binders.map((b) => b.write);
  const sizes = domains.map((d) => d.size);

  // Unrank the first product ordinal of chunk `first`: mixed-radix division, last binder fastest.
  const idx = new Array<number>(count).fill(0);
  const env: Value[] = new Array<Value>(count);
  let rest = first * chunkSize;
  for (let k = count - 1; k >= 0; k -= 1) {
    idx[k] = rest % (sizes[k] as number);
    rest = Math.floor(rest / (sizes[k] as number));
  }
  for (let k = 0; k < count; k += 1) env[k] = (domains[k] as Domain).nth(idx[k] as number);

  const admitConstant = suite.admitAlways;
  const admit = suite.admit.run;
  const bodyTrace = suite.body.trace;
  const fixedCalls = "fixed" in bodyTrace ? bodyTrace.fixed : -1;
  const dynamicCalls = "dynamic" in bodyTrace ? bodyTrace.dynamic : undefined;
  const expectedRun = suite.expected?.run;
  const writeExpected = suite.writeExpected;
  const context = streamContext(suite);

  const wholeInputs = options.whole === true ? createHash("sha256") : undefined;
  const wholeExpected = options.whole === true && suite.reference ? createHash("sha256") : undefined;
  wholeInputs?.update(context);
  wholeExpected?.update(context);

  const results: ChunkResult[] = [];
  for (let chunk = first; chunk <= last; chunk += 1) {
    const inputHash = createHash("sha256");
    const expectedHash = suite.reference ? createHash("sha256") : undefined;
    const prefix = new Uint8Array(context.length + 4);
    prefix.set(context, 0);
    new DataView(prefix.buffer).setUint32(context.length, chunk, true);
    inputHash.update(prefix);
    expectedHash?.update(prefix);
    const inputs = new Sink((bytes) => {
      inputHash.update(bytes);
      wholeInputs?.update(bytes);
    });
    const outputs = expectedHash === undefined ? undefined : new Sink((bytes) => {
      expectedHash.update(bytes);
      wholeExpected?.update(bytes);
    });

    const start = chunk * chunkSize;
    const end = Math.min(product, start + chunkSize);
    let cases = 0;
    let calls = 0;
    for (let ordinal = start; ordinal < end; ordinal += 1) {
      if (admitConstant || admit(env) === true) {
        cases += 1;
        calls += fixedCalls >= 0 ? fixedCalls : (dynamicCalls as (e: Value[]) => number)(env);
        for (let k = 0; k < count; k += 1) (writers[k] as (s: StreamSink, v: unknown) => void)(inputs, env[k]);
        if (outputs !== undefined) (writeExpected as (s: StreamSink, v: unknown) => void)(outputs, (expectedRun as (e: Value[]) => Value)(env));
      }
      // The odometer: increment the last binder, carry leftwards.  The values of binders that did not change are reused.
      let k = count - 1;
      while (k >= 0) {
        const next = (idx[k] as number) + 1;
        if (next < (sizes[k] as number)) {
          idx[k] = next;
          env[k] = (domains[k] as Domain).nth(next);
          break;
        }
        idx[k] = 0;
        env[k] = (domains[k] as Domain).nth(0);
        k -= 1;
      }
    }
    inputs.flush();
    outputs?.flush();
    results.push({
      index: chunk,
      cases,
      calls,
      inputs: inputHash.digest("hex"),
      expected: expectedHash?.digest("hex"),
    });
  }
  const out: { results: ChunkResult[]; whole?: WholeDigests } = { results };
  if (wholeInputs !== undefined) {
    out.whole = { inputs: wholeInputs.digest("hex"), expected: wholeExpected?.digest("hex") };
  }
  return out;
}

/** Chunk `chunk` of `suite`, enumerated alone. */
export function enumerateChunk(suite: Suite, chunk: number): ChunkResult {
  return (enumerateChunks(suite, chunk, chunk).results[0]) as ChunkResult;
}
