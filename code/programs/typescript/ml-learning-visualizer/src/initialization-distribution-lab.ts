export type InitializationKind = "tiny" | "xavier" | "he" | "large";
export type DistributionActivation = "tanh" | "relu";

export interface DistributionSummary {
  mean: number;
  variance: number;
  standardDeviation: number;
  minimum: number;
  maximum: number;
  zeroFraction: number;
  saturatedFraction: number;
}

export interface InitializationLayerTrace {
  layer: number;
  fanIn: number;
  scale: number;
  weights: number[][];
  inputs: number[][];
  preactivations: number[][];
  activations: number[][];
  summary: DistributionSummary;
}

export interface InitializationDistributionTrace {
  initializer: InitializationKind;
  activation: DistributionActivation;
  inputSummary: DistributionSummary;
  layers: InitializationLayerTrace[];
}

export const INITIALIZATION_KINDS: readonly InitializationKind[] = [
  "tiny",
  "xavier",
  "he",
  "large",
];

export const DEFAULT_DISTRIBUTION_INPUTS = [
  [1, 0],
  [0, 1],
  [-1, 0],
  [0, -1],
];

export const DEFAULT_WEIGHT_TEMPLATES = [
  [[1, -1], [1, 1]],
  [[1, -1], [1, 1]],
  [[1, -1], [1, 1]],
];

export function initializerScale(kind: InitializationKind, fanIn: number): number {
  if (!Number.isInteger(fanIn) || fanIn < 1) {
    throw new Error("NN23 fan-in must be a positive integer.");
  }
  if (kind === "tiny") return 0.1;
  if (kind === "xavier") return Math.sqrt(1 / fanIn);
  if (kind === "he") return Math.sqrt(2 / fanIn);
  return 2;
}

export function summarizeDistribution(
  matrix: readonly (readonly number[])[],
  activation: DistributionActivation,
): DistributionSummary {
  const values = matrix.flat();
  if (values.length === 0 || !values.every(Number.isFinite)) {
    throw new Error("NN23 distributions need at least one finite value.");
  }
  const mean = values.reduce((sum, value) => sum + value, 0) / values.length;
  const variance = values.reduce((sum, value) => sum + (value - mean) ** 2, 0) / values.length;
  return {
    mean,
    variance,
    standardDeviation: Math.sqrt(variance),
    minimum: Math.min(...values),
    maximum: Math.max(...values),
    zeroFraction: values.filter((value) => Math.abs(value) < 1e-12).length / values.length,
    saturatedFraction: activation === "tanh"
      ? values.filter((value) => Math.abs(value) >= 0.95).length / values.length
      : 0,
  };
}

function activate(value: number, kind: DistributionActivation): number {
  return kind === "tanh" ? Math.tanh(value) : Math.max(0, value);
}

export function traceInitializationDistributions(
  initializer: InitializationKind = "xavier",
  activation: DistributionActivation = "tanh",
  inputs: readonly (readonly number[])[] = DEFAULT_DISTRIBUTION_INPUTS,
  templates: readonly (readonly (readonly number[])[])[] = DEFAULT_WEIGHT_TEMPLATES,
): InitializationDistributionTrace {
  if (!INITIALIZATION_KINDS.includes(initializer)) {
    throw new Error("NN23 initializer is not supported.");
  }
  if (activation !== "tanh" && activation !== "relu") {
    throw new Error("NN23 activation must be tanh or ReLU.");
  }
  if (inputs.length < 2 || inputs[0]!.length < 1) {
    throw new Error("NN23 needs at least two non-empty input rows.");
  }
  const inputWidth = inputs[0]!.length;
  if (inputs.some((row) => row.length !== inputWidth || !row.every(Number.isFinite))) {
    throw new Error("NN23 inputs must be a finite rectangular matrix.");
  }
  if (templates.length < 1) {
    throw new Error("NN23 needs at least one weight template.");
  }

  let current = inputs.map((row) => [...row]);
  const layers = templates.map((template, layerIndex) => {
    const fanIn = current[0]!.length;
    if (template.length !== fanIn || template.length === 0) {
      throw new Error(`NN23 layer ${layerIndex + 1} template must match fan-in.`);
    }
    const width = template[0]!.length;
    if (width < 1 || template.some((row) => row.length !== width || !row.every(Number.isFinite))) {
      throw new Error(`NN23 layer ${layerIndex + 1} template must be finite and rectangular.`);
    }
    const scale = initializerScale(initializer, fanIn);
    const weights = template.map((row) => row.map((value) => value * scale));
    const preactivations = current.map((row) => (
      Array.from({ length: width }, (_, output) => (
        row.reduce((sum, value, input) => sum + value * weights[input]![output]!, 0)
      ))
    ));
    const activations = preactivations.map((row) => row.map((value) => activate(value, activation)));
    const trace: InitializationLayerTrace = {
      layer: layerIndex + 1,
      fanIn,
      scale,
      weights,
      inputs: current,
      preactivations,
      activations,
      summary: summarizeDistribution(activations, activation),
    };
    current = activations;
    return trace;
  });

  return {
    initializer,
    activation,
    inputSummary: summarizeDistribution(inputs, activation),
    layers,
  };
}
