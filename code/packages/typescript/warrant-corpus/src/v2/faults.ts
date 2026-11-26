/**
 * # Fault wrappers: a fault is data (VER04r section 8)
 *
 * VER04i made each fault a per-lane wrapper with a name-keyed table.  v2 moves
 * the table into the corpus, where a fault is one row of IR, and a lane
 * implements **three wrappers, once**:
 *
 * | kind      | effect                                                                    |
 * |-----------|---------------------------------------------------------------------------|
 * | `pre`     | rewrites parameters before the call: a `rewrites` list of `{param, expr}`; each `expr` is over **all** parameters *as passed in*, and the rewrites are applied **simultaneously** (one rewrite never sees another's output) |
 * | `post`    | rewrites the result: `expr` over the parameters and `result`              |
 * | `replace` | supplies the result without trusting the call's value (the call is still made and counted) |
 *
 * `result` is the one reserved name for the call's result.  A fault expression
 * uses only tags of its declared level and **never `subject_op`**.
 *
 * ## Typing a fault row (R70, R94)
 *
 * A lane wraps the op once, with no call site, so there are no static widths to
 * substitute: it types the row with each named width **left unstated**, a
 * parameter of type `Bytes<N>` with a named `N` being plain `Bytes` there.  A
 * rewrite's static type and the type of the parameter it rewrites -- and a
 * `post` or `replace` expression's type and the op's result type -- must have a
 * **common type** (`unify`, R51), *not* a flow: a fault may change a length by
 * design (`drop-last-result-byte` has type plain `Bytes` against a `Bytes<32>`
 * result, and plain `Bytes` does not flow to `Bytes<32>`).  A pair with no
 * common type is a corpus error.
 *
 * The check is static.  It says nothing about a faulted *value* that is not a
 * value of the declared type (the 31 bytes a dropped byte leaves of a
 * `Bytes<32>` result): that goes to the claims as it evaluates, which is the
 * point of a fault, and the lane neither pads nor refuses it.
 */

import { KitError } from "./errors.ts";
import { DEFINED_LEVELS } from "./levels.ts";
import { type Compiled, type OpSig, type Param, type Program } from "./ir.ts";
import { eraseNamedWidths, typeLevels, unify } from "./types.ts";
import type { Value } from "./values.ts";

/** A compiled fault: apply it to a call's arguments and result. */
export interface CompiledFault {
  readonly kind: "pre" | "post" | "replace";
  readonly fault: string;
  readonly op: OpSig;
  /** `pre`: every argument after the rewrite, unchanged ones included, in parameter order. */
  rewriteArgs(args: readonly Value[]): Value[];
  /** `post` and `replace`: the value the call is reported to have returned. */
  rewriteResult(args: readonly Value[], result: Value): Value;
}

function refuse(why: string): never {
  throw new KitError("corpus", why);
}

/**
 * Compile one fault row for `op`.  Refusals: `level-missing` (the row's expressions or the op's
 * parameter and result types reach a level this lane lacks, or its declared `requires` names
 * one -- a lane records such a control `not-live`), `subject-forbidden` (a `subject_op` anywhere
 * in it), `ill-typed` (no common type with the parameter or result it rewrites), `corpus`
 * (a malformed row).
 */
export function compileFault(program: Program, raw: unknown, op: OpSig): CompiledFault {
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) return refuse("a fault row is an object");
  const row = raw as Record<string, unknown>;
  const kind = row["kind"];
  if (kind !== "pre" && kind !== "post" && kind !== "replace") return refuse("a fault has kind pre, post or replace");
  const expected = kind === "pre" ? ["fault", "op", "kind", "requires", "rewrites"] : ["fault", "op", "kind", "requires", "expr"];
  const keys = Object.keys(row);
  if (keys.length !== expected.length || !expected.every((key) => Object.hasOwn(row, key))) refuse("a fault row has a missing or unknown key");
  if (typeof row["fault"] !== "string" || row["op"] !== op.name) refuse("a fault names its op");
  const requires = row["requires"];
  if (!Array.isArray(requires) || requires.some((level) => typeof level !== "string" || !DEFINED_LEVELS.has(level))) {
    refuse("requires is a list of levels this document defines");
  }

  // The row's shape, whole, before any level is read (R126: a malformed row is a schema break, never a vector).
  const rewrites: { slot: number; expr: unknown }[] = [];
  if (kind === "pre") {
    const list = row["rewrites"];
    if (!Array.isArray(list) || list.length < 1) return refuse("rewrites is a non-empty list");
    const seen = new Set<string>();
    for (const rewrite of list as unknown[]) {
      if (typeof rewrite !== "object" || rewrite === null || Array.isArray(rewrite)) return refuse("a rewrite is an object");
      const entry = rewrite as Record<string, unknown>;
      if (Object.keys(entry).length !== 2 || typeof entry["param"] !== "string" || !Object.hasOwn(entry, "expr")) return refuse("each rewrite carries exactly param and expr");
      const slot = op.params.findIndex((p) => p.name === entry["param"]);
      if (slot < 0 || seen.has(entry["param"])) return refuse("a rewrite names a parameter once");
      seen.add(entry["param"]);
      rewrites.push({ slot, expr: entry["expr"] });
    }
  }

  // Levels first: the row's declared levels, the op's parameter and result types, then (below) its expressions.
  const levels = new Set<string>(requires as string[]);
  op.params.forEach((p) => typeLevels(p.ty, levels));
  typeLevels(op.ret, levels);
  const lacking = program.lacks(levels);

  const params: Param[] = op.params.map((p) => ({ name: p.name, ty: eraseNamedWidths(p.ty) }));
  if (params.some((p) => p.name === "result")) refuse("an op may not have a parameter named result");
  const ret = eraseNamedWidths(op.ret);

  // A row that reaches a level this lane lacks is not compiled (a lane records its control `not-live`); the level
  // refusal precedes `subject-forbidden`, which only a row inside this lane's levels can raise.
  if (lacking !== undefined) throw new KitError("level-missing", "the fault reaches a level this lane lacks", { level: lacking });
  const withResult: Param[] = [...params, { name: "result", ty: ret }];

  // Every step runs over every expression of the row before the next step starts (R119 (c); section 11's order):
  // the three codes, then `level-missing` of what the expressions reach, then `subject-forbidden`, then the typing.
  const stages = (items: readonly { ir: unknown; vars: readonly Param[] }[]): void => {
    for (const item of items) program.scanTags(item.ir, "fault-reach");
    for (const item of items) program.checkReach(item.ir, item.vars);
    for (const item of items) program.scanTags(item.ir, "fault");
  };

  if (kind === "pre") {
    const compiled: { slot: number; run: Compiled["run"] }[] = [];
    const slots = rewrites.map((entry) => entry.slot);
    stages(rewrites.map((entry) => ({ ir: entry.expr, vars: params })));
    rewrites.forEach((entry) => {
      const typed = program.compileExpr(entry.expr, params, "fault");
      if (unify(typed.ty, (params[entry.slot] as Param).ty) === null) throw new KitError("ill-typed", "a rewrite shares no type with the parameter it replaces");
      compiled.push({ slot: slots[compiled.length] as number, run: typed.run });
    });
    return {
      kind,
      fault: row["fault"],
      op,
      rewriteArgs(args) {
        // Simultaneous: every expression reads the arguments as they were passed in.
        const out = args.slice();
        for (const rewrite of compiled) out[rewrite.slot] = rewrite.run(args.slice());
        return out;
      },
      rewriteResult(_args, result) {
        return result;
      },
    };
  }

  stages([{ ir: row["expr"], vars: withResult }]);
  const typed = program.compileExpr(row["expr"], withResult, "fault");
  if (unify(typed.ty, ret) === null) throw new KitError("ill-typed", "the expression shares no type with the op's result");
  return {
    kind,
    fault: row["fault"],
    op,
    rewriteArgs(args) {
      return args.slice();
    },
    rewriteResult(args, result) {
      return typed.run([...args, result]);
    },
  };
}
