import { predictLinearWithVm } from "./neural-vm.js";

export type LossKind = "mse" | "mae";

export interface TrainingPoint {
  x: number;
  y: number;
  label?: string;
  group?: string;
}

export interface ModelState {
  weight: number;
  bias: number;
  epoch: number;
}

export interface StepResult {
  previousState: ModelState;
  previousLoss: number;
  state: ModelState;
  loss: number;
  mae: number;
  gradientWeight: number;
  gradientBias: number;
}

export const CELSIUS_DATASET: TrainingPoint[] = [
  { x: -40, y: -40 },
  { x: -10, y: 14 },
  { x: 0, y: 32 },
  { x: 8, y: 46.4 },
  { x: 15, y: 59 },
  { x: 22, y: 71.6 },
  { x: 38, y: 100.4 },
];

export function predict(x: number, state: ModelState): number {
  return predictLinearWithVm([x], state).predictions[0] ?? 0;
}

export function predictions(points: TrainingPoint[], state: ModelState): number[] {
  return predictLinearWithVm(points.map((point) => point.x), state).predictions;
}

export function loss(points: TrainingPoint[], state: ModelState, lossKind: LossKind): number {
  const yPred = predictions(points, state);

  if (lossKind === "mse") {
    return (
      points.reduce((sum, point, index) => {
        const error = yPred[index]! - point.y;
        return sum + error * error;
      }, 0) / points.length
    );
  }

  return (
    points.reduce((sum, point, index) => {
      return sum + Math.abs(yPred[index]! - point.y);
    }, 0) / points.length
  );
}

export function meanAbsoluteError(points: TrainingPoint[], state: ModelState): number {
  return loss(points, state, "mae");
}

export function gradients(
  points: TrainingPoint[],
  state: ModelState,
  lossKind: LossKind,
): { gradientWeight: number; gradientBias: number } {
  const yPred = predictions(points, state);
  const n = points.length;

  return points.reduce(
    (acc, point, index) => {
      const error = yPred[index]! - point.y;
      const predictionGradient =
        lossKind === "mse" ? (2 / n) * error : Math.sign(error) / n;

      return {
        gradientWeight: acc.gradientWeight + predictionGradient * point.x,
        gradientBias: acc.gradientBias + predictionGradient,
      };
    },
    { gradientWeight: 0, gradientBias: 0 },
  );
}

export function trainStep(
  points: TrainingPoint[],
  state: ModelState,
  learningRate: number,
  lossKind: LossKind,
): StepResult {
  const { gradientWeight, gradientBias } = gradients(points, state, lossKind);
  const previousLoss = loss(points, state, lossKind);
  const nextState = {
    weight: state.weight - learningRate * gradientWeight,
    bias: state.bias - learningRate * gradientBias,
    epoch: state.epoch + 1,
  };

  return {
    previousState: state,
    previousLoss,
    state: nextState,
    loss: loss(points, nextState, lossKind),
    mae: meanAbsoluteError(points, nextState),
    gradientWeight,
    gradientBias,
  };
}

export function trainSteps(
  points: TrainingPoint[],
  state: ModelState,
  learningRate: number,
  lossKind: LossKind,
  count: number,
): StepResult[] {
  const results: StepResult[] = [];
  let current = state;

  for (let index = 0; index < count; index += 1) {
    const result = trainStep(points, current, learningRate, lossKind);
    results.push(result);
    current = result.state;
  }

  return results;
}

export function fitLinearClosedForm(points: TrainingPoint[]): ModelState {
  const n = points.length;
  const meanX = points.reduce((sum, point) => sum + point.x, 0) / n;
  const meanY = points.reduce((sum, point) => sum + point.y, 0) / n;
  const numerator = points.reduce((sum, point) => sum + (point.x - meanX) * (point.y - meanY), 0);
  const denominator = points.reduce((sum, point) => sum + (point.x - meanX) ** 2, 0);
  const weight = denominator === 0 ? 0 : numerator / denominator;

  return {
    weight,
    bias: meanY - weight * meanX,
    epoch: 0,
  };
}
