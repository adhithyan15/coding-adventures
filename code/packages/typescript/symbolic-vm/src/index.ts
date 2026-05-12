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
  FALSE,
  GREATER,
  GREATER_EQUAL,
  IF,
  INTEGRATE,
  INV,
  IRApply,
  IRNode,
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
  if (simplify) {
    table.set(D.name, differentiate());
    table.set(INTEGRATE.name, integrate());
  }

  return table;
}

function integrate(): Handler {
  return (vm, expr) => {
    if (expr.args.length !== 2) {
      throw new ArityError(`Integrate expects 2 arguments, got ${expr.args.length}`);
    }
    const [f, x] = expr.args;
    if (x.kind !== "symbol") {
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
