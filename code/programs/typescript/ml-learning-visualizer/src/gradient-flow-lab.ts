export type GradientActivation = "tanh" | "relu";
export type GradientFlowClassification = "vanishing" | "stable" | "exploding";

export interface GradientScenario {
  id: string;
  label: string;
  summary: string;
  input: number;
  weights: number[];
  activation: GradientActivation;
  target: number;
}

export interface GradientLayerTrace {
  layer: number;
  input: number;
  weight: number;
  preactivation: number;
  activation: number;
  activationDerivative: number;
  localJacobian: number;
  upstreamGradient: number;
  preactivationGradient: number;
  weightGradient: number;
  inputGradient: number;
}

export interface GradientFlowTrace {
  scenario: GradientScenario;
  output: number;
  outputError: number;
  loss: number;
  chainJacobian: number;
  inputGradient: number;
  finiteDifferenceInputGradient: number;
  finiteDifferenceError: number;
  classification: GradientFlowClassification;
  layers: GradientLayerTrace[];
}

export const GRADIENT_SCENARIOS: readonly GradientScenario[] = [
  {
    id: "small-tanh",
    label: "Small tanh",
    summary: "Weights and tanh derivatives shrink the chain",
    input: 1,
    weights: [0.5, 0.5, 0.5, 0.5],
    activation: "tanh",
    target: 0,
  },
  {
    id: "saturated-tanh",
    label: "Saturated tanh",
    summary: "Large preactivations make tanh derivatives tiny",
    input: 1,
    weights: [3, 3, 3, 3],
    activation: "tanh",
    target: 0,
  },
  {
    id: "unit-relu",
    label: "Unit ReLU",
    summary: "Local Jacobians stay at one",
    input: 1,
    weights: [1, 1, 1, 1],
    activation: "relu",
    target: 0,
  },
  {
    id: "large-relu",
    label: "Large ReLU",
    summary: "Every layer doubles the forward and backward signal",
    input: 1,
    weights: [2, 2, 2, 2],
    activation: "relu",
    target: 0,
  },
];

function scenarioById(id: string): GradientScenario {
  const scenario = GRADIENT_SCENARIOS.find((item) => item.id === id);
  if (!scenario) throw new Error(`NN24 unknown gradient scenario: ${id}`);
  return scenario;
}

function activate(value: number, kind: GradientActivation): number {
  return kind === "tanh" ? Math.tanh(value) : Math.max(0, value);
}

function derivative(preactivation: number, activation: number, kind: GradientActivation): number {
  return kind === "tanh" ? 1 - activation ** 2 : preactivation > 0 ? 1 : 0;
}

function lossAtInput(scenario: GradientScenario, input: number): number {
  const output = scenario.weights.reduce(
    (value, weight) => activate(weight * value, scenario.activation),
    input,
  );
  return 0.5 * (output - scenario.target) ** 2;
}

export function traceGradientFlow(
  scenarioId = "small-tanh",
  finiteDifferenceEpsilon = 1e-6,
): GradientFlowTrace {
  const scenario = scenarioById(scenarioId);
  if (!Number.isFinite(finiteDifferenceEpsilon) || finiteDifferenceEpsilon <= 0) {
    throw new Error("NN24 finite-difference epsilon must be positive and finite.");
  }
  if (
    !Number.isFinite(scenario.input)
    || !Number.isFinite(scenario.target)
    || scenario.weights.length < 2
    || !scenario.weights.every(Number.isFinite)
  ) {
    throw new Error("NN24 scenarios need finite values and at least two weights.");
  }

  let current = scenario.input;
  const layers: GradientLayerTrace[] = scenario.weights.map((weight, index) => {
    const preactivation = weight * current;
    const activation = activate(preactivation, scenario.activation);
    const activationDerivative = derivative(preactivation, activation, scenario.activation);
    const layer: GradientLayerTrace = {
      layer: index + 1,
      input: current,
      weight,
      preactivation,
      activation,
      activationDerivative,
      localJacobian: weight * activationDerivative,
      upstreamGradient: 0,
      preactivationGradient: 0,
      weightGradient: 0,
      inputGradient: 0,
    };
    current = activation;
    return layer;
  });

  const output = current;
  const outputError = output - scenario.target;
  const loss = 0.5 * outputError ** 2;
  let upstreamGradient = outputError;
  for (let index = layers.length - 1; index >= 0; index -= 1) {
    const layer = layers[index]!;
    const preactivationGradient = upstreamGradient * layer.activationDerivative;
    layer.upstreamGradient = upstreamGradient;
    layer.preactivationGradient = preactivationGradient;
    layer.weightGradient = preactivationGradient * layer.input;
    layer.inputGradient = preactivationGradient * layer.weight;
    upstreamGradient = layer.inputGradient;
  }

  const chainJacobian = layers.reduce((product, layer) => product * layer.localJacobian, 1);
  const finiteDifferenceInputGradient = (
    lossAtInput(scenario, scenario.input + finiteDifferenceEpsilon)
    - lossAtInput(scenario, scenario.input - finiteDifferenceEpsilon)
  ) / (2 * finiteDifferenceEpsilon);
  const magnitude = Math.abs(chainJacobian);
  const classification: GradientFlowClassification = magnitude < 0.1
    ? "vanishing"
    : magnitude > 10
      ? "exploding"
      : "stable";

  return {
    scenario: { ...scenario, weights: [...scenario.weights] },
    output,
    outputError,
    loss,
    chainJacobian,
    inputGradient: upstreamGradient,
    finiteDifferenceInputGradient,
    finiteDifferenceError: Math.abs(upstreamGradient - finiteDifferenceInputGradient),
    classification,
    layers,
  };
}
