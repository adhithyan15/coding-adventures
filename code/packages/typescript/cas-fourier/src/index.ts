import {
  ADD,
  COS,
  DIV,
  EXP,
  MUL,
  NEG,
  POW,
  SIN,
  SQRT,
  SUB,
  app,
  equals,
  int,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";

export const FOURIER = sym("Fourier");
export const IFOURIER = sym("IFourier");
export const DIRAC_DELTA = sym("DiracDelta");
export const UNIT_STEP = sym("UnitStep");
export const IMAGINARY_UNIT = sym("ImaginaryUnit");
export const PI = sym("%pi");

export type EvalFn = (node: IRNode) => IRNode;

export function fourierTransform(f: IRNode, t: IRNode, omega: IRNode): IRNode {
  const add = binaryArgs(f, ADD);
  if (add !== undefined) return bin(ADD, fourierTransform(add[0], t, omega), fourierTransform(add[1], t, omega));

  const extracted = extractCoeffAndFn(f, t);
  if (extracted !== undefined && !isInt(extracted.coeff, 1n)) {
    return bin(MUL, extracted.coeff, fourierTransform(extracted.body, t, omega));
  }

  return forwardLookup(f, t, omega) ?? app(FOURIER, [f, t, omega]);
}

export function ifourierTransform(f: IRNode, omega: IRNode, t: IRNode): IRNode {
  return inverseLookup(f, omega, t) ?? app(IFOURIER, [f, omega, t]);
}

export function fourierHandler(expr: IRNode, evalFn: EvalFn): IRNode {
  const args = applyArgs(expr, FOURIER);
  if (args === undefined || args.length !== 3 || args[1].kind !== "symbol" || args[2].kind !== "symbol") return expr;
  return evalFn(fourierTransform(args[0], args[1], args[2]));
}

export function ifourierHandler(expr: IRNode, evalFn: EvalFn): IRNode {
  const args = applyArgs(expr, IFOURIER);
  if (args === undefined || args.length !== 3 || args[1].kind !== "symbol" || args[2].kind !== "symbol") return expr;
  return evalFn(ifourierTransform(args[0], args[1], args[2]));
}

export function buildFourierHandlerTable(): ReadonlyMap<string, (expr: IRNode, evalFn: EvalFn) => IRNode> {
  return new Map([
    ["Fourier", fourierHandler],
    ["IFourier", ifourierHandler],
  ]);
}

function forwardLookup(f: IRNode, t: IRNode, omega: IRNode): IRNode | undefined {
  if (isApplyOfVar(f, DIRAC_DELTA, t)) return int(1);
  if (isOne(f)) return twoPiDelta(omega);

  const causal = matchCausalExp(f, t);
  if (causal !== undefined) return bin(DIV, int(1), bin(ADD, causal, bin(MUL, IMAGINARY_UNIT, omega)));

  const complex = matchComplexExp(f, t);
  if (complex !== undefined) return twoPiDelta(bin(SUB, omega, complex));

  const sinOmega = matchUnaryLinear(f, SIN, t);
  if (sinOmega !== undefined) {
    return bin(MUL, bin(MUL, IMAGINARY_UNIT, PI), bin(SUB, app(DIRAC_DELTA, [bin(ADD, omega, sinOmega)]), app(DIRAC_DELTA, [bin(SUB, omega, sinOmega)])));
  }

  const cosOmega = matchUnaryLinear(f, COS, t);
  if (cosOmega !== undefined) {
    return bin(MUL, PI, bin(ADD, app(DIRAC_DELTA, [bin(SUB, omega, cosOmega)]), app(DIRAC_DELTA, [bin(ADD, omega, cosOmega)])));
  }

  const gaussian = matchGaussian(f, t);
  if (gaussian !== undefined) {
    return bin(
      MUL,
      app(SQRT, [bin(DIV, PI, gaussian)]),
      app(EXP, [app(NEG, [bin(DIV, bin(POW, omega, int(2)), bin(MUL, int(4), gaussian))])]),
    );
  }

  const tExp = matchTCausalExp(f, t);
  if (tExp !== undefined) {
    const denom = bin(ADD, tExp, bin(MUL, IMAGINARY_UNIT, omega));
    return bin(DIV, int(1), bin(POW, denom, int(2)));
  }

  return undefined;
}

function inverseLookup(f: IRNode, omega: IRNode, t: IRNode): IRNode | undefined {
  if (isInt(f, 1n)) return app(DIRAC_DELTA, [t]);
  if (isApplyOfVar(f, DIRAC_DELTA, omega)) return bin(DIV, int(1), bin(MUL, int(2), PI));

  const deltaArg = matchTwoPiDelta(f);
  if (deltaArg !== undefined) {
    if (equals(deltaArg, omega)) return int(1);
    const shift = matchOmegaMinusA(deltaArg, omega);
    if (shift !== undefined) return app(EXP, [bin(MUL, bin(MUL, IMAGINARY_UNIT, shift), t)]);
  }

  const causal = matchCausalDenom(f, omega, false);
  if (causal !== undefined) return bin(MUL, app(EXP, [app(NEG, [bin(MUL, causal, t)])]), app(UNIT_STEP, [t]));

  const squared = matchCausalDenom(f, omega, true);
  if (squared !== undefined) return bin(MUL, t, bin(MUL, app(EXP, [app(NEG, [bin(MUL, squared, t)])]), app(UNIT_STEP, [t])));

  return undefined;
}

function matchCausalExp(f: IRNode, t: IRNode): IRNode | undefined {
  const args = applyArgs(f, EXP);
  if (args === undefined || args.length !== 1) return undefined;
  const neg = applyArgs(args[0], NEG);
  return neg !== undefined && neg.length === 1 ? extractLinearArg(neg[0], t) : undefined;
}

function matchComplexExp(f: IRNode, t: IRNode): IRNode | undefined {
  const args = applyArgs(f, EXP);
  return args !== undefined && args.length === 1 ? matchIAT(args[0], t) : undefined;
}

function matchIAT(node: IRNode, t: IRNode): IRNode | undefined {
  const mul = binaryArgs(node, MUL);
  if (mul === undefined) return undefined;
  for (const [left, right] of [mul, [mul[1], mul[0]]] as const) {
    if (!equals(right, t)) continue;
    if (equals(left, IMAGINARY_UNIT)) return int(1);
    const inner = binaryArgs(left, MUL);
    if (inner !== undefined) {
      if (equals(inner[0], IMAGINARY_UNIT) && isConstant(inner[1], t)) return inner[1];
      if (equals(inner[1], IMAGINARY_UNIT) && isConstant(inner[0], t)) return inner[0];
    }
  }
  return undefined;
}

function matchGaussian(f: IRNode, t: IRNode): IRNode | undefined {
  const args = applyArgs(f, EXP);
  const neg = args !== undefined && args.length === 1 ? applyArgs(args[0], NEG) : undefined;
  if (neg === undefined || neg.length !== 1) return undefined;
  const inner = neg[0];
  const pow = binaryArgs(inner, POW);
  if (pow !== undefined && equals(pow[0], t) && isInt(pow[1], 2n)) return int(1);
  const mul = binaryArgs(inner, MUL);
  if (mul === undefined) return undefined;
  for (const [coeff, powNode] of [mul, [mul[1], mul[0]]] as const) {
    const p = binaryArgs(powNode, POW);
    if (p !== undefined && equals(p[0], t) && isInt(p[1], 2n) && isConstant(coeff, t)) return coeff;
  }
  return undefined;
}

function matchTCausalExp(f: IRNode, t: IRNode): IRNode | undefined {
  const mul = binaryArgs(f, MUL);
  if (mul === undefined) return undefined;
  for (const [left, right] of [mul, [mul[1], mul[0]]] as const) {
    if (equals(left, t)) {
      const coeff = matchCausalExp(right, t);
      if (coeff !== undefined) return coeff;
    }
  }
  return undefined;
}

function matchCausalDenom(f: IRNode, omega: IRNode, squared: boolean): IRNode | undefined {
  const div = binaryArgs(f, DIV);
  if (div === undefined || !isInt(div[0], 1n)) return undefined;
  let denom = div[1];
  if (squared) {
    const pow = binaryArgs(denom, POW);
    if (pow === undefined || !isInt(pow[1], 2n)) return undefined;
    denom = pow[0];
  }
  const add = binaryArgs(denom, ADD);
  if (add === undefined) return undefined;
  if (isIOmega(add[1], omega)) return add[0];
  if (isIOmega(add[0], omega)) return add[1];
  return undefined;
}

function matchTwoPiDelta(f: IRNode): IRNode | undefined {
  const mul = binaryArgs(f, MUL);
  if (mul === undefined) return undefined;
  for (const [left, right] of [mul, [mul[1], mul[0]]] as const) {
    const delta = applyArgs(right, DIRAC_DELTA);
    if (isTwoPi(left) && delta !== undefined && delta.length === 1) return delta[0];
  }
  return undefined;
}

function matchOmegaMinusA(node: IRNode, omega: IRNode): IRNode | undefined {
  const sub = binaryArgs(node, SUB);
  return sub !== undefined && equals(sub[0], omega) ? sub[1] : undefined;
}

function isIOmega(node: IRNode, omega: IRNode): boolean {
  const mul = binaryArgs(node, MUL);
  return mul !== undefined && ((equals(mul[0], IMAGINARY_UNIT) && equals(mul[1], omega)) || (equals(mul[1], IMAGINARY_UNIT) && equals(mul[0], omega)));
}

function isTwoPi(node: IRNode): boolean {
  const mul = binaryArgs(node, MUL);
  return mul !== undefined && ((isInt(mul[0], 2n) && equals(mul[1], PI)) || (isInt(mul[1], 2n) && equals(mul[0], PI)));
}

function twoPiDelta(arg: IRNode): IRNode {
  return bin(MUL, bin(MUL, int(2), PI), app(DIRAC_DELTA, [arg]));
}

function extractCoeffAndFn(node: IRNode, t: IRNode): { coeff: IRNode; body: IRNode } | undefined {
  const args = binaryArgs(node, MUL);
  if (args === undefined) return undefined;
  if (isConstant(args[0], t)) return { coeff: args[0], body: args[1] };
  if (isConstant(args[1], t)) return { coeff: args[1], body: args[0] };
  return undefined;
}

function matchUnaryLinear(f: IRNode, head: IRNode, t: IRNode): IRNode | undefined {
  const args = applyArgs(f, head);
  return args !== undefined && args.length === 1 ? extractLinearArg(args[0], t) : undefined;
}

function extractLinearArg(arg: IRNode, t: IRNode): IRNode | undefined {
  if (equals(arg, t)) return int(1);
  const mul = binaryArgs(arg, MUL);
  if (mul === undefined) return undefined;
  if (equals(mul[0], t) && isConstant(mul[1], t)) return mul[1];
  if (equals(mul[1], t) && isConstant(mul[0], t)) return mul[0];
  return undefined;
}

function isApplyOfVar(node: IRNode, head: IRNode, variable: IRNode): boolean {
  const args = applyArgs(node, head);
  return args !== undefined && args.length === 1 && equals(args[0], variable);
}

function isConstant(node: IRNode, variable: IRNode): boolean {
  if (equals(node, variable)) return false;
  return node.kind !== "apply" || node.args.every((arg) => isConstant(arg, variable));
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

function isInt(node: IRNode, value: bigint): boolean {
  return node.kind === "integer" && node.value === value;
}

function isOne(node: IRNode): boolean {
  return isInt(node, 1n) || (node.kind === "rational" && node.numer === node.denom);
}
