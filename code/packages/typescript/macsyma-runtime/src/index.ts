import { compileMacsyma } from "@coding-adventures/macsyma-compiler";
import {
  append,
  applyList,
  first,
  flatten,
  last,
  length,
  mapList,
  part,
  range as rangeList,
  rest,
  reverse,
  sortList,
} from "@coding-adventures/cas-list-operations";
import { AssumptionContext, simplify as simplifyCas } from "@coding-adventures/cas-simplify";
import { MacsymaDialect, pretty } from "@coding-adventures/cas-pretty-printer";
import { solveLinearSystem, trySolveInequality, trySolveTranscendental } from "@coding-adventures/cas-solve";
import { subst } from "@coding-adventures/cas-substitution";
import { expandTrig, trigReduce, trigSimplify } from "@coding-adventures/cas-trig";
import {
  ACOS,
  ACOSH,
  ASSUME,
  ASIN,
  ASINH,
  ATAN,
  ATANH,
  COS,
  COSH,
  D,
  EXP,
  FACTOR,
  FALSE,
  FORGET,
  GREATER,
  GREATER_EQUAL,
  INTEGRATE,
  IS,
  LESS,
  LESS_EQUAL,
  LIST,
  LOG,
  SIN,
  SINH,
  SOLVE,
  SQRT,
  SUBST,
  SUM,
  TAN,
  TANH,
  TRUE,
  app,
  equals,
  numberNode,
  stringNode,
  sym,
  toDisplayString,
  type IRApply,
  type IRNode,
  type IRSymbol,
} from "@coding-adventures/symbolic-ir";
import { SymbolicBackend, VM, type Handler } from "@coding-adventures/symbolic-vm";

export const DISPLAY = sym("Display");
export const SUPPRESS = sym("Suppress");
export const KILL = sym("Kill");
export const EV = sym("Ev");
export const ALL_SYMBOL = sym("all");
export const DECLARE = sym("Declare");
export const PROPERTIES = sym("Properties");
export const PROP_VARS = sym("PropVars");

export const SIMPLIFY = sym("Simplify");
export const EXPAND = sym("Expand");
export const RAT_SIMPLIFY = sym("RatSimplify");
export const TRIG_SIMPLIFY = sym("TrigSimplify");
export const TRIG_EXPAND = sym("TrigExpand");
export const TRIG_REDUCE = sym("TrigReduce");
export const FLOAT_FUNC = sym("Float");
export const LENGTH = sym("Length");
export const FIRST = sym("First");
export const REST = sym("Rest");
export const LAST = sym("Last");
export const APPEND = sym("Append");
export const REVERSE = sym("Reverse");
export const RANGE = sym("Range");
export const MAP = sym("Map");
export const APPLY = sym("Apply");
export const SORT = sym("Sort");
export const PART = sym("Part");
export const FLATTEN = sym("Flatten");
export const JOIN = sym("Join");

export const MACSYMA_NAME_TABLE: ReadonlyMap<string, IRSymbol> = new Map<string, IRSymbol>([
  ["diff", D],
  ["integrate", INTEGRATE],
  ["sin", SIN],
  ["cos", COS],
  ["tan", TAN],
  ["asin", ASIN],
  ["acos", ACOS],
  ["atan", ATAN],
  ["sinh", SINH],
  ["cosh", COSH],
  ["tanh", TANH],
  ["asinh", ASINH],
  ["acosh", ACOSH],
  ["atanh", ATANH],
  ["coth", sym("Coth")],
  ["sech", sym("Sech")],
  ["csch", sym("Csch")],
  ["log", LOG],
  ["exp", EXP],
  ["sqrt", SQRT],
  ["sum", SUM],
  ["product", sym("Product")],
  ["subst", SUBST],
  ["simplify", SIMPLIFY],
  ["expand", EXPAND],
  ["factor", FACTOR],
  ["solve", SOLVE],
  ["nsolve", sym("NSolve")],
  ["linsolve", SOLVE],
  ["taylor", sym("Taylor")],
  ["limit", sym("Limit")],
  ["float", FLOAT_FUNC],
  ["length", LENGTH],
  ["first", FIRST],
  ["rest", REST],
  ["last", LAST],
  ["append", APPEND],
  ["reverse", REVERSE],
  ["makelist", sym("MakeList")],
  ["map", MAP],
  ["apply", APPLY],
  ["sublist", sym("Select")],
  ["sort", SORT],
  ["part", PART],
  ["flatten", FLATTEN],
  ["join", JOIN],
  ["matrix", sym("Matrix")],
  ["transpose", sym("Transpose")],
  ["determinant", sym("Determinant")],
  ["invert", sym("Inverse")],
  ["dot", sym("Dot")],
  ["mattrace", sym("Trace")],
  ["matrix_size", sym("Dimensions")],
  ["ident", sym("IdentityMatrix")],
  ["zeromatrix", sym("ZeroMatrix")],
  ["rank", sym("Rank")],
  ["rowreduce", sym("RowReduce")],
  ["gcd", sym("Gcd")],
  ["lcm", sym("Lcm")],
  ["mod", sym("Mod")],
  ["floor", sym("Floor")],
  ["ceiling", sym("Ceiling")],
  ["abs", sym("Abs")],
  ["sign", sym("Sign")],
  ["lhs", sym("Lhs")],
  ["rhs", sym("Rhs")],
  ["at", sym("At")],
  ["primep", sym("IsPrime")],
  ["is_prime", sym("IsPrime")],
  ["next_prime", sym("NextPrime")],
  ["prev_prime", sym("PrevPrime")],
  ["ifactor", sym("FactorInteger")],
  ["divisors", sym("Divisors")],
  ["totient", sym("Totient")],
  ["moebius", sym("MoebiusMu")],
  ["jacobi", sym("JacobiSymbol")],
  ["chinese", sym("ChineseRemainder")],
  ["numdigits", sym("IntegerLength")],
  ["radcan", sym("Radcan")],
  ["logcontract", sym("LogContract")],
  ["logexpand", sym("LogExpand")],
  ["exponentialize", sym("Exponentialize")],
  ["demoivre", sym("DeMoivre")],
  ["cbrt", sym("Cbrt")],
  ["trigsimp", TRIG_SIMPLIFY],
  ["trigexpand", TRIG_EXPAND],
  ["trigreduce", TRIG_REDUCE],
  ["collect", sym("Collect")],
  ["together", sym("Together")],
  ["ratsimp", RAT_SIMPLIFY],
  ["partfrac", sym("Apart")],
  ["%i", sym("ImaginaryUnit")],
  ["realpart", sym("Re")],
  ["imagpart", sym("Im")],
  ["conjugate", sym("Conjugate")],
  ["cabs", sym("Abs")],
  ["carg", sym("Arg")],
  ["rectform", sym("RectForm")],
  ["polarform", sym("PolarForm")],
  ["laplace", sym("Laplace")],
  ["ilt", sym("ILT")],
  ["delta", sym("DiracDelta")],
  ["hstep", sym("UnitStep")],
  ["unit_step", sym("UnitStep")],
  ["fourier", sym("Fourier")],
  ["ifourier", sym("IFourier")],
  ["ode2", sym("Ode2")],
  ["algfactor", sym("AlgFactor")],
  ["groebner", sym("Groebner")],
  ["poly_reduce", sym("PolyReduce")],
  ["ideal_solve", sym("IdealSolve")],
  ["kill", KILL],
  ["ev", EV],
  ["block", sym("Block")],
  ["assume", ASSUME],
  ["forget", FORGET],
  ["is", IS],
  ["declare", DECLARE],
  ["properties", PROPERTIES],
  ["propvars", PROP_VARS],
  ["matchdeclare", sym("MatchDeclare")],
  ["defrule", sym("DefRule")],
  ["apply1", sym("Apply1")],
  ["apply2", sym("Apply2")],
  ["tellsimp", sym("TellSimp")],
  ["erf", sym("Erf")],
  ["erfc", sym("Erfc")],
  ["erfi", sym("Erfi")],
  ["si", sym("Si")],
  ["ci", sym("Ci")],
  ["shi", sym("Shi")],
  ["chi", sym("Chi")],
  ["li2", sym("Li2")],
  ["gamma", sym("Gamma")],
  ["beta", sym("Beta")],
  ["fresnel_s", sym("FresnelS")],
  ["fresnel_c", sym("FresnelC")],
  ["lambert_w", sym("LambertW")],
]);

const MACSYMA_HELP_TOPICS: Readonly<Record<string, string>> = Object.freeze({
  arithmetic: "Arithmetic: use +, -, *, /, and ^. Example: expand((x + 1)^2);",
  calculus: "Calculus: diff(expr, var), integrate(expr, var), limit(expr, var, point), and taylor(expr, var, point, order).",
  diff: "diff(expr, var) differentiates expr with respect to var. Example: diff(x^3, x);",
  integrate: "integrate(expr, var) computes an antiderivative when supported. Example: integrate(x^2, x);",
  solve: "solve(expr, var) solves equations or supported inequalities. Use linsolve([...], [...]) for linear systems and nsolve(poly, var) for numeric polynomial roots.",
  matrix: "Matrix tools: matrix([...], ...), transpose, determinant, invert, dot, rank, rowreduce, ident, zeromatrix, and matrix_size.",
  lists: "List tools: length, first, rest, last, append, reverse, range, map, apply, sublist, sort, part, flatten, join, and makelist.",
  assumptions: "Assumptions: assume(x > 0), declare(x, positive), is(x > 0), forget(), properties(x), and propvars().",
  properties: "properties(symbol) lists declared properties. propvars() lists symbols with declared properties.",
  display: "Display: terminate with ; to show output and $ to suppress it. ev(expr, display2d) renders 2D output.",
  history: "History: % is the last output; %iN and %oN refer to input and output number N.",
  showtime: "showtime:true enables per-expression timing; showtime:false disables it.",
  repl: "REPL commands: :quit exits. Use --file path.mac for batch execution.",
});

const MACSYMA_HELP_ALIASES: Readonly<Record<string, string>> = Object.freeze({
  d: "diff",
  derivative: "diff",
  integral: "integrate",
  matrices: "matrix",
  list: "lists",
  assume: "assumptions",
  declare: "assumptions",
  propvars: "properties",
  display2d: "display",
  "%": "history",
  timing: "showtime",
  quit: "repl",
});

export function parseMacsymaHelpQuery(source: string): string | undefined {
  const stripped = source.trim();
  if (!stripped.startsWith("?")) return undefined;
  let topic = stripped.replace(/^\?+/, "").trim();
  if (topic.endsWith(";") || topic.endsWith("$")) {
    topic = topic.slice(0, -1).trim();
  }
  return topic;
}

export function macsymaHelpText(topic = ""): string {
  const rawKey = topic.trim().toLowerCase();
  if (rawKey === "") {
    return `MACSYMA help topics: ${Object.keys(MACSYMA_HELP_TOPICS).sort().join(", ")}. Use ? topic for details.`;
  }
  const key = MACSYMA_HELP_ALIASES[rawKey] ?? rawKey;
  const text = MACSYMA_HELP_TOPICS[key];
  if (text !== undefined) return text;
  return `No MACSYMA help topic named ${JSON.stringify(topic)}. Available topics: ${Object.keys(MACSYMA_HELP_TOPICS).sort().join(", ")}.`;
}

export type CompilerNameTableTarget =
  | Map<string, IRSymbol>
  | { [name: string]: IRSymbol | undefined };

export function extendCompilerNameTable(target: CompilerNameTableTarget): void {
  for (const [name, head] of MACSYMA_NAME_TABLE) {
    if (target instanceof Map) {
      target.set(name, head);
    } else {
      target[name] = head;
    }
  }
}

export interface EvalResult {
  readonly inputIndex: number;
  readonly outputIndex: number;
  readonly input: IRNode;
  readonly output: IRNode;
  readonly outputText: string;
  readonly display: boolean;
  readonly timingText?: string;
}

export class History {
  private readonly inputNodes: IRNode[] = [];
  private readonly outputNodes: IRNode[] = [];

  recordInput(node: IRNode): number {
    this.inputNodes.push(node);
    return this.inputNodes.length;
  }

  recordOutput(node: IRNode): number {
    this.outputNodes.push(node);
    return this.outputNodes.length;
  }

  getInput(index: number): IRNode | undefined {
    return integerIndex(index) ? this.inputNodes[index - 1] : undefined;
  }

  getOutput(index: number): IRNode | undefined {
    return integerIndex(index) ? this.outputNodes[index - 1] : undefined;
  }

  lastOutput(): IRNode | undefined {
    return this.outputNodes.at(-1);
  }

  nextInputIndex(): number {
    return this.inputNodes.length + 1;
  }

  inputs(): readonly IRNode[] {
    return this.inputNodes;
  }

  outputs(): readonly IRNode[] {
    return this.outputNodes;
  }

  reset(): void {
    this.inputNodes.length = 0;
    this.outputNodes.length = 0;
  }

  resolveHistorySymbol(name: string): IRNode | undefined {
    if (name === "%") return this.lastOutput();
    if (name.startsWith("%i") && /^\d+$/.test(name.slice(2))) {
      return this.getInput(Number(name.slice(2)));
    }
    if (name.startsWith("%o") && /^\d+$/.test(name.slice(2))) {
      return this.getOutput(Number(name.slice(2)));
    }
    return undefined;
  }
}

export class MacsymaBackend extends SymbolicBackend {
  numer = false;
  showtime = false;
  readonly assumptions = new AssumptionContext();
  private readonly runtimeTable: ReadonlyMap<string, Handler>;
  private readonly runtimeHeld: ReadonlySet<string>;

  constructor(private readonly history: History) {
    super();
    this.bindMacsymaConstants();

    const table = new Map(super.handlers());
    table.set(DISPLAY.name, displayHandler);
    table.set(SUPPRESS.name, suppressHandler);
    table.set(KILL.name, makeKillHandler(this));
    table.set(EV.name, makeEvHandler());
    table.set(ASSUME.name, assumeHandler);
    table.set(FORGET.name, forgetHandler);
    table.set(IS.name, isHandler);
    table.set(DECLARE.name, declareHandler);
    table.set(PROPERTIES.name, propertiesHandler);
    table.set(PROP_VARS.name, propvarsHandler);
    table.set(SOLVE.name, solveHandler);
    table.set(SUBST.name, substHandler);
    table.set(SIMPLIFY.name, unaryHandler((value) => simplifyCas(value)));
    table.set(RAT_SIMPLIFY.name, unaryHandler((value) => simplifyCas(value)));
    table.set(TRIG_SIMPLIFY.name, unaryHandler((value) => simplifyCas(trigSimplify(value))));
    table.set(TRIG_EXPAND.name, unaryHandler((value) => expandTrig(value)));
    table.set(TRIG_REDUCE.name, unaryHandler((value) => trigReduce(value)));
    table.set(LENGTH.name, listHandler(1, ([value]) => length(value)));
    table.set(FIRST.name, listHandler(1, ([value]) => first(value)));
    table.set(REST.name, listHandler(1, ([value]) => rest(value)));
    table.set(LAST.name, listHandler(1, ([value]) => last(value)));
    table.set(REVERSE.name, listHandler(1, ([value]) => reverse(value)));
    table.set(APPEND.name, listHandler(null, (args) => append(...args)));
    table.set(JOIN.name, listHandler(null, (args) => append(...args)));
    table.set(RANGE.name, listHandler(null, rangeHandler));
    table.set(MAP.name, listHandler(2, ([head, value]) => mapList(head, value)));
    table.set(APPLY.name, listHandler(2, ([head, value]) => applyList(head, value)));
    table.set(SORT.name, listHandler(1, ([value]) => sortList(value)));
    table.set(PART.name, listHandler(2, ([value, index]) => part(value, integerArgument(index))));
    table.set(FLATTEN.name, listHandler(null, flattenHandler));
    this.runtimeTable = table;
    this.runtimeHeld = new Set([
      ...super.holdHeads(),
      KILL.name,
      EV.name,
      ASSUME.name,
      FORGET.name,
      IS.name,
      DECLARE.name,
      PROPERTIES.name,
      PROP_VARS.name,
      SOLVE.name,
      SUBST.name,
    ]);
  }

  override lookup(name: string): IRNode | undefined {
    const envValue = super.lookup(name);
    if (envValue !== undefined) return envValue;
    return this.history.resolveHistorySymbol(name);
  }

  override bind(name: string, value: IRNode): void {
    super.bind(name, value);
    if (name === "showtime") {
      this.showtime = equals(value, TRUE);
    }
  }

  override unbind(name: string): void {
    super.unbind(name);
    if (name === "showtime") {
      this.showtime = false;
      super.bind("showtime", FALSE);
    }
  }

  override handlers(): ReadonlyMap<string, Handler> {
    return this.runtimeTable;
  }

  override holdHeads(): ReadonlySet<string> {
    return this.runtimeHeld;
  }

  resetEnvironment(): void {
    this.env.clear();
    this.env.set(TRUE.name, TRUE);
    this.env.set(FALSE.name, FALSE);
    this.showtime = false;
    this.env.set("showtime", FALSE);
    this.bindMacsymaConstants();
    this.history.reset();
  }

  withNumer<T>(body: () => T): T {
    const previous = this.numer;
    this.numer = true;
    try {
      return body();
    } finally {
      this.numer = previous;
    }
  }

  private bindMacsymaConstants(): void {
    this.bind("%pi", numberNode(Math.PI));
    this.bind("%e", numberNode(Math.E));
    this.bind("%i", sym("ImaginaryUnit"));
    this.bind("showtime", this.showtime ? TRUE : FALSE);
  }
}

export class MacsymaSession {
  private readonly sessionHistory = new History();
  private readonly backend = new MacsymaBackend(this.sessionHistory);
  private readonly vm = new VM(this.backend);

  history(): History {
    return this.sessionHistory;
  }

  evalSource(source: string): EvalResult[] {
    const helpTopic = parseMacsymaHelpQuery(source);
    if (helpTopic !== undefined) {
      return [this.evalHelpQuery(helpTopic)];
    }
    return this.evalStatements(compileMacsyma(source, { wrapTerminators: true }));
  }

  evalStatements(statements: readonly IRNode[]): EvalResult[] {
    return statements.map((statement) => this.evalStatement(canonicalizeRuntimeNames(statement)));
  }

  evalJson(source: string): string {
    try {
      return stringifyJsonResponse(evalResponseOk(this.evalSource(source), this.sessionHistory));
    } catch (error) {
      return stringifyJsonResponse(evalResponseError(errorMessage(error), this.sessionHistory));
    }
  }

  resetHistory(): void {
    this.sessionHistory.reset();
  }

  private evalStatement(statement: IRNode): EvalResult {
    const [input, display] = unwrapDisplay(statement);
    const showTiming = this.backend.showtime && !isShowtimeAssignment(input);
    const startedAt = Date.now();
    const inputIndex = this.sessionHistory.recordInput(input);
    const output = this.vm.eval(input);
    const elapsedSeconds = (Date.now() - startedAt) / 1000;
    const outputIndex = this.sessionHistory.recordOutput(output);
    const outputText = displayTextFor(input, output);
    if (isKillAll(input)) {
      this.sessionHistory.reset();
    }
    return {
      inputIndex,
      outputIndex,
      input,
      output,
      outputText,
      display,
      ...(showTiming ? { timingText: formatTiming(elapsedSeconds) } : {}),
    };
  }

  private evalHelpQuery(topic: string): EvalResult {
    const query = topic === "" ? "?" : `? ${topic}`;
    const text = macsymaHelpText(topic);
    const input = stringNode(query);
    const output = stringNode(text);
    const inputIndex = this.sessionHistory.recordInput(input);
    const outputIndex = this.sessionHistory.recordOutput(output);
    return {
      inputIndex,
      outputIndex,
      input,
      output,
      outputText: text,
      display: true,
    };
  }
}

export function evalSourceJson(source: string): string {
  return new MacsymaSession().evalJson(source);
}

export type JsonIrNode =
  | { readonly kind: "symbol"; readonly name: string }
  | { readonly kind: "integer"; readonly value: string }
  | { readonly kind: "rational"; readonly numerator: string; readonly denominator: string }
  | { readonly kind: "float"; readonly value: number }
  | { readonly kind: "string"; readonly value: string }
  | { readonly kind: "apply"; readonly head: JsonIrNode; readonly args: readonly JsonIrNode[] };

export interface JsonEvalResult {
  readonly inputIndex: number;
  readonly outputIndex: number;
  readonly display: boolean;
  readonly inputText: string;
  readonly outputText: string;
  readonly timingText?: string;
  readonly inputIr: JsonIrNode;
  readonly outputIr: JsonIrNode;
}

export interface JsonHistory {
  readonly inputCount: number;
  readonly outputCount: number;
  readonly nextInputIndex: number;
  readonly lastOutputText: string | null;
}

export interface JsonEvalResponse {
  readonly ok: boolean;
  readonly results: readonly JsonEvalResult[];
  readonly visibleOutputs: readonly string[];
  readonly history: JsonHistory;
  readonly error?: {
    readonly kind: "runtime";
    readonly message: string;
  };
}

export function irToJson(node: IRNode): JsonIrNode {
  switch (node.kind) {
    case "symbol":
      return { kind: "symbol", name: node.name };
    case "integer":
      return { kind: "integer", value: node.value.toString() };
    case "rational":
      return { kind: "rational", numerator: node.numer.toString(), denominator: node.denom.toString() };
    case "float":
      return { kind: "float", value: node.value };
    case "string":
      return { kind: "string", value: node.value };
    case "apply":
      return { kind: "apply", head: irToJson(node.head), args: node.args.map(irToJson) };
  }
}

function evalResponseOk(results: readonly EvalResult[], history: History): JsonEvalResponse {
  const jsonResults = results.map(resultToJson);
  return {
    ok: true,
    results: jsonResults,
    visibleOutputs: jsonResults.flatMap((result) => [
      ...(result.display ? [result.outputText] : []),
      ...(result.timingText === undefined ? [] : [result.timingText]),
    ]),
    history: historyToJson(history),
  };
}

function evalResponseError(message: string, history: History): JsonEvalResponse {
  return {
    ok: false,
    results: [],
    visibleOutputs: [],
    history: historyToJson(history),
    error: { kind: "runtime", message },
  };
}

function resultToJson(result: EvalResult): JsonEvalResult {
  return {
    inputIndex: result.inputIndex,
    outputIndex: result.outputIndex,
    display: result.display,
    inputText: toDisplayString(result.input),
    outputText: result.outputText,
    ...(result.timingText === undefined ? {} : { timingText: result.timingText }),
    inputIr: irToJson(result.input),
    outputIr: irToJson(result.output),
  };
}

function historyToJson(history: History): JsonHistory {
  const lastOutput = history.lastOutput();
  return {
    inputCount: history.inputs().length,
    outputCount: history.outputs().length,
    nextInputIndex: history.nextInputIndex(),
    lastOutputText: lastOutput === undefined ? null : toDisplayString(lastOutput),
  };
}

function stringifyJsonResponse(response: JsonEvalResponse): string {
  return JSON.stringify(response);
}

function unwrapDisplay(statement: IRNode): readonly [IRNode, boolean] {
  if (statement.kind !== "apply" || statement.args.length !== 1) return [statement, true];
  if (equals(statement.head, DISPLAY)) return [statement.args[0], true];
  if (equals(statement.head, SUPPRESS)) return [statement.args[0], false];
  return [statement, true];
}

function canonicalizeRuntimeNames(node: IRNode): IRNode {
  if (node.kind !== "apply") return node;
  const head = canonicalCallHead(node.head);
  return app(head, node.args.map(canonicalizeRuntimeNames));
}

function canonicalCallHead(head: IRNode): IRNode {
  if (head.kind !== "symbol") return canonicalizeRuntimeNames(head);
  return MACSYMA_NAME_TABLE.get(head.name) ?? head;
}

function displayHandler(_vm: VM, expr: IRApply): IRNode {
  if (expr.args.length !== 1) throw new Error(`Display takes 1 arg, got ${expr.args.length}`);
  return expr.args[0];
}

function suppressHandler(_vm: VM, expr: IRApply): IRNode {
  if (expr.args.length !== 1) throw new Error(`Suppress takes 1 arg, got ${expr.args.length}`);
  return expr.args[0];
}

function makeKillHandler(backend: MacsymaBackend): Handler {
  return (_vm, expr) => {
    for (const arg of expr.args) {
      if (arg.kind !== "symbol") continue;
      if (arg.name === ALL_SYMBOL.name) {
        backend.resetEnvironment();
      } else {
        backend.unbind(arg.name);
      }
    }
    return sym("done");
  };
}

function makeEvHandler(): Handler {
  return (vm, expr) => {
    if (expr.args.length === 0) return expr;

    const flags = new Set(
      expr.args
        .slice(1)
        .filter((arg): arg is IRSymbol => arg.kind === "symbol")
        .map((arg) => arg.name),
    );
    const backend = vm.backend;
    let result = backend instanceof MacsymaBackend
      ? backend.withNumer(() => vm.eval(expr.args[0]))
      : vm.eval(expr.args[0]);

    if (flags.has("expand")) result = evalSupportedFlag(vm, result, EXPAND);
    if (flags.has("factor")) result = evalSupportedFlag(vm, result, FACTOR);
    if (flags.has("ratsimp")) result = evalSupportedFlag(vm, result, RAT_SIMPLIFY);
    if (flags.has("trigsimp")) result = evalSupportedFlag(vm, result, TRIG_SIMPLIFY);
    if (flags.has("trigexpand")) result = evalSupportedFlag(vm, result, TRIG_EXPAND);
    if (flags.has("trigreduce")) result = evalSupportedFlag(vm, result, TRIG_REDUCE);
    if (flags.has("numer") || flags.has("float")) result = numerFold(result);

    return result;
  };
}

function assumeHandler(vm: VM, expr: IRApply): IRNode {
  const ctx = assumptionContext(vm);
  if (ctx === undefined) return expr;
  if (expr.args.length === 1) {
    ctx.assumeRelation(expr.args[0]);
  } else if (expr.args.length === 2) {
    ctx.assumeProperty(expr.args[0], expr.args[1]);
  }
  return sym("done");
}

function forgetHandler(vm: VM, expr: IRApply): IRNode {
  const ctx = assumptionContext(vm);
  if (ctx === undefined) return expr;
  if (expr.args.length === 0) {
    ctx.forgetAll();
  } else {
    ctx.forgetRelation(expr.args[0]);
  }
  return sym("done");
}

function isHandler(vm: VM, expr: IRApply): IRNode {
  const ctx = assumptionContext(vm);
  if (ctx === undefined || expr.args.length !== 1) return expr;
  const result = ctx.isTrueRelation(expr.args[0]);
  if (result === true) return TRUE;
  if (result === false) return FALSE;
  return sym("unknown");
}

function declareHandler(vm: VM, expr: IRApply): IRNode {
  const ctx = assumptionContext(vm);
  if (ctx === undefined || expr.args.length % 2 !== 0) return expr;
  for (let i = 0; i < expr.args.length; i += 2) {
    ctx.assumeProperty(expr.args[i], expr.args[i + 1]);
  }
  return sym("done");
}

function propertiesHandler(vm: VM, expr: IRApply): IRNode {
  const ctx = assumptionContext(vm);
  if (ctx === undefined || expr.args.length !== 1) return expr;
  const [target] = expr.args;
  if (target.kind !== "symbol") return app(LIST, []);
  return app(LIST, ctx.factsFor(target.name).map((fact) => sym(fact)));
}

function propvarsHandler(vm: VM, expr: IRApply): IRNode {
  const ctx = assumptionContext(vm);
  if (ctx === undefined || expr.args.length !== 0) return expr;
  return app(LIST, ctx.symbolsWithFacts().map((name) => sym(name)));
}

function assumptionContext(vm: VM): AssumptionContext | undefined {
  return vm.backend instanceof MacsymaBackend ? vm.backend.assumptions : undefined;
}

function solveHandler(_vm: VM, expr: IRApply): IRNode {
  if (expr.args.length !== 2) return expr;
  const [equationsNode, variablesNode] = expr.args;

  if (variablesNode.kind === "symbol" && isInequality(equationsNode)) {
    const solutions = trySolveInequality(equationsNode, variablesNode);
    return solutions === null ? expr : app(LIST, solutions);
  }

  if (variablesNode.kind === "symbol") {
    const solutions = trySolveTranscendental(equationsNode, variablesNode);
    return solutions === null ? expr : app(LIST, solutions);
  }

  if (!isList(equationsNode) || !isList(variablesNode)) return expr;

  const variables: IRSymbol[] = [];
  for (const variable of variablesNode.args) {
    if (variable.kind !== "symbol") return expr;
    variables.push(variable);
  }

  const rules = solveLinearSystem(equationsNode.args, variables);
  return rules === null ? expr : app(LIST, rules);
}

function substHandler(_vm: VM, expr: IRApply): IRNode {
  if (expr.args.length !== 3) return expr;
  const [value, variable, target] = expr.args;
  return subst(value, variable, target);
}

function unaryHandler(body: (value: IRNode) => IRNode): Handler {
  return (_vm, expr) => {
    if (expr.args.length !== 1) return expr;
    try {
      return body(expr.args[0]);
    } catch {
      return expr;
    }
  };
}

function listHandler(
  arity: number | null,
  body: (args: readonly IRNode[]) => IRNode,
): Handler {
  return (_vm, expr) => {
    if (arity !== null && expr.args.length !== arity) return expr;
    try {
      return body(expr.args);
    } catch {
      return expr;
    }
  };
}

function rangeHandler(args: readonly IRNode[]): IRNode {
  if (args.length < 1 || args.length > 3) throw new Error("Range takes 1 to 3 arguments");
  const start = integerArgument(args[0]);
  const stop = args.length >= 2 ? integerArgument(args[1]) : undefined;
  const step = args.length >= 3 ? integerArgument(args[2]) : 1;
  return rangeList(start, stop, step);
}

function flattenHandler(args: readonly IRNode[]): IRNode {
  if (args.length < 1 || args.length > 2) throw new Error("Flatten takes 1 or 2 arguments");
  const depth = args.length === 2 ? integerArgument(args[1]) : 1;
  return flatten(args[0], depth);
}

function integerArgument(node: IRNode): number {
  if (node.kind !== "integer") throw new Error("expected integer argument");
  const value = Number(node.value);
  if (!Number.isSafeInteger(value) || BigInt(value) !== node.value) {
    throw new Error("integer argument is outside the safe integer range");
  }
  return value;
}

function isInequality(node: IRNode): node is IRApply {
  return node.kind === "apply"
    && (equals(node.head, LESS)
      || equals(node.head, GREATER)
      || equals(node.head, LESS_EQUAL)
      || equals(node.head, GREATER_EQUAL));
}

function evalSupportedFlag(vm: VM, result: IRNode, head: IRSymbol): IRNode {
  if (!vm.backend.handlers().has(head.name)) return result;
  try {
    return vm.eval(app(head, [result]));
  } catch {
    return result;
  }
}

function isList(node: IRNode): node is IRApply {
  return node.kind === "apply" && equals(node.head, LIST);
}

function numerFold(node: IRNode): IRNode {
  switch (node.kind) {
    case "integer":
      return numberNode(Number(node.value));
    case "rational":
      return numberNode(Number(node.numer) / Number(node.denom));
    case "apply": {
      const args = node.args.map((arg, index) => {
        if (equals(node.head, sym("Pow")) && index === 1) return arg;
        return numerFold(arg);
      });
      return app(node.head, args);
    }
    default:
      return node;
  }
}

function displayTextFor(input: IRNode, output: IRNode): string {
  if (hasEvFlag(input, "display2d")) {
    return pretty(output, MacsymaDialect, "2d");
  }
  return toDisplayString(output);
}

function hasEvFlag(input: IRNode, flag: string): boolean {
  return input.kind === "apply"
    && equals(input.head, EV)
    && input.args.slice(1).some((arg) => arg.kind === "symbol" && arg.name === flag);
}

function isKillAll(input: IRNode): boolean {
  return input.kind === "apply"
    && equals(input.head, KILL)
    && input.args.some((arg) => arg.kind === "symbol" && arg.name === ALL_SYMBOL.name);
}

function isShowtimeAssignment(input: IRNode): boolean {
  return input.kind === "apply"
    && input.head.kind === "symbol"
    && input.head.name === "Assign"
    && input.args.length === 2
    && input.args[0].kind === "symbol"
    && input.args[0].name === "showtime";
}

function formatTiming(elapsedSeconds: number): string {
  return `Evaluation took ${elapsedSeconds.toFixed(6)} seconds.`;
}

function integerIndex(value: number): boolean {
  return Number.isSafeInteger(value) && value >= 1;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export { app, sym, toDisplayString };
export type { IRApply, IRNode };
