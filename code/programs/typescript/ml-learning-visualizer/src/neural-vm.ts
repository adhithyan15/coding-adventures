import {
  createFeedForwardNetwork,
  type ActivationKind,
} from "@coding-adventures/neural-network";
import {
  WebGpuMatrixBackend,
  compileBytecodeToMatrixPlan,
  compileNeuralNetworkToBytecode,
  runNeuralMatrixForward,
  runNeuralMatrixForwardAsync,
  type NeuralMatrixInputs,
  type NeuralMatrixPlan,
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

export type MatrixExecutionBackend = "cpu" | "webgpu";

export interface LinearVmRun extends VmRunMetadata {
  readonly predictions: number[];
}

export interface TwoLayerVmRun extends VmRunMetadata {
  readonly predictions: MatrixData;
}

export interface LayeredVmRun extends VmRunMetadata {
  readonly predictions: MatrixData;
}

export interface AcceleratedLayeredVmRun extends LayeredVmRun {
  readonly backend: MatrixExecutionBackend;
  readonly fallbackReason?: string;
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
  const compiled = compileLayeredMatrixPlan(inputs, parameters, options);
  const result = runNeuralMatrixForward(compiled.matrixPlan, compiled.matrixInputs);
  const predictions = collectMatrixPredictions(inputs, compiled.outputNames, result.outputs);

  return {
    predictions,
    bytecodeInstructionCount: compiled.bytecodeInstructionCount,
    matrixInstructionCount: compiled.matrixInstructionCount,
  };
}

export async function predictLayeredWithBestMatrixBackend(
  inputs: MatrixData,
  parameters: LayeredParameters,
  options: {
    readonly inputNames?: readonly string[];
    readonly outputNames?: readonly string[];
  } = {},
): Promise<AcceleratedLayeredVmRun> {
  const compiled = compileLayeredMatrixPlan(inputs, parameters, options);
  const gpuProbe = await getWebGpuBackendProbe();

  if (gpuProbe.backend !== null) {
    const result = await runNeuralMatrixForwardAsync(
      compiled.matrixPlan,
      compiled.matrixInputs,
      gpuProbe.backend,
    );
    return {
      predictions: collectMatrixPredictions(inputs, compiled.outputNames, result.outputs),
      bytecodeInstructionCount: compiled.bytecodeInstructionCount,
      matrixInstructionCount: compiled.matrixInstructionCount,
      backend: "webgpu",
    };
  }

  const result = runNeuralMatrixForward(compiled.matrixPlan, compiled.matrixInputs);
  return {
    predictions: collectMatrixPredictions(inputs, compiled.outputNames, result.outputs),
    bytecodeInstructionCount: compiled.bytecodeInstructionCount,
    matrixInstructionCount: compiled.matrixInstructionCount,
    backend: "cpu",
    fallbackReason: gpuProbe.reason,
  };
}

export function canUseWebGpuMatrixBackend(): boolean {
  return WebGpuMatrixBackend.isNavigatorAvailable();
}

interface LayeredMatrixCompilation {
  readonly matrixPlan: NeuralMatrixPlan;
  readonly matrixInputs: NeuralMatrixInputs;
  readonly outputNames: readonly string[];
  readonly bytecodeInstructionCount: number;
  readonly matrixInstructionCount: number;
}

interface WebGpuBackendProbe {
  readonly backend: WebGpuMatrixBackend | null;
  readonly reason?: string;
}

let webGpuBackendProbe: Promise<WebGpuBackendProbe> | undefined;

function compileLayeredMatrixPlan(
  inputs: MatrixData,
  parameters: LayeredParameters,
  options: {
    readonly inputNames?: readonly string[];
    readonly outputNames?: readonly string[];
  },
): LayeredMatrixCompilation {
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
  return {
    matrixPlan,
    matrixInputs,
    outputNames,
    bytecodeInstructionCount: bytecode.functions[0]?.instructions.length ?? 0,
    matrixInstructionCount: matrixPlan.instructions.length,
  };
}

function collectMatrixPredictions(
  inputs: MatrixData,
  outputNames: readonly string[],
  outputs: Record<string, readonly number[]>,
): MatrixData {
  return inputs.map((_, rowIndex) => (
    outputNames.map((name) => outputs[name]?.[rowIndex] ?? 0)
  ));
}

async function getWebGpuBackendProbe(): Promise<WebGpuBackendProbe> {
  webGpuBackendProbe ??= createWebGpuBackendProbe();
  return webGpuBackendProbe;
}

async function createWebGpuBackendProbe(): Promise<WebGpuBackendProbe> {
  if (!WebGpuMatrixBackend.isNavigatorAvailable()) {
    return {
      backend: null,
      reason: "WebGPU is not exposed by this browser",
    };
  }

  try {
    const backend = await WebGpuMatrixBackend.createFromNavigator({
      powerPreference: "high-performance",
    });
    return {
      backend,
      reason: backend === null ? "WebGPU is not exposed by this browser" : undefined,
    };
  } catch (error) {
    return {
      backend: null,
      reason: error instanceof Error ? error.message : "WebGPU initialization failed",
    };
  }
}

function toGraphActivation(activation: ActivationName): ActivationKind {
  return activation === "linear" ? "none" : activation;
}
