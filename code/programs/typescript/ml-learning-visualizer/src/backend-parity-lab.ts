import {
  addConstant,
  addInput,
  addOutput,
  addWeightedSum,
  createNeuralGraph,
} from "@coding-adventures/neural-network";
import {
  WebGpuMatrixBackend,
  compileBytecodeToMatrixPlan,
  compileNeuralGraphToBytecode,
  runNeuralBytecodeForwardWithTrace,
  runNeuralMatrixForward,
  runNeuralMatrixForwardAsync,
  type AsyncNeuralMatrixBackend,
  type NeuralMatrixPlan,
} from "@coding-adventures/neural-graph-vm";
import fixtureDocument from "../../../../specs/fixtures/backend-parity-v1/labs/00-dense-batch.json";

const IDENTIFIER = /^[a-z][a-z0-9_]{0,63}$/;
const MAX_TEXT = 512;
const MAX_ABSOLUTE_NUMBER = 1e6;
const CANONICAL_LANE_IDS = [
  "scalar_cpu",
  "typescript_matrix_cpu",
  "rust_matrix_cpu",
  "webgpu_accelerated",
] as const;

export type BackendParityLaneId = typeof CANONICAL_LANE_IDS[number];
export type BackendEvidence =
  | "executed-production"
  | "validated-native-fixture"
  | "deterministic-oracle";

export interface BackendParityLane {
  readonly id: BackendParityLaneId;
  readonly title: string;
  readonly runtime: string;
  readonly precision: "binary64" | "f32";
  readonly availability: "required" | "required-in-native-test" | "optional-runtime-probe";
  readonly steps: readonly string[];
  readonly residency: readonly string[];
  readonly expectedOutputs: readonly number[];
}

export interface BackendParityFixture {
  readonly id: "dense-backend-parity";
  readonly title: string;
  readonly question: string;
  readonly absoluteTolerance: number;
  readonly graph: {
    readonly equation: "y = XW + B";
    readonly dtype: "f32";
    readonly weight: number;
    readonly bias: readonly number[];
    readonly shapes: {
      readonly input: readonly number[];
      readonly weight: readonly number[];
      readonly bias: readonly number[];
      readonly output: readonly number[];
    };
  };
  readonly scenario: {
    readonly id: "three_row_dense";
    readonly inputs: readonly number[];
    readonly products: readonly number[];
    readonly outputs: readonly number[];
  };
  readonly lanes: readonly BackendParityLane[];
}

export interface BackendLaneTrace extends BackendParityLane {
  readonly outputs: readonly number[];
  readonly maxAbsoluteError: number;
  readonly evidence: BackendEvidence;
}

export interface BackendParityTrace {
  readonly fixture: BackendParityFixture;
  readonly products: readonly number[];
  readonly scalarInstructionCount: number;
  readonly matrixOperationCount: number;
  readonly lanes: readonly BackendLaneTrace[];
  readonly maxAbsoluteError: number;
}

export type AcceleratorProbeResult =
  | { readonly status: "executed"; readonly outputs: readonly number[]; readonly maxAbsoluteError: number; readonly withinTolerance: boolean; readonly message: string }
  | { readonly status: "unavailable" | "failed"; readonly message: string };

function object(value: unknown, keys: readonly string[], context: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`${context} must be an object`);
  }
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  if (actual.join(",") !== expected.join(",")) {
    throw new Error(`${context} has unexpected fields`);
  }
  return value as Record<string, unknown>;
}

function text(value: unknown, context: string): string {
  if (typeof value !== "string" || value.length < 1 || value.length > MAX_TEXT) {
    throw new Error(`${context} must be bounded text`);
  }
  return value;
}

function number(value: unknown, context: string): number {
  if (
    typeof value !== "number"
    || !Number.isFinite(value)
    || Math.abs(value) > MAX_ABSOLUTE_NUMBER
  ) {
    throw new Error(`${context} must be finite and bounded`);
  }
  return value;
}

function numbers(value: unknown, length: number, context: string): number[] {
  if (!Array.isArray(value) || value.length !== length) {
    throw new Error(`${context} must contain exactly ${length} numbers`);
  }
  return value.map((item, index) => number(item, `${context}[${index}]`));
}

function strings(value: unknown, minimum: number, maximum: number, context: string): string[] {
  if (!Array.isArray(value) || value.length < minimum || value.length > maximum) {
    throw new Error(`${context} has an invalid length`);
  }
  return value.map((item, index) => text(item, `${context}[${index}]`));
}

export function normalizeBackendParityFixture(value: unknown): BackendParityFixture {
  const lab = object(
    value,
    ["schema_version", "id", "title", "question", "absolute_tolerance", "graph", "scenario", "lanes"],
    "backend parity fixture",
  );
  if (lab.schema_version !== 1 || lab.id !== "dense-backend-parity") {
    throw new Error("backend parity fixture identity is not canonical");
  }
  const tolerance = number(lab.absolute_tolerance, "absolute tolerance");
  if (tolerance !== 1e-6) throw new Error("backend parity tolerance is not canonical");

  const graph = object(
    lab.graph,
    ["equation", "dtype", "input_shape", "weight_shape", "bias_shape", "output_shape", "weight", "bias", "matrix_ir_file"],
    "backend parity graph",
  );
  if (
    graph.equation !== "y = XW + B"
    || graph.dtype !== "f32"
    || graph.matrix_ir_file !== "../matrix-ir/00-dense-batch.graph.json"
  ) {
    throw new Error("backend parity graph contract is not canonical");
  }
  const weightValues = numbers(graph.weight, 1, "graph weight");
  const bias = numbers(graph.bias, 3, "graph bias");
  const shapes = {
    input: numbers(graph.input_shape, 2, "input shape"),
    weight: numbers(graph.weight_shape, 2, "weight shape"),
    bias: numbers(graph.bias_shape, 2, "bias shape"),
    output: numbers(graph.output_shape, 2, "output shape"),
  };
  if (
    weightValues[0] !== 2
    || bias.join(",") !== "1,1,1"
    || shapes.input.join(",") !== "3,1"
    || shapes.weight.join(",") !== "1,1"
    || shapes.bias.join(",") !== "3,1"
    || shapes.output.join(",") !== "3,1"
  ) {
    throw new Error("backend parity dense values and shapes are not canonical");
  }

  const scenario = object(
    lab.scenario,
    ["id", "inputs", "input_payload_file", "expected_payload_file", "expected"],
    "backend parity scenario",
  );
  if (
    scenario.id !== "three_row_dense"
    || scenario.input_payload_file !== "../payloads/00-input-x.f32le.hex"
    || scenario.expected_payload_file !== "../payloads/00-expected-output.f32le.hex"
  ) {
    throw new Error("backend parity scenario contract is not canonical");
  }
  const expected = object(scenario.expected, ["products", "outputs"], "scenario expected");
  const inputs = numbers(scenario.inputs, 3, "scenario inputs");
  const products = numbers(expected.products, 3, "scenario products");
  const outputs = numbers(expected.outputs, 3, "scenario outputs");
  if (inputs.join(",") !== "1,2,3" || products.join(",") !== "2,4,6" || outputs.join(",") !== "3,5,7") {
    throw new Error("backend parity scenario values are not canonical");
  }

  if (!Array.isArray(lab.lanes) || lab.lanes.length !== 4) {
    throw new Error("backend parity fixture must contain four lanes");
  }
  const lanes = lab.lanes.map((rawLane, index): BackendParityLane => {
    const lane = object(
      rawLane,
      ["id", "title", "runtime", "precision", "availability", "steps", "residency", "expected_outputs"],
      `backend lane ${index}`,
    );
    const id = text(lane.id, `backend lane ${index} id`);
    if (!IDENTIFIER.test(id) || id !== CANONICAL_LANE_IDS[index]) {
      throw new Error("backend parity lane roster is not canonical");
    }
    const precision = lane.precision;
    if (precision !== "binary64" && precision !== "f32") {
      throw new Error(`backend lane ${index} precision is invalid`);
    }
    const availability = lane.availability;
    if (
      availability !== "required"
      && availability !== "required-in-native-test"
      && availability !== "optional-runtime-probe"
    ) {
      throw new Error(`backend lane ${index} availability is invalid`);
    }
    const laneOutputs = numbers(lane.expected_outputs, 3, `backend lane ${index} outputs`);
    if (laneOutputs.join(",") !== outputs.join(",")) {
      throw new Error(`backend lane ${index} output oracle is dishonest`);
    }
    return {
      id: id as BackendParityLaneId,
      title: text(lane.title, `backend lane ${index} title`),
      runtime: text(lane.runtime, `backend lane ${index} runtime`),
      precision,
      availability,
      steps: strings(lane.steps, 4, 4, `backend lane ${index} steps`),
      residency: strings(lane.residency, 3, 4, `backend lane ${index} residency`),
      expectedOutputs: laneOutputs,
    };
  });

  return deepFreeze({
    id: "dense-backend-parity" as const,
    title: text(lab.title, "fixture title"),
    question: text(lab.question, "fixture question"),
    absoluteTolerance: tolerance,
    graph: {
      equation: "y = XW + B" as const,
      dtype: "f32" as const,
      weight: weightValues[0]!,
      bias,
      shapes,
    },
    scenario: {
      id: "three_row_dense" as const,
      inputs,
      products,
      outputs,
    },
    lanes,
  });
}

export const BACKEND_PARITY_FIXTURE = normalizeBackendParityFixture(fixtureDocument);

function buildGraph(fixture: BackendParityFixture = BACKEND_PARITY_FIXTURE) {
  const graph = createNeuralGraph("backend-parity-dense");
  addInput(graph, "x");
  addConstant(graph, "bias", fixture.graph.bias[0]!);
  addWeightedSum(graph, "dense", [
    { from: "x", weight: fixture.graph.weight, edgeId: "weight" },
    { from: "bias", weight: 1, edgeId: "bias" },
  ]);
  addOutput(graph, "output", "dense", "y", {}, "dense_to_output");
  return graph;
}

function compilePlan(): { readonly bytecode: ReturnType<typeof compileNeuralGraphToBytecode>; readonly plan: NeuralMatrixPlan } {
  const bytecode = compileNeuralGraphToBytecode(buildGraph());
  return { bytecode, plan: compileBytecodeToMatrixPlan(bytecode) };
}

function finiteValues(values: readonly number[], context: string): number[] {
  return values.map((value, index) => {
    if (!Number.isFinite(value) || Math.abs(value) > MAX_ABSOLUTE_NUMBER) {
      throw new Error(`${context}[${index}] is not finite and bounded`);
    }
    return value;
  });
}

function maxError(actual: readonly number[], expected: readonly number[]): number {
  return Math.max(...actual.map((value, index) => Math.abs(value - expected[index]!)));
}

function laneEvidence(id: BackendParityLaneId): BackendEvidence {
  if (id === "rust_matrix_cpu") return "validated-native-fixture";
  if (id === "webgpu_accelerated") return "deterministic-oracle";
  return "executed-production";
}

export function traceBackendParity(): BackendParityTrace {
  const fixture = BACKEND_PARITY_FIXTURE;
  const { bytecode, plan } = compilePlan();
  const scalarOutputs = fixture.scenario.inputs.map((input) => (
    runNeuralBytecodeForwardWithTrace(bytecode, { x: input }).outputs.y!
  ));
  const matrixOutputs = runNeuralMatrixForward(plan, { x: fixture.scenario.inputs }).outputs.y ?? [];
  const outputsByLane: Record<BackendParityLaneId, readonly number[]> = {
    scalar_cpu: finiteValues(scalarOutputs, "scalar outputs"),
    typescript_matrix_cpu: finiteValues(matrixOutputs, "matrix outputs"),
    rust_matrix_cpu: fixture.scenario.outputs,
    webgpu_accelerated: fixture.scenario.outputs.map((value) => Math.fround(value)),
  };
  const lanes = fixture.lanes.map((lane): BackendLaneTrace => {
    const outputs = outputsByLane[lane.id];
    return {
      ...lane,
      outputs,
      maxAbsoluteError: maxError(outputs, fixture.scenario.outputs),
      evidence: laneEvidence(lane.id),
    };
  });
  return deepFreeze({
    fixture,
    products: fixture.scenario.inputs.map((value) => value * fixture.graph.weight),
    scalarInstructionCount: bytecode.functions[0]?.instructions.length ?? 0,
    matrixOperationCount: plan.instructions.length,
    lanes,
    maxAbsoluteError: Math.max(...lanes.map((lane) => lane.maxAbsoluteError)),
  });
}

export async function runBackendParityWithAsyncBackend<M>(
  backend: AsyncNeuralMatrixBackend<M>,
): Promise<AcceleratorProbeResult> {
  const { plan } = compilePlan();
  const result = await runNeuralMatrixForwardAsync(
    plan,
    { x: BACKEND_PARITY_FIXTURE.scenario.inputs },
    backend,
  );
  const outputs = finiteValues(result.outputs.y ?? [], "accelerated outputs");
  if (outputs.length !== BACKEND_PARITY_FIXTURE.scenario.outputs.length) {
    throw new Error("accelerated backend returned the wrong output shape");
  }
  const maxAbsoluteError = maxError(outputs, BACKEND_PARITY_FIXTURE.scenario.outputs);
  const withinTolerance = maxAbsoluteError <= BACKEND_PARITY_FIXTURE.absoluteTolerance;
  return {
    status: "executed",
    outputs,
    maxAbsoluteError,
    withinTolerance,
    message: withinTolerance
      ? "The async backend executed the production matrix plan and matched the oracle."
      : "The async backend executed the production matrix plan but missed the tolerance.",
  };
}

export async function probeWebGpuBackendParity(): Promise<AcceleratorProbeResult> {
  if (!WebGpuMatrixBackend.isNavigatorAvailable()) {
    return { status: "unavailable", message: "This browser does not expose WebGPU." };
  }
  let backend: WebGpuMatrixBackend | null = null;
  try {
    backend = await WebGpuMatrixBackend.createFromNavigator({
      powerPreference: "high-performance",
    });
    if (backend === null) {
      return { status: "unavailable", message: "No WebGPU adapter was available." };
    }
    return await runBackendParityWithAsyncBackend(backend);
  } catch (error) {
    const message = error instanceof Error ? error.message : "WebGPU execution failed";
    return { status: "failed", message: message.slice(0, 256) };
  } finally {
    backend?.destroy();
  }
}

function deepFreeze<T>(value: T): T {
  if (typeof value !== "object" || value === null || Object.isFrozen(value)) return value;
  Object.freeze(value);
  Object.values(value).forEach((child) => deepFreeze(child));
  return value;
}
