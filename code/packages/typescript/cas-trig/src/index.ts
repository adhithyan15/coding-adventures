import {
  ACOS,
  ADD,
  ASIN,
  ATAN,
  COS,
  MUL,
  NEG,
  POW,
  SIN,
  SQRT,
  SUB,
  TAN,
  app,
  equals,
  headName,
  int,
  numberNode,
  rational,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";

export const PI = "Pi";
export const E = "E";

export type PiMultiple = readonly [num: bigint, den: bigint];

export function toFloat(node: IRNode): number | null {
  switch (node.kind) {
    case "integer":
      return Number(node.value);
    case "rational":
      return Number(node.numer) / Number(node.denom);
    case "float":
      return node.value;
    case "symbol":
      return node.name === PI ? Math.PI : null;
    default:
      return null;
  }
}

export function sinEval(arg: IRNode): IRNode {
  const multiple = extractPiMultiple(arg);
  if (multiple !== null) {
    const value = sinAtPiMultiple(multiple[0], multiple[1]);
    if (value !== null) return value;
  }
  const v = toFloat(arg);
  return v === null ? app(SIN, [arg]) : sinNumeric(v);
}

export function cosEval(arg: IRNode): IRNode {
  const multiple = extractPiMultiple(arg);
  if (multiple !== null) {
    const value = cosAtPiMultiple(multiple[0], multiple[1]);
    if (value !== null) return value;
  }
  const v = toFloat(arg);
  return v === null ? app(COS, [arg]) : cosNumeric(v);
}

export function tanEval(arg: IRNode): IRNode {
  const multiple = extractPiMultiple(arg);
  if (multiple !== null) {
    const value = tanAtPiMultiple(multiple[0], multiple[1]);
    return value ?? app(TAN, [arg]);
  }
  const v = toFloat(arg);
  if (v === null || Math.abs(Math.cos(v)) < 1e-15) return app(TAN, [arg]);
  return snap(Math.sin(v) / Math.cos(v));
}

export function atanEval(arg: IRNode): IRNode {
  const v = toFloat(arg);
  return v === null ? app(ATAN, [arg]) : numberNode(Math.atan(v));
}

export function asinEval(arg: IRNode): IRNode {
  const v = toFloat(arg);
  return v === null || Math.abs(v) > 1 ? app(ASIN, [arg]) : numberNode(Math.asin(v));
}

export function acosEval(arg: IRNode): IRNode {
  const v = toFloat(arg);
  return v === null || Math.abs(v) > 1 ? app(ACOS, [arg]) : numberNode(Math.acos(v));
}

export function trigSimplify(expr: IRNode): IRNode {
  if (expr.kind !== "apply") return expr;
  const simplifiedArgs = expr.args.map(trigSimplify);
  switch (headName(expr.head)) {
    case "Sin":
      return simplifiedArgs.length === 1 ? sinEval(simplifiedArgs[0]) : app(expr.head, simplifiedArgs);
    case "Cos":
      return simplifiedArgs.length === 1 ? cosEval(simplifiedArgs[0]) : app(expr.head, simplifiedArgs);
    case "Tan":
      return simplifiedArgs.length === 1 ? tanEval(simplifiedArgs[0]) : app(expr.head, simplifiedArgs);
    case "Atan":
      return simplifiedArgs.length === 1 ? atanEval(simplifiedArgs[0]) : app(expr.head, simplifiedArgs);
    case "Asin":
      return simplifiedArgs.length === 1 ? asinEval(simplifiedArgs[0]) : app(expr.head, simplifiedArgs);
    case "Acos":
      return simplifiedArgs.length === 1 ? acosEval(simplifiedArgs[0]) : app(expr.head, simplifiedArgs);
    default:
      return app(expr.head, simplifiedArgs);
  }
}

export function extractPiMultiple(arg: IRNode): PiMultiple | null {
  if (arg.kind === "integer" && arg.value === 0n) return [0n, 1n];
  if (isPi(arg)) return [1n, 1n];
  if (arg.kind !== "apply") return null;
  const name = headName(arg.head);
  if (name === "Neg" && arg.args.length === 1) {
    const inner = extractPiMultiple(arg.args[0]);
    return inner === null ? null : [-inner[0], inner[1]];
  }
  if (name === "Mul" && arg.args.length === 2) {
    const [left, right] = arg.args;
    if (isPi(right)) return extractRational(left);
    if (isPi(left)) return extractRational(right);
  }
  return null;
}

export function sinAtPiMultiple(num: bigint, den: bigint): IRNode | null {
  const [k, m] = reducePiFraction(num, den);
  if (k === 0n) return int(0);
  if ((k === 1n && m === 6n) || (k === 5n && m === 6n)) return rational(1, 2);
  if ((k === 7n && m === 6n) || (k === 11n && m === 6n)) return rational(-1, 2);
  if ((k === 1n && m === 4n) || (k === 3n && m === 4n)) return sqrt2Over2();
  if ((k === 5n && m === 4n) || (k === 7n && m === 4n)) return negSurd(sqrt2Over2());
  if ((k === 1n && m === 3n) || (k === 2n && m === 3n)) return sqrt3Over2();
  if ((k === 4n && m === 3n) || (k === 5n && m === 3n)) return negSurd(sqrt3Over2());
  if (k === 1n && m === 2n) return int(1);
  if (k === 3n && m === 2n) return int(-1);
  if (k === 1n && m === 1n) return int(0);
  return null;
}

export function cosAtPiMultiple(num: bigint, den: bigint): IRNode | null {
  const [k, m] = reducePiFraction(num, den);
  if (k === 0n) return int(1);
  if ((k === 1n && m === 6n) || (k === 11n && m === 6n)) return sqrt3Over2();
  if ((k === 5n && m === 6n) || (k === 7n && m === 6n)) return negSurd(sqrt3Over2());
  if ((k === 1n && m === 4n) || (k === 7n && m === 4n)) return sqrt2Over2();
  if ((k === 3n && m === 4n) || (k === 5n && m === 4n)) return negSurd(sqrt2Over2());
  if ((k === 1n && m === 3n) || (k === 5n && m === 3n)) return rational(1, 2);
  if ((k === 2n && m === 3n) || (k === 4n && m === 3n)) return rational(-1, 2);
  if ((k === 1n && m === 2n) || (k === 3n && m === 2n)) return int(0);
  if (k === 1n && m === 1n) return int(-1);
  return null;
}

export function tanAtPiMultiple(num: bigint, den: bigint): IRNode | null {
  const [k, m] = reducePiFraction(num, den);
  if (k === 0n || (k === 1n && m === 1n)) return int(0);
  if ((k === 1n && m === 6n) || (k === 7n && m === 6n)) return invSqrt3();
  if ((k === 5n && m === 6n) || (k === 11n && m === 6n)) return negSurd(invSqrt3());
  if ((k === 1n && m === 4n) || (k === 5n && m === 4n)) return int(1);
  if ((k === 3n && m === 4n) || (k === 7n && m === 4n)) return int(-1);
  if ((k === 1n && m === 3n) || (k === 4n && m === 3n)) return sqrt3();
  if ((k === 2n && m === 3n) || (k === 5n && m === 3n)) return negSurd(sqrt3());
  return null;
}

export function expandTrig(expr: IRNode): IRNode {
  if (expr.kind !== "apply") return expr;
  const name = headName(expr.head);
  if (name === "Sin" && expr.args.length === 1) return expandSin(expr.args[0]);
  if (name === "Cos" && expr.args.length === 1) return expandCos(expr.args[0]);
  return app(expr.head, expr.args.map(expandTrig));
}

export function powerReduce(expr: IRNode): IRNode {
  if (expr.kind !== "apply") return expr;
  if (headName(expr.head) === "Pow" && expr.args.length === 2 && equals(expr.args[1], int(2))) {
    const sinArg = unaryArg(expr.args[0], "Sin");
    if (sinArg !== null) return sinSquared(sinArg);
    const cosArg = unaryArg(expr.args[0], "Cos");
    if (cosArg !== null) return cosSquared(cosArg);
  }
  return app(expr.head, expr.args.map(powerReduce));
}

function sinNumeric(value: number): IRNode {
  return snap(Math.sin(value));
}

function cosNumeric(value: number): IRNode {
  return snap(Math.cos(value));
}

function snap(value: number): IRNode {
  const rounded = Math.round(value);
  if (Math.abs(value - rounded) < 1e-9 && Number.isSafeInteger(rounded)) {
    return int(rounded);
  }
  return numberNode(value);
}

function reducePiFraction(numInput: bigint, denInput: bigint): PiMultiple {
  if (denInput === 0n) throw new RangeError("denominator must not be zero");
  let num = numInput;
  let den = denInput;
  if (den < 0n) {
    num = -num;
    den = -den;
  }
  const modulus = 2n * den;
  const k = ((num % modulus) + modulus) % modulus;
  const g = gcd(abs(k), den);
  return [k / g, den / g];
}

function extractRational(node: IRNode): PiMultiple | null {
  if (node.kind === "integer") return [node.value, 1n];
  if (node.kind === "rational") return [node.numer, node.denom];
  return null;
}

function isPi(node: IRNode): boolean {
  return node.kind === "symbol" && node.name === PI;
}

function negSurd(node: IRNode): IRNode {
  if (node.kind === "integer") return int(-node.value);
  if (node.kind === "rational") return rational(-node.numer, node.denom);
  return app(NEG, [node]);
}

function sqrt2Over2(): IRNode {
  return app(MUL, [rational(1, 2), app(SQRT, [int(2)])]);
}

function sqrt3Over2(): IRNode {
  return app(MUL, [rational(1, 2), app(SQRT, [int(3)])]);
}

function sqrt3(): IRNode {
  return app(SQRT, [int(3)]);
}

function invSqrt3(): IRNode {
  return app(MUL, [rational(1, 3), app(SQRT, [int(3)])]);
}

function expandSin(arg: IRNode): IRNode {
  if (arg.kind === "apply" && headName(arg.head) === "Add" && arg.args.length === 2) {
    const [a, b] = arg.args;
    return add(mul(expandSin(a), expandCos(b)), mul(expandCos(a), expandSin(b)));
  }
  if (arg.kind === "apply" && headName(arg.head) === "Sub" && arg.args.length === 2) {
    const [a, b] = arg.args;
    return sub(mul(expandSin(a), expandCos(b)), mul(expandCos(a), expandSin(b)));
  }
  if (arg.kind === "apply" && headName(arg.head) === "Neg" && arg.args.length === 1) {
    return neg(expandSin(arg.args[0]));
  }
  if (arg.kind === "apply" && headName(arg.head) === "Mul" && arg.args.length === 2) {
    const inner = extractDoubleAngle(arg.args);
    return inner === null ? app(SIN, [expandTrig(arg)]) : mul(int(2), mul(expandSin(inner), expandCos(inner)));
  }
  return app(SIN, [expandTrig(arg)]);
}

function expandCos(arg: IRNode): IRNode {
  if (arg.kind === "apply" && headName(arg.head) === "Add" && arg.args.length === 2) {
    const [a, b] = arg.args;
    return sub(mul(expandCos(a), expandCos(b)), mul(expandSin(a), expandSin(b)));
  }
  if (arg.kind === "apply" && headName(arg.head) === "Sub" && arg.args.length === 2) {
    const [a, b] = arg.args;
    return add(mul(expandCos(a), expandCos(b)), mul(expandSin(a), expandSin(b)));
  }
  if (arg.kind === "apply" && headName(arg.head) === "Neg" && arg.args.length === 1) {
    return expandCos(arg.args[0]);
  }
  if (arg.kind === "apply" && headName(arg.head) === "Mul" && arg.args.length === 2) {
    const inner = extractDoubleAngle(arg.args);
    if (inner === null) return app(COS, [expandTrig(arg)]);
    const c = expandCos(inner);
    const s = expandSin(inner);
    return sub(mul(c, c), mul(s, s));
  }
  return app(COS, [expandTrig(arg)]);
}

function extractDoubleAngle(args: readonly IRNode[]): IRNode | null {
  if (args.length !== 2) return null;
  if (equals(args[0], int(2))) return args[1];
  if (equals(args[1], int(2))) return args[0];
  return null;
}

function sinSquared(inner: IRNode): IRNode {
  const innerReduced = powerReduce(inner);
  return app(MUL, [rational(1, 2), app(SUB, [int(1), app(COS, [app(MUL, [int(2), innerReduced])])])]);
}

function cosSquared(inner: IRNode): IRNode {
  const innerReduced = powerReduce(inner);
  return app(MUL, [rational(1, 2), app(ADD, [int(1), app(COS, [app(MUL, [int(2), innerReduced])])])]);
}

function unaryArg(node: IRNode, name: string): IRNode | null {
  return node.kind === "apply" && headName(node.head) === name && node.args.length === 1 ? node.args[0] : null;
}

function add(a: IRNode, b: IRNode): IRNode {
  return app(ADD, [a, b]);
}

function sub(a: IRNode, b: IRNode): IRNode {
  return app(SUB, [a, b]);
}

function mul(a: IRNode, b: IRNode): IRNode {
  return app(MUL, [a, b]);
}

function neg(a: IRNode): IRNode {
  return app(NEG, [a]);
}

function gcd(aInput: bigint, bInput: bigint): bigint {
  let a = aInput;
  let b = bInput;
  while (b !== 0n) {
    const t = b;
    b = a % b;
    a = t;
  }
  return a === 0n ? 1n : a;
}

function abs(value: bigint): bigint {
  return value < 0n ? -value : value;
}
