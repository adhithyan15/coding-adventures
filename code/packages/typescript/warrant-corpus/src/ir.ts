/**
 * # The model IR: compile a language-neutral expression tree to a closure
 *
 * Every claim body in the corpus is a tiny expression tree, for example:
 *
 *     ["eq", ["var", "a"], ["var", "b"]]                       a == b
 *     ["ok", ["if", ["var","choice"], ["var","a"], ["var","b"]]]
 *     ["subject_op", "eq", [["var","a"], ["var","b"]]]         call the real export
 *
 * There are exactly two kinds of call, and keeping them apart is what keeps the
 * oracle outside the library (VER04i 2.5):
 *
 * - `subject_op` dispatches through the binding to the **real** export. It is
 *   allowed only in a claim's subject side, never in a model.
 * - `model_call` invokes a named, pure model (`same`, `same_u64`).
 *
 * ## No `eval`, no `Function`
 *
 * The tree is *interpreted by compilation to closures*: each node becomes a
 * small arrow function over an array of variable slots, built before any case
 * runs. Nothing here turns corpus text into code (no `eval`, no `new
 * Function`, no dynamic `import`), so a hostile corpus can only choose among the
 * node kinds this file implements. An unknown or malformed node is a corpus
 * error immediately, never a skipped case. Evaluating 140,000 cases then costs a
 * function call per node instead of re-walking a list of lists.
 *
 * ## Bounds
 *
 * - Nesting deeper than {@link MAX_DEPTH} is refused.
 * - {@link MAX_IR_COST} is a *static* upper bound on node evaluations per case,
 *   counting every model call at the full cost of its body. A chain of models
 *   that each call the next twice costs 2^n; it is refused at compile time,
 *   before any case runs, instead of burning CPU for hours.
 * - Arithmetic (`add sub mul`) is checked, not wrapped: this profile has no
 *   `U64` arithmetic in any model, and a silent wrap would hide a corpus that
 *   needed one. Operands are range-checked **before** the operation, so `mul`
 *   can never build a huge intermediate just to be rejected afterwards.
 *
 * Values are canonical (see `values.ts`): `boolean`, `bigint`, `Uint8Array`,
 * and `[tag, name]` pairs.
 */

import { CorpusError, need } from "./errors.ts";
import { U64_MAX, valuesEqual, type OpSig } from "./values.ts";

/** Variable slots: a case's binder values, or a model call's arguments. */
export type Env = unknown[];
export type Compiled = (env: Env) => unknown;
/** `(op name, evaluated arguments, static width or undefined) -> canonical result`. */
export type SubjectCall = (op: string, args: unknown[], width: number | undefined) => unknown;

export const MAX_DEPTH = 64;
/** Static bound on node evaluations per expression and case. */
export const MAX_IR_COST = 2_000;

type Node = readonly unknown[];

function isNode(value: unknown): value is Node {
  return Array.isArray(value) && value.length > 0 && typeof value[0] === "string";
}

function bool(value: unknown): boolean {
  if (typeof value !== "boolean") throw new CorpusError("ir-type", "a boolean was expected");
  return value;
}

function ordering(left: unknown, right: unknown): number {
  if (typeof left === "bigint" && typeof right === "bigint") {
    return left < right ? -1 : left > right ? 1 : 0;
  }
  if (left instanceof Uint8Array && right instanceof Uint8Array) {
    const shared = Math.min(left.length, right.length);
    for (let index = 0; index < shared; index += 1) {
      const difference = (left[index] as number) - (right[index] as number);
      if (difference !== 0) return difference;
    }
    return left.length - right.length;
  }
  throw new CorpusError("ir-type", "ordering needs two integers or two byte strings");
}

const BINARY: Readonly<Record<string, (a: unknown, b: unknown) => unknown>> = {
  eq: (a, b) => valuesEqual(a, b),
  ne: (a, b) => !valuesEqual(a, b),
  lt: (a, b) => ordering(a, b) < 0,
  le: (a, b) => ordering(a, b) <= 0,
  gt: (a, b) => ordering(a, b) > 0,
  ge: (a, b) => ordering(a, b) >= 0,
  and: (a, b) => bool(a) && bool(b),
  or: (a, b) => bool(a) || bool(b),
  implies: (a, b) => !bool(a) || bool(b),
};

const ARITHMETIC: Readonly<Record<string, (a: bigint, b: bigint) => bigint>> = {
  add: (a, b) => a + b,
  sub: (a, b) => a - b,
  mul: (a, b) => a * b,
};

/** An arithmetic operand must be a real `bigint` in `0 .. 2^64-1`, checked *before* the operation. */
function operand(value: unknown): bigint {
  if (typeof value !== "bigint" || value < 0n || value > U64_MAX) {
    throw new CorpusError("ir-overflow", "arithmetic operand is outside U64");
  }
  return value;
}

function checked(result: bigint): bigint {
  if (result < 0n || result > U64_MAX) {
    throw new CorpusError("ir-overflow", "model arithmetic left the U64 range");
  }
  return result;
}

interface Model {
  readonly params: string[];
  readonly body: Compiled;
}

/** Compiles IR nodes against a model table and an optional subject hook. */
export class Compiler {
  private compiledModels = new Map<string, Model>();
  private inProgress = new Set<string>();
  private modelCosts = new Map<string, number>();

  private readonly models: Readonly<Record<string, unknown>>;
  private readonly subject: SubjectCall | null;
  private readonly ops: ReadonlyMap<string, OpSig>;

  constructor(
    models: Readonly<Record<string, unknown>>,
    subject: SubjectCall | null,
    ops: ReadonlyMap<string, OpSig>,
  ) {
    this.models = models;
    this.subject = subject;
    this.ops = ops;
  }

  // -- models ---------------------------------------------------------
  private model(name: string): Model {
    const known = this.compiledModels.get(name);
    if (known) return known;
    need(Object.hasOwn(this.models, name), "unsupported-ir", "unknown model call");
    need(!this.inProgress.has(name), "unsupported-ir", "recursive model");
    const model = this.models[name] as Record<string, unknown> | null;
    need(
      typeof model === "object" &&
        model !== null &&
        !Array.isArray(model) &&
        Object.keys(model).length === 2 &&
        "params" in model &&
        "body" in model,
      "unsupported-ir",
      "malformed model",
    );
    const params = model["params"];
    need(
      Array.isArray(params) && params.every((item) => typeof item === "string"),
      "unsupported-ir",
      "malformed model parameters",
    );
    this.inProgress.add(name);
    // A model body may only call other pure models: no subject hook.
    const body = new Compiler(this.models, null, this.ops);
    body.compiledModels = this.compiledModels;
    body.inProgress = this.inProgress;
    body.modelCosts = this.modelCosts;
    const compiled = body.compile(model["body"], params as string[]);
    this.inProgress.delete(name);
    const entry = { params: [...(params as string[])], body: compiled };
    this.compiledModels.set(name, entry);
    return entry;
  }

  // -- expressions ----------------------------------------------------
  /**
   * Compile `node`; `scope` lists the variable names, in slot order.
   *
   * A top-level call also enforces {@link MAX_IR_COST} and turns a JavaScript
   * stack overflow (an absurdly long model chain) into a corpus error.
   */
  compile(node: unknown, scope: readonly string[]): Compiled {
    try {
      const compiled = this.compileNode(node, scope, 0);
      this.cost(node as Node);
      return compiled;
    } catch (error) {
      if (error instanceof RangeError) throw new CorpusError("ir-limit", "model IR nests too deeply");
      throw error;
    }
  }

  /** Static upper bound on node evaluations (see {@link MAX_IR_COST}). */
  private cost(node: Node): number {
    const tag = node[0] as string;
    let children: readonly unknown[] = [];
    if (tag === "len" || tag === "ok" || tag === "not") children = [node[1]];
    else if (tag === "if") children = node.slice(1, 4);
    else if (Object.hasOwn(BINARY, tag) || Object.hasOwn(ARITHMETIC, tag)) children = node.slice(1, 3);
    else if (tag === "model_call" || tag === "subject_op") children = node[2] as unknown[];
    let total = 1;
    for (const child of children) total += this.cost(child as Node);
    if (tag === "model_call") total += this.modelCost(String(node[1]));
    need(total <= MAX_IR_COST, "ir-limit", "model IR cost exceeds the work limit");
    return total;
  }

  private modelCost(name: string): number {
    let known = this.modelCosts.get(name);
    if (known === undefined) {
      known = this.cost((this.models[name] as { body: Node }).body);
      this.modelCosts.set(name, known);
    }
    return known;
  }

  private compileNode(node: unknown, scope: readonly string[], depth: number): Compiled {
    need(depth <= MAX_DEPTH, "ir-limit", "model IR nests too deeply");
    need(isNode(node), "unsupported-ir", "malformed or unknown model IR node");
    const tag = node[0] as string;
    const sub = (child: unknown): Compiled => this.compileNode(child, scope, depth + 1);

    switch (tag) {
      case "lit": {
        need(node.length === 2, "unsupported-ir", "malformed literal");
        const value = this.literal(node[1]);
        return () => value;
      }
      case "var": {
        const slot = typeof node[1] === "string" ? scope.indexOf(node[1]) : -1;
        need(node.length === 2 && slot >= 0, "unsupported-ir", "unknown variable");
        return (env) => env[slot];
      }
      case "len": {
        need(node.length === 2, "unsupported-ir", "malformed len");
        const inner = sub(node[1]);
        return (env) => {
          const value = inner(env);
          if (!(value instanceof Uint8Array)) throw new CorpusError("ir-type", "len needs a byte string");
          return BigInt(value.length);
        };
      }
      case "ok": {
        need(node.length === 2, "unsupported-ir", "malformed ok");
        const inner = sub(node[1]);
        return (env) => ["ok", inner(env)];
      }
      case "err":
      case "name": {
        need(node.length === 2 && typeof node[1] === "string", "unsupported-ir", "malformed err/name");
        const variant = node[1] as string;
        return () => [tag, variant];
      }
      case "not": {
        need(node.length === 2, "unsupported-ir", "malformed not");
        const inner = sub(node[1]);
        return (env) => !bool(inner(env));
      }
      case "if": {
        need(node.length === 4, "unsupported-ir", "malformed if");
        const [condition, then, otherwise] = [sub(node[1]), sub(node[2]), sub(node[3])];
        return (env) => (bool(condition(env)) ? then(env) : otherwise(env));
      }
      case "model_call":
        return this.modelCall(node, sub);
      case "subject_op":
        return this.subjectOp(node, sub);
      default:
        break;
    }
    if (Object.hasOwn(BINARY, tag)) {
      need(node.length === 3, "unsupported-ir", "malformed binary node");
      const [left, right] = [sub(node[1]), sub(node[2])];
      const apply = BINARY[tag] as (a: unknown, b: unknown) => unknown;
      return (env) => apply(left(env), right(env));
    }
    if (Object.hasOwn(ARITHMETIC, tag)) {
      need(node.length === 3, "unsupported-ir", "malformed arithmetic node");
      const [left, right] = [sub(node[1]), sub(node[2])];
      const apply = ARITHMETIC[tag] as (a: bigint, b: bigint) => bigint;
      return (env) => checked(apply(operand(left(env)), operand(right(env))));
    }
    throw new CorpusError("unsupported-ir", "unknown model IR tag");
  }

  private literal(value: unknown): bigint | boolean {
    if (typeof value === "boolean") return value;
    if (typeof value === "number" && Number.isSafeInteger(value)) return BigInt(value);
    throw new CorpusError("unsupported-ir", "unsupported literal");
  }

  private modelCall(node: Node, sub: (child: unknown) => Compiled): Compiled {
    need(
      node.length === 3 && typeof node[1] === "string" && Array.isArray(node[2]),
      "unsupported-ir",
      "malformed model_call",
    );
    const { params, body } = this.model(node[1] as string);
    const args = (node[2] as unknown[]).map(sub);
    need(args.length === params.length, "unsupported-ir", "model arity mismatch");
    return (env) => body(args.map((argument) => argument(env)));
  }

  private subjectOp(node: Node, sub: (child: unknown) => Compiled): Compiled {
    const call = this.subject;
    need(call !== null, "unsupported-ir", "a subject call is forbidden in a model or admission expression");
    need(
      (node.length === 3 || node.length === 4) && typeof node[1] === "string" && Array.isArray(node[2]),
      "unsupported-ir",
      "malformed subject_op",
    );
    const name = node[1] as string;
    const signature = this.ops.get(name);
    need(signature !== undefined, "unsupported-ir", "subject call to an unknown op");
    let width: number | undefined;
    if (node.length === 4) {
      const widths = node[3];
      need(
        Array.isArray(widths) &&
          widths.length === 1 &&
          typeof widths[0] === "number" &&
          Number.isSafeInteger(widths[0]) &&
          widths[0] >= 0,
        "unsupported-ir",
        "malformed static width",
      );
      width = (widths as number[])[0];
    }
    need(
      signature.widthParams.length > 0 === (width !== undefined),
      "unsupported-ir",
      "a fixed-width op needs its width, and any other op must not state one",
    );
    const args = (node[2] as unknown[]).map(sub);
    return (env) => call(name, args.map((argument) => argument(env)), width);
  }
}
