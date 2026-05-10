import { Matrix } from "matrix/src/matrix";
import type {
  ActivationName,
  ExampleTrace,
  LayerTrace,
  MatrixData,
  NeuronTrace,
} from "coding-adventures-two-layer-network/src/index";

export interface LayerParameters {
  readonly name: string;
  readonly weights: MatrixData;
  readonly biases: number[];
  readonly activation: ActivationName;
}

export interface LayeredParameters {
  readonly layers: readonly LayerParameters[];
}

export interface LayeredForwardPass {
  readonly rawByLayer: MatrixData[];
  readonly activationsByLayer: MatrixData[];
  readonly predictions: MatrixData;
}

export interface LayeredTrainingStep extends LayeredForwardPass {
  readonly errors: MatrixData;
  readonly deltas: MatrixData[];
  readonly weightGradients: MatrixData[];
  readonly biasGradients: number[][];
  readonly nextParameters: LayeredParameters;
  readonly loss: number;
}

function cloneMatrix(rows: MatrixData): MatrixData {
  return rows.map((row) => [...row]);
}

function cloneLayer(layer: LayerParameters): LayerParameters {
  return {
    name: layer.name,
    weights: cloneMatrix(layer.weights),
    biases: [...layer.biases],
    activation: layer.activation,
  };
}

function validateMatrix(name: string, rows: MatrixData): { rows: number; cols: number } {
  if (rows.length === 0 || rows[0] === undefined || rows[0].length === 0) {
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

function makeRandom(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (Math.imul(1664525, state) + 1013904223) >>> 0;
    return state / 0x100000000;
  };
}

function centered(random: () => number, scale: number): number {
  return (random() * 2 - 1) * scale;
}

function addBiases(rows: MatrixData, biases: number[]): MatrixData {
  return rows.map((row) => row.map((value, col) => value + biases[col]!));
}

function columnSums(rows: MatrixData): number[] {
  const width = rows[0]?.length ?? 0;
  const sums = Array(width).fill(0);
  for (const row of rows) {
    for (let col = 0; col < width; col += 1) {
      sums[col] += row[col]!;
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
  return count === 0 ? 0 : total / count;
}

function rowLoss(errors: readonly number[]): number {
  if (errors.length === 0) {
    return 0;
  }
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
  return rows.map((row) => row.map((value) => activateValue(value, activation)));
}

function activationGradient(raw: MatrixData, activated: MatrixData, activation: ActivationName): MatrixData {
  return raw.map((row, rowIndex) => row.map((value, colIndex) => (
    activationDerivative(value, activated[rowIndex]![colIndex]!, activation)
  )));
}

function elementwiseMultiply(left: MatrixData, right: MatrixData): MatrixData {
  return left.map((row, rowIndex) => row.map((value, colIndex) => value * right[rowIndex]![colIndex]!));
}

function subtractScaled(matrix: MatrixData, gradients: MatrixData, learningRate: number): MatrixData {
  return new Matrix(matrix).subtract(new Matrix(gradients).scale(learningRate)).data;
}

function subtractScaledVector(values: readonly number[], gradients: readonly number[], learningRate: number): number[] {
  return values.map((value, index) => value - learningRate * gradients[index]!);
}

function validateLayer(layer: LayerParameters, inputCount: number, layerIndex: number): void {
  const shape = validateMatrix(`${layer.name} weights`, layer.weights);
  if (shape.rows !== inputCount) {
    throw new Error(`${layer.name} weight row count must match previous layer width`);
  }
  if (shape.cols !== layer.biases.length) {
    throw new Error(`${layer.name} weight columns must match bias count`);
  }
  if (layer.biases.length === 0) {
    throw new Error(`${layer.name} must have at least one neuron`);
  }
  for (const row of layer.weights) {
    for (const value of row) {
      if (!Number.isFinite(value)) {
        throw new Error(`${layer.name} weights must be finite`);
      }
    }
  }
  for (const value of layer.biases) {
    if (!Number.isFinite(value)) {
      throw new Error(`${layer.name} biases must be finite`);
    }
  }
  if (layerIndex < 0) {
    throw new Error("layer index must be non-negative");
  }
}

export function createSeededLayeredParameters(
  inputCount: number,
  hiddenWidth: number,
  hiddenLayerCount: number,
  outputCount: number,
  seed = 7,
  scale = 1,
): LayeredParameters {
  if (hiddenLayerCount < 1) {
    throw new Error("hiddenLayerCount must be at least one");
  }
  const random = makeRandom(seed);
  const layers: LayerParameters[] = [];
  let previousWidth = inputCount;

  for (let layerIndex = 0; layerIndex < hiddenLayerCount; layerIndex += 1) {
    const layerScale = hiddenLayerCount === 1 ? scale : scale / Math.sqrt(Math.max(1, previousWidth));
    layers.push({
      name: `hidden${layerIndex + 1}`,
      weights: Array.from({ length: previousWidth }, () => (
        Array.from({ length: hiddenWidth }, () => centered(random, layerScale))
      )),
      biases: Array.from({ length: hiddenWidth }, () => centered(random, layerScale)),
      activation: "sigmoid",
    });
    previousWidth = hiddenWidth;
  }

  const outputScale = hiddenLayerCount === 1 ? scale : scale / Math.sqrt(Math.max(1, previousWidth));
  layers.push({
    name: "output",
    weights: Array.from({ length: previousWidth }, () => (
      Array.from({ length: outputCount }, () => centered(random, outputScale))
    )),
    biases: Array.from({ length: outputCount }, () => centered(random, outputScale)),
    activation: "sigmoid",
  });

  return { layers };
}

export function forwardLayered(
  inputs: MatrixData,
  parameters: LayeredParameters,
): LayeredForwardPass {
  const inputShape = validateMatrix("inputs", inputs);
  if (parameters.layers.length === 0) {
    throw new Error("layered network must have at least one layer");
  }

  const rawByLayer: MatrixData[] = [];
  const activationsByLayer: MatrixData[] = [];
  let current = inputs;
  let previousWidth = inputShape.cols;

  for (const [layerIndex, layer] of parameters.layers.entries()) {
    validateLayer(layer, previousWidth, layerIndex);
    const raw = addBiases(new Matrix(current).dot(new Matrix(layer.weights)).data, layer.biases);
    const activated = applyActivation(raw, layer.activation);
    rawByLayer.push(raw);
    activationsByLayer.push(activated);
    current = activated;
    previousWidth = layer.biases.length;
  }

  return {
    rawByLayer,
    activationsByLayer,
    predictions: activationsByLayer[activationsByLayer.length - 1]!,
  };
}

export function trainOneEpochLayered(
  inputs: MatrixData,
  targets: MatrixData,
  parameters: LayeredParameters,
  learningRate: number,
): LayeredTrainingStep {
  const inputShape = validateMatrix("inputs", inputs);
  const targetShape = validateMatrix("targets", targets);
  if (inputShape.rows !== targetShape.rows) {
    throw new Error("inputs and targets must have the same number of rows");
  }

  const forward = forwardLayered(inputs, parameters);
  const predictionShape = validateMatrix("predictions", forward.predictions);
  if (predictionShape.cols !== targetShape.cols) {
    throw new Error("prediction width must match target width");
  }

  const errors = new Matrix(forward.predictions).subtract(new Matrix(targets)).data;
  const scale = 2 / (targetShape.rows * targetShape.cols);
  const lossGradient = errors.map((row) => row.map((error) => scale * error));
  const deltas: MatrixData[] = Array(parameters.layers.length);
  const lastLayerIndex = parameters.layers.length - 1;
  deltas[lastLayerIndex] = elementwiseMultiply(
    lossGradient,
    activationGradient(
      forward.rawByLayer[lastLayerIndex]!,
      forward.activationsByLayer[lastLayerIndex]!,
      parameters.layers[lastLayerIndex]!.activation,
    ),
  );

  for (let layerIndex = lastLayerIndex - 1; layerIndex >= 0; layerIndex -= 1) {
    const downstreamErrors = new Matrix(deltas[layerIndex + 1]!)
      .dot(new Matrix(parameters.layers[layerIndex + 1]!.weights).transpose())
      .data;
    deltas[layerIndex] = elementwiseMultiply(
      downstreamErrors,
      activationGradient(
        forward.rawByLayer[layerIndex]!,
        forward.activationsByLayer[layerIndex]!,
        parameters.layers[layerIndex]!.activation,
      ),
    );
  }

  const weightGradients: MatrixData[] = [];
  const biasGradients: number[][] = [];
  const nextLayers = parameters.layers.map((layer, layerIndex) => {
    const previousActivations = layerIndex === 0
      ? inputs
      : forward.activationsByLayer[layerIndex - 1]!;
    const layerDeltas = deltas[layerIndex]!;
    const weightGradient = new Matrix(previousActivations).transpose().dot(new Matrix(layerDeltas)).data;
    const biasGradient = columnSums(layerDeltas);
    weightGradients.push(weightGradient);
    biasGradients.push(biasGradient);

    return {
      ...cloneLayer(layer),
      weights: subtractScaled(layer.weights, weightGradient, learningRate),
      biases: subtractScaledVector(layer.biases, biasGradient, learningRate),
    };
  });

  return {
    ...forward,
    errors,
    deltas,
    weightGradients,
    biasGradients,
    nextParameters: { layers: nextLayers },
    loss: meanSquaredError(errors),
  };
}

export function traceExampleLayered(
  inputs: MatrixData,
  parameters: LayeredParameters,
  exampleIndex = 0,
  target?: number[],
): ExampleTrace {
  const forward = forwardLayered(inputs, parameters);
  if (exampleIndex < 0 || exampleIndex >= inputs.length) {
    throw new Error("exampleIndex must refer to an input row");
  }
  const inputRow = inputs[exampleIndex]!;
  const predictionRow = forward.predictions[exampleIndex]!;
  if (target !== undefined && target.length !== predictionRow.length) {
    throw new Error("target width must match prediction width");
  }

  let trainingDeltas: MatrixData[] | undefined;
  let error: number[] | undefined;
  let loss: number | undefined;
  if (target !== undefined) {
    error = predictionRow.map((prediction, output) => prediction - target[output]!);
    loss = rowLoss(error);
    trainingDeltas = trainOneEpochLayered([inputRow], [target], parameters, 0).deltas;
  }

  const layers: LayerTrace[] = parameters.layers.map((layer, layerIndex) => {
    const previousValues = layerIndex === 0
      ? inputRow
      : forward.activationsByLayer[layerIndex - 1]![exampleIndex]!;
    const sourcePrefix = layerIndex === 0
      ? "input"
      : parameters.layers[layerIndex - 1]!.name;
    const rawRow = forward.rawByLayer[layerIndex]![exampleIndex]!;
    const outputRow = forward.activationsByLayer[layerIndex]![exampleIndex]!;
    const neurons: NeuronTrace[] = outputRow.map((output, unit) => ({
      neuron: `${layer.name}[${unit}]`,
      incoming: previousValues.map((value, previousUnit) => {
        const weight = layer.weights[previousUnit]![unit]!;
        return {
          source: `${sourcePrefix}[${previousUnit}]`,
          value,
          weight,
          contribution: value * weight,
        };
      }),
      bias: layer.biases[unit]!,
      rawSum: rawRow[unit]!,
      activation: layer.activation,
      output,
      delta: trainingDeltas?.[layerIndex]?.[0]?.[unit],
    }));
    return { layer: layer.name, neurons };
  });

  return {
    exampleIndex,
    inputs: [...inputRow],
    target: target === undefined ? undefined : [...target],
    prediction: [...predictionRow],
    error,
    loss,
    layers,
  };
}

export function hiddenLayerCount(parameters: LayeredParameters): number {
  return Math.max(0, parameters.layers.length - 1);
}

export function gradientShape(rows: readonly (readonly number[])[] | undefined): string {
  if (rows === undefined || rows.length === 0) {
    return "0x0";
  }
  return `${rows.length}x${rows[0]?.length ?? 0}`;
}
