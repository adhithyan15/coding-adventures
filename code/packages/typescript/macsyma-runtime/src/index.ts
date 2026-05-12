import { compileMacsyma } from "@coding-adventures/macsyma-compiler";
import { solveLinearSystem } from "@coding-adventures/cas-solve";
import {
  ACOS,
  ACOSH,
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
  INTEGRATE,
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

export const EXPAND = sym("Expand");
export const RAT_SIMPLIFY = sym("RatSimplify");
export const TRIG_SIMPLIFY = sym("TrigSimplify");
export const FLOAT_FUNC = sym("Float");

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
  ["simplify", sym("Simplify")],
  ["expand", EXPAND],
  ["factor", FACTOR],
  ["solve", SOLVE],
  ["nsolve", sym("NSolve")],
  ["linsolve", SOLVE],
  ["taylor", sym("Taylor")],
  ["limit", sym("Limit")],
  ["float", FLOAT_FUNC],
  ["length", sym("Length")],
  ["first", sym("First")],
  ["rest", sym("Rest")],
  ["last", sym("Last")],
  ["append", sym("Append")],
  ["reverse", sym("Reverse")],
  ["makelist", sym("MakeList")],
  ["map", sym("Map")],
  ["apply", sym("Apply")],
  ["sublist", sym("Select")],
  ["sort", sym("Sort")],
  ["part", sym("Part")],
  ["flatten", sym("Flatten")],
  ["join", sym("Join")],
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
  ["trigexpand", sym("TrigExpand")],
  ["trigreduce", sym("TrigReduce")],
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
  ["assume", sym("Assume")],
  ["forget", sym("Forget")],
  ["is", sym("Is")],
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
  readonly display: boolean;
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
    table.set(SOLVE.name, solveHandler);
    this.runtimeTable = table;
    this.runtimeHeld = new Set([...super.holdHeads(), KILL.name, EV.name, SOLVE.name]);
  }

  override lookup(name: string): IRNode | undefined {
    const envValue = super.lookup(name);
    if (envValue !== undefined) return envValue;
    return this.history.resolveHistorySymbol(name);
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
    const inputIndex = this.sessionHistory.recordInput(input);
    const output = this.vm.eval(input);
    const outputIndex = this.sessionHistory.recordOutput(output);
    if (isKillAll(input)) {
      this.sessionHistory.reset();
    }
    return { inputIndex, outputIndex, input, output, display };
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
    visibleOutputs: jsonResults.filter((result) => result.display).map((result) => result.outputText),
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
    outputText: toDisplayString(result.output),
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
    if (flags.has("numer") || flags.has("float")) result = numerFold(result);

    return result;
  };
}

function solveHandler(_vm: VM, expr: IRApply): IRNode {
  if (expr.args.length !== 2) return expr;
  const [equationsNode, variablesNode] = expr.args;
  if (!isList(equationsNode) || !isList(variablesNode)) return expr;

  const variables: IRSymbol[] = [];
  for (const variable of variablesNode.args) {
    if (variable.kind !== "symbol") return expr;
    variables.push(variable);
  }

  const rules = solveLinearSystem(equationsNode.args, variables);
  return rules === null ? expr : app(LIST, rules);
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

function isKillAll(input: IRNode): boolean {
  return input.kind === "apply"
    && equals(input.head, KILL)
    && input.args.some((arg) => arg.kind === "symbol" && arg.name === ALL_SYMBOL.name);
}

function integerIndex(value: number): boolean {
  return Number.isSafeInteger(value) && value >= 1;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export { app, sym, toDisplayString };
export type { IRApply, IRNode };
