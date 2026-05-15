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
import { factorIntegerPolynomial } from "@coding-adventures/cas-factor";

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

function buildHandlerTable(simplify: boolean): ReadonlyMap<string, Handler> {
  const table = new Map<string, Handler>();
  table.set(ADD.name, binaryNumeric("Add", simplify, (a, b) => addNumeric(a, b), (expr, a, b) => {
    if (isZero(a)) return b;
    if (isZero(b)) return a;
    return expr;
  }));
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

  table.set(SIN.name, elementary("Sin", Math.sin, new Map([["0", int(0)]]), simplify));
  table.set(COS.name, elementary("Cos", Math.cos, new Map([["0", int(1)]]), simplify));
  table.set(TAN.name, elementary("Tan", Math.tan, new Map([["0", int(0)]]), simplify));
  table.set(EXP.name, elementary("Exp", Math.exp, new Map([["0", int(1)]]), simplify));
  table.set(LOG.name, elementary("Log", Math.log, new Map([["1", int(0)]]), simplify));
  table.set(SQRT.name, elementary("Sqrt", Math.sqrt, new Map([["0", int(0)], ["1", int(1)]]), simplify));
  table.set(ATAN.name, elementary("Atan", Math.atan, new Map([["0", int(0)]]), simplify));
  table.set(ASIN.name, elementary("Asin", Math.asin, new Map([["0", int(0)]]), simplify));
  table.set(ACOS.name, elementary("Acos", Math.acos, new Map(), simplify));
  table.set(SINH.name, elementary("Sinh", Math.sinh, new Map([["0", int(0)]]), simplify));
  table.set(COSH.name, elementary("Cosh", Math.cosh, new Map([["0", int(1)]]), simplify));
  table.set(TANH.name, elementary("Tanh", Math.tanh, new Map([["0", int(0)]]), simplify));
  table.set(ASINH.name, elementary("Asinh", Math.asinh, new Map([["0", int(0)]]), simplify));
  table.set(ACOSH.name, elementary("Acosh", Math.acosh, new Map(), simplify));
  table.set(ATANH.name, elementary("Atanh", Math.atanh, new Map([["0", int(0)]]), simplify));
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
  if (simplify) {
    table.set(D.name, differentiate());
    table.set(INTEGRATE.name, integrate());
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
    return commonFactored === undefined ? expr : vm.eval(commonFactored);
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

function integrate(): Handler {
  return (vm, expr) => {
    if (expr.args.length !== 2 && expr.args.length !== 4) {
      throw new ArityError(`Integrate expects 2 or 4 arguments, got ${expr.args.length}`);
    }
    const [f, x] = expr.args;
    if (x.kind !== "symbol") {
      return expr;
    }
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
      return expr;
    }
    return isDeferredIntegral(result, f, x) ? result : vm.eval(result);
  };
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
    return undefined;
  }
  if (equals(f.head, DIV)) {
    const [numerator, denominator] = binaryArgs(f);
    if (!dependsOn(numerator, x) && equals(denominator, x)) {
      return app(MUL, [numerator, app(LOG, [x])]);
    }
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

  return undefined;
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
