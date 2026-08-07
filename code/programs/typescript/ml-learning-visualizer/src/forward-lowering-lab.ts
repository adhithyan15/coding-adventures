import {
  addActivation,
  addConstant,
  addInput,
  addOutput,
  addWeightedSum,
  createNeuralGraph,
} from "@coding-adventures/neural-network";
import {
  compileBytecodeToMatrixPlan,
  compileNeuralGraphToBytecode,
  runNeuralBytecodeForwardWithTrace,
  runNeuralMatrixForward,
  type NeuralBytecodeInstruction,
  type NeuralMatrixPlanInstruction,
} from "@coding-adventures/neural-graph-vm";

const IDENTIFIER = /^[A-Za-z][A-Za-z0-9_]{0,63}$/;
const MAX_BATCH = 8;
const MAX_ABSOLUTE_INPUT = 1e3;
const MAX_ABSOLUTE_DERIVED = 1e12;

export type ForwardLoweringScenarioId = "single_row" | "two_row_batch";

export interface ForwardLoweringScenario {
  readonly id: string;
  readonly title: string;
  readonly summary: string;
  readonly inputs: Readonly<Record<"x0" | "x1", readonly number[]>>;
}

export interface LoweredGraphNode {
  readonly id: string;
  readonly op: "input" | "constant" | "weighted_sum" | "activation" | "output";
  readonly detail: string;
}

export interface LoweredGraphEdge {
  readonly id: string;
  readonly from: string;
  readonly to: string;
  readonly weight: number;
}

export interface NormalizedNeuralInstruction {
  readonly id: string;
  readonly op: string;
  readonly output: string | null;
  readonly inputs: readonly string[];
  readonly attributes: Readonly<Record<string, string | number>>;
  readonly sourceNodes: readonly string[];
  readonly sourceEdges: readonly string[];
}

export interface NormalizedMatrixOperation {
  readonly id: string;
  readonly op: string;
  readonly output: string | null;
  readonly inputs: readonly string[];
  readonly attributes: Readonly<Record<string, string | number | readonly string[] | readonly number[]>>;
  readonly sourceInstructions: readonly string[];
  readonly sourceNodes: readonly string[];
  readonly sourceEdges: readonly string[];
}

export interface NeuralInstructionReading {
  readonly instructionId: string;
  readonly reads: readonly { readonly valueId: string; readonly value: number }[];
  readonly write?: { readonly valueId: string; readonly value: number };
  readonly output?: { readonly outputName: string; readonly value: number };
}

export interface ForwardLoweringTrace {
  readonly scenario: ForwardLoweringScenario;
  readonly graph: {
    readonly nodes: readonly LoweredGraphNode[];
    readonly edges: readonly LoweredGraphEdge[];
    readonly topologicalOrder: readonly string[];
  };
  readonly neuralIr: {
    readonly magic: "CANN";
    readonly version: 0;
    readonly instructions: readonly NormalizedNeuralInstruction[];
  };
  readonly matrixIr: {
    readonly magic: "CANM";
    readonly version: 0;
    readonly sourceNeuralIrVersion: 0;
    readonly operations: readonly NormalizedMatrixOperation[];
  };
  readonly directOutputs: readonly number[];
  readonly neuralIrOutputs: readonly number[];
  readonly matrixIrOutputs: readonly number[];
  readonly neuralValueRows: readonly (readonly number[])[];
  readonly matrixValueColumns: readonly {
    readonly valueId: string;
    readonly values: readonly number[];
  }[];
  readonly firstRowInstructionReadings: readonly NeuralInstructionReading[];
  readonly maxParityError: number;
}

export const FORWARD_LOWERING_SCENARIOS: readonly ForwardLoweringScenario[] = [
  {
    id: "single_row",
    title: "One row by hand",
    summary: "Follow 4 and 8 through every graph, NeuralIR, and MatrixIR value.",
    inputs: { x0: [4], x1: [8] },
  },
  {
    id: "two_row_batch",
    title: "The same plan, two rows",
    summary: "Keep the lowered program fixed while its input columns grow to two rows.",
    inputs: { x0: [4, 8], x1: [8, 16] },
  },
] as const;

const GRAPH_NODES: readonly LoweredGraphNode[] = [
  { id: "x0", op: "input", detail: "runtime input x0" },
  { id: "x1", op: "input", detail: "runtime input x1" },
  { id: "bias", op: "constant", detail: "constant 1" },
  { id: "sum", op: "weighted_sum", detail: "three weighted terms" },
  { id: "relu", op: "activation", detail: "max(0, sum)" },
  { id: "out", op: "output", detail: "prediction" },
] as const;

const GRAPH_EDGES: readonly LoweredGraphEdge[] = [
  { id: "w0", from: "x0", to: "sum", weight: 0.25 },
  { id: "w1", from: "x1", to: "sum", weight: 0.75 },
  { id: "bias_to_sum", from: "bias", to: "sum", weight: -1 },
  { id: "sum_to_relu", from: "sum", to: "relu", weight: 1 },
  { id: "relu_to_out", from: "relu", to: "out", weight: 1 },
] as const;

function boundedString(value: unknown, context: string, maxLength = 512): string {
  if (typeof value !== "string" || value.length < 1 || value.length > maxLength) {
    throw new Error(`${context} must be a bounded string`);
  }
  return value;
}

function boundedNumber(value: unknown, context: string): number {
  if (typeof value !== "number" || !Number.isFinite(value) || Math.abs(value) > MAX_ABSOLUTE_INPUT) {
    throw new Error(`${context} must be finite and bounded`);
  }
  return value;
}

function finite(value: number, context: string): number {
  if (!Number.isFinite(value) || Math.abs(value) > MAX_ABSOLUTE_DERIVED) {
    throw new Error(`${context} must remain finite and bounded`);
  }
  return value;
}

function validateScenario(scenario: ForwardLoweringScenario): ForwardLoweringScenario {
  if (typeof scenario !== "object" || scenario === null || Array.isArray(scenario)) {
    throw new Error("forward lowering scenario must be an object");
  }
  const scenarioKeys = Object.keys(scenario).sort();
  if (scenarioKeys.join(",") !== "id,inputs,summary,title") {
    throw new Error("forward lowering scenario has an unexpected field");
  }
  const id = boundedString(scenario.id, "scenario id", 64);
  if (!IDENTIFIER.test(id)) throw new Error("scenario id must be a bounded identifier");
  const title = boundedString(scenario.title, "scenario title");
  const summary = boundedString(scenario.summary, "scenario summary");
  if (typeof scenario.inputs !== "object" || scenario.inputs === null || Array.isArray(scenario.inputs)) {
    throw new Error("scenario inputs must be an object");
  }
  const inputKeys = Object.keys(scenario.inputs).sort();
  if (inputKeys.join(",") !== "x0,x1") {
    throw new Error("scenario inputs must contain exactly x0 and x1");
  }
  const x0 = scenario.inputs.x0;
  const x1 = scenario.inputs.x1;
  if (!Array.isArray(x0) || !Array.isArray(x1)) {
    throw new Error("scenario input columns must be arrays");
  }
  if (x0.length < 1 || x0.length > MAX_BATCH || x1.length !== x0.length) {
    throw new Error("scenario input columns must have the same bounded length");
  }
  const snapshot = {
    id,
    title,
    summary,
    inputs: {
      x0: x0.map((value, index) => boundedNumber(value, `x0[${index}]`)),
      x1: x1.map((value, index) => boundedNumber(value, `x1[${index}]`)),
    },
  };
  return deepFreeze(snapshot);
}

function buildCanonicalGraph() {
  const graph = createNeuralGraph("tiny-weighted-relu");
  addInput(graph, "x0");
  addInput(graph, "x1");
  addConstant(graph, "bias", 1);
  addWeightedSum(graph, "sum", [
    { from: "x0", weight: 0.25, edgeId: "w0" },
    { from: "x1", weight: 0.75, edgeId: "w1" },
    { from: "bias", weight: -1, edgeId: "bias_to_sum" },
  ]);
  addActivation(graph, "relu", "sum", "relu", {}, "sum_to_relu");
  addOutput(graph, "out", "relu", "prediction", {}, "relu_to_out");
  return graph;
}

function normalizedInputs(instruction: NeuralBytecodeInstruction): string[] {
  if (instruction.op === "MUL") return [instruction.left!, instruction.right!];
  if (instruction.op === "ADD") return [...(instruction.inputs ?? [])];
  if (instruction.op === "ACTIVATE" || instruction.op === "STORE_OUTPUT") {
    return [instruction.input!];
  }
  return [];
}

function normalizedAttributes(
  instruction: NeuralBytecodeInstruction,
): Record<string, string | number> {
  switch (instruction.op) {
    case "LOAD_INPUT": return { input_name: instruction.inputName! };
    case "LOAD_CONST": return { value: instruction.value ?? 0 };
    case "LOAD_EDGE_WEIGHT": return { edge_id: instruction.edgeId! };
    case "ACTIVATE": return { activation: instruction.activation ?? "relu" };
    case "STORE_OUTPUT": return { output_name: instruction.outputName ?? "output" };
    default: return {};
  }
}

function normalizeNeuralInstructions(
  instructions: readonly NeuralBytecodeInstruction[],
): NormalizedNeuralInstruction[] {
  return instructions.map((instruction, index) => ({
    id: `i${index}`,
    op: instruction.op,
    output: instruction.dst ?? null,
    inputs: normalizedInputs(instruction),
    attributes: normalizedAttributes(instruction),
    sourceNodes: instruction.sourceNode === undefined ? [] : [instruction.sourceNode],
    sourceEdges: instruction.sourceEdge === undefined ? [] : [instruction.sourceEdge],
  }));
}

function weightedSourceInstructions(
  operation: NeuralMatrixPlanInstruction,
  neural: readonly NormalizedNeuralInstruction[],
): string[] {
  const edgeIds = new Set((operation.terms ?? []).map((term) => term.edgeId));
  return neural
    .filter((instruction) => (
      instruction.sourceEdges.some((edgeId) => edgeIds.has(edgeId))
      || (instruction.op === "ADD" && instruction.output === operation.dst)
    ))
    .map((instruction) => instruction.id);
}

function normalizeMatrixOperations(
  operations: readonly NeuralMatrixPlanInstruction[],
  neural: readonly NormalizedNeuralInstruction[],
): NormalizedMatrixOperation[] {
  return operations.map((operation, index) => {
    const weighted = operation.op === "WEIGHTED_SUM_MATRIX";
    const terms = operation.terms ?? [];
    const inputs = weighted
      ? terms.map((term) => term.sourceValue)
      : operation.input === undefined ? [] : [operation.input];
    const attributes: Record<string, string | number | readonly string[] | readonly number[]> = {};
    if (operation.op === "LOAD_INPUT_MATRIX") attributes.input_name = operation.inputName!;
    if (operation.op === "LOAD_CONST_MATRIX") attributes.value = operation.value ?? 0;
    if (weighted) {
      attributes.edge_ids = terms.map((term) => term.edgeId);
      attributes.weights = terms.map((term) => term.weight);
    }
    if (operation.op === "ACTIVATE_MATRIX") attributes.activation = operation.activation ?? "relu";
    if (operation.op === "STORE_OUTPUT_MATRIX") attributes.output_name = operation.outputName ?? "output";
    return {
      id: `m${index}`,
      op: operation.op,
      output: operation.dst ?? null,
      inputs,
      attributes,
      sourceInstructions: weighted
        ? weightedSourceInstructions(operation, neural)
        : operation.sourceInstructionIndexes.map((sourceIndex) => `i${sourceIndex}`),
      sourceNodes: operation.sourceNode === undefined ? [] : [operation.sourceNode],
      sourceEdges: weighted ? terms.map((term) => term.edgeId) : [],
    };
  });
}

function directOutputs(scenario: ForwardLoweringScenario): number[] {
  return scenario.inputs.x0.map((x0, index) => {
    const weighted = finite(
      finite(1 * -1, "bias term")
      + finite(x0 * 0.25, "x0 term")
      + finite(scenario.inputs.x1[index]! * 0.75, "x1 term"),
      `direct row ${index}`,
    );
    return Math.max(0, weighted);
  });
}

function maxDifference(groups: readonly (readonly number[])[]): number {
  let maximum = 0;
  for (let row = 0; row < groups[0]!.length; row += 1) {
    for (let left = 0; left < groups.length; left += 1) {
      for (let right = left + 1; right < groups.length; right += 1) {
        maximum = Math.max(maximum, Math.abs(groups[left]![row]! - groups[right]![row]!));
      }
    }
  }
  return finite(maximum, "parity error");
}

export function traceForwardLoweringProgram(
  scenario: ForwardLoweringScenario,
): ForwardLoweringTrace {
  const snapshot = validateScenario(scenario);
  const graph = buildCanonicalGraph();
  const bytecode = compileNeuralGraphToBytecode(graph);
  const forward = bytecode.functions[0]!;
  const matrixPlan = compileBytecodeToMatrixPlan(bytecode);
  const neuralInstructions = normalizeNeuralInstructions(forward.instructions);
  const matrixOperations = normalizeMatrixOperations(matrixPlan.instructions, neuralInstructions);

  const neuralRows = snapshot.inputs.x0.map((x0, rowIndex) => (
    runNeuralBytecodeForwardWithTrace(bytecode, {
      x0,
      x1: snapshot.inputs.x1[rowIndex]!,
    })
  ));
  const neuralOutputs = neuralRows.map((row) => finite(row.outputs.prediction!, "NeuralIR output"));
  const neuralValueRows = neuralRows.map((row) => (
    Object.values(row.values).map((value) => finite(value, "NeuralIR value"))
  ));
  const matrixResult = runNeuralMatrixForward(matrixPlan, snapshot.inputs);
  const matrixOutputs = (matrixResult.outputs.prediction ?? [])
    .map((value) => finite(value, "MatrixIR output"));
  const matrixValueColumns = Object.entries(matrixResult.values).map(([valueId, values]) => ({
    valueId,
    values: values.map((value) => finite(value, `MatrixIR ${valueId}`)),
  }));
  const direct = directOutputs(snapshot);
  const maxParityError = maxDifference([direct, neuralOutputs, matrixOutputs]);
  const firstRow = neuralRows[0]!;
  const firstRowInstructionReadings = firstRow.instructions.map((reading, index) => ({
    instructionId: `i${index}`,
    reads: reading.reads.map((item) => ({ ...item })),
    write: reading.write === undefined ? undefined : { ...reading.write },
    output: reading.output === undefined ? undefined : { ...reading.output },
  }));

  return deepFreeze({
    scenario: snapshot,
    graph: {
      nodes: GRAPH_NODES.map((node) => ({ ...node })),
      edges: GRAPH_EDGES.map((edge) => ({ ...edge })),
      topologicalOrder: ["bias", "x0", "x1", "sum", "relu", "out"],
    },
    neuralIr: {
      magic: "CANN" as const,
      version: 0 as const,
      instructions: neuralInstructions,
    },
    matrixIr: {
      magic: "CANM" as const,
      version: 0 as const,
      sourceNeuralIrVersion: 0 as const,
      operations: matrixOperations,
    },
    directOutputs: direct,
    neuralIrOutputs: neuralOutputs,
    matrixIrOutputs: matrixOutputs,
    neuralValueRows,
    matrixValueColumns,
    firstRowInstructionReadings,
    maxParityError,
  });
}

export function traceForwardLowering(id: ForwardLoweringScenarioId): ForwardLoweringTrace {
  const scenario = FORWARD_LOWERING_SCENARIOS.find((candidate) => candidate.id === id);
  if (scenario === undefined) throw new Error(`unknown forward lowering scenario: ${id}`);
  return traceForwardLoweringProgram(scenario);
}

function deepFreeze<T>(value: T): T {
  if (typeof value !== "object" || value === null || Object.isFrozen(value)) return value;
  Object.freeze(value);
  Object.values(value).forEach((child) => deepFreeze(child));
  return value;
}
