import {
  ACOS,
  ACOSH,
  ADD,
  AND,
  ASIN,
  ASINH,
  ASSIGN,
  ATAN,
  ATANH,
  COS,
  COSH,
  COTH,
  CSCH,
  D,
  DEFINE,
  DIV,
  EQUAL,
  EXP,
  FACTOR,
  FALSE,
  GREATER,
  GREATER_EQUAL,
  IF,
  INTEGRATE,
  INV,
  IRApply,
  IRNode,
  IRSymbol,
  LESS,
  LESS_EQUAL,
  LIST,
  LOG,
  MUL,
  NEG,
  NOT,
  NOT_EQUAL,
  OR,
  POW,
  SECH,
  SIN,
  SINH,
  SQRT,
  SUB,
  TAN,
  TANH,
  TRUE,
  app,
  equals,
  headName,
  int,
  isOne,
  isZero,
  numberNode,
  rational,
  sym,
} from "@coding-adventures/symbolic-ir";
import { BiRational, factorIntegerPolynomial, tryBivariateHensel, tryNVariateHensel } from "@coding-adventures/cas-factor";
import type { BiPoly, NPoly } from "@coding-adventures/cas-factor";
import { AssumptionContext } from "@coding-adventures/cas-simplify";

export type Handler = (vm: VM, expr: IRApply) => IRNode;
export type RulePredicate = (expr: IRApply) => boolean;
export type RuleTransform = (expr: IRApply) => IRNode;
export type Rule = readonly [RulePredicate, RuleTransform];

export interface Backend {
  lookup(name: string): IRNode | undefined;
  bind(name: string, value: IRNode): void;
  unbind(name: string): void;
  onUnresolved(symbol: string): IRNode;
  onUnknownHead(expr: IRApply): IRNode;
  rules(): readonly Rule[];
  handlers(): ReadonlyMap<string, Handler>;
  holdHeads(): ReadonlySet<string>;
}

export class StrictEvaluationError extends Error {}
export class ArityError extends Error {}

class BaseBackend {
  protected readonly env = new Map<string, IRNode>([
    [TRUE.name, TRUE],
    [FALSE.name, FALSE],
  ]);

  protected readonly held = new Set<string>([
    ASSIGN.name,
    DEFINE.name,
    IF.name,
    // Track G2: `Assume(rel)` and `Forget(rel)` must NOT pre-evaluate
    // their relational argument — `Greater(a^2, b^2)` would otherwise be
    // reduced (or echoed by the symbolic backend) before reaching the
    // handler, which would then have no symbolic relation to record.
    "Assume",
    "Forget",
  ]);

  lookup(name: string): IRNode | undefined {
    return this.env.get(name);
  }

  bind(name: string, value: IRNode): void {
    this.env.set(name, value);
  }

  unbind(name: string): void {
    this.env.delete(name);
  }

  holdHeads(): ReadonlySet<string> {
    return this.held;
  }
}

export class StrictBackend extends BaseBackend implements Backend {
  private readonly table = buildHandlerTable(false);

  onUnresolved(symbol: string): IRNode {
    throw new StrictEvaluationError(`undefined symbol: ${symbol}`);
  }

  onUnknownHead(expr: IRApply): IRNode {
    throw new StrictEvaluationError(`no handler for head: ${headName(expr.head) || "?"}`);
  }

  rules(): readonly Rule[] {
    return [];
  }

  handlers(): ReadonlyMap<string, Handler> {
    return this.table;
  }
}

export class SymbolicBackend extends BaseBackend implements Backend {
  private readonly table = buildHandlerTable(true);

  onUnresolved(symbol: string): IRNode {
    return sym(symbol);
  }

  onUnknownHead(expr: IRApply): IRNode {
    return expr;
  }

  rules(): readonly Rule[] {
    return [];
  }

  handlers(): ReadonlyMap<string, Handler> {
    return this.table;
  }
}

export class VM {
  /**
   * Track G2 (TypeScript port): per-VM assumption store consulted by
   * the symbolic-coefficient Weierstrass integrator (and any future
   * sign-aware helper).  Mutated only via the `Assume(...)` /
   * `Forget(...)` / `ForgetAll()` handlers — direct field access is
   * supported for tests and embedders.
   */
  readonly assumptions: AssumptionContext = new AssumptionContext();

  constructor(public readonly backend: Backend) {}

  eval(node: IRNode): IRNode {
    if (node.kind === "symbol") {
      return this.evalSymbol(node.name, node);
    }
    if (node.kind === "apply") {
      return this.evalApply(node);
    }
    return node;
  }

  evalProgram(statements: readonly IRNode[]): IRNode | undefined {
    let result: IRNode | undefined;
    for (const statement of statements) {
      result = this.eval(statement);
    }
    return result;
  }

  private evalSymbol(name: string, original: IRNode): IRNode {
    const value = this.backend.lookup(name);
    if (value === undefined) {
      return this.backend.onUnresolved(name);
    }
    if (equals(value, original)) {
      return original;
    }
    return this.eval(value);
  }

  private evalApply(node: IRApply): IRNode {
    const name = headName(node.head);
    const args = this.backend.holdHeads().has(name)
      ? node.args
      : node.args.map((arg) => this.eval(arg));
    const expr = app(node.head, args);

    for (const [predicate, transform] of this.backend.rules()) {
      if (predicate(expr)) {
        return this.eval(transform(expr));
      }
    }

    const handler = this.backend.handlers().get(name);
    if (handler !== undefined) {
      return handler(this, expr);
    }

    if (node.head.kind === "symbol") {
      const bound = this.backend.lookup(node.head.name);
      if (bound?.kind === "apply" && equals(bound.head, DEFINE)) {
        return this.eval(this.applyUserFunction(bound, args));
      }
    }

    return this.backend.onUnknownHead(expr);
  }

  private applyUserFunction(definition: IRApply, args: readonly IRNode[]): IRNode {
    if (definition.args.length !== 3) {
      throw new ArityError("Define record must have name, params, and body");
    }
    const params = definition.args[1];
    const body = definition.args[2];
    if (params.kind !== "apply" || !equals(params.head, LIST)) {
      throw new TypeError("Define params must be a List");
    }
    const paramNames = params.args.map((param) => {
      if (param.kind !== "symbol") {
        throw new TypeError("Define params must be symbols");
      }
      return param.name;
    });
    if (paramNames.length !== args.length) {
      throw new ArityError(`arity mismatch: expected ${paramNames.length}, got ${args.length}`);
    }
    return substitute(body, new Map(paramNames.map((name, i) => [name, args[i]])));
  }
}

export function substitute(node: IRNode, mapping: ReadonlyMap<string, IRNode>): IRNode {
  if (node.kind === "symbol") {
    return mapping.get(node.name) ?? node;
  }
  if (node.kind === "apply") {
    return app(substitute(node.head, mapping), node.args.map((arg) => substitute(arg, mapping)));
  }
  return node;
}

// ---------------------------------------------------------------------------
// Phase 29–33: algebraic helpers and lookup tables
// ---------------------------------------------------------------------------

// Reduced fraction [numerator, denominator] with denominator > 0.
type Frac = readonly [bigint, bigint];

/** GCD of two non-negative bigints. */
function fracGcd(a: bigint, b: bigint): bigint {
  while (b !== 0n) [a, b] = [b, a % b];
  return a;
}

/** Reduce a rational number p/q to lowest terms with q > 0. */
function fracMake(p: bigint, q: bigint): Frac {
  if (q === 0n) throw new RangeError("zero denominator in fracMake");
  if (q < 0n) { p = -p; q = -q; }
  const g = fracGcd(p < 0n ? -p : p, q === 0n ? 1n : q);
  return [p / g, q / g];
}

/** String key for a reduced fraction, used as Map key in PI tables. */
function fracKey(f: Frac): string { return `${f[0]}/${f[1]}`; }

/** Modular reduction: (p/q) mod m, result in [0, m). */
function fracMod(f: Frac, m: bigint): Frac {
  const [p, q] = f;
  let r = p % (m * q);
  if (r < 0n) r += m * q;
  return fracMake(r, q);
}

/** Extract the fraction represented by a plain numeric IR node (integer or rational). */
function fracFromIR(node: IRNode): Frac | undefined {
  if (node.kind === "integer") return [node.value, 1n];
  if (node.kind === "rational") return fracMake(node.numer, node.denom);
  return undefined;
}

/**
 * Phase 33: If `arg` equals `q · %pi` for a rational `q` with small
 * denominator, return `q` as a reduced {@link Frac}.  Otherwise `undefined`.
 *
 * Two strategies:
 * 1. **Float** — arg ≈ q·π; check denominators {1,2,3,4,6}.
 * 2. **Structural** — matches `%pi`, `Neg(%pi)`, `Mul(n, %pi)`,
 *    `Div(%pi, n)`, `Div(Mul(n, %pi), d)` (both orderings of n and %pi).
 */
function tryPiMultiple(arg: IRNode): Frac | undefined {
  // Strategy 1: float value ≈ q·π
  if (arg.kind === "float") {
    const qf = arg.value / Math.PI;
    for (const d of [1, 2, 3, 4, 6]) {
      const pCand = Math.round(qf * d);
      if (Math.abs(qf * d - pCand) < 1e-9)
        return fracMake(BigInt(pCand), BigInt(d));
    }
    return undefined;
  }
  // Strategy 2: structural match
  if (arg.kind === "symbol" && arg.name === "%pi") return [1n, 1n];
  if (arg.kind !== "apply") return undefined;

  // Neg(anything) — recurse and negate
  if (equals(arg.head, NEG) && arg.args.length === 1) {
    const inner = tryPiMultiple(arg.args[0]);
    return inner === undefined ? undefined : fracMake(-inner[0], inner[1]);
  }

  // Mul(n, %pi) or Mul(%pi, n)
  if (equals(arg.head, MUL) && arg.args.length === 2) {
    const [a, b] = arg.args;
    const piB = b.kind === "symbol" && b.name === "%pi";
    const piA = a.kind === "symbol" && a.name === "%pi";
    if (piB) return fracFromIR(a);
    if (piA) return fracFromIR(b);
  }

  // Div(%pi, n) or Div(Mul(n,%pi), d)
  if (equals(arg.head, DIV) && arg.args.length === 2) {
    const [num, den] = arg.args;
    const dFrac = fracFromIR(den);
    if (dFrac === undefined || dFrac[0] === 0n) return undefined;

    // Div(%pi, n) → 1 / dFrac = [dFrac[1], dFrac[0]]
    if (num.kind === "symbol" && num.name === "%pi")
      return fracMake(dFrac[1], dFrac[0]);

    // Div(Mul(n,%pi), d) or Div(Mul(%pi,n), d)
    if (num.kind === "apply" && equals(num.head, MUL) && num.args.length === 2) {
      const [ma, mb] = num.args;
      const piMb = mb.kind === "symbol" && mb.name === "%pi";
      const piMa = ma.kind === "symbol" && ma.name === "%pi";
      if (piMb) {
        const nFrac = fracFromIR(ma);
        return nFrac === undefined ? undefined
          : fracMake(nFrac[0] * dFrac[1], nFrac[1] * dFrac[0]);
      }
      if (piMa) {
        const nFrac = fracFromIR(mb);
        return nFrac === undefined ? undefined
          : fracMake(nFrac[0] * dFrac[1], nFrac[1] * dFrac[0]);
      }
    }
  }
  return undefined;
}

// ---------------------------------------------------------------------------
// Phase 33: exact algebraic IR constants shared by sin/cos/tan tables
// ---------------------------------------------------------------------------
const _P33_SQRT2 = app(sym("Sqrt"), [int(2)]);
const _P33_SQRT3 = app(sym("Sqrt"), [int(3)]);
const _P33_SQRT2_OVER_2 = app(sym("Div"), [_P33_SQRT2, int(2)]);
const _P33_SQRT3_OVER_2 = app(sym("Div"), [_P33_SQRT3, int(2)]);
const _P33_SQRT3_OVER_3 = app(sym("Div"), [_P33_SQRT3, int(3)]);
const _P33_NEG_SQRT2_OVER_2 = app(sym("Neg"), [_P33_SQRT2_OVER_2]);
const _P33_NEG_SQRT3_OVER_2 = app(sym("Neg"), [_P33_SQRT3_OVER_2]);
const _P33_NEG_SQRT3 = app(sym("Neg"), [_P33_SQRT3]);
const _P33_NEG_SQRT3_OVER_3 = app(sym("Neg"), [_P33_SQRT3_OVER_3]);

/**
 * sin(q·π) for q ∈ [0, 2)  — period 2π → reduce mod 2.
 *
 * Values:
 *   0      → 0,   1/6 → 1/2,  1/4 → √2/2, 1/3 → √3/2,
 *   1/2    → 1,   2/3 → √3/2, 3/4 → √2/2, 5/6 → 1/2,
 *   1      → 0,   7/6 → −1/2, 5/4 → −√2/2, 4/3 → −√3/2,
 *   3/2    → −1,  5/3 → −√3/2, 7/4 → −√2/2, 11/6 → −1/2.
 */
const SIN_PI_TABLE = new Map<string, IRNode>([
  ["0/1",   int(0)],
  ["1/6",   rational(1n, 2n)],
  ["1/4",   _P33_SQRT2_OVER_2],
  ["1/3",   _P33_SQRT3_OVER_2],
  ["1/2",   int(1)],
  ["2/3",   _P33_SQRT3_OVER_2],
  ["3/4",   _P33_SQRT2_OVER_2],
  ["5/6",   rational(1n, 2n)],
  ["1/1",   int(0)],
  ["7/6",   rational(-1n, 2n)],
  ["5/4",   _P33_NEG_SQRT2_OVER_2],
  ["4/3",   _P33_NEG_SQRT3_OVER_2],
  ["3/2",   int(-1)],
  ["5/3",   _P33_NEG_SQRT3_OVER_2],
  ["7/4",   _P33_NEG_SQRT2_OVER_2],
  ["11/6",  rational(-1n, 2n)],
]);

/**
 * cos(q·π) for q ∈ [0, 2)  — period 2π → reduce mod 2.
 *
 * Values:
 *   0      → 1,   1/6 → √3/2, 1/4 → √2/2, 1/3 → 1/2,
 *   1/2    → 0,   2/3 → −1/2, 3/4 → −√2/2, 5/6 → −√3/2,
 *   1      → −1,  7/6 → −√3/2, 5/4 → −√2/2, 4/3 → −1/2,
 *   3/2    → 0,   5/3 → 1/2, 7/4 → √2/2, 11/6 → √3/2.
 */
const COS_PI_TABLE = new Map<string, IRNode>([
  ["0/1",   int(1)],
  ["1/6",   _P33_SQRT3_OVER_2],
  ["1/4",   _P33_SQRT2_OVER_2],
  ["1/3",   rational(1n, 2n)],
  ["1/2",   int(0)],
  ["2/3",   rational(-1n, 2n)],
  ["3/4",   _P33_NEG_SQRT2_OVER_2],
  ["5/6",   _P33_NEG_SQRT3_OVER_2],
  ["1/1",   int(-1)],
  ["7/6",   _P33_NEG_SQRT3_OVER_2],
  ["5/4",   _P33_NEG_SQRT2_OVER_2],
  ["4/3",   rational(-1n, 2n)],
  ["3/2",   int(0)],
  ["5/3",   rational(1n, 2n)],
  ["7/4",   _P33_SQRT2_OVER_2],
  ["11/6",  _P33_SQRT3_OVER_2],
]);

/**
 * tan(q·π) for q ∈ [0, 1) — period π → reduce mod 1.
 * q = 1/2 is omitted — tan(π/2) is undefined; the handler leaves it unevaluated.
 *
 * Values:
 *   0   → 0, 1/6 → √3/3, 1/4 → 1, 1/3 → √3,
 *   2/3 → −√3, 3/4 → −1, 5/6 → −√3/3.
 */
const TAN_PI_TABLE = new Map<string, IRNode>([
  ["0/1",  int(0)],
  ["1/6",  _P33_SQRT3_OVER_3],
  ["1/4",  int(1)],
  ["1/3",  _P33_SQRT3],
  ["2/3",  _P33_NEG_SQRT3],
  ["3/4",  int(-1)],
  ["5/6",  _P33_NEG_SQRT3_OVER_3],
]);

// The %pi symbol used in acos reflection identity: acos(-x) = %pi − acos(x).
const _INV_TRIG_PI = sym("%pi");

// The Abs head symbol for sqrt/abs rules.
const _ABS_HEAD = sym("Abs");

function buildHandlerTable(simplify: boolean): ReadonlyMap<string, Handler> {
  const table = new Map<string, Handler>();
  // Phase 47 (TypeScript port): nested-Add flattening.  When either
  // binary Add operand is itself an Add(...) apply, gather every
  // non-Add leaf via the existing `flattenAddTerms` walker, sum the
  // numeric literals once, and rebuild a left-associated chain.
  // Without this, the structural-equality check inside cas-summation's
  // telescope detector misses telescopes whose denominators contain
  // (k + a) shifts produced by Apart (e.g. ∑ 1/((k+1)(k+2))).
  // Strict mode (simplify=false) keeps the original binary semantics.
  const addBinary = binaryNumeric("Add", simplify, (a, b) => addNumeric(a, b), (expr, a, b) => {
    if (isZero(a)) return b;
    if (isZero(b)) return a;
    return expr;
  });
  table.set(ADD.name, (vm, expr) => {
    if (simplify && expr.args.length === 2) {
      const [aPre, bPre] = expr.args;
      const aIsAdd = aPre.kind === "apply" && equals(aPre.head, ADD);
      const bIsAdd = bPre.kind === "apply" && equals(bPre.head, ADD);
      if (aIsAdd || bIsAdd) {
        const leaves = [...flattenAddTerms(aPre), ...flattenAddTerms(bPre)];
        // Re-evaluation guard: only rewrite when flattening actually
        // changed the operand list (saves a needless rebuild when
        // flattenAddTerms walked through a SUB it normalised).
        const rebuilt =
          leaves.length !== 2 ||
          leaves.some((leaf) => leaf.kind === "apply" && equals(leaf.head, ADD));
        if (rebuilt) {
          // Sum numeric leaves once; collect symbolic ones in order.
          let litAcc: Numeric | undefined;
          const nonLiterals: IRNode[] = [];
          for (const leaf of leaves) {
            const n = toNumeric(leaf);
            if (n === undefined) {
              nonLiterals.push(leaf);
            } else {
              litAcc = litAcc === undefined ? n : addNumeric(litAcc, n);
            }
          }
          if (nonLiterals.length === 0) {
            // Whole expression folded to a single numeric literal.
            return fromNumeric(litAcc ?? { kind: "int", value: 0n });
          }
          const litIsZero =
            litAcc === undefined ||
            (litAcc.kind === "int" && litAcc.value === 0n) ||
            (litAcc.kind === "rat" && litAcc.numer === 0n);
          const finalArgs = litIsZero
            ? nonLiterals
            : [...nonLiterals, fromNumeric(litAcc!)];
          if (finalArgs.length === 1) {
            return finalArgs[0];
          }
          // Left-associate the chain for predictable structural equality.
          let out: IRNode = finalArgs[0];
          for (let i = 1; i < finalArgs.length; i += 1) {
            out = app(ADD, [out, finalArgs[i]]);
          }
          return out;
        }
      }
    }
    return addBinary(vm, expr);
  });
  table.set(SUB.name, binaryNumeric("Sub", simplify, (a, b) => subNumeric(a, b), (expr, _a, b) => {
    if (isZero(b)) return expr.args[0];
    return expr;
  }));
  table.set(MUL.name, binaryNumeric("Mul", simplify, (a, b) => mulNumeric(a, b), (expr, a, b) => {
    if (isZero(a) || isZero(b)) return int(0);
    if (isOne(a)) return b;
    if (isOne(b)) return a;
    return expr;
  }));
  table.set(DIV.name, binaryNumeric("Div", simplify, (a, b) => divNumeric(a, b), (expr, a, b) => {
    if (isZero(a)) return int(0);
    if (isOne(b)) return expr.args[0];
    return expr;
  }));
  table.set(POW.name, binaryNumeric("Pow", simplify, (a, b) => powNumeric(a, b), (expr, base, exponent) => {
    if (isZero(exponent)) return int(1);
    if (isOne(exponent)) return base;
    if (isZero(base)) return int(0);
    if (isOne(base)) return int(1);
    return expr;
  }));
  table.set(NEG.name, unaryNumeric("Neg", simplify, negNumeric, (expr, a) => {
    if (a.kind === "apply" && equals(a.head, NEG) && a.args.length === 1) {
      return a.args[0];
    }
    return expr;
  }));
  table.set(INV.name, unaryNumeric("Inv", simplify, invNumeric, (expr) => expr));

  // ----- Phase 29: Abs — idempotency, negation-strip, even-power -----
  // The Abs head is not in symbolic-ir's export list, so we register it
  // by string name here.  Any expression of the form Abs(x) produced by
  // the sqrt handler will be caught by this handler on re-evaluation.
  table.set("Abs", (vm, expr) => {
    if (expr.args.length !== 1) return expr;
    const inner = expr.args[0];
    const na = toNumeric(inner);
    if (na !== undefined) {
      // Numeric fold: abs(n) = |n|.
      if (na.kind === "int") return int(na.value < 0n ? -na.value : na.value);
      if (na.kind === "rat") {
        const n = na.numer < 0n ? -na.numer : na.numer;
        return rational(n, na.denom);
      }
      return numberNode(Math.abs(toNumber(na)));
    }
    if (!simplify) throw new TypeError(`Abs requires a numeric argument: ${headName(expr.head)}`);
    // Rule 4a: abs(abs(x)) = abs(x)
    if (inner.kind === "apply" && equals(inner.head, _ABS_HEAD))
      return inner;
    // Rule 4b: abs(-x) = abs(x)  (strip NEG)
    if (inner.kind === "apply" && equals(inner.head, NEG) && inner.args.length === 1)
      return vm.eval(app(_ABS_HEAD, [inner.args[0]]));
    // Rule 4c: abs(Mul(-1, x)) = abs(x)  (-x encoded as Mul after numeric fold)
    if (inner.kind === "apply" && equals(inner.head, MUL) && inner.args.length === 2) {
      const [mA] = inner.args;
      if (mA.kind === "integer" && mA.value === -1n)
        return vm.eval(app(_ABS_HEAD, [inner.args[1]]));
    }
    // Rule 4d: abs(x^{2k}) = x^{2k}  (even power ≥ 0 always)
    if (inner.kind === "apply" && equals(inner.head, POW) && inner.args.length === 2) {
      const [, expNode] = inner.args;
      if (expNode.kind === "integer" && expNode.value >= 2n && expNode.value % 2n === 0n)
        return inner;
    }
    return expr;
  });

  // ----- Phase 29: Sqrt — perfect-square fold, even-power rewrite -----
  // Overrides the numeric-only elementary() factory.
  table.set(SQRT.name, (vm, expr) => {
    if (expr.args.length !== 1) return expr;
    const arg = expr.args[0];
    const na = toNumeric(arg);
    if (na !== undefined) {
      if (numericKey(na) === "0") return int(0);
      if (numericKey(na) === "1") return int(1);
      const result = Math.sqrt(toNumber(na));
      // Perfect-square detection: if round(√n)² == n, return integer.
      const intResult = Math.round(result);
      if (intResult * intResult === toNumber(na)) return int(intResult);
      return numberNode(result);
    }
    if (!simplify) throw new TypeError(`Sqrt requires a numeric argument: ${headName(expr.head)}`);
    // sqrt(x^{2k}): split into even-k (x^k is non-negative) and odd-k (need Abs).
    if (arg.kind === "apply" && equals(arg.head, POW) && arg.args.length === 2) {
      const [base, expNode] = arg.args;
      if (expNode.kind === "integer" && expNode.value > 0n && expNode.value % 2n === 0n) {
        const k = expNode.value / 2n;
        if (k % 2n === 0n) {
          // k even → x^k ≥ 0 always, e.g. sqrt(x^4) = x^2.
          return app(POW, [base, int(k)]);
        }
        // k odd → |x^k|, e.g. sqrt(x^2) = |x|, sqrt(x^6) = |x^3|.
        if (k === 1n) return vm.eval(app(_ABS_HEAD, [base]));
        return vm.eval(app(_ABS_HEAD, [app(POW, [base, int(k)])]));
      }
    }
    return expr;
  });

  // ----- Phase 30: Log — log(exp(x))→x cancellation -----
  table.set(LOG.name, (_vm, expr) => {
    if (expr.args.length !== 1) return expr;
    const arg = expr.args[0];
    const na = toNumeric(arg);
    if (na !== undefined) {
      if (numericKey(na) === "1") return int(0);
      if (toNumber(na) <= 0) return expr; // log undefined for non-positive reals
      return numberNode(Math.log(toNumber(na)));
    }
    if (!simplify) throw new TypeError(`Log requires a numeric argument: ${headName(expr.head)}`);
    // Rule 3: log(exp(x)) = x  (structural cancellation, always valid for real domain).
    if (arg.kind === "apply" && equals(arg.head, EXP) && arg.args.length === 1)
      return arg.args[0];
    // Note: log(x^n) = n·log(x) requires an assumption context; skipped here.
    return expr;
  });

  // ----- Phase 30: Exp — exp(log(x))→x and exp(n·log(x))→x^n -----
  table.set(EXP.name, (_vm, expr) => {
    if (expr.args.length !== 1) return expr;
    const arg = expr.args[0];
    const na = toNumeric(arg);
    if (na !== undefined) {
      if (numericKey(na) === "0") return int(1);
      return numberNode(Math.exp(toNumber(na)));
    }
    if (!simplify) throw new TypeError(`Exp requires a numeric argument: ${headName(expr.head)}`);
    // Rule 3: exp(log(x)) = x.
    if (arg.kind === "apply" && equals(arg.head, LOG) && arg.args.length === 1)
      return arg.args[0];
    // Rule 4: exp(n·log(x)) = x^n  — handles both Mul(n, log(x)) and Mul(log(x), n).
    if (arg.kind === "apply" && equals(arg.head, MUL) && arg.args.length === 2) {
      const [a, b] = arg.args;
      if (a.kind === "apply" && equals(a.head, LOG) && a.args.length === 1)
        return app(POW, [a.args[0], b]);
      if (b.kind === "apply" && equals(b.head, LOG) && b.args.length === 1)
        return app(POW, [b.args[0], a]);
    }
    return expr;
  });

  // ----- Phase 31+33: Sin — odd symmetry, arc-cancel, π-multiples -----
  table.set(SIN.name, (vm, expr) => {
    if (expr.args.length !== 1) return expr;
    const arg = expr.args[0];
    // Rule 4 (Phase 33): π-multiple exact values (checked before numeric fold so
    // that %pi symbols are detected before any backend evaluates them to floats).
    const q = tryPiMultiple(arg);
    if (q !== undefined) {
      const val = SIN_PI_TABLE.get(fracKey(fracMod(q, 2n)));
      if (val !== undefined) return val;
    }
    // Rule 1: numeric fold.
    const na = toNumeric(arg);
    if (na !== undefined) {
      if (numericKey(na) === "0") return int(0);
      return numberNode(Math.sin(toNumber(na)));
    }
    if (!simplify) throw new TypeError(`Sin requires a numeric argument: ${headName(expr.head)}`);
    // Rule 2 (Phase 31): odd symmetry — sin(-x) = -sin(x).
    if (arg.kind === "apply" && equals(arg.head, NEG) && arg.args.length === 1)
      return vm.eval(app(NEG, [app(SIN, [arg.args[0]])]));
    // Rule 3 (Phase 31): arc-cancellation — sin(asin(x)) = x.
    if (arg.kind === "apply" && equals(arg.head, ASIN) && arg.args.length === 1)
      return arg.args[0];
    return expr;
  });

  // ----- Phase 31+33: Cos — even symmetry, arc-cancel, π-multiples -----
  table.set(COS.name, (vm, expr) => {
    if (expr.args.length !== 1) return expr;
    const arg = expr.args[0];
    // Rule 4 (Phase 33): π-multiple exact values.
    // cos(-q·π) = cos(q·π) by even symmetry; this is handled automatically by the
    // modular reduction since Fraction(-1/3) % 2 = Fraction(5/3) → same table entry.
    const q = tryPiMultiple(arg);
    if (q !== undefined) {
      const val = COS_PI_TABLE.get(fracKey(fracMod(q, 2n)));
      if (val !== undefined) return val;
    }
    const na = toNumeric(arg);
    if (na !== undefined) {
      if (numericKey(na) === "0") return int(1);
      return numberNode(Math.cos(toNumber(na)));
    }
    if (!simplify) throw new TypeError(`Cos requires a numeric argument: ${headName(expr.head)}`);
    // Rule 2 (Phase 31): even symmetry — cos(-x) = cos(x).
    if (arg.kind === "apply" && equals(arg.head, NEG) && arg.args.length === 1)
      return vm.eval(app(COS, [arg.args[0]]));
    // Rule 3 (Phase 31): arc-cancellation — cos(acos(x)) = x.
    if (arg.kind === "apply" && equals(arg.head, ACOS) && arg.args.length === 1)
      return arg.args[0];
    return expr;
  });

  // ----- Phase 31+33: Tan — odd symmetry, arc-cancel, π-multiples -----
  table.set(TAN.name, (vm, expr) => {
    if (expr.args.length !== 1) return expr;
    const arg = expr.args[0];
    // Rule 4 (Phase 33): π-multiple exact values.
    // tan(-q·π) = -tan(q·π) by odd symmetry — handled via sign and abs.
    const q = tryPiMultiple(arg);
    if (q !== undefined) {
      const sign = q[0] < 0n ? -1 : 1;
      const qAbs: Frac = q[0] < 0n ? [-q[0], q[1]] : q;
      const qMod = fracKey(fracMod(qAbs, 1n)); // period π
      const val = TAN_PI_TABLE.get(qMod);
      if (val !== undefined)
        return sign < 0 ? app(NEG, [val]) : val;
      // q = 1/2 (mod 1) → undefined, fall through
    }
    const na = toNumeric(arg);
    if (na !== undefined) {
      if (numericKey(na) === "0") return int(0);
      return numberNode(Math.tan(toNumber(na)));
    }
    if (!simplify) throw new TypeError(`Tan requires a numeric argument: ${headName(expr.head)}`);
    // Rule 2 (Phase 31): odd symmetry — tan(-x) = -tan(x).
    if (arg.kind === "apply" && equals(arg.head, NEG) && arg.args.length === 1)
      return vm.eval(app(NEG, [app(TAN, [arg.args[0]])]));
    // Rule 3 (Phase 31): arc-cancellation — tan(atan(x)) = x.
    if (arg.kind === "apply" && equals(arg.head, ATAN) && arg.args.length === 1)
      return arg.args[0];
    return expr;
  });

  // ----- Phase 32: Atan — odd symmetry -----
  table.set(ATAN.name, (vm, expr) => {
    if (expr.args.length !== 1) return expr;
    const arg = expr.args[0];
    const na = toNumeric(arg);
    if (na !== undefined) {
      if (numericKey(na) === "0") return int(0);
      return numberNode(Math.atan(toNumber(na)));
    }
    if (!simplify) throw new TypeError(`Atan requires a numeric argument: ${headName(expr.head)}`);
    // Odd symmetry — atan(-x) = -atan(x).
    if (arg.kind === "apply" && equals(arg.head, NEG) && arg.args.length === 1)
      return vm.eval(app(NEG, [app(ATAN, [arg.args[0]])]));
    return expr;
  });

  // ----- Phase 32: Asin — odd symmetry -----
  table.set(ASIN.name, (vm, expr) => {
    if (expr.args.length !== 1) return expr;
    const arg = expr.args[0];
    const na = toNumeric(arg);
    if (na !== undefined) {
      if (numericKey(na) === "0") return int(0);
      return numberNode(Math.asin(toNumber(na)));
    }
    if (!simplify) throw new TypeError(`Asin requires a numeric argument: ${headName(expr.head)}`);
    // Odd symmetry — asin(-x) = -asin(x).
    if (arg.kind === "apply" && equals(arg.head, NEG) && arg.args.length === 1)
      return vm.eval(app(NEG, [app(ASIN, [arg.args[0]])]));
    return expr;
  });

  // ----- Phase 32: Acos — reflection identity acos(-x) = π - acos(x) -----
  table.set(ACOS.name, (vm, expr) => {
    if (expr.args.length !== 1) return expr;
    const arg = expr.args[0];
    const na = toNumeric(arg);
    if (na !== undefined) {
      if (numericKey(na) === "1") return int(0);
      return numberNode(Math.acos(toNumber(na)));
    }
    if (!simplify) throw new TypeError(`Acos requires a numeric argument: ${headName(expr.head)}`);
    // Reflection: acos(-x) = π − acos(x).
    if (arg.kind === "apply" && equals(arg.head, NEG) && arg.args.length === 1) {
      const innerAcos = vm.eval(app(ACOS, [arg.args[0]]));
      return app(SUB, [_INV_TRIG_PI, innerAcos]);
    }
    return expr;
  });

  // ----- Phase 31: Sinh — odd symmetry, arc-cancellation -----
  table.set(SINH.name, (vm, expr) => {
    if (expr.args.length !== 1) return expr;
    const arg = expr.args[0];
    const na = toNumeric(arg);
    if (na !== undefined) {
      if (numericKey(na) === "0") return int(0);
      return numberNode(Math.sinh(toNumber(na)));
    }
    if (!simplify) throw new TypeError(`Sinh requires a numeric argument: ${headName(expr.head)}`);
    // Odd symmetry — sinh(-x) = -sinh(x).
    if (arg.kind === "apply" && equals(arg.head, NEG) && arg.args.length === 1)
      return vm.eval(app(NEG, [app(SINH, [arg.args[0]])]));
    // Arc-cancellation — sinh(asinh(x)) = x.
    if (arg.kind === "apply" && equals(arg.head, ASINH) && arg.args.length === 1)
      return arg.args[0];
    return expr;
  });

  // ----- Phase 31: Cosh — even symmetry, arc-cancellation -----
  table.set(COSH.name, (vm, expr) => {
    if (expr.args.length !== 1) return expr;
    const arg = expr.args[0];
    const na = toNumeric(arg);
    if (na !== undefined) {
      if (numericKey(na) === "0") return int(1);
      return numberNode(Math.cosh(toNumber(na)));
    }
    if (!simplify) throw new TypeError(`Cosh requires a numeric argument: ${headName(expr.head)}`);
    // Even symmetry — cosh(-x) = cosh(x).
    if (arg.kind === "apply" && equals(arg.head, NEG) && arg.args.length === 1)
      return vm.eval(app(COSH, [arg.args[0]]));
    // Arc-cancellation — cosh(acosh(x)) = x.
    if (arg.kind === "apply" && equals(arg.head, ACOSH) && arg.args.length === 1)
      return arg.args[0];
    return expr;
  });

  // ----- Phase 31: Tanh — odd symmetry, arc-cancellation -----
  table.set(TANH.name, (vm, expr) => {
    if (expr.args.length !== 1) return expr;
    const arg = expr.args[0];
    const na = toNumeric(arg);
    if (na !== undefined) {
      if (numericKey(na) === "0") return int(0);
      return numberNode(Math.tanh(toNumber(na)));
    }
    if (!simplify) throw new TypeError(`Tanh requires a numeric argument: ${headName(expr.head)}`);
    // Odd symmetry — tanh(-x) = -tanh(x).
    if (arg.kind === "apply" && equals(arg.head, NEG) && arg.args.length === 1)
      return vm.eval(app(NEG, [app(TANH, [arg.args[0]])]));
    // Arc-cancellation — tanh(atanh(x)) = x.
    if (arg.kind === "apply" && equals(arg.head, ATANH) && arg.args.length === 1)
      return arg.args[0];
    return expr;
  });

  // ----- Phase 32: Asinh — odd symmetry -----
  table.set(ASINH.name, (vm, expr) => {
    if (expr.args.length !== 1) return expr;
    const arg = expr.args[0];
    const na = toNumeric(arg);
    if (na !== undefined) {
      if (numericKey(na) === "0") return int(0);
      return numberNode(Math.asinh(toNumber(na)));
    }
    if (!simplify) throw new TypeError(`Asinh requires a numeric argument: ${headName(expr.head)}`);
    // Odd symmetry — asinh(-x) = -asinh(x).
    if (arg.kind === "apply" && equals(arg.head, NEG) && arg.args.length === 1)
      return vm.eval(app(NEG, [app(ASINH, [arg.args[0]])]));
    return expr;
  });

  // ----- Acosh — numeric fold only (domain [1,∞), no symmetry rule) -----
  table.set(ACOSH.name, elementary("Acosh", Math.acosh, new Map([["1", int(0)]]), simplify));

  // ----- Phase 32: Atanh — odd symmetry -----
  table.set(ATANH.name, (vm, expr) => {
    if (expr.args.length !== 1) return expr;
    const arg = expr.args[0];
    const na = toNumeric(arg);
    if (na !== undefined) {
      if (numericKey(na) === "0") return int(0);
      return numberNode(Math.atanh(toNumber(na)));
    }
    if (!simplify) throw new TypeError(`Atanh requires a numeric argument: ${headName(expr.head)}`);
    // Odd symmetry — atanh(-x) = -atanh(x).
    if (arg.kind === "apply" && equals(arg.head, NEG) && arg.args.length === 1)
      return vm.eval(app(NEG, [app(ATANH, [arg.args[0]])]));
    return expr;
  });
  table.set(COTH.name, elementary("Coth", (x) => Math.cosh(x) / Math.sinh(x), new Map(), simplify));
  table.set(SECH.name, elementary("Sech", (x) => 1 / Math.cosh(x), new Map([["0", int(1)]]), simplify));
  table.set(CSCH.name, elementary("Csch", (x) => 1 / Math.sinh(x), new Map(), simplify));

  table.set(EQUAL.name, (_vm, expr) => boolNode(equals(binaryArgs(expr)[0], binaryArgs(expr)[1])));
  table.set(NOT_EQUAL.name, (_vm, expr) => boolNode(!equals(binaryArgs(expr)[0], binaryArgs(expr)[1])));
  table.set(LESS.name, compare("Less", (a, b) => a < b, simplify));
  table.set(GREATER.name, compare("Greater", (a, b) => a > b, simplify));
  table.set(LESS_EQUAL.name, compare("LessEqual", (a, b) => a <= b, simplify));
  table.set(GREATER_EQUAL.name, compare("GreaterEqual", (a, b) => a >= b, simplify));

  table.set(AND.name, (vm, expr) => {
    const [a, b] = binaryArgs(expr);
    const av = truthy(a);
    if (av === false) return FALSE;
    if (av === true) return vm.eval(b);
    if (!simplify) throw new TypeError(`And requires boolean arguments: ${formatHead(expr)}`);
    return expr;
  });
  table.set(OR.name, (vm, expr) => {
    const [a, b] = binaryArgs(expr);
    const av = truthy(a);
    if (av === true) return TRUE;
    if (av === false) return vm.eval(b);
    if (!simplify) throw new TypeError(`Or requires boolean arguments: ${formatHead(expr)}`);
    return expr;
  });
  table.set(NOT.name, (_vm, expr) => {
    const [a] = unaryArgs(expr);
    const av = truthy(a);
    if (av !== undefined) return boolNode(!av);
    if (!simplify) throw new TypeError(`Not requires a boolean argument: ${formatHead(expr)}`);
    return expr;
  });
  table.set(IF.name, (vm, expr) => {
    if (expr.args.length !== 3) {
      throw new ArityError(`If expects 3 arguments, got ${expr.args.length}`);
    }
    const condition = vm.eval(expr.args[0]);
    const cv = truthy(condition);
    if (cv === true) return vm.eval(expr.args[1]);
    if (cv === false) return vm.eval(expr.args[2]);
    if (!simplify) throw new TypeError(`If condition must be boolean: ${formatHead(expr)}`);
    return app(IF, [condition, expr.args[1], expr.args[2]]);
  });

  table.set(ASSIGN.name, (vm, expr) => {
    const [lhs, rhs] = binaryArgs(expr);
    if (lhs.kind !== "symbol") {
      throw new TypeError("Assign lhs must be a symbol");
    }
    const value = vm.eval(rhs);
    vm.backend.bind(lhs.name, value);
    return value;
  });
  table.set(DEFINE.name, (vm, expr) => {
    if (expr.args.length !== 3) {
      throw new ArityError(`Define expects 3 arguments, got ${expr.args.length}`);
    }
    const [name] = expr.args;
    if (name.kind !== "symbol") {
      throw new TypeError("Define name must be a symbol");
    }
    vm.backend.bind(name.name, expr);
    return name;
  });
  table.set(LIST.name, (_vm, expr) => expr);
  table.set(FACTOR.name, factorHandler);
  // Track B1 (Phase 1 partial-fraction decomposition).  Registered by string
  // name because ``Apart`` has no exported constant in symbolic-ir.
  if (simplify) {
    table.set("Apart", apartHandler);
  }
  if (simplify) {
    table.set(D.name, differentiate());
    table.set(INTEGRATE.name, integrate());
    // Track G2 (TS port): `Assume(rel)` records a sign / equality fact
    // on the VM's `assumptions` store; `Forget(rel)` removes one;
    // `ForgetAll()` clears the whole table.  Returning the relation
    // verbatim mirrors the Python handler's behaviour and lets MACSYMA
    // chains like `Assume(x > 0); Sqrt(x^2)` thread the assertion
    // through without producing an extraneous result expression.
    table.set("Assume", (vm, expr) => {
      if (expr.args.length === 1) {
        vm.assumptions.assumeRelation(expr.args[0]);
      }
      return expr;
    });
    table.set("Forget", (vm, expr) => {
      if (expr.args.length === 1) {
        vm.assumptions.forgetRelation(expr.args[0]);
      }
      return expr;
    });
    table.set("ForgetAll", (vm, expr) => {
      vm.assumptions.forgetAll();
      return expr;
    });
  }

  return table;
}

function factorHandler(vm: VM, expr: IRApply): IRNode {
  if (expr.args.length !== 1) return expr;
  const inner = expr.args[0];
  const variable = findVariable(inner);
  if (variable === undefined) return inner;

  const coeffs = irToIntegerPoly(inner, variable);
  if (coeffs === undefined) {
    const perfectCube = extractMultivariatePerfectCube(inner);
    if (perfectCube !== undefined) return perfectCube;
    const cubicIdentity = extractMultivariateCubicIdentity(inner);
    if (cubicIdentity !== undefined) return cubicIdentity;
    const difference = extractMultivariateDifferenceOfSquares(inner);
    if (difference !== undefined) return difference;
    const perfectSquare = extractMultivariatePerfectSquare(inner);
    if (perfectSquare !== undefined) return perfectSquare;
    const grouping = extractMultivariateGrouping(inner);
    if (grouping !== undefined) return grouping;
    const commonFactored = extractCommonSymbolicFactor(inner);
    if (commonFactored !== undefined) return vm.eval(commonFactored);
    // Generic bivariate Hensel lifting fallback — for multivariate inputs
    // the pattern handlers above can't recognise.  Mirrors the Python
    // ``_try_bivariate_hensel_ir`` glue in ``symbolic-vm/cas_handlers.py``.
    const hensel = tryBivariateHenselIr(inner);
    if (hensel !== undefined) return vm.eval(hensel);
    // n-variate (n ≥ 3) Hensel — Track K2.  Generalised algorithmic
    // fallback for tri- and higher-variate polynomials (e.g.,
    // x³ + y³ + z³ − 3xyz = (x+y+z)(…)).  Returns undefined for
    // transcendentals, foreign symbols, or when the iterated lift can't
    // pin down a factorisation.
    const nHensel = tryNVariateHenselIr(inner);
    if (nHensel !== undefined) return vm.eval(nHensel);
    return expr;
  }

  const [content, factors] = factorIntegerPolynomial(coeffs);
  if (factors.length === 0) return inner;
  if (
    coeffs.length > 2
    && content === 1n
    && factors.length === 1
    && factors[0][1] === 1
    && polyEquals(factors[0][0], coeffs)
  ) {
    return expr;
  }
  return factorResultToIr(content, factors, variable);
}

function findVariable(node: IRNode): IRSymbol | undefined {
  if (node.kind === "symbol") {
    return node.name.startsWith("%") ? undefined : node;
  }
  if (node.kind === "apply") {
    for (const arg of node.args) {
      const found = findVariable(arg);
      if (found !== undefined) return found;
    }
  }
  return undefined;
}

function irToIntegerPoly(node: IRNode, variable: IRSymbol): bigint[] | undefined {
  if (node.kind === "integer") return [node.value];
  if (equals(node, variable)) return [0n, 1n];
  if (node.kind !== "apply" || node.head.kind !== "symbol") return undefined;

  if (node.head.name === ADD.name) {
    return node.args.reduce<bigint[] | undefined>((acc, arg) => {
      const p = irToIntegerPoly(arg, variable);
      return acc === undefined || p === undefined ? undefined : polyAdd(acc, p);
    }, [0n]);
  }
  if (node.head.name === SUB.name && node.args.length === 2) {
    const a = irToIntegerPoly(node.args[0], variable);
    const b = irToIntegerPoly(node.args[1], variable);
    return a === undefined || b === undefined ? undefined : polySub(a, b);
  }
  if (node.head.name === MUL.name) {
    return node.args.reduce<bigint[] | undefined>((acc, arg) => {
      const p = irToIntegerPoly(arg, variable);
      return acc === undefined || p === undefined ? undefined : polyMul(acc, p);
    }, [1n]);
  }
  if (
    node.head.name === POW.name
    && node.args.length === 2
    && node.args[1].kind === "integer"
    && node.args[1].value >= 0n
  ) {
    const base = irToIntegerPoly(node.args[0], variable);
    return base === undefined ? undefined : polyPow(base, node.args[1].value);
  }
  return undefined;
}

// ---------------------------------------------------------------------------
// Bivariate Hensel-lifting IR glue.  Mirrors the Python ``_try_bivariate_*``
// helpers in ``symbolic-vm/cas_handlers.py``.
// ---------------------------------------------------------------------------

function findTwoVariables(node: IRNode): [IRSymbol, IRSymbol] | undefined {
  const seen: IRSymbol[] = [];
  function walk(n: IRNode): boolean {
    if (n.kind === "symbol") {
      if (n.name.startsWith("%")) return true;
      if (!seen.some((s) => s.name === n.name)) {
        seen.push(n);
        if (seen.length > 2) return false;
      }
      return true;
    }
    if (n.kind === "apply") {
      for (const arg of n.args) {
        if (!walk(arg)) return false;
      }
    }
    return true;
  }
  if (!walk(node)) return undefined;
  if (seen.length !== 2) return undefined;
  return [seen[0], seen[1]];
}

function biKey(i: number, j: number): string {
  return `${i},${j}`;
}

function biParseKey(k: string): [number, number] {
  const idx = k.indexOf(",");
  return [Number(k.slice(0, idx)), Number(k.slice(idx + 1))];
}

function biAddInPlace(acc: BiPoly, other: BiPoly): BiPoly {
  for (const [k, v] of other) {
    const cur = acc.get(k);
    acc.set(k, cur === undefined ? v : cur.add(v));
  }
  for (const [k, v] of [...acc]) {
    if (v.isZero()) acc.delete(k);
  }
  return acc;
}

function biMulMaps(a: BiPoly, b: BiPoly): BiPoly {
  const out: BiPoly = new Map();
  for (const [k1, c1] of a) {
    const [i1, j1] = biParseKey(k1);
    for (const [k2, c2] of b) {
      const [i2, j2] = biParseKey(k2);
      const k = biKey(i1 + i2, j1 + j2);
      const cur = out.get(k);
      out.set(k, cur === undefined ? c1.mul(c2) : cur.add(c1.mul(c2)));
    }
  }
  for (const [k, v] of [...out]) {
    if (v.isZero()) out.delete(k);
  }
  return out;
}

function irToBipoly(node: IRNode, x: IRSymbol, y: IRSymbol): BiPoly | undefined {
  // Numeric literals.
  if (node.kind === "integer") {
    if (node.value === 0n) return new Map();
    return new Map([[biKey(0, 0), BiRational.fromInt(node.value)]]);
  }
  if (node.kind === "rational") {
    return new Map([[biKey(0, 0), new BiRational(node.numer, node.denom)]]);
  }
  if (node.kind === "float") return undefined;
  if (node.kind === "string") return undefined;
  if (node.kind === "symbol") {
    if (node.name.startsWith("%")) return undefined;
    if (node.name === x.name) return new Map([[biKey(1, 0), BiRational.ONE]]);
    if (node.name === y.name) return new Map([[biKey(0, 1), BiRational.ONE]]);
    return undefined;
  }
  // Apply nodes.
  const apply = node;
  if (apply.head.kind !== "symbol") return undefined;
  const head = apply.head.name;
  if (head === ADD.name) {
    const acc: BiPoly = new Map();
    for (const arg of apply.args) {
      const sub = irToBipoly(arg, x, y);
      if (sub === undefined) return undefined;
      biAddInPlace(acc, sub);
    }
    return acc;
  }
  if (head === SUB.name && apply.args.length === 2) {
    const a = irToBipoly(apply.args[0], x, y);
    const b = irToBipoly(apply.args[1], x, y);
    if (a === undefined || b === undefined) return undefined;
    const negB: BiPoly = new Map();
    for (const [k, v] of b) negB.set(k, v.neg());
    biAddInPlace(a, negB);
    return a;
  }
  if (head === NEG.name && apply.args.length === 1) {
    const sub = irToBipoly(apply.args[0], x, y);
    if (sub === undefined) return undefined;
    const out: BiPoly = new Map();
    for (const [k, v] of sub) {
      if (!v.isZero()) out.set(k, v.neg());
    }
    return out;
  }
  if (head === MUL.name) {
    let acc: BiPoly = new Map([[biKey(0, 0), BiRational.ONE]]);
    for (const arg of apply.args) {
      const sub = irToBipoly(arg, x, y);
      if (sub === undefined) return undefined;
      acc = biMulMaps(acc, sub);
    }
    return acc;
  }
  if (head === POW.name && apply.args.length === 2) {
    const expNode = apply.args[1];
    if (expNode.kind !== "integer") return undefined;
    const eBig = expNode.value;
    if (eBig < 0n) return undefined;
    const base = irToBipoly(apply.args[0], x, y);
    if (base === undefined) return undefined;
    if (eBig === 0n) return new Map([[biKey(0, 0), BiRational.ONE]]);
    let result = base;
    const e = Number(eBig);
    for (let i = 1; i < e; i += 1) result = biMulMaps(result, base);
    return result;
  }
  return undefined;
}

function bipolyToIr(p: BiPoly, x: IRSymbol, y: IRSymbol): IRNode {
  if (p.size === 0) return int(0);

  function monomialNode(i: number, j: number, c: BiRational): IRNode {
    const parts: IRNode[] = [];
    const isConstantTerm = i === 0 && j === 0;
    if (!c.equals(BiRational.ONE) || isConstantTerm) {
      if (c.denom === 1n) {
        parts.push(int(c.numer));
      } else {
        parts.push(rational(c.numer, c.denom));
      }
    }
    if (i > 0) {
      parts.push(i === 1 ? x : app(POW, [x, int(i)]));
    }
    if (j > 0) {
      parts.push(j === 1 ? y : app(POW, [y, int(j)]));
    }
    if (parts.length === 1) return parts[0];
    return app(MUL, parts);
  }

  // Sort by descending total degree, then by descending i, then j.
  const keys = [...p.keys()].sort((a, b) => {
    const [ai, aj] = biParseKey(a);
    const [bi, bj] = biParseKey(b);
    const aTot = ai + aj;
    const bTot = bi + bj;
    if (aTot !== bTot) return bTot - aTot;
    if (ai !== bi) return bi - ai;
    return bj - aj;
  });
  const terms = keys.map((k) => {
    const [i, j] = biParseKey(k);
    return monomialNode(i, j, p.get(k)!);
  });
  if (terms.length === 1) return terms[0];
  return app(ADD, terms);
}

function tryBivariateHenselIr(inner: IRNode): IRNode | undefined {
  const vars = findTwoVariables(inner);
  if (vars === undefined) return undefined;
  const [x, y] = vars;
  const bipoly = irToBipoly(inner, x, y);
  if (bipoly === undefined) return undefined;
  const factors = tryBivariateHensel(bipoly);
  if (factors === null || factors.length < 2) return undefined;
  const factorNodes = factors.map((f) => bipolyToIr(f, x, y));
  if (factorNodes.length === 1) return factorNodes[0];
  return app(MUL, factorNodes);
}

// ---------------------------------------------------------------------------
// n-variate (n ≥ 3) Hensel-lifting IR glue — Track K2.  Mirrors the
// Python ``_find_n_variables``, ``_ir_to_npoly``, ``_npoly_to_ir``, and
// ``_try_n_variate_hensel_ir`` helpers in ``cas_handlers.py``.
//
// Output convention: LEFT-NESTED BINARY Add/Mul.  The symbolic-vm
// primitive Add/Mul handlers are strictly binary, so an
// ``IRApply(ADD, (a, b, c))`` with three or more children would crash
// when re-evaluated.  We mirror the cubic-identity handler's nesting
// convention: ``Add(Add(a, b), c)`` for three terms, etc.
// ---------------------------------------------------------------------------

const MAX_N_VARS = 8;

function findNVariables(node: IRNode): IRSymbol[] | undefined {
  const seen: IRSymbol[] = [];
  function walk(n: IRNode): boolean {
    if (n.kind === "symbol") {
      if (n.name.startsWith("%")) return true;
      if (!seen.some((s) => s.name === n.name)) {
        seen.push(n);
        if (seen.length > MAX_N_VARS) return false;
      }
      return true;
    }
    if (n.kind === "apply") {
      for (const arg of n.args) {
        if (!walk(arg)) return false;
      }
    }
    return true;
  }
  if (!walk(node)) return undefined;
  if (seen.length === 0) return undefined;
  return seen;
}

function nKeyJoin(tup: number[]): string {
  return tup.join(",");
}

function nParseKeyJoin(k: string, numVars: number): number[] {
  const out: number[] = [];
  let start = 0;
  for (let i = 0; i < numVars - 1; i += 1) {
    const idx = k.indexOf(",", start);
    out.push(Number(k.slice(start, idx)));
    start = idx + 1;
  }
  out.push(Number(k.slice(start)));
  return out;
}

function nAddInto(acc: NPoly, other: NPoly): void {
  for (const [k, v] of other) {
    const cur = acc.get(k);
    acc.set(k, cur === undefined ? v : cur.add(v));
  }
  for (const [k, v] of [...acc]) {
    if (v.isZero()) acc.delete(k);
  }
}

function nMulMaps(a: NPoly, b: NPoly, numVars: number): NPoly {
  const out: NPoly = new Map();
  for (const [k1, c1] of a) {
    if (c1.isZero()) continue;
    const t1 = nParseKeyJoin(k1, numVars);
    for (const [k2, c2] of b) {
      if (c2.isZero()) continue;
      const t2 = nParseKeyJoin(k2, numVars);
      const t: number[] = new Array<number>(numVars);
      for (let i = 0; i < numVars; i += 1) t[i] = t1[i] + t2[i];
      const k = nKeyJoin(t);
      const cur = out.get(k);
      out.set(k, cur === undefined ? c1.mul(c2) : cur.add(c1.mul(c2)));
    }
  }
  for (const [k, v] of [...out]) {
    if (v.isZero()) out.delete(k);
  }
  return out;
}

function irToNpoly(node: IRNode, vars: IRSymbol[]): NPoly | undefined {
  const numVars = vars.length;
  const zeroKey = nKeyJoin(new Array<number>(numVars).fill(0));
  const varIndex = new Map<string, number>();
  vars.forEach((v, i) => varIndex.set(v.name, i));

  function unitFor(v: IRSymbol): string {
    const i = varIndex.get(v.name);
    if (i === undefined) throw new Error("unitFor on non-tracked var");
    const tup = new Array<number>(numVars).fill(0);
    tup[i] = 1;
    return nKeyJoin(tup);
  }

  function walk(n: IRNode): NPoly | undefined {
    if (n.kind === "integer") {
      if (n.value === 0n) return new Map();
      return new Map([[zeroKey, BiRational.fromInt(n.value)]]);
    }
    if (n.kind === "rational") {
      return new Map([[zeroKey, new BiRational(n.numer, n.denom)]]);
    }
    if (n.kind === "float" || n.kind === "string") return undefined;
    if (n.kind === "symbol") {
      if (n.name.startsWith("%")) return undefined;
      if (varIndex.has(n.name)) return new Map([[unitFor(n), BiRational.ONE]]);
      return undefined;
    }
    const apply = n;
    if (apply.head.kind !== "symbol") return undefined;
    const head = apply.head.name;
    if (head === ADD.name) {
      const acc: NPoly = new Map();
      for (const arg of apply.args) {
        const sub = walk(arg);
        if (sub === undefined) return undefined;
        nAddInto(acc, sub);
      }
      return acc;
    }
    if (head === SUB.name && apply.args.length === 2) {
      const a = walk(apply.args[0]);
      const b = walk(apply.args[1]);
      if (a === undefined || b === undefined) return undefined;
      const negB: NPoly = new Map();
      for (const [k, v] of b) negB.set(k, v.neg());
      nAddInto(a, negB);
      return a;
    }
    if (head === NEG.name && apply.args.length === 1) {
      const sub = walk(apply.args[0]);
      if (sub === undefined) return undefined;
      const out: NPoly = new Map();
      for (const [k, v] of sub) {
        if (!v.isZero()) out.set(k, v.neg());
      }
      return out;
    }
    if (head === MUL.name) {
      let acc: NPoly = new Map([[zeroKey, BiRational.ONE]]);
      for (const arg of apply.args) {
        const sub = walk(arg);
        if (sub === undefined) return undefined;
        acc = nMulMaps(acc, sub, numVars);
      }
      return acc;
    }
    if (head === POW.name && apply.args.length === 2) {
      const expNode = apply.args[1];
      if (expNode.kind !== "integer") return undefined;
      const eBig = expNode.value;
      if (eBig < 0n) return undefined;
      const base = walk(apply.args[0]);
      if (base === undefined) return undefined;
      if (eBig === 0n) return new Map([[zeroKey, BiRational.ONE]]);
      let result = base;
      const e = Number(eBig);
      for (let i = 1; i < e; i += 1) result = nMulMaps(result, base, numVars);
      return result;
    }
    return undefined;
  }

  return walk(node);
}

/** Left-fold a list of children into nested binary IRApply nodes. */
function foldBinary(head: IRSymbol, parts: IRNode[]): IRNode {
  if (parts.length === 0) throw new Error("foldBinary requires at least one node");
  let result = parts[0];
  for (let i = 1; i < parts.length; i += 1) {
    result = app(head, [result, parts[i]]);
  }
  return result;
}

function npolyToIr(p: NPoly, vars: IRSymbol[]): IRNode {
  if (p.size === 0) return int(0);
  const numVars = vars.length;

  function monomialNode(tup: number[], c: BiRational): IRNode {
    const parts: IRNode[] = [];
    const allZero = tup.every((e) => e === 0);
    if (!c.equals(BiRational.ONE) || allZero) {
      if (c.denom === 1n) parts.push(int(c.numer));
      else parts.push(rational(c.numer, c.denom));
    }
    for (let i = 0; i < numVars; i += 1) {
      const e = tup[i];
      if (e <= 0) continue;
      const v = vars[i];
      parts.push(e === 1 ? v : app(POW, [v, int(e)]));
    }
    if (parts.length === 0) return int(1);
    return foldBinary(MUL, parts);
  }

  // Sort: descending total degree, then lex on negated exponents (matches
  // Python ``_npoly_to_ir`` key order).
  const keys = [...p.keys()].sort((a, b) => {
    const ta = nParseKeyJoin(a, numVars);
    const tb = nParseKeyJoin(b, numVars);
    const sa = ta.reduce((x, y) => x + y, 0);
    const sb = tb.reduce((x, y) => x + y, 0);
    if (sa !== sb) return sb - sa;
    for (let i = 0; i < numVars; i += 1) {
      if (ta[i] !== tb[i]) return tb[i] - ta[i];
    }
    return 0;
  });
  const terms = keys.map((k) => monomialNode(nParseKeyJoin(k, numVars), p.get(k)!));
  return foldBinary(ADD, terms);
}

function tryNVariateHenselIr(inner: IRNode): IRNode | undefined {
  const vars = findNVariables(inner);
  if (vars === undefined || vars.length < 2) return undefined;
  const npoly = irToNpoly(inner, vars);
  if (npoly === undefined) return undefined;
  const factors = tryNVariateHensel(npoly, vars.length);
  if (factors === null || factors.length < 2) return undefined;
  const factorNodes = factors.map((f) => npolyToIr(f, vars));
  if (factorNodes.length === 1) return factorNodes[0];
  // Left-nested binary Mul output for binary-handler compatibility.
  return foldBinary(MUL, factorNodes);
}

function factorResultToIr(content: bigint, factors: Array<[bigint[], number]>, variable: IRSymbol): IRNode {
  const pieces: IRNode[] = [];
  if (content !== 1n) pieces.push(int(content));
  for (const [coeffs, multiplicity] of factors) {
    const factor = polyToIr(coeffs, variable);
    pieces.push(multiplicity === 1 ? factor : app(POW, [factor, int(multiplicity)]));
  }
  return multiplyNodes(pieces);
}

function polyToIr(coeffs: readonly bigint[], variable: IRSymbol): IRNode {
  const terms: IRNode[] = [];
  for (let degree = 0; degree < coeffs.length; degree += 1) {
    const coeff = coeffs[degree];
    if (coeff === 0n) continue;
    terms.push(monomialToIr(coeff, degree, variable));
  }
  return terms.length === 0 ? int(0) : addNodes(terms);
}

function monomialToIr(coeff: bigint, degree: number, variable: IRSymbol): IRNode {
  if (degree === 0) return int(coeff);
  const power = degree === 1 ? variable : app(POW, [variable, int(degree)]);
  if (coeff === 1n) return power;
  if (coeff === -1n) return app(MUL, [int(-1), power]);
  return app(MUL, [int(coeff), power]);
}

function flattenFactorTerms(node: IRNode): IRNode[] {
  if (node.kind === "apply" && equals(node.head, MUL)) {
    return node.args.flatMap((arg) => flattenFactorTerms(arg));
  }
  return [node];
}

function flattenAddTerms(node: IRNode): IRNode[] {
  if (node.kind === "apply" && equals(node.head, ADD)) {
    return node.args.flatMap((arg) => flattenAddTerms(arg));
  }
  if (node.kind === "apply" && equals(node.head, SUB) && node.args.length === 2) {
    return [...flattenAddTerms(node.args[0]), negateIr(node.args[1])];
  }
  return [node];
}

function negateIr(node: IRNode): IRNode {
  if (node.kind === "integer") return int(-node.value);
  if (node.kind === "apply" && equals(node.head, NEG) && node.args.length === 1) return node.args[0];
  return app(MUL, [int(-1), node]);
}

function powNode(base: IRNode, exponent: number): IRNode {
  return exponent === 1 ? base : app(POW, [base, int(exponent)]);
}

function addNodes(nodes: readonly IRNode[]): IRNode {
  if (nodes.length === 0) return int(0);
  if (nodes.length === 1) return nodes[0];
  return app(ADD, nodes);
}

function multiplyNodes(nodes: readonly IRNode[]): IRNode {
  if (nodes.length === 0) return int(1);
  if (nodes.length === 1) return nodes[0];
  return app(MUL, nodes);
}

type FactorPower = { readonly base: IRNode; exponent: number };

type CoefficientPowers = {
  readonly coefficient: bigint;
  readonly powers: Map<string, FactorPower>;
};

function splitCommonFactorTerm(node: IRNode): Map<string, FactorPower> {
  const powers = new Map<string, FactorPower>();
  for (const factor of flattenFactorTerms(node)) {
    if (factor.kind === "integer") continue;
    if (factor.kind === "symbol" && factor.name.startsWith("%")) continue;

    if (factor.kind === "apply" && equals(factor.head, POW) && factor.args.length === 2) {
      const [base, exponent] = factor.args;
      if (exponent.kind === "integer" && exponent.value > 0n) {
        addPower(powers, base, Number(exponent.value));
        continue;
      }
    }
    addPower(powers, factor, 1);
  }
  return powers;
}

function splitIntegerCoefficientAndPowers(node: IRNode): CoefficientPowers | undefined {
  let coefficient = 1n;
  const powers = new Map<string, FactorPower>();

  const visitFactor = (factor: IRNode): boolean => {
    if (factor.kind === "integer") {
      coefficient *= factor.value;
      return true;
    }
    if (factor.kind === "apply" && equals(factor.head, NEG) && factor.args.length === 1) {
      coefficient *= -1n;
      for (const nested of flattenFactorTerms(factor.args[0])) {
        if (!visitFactor(nested)) return false;
      }
      return true;
    }
    if (factor.kind === "symbol" && factor.name.startsWith("%")) return false;

    if (factor.kind === "apply" && equals(factor.head, POW) && factor.args.length === 2) {
      const [base, exponent] = factor.args;
      if (exponent.kind === "integer" && exponent.value > 0n) {
        addPower(powers, base, Number(exponent.value));
        return true;
      }
    }
    addPower(powers, factor, 1);
    return true;
  };

  for (const factor of flattenFactorTerms(node)) {
    if (!visitFactor(factor)) return undefined;
  }
  return { coefficient, powers };
}

function termFromIntegerCoefficientAndPowers(coefficient: bigint, powers: ReadonlyMap<string, FactorPower>): IRNode {
  const terms: IRNode[] = [];
  if (coefficient !== 1n || powers.size === 0) terms.push(int(coefficient));
  terms.push(...[...powers.values()]
    .sort((a, b) => nodeKey(a.base).localeCompare(nodeKey(b.base)))
    .map((power) => powNode(power.base, power.exponent)));
  return multiplyNodes(terms);
}

function addPower(powers: Map<string, FactorPower>, base: IRNode, exponent: number): void {
  const key = nodeKey(base);
  const existing = powers.get(key);
  powers.set(key, { base, exponent: (existing?.exponent ?? 0) + exponent });
}

function removeCommonFactor(node: IRNode, common: ReadonlyMap<string, FactorPower>): IRNode {
  const remaining = new Map<string, number>();
  for (const [key, power] of common.entries()) remaining.set(key, power.exponent);

  const rebuilt: IRNode[] = [];
  for (const factor of flattenFactorTerms(node)) {
    if (factor.kind === "apply" && equals(factor.head, POW) && factor.args.length === 2) {
      const [base, exponent] = factor.args;
      if (exponent.kind === "integer" && exponent.value > 0n) {
        const key = nodeKey(base);
        const take = Math.min(Number(exponent.value), remaining.get(key) ?? 0);
        if (take > 0) {
          remaining.set(key, (remaining.get(key) ?? 0) - take);
          const leftover = Number(exponent.value) - take;
          if (leftover > 0) rebuilt.push(powNode(base, leftover));
          continue;
        }
      }
    }
    const key = nodeKey(factor);
    const take = remaining.get(key) ?? 0;
    if (take > 0) {
      remaining.set(key, take - 1);
    } else {
      rebuilt.push(factor);
    }
  }
  return multiplyNodes(rebuilt);
}

function gcdBigInt(a: bigint, b: bigint): bigint {
  let x = a < 0n ? -a : a;
  let y = b < 0n ? -b : b;
  while (y !== 0n) {
    const next = x % y;
    x = y;
    y = next;
  }
  return x;
}

function maybeFactorResidual(residual: IRNode): IRNode {
  const variable = findVariable(residual);
  return variable !== undefined && irToIntegerPoly(residual, variable) !== undefined
    ? app(FACTOR, [residual])
    : residual;
}

function extractCommonSymbolicFactor(inner: IRNode): IRNode | undefined {
  const terms = flattenAddTerms(inner);
  if (terms.length < 2) return undefined;

  const parsed = terms.map((term) => splitIntegerCoefficientAndPowers(term));
  if (parsed.some((term) => term === undefined)) return undefined;

  const parsedTerms = parsed as CoefficientPowers[];
  const coefficients = parsedTerms.map((term) => term.coefficient);
  let commonCoefficient = coefficients.reduce(
    (acc, coefficient) => gcdBigInt(acc, coefficient),
    0n,
  );
  if (commonCoefficient !== 0n && coefficients.every((coefficient) => coefficient < 0n)) {
    commonCoefficient = -commonCoefficient;
  }
  if (commonCoefficient === 0n) commonCoefficient = 1n;

  const common = new Map(parsedTerms[0].powers);
  for (const { powers } of parsedTerms.slice(1)) {
    for (const [key, power] of [...common.entries()]) {
      const shared = Math.min(power.exponent, powers.get(key)?.exponent ?? 0);
      if (shared > 0) {
        common.set(key, { base: power.base, exponent: shared });
      } else {
        common.delete(key);
      }
    }
  }
  if (commonCoefficient === 1n && common.size === 0) return undefined;

  const commonFactor = termFromIntegerCoefficientAndPowers(commonCoefficient, common);
  const residualTerms = parsedTerms.map(({ coefficient, powers }) => {
    const residualPowers = new Map<string, FactorPower>();
    for (const [key, power] of powers.entries()) {
      const exponent = power.exponent - (common.get(key)?.exponent ?? 0);
      if (exponent > 0) residualPowers.set(key, { base: power.base, exponent });
    }
    return termFromIntegerCoefficientAndPowers(coefficient / commonCoefficient, residualPowers);
  });
  return app(MUL, [commonFactor, maybeFactorResidual(addNodes(residualTerms))]);
}

function extractMultivariatePerfectSquare(inner: IRNode): IRNode | undefined {
  const terms = flattenAddTerms(inner);
  if (terms.length !== 3) return undefined;

  const parsed = terms.map((term) => splitIntegerCoefficientAndPowers(term));
  if (parsed.some((term) => term === undefined)) return undefined;

  const squares: FactorPower[] = [];
  let cross: { readonly coefficient: bigint; readonly factors: readonly [FactorPower, FactorPower] } | undefined;
  for (const parsedTerm of parsed) {
    if (parsedTerm === undefined) return undefined;
    const { coefficient, powers } = parsedTerm;
    if (coefficient === 1n && powers.size === 1) {
      const [power] = [...powers.values()];
      if (power.exponent === 2) {
        squares.push(power);
        continue;
      }
    }
    if ((coefficient === 2n || coefficient === -2n) && powers.size === 2) {
      const factors = [...powers.values()];
      if (factors[0].exponent === 1 && factors[1].exponent === 1) {
        cross = { coefficient, factors: [factors[0], factors[1]] };
        continue;
      }
    }
    return undefined;
  }

  if (squares.length !== 2 || cross === undefined) return undefined;
  const squareKeys = new Set(squares.map((power) => nodeKey(power.base)));
  const crossKeys = new Set(cross.factors.map((power) => nodeKey(power.base)));
  if (
    squareKeys.size !== 2
    || crossKeys.size !== 2
    || [...squareKeys].some((key) => !crossKeys.has(key))
  ) {
    return undefined;
  }

  const [first, second] = squares;
  const base = cross.coefficient > 0n
    ? app(ADD, [first.base, second.base])
    : app(SUB, [first.base, second.base]);
  return app(POW, [base, int(2)]);
}

function extractMultivariateDifferenceOfSquares(inner: IRNode): IRNode | undefined {
  const terms = flattenAddTerms(inner);
  if (terms.length !== 2) return undefined;

  const parsed = terms.map((term) => splitIntegerCoefficientAndPowers(term));
  if (parsed.some((term) => term === undefined)) return undefined;

  let positiveSquare: IRNode | undefined;
  let negativeSquare: IRNode | undefined;
  for (const parsedTerm of parsed) {
    if (parsedTerm === undefined) return undefined;
    const { coefficient, powers } = parsedTerm;
    if (powers.size !== 1) return undefined;
    const [power] = [...powers.values()];
    if (power.exponent !== 2) return undefined;
    if (coefficient === 1n) {
      positiveSquare = power.base;
    } else if (coefficient === -1n) {
      negativeSquare = power.base;
    } else {
      return undefined;
    }
  }

  if (positiveSquare === undefined || negativeSquare === undefined) return undefined;
  return app(MUL, [
    app(SUB, [positiveSquare, negativeSquare]),
    app(ADD, [positiveSquare, negativeSquare]),
  ]);
}

function extractMultivariateCubicIdentity(inner: IRNode): IRNode | undefined {
  const terms = flattenAddTerms(inner);
  if (terms.length !== 2) return undefined;

  const parsed = terms.map((term) => splitIntegerCoefficientAndPowers(term));
  if (parsed.some((term) => term === undefined)) return undefined;

  const cubes: Array<{ readonly coefficient: bigint; readonly base: IRNode }> = [];
  for (const parsedTerm of parsed) {
    if (parsedTerm === undefined) return undefined;
    const { coefficient, powers } = parsedTerm;
    if ((coefficient !== 1n && coefficient !== -1n) || powers.size !== 1) return undefined;
    const [power] = [...powers.values()];
    if (power.exponent !== 3) return undefined;
    cubes.push({ coefficient, base: power.base });
  }

  const signs = cubes.map((cube) => cube.coefficient).join(",");
  let first: IRNode;
  let second: IRNode;
  let linear: IRNode;
  let middle: IRNode;
  if (signs === "1,1") {
    first = cubes[0].base;
    second = cubes[1].base;
    linear = app(ADD, [first, second]);
    middle = app(MUL, [int(-1), app(MUL, [first, second])]);
  } else if (cubes.some((cube) => cube.coefficient === 1n) && cubes.some((cube) => cube.coefficient === -1n)) {
    const positive = cubes.find((cube) => cube.coefficient === 1n);
    const negative = cubes.find((cube) => cube.coefficient === -1n);
    if (positive === undefined || negative === undefined) return undefined;
    first = positive.base;
    second = negative.base;
    linear = app(SUB, [first, second]);
    middle = app(MUL, [first, second]);
  } else {
    return undefined;
  }

  return app(MUL, [
    linear,
    app(ADD, [
      app(ADD, [app(POW, [first, int(2)]), middle]),
      app(POW, [second, int(2)]),
    ]),
  ]);
}

function extractMultivariatePerfectCube(inner: IRNode): IRNode | undefined {
  // Recognise (a±b)^3 perfect-cube expansions.
  //
  //   a^3 + 3·a^2·b + 3·a·b^2 + b^3  →  (a + b)^3   [sum cube]
  //   a^3 − 3·a^2·b + 3·a·b^2 − b^3  →  (a − b)^3   [difference cube]
  //
  // Requires exactly 4 additive terms: 2 pure-cube terms (|coeff|=1, one
  // variable, exponent=3) and 2 cross terms (two variables, |coeff|=3).
  const terms = flattenAddTerms(inner);
  if (terms.length !== 4) return undefined;

  const parsed = terms.map((term) => splitIntegerCoefficientAndPowers(term));
  if (parsed.some((term) => term === undefined)) return undefined;

  const pureCubes: Array<{ readonly coefficient: bigint; readonly base: IRNode }> = [];
  const crossTerms: Array<{ readonly coefficient: bigint; readonly powers: Map<string, FactorPower> }> = [];

  for (const parsedTerm of parsed) {
    if (parsedTerm === undefined) return undefined;
    const { coefficient, powers } = parsedTerm;
    if (powers.size === 1) {
      const [power] = [...powers.values()];
      if (power.exponent === 3 && (coefficient === 1n || coefficient === -1n)) {
        pureCubes.push({ coefficient, base: power.base });
        continue;
      }
    }
    if (powers.size === 2) {
      crossTerms.push({ coefficient, powers });
      continue;
    }
    return undefined; // unexpected shape (wrong exponent or variable count)
  }

  if (pureCubes.length !== 2 || crossTerms.length !== 2) return undefined;

  // Identify a and b; determine sum vs. difference from the pure-cube signs.
  let aNode: IRNode;
  let bNode: IRNode;
  let isSum: boolean;

  if (pureCubes[0].coefficient === 1n && pureCubes[1].coefficient === 1n) {
    // Sum: (a + b)^3 — both cubic terms positive.
    aNode = pureCubes[0].base;
    bNode = pureCubes[1].base;
    isSum = true;
  } else if (
    pureCubes.some((c) => c.coefficient === 1n)
    && pureCubes.some((c) => c.coefficient === -1n)
  ) {
    // Difference: (a − b)^3 — a^3 positive, b^3 negative.
    const pos = pureCubes.find((c) => c.coefficient === 1n);
    const neg = pureCubes.find((c) => c.coefficient === -1n);
    if (pos === undefined || neg === undefined) return undefined;
    aNode = pos.base;
    bNode = neg.base;
    isSum = false;
  } else {
    return undefined;
  }

  // Cross-term variable sets must equal exactly {aNode, bNode}.
  const aKey = nodeKey(aNode);
  const bKey = nodeKey(bNode);
  const variablePair = new Set([aKey, bKey]);
  for (const { powers } of crossTerms) {
    if (powers.size !== 2) return undefined;
    if (![...powers.keys()].every((k) => variablePair.has(k))) return undefined;
  }

  // Validate cross-term coefficients and exponent distributions.
  if (isSum) {
    // Expect +3·a^2·b and +3·a·b^2 in any order.
    let foundA2b = false;
    let foundAb2 = false;
    for (const { coefficient, powers } of crossTerms) {
      const expA = powers.get(aKey)?.exponent ?? 0;
      const expB = powers.get(bKey)?.exponent ?? 0;
      if (coefficient === 3n && expA === 2 && expB === 1) {
        foundA2b = true;
      } else if (coefficient === 3n && expA === 1 && expB === 2) {
        foundAb2 = true;
      } else {
        return undefined;
      }
    }
    if (!foundA2b || !foundAb2) return undefined;
    return app(POW, [app(ADD, [aNode, bNode]), int(3)]);
  }

  // Difference: expect −3·a^2·b and +3·a·b^2.
  // The a^2·b sign flips because (a−b)^3 expansion gives −3a^2b.
  let foundNegA2b = false;
  let foundPosAb2 = false;
  for (const { coefficient, powers } of crossTerms) {
    const expA = powers.get(aKey)?.exponent ?? 0;
    const expB = powers.get(bKey)?.exponent ?? 0;
    if (coefficient === -3n && expA === 2 && expB === 1) {
      foundNegA2b = true;
    } else if (coefficient === 3n && expA === 1 && expB === 2) {
      foundPosAb2 = true;
    } else {
      return undefined;
    }
  }
  if (!foundNegA2b || !foundPosAb2) return undefined;
  return app(POW, [app(SUB, [aNode, bNode]), int(3)]);
}

function commonSymbolicPowers(terms: readonly IRNode[]): Map<string, FactorPower> {
  if (terms.length === 0) return new Map();

  const common = new Map(splitCommonFactorTerm(terms[0]));
  for (const term of terms.slice(1)) {
    const powers = splitCommonFactorTerm(term);
    for (const [key, power] of [...common.entries()]) {
      const shared = Math.min(power.exponent, powers.get(key)?.exponent ?? 0);
      if (shared > 0) {
        common.set(key, { base: power.base, exponent: shared });
      } else {
        common.delete(key);
      }
    }
  }
  return common;
}

function sameTwoTerms(left: readonly IRNode[], right: readonly IRNode[]): boolean {
  if (left.length !== 2 || right.length !== 2) return false;
  const leftKeys = left.map(nodeKey);
  const rightKeys = right.map(nodeKey);
  return (
    (leftKeys[0] === rightKeys[0] && leftKeys[1] === rightKeys[1])
    || (leftKeys[0] === rightKeys[1] && leftKeys[1] === rightKeys[0])
  );
}

function extractMultivariateGrouping(inner: IRNode): IRNode | undefined {
  const terms = flattenAddTerms(inner);
  if (terms.length !== 4) return undefined;

  for (let firstIndex = 0; firstIndex < terms.length - 1; firstIndex += 1) {
    for (let secondIndex = firstIndex + 1; secondIndex < terms.length; secondIndex += 1) {
      const grouped = [terms[firstIndex], terms[secondIndex]];
      const firstCommon = commonSymbolicPowers(grouped);
      if (firstCommon.size === 0) continue;

      const rest = terms.filter((_term, index) => index !== firstIndex && index !== secondIndex);
      const firstResidual = grouped.map((term) => removeCommonFactor(term, firstCommon));
      const secondCommon = commonSymbolicPowers(rest);
      const secondResidual = rest.map((term) => removeCommonFactor(term, secondCommon));
      if (!sameTwoTerms(firstResidual, secondResidual)) continue;

      const firstFactor = multiplyNodes([...firstCommon.values()]
        .sort((a, b) => nodeKey(a.base).localeCompare(nodeKey(b.base)))
        .map((power) => powNode(power.base, power.exponent)));
      const secondFactor = secondCommon.size === 0
        ? int(1)
        : multiplyNodes([...secondCommon.values()]
          .sort((a, b) => nodeKey(a.base).localeCompare(nodeKey(b.base)))
          .map((power) => powNode(power.base, power.exponent)));

      return app(MUL, [
        app(ADD, [firstFactor, secondFactor]),
        addNodes(firstResidual),
      ]);
    }
  }

  return undefined;
}

function polyAdd(a: readonly bigint[], b: readonly bigint[]): bigint[] {
  const len = Math.max(a.length, b.length);
  const out = Array<bigint>(len).fill(0n);
  for (let i = 0; i < len; i += 1) out[i] = (a[i] ?? 0n) + (b[i] ?? 0n);
  return trimPoly(out);
}

function polySub(a: readonly bigint[], b: readonly bigint[]): bigint[] {
  const len = Math.max(a.length, b.length);
  const out = Array<bigint>(len).fill(0n);
  for (let i = 0; i < len; i += 1) out[i] = (a[i] ?? 0n) - (b[i] ?? 0n);
  return trimPoly(out);
}

function polyMul(a: readonly bigint[], b: readonly bigint[]): bigint[] {
  const out = Array<bigint>(a.length + b.length - 1).fill(0n);
  for (let i = 0; i < a.length; i += 1) {
    for (let j = 0; j < b.length; j += 1) out[i + j] += a[i] * b[j];
  }
  return trimPoly(out);
}

function polyPow(base: readonly bigint[], exp: bigint): bigint[] | undefined {
  if (exp > 32n) return undefined;
  let out = [1n];
  for (let i = 0n; i < exp; i += 1n) out = polyMul(out, base);
  return out;
}

function trimPoly(poly: bigint[]): bigint[] {
  while (poly.length > 1 && poly[poly.length - 1] === 0n) poly.pop();
  return poly;
}

function polyEquals(a: readonly bigint[], b: readonly bigint[]): boolean {
  const ta = trimPoly([...a]);
  const tb = trimPoly([...b]);
  return ta.length === tb.length && ta.every((value, index) => value === tb[index]);
}

function nodeKey(node: IRNode): string {
  switch (node.kind) {
    case "integer":
      return `i:${node.value}`;
    case "rational":
      return `q:${node.numer}/${node.denom}`;
    case "float":
      return `f:${node.value}`;
    case "string":
      return `s:${node.value}`;
    case "symbol":
      return `y:${node.name}`;
    case "apply":
      return `a:${nodeKey(node.head)}(${node.args.map((arg) => nodeKey(arg)).join(",")})`;
  }
}

// ---------------------------------------------------------------------------
// Apart (Track B1) — partial-fraction decomposition over Q(x).
//
// Supports simple rational roots, repeated rational roots, and proper
// irreducible denominators that are already apart, and mixed rational-root
// plus irreducible residual factors.
//
// Polynomials here use a coefficient-tuple representation indexed by power
// (lowest degree first) — same convention as ``polynomial-bridge.py``:
//
//     [c0, c1, c2]  ↔  c0 + c1·x + c2·x²
//
// Coefficients are ``RatQ`` values (bigint numerator + bigint denominator,
// always lowest terms with denom > 0).  We deliberately roll our own thin
// fraction type rather than reusing ``Numeric`` because Numeric also admits
// floats — Apart must stay exact.
// ---------------------------------------------------------------------------

/** Exact rational coefficient: numerator + positive denominator in lowest terms. */
type RatQ = readonly [bigint, bigint];

const RQ_ZERO: RatQ = [0n, 1n];
const RQ_ONE: RatQ = [1n, 1n];

function rqAbs(n: bigint): bigint {
  return n < 0n ? -n : n;
}

/** Greatest common divisor of two non-negative bigints. */
function rqGcd(a: bigint, b: bigint): bigint {
  let x = a < 0n ? -a : a;
  let y = b < 0n ? -b : b;
  while (y !== 0n) {
    [x, y] = [y, x % y];
  }
  return x;
}

/** Build a RatQ from raw numer/denom, reducing to lowest terms with denom > 0. */
function rqMake(n: bigint, d: bigint): RatQ {
  if (d === 0n) throw new RangeError("zero denominator in RatQ");
  if (d < 0n) {
    n = -n;
    d = -d;
  }
  if (n === 0n) return RQ_ZERO;
  const g = rqGcd(rqAbs(n), d);
  return [n / g, d / g];
}

function rqIsZero(r: RatQ): boolean {
  return r[0] === 0n;
}

function rqEquals(a: RatQ, b: RatQ): boolean {
  return a[0] === b[0] && a[1] === b[1];
}

function rqAdd(a: RatQ, b: RatQ): RatQ {
  return rqMake(a[0] * b[1] + b[0] * a[1], a[1] * b[1]);
}

function rqSub(a: RatQ, b: RatQ): RatQ {
  return rqMake(a[0] * b[1] - b[0] * a[1], a[1] * b[1]);
}

function rqMul(a: RatQ, b: RatQ): RatQ {
  return rqMake(a[0] * b[0], a[1] * b[1]);
}

function rqDiv(a: RatQ, b: RatQ): RatQ {
  if (b[0] === 0n) throw new RangeError("division by zero in RatQ");
  return rqMake(a[0] * b[1], a[1] * b[0]);
}

function rqNeg(r: RatQ): RatQ {
  return [-r[0], r[1]];
}

/** Convert a RatQ to IR (Integer when denom == 1, otherwise Rational). */
function rqToIr(r: RatQ): IRNode {
  if (r[0] === 0n) return int(0);
  if (r[1] === 1n) return int(r[0]);
  return rational(r[0], r[1]);
}

/** Coefficient tuple (lowest-degree first); empty array == zero polynomial. */
type PolyQ = readonly RatQ[];

/** Strip trailing zeros — analog of ``polynomial.normalize``. */
function polyQNormalize(p: PolyQ): RatQ[] {
  const result = [...p];
  while (result.length > 0 && rqIsZero(result[result.length - 1])) result.pop();
  return result;
}

/** Degree (-1 for the zero polynomial). */
function polyQDegree(p: PolyQ): number {
  const n = polyQNormalize(p);
  return n.length - 1;
}

/** Horner evaluation at a rational point. */
function polyQEvaluate(p: PolyQ, x: RatQ): RatQ {
  const n = polyQNormalize(p);
  if (n.length === 0) return RQ_ZERO;
  let acc: RatQ = RQ_ZERO;
  for (let i = n.length - 1; i >= 0; i -= 1) {
    acc = rqAdd(rqMul(acc, x), n[i]);
  }
  return acc;
}

/** Formal derivative; constant / zero polynomial → []. */
function polyQDeriv(p: PolyQ): RatQ[] {
  const n = polyQNormalize(p);
  if (n.length <= 1) return [];
  const result: RatQ[] = [];
  for (let i = 1; i < n.length; i += 1) {
    result.push(rqMul(n[i], [BigInt(i), 1n]));
  }
  return polyQNormalize(result);
}

/** Polynomial long division: returns { q, r } such that ``a = b·q + r``. */
function polyQDivmod(a: PolyQ, b: PolyQ): { q: RatQ[]; r: RatQ[] } {
  const nb = polyQNormalize(b);
  if (nb.length === 0) throw new RangeError("polynomial division by zero");
  const na = polyQNormalize(a);
  const degA = na.length - 1;
  const degB = nb.length - 1;
  if (degA < degB) return { q: [], r: na };
  const rem: RatQ[] = [...na];
  const quot: RatQ[] = Array.from({ length: degA - degB + 1 }, () => RQ_ZERO);
  const leadB = nb[degB];
  let degRem = degA;
  while (degRem >= degB) {
    const coeff = rqDiv(rem[degRem], leadB);
    const power = degRem - degB;
    quot[power] = coeff;
    for (let j = 0; j < nb.length; j += 1) {
      rem[power + j] = rqSub(rem[power + j], rqMul(coeff, nb[j]));
    }
    degRem -= 1;
    while (degRem >= 0 && rqIsZero(rem[degRem])) degRem -= 1;
  }
  return { q: polyQNormalize(quot), r: polyQNormalize(rem) };
}

/** Distinct rational roots via the Rational-Roots Theorem.  Returns roots
 *  as ``RatQ`` values in arbitrary order. */
function polyQRationalRoots(p: PolyQ): RatQ[] {
  const n = polyQNormalize(p);
  if (n.length <= 1) return [];

  // Clear denominators so candidates come from integer divisors of p.
  let lcmDen = 1n;
  for (const [, d] of n) {
    lcmDen = (lcmDen * d) / rqGcd(lcmDen, d);
  }
  let intCoeffs = n.map(([num, den]) => (num * lcmDen) / den);
  if (intCoeffs[intCoeffs.length - 1] < 0n) {
    intCoeffs = intCoeffs.map((c) => -c);
  }

  const a0 = intCoeffs[0];
  const an = intCoeffs[intCoeffs.length - 1];

  if (a0 === 0n) {
    // x = 0 is a root; strip it and recurse on the tail.
    const tailInt = intCoeffs.slice(1);
    const tailPoly: RatQ[] = tailInt.map((c) => rqMake(c, 1n));
    const tailRoots = polyQRationalRoots(tailPoly);
    const found = new Map<string, RatQ>();
    found.set("0/1", RQ_ZERO);
    for (const r of tailRoots) found.set(`${r[0]}/${r[1]}`, r);
    // Sort ascending — matches Python's ``sorted(roots)`` (Phase 48 needs
    // a stable ascending order across multi-root denominators that include
    // ``x = 0``; B1's simple-root tests never exercised this branch).
    const all = [...found.values()];
    all.sort((a, b) => {
      const lhs = a[0] * b[1];
      const rhs = b[0] * a[1];
      if (lhs < rhs) return -1;
      if (lhs > rhs) return 1;
      return 0;
    });
    return all;
  }

  const divisors = (m: bigint): bigint[] => {
    const abs = m < 0n ? -m : m;
    const out: bigint[] = [];
    for (let d = 1n; d <= abs; d += 1n) {
      if (abs % d === 0n) out.push(d);
    }
    return out;
  };

  const pDivs = divisors(a0);
  const qDivs = divisors(an);

  const candidates = new Map<string, RatQ>();
  for (const u of pDivs) {
    for (const v of qDivs) {
      const pos = rqMake(u, v);
      const neg = rqMake(-u, v);
      candidates.set(`${pos[0]}/${pos[1]}`, pos);
      candidates.set(`${neg[0]}/${neg[1]}`, neg);
    }
  }

  const intPoly: RatQ[] = intCoeffs.map((c) => rqMake(c, 1n));
  const roots: RatQ[] = [];
  for (const cand of candidates.values()) {
    if (rqIsZero(polyQEvaluate(intPoly, cand))) roots.push(cand);
  }
  // Sort ascending by rational value to match Python's ``sorted(roots)``
  // — keeps the IR output shape stable across regression tests.
  roots.sort((a, b) => {
    const lhs = a[0] * b[1];
    const rhs = b[0] * a[1];
    if (lhs < rhs) return -1;
    if (lhs > rhs) return 1;
    return 0;
  });
  return roots;
}

/** For each rational root r, count how many times (x − r) divides ``den``.
 *  Also returns the remaining factor, which is constant iff ``den`` fully
 *  splits over Q. */
function polyQRootMultiplicitiesAndResidual(
  den: PolyQ,
  roots: readonly RatQ[],
): { mults: Map<string, { root: RatQ; mult: number }>; residual: RatQ[] } | undefined {
  const out = new Map<string, { root: RatQ; mult: number }>();
  let remaining: RatQ[] = polyQNormalize(den);
  for (const r of roots) {
    let m = 0;
    const linear: RatQ[] = [rqNeg(r), RQ_ONE]; // (x − r)
    // Repeatedly divide while exact.
    /* eslint-disable no-constant-condition */
    while (true) {
      const { q, r: rem } = polyQDivmod(remaining, linear);
      if (polyQNormalize(rem).length === 0) {
        remaining = q;
        m += 1;
      } else {
        break;
      }
    }
    /* eslint-enable no-constant-condition */
    if (m === 0) return undefined;
    out.set(`${r[0]}/${r[1]}`, { root: r, mult: m });
  }
  return { mults: out, residual: polyQNormalize(remaining) };
}

// --- IR ↔ polynomial bridge -------------------------------------------------

type RationalForm = { num: RatQ[]; den: RatQ[] };

/** Mirror of Python ``polynomial_bridge.to_rational``: walks an IR tree and
 *  returns ``{ num, den }`` if it lives in Q(x); otherwise ``undefined``. */
function toRational(node: IRNode, x: IRSymbol): RationalForm | undefined {
  return toRationalWalk(node, x);
}

function toRationalWalk(node: IRNode, x: IRSymbol): RationalForm | undefined {
  if (node.kind === "integer") {
    return { num: [rqMake(node.value, 1n)], den: [RQ_ONE] };
  }
  if (node.kind === "rational") {
    return { num: [rqMake(node.numer, node.denom)], den: [RQ_ONE] };
  }
  if (node.kind === "float") return undefined; // exact only
  if (node.kind === "symbol") {
    if (node.name === x.name) {
      return { num: [RQ_ZERO, RQ_ONE], den: [RQ_ONE] };
    }
    return undefined;
  }
  if (node.kind !== "apply") return undefined;
  const head = node.head;

  if (equals(head, ADD)) {
    return reduceRational(node.args, x, addRational);
  }
  if (equals(head, SUB)) {
    if (node.args.length !== 2) return undefined;
    const a = toRationalWalk(node.args[0], x);
    const b = toRationalWalk(node.args[1], x);
    if (!a || !b) return undefined;
    return subRational(a, b);
  }
  if (equals(head, NEG)) {
    if (node.args.length !== 1) return undefined;
    const a = toRationalWalk(node.args[0], x);
    if (!a) return undefined;
    return { num: a.num.map(rqNeg), den: a.den };
  }
  if (equals(head, MUL)) {
    return reduceRational(node.args, x, mulRational);
  }
  if (equals(head, DIV)) {
    if (node.args.length !== 2) return undefined;
    const a = toRationalWalk(node.args[0], x);
    const b = toRationalWalk(node.args[1], x);
    if (!a || !b) return undefined;
    return divRational(a, b);
  }
  if (equals(head, POW)) {
    if (node.args.length !== 2) return undefined;
    return powRational(node.args[0], node.args[1], x);
  }
  return undefined;
}

function polyQAdd(a: PolyQ, b: PolyQ): RatQ[] {
  const n = Math.max(a.length, b.length);
  const out: RatQ[] = [];
  for (let i = 0; i < n; i += 1) {
    const av = i < a.length ? a[i] : RQ_ZERO;
    const bv = i < b.length ? b[i] : RQ_ZERO;
    out.push(rqAdd(av, bv));
  }
  return polyQNormalize(out);
}

function polyQSub(a: PolyQ, b: PolyQ): RatQ[] {
  const n = Math.max(a.length, b.length);
  const out: RatQ[] = [];
  for (let i = 0; i < n; i += 1) {
    const av = i < a.length ? a[i] : RQ_ZERO;
    const bv = i < b.length ? b[i] : RQ_ZERO;
    out.push(rqSub(av, bv));
  }
  return polyQNormalize(out);
}

function polyQMul(a: PolyQ, b: PolyQ): RatQ[] {
  if (a.length === 0 || b.length === 0) return [];
  const out: RatQ[] = Array.from({ length: a.length + b.length - 1 }, () => RQ_ZERO);
  for (let i = 0; i < a.length; i += 1) {
    if (rqIsZero(a[i])) continue;
    for (let j = 0; j < b.length; j += 1) {
      if (rqIsZero(b[j])) continue;
      out[i + j] = rqAdd(out[i + j], rqMul(a[i], b[j]));
    }
  }
  return polyQNormalize(out);
}

function polyQPow(p: PolyQ, n: number): RatQ[] {
  let result: RatQ[] = [RQ_ONE];
  for (let i = 0; i < n; i += 1) {
    result = polyQMul(result, p);
  }
  return result;
}

function addRational(a: RationalForm, b: RationalForm): RationalForm {
  return {
    num: polyQAdd(polyQMul(a.num, b.den), polyQMul(b.num, a.den)),
    den: polyQMul(a.den, b.den),
  };
}

function subRational(a: RationalForm, b: RationalForm): RationalForm {
  return {
    num: polyQSub(polyQMul(a.num, b.den), polyQMul(b.num, a.den)),
    den: polyQMul(a.den, b.den),
  };
}

function mulRational(a: RationalForm, b: RationalForm): RationalForm {
  return { num: polyQMul(a.num, b.num), den: polyQMul(a.den, b.den) };
}

function divRational(a: RationalForm, b: RationalForm): RationalForm | undefined {
  const newDen = polyQMul(a.den, b.num);
  if (polyQNormalize(newDen).length === 0) return undefined;
  return { num: polyQMul(a.num, b.den), den: newDen };
}

function powRational(
  baseNode: IRNode,
  expNode: IRNode,
  x: IRSymbol,
): RationalForm | undefined {
  if (expNode.kind !== "integer") return undefined;
  const baseR = toRationalWalk(baseNode, x);
  if (!baseR) return undefined;
  const n = Number(expNode.value);
  if (!Number.isFinite(n)) return undefined;
  if (n === 0) return { num: [RQ_ONE], den: [RQ_ONE] };
  if (n < 0) {
    if (polyQNormalize(baseR.num).length === 0) return undefined;
    return { num: polyQPow(baseR.den, -n), den: polyQPow(baseR.num, -n) };
  }
  return { num: polyQPow(baseR.num, n), den: polyQPow(baseR.den, n) };
}

function reduceRational(
  args: readonly IRNode[],
  x: IRSymbol,
  op: (a: RationalForm, b: RationalForm) => RationalForm | undefined,
): RationalForm | undefined {
  if (args.length === 0) return undefined;
  let acc = toRationalWalk(args[0], x);
  if (!acc) return undefined;
  for (let i = 1; i < args.length; i += 1) {
    const other = toRationalWalk(args[i], x);
    if (!other) return undefined;
    const next = op(acc, other);
    if (!next) return undefined;
    acc = next;
  }
  return acc;
}

/** Build the canonical IR tree for a polynomial.  Mirrors
 *  ``polynomial_bridge.from_polynomial``: emits ``Add(term_0, term_1, …)``
 *  left-associated, drops zero terms, special-cases ±1 coefficients. */
function fromPolynomial(p: PolyQ, x: IRSymbol): IRNode {
  const n = polyQNormalize(p);
  if (n.length === 0) return int(0);
  if (n.length === 1) return rqToIr(n[0]);
  const terms: IRNode[] = [];
  for (let i = 0; i < n.length; i += 1) {
    const c = n[i];
    if (rqIsZero(c)) continue;
    terms.push(polyTerm(c, i, x));
  }
  if (terms.length === 0) return int(0);
  if (terms.length === 1) return terms[0];
  let acc: IRNode = terms[0];
  for (let i = 1; i < terms.length; i += 1) {
    acc = app(ADD, [acc, terms[i]]);
  }
  return acc;
}

function polyTerm(c: RatQ, i: number, x: IRSymbol): IRNode {
  if (i === 0) return rqToIr(c);
  const power: IRNode = i === 1 ? x : app(POW, [x, int(i)]);
  if (rqEquals(c, RQ_ONE)) return power;
  if (rqEquals(c, [-1n, 1n])) return app(NEG, [power]);
  return app(MUL, [rqToIr(c), power]);
}

// --- Apart simple-roots (Phase 1) + repeated linear factors (Phase 48) -----

/** Phase 1 simple-root path; mirrors ``_apart_simple_roots`` in Python.
 *  Returns ``undefined`` when any residue blows up (repeated root that
 *  slipped through — defensive, the caller already gates on this). */
function apartSimpleRoots(
  num: PolyQ,
  den: PolyQ,
  roots: readonly RatQ[],
  x: IRSymbol,
): IRNode | undefined {
  const denDeriv = polyQDeriv(den);
  const terms: IRNode[] = [];
  for (const r of roots) {
    const numVal = polyQEvaluate(num, r);
    const denDVal = polyQEvaluate(denDeriv, r);
    if (rqIsZero(denDVal)) return undefined;
    const A = rqDiv(numVal, denDVal);
    const negR = rqNeg(r);
    const factorIr = fromPolynomial([negR, RQ_ONE], x);
    if (rqEquals(A, RQ_ONE)) {
      terms.push(app(DIV, [int(1), factorIr]));
    } else if (rqEquals(A, [-1n, 1n])) {
      terms.push(app(NEG, [app(DIV, [int(1), factorIr])]));
    } else {
      terms.push(app(DIV, [rqToIr(A), factorIr]));
    }
  }
  if (terms.length === 0) return int(0);
  if (terms.length === 1) return terms[0];
  let acc: IRNode = terms[0];
  for (let i = 1; i < terms.length; i += 1) {
    acc = app(ADD, [acc, terms[i]]);
  }
  return acc;
}

/** Binomial coefficient ``C(n, k)``.  Returns ``0n`` when k is out of
 *  range so callers can sum unconditionally.  Mirrors Python ``_binomial``. */
function binomialBig(n: number, k: number): bigint {
  if (k < 0 || k > n) return 0n;
  if (k === 0 || k === n) return 1n;
  const kk = Math.min(k, n - k);
  let result = 1n;
  for (let i = 0; i < kk; i += 1) {
    result = (result * BigInt(n - i)) / BigInt(i + 1);
  }
  return result;
}

/** Return the first ``length`` Taylor coefficients of ``poly(r + t)`` as
 *  a polynomial in ``t``.  Mirrors Python ``_taylor_expand_around_r``:
 *  for ``poly(x) = ∑ c_i x^i``,
 *      poly(r + t) = ∑_j t^j · [∑_{i ≥ j} c_i · C(i, j) · r^(i − j)].
 *  When ``length`` exceeds ``deg poly`` trailing entries are filled with 0. */
function polyTaylorExpandAroundR(poly: PolyQ, r: RatQ, length: number): RatQ[] {
  const deg = poly.length - 1;
  const result: RatQ[] = [];
  for (let j = 0; j < length; j += 1) {
    let cj: RatQ = RQ_ZERO;
    let rPow: RatQ = RQ_ONE; // r^(i - j) starts at r^0 when i == j
    for (let i = j; i <= deg; i += 1) {
      const coef = poly[i];
      const binom = binomialBig(i, j);
      if (binom !== 0n) {
        const term = rqMul(rqMul(coef, [binom, 1n]), rPow);
        cj = rqAdd(cj, term);
      }
      rPow = rqMul(rPow, r);
    }
    result.push(cj);
  }
  return result;
}

/** Formal power-series division ``N(t) / D(t)`` up to ``t^(length − 1)``.
 *  Requires ``D(0) ≠ 0`` — returns ``undefined`` otherwise (signal of a
 *  repeated-root miscount upstream).  Mirrors Python ``_series_div``. */
function polySeriesDiv(
  nCoeffs: readonly RatQ[],
  dCoeffs: readonly RatQ[],
  length: number,
): RatQ[] | undefined {
  if (dCoeffs.length === 0 || rqIsZero(dCoeffs[0])) return undefined;
  const d0 = dCoeffs[0];
  const q: RatQ[] = [];
  for (let j = 0; j < length; j += 1) {
    const nj = j < nCoeffs.length ? nCoeffs[j] : RQ_ZERO;
    let s: RatQ = RQ_ZERO;
    for (let k = 1; k <= j; k += 1) {
      const dk = k < dCoeffs.length ? dCoeffs[k] : RQ_ZERO;
      s = rqAdd(s, rqMul(dk, q[j - k]));
    }
    q.push(rqDiv(rqSub(nj, s), d0));
  }
  return q;
}

/** Build the IR for ``A / (x − r)^power``.  Drops ±1 numerator coefficients
 *  to match the formatting convention in ``apartSimpleRoots``.  Mirrors
 *  Python ``_build_apart_term``. */
function buildApartTerm(A: RatQ, r: RatQ, power: number, x: IRSymbol): IRNode {
  const negR = rqNeg(r);
  const factorIr = fromPolynomial([negR, RQ_ONE], x);
  const denomIr: IRNode = power === 1 ? factorIr : app(POW, [factorIr, int(power)]);
  if (rqEquals(A, RQ_ONE)) {
    return app(DIV, [int(1), denomIr]);
  }
  if (rqEquals(A, [-1n, 1n])) {
    return app(NEG, [app(DIV, [int(1), denomIr])]);
  }
  return app(DIV, [rqToIr(A), denomIr]);
}

/** Decompose a *proper* rational function (deg num < deg den).
 *
 *  Phase 1 (simple roots) — uses the residue formula ``A_i = P(r_i)/Q'(r_i)``
 *  for each distinct rational root ``r_i``.
 *
 *  Phase 48 (repeated linear factors) — for each root ``r`` of multiplicity
 *  ``m`` compute ``Q(x) = den(x)/(x − r)^m`` and expand
 *  ``φ(t) = P(r + t)/Q(r + t)`` as a Taylor series in ``t`` up to ``t^(m−1)``.
 *  Then ``A_{r, m − j} = φ_j``.  Emits terms ``A / (x − r)^power`` for
 *  ``power = 1..m``.
 *
 *  Mixed rational-root plus irreducible residual factors are decomposed into
 *  rational-pole terms plus a proper residual rational term. */
function apartProper(num: PolyQ, den: PolyQ, x: IRSymbol): IRNode | undefined {
  const roots = polyQRationalRoots(den);
  if (roots.length === 0) return properRationalToIr(num, den, x);
  const split = polyQRootMultiplicitiesAndResidual(den, roots);
  if (split === undefined) return undefined;
  const { mults, residual: residualDen } = split;
  const hasResidual = polyQDegree(residualDen) >= 1;

  // Phase 1 fast path — preserves the existing output shape for the
  // regression tests written against B1.
  let allSimple = true;
  for (const { mult } of mults.values()) {
    if (mult > 1) {
      allSimple = false;
      break;
    }
  }
  if (!hasResidual && allSimple) return apartSimpleRoots(num, den, roots, x);

  // Phase 48 generic path: Taylor + series-division per root.
  const terms: IRNode[] = [];
  let linearPart: RatQ[] = [RQ_ONE];
  let residualNum: RatQ[] = polyQNormalize(num);
  for (const r of roots) {
    const key = `${r[0]}/${r[1]}`;
    const entry = mults.get(key);
    if (entry === undefined) return undefined; // defensive — shouldn't happen
    const m = entry.mult;
    // Q(x) = den(x) / (x − r)^m.  Successive divisions are exact because
    // we just verified the multiplicity above.
    let qPoly: RatQ[] = polyQNormalize(den);
    const linear: RatQ[] = [rqNeg(r), RQ_ONE];
    for (let i = 0; i < m; i += 1) {
      linearPart = polyQMul(linearPart, linear);
    }
    for (let i = 0; i < m; i += 1) {
      const { q } = polyQDivmod(qPoly, linear);
      qPoly = q;
    }
    // Taylor-expand both P(r + t) and Q(r + t) up to t^(m − 1).
    const nTaylor = polyTaylorExpandAroundR(num, r, m);
    const dTaylor = polyTaylorExpandAroundR(qPoly, r, m);
    const phi = polySeriesDiv(nTaylor, dTaylor, m);
    if (phi === undefined) return undefined;
    // A_{r, m − j} = phi[j].  Emit ascending power order:
    // 1/(x − r), 1/(x − r)^2, …, 1/(x − r)^m.
    for (let power = 1; power <= m; power += 1) {
      const j = m - power;
      const A = phi[j];
      if (rqIsZero(A)) continue;
      terms.push(buildApartTerm(A, r, power, x));
      let poleDenom: RatQ[] = [RQ_ONE];
      for (let i = 0; i < power; i += 1) {
        poleDenom = polyQMul(poleDenom, linear);
      }
      const { q, r: rem } = polyQDivmod(den, poleDenom);
      if (polyQNormalize(rem).length !== 0) return undefined;
      residualNum = polyQSub(
        residualNum,
        q.map((c) => rqMul(c, A)),
      );
    }
  }
  if (hasResidual) {
    const { q: residualQuotient, r: residualRem } = polyQDivmod(residualNum, linearPart);
    if (polyQNormalize(residualRem).length !== 0) return undefined;
    const residualIr = properRationalToIr(residualQuotient, residualDen, x);
    if (!(residualIr.kind === "integer" && residualIr.value === 0n)) {
      terms.push(residualIr);
    }
  }
  if (terms.length === 0) return int(0);
  if (terms.length === 1) return terms[0];
  let acc: IRNode = terms[0];
  for (let i = 1; i < terms.length; i += 1) {
    acc = app(ADD, [acc, terms[i]]);
  }
  return acc;
}

function properRationalToIr(num: PolyQ, den: PolyQ, x: IRSymbol): IRNode {
  if (polyQNormalize(num).length === 0) return int(0);
  return app(DIV, [fromPolynomial(num, x), fromPolynomial(den, x)]);
}

function apartHandler(_vm: VM, expr: IRApply): IRNode {
  if (expr.args.length !== 2) return expr;
  const inner = expr.args[0];
  const varNode = expr.args[1];
  if (varNode.kind !== "symbol") return expr;
  const rational = toRational(inner, varNode);
  if (!rational) return expr;
  const num = polyQNormalize(rational.num);
  const den = polyQNormalize(rational.den);
  if (den.length === 1 && rqEquals(den[0], RQ_ONE)) {
    // Already a polynomial.
    return fromPolynomial(num, varNode);
  }
  const numDeg = polyQDegree(num);
  const denDeg = polyQDegree(den);
  if (numDeg >= denDeg) {
    // Improper fraction — polynomial division first, then Apart on the
    // proper remainder.
    const { q, r } = polyQDivmod(num, den);
    if (polyQNormalize(r).length === 0) {
      return fromPolynomial(q, varNode);
    }
    const properResult = apartProper(r, den, varNode);
    if (properResult === undefined) return expr;
    const polyPart = fromPolynomial(q, varNode);
    return app(ADD, [polyPart, properResult]);
  }
  const result = apartProper(num, den, varNode);
  if (result === undefined) return expr;
  return result;
}

// ---------------------------------------------------------------------------
// Track G2 — current-VM assumption store, mirror of the Python
// ``_CURRENT_VM`` ContextVar.
//
// Most integrator helpers operate on pure IR and don't need the VM.
// The symbolic-coefficient Weierstrass helper (added in Track G2)
// needs to query ``vm.assumptions`` for the discriminant sign.
// Threading ``vm`` through every helper signature would touch ~30
// call sites; we instead publish the live store via a module-level
// reference that the top-level ``Integrate`` handler sets for the
// duration of one evaluation.  JavaScript is single-threaded so a
// plain ``let`` is the natural mirror of Python's ``ContextVar``.
// ---------------------------------------------------------------------------
let CURRENT_ASSUMPTIONS: AssumptionContext | undefined;

function currentAssumptions(): AssumptionContext | undefined {
  return CURRENT_ASSUMPTIONS;
}

function integrate(): Handler {
  return (vm, expr) => {
    if (expr.args.length !== 2 && expr.args.length !== 4) {
      throw new ArityError(`Integrate expects 2 or 4 arguments, got ${expr.args.length}`);
    }
    const [f, x] = expr.args;
    if (x.kind !== "symbol") {
      return expr;
    }
    // Track G2: publish the live assumption store for helpers that
    // consult it (currently: symbolic-coefficient Weierstrass).  The
    // previous value is restored in `finally` so nested calls and
    // exceptions can't strand the module-level mirror in an
    // inconsistent state.
    const previousAssumptions = CURRENT_ASSUMPTIONS;
    CURRENT_ASSUMPTIONS = vm.assumptions;
    try {
      if (expr.args.length === 4) {
        const resultK = completeEllipticFirstKind(f, x, expr.args[2], expr.args[3]);
        if (resultK !== undefined) return vm.eval(resultK);
        const resultE = completeEllipticSecondKind(f, x, expr.args[2], expr.args[3]);
        if (resultE !== undefined) return vm.eval(resultE);
        const resultPi = completeEllipticThirdKind(f, x, expr.args[2], expr.args[3]);
        if (resultPi !== undefined) return vm.eval(resultPi);
        return expr;
      }
      const result = integrateIndefinite(f, x);
      if (result === undefined) {
        // Track E2: generic tabular IBP fallback.  Fires after every
        // shape-specific handler in integrateIndefinite returned undefined.
        // Mirrors the Python ``try_ibp_tabular`` hook in ``integrate.py``.
        const ibpResult = tryIbpTabular(
          f,
          x,
          (g) => integrateIndefinite(g, x),
          (g) => diff(g, x),
          (n) => vm.eval(n),
        );
        if (ibpResult !== undefined) {
          return vm.eval(ibpResult);
        }
        return expr;
      }
      return isDeferredIntegral(result, f, x) ? result : vm.eval(result);
    } finally {
      CURRENT_ASSUMPTIONS = previousAssumptions;
    }
  };
}

// ---------------------------------------------------------------------------
// Phase 34 — Weierstrass substitution for ∫ c/(a + b·sin(x)) dx and
// ∫ c/(a + b·cos(x)) dx with numeric a, b satisfying a² > b².
// Mirrors the Python port at symbolic-vm 0.59.0.
//
// Closed forms:
//   ∫ 1/(a + b·sin x) dx = (2/√(a²−b²)) · arctan((a·tan(x/2) + b)/√(a²−b²))
//   ∫ 1/(a + b·cos x) dx = (2/√(a²−b²)) · arctan(√((a−b)/(a+b)) · tan(x/2))
//
// Numerator constants c scale the result.  Discriminant cases a² ≤ b² and
// symbolic-coefficient discriminant cases are deliberately deferred — they
// need sign analysis that the assumption-free TS port cannot perform.
// ---------------------------------------------------------------------------

/**
 * Rational `p/q` with `q > 0` and `gcd(|p|, q) = 1`, stored as Numeric so we
 * can reuse the existing arithmetic helpers (addNumeric / mulNumeric / ...).
 */

function weierstrassSqrtFractionIR(f: Numeric): IRNode {
  // Defensive: only called with f > 0.  We fold p/q when both p and q are
  // perfect integer squares; otherwise emit Sqrt(rational).
  const rat = asRat(f);
  const p = rat.numer;
  const q = rat.denom;
  if (p <= 0n || q <= 0n) {
    return app(SQRT, [fromNumeric(f)]);
  }
  const pRoot = bigIntIsqrt(p);
  const qRoot = bigIntIsqrt(q);
  if (pRoot !== undefined && qRoot !== undefined) {
    return fromNumeric(makeRat(pRoot, qRoot));
  }
  return app(SQRT, [fromNumeric(f)]);
}

/**
 * Phase 38: Parse a linear-in-``x`` rational expression ``α·x + β`` and
 * return ``{ alpha, beta }`` with ``α, β`` exact Numeric (Int/Rat) and
 * ``α ≠ 0``.  Recognised shapes:
 *
 *   x              → (1, 0)
 *   α·x            → (α, 0)
 *   α·x + β        → (α, β)
 *   β + α·x        → (α, β)
 *   α·x − β        → (α, −β)
 *   β − α·x        → (−α, β)
 *   −(α·x + β)     → (−α, −β)
 *
 * Returns ``undefined`` when the expression is not linear in ``x`` (e.g.
 * ``x²``, ``sin(x)``, pure constants free of x, or a nested nonlinear
 * form).  ``α = 0`` is filtered out so callers may rely on ``α ≠ 0``.
 */
function weierstrassParseLinearInX(
  node: IRNode,
  x: IRNode
): { readonly alpha: Numeric; readonly beta: Numeric } | undefined {
  if (equals(node, x)) {
    return { alpha: { kind: "int", value: 1n }, beta: { kind: "int", value: 0n } };
  }
  if (!dependsOn(node, x)) {
    return undefined; // pure constant — no x term
  }
  if (node.kind === "apply" && equals(node.head, MUL) && node.args.length === 2) {
    const [left, right] = node.args;
    const cLeft = toNumeric(left);
    if (cLeft !== undefined && cLeft.kind !== "float" && equals(right, x) && !isZeroNumeric(cLeft)) {
      return { alpha: cLeft, beta: { kind: "int", value: 0n } };
    }
    const cRight = toNumeric(right);
    if (cRight !== undefined && cRight.kind !== "float" && equals(left, x) && !isZeroNumeric(cRight)) {
      return { alpha: cRight, beta: { kind: "int", value: 0n } };
    }
    return undefined;
  }
  if (node.kind === "apply" && equals(node.head, NEG) && node.args.length === 1) {
    const inner = weierstrassParseLinearInX(node.args[0], x);
    if (inner === undefined) return undefined;
    return { alpha: negNumeric(inner.alpha), beta: negNumeric(inner.beta) };
  }
  if (node.kind === "apply" && equals(node.head, ADD) && node.args.length === 2) {
    const [left, right] = node.args;
    for (const [constSide, linSide] of [
      [left, right],
      [right, left],
    ] as const) {
      const c = toNumeric(constSide);
      if (c === undefined || c.kind === "float") continue;
      const lin = weierstrassParseLinearInX(linSide, x);
      if (lin === undefined) continue;
      return { alpha: lin.alpha, beta: addNumeric(lin.beta, c) };
    }
    return undefined;
  }
  if (node.kind === "apply" && equals(node.head, SUB) && node.args.length === 2) {
    const [left, right] = node.args;
    // Case A: linear − constant → (α, β − c)
    const cRight = toNumeric(right);
    if (cRight !== undefined && cRight.kind !== "float") {
      const lin = weierstrassParseLinearInX(left, x);
      if (lin !== undefined) {
        return { alpha: lin.alpha, beta: subNumeric(lin.beta, cRight) };
      }
    }
    // Case B: constant − linear → (−α, c − β)
    const cLeft = toNumeric(left);
    if (cLeft !== undefined && cLeft.kind !== "float") {
      const lin = weierstrassParseLinearInX(right, x);
      if (lin !== undefined) {
        return {
          alpha: negNumeric(lin.alpha),
          beta: subNumeric(cLeft, lin.beta),
        };
      }
    }
    return undefined;
  }
  return undefined;
}

/**
 * Phase 38: Build the IR node for ``α·x + β`` collapsing trivial cases so the
 * downstream ``tan(arg/2)`` constructions emit the simplest equivalent form.
 *   α=1, β=0 → x          (the historical bare-x path; bit-for-bit identical)
 *   α=1, β≠0 → x + β
 *   β=0, α≠1 → α·x
 *   otherwise → α·x + β
 */
function weierstrassBuildLinearArgIR(
  alpha: Numeric,
  beta: Numeric,
  x: IRNode
): IRNode {
  const alphaIsOne =
    (alpha.kind === "int" && alpha.value === 1n) ||
    (alpha.kind === "rat" && alpha.numer === 1n && alpha.denom === 1n);
  const betaIsZero = isZeroNumeric(beta);
  if (alphaIsOne && betaIsZero) return x;
  if (betaIsZero) return app(MUL, [fromNumeric(alpha), x]);
  if (alphaIsOne) return app(ADD, [x, fromNumeric(beta)]);
  return app(ADD, [app(MUL, [fromNumeric(alpha), x]), fromNumeric(beta)]);
}

/** Phase 38: match ``c·sin(α·x+β)`` / ``c·cos(α·x+β)`` (and the c=1 / α=1 /
 *  β=0 degenerate variants) and return ``{ c, head, alpha, beta }``.
 *
 *  Accepts both argument orders within ``Mul`` and unwraps a leading
 *  ``Neg``.  The trig argument must be linear in ``x`` per
 *  :func:`weierstrassParseLinearInX`.  Supersedes the Phase 34 bare-``x``
 *  predecessor `weierstrassParseConstTimesTrigX`. */
function weierstrassParseConstTimesTrigLinear(
  node: IRNode,
  x: IRNode
):
  | { readonly c: Numeric; readonly head: IRNode; readonly alpha: Numeric; readonly beta: Numeric }
  | undefined {
  if (
    node.kind === "apply" &&
    (equals(node.head, SIN) || equals(node.head, COS)) &&
    node.args.length === 1
  ) {
    const lin = weierstrassParseLinearInX(node.args[0], x);
    if (lin !== undefined) {
      return {
        c: { kind: "int", value: 1n },
        head: node.head,
        alpha: lin.alpha,
        beta: lin.beta,
      };
    }
  }
  if (node.kind === "apply" && equals(node.head, MUL) && node.args.length === 2) {
    const [left, right] = node.args;
    for (const [constSide, trigSide] of [
      [left, right],
      [right, left],
    ] as const) {
      const c = toNumeric(constSide);
      if (c === undefined || c.kind === "float") continue;
      if (
        trigSide.kind === "apply" &&
        (equals(trigSide.head, SIN) || equals(trigSide.head, COS)) &&
        trigSide.args.length === 1
      ) {
        const lin = weierstrassParseLinearInX(trigSide.args[0], x);
        if (lin !== undefined) {
          return { c, head: trigSide.head, alpha: lin.alpha, beta: lin.beta };
        }
      }
    }
  }
  if (node.kind === "apply" && equals(node.head, NEG) && node.args.length === 1) {
    const inner = weierstrassParseConstTimesTrigLinear(node.args[0], x);
    if (inner !== undefined) {
      return {
        c: negNumeric(inner.c),
        head: inner.head,
        alpha: inner.alpha,
        beta: inner.beta,
      };
    }
  }
  return undefined;
}

/** Parse ``a + b·sin(α·x+β)`` or ``a + b·cos(α·x+β)`` (any operand ordering,
 *  plus the SUB head variant).  Returns ``{ a, b, trigHead, alpha, beta }``
 *  or undefined.  Phase 38 generalises the Phase 34 bare-``x`` predecessor. */
function weierstrassParseAPlusBSincos(
  node: IRNode,
  x: IRNode
):
  | {
      readonly a: Numeric;
      readonly b: Numeric;
      readonly trigHead: IRNode;
      readonly alpha: Numeric;
      readonly beta: Numeric;
    }
  | undefined {
  const bareTrig = weierstrassParseConstTimesTrigLinear(node, x);
  if (bareTrig !== undefined) {
    return {
      a: { kind: "int", value: 0n },
      b: bareTrig.c,
      trigHead: bareTrig.head,
      alpha: bareTrig.alpha,
      beta: bareTrig.beta,
    };
  }
  if (node.kind !== "apply" || node.args.length !== 2) return undefined;
  if (equals(node.head, ADD)) {
    const [left, right] = node.args;
    for (const [constSide, trigSide] of [
      [left, right],
      [right, left],
    ] as const) {
      const a = toNumeric(constSide);
      if (a === undefined || a.kind === "float") continue;
      const trigParse = weierstrassParseConstTimesTrigLinear(trigSide, x);
      if (trigParse === undefined) continue;
      return {
        a,
        b: trigParse.c,
        trigHead: trigParse.head,
        alpha: trigParse.alpha,
        beta: trigParse.beta,
      };
    }
    return undefined;
  }
  if (equals(node.head, SUB)) {
    // `a − b·trig(...)` = `a + (−b)·trig(...)` and the symmetric reversal.
    const [left, right] = node.args;
    const aLeft = toNumeric(left);
    if (aLeft !== undefined && aLeft.kind !== "float") {
      const trigParse = weierstrassParseConstTimesTrigLinear(right, x);
      if (trigParse !== undefined) {
        return {
          a: aLeft,
          b: negNumeric(trigParse.c),
          trigHead: trigParse.head,
          alpha: trigParse.alpha,
          beta: trigParse.beta,
        };
      }
    }
    const bTrigLeft = weierstrassParseConstTimesTrigLinear(left, x);
    const aRight = toNumeric(right);
    if (bTrigLeft !== undefined && aRight !== undefined && aRight.kind !== "float") {
      return {
        a: negNumeric(aRight),
        b: bTrigLeft.c,
        trigHead: bTrigLeft.head,
        alpha: bTrigLeft.alpha,
        beta: bTrigLeft.beta,
      };
    }
    return undefined;
  }
  return undefined;
}

/** Phase 34 + 38 entry point.  Returns the closed form for
 *  ``∫ c / (a + b·trig(α·x + β)) dx`` (with `a, b, α, β ∈ ℚ`, `α ≠ 0`), or
 *  undefined when the shape doesn't match or the discriminant fails the
 *  branch-specific guards.  When `α = 1, β = 0` this is bit-for-bit
 *  identical to the original Phase 34/35/36/37 behaviour. */
function tryWeierstrassOneOverLinearTrig(
  integrand: IRNode,
  x: IRNode
): IRNode | undefined {
  if (integrand.kind !== "apply" || !equals(integrand.head, DIV)) return undefined;
  if (integrand.args.length !== 2) return undefined;
  const [num, den] = integrand.args;
  if (dependsOn(num, x)) return undefined;
  const cIn = toNumeric(num);
  if (cIn === undefined || cIn.kind === "float") return undefined;
  const parsed = weierstrassParseAPlusBSincos(den, x);
  if (parsed === undefined) return undefined;
  const { a, b, trigHead, alpha, beta } = parsed;
  // Phase 38: fold the inner substitution u = α·x + β (du = α·dx) into
  // the numerator constant once at entry: c ← c/α.  Every branch below
  // can then use the original closed-form formulas with `tan(arg/2)`
  // in place of `tan(x/2)`.  α=0 is excluded by `parseLinearInX`.
  const c = divNumeric(cIn, alpha);
  const argNode = weierstrassBuildLinearArgIR(alpha, beta, x);
  // disc = a² − b² (exact Numeric arithmetic — both a, b are Int/Rat).
  const disc = subNumeric(mulNumeric(a, a), mulNumeric(b, b));
  // Three-way dispatch on the discriminant:
  //   disc > 0  → Phase 34 arctan form (below)
  //   disc == 0 → Phase 35 degenerate form (four sign combinations)
  //   disc < 0  → Phase 36/37 log form
  if (isZeroNumeric(disc)) {
    return tryWeierstrassDegenerate(c, a, b, trigHead, argNode);
  }
  if (!isPositiveNumeric(disc)) {
    return tryWeierstrassLogForm(c, a, b, trigHead, argNode);
  }
  const sqrtDiscIR = weierstrassSqrtFractionIR(disc);
  const tanHalf = app(TAN, [app(DIV, [argNode, int(2)])]);
  let coefSign: Numeric = { kind: "int", value: 1n };
  let atanArg: IRNode;
  if (equals(trigHead, SIN)) {
    // (a·tan(arg/2) + b) / √(a²−b²)
    const top = app(ADD, [app(MUL, [fromNumeric(a), tanHalf]), fromNumeric(b)]);
    atanArg = app(DIV, [top, sqrtDiscIR]);
  } else {
    // COS branch: a < 0 uses the same atan argument, but the denominator
    // quadratic has an overall negative factor.
    if (!isPositiveNumeric(a)) coefSign = { kind: "int", value: -1n };
    // ratio = (a − b) / (a + b)
    const ratio = divNumeric(subNumeric(a, b), addNumeric(a, b));
    if (!isPositiveNumeric(ratio)) return undefined;
    const sqrtRatioIR = weierstrassSqrtFractionIR(ratio);
    atanArg = app(MUL, [sqrtRatioIR, tanHalf]);
  }
  // Outer coefficient: 2c / √(a²−b²)
  const coefFrac = mulNumeric(mulNumeric(c, { kind: "int", value: 2n }), coefSign);
  const coefIR = app(DIV, [fromNumeric(coefFrac), sqrtDiscIR]);
  return app(MUL, [coefIR, app(ATAN, [atanArg])]);
}

/** True when ``v`` is an exact rational/integer strictly greater than zero. */
function isPositiveNumeric(v: Numeric): boolean {
  if (v.kind === "int") return v.value > 0n;
  if (v.kind === "rat") return v.numer > 0n; // denominator is always positive
  return v.value > 0;
}

/** True when ``v`` is exactly zero (integer 0, rational 0/q, or float 0.0). */
function isZeroNumeric(v: Numeric): boolean {
  if (v.kind === "int") return v.value === 0n;
  if (v.kind === "rat") return v.numer === 0n;
  return v.value === 0;
}

/** True when two Numeric values are exactly equal (Int/Rat only — float
 *  comparisons elsewhere use tolerance; Phase 35 needs exact equality). */
function eqNumeric(a: Numeric, b: Numeric): boolean {
  if (a.kind === "int" && b.kind === "int") return a.value === b.value;
  if (a.kind === "rat" && b.kind === "rat")
    return a.numer === b.numer && a.denom === b.denom;
  if (a.kind === "int" && b.kind === "rat")
    return b.denom === 1n && b.numer === a.value;
  if (a.kind === "rat" && b.kind === "int")
    return a.denom === 1n && a.numer === b.value;
  return false;
}

/**
 * Phase 35: degenerate ``a² = b²`` Weierstrass cases. Four sign
 * combinations × {SIN, COS} yield clean closed forms in ``tan(x/2)``
 * without any ``Sqrt`` or ``Atan`` wrapper:
 *
 *  - sin, b ==  a : ``-2c / (a · (tan(x/2) + 1))``
 *  - sin, b == -a : `` 2c / (a · (1 − tan(x/2)))``
 *  - cos, b ==  a : ``c · tan(x/2) / a``
 *  - cos, b == -a : ``-c / (a · tan(x/2))``  (= -c·cot(x/2)/a)
 *
 * Returns ``undefined`` when neither ``b == a`` nor ``b == -a``, or when
 * ``a == 0`` (zero denominator, not integrable).
 */
function tryWeierstrassDegenerate(
  c: Numeric,
  a: Numeric,
  b: Numeric,
  trigHead: IRNode,
  argNode: IRNode
): IRNode | undefined {
  if (isZeroNumeric(a)) return undefined;
  // Phase 38 generalisation: `argNode` is the IR for ``α·x + β``; the
  // inner factor ``α`` has been pre-absorbed into ``c`` by the caller.
  const tanHalf = app(TAN, [app(DIV, [argNode, int(2)])]);
  const negA = negNumeric(a);
  if (equals(trigHead, SIN)) {
    if (eqNumeric(b, a)) {
      // -2c / (a · (tan(x/2) + 1))
      const neg2c = negNumeric(mulNumeric(c, { kind: "int", value: 2n }));
      const denom = app(MUL, [fromNumeric(a), app(ADD, [tanHalf, int(1)])]);
      return app(DIV, [fromNumeric(neg2c), denom]);
    }
    if (eqNumeric(b, negA)) {
      // 2c / (a · (1 − tan(x/2)))
      const two_c = mulNumeric(c, { kind: "int", value: 2n });
      const denom = app(MUL, [fromNumeric(a), app(SUB, [int(1), tanHalf])]);
      return app(DIV, [fromNumeric(two_c), denom]);
    }
    return undefined;
  }
  // COS branch
  if (eqNumeric(b, a)) {
    // c · tan(x/2) / a
    const numer = app(MUL, [fromNumeric(c), tanHalf]);
    return app(DIV, [numer, fromNumeric(a)]);
  }
  if (eqNumeric(b, negA)) {
    // -c / (a · tan(x/2))
    const negC = negNumeric(c);
    const denom = app(MUL, [fromNumeric(a), tanHalf]);
    return app(DIV, [fromNumeric(negC), denom]);
  }
  return undefined;
}

/** Numeric absolute value (Int/Rat only — float passes through). */
function absNumeric(v: Numeric): Numeric {
  if (v.kind === "int") return { kind: "int", value: v.value < 0n ? -v.value : v.value };
  if (v.kind === "rat") return { kind: "rat", numer: v.numer < 0n ? -v.numer : v.numer, denom: v.denom };
  return { kind: "float", value: Math.abs(v.value) };
}

/**
 * Phase 36: Weierstrass log form for ``a² < b²``.
 *
 *   ∫ c/(a + b·sin x) dx  =  (c/D)·log|(a·tan(x/2)+b−D)/(a·tan(x/2)+b+D)| + C
 *   ∫ c/(a + b·cos x) dx  =  (c/D)·log|(D+(b−a)·tan(x/2))/(D−(b−a)·tan(x/2))| + C
 *
 * with ``D = √(b²−a²) > 0``.  The ``a = 0`` sin/csc subcase closes as
 * ``(c/b)·log|tan(x/2)|``.  The cos branch handles both sign regimes via
 * the Abs-wrapped Phase 37 form.
 */
function tryWeierstrassLogForm(
  c: Numeric,
  a: Numeric,
  b: Numeric,
  trigHead: IRNode,
  argNode: IRNode
): IRNode | undefined {
  // Phase 38 generalisation: `argNode` is the IR for ``α·x + β``; the
  // inner factor ``α`` has been pre-absorbed into ``c`` by the caller.
  // discSq = b² − a²; caller passes disc = a² − b² < 0, so discSq > 0.
  const discSq = subNumeric(mulNumeric(b, b), mulNumeric(a, a));
  if (!isPositiveNumeric(discSq)) return undefined;
  const sqrtDiscIR = weierstrassSqrtFractionIR(discSq);
  const tanHalf = app(TAN, [app(DIV, [argNode, int(2)])]);
  const absHead = sym("Abs");
  if (equals(trigHead, SIN)) {
    if (isZeroNumeric(a)) {
      // ∫ c/(b·sin u) dx = (c/b)·log|tan(u/2)|.  Any linear argument
      // scaling has already been absorbed into c by the dispatcher.
      const coefIR = fromNumeric(divNumeric(c, b));
      const logArg = app(absHead, [tanHalf]);
      return app(MUL, [coefIR, app(LOG, [logArg])]);
    }
    // log|(a·tan(x/2) + b − D) / (a·tan(x/2) + b + D)|
    const aTan = app(MUL, [fromNumeric(a), tanHalf]);
    const aTanPlusB = app(ADD, [aTan, fromNumeric(b)]);
    const numer = app(SUB, [aTanPlusB, sqrtDiscIR]);
    const denom = app(ADD, [aTanPlusB, sqrtDiscIR]);
    const logArg = app(absHead, [app(DIV, [numer, denom])]);
    const coefIR = app(DIV, [fromNumeric(c), sqrtDiscIR]);
    return app(MUL, [coefIR, app(LOG, [logArg])]);
  }
  // COS branch — handles both b > |a| and b < −|a| (Phase 37 extension).
  //
  // The same expression log|(D + (b−a)·tan(x/2)) / (D − (b−a)·tan(x/2))|
  // is valid for both sign regimes because the inner rational is wrapped
  // in Abs: when (b−a) flips sign, the numerator and denominator of the
  // log argument swap (D − k·u and D + k·u), but |N/D'| = |D'/N| so the
  // absolute value collapses them to the same value.  The antiderivative
  // is continuous across both sides of b = ±|a|.
  //
  // Caller already ensures b² > a² (disc < 0 entry); the only additional
  // precondition is a + b ≠ 0, which is automatic because b² > a² rules
  // out b = −a.
  const bMinusA = subNumeric(b, a);
  // log|(D + (b−a)·tan(x/2)) / (D − (b−a)·tan(x/2))|
  const bmaTan = app(MUL, [fromNumeric(bMinusA), tanHalf]);
  const numer = app(ADD, [sqrtDiscIR, bmaTan]);
  const denom = app(SUB, [sqrtDiscIR, bmaTan]);
  const logArg = app(absHead, [app(DIV, [numer, denom])]);
  const coefIR = app(DIV, [fromNumeric(c), sqrtDiscIR]);
  return app(MUL, [coefIR, app(LOG, [logArg])]);
}

// ---------------------------------------------------------------------------
// Track G2 — symbolic-coefficient Weierstrass lift (TypeScript port).
//
// The numeric helpers above parse ``a, b`` as ``Numeric`` and bail out
// when either is not a literal Int/Rat.  Track G2 generalises them:
// when the numeric path can't fire because ``a`` and/or ``b`` is a
// free IR symbol or any non-numeric IR expression, we re-parse the
// integrand keeping ``a, b`` as IR nodes (``α, β, c`` stay rational —
// only the outer trig coefficient pair is allowed to be symbolic),
// then query ``vm.assumptions`` for the sign of the discriminant
// ``a² − b²`` to decide which closed form to emit:
//
//   disc > 0 → arctan form with Sqrt(a²−b²)
//   disc < 0 → log form with Sqrt(b²−a²)
//   disc = 0 → degenerate rational-in-tan(arg/2) form
//   no fact  → return undefined (integrator leaves it unevaluated)
//
// Linear-argument lifting ``α·x + β`` composes unchanged — the inner
// substitution ``u = tan((α·x+β)/2)`` does not depend on the values
// of the outer coefficients ``a, b``, so we still fold ``1/α`` into
// the numerator scaling exactly as the numeric path does.  Mirrors
// the Python helper at ``symbolic_vm/integrate.py``.
// ---------------------------------------------------------------------------

/** Match ``c·sin(α·x+β)`` / ``c·cos(α·x+β)`` returning ``c`` as an IR
 *  node (instead of a Numeric).  ``α, β`` stay rational because the
 *  inner linear-form is what makes the Weierstrass substitution
 *  composable; only the outer scalar ``c`` is allowed to be symbolic,
 *  since that's what flows into ``a, b`` for the dispatcher below. */
function weierstrassParseConstTimesTrigLinearSymbolic(
  node: IRNode,
  x: IRNode,
):
  | { readonly c: IRNode; readonly head: IRNode; readonly alpha: Numeric; readonly beta: Numeric }
  | undefined {
  if (
    node.kind === "apply" &&
    (equals(node.head, SIN) || equals(node.head, COS)) &&
    node.args.length === 1
  ) {
    const lin = weierstrassParseLinearInX(node.args[0], x);
    if (lin !== undefined) {
      return { c: int(1), head: node.head, alpha: lin.alpha, beta: lin.beta };
    }
  }
  if (node.kind === "apply" && equals(node.head, MUL) && node.args.length === 2) {
    const [left, right] = node.args;
    for (const [constSide, trigSide] of [
      [left, right],
      [right, left],
    ] as const) {
      if (dependsOn(constSide, x)) continue;
      if (
        trigSide.kind === "apply" &&
        (equals(trigSide.head, SIN) || equals(trigSide.head, COS)) &&
        trigSide.args.length === 1
      ) {
        const lin = weierstrassParseLinearInX(trigSide.args[0], x);
        if (lin !== undefined) {
          return { c: constSide, head: trigSide.head, alpha: lin.alpha, beta: lin.beta };
        }
      }
    }
  }
  if (node.kind === "apply" && equals(node.head, NEG) && node.args.length === 1) {
    const inner = weierstrassParseConstTimesTrigLinearSymbolic(node.args[0], x);
    if (inner !== undefined) {
      return { c: app(NEG, [inner.c]), head: inner.head, alpha: inner.alpha, beta: inner.beta };
    }
  }
  return undefined;
}

/** Symbolic-coefficient sibling of {@link weierstrassParseAPlusBSincos}.
 *  Parses ``a + b·sin(α·x+β)`` / ``a + b·cos(α·x+β)`` (any operand
 *  order, ADD or SUB) into ``(a, b, head, α, β)`` where ``a`` and
 *  ``b`` are IR nodes free of ``x`` and ``α, β`` are rational with
 *  ``α ≠ 0``.  Returns undefined when the shape doesn't fit. */
function weierstrassParseAPlusBSincosSymbolic(
  node: IRNode,
  x: IRNode,
):
  | {
      readonly a: IRNode;
      readonly b: IRNode;
      readonly trigHead: IRNode;
      readonly alpha: Numeric;
      readonly beta: Numeric;
    }
  | undefined {
  const bareTrig = weierstrassParseConstTimesTrigLinearSymbolic(node, x);
  if (bareTrig !== undefined) {
    return { a: int(0), b: bareTrig.c, trigHead: bareTrig.head, alpha: bareTrig.alpha, beta: bareTrig.beta };
  }
  if (node.kind !== "apply" || node.args.length !== 2) return undefined;
  if (equals(node.head, ADD)) {
    const [left, right] = node.args;
    for (const [constSide, trigSide] of [
      [left, right],
      [right, left],
    ] as const) {
      if (dependsOn(constSide, x)) continue;
      const trigParse = weierstrassParseConstTimesTrigLinearSymbolic(trigSide, x);
      if (trigParse === undefined) continue;
      return {
        a: constSide,
        b: trigParse.c,
        trigHead: trigParse.head,
        alpha: trigParse.alpha,
        beta: trigParse.beta,
      };
    }
    return undefined;
  }
  if (equals(node.head, SUB)) {
    const [left, right] = node.args;
    // ``a − b·trig(...)`` → ``(a, −b, head, α, β)``.
    if (!dependsOn(left, x)) {
      const trigParse = weierstrassParseConstTimesTrigLinearSymbolic(right, x);
      if (trigParse !== undefined) {
        return {
          a: left,
          b: app(NEG, [trigParse.c]),
          trigHead: trigParse.head,
          alpha: trigParse.alpha,
          beta: trigParse.beta,
        };
      }
    }
    // ``b·trig(...) − a`` → ``(−a, b, head, α, β)``.
    if (!dependsOn(right, x)) {
      const trigParse = weierstrassParseConstTimesTrigLinearSymbolic(left, x);
      if (trigParse !== undefined) {
        return {
          a: app(NEG, [right]),
          b: trigParse.c,
          trigHead: trigParse.head,
          alpha: trigParse.alpha,
          beta: trigParse.beta,
        };
      }
    }
    return undefined;
  }
  return undefined;
}

/** Construct the discriminant ``a² − b²`` as IR.  Used as both the
 *  query operand for ``vm.assumptions`` and the radicand of the
 *  closed-form ``Sqrt(...)`` term. */
function weierstrassDiscExpr(a: IRNode, b: IRNode): IRNode {
  return app(SUB, [app(POW, [a, int(2)]), app(POW, [b, int(2)])]);
}

/** ``b² − a²`` — radicand of the log-branch ``Sqrt(b²−a²)`` for the
 *  ``disc < 0`` case. */
function weierstrassNegDiscExpr(a: IRNode, b: IRNode): IRNode {
  return app(SUB, [app(POW, [b, int(2)]), app(POW, [a, int(2)])]);
}

/** Symbolic-coefficient arctan branch: emitted when the assumption
 *  store says ``a² > b²``.  ``c_scaled`` already absorbs ``1/α`` from
 *  the linear-arg lift; we just need to emit the closed form with
 *  symbolic ``Sqrt(a²−b²)``. */
function tryWeierstrassArctanSymbolic(
  cScaled: IRNode,
  a: IRNode,
  b: IRNode,
  trigHead: IRNode,
  argNode: IRNode,
): IRNode {
  const sqrtDisc = app(SQRT, [weierstrassDiscExpr(a, b)]);
  const tanHalf = app(TAN, [app(DIV, [argNode, int(2)])]);
  let atanArgTop: IRNode;
  if (equals(trigHead, SIN)) {
    // (a·tan(arg/2) + b) / √(a²−b²)
    atanArgTop = app(ADD, [app(MUL, [a, tanHalf]), b]);
  } else {
    // cos branch — same sign-clean form as the Python port:
    // (a−b)·tan(arg/2) / √(a²−b²).  See the inline derivation in
    // the Python helper for why this is correct on the whole disc>0
    // region.
    atanArgTop = app(MUL, [app(SUB, [a, b]), tanHalf]);
  }
  const atanArg = app(DIV, [atanArgTop, sqrtDisc]);
  const coef = app(DIV, [app(MUL, [int(2), cScaled]), sqrtDisc]);
  return app(MUL, [coef, app(ATAN, [atanArg])]);
}

/** Symbolic-coefficient log branch: emitted when the assumption store
 *  says ``a² < b²``.  Mirrors the numeric :func:`tryWeierstrassLogForm`. */
function tryWeierstrassLogSymbolic(
  cScaled: IRNode,
  a: IRNode,
  b: IRNode,
  trigHead: IRNode,
  argNode: IRNode,
): IRNode {
  const sqrtNegDisc = app(SQRT, [weierstrassNegDiscExpr(a, b)]);
  const tanHalf = app(TAN, [app(DIV, [argNode, int(2)])]);
  const absHead = sym("Abs");
  let numer: IRNode;
  let denom: IRNode;
  if (equals(trigHead, SIN)) {
    const aTan = app(MUL, [a, tanHalf]);
    const aTanPlusB = app(ADD, [aTan, b]);
    numer = app(SUB, [aTanPlusB, sqrtNegDisc]);
    denom = app(ADD, [aTanPlusB, sqrtNegDisc]);
  } else {
    const bma = app(SUB, [b, a]);
    const bmaTan = app(MUL, [bma, tanHalf]);
    numer = app(ADD, [sqrtNegDisc, bmaTan]);
    denom = app(SUB, [sqrtNegDisc, bmaTan]);
  }
  const logArg = app(absHead, [app(DIV, [numer, denom])]);
  const coef = app(DIV, [cScaled, sqrtNegDisc]);
  return app(MUL, [coef, app(LOG, [logArg])]);
}

/** Symbolic-coefficient degenerate branch: ``a² = b²``.  See the
 *  Python helper for the derivation; both forms reduce to the numeric
 *  Phase-35 results when ``a, b`` happen to be concrete. */
function tryWeierstrassDegenerateSymbolic(
  cScaled: IRNode,
  a: IRNode,
  b: IRNode,
  trigHead: IRNode,
  argNode: IRNode,
): IRNode {
  const tanHalf = app(TAN, [app(DIV, [argNode, int(2)])]);
  const aPlusB = app(ADD, [a, b]);
  const aMinusB = app(SUB, [a, b]);
  if (equals(trigHead, SIN)) {
    // −2·c / ( (a+b)·tan(arg/2) + (a−b) )
    const numer = app(MUL, [int(-2), cScaled]);
    const denom = app(ADD, [app(MUL, [aPlusB, tanHalf]), aMinusB]);
    return app(DIV, [numer, denom]);
  }
  // cos: 2·c·tan(arg/2) / ( (a−b)·tan²(arg/2) + (a+b) )
  const tanSq = app(POW, [tanHalf, int(2)]);
  const numer = app(MUL, [app(MUL, [int(2), cScaled]), tanHalf]);
  const denom = app(ADD, [app(MUL, [aMinusB, tanSq]), aPlusB]);
  return app(DIV, [numer, denom]);
}

/** Track G2 entry point.  Mirrors the numeric
 *  {@link tryWeierstrassOneOverLinearTrig} but accepts non-numeric
 *  ``a, b`` (IR nodes free of ``x``).  Returns undefined when:
 *   - the integrand doesn't match the shape,
 *   - the numerator ``c`` depends on ``x``,
 *   - ``α, β`` aren't rational,
 *   - no assumption context is available (called outside the handler),
 *   - or no assumption pins down the sign of ``a² − b²``.
 *
 *  The numeric path is left untouched: the dispatcher tries it first
 *  and only falls through to here if it returned undefined. */
function tryWeierstrassSymbolicCoefficients(
  integrand: IRNode,
  x: IRNode,
): IRNode | undefined {
  if (integrand.kind !== "apply" || !equals(integrand.head, DIV)) return undefined;
  if (integrand.args.length !== 2) return undefined;
  const [num, den] = integrand.args;
  if (dependsOn(num, x)) return undefined;
  const parsed = weierstrassParseAPlusBSincosSymbolic(den, x);
  if (parsed === undefined) return undefined;
  const { a, b, trigHead, alpha, beta } = parsed;
  if (isZeroNumeric(alpha)) return undefined;
  // Numeric path is tried first; if both ``a`` and ``b`` are
  // numeric, that path would have closed the integral already.  Bail
  // out gracefully to avoid emitting a second (potentially uglier)
  // result.
  if (toNumeric(a) !== undefined && toNumeric(b) !== undefined) return undefined;
  const assumptions = currentAssumptions();
  if (assumptions === undefined) return undefined;
  // Apply ``u = α·x + β`` change of variable: scale numerator by 1/α.
  const oneOverAlpha = divNumeric({ kind: "int", value: 1n }, alpha);
  const cScaled = isOne(fromNumeric(oneOverAlpha))
    ? num
    : app(MUL, [fromNumeric(oneOverAlpha), num]);
  const argNode = weierstrassBuildLinearArgIR(alpha, beta, x);
  // Probe both surface forms of the discriminant sign — the natural
  // ``a² > b²`` written by the user and the canonical-against-zero
  // form ``a² − b² > 0`` someone might write programmatically.
  const aSq = app(POW, [a, int(2)]);
  const bSq = app(POW, [b, int(2)]);
  const disc = app(SUB, [aSq, bSq]);
  if (
    assumptions.isTrueRelation(app(GREATER, [aSq, bSq])) === true ||
    assumptions.isTrueRelation(app(GREATER, [disc, int(0)])) === true
  ) {
    return tryWeierstrassArctanSymbolic(cScaled, a, b, trigHead, argNode);
  }
  if (
    assumptions.isTrueRelation(app(LESS, [aSq, bSq])) === true ||
    assumptions.isTrueRelation(app(LESS, [disc, int(0)])) === true
  ) {
    return tryWeierstrassLogSymbolic(cScaled, a, b, trigHead, argNode);
  }
  if (
    assumptions.isTrueRelation(app(EQUAL, [aSq, bSq])) === true ||
    assumptions.isTrueRelation(app(EQUAL, [disc, int(0)])) === true
  ) {
    return tryWeierstrassDegenerateSymbolic(cScaled, a, b, trigHead, argNode);
  }
  return undefined;
}

function integrateIndefinite(f: IRNode, x: IRNode): IRNode | undefined {
  if (!dependsOn(f, x)) {
    return app(MUL, [f, x]);
  }
  if (equals(f, x)) {
    return app(MUL, [rational(1, 2), app(POW, [x, int(2)])]);
  }
  const elliptic = incompleteEllipticFirstKind(f, x);
  if (elliptic !== undefined) {
    return elliptic;
  }
  const ellipticE = incompleteEllipticSecondKind(f, x);
  if (ellipticE !== undefined) {
    return ellipticE;
  }
  if (f.kind !== "apply") {
    return undefined;
  }

  const erf = tryErfIntegral(f, x);
  if (erf !== undefined) return erf;

  const fresnel = tryFresnelIntegral(f, x);
  if (fresnel !== undefined) return fresnel;

  if (equals(f.head, ADD)) {
    const pieces = f.args.map((arg) => integrateIndefinite(arg, x));
    if (pieces.some((piece) => piece === undefined)) return undefined;
    return binaryChain(ADD, pieces as IRNode[]);
  }
  if (equals(f.head, SUB)) {
    const [a, b] = binaryArgs(f);
    const ia = integrateIndefinite(a, x);
    const ib = integrateIndefinite(b, x);
    return ia === undefined || ib === undefined ? undefined : app(SUB, [ia, ib]);
  }
  if (equals(f.head, NEG)) {
    const [inner] = unaryArgs(f);
    const integrated = integrateIndefinite(inner, x);
    return integrated === undefined ? undefined : app(NEG, [integrated]);
  }
  if (equals(f.head, MUL)) {
    const [a, b] = binaryArgs(f);
    if (!dependsOn(a, x)) {
      const ib = integrateIndefinite(b, x);
      return ib === undefined ? undefined : app(MUL, [a, ib]);
    }
    if (!dependsOn(b, x)) {
      const ia = integrateIndefinite(a, x);
      return ia === undefined ? undefined : app(MUL, [b, ia]);
    }
    // Phase 26: Q(x) · log(x)^n for integer n ≥ 2
    const lp26 =
      tryLogPowerProduct(a, b, x) ?? tryLogPowerProduct(b, a, x);
    if (lp26 !== undefined) return lp26;
    // Phase 27: Q(x) · sin(log(x)) or Q(x) · cos(log(x))
    const tl27 =
      tryTrigLogProduct(a, b, x) ?? tryTrigLogProduct(b, a, x);
    if (tl27 !== undefined) return tl27;
    // Phase 28: ∫ P(x)·log(Q(x)) dx  and  ∫ P(x)·atan(Q(x)) dx  (non-linear Q)
    // Phase 12: ∫ P(x)·asin/acos(ax+b) dx via IBP.
    const invTrig12 = tryAsinAcosPolyProduct(a, b, x) ?? tryAsinAcosPolyProduct(b, a, x);
    if (invTrig12 !== undefined) return invTrig12;
    const hyp13 =
      trySinhCoshPolyProduct(a, b, x) ?? trySinhCoshPolyProduct(b, a, x) ??
      tryAsinhAcoshPolyProduct(a, b, x) ?? tryAsinhAcoshPolyProduct(b, a, x);
    if (hyp13 !== undefined) return hyp13;
    const lp28 = tryLogPolyProduct(a, b, x) ?? tryLogPolyProduct(b, a, x);
    if (lp28 !== undefined) return lp28;
    const ap28 = tryAtanPolyProduct(a, b, x) ?? tryAtanPolyProduct(b, a, x);
    if (ap28 !== undefined) return ap28;
    return undefined;
  }
  if (equals(f.head, DIV)) {
    const [numerator, denominator] = binaryArgs(f);
    if (!dependsOn(numerator, x) && equals(denominator, x)) {
      return app(MUL, [numerator, app(LOG, [x])]);
    }
    // Phase 34: Weierstrass substitution for c / (a + b·sin(x)) and
    // c / (a + b·cos(x)) when a, b numeric and a² > b² (a > 0 for cos).
    const weier = tryWeierstrassOneOverLinearTrig(f, x);
    if (weier !== undefined) return weier;
    // Track G2: symbolic-coefficient Weierstrass.  Fires only when the
    // numeric path returns undefined and ``vm.assumptions`` records a
    // sign for ``a² − b²``.  See
    // {@link tryWeierstrassSymbolicCoefficients}.
    const weierSym = tryWeierstrassSymbolicCoefficients(f, x);
    if (weierSym !== undefined) return weierSym;
    return undefined;
  }
  if (equals(f.head, POW)) {
    const [base, exponent] = binaryArgs(f);
    if (equals(base, x) && exactRational(exponent) !== undefined) {
      const n = exactRational(exponent)!;
      if (n.numer === -n.denom) {
        return app(LOG, [x]);
      }
      const next = makeRat(n.numer + n.denom, n.denom);
      return app(MUL, [fromNumeric(divNumeric({ kind: "int", value: 1n }, next)), app(POW, [x, fromNumeric(next)])]);
    }
    if (!dependsOn(base, x) && equals(exponent, x)) {
      return app(DIV, [f, app(LOG, [base])]);
    }
    // Phase 26: ∫ log(x)^n dx for integer n ≥ 2 (standalone, no poly factor).
    if (
      base.kind === "apply" &&
      equals(base.head, LOG) &&
      base.args.length === 1 &&
      equals(base.args[0], x)
    ) {
      const n26 = exactRational(exponent);
      if (n26 !== undefined && n26.denom === 1n && n26.numer >= 2n) {
        return polyLogPowerTerm(0, Number(n26.numer), x);
      }
    }
    const recipHyp16 = tryRecipHypPower(base, exponent, x);
    if (recipHyp16 !== undefined) return recipHyp16;
    return undefined;
  }

  const [inner] = f.args.length === 1 ? [f.args[0]] : [undefined];
  if (inner !== undefined && equals(inner, x)) {
    if (equals(f.head, SIN)) return app(NEG, [app(COS, [x])]);
    if (equals(f.head, COS)) return app(SIN, [x]);
    if (equals(f.head, EXP)) return app(EXP, [x]);
    if (equals(f.head, LOG)) return app(SUB, [app(MUL, [x, app(LOG, [x])]), x]);
    if (equals(f.head, SQRT)) {
      return app(MUL, [rational(2, 3), app(POW, [x, rational(3, 2)])]);
    }
  }

  // Phase 27: ∫ sin(log(x)) dx and ∫ cos(log(x)) dx (k=0 single-factor form).
  if (
    inner !== undefined &&
    inner.kind === "apply" &&
    equals(inner.head, LOG) &&
    inner.args.length === 1 &&
    equals(inner.args[0], x)
  ) {
    if (equals(f.head, SIN)) return trigLogIntegral(SIN, 0, x);
    if (equals(f.head, COS)) return trigLogIntegral(COS, 0, x);
  }

  // Phase 28: bare ∫ log(Q(x)) dx and ∫ atan(Q(x)) dx for non-linear Q(x).
  // These are handled as ∫ 1·log(Q) dx and ∫ 1·atan(Q) dx via IBP.
  if (f.args.length === 1 && equals(f.head, LOG)) {
    const lp28 = tryLogPolyProduct(f, int(1), x);
    if (lp28 !== undefined) return lp28;
  }
  if (f.args.length === 1 && equals(f.head, ATAN)) {
    const ap28 = tryAtanPolyProduct(f, int(1), x);
    if (ap28 !== undefined) return ap28;
  }
  // Phase 12: bare ∫ asin/acos(ax+b) dx.
  if (f.args.length === 1 && (equals(f.head, ASIN) || equals(f.head, ACOS))) {
    const invTrig12 = tryAsinAcosPolyProduct(f, int(1), x);
    if (invTrig12 !== undefined) return invTrig12;
  }
  if (f.args.length === 1 && (equals(f.head, SINH) || equals(f.head, COSH))) {
    const hyp13 = trySinhCoshPolyProduct(f, int(1), x);
    if (hyp13 !== undefined) return hyp13;
  }
  if (f.args.length === 1 && (equals(f.head, ASINH) || equals(f.head, ACOSH))) {
    const invHyp13 = tryAsinhAcoshPolyProduct(f, int(1), x);
    if (invHyp13 !== undefined) return invHyp13;
  }
  if (f.args.length === 1 && (equals(f.head, COTH) || equals(f.head, SECH) || equals(f.head, CSCH))) {
    const recipHyp15 = tryRecipHypLinear(f, x);
    if (recipHyp15 !== undefined) return recipHyp15;
  }
  if (f.args.length === 1 && (equals(f.head, TANH) || equals(f.head, ATANH))) {
    const tanh13 = tryTanhAtanhLinear(f, x);
    if (tanh13 !== undefined) return tanh13;
  }

  return undefined;
}

// ---------------------------------------------------------------------------
// Track E2 — Generic tabular integration-by-parts fallback.
//
// Mirrors ``ibp_tabular.py`` from the Python reference port (Track E1).
// When the pipeline's shape-specific handlers in ``integrateIndefinite``
// have all returned ``undefined`` for a ``Mul``-shaped integrand, this
// fallback makes a last-ditch attempt by **generic tabular IBP**:
//
//   For ``f = u(x) · w(x)`` where ``u`` is polynomial in ``x``:
//     ∫ u·w dx = Σ_{k=0}^{N-1} (-1)^k · u^(k)(x) · I^(k+1)(w)
//
// where N = deg(u) + 1 (so u^(N) = 0 and the trailing remainder
// vanishes).  The I-column entries ``∫w, ∫∫w, ..., ∫^N w`` come from
// the recursive ``integrateIndefinite`` callback; if any step fails
// to close, the partition is abandoned.
//
// Bounded by ``IBP_MAX_FACTORS = 5`` (number of flattened Mul factors)
// and ``IBP_MAX_POLY_DEGREE = 8`` (degree of the polynomial column).
// ---------------------------------------------------------------------------

const IBP_MAX_FACTORS = 5;
const IBP_MAX_POLY_DEGREE = 8;

/** Flatten a (possibly nested-binary) ``Mul`` tree into a list of leaves.
 *  ``Mul(a, Mul(b, Mul(c, d)))`` → ``[a, b, c, d]``.  Without flattening
 *  the IBP search would miss splits like ``u = a·c, w = b·d`` purely
 *  because the parse tree happened to group differently. */
function ibpFlattenMul(node: IRNode): IRNode[] {
  if (node.kind !== "apply" || !equals(node.head, MUL)) {
    return [node];
  }
  const out: IRNode[] = [];
  for (const arg of node.args) {
    out.push(...ibpFlattenMul(arg));
  }
  return out;
}

/** Rebuild a left-associative ``Mul`` chain from a list of factors.
 *  Empty list → ``1``; single factor returns itself. */
function ibpMultiplyIr(factors: readonly IRNode[]): IRNode {
  if (factors.length === 0) return int(1);
  if (factors.length === 1) return factors[0];
  let acc: IRNode = factors[0];
  for (let i = 1; i < factors.length; i += 1) {
    acc = app(MUL, [acc, factors[i]]);
  }
  return acc;
}

/** Return the polynomial degree of ``node`` in ``x``, or ``undefined``
 *  if it is not in Q[x].  ``-1`` denotes the zero polynomial.  Mirrors
 *  the Python ``_polynomial_degree`` helper. */
function ibpPolynomialDegree(node: IRNode, x: IRSymbol): number | undefined {
  const r = toRational(node, x);
  if (r === undefined) return undefined;
  if (polyQDegree(r.den) > 0) return undefined; // rational, not polynomial
  const n = polyQNormalize(r.num);
  if (n.length === 0) return -1; // zero polynomial
  return n.length - 1;
}

/** True if ``node`` contains any unevaluated ``Integrate(...)`` sub-tree.
 *  Used to reject I-column entries the recursive integrator could not
 *  close to a true antiderivative. */
function ibpContainsIntegrate(node: IRNode): boolean {
  if (node.kind === "apply") {
    if (equals(node.head, INTEGRATE)) return true;
    return node.args.some((a) => ibpContainsIntegrate(a));
  }
  return false;
}

/** True iff ``node`` canonicalises to the integer literal ``0``.
 *  Also recognises ``Neg(0)``. */
function ibpIsZero(node: IRNode): boolean {
  if (node.kind === "integer" && node.value === 0n) return true;
  if (node.kind === "apply" && equals(node.head, NEG) && node.args.length === 1) {
    return ibpIsZero(node.args[0]);
  }
  return false;
}

/** Enumerate ``k``-element subsets of ``[0, n)`` as index arrays. */
function ibpCombinations(n: number, k: number): number[][] {
  const out: number[][] = [];
  const pick: number[] = [];
  const walk = (start: number): void => {
    if (pick.length === k) {
      out.push(pick.slice());
      return;
    }
    for (let i = start; i < n; i += 1) {
      pick.push(i);
      walk(i + 1);
      pick.pop();
    }
  };
  walk(0);
  return out;
}

/** Attempt generic tabular IBP on a ``Mul``-shaped integrand.
 *  Returns the closed-form antiderivative as IR, or ``undefined`` when
 *  no viable ``(u, w)`` split was found. */
function tryIbpTabular(
  f: IRNode,
  x: IRSymbol,
  integrateFn: (g: IRNode) => IRNode | undefined,
  diffFn: (g: IRNode) => IRNode,
  simplifyFn: (n: IRNode) => IRNode,
): IRNode | undefined {
  // Only fires on Mul — every other shape has dedicated handlers.
  if (f.kind !== "apply" || !equals(f.head, MUL)) return undefined;
  const factors = ibpFlattenMul(f);
  if (factors.length < 2 || factors.length > IBP_MAX_FACTORS) return undefined;
  const n = factors.length;
  // Prefer smaller ``u`` first — tabular IBP is most efficient when ``u``
  // is low-degree.  Enumerate subset partitions of size 1 .. n-1.
  for (let uSize = 1; uSize < n; uSize += 1) {
    for (const uIdx of ibpCombinations(n, uSize)) {
      const uSet = new Set(uIdx);
      const uFactors: IRNode[] = [];
      const wFactors: IRNode[] = [];
      for (let i = 0; i < n; i += 1) {
        if (uSet.has(i)) uFactors.push(factors[i]);
        else wFactors.push(factors[i]);
      }
      const result = ibpTrySplit(uFactors, wFactors, x, integrateFn, diffFn, simplifyFn);
      if (result !== undefined) return result;
    }
  }
  return undefined;
}

/** Try ``u = ∏ uFactors``, ``w = ∏ wFactors`` as the tabular split. */
function ibpTrySplit(
  uFactors: readonly IRNode[],
  wFactors: readonly IRNode[],
  x: IRSymbol,
  integrateFn: (g: IRNode) => IRNode | undefined,
  diffFn: (g: IRNode) => IRNode,
  simplifyFn: (n: IRNode) => IRNode,
): IRNode | undefined {
  const uIr = simplifyFn(ibpMultiplyIr(uFactors));
  const deg = ibpPolynomialDegree(uIr, x);
  if (deg === undefined) return undefined;
  if (deg < 0) {
    // u is the zero polynomial — ∫ 0·w dx = 0.
    return int(0);
  }
  if (deg > IBP_MAX_POLY_DEGREE) return undefined;

  // D-column: u, u', u'', ..., 0.
  const dCol: IRNode[] = [uIr];
  let cur: IRNode = uIr;
  for (let i = 0; i <= deg; i += 1) {
    cur = simplifyFn(diffFn(cur));
    dCol.push(cur);
    if (ibpIsZero(cur)) break;
  }
  if (!ibpIsZero(dCol[dCol.length - 1])) return undefined;
  const N = dCol.length - 1; // u^(N) = 0

  // I-column: w, ∫w, ∫∫w, ..., ∫^N w.
  const wIr = simplifyFn(ibpMultiplyIr(wFactors));
  const iCol: IRNode[] = [wIr];
  cur = wIr;
  for (let k = 0; k < N; k += 1) {
    const integrated = integrateFn(cur);
    if (integrated === undefined) return undefined;
    const simplified = simplifyFn(integrated);
    if (ibpContainsIntegrate(simplified)) return undefined;
    iCol.push(simplified);
    cur = simplified;
  }

  // Assemble: Σ_{k=0}^{N-1} (-1)^k · D[k] · I[k+1].
  const pieces: IRNode[] = [];
  for (let k = 0; k < N; k += 1) {
    let term: IRNode = app(MUL, [dCol[k], iCol[k + 1]]);
    if (k % 2 === 1) term = app(NEG, [term]);
    pieces.push(term);
  }
  if (pieces.length === 0) return int(0);
  let result: IRNode = pieces[0];
  for (let i = 1; i < pieces.length; i += 1) {
    result = app(ADD, [result, pieces[i]]);
  }
  return result;
}

function completeEllipticFirstKind(f: IRNode, x: IRNode, lower: IRNode, upper: IRNode): IRNode | undefined {
  if (!isZero(lower) || !isPiOverTwo(upper)) {
    return undefined;
  }
  const modulus = ellipticFirstKindModulus(f, x);
  return modulus === undefined ? undefined : app(sym("EllipticK"), [modulus]);
}

function incompleteEllipticFirstKind(f: IRNode, x: IRNode): IRNode | undefined {
  const modulus = ellipticFirstKindModulus(f, x);
  return modulus === undefined ? undefined : app(sym("EllipticF"), [x, modulus]);
}

function ellipticFirstKindModulus(f: IRNode, x: IRNode): IRNode | undefined {
  let radicand: IRNode | undefined;
  if (f.kind === "apply" && equals(f.head, DIV)) {
    const [numerator, denominator] = binaryArgs(f);
    if (isOne(numerator) && denominator.kind === "apply" && equals(denominator.head, SQRT)) {
      [radicand] = unaryArgs(denominator);
    }
  } else if (f.kind === "apply" && equals(f.head, POW)) {
    const [base, exponent] = binaryArgs(f);
    const n = exactRational(exponent);
    if (n?.numer === -1n && n.denom === 2n) {
      radicand = base;
    }
  }
  if (radicand?.kind !== "apply" || !equals(radicand.head, SUB)) {
    return undefined;
  }

  const [constant, product] = binaryArgs(radicand);
  if (!isOne(constant) || product.kind !== "apply" || !equals(product.head, MUL)) {
    return undefined;
  }
  const [left, right] = binaryArgs(product);
  return modulusFromSquaredFactor(left, right, x) ?? modulusFromSquaredFactor(right, left, x);
}

/**
 * Compute the integer square root of a non-negative BigInt.
 *
 * Returns the root only when ``n`` is a perfect square; returns ``undefined``
 * otherwise.  The implementation uses a floating-point seed followed by two
 * Newton correction steps, which is exact for all BigInts whose magnitude fits
 * in a 64-bit double (i.e. n < 2^53) and safe for the small integers that
 * appear in CAS modulus expressions.
 */
function bigIntIsqrt(n: bigint): bigint | undefined {
  if (n < 0n) return undefined;
  if (n === 0n) return 0n;
  let x = BigInt(Math.round(Math.sqrt(Number(n))));
  // Clamp down if floating-point over-shot
  while (x > 0n && x * x > n) x--;
  // Clamp up if floating-point under-shot
  while ((x + 1n) * (x + 1n) <= n) x++;
  return x * x === n ? x : undefined;
}

/**
 * Extract the elliptic modulus ``k`` from a product factor ``modulusSquare *
 * sineSquare`` inside a ``1 - k²·sin²(x)`` radicand.
 *
 * Handles two cases:
 *
 * 1. **Symbolic form** ``Pow(k, 2)`` — returns the base ``k``.
 * 2. **Pre-evaluated numeric literal** — the compiler may fold ``(1/2)^2``
 *    to ``IRRational(1/4)`` or ``0.5^2`` to ``IRFloat(0.25)`` before the
 *    integration handler runs.  In those cases we compute ``sqrt(k²)``
 *    analytically and return the simplified numeric node:
 *    - ``IRFloat(v)``     → ``IRFloat(Math.sqrt(v))``
 *    - ``IRRational(p/q)`` where both ``p`` and ``q`` are perfect squares
 *                         → ``IRRational(√p / √q)``
 *    - ``IRInteger(n)``   where ``n`` is a perfect square → ``IRInteger(√n)``
 *    - Non-perfect-square rationals/integers → ``Sqrt(k²)`` (unevaluated)
 */
function modulusFromSquaredFactor(modulusSquare: IRNode, sineSquare: IRNode, x: IRNode): IRNode | undefined {
  // Validate sineSquare = Pow(Sin(x), 2) first.
  if (sineSquare.kind !== "apply" || !equals(sineSquare.head, POW)) {
    return undefined;
  }
  const [sine, sineExponent] = binaryArgs(sineSquare);
  if (!equals(sineExponent, int(2)) || sine.kind !== "apply" || !equals(sine.head, SIN)) {
    return undefined;
  }
  const [inner] = unaryArgs(sine);
  if (!equals(inner, x)) {
    return undefined;
  }

  // Case 1: Pow(k, 2) — the symbolic form; return the base k directly.
  if (modulusSquare.kind === "apply" && equals(modulusSquare.head, POW)) {
    const [modulus, modulusExponent] = binaryArgs(modulusSquare);
    return equals(modulusExponent, int(2)) ? modulus : undefined;
  }

  // Case 2: Pre-evaluated numeric literal k² → return √(k²) = k.
  // This handles inputs like (1/2)^2 which the compiler folds to IRRational(1,4)
  // or 0.5^2 which folds to IRFloat(0.25) before the integration handler runs.
  if (modulusSquare.kind === "float") {
    const val = modulusSquare.value;
    if (val < 0) return undefined;
    return numberNode(Math.sqrt(val));
  }
  if (modulusSquare.kind === "integer") {
    const val = modulusSquare.value;
    if (val < 0n) return undefined;
    const root = bigIntIsqrt(val);
    if (root !== undefined) return int(root);
    // Non-perfect-square integer: leave as Sqrt(k²)
    return app(SQRT, [modulusSquare]);
  }
  if (modulusSquare.kind === "rational") {
    const { numer, denom } = modulusSquare;
    if (numer < 0n) return undefined;
    const rootNum = bigIntIsqrt(numer);
    const rootDen = bigIntIsqrt(denom);
    if (rootNum !== undefined && rootDen !== undefined) {
      return rational(rootNum, rootDen);
    }
    // Non-perfect-square rational: leave as Sqrt(k²)
    return app(SQRT, [modulusSquare]);
  }

  return undefined;
}

function isPiOverTwo(node: IRNode): boolean {
  if (node.kind === "float") {
    return Math.abs(node.value - Math.PI / 2) < 1e-12;
  }
  if (node.kind !== "apply" || !equals(node.head, DIV)) {
    return false;
  }
  const [numerator, denominator] = binaryArgs(node);
  return numerator.kind === "symbol" && numerator.name === "%pi" && equals(denominator, int(2));
}

/** Return k when f = Sqrt(1 - k² sin²(x)), else undefined. */
function ellipticSecondKindRadicand(f: IRNode, x: IRNode): IRNode | undefined {
  if (f.kind !== "apply" || !equals(f.head, SQRT)) return undefined;
  const [radicand] = unaryArgs(f);
  if (radicand.kind !== "apply" || !equals(radicand.head, SUB)) return undefined;
  const [constant, product] = binaryArgs(radicand);
  if (!isOne(constant) || product.kind !== "apply" || !equals(product.head, MUL)) return undefined;
  const [left, right] = binaryArgs(product);
  return modulusFromSquaredFactor(left, right, x) ?? modulusFromSquaredFactor(right, left, x);
}

/** ∫₀^(π/2) sqrt(1-k²sin²θ)dθ → EllipticE(k) */
function completeEllipticSecondKind(f: IRNode, x: IRNode, lower: IRNode, upper: IRNode): IRNode | undefined {
  if (!isZero(lower) || !isPiOverTwo(upper)) return undefined;
  const modulus = ellipticSecondKindRadicand(f, x);
  return modulus === undefined ? undefined : app(sym("EllipticE"), [modulus]);
}

/** ∫ sqrt(1-k²sin²θ)dθ → EllipticE(θ, k) */
function incompleteEllipticSecondKind(f: IRNode, x: IRNode): IRNode | undefined {
  const modulus = ellipticSecondKindRadicand(f, x);
  return modulus === undefined ? undefined : app(sym("EllipticE"), [x, modulus]);
}

type PositiveRat = readonly [bigint, bigint];
type SignedRat = readonly [bigint, bigint];

function makeSignedRat(numer: bigint, denom: bigint): SignedRat | undefined {
  if (denom === 0n) return undefined;
  if (denom < 0n) {
    numer = -numer;
    denom = -denom;
  }
  if (numer === 0n) return undefined;
  const gcd = rqGcd(numer < 0n ? -numer : numer, denom);
  return [numer / gcd, denom / gcd];
}

function makePositiveRat(numer: bigint, denom: bigint): PositiveRat | undefined {
  if (denom === 0n) return undefined;
  if (denom < 0n) {
    numer = -numer;
    denom = -denom;
  }
  if (numer <= 0n) return undefined;
  const gcd = rqGcd(numer, denom);
  return [numer / gcd, denom / gcd];
}

function multiplyPositiveRat(a: PositiveRat, b: PositiveRat): PositiveRat {
  return makePositiveRat(a[0] * b[0], a[1] * b[1])!;
}

function dividePositiveRat(a: PositiveRat, b: PositiveRat): PositiveRat {
  return makePositiveRat(a[0] * b[1], a[1] * b[0])!;
}

function positiveRatNode(value: PositiveRat): IRNode {
  return value[1] === 1n ? int(value[0]) : rational(value[0], value[1]);
}

function exactPositiveRat(node: IRNode): PositiveRat | undefined {
  const r = exactRational(node);
  return r === undefined ? undefined : makePositiveRat(r.numer, r.denom);
}

function exactSignedRat(node: IRNode): SignedRat | undefined {
  const r = exactRational(node);
  return r === undefined ? undefined : makeSignedRat(r.numer, r.denom);
}

function multiplySignedRat(a: SignedRat, b: SignedRat): SignedRat {
  return makeSignedRat(a[0] * b[0], a[1] * b[1])!;
}

function divideSignedRat(a: SignedRat, b: SignedRat): SignedRat {
  return makeSignedRat(a[0] * b[1], a[1] * b[0])!;
}

function absSignedRat(value: SignedRat): PositiveRat {
  return makePositiveRat(value[0] < 0n ? -value[0] : value[0], value[1])!;
}

function isSquareOfIntegrationVar(node: IRNode, x: IRNode): boolean {
  if (node.kind !== "apply" || !equals(node.head, POW) || node.args.length !== 2) {
    return false;
  }
  const [base, exponent] = binaryArgs(node);
  return equals(base, x) && equals(exponent, int(2));
}

type FresnelFactors = {
  readonly coeff: PositiveRat;
  readonly hasPi: boolean;
  readonly hasXSquared: boolean;
};

function combineFresnelFactors(a: FresnelFactors, b: FresnelFactors): FresnelFactors | undefined {
  if ((a.hasPi && b.hasPi) || (a.hasXSquared && b.hasXSquared)) {
    return undefined;
  }
  return {
    coeff: multiplyPositiveRat(a.coeff, b.coeff),
    hasPi: a.hasPi || b.hasPi,
    hasXSquared: a.hasXSquared || b.hasXSquared,
  };
}

function scanFresnelFactors(node: IRNode, x: IRNode): FresnelFactors | undefined {
  const one: PositiveRat = [1n, 1n];
  if (isSquareOfIntegrationVar(node, x)) {
    return { coeff: one, hasPi: false, hasXSquared: true };
  }
  if (node.kind === "symbol" && node.name === "%pi") {
    return { coeff: one, hasPi: true, hasXSquared: false };
  }
  const numeric = exactPositiveRat(node);
  if (numeric !== undefined) {
    return { coeff: numeric, hasPi: false, hasXSquared: false };
  }
  if (node.kind !== "apply") {
    return undefined;
  }
  if (equals(node.head, MUL)) {
    let acc: FresnelFactors = { coeff: one, hasPi: false, hasXSquared: false };
    for (const arg of node.args) {
      const scanned = scanFresnelFactors(arg, x);
      if (scanned === undefined) return undefined;
      const combined = combineFresnelFactors(acc, scanned);
      if (combined === undefined) return undefined;
      acc = combined;
    }
    return acc;
  }
  if (equals(node.head, DIV) && node.args.length === 2) {
    const [numerator, denominator] = binaryArgs(node);
    const scanned = scanFresnelFactors(numerator, x);
    const denom = exactPositiveRat(denominator);
    return scanned === undefined || denom === undefined
      ? undefined
      : { ...scanned, coeff: dividePositiveRat(scanned.coeff, denom) };
  }
  return undefined;
}

function fresnelPiQuadraticCoeff(arg: IRNode, x: IRNode): PositiveRat | undefined {
  const factors = scanFresnelFactors(arg, x);
  return factors !== undefined && factors.hasPi && factors.hasXSquared ? factors.coeff : undefined;
}

function fresnelPureQuadraticCoeff(arg: IRNode, x: IRNode): PositiveRat | undefined {
  const factors = scanFresnelFactors(arg, x);
  return factors !== undefined && !factors.hasPi && factors.hasXSquared ? factors.coeff : undefined;
}

function ratTimesInt(value: PositiveRat, factor: bigint): PositiveRat {
  return makePositiveRat(value[0] * factor, value[1])!;
}

type SignedQuadraticFactors = {
  readonly coeff: SignedRat;
  readonly hasXSquared: boolean;
};

function combineSignedQuadraticFactors(
  a: SignedQuadraticFactors,
  b: SignedQuadraticFactors,
): SignedQuadraticFactors | undefined {
  if (a.hasXSquared && b.hasXSquared) return undefined;
  return {
    coeff: multiplySignedRat(a.coeff, b.coeff),
    hasXSquared: a.hasXSquared || b.hasXSquared,
  };
}

function scanSignedQuadraticFactors(node: IRNode, x: IRNode): SignedQuadraticFactors | undefined {
  const one: SignedRat = [1n, 1n];
  if (isSquareOfIntegrationVar(node, x)) {
    return { coeff: one, hasXSquared: true };
  }
  const numeric = exactSignedRat(node);
  if (numeric !== undefined) {
    return { coeff: numeric, hasXSquared: false };
  }
  if (node.kind !== "apply") return undefined;
  if (equals(node.head, NEG) && node.args.length === 1) {
    const scanned = scanSignedQuadraticFactors(node.args[0], x);
    return scanned === undefined
      ? undefined
      : { coeff: makeSignedRat(-scanned.coeff[0], scanned.coeff[1])!, hasXSquared: scanned.hasXSquared };
  }
  if (equals(node.head, MUL)) {
    let acc: SignedQuadraticFactors = { coeff: one, hasXSquared: false };
    for (const arg of node.args) {
      const scanned = scanSignedQuadraticFactors(arg, x);
      if (scanned === undefined) return undefined;
      const combined = combineSignedQuadraticFactors(acc, scanned);
      if (combined === undefined) return undefined;
      acc = combined;
    }
    return acc;
  }
  if (equals(node.head, DIV) && node.args.length === 2) {
    const [numerator, denominator] = binaryArgs(node);
    const scanned = scanSignedQuadraticFactors(numerator, x);
    const denom = exactSignedRat(denominator);
    return scanned === undefined || denom === undefined
      ? undefined
      : { ...scanned, coeff: divideSignedRat(scanned.coeff, denom) };
  }
  return undefined;
}

function signedQuadraticCoeff(arg: IRNode, x: IRNode): SignedRat | undefined {
  const factors = scanSignedQuadraticFactors(arg, x);
  return factors !== undefined && factors.hasXSquared ? factors.coeff : undefined;
}

function sqrtPositiveRatNode(value: PositiveRat): IRNode {
  const rootNumer = bigIntIsqrt(value[0]);
  const rootDenom = bigIntIsqrt(value[1]);
  if (rootNumer !== undefined && rootDenom !== undefined) {
    return positiveRatNode([rootNumer, rootDenom]);
  }
  return app(SQRT, [positiveRatNode(value)]);
}

function isOneRat(value: PositiveRat): boolean {
  return value[0] === value[1];
}

function tryErfIntegral(f: IRNode, x: IRNode): IRNode | undefined {
  if (f.kind !== "apply" || !equals(f.head, EXP) || f.args.length !== 1) {
    return undefined;
  }
  const c = signedQuadraticCoeff(f.args[0], x);
  if (c === undefined) return undefined;

  const absC = absSignedRat(c);
  const alpha = sqrtPositiveRatNode(absC);
  const arg = isOneRat(absC) ? x : app(MUL, [alpha, x]);
  const specialHead = c[0] < 0n ? sym("Erf") : sym("Erfi");
  const sqrtPi = app(SQRT, [_INV_TRIG_PI]);
  const coeff = isOneRat(absC)
    ? app(DIV, [sqrtPi, int(2)])
    : app(DIV, [sqrtPi, app(MUL, [int(2), alpha])]);
  return app(MUL, [coeff, app(specialHead, [arg])]);
}

function tryFresnelIntegral(f: IRNode, x: IRNode): IRNode | undefined {
  if (f.kind !== "apply" || f.args.length !== 1 || (!equals(f.head, SIN) && !equals(f.head, COS))) {
    return undefined;
  }
  const [arg] = unaryArgs(f);
  const fresnelHead = equals(f.head, SIN) ? sym("FresnelS") : sym("FresnelC");

  const q = fresnelPiQuadraticCoeff(arg, x);
  if (q !== undefined) {
    const twoQ = ratTimesInt(q, 2n);
    if (twoQ[0] === twoQ[1]) {
      return app(fresnelHead, [x]);
    }
    const sqrtTwoQ = app(SQRT, [positiveRatNode(twoQ)]);
    const scaleArg = app(MUL, [sqrtTwoQ, x]);
    return app(MUL, [app(DIV, [int(1), sqrtTwoQ]), app(fresnelHead, [scaleArg])]);
  }

  const a = fresnelPureQuadraticCoeff(arg, x);
  if (a !== undefined) {
    const twoA = ratTimesInt(a, 2n);
    const twoANode = positiveRatNode(twoA);
    const sqrtPiOverTwoA = app(SQRT, [app(DIV, [_INV_TRIG_PI, twoANode])]);
    const sqrtTwoAOverPi = app(SQRT, [app(DIV, [twoANode, _INV_TRIG_PI])]);
    return app(MUL, [sqrtPiOverTwoA, app(fresnelHead, [app(MUL, [x, sqrtTwoAOverPi])])]);
  }

  return undefined;
}

/** Return n when bracket = Add(1, Mul(n, Pow(Sin(x), 2))), else undefined. */
function extractCharacteristicN(bracket: IRNode, x: IRNode): IRNode | undefined {
  if (bracket.kind !== "apply" || !equals(bracket.head, ADD)) return undefined;
  const [a, b] = binaryArgs(bracket);
  for (const [onePart, prodPart] of [[a, b], [b, a]] as [IRNode, IRNode][]) {
    if (!isOne(onePart)) continue;
    if (prodPart.kind !== "apply" || !equals(prodPart.head, MUL)) continue;
    const [p1, p2] = binaryArgs(prodPart);
    for (const [nCandidate, sinSq] of [[p1, p2], [p2, p1]] as [IRNode, IRNode][]) {
      if (sinSq.kind !== "apply" || !equals(sinSq.head, POW)) continue;
      const [sine, sineExp] = binaryArgs(sinSq);
      if (!equals(sineExp, int(2))) continue;
      if (sine.kind !== "apply" || !equals(sine.head, SIN)) continue;
      const [inner] = unaryArgs(sine);
      if (!equals(inner, x)) continue;
      if (!dependsOn(nCandidate, x)) return nCandidate;
    }
  }
  return undefined;
}

/** Return {n, k} when f = 1/((1+n·sin²x)·sqrt(1-k²sin²x)), else undefined. */
function ellipticThirdKindParams(f: IRNode, x: IRNode): { n: IRNode; k: IRNode } | undefined {
  if (f.kind !== "apply" || !equals(f.head, DIV)) return undefined;
  const [numerator, denominator] = binaryArgs(f);
  if (!isOne(numerator)) return undefined;
  if (denominator.kind !== "apply" || !equals(denominator.head, MUL)) return undefined;
  const [a, b] = binaryArgs(denominator);
  for (const [bracket, sqrtTerm] of [[a, b], [b, a]] as [IRNode, IRNode][]) {
    if (sqrtTerm.kind !== "apply" || !equals(sqrtTerm.head, SQRT)) continue;
    const [radicand] = unaryArgs(sqrtTerm);
    if (radicand.kind !== "apply" || !equals(radicand.head, SUB)) continue;
    const [constant, product] = binaryArgs(radicand);
    if (!isOne(constant) || product.kind !== "apply" || !equals(product.head, MUL)) continue;
    const [left, right] = binaryArgs(product);
    const k = modulusFromSquaredFactor(left, right, x) ?? modulusFromSquaredFactor(right, left, x);
    if (k === undefined) continue;
    const n = extractCharacteristicN(bracket, x);
    if (n === undefined) continue;
    return { n, k };
  }
  return undefined;
}

/** ∫₀^(π/2) 1/((1+n·sin²θ)·sqrt(1-k²sin²θ))dθ → EllipticPi(n, k) */
function completeEllipticThirdKind(f: IRNode, x: IRNode, lower: IRNode, upper: IRNode): IRNode | undefined {
  if (!isZero(lower) || !isPiOverTwo(upper)) return undefined;
  const params = ellipticThirdKindParams(f, x);
  return params === undefined ? undefined : app(sym("EllipticPi"), [params.n, params.k]);
}

// ---------------------------------------------------------------------------
// Phase 26 — log-power integration via IBP reduction
// ---------------------------------------------------------------------------

/**
 * Build the polynomial coefficient map for ``expr`` as a polynomial in ``x``.
 *
 * Returns a ``Map<degree, coefficient>`` where coefficients are unevaluated
 * ``IRNode`` constant expressions, or ``undefined`` if ``expr`` is not a
 * polynomial in ``x``.  Handles:
 *
 * - constants free of x (degree 0)
 * - x itself (degree 1)
 * - x^k for non-negative integer k
 * - c·f and f·c where c is free of x — scalar-multiply the inner poly
 * - ADD and SUB of two polynomials — merge coefficient maps
 * - NEG of a polynomial — negate all coefficients
 */
function toPolynomialCoeffs(
  expr: IRNode,
  x: IRNode,
): Map<number, IRNode> | undefined {
  if (!dependsOn(expr, x)) {
    return new Map([[0, expr]]);
  }
  if (equals(expr, x)) {
    return new Map([[1, int(1)]]);
  }
  if (expr.kind !== "apply") return undefined;

  if (equals(expr.head, POW)) {
    const [base, exponent] = binaryArgs(expr);
    if (equals(base, x)) {
      const n = exactRational(exponent);
      if (n !== undefined && n.denom === 1n && n.numer >= 0n) {
        return new Map([[Number(n.numer), int(1)]]);
      }
    }
    return undefined;
  }

  if (equals(expr.head, MUL)) {
    const [a, b] = binaryArgs(expr);
    if (!dependsOn(a, x)) {
      const polyB = toPolynomialCoeffs(b, x);
      if (polyB === undefined) return undefined;
      const result = new Map<number, IRNode>();
      for (const [k, v] of polyB) {
        result.set(k, isOne(a) ? v : app(MUL, [a, v]));
      }
      return result;
    }
    if (!dependsOn(b, x)) {
      const polyA = toPolynomialCoeffs(a, x);
      if (polyA === undefined) return undefined;
      const result = new Map<number, IRNode>();
      for (const [k, v] of polyA) {
        result.set(k, isOne(b) ? v : app(MUL, [b, v]));
      }
      return result;
    }
    return undefined;
  }

  if (equals(expr.head, ADD)) {
    const [a, b] = binaryArgs(expr);
    const polyA = toPolynomialCoeffs(a, x);
    const polyB = toPolynomialCoeffs(b, x);
    if (polyA === undefined || polyB === undefined) return undefined;
    const result = new Map<number, IRNode>(polyA);
    for (const [k, v] of polyB) {
      const existing = result.get(k);
      result.set(k, existing !== undefined ? app(ADD, [existing, v]) : v);
    }
    return result;
  }

  if (equals(expr.head, SUB)) {
    const [a, b] = binaryArgs(expr);
    const polyA = toPolynomialCoeffs(a, x);
    const polyB = toPolynomialCoeffs(b, x);
    if (polyA === undefined || polyB === undefined) return undefined;
    const result = new Map<number, IRNode>(polyA);
    for (const [k, v] of polyB) {
      const existing = result.get(k);
      result.set(k, existing !== undefined ? app(SUB, [existing, v]) : app(NEG, [v]));
    }
    return result;
  }

  if (equals(expr.head, NEG)) {
    const [inner] = unaryArgs(expr);
    const polyInner = toPolynomialCoeffs(inner, x);
    if (polyInner === undefined) return undefined;
    const result = new Map<number, IRNode>();
    for (const [k, v] of polyInner) {
      result.set(k, app(NEG, [v]));
    }
    return result;
  }

  return undefined;
}

/**
 * Phase 26: closed form of ``∫ x^k · log(x)^n dx`` for any k ≥ 0, n ≥ 1.
 *
 * Reduction formula (IBP with u = log(x)^n, dv = x^k dx):
 *
 *     ∫ x^k · log(x)^n dx  =  x^(k+1)/(k+1) · log(x)^n
 *                              − n/(k+1) · ∫ x^k · log(x)^(n-1) dx
 *
 * Iterating from the base case G_{k,0}(x) = x^(k+1)/(k+1):
 *
 *     G_{k,0}(x) = x^(k+1)/(k+1)
 *     G_{k,m}(x) = x^(k+1)/(k+1) · log(x)^m  −  m/(k+1) · G_{k,m-1}(x)
 *
 * For k = 0, G_{0,0}(x) = x and the first factor simplifies accordingly.
 */
function polyLogPowerTerm(k: number, n: number, x: IRNode): IRNode {
  const kp1 = k + 1;
  const kp1Frac: Numeric = makeRat(1n, BigInt(kp1));

  // Base: G_{k,0} = x (when k=0) or x^(k+1)/(k+1) (otherwise).
  let acc: IRNode =
    kp1 === 1
      ? x
      : app(MUL, [fromNumeric(kp1Frac), app(POW, [x, int(kp1)])]);

  const logNode = app(LOG, [x]);
  for (let m = 1; m <= n; m++) {
    const logPow: IRNode = m === 1 ? logNode : app(POW, [logNode, int(m)]);
    // first = x^(k+1)/(k+1) · log(x)^m
    const first: IRNode =
      kp1 === 1
        ? app(MUL, [x, logPow])
        : app(MUL, [
            fromNumeric(kp1Frac),
            app(MUL, [app(POW, [x, int(kp1)]), logPow]),
          ]);
    const nCoef = fromNumeric(makeRat(BigInt(m), BigInt(kp1))); // m/(k+1)
    acc = app(SUB, [first, app(MUL, [nCoef, acc])]);
  }
  return acc;
}

/**
 * Phase 26: ``∫ Q(x) · log(x)^n dx`` via term-by-term IBP, for integer n ≥ 2.
 *
 * ``transcendental`` must be ``Pow(Log(x), n)`` with integer n ≥ 2.
 * (n = 1 is covered by the existing Phase 3 log-product handler.)
 * ``polyCandidate`` must be a polynomial in x over Q.
 *
 * Applies linearity: Σᵢ cᵢ · ∫ xⁱ · log(x)^n dx, each term via
 * ``polyLogPowerTerm``.
 */
function tryLogPowerProduct(
  transcendental: IRNode,
  polyCandidate: IRNode,
  x: IRNode,
): IRNode | undefined {
  if (transcendental.kind !== "apply") return undefined;
  if (!equals(transcendental.head, POW) || transcendental.args.length !== 2)
    return undefined;
  const [logNode, expNode] = binaryArgs(transcendental);
  if (logNode.kind !== "apply" || !equals(logNode.head, LOG)) return undefined;
  if (logNode.args.length !== 1 || !equals(logNode.args[0], x)) return undefined;
  const expVal = exactRational(expNode);
  if (expVal === undefined || expVal.denom !== 1n || expVal.numer < 2n)
    return undefined;
  const n = Number(expVal.numer);

  const poly = toPolynomialCoeffs(polyCandidate, x);
  if (poly === undefined || poly.size === 0) return undefined;

  const pieces: IRNode[] = [];
  for (const [k, coef] of poly) {
    if (isZero(coef)) continue;
    const term = polyLogPowerTerm(k, n, x);
    pieces.push(isOne(coef) ? term : app(MUL, [coef, term]));
  }
  if (pieces.length === 0) return int(0);
  return pieces.reduce((acc, p) => app(ADD, [acc, p]));
}

// ---------------------------------------------------------------------------
// Phase 27 — trig(log(x)) integration via u = log(x) substitution
// ---------------------------------------------------------------------------
//
// The substitution u = log(x) (so x = eᵘ, dx = eᵘ du) converts:
//
//   ∫ xᵏ sin(log x) dx = ∫ e^((k+1)u) sin(u) du
//
// The standard exp×trig formula gives:
//   ∫ e^(αu) sin(u) du = e^(αu) (α sin(u) − cos(u)) / (α² + 1)
//
// Setting α = k+1 and back-substituting u = log(x):
//
//   ∫ xᵏ sin(log x) dx = x^(k+1) · ((k+1)sin(log x) − cos(log x)) / ((k+1)² + 1)
//   ∫ xᵏ cos(log x) dx = x^(k+1) · ((k+1)cos(log x) + sin(log x)) / ((k+1)² + 1)
//
// The k=0 forms are:
//   ∫ sin(log x) dx = x/2 · (sin(log x) − cos(log x))
//   ∫ cos(log x) dx = x/2 · (sin(log x) + cos(log x))

/**
 * Phase 27: closed form of ``∫ x^k · trig(log(x)) dx``.
 *
 * ``trigHead`` is the symbol node for ``SIN`` or ``COS``; ``k`` is the
 * integer power of x (k=0 means the integrand is just trig(log(x))).
 *
 *   ∫ xᵏ sin(log x) dx = x^(k+1) · ((k+1)·sin(log x) − cos(log x)) / ((k+1)² + 1)
 *   ∫ xᵏ cos(log x) dx = x^(k+1) · ((k+1)·cos(log x) + sin(log x)) / ((k+1)² + 1)
 */
function trigLogIntegral(trigHead: IRNode, k: number, x: IRNode): IRNode {
  const kp1 = k + 1;
  const denom = kp1 * kp1 + 1;
  const logX = app(LOG, [x]);
  const sinLogX = app(SIN, [logX]);
  const cosLogX = app(COS, [logX]);
  const xPow: IRNode = kp1 === 1 ? x : app(POW, [x, int(kp1)]);
  const kp1Ir = int(kp1);
  const denomIr = int(denom);
  const numerator: IRNode = equals(trigHead, SIN)
    ? app(SUB, [app(MUL, [kp1Ir, sinLogX]), cosLogX])
    : app(ADD, [app(MUL, [kp1Ir, cosLogX]), sinLogX]);
  return app(DIV, [app(MUL, [xPow, numerator]), denomIr]);
}

/**
 * Phase 27: ``∫ Q(x) · sin(log(x)) dx`` or ``∫ Q(x) · cos(log(x)) dx``.
 *
 * ``transcendental`` must be ``Sin(Log(x))`` or ``Cos(Log(x))``.
 * ``polyCandidate`` must be a polynomial in x.
 * Applies linearity: Σᵢ cᵢ · ∫ xⁱ · trig(log(x)) dx via ``trigLogIntegral``.
 */
function tryTrigLogProduct(
  transcendental: IRNode,
  polyCandidate: IRNode,
  x: IRNode,
): IRNode | undefined {
  if (transcendental.kind !== "apply") return undefined;
  if (!equals(transcendental.head, SIN) && !equals(transcendental.head, COS))
    return undefined;
  if (transcendental.args.length !== 1) return undefined;
  const trigArg = transcendental.args[0];
  if (
    trigArg.kind !== "apply" ||
    !equals(trigArg.head, LOG) ||
    trigArg.args.length !== 1 ||
    !equals(trigArg.args[0], x)
  )
    return undefined;

  const poly = toPolynomialCoeffs(polyCandidate, x);
  if (poly === undefined || poly.size === 0) return undefined;

  const trigHead = transcendental.head;
  const pieces: IRNode[] = [];
  for (const [k, coef] of poly) {
    if (isZero(coef)) continue;
    const term = trigLogIntegral(trigHead, k, x);
    pieces.push(isOne(coef) ? term : app(MUL, [coef, term]));
  }
  if (pieces.length === 0) return int(0);
  return pieces.reduce((acc, p) => app(ADD, [acc, p]));
}

function differentiate(): Handler {
  return (vm, expr) => {
    if (expr.args.length !== 2) {
      throw new ArityError(`D expects 2 arguments, got ${expr.args.length}`);
    }
    const [f, x] = expr.args;
    if (x.kind !== "symbol") {
      return expr;
    }
    const result = diff(f, x);
    return isDeferredDerivative(result, f, x) ? result : vm.eval(result);
  };
}

function diff(f: IRNode, x: IRNode): IRNode {
  if (!dependsOn(f, x)) {
    return int(0);
  }
  if (equals(f, x)) {
    return int(1);
  }
  if (f.kind !== "apply") {
    return app(D, [f, x]);
  }

  if (equals(f.head, ADD)) {
    return f.args.map((arg) => diff(arg, x)).reduce((left, right) => app(ADD, [left, right]));
  }
  if (equals(f.head, SUB)) {
    const [a, b] = binaryArgs(f);
    return app(SUB, [diff(a, x), diff(b, x)]);
  }
  if (equals(f.head, NEG)) {
    const [inner] = unaryArgs(f);
    return app(NEG, [diff(inner, x)]);
  }
  if (equals(f.head, MUL)) {
    const [a, b] = binaryArgs(f);
    return app(ADD, [
      app(MUL, [diff(a, x), b]),
      app(MUL, [a, diff(b, x)]),
    ]);
  }
  if (equals(f.head, DIV)) {
    const [a, b] = binaryArgs(f);
    return app(DIV, [
      app(SUB, [
        app(MUL, [diff(a, x), b]),
        app(MUL, [a, diff(b, x)]),
      ]),
      app(POW, [b, int(2)]),
    ]);
  }
  if (equals(f.head, POW)) {
    const [base, exponent] = binaryArgs(f);
    const baseDepends = dependsOn(base, x);
    const exponentDepends = dependsOn(exponent, x);
    if (!exponentDepends) {
      return app(MUL, [
        app(MUL, [
          exponent,
          app(POW, [base, app(SUB, [exponent, int(1)])]),
        ]),
        diff(base, x),
      ]);
    }
    if (!baseDepends) {
      return app(MUL, [
        app(MUL, [f, app(LOG, [base])]),
        diff(exponent, x),
      ]);
    }
    return diff(app(EXP, [app(MUL, [exponent, app(LOG, [base])])]), x);
  }

  const [inner] = f.args.length === 1 ? [f.args[0]] : [undefined];
  if (inner !== undefined) {
    const innerDiff = diff(inner, x);
    if (equals(f.head, SIN)) {
      return app(MUL, [app(COS, [inner]), innerDiff]);
    }
    if (equals(f.head, COS)) {
      return app(MUL, [app(NEG, [app(SIN, [inner])]), innerDiff]);
    }
    if (equals(f.head, TAN)) {
      return app(DIV, [innerDiff, app(POW, [app(COS, [inner]), int(2)])]);
    }
    if (equals(f.head, EXP)) {
      return app(MUL, [app(EXP, [inner]), innerDiff]);
    }
    if (equals(f.head, LOG)) {
      return app(DIV, [innerDiff, inner]);
    }
    if (equals(f.head, SQRT)) {
      return app(DIV, [innerDiff, app(MUL, [int(2), app(SQRT, [inner])])]);
    }
    if (equals(f.head, ASIN)) {
      return app(DIV, [innerDiff, app(SQRT, [app(SUB, [int(1), app(POW, [inner, int(2)])])])]);
    }
    if (equals(f.head, ACOS)) {
      return app(NEG, [
        app(DIV, [innerDiff, app(SQRT, [app(SUB, [int(1), app(POW, [inner, int(2)])])])]),
      ]);
    }
    if (equals(f.head, SINH)) {
      return app(MUL, [app(COSH, [inner]), innerDiff]);
    }
    if (equals(f.head, COSH)) {
      return app(MUL, [app(SINH, [inner]), innerDiff]);
    }
    if (equals(f.head, TANH)) {
      return app(DIV, [innerDiff, app(POW, [app(COSH, [inner]), int(2)])]);
    }
    if (equals(f.head, ASINH)) {
      return app(DIV, [innerDiff, app(SQRT, [app(ADD, [app(POW, [inner, int(2)]), int(1)])])]);
    }
    if (equals(f.head, ACOSH)) {
      return app(DIV, [innerDiff, app(SQRT, [app(SUB, [app(POW, [inner, int(2)]), int(1)])])]);
    }
    if (equals(f.head, ATANH)) {
      return app(DIV, [innerDiff, app(SUB, [int(1), app(POW, [inner, int(2)])])]);
    }
    if (equals(f.head, COTH)) {
      return app(NEG, [
        app(DIV, [
          innerDiff,
          app(POW, [app(SINH, [inner]), int(2)]),
        ]),
      ]);
    }
    if (equals(f.head, SECH)) {
      return app(NEG, [
        app(DIV, [
          app(MUL, [innerDiff, app(SINH, [inner])]),
          app(POW, [app(COSH, [inner]), int(2)]),
        ]),
      ]);
    }
    if (equals(f.head, CSCH)) {
      return app(NEG, [
        app(DIV, [
          app(MUL, [innerDiff, app(COSH, [inner])]),
          app(POW, [app(SINH, [inner]), int(2)]),
        ]),
      ]);
    }
  }

  return app(D, [f, x]);
}

function dependsOn(node: IRNode, variable: IRNode): boolean {
  if (node.kind === "symbol") {
    return equals(node, variable);
  }
  if (node.kind === "apply") {
    return dependsOn(node.head, variable) || node.args.some((arg) => dependsOn(arg, variable));
  }
  return false;
}

function isDeferredIntegral(node: IRNode, f: IRNode, x: IRNode): boolean {
  return node.kind === "apply"
    && equals(node.head, INTEGRATE)
    && node.args.length === 2
    && equals(node.args[0], f)
    && equals(node.args[1], x);
}

function isDeferredDerivative(node: IRNode, f: IRNode, x: IRNode): boolean {
  return node.kind === "apply"
    && equals(node.head, D)
    && node.args.length === 2
    && equals(node.args[0], f)
    && equals(node.args[1], x);
}

function binaryChain(head: IRNode, pieces: readonly IRNode[]): IRNode {
  if (pieces.length === 0) {
    throw new ArityError(`${headName(head) || "<non-symbol-head>"} requires at least one argument`);
  }
  return pieces.slice(1).reduce((left, right) => app(head, [left, right]), pieces[0]);
}

function exactRational(node: IRNode): { readonly numer: bigint; readonly denom: bigint } | undefined {
  if (node.kind === "integer") return { numer: node.value, denom: 1n };
  if (node.kind === "rational") return { numer: node.numer, denom: node.denom };
  return undefined;
}

type Numeric =
  | { readonly kind: "int"; readonly value: bigint }
  | { readonly kind: "rat"; readonly numer: bigint; readonly denom: bigint }
  | { readonly kind: "float"; readonly value: number };

function binaryNumeric(
  name: string,
  simplify: boolean,
  op: (a: Numeric, b: Numeric) => Numeric,
  symbolic: (expr: IRApply, a: IRNode, b: IRNode) => IRNode,
): Handler {
  return (_vm, expr) => {
    const [a, b] = binaryArgs(expr);
    const na = toNumeric(a);
    const nb = toNumeric(b);
    if (na !== undefined && nb !== undefined) {
      return fromNumeric(op(na, nb));
    }
    if (!simplify) {
      throw new TypeError(`${name} requires numeric arguments: ${formatHead(expr)}`);
    }
    return symbolic(expr, a, b);
  };
}

function unaryNumeric(
  name: string,
  simplify: boolean,
  op: (a: Numeric) => Numeric,
  symbolic: (expr: IRApply, a: IRNode) => IRNode,
): Handler {
  return (_vm, expr) => {
    const [a] = unaryArgs(expr);
    const na = toNumeric(a);
    if (na !== undefined) {
      return fromNumeric(op(na));
    }
    if (!simplify) {
      throw new TypeError(`${name} requires a numeric argument: ${formatHead(expr)}`);
    }
    return symbolic(expr, a);
  };
}

function elementary(
  name: string,
  fn: (value: number) => number,
  exact: ReadonlyMap<string, IRNode>,
  simplify: boolean,
): Handler {
  return (_vm, expr) => {
    const [a] = unaryArgs(expr);
    const na = toNumeric(a);
    if (na !== undefined) {
      const exactValue = exact.get(numericKey(na));
      if (exactValue !== undefined) return exactValue;
      return numberNode(fn(toNumber(na)));
    }
    if (!simplify) {
      throw new TypeError(`${name} requires a numeric argument: ${formatHead(expr)}`);
    }
    return expr;
  };
}

function compare(name: string, fn: (a: number, b: number) => boolean, simplify: boolean): Handler {
  return (_vm, expr) => {
    const [a, b] = binaryArgs(expr);
    const na = toNumeric(a);
    const nb = toNumeric(b);
    if (na !== undefined && nb !== undefined) {
      return boolNode(fn(toNumber(na), toNumber(nb)));
    }
    if (!simplify) {
      throw new TypeError(`${name} requires numeric arguments: ${formatHead(expr)}`);
    }
    return expr;
  };
}

function toNumeric(node: IRNode): Numeric | undefined {
  switch (node.kind) {
    case "integer":
      return { kind: "int", value: node.value };
    case "rational":
      return { kind: "rat", numer: node.numer, denom: node.denom };
    case "float":
      return { kind: "float", value: node.value };
    default:
      return undefined;
  }
}

function fromNumeric(value: Numeric): IRNode {
  if (value.kind === "int") return int(value.value);
  if (value.kind === "float") return numberNode(value.value);
  return value.denom === 1n ? int(value.numer) : rational(value.numer, value.denom);
}

function addNumeric(a: Numeric, b: Numeric): Numeric {
  if (a.kind === "float" || b.kind === "float") return floatNumeric(toNumber(a) + toNumber(b));
  const [ar, br] = [asRat(a), asRat(b)];
  return makeRat(ar.numer * br.denom + br.numer * ar.denom, ar.denom * br.denom);
}

function subNumeric(a: Numeric, b: Numeric): Numeric {
  if (a.kind === "float" || b.kind === "float") return floatNumeric(toNumber(a) - toNumber(b));
  const [ar, br] = [asRat(a), asRat(b)];
  return makeRat(ar.numer * br.denom - br.numer * ar.denom, ar.denom * br.denom);
}

function mulNumeric(a: Numeric, b: Numeric): Numeric {
  if (a.kind === "float" || b.kind === "float") return floatNumeric(toNumber(a) * toNumber(b));
  const [ar, br] = [asRat(a), asRat(b)];
  return makeRat(ar.numer * br.numer, ar.denom * br.denom);
}

function divNumeric(a: Numeric, b: Numeric): Numeric {
  if (numericKey(b) === "0") {
    throw new RangeError("division by zero");
  }
  if (a.kind === "float" || b.kind === "float") return floatNumeric(toNumber(a) / toNumber(b));
  const [ar, br] = [asRat(a), asRat(b)];
  return makeRat(ar.numer * br.denom, ar.denom * br.numer);
}

function powNumeric(a: Numeric, b: Numeric): Numeric {
  if (b.kind === "int" && b.value >= 0n && a.kind !== "float") {
    let result: Numeric = { kind: "int", value: 1n };
    for (let i = 0n; i < b.value; i += 1n) {
      result = mulNumeric(result, a);
    }
    return result;
  }
  if (b.kind === "int" && b.value < 0n && a.kind !== "float") {
    const positive = powNumeric(a, { kind: "int", value: -b.value });
    return divNumeric({ kind: "int", value: 1n }, positive);
  }
  return floatNumeric(Math.pow(toNumber(a), toNumber(b)));
}

function negNumeric(a: Numeric): Numeric {
  if (a.kind === "float") return floatNumeric(-a.value);
  if (a.kind === "int") return { kind: "int", value: -a.value };
  return { kind: "rat", numer: -a.numer, denom: a.denom };
}

function invNumeric(a: Numeric): Numeric {
  return divNumeric({ kind: "int", value: 1n }, a);
}

function asRat(value: Numeric): { readonly numer: bigint; readonly denom: bigint } {
  if (value.kind === "int") return { numer: value.value, denom: 1n };
  if (value.kind === "rat") return { numer: value.numer, denom: value.denom };
  throw new TypeError("float is not an exact rational");
}

function makeRat(numer: bigint, denom: bigint): Numeric {
  if (denom === 0n) throw new RangeError("division by zero");
  let n = numer;
  let d = denom;
  if (d < 0n) {
    n = -n;
    d = -d;
  }
  const g = gcd(abs(n), d);
  n /= g;
  d /= g;
  return d === 1n ? { kind: "int", value: n } : { kind: "rat", numer: n, denom: d };
}

function floatNumeric(value: number): Numeric {
  if (!Number.isFinite(value)) {
    throw new RangeError("numeric operation produced a non-finite float");
  }
  return { kind: "float", value };
}

function toNumber(value: Numeric): number {
  if (value.kind === "int") return Number(value.value);
  if (value.kind === "rat") return Number(value.numer) / Number(value.denom);
  return value.value;
}

function numericKey(value: Numeric): string {
  if (value.kind === "int") return value.value.toString();
  if (value.kind === "rat") return value.denom === 1n ? value.numer.toString() : `${value.numer}/${value.denom}`;
  return String(value.value);
}

function binaryArgs(expr: IRApply): readonly [IRNode, IRNode] {
  if (expr.args.length !== 2) {
    throw new ArityError(`${formatHead(expr)} expects 2 arguments, got ${expr.args.length}`);
  }
  return [expr.args[0], expr.args[1]];
}

function unaryArgs(expr: IRApply): readonly [IRNode] {
  if (expr.args.length !== 1) {
    throw new ArityError(`${formatHead(expr)} expects 1 argument, got ${expr.args.length}`);
  }
  return [expr.args[0]];
}

function boolNode(value: boolean): IRNode {
  return value ? TRUE : FALSE;
}

function truthy(node: IRNode): boolean | undefined {
  if (equals(node, TRUE)) return true;
  if (equals(node, FALSE)) return false;
  return undefined;
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 28 — General IBP: ∫ P(x)·log(Q(x)) dx and ∫ P(x)·atan(Q(x)) dx
// ─────────────────────────────────────────────────────────────────────────────
//
// Integration by parts with u = transcendental, dv = P(x) dx:
//
//   ∫ P·log(Q) dx  =  R(x)·log(Q(x))  −  ∫ R(x)·Q′(x)/Q(x) dx
//   ∫ P·atan(Q) dx =  R(x)·atan(Q(x)) −  ∫ R(x)·Q′(x)/(1+Q(x)²) dx
//
// where R = ∫P (polynomial antiderivative, constant term = 0).  The residual
// rational integral is handled by a targeted integrator covering:
//
//   Case A: numerator = c · (denominator′)  →  c · log(denominator)
//   Case B: constant numerator / quadratic ax²+b with rational √(b/a)
//           →  r₀/(a₂·√(a₀/a₂)) · atan(x/√(a₀/a₂))
//
// Examples that close under Phase 28:
//   ∫ log(x²+1) dx    = x·log(x²+1) − 2x + 2·atan(x)
//   ∫ x·log(x²+1) dx  = (x²/2)·log(x²+1) − x²/2 + ½·log(x²+1)
//   ∫ x²·log(x²+1) dx = (x³/3)·log(x²+1) − 2x³/9 + 2x/3 − (2/3)·atan(x)
//   ∫ x·atan(x²) dx   = (x²/2)·atan(x²) − ¼·log(1+x⁴)
//
// Fallthrough cases (returned unevaluated by integrateRationalSimple):
//   ∫ atan(x²) dx      — residual 2x²/(1+x⁴) requires irrational PFDs
//   ∫ x²·atan(x²) dx   — same reason
// ─────────────────────────────────────────────────────────────────────────────

/** A rational coefficient p/q with q > 0 and gcd(|p|,q) = 1 (bigint). */
type RatCoeff = { readonly numer: bigint; readonly denom: bigint };

/** Dense rational-coefficient polynomial: element at index i = coefficient of xⁱ. */
type RatPoly = readonly RatCoeff[];

/** Normalised rational coefficient constructor. */
function rc(numer: bigint, denom: bigint): RatCoeff {
  if (denom === 0n) throw new RangeError("zero denominator in RatCoeff");
  if (numer === 0n) return { numer: 0n, denom: 1n };
  const sign = denom < 0n ? -1n : 1n;
  const n = sign * numer;
  const d = sign * denom;
  const g = gcd(n < 0n ? -n : n, d);
  return { numer: n / g, denom: d / g };
}

const RC_ZERO: RatCoeff = { numer: 0n, denom: 1n };
const RC_ONE: RatCoeff = { numer: 1n, denom: 1n };

function rcIsZero(c: RatCoeff): boolean { return c.numer === 0n; }
function rcIsOne(c: RatCoeff): boolean { return c.numer === c.denom; }
function rcAdd(a: RatCoeff, b: RatCoeff): RatCoeff {
  return rc(a.numer * b.denom + b.numer * a.denom, a.denom * b.denom);
}
function rcSub(a: RatCoeff, b: RatCoeff): RatCoeff {
  return rc(a.numer * b.denom - b.numer * a.denom, a.denom * b.denom);
}
function rcMul(a: RatCoeff, b: RatCoeff): RatCoeff {
  return rc(a.numer * b.numer, a.denom * b.denom);
}
function rcDiv(a: RatCoeff, b: RatCoeff): RatCoeff {
  return rc(a.numer * b.denom, a.denom * b.numer);
}
function rcFromBigInt(n: bigint): RatCoeff { return rc(n, 1n); }
function rcToIR(c: RatCoeff): IRNode {
  return c.denom === 1n ? int(c.numer) : rational(c.numer, c.denom);
}

/** Degree of a RatPoly (−1 for the zero polynomial). */
function rpDeg(p: RatPoly): number {
  for (let i = p.length - 1; i >= 0; i--) {
    if (!rcIsZero(p[i]!)) return i;
  }
  return -1;
}
function rpIsZero(p: RatPoly): boolean { return rpDeg(p) < 0; }

/** Coefficient at degree d (zero when out of bounds). */
function rpCoeff(p: RatPoly, d: number): RatCoeff {
  return d < p.length ? p[d]! : RC_ZERO;
}

function rpAdd(a: RatPoly, b: RatPoly): RatPoly {
  const len = Math.max(a.length, b.length);
  return Array.from({ length: len }, (_, i) => rcAdd(rpCoeff(a, i), rpCoeff(b, i)));
}

function rpScale(p: RatPoly, c: RatCoeff): RatPoly {
  if (rcIsZero(c)) return [];
  return p.map((coef) => rcMul(coef, c));
}

function rpMul(a: RatPoly, b: RatPoly): RatPoly {
  const da = rpDeg(a);
  const db = rpDeg(b);
  if (da < 0 || db < 0) return [];
  const result: RatCoeff[] = Array.from({ length: da + db + 1 }, () => RC_ZERO);
  for (let i = 0; i <= da; i++) {
    if (rcIsZero(rpCoeff(a, i))) continue;
    for (let j = 0; j <= db; j++) {
      result[i + j] = rcAdd(result[i + j]!, rcMul(rpCoeff(a, i), rpCoeff(b, j)));
    }
  }
  return result;
}

/** Horner composition p(a*x+b). */
function rpComposeLinear(p: RatPoly, a: RatCoeff, b: RatCoeff): RatPoly {
  if (rpIsZero(p)) return [];
  const sub: RatPoly = [b, a];
  let result: RatPoly = [rpCoeff(p, rpDeg(p))];
  for (let i = rpDeg(p) - 1; i >= 0; i--) {
    result = rpAdd(rpMul(result, sub), [rpCoeff(p, i)]);
  }
  return result;
}

/** Compose Q((t-b)/a), represented as a t-polynomial. */
function rpComposeToT(Q: RatPoly, a: RatCoeff, b: RatCoeff): RatPoly {
  return rpComposeLinear(Q, rcDiv(RC_ONE, a), rcDiv(rc(0n - b.numer, b.denom), a));
}

/** Formal derivative: d/dx P(x). */
function rpDeriv(p: RatPoly): RatPoly {
  if (p.length <= 1) return [];
  return p.slice(1).map((c, i) => rcMul(c, rcFromBigInt(BigInt(i + 1))));
}

/**
 * Polynomial antiderivative (constant term = 0):
 *   result[0] = 0,  result[k] = p[k−1] / k
 *
 * The integration constant is fixed to zero because IBP cancels it.
 */
function rpIntegrate(p: RatPoly): RatPoly {
  const result: RatCoeff[] = [RC_ZERO];
  for (let i = 0; i < p.length; i++) {
    result.push(rcDiv(p[i] ?? RC_ZERO, rcFromBigInt(BigInt(i + 1))));
  }
  return result;
}

/**
 * Polynomial long division: dividend = quotient × divisor + remainder
 * with deg(remainder) < deg(divisor).
 * Returns ``undefined`` when the divisor is the zero polynomial.
 */
function rpDiv(
  dividend: RatPoly,
  divisor: RatPoly,
): { quotient: RatPoly; remainder: RatPoly } | undefined {
  const dd = rpDeg(divisor);
  if (dd < 0) return undefined;
  const dn = rpDeg(dividend);
  if (dn < dd) return { quotient: [], remainder: [...dividend] };

  const rem: RatCoeff[] = [...dividend];
  const quot: RatCoeff[] = Array.from({ length: dn - dd + 1 }, () => RC_ZERO);
  const leading = rpCoeff(divisor, dd);

  for (let shift = dn - dd; shift >= 0; shift--) {
    const c = rcDiv(rpCoeff(rem as RatPoly, shift + dd), leading);
    quot[shift] = c;
    for (let i = 0; i <= dd; i++) {
      rem[shift + i] = rcSub(rem[shift + i] ?? RC_ZERO, rcMul(c, rpCoeff(divisor, i)));
    }
  }
  return { quotient: quot, remainder: rem as unknown as RatPoly };
}

/** Convert a RatPoly to an IRNode expression (sum of terms, zero coefficients omitted). */
function rpToIR(p: RatPoly, x: IRNode): IRNode {
  const terms: IRNode[] = [];
  for (let i = 0; i < p.length; i++) {
    const c = p[i]!;
    if (rcIsZero(c)) continue;
    if (i === 0) {
      terms.push(rcToIR(c));
    } else {
      const xPow: IRNode = i === 1 ? x : app(POW, [x, int(BigInt(i))]);
      terms.push(rcIsOne(c) ? xPow : app(MUL, [rcToIR(c), xPow]));
    }
  }
  if (terms.length === 0) return int(0);
  return terms.reduce((acc, t) => app(ADD, [acc, t]));
}

/**
 * Recursively evaluate a closed (variable-free) IR expression as an exact rational.
 *
 * ``toPolynomialCoeffs`` may return compound coefficient nodes such as
 * ``MUL(int(2), int(1))`` (from scalar × monomial decomposition).  This
 * evaluator handles such cases so that ``rpFromCoeffsMap`` can extract
 * bigint rational values from them.
 *
 * Only exact rational arithmetic is supported; returns ``undefined`` when any
 * sub-expression involves a float or an unrecognised pattern.
 */
function evalNumericNode(node: IRNode): Numeric | undefined {
  const direct = toNumeric(node);
  if (direct !== undefined) return direct;
  if (node.kind !== "apply") return undefined;
  const { head, args } = node;
  if (equals(head, MUL) && args.length === 2) {
    const a = evalNumericNode(args[0]);
    const b = evalNumericNode(args[1]);
    if (a === undefined || b === undefined) return undefined;
    if (a.kind === "float" || b.kind === "float") return undefined;
    return mulNumeric(a, b);
  }
  if (equals(head, DIV) && args.length === 2) {
    const a = evalNumericNode(args[0]);
    const b = evalNumericNode(args[1]);
    if (a === undefined || b === undefined) return undefined;
    if (a.kind === "float" || b.kind === "float") return undefined;
    return divNumeric(a, b);
  }
  if (equals(head, NEG) && args.length === 1) {
    const a = evalNumericNode(args[0]);
    if (a === undefined || a.kind === "float") return undefined;
    return negNumeric(a);
  }
  if (equals(head, ADD) && args.length === 2) {
    const a = evalNumericNode(args[0]);
    const b = evalNumericNode(args[1]);
    if (a === undefined || b === undefined) return undefined;
    if (a.kind === "float" || b.kind === "float") return undefined;
    return addNumeric(a, b);
  }
  if (equals(head, SUB) && args.length === 2) {
    const a = evalNumericNode(args[0]);
    const b = evalNumericNode(args[1]);
    if (a === undefined || b === undefined) return undefined;
    if (a.kind === "float" || b.kind === "float") return undefined;
    return subNumeric(a, b);
  }
  return undefined;
}

/**
 * Extract exact rational coefficients from a ``toPolynomialCoeffs`` map.
 *
 * Coefficient nodes from ``toPolynomialCoeffs`` may be compound expressions
 * such as ``MUL(int(2), int(1))`` rather than bare numeric literals.
 * ``evalNumericNode`` is used to reduce each coefficient to an exact rational.
 * Returns ``undefined`` when any coefficient cannot be reduced to an exact
 * (non-float) rational.
 */
function rpFromCoeffsMap(map: Map<number, IRNode>): RatPoly | undefined {
  if (map.size === 0) return [];
  const maxDeg = Math.max(...map.keys());
  const result: RatCoeff[] = Array.from({ length: maxDeg + 1 }, () => RC_ZERO);
  for (const [deg, node] of map) {
    const n = evalNumericNode(node);
    if (n === undefined || n.kind === "float") return undefined;
    const r = asRat(n);
    result[deg] = rc(r.numer, r.denom);
  }
  return result;
}

/**
 * Test whether p = c · q for some rational scalar c.
 * Returns c when proportional, ``undefined`` otherwise (including when degrees differ).
 */
function rpProportional(p: RatPoly, q: RatPoly): RatCoeff | undefined {
  const dp = rpDeg(p);
  const dq = rpDeg(q);
  if (dp !== dq) return undefined;
  if (dp < 0) return RC_ZERO; // both zero — degenerate
  let c: RatCoeff | undefined;
  for (let i = 0; i <= Math.max(dp, dq); i++) {
    const pi = rpCoeff(p, i);
    const qi = rpCoeff(q, i);
    if (rcIsZero(pi) && rcIsZero(qi)) continue;
    if (rcIsZero(pi) !== rcIsZero(qi)) return undefined; // mismatch
    const ci = rcDiv(pi, qi);
    if (c === undefined) {
      c = ci;
    } else if (c.numer * ci.denom !== ci.numer * c.denom) {
      return undefined; // inconsistent ratio
    }
  }
  return c;
}

/**
 * Exact integer square root.  Returns ``√n`` if n is a perfect square,
 * ``undefined`` otherwise.
 */
function bigIntSqrt(n: bigint): bigint | undefined {
  if (n < 0n) return undefined;
  if (n === 0n) return 0n;
  // Newton's method converges in O(log log n) iterations.
  let x = n;
  let y = (x + 1n) / 2n;
  while (y < x) {
    x = y;
    y = (x + n / x) / 2n;
  }
  return x * x === n ? x : undefined;
}

/**
 * Exact rational square root of c = p/q.
 * Returns ``√c`` only when both p and q are perfect integer squares.
 */
function rcSqrt(c: RatCoeff): RatCoeff | undefined {
  if (c.numer <= 0n || c.denom <= 0n) return undefined;
  const sqN = bigIntSqrt(c.numer);
  const sqD = bigIntSqrt(c.denom);
  if (sqN === undefined || sqD === undefined) return undefined;
  return rc(sqN, sqD);
}

/**
 * Returns ``true`` if ``expr`` is a polynomial of degree exactly 1 in ``x``.
 *
 * Phase 3 handles log(ax+b) and Phase 11 handles atan(ax+b); we skip linear
 * Q here to avoid duplicating those specialised code paths.
 */
function isLinearIn(expr: IRNode, x: IRNode): boolean {
  const coeffs = toPolynomialCoeffs(expr, x);
  if (coeffs === undefined) return false;
  let maxDeg = 0;
  for (const [deg] of coeffs) {
    if (deg > maxDeg) maxDeg = deg;
  }
  return maxDeg === 1;
}

/**
 * Targeted rational function integrator for Phase 28 IBP residuals.
 *
 * Attempts to integrate N(x)/D(x) where N and D are polynomials with
 * rational coefficients.  After polynomial long division N = Q·D + R:
 *
 *   1. The polynomial quotient Q is integrated term-by-term.
 *   2. For the remainder R (deg R < deg D), two patterns are tried:
 *      Case A: R = c · D′  →  c · log(D)
 *      Case B: R is linear and D = a₂x² + a₁x + a₀ with rational
 *              √(4a₂a₀-a₁²). Split off the D′ log term, then close the
 *              remaining constant-over-quadratic term with atan.
 *
 * Returns ``undefined`` when no pattern matches (caller falls through).
 */
function integrateRationalSimple(
  N_ir: IRNode,
  D_ir: IRNode,
  x: IRNode,
): IRNode | undefined {
  const Nm = toPolynomialCoeffs(N_ir, x);
  const Dm = toPolynomialCoeffs(D_ir, x);
  if (Nm === undefined || Dm === undefined) return undefined;
  const N = rpFromCoeffsMap(Nm);
  const D = rpFromCoeffsMap(Dm);
  if (N === undefined || D === undefined) return undefined;
  if (rpIsZero(D)) return undefined;

  // Polynomial long division: N = Q·D + R
  const divResult = rpDiv(N, D);
  if (divResult === undefined) return undefined;
  const { quotient: Q, remainder: R } = divResult;

  const qAntideriv = rpIntegrate(Q);
  const Dprime = rpDeriv(D);

  // Try to express the remainder R/D in closed form.
  const remResult = closeRemainderOverD(R, D, Dprime, D_ir, x);
  if (remResult === null) {
    // Remainder is zero — only the quotient polynomial contributes.
    return rpIsZero(Q) ? int(0) : rpToIR(qAntideriv, x);
  }
  if (remResult === undefined) return undefined; // cannot close

  if (rpIsZero(Q)) return remResult;
  return app(ADD, [rpToIR(qAntideriv, x), remResult]);
}

/**
 * Attempt to express ∫ R/D dx in closed form (deg R < deg D, rational coeffs).
 *
 * Returns:
 *   - ``null``      — R is the zero polynomial (no contribution)
 *   - ``IRNode``    — closed-form antiderivative
 *   - ``undefined`` — cannot close with available patterns
 */
function closeRemainderOverD(
  R: RatPoly,
  D: RatPoly,
  Dprime: RatPoly,
  D_ir: IRNode,
  x: IRNode,
): IRNode | null | undefined {
  if (rpIsZero(R)) return null;

  // Case A: R = c · D′  →  c · log(D)
  if (!rpIsZero(Dprime)) {
    const c = rpProportional(R, Dprime);
    if (c !== undefined) {
      return rcIsZero(c) ? null : app(MUL, [rcToIR(c), app(LOG, [D_ir])]);
    }
  }

  // Case B: linear remainder over a positive shifted quadratic.
  //
  // Let R = r1*x + r0 and D = a2*x^2 + a1*x + a0.  Split
  // R = c*D' + k, where c = r1/(2*a2) and k = r0 - c*a1. Then:
  //   ∫ R/D dx = c*log(D) + (2k/sqrt(4*a2*a0-a1^2))
  //              * atan((2*a2*x+a1)/sqrt(4*a2*a0-a1^2)).
  const dR = rpDeg(R);
  const dD = rpDeg(D);
  if ((dR === 0 || dR === 1) && dD === 2) {
    const r1 = rpCoeff(R, 1);
    const r0 = rpCoeff(R, 0);
    const a2 = rpCoeff(D, 2);
    const a1 = rpCoeff(D, 1);
    const a0 = rpCoeff(D, 0);
    const two = rcFromBigInt(2n);
    const four = rcFromBigInt(4n);
    if (!rcIsZero(a2) && a2.numer > 0n) {
      const c = rcDiv(r1, rcMul(two, a2));
      const k = rcSub(r0, rcMul(c, a1));
      const delta = rcSub(rcMul(four, rcMul(a2, a0)), rcMul(a1, a1));
      const sqrtDelta = delta.numer > 0n ? rcSqrt(delta) : undefined;
      if (sqrtDelta !== undefined && !rcIsZero(sqrtDelta)) {
        const terms: IRNode[] = [];
        if (!rcIsZero(c)) {
          terms.push(app(MUL, [rcToIR(c), app(LOG, [D_ir])]));
        }
        if (!rcIsZero(k)) {
          const atanCoeff = rcDiv(rcMul(two, k), sqrtDelta);
          const atanNumer = app(ADD, [
            app(MUL, [rcToIR(rcMul(two, a2)), x]),
            rcToIR(a1),
          ]);
          const atanArg = rcIsOne(sqrtDelta) ? atanNumer : app(DIV, [atanNumer, rcToIR(sqrtDelta)]);
          const atanTerm = app(MUL, [rcToIR(atanCoeff), app(ATAN, [atanArg])]);
          terms.push(atanTerm);
        }
        if (terms.length === 0) return null;
        return terms.reduce((acc, term) => app(ADD, [acc, term]));
      }
    }
  }

  return undefined;
}

/**
 * Phase 28: attempt to integrate P(x) · log(Q(x)) for non-linear polynomial Q(x).
 *
 * IBP formula (u = log(Q), dv = P dx):
 *   ∫ P·log(Q) dx  =  R·log(Q) − ∫ R·Q′/Q dx
 *
 * Linear Q is skipped so that Phase 3 handles it instead.
 * Returns ``undefined`` when the pattern does not match or the residual
 * rational integral cannot be closed.
 */
function tryLogPolyProduct(
  transcendental: IRNode,
  polyCandidate: IRNode,
  x: IRNode,
): IRNode | undefined {
  if (transcendental.kind !== "apply") return undefined;
  if (!equals(transcendental.head, LOG)) return undefined;
  if (transcendental.args.length !== 1) return undefined;
  const Q_ir = transcendental.args[0]!;

  if (!dependsOn(Q_ir, x)) return undefined; // log(constant) — handled elsewhere
  if (isLinearIn(Q_ir, x)) return undefined;  // Phase 3 handles log(ax+b)

  // Q must be a polynomial with exact rational coefficients.
  const Qmap = toPolynomialCoeffs(Q_ir, x);
  if (Qmap === undefined) return undefined;
  const Q = rpFromCoeffsMap(Qmap);
  if (Q === undefined || rpIsZero(Q)) return undefined;

  // P must be a polynomial with exact rational coefficients.
  const Pmap = toPolynomialCoeffs(polyCandidate, x);
  if (Pmap === undefined) return undefined;
  const P = rpFromCoeffsMap(Pmap);
  if (P === undefined || rpIsZero(P)) return undefined;

  // R = ∫P (polynomial antiderivative, constant = 0).
  const R = rpIntegrate(P);
  if (rpIsZero(R)) return undefined;
  const R_ir = rpToIR(R, x);

  // Q′ = d/dx Q(x).
  const Qprime = rpDeriv(Q);
  if (rpIsZero(Qprime)) return undefined; // Q is constant — unexpected

  // Residual numerator N = R · Q′.
  const N = rpMul(R, Qprime);
  if (rpIsZero(N)) {
    // Zero residual: ∫ P·log(Q) dx = R·log(Q).
    return app(MUL, [R_ir, transcendental]);
  }
  const N_ir = rpToIR(N, x);

  // Delegate ∫ N/Q dx to the rational integrator.
  const residual = integrateRationalSimple(N_ir, Q_ir, x);
  if (residual === undefined) return undefined;

  // ∫ P·log(Q) dx = R·log(Q) − ∫ R·Q′/Q dx.
  return app(SUB, [app(MUL, [R_ir, transcendental]), residual]);
}

/**
 * Phase 28: attempt to integrate P(x) · atan(Q(x)) for non-linear polynomial Q(x).
 *
 * IBP formula (u = atan(Q), dv = P dx):
 *   ∫ P·atan(Q) dx  =  R·atan(Q) − ∫ R·Q′/(1+Q²) dx
 *
 * Handles both linear and non-linear polynomial Q.  Linear Q covers MACSYMA
 * Phase 11; non-linear Q covers Phase 28.
 */
function tryAtanPolyProduct(
  transcendental: IRNode,
  polyCandidate: IRNode,
  x: IRNode,
): IRNode | undefined {
  if (transcendental.kind !== "apply") return undefined;
  if (!equals(transcendental.head, ATAN)) return undefined;
  if (transcendental.args.length !== 1) return undefined;
  const Q_ir = transcendental.args[0]!;

  if (!dependsOn(Q_ir, x)) return undefined;

  const Qmap = toPolynomialCoeffs(Q_ir, x);
  if (Qmap === undefined) return undefined;
  const Q = rpFromCoeffsMap(Qmap);
  if (Q === undefined || rpIsZero(Q)) return undefined;

  const Pmap = toPolynomialCoeffs(polyCandidate, x);
  if (Pmap === undefined) return undefined;
  const P = rpFromCoeffsMap(Pmap);
  if (P === undefined || rpIsZero(P)) return undefined;

  const R = rpIntegrate(P);
  if (rpIsZero(R)) return undefined;
  const R_ir = rpToIR(R, x);

  const Qprime = rpDeriv(Q);
  if (rpIsZero(Qprime)) return undefined;

  const N = rpMul(R, Qprime);
  if (rpIsZero(N)) {
    return app(MUL, [R_ir, transcendental]);
  }
  const N_ir = rpToIR(N, x);

  // Denominator for the atan residual: 1 + Q(x)².
  // Compute Q² as a polynomial, then prepend the constant 1.
  const Q2 = rpMul(Q, Q);
  const denom: RatPoly = rpAdd(Q2, [RC_ONE]); // 1 + Q²
  const denom_ir = rpToIR(denom, x);

  const residual = integrateRationalSimple(N_ir, denom_ir, x);
  if (residual === undefined) return undefined;

  // ∫ P·atan(Q) dx = R·atan(Q) − ∫ R·Q′/(1+Q²) dx.
  return app(SUB, [app(MUL, [R_ir, transcendental]), residual]);
}

/**
 * Phase 12: integrate P(x) · asin(ax+b) and P(x) · acos(ax+b).
 *
 * Uses the Python reference formula:
 *   asin: [Q(x)-B(ax+b)]·asin(ax+b) - A(ax+b)·sqrt(1-(ax+b)^2)
 *   acos: Q(x)·acos(ax+b) + A(ax+b)·sqrt(1-(ax+b)^2) + B(ax+b)·asin(ax+b)
 * where Q = ∫P dx and ∫Q((t-b)/a)/sqrt(1-t^2) dt = A(t)·sqrt(1-t^2)+B(t)·asin(t).
 */
function tryAsinAcosPolyProduct(
  transcendental: IRNode,
  polyCandidate: IRNode,
  x: IRNode,
): IRNode | undefined {
  if (transcendental.kind !== "apply") return undefined;
  if (!equals(transcendental.head, ASIN) && !equals(transcendental.head, ACOS)) return undefined;
  if (transcendental.args.length !== 1) return undefined;
  const arg = transcendental.args[0]!;
  if (!dependsOn(arg, x)) return undefined;

  const argMap = toPolynomialCoeffs(arg, x);
  if (argMap === undefined) return undefined;
  const argPoly = rpFromCoeffsMap(argMap);
  if (argPoly === undefined || rpDeg(argPoly) !== 1) return undefined;
  const b = rpCoeff(argPoly, 0);
  const a = rpCoeff(argPoly, 1);
  if (rcIsZero(a)) return undefined;

  const Pmap = toPolynomialCoeffs(polyCandidate, x);
  if (Pmap === undefined) return undefined;
  const P = rpFromCoeffsMap(Pmap);
  if (P === undefined || rpIsZero(P)) return undefined;

  const Q = rpIntegrate(P);
  const Q_tilde = rpComposeToT(Q, a, b);
  const [A_t, B_t] = sqrtOneMinusTSquaredDecompose(Q_tilde);
  const A_x = rpComposeLinear(A_t, a, b);
  const B_x = rpComposeLinear(B_t, a, b);

  const Q_ir = rpToIR(Q, x);
  const argSquared = app(POW, [arg, int(2)]);
  const sqrtIr = app(SQRT, [app(SUB, [int(1), argSquared])]);

  if (equals(transcendental.head, ASIN)) {
    const asinCoef = rpIsZero(B_x) ? Q_ir : app(SUB, [Q_ir, rpToIR(B_x, x)]);
    let result: IRNode = app(MUL, [asinCoef, transcendental]);
    if (!rpIsZero(A_x)) {
      result = app(SUB, [result, app(MUL, [rpToIR(A_x, x), sqrtIr])]);
    }
    return result;
  }

  let result: IRNode = app(MUL, [Q_ir, transcendental]);
  if (!rpIsZero(A_x)) {
    result = app(ADD, [result, app(MUL, [rpToIR(A_x, x), sqrtIr])]);
  }
  if (!rpIsZero(B_x)) {
    result = app(ADD, [result, app(MUL, [rpToIR(B_x, x), app(ASIN, [arg])])]);
  }
  return result;
}

function linearArgCoeffs(arg: IRNode, x: IRNode): { readonly a: RatCoeff; readonly b: RatCoeff } | undefined {
  const argMap = toPolynomialCoeffs(arg, x);
  if (argMap === undefined) return undefined;
  const argPoly = rpFromCoeffsMap(argMap);
  if (argPoly === undefined || rpDeg(argPoly) !== 1) return undefined;
  const a = rpCoeff(argPoly, 1);
  if (rcIsZero(a)) return undefined;
  return { a, b: rpCoeff(argPoly, 0) };
}

function hypProductTerm(poly: RatPoly, head: IRNode, arg: IRNode, x: IRNode): IRNode | undefined {
  if (rpIsZero(poly)) return undefined;
  const polyIr = rpToIR(poly, x);
  const hypIr = app(head, [arg]);
  return rpDeg(poly) === 0 && rcIsOne(rpCoeff(poly, 0)) ? hypIr : app(MUL, [polyIr, hypIr]);
}

function addDefinedTerms(terms: readonly (IRNode | undefined)[]): IRNode | undefined {
  const present = terms.filter((term): term is IRNode => term !== undefined);
  if (present.length === 0) return undefined;
  return present.reduce((acc, term) => app(ADD, [acc, term]));
}

/**
 * Phase 13: integrate P(x) * sinh(ax+b) and P(x) * cosh(ax+b).
 *
 * Tabular IBP terminates for polynomial P:
 *   integral P*sinh(u) = P/a*cosh(u) - P'/a^2*sinh(u) + ...
 *   integral P*cosh(u) = P/a*sinh(u) - P'/a^2*cosh(u) + ...
 */
function trySinhCoshPolyProduct(
  transcendental: IRNode,
  polyCandidate: IRNode,
  x: IRNode,
): IRNode | undefined {
  if (transcendental.kind !== "apply") return undefined;
  if (!equals(transcendental.head, SINH) && !equals(transcendental.head, COSH)) return undefined;
  if (transcendental.args.length !== 1) return undefined;
  const arg = transcendental.args[0]!;
  if (!dependsOn(arg, x)) return undefined;
  const linear = linearArgCoeffs(arg, x);
  if (linear === undefined) return undefined;

  const Pmap = toPolynomialCoeffs(polyCandidate, x);
  if (Pmap === undefined) return undefined;
  let derivative = rpFromCoeffsMap(Pmap);
  if (derivative === undefined || rpIsZero(derivative)) return undefined;

  let coshPoly: RatPoly = [];
  let sinhPoly: RatPoly = [];
  let aPower = linear.a;
  let sign = RC_ONE;
  let degree = 0;
  while (!rpIsZero(derivative)) {
    const scale = rcDiv(sign, aPower);
    if (equals(transcendental.head, SINH)) {
      if (degree % 2 === 0) coshPoly = rpAdd(coshPoly, rpScale(derivative, scale));
      else sinhPoly = rpAdd(sinhPoly, rpScale(derivative, scale));
    } else {
      if (degree % 2 === 0) sinhPoly = rpAdd(sinhPoly, rpScale(derivative, scale));
      else coshPoly = rpAdd(coshPoly, rpScale(derivative, scale));
    }
    derivative = rpDeriv(derivative);
    aPower = rcMul(aPower, linear.a);
    sign = rc(0n - sign.numer, sign.denom);
    degree += 1;
  }

  return addDefinedTerms([
    hypProductTerm(coshPoly, COSH, arg, x),
    hypProductTerm(sinhPoly, SINH, arg, x),
  ]);
}

/**
 * Phase 13: integrate P(x) * asinh(ax+b) and P(x) * acosh(ax+b).
 *
 * Mirrors Phase 12's IBP shape, replacing the inverse-trig residual with
 * decompositions over sqrt(t^2 + 1) or sqrt(t^2 - 1).
 */
function tryAsinhAcoshPolyProduct(
  transcendental: IRNode,
  polyCandidate: IRNode,
  x: IRNode,
): IRNode | undefined {
  if (transcendental.kind !== "apply") return undefined;
  if (!equals(transcendental.head, ASINH) && !equals(transcendental.head, ACOSH)) return undefined;
  if (transcendental.args.length !== 1) return undefined;
  const arg = transcendental.args[0]!;
  if (!dependsOn(arg, x)) return undefined;
  const linear = linearArgCoeffs(arg, x);
  if (linear === undefined) return undefined;

  const Pmap = toPolynomialCoeffs(polyCandidate, x);
  if (Pmap === undefined) return undefined;
  const P = rpFromCoeffsMap(Pmap);
  if (P === undefined || rpIsZero(P)) return undefined;

  const Q = rpIntegrate(P);
  const Q_tilde = rpComposeToT(Q, linear.a, linear.b);
  const [A_t, B_t] = equals(transcendental.head, ASINH)
    ? sqrtTPlusOneDecompose(Q_tilde)
    : sqrtTMinusOneDecompose(Q_tilde);
  const A_x = rpComposeLinear(A_t, linear.a, linear.b);
  const B_x = rpComposeLinear(B_t, linear.a, linear.b);

  const Q_ir = rpToIR(Q, x);
  const mainCoef = rpIsZero(B_x) ? Q_ir : app(SUB, [Q_ir, rpToIR(B_x, x)]);
  let result: IRNode = app(MUL, [mainCoef, transcendental]);

  if (!rpIsZero(A_x)) {
    const argSquared = app(POW, [arg, int(2)]);
    const sqrtInner = equals(transcendental.head, ASINH)
      ? app(ADD, [argSquared, int(1)])
      : app(SUB, [argSquared, int(1)]);
    result = app(SUB, [result, app(MUL, [rpToIR(A_x, x), app(SQRT, [sqrtInner])])]);
  }
  return result;
}

function tryTanhAtanhLinear(transcendental: IRNode, x: IRNode): IRNode | undefined {
  if (transcendental.kind !== "apply") return undefined;
  if (!equals(transcendental.head, TANH) && !equals(transcendental.head, ATANH)) return undefined;
  if (transcendental.args.length !== 1) return undefined;
  const arg = transcendental.args[0]!;
  if (!dependsOn(arg, x)) return undefined;
  const linear = linearArgCoeffs(arg, x);
  if (linear === undefined) return undefined;

  const invA = rcDiv(RC_ONE, linear.a);
  if (equals(transcendental.head, TANH)) {
    return app(MUL, [rcToIR(invA), app(LOG, [app(COSH, [arg])])]);
  }

  const argOverA = app(MUL, [rcToIR(invA), arg]);
  const logCoef = rcDiv(RC_ONE, rcMul(rcFromBigInt(2n), linear.a));
  const logArg = app(SUB, [int(1), app(POW, [arg, int(2)])]);
  return app(ADD, [
    app(MUL, [argOverA, transcendental]),
    app(MUL, [rcToIR(logCoef), app(LOG, [logArg])]),
  ]);
}

function tryRecipHypLinear(transcendental: IRNode, x: IRNode): IRNode | undefined {
  if (transcendental.kind !== "apply") return undefined;
  if (!equals(transcendental.head, COTH) && !equals(transcendental.head, SECH) && !equals(transcendental.head, CSCH)) {
    return undefined;
  }
  if (transcendental.args.length !== 1) return undefined;
  const arg = transcendental.args[0]!;
  if (!dependsOn(arg, x)) return undefined;
  const linear = linearArgCoeffs(arg, x);
  if (linear === undefined) return undefined;

  const invA = rcToIR(rcDiv(RC_ONE, linear.a));
  if (equals(transcendental.head, COTH)) {
    return app(MUL, [invA, app(LOG, [app(SINH, [arg])])]);
  }
  if (equals(transcendental.head, SECH)) {
    return app(MUL, [invA, app(ATAN, [app(SINH, [arg])])]);
  }

  const halfArg = app(MUL, [rational(1, 2), arg]);
  return app(MUL, [invA, app(LOG, [app(TANH, [halfArg])])]);
}

function tryRecipHypPower(base: IRNode, exponent: IRNode, x: IRNode): IRNode | undefined {
  if (base.kind !== "apply" || base.args.length !== 1) return undefined;
  if (!equals(base.head, SECH) && !equals(base.head, CSCH) && !equals(base.head, COTH) && !equals(base.head, TANH)) {
    return undefined;
  }
  const nRat = exactRational(exponent);
  if (nRat === undefined || nRat.denom !== 1n || nRat.numer < 0n || nRat.numer > BigInt(Number.MAX_SAFE_INTEGER)) {
    return undefined;
  }
  const arg = base.args[0]!;
  if (!dependsOn(arg, x)) return undefined;
  const linear = linearArgCoeffs(arg, x);
  if (linear === undefined) return undefined;

  const n = Number(nRat.numer);
  if (equals(base.head, SECH)) return sechPowerIntegral(n, arg, linear.a, x);
  if (equals(base.head, CSCH)) return cschPowerIntegral(n, arg, linear.a, x);
  if (equals(base.head, TANH)) return tanhPowerIntegral(n, arg, linear.a, x);
  return cothPowerIntegral(n, arg, linear.a, x);
}

function powIfNeeded(base: IRNode, exponent: number): IRNode {
  return exponent === 1 ? base : app(POW, [base, int(BigInt(exponent))]);
}

function recipHypCoeff(numer: bigint, denom: bigint, a: RatCoeff): IRNode {
  return rcToIR(rcDiv(rc(numer, denom), a));
}

function sechPowerIntegral(n: number, arg: IRNode, a: RatCoeff, x: IRNode): IRNode {
  if (n === 0) return x;
  if (n === 1) return app(MUL, [rcToIR(rcDiv(RC_ONE, a)), app(ATAN, [app(SINH, [arg])])]);
  if (n === 2) return app(MUL, [rcToIR(rcDiv(RC_ONE, a)), app(TANH, [arg])]);

  const sechPow = powIfNeeded(app(SECH, [arg]), n - 2);
  const mainTerm = app(MUL, [
    recipHypCoeff(1n, BigInt(n - 1), a),
    app(MUL, [sechPow, app(TANH, [arg])]),
  ]);
  const tail = sechPowerIntegral(n - 2, arg, a, x);
  return app(ADD, [
    mainTerm,
    app(MUL, [rational(BigInt(n - 2), BigInt(n - 1)), tail]),
  ]);
}

function cschPowerIntegral(n: number, arg: IRNode, a: RatCoeff, x: IRNode): IRNode {
  if (n === 0) return x;
  if (n === 1) {
    const halfArg = app(MUL, [rational(1, 2), arg]);
    return app(MUL, [rcToIR(rcDiv(RC_ONE, a)), app(LOG, [app(TANH, [halfArg])])]);
  }
  if (n === 2) {
    const negInvA = rcToIR(rcDiv(rc(-1n, 1n), a));
    return app(MUL, [negInvA, app(COTH, [arg])]);
  }

  const cschPow = powIfNeeded(app(CSCH, [arg]), n - 2);
  const mainTerm = app(MUL, [
    recipHypCoeff(-1n, BigInt(n - 1), a),
    app(MUL, [cschPow, app(COTH, [arg])]),
  ]);
  const tail = cschPowerIntegral(n - 2, arg, a, x);
  return app(SUB, [
    mainTerm,
    app(MUL, [rational(BigInt(n - 2), BigInt(n - 1)), tail]),
  ]);
}

function cothPowerIntegral(n: number, arg: IRNode, a: RatCoeff, x: IRNode): IRNode {
  if (n === 0) return x;
  if (n === 1) return app(MUL, [rcToIR(rcDiv(RC_ONE, a)), app(LOG, [app(SINH, [arg])])]);

  const cothPow = powIfNeeded(app(COTH, [arg]), n - 1);
  const powerTerm = app(MUL, [recipHypCoeff(1n, BigInt(n - 1), a), cothPow]);
  return app(SUB, [cothPowerIntegral(n - 2, arg, a, x), powerTerm]);
}

function tanhPowerIntegral(n: number, arg: IRNode, a: RatCoeff, x: IRNode): IRNode {
  if (n === 0) return x;
  if (n === 1) return app(MUL, [rcToIR(rcDiv(RC_ONE, a)), app(LOG, [app(COSH, [arg])])]);

  const tanhPow = powIfNeeded(app(TANH, [arg]), n - 1);
  const powerTerm = app(MUL, [recipHypCoeff(1n, BigInt(n - 1), a), tanhPow]);
  return app(SUB, [tanhPowerIntegral(n - 2, arg, a, x), powerTerm]);
}

function sqrtTPlusOneDecompose(Q_tilde: RatPoly): [RatPoly, RatPoly] {
  const memo = new Map<number, [RatPoly, RatPoly]>();
  const monomial = (n: number): [RatPoly, RatPoly] => {
    const cached = memo.get(n);
    if (cached !== undefined) return cached;
    let result: [RatPoly, RatPoly];
    if (n === 0) {
      result = [[], [RC_ONE]];
    } else if (n === 1) {
      result = [[RC_ONE], []];
    } else {
      const aNew: RatCoeff[] = Array.from({ length: n }, () => RC_ZERO);
      aNew[n - 1] = rc(1n, BigInt(n));
      const [aRec, bRec] = monomial(n - 2);
      const coef = rc(0n - BigInt(n - 1), BigInt(n));
      result = [rpAdd(aNew, rpScale(aRec, coef)), rpScale(bRec, coef)];
    }
    memo.set(n, result);
    return result;
  };
  return decomposeByMonomial(Q_tilde, monomial);
}

function sqrtTMinusOneDecompose(Q_tilde: RatPoly): [RatPoly, RatPoly] {
  const memo = new Map<number, [RatPoly, RatPoly]>();
  const monomial = (n: number): [RatPoly, RatPoly] => {
    const cached = memo.get(n);
    if (cached !== undefined) return cached;
    let result: [RatPoly, RatPoly];
    if (n === 0) {
      result = [[], [RC_ONE]];
    } else if (n === 1) {
      result = [[RC_ONE], []];
    } else {
      const aNew: RatCoeff[] = Array.from({ length: n }, () => RC_ZERO);
      aNew[n - 1] = rc(1n, BigInt(n));
      const [aRec, bRec] = monomial(n - 2);
      const coef = rc(BigInt(n - 1), BigInt(n));
      result = [rpAdd(aNew, rpScale(aRec, coef)), rpScale(bRec, coef)];
    }
    memo.set(n, result);
    return result;
  };
  return decomposeByMonomial(Q_tilde, monomial);
}

function decomposeByMonomial(
  Q_tilde: RatPoly,
  monomial: (degree: number) => [RatPoly, RatPoly],
): [RatPoly, RatPoly] {
  let A: RatPoly = [];
  let B: RatPoly = [];
  for (let degree = 0; degree < Q_tilde.length; degree += 1) {
    const coef = rpCoeff(Q_tilde, degree);
    if (rcIsZero(coef)) continue;
    const [aN, bN] = monomial(degree);
    A = rpAdd(A, rpScale(aN, coef));
    B = rpAdd(B, rpScale(bN, coef));
  }
  return [A, B];
}

function sqrtOneMinusTSquaredDecompose(Q_tilde: RatPoly): [RatPoly, RatPoly] {
  const memo = new Map<number, [RatPoly, RatPoly]>();
  const monomial = (n: number): [RatPoly, RatPoly] => {
    const cached = memo.get(n);
    if (cached !== undefined) return cached;
    let result: [RatPoly, RatPoly];
    if (n === 0) {
      result = [[], [RC_ONE]];
    } else if (n === 1) {
      result = [[rcFromBigInt(-1n)], []];
    } else {
      const aNew: RatCoeff[] = Array.from({ length: n }, () => RC_ZERO);
      aNew[n - 1] = rc(-1n, BigInt(n));
      const [aRec, bRec] = monomial(n - 2);
      const coef = rc(BigInt(n - 1), BigInt(n));
      result = [rpAdd(aNew, rpScale(aRec, coef)), rpScale(bRec, coef)];
    }
    memo.set(n, result);
    return result;
  };

  let A: RatPoly = [];
  let B: RatPoly = [];
  for (let degree = 0; degree < Q_tilde.length; degree += 1) {
    const coef = rpCoeff(Q_tilde, degree);
    if (rcIsZero(coef)) continue;
    const [aN, bN] = monomial(degree);
    A = rpAdd(A, rpScale(aN, coef));
    B = rpAdd(B, rpScale(bN, coef));
  }
  return [A, B];
}

function gcd(a: bigint, b: bigint): bigint {
  while (b !== 0n) {
    const t = b;
    b = a % b;
    a = t;
  }
  return a === 0n ? 1n : a;
}

function abs(n: bigint): bigint {
  return n < 0n ? -n : n;
}

function formatHead(expr: IRApply): string {
  return headName(expr.head) || "<non-symbol-head>";
}
