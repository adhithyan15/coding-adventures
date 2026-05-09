import {
  ADD,
  COS,
  COSH,
  DIV,
  EXP,
  MUL,
  NEG,
  POW,
  SIN,
  SINH,
  SUB,
  app,
  equals,
  int,
  rational,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";

export const LAPLACE = sym("Laplace");
export const ILT = sym("ILT");
export const DIRAC_DELTA = sym("DiracDelta");
export const UNIT_STEP = sym("UnitStep");

export type EvalFn = (node: IRNode) => IRNode;

export function laplaceTransform(f: IRNode, t: IRNode, s: IRNode): IRNode {
  const addArgs = binaryArgs(f, ADD);
  if (addArgs !== undefined) {
    return bin(ADD, laplaceTransform(addArgs[0], t, s), laplaceTransform(addArgs[1], t, s));
  }

  const extracted = extractCoeffAndFn(f, t);
  if (extracted !== undefined && !isInt(extracted.coeff, 1n)) {
    return bin(MUL, extracted.coeff, laplaceTransform(extracted.body, t, s));
  }

  return tableLookup(f, t, s) ?? app(LAPLACE, [f, t, s]);
}

export function inverseLaplace(f: IRNode, s: IRNode, t: IRNode): IRNode {
  return inverseLookup(f, s, t) ?? app(ILT, [f, s, t]);
}

export function laplaceHandler(expr: IRNode, evalFn: EvalFn): IRNode {
  const args = applyArgs(expr, LAPLACE);
  if (args === undefined || args.length !== 3 || args[1].kind !== "symbol" || args[2].kind !== "symbol") return expr;
  return evalFn(laplaceTransform(args[0], args[1], args[2]));
}

export function iltHandler(expr: IRNode, evalFn: EvalFn): IRNode {
  const args = applyArgs(expr, ILT);
  if (args === undefined || args.length !== 3 || args[1].kind !== "symbol" || args[2].kind !== "symbol") return expr;
  return evalFn(inverseLaplace(args[0], args[1], args[2]));
}

export function diracDeltaHandler(expr: IRNode): IRNode {
  const args = applyArgs(expr, DIRAC_DELTA);
  return args !== undefined && args.length === 1 && isInt(args[0], 0n) ? int(1) : expr;
}

export function unitStepHandler(expr: IRNode): IRNode {
  const args = applyArgs(expr, UNIT_STEP);
  if (args === undefined || args.length !== 1 || args[0].kind !== "integer") return expr;
  if (args[0].value < 0n) return int(0);
  if (args[0].value > 0n) return int(1);
  return rational(1, 2);
}

export function buildLaplaceHandlerTable(): ReadonlyMap<string, (expr: IRNode, evalFn: EvalFn) => IRNode> {
  return new Map([
    ["Laplace", laplaceHandler],
    ["ILT", iltHandler],
    ["DiracDelta", (expr) => diracDeltaHandler(expr)],
    ["UnitStep", (expr) => unitStepHandler(expr)],
  ]);
}

function tableLookup(f: IRNode, t: IRNode, s: IRNode): IRNode | undefined {
  if (isOne(f)) return bin(DIV, int(1), s);
  const n = matchPowerOfT(f, t);
  if (n !== undefined) return bin(DIV, int(factorial(n)), bin(POW, s, int(n + 1n)));

  const expShift = matchUnaryLinear(f, EXP, t);
  if (expShift !== undefined) return bin(DIV, int(1), bin(SUB, s, expShift));

  const sinOmega = matchUnaryLinear(f, SIN, t);
  if (sinOmega !== undefined) return bin(DIV, sinOmega, sumSSqParamSq(s, sinOmega));

  const cosOmega = matchUnaryLinear(f, COS, t);
  if (cosOmega !== undefined) return bin(DIV, s, sumSSqParamSq(s, cosOmega));

  const sinhA = matchUnaryLinear(f, SINH, t);
  if (sinhA !== undefined) return bin(DIV, sinhA, subSSqParamSq(s, sinhA));

  const coshA = matchUnaryLinear(f, COSH, t);
  if (coshA !== undefined) return bin(DIV, s, subSSqParamSq(s, coshA));

  if (isApplyOfVar(f, DIRAC_DELTA, t)) return int(1);
  if (isApplyOfVar(f, UNIT_STEP, t)) return bin(DIV, int(1), s);

  const expTrig = matchExpTimesTrig(f, t);
  if (expTrig !== undefined) {
    const shifted = bin(SUB, s, expTrig.shift);
    const denom = bin(ADD, bin(POW, shifted, int(2)), bin(POW, expTrig.omega, int(2)));
    return equals(expTrig.trigHead, SIN) ? bin(DIV, expTrig.omega, denom) : bin(DIV, shifted, denom);
  }

  const tExp = matchTPowerTimesExp(f, t);
  if (tExp !== undefined) {
    return bin(DIV, int(factorial(tExp.power)), bin(POW, bin(SUB, s, tExp.shift), int(tExp.power + 1n)));
  }

  const tTrig = matchTTimesTrig(f, t);
  if (tTrig !== undefined) {
    const denom = bin(POW, sumSSqParamSq(s, tTrig.omega), int(2));
    return equals(tTrig.trigHead, SIN)
      ? bin(DIV, bin(MUL, int(2), bin(MUL, tTrig.omega, s)), denom)
      : bin(DIV, bin(SUB, bin(POW, s, int(2)), bin(POW, tTrig.omega, int(2))), denom);
  }

  return undefined;
}

function inverseLookup(f: IRNode, s: IRNode, t: IRNode): IRNode | undefined {
  const div = binaryArgs(f, DIV);
  if (div === undefined) return undefined;
  const [num, den] = div;

  if (isInt(num, 1n) && equals(den, s)) return app(UNIT_STEP, [t]);
  if (isInt(num, 1n)) {
    const shift = matchSMinusA(den, s);
    if (shift !== undefined) return app(EXP, [bin(MUL, shift, t)]);
    const pow = matchPowOf(den, s);
    if (pow !== undefined && pow >= 2n) {
      const power = pow === 2n ? t : bin(POW, t, int(pow - 1n));
      return pow === 2n ? power : bin(DIV, power, int(factorial(pow - 1n)));
    }
  }

  const plusParam = matchSSqPlusParamSq(den, s);
  if (plusParam !== undefined) {
    if (equals(num, plusParam)) return app(SIN, [bin(MUL, plusParam, t)]);
    if (equals(num, s)) return app(COS, [bin(MUL, plusParam, t)]);
  }

  const minusParam = matchSSqMinusParamSq(den, s);
  if (minusParam !== undefined) {
    if (equals(num, minusParam)) return app(SINH, [bin(MUL, minusParam, t)]);
    if (equals(num, s)) return app(COSH, [bin(MUL, minusParam, t)]);
  }

  return undefined;
}

function matchExpTimesTrig(f: IRNode, t: IRNode): { shift: IRNode; trigHead: IRNode; omega: IRNode } | undefined {
  const args = binaryArgs(f, MUL);
  if (args === undefined) return undefined;
  for (const [expNode, trigNode] of [args, [args[1], args[0]]] as const) {
    const shift = matchUnaryLinear(expNode, EXP, t);
    if (shift === undefined) continue;
    const sinOmega = matchUnaryLinear(trigNode, SIN, t);
    if (sinOmega !== undefined) return { shift, trigHead: SIN, omega: sinOmega };
    const cosOmega = matchUnaryLinear(trigNode, COS, t);
    if (cosOmega !== undefined) return { shift, trigHead: COS, omega: cosOmega };
  }
  return undefined;
}

function matchTPowerTimesExp(f: IRNode, t: IRNode): { power: bigint; shift: IRNode } | undefined {
  const args = binaryArgs(f, MUL);
  if (args === undefined) return undefined;
  for (const [powerNode, expNode] of [args, [args[1], args[0]]] as const) {
    const power = matchPowerOfT(powerNode, t);
    const shift = matchUnaryLinear(expNode, EXP, t);
    if (power !== undefined && shift !== undefined) return { power, shift };
  }
  return undefined;
}

function matchTTimesTrig(f: IRNode, t: IRNode): { trigHead: IRNode; omega: IRNode } | undefined {
  const args = binaryArgs(f, MUL);
  if (args === undefined) return undefined;
  for (const [left, right] of [args, [args[1], args[0]]] as const) {
    if (!equals(left, t)) continue;
    const sinOmega = matchUnaryLinear(right, SIN, t);
    if (sinOmega !== undefined) return { trigHead: SIN, omega: sinOmega };
    const cosOmega = matchUnaryLinear(right, COS, t);
    if (cosOmega !== undefined) return { trigHead: COS, omega: cosOmega };
  }
  return undefined;
}

function matchPowerOfT(f: IRNode, t: IRNode): bigint | undefined {
  if (equals(f, t)) return 1n;
  const pow = binaryArgs(f, POW);
  if (pow !== undefined && equals(pow[0], t) && pow[1].kind === "integer" && pow[1].value >= 1n) return pow[1].value;
  return undefined;
}

function matchUnaryLinear(f: IRNode, head: IRNode, t: IRNode): IRNode | undefined {
  const args = applyArgs(f, head);
  return args !== undefined && args.length === 1 ? extractLinearArg(args[0], t) : undefined;
}

function extractLinearArg(arg: IRNode, t: IRNode): IRNode | undefined {
  if (equals(arg, t)) return int(1);
  const mul = binaryArgs(arg, MUL);
  if (mul !== undefined) {
    if (equals(mul[0], t) && isConstant(mul[1], t)) return mul[1];
    if (equals(mul[1], t) && isConstant(mul[0], t)) return mul[0];
  }
  const neg = applyArgs(arg, NEG);
  if (neg !== undefined && neg.length === 1) {
    const inner = extractLinearArg(neg[0], t);
    if (inner !== undefined) return negate(inner);
  }
  return undefined;
}

function extractCoeffAndFn(node: IRNode, t: IRNode): { coeff: IRNode; body: IRNode } | undefined {
  const args = binaryArgs(node, MUL);
  if (args === undefined) return undefined;
  if (isConstant(args[0], t)) return { coeff: args[0], body: args[1] };
  if (isConstant(args[1], t)) return { coeff: args[1], body: args[0] };
  return undefined;
}

function isConstant(node: IRNode, variable: IRNode): boolean {
  if (equals(node, variable)) return false;
  return node.kind !== "apply" || node.args.every((arg) => isConstant(arg, variable));
}

function matchSMinusA(node: IRNode, s: IRNode): IRNode | undefined {
  const args = binaryArgs(node, SUB);
  return args !== undefined && equals(args[0], s) ? args[1] : undefined;
}

function matchPowOf(node: IRNode, base: IRNode): bigint | undefined {
  const args = binaryArgs(node, POW);
  return args !== undefined && equals(args[0], base) && args[1].kind === "integer" ? args[1].value : undefined;
}

function matchSSqPlusParamSq(node: IRNode, s: IRNode): IRNode | undefined {
  const args = binaryArgs(node, ADD);
  if (args === undefined) return undefined;
  return matchSSqParamSq(args[0], args[1], s) ?? matchSSqParamSq(args[1], args[0], s);
}

function matchSSqMinusParamSq(node: IRNode, s: IRNode): IRNode | undefined {
  const args = binaryArgs(node, SUB);
  return args === undefined ? undefined : matchSSqParamSq(args[0], args[1], s);
}

function matchSSqParamSq(sSq: IRNode, paramSq: IRNode, s: IRNode): IRNode | undefined {
  return matchPowOf(sSq, s) === 2n ? sqrtParam(paramSq) : undefined;
}

function sqrtParam(node: IRNode): IRNode | undefined {
  const pow = binaryArgs(node, POW);
  if (pow !== undefined && isInt(pow[1], 2n)) return pow[0];
  if (node.kind === "integer" && node.value >= 0n) {
    const root = bigintSqrt(node.value);
    if (root * root === node.value) return int(root);
  }
  return undefined;
}

function isApplyOfVar(node: IRNode, head: IRNode, variable: IRNode): boolean {
  const args = applyArgs(node, head);
  return args !== undefined && args.length === 1 && equals(args[0], variable);
}

function sumSSqParamSq(s: IRNode, param: IRNode): IRNode {
  return bin(ADD, bin(POW, s, int(2)), bin(POW, param, int(2)));
}

function subSSqParamSq(s: IRNode, param: IRNode): IRNode {
  return bin(SUB, bin(POW, s, int(2)), bin(POW, param, int(2)));
}

function applyArgs(node: IRNode, head: IRNode): readonly IRNode[] | undefined {
  return node.kind === "apply" && equals(node.head, head) ? node.args : undefined;
}

function binaryArgs(node: IRNode, head: IRNode): readonly [IRNode, IRNode] | undefined {
  const args = applyArgs(node, head);
  return args !== undefined && args.length === 2 ? [args[0], args[1]] : undefined;
}

function bin(head: IRNode, a: IRNode, b: IRNode): IRNode {
  return app(head, [a, b]);
}

function negate(node: IRNode): IRNode {
  return node.kind === "integer" ? int(-node.value) : app(NEG, [node]);
}

function isInt(node: IRNode, value: bigint): boolean {
  return node.kind === "integer" && node.value === value;
}

function isOne(node: IRNode): boolean {
  return isInt(node, 1n) || (node.kind === "rational" && node.numer === node.denom);
}

function factorial(n: bigint): bigint {
  let out = 1n;
  for (let i = 2n; i <= n; i += 1n) out *= i;
  return out;
}

function bigintSqrt(n: bigint): bigint {
  if (n < 2n) return n;
  let lo = 1n;
  let hi = n;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1n;
    const sq = mid * mid;
    if (sq === n) return mid;
    if (sq < n) lo = mid + 1n;
    else hi = mid - 1n;
  }
  return hi;
}
