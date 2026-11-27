/**
 * # Reading a committed v2 corpus at `ir.core`, `ir.fixed` and `ir.tuple`
 *
 * This is the **library half** of the lane's corpus reader: it loads a
 * `<module>-warrant-v2/` directory, classifies every suite and control by what
 * it *reaches* ([D64], R69), compiles and enumerates the suites this lane can
 * run, and compiles the fault rows.  The session, the binding and the manifest
 * (S7b) sit on top of it and are not here.
 *
 * ## Per-unit, never whole-corpus
 *
 * A corpus that mixes levels is handled **unit by unit**.  A suite (or control)
 * that reaches a level this lane lacks is recorded `not-live` with the level,
 * counted in `not_live`, and *not compiled*; everything else runs.  A lane at
 * `ir.core` therefore still runs the `ir.core` suites of lzw, x25519 and
 * wasm-leb128, whose models, ops and other suites need `ir.fixed` and `ir.tuple`.
 * (This lane implements both since S10f, so for it all four committed corpora
 * are live; a lane handed a smaller `levels` list, as the tests do, still
 * records the rest `not-live`.)
 *
 * What the library reports is exactly **suites** and **controls**:
 *
 * ```
 * { suites:   [{ id, state: "live" | "not-live", level?, chunks?, cases?, calls? }],
 *   controls: [{ id, op, fault, state: "ok" | "not-live", level? }],
 *   notLive:  <not-live suites> + <not-live controls> }
 * ```
 *
 * A model or an op that needs a missing level gets **no row** [D65]: every
 * consequence of it is already visible in the suites and controls that reach it,
 * and the manifest's key set stays closed.
 *
 * ## Verify before run (section 14)
 *
 * The whole corpus is read as hostile input: size-capped before parse, strict
 * JSON, the closed key set of section 1.4, `profile`/`schema_version`, digests
 * re-derived from the bytes on disk, chunk files opened *only* under the names
 * derived from validated claim fields (a corpus carries no path), every opened
 * file a regular file inside the directory, and an undeclared file in `chunks/`
 * a corpus error.
 */

import { createHash } from "node:crypto";
import { lstatSync, readdirSync, realpathSync } from "node:fs";
import { join, sep } from "node:path";
import { KitError, type KitCode } from "./errors.ts";
import { parseJson, readBytes } from "../jsonio.ts";
import { MAX_CHUNK_FILE_BYTES, asCorpusError, checkClaimDevice, enumerateChunks, loadSuite, readChunkFile, readCorpusPart, type CorpusPart, type Suite } from "./chunks.ts";
import { type CompiledFault, compileFault } from "./faults.ts";
import { DEFINED_LEVELS, IMPLEMENTED_LEVELS } from "./levels.ts";
import { readOp } from "./ir.ts";

/** `corpus.json` is size-capped before parse (section 14). */
export const MAX_CORPUS_JSON_BYTES = 16 * 1024 * 1024;

/** The closed key set of `corpus.json` (section 1.4); `omitted_examples` is the one conditional key. */
const CORPUS_KEYS = [
  "profile", "schema_version", "module", "document", "spec", "spec_sha256", "profile_sha256", "generator", "budget",
  "chunk_size", "max_ir_cost", "levels", "types", "ops", "models", "model_order", "fault_models", "suites", "examples",
  "faults", "controls", "accepted_blind_faults", "assumes",
]; // prettier-ignore

/** How deep a run goes (section 5.6): every chunk, or the corpus-chosen `slim_chunks`. */
export type Depth = "full" | "slim";

/** One suite's classification and (once run) its enumeration. */
export interface SuiteRecord {
  readonly id: string;
  readonly state: "live" | "not-live";
  /** The level that makes it not-live. */
  readonly level?: string;
  readonly suite?: Suite;
}

/** One control's classification. */
export interface ControlRecord {
  readonly id: string;
  readonly op: string;
  readonly fault: string;
  readonly state: "ok" | "not-live";
  readonly level?: string;
  /** The suites the control re-runs (its `required_claims`), counted by {@link checkWork}. */
  readonly requiredClaims: readonly string[];
  readonly compiled?: CompiledFault;
}

/** A loaded corpus. */
export interface V2Corpus {
  readonly dir: string;
  readonly module: string;
  readonly levels: ReadonlySet<string>;
  readonly part: CorpusPart;
  readonly suites: SuiteRecord[];
  readonly controls: ControlRecord[];
  /** The number of not-live suites plus the number of not-live controls (section 4.1). */
  readonly notLive: number;
}

/** What one suite's run found. */
export interface SuiteRun {
  readonly id: string;
  readonly chunks: number;
  readonly cases: number;
  readonly calls: number;
}

function corrupt(why: string): never {
  throw new KitError("corpus", why);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * The chunk file name of a claim: every `/` replaced by `.`, plus `.txt` (section 5.5).
 *
 * The claim field is one or two names matching `[A-Za-z_][A-Za-z0-9_]{0,63}`
 * joined by `/`, so the result holds no path separator, no `..` and no leading
 * dot; a corpus carries no path, so there is nothing for a hostile corpus to
 * point elsewhere.  A name any of whose dot-separated segments is a Windows
 * reserved device name is a corpus error, and so is one longer than 133 bytes.
 */
export function chunkFileName(claim: string): string {
  const name = `${claim.replaceAll("/", ".")}.txt`;
  checkClaimDevice(claim);
  if (name.length > 133) corrupt("a chunk file name is longer than 133 bytes");
  return name;
}

/**
 * The path `parts` below `dir`, after every component has been looked at with
 * `lstat`: a component that is missing is `missing` (a clean refusal, never a
 * raw `ENOENT`), a symbolic link is refused and not followed (section 14), and
 * the resolved path must still lie inside the fixture directory.  `dir` itself
 * is the caller's and may be reached through a link; only what is *below* it is
 * the corpus's to control.
 */
function pathBelow(dir: string, parts: readonly string[], missing: KitCode): string {
  let at = dir;
  for (const part of parts) {
    at = join(at, part);
    let stat;
    try {
      stat = lstatSync(at);
    } catch {
      throw new KitError(missing, "a file or directory the corpus needs is missing");
    }
    if (stat.isSymbolicLink()) corrupt("a path component is a symbolic link");
  }
  const root = realpathSync(dir);
  if (!realpathSync(at).startsWith(root + sep)) corrupt("a path resolves outside the fixture directory");
  return at;
}

/** A file below `dir`, opened only if it is a regular file with no symbolic link on its way. */
function regularFile(dir: string, parts: readonly string[], limit: number, missing: KitCode = "corpus"): Uint8Array {
  const path = pathBelow(dir, parts, missing);
  if (!lstatSync(path).isFile()) corrupt("an opened file is a regular file");
  return readBytes(path, limit);
}

function sha256(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

/**
 * Load `dir` (a `<module>-warrant-v2/` directory) for a lane with `levels`.
 *
 * Every suite and every control is classified by what it reaches; nothing is
 * refused for a level, and nothing of a missing level is compiled.
 */
export function loadV2Corpus(dir: string, levels: readonly string[] = IMPLEMENTED_LEVELS): V2Corpus {
  const laneLevels = new Set(levels);
  const bytes = regularFile(dir, ["corpus.json"], MAX_CORPUS_JSON_BYTES);
  const raw = parseJson(bytes);
  if (!isRecord(raw)) corrupt("corpus.json is an object");
  const keys = Object.keys(raw);
  const expected = Object.hasOwn(raw, "omitted_examples") ? [...CORPUS_KEYS, "omitted_examples"] : CORPUS_KEYS;
  if (keys.length !== expected.length || !expected.every((key) => Object.hasOwn(raw, key))) corrupt("corpus.json has a missing or unknown key");
  if (raw["profile"] !== "warrant-corpus-v2" || raw["schema_version"] !== 2) corrupt("not a warrant-corpus-v2 document");
  // An unknown level name is a corpus error wherever it stands (section 4.2 rule 7).
  const declaredLevels = raw["levels"];
  if (!Array.isArray(declaredLevels) || declaredLevels.some((level) => typeof level !== "string" || !DEFINED_LEVELS.has(level))) {
    corrupt("levels names a level this document does not define");
  }
  if (typeof raw["module"] !== "string") corrupt("module is text");
  if (Object.hasOwn(raw, "omitted_examples")) checkOmittedExamples(raw["omitted_examples"]);
  if (raw["profile_sha256"] !== sha256(regularFile(dir, ["corpus.profile"], 64 * 1024))) corrupt("profile_sha256 is stale");

  const part = readCorpusPart(
    {
      chunk_size: raw["chunk_size"],
      max_ir_cost: raw["max_ir_cost"],
      types: raw["types"],
      ops: raw["ops"],
      models: raw["models"],
      model_order: raw["model_order"],
    },
    laneLevels,
  );

  // Suites, one by one.
  const suitesRaw = raw["suites"];
  if (!Array.isArray(suitesRaw)) corrupt("suites is a list");
  const seen = new Set<string>();
  const derived = new Set<string>();
  const unionOfRequires = new Set<string>();
  const suites: SuiteRecord[] = [];
  for (const row of suitesRaw as unknown[]) {
    const id = isRecord(row) && typeof row["id"] === "string" ? row["id"] : corrupt("a suite row has an id");
    if (seen.has(id)) corrupt("a suite id appears twice");
    seen.add(id);
    const claim = (row as Record<string, unknown>)["claim"];
    if (typeof claim === "string") {
      const name = chunkFileName(claim);
      if (derived.has(name.toLowerCase())) corrupt("two claims have one chunk file name ignoring case");
      derived.add(name.toLowerCase());
    }
    const declared = (row as Record<string, unknown>)["requires"];
    const declaredMissing = Array.isArray(declared) ? [...declared].filter((level): level is string => typeof level === "string" && !laneLevels.has(level)).sort()[0] : undefined;
    for (const level of Array.isArray(declared) ? declared : []) if (typeof level === "string") unionOfRequires.add(level);
    try {
      const suite = loadSuite(part, row);
      // R69 (e): a row whose `requires` is inside the lane's levels but whose reach is not is a corpus error; the opposite is not-live.
      suites.push(declaredMissing === undefined ? { id, state: "live", suite } : { id, state: "not-live", level: declaredMissing });
    } catch (error) {
      if (error instanceof KitError && error.code === "slim-chunks") corrupt("a suite's slim_chunks is malformed");
      if (!(error instanceof KitError) || error.code !== "level-missing") throw error;
      if (declaredMissing === undefined) corrupt("a suite's requires disagrees with what it reaches");
      suites.push({ id, state: "not-live", level: declaredMissing });
    }
  }

  // Controls and faults.
  const controls: ControlRecord[] = [];
  const faultsRaw = raw["faults"];
  const controlsRaw = raw["controls"];
  if (!Array.isArray(faultsRaw) || !Array.isArray(controlsRaw)) corrupt("faults and controls are lists");
  const compiled = new Map<string, ControlRecord>();
  const claimsOf = new Map<string, readonly string[]>();
  for (const control of controlsRaw as unknown[]) {
    if (!isRecord(control) || typeof control["id"] !== "string") corrupt("a control row has an id");
    if (claimsOf.has(control["id"])) corrupt("a control id appears twice");
    const claims = control["required_claims"];
    if (!Array.isArray(claims) || claims.some((claim) => typeof claim !== "string")) corrupt("a control's required_claims is a list of suite ids");
    claimsOf.set(control["id"], claims as string[]);
    for (const level of Array.isArray(control["requires"]) ? (control["requires"] as unknown[]) : []) if (typeof level === "string") unionOfRequires.add(level);
  }
  for (const fault of faultsRaw as unknown[]) {
    if (!isRecord(fault) || typeof fault["op"] !== "string" || typeof fault["fault"] !== "string") corrupt("a fault row names its op and fault");
    const op = part.program.ops.get(fault["op"]);
    if (op === undefined) corrupt("a fault names an op that is not in ops");
    const id = `warrant/${raw["module"]}/negctl/exhaustive/${fault["op"]}/${fault["fault"]}`;
    if (compiled.has(id)) corrupt("a fault row appears twice");
    for (const level of Array.isArray(fault["requires"]) ? (fault["requires"] as unknown[]) : []) if (typeof level === "string") unionOfRequires.add(level);
    const requiredClaims = claimsOf.get(id) ?? [];
    try {
      const wrapper = compileFault(part.program, fault, op);
      compiled.set(id, { id, op: fault["op"], fault: fault["fault"], state: "ok", requiredClaims, compiled: wrapper });
    } catch (error) {
      // A fault row that does not compile is a corrupt corpus (`corpus`), whatever the kit's own vocabulary calls the reason; only a missing level is not-live.
      const refusal = asCorpusError(error);
      if (!(refusal instanceof KitError) || refusal.code !== "level-missing") throw refusal;
      compiled.set(id, { id, op: fault["op"], fault: fault["fault"], state: "not-live", requiredClaims, ...(refusal.level === undefined ? {} : { level: refusal.level }) });
    }
  }
  for (const control of controlsRaw as unknown[]) {
    const found = compiled.get((control as Record<string, string>)["id"] as string);
    if (found === undefined) corrupt("a control has no fault row");
    controls.push(found);
  }

  // Section 1.4: `levels` is the union of every `requires`, no more and no less.
  const levelSet = new Set(declaredLevels as string[]);
  for (const op of part.program.ops.values()) for (const level of op.requires) unionOfRequires.add(level);
  if (levelSet.size !== (declaredLevels as string[]).length || levelSet.size !== unionOfRequires.size || [...levelSet].some((level) => !unionOfRequires.has(level))) {
    corrupt("levels is not the union of every requires");
  }

  // `chunks/` may hold only the derived names (an extra file is a corpus error); the directory itself may not be a link.
  const chunksDir = pathBelow(dir, ["chunks"], "corpus");
  if (!lstatSync(chunksDir).isDirectory()) corrupt("chunks is a directory");
  for (const entry of readdirSync(chunksDir)) {
    if (!derived.has(entry.toLowerCase())) corrupt("an undeclared file in chunks/");
    if (!lstatSync(join(chunksDir, entry)).isFile()) corrupt("a chunks/ entry is not a regular file");
  }

  const notLive = suites.filter((s) => s.state === "not-live").length + controls.filter((c) => c.state === "not-live").length;
  return { dir, module: raw["module"] as string, levels: laneLevels, part, suites, controls, notLive };
}

/** The closed vocabulary of an omitted example's `reason` (section 1.4, [D34]). */
const OMIT_REASONS = new Set(["value-over-limit", "not-op-eq-pure"]);

/**
 * The consumer's checks of the conditional `omitted_examples` key (section 1.4): a present key is never `[]`, its rows
 * are `{id, reason}` with a reason of the closed set, sorted by `id` (bytes) and without a duplicate.
 */
function checkOmittedExamples(raw: unknown): void {
  if (!Array.isArray(raw) || raw.length === 0) corrupt("omitted_examples is present only when it is not empty");
  let previous: string | undefined;
  for (const row of raw as unknown[]) {
    if (!isRecord(row) || Object.keys(row).length !== 2 || typeof row["id"] !== "string" || typeof row["reason"] !== "string") {
      corrupt("an omitted example is {id, reason}");
    }
    if (!OMIT_REASONS.has(row["reason"])) corrupt("an omitted example has an unknown reason");
    // Sorted by id, bytes: ids are text, compared by their UTF-8 bytes.
    if (previous !== undefined && Buffer.compare(Buffer.from(previous), Buffer.from(row["id"])) >= 0) corrupt("omitted_examples is sorted by id without a duplicate");
    previous = row["id"];
  }
}

/**
 * Run one live suite at `depth`: read its chunk file (by the derived name),
 * cross-check it against the row, enumerate the chosen chunks and require every
 * cases, calls and digest value to match the file -- and, at full depth, the
 * v1 whole-suite digests over the unchunked stream where the suite has them.
 */
export function runSuite(corpus: V2Corpus, record: SuiteRecord, depth: Depth = "full"): SuiteRun {
  const suite = record.suite;
  if (suite === undefined) throw new KitError("corpus", "a not-live suite is not run");
  // One suite, no control: it is charged its own work only.  The whole-run meter (every suite, control re-runs included) is the
  // planner's, `checkWork(corpus, depth)`, and a caller that runs the corpus calls it once before the first suite.
  checkWork(corpus, depth, record);
  const file = regularFile(corpus.dir, ["chunks", chunkFileName(suite.claim)], MAX_CHUNK_FILE_BYTES, "chunk-file");
  const rows = readChunkFile(file, suite);
  const wanted = depth === "full" ? rows.map((r) => r.index) : suite.slimChunks.slice();
  // A run that ran nothing proves nothing: never report success over zero chunks.
  if (wanted.length === 0) throw new KitError("corpus", "a run has no chunk to execute");
  let cases = 0;
  let calls = 0;
  const check = (actual: { cases: number; calls: number; inputs: string; expected: string | undefined }, index: number): void => {
    const row = rows[index];
    if (row === undefined || row.cases !== actual.cases || row.calls !== actual.calls || row.inputs !== actual.inputs || row.expected !== actual.expected) {
      throw new KitError("chunk-file", "a chunk differs from its line of the chunk file");
    }
    cases += actual.cases;
    calls += actual.calls;
  };
  if (depth === "full") {
    const walked = enumerateChunks(suite, 0, suite.chunks - 1, { whole: suite.stream === "warrant-stream-v1" });
    walked.results.forEach((result, index) => check(result, index));
    if (walked.whole !== undefined) {
      if (walked.whole.inputs !== suite.v1InputsSha256 || walked.whole.expected !== suite.v1ExpectedSha256) {
        throw new KitError("chunk-file", "the unchunked v1 stream digest is not reproduced");
      }
    }
  } else {
    for (const index of wanted) check(enumerateChunks(suite, index, index).results[0] as ReturnType<typeof enumerateChunks>["results"][number], index);
  }
  return { id: suite.id, chunks: wanted.length, cases, calls };
}

/** Re-export so a caller holding only this module can name an op row reader (used by tests). */
export { readOp };

// ---------------------------------------------------------------------------
// Total work, checked before the first case (section 4.2 rule 6, section 14)
// ---------------------------------------------------------------------------

/** The cases a `slim` run may spend over all suites, control re-runs included. */
export const MAX_TOTAL_WORK_SLIM = 32_000_000;
/** The product ordinals a `full` run may spend over all suites. */
export const MAX_TOTAL_WORK_FULL = 2 ** 32;
/** The IR work of a run: `ir_cost` times the product ordinals it executes, over all suites. */
export const MAX_IR_WORK_SLIM = 64_000_000_000;
export const MAX_IR_WORK_FULL = 8_589_934_592_000;

/** What a run would cost, in the two meters of section 14. */
export interface Work {
  /** Product ordinals executed (`full`) or admitted cases bounded by the slim chunks' ordinals (`slim`). */
  readonly ordinals: number;
  /** The sum over suites of `ir_cost` times the ordinals executed. */
  readonly irWork: number;
}

/**
 * The work of running the live suites of `corpus` at `depth`, with the case
 * limits and the IR-work limits checked.  A `slim` run executes the ordinals of
 * its chosen chunks (at most 32 chunks of at most 65,536); a `full` run all of
 * them.  Exceeding a bound is a corpus error, raised before any case runs.
 */
export function checkWork(corpus: V2Corpus, depth: Depth, only?: SuiteRecord): Work {
  if (depth !== "full" && depth !== "slim") corrupt("depth is full or slim");
  let ordinals = 0;
  let irWork = 0;
  const liveBy = new Map<string, Suite>();
  // The product ordinals a suite's slim chunk set executes.  The set is validated again here: this meter must never add a
  // value that is not a chunk of the suite, or one bad entry would turn the total into NaN and no limit could fire.
  const slimOrdinals = (suite: Suite): number => {
    let sum = 0;
    for (const chunk of suite.slimChunks) {
      if (!Number.isSafeInteger(chunk) || chunk < 0 || chunk >= suite.chunks) corrupt("a slim chunk is not a chunk of its suite");
      sum += Math.min(suite.chunkSize, suite.product - chunk * suite.chunkSize);
    }
    return sum;
  };
  const add = (suite: Suite, count: number): void => {
    ordinals += count;
    irWork += suite.irCost * count;
  };
  for (const record of only === undefined ? corpus.suites : [only]) {
    const suite = record.suite;
    if (suite === undefined || record.state !== "live") continue;
    liveBy.set(suite.id, suite);
    add(suite, depth === "slim" ? slimOrdinals(suite) : suite.product);
  }
  // Section 14 counts negative-control re-runs in the slim total: a live control re-runs each suite it requires
  // (its `required_claims`) over that suite's slim chunk set.  A full run runs no control, so adds none.
  if (depth === "slim" && only === undefined) {
    for (const control of corpus.controls) {
      if (control.state !== "ok") continue;
      for (const claim of control.requiredClaims) {
        const suite = liveBy.get(claim);
        if (suite !== undefined) add(suite, slimOrdinals(suite));
      }
    }
  }
  const [limit, irLimit] = depth === "slim" ? [MAX_TOTAL_WORK_SLIM, MAX_IR_WORK_SLIM] : [MAX_TOTAL_WORK_FULL, MAX_IR_WORK_FULL];
  // `!(x <= limit)` and not `x > limit`: a total that is NaN would pass the second and no limit could ever fire.
  if (!(ordinals <= limit)) corrupt("the run exceeds the total work limit");
  if (!(irWork <= irLimit)) corrupt("the run exceeds the total IR work limit");
  return { ordinals, irWork };
}
