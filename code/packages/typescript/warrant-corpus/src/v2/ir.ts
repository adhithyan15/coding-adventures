/**
 * # The model IR at levels `ir.core`, `ir.fixed` and `ir.tuple`: scan, type, cost, evaluate
 *
 * A model IR expression is a JSON tree, `["tag", operand...]`, with no
 * recursion, no loop and no mutable state (VER04r section 4.2).  This module
 * turns one into a closure in four steps, and each step has its own refusals,
 * so that a hostile corpus is rejected *before the first case runs*:
 *
 * 1. **scan** (tags only).  Checks everything that can be decided by reading
 *    tag names alone: expression depth (at most 64 nodes on a path), unknown
 *    tags, a `subject_op` where none may stand, and every `model_call` against
 *    the order of `model_order`.  It also collects which levels, models and ops
 *    the tree *reaches*.
 * 2. **reach** ([D64], R69).  A tag, a type or a signature of a level this lane
 *    lacks is `level-missing`, but only for what is *reached*; a model that
 *    needs a missing level and that nothing calls is merely not compiled.
 * 3. **type**.  Infers a static type per node (R51's `flows` and `unify`),
 *    finds impure guards, and computes the worst-path cost.
 * 4. **compile**.  Turns the typed tree into closures specialised by type: the
 *    dispatch on a node's tag and type happens once, not once per case.
 *
 * ## The three tiers of R69 (c)
 *
 * (1) A model or op whose own tags and types are inside this lane's levels is
 * compiled and validated in full whether or not anything reaches it.  (2) One
 * that needs a missing level is not compiled, and only the checks that read tag
 * names alone apply to it.  (3) Only `level-missing` is scoped by reach.
 *
 * ## Arithmetic: the single rule [D4]
 *
 * Every `+ - *` and unary `-` on a fixed-width type **wraps** modulo 2^w (two's
 * complement for signed types).  `Nat` and `Int` are exact, and the result of
 * every arithmetic node on them has type `Int` (R42); they live in the window
 * `-(2^127-1) ..= 2^127-1`, and a result outside it is `ir-overflow`.  A lane
 * never continues with a wrapped exact value.
 *
 * ## The two classes of overflow (section 3.2, R71)
 *
 * An overflow is a *corpus* error when the generator would have evaluated the
 * node (so an honest corpus never contains it) and a *case failure* otherwise.
 * R71's lane rule needs no trace: it is a **case** failure when the node has a
 * `subject_op` beneath it, or is evaluated inside a model entered through a
 * `model_call` with a subject argument, or lies in a branch, right operand or
 * arm whose guard contains a `subject_op`.  The compiler threads one boolean,
 * "tainted", through exactly those places.
 */

import { KitError, type KitCode } from "./errors.ts";
import { type Fx, binaryFixed, boundValue, smallWrap, unaryFixed } from "./fixed.ts";
import { DEFINED_LEVELS, firstMissing, levelOfTag } from "./levels.ts";
import {
  BOOL,
  BYTES,
  INT,
  NAT,
  type Ty,
  type TypeTable,
  checkNames,
  faultCommon,
  flows,
  hasEquality,
  isExact,
  isFixed,
  isInteger,
  isName,
  parseType,
  readTypeTable,
  sameType,
  showType,
  substituteWidths,
  typeLevels,
  unify,
} from "./types.ts";
import {
  INT_WINDOW,
  type Value,
  fromHex,
  isSmall,
  parseDecimal,
  valuesEqual,
  fixedRange,
} from "./values.ts";

/** Expression nesting limit and active `model_call` frame limit (section 14). */
export const MAX_DEPTH = 64;
/** The default and the ceiling of `max_ir_cost` (section 4.2 rule 6). */
export const DEFAULT_MAX_COST = 2_000;
export const COST_CEILING = 65_536;
/** Any `Bytes` an evaluator produces is at most this many bytes, checked before the allocation. */
export const MAX_IR_VALUE = 65_536;

/** Where an expression stands: a body may call the subject; the others may not. */
/**
 * Where an expression stands.  `fault-reach` is a fault expression read with `subject_op` allowed: a fault row reads the
 * three codes and the levels of every expression before it reads `subject-forbidden` (R119 (c)).
 */
export type Role = "body" | "admit" | "fault" | "fault-reach" | "model" | "pure";

/** A parameter or binder: a name and its type. */
export interface Param {
  readonly name: string;
  readonly ty: Ty;
}

/** A parsed op row (section 6): the part of it a lane needs to type and call. */
export interface OpSig {
  readonly name: string;
  readonly params: readonly Param[];
  readonly ret: Ty;
  readonly widthParams: readonly string[];
  readonly requires: readonly string[];
}

/** An evaluator frame: the values of binders or parameters by slot. */
export type Env = Value[];
type Closure = (env: Env) => Value;

// ---------------------------------------------------------------------------
// The typed tree
// ---------------------------------------------------------------------------

interface T {
  readonly tag: string;
  readonly ty: Ty;
  readonly kids: readonly T[];
  /** Does a `subject_op` sit in this subtree? */
  readonly sub: boolean;
  /** Worst-path cost (section 4.2 rule 6). */
  readonly cost: number;
  readonly value?: Value;
  readonly slot?: number;
  readonly name?: string;
  /** `project`: the 0-based component. */
  readonly index?: number;
  /** `widen wrap try from_be from_le max_value min_value`: the static type argument. */
  readonly target?: Fx;
  /** `match`: the typed patterns, one per arm (`kids[0]` is the scrutinee, `kids[i + 1]` the body of arm `i`). */
  readonly pats?: readonly TPat[];
}

/**
 * A typed `match` pattern (Appendix A.1).  `bind` and `wild` both match anything; `bind` also stores the value in `slot`
 * of the environment before its arm runs.  `err` with `c === null` is `err_any`.
 */
type TPat =
  | { readonly k: "wild" }
  | { readonly k: "bind"; readonly slot: number }
  | { readonly k: "ok"; readonly p: TPat }
  | { readonly k: "err"; readonly c: string | null }
  | { readonly k: "tuple"; readonly ps: readonly TPat[] };

/** The type a fault row's declared position hands down (R67, R98), and whether the position is below a tuple (R96). */
interface Expect {
  readonly ty: Ty;
  readonly narrow: boolean;
}

type Scope = ReadonlyMap<string, { slot: number; ty: Ty }>;

/** What a scan learned about a tree. */
interface ScanInfo {
  readonly levels: Set<string>;
  readonly models: Set<string>;
  readonly ops: Set<string>;
  readonly litTypes: string[];
}

function newInfo(): ScanInfo {
  return { levels: new Set(), models: new Set(), ops: new Set(), litTypes: [] };
}

function fail(code: KitCode, why: string): never {
  throw new KitError(code, why);
}

// ---------------------------------------------------------------------------
// Reading ops
// ---------------------------------------------------------------------------

/** Two parameters (or binders) of one signature may not share a name: the later would silently shadow the earlier. */
function distinctNames(params: readonly Param[], why: string): void {
  const seen = new Set<string>();
  for (const p of params) {
    if (seen.has(p.name)) fail("ill-typed", why);
    seen.add(p.name);
  }
}

/** Read one op row.  Types parse with named widths allowed; each named width must be declared. */
export function readOp(name: string, raw: unknown): OpSig {
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) return fail("ill-typed", "an op row is an object");
  const row = raw as Record<string, unknown>;
  const widthRaw = row["width_params"];
  const widthParams: string[] = [];
  if (widthRaw !== undefined) {
    if (!Array.isArray(widthRaw)) return fail("ill-typed", "width_params is a list");
    for (const item of widthRaw) {
      if (typeof item !== "string" || !isName(item) || widthParams.includes(item)) return fail("ill-typed", "a width parameter is a unique name");
      widthParams.push(item);
    }
  }
  const paramsRaw = row["params"];
  if (!Array.isArray(paramsRaw)) return fail("ill-typed", "params is a list");
  const params: Param[] = paramsRaw.map((pair) => {
    if (!Array.isArray(pair) || pair.length !== 2 || typeof pair[0] !== "string" || typeof pair[1] !== "string") {
      return fail("ill-typed", "an op parameter is written [name, type]");
    }
    return { name: pair[0], ty: parseType(pair[1], { allowNamedWidth: true, code: "ill-typed" }) };
  });
  distinctNames(params, "an op names each parameter once");
  if (typeof row["ret"] !== "string") return fail("ill-typed", "ret is a type");
  const ret = parseType(row["ret"], { allowNamedWidth: true, code: "ill-typed" });
  const named = (ty: Ty): void => {
    if (ty.k === "Bytes" && typeof ty.n === "string" && !widthParams.includes(ty.n)) fail("ill-typed", "a named width is not declared");
    if (ty.k === "Tuple") ty.of.forEach(named);
    if (ty.k === "List") named(ty.of);
    if (ty.k === "Option" && ty.of !== null) named(ty.of);
    if (ty.k === "Result" && ty.ok !== null) named(ty.ok);
  };
  params.forEach((p) => named(p.ty));
  named(ret);
  // An op's `requires` is a list of defined level names (section 4.2 rule 7: an undefined level is a corpus error wherever it stands).
  const requiresRaw = row["requires"];
  let requires: string[] = [];
  if (requiresRaw !== undefined) {
    if (!Array.isArray(requiresRaw) || requiresRaw.some((x) => typeof x !== "string" || !DEFINED_LEVELS.has(x))) {
      return fail("ill-typed", "an op's requires lists only levels this document defines");
    }
    requires = requiresRaw as string[];
  }
  return { name, params, ret, widthParams, requires };
}

// ---------------------------------------------------------------------------
// Step 1: scan (tag names only)
// ---------------------------------------------------------------------------

function isNodeArray(value: unknown): value is unknown[] {
  return Array.isArray(value) && value.length > 0 && typeof value[0] === "string";
}

/** The tags whose position 1 is a static type argument (a fixed-width type name): its text counts for the level (R69 (b)). */
const TYPE_ARG_TAGS: ReadonlySet<string> = new Set(["widen", "wrap", "try", "max_value", "min_value", "from_be", "from_le"]);

/**
 * The levels of a `match` pattern (R76): a data pattern (`some none ctor bool`), at any depth, needs `ir.data`; the patterns
 * of Appendix A.1 (`ok err err_any tuple bind wild`) add nothing.  A pattern is no node, so this reads it by its own head word
 * and stops at a nesting of 200, far past the limit the typer reports.
 */
function patternLevels(pat: unknown, depth: number, info: ScanInfo): void {
  if (depth > 200 || !Array.isArray(pat) || typeof pat[0] !== "string") return;
  const head = pat[0];
  if (head === "some" || head === "none" || head === "ctor" || head === "bool") info.levels.add("ir.data");
  if (head === "ok" || head === "some") patternLevels(pat[1], depth + 1, info);
  const inner = head === "tuple" ? pat[1] : head === "ctor" ? pat[2] : undefined;
  if (Array.isArray(inner)) for (const item of inner) patternLevels(item, depth + 1, info);
}

/**
 * Walk a tree reading tag names only.
 *
 * `callerIndex` is the position of the model whose body this is in `model_order`
 * (`Infinity` for a case's own `ir`, from which every listed model is callable).
 */
function scanExpr(
  ir: unknown,
  depth: number,
  role: Role,
  callerIndex: number,
  order: ReadonlyMap<string, number>,
  info: ScanInfo,
): void {
  if (depth > MAX_DEPTH) fail("depth", "an expression is nested deeper than 64 nodes");
  if (!isNodeArray(ir)) return fail("ill-typed", "an IR node is an array that starts with a tag");
  const tag = ir[0] as string;
  const level = levelOfTag(tag);
  if (level === undefined) return fail("unknown-tag", "a tag no level defines");
  info.levels.add(level);
  if (tag === "lit") {
    const body = ir[1];
    if (ir.length !== 2) fail("ill-typed", "a lit has one operand");
    if (typeof body === "object" && body !== null && !Array.isArray(body)) {
      const written = (body as Record<string, unknown>)["type"];
      if (typeof written === "string") info.litTypes.push(written);
    }
    return;
  }
  if (TYPE_ARG_TAGS.has(tag) && typeof ir[1] === "string") info.litTypes.push(ir[1]);
  if (tag === "var" || tag === "err") return;
  if (tag === "subject_op") {
    if (role !== "body" && role !== "fault-reach") fail("subject-forbidden", "a subject_op may not stand here");
    if (typeof ir[1] === "string") info.ops.add(ir[1]);
  }
  if (tag === "model_call") {
    const name = ir[1];
    // R119 (a): in a fault case (no models at all) every `model_call`, a bare one included, calls a model that is missing.
    if (typeof name !== "string") return (role === "fault" || role === "fault-reach") && order.size === 0 ? fail("model-order", "a fault case has no models to call") : fail("ill-typed", "a model_call names a model");
    const position = order.get(name);
    if (position === undefined || position >= callerIndex) fail("model-order", "a model_call must name an earlier model of model_order");
    info.models.add(name);
  }
  const visitList = (list: unknown): void => {
    if (!Array.isArray(list)) return fail("ill-typed", "an argument list is an array");
    for (const item of list) scanExpr(item, depth + 1, role, callerIndex, order, info);
  };
  if (tag === "subject_op" || tag === "model_call") {
    visitList(ir[2]);
    return;
  }
  if (tag === "match") {
    // Patterns carry tag-like words (`bind`, `wild`, `err_any`) that are not nodes: only the scrutinee and the arm bodies are scanned.
    scanExpr(ir[1], depth + 1, role, callerIndex, order, info);
    const arms = ir[2];
    if (Array.isArray(arms)) {
      for (const arm of arms) {
        if (!Array.isArray(arm) || arm.length !== 2) continue;
        patternLevels(arm[0], 0, info);
        scanExpr(arm[1], depth + 1, role, callerIndex, order, info);
      }
    }
    return;
  }
  for (let i = 1; i < ir.length; i += 1) {
    const child = ir[i];
    if (!Array.isArray(child)) continue;
    if (isNodeArray(child)) scanExpr(child, depth + 1, role, callerIndex, order, info);
    else for (const item of child) if (Array.isArray(item)) scanExpr(item, depth + 1, role, callerIndex, order, info);
  }
}

/**
 * The levels of every tag that belongs to a defined level, read without refusing anything (R119 (c) step 2): an unknown
 * tag has no level and is skipped, a node that is not a node, a nesting past 64 and a missing model are left to the
 * steps that report them.  The walk stops at a nesting of 200, far past the limit that step 3 reports.
 */
function reachExpr(ir: unknown, depth: number, order: ReadonlyMap<string, number>, info: ScanInfo): void {
  if (depth > 200 || !isNodeArray(ir)) return;
  const tag = ir[0] as string;
  const level = levelOfTag(tag);
  if (level !== undefined) info.levels.add(level);
  if (tag === "lit") {
    const body = ir[1];
    if (typeof body === "object" && body !== null && !Array.isArray(body)) {
      const written = (body as Record<string, unknown>)["type"];
      if (typeof written === "string") info.litTypes.push(written);
    }
    return;
  }
  if (TYPE_ARG_TAGS.has(tag) && typeof ir[1] === "string") info.litTypes.push(ir[1]);
  if (tag === "var" || tag === "err") return;
  if (tag === "subject_op" && typeof ir[1] === "string") info.ops.add(ir[1]);
  if (tag === "model_call" && typeof ir[1] === "string" && order.has(ir[1])) info.models.add(ir[1]);
  if (tag === "match") {
    reachExpr(ir[1], depth + 1, order, info);
    if (Array.isArray(ir[2])) {
      for (const arm of ir[2]) {
        if (!Array.isArray(arm) || arm.length !== 2) continue;
        patternLevels(arm[0], 0, info);
        reachExpr(arm[1], depth + 1, order, info);
      }
    }
    return;
  }
  const start = tag === "subject_op" || tag === "model_call" ? 2 : 1;
  for (let i = start; i < ir.length; i += 1) {
    const child = ir[i];
    if (!Array.isArray(child)) continue;
    if (isNodeArray(child)) reachExpr(child, depth + 1, order, info);
    else for (const item of child) reachExpr(item, depth + 1, order, info);
  }
}

// ---------------------------------------------------------------------------
// The program: ops, models, and what they reach
// ---------------------------------------------------------------------------

interface ModelInfo {
  readonly name: string;
  readonly index: number;
  readonly params: readonly Param[];
  readonly rawBody: unknown;
  readonly scan: ScanInfo;
  needsMissing: boolean;
  /**
   * A level the typer found the body to need once it had types (a `match` on a scrutinee that is not a `Result`, an `Option`
   * that is not an intermediate): tags and signatures alone did not show it, so the model becomes not-live and everything
   * that reaches it needs the level too.
   */
  lateLevel?: string;
  body?: T;
  ty?: Ty;
  cost: number;
  chain: number;
  readonly runners: [Cell | undefined, Cell | undefined];
}

/**
 * A model body, compiled lazily and *outside* the recursion that asked for it.
 *
 * A `model_call` closure holds the model's `Cell` and calls `cell.run` at
 * evaluation time.  The body is compiled later, by {@link Program.drain}, from a
 * work list: compiling a chain of 64 models therefore never nests 64 compilers
 * (each of which recurses over a body up to 64 deep), so a spec-legal corpus
 * cannot overflow the stack while it is being compiled (section 14: 64 frames
 * of depth 64).
 */
interface Cell {
  run: (frame: Env) => Value;
}

/** The inputs of a program: the parts of a `corpus.json` an expression needs (section 1.4). */
export interface ProgramInput {
  readonly types: unknown;
  readonly ops: unknown;
  readonly models: unknown;
  readonly modelOrder: unknown;
  /** The levels the evaluator is configured with. */
  readonly levels: ReadonlySet<string>;
  /** `max_ir_cost`; the default 2,000 when absent. */
  readonly maxIrCost?: number;
  /** The code a refusal of the corpus parts themselves carries (`corpus` for a chunk case). */
  readonly structuralCode?: KitCode;
}

/** An expression, compiled. */
export interface Compiled {
  /** The static type of the expression. */
  readonly ty: Ty;
  /** Its worst-path cost. */
  readonly cost: number;
  /** Does it contain a `subject_op`? */
  readonly sub: boolean;
  /** Evaluate it over an environment (the slots are the `vars` it was compiled with). */
  readonly run: Closure;
  /** The ops the expression reaches, through the models it calls too (R100 compiles a stand-in for each). */
  readonly ops: readonly string[];
  /** The number of subject calls on the pure trace of section 4.4, as a constant or a function of the environment. */
  readonly trace: { readonly fixed: number } | { readonly dynamic: (env: Env) => number };
}

/** The mutable part of a run: the call counter and the subject dispatch. */
export interface RunState {
  calls: number;
  subject: (op: string, args: Value[]) => Value;
}

/**
 * Ops, models and types, validated.  Building one **is** the "verify before
 * run" of section 14 for the part of a corpus that is shared by every suite.
 */
/** Steps of the usefulness check one `match` may spend: far above any kit case or adopter match, far below a hostile one. */
export const MATCH_BUDGET = 200_000;

/** The type of a name bound to an undetermined payload: it may be ignored, never used. */
const UNDETERMINED: Ty = { k: "Option", of: null };

export class Program {
  private matchSteps = MATCH_BUDGET;
  readonly table: TypeTable;
  readonly ops = new Map<string, OpSig>();
  readonly levels: ReadonlySet<string>;
  readonly maxCost: number;
  readonly state: RunState = {
    calls: 0,
    subject: () => {
      throw new Error("no subject is bound");
    },
  };
  private readonly models = new Map<string, ModelInfo>();
  private readonly order = new Map<string, number>();
  /** Bodies whose `Cell` has been handed out but whose closure is not built yet. */
  private readonly pending: (() => void)[] = [];

  constructor(input: ProgramInput) {
    const structural = input.structuralCode ?? "corpus";
    this.levels = input.levels;
    const max = input.maxIrCost ?? DEFAULT_MAX_COST;
    if (!Number.isInteger(max) || max < DEFAULT_MAX_COST || max > COST_CEILING) {
      throw new KitError(structural === "corpus" ? "corpus" : "ir-cost", "max_ir_cost is outside 2,000..=65,536");
    }
    this.maxCost = max;
    this.table = readTypeTable(input.types, structural);

    const opsRaw = input.ops;
    if (typeof opsRaw !== "object" || opsRaw === null || Array.isArray(opsRaw)) fail(structural, "ops is not an object");
    for (const [name, row] of Object.entries(opsRaw as Record<string, unknown>)) this.ops.set(name, readOp(name, row));
    // Ops are checked like models: a name must be declared whatever its level (an undeclared one is a typing error).  The
    // rest of an op's validation is tier 1 and tier 2 of R69 (c): an op of a missing level is not compiled, and is read only
    // when something reaches it.
    for (const op of this.ops.values()) [...op.params.map((p) => p.ty), op.ret].forEach((ty) => checkNames(ty, this.table, "ill-typed"));

    const modelsRaw = input.models;
    const orderRaw = input.modelOrder;
    if (typeof modelsRaw !== "object" || modelsRaw === null || Array.isArray(modelsRaw)) fail(structural, "models is not an object");
    if (!Array.isArray(orderRaw)) fail(structural, "model_order is not a list");
    const models = modelsRaw as Record<string, unknown>;
    (orderRaw as unknown[]).forEach((name, index) => {
      if (typeof name !== "string" || !Object.hasOwn(models, name) || this.order.has(name)) {
        fail("model-order", "model_order names a model that is missing from models or listed twice");
      }
      this.order.set(name as string, index);
    });
    // Section 1.4: `model_order` holds "the same names" as `models`, so a model that is declared and not listed is refused
    // too; every model of the lane's levels is then typed, costed and chain-checked below (R69 (c) tier 1).
    if (this.order.size !== Object.keys(models).length) fail("model-order", "a model is missing from model_order");

    // Tier 1 and 2 (scan): every model, listed or not, is read by tag name.
    for (const [name, row] of Object.entries(models)) {
      if (typeof row !== "object" || row === null || Array.isArray(row)) return fail(structural, "a model row is an object");
      const record = row as Record<string, unknown>;
      const paramsRaw = record["params"];
      if (!Array.isArray(paramsRaw)) return fail(structural, "a model's params is a list");
      const params: Param[] = paramsRaw.map((pair) => {
        if (!Array.isArray(pair) || pair.length !== 2 || typeof pair[0] !== "string" || typeof pair[1] !== "string") {
          return fail("ill-typed", "a model parameter is written [name, type]");
        }
        return { name: pair[0], ty: parseType(pair[1], { allowInt: true, code: "ill-typed" }) };
      });
      distinctNames(params, "a model names each parameter once");
      const scan = newInfo();
      const index = this.order.get(name) ?? -1;
      scanExpr(record["body"], 1, "model", index === -1 ? -1 : index, this.order, scan);
      this.models.set(name, {
        name,
        index,
        params,
        rawBody: record["body"],
        scan,
        needsMissing: false,
        cost: 0,
        chain: 0,
        runners: [undefined, undefined],
      });
    }

    // R77: the 64-frame limit covers every model chain, called or not.  A model's depth is 1 plus the largest depth of
    // a model its body calls, so a lane computes it from the model_call tags alone (a not-live model is counted like any other).
    for (const name of orderRaw as string[]) {
      const info = this.models.get(name) as ModelInfo;
      let deepest = 0;
      for (const callee of info.scan.models) deepest = Math.max(deepest, (this.models.get(callee) as ModelInfo).chain);
      info.chain = deepest + 1;
      if (info.chain > MAX_DEPTH) fail("depth", "a model chain is deeper than 64 frames");
    }

    // Tier 1 (type and cost) in model_order; tier 2 models are left alone.
    for (const name of orderRaw as string[]) {
      const info = this.models.get(name) as ModelInfo;
      const missing = this.missingLevel(info.scan, info.params.map((p) => p.ty));
      info.needsMissing = missing !== undefined;
      if (info.needsMissing) continue;
      info.params.forEach((p) => checkNames(p.ty, this.table, "ill-typed"));
      const scope = new Map(info.params.map((p, slot) => [p.name, { slot, ty: p.ty }]));
      let body: T;
      try {
        body = this.typeExpr(info.rawBody, scope);
      } catch (error) {
        // Only the types showed that this body needs a level the lane lacks: the model is not live (R69 (c) tier 2).
        if (!(error instanceof KitError) || error.code !== "level-missing" || error.level === undefined) throw error;
        info.needsMissing = true;
        info.lateLevel = error.level;
        continue;
      }
      if (body.cost > this.maxCost) fail("ir-cost", "a model body costs more than max_ir_cost");
      info.body = body;
      info.ty = body.ty;
      info.cost = body.cost;
    }
  }

  /** Is `name` an op, model or type this program knows?  (used by the harness) */
  hasModel(name: string): boolean {
    return this.models.has(name);
  }

  /** Every model and op a tree reaches, transitively through the models it calls (a work list, not a recursion). */
  private closure(scan: ScanInfo): { models: Set<string>; ops: Set<string> } {
    const models = new Set<string>();
    const ops = new Set<string>();
    const work: ScanInfo[] = [scan];
    for (let info = work.pop(); info !== undefined; info = work.pop()) {
      for (const op of info.ops) if (this.ops.has(op)) ops.add(op);
      for (const name of info.models) {
        if (models.has(name)) continue;
        models.add(name);
        work.push((this.models.get(name) as ModelInfo).scan);
      }
    }
    return { models, ops };
  }

  /** The static signature of a live model (the parameters and the result type), or `undefined` when it is unknown or not live. */
  modelSignature(name: string): { readonly params: readonly Param[]; readonly ty: Ty } | undefined {
    const info = this.models.get(name);
    return info === undefined || info.ty === undefined ? undefined : { params: info.params, ty: info.ty };
  }

  /**
   * The tag-name checks alone, over a tree that may need a level this lane lacks:
   * `depth`, `unknown-tag`, `subject-forbidden`, `model-order`.  A suite runs this over every expression it holds
   * *before* it records `level-missing`, so that an unknown tag cannot hide inside a not-live suite (R69 (c) tier 2, (d)).
   */
  scanTags(ir: unknown, role: Role): void {
    scanExpr(ir, 1, role, Number.POSITIVE_INFINITY, this.order, newInfo());
  }

  /**
   * The `level-missing` check alone, over one expression (its tags, the models and ops it reaches, and the types of `vars`).
   * A fault row runs it over every expression of the row before it types any (R119 (c)).  It reads the tree with
   * an unknown tag skipped (it has no level) and nothing refused but a level.
   */
  checkReach(ir: unknown, vars: readonly Param[]): void {
    const scan = newInfo();
    reachExpr(ir, 1, this.order, scan);
    const missing = this.missingLevel(scan, vars.map((v) => v.ty));
    if (missing !== undefined) throw new KitError("level-missing", "the expression reaches a construct of a level this lane lacks", { level: missing });
  }

  /**
   * The first level this lane lacks among what `scan` reaches (its own tags and
   * types, the models and ops it reaches transitively, and the extra types the
   * caller passes: an environment, an op signature), or `undefined`.
   */
  private missingLevel(scan: ScanInfo, extraTypes: readonly Ty[]): string | undefined {
    const levels = new Set<string>();
    const seenModels = new Set<string>();
    const seenOps = new Set<string>();
    // A name must be declared before its level can be read: an undeclared one is a typing error wherever it stands.
    const typeSet = (ty: Ty): void => {
      checkNames(ty, this.table, "ill-typed");
      typeLevels(ty, levels);
    };
    const visitScan = (info: ScanInfo): void => {
      for (const level of info.levels) levels.add(level);
      for (const written of info.litTypes) {
        try {
          typeSet(parseType(written, { allowInt: true }));
        } catch {
          // A literal whose type does not parse is a typing error, reported by the typer.
        }
      }
      for (const opName of info.ops) {
        const op = this.ops.get(opName);
        if (op === undefined || seenOps.has(opName)) continue;
        seenOps.add(opName);
        op.params.forEach((p) => typeSet(p.ty));
        typeSet(op.ret);
      }
      for (const modelName of info.models) {
        if (seenModels.has(modelName)) continue;
        seenModels.add(modelName);
        const model = this.models.get(modelName) as ModelInfo;
        model.params.forEach((p) => typeSet(p.ty));
        if (model.lateLevel !== undefined) levels.add(model.lateLevel);
        visitScan(model.scan);
      }
    };
    visitScan(scan);
    extraTypes.forEach(typeSet);
    return firstMissing(levels, this.levels);
  }

  /**
   * Compile an expression over the given variables (their order is the slot
   * order of the environments it will run on).  `expected` is the declared type of a
   * **fault row**'s position (the parameter it rewrites, the op's result): it is handed
   * down as R67 and R98 say, and every position it reaches is compared with it by the
   * common-type relation of R94 to R98.
   *
   * Refusals, in this order: `depth`, `unknown-tag`, `subject-forbidden`,
   * `model-order` (scan); `level-missing` (reach); `ill-typed`, `impure-guard`
   * (type); `ir-cost`.
   */
  compileExpr(ir: unknown, vars: readonly Param[], role: Role, expected?: Ty): Compiled {
    let scan = newInfo();
    let forbidden: KitError | undefined;
    try {
      scanExpr(ir, 1, role, Number.POSITIVE_INFINITY, this.order, scan);
    } catch (error) {
      if (!(error instanceof KitError) || error.code !== "subject-forbidden") throw error;
      // Where two refusals could hold at once the code earlier in section 11's order wins, and `level-missing`
      // precedes `subject-forbidden` (the v2-fault table): read the tree again, allowing the call, to learn its levels.
      forbidden = error;
      scan = newInfo();
      scanExpr(ir, 1, "body", Number.POSITIVE_INFINITY, this.order, scan);
    }
    const missing = this.missingLevel(scan, vars.map((v) => v.ty));
    if (missing !== undefined) {
      throw new KitError("level-missing", "the expression reaches a construct of a level this lane lacks", { level: missing });
    }
    if (forbidden !== undefined) throw forbidden;
    vars.forEach((v) => checkNames(v.ty, this.table, "ill-typed"));
    const scope = new Map(vars.map((v, slot) => [v.name, { slot, ty: v.ty }]));
    const typed = this.typeExpr(ir, scope, expected === undefined ? undefined : { ty: expected, narrow: false });
    if (typed.cost > this.maxCost) fail("ir-cost", "an expression costs more than max_ir_cost");
    const run = this.compile(typed, false);
    const trace = this.trace(typed);
    this.drain();
    return { ty: typed.ty, cost: typed.cost, sub: typed.sub, run, trace, ops: [...this.closure(scan).ops] };
  }

  /** Does this program's lane have every level of `needed`?  (a suite's or fault's declared `requires`) */
  lacks(needed: Iterable<string>): string | undefined {
    return firstMissing(needed, this.levels);
  }

  /** Run model `name` on `args` as the stand-in for a subject: its nodes are tainted (the subject's answer is the unknown). */
  callModel(name: string, args: Value[]): Value {
    const info = this.models.get(name);
    if (info === undefined || info.body === undefined) throw new Error("the stand-in model is not live");
    const cell = this.runner(info, true);
    this.drain();
    return cell.run(args.slice());
  }

  /** Build every model body that a compile asked for, one at a time and iteratively. */
  private drain(): void {
    for (let job = this.pending.pop(); job !== undefined; job = this.pending.pop()) job();
  }

  // -------------------------------------------------------------------------
  // Step 3: typing
  // -------------------------------------------------------------------------

  /**
   * Type a node.  `inter` is true only for the first operand of `unwrap_or`: the **intermediate use** of an `Option`
   * (Appendix A.1, [D23]) needs no `ir.data`.  An `Option` anywhere else is the real type of `ir.data`, a level this lane
   * lacks, and is `level-missing` here; `expected` is a fault row's declared type for this position, if any.
   */
  private typeExpr(ir: unknown, scope: Scope, expected?: Expect, inter = false): T {
    const typed = this.typeNode(ir, scope, expected);
    if (typed.ty.k === "Option" && !inter) {
      throw new KitError("level-missing", "an Option that is not an unwrap_or operand needs ir.data", { level: "ir.data" });
    }
    return typed;
  }

  private typeNode(ir: unknown, scope: Scope, expected?: Expect): T {
    if (!isNodeArray(ir)) return fail("ill-typed", "an IR node is an array that starts with a tag");
    const tag = ir[0] as string;
    const sized = (count: number): void => {
      if (ir.length !== count + 1) fail("ill-typed", `${tag} must carry ${count} operand${count === 1 ? "" : "s"}`);
    };
    const kidsOf = (from: number, count: number): T[] => {
      sized(count);
      return ir.slice(from, from + count).map((child) => this.typeExpr(child, scope));
    };
    const make = (ty: Ty, kids: readonly T[], extra: Partial<T> = {}): T => {
      const sub = tag === "subject_op" || kids.some((k) => k.sub);
      let cost = 1;
      for (const k of kids) cost += k.cost;
      return { tag, ty, kids, sub, cost, ...extra };
    };

    switch (tag) {
      case "lit": {
        sized(1);
        const body = ir[1];
        if (body === true || body === false) return { tag, ty: BOOL, kids: [], sub: false, cost: 1, value: body };
        if (typeof body !== "object" || body === null || Array.isArray(body)) return fail("ill-typed", "a literal is a Bool or {type,value}");
        const record = body as Record<string, unknown>;
        if (Object.keys(record).length !== 2 || typeof record["type"] !== "string") return fail("ill-typed", "a literal object has type and value");
        const ty = parseType(record["type"], { allowInt: true, code: "ill-typed" });
        if (ty.k === "Bool") return fail("ill-typed", "a Bool literal is the bare form");
        if (ty.k === "Bytes") {
          if (ty.n !== null) fail("ill-typed", "the written type of a bytes literal is Bytes");
          const data = typeof record["value"] === "string" ? fromHex(record["value"]) : undefined;
          if (data === undefined || data.length > 4096) return fail("ill-typed", "a bytes literal is lowercase hex of at most 4096 bytes");
          // R67: a bytes literal in a position whose declared type is plain `Bytes` has the type `Bytes`; anywhere else `Bytes<len>`.
          const plain = expected !== undefined && expected.ty.k === "Bytes" && expected.ty.n === null;
          return { tag, ty: plain ? BYTES : { k: "Bytes", n: data.length }, kids: [], sub: false, cost: 1, value: data };
        }
        if (ty.k === "U" || ty.k === "I" || ty.k === "Nat" || ty.k === "Int") {
          const number = parseDecimal(record["value"]);
          if (number === undefined) return fail("ill-typed", "an integer literal is canonical decimal text");
          if (ty.k === "Nat" && (number < 0n || number >= 1n << 64n)) fail("ill-typed", "a Nat literal is in 0..=2^64-1");
          if (ty.k === "U" || ty.k === "I") {
            const [low, high] = fixedRange(ty.k, ty.w);
            if (number < low || number > high) fail("ill-typed", "a literal is outside its type");
          }
          return { tag, ty, kids: [], sub: false, cost: 1, value: isSmall(ty) ? Number(number) : number };
        }
        return fail("ill-typed", "a literal of this type has no literal form");
      }
      case "var": {
        sized(1);
        const found = typeof ir[1] === "string" ? scope.get(ir[1]) : undefined;
        if (found === undefined) return fail("ill-typed", "a var names nothing in scope");
        if (found.ty === UNDETERMINED) return fail("ill-typed", "a bind of an undetermined payload is used");
        return { tag, ty: found.ty, kids: [], sub: false, cost: 1, slot: found.slot };
      }
      case "err": {
        sized(1);
        const name = ir[1];
        if (typeof name !== "string") return fail("ill-typed", "err must name an error case as text");
        const owners = [...this.table.errors].filter(([, cases]) => cases.includes(name)).map(([type]) => type);
        if (owners.length === 0) return fail("ill-typed", "no error type declares this case");
        const ty: Ty = owners.length === 1 ? { k: "Result", ok: null, err: owners[0] as string } : { k: "Result", ok: null, err: null, cands: owners };
        return { tag, ty, kids: [], sub: false, cost: 1, name };
      }
      case "ok": {
        sized(1);
        // R98: the payload of an `ok` is compared with the declared payload type of its position.
        const want = expected !== undefined && expected.ty.k === "Result" && expected.ty.ok !== null ? { ty: expected.ty.ok, narrow: expected.narrow } : undefined;
        const payload = this.typeExpr(ir[1], scope, want);
        if (want !== undefined && faultCommon(payload.ty, want.ty, want.narrow) === null) fail("ill-typed", "the payload of an ok shares no type with the declared one");
        return make({ k: "Result", ok: payload.ty, err: null }, [payload]);
      }
      case "is_err": {
        const [operand] = kidsOf(1, 1) as [T];
        if (operand.ty.k !== "Result") fail("ill-typed", "is_err inspects a Result value");
        return make(BOOL, [operand]);
      }
      case "len": {
        const [operand] = kidsOf(1, 1) as [T];
        if (operand.ty.k !== "Bytes") fail("ill-typed", "len measures a Bytes value");
        return make(NAT, [operand]);
      }
      case "concat": {
        const kids = kidsOf(1, 2);
        const [a, b] = kids as [T, T];
        if (a.ty.k !== "Bytes" || b.ty.k !== "Bytes") fail("ill-typed", "concat joins two Bytes values");
        const widths = typeof a.ty.n === "number" && typeof b.ty.n === "number" ? a.ty.n + b.ty.n : null;
        return make(widths === null ? BYTES : { k: "Bytes", n: widths }, kids);
      }
      case "take":
      case "drop": {
        const kids = kidsOf(1, 2);
        const [bytes, count] = kids as [T, T];
        if (bytes.ty.k !== "Bytes") fail("ill-typed", `${tag} cuts a Bytes value`);
        if (!isExact(count.ty)) fail("ill-typed", `a ${tag} count is a Nat or an Int`);
        return make(BYTES, kids);
      }
      case "not": {
        const [operand] = kidsOf(1, 1) as [T];
        if (operand.ty.k !== "Bool") fail("ill-typed", "the operand of not must be a Bool");
        return make(BOOL, [operand]);
      }
      case "and":
      case "or":
      case "implies": {
        const kids = kidsOf(1, 2);
        const [left, right] = kids as [T, T];
        if (left.ty.k !== "Bool" || right.ty.k !== "Bool") fail("ill-typed", `${tag} combines two Bool operands`);
        // Section 4.4: the left operand guards the right one.
        if (left.sub && right.sub) fail("impure-guard", "a subject call under a guard that contains a subject call");
        return make(BOOL, kids);
      }
      case "if": {
        sized(3);
        const cond = this.typeExpr(ir[1], scope);
        const then = this.typeExpr(ir[2], scope, expected);
        const otherwise = this.typeExpr(ir[3], scope, expected);
        const kids = [cond, then, otherwise];
        if (cond.ty.k !== "Bool") fail("ill-typed", "an if condition is a Bool");
        if (cond.sub && (then.sub || otherwise.sub)) fail("impure-guard", "a subject call under a guard that contains a subject call");
        const common = this.branchType([then.ty, otherwise.ty], expected);
        if (common === null) return fail("ill-typed", "the two arms of an if share no type");
        // Worst path: 1 + cond + max(then, else) (a short circuit is never credited, but a branch not taken is not charged).
        return { tag, ty: common, kids, sub: kids.some((k) => k.sub), cost: 1 + cond.cost + Math.max(then.cost, otherwise.cost) };
      }
      case "eq":
      case "ne": {
        const kids = kidsOf(1, 2);
        const [a, b] = kids as [T, T];
        const common = unify(a.ty, b.ty);
        if (common === null) return fail("ill-typed", "the sides of an equality share no type");
        if (!hasEquality(common)) fail("ill-typed", "this type has no equality at this level");
        return make(BOOL, kids);
      }
      case "lt":
      case "gt":
      case "le":
      case "ge": {
        const kids = kidsOf(1, 2);
        const [a, b] = kids as [T, T];
        if (!isInteger(a.ty) || !isInteger(b.ty)) fail("ill-typed", "an ordering compares integers");
        if (unify(a.ty, b.ty) === null) fail("ill-typed", "the sides of a comparison share no type");
        return make(BOOL, kids);
      }
      case "add":
      case "sub":
      case "mul": {
        const kids = kidsOf(1, 2);
        const [a, b] = kids as [T, T];
        if (isFixed(a.ty) && isFixed(b.ty)) {
          if (!sameType(a.ty, b.ty)) fail("ill-typed", "a mixed fixed-width pair");
          return make(a.ty, kids);
        }
        if (isExact(a.ty) && isExact(b.ty)) return make(INT, kids);
        return fail("ill-typed", "arithmetic needs two integers of the same kind");
      }
      case "neg": {
        const [operand] = kidsOf(1, 1) as [T];
        if (isFixed(operand.ty)) return make(operand.ty, [operand]);
        if (isExact(operand.ty)) return make(INT, [operand]);
        return fail("ill-typed", "neg needs an integer");
      }
      case "model_call": {
        if (ir.length !== 3 || typeof ir[1] !== "string" || !Array.isArray(ir[2])) return fail("ill-typed", "model_call is a model name and an argument list");
        const info = this.models.get(ir[1]);
        if (info === undefined || info.body === undefined || info.ty === undefined) return fail("model-order", "a model_call names a model that is not live");
        const args = (ir[2] as unknown[]).map((child) => this.typeExpr(child, scope));
        if (args.length !== info.params.length) fail("ill-typed", "a model_call has the wrong number of arguments");
        args.forEach((arg, i) => {
          if (!flows(arg.ty, (info.params[i] as Param).ty)) fail("ill-typed", "a model_call argument does not fit its parameter");
        });
        const node = make(info.ty, args, { name: ir[1] });
        return { ...node, cost: node.cost + info.cost };
      }
      case "subject_op": {
        if (ir.length < 3 || ir.length > 4 || typeof ir[1] !== "string" || !Array.isArray(ir[2])) return fail("ill-typed", "subject_op is an op name, an argument list and optionally widths");
        const op = this.ops.get(ir[1]);
        if (op === undefined) return fail("ill-typed", "a subject_op names an op that is missing from ops");
        let widths: number[] = [];
        if (op.widthParams.length > 0) {
          const list = ir[3];
          if (!Array.isArray(list) || list.length !== op.widthParams.length) return fail("ill-typed", "the widths do not match the op's width_params");
          for (const w of list) {
            if (typeof w !== "number" || !Number.isInteger(w) || w < 0 || w > 4096) return fail("ill-typed", "a static width is an integer in 0..=4096");
          }
          widths = list as number[];
        } else if (ir.length === 4) {
          return fail("ill-typed", "a widths list on an op without width_params");
        }
        const args = (ir[2] as unknown[]).map((child) => this.typeExpr(child, scope));
        if (args.length !== op.params.length) fail("ill-typed", "a subject_op has the wrong number of arguments");
        args.forEach((arg, i) => {
          const wanted = substituteWidths((op.params[i] as Param).ty, op.widthParams, widths);
          if (!flows(arg.ty, wanted)) fail("ill-typed", "a subject_op argument does not fit its parameter");
        });
        return make(substituteWidths(op.ret, op.widthParams, widths), args, { name: ir[1] });
      }

      // ---- ir.fixed (Appendix A.1) ------------------------------------------------------------------------------------
      case "bit_and":
      case "bit_or":
      case "bit_xor": {
        const kids = kidsOf(1, 2);
        const [a, b] = kids as [T, T];
        if (!isFixed(a.ty) || !sameType(a.ty, b.ty)) fail("ill-typed", `${tag} needs two fixed-width operands of one type`);
        return make(a.ty, kids);
      }
      case "bit_not": {
        const [a] = kidsOf(1, 1) as [T];
        if (!isFixed(a.ty)) fail("ill-typed", "bit_not needs a fixed-width operand");
        return make(a.ty, [a]);
      }
      case "shl":
      case "shr":
      case "sar":
      case "rotl":
      case "rotr": {
        const kids = kidsOf(1, 2);
        const [a, n] = kids as [T, T];
        if (!isFixed(a.ty)) fail("ill-typed", `${tag} needs a fixed-width operand`);
        if (tag === "sar" && a.ty.k !== "I") fail("ill-typed", "sar needs a signed operand");
        // The count is an exact `Nat`: not `Int`, not a fixed-width type (a signed count would need a sign rule).
        if (n.ty.k !== "Nat") fail("ill-typed", `the count of ${tag} is a Nat`);
        return make(a.ty, kids);
      }
      case "popcount":
      case "clz":
      case "ctz": {
        const [a] = kidsOf(1, 1) as [T];
        if (!isFixed(a.ty)) fail("ill-typed", `${tag} needs a fixed-width operand`);
        return make(NAT, [a]);
      }
      case "bit": {
        const kids = kidsOf(1, 2);
        const [a, i] = kids as [T, T];
        if (!isFixed(a.ty) || i.ty.k !== "Nat") fail("ill-typed", "bit takes a fixed-width operand and a Nat index");
        return make(BOOL, kids);
      }
      case "widen":
      case "wrap":
      case "try": {
        sized(2);
        const to = staticFixed(ir[1]);
        const operand = this.typeExpr(ir[2], scope);
        if (tag === "widen") {
          // Value preserved: the operand's whole range must lie inside the target's.
          if (!isFixed(operand.ty)) fail("ill-typed", "widen takes a fixed-width operand");
          const [lo, hi] = fixedRange(operand.ty.k, operand.ty.w);
          const [tlo, thi] = fixedRange(to.k, to.w);
          if (lo < tlo || hi > thi) fail("ill-typed", "widen needs a range inside the target");
        } else if (!isFixed(operand.ty) && !isExact(operand.ty)) {
          fail("ill-typed", `${tag} takes a fixed-width, Int or Nat operand`);
        }
        return make(tag === "try" ? { k: "Option", of: to as Ty } : (to as Ty), [operand], { target: to });
      }
      case "to_int":
      case "to_nat": {
        const [a] = kidsOf(1, 1) as [T];
        if (!isFixed(a.ty) || (tag === "to_nat" && a.ty.k !== "U")) fail("ill-typed", tag === "to_nat" ? "to_nat takes an unsigned operand" : "to_int takes a fixed-width operand");
        return make(tag === "to_int" ? INT : NAT, [a]);
      }
      case "max_value":
      case "min_value": {
        sized(1);
        const to = staticFixed(ir[1]);
        // Not folded to a `lit` (section 10, unoptimised lowering): a node of its own, cost 1.
        return { tag, ty: to as Ty, kids: [], sub: false, cost: 1, target: to };
      }
      case "checked_add":
      case "checked_sub":
      case "checked_mul": {
        const kids = kidsOf(1, 2);
        const [a, b] = kids as [T, T];
        if (!isFixed(a.ty) || !sameType(a.ty, b.ty)) fail("ill-typed", `${tag} needs two fixed-width operands of one type`);
        return make({ k: "Option", of: a.ty }, kids);
      }
      case "div":
      case "rem": {
        const kids = kidsOf(1, 2);
        const [a, b] = kids as [T, T];
        // Normative [D24, R7]: unsigned pairs give `Option<U>`, `Int`/`Nat` pairs `Option<Int>`; a signed pair is refused.
        if (isFixed(a.ty) && isFixed(b.ty)) {
          if (a.ty.k !== "U" || !sameType(a.ty, b.ty)) fail("ill-typed", `${tag} takes unsigned operands of one type (signed ones use ${tag}_trunc)`);
          return make({ k: "Option", of: a.ty }, kids);
        }
        if (isExact(a.ty) && isExact(b.ty)) return make({ k: "Option", of: INT }, kids);
        return fail("ill-typed", `${tag} takes two unsigned operands of one type or two Int/Nat operands`);
      }
      case "div_trunc":
      case "rem_trunc": {
        const kids = kidsOf(1, 2);
        const [a, b] = kids as [T, T];
        if (!isFixed(a.ty) || a.ty.k !== "I" || !sameType(a.ty, b.ty)) fail("ill-typed", `${tag} takes two signed operands of one type`);
        return make({ k: "Option", of: a.ty }, kids);
      }
      case "imin":
      case "imax": {
        const kids = kidsOf(1, 2);
        if (!kids.every((k) => isExact(k.ty))) fail("ill-typed", `${tag} takes Int or Nat operands`);
        return make(INT, kids);
      }
      case "be_bytes":
      case "le_bytes": {
        const [a] = kidsOf(1, 1) as [T];
        if (!isFixed(a.ty)) fail("ill-typed", `${tag} takes a fixed-width operand`);
        return make({ k: "Bytes", n: a.ty.w / 8 }, [a]);
      }
      case "from_be":
      case "from_le": {
        sized(2);
        const to = staticFixed(ir[1]);
        const operand = this.typeExpr(ir[2], scope);
        // The static width must be exactly `w/8` (R51's static-width rule): a plain `Bytes` is refused by every tool.
        if (operand.ty.k !== "Bytes" || operand.ty.n !== to.w / 8) fail("ill-typed", `${tag} needs a Bytes<${to.w / 8}> operand`);
        return make(to as Ty, [operand], { target: to });
      }
      case "at": {
        const kids = kidsOf(1, 2);
        const [bytes, index] = kids as [T, T];
        if (bytes.ty.k !== "Bytes" || !isExact(index.ty)) fail("ill-typed", "at takes a Bytes value and a Nat or Int index");
        return make({ k: "Option", of: { k: "U", w: 8 } }, kids);
      }
      case "unwrap_or": {
        sized(2);
        // The one consumer of an `Option` at `ir.fixed`: the producer is the first operand, directly [D23].
        const option = this.typeExpr(ir[1], scope, undefined, true);
        const fallback = this.typeExpr(ir[2], scope);
        if (option.ty.k !== "Option" || option.ty.of === null) fail("ill-typed", "unwrap_or takes an Option and a default");
        const payload = (option.ty as Extract<Ty, { k: "Option" }>).of as Ty;
        if (!flows(fallback.ty, payload)) fail("ill-typed", "the default of unwrap_or does not fit the payload type");
        return make(payload, [option, fallback]);
      }

      // ---- ir.tuple (Appendix A.1) ------------------------------------------------------------------------------------
      case "tuple": {
        sized(1);
        const list = ir[1];
        if (!Array.isArray(list) || list.length < 2 || list.length > 8) return fail("ill-typed", "a tuple literal has 2 to 8 components");
        const want = expected !== undefined && expected.ty.k === "Tuple" && expected.ty.of.length === list.length ? expected.ty.of : undefined;
        const kids = list.map((child, i) => {
          // R98: component `i` is compared with component `i` of the declared tuple, under the narrower relation of R96.
          const hint = want === undefined ? undefined : { ty: want[i] as Ty, narrow: true };
          const kid = this.typeExpr(child, scope, hint);
          if (hint !== undefined && faultCommon(kid.ty, hint.ty, true) === null) fail("ill-typed", "a tuple component shares no type with the declared one");
          return kid;
        });
        return make({ k: "Tuple", of: kids.map((k) => k.ty) }, kids);
      }
      case "project": {
        sized(2);
        const operand = this.typeExpr(ir[1], scope);
        const at = ir[2];
        if (operand.ty.k !== "Tuple") fail("ill-typed", "project takes a tuple");
        const parts = (operand.ty as Extract<Ty, { k: "Tuple" }>).of;
        if (typeof at !== "number" || !Number.isInteger(at) || at < 0 || at >= parts.length) fail("ill-typed", "a project index is a JSON integer below the arity");
        return make(parts[at as number] as Ty, [operand], { index: at as number });
      }
      case "match":
        return this.typeMatch(ir, scope, expected);
      default:
        // A tag of a level this lane lacks was refused by the reach step; a tag of no level by the scan.
        return fail("level-missing", "a construct this lane does not implement");
    }
  }

  /** The type of the branches of an `if` or the arms of a `match`: R51's `unify`, or, in a fault row, R94 to R98 against the declared type. */
  private branchType(types: readonly Ty[], expected: Expect | undefined): Ty | null {
    let acc: Ty | null = null;
    for (const ty of types) {
      if (expected !== undefined && faultCommon(ty, expected.ty, expected.narrow) === null) return null;
      acc = acc === null ? ty : expected === undefined ? unify(acc, ty) : faultCommon(acc, ty, expected.narrow);
      if (acc === null) return null;
    }
    return acc;
  }

  /** The case names of the error type of `ty`, in declaration order, or `null` when the type does not say which it is. */
  private errorCases(ty: Extract<Ty, { k: "Result" }>): readonly string[] | null {
    if (ty.err !== null) return this.table.errors.get(ty.err) ?? null;
    if (ty.cands !== undefined && ty.cands.length === 1) return this.table.errors.get(ty.cands[0] as string) ?? null;
    return null;
  }

  /**
   * Type a pattern against the type of the value it meets (Appendix A.1) and record what it binds in `binds`.
   * A data pattern (`some none ctor bool`) is `ir.data`; the reach step has normally said so already, and a lane without
   * the level never gets here with one.  Every other mistake is `ill-typed` (a corpus error at source).
   */
  private typePattern(pat: unknown, ty: Ty, binds: Map<string, { slot: number; ty: Ty }>, seen: Set<string>, depth: number): TPat {
    // The outermost pattern is level 1: a pattern nests at most 8 deep (VER04j section 4.3), so a ninth level is refused.
    if (depth > 8) return fail("depth", "a pattern nests at most 8 deep");
    if (!Array.isArray(pat) || typeof pat[0] !== "string") return fail("ill-typed", "a pattern is an array that starts with a pattern word");
    const head = pat[0];
    switch (head) {
      case "wild":
        if (pat.length !== 1) fail("ill-typed", "wild has no operand");
        return { k: "wild" };
      case "bind": {
        const name = pat[1];
        if (pat.length !== 2 || typeof name !== "string" || !isName(name)) return fail("ill-typed", "bind names a variable");
        // No shadowing: not a binder, a parameter, `result`, an outer `bind`, or the same name twice in one pattern.
        if (seen.has(name) || binds.has(name) || name === "result") fail("ill-typed", "a bind name shadows another name or repeats");
        seen.add(name);
        let slot = 0;
        for (const entry of binds.values()) slot = Math.max(slot, entry.slot + 1);
        binds.set(name, { slot, ty });
        return { k: "bind", slot };
      }
      case "ok": {
        if (pat.length !== 2 || ty.k !== "Result") return fail("ill-typed", "ok matches a Result");
        // An undetermined payload (a bare `err`) can only be bound or ignored; a body that *uses* the bind is ill-typed (see `var`).
        return { k: "ok", p: this.typePattern(pat[1], ty.ok ?? UNDETERMINED, binds, seen, depth + 1) };
      }
      case "err": {
        if (pat.length !== 2 || typeof pat[1] !== "string" || ty.k !== "Result") return fail("ill-typed", "err matches a Result and names a case");
        const cases = this.errorCases(ty);
        if (cases !== null ? !cases.includes(pat[1]) : ![...this.table.errors.values()].some((list) => list.includes(pat[1] as string))) {
          fail("ill-typed", "the case is not one of the error type");
        }
        return { k: "err", c: pat[1] };
      }
      case "err_any":
        if (pat.length !== 1 || ty.k !== "Result") return fail("ill-typed", "err_any matches a Result");
        return { k: "err", c: null };
      case "tuple": {
        const list = pat[1];
        if (pat.length !== 2 || !Array.isArray(list) || ty.k !== "Tuple" || list.length !== ty.of.length) {
          return fail("ill-typed", "a tuple pattern has the arity of the tuple it meets");
        }
        const parts = ty.of;
        return { k: "tuple", ps: list.map((item, i) => this.typePattern(item, parts[i] as Ty, binds, seen, depth + 1)) };
      }
      case "some":
      case "none":
      case "ctor":
      case "bool":
        throw new KitError("level-missing", "a data pattern needs ir.data", { level: "ir.data" });
      default:
        return fail("ill-typed", "a pattern word this document does not define");
    }
  }

  /**
   * Is the pattern vector `vec` *useful* against the rows `rows` (Maranget's usefulness check): is there a value that `vec`
   * matches and no row does?  An arm no value can reach is an arm that is not useful against the arms before it; a `match`
   * that can fall through is one where a wildcard is useful against all of its arms.  `tys` are the types of the columns,
   * `null` for a payload that is undetermined (only a wildcard can stand there).
   */
  /**
   * One usefulness question, spending the `match`'s one budget: `budget.left` is loaded into the counter and taken back, so an
   * inner `match` typed in between cannot spend (or refill) the outer one's.
   */
  private usefulWithin(budget: { left: number }, rows: readonly (readonly TPat[])[], vec: readonly TPat[], tys: readonly (Ty | null)[]): boolean {
    this.matchSteps = budget.left;
    try {
      return this.useful(rows, vec, tys);
    } finally {
      budget.left = this.matchSteps;
    }
  }

  private useful(rows: readonly (readonly TPat[])[], vec: readonly TPat[], tys: readonly (Ty | null)[]): boolean {
    // A work budget (section 14: a corpus is hostile): the check is exponential in the worst case, and a match that needs more
    // than MATCH_BUDGET steps (one budget per `match`; a step costs 1 plus the rows it looks at) is refused, as a corpus error, rather than left to run.
    this.matchSteps -= 1 + rows.length;
    if (this.matchSteps < 0) fail("ill-typed", "a match is too costly to check");
    if (vec.length === 0) return rows.length === 0;
    const head = vec[0] as TPat;
    const ty = tys[0] ?? null;
    const restTys = tys.slice(1);
    const restVec = vec.slice(1);
    const isWild = (p: TPat): boolean => p.k === "wild" || p.k === "bind";
    // Specialisations: keep the rows that can match the constructor, replacing the head by its sub-patterns.
    const onOk = (): TPat[][] =>
      rows.flatMap((row) => {
        const first = row[0] as TPat;
        if (first.k === "ok") return [[first.p, ...row.slice(1)]];
        return isWild(first) ? [[{ k: "wild" } as TPat, ...row.slice(1)]] : [];
      });
    const onErr = (c: string): TPat[][] =>
      rows.flatMap((row) => {
        const first = row[0] as TPat;
        return isWild(first) || (first.k === "err" && (first.c === null || first.c === c)) ? [row.slice(1)] : [];
      });
    const onTuple = (n: number): TPat[][] =>
      rows.flatMap((row) => {
        const first = row[0] as TPat;
        if (first.k === "tuple") return [[...first.ps, ...row.slice(1)]];
        return isWild(first) ? [[...Array.from({ length: n }, () => ({ k: "wild" }) as TPat), ...row.slice(1)]] : [];
      });
    const resultTy = ty !== null && ty.k === "Result" ? ty : undefined;
    const tupleTy = ty !== null && ty.k === "Tuple" ? ty : undefined;
    const okTy = resultTy === undefined ? null : resultTy.ok;
    const cases = resultTy === undefined ? null : this.errorCases(resultTy);
    // The error cases worth trying: those some row names, plus one representative ("", never a case name) for all the others, which
    // `err_any` and a wildcard cover alike.  Trying every declared case (up to 64 per type, a tuple of 8 such types) would make a
    // hostile `match` exponential for nothing: two cases no row tells apart are one case.
    const named = new Set(rows.flatMap((row) => ((row[0] as TPat).k === "err" && (row[0] as { c: string | null }).c !== null ? [(row[0] as { c: string }).c] : [])));
    const errNames: readonly string[] = cases !== null && cases.every((name) => named.has(name)) ? [...named] : [...named, ""];

    switch (head.k) {
      case "ok":
        return this.useful(onOk(), [head.p, ...restVec], [okTy, ...restTys]);
      case "err":
        if (head.c === null) return errNames.some((name) => this.useful(onErr(name), restVec, restTys));
        return this.useful(onErr(head.c), restVec, restTys);
      case "tuple": {
        const parts = tupleTy === undefined ? head.ps.map(() => null) : tupleTy.of;
        return this.useful(onTuple(head.ps.length), [...head.ps, ...restVec], [...parts, ...restTys]);
      }
      default: {
        // A wildcard: useful if the column's constructors are not all present and the default matrix says so, or if some constructor is.
        const firsts = rows.map((row) => row[0] as TPat);
        if (resultTy !== undefined) {
          const hasOk = firsts.some((p) => p.k === "ok");
          const hasAny = firsts.some((p) => p.k === "err" && p.c === null);
          const present = new Set(firsts.flatMap((p) => (p.k === "err" && p.c !== null ? [p.c] : [])));
          const complete = hasOk && (hasAny || (cases !== null && cases.every((name) => present.has(name))));
          if (complete) {
            return (
              this.useful(onOk(), [{ k: "wild" }, ...restVec], [okTy, ...restTys]) ||
              errNames.some((name) => this.useful(onErr(name), restVec, restTys))
            );
          }
        } else if (tupleTy !== undefined && firsts.some((p) => p.k === "tuple")) {
          const n = tupleTy.of.length;
          return this.useful(onTuple(n), [...Array.from({ length: n }, () => ({ k: "wild" }) as TPat), ...restVec], [...tupleTy.of, ...restTys]);
        }
        const defaults = rows.filter((row) => isWild(row[0] as TPat)).map((row) => row.slice(1));
        return this.useful(defaults, restVec, restTys);
      }
    }
  }

  /**
   * A `match` on a `Result` (`ir.tuple`, R76).  A scrutinee of any other type, or a data pattern, is `ir.data`: this lane
   * types the scrutinee before it accepts the node and refuses the others as `level-missing`.  The patterns are tried in
   * order and exactly one arm runs; a `match` that can fall through, an arm no value can reach, a repeated `bind`, and a
   * `bind` that shadows are corpus errors (`ill-typed` here), and so is an arm body of another type than the others.
   */
  private typeMatch(ir: unknown[], scope: Scope, expected: Expect | undefined): T {
    if (ir.length !== 3 || !Array.isArray(ir[2])) return fail("ill-typed", "match is a scrutinee and a list of arms");
    const scrutinee = this.typeExpr(ir[1], scope);
    if (scrutinee.ty.k !== "Result") throw new KitError("level-missing", "a match on anything but a Result needs ir.data", { level: "ir.data" });
    const armsRaw = ir[2] as unknown[];
    if (armsRaw.length < 1 || armsRaw.length > 256) return fail("ill-typed", "a match has 1 to 256 arms");
    const pats: TPat[] = [];
    const bodies: T[] = [];
    const budget = { left: MATCH_BUDGET };
    for (const arm of armsRaw) {
      if (!Array.isArray(arm) || arm.length !== 2) return fail("ill-typed", "an arm is [pattern, body]");
      const binds = new Map(scope);
      const pat = this.typePattern(arm[0], scrutinee.ty, binds, new Set(), 1);
      // An arm no value can reach: every value it matches is matched by an earlier arm.
      if (!this.usefulWithin(budget, pats.map((p) => [p]), [pat], [scrutinee.ty])) fail("ill-typed", "an arm no value can reach");
      pats.push(pat);
      const body = this.typeExpr(arm[1], binds, expected);
      if (expected !== undefined && faultCommon(body.ty, expected.ty, expected.narrow) === null) fail("ill-typed", "an arm shares no type with the declared one");
      bodies.push(body);
    }
    // Exhaustive: a wildcard must not be useful against the arms.
    if (this.usefulWithin(budget, pats.map((p) => [p]), [{ k: "wild" }], [scrutinee.ty])) fail("ill-typed", "a match that can fall through");
    const common = this.branchType(bodies.map((b) => b.ty), expected);
    if (common === null) return fail("ill-typed", "the arms of a match share no type");
    if (scrutinee.sub && bodies.some((b) => b.sub)) fail("impure-guard", "a subject call under a guard that contains a subject call");
    const kids = [scrutinee, ...bodies];
    // Cost: 1 + the scrutinee + the largest arm (section 4.2 rule 6).
    const cost = 1 + scrutinee.cost + Math.max(...bodies.map((b) => b.cost));
    return { tag: "match", ty: common, kids, sub: kids.some((k) => k.sub), cost, pats };
  }

  // -------------------------------------------------------------------------
  // Step 4: compile
  // -------------------------------------------------------------------------

  private runner(info: ModelInfo, tainted: boolean): Cell {
    const slot = tainted ? 1 : 0;
    let made = info.runners[slot];
    if (made === undefined) {
      const cell: Cell = {
        run: () => {
          throw new Error("a model body was called before it was compiled");
        },
      };
      made = cell;
      info.runners[slot] = cell;
      this.pending.push(() => {
        cell.run = this.compile(info.body as T, tainted);
      });
    }
    return made;
  }

  /**
   * Compile a typed node.  `ctx` is the "tainted" context of R71: true when this node
   * is evaluated in a place whose overflow is a case failure.
   */
  private compile(t: T, ctx: boolean): Closure {
    const own = ctx || t.sub;
    const overflow = (): never => {
      throw new KitError("ir-overflow", "an exact integer or a bytes value left its bound", { overflowClass: own ? "case" : "corpus" });
    };
    const kid = (i: number, context = ctx): Closure => this.compile(t.kids[i] as T, context);

    switch (t.tag) {
      case "lit": {
        const value = t.value as Value;
        return () => value;
      }
      case "var": {
        const slot = t.slot as number;
        return (env) => env[slot] as Value;
      }
      case "err": {
        const value = { err: t.name as string };
        return () => value;
      }
      case "ok": {
        const a = kid(0);
        return (env) => ({ ok: a(env) });
      }
      case "is_err": {
        const a = kid(0);
        return (env) => (a(env) as { err?: string }).err !== undefined;
      }
      case "len": {
        const a = kid(0);
        return (env) => BigInt((a(env) as Uint8Array).length);
      }
      case "not": {
        const a = kid(0);
        return (env) => a(env) !== true;
      }
      case "and": {
        const a = kid(0);
        const b = kid(1, ctx || (t.kids[0] as T).sub);
        return (env) => a(env) === true && b(env) === true;
      }
      case "or": {
        const a = kid(0);
        const b = kid(1, ctx || (t.kids[0] as T).sub);
        return (env) => a(env) === true || b(env) === true;
      }
      case "implies": {
        const a = kid(0);
        const b = kid(1, ctx || (t.kids[0] as T).sub);
        return (env) => a(env) !== true || b(env) === true;
      }
      case "if": {
        const c = kid(0);
        const guarded = ctx || (t.kids[0] as T).sub;
        const a = kid(1, guarded);
        const b = kid(2, guarded);
        return (env) => (c(env) === true ? a(env) : b(env));
      }
      case "eq":
      case "ne": {
        const a = kid(0);
        const b = kid(1);
        const common = unify((t.kids[0] as T).ty, (t.kids[1] as T).ty) as Ty;
        const negate = t.tag === "ne";
        if (common.k === "Bytes" || common.k === "Result" || common.k === "Tuple") {
          return negate ? (env) => !valuesEqual(a(env), b(env)) : (env) => valuesEqual(a(env), b(env));
        }
        return negate ? (env) => a(env) !== b(env) : (env) => a(env) === b(env);
      }
      case "lt":
      case "gt":
      case "le":
      case "ge": {
        const a = kid(0);
        const b = kid(1);
        // Numbers and bigints both order by mathematical value; the static type of an operand pair is one host form.
        switch (t.tag) {
          case "lt":
            return (env) => (a(env) as number) < (b(env) as number);
          case "gt":
            return (env) => (a(env) as number) > (b(env) as number);
          case "le":
            return (env) => (a(env) as number) <= (b(env) as number);
          default:
            return (env) => (a(env) as number) >= (b(env) as number);
        }
      }
      case "add":
      case "sub":
      case "mul":
        return this.compileArith(t, ctx, overflow);
      case "neg": {
        const a = kid(0);
        const ty = t.ty;
        if (ty.k === "Int") {
          return (env) => {
            const r = -(a(env) as bigint);
            if (r > INT_WINDOW || r < -INT_WINDOW) overflow();
            return r;
          };
        }
        if (isSmall(ty)) {
          const wrap = smallWrap(ty);
          return (env) => wrap(-(a(env) as number));
        }
        const fixed = ty as { k: "U" | "I"; w: number };
        return fixed.k === "U"
          ? (env) => BigInt.asUintN(64, -(a(env) as bigint))
          : (env) => BigInt.asIntN(64, -(a(env) as bigint));
      }
      case "concat": {
        const a = kid(0);
        const b = kid(1);
        return (env) => {
          const left = a(env) as Uint8Array;
          const right = b(env) as Uint8Array;
          // Checked before the allocation: a doubling chain must not build the big value first.
          if (left.length + right.length > MAX_IR_VALUE) overflow();
          const out = new Uint8Array(left.length + right.length);
          out.set(left, 0);
          out.set(right, left.length);
          return out;
        };
      }
      case "take":
      case "drop": {
        const a = kid(0);
        const b = kid(1);
        const isTake = t.tag === "take";
        return (env) => {
          const data = a(env) as Uint8Array;
          const wanted = b(env) as bigint;
          // total: a negative count is 0 and a count past the end clamps.
          const count = wanted <= 0n ? 0 : wanted >= BigInt(data.length) ? data.length : Number(wanted);
          return isTake ? data.slice(0, count) : data.slice(count);
        };
      }
      // ---- ir.fixed ---------------------------------------------------------------------------------------------------
      case "bit_and":
      case "bit_or":
      case "bit_xor":
      case "shl":
      case "shr":
      case "sar":
      case "rotl":
      case "rotr":
      case "bit":
      case "checked_add":
      case "checked_sub":
      case "checked_mul":
      case "div":
      case "rem":
      case "div_trunc":
      case "rem_trunc":
      case "imin":
      case "imax":
      case "at": {
        const a = kid(0);
        const b = kid(1);
        const first = (t.kids[0] as T).ty;
        // `div`, `rem`, `imin`, `imax` on `Nat` and `Int` are over `bigint`s and need no width; `at` takes bytes.
        const f = binaryFixed(t.tag, isFixed(first) ? first : undefined) as (x: unknown, y: unknown) => unknown;
        return (env) => f(a(env), b(env)) as Value;
      }
      case "bit_not":
      case "popcount":
      case "clz":
      case "ctz":
      case "to_int":
      case "to_nat":
      case "be_bytes":
      case "le_bytes":
      case "widen":
      case "wrap":
      case "try": {
        const a = kid(0);
        const first = (t.kids[0] as T).ty;
        // The operand type only sizes the work; `wrap` and `try` take `Int` and `Nat` too, and then the target stands in.
        const f = unaryFixed(t.tag, isFixed(first) ? first : (t.target as Fx), t.target) as (x: unknown) => unknown;
        return (env) => f(a(env)) as Value;
      }
      case "from_be":
      case "from_le": {
        const a = kid(0);
        const f = unaryFixed(t.tag, t.target as Fx, t.target) as (x: unknown) => unknown;
        return (env) => {
          try {
            return f(a(env)) as Value;
          } catch (error) {
            // A runtime length other than w/8 (a faulted value) is a corpus error, never a truncation (Appendix A.1).
            if (error instanceof RangeError) return overflow();
            throw error;
          }
        };
      }
      case "max_value":
      case "min_value": {
        const value = boundValue(t.tag, t.target as Fx);
        return () => value;
      }
      case "unwrap_or": {
        // Both operands are evaluated, the producer first; no short circuit (Appendix A.1).
        const a = kid(0);
        const b = kid(1);
        return (env) => {
          const option = a(env) as unknown;
          const fallback = b(env);
          return (option === undefined ? fallback : option) as Value;
        };
      }
      // ---- ir.tuple ---------------------------------------------------------------------------------------------------
      case "tuple": {
        const parts = t.kids.map((_k, i) => kid(i));
        const count = parts.length;
        return (env) => {
          const out = new Array<unknown>(count);
          for (let i = 0; i < count; i += 1) out[i] = (parts[i] as Closure)(env);
          return out;
        };
      }
      case "project": {
        const a = kid(0);
        const index = t.index as number;
        return (env) => (a(env) as readonly unknown[])[index] as Value;
      }
      case "match": {
        const select = this.armSelector(t, kid(0));
        // R71: an arm lies under the scrutinee as its guard.
        const guarded = ctx || (t.kids[0] as T).sub;
        const bodies = t.kids.slice(1).map((_k, i) => kid(i + 1, guarded));
        return (env) => (bodies[select(env)] as Closure)(env);
      }
      case "model_call": {
        const info = this.models.get(t.name as string) as ModelInfo;
        const args = t.kids.map((_k, i) => kid(i));
        const entered = ctx || t.kids.some((k) => k.sub);
        const cell = this.runner(info, entered);
        const count = args.length;
        return (env) => {
          const frame: Env = new Array<Value>(count);
          for (let i = 0; i < count; i += 1) frame[i] = (args[i] as Closure)(env);
          return cell.run(frame);
        };
      }
      case "subject_op": {
        const args = t.kids.map((_k, i) => kid(i));
        const name = t.name as string;
        const state = this.state;
        return (env) => {
          const values: Value[] = args.map((arg) => arg(env));
          state.calls += 1;
          return state.subject(name, values);
        };
      }
      default:
        throw new KitError("level-missing", "a construct this lane does not implement");
    }
  }

  /**
   * The index of the arm a `match` runs: the scrutinee is evaluated once, the patterns are tried in order, and the first
   * that matches binds its names into the environment slots the typer gave them (`bind` writes `env[slot]`; sibling arms
   * share slots, which is safe because only one runs and an arm's body is finished before anything else reads the slot).
   */
  private armSelector(t: T, scrutinee: Closure): (env: Env) => number {
    const matchers = (t.pats as readonly TPat[]).map(compilePattern);
    const count = matchers.length;
    return (env) => {
      const value = scrutinee(env);
      for (let i = 0; i < count; i += 1) if ((matchers[i] as PatternRun)(value, env)) return i;
      // The typer proved the match exhaustive, so a fall-through is a bug here, never a case outcome.
      throw new Error("a match fell through");
    };
  }

  private compileArith(t: T, ctx: boolean, overflow: () => never): Closure {
    const a = this.compile(t.kids[0] as T, ctx);
    const b = this.compile(t.kids[1] as T, ctx);
    const ty = t.ty;
    const op = t.tag;
    if (ty.k === "Int") {
      // Exact; every arithmetic node on Nat or Int has type Int (R42), bounded by the 2^127 window.
      return (env) => {
        const x = a(env) as bigint;
        const y = b(env) as bigint;
        const r = op === "add" ? x + y : op === "sub" ? x - y : x * y;
        if (r > INT_WINDOW || r < -INT_WINDOW) overflow();
        return r;
      };
    }
    if (isSmall(ty)) {
      const wrap = smallWrap(ty);
      const thirtyTwo = (ty as { w: number }).w === 32;
      if (op === "add") return (env) => wrap((a(env) as number) + (b(env) as number));
      if (op === "sub") return (env) => wrap((a(env) as number) - (b(env) as number));
      // A 32-bit product needs 64 bits, so it goes through Math.imul; narrower products are exact in a double.
      if (thirtyTwo) return (env) => wrap(Math.imul(a(env) as number, b(env) as number));
      return (env) => wrap((a(env) as number) * (b(env) as number));
    }
    const unsigned = (ty as { k: "U" | "I" }).k === "U";
    const norm = unsigned ? (v: bigint): bigint => BigInt.asUintN(64, v) : (v: bigint): bigint => BigInt.asIntN(64, v);
    if (op === "add") return (env) => norm((a(env) as bigint) + (b(env) as bigint));
    if (op === "sub") return (env) => norm((a(env) as bigint) - (b(env) as bigint));
    return (env) => norm((a(env) as bigint) * (b(env) as bigint));
  }

  // -------------------------------------------------------------------------
  // The pure trace (section 4.4)
  // -------------------------------------------------------------------------

  /**
   * The number of `subject_op` nodes on the evaluation path under the pure
   * guards, as a constant when no guard decides it.  The count never depends on
   * what the subject returned: a guard that is pure is evaluated here, and a
   * guard that contains a subject call guards no call (the typer refused it).
   */
  private trace(t: T): { fixed: number } | { dynamic: (env: Env) => number } {
    if (t.tag === "subject_op") {
      const inner = this.sumTrace(t.kids);
      return "fixed" in inner ? { fixed: 1 + inner.fixed } : { dynamic: (env) => 1 + inner.dynamic(env) };
    }
    if (!t.sub) return { fixed: 0 };
    if (t.tag === "if") {
      const cond = this.trace(t.kids[0] as T);
      const left = this.trace(t.kids[1] as T);
      const right = this.trace(t.kids[2] as T);
      const decide = (t.kids[0] as T).sub ? undefined : this.compile(t.kids[0] as T, false);
      if (decide === undefined) {
        // The condition contains a subject call, so no branch calls one (the typer refused it): count the condition.
        return cond;
      }
      if ("fixed" in left && "fixed" in right && left.fixed === right.fixed) return { fixed: left.fixed };
      const l = asFunction(left);
      const r = asFunction(right);
      return { dynamic: (env) => (decide(env) === true ? l(env) : r(env)) };
    }
    if (t.tag === "match") {
      const scrutinee = t.kids[0] as T;
      // A scrutinee that calls the subject guards no call (the typer refused it): its count is the whole count.
      if (scrutinee.sub) return this.trace(scrutinee);
      const traces = t.kids.slice(1).map((arm) => this.trace(arm));
      const fixed = traces.map((tr) => ("fixed" in tr ? tr.fixed : -1));
      if (fixed.every((n) => n >= 0 && n === fixed[0])) return { fixed: fixed[0] as number };
      // The scrutinee is pure, so evaluating it here learns the arm that runs without touching the subject.
      const select = this.armSelector(t, this.compile(scrutinee, false));
      const counts = traces.map(asFunction);
      return { dynamic: (env) => (counts[select(env)] as (e: Env) => number)(env) };
    }
    if (t.tag === "and" || t.tag === "or" || t.tag === "implies") {
      const leftTrace = this.trace(t.kids[0] as T);
      const rightTrace = this.trace(t.kids[1] as T);
      if (!(t.kids[1] as T).sub) return leftTrace;
      const decide = this.compile(t.kids[0] as T, false);
      const runsRight = t.tag === "or" ? (v: Value): boolean => v !== true : (v: Value): boolean => v === true;
      const rightRuns = asFunction(rightTrace);
      const leftCount = asFunction(leftTrace);
      return { dynamic: (env) => leftCount(env) + (runsRight(decide(env)) ? rightRuns(env) : 0) };
    }
    return this.sumTrace(t.kids);
  }

  private sumTrace(kids: readonly T[]): { fixed: number } | { dynamic: (env: Env) => number } {
    const parts = kids.map((k) => this.trace(k));
    if (parts.every((p) => "fixed" in p)) return { fixed: parts.reduce((sum, p) => sum + (p as { fixed: number }).fixed, 0) };
    const fns = parts.map(asFunction);
    return { dynamic: (env) => fns.reduce((sum, f) => sum + f(env), 0) };
  }
}

type PatternRun = (value: unknown, env: Env) => boolean;

/** Turn a typed pattern into a test that also stores what it binds. */
function compilePattern(p: TPat): PatternRun {
  switch (p.k) {
    case "wild":
      return () => true;
    case "bind": {
      const slot = p.slot;
      return (value, env) => {
        env[slot] = value as Value;
        return true;
      };
    }
    case "ok": {
      const inner = compilePattern(p.p);
      return (value, env) => (value as { err?: string }).err === undefined && inner((value as { ok: unknown }).ok, env);
    }
    case "err": {
      const c = p.c;
      return c === null ? (value) => (value as { err?: string }).err !== undefined : (value) => (value as { err?: string }).err === c;
    }
    case "tuple": {
      const inner = p.ps.map(compilePattern);
      return (value, env) => {
        const parts = value as readonly unknown[];
        for (let i = 0; i < inner.length; i += 1) if (!(inner[i] as PatternRun)(parts[i], env)) return false;
        return true;
      };
    }
  }
}

function asFunction(trace: { fixed: number } | { dynamic: (env: Env) => number }): (env: Env) => number {
  if ("fixed" in trace) {
    const n = trace.fixed;
    return () => n;
  }
  return trace.dynamic;
}

/** The static type argument of `widen wrap try max_value min_value from_be from_le`: the name of a fixed-width type, or `ill-typed`. */
function staticFixed(text: unknown): Fx {
  if (typeof text !== "string") return fail("ill-typed", "a static type argument is a type name");
  const ty = parseType(text, { code: "ill-typed" });
  if (!isFixed(ty)) return fail("ill-typed", "a static type argument names a fixed-width type");
  return ty;
}

/** The text of a static type for the kit's `type` field: widths kept, `Bytes<2>` for a two-byte result. */
export function typeText(ty: Ty): string {
  return showType(ty);
}
