import { Matrix } from "../../matrix/src/matrix";

export const VERSION = "0.1.0";

export type MatrixData = number[][];
export type ActivationName = "linear" | "sigmoid";

export interface SingleLayerNetworkOptions {
  inputCount?: number;
  outputCount?: number;
  activation?: ActivationName;
  learningRate?: number;
  initialWeights?: MatrixData;
  initialBiases?: number[];
}

export interface FitOptions {
  epochs?: number;
  learningRate?: number;
  logEvery?: number;
  onEpoch?: (snapshot: TrainingSnapshot) => void;
}

export interface TrainingSnapshot {
  epoch: number;
  loss: number;
  weights: MatrixData;
  biases: number[];
  predictions: MatrixData;
  errors: MatrixData;
  weightGradients: MatrixData;
  biasGradients: number[];
}

export interface MatrixTrainingStep {
  rawOutputs: MatrixData;
  predictions: MatrixData;
  errors: MatrixData;
  lossGradient: MatrixData;
  weightGradients: MatrixData;
  biasGradients: number[];
  nextWeights: MatrixData;
  nextBiases: number[];
  loss: number;
}

function cloneMatrix(rows: MatrixData): MatrixData {
  return rows.map(row => [...row]);
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

function applyActivation(rows: MatrixData, activation: ActivationName): MatrixData {
  if (activation === "linear") {
    return cloneMatrix(rows);
  }
  return rows.map(row => row.map(value => 1 / (1 + Math.exp(-value))));
}

function activationDerivative(rawValue: number, activatedValue: number, activation: ActivationName): number {
  if (activation === "linear") {
    return 1;
  }
  return activatedValue * (1 - activatedValue);
}

function elementwiseActivationGradient(
  rawOutputs: MatrixData,
  predictions: MatrixData,
  errors: MatrixData,
  activation: ActivationName,
): MatrixData {
  const sampleCount = errors.length;
  const outputCount = errors[0].length;
  const scale = 2 / (sampleCount * outputCount);

  return errors.map((row, sample) => row.map((error, output) => (
    scale * error * activationDerivative(rawOutputs[sample][output], predictions[sample][output], activation)
  )));
}

function columnSums(rows: MatrixData): number[] {
  const width = rows[0].length;
  const sums = Array(width).fill(0);
  for (const row of rows) {
    for (let col = 0; col < width; col++) {
      sums[col] += row[col];
    }
  }
  return sums;
}

function meanSquaredError(errors: MatrixData): number {
  let total = 0;
  let count = 0;
  for (const row of errors) {
    for (const value of row) {
      total += value * value;
      count += 1;
    }
  }
  return total / count;
}

function defaultWeights(inputCount: number, outputCount: number): MatrixData {
  return zeros(inputCount, outputCount);
}

export function trainOneEpochWithMatrices(
  inputs: MatrixData,
  targets: MatrixData,
  weights: MatrixData,
  biases: number[],
  learningRate: number,
  activation: ActivationName = "linear",
): MatrixTrainingStep {
  const inputShape = validateMatrix("inputs", inputs);
  const targetShape = validateMatrix("targets", targets);
  const weightShape = validateMatrix("weights", weights);

  if (inputShape.rows !== targetShape.rows) {
    throw new Error("inputs and targets must have the same number of rows");
  }
  if (inputShape.cols !== weightShape.rows) {
    throw new Error("input width must match weight row count");
  }
  if (targetShape.cols !== weightShape.cols) {
    throw new Error("target width must match weight column count");
  }
  if (biases.length !== targetShape.cols) {
    throw new Error("bias count must match target width");
  }

  const rawOutputs = addBiases(new Matrix(inputs).dot(new Matrix(weights)).data, biases);
  const predictions = applyActivation(rawOutputs, activation);
  const errors = new Matrix(predictions).subtract(new Matrix(targets)).data;
  const lossGradient = elementwiseActivationGradient(rawOutputs, predictions, errors, activation);
  const weightGradients = new Matrix(inputs).transpose().dot(new Matrix(lossGradient)).data;
  const biasGradients = columnSums(lossGradient);
  const nextWeights = new Matrix(weights).subtract(new Matrix(weightGradients).scale(learningRate)).data;
  const nextBiases = biases.map((bias, index) => bias - learningRate * biasGradients[index]);

  return {
    rawOutputs,
    predictions,
    errors,
    lossGradient,
    weightGradients,
    biasGradients,
    nextWeights,
    nextBiases,
    loss: meanSquaredError(errors),
  };
}

export class SingleLayerNetwork {
  inputCount: number | null;
  outputCount: number | null;
  activation: ActivationName;
  learningRate: number;
  weights: MatrixData | null;
  biases: number[] | null;

  constructor(options: SingleLayerNetworkOptions = {}) {
    this.inputCount = options.inputCount ?? null;
    this.outputCount = options.outputCount ?? null;
    this.activation = options.activation ?? "linear";
    this.learningRate = options.learningRate ?? 0.01;
    this.weights = options.initialWeights ? cloneMatrix(options.initialWeights) : null;
    this.biases = options.initialBiases ? [...options.initialBiases] : null;
  }

  fit(inputs: MatrixData, targets: MatrixData, options: FitOptions = {}): TrainingSnapshot[] {
    const inputShape = validateMatrix("inputs", inputs);
    const targetShape = validateMatrix("targets", targets);
    if (inputShape.rows !== targetShape.rows) {
      throw new Error("inputs and targets must have the same number of rows");
    }

    if (this.inputCount !== null && this.inputCount !== inputShape.cols) {
      throw new Error("configured input count does not match training input width");
    }
    if (this.outputCount !== null && this.outputCount !== targetShape.cols) {
      throw new Error("configured output count does not match training target width");
    }

    this.inputCount = inputShape.cols;
    this.outputCount = targetShape.cols;
    this.weights ??= defaultWeights(this.inputCount, this.outputCount);
    this.biases ??= Array(this.outputCount).fill(0);

    const epochs = options.epochs ?? 1000;
    const learningRate = options.learningRate ?? this.learningRate;
    const history: TrainingSnapshot[] = [];

    for (let epoch = 0; epoch <= epochs; epoch++) {
      const step = trainOneEpochWithMatrices(
        inputs,
        targets,
        this.weights,
        this.biases,
        learningRate,
        this.activation,
      );

      this.weights = step.nextWeights;
      this.biases = step.nextBiases;

      if (options.logEvery !== undefined && epoch % options.logEvery === 0) {
        const snapshot: TrainingSnapshot = {
          epoch,
          loss: step.loss,
          weights: cloneMatrix(this.weights),
          biases: [...this.biases],
          predictions: step.predictions,
          errors: step.errors,
          weightGradients: step.weightGradients,
          biasGradients: [...step.biasGradients],
        };
        history.push(snapshot);
        options.onEpoch?.(snapshot);
      }
    }

    return history;
  }

  predict(inputs: MatrixData): MatrixData {
    if (this.weights === null || this.biases === null) {
      throw new Error("predict called before fit");
    }
    validateMatrix("inputs", inputs);
    const rawOutputs = addBiases(new Matrix(inputs).dot(new Matrix(this.weights)).data, this.biases);
    return applyActivation(rawOutputs, this.activation);
  }
}

export function fitSingleLayerNetwork(
  inputs: MatrixData,
  targets: MatrixData,
  options: SingleLayerNetworkOptions & FitOptions = {},
): SingleLayerNetwork {
  const model = new SingleLayerNetwork(options);
  model.fit(inputs, targets, options);
  return model;
}
