export type LossKind = "mse" | "mae";

export interface TrainingPoint {
  celsius: number;
  fahrenheit: number;
}

export interface ModelState {
  weight: number;
  bias: number;
  epoch: number;
}

export interface StepResult {
  state: ModelState;
  loss: number;
  mae: number;
  gradientWeight: number;
  gradientBias: number;
}

export const CELSIUS_DATASET: TrainingPoint[] = [
  { celsius: -40, fahrenheit: -40 },
  { celsius: -10, fahrenheit: 14 },
  { celsius: 0, fahrenheit: 32 },
  { celsius: 8, fahrenheit: 46.4 },
  { celsius: 15, fahrenheit: 59 },
  { celsius: 22, fahrenheit: 71.6 },
  { celsius: 38, fahrenheit: 100.4 },
];

export function predict(celsius: number, state: ModelState): number {
  return state.weight * celsius + state.bias;
}

export function predictions(points: TrainingPoint[], state: ModelState): number[] {
  return points.map((point) => predict(point.celsius, state));
}

export function loss(points: TrainingPoint[], state: ModelState, lossKind: LossKind): number {
  const yPred = predictions(points, state);

  if (lossKind === "mse") {
    return (
      points.reduce((sum, point, index) => {
        const error = yPred[index]! - point.fahrenheit;
        return sum + error * error;
      }, 0) / points.length
    );
  }

  return (
    points.reduce((sum, point, index) => {
      return sum + Math.abs(yPred[index]! - point.fahrenheit);
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
      const error = yPred[index]! - point.fahrenheit;
      const predictionGradient =
        lossKind === "mse" ? (2 / n) * error : Math.sign(error) / n;

      return {
        gradientWeight: acc.gradientWeight + predictionGradient * point.celsius,
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
  const nextState = {
    weight: state.weight - learningRate * gradientWeight,
    bias: state.bias - learningRate * gradientBias,
    epoch: state.epoch + 1,
  };

  return {
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
