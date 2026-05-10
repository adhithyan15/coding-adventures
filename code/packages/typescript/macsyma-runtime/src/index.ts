import { DISPLAY, SUPPRESS, compileMacsyma } from "@coding-adventures/macsyma-compiler";
import {
  app,
  equals,
  sym,
  toDisplayString,
  type IRApply,
  type IRNode,
} from "@coding-adventures/symbolic-ir";
import { SymbolicBackend, VM } from "@coding-adventures/symbolic-vm";

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
}

export class MacsymaBackend extends SymbolicBackend {
  constructor(private readonly history: History) {
    super();
    this.bind("%pi", { kind: "float", value: Math.PI });
    this.bind("%e", { kind: "float", value: Math.E });
    this.bind("%i", sym("ImaginaryUnit"));
  }

  override lookup(name: string): IRNode | undefined {
    if (name === "%") return this.history.lastOutput();
    const inputMatch = /^%i([1-9]\d*)$/.exec(name);
    if (inputMatch !== null) return this.history.getInput(Number(inputMatch[1]));
    const outputMatch = /^%o([1-9]\d*)$/.exec(name);
    if (outputMatch !== null) return this.history.getOutput(Number(outputMatch[1]));
    return super.lookup(name);
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
    return statements.map((statement) => this.evalStatement(statement));
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

function integerIndex(value: number): boolean {
  return Number.isSafeInteger(value) && value >= 1;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export { app, sym, toDisplayString };
export type { IRApply, IRNode };
