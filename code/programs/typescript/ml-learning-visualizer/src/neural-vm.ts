import {
  createFeedForwardNetwork,
  type ActivationKind,
} from "@coding-adventures/neural-network";
import {
  compileBytecodeToMatrixPlan,
  compileNeuralNetworkToBytecode,
  runNeuralMatrixForward,
} from "@coding-adventures/neural-graph-vm";
import type {
  ActivationName,
  MatrixData,
  TwoLayerParameters,
} from "coding-adventures-two-layer-network/src/index";

export interface LinearGraphParameters {
  readonly weight: number;
  readonly bias: number;
}

export interface VmRunMetadata {
  readonly bytecodeInstructionCount: number;
  readonly matrixInstructionCount: number;
}

export interface LinearVmRun extends VmRunMetadata {
  readonly predictions: number[];
}

export interface TwoLayerVmRun extends VmRunMetadata {
  readonly predictions: MatrixData;
}

export function predictLinearWithVm(
  xs: readonly number[],
  parameters: LinearGraphParameters,
): LinearVmRun {
  const network = createFeedForwardNetwork({
    name: "ml-learning-linear-visualizer",
    inputNames: ["x"],
    layers: [
      {
        name: "output",
        weights: [[parameters.weight]],
        biases: [parameters.bias],
        activation: "none",
        outputNames: ["prediction"],
      },
    ],
  });
  const bytecode = compileNeuralNetworkToBytecode(network);
  const matrixPlan = compileBytecodeToMatrixPlan(bytecode);
  const result = runNeuralMatrixForward(matrixPlan, { x: xs });

  return {
    predictions: result.outputs.prediction ?? [],
    bytecodeInstructionCount: bytecode.functions[0]?.instructions.length ?? 0,
    matrixInstructionCount: matrixPlan.instructions.length,
  };
}

export function predictTwoLayerWithVm(
  inputs: MatrixData,
  parameters: TwoLayerParameters,
  options: {
    readonly inputNames?: readonly string[];
    readonly outputNames?: readonly string[];
    readonly hiddenActivation?: ActivationName;
    readonly outputActivation?: ActivationName;
  } = {},
): TwoLayerVmRun {
  const inputCount = inputs[0]?.length ?? parameters.inputToHiddenWeights.length;
  const outputCount = parameters.outputBiases.length;
  const inputNames = options.inputNames ?? Array.from(
    { length: inputCount },
    (_, index) => `input${index}`,
  );
  const outputNames = options.outputNames ?? Array.from(
    { length: outputCount },
    (_, index) => (outputCount === 1 ? "prediction" : `output${index}`),
  );
  const network = createFeedForwardNetwork({
    name: "ml-learning-hidden-visualizer",
    inputNames,
    layers: [
      {
        name: "hidden",
        weights: parameters.inputToHiddenWeights,
        biases: parameters.hiddenBiases,
        activation: toGraphActivation(options.hiddenActivation ?? "sigmoid"),
      },
      {
        name: "output",
        weights: parameters.hiddenToOutputWeights,
        biases: parameters.outputBiases,
        activation: toGraphActivation(options.outputActivation ?? "sigmoid"),
        outputNames,
      },
    ],
  });
  const bytecode = compileNeuralNetworkToBytecode(network);
  const matrixPlan = compileBytecodeToMatrixPlan(bytecode);
  const matrixInputs = Object.fromEntries(
    inputNames.map((name, inputIndex) => [
      name,
      inputs.map((row) => row[inputIndex] ?? 0),
    ]),
  );
  const result = runNeuralMatrixForward(matrixPlan, matrixInputs);
  const predictions = inputs.map((_, rowIndex) => (
    outputNames.map((name) => result.outputs[name]?.[rowIndex] ?? 0)
  ));

  return {
    predictions,
    bytecodeInstructionCount: bytecode.functions[0]?.instructions.length ?? 0,
    matrixInstructionCount: matrixPlan.instructions.length,
  };
}

function toGraphActivation(activation: ActivationName): ActivationKind {
  return activation === "linear" ? "none" : activation;
}
