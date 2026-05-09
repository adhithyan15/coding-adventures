export type WasmInitInput = string | URL | Request | Response | BufferSource | WebAssembly.Module;

export interface WasmMacsymaSessionBinding {
  eval(source: string): string;
  historyJson(): string;
  resetHistory(): void;
  free?(): void;
}

export interface MacsymaWasmGeneratedModule {
  default?: (input?: WasmInitInput) => Promise<unknown> | unknown;
  initSync?: (module?: WebAssembly.Module | BufferSource) => unknown;
  WasmMacsymaSession: new () => WasmMacsymaSessionBinding;
  evalSource(source: string): string;
}

export type MacsymaWasmModuleLoader = () => Promise<MacsymaWasmGeneratedModule> | MacsymaWasmGeneratedModule;

export interface MacsymaWasmLoadOptions {
  readonly initInput?: WasmInitInput;
  readonly skipDefaultInit?: boolean;
}

export type JsonIrNode =
  | { readonly kind: "symbol"; readonly name: string }
  | { readonly kind: "integer"; readonly value: number }
  | { readonly kind: "rational"; readonly numerator: number; readonly denominator: number }
  | { readonly kind: "float"; readonly value: number }
  | { readonly kind: "string"; readonly value: string }
  | { readonly kind: "apply"; readonly head: JsonIrNode; readonly args: readonly JsonIrNode[] };

export interface MacsymaWasmEvalResult {
  readonly inputIndex: number;
  readonly outputIndex: number;
  readonly display: boolean;
  readonly inputMacsyma: string;
  readonly outputMacsyma: string;
  readonly inputLisp: string;
  readonly outputLisp: string;
  readonly inputIr: JsonIrNode;
  readonly outputIr: JsonIrNode;
}

export interface MacsymaWasmHistory {
  readonly inputCount: number;
  readonly outputCount: number;
  readonly nextInputIndex: number;
  readonly lastOutputMacsyma: string | null;
  readonly lastOutputLisp: string | null;
}

export interface MacsymaWasmError {
  readonly kind: string;
  readonly message: string;
}

export interface MacsymaWasmEvalResponse {
  readonly ok: boolean;
  readonly results: readonly MacsymaWasmEvalResult[];
  readonly visibleOutputs: readonly string[];
  readonly history: MacsymaWasmHistory;
  readonly error?: MacsymaWasmError;
}

interface RawEvalResponse {
  readonly ok?: unknown;
  readonly results?: readonly RawEvalResult[];
  readonly visible_outputs?: readonly string[];
  readonly history?: RawHistory;
  readonly error?: RawError;
}

interface RawEvalResult {
  readonly input_index?: unknown;
  readonly output_index?: unknown;
  readonly display?: unknown;
  readonly input_macsyma?: unknown;
  readonly output_macsyma?: unknown;
  readonly input_lisp?: unknown;
  readonly output_lisp?: unknown;
  readonly input_ir?: unknown;
  readonly output_ir?: unknown;
}

interface RawHistory {
  readonly input_count?: unknown;
  readonly output_count?: unknown;
  readonly next_input_index?: unknown;
  readonly last_output_macsyma?: unknown;
  readonly last_output_lisp?: unknown;
}

interface RawError {
  readonly kind?: unknown;
  readonly message?: unknown;
}

export class MacsymaWasmRuntime {
  constructor(
    private readonly module: MacsymaWasmGeneratedModule,
    private readonly session: WasmMacsymaSessionBinding = new module.WasmMacsymaSession(),
  ) {}

  eval(source: string): MacsymaWasmEvalResponse {
    return parseEvalResponse(this.session.eval(source));
  }

  evalFresh(source: string): MacsymaWasmEvalResponse {
    return parseEvalResponse(this.module.evalSource(source));
  }

  history(): MacsymaWasmHistory {
    return parseHistoryResponse(this.session.historyJson());
  }

  resetHistory(): void {
    this.session.resetHistory();
  }

  dispose(): void {
    this.session.free?.();
  }
}

export async function loadMacsymaWasmRuntime(
  loadModule: MacsymaWasmModuleLoader,
  options: MacsymaWasmLoadOptions = {},
): Promise<MacsymaWasmRuntime> {
  const module = await loadModule();
  if (!options.skipDefaultInit && module.default !== undefined) {
    await module.default(options.initInput);
  }
  return new MacsymaWasmRuntime(module);
}

export function parseEvalResponse(json: string): MacsymaWasmEvalResponse {
  const raw = parseObject<RawEvalResponse>(json, "MACSYMA WASM eval response");
  const ok = expectBoolean(raw.ok, "ok");
  const results = expectArray(raw.results, "results").map((value, index) =>
    normalizeEvalResult(expectObject<RawEvalResult>(value, `results[${index}]`), index),
  );
  const visibleOutputs = expectArray(raw.visible_outputs, "visible_outputs").map((value, index) =>
    expectString(value, `visible_outputs[${index}]`),
  );
  const history = normalizeHistory(raw.history);
  const error = raw.error === undefined ? undefined : normalizeError(raw.error);
  return { ok, results, visibleOutputs, history, ...(error === undefined ? {} : { error }) };
}

function parseHistoryResponse(json: string): MacsymaWasmHistory {
  const raw = parseObject<{ readonly ok?: unknown; readonly history?: RawHistory }>(json, "MACSYMA WASM history response");
  if (!expectBoolean(raw.ok, "ok")) {
    throw new Error("MACSYMA WASM history response was not ok");
  }
  return normalizeHistory(raw.history);
}

function normalizeEvalResult(raw: RawEvalResult, index: number): MacsymaWasmEvalResult {
  return {
    inputIndex: expectNumber(raw.input_index, `results[${index}].input_index`),
    outputIndex: expectNumber(raw.output_index, `results[${index}].output_index`),
    display: expectBoolean(raw.display, `results[${index}].display`),
    inputMacsyma: expectString(raw.input_macsyma, `results[${index}].input_macsyma`),
    outputMacsyma: expectString(raw.output_macsyma, `results[${index}].output_macsyma`),
    inputLisp: expectString(raw.input_lisp, `results[${index}].input_lisp`),
    outputLisp: expectString(raw.output_lisp, `results[${index}].output_lisp`),
    inputIr: normalizeIr(raw.input_ir, `results[${index}].input_ir`),
    outputIr: normalizeIr(raw.output_ir, `results[${index}].output_ir`),
  };
}

function normalizeHistory(raw: RawHistory | undefined): MacsymaWasmHistory {
  if (raw === undefined || raw === null || typeof raw !== "object") {
    throw new Error("Expected history to be an object");
  }
  return {
    inputCount: expectNumber(raw.input_count, "history.input_count"),
    outputCount: expectNumber(raw.output_count, "history.output_count"),
    nextInputIndex: expectNumber(raw.next_input_index, "history.next_input_index"),
    lastOutputMacsyma: expectNullableString(raw.last_output_macsyma, "history.last_output_macsyma"),
    lastOutputLisp: expectNullableString(raw.last_output_lisp, "history.last_output_lisp"),
  };
}

function normalizeError(raw: RawError): MacsymaWasmError {
  return {
    kind: expectString(raw.kind, "error.kind"),
    message: expectString(raw.message, "error.message"),
  };
}

function normalizeIr(value: unknown, path: string): JsonIrNode {
  if (value === null || typeof value !== "object") throw new Error(`Expected ${path} to be an object`);
  const node = value as Record<string, unknown>;
  const kind = expectString(node.kind, `${path}.kind`);
  switch (kind) {
    case "symbol":
      return { kind, name: expectString(node.name, `${path}.name`) };
    case "integer":
      return { kind, value: expectNumber(node.value, `${path}.value`) };
    case "rational":
      return {
        kind,
        numerator: expectNumber(node.numerator, `${path}.numerator`),
        denominator: expectNumber(node.denominator, `${path}.denominator`),
      };
    case "float":
      return { kind, value: expectNumber(node.value, `${path}.value`) };
    case "string":
      return { kind, value: expectString(node.value, `${path}.value`) };
    case "apply":
      return {
        kind,
        head: normalizeIr(node.head, `${path}.head`),
        args: expectArray(node.args, `${path}.args`).map((arg, index) => normalizeIr(arg, `${path}.args[${index}]`)),
      };
    default:
      throw new Error(`Unknown ${path}.kind: ${kind}`);
  }
}

function parseObject<T>(json: string, label: string): T {
  const parsed: unknown = JSON.parse(json);
  return expectObject<T>(parsed, label);
}

function expectObject<T>(parsed: unknown, label: string): T {
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error(`Expected ${label} to be a JSON object`);
  }
  return parsed as T;
}

function expectArray(value: unknown, path: string): readonly unknown[] {
  if (!Array.isArray(value)) throw new Error(`Expected ${path} to be an array`);
  return value;
}

function expectBoolean(value: unknown, path: string): boolean {
  if (typeof value !== "boolean") throw new Error(`Expected ${path} to be a boolean`);
  return value;
}

function expectNumber(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) throw new Error(`Expected ${path} to be a finite number`);
  return value;
}

function expectString(value: unknown, path: string): string {
  if (typeof value !== "string") throw new Error(`Expected ${path} to be a string`);
  return value;
}

function expectNullableString(value: unknown, path: string): string | null {
  if (value === null) return null;
  return expectString(value, path);
}
