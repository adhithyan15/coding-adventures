import {
  EXP,
  IRNode,
  LOG,
  MUL,
  POW,
  SQRT,
  app,
  equals,
  headName,
  int,
  rational,
} from "@coding-adventures/symbolic-ir";
import { AssumptionContext } from "./assumptions.js";

interface Fraction {
  readonly numer: bigint;
  readonly denom: bigint;
}

export function radcan(expr: IRNode, ctx?: AssumptionContext): IRNode {
  if (expr.kind !== "apply") return expr;
  const args = expr.args.map((arg) => radcan(arg, ctx));
  return applyRadcanRules(app(expr.head, args), ctx);
}

function applyRadcanRules(expr: IRNode, ctx: AssumptionContext | undefined): IRNode {
  if (expr.kind !== "apply") return expr;
  const name = headName(expr.head);
  if (name === MUL.name) return ruleMul(expr.args, ctx);
  if (name === SQRT.name) return ruleSqrt(expr, ctx);
  if (name === POW.name) return rulePow(expr);
  if (name === EXP.name) return ruleExp(expr);
  if (name === LOG.name) return ruleLog(expr);
  return expr;
}

function ruleMul(argsIn: readonly IRNode[], ctx: AssumptionContext | undefined): IRNode {
  let args = [...argsIn];
  const sqrtRadicands: IRNode[] = [];
  const nonSqrt: IRNode[] = [];

  for (const arg of args) {
    if (isApplyHead(arg, SQRT.name) && arg.args.length === 1) sqrtRadicands.push(arg.args[0]);
    else nonSqrt.push(arg);
  }

  if (sqrtRadicands.length >= 2) {
    const mergedRadicand = mulOrOne(sqrtRadicands);
    args = [...nonSqrt, radcan(app(SQRT, [mergedRadicand]), ctx)];
  }

  const groups = new Map<string, { fraction: Fraction; bases: IRNode[] }>();
  const remaining: IRNode[] = [];
  for (const arg of args) {
    const exp = rationalExponent(arg);
    const base = baseOf(arg);
    if (exp !== undefined && base !== undefined && exp.denom > 1n && !sameFraction(exp, { numer: 1n, denom: 2n })) {
      const key = `${exp.numer}/${exp.denom}`;
      const group = groups.get(key);
      if (group === undefined) groups.set(key, { fraction: exp, bases: [base] });
      else group.bases.push(base);
    } else {
      remaining.push(arg);
    }
  }

  const merged: IRNode[] = [];
  for (const group of groups.values()) {
    if (group.bases.length === 1) remaining.push(app(POW, [group.bases[0], fractionToIr(group.fraction)]));
    else merged.push(app(POW, [mulOrOne(group.bases), fractionToIr(group.fraction)]));
  }

  return mulOrOne([...remaining, ...merged]);
}

function ruleSqrt(expr: IRNode, ctx: AssumptionContext | undefined): IRNode {
  if (!isApplyHead(expr, SQRT.name) || expr.args.length !== 1) return expr;
  const arg = expr.args[0];

  if (arg.kind === "integer" && arg.value >= 0n) {
    const root = integerSqrt(arg.value);
    if (root !== undefined) return int(root);
  }

  if (isSquarePower(arg)) {
    const base = baseOf(arg);
    if (base !== undefined) return absOrPositive(base, ctx);
  }

  if (isApplyHead(arg, MUL.name)) {
    const outer: IRNode[] = [];
    const inner: IRNode[] = [];
    for (const factor of arg.args) {
      const extracted = tryExtractFromSqrt(factor, ctx);
      if (extracted === undefined) inner.push(factor);
      else outer.push(extracted);
    }
    if (outer.length > 0) {
      const outerProduct = mulOrOne(outer);
      const innerProduct = mulOrOne(inner);
      if (equals(innerProduct, int(1))) return outerProduct;
      return app(MUL, [outerProduct, app(SQRT, [innerProduct])]);
    }
  }

  return expr;
}

function tryExtractFromSqrt(factor: IRNode, ctx: AssumptionContext | undefined): IRNode | undefined {
  if (isSquarePower(factor)) {
    const base = baseOf(factor);
    if (base?.kind === "integer" && base.value > 0n) return base;
    if (base?.kind === "symbol" && ctx?.isPositive(base.name) === true) return base;
    return undefined;
  }

  if (factor.kind === "integer" && factor.value > 0n) {
    const root = integerSqrt(factor.value);
    if (root !== undefined) return int(root);
  }

  return undefined;
}

function rulePow(expr: IRNode): IRNode {
  if (!isApplyHead(expr, POW.name) || expr.args.length !== 2) return expr;
  const [base, exp] = expr.args;
  if (isApplyHead(base, SQRT.name) && base.args.length === 1 && equals(exp, int(2))) {
    return base.args[0];
  }
  return expr;
}

function ruleExp(expr: IRNode): IRNode {
  if (!isApplyHead(expr, EXP.name) || expr.args.length !== 1) return expr;
  const arg = expr.args[0];
  if (isApplyHead(arg, LOG.name) && arg.args.length === 1) return arg.args[0];
  return expr;
}

function ruleLog(expr: IRNode): IRNode {
  if (!isApplyHead(expr, LOG.name) || expr.args.length !== 1) return expr;
  const arg = expr.args[0];
  if (isApplyHead(arg, EXP.name) && arg.args.length === 1) return arg.args[0];
  return expr;
}

function isSquarePower(node: IRNode): boolean {
  return isApplyHead(node, POW.name) && node.args.length === 2 && equals(node.args[1], int(2));
}

function absOrPositive(base: IRNode, ctx: AssumptionContext | undefined): IRNode {
  if (base.kind === "integer" && base.value > 0n) return base;
  if (base.kind === "symbol" && ctx?.isPositive(base.name) === true) return base;
  return app(SQRT, [app(POW, [base, int(2)])]);
}

function rationalExponent(node: IRNode): Fraction | undefined {
  if (!isApplyHead(node, POW.name) || node.args.length !== 2) return undefined;
  const exp = node.args[1];
  if (exp.kind === "integer") return normalizeFraction({ numer: exp.value, denom: 1n });
  if (exp.kind === "rational") return normalizeFraction({ numer: exp.numer, denom: exp.denom });
  return undefined;
}

function baseOf(node: IRNode): IRNode | undefined {
  return isApplyHead(node, POW.name) && node.args.length === 2 ? node.args[0] : undefined;
}

function integerSqrt(value: bigint): bigint | undefined {
  if (value < 0n) return undefined;
  if (value < 2n) return value;
  let low = 1n;
  let high = value;
  while (low <= high) {
    const mid = (low + high) >> 1n;
    const square = mid * mid;
    if (square === value) return mid;
    if (square < value) low = mid + 1n;
    else high = mid - 1n;
  }
  return undefined;
}

function fractionToIr(frac: Fraction): IRNode {
  return frac.denom === 1n ? int(frac.numer) : rational(frac.numer, frac.denom);
}

function normalizeFraction(frac: Fraction): Fraction {
  if (frac.denom < 0n) return normalizeFraction({ numer: -frac.numer, denom: -frac.denom });
  const divisor = gcd(abs(frac.numer), frac.denom);
  return { numer: frac.numer / divisor, denom: frac.denom / divisor };
}

function sameFraction(a: Fraction, b: Fraction): boolean {
  return a.numer === b.numer && a.denom === b.denom;
}

function gcd(a: bigint, b: bigint): bigint {
  while (b !== 0n) {
    const next = a % b;
    a = b;
    b = next;
  }
  return a === 0n ? 1n : a;
}

function abs(value: bigint): bigint {
  return value < 0n ? -value : value;
}

function mulOrOne(nodes: readonly IRNode[]): IRNode {
  if (nodes.length === 0) return int(1);
  if (nodes.length === 1) return nodes[0];
  return app(MUL, nodes);
}

function isApplyHead(node: IRNode, name: string): node is Extract<IRNode, { kind: "apply" }> {
  return node.kind === "apply" && headName(node.head) === name;
}
