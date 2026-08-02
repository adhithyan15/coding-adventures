import {
  addActivation,
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
} from "@coding-adventures/neural-graph-vm";

const MAX_BATCH = 8;
const MAX_TEXT = 512;
const MAX_ABSOLUTE_INPUT = 1e3;
const MAX_ABSOLUTE_DERIVED = 1e12;
const EPSILON = 1e-5;

type AttributeValue = string | number | boolean | readonly string[] | readonly number[];

export interface TrainingLoweringInstruction {
  readonly id: string;
  readonly op: string;
  readonly output: string;
  readonly inputs: readonly string[];
  readonly attributes: Readonly<Record<string, AttributeValue>>;
  readonly sourceNodes: readonly string[];
  readonly sourceEdges: readonly string[];
  readonly sourceInstructions: readonly string[];
}

export interface TrainingLoweringStream {
  readonly magic: "CANB" | "CANO" | "CANM-TRAIN";
  readonly version: 0;
  readonly instructions: readonly TrainingLoweringInstruction[];
}

export interface BackwardOptimizerLoweringScenario {
  readonly id: string;
  readonly title: string;
  readonly summary: string;
  readonly initialParameter: number;
  readonly learningRate: number;
  readonly inputs: readonly number[];
  readonly targets: readonly number[];
  readonly gradientBufferBefore: number;
  readonly divisor: number;
}

export type BackwardOptimizerLoweringScenarioId =
  | "one_row_by_hand"
  | "two_row_mean"
  | "persistent_buffer";

export const BACKWARD_OPTIMIZER_LOWERING_SCENARIOS: readonly BackwardOptimizerLoweringScenario[] = [
  {
    id: "one_row_by_hand",
    title: "One row by hand",
    summary: "Save one forward row, reverse it, then apply one scalar SGD update.",
    initialParameter: 0.5,
    learningRate: 0.1,
    inputs: [2],
    targets: [0],
    gradientBufferBefore: 0,
    divisor: 1,
  },
  {
    id: "two_row_mean",
    title: "The same plan, two-row mean",
    summary: "Keep every instruction ID fixed while two row gradients reduce and average.",
    initialParameter: 1,
    learningRate: 0.1,
    inputs: [2, -1],
    targets: [1, 1],
    gradientBufferBefore: 0,
    divisor: 2,
  },
  {
    id: "persistent_buffer",
    title: "Continue a persistent buffer",
    summary: "Enter with grad_w = 3, add one new row gradient of 2, and keep 5 after SGD.",
    initialParameter: 0.5,
    learningRate: 0.1,
    inputs: [2],
    targets: [0],
    gradientBufferBefore: 3,
    divisor: 1,
  },
];

export interface SavedTrainingValues {
  readonly x: readonly number[];
  readonly target: readonly number[];
  readonly prediction: readonly number[];
  readonly residual: readonly number[];
  readonly loss: readonly number[];
}

export interface BackwardTrainingValues {
  readonly dLoss: readonly number[];
  readonly dResidual: readonly number[];
  readonly dPrediction: readonly number[];
  readonly localDW: readonly number[];
  readonly dX: readonly number[];
  readonly gradientBufferBefore: number;
  readonly batchGradient: number;
  readonly gradW: number;
}

export interface OptimizerTrainingValues {
  readonly parameterBefore: number;
  readonly appliedGradient: number;
  readonly parameterDelta: number;
  readonly parameterAfter: number;
  readonly gradientBufferAfterStep: number;
}

export interface MatrixTrainingValues {
  readonly columns: {
    readonly x: readonly number[];
    readonly residual: readonly number[];
    readonly dPrediction: readonly number[];
    readonly localDW: readonly number[];
    readonly dX: readonly number[];
  };
  readonly gradientBufferBefore: number;
  readonly batchGradient: number;
  readonly gradW: number;
  readonly appliedGradient: number;
  readonly parameterAfter: number;
  readonly gradientBufferAfterStep: number;
}

export interface BackwardOptimizerLoweringTrace {
  readonly scenario: BackwardOptimizerLoweringScenario;
  readonly forward: {
    readonly directOutputs: readonly number[];
    readonly neuralIrOutputs: readonly number[];
    readonly matrixIrOutputs: readonly number[];
    readonly neuralOps: readonly string[];
    readonly matrixOps: readonly string[];
    readonly maxError: number;
  };
  readonly savedValues: SavedTrainingValues;
  readonly backwardIr: TrainingLoweringStream;
  readonly optimizerIr: TrainingLoweringStream;
  readonly matrixTrainingIr: TrainingLoweringStream;
  readonly backward: BackwardTrainingValues;
  readonly optimizer: OptimizerTrainingValues;
  readonly matrixTraining: MatrixTrainingValues;
  readonly gradientAudit: {
    readonly analytical: number;
    readonly numerical: number;
    readonly absoluteError: number;
  };
  readonly maxPathError: number;
}

function finite(value: number, context: string, derived = true): number {
  const limit = derived ? MAX_ABSOLUTE_DERIVED : MAX_ABSOLUTE_INPUT;
  if (!Number.isFinite(value) || Math.abs(value) > limit) {
    throw new Error(`${context} must be finite and bounded by ${limit}`);
  }
  return value;
}

function boundedString(value: unknown, context: string): string {
  if (typeof value !== "string" || value.length < 1 || value.length > MAX_TEXT) {
    throw new Error(`${context} must be a bounded string`);
  }
  return value;
}

function exactKeys(value: object, expected: readonly string[], context: string): void {
  const keys = Object.keys(value).sort();
  const wanted = [...expected].sort();
  if (keys.length !== wanted.length || keys.some((key, index) => key !== wanted[index])) {
    throw new Error(`${context} must contain exactly ${wanted.join(", ")}`);
  }
}

function boundedColumn(value: unknown, context: string): number[] {
  if (!Array.isArray(value) || value.length < 1 || value.length > MAX_BATCH) {
    throw new Error(`${context} must contain 1 to ${MAX_BATCH} values`);
  }
  return value.map((item, index) => {
    if (typeof item !== "number") throw new Error(`${context}[${index}] must be numeric`);
    return finite(item, `${context}[${index}]`, false);
  });
}

function validateScenario(
  value: BackwardOptimizerLoweringScenario,
): BackwardOptimizerLoweringScenario {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error("scenario must be an object");
  }
  exactKeys(
    value,
    ["id", "title", "summary", "initialParameter", "learningRate", "inputs", "targets", "gradientBufferBefore", "divisor"],
    "scenario",
  );
  const inputs = boundedColumn(value.inputs, "scenario.inputs");
  const targets = boundedColumn(value.targets, "scenario.targets");
  if (inputs.length !== targets.length) throw new Error("inputs and targets must have the same bounded length");
  const divisor = value.divisor;
  if (!Number.isInteger(divisor) || divisor < 1 || divisor > inputs.length) {
    throw new Error("divisor must be an integer within the batch length");
  }
  const initialParameter = finite(value.initialParameter, "initial parameter", false);
  const learningRate = finite(value.learningRate, "learning rate", false);
  const gradientBufferBefore = finite(value.gradientBufferBefore, "gradient buffer before", false);
  if (learningRate <= 0) throw new Error("learning rate must be positive");
  return {
    id: boundedString(value.id, "scenario.id"),
    title: boundedString(value.title, "scenario.title"),
    summary: boundedString(value.summary, "scenario.summary"),
    initialParameter,
    learningRate,
    inputs,
    targets,
    gradientBufferBefore,
    divisor,
  };
}

function instruction(
  id: string,
  op: string,
  output: string,
  inputs: readonly string[],
  attributes: Readonly<Record<string, AttributeValue>> = {},
  sourceNodes: readonly string[] = [],
  sourceEdges: readonly string[] = [],
  sourceInstructions: readonly string[] = [],
): TrainingLoweringInstruction {
  return { id, op, output, inputs, attributes, sourceNodes, sourceEdges, sourceInstructions };
}

export function compileBackwardTrainingIr(): TrainingLoweringStream {
  return {
    magic: "CANB",
    version: 0,
    instructions: [
      instruction("b0", "SEED_LOSS_GRAD", "d_loss", [], { value: 1 }, ["loss"]),
      instruction("b1", "HALF_SQUARED_ERROR_GRAD", "d_residual", ["residual", "d_loss"], {}, ["loss", "residual"]),
      instruction("b2", "PROPAGATE_GRAD", "d_prediction", ["d_residual"], { through: "subtract_prediction" }, ["residual", "prediction"]),
      instruction("b3", "PARAMETER_LOCAL_GRAD", "local_d_w", ["x", "d_prediction"], { parameter_id: "w" }, ["prediction"], ["w"]),
      instruction("b4", "ACCUMULATE_GRAD", "grad_w", ["grad_w", "local_d_w"], { parameter_id: "w", order: "row_ascending" }, [], ["w"]),
      instruction("b5", "INPUT_GRAD", "d_x", ["w", "d_prediction"], { input_id: "x" }, ["x", "prediction"], ["w"]),
    ],
  };
}

export function compileOptimizerTrainingIr(): TrainingLoweringStream {
  return {
    magic: "CANO",
    version: 0,
    instructions: [
      instruction("o0", "READ_GRAD_BUFFER", "total_d_w", ["grad_w"], { parameter_id: "w" }, [], ["w"]),
      instruction("o1", "DIVIDE_GRAD", "applied_d_w", ["total_d_w"], { divisor_source: "scenario.divisor" }, [], ["w"], ["o0"]),
      instruction("o2", "SGD_UPDATE", "w_next", ["w", "applied_d_w"], { learning_rate_source: "scenario.learning_rate" }, [], ["w"], ["o1"]),
      instruction("o3", "KEEP_GRAD_BUFFER", "grad_w_after_step", ["grad_w"], { optimizer_step_zeroes_gradient: false }, [], ["w"], ["o2"]),
    ],
  };
}

export function compileMatrixTrainingIr(): TrainingLoweringStream {
  return {
    magic: "CANM-TRAIN",
    version: 0,
    instructions: [
      instruction("t0", "LOAD_SAVED_COLUMN", "x_col", ["x"], { saved_value: "x" }, ["x"]),
      instruction("t1", "LOAD_SAVED_COLUMN", "residual_col", ["residual"], { saved_value: "residual" }, ["residual"]),
      instruction("t2", "LOSS_GRAD_COLUMN", "d_prediction_col", ["residual_col"], { loss: "half_squared_error" }, ["loss", "prediction"], [], ["b0", "b1", "b2"]),
      instruction("t3", "PARAMETER_LOCAL_GRAD_COLUMN", "local_d_w_col", ["x_col", "d_prediction_col"], { parameter_id: "w" }, ["prediction"], ["w"], ["b3"]),
      instruction("t4", "INPUT_GRAD_COLUMN", "d_x_col", ["d_prediction_col"], { input_id: "x", parameter_id: "w" }, ["x", "prediction"], ["w"], ["b5"]),
      instruction("t5", "REDUCE_SUM_GRAD", "batch_d_w", ["local_d_w_col"], { order: "row_ascending", parameter_id: "w" }, [], ["w"], ["b4"]),
      instruction("t6", "ACCUMULATE_GRAD_BUFFER", "grad_w", ["grad_w", "batch_d_w"], { parameter_id: "w" }, [], ["w"], ["b4"]),
      instruction("t7", "DIVIDE_GRAD", "applied_d_w", ["grad_w"], { divisor_source: "scenario.divisor" }, [], ["w"], ["o0", "o1"]),
      instruction("t8", "SGD_UPDATE_SCALAR", "w_next", ["w", "applied_d_w"], { learning_rate_source: "scenario.learning_rate" }, [], ["w"], ["o2"]),
      instruction("t9", "KEEP_GRAD_BUFFER", "grad_w_after_step", ["grad_w"], { optimizer_step_zeroes_gradient: false }, [], ["w"], ["o3"]),
    ],
  };
}

function buildForwardGraph(parameter: number) {
  const graph = createNeuralGraph("nn30_scalar_training");
  addInput(graph, "x", "x");
  addWeightedSum(graph, "prediction_sum", [{
    from: "x",
    weight: parameter,
    edgeId: "w",
    properties: { "nn.trainable": true },
  }]);
  addActivation(graph, "prediction", "prediction_sum", "none", {}, "sum_to_prediction");
  addOutput(graph, "out", "prediction", "prediction", {}, "prediction_to_out");
  return graph;
}

function runProductionForward(scenario: BackwardOptimizerLoweringScenario) {
  const graph = buildForwardGraph(scenario.initialParameter);
  const bytecode = compileNeuralGraphToBytecode(graph);
  const matrixPlan = compileBytecodeToMatrixPlan(bytecode);
  const neuralOutputs = scenario.inputs.map((x) => {
    const trace = runNeuralBytecodeForwardWithTrace(bytecode, { x });
    return finite(trace.outputs.prediction!, "NeuralIR prediction");
  });
  const matrixOutputs = runNeuralMatrixForward(matrixPlan, { x: scenario.inputs })
    .outputs.prediction!.map((value) => finite(value, "MatrixIR prediction"));
  const directOutputs = scenario.inputs.map((x) => finite(scenario.initialParameter * x, "direct prediction"));
  return {
    directOutputs,
    neuralIrOutputs: neuralOutputs,
    matrixIrOutputs: matrixOutputs,
    neuralOps: bytecode.functions[0]!.instructions.map((item) => item.op),
    matrixOps: matrixPlan.instructions.map((item) => item.op),
    maxError: maxDifference([directOutputs, neuralOutputs, matrixOutputs]),
  };
}

function savedValues(
  scenario: BackwardOptimizerLoweringScenario,
  predictions: readonly number[],
): SavedTrainingValues {
  const residual = predictions.map((prediction, index) => finite(prediction - scenario.targets[index]!, `residual ${index}`));
  const loss = residual.map((value, index) => finite(0.5 * value * value, `loss ${index}`));
  return {
    x: [...scenario.inputs],
    target: [...scenario.targets],
    prediction: [...predictions],
    residual,
    loss,
  };
}

function runBackwardProgram(
  stream: TrainingLoweringStream,
  saved: SavedTrainingValues,
  parameter: number,
  gradientBufferBefore: number,
): BackwardTrainingValues {
  const values = new Map<string, number | number[]>();
  values.set("x", [...saved.x]);
  values.set("residual", [...saved.residual]);
  values.set("w", parameter);
  values.set("grad_w", gradientBufferBefore);
  for (const item of stream.instructions) {
    switch (item.op) {
      case "SEED_LOSS_GRAD":
        values.set(item.output, Array(saved.x.length).fill(1));
        break;
      case "HALF_SQUARED_ERROR_GRAD": {
        const residual = readColumn(values, "residual");
        const seed = readColumn(values, "d_loss");
        values.set(item.output, residual.map((value, index) => finite(value * seed[index]!, `d_residual ${index}`)));
        break;
      }
      case "PROPAGATE_GRAD":
        values.set(item.output, [...readColumn(values, "d_residual")]);
        break;
      case "PARAMETER_LOCAL_GRAD": {
        const x = readColumn(values, "x");
        const upstream = readColumn(values, "d_prediction");
        values.set(item.output, x.map((value, index) => finite(value * upstream[index]!, `local_d_w ${index}`)));
        break;
      }
      case "ACCUMULATE_GRAD": {
        let gradient = readScalar(values, "grad_w");
        for (const value of readColumn(values, "local_d_w")) gradient = finite(gradient + value, "grad_w reduction");
        values.set(item.output, gradient);
        break;
      }
      case "INPUT_GRAD":
        values.set(item.output, readColumn(values, "d_prediction").map((value, index) => finite(parameter * value, `d_x ${index}`)));
        break;
      default:
        throw new Error(`unsupported backward op: ${item.op}`);
    }
  }
  const localDW = readColumn(values, "local_d_w");
  const batchGradient = stableSum(localDW, "backward batch gradient");
  return {
    dLoss: readColumn(values, "d_loss"),
    dResidual: readColumn(values, "d_residual"),
    dPrediction: readColumn(values, "d_prediction"),
    localDW,
    dX: readColumn(values, "d_x"),
    gradientBufferBefore,
    batchGradient,
    gradW: readScalar(values, "grad_w"),
  };
}

function runOptimizerProgram(
  stream: TrainingLoweringStream,
  scenario: BackwardOptimizerLoweringScenario,
  gradW: number,
): OptimizerTrainingValues {
  const values = new Map<string, number>([["w", scenario.initialParameter], ["grad_w", gradW]]);
  for (const item of stream.instructions) {
    switch (item.op) {
      case "READ_GRAD_BUFFER": values.set(item.output, scalar(values, "grad_w")); break;
      case "DIVIDE_GRAD": values.set(item.output, finite(scalar(values, "total_d_w") / scenario.divisor, "applied gradient")); break;
      case "SGD_UPDATE": values.set(item.output, finite(scalar(values, "w") - scenario.learningRate * scalar(values, "applied_d_w"), "w_next")); break;
      case "KEEP_GRAD_BUFFER": values.set(item.output, scalar(values, "grad_w")); break;
      default: throw new Error(`unsupported optimizer op: ${item.op}`);
    }
  }
  const appliedGradient = scalar(values, "applied_d_w");
  const parameterAfter = scalar(values, "w_next");
  return {
    parameterBefore: scenario.initialParameter,
    appliedGradient,
    parameterDelta: finite(parameterAfter - scenario.initialParameter, "parameter delta"),
    parameterAfter,
    gradientBufferAfterStep: scalar(values, "grad_w_after_step"),
  };
}

function runMatrixTrainingProgram(
  stream: TrainingLoweringStream,
  scenario: BackwardOptimizerLoweringScenario,
  saved: SavedTrainingValues,
): MatrixTrainingValues {
  const values = new Map<string, number | number[]>([
    ["x", [...saved.x]], ["residual", [...saved.residual]], ["w", scenario.initialParameter],
    ["grad_w", scenario.gradientBufferBefore],
  ]);
  for (const item of stream.instructions) {
    switch (item.op) {
      case "LOAD_SAVED_COLUMN": values.set(item.output, [...readColumn(values, item.inputs[0]!)]); break;
      case "LOSS_GRAD_COLUMN": values.set(item.output, [...readColumn(values, "residual_col")]); break;
      case "PARAMETER_LOCAL_GRAD_COLUMN": {
        const x = readColumn(values, "x_col");
        const gradient = readColumn(values, "d_prediction_col");
        values.set(item.output, x.map((value, index) => finite(value * gradient[index]!, `matrix local d_w ${index}`)));
        break;
      }
      case "INPUT_GRAD_COLUMN": values.set(item.output, readColumn(values, "d_prediction_col").map((value, index) => finite(scenario.initialParameter * value, `matrix d_x ${index}`))); break;
      case "REDUCE_SUM_GRAD": {
        values.set(item.output, stableSum(readColumn(values, "local_d_w_col"), "matrix batch gradient"));
        break;
      }
      case "ACCUMULATE_GRAD_BUFFER": values.set(item.output, finite(readScalar(values, "grad_w") + readScalar(values, "batch_d_w"), "matrix grad buffer accumulation")); break;
      case "DIVIDE_GRAD": values.set(item.output, finite(readScalar(values, "grad_w") / scenario.divisor, "matrix applied gradient")); break;
      case "SGD_UPDATE_SCALAR": values.set(item.output, finite(scenario.initialParameter - scenario.learningRate * readScalar(values, "applied_d_w"), "matrix w_next")); break;
      case "KEEP_GRAD_BUFFER": values.set(item.output, readScalar(values, "grad_w")); break;
      default: throw new Error(`unsupported matrix training op: ${item.op}`);
    }
  }
  return {
    columns: {
      x: readColumn(values, "x_col"),
      residual: readColumn(values, "residual_col"),
      dPrediction: readColumn(values, "d_prediction_col"),
      localDW: readColumn(values, "local_d_w_col"),
      dX: readColumn(values, "d_x_col"),
    },
    gradientBufferBefore: scenario.gradientBufferBefore,
    batchGradient: readScalar(values, "batch_d_w"),
    gradW: readScalar(values, "grad_w"),
    appliedGradient: readScalar(values, "applied_d_w"),
    parameterAfter: readScalar(values, "w_next"),
    gradientBufferAfterStep: readScalar(values, "grad_w_after_step"),
  };
}

function readColumn(values: ReadonlyMap<string, number | number[]>, id: string): number[] {
  const value = values.get(id);
  if (!Array.isArray(value)) throw new Error(`missing column: ${id}`);
  return [...value];
}

function readScalar(values: ReadonlyMap<string, number | number[]>, id: string): number {
  const value = values.get(id);
  if (typeof value !== "number") throw new Error(`missing scalar: ${id}`);
  return value;
}

function scalar(values: ReadonlyMap<string, number>, id: string): number {
  const value = values.get(id);
  if (value === undefined) throw new Error(`missing scalar: ${id}`);
  return value;
}

function stableSum(values: readonly number[], context: string): number {
  let total = 0;
  for (const value of values) total = finite(total + value, context);
  return total;
}

function lossSum(parameter: number, scenario: BackwardOptimizerLoweringScenario): number {
  let total = 0;
  scenario.inputs.forEach((x, index) => {
    const residual = finite(parameter * x - scenario.targets[index]!, `audit residual ${index}`);
    total = finite(total + 0.5 * residual * residual, "audit loss sum");
  });
  return total;
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

export function traceBackwardOptimizerLoweringProgram(
  inputScenario: BackwardOptimizerLoweringScenario,
): BackwardOptimizerLoweringTrace {
  const scenario = validateScenario(inputScenario);
  const forward = runProductionForward(scenario);
  const saved = savedValues(scenario, forward.neuralIrOutputs);
  const backwardIr = compileBackwardTrainingIr();
  const optimizerIr = compileOptimizerTrainingIr();
  const matrixTrainingIr = compileMatrixTrainingIr();
  const backward = runBackwardProgram(
    backwardIr,
    saved,
    scenario.initialParameter,
    scenario.gradientBufferBefore,
  );
  const optimizer = runOptimizerProgram(optimizerIr, scenario, backward.gradW);
  const matrixTraining = runMatrixTrainingProgram(matrixTrainingIr, scenario, saved);
  const numerical = finite((lossSum(scenario.initialParameter + EPSILON, scenario) - lossSum(scenario.initialParameter - EPSILON, scenario)) / (2 * EPSILON), "numerical gradient");
  const absoluteError = finite(Math.abs(backward.batchGradient - numerical), "gradient error");
  const maxPathError = Math.max(
    Math.abs(backward.batchGradient - matrixTraining.batchGradient),
    Math.abs(backward.gradW - matrixTraining.gradW),
    Math.abs(optimizer.appliedGradient - matrixTraining.appliedGradient),
    Math.abs(optimizer.parameterAfter - matrixTraining.parameterAfter),
    Math.abs(optimizer.gradientBufferAfterStep - matrixTraining.gradientBufferAfterStep),
  );
  return deepFreeze({
    scenario,
    forward,
    savedValues: saved,
    backwardIr,
    optimizerIr,
    matrixTrainingIr,
    backward,
    optimizer,
    matrixTraining,
    gradientAudit: { analytical: backward.batchGradient, numerical, absoluteError },
    maxPathError: finite(maxPathError, "training path error"),
  });
}

export function traceBackwardOptimizerLowering(
  id: BackwardOptimizerLoweringScenarioId,
): BackwardOptimizerLoweringTrace {
  const scenario = BACKWARD_OPTIMIZER_LOWERING_SCENARIOS.find((candidate) => candidate.id === id);
  if (scenario === undefined) throw new Error(`unknown backward/optimizer lowering scenario: ${id}`);
  return traceBackwardOptimizerLoweringProgram(scenario);
}

function deepFreeze<T>(value: T): T {
  if (typeof value !== "object" || value === null || Object.isFrozen(value)) return value;
  Object.freeze(value);
  Object.values(value).forEach((child) => deepFreeze(child));
  return value;
}
