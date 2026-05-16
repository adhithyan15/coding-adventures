/**
 * Pure TypeScript symbolic IR.
 *
 * The shape intentionally mirrors the Python and Rust symbolic-ir packages:
 * every expression is one of six immutable node variants, and every compound
 * expression uses the single Apply(head, args) form.
 */

export type IRNode =
  | IRSymbol
  | IRInteger
  | IRRational
  | IRFloat
  | IRString
  | IRApply;

export interface IRSymbol {
  readonly kind: "symbol";
  readonly name: string;
}

export interface IRInteger {
  readonly kind: "integer";
  readonly value: bigint;
}

export interface IRRational {
  readonly kind: "rational";
  readonly numer: bigint;
  readonly denom: bigint;
}

export interface IRFloat {
  readonly kind: "float";
  readonly value: number;
}

export interface IRString {
  readonly kind: "string";
  readonly value: string;
}

export interface IRApply {
  readonly kind: "apply";
  readonly head: IRNode;
  readonly args: readonly IRNode[];
}

export type IntegerInput = bigint | number | string;

export function sym(name: string): IRSymbol {
  return Object.freeze({ kind: "symbol", name });
}

export function int(value: IntegerInput): IRInteger {
  return Object.freeze({ kind: "integer", value: toBigInt(value) });
}

export function rational(numer: IntegerInput, denom: IntegerInput): IRRational {
  let n = toBigInt(numer);
  let d = toBigInt(denom);
  if (d === 0n) {
    throw new RangeError("IRRational denominator cannot be zero");
  }
  if (d < 0n) {
    n = -n;
    d = -d;
  }
  const g = gcd(abs(n), d);
  return Object.freeze({ kind: "rational", numer: n / g, denom: d / g });
}

export function numberNode(value: number): IRFloat {
  if (!Number.isFinite(value)) {
    throw new RangeError("IRFloat value must be finite");
  }
  return Object.freeze({ kind: "float", value });
}

export function stringNode(value: string): IRString {
  return Object.freeze({ kind: "string", value });
}

export function app(head: IRNode, args: readonly IRNode[]): IRApply {
  return Object.freeze({
    kind: "apply",
    head,
    args: Object.freeze([...args]),
  });
}

export function isNumeric(node: IRNode): node is IRInteger | IRRational | IRFloat {
  return node.kind === "integer" || node.kind === "rational" || node.kind === "float";
}

export function isZero(node: IRNode): boolean {
  if (node.kind === "integer") return node.value === 0n;
  if (node.kind === "rational") return node.numer === 0n;
  if (node.kind === "float") return node.value === 0;
  return false;
}

export function isOne(node: IRNode): boolean {
  if (node.kind === "integer") return node.value === 1n;
  if (node.kind === "rational") return node.numer === node.denom;
  if (node.kind === "float") return node.value === 1;
  return false;
}

export function equals(a: IRNode, b: IRNode): boolean {
  if (a.kind !== b.kind) return false;
  switch (a.kind) {
    case "symbol":
      return a.name === (b as IRSymbol).name;
    case "integer":
      return a.value === (b as IRInteger).value;
    case "rational": {
      const rb = b as IRRational;
      return a.numer === rb.numer && a.denom === rb.denom;
    }
    case "float":
      return Object.is(a.value, (b as IRFloat).value);
    case "string":
      return a.value === (b as IRString).value;
    case "apply": {
      const ab = b as IRApply;
      return equals(a.head, ab.head)
        && a.args.length === ab.args.length
        && a.args.every((arg, i) => equals(arg, ab.args[i]));
    }
  }
}

export function structuralKey(node: IRNode): string {
  switch (node.kind) {
    case "symbol":
      return `S:${escapeKey(node.name)}`;
    case "integer":
      return `I:${node.value}`;
    case "rational":
      return `Q:${node.numer}/${node.denom}`;
    case "float":
      return `F:${Object.is(node.value, -0) ? "-0" : String(node.value)}`;
    case "string":
      return `T:${escapeKey(node.value)}`;
    case "apply":
      return `A:${structuralKey(node.head)}(${node.args.map(structuralKey).join(",")})`;
  }
}

export function toDisplayString(node: IRNode): string {
  switch (node.kind) {
    case "symbol":
      return node.name;
    case "integer":
      return node.value.toString();
    case "rational":
      return `${node.numer}/${node.denom}`;
    case "float":
      return String(node.value);
    case "string":
      return JSON.stringify(node.value);
    case "apply":
      return `${toDisplayString(node.head)}(${node.args.map(toDisplayString).join(", ")})`;
  }
}

export function headName(node: IRNode): string {
  return node.kind === "symbol" ? node.name : "";
}

function toBigInt(value: IntegerInput): bigint {
  if (typeof value === "bigint") return value;
  if (typeof value === "string") return BigInt(value);
  if (!Number.isSafeInteger(value)) {
    throw new RangeError("integer number inputs must be safe integers; pass a string or bigint for larger values");
  }
  return BigInt(value);
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

function escapeKey(value: string): string {
  return JSON.stringify(value);
}

// Arithmetic
export const ADD = sym("Add");
export const SUB = sym("Sub");
export const MUL = sym("Mul");
export const DIV = sym("Div");
export const POW = sym("Pow");
export const NEG = sym("Neg");
export const INV = sym("Inv");

// Elementary functions
export const EXP = sym("Exp");
export const LOG = sym("Log");
export const SIN = sym("Sin");
export const COS = sym("Cos");
export const TAN = sym("Tan");
export const SQRT = sym("Sqrt");
export const ATAN = sym("Atan");
export const ASIN = sym("Asin");
export const ACOS = sym("Acos");
export const SINH = sym("Sinh");
export const COSH = sym("Cosh");
export const TANH = sym("Tanh");
export const ASINH = sym("Asinh");
export const ACOSH = sym("Acosh");
export const ATANH = sym("Atanh");
export const COTH = sym("Coth");
export const SECH = sym("Sech");
export const CSCH = sym("Csch");

// Calculus and CAS heads
export const D = sym("D");
export const INTEGRATE = sym("Integrate");
export const SUM = sym("Sum");
export const PRODUCT = sym("Product");
export const FACTOR = sym("Factor");
export const SOLVE = sym("Solve");
export const SIMPLIFY = sym("Simplify");
export const SUBST = sym("Subst");

// Relations
export const EQUAL = sym("Equal");
export const NOT_EQUAL = sym("NotEqual");
export const LESS = sym("Less");
export const GREATER = sym("Greater");
export const LESS_EQUAL = sym("LessEqual");
export const GREATER_EQUAL = sym("GreaterEqual");

// Logic
export const TRUE = sym("True");
export const FALSE = sym("False");
export const AND = sym("And");
export const OR = sym("Or");
export const NOT = sym("Not");
export const IF = sym("If");

// Containers and binding
export const LIST = sym("List");
export const ASSIGN = sym("Assign");
export const DEFINE = sym("Define");
export const RULE = sym("Rule");

// MACSYMA/runtime-oriented heads used by later packages.
export const BLOCK = sym("Block");
export const RETURN = sym("Return");
export const WHILE = sym("While");
export const FOR_RANGE = sym("ForRange");
export const FOR_EACH = sym("ForEach");
export const ASSUME = sym("Assume");
export const FORGET = sym("Forget");
export const IS = sym("Is");
export const SIGN = sym("Sign");

// Named ODE solution function heads (Phase 27)
// Legendre ODE: (1-x²)y'' - 2xy' + n(n+1)y = 0
export const LEGENDRE_P = sym("LegendreP");  // Legendre polynomial of the first kind P_n(x)
export const LEGENDRE_Q = sym("LegendreQ");  // Legendre function of the second kind Q_n(x)
// Bessel ODE: x²y'' + xy' + (x²-ν²)y = 0
export const BESSEL_J = sym("BesselJ");      // Bessel function of the first kind J_ν(x)
export const BESSEL_Y = sym("BesselY");      // Bessel function of the second kind Y_ν(x)
// Hermite ODE: y'' - 2xy' + 2ny = 0
export const HERMITE_H = sym("HermiteH");    // Hermite polynomial H_n(x)
export const HERMITE_H2 = sym("HermiteH2");  // Second independent solution (parabolic cylinder)
// Chebyshev ODE: (1-x²)y'' - xy' + n²y = 0
export const CHEBYSHEV_T = sym("ChebyshevT"); // Chebyshev polynomial of the first kind T_n(x)
export const CHEBYSHEV_U = sym("ChebyshevU"); // Chebyshev polynomial of the second kind U_n(x)
