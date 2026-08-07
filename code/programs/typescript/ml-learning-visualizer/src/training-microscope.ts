import { activate, type ActivationKind } from "./activation.js";

export type MicroscopeActivation = Extract<ActivationKind, "linear" | "sigmoid" | "tanh" | "relu">;

export interface MicroscopeState {
  input: number;
  target: number;
  weight: number;
  bias: number;
  learningRate: number;
  activation: MicroscopeActivation;
}

export interface MicroscopeTrace extends MicroscopeState {
  weightedInput: number;
  preActivation: number;
  prediction: number;
  error: number;
  loss: number;
  lossPredictionDerivative: number;
  activationDerivative: number;
  preActivationWeightDerivative: number;
  preActivationBiasDerivative: number;
  gradientWeight: number;
  gradientBias: number;
  nextWeight: number;
  nextBias: number;
  nextPrediction: number;
  nextLoss: number;
}

export const DEFAULT_MICROSCOPE_STATE: MicroscopeState = {
  input: 2,
  target: 1,
  weight: 0.5,
  bias: 0.1,
  learningRate: 0.1,
  activation: "linear",
};

export function activationDerivative(
  raw: number,
  output: number,
  activation: MicroscopeActivation,
): number {
  switch (activation) {
    case "linear":
      return 1;
    case "sigmoid":
      return output * (1 - output);
    case "tanh":
      return 1 - output * output;
    case "relu":
      return raw > 0 ? 1 : 0;
  }
}

export function traceTrainingStep(state: MicroscopeState): MicroscopeTrace {
  const weightedInput = state.input * state.weight;
  const preActivation = weightedInput + state.bias;
  const prediction = activate(preActivation, state.activation);
  const error = prediction - state.target;
  const loss = error * error;
  const lossPredictionDerivative = 2 * error;
  const localActivationDerivative = activationDerivative(
    preActivation,
    prediction,
    state.activation,
  );
  const preActivationWeightDerivative = state.input;
  const preActivationBiasDerivative = 1;
  const gradientWeight = lossPredictionDerivative
    * localActivationDerivative
    * preActivationWeightDerivative;
  const gradientBias = lossPredictionDerivative
    * localActivationDerivative
    * preActivationBiasDerivative;
  const nextWeight = state.weight - state.learningRate * gradientWeight;
  const nextBias = state.bias - state.learningRate * gradientBias;
  const nextPrediction = activate(state.input * nextWeight + nextBias, state.activation);
  const nextError = nextPrediction - state.target;

  return {
    ...state,
    weightedInput,
    preActivation,
    prediction,
    error,
    loss,
    lossPredictionDerivative,
    activationDerivative: localActivationDerivative,
    preActivationWeightDerivative,
    preActivationBiasDerivative,
    gradientWeight,
    gradientBias,
    nextWeight,
    nextBias,
    nextPrediction,
    nextLoss: nextError * nextError,
  };
}
