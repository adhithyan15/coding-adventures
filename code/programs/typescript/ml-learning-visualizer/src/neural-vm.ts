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
import type { LayeredParameters } from "./layered-network.js";

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

export interface LayeredVmRun extends VmRunMetadata {
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

export function predictLayeredWithVm(
  inputs: MatrixData,
  parameters: LayeredParameters,
  options: {
    readonly inputNames?: readonly string[];
    readonly outputNames?: readonly string[];
  } = {},
): LayeredVmRun {
  const firstLayer = parameters.layers[0];
  const lastLayer = parameters.layers[parameters.layers.length - 1];
  if (firstLayer === undefined || lastLayer === undefined) {
    throw new Error("layered VM prediction requires at least one layer");
  }
  const inputCount = inputs[0]?.length ?? firstLayer.weights.length;
  const outputCount = lastLayer.biases.length;
  const inputNames = options.inputNames ?? Array.from(
    { length: inputCount },
    (_, index) => `input${index}`,
  );
  const outputNames = options.outputNames ?? Array.from(
    { length: outputCount },
    (_, index) => (outputCount === 1 ? "prediction" : `output${index}`),
  );
  const network = createFeedForwardNetwork({
    name: "ml-learning-layered-visualizer",
    inputNames,
    layers: parameters.layers.map((layer, layerIndex) => ({
      name: layer.name,
      weights: layer.weights,
      biases: layer.biases,
      activation: toGraphActivation(layer.activation),
      outputNames: layerIndex === parameters.layers.length - 1 ? outputNames : undefined,
    })),
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
