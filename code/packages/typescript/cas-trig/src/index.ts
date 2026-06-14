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
  return trigReduce(expr);
}

export function trigReduce(expr: IRNode): IRNode {
  let current = expr;
  for (let i = 0; i < 20; i += 1) {
    const next = reduceWalk(current);
    if (equals(next, current)) return next;
    current = next;
  }
  return current;
}

function reduceWalk(expr: IRNode): IRNode {
  if (expr.kind !== "apply") return expr;
  const reducedArgs = expr.args.map(reduceWalk);
  const name = headName(expr.head);

  if (name === "Pow" && reducedArgs.length === 2) {
    const [base, exponent] = reducedArgs;
    if (exponent.kind === "integer" && exponent.value >= 2n && exponent.value <= 6n) {
      const reducedPower = reduceTrigPower(base, Number(exponent.value));
      if (reducedPower !== null) return reducedPower;
    }
  }

  if (name === "Mul") {
    const product = reduceSinCosProduct(reducedArgs);
    if (product !== null) return product;
  }

  return app(expr.head, reducedArgs);
}

function reduceTrigPower(base: IRNode, exponent: number): IRNode | null {
  const sinArg = unaryArg(base, "Sin");
  if (sinArg !== null) return sinPower(sinArg, exponent);
  const cosArg = unaryArg(base, "Cos");
  if (cosArg !== null) return cosPower(cosArg, exponent);
  return null;
}

function sinPower(x: IRNode, exponent: number): IRNode | null {
  switch (exponent) {
    case 2:
      return sinSquared(x);
    case 3:
      return frac(sub(mul(int(3), app(SIN, [x])), sinNx(3, x)), 4);
    case 4:
      return frac(add(sub(int(3), mul(int(4), cosNx(2, x))), cosNx(4, x)), 8);
    case 5:
      return frac(add(sub(mul(int(10), app(SIN, [x])), mul(int(5), sinNx(3, x))), sinNx(5, x)), 16);
    case 6:
      return frac(sub(add(sub(int(10), mul(int(15), cosNx(2, x))), mul(int(6), cosNx(4, x))), cosNx(6, x)), 32);
    default:
      return null;
  }
}

function cosPower(x: IRNode, exponent: number): IRNode | null {
  switch (exponent) {
    case 2:
      return cosSquared(x);
    case 3:
      return frac(add(mul(int(3), app(COS, [x])), cosNx(3, x)), 4);
    case 4:
      return frac(add(add(int(3), mul(int(4), cosNx(2, x))), cosNx(4, x)), 8);
    case 5:
      return frac(add(add(mul(int(10), app(COS, [x])), mul(int(5), cosNx(3, x))), cosNx(5, x)), 16);
    case 6:
      return frac(add(add(add(int(10), mul(int(15), cosNx(2, x))), mul(int(6), cosNx(4, x))), cosNx(6, x)), 32);
    default:
      return null;
  }
}

function reduceSinCosProduct(args: readonly IRNode[]): IRNode | null {
  if (args.length < 2) return null;

  let sinArg: IRNode | null = null;
  let cosArg: IRNode | null = null;
  const other: IRNode[] = [];

  for (const arg of args) {
    if (sinArg === null) {
      const currentSinArg = unaryArg(arg, "Sin");
      if (currentSinArg !== null) {
        sinArg = currentSinArg;
        continue;
      }
    }
    if (cosArg === null) {
      const currentCosArg = unaryArg(arg, "Cos");
      if (currentCosArg !== null) {
        cosArg = currentCosArg;
        continue;
      }
    }
    other.push(arg);
  }

  if (sinArg === null || cosArg === null || !equals(sinArg, cosArg)) return null;

  const halfSinDoubleAngle = frac(sinNx(2, sinArg), 2);
  return other.length === 0 ? halfSinDoubleAngle : app(MUL, [...other, halfSinDoubleAngle]);
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
  return frac(app(SUB, [int(1), cosNx(2, inner)]), 2);
}

function cosSquared(inner: IRNode): IRNode {
  return frac(app(ADD, [int(1), cosNx(2, inner)]), 2);
}

function unaryArg(node: IRNode, name: string): IRNode | null {
  return node.kind === "apply" && headName(node.head) === name && node.args.length === 1 ? node.args[0] : null;
}

function sinNx(n: number, x: IRNode): IRNode {
  return app(SIN, [mul(int(n), x)]);
}

function cosNx(n: number, x: IRNode): IRNode {
  return app(COS, [mul(int(n), x)]);
}

function frac(numerator: IRNode, denominator: number): IRNode {
  return app(MUL, [rational(1, denominator), numerator]);
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
