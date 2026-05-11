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
  DEFINE,
  DIV,
  EQUAL,
  EXP,
  FALSE,
  GREATER,
  GREATER_EQUAL,
  IF,
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

  return table;
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
