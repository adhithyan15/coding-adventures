import {
  ADD,
  MUL,
  NEG,
  POW,
  SUB,
  app,
  equals,
  headName,
  int,
  isOne,
  isZero,
  numberNode,
  rational,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";

export const IMAGINARY_UNIT = "I";
export const RE = "Re";
export const IM = "Im";
export const CONJUGATE = "Conjugate";
export const ABS = "Abs";
export const ARG = "Arg";

export const IMAGINARY_UNIT_NODE = sym(IMAGINARY_UNIT);
export const RE_HEAD = sym(RE);
export const IM_HEAD = sym(IM);
export const CONJUGATE_HEAD = sym(CONJUGATE);
export const ABS_HEAD = sym(ABS);
export const ARG_HEAD = sym(ARG);

export type ComplexParts = readonly [real: IRNode, imag: IRNode];

export function complexNormalize(expr: IRNode): IRNode {
  const [re, im] = splitComplex(expr);
  return assemble(re, im);
}

export function splitComplex(expr: IRNode): ComplexParts {
  switch (expr.kind) {
    case "integer":
    case "rational":
    case "float":
      return [expr, int(0)];
    case "symbol":
      return expr.name === IMAGINARY_UNIT ? [int(0), int(1)] : [expr, int(0)];
    case "string":
      return [expr, int(0)];
    case "apply":
      return splitApply(expr);
  }
}

export function realPart(expr: IRNode): IRNode {
  return splitComplex(expr)[0];
}

export function imagPart(expr: IRNode): IRNode {
  return splitComplex(expr)[1];
}

export function conjugate(expr: IRNode): IRNode {
  const [re, im] = splitComplex(expr);
  return assemble(re, negIr(im));
}

export function modulus(expr: IRNode): IRNode {
  const [re, im] = splitComplex(expr);
  const a = toFloat(re);
  const b = toFloat(im);
  if (a === null || b === null) return app(ABS_HEAD, [expr]);
  return numberNode(Math.sqrt(a * a + b * b));
}

export function argument(expr: IRNode): IRNode {
  const [re, im] = splitComplex(expr);
  const a = toFloat(re);
  const b = toFloat(im);
  if (a === null || b === null) return app(ARG_HEAD, [expr]);
  return numberNode(Math.atan2(b, a));
}

export function complexPow(base: IRNode, exp: IRNode): IRNode {
  if (exp.kind !== "integer") return app(POW, [base, exp]);
  const n = exp.value;
  if (n === 0n) return int(1);
  if (n === 1n) return base;

  const [re, im] = splitComplex(base);
  const a = toFloat(re);
  const b = toFloat(im);
  if (a === null || b === null) return app(POW, [base, exp]);

  if (n === -1n) {
    const magSq = a * a + b * b;
    if (magSq === 0) return app(POW, [base, exp]);
    return assembleFloat(a / magSq, -b / magSq);
  }

  if (n < 0n) {
    const magSq = a * a + b * b;
    if (magSq === 0) return app(POW, [base, exp]);
    return positiveIntegerPower(a / magSq, -b / magSq, -n, base, exp);
  }

  return positiveIntegerPower(a, b, n, base, exp);
}

function splitApply(expr: Extract<IRNode, { kind: "apply" }>): ComplexParts {
  const name = headName(expr.head);
  switch (name) {
    case "Add":
      return expr.args.reduce<ComplexParts>(([re, im], arg) => {
        const [ar, ai] = splitComplex(arg);
        return [addIr(re, ar), addIr(im, ai)];
      }, [int(0), int(0)]);
    case "Sub":
      if (expr.args.length !== 2) return [expr, int(0)];
      {
        const [ar, ai] = splitComplex(expr.args[0]);
        const [br, bi] = splitComplex(expr.args[1]);
        return [subIr(ar, br), subIr(ai, bi)];
      }
    case "Neg":
      if (expr.args.length !== 1) return [expr, int(0)];
      {
        const [ar, ai] = splitComplex(expr.args[0]);
        return [negIr(ar), negIr(ai)];
      }
    case "Mul":
      return expr.args.reduce<ComplexParts>(([re, im], arg) => {
        const [ar, ai] = splitComplex(arg);
        const newRe = subIr(mulIr(re, ar), mulIr(im, ai));
        const newIm = addIr(mulIr(re, ai), mulIr(im, ar));
        return [newRe, newIm];
      }, [int(1), int(0)]);
    case "Pow":
      if (expr.args.length === 2 && equals(expr.args[0], IMAGINARY_UNIT_NODE) && expr.args[1].kind === "integer") {
        return iPower(expr.args[1].value);
      }
      return [expr, int(0)];
    default:
      return [expr, int(0)];
  }
}

function iPower(n: bigint): ComplexParts {
  const r = ((n % 4n) + 4n) % 4n;
  if (r === 0n) return [int(1), int(0)];
  if (r === 1n) return [int(0), int(1)];
  if (r === 2n) return [int(-1), int(0)];
  return [int(0), int(-1)];
}

function assemble(re: IRNode, im: IRNode): IRNode {
  const reZero = isZero(re);
  const imZero = isZero(im);
  if (reZero && imZero) return int(0);
  if (imZero) return re;
  if (reZero) return imTerm(im);
  return app(ADD, [re, imTerm(im)]);
}

function imTerm(im: IRNode): IRNode {
  return isOne(im) ? IMAGINARY_UNIT_NODE : app(MUL, [im, IMAGINARY_UNIT_NODE]);
}

function addIr(a: IRNode, b: IRNode): IRNode {
  if (isZero(a)) return b;
  if (isZero(b)) return a;
  if (a.kind === "integer" && b.kind === "integer") return int(a.value + b.value);
  return app(ADD, [a, b]);
}

function subIr(a: IRNode, b: IRNode): IRNode {
  if (isZero(b)) return a;
  if (isZero(a)) return negIr(b);
  if (a.kind === "integer" && b.kind === "integer") return int(a.value - b.value);
  return app(SUB, [a, b]);
}

function negIr(node: IRNode): IRNode {
  if (isZero(node)) return int(0);
  if (node.kind === "integer") return int(-node.value);
  if (node.kind === "float") return numberNode(-node.value);
  if (node.kind === "rational") return rational(-node.numer, node.denom);
  return app(NEG, [node]);
}

function mulIr(a: IRNode, b: IRNode): IRNode {
  if (isZero(a) || isZero(b)) return int(0);
  if (isOne(a)) return b;
  if (isOne(b)) return a;
  if (a.kind === "integer" && b.kind === "integer") return int(a.value * b.value);
  return app(MUL, [a, b]);
}

function toFloat(node: IRNode): number | null {
  switch (node.kind) {
    case "integer":
      return Number(node.value);
    case "rational":
      return Number(node.numer) / Number(node.denom);
    case "float":
      return node.value;
    default:
      return null;
  }
}

function positiveIntegerPower(a: number, b: number, n: bigint, base: IRNode, exp: IRNode): IRNode {
  if (n > BigInt(Number.MAX_SAFE_INTEGER)) return app(POW, [base, exp]);
  const exponent = Number(n);
  const r = Math.sqrt(a * a + b * b);
  const theta = Math.atan2(b, a);
  const rn = Math.pow(r, exponent);
  const angle = theta * exponent;
  return assembleFloat(rn * Math.cos(angle), rn * Math.sin(angle));
}

function assembleFloat(re: number, im: number): IRNode {
  return assemble(snap(re), snap(im));
}

function snap(value: number): IRNode {
  const rounded = Math.round(value);
  if (Math.abs(value - rounded) < 1e-9 && Number.isSafeInteger(rounded)) {
    return int(rounded);
  }
  return numberNode(value);
}
