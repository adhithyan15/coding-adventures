import { Matrix } from "../../matrix/src/matrix";

export const VERSION = "0.1.0";

export type MatrixData = number[][];
export type ActivationName = "linear" | "sigmoid" | "tanh" | "relu";

export interface TwoLayerParameters {
  inputToHiddenWeights: MatrixData;
  hiddenBiases: number[];
  hiddenToOutputWeights: MatrixData;
  outputBiases: number[];
}

export interface TwoLayerNetworkOptions {
  inputCount: number;
  hiddenCount: number;
  outputCount: number;
  hiddenActivation?: ActivationName;
  outputActivation?: ActivationName;
  learningRate?: number;
  initialParameters?: TwoLayerParameters;
  seed?: number;
  initialScale?: number;
}

export interface FitOptions {
  epochs?: number;
  learningRate?: number;
  logEvery?: number;
  onEpoch?: (snapshot: TrainingSnapshot) => void;
}

export interface ForwardPass {
  hiddenRaw: MatrixData;
  hiddenActivations: MatrixData;
  outputRaw: MatrixData;
  predictions: MatrixData;
}

export interface WeightedTermTrace {
  source: string;
  value: number;
  weight: number;
  contribution: number;
}

export interface NeuronTrace {
  neuron: string;
  incoming: WeightedTermTrace[];
  bias: number;
  rawSum: number;
  activation: ActivationName;
  output: number;
  delta?: number;
}

export interface LayerTrace {
  layer: string;
  neurons: NeuronTrace[];
}

export interface ExampleTrace {
  exampleIndex: number;
  inputs: number[];
  target?: number[];
  prediction: number[];
  error?: number[];
  loss?: number;
  layers: LayerTrace[];
}

export interface TwoLayerTrainingStep extends ForwardPass {
  errors: MatrixData;
  outputDeltas: MatrixData;
  hiddenDeltas: MatrixData;
  hiddenToOutputWeightGradients: MatrixData;
  outputBiasGradients: number[];
  inputToHiddenWeightGradients: MatrixData;
  hiddenBiasGradients: number[];
  nextParameters: TwoLayerParameters;
  loss: number;
}

export interface TrainingSnapshot {
  epoch: number;
  loss: number;
  parameters: TwoLayerParameters;
  predictions: MatrixData;
  hiddenActivations: MatrixData;
}

function cloneMatrix(rows: MatrixData): MatrixData {
  return rows.map(row => [...row]);
}

function cloneParameters(parameters: TwoLayerParameters): TwoLayerParameters {
  return {
    inputToHiddenWeights: cloneMatrix(parameters.inputToHiddenWeights),
    hiddenBiases: [...parameters.hiddenBiases],
    hiddenToOutputWeights: cloneMatrix(parameters.hiddenToOutputWeights),
    outputBiases: [...parameters.outputBiases],
  };
}

function validateMatrix(name: string, rows: MatrixData): { rows: number; cols: number } {
  if (rows.length === 0 || rows[0].length === 0) {
    throw new Error(`${name} must have at least one row and one column`);
  }
  const cols = rows[0].length;
  for (const row of rows) {
    if (row.length !== cols) {
      throw new Error(`${name} must be rectangular`);
    }
  }
  return { rows: rows.length, cols };
}

function zeros(rows: number, cols: number): MatrixData {
  return Array.from({ length: rows }, () => Array(cols).fill(0));
}

function addBiases(rows: MatrixData, biases: number[]): MatrixData {
  return rows.map(row => row.map((value, col) => value + biases[col]));
}

function columnSums(rows: MatrixData): number[] {
  const width = rows[0].length;
  const sums = Array(width).fill(0);
  for (const row of rows) {
    for (let col = 0; col < width; col += 1) {
      sums[col] += row[col];
    }
  }
  return sums;
}

function meanSquaredError(errors: MatrixData): number {
  let total = 0;
  let count = 0;
  for (const row of errors) {
    for (const error of row) {
      total += error * error;
      count += 1;
    }
  }
  return total / count;
}

function rowLoss(errors: number[]): number {
  return errors.reduce((total, error) => total + error * error, 0) / errors.length;
}

function activateValue(value: number, activation: ActivationName): number {
  switch (activation) {
    case "linear":
      return value;
    case "sigmoid":
      if (value >= 0) {
        const z = Math.exp(-value);
        return 1 / (1 + z);
      }
      {
        const z = Math.exp(value);
        return z / (1 + z);
      }
    case "tanh":
      return Math.tanh(value);
    case "relu":
      return Math.max(0, value);
  }
}

function activationDerivative(rawValue: number, activatedValue: number, activation: ActivationName): number {
  switch (activation) {
    case "linear":
      return 1;
    case "sigmoid":
      return activatedValue * (1 - activatedValue);
    case "tanh":
      return 1 - activatedValue * activatedValue;
    case "relu":
      return rawValue > 0 ? 1 : 0;
  }
}

function applyActivation(rows: MatrixData, activation: ActivationName): MatrixData {
  return rows.map(row => row.map(value => activateValue(value, activation)));
}

function elementwiseMultiply(left: MatrixData, right: MatrixData): MatrixData {
  return left.map((row, rowIndex) => row.map((value, colIndex) => value * right[rowIndex]![colIndex]!));
}

function activationGradient(raw: MatrixData, activated: MatrixData, activation: ActivationName): MatrixData {
  return raw.map((row, rowIndex) => row.map((value, colIndex) => (
    activationDerivative(value, activated[rowIndex]![colIndex]!, activation)
  )));
}

function subtractScaled(matrix: MatrixData, gradients: MatrixData, learningRate: number): MatrixData {
  return new Matrix(matrix).subtract(new Matrix(gradients).scale(learningRate)).data;
}

function subtractScaledVector(values: number[], gradients: number[], learningRate: number): number[] {
  return values.map((value, index) => value - learningRate * gradients[index]!);
}

function makeRandom(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (Math.imul(1664525, state) + 1013904223) >>> 0;
    return state / 0x100000000;
  };
}

export function createSeededParameters(
  inputCount: number,
  hiddenCount: number,
  outputCount: number,
  seed = 7,
  scale = 1,
): TwoLayerParameters {
  const random = makeRandom(seed);
  const centered = () => (random() * 2 - 1) * scale;
  return {
    inputToHiddenWeights: Array.from({ length: inputCount }, () => (
      Array.from({ length: hiddenCount }, centered)
    )),
    hiddenBiases: Array.from({ length: hiddenCount }, centered),
    hiddenToOutputWeights: Array.from({ length: hiddenCount }, () => (
      Array.from({ length: outputCount }, centered)
    )),
    outputBiases: Array.from({ length: outputCount }, centered),
  };
}

export function createXorWarmStartParameters(): TwoLayerParameters {
  return {
    inputToHiddenWeights: [
      [4, -4],
      [4, -4],
    ],
    hiddenBiases: [-2, 6],
    hiddenToOutputWeights: [
      [4],
      [4],
    ],
    outputBiases: [-6],
  };
}

export function forwardTwoLayer(
  inputs: MatrixData,
  parameters: TwoLayerParameters,
  hiddenActivation: ActivationName = "sigmoid",
  outputActivation: ActivationName = "sigmoid",
): ForwardPass {
  const inputShape = validateMatrix("inputs", inputs);
  const inputWeightShape = validateMatrix("inputToHiddenWeights", parameters.inputToHiddenWeights);
  const hiddenWeightShape = validateMatrix("hiddenToOutputWeights", parameters.hiddenToOutputWeights);

  if (inputShape.cols !== inputWeightShape.rows) {
    throw new Error("input width must match input-to-hidden weight row count");
  }
  if (parameters.hiddenBiases.length !== inputWeightShape.cols) {
    throw new Error("hidden bias count must match hidden width");
  }
  if (inputWeightShape.cols !== hiddenWeightShape.rows) {
    throw new Error("hidden width must match hidden-to-output weight row count");
  }
  if (parameters.outputBiases.length !== hiddenWeightShape.cols) {
    throw new Error("output bias count must match output width");
  }

  const hiddenRaw = addBiases(new Matrix(inputs).dot(new Matrix(parameters.inputToHiddenWeights)).data, parameters.hiddenBiases);
  const hiddenActivations = applyActivation(hiddenRaw, hiddenActivation);
  const outputRaw = addBiases(new Matrix(hiddenActivations).dot(new Matrix(parameters.hiddenToOutputWeights)).data, parameters.outputBiases);
  const predictions = applyActivation(outputRaw, outputActivation);

  return {
    hiddenRaw,
    hiddenActivations,
    outputRaw,
    predictions,
  };
}

export function trainOneEpochTwoLayer(
  inputs: MatrixData,
  targets: MatrixData,
  parameters: TwoLayerParameters,
  learningRate: number,
  hiddenActivation: ActivationName = "sigmoid",
  outputActivation: ActivationName = "sigmoid",
): TwoLayerTrainingStep {
  const inputShape = validateMatrix("inputs", inputs);
  const targetShape = validateMatrix("targets", targets);
  if (inputShape.rows !== targetShape.rows) {
    throw new Error("inputs and targets must have the same number of rows");
  }

  const forward = forwardTwoLayer(inputs, parameters, hiddenActivation, outputActivation);
  const predictionShape = validateMatrix("predictions", forward.predictions);
  if (predictionShape.cols !== targetShape.cols) {
    throw new Error("prediction width must match target width");
  }

  const errors = new Matrix(forward.predictions).subtract(new Matrix(targets)).data;
  const scale = 2 / (targetShape.rows * targetShape.cols);
  const lossGradient = errors.map(row => row.map(error => scale * error));
  const outputDeltas = elementwiseMultiply(
    lossGradient,
    activationGradient(forward.outputRaw, forward.predictions, outputActivation),
  );
  const hiddenToOutputWeightGradients = new Matrix(forward.hiddenActivations).transpose().dot(new Matrix(outputDeltas)).data;
  const outputBiasGradients = columnSums(outputDeltas);

  const hiddenErrors = new Matrix(outputDeltas).dot(new Matrix(parameters.hiddenToOutputWeights).transpose()).data;
  const hiddenDeltas = elementwiseMultiply(
    hiddenErrors,
    activationGradient(forward.hiddenRaw, forward.hiddenActivations, hiddenActivation),
  );
  const inputToHiddenWeightGradients = new Matrix(inputs).transpose().dot(new Matrix(hiddenDeltas)).data;
  const hiddenBiasGradients = columnSums(hiddenDeltas);

  return {
    ...forward,
    errors,
    outputDeltas,
    hiddenDeltas,
    hiddenToOutputWeightGradients,
    outputBiasGradients,
    inputToHiddenWeightGradients,
    hiddenBiasGradients,
    nextParameters: {
      inputToHiddenWeights: subtractScaled(parameters.inputToHiddenWeights, inputToHiddenWeightGradients, learningRate),
      hiddenBiases: subtractScaledVector(parameters.hiddenBiases, hiddenBiasGradients, learningRate),
      hiddenToOutputWeights: subtractScaled(parameters.hiddenToOutputWeights, hiddenToOutputWeightGradients, learningRate),
      outputBiases: subtractScaledVector(parameters.outputBiases, outputBiasGradients, learningRate),
    },
    loss: meanSquaredError(errors),
  };
}

export function traceExampleTwoLayer(
  inputs: MatrixData,
  parameters: TwoLayerParameters,
  exampleIndex = 0,
  target?: number[],
  hiddenActivation: ActivationName = "sigmoid",
  outputActivation: ActivationName = "sigmoid",
): ExampleTrace {
  const forward = forwardTwoLayer(inputs, parameters, hiddenActivation, outputActivation);
  if (exampleIndex < 0 || exampleIndex >= inputs.length) {
    throw new Error("exampleIndex must refer to an input row");
  }
  const inputRow = inputs[exampleIndex]!;
  const hiddenRawRow = forward.hiddenRaw[exampleIndex]!;
  const hiddenActivationRow = forward.hiddenActivations[exampleIndex]!;
  const outputRawRow = forward.outputRaw[exampleIndex]!;
  const predictionRow = forward.predictions[exampleIndex]!;
  if (target !== undefined && target.length !== predictionRow.length) {
    throw new Error("target width must match prediction width");
  }

  let outputDeltas: number[] | undefined;
  let hiddenDeltas: number[] | undefined;
  let error: number[] | undefined;
  let loss: number | undefined;
  if (target !== undefined) {
    error = predictionRow.map((prediction, output) => prediction - target[output]!);
    loss = rowLoss(error);
    outputDeltas = error.map((value, output) => (
      value * activationDerivative(outputRawRow[output]!, predictionRow[output]!, outputActivation)
    ));
    hiddenDeltas = hiddenActivationRow.map((_, hidden) => {
      let hiddenError = 0;
      for (let output = 0; output < outputDeltas!.length; output += 1) {
        hiddenError += outputDeltas![output]! * parameters.hiddenToOutputWeights[hidden]![output]!;
      }
      return hiddenError * activationDerivative(hiddenRawRow[hidden]!, hiddenActivationRow[hidden]!, hiddenActivation);
    });
  }

  const hiddenNeurons: NeuronTrace[] = hiddenActivationRow.map((output, hidden) => ({
    neuron: `hidden[${hidden}]`,
    incoming: inputRow.map((value, input) => {
      const weight = parameters.inputToHiddenWeights[input]![hidden]!;
      return {
        source: `input[${input}]`,
        value,
        weight,
        contribution: value * weight,
      };
    }),
    bias: parameters.hiddenBiases[hidden]!,
    rawSum: hiddenRawRow[hidden]!,
    activation: hiddenActivation,
    output,
    delta: hiddenDeltas?.[hidden],
  }));

  const outputNeurons: NeuronTrace[] = predictionRow.map((prediction, output) => ({
    neuron: `output[${output}]`,
    incoming: hiddenActivationRow.map((value, hidden) => {
      const weight = parameters.hiddenToOutputWeights[hidden]![output]!;
      return {
        source: `hidden[${hidden}]`,
        value,
        weight,
        contribution: value * weight,
      };
    }),
    bias: parameters.outputBiases[output]!,
    rawSum: outputRawRow[output]!,
    activation: outputActivation,
    output: prediction,
    delta: outputDeltas?.[output],
  }));

  return {
    exampleIndex,
    inputs: [...inputRow],
    target: target === undefined ? undefined : [...target],
    prediction: [...predictionRow],
    error,
    loss,
    layers: [
      { layer: "hidden", neurons: hiddenNeurons },
      { layer: "output", neurons: outputNeurons },
    ],
  };
}

export class TwoLayerNetwork {
  hiddenActivation: ActivationName;
  outputActivation: ActivationName;
  learningRate: number;
  parameters: TwoLayerParameters;

  constructor(options: TwoLayerNetworkOptions) {
    this.hiddenActivation = options.hiddenActivation ?? "sigmoid";
    this.outputActivation = options.outputActivation ?? "sigmoid";
    this.learningRate = options.learningRate ?? 0.5;
    this.parameters = options.initialParameters
      ? cloneParameters(options.initialParameters)
      : createSeededParameters(
        options.inputCount,
        options.hiddenCount,
        options.outputCount,
        options.seed,
        options.initialScale ?? 1,
      );
  }

  fit(inputs: MatrixData, targets: MatrixData, options: FitOptions = {}): TrainingSnapshot[] {
    const epochs = options.epochs ?? 1000;
    const learningRate = options.learningRate ?? this.learningRate;
    const logEvery = options.logEvery ?? epochs;
    const history: TrainingSnapshot[] = [];
    if (logEvery <= 0) {
      throw new Error("logEvery must be greater than zero");
    }

    for (let epoch = 0; epoch <= epochs; epoch += 1) {
      const step = trainOneEpochTwoLayer(
        inputs,
        targets,
        this.parameters,
        learningRate,
        this.hiddenActivation,
        this.outputActivation,
      );
      this.parameters = step.nextParameters;

      if (epoch % logEvery === 0 || epoch === epochs) {
        const snapshot: TrainingSnapshot = {
          epoch,
          loss: step.loss,
          parameters: cloneParameters(this.parameters),
          predictions: cloneMatrix(step.predictions),
          hiddenActivations: cloneMatrix(step.hiddenActivations),
        };
        history.push(snapshot);
        options.onEpoch?.(snapshot);
      }
    }

    return history;
  }

  predict(inputs: MatrixData): MatrixData {
    return forwardTwoLayer(inputs, this.parameters, this.hiddenActivation, this.outputActivation).predictions;
  }

  inspect(inputs: MatrixData): ForwardPass {
    return forwardTwoLayer(inputs, this.parameters, this.hiddenActivation, this.outputActivation);
  }

  trace(inputs: MatrixData, exampleIndex = 0, target?: number[]): ExampleTrace {
    return traceExampleTwoLayer(
      inputs,
      this.parameters,
      exampleIndex,
      target,
      this.hiddenActivation,
      this.outputActivation,
    );
  }
}
