/**
 * NN37: sequential dense networks above the existing matrix foundation.
 *
 * A forward pass saves each layer's inputs and outputs. A backward pass walks
 * those saved values in reverse, so every weight receives its own derivative.
 * This is native TypeScript arithmetic; optional accelerated matrix backends
 * are a later, separately measured contract.
 */
import { Matrix } from "matrix";

export type Activation = "linear" | "sigmoid" | "tanh" | "relu";
export type Grid = number[][];
export interface DenseLayer {
  weights: Grid;
  biases: number[];
  activation: Activation;
}
export interface GradientTrace {
  loss: number;
  predictions: Grid;
  weightGradients: Grid[];
  biasGradients: number[][];
}
export interface Checkpoint {
  format: "coding-adventures-dense";
  version: 1;
  step: number;
  layers: DenseLayer[];
}
const ACTIVATIONS = ["linear", "sigmoid", "tanh", "relu"];

function finite(value: unknown): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new Error("expected a finite number");
  }
  return value;
}
function count(value: unknown, minimum = 0, maximum = Number.MAX_SAFE_INTEGER): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < minimum || value > maximum) {
    throw new Error("count must be an integer in range");
  }
  return value;
}
function vector(value: unknown): number[] {
  if (!Array.isArray(value) || value.length === 0) throw new Error("expected a nonempty numeric array");
  // Array.from visits sparse holes too; Array.map would silently preserve them.
  return Array.from(value, finite);
}
function grid(value: unknown): Grid {
  if (!Array.isArray(value) || value.length === 0) throw new Error("expected a nonempty matrix");
  const rows = Array.from(value, vector);
  if (rows.some(row => row.length !== rows[0].length)) throw new Error("matrix must be rectangular");
  return rows;
}
function activation(value: unknown): Activation {
  if (typeof value !== "string" || !ACTIVATIONS.includes(value)) throw new Error("unsupported activation");
  return value as Activation;
}
function activate(value: number, kind: Activation): number {
  if (kind === "linear") return value;
  if (kind === "relu") return Math.max(0, value);
  if (kind === "tanh") return Math.tanh(value);
  const factor = Math.exp(-Math.abs(value));
  return value >= 0 ? 1 / (1 + factor) : factor / (1 + factor);
}
function derivative(raw: number, activated: number, kind: Activation): number {
  if (kind === "linear") return 1;
  if (kind === "relu") return raw > 0 ? 1 : 0;
  if (kind === "tanh") return 1 - activated * activated;
  return activated * (1 - activated);
}
function fields(value: unknown, keys: string[]): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error("invalid fields");
  const own = Reflect.ownKeys(value);
  if (own.length !== keys.length || !keys.every(key => Object.hasOwn(value, key))) {
    throw new Error("invalid fields");
  }
  // A checkpoint is a data document, never an object with executable getters.
  if (own.some(key => !("value" in Object.getOwnPropertyDescriptor(value, key)!))) {
    throw new Error("checkpoint requires data fields");
  }
  return value as Record<string, unknown>;
}
function copyLayer(value: unknown): DenseLayer {
  const layer = fields(value, ["activation", "weights", "biases"]);
  const weights = grid(layer.weights), biases = vector(layer.biases);
  if (weights[0].length !== biases.length) throw new Error("weight columns must match bias count");
  return { weights, biases, activation: activation(layer.activation) };
}

export class DenseNetwork {
  #layers: DenseLayer[];
  #step: number;

  constructor(layers: DenseLayer[], step = 0) {
    if (!Array.isArray(layers) || layers.length === 0) throw new Error("network needs at least one layer");
    this.#step = count(step);
    this.#layers = Array.from(layers, copyLayer);
    this.#layers.forEach((layer, index) => {
      if (index && layer.weights.length !== this.#layers[index - 1].biases.length) {
        throw new Error("consecutive layer widths do not match");
      }
    });
  }
  get step(): number { return this.#step; }

  static seeded(widths: number[], activations: Activation[], seed = 7): DenseNetwork {
    if (!Array.isArray(widths) || widths.length < 2) throw new Error("widths must include input and output");
    const sizes = Array.from(widths, width => count(width, 1));
    if (!Array.isArray(activations) || activations.length !== sizes.length - 1) {
      throw new Error("one activation is required per layer");
    }
    const kinds = Array.from(activations, activation);
    let state = count(seed, 0, 0xffffffff);
    const layers = kinds.map((kind, index) => ({
      activation: kind,
      weights: Array.from({ length: sizes[index] }, () => (
        Array.from({ length: sizes[index + 1] }, () => {
          state = (Math.imul(1664525, state) + 1013904223) >>> 0;
          return (2 * state / 2 ** 32 - 1) / Math.sqrt(sizes[index]);
        })
      )),
      biases: Array(sizes[index + 1]).fill(0),
    }));
    return new DenseNetwork(layers);
  }

  #forward(inputs: Grid): { saved: Grid[]; rawLayers: Grid[] } {
    let current = grid(inputs);
    if (current[0].length !== this.#layers[0].weights.length) throw new Error("input width does not match model");
    const saved = [current], rawLayers: Grid[] = [];
    for (const layer of this.#layers) {
      const products = new Matrix(current).dot(new Matrix(layer.weights)).data;
      const raw = grid(products.map(row => row.map((v, j) => v + layer.biases[j])));
      current = grid(raw.map(row => row.map(v => activate(v, layer.activation))));
      rawLayers.push(raw);
      saved.push(current);
    }
    return { saved, rawLayers };
  }
  predict(inputs: Grid): Grid { return this.#forward(inputs).saved.at(-1)!; }

  gradients(inputs: Grid, targets: Grid): GradientTrace {
    const { saved, rawLayers } = this.#forward(inputs);
    const predictions = saved.at(-1)!, wanted = grid(targets);
    if (wanted.length !== predictions.length || wanted[0].length !== predictions[0].length) {
      throw new Error("target shape does not match predictions");
    }
    const errors = grid(predictions.map((row, i) => row.map((p, j) => p - wanted[i][j])));
    const size = errors.length * errors[0].length;
    const loss = finite(errors.reduce((sum, row) => sum + row.reduce((s, e) => s + e * e, 0), 0) / size);
    let upstream = grid(errors.map(row => row.map(e => 2 * e / size)));
    const weightGradients: Grid[] = [], biasGradients: number[][] = [];
    for (let index = this.#layers.length - 1; index >= 0; index--) {
      const layer = this.#layers[index];
      const delta = grid(upstream.map((row, i) => row.map((g, j) => (
        g * derivative(rawLayers[index][i][j], saved[index + 1][i][j], layer.activation)
      ))));
      weightGradients.unshift(grid(new Matrix(saved[index]).transpose().dot(new Matrix(delta)).data));
      biasGradients.unshift(vector(delta[0].map((_, j) => delta.reduce((sum, row) => sum + row[j], 0))));
      if (index) upstream = grid(new Matrix(delta).dot(new Matrix(layer.weights).transpose()).data);
    }
    return { loss, predictions, weightGradients, biasGradients };
  }

  trainStep(inputs: Grid, targets: Grid, learningRate: number): GradientTrace {
    const rate = finite(learningRate);
    if (rate <= 0) throw new Error("learning rate must be positive");
    const nextStep = count(this.#step + 1), trace = this.gradients(inputs, targets);
    const layers = this.#layers.map((layer, i) => ({
      activation: layer.activation,
      weights: layer.weights.map((row, r) => row.map((v, c) => v - rate * trace.weightGradients[i][r][c])),
      biases: layer.biases.map((v, c) => v - rate * trace.biasGradients[i][c]),
    }));
    const candidate = new DenseNetwork(layers, nextStep);
    candidate.gradients(inputs, targets); // Validate the complete candidate before adopting any part.
    this.#layers = candidate.#layers;
    this.#step = candidate.#step;
    return trace;
  }

  fit(inputs: Grid, targets: Grid, epochs: number, learningRate: number): number[] {
    count(epochs);
    const rate = finite(learningRate);
    if (rate <= 0) throw new Error("learning rate must be positive");
    const losses = [this.gradients(inputs, targets).loss];
    for (let epoch = 0; epoch < epochs; epoch++) {
      this.trainStep(inputs, targets, rate);
      losses.push(this.gradients(inputs, targets).loss);
    }
    return losses;
  }

  toCheckpoint(): Checkpoint {
    return { format: "coding-adventures-dense", version: 1, step: this.step,
      layers: this.#layers.map(copyLayer) };
  }
  static fromCheckpoint(value: unknown): DenseNetwork {
    const doc = fields(value, ["format", "version", "step", "layers"]);
    if (doc.format !== "coding-adventures-dense" || doc.version !== 1) throw new Error("unsupported checkpoint");
    if (!Array.isArray(doc.layers)) throw new Error("invalid checkpoint layers");
    return new DenseNetwork(Array.from(doc.layers, copyLayer), count(doc.step));
  }
}
