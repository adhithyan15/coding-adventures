import {
  TwoLayerNetwork,
  createSeededParameters,
  type ExampleTrace,
  type MatrixData,
  type TrainingSnapshot,
} from "coding-adventures-two-layer-network/src/index";
import { SingleLayerNetwork } from "coding-adventures-single-layer-network/src/index";
import { createXorNetwork } from "@coding-adventures/neural-network";
import {
  compileBytecodeToMatrixPlan,
  compileNeuralNetworkToBytecode,
  runNeuralMatrixForwardScalars,
} from "@coding-adventures/neural-graph-vm";

export const VERSION = "0.1.0";

export const XOR_INPUTS: MatrixData = [
  [0, 0],
  [0, 1],
  [1, 0],
  [1, 1],
];

export const XOR_TARGETS: MatrixData = [
  [0],
  [1],
  [1],
  [0],
];

export interface XorRunOptions {
  epochs?: number;
  learningRate?: number;
  logEvery?: number;
  seed?: number;
  initialScale?: number;
}

export interface XorPredictionRow {
  input: number[];
  target: number;
  prediction: number;
  rounded: 0 | 1;
  hidden: number[];
}

export interface XorRunResult {
  history: TrainingSnapshot[];
  rows: XorPredictionRow[];
  vmRows: XorPredictionRow[];
  finalLoss: number;
  trace: ExampleTrace;
  bytecodeInstructionCount: number;
  matrixInstructionCount: number;
}

export interface LinearFailureResult {
  finalLoss: number;
  rows: XorPredictionRow[];
}

export function runLinearFailureDemo(epochs = 50000, learningRate = 1): LinearFailureResult {
  const network = new SingleLayerNetwork({
    inputCount: 2,
    outputCount: 1,
    activation: "sigmoid",
    learningRate,
  });
  const history = network.fit(XOR_INPUTS, XOR_TARGETS, {
    epochs,
    logEvery: epochs,
  });
  const predictions = network.predict(XOR_INPUTS);
  const rows = XOR_INPUTS.map((input, index) => {
    const prediction = predictions[index]![0]!;
    const rounded: 0 | 1 = prediction >= 0.5 ? 1 : 0;
    return {
      input,
      target: XOR_TARGETS[index]![0]!,
      prediction,
      rounded,
      hidden: [],
    };
  });

  return {
    finalLoss: history[history.length - 1]?.loss ?? 0,
    rows,
  };
}

export function runXorDemo(options: XorRunOptions = {}): XorRunResult {
  const network = new TwoLayerNetwork({
    inputCount: 2,
    hiddenCount: 2,
    outputCount: 1,
    hiddenActivation: "sigmoid",
    outputActivation: "sigmoid",
    learningRate: options.learningRate ?? 1.8,
    initialParameters: createSeededParameters(2, 2, 1, options.seed ?? 11, options.initialScale ?? 2),
  });

  const history = network.fit(XOR_INPUTS, XOR_TARGETS, {
    epochs: options.epochs ?? 12000,
    logEvery: options.logEvery ?? 3000,
  });
  const inspection = network.inspect(XOR_INPUTS);
  const rows = XOR_INPUTS.map((input, index) => {
    const prediction = inspection.predictions[index]![0]!;
    const rounded: 0 | 1 = prediction >= 0.5 ? 1 : 0;
    return {
      input,
      target: XOR_TARGETS[index]![0]!,
      prediction,
      rounded,
      hidden: inspection.hiddenActivations[index]!,
    };
  });
  const vm = runXorGraphVmDemo();

  return {
    history,
    rows,
    vmRows: vm.rows,
    finalLoss: history[history.length - 1]?.loss ?? 0,
    trace: network.trace(XOR_INPUTS, 1, XOR_TARGETS[1]),
    bytecodeInstructionCount: vm.bytecodeInstructionCount,
    matrixInstructionCount: vm.matrixInstructionCount,
  };
}

export function runXorGraphVmDemo(): {
  rows: XorPredictionRow[];
  bytecodeInstructionCount: number;
  matrixInstructionCount: number;
} {
  const bytecode = compileNeuralNetworkToBytecode(createXorNetwork("xor-graph-vm"));
  const matrixPlan = compileBytecodeToMatrixPlan(bytecode);
  const rows = XOR_INPUTS.map((input, index) => {
    const outputs = runNeuralMatrixForwardScalars(matrixPlan, {
      x0: input[0]!,
      x1: input[1]!,
    });
    const prediction = outputs.prediction;
    return {
      input,
      target: XOR_TARGETS[index]![0]!,
      prediction,
      rounded: prediction >= 0.5 ? 1 : 0,
      hidden: [],
    };
  });

  return {
    rows,
    bytecodeInstructionCount: bytecode.functions[0].instructions.length,
    matrixInstructionCount: matrixPlan.instructions.length,
  };
}

function fmt(value: number): string {
  return value.toFixed(4);
}

export function formatXorRun(result: XorRunResult): string {
  const checkpoints = result.history
    .map(snapshot => `epoch ${snapshot.epoch.toString().padStart(5, " ")}  loss ${fmt(snapshot.loss)}`)
    .join("\n");
  const rows = result.rows
    .map(row => {
      const hidden = row.hidden.map(fmt).join(", ");
      return `[${row.input.join(", ")}] target=${row.target} prediction=${fmt(row.prediction)} rounded=${row.rounded} hidden=[${hidden}]`;
    })
    .join("\n");

  const vmRows = result.vmRows
    .map(row => `[${row.input.join(", ")}] target=${row.target} vm_prediction=${fmt(row.prediction)} rounded=${row.rounded}`)
    .join("\n");

  return `${checkpoints}\n\n${rows}\n\nGraph VM matrix path (${result.bytecodeInstructionCount} bytecode ops -> ${result.matrixInstructionCount} matrix ops)\n${vmRows}`;
}

export function formatLinearFailure(result: LinearFailureResult): string {
  const rows = result.rows
    .map(row => `[${row.input.join(", ")}] target=${row.target} prediction=${fmt(row.prediction)} rounded=${row.rounded}`)
    .join("\n");
  return `No hidden layer after many runs: loss ${fmt(result.finalLoss)}\n${rows}`;
}

export function formatTrace(trace: ExampleTrace): string {
  const lines = trace.layers.flatMap(layer => [
    `${layer.layer} layer`,
    ...layer.neurons.map(neuron => {
      const terms = neuron.incoming
        .map(term => `${term.source}:${fmt(term.value)}*${fmt(term.weight)}=${fmt(term.contribution)}`)
        .join(", ");
      const delta = neuron.delta === undefined ? "" : ` delta=${fmt(neuron.delta)}`;
      return `  ${neuron.neuron} terms=[${terms}] bias=${fmt(neuron.bias)} raw=${fmt(neuron.rawSum)} ${neuron.activation}=${fmt(neuron.output)}${delta}`;
    }),
  ]);
  return `Trace for XOR row [${trace.inputs.join(", ")}] -> target [${trace.target?.join(", ")}]\n${lines.join("\n")}`;
}

if (require.main === module) {
  const linear = runLinearFailureDemo();
  const hidden = runXorDemo();
  console.log(formatLinearFailure(linear));
  console.log("");
  console.log("With one hidden layer:");
  console.log(formatXorRun(hidden));
  console.log("");
  console.log(formatTrace(hidden.trace));
}
