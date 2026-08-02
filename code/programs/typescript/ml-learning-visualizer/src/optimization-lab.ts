export type BatchStrategy = "stochastic" | "mini-batch" | "full-batch";

export interface OptimizationPoint {
  x: number;
  y: number;
}

export interface OptimizationState {
  weight: number;
  bias: number;
  step: number;
}

export interface GradientVector {
  weight: number;
  bias: number;
}

export interface GradientCheck {
  analytical: GradientVector;
  numerical: GradientVector;
  absoluteError: GradientVector;
  maximumRelativeError: number;
  passes: boolean;
}

export interface OptimizationTracePoint extends OptimizationState {
  loss: number;
  batchIndices: number[];
}

export interface LandscapePoint {
  weight: number;
  bias: number;
  loss: number;
  column: number;
  row: number;
}

export const OPTIMIZATION_DATASET: readonly OptimizationPoint[] = [
  { x: -1, y: -1 },
  { x: 0, y: 1 },
  { x: 1, y: 3 },
  { x: 2, y: 5 },
];

export const DEFAULT_OPTIMIZATION_STATE: OptimizationState = {
  weight: -0.5,
  bias: 0,
  step: 0,
};

export const OPTIMUM_PARAMETERS = { weight: 2, bias: 1 };

export function predictPoint(point: OptimizationPoint, state: OptimizationState): number {
  return state.weight * point.x + state.bias;
}

export function meanSquaredError(
  points: readonly OptimizationPoint[],
  state: OptimizationState,
): number {
  if (points.length === 0) {
    throw new Error("meanSquaredError requires at least one point");
  }

  return points.reduce((sum, point) => {
    const error = predictPoint(point, state) - point.y;
    return sum + error * error;
  }, 0) / points.length;
}

export function analyticalGradient(
  points: readonly OptimizationPoint[],
  state: OptimizationState,
): GradientVector {
  if (points.length === 0) {
    throw new Error("analyticalGradient requires at least one point");
  }

  const scale = 2 / points.length;
  return points.reduce(
    (gradient, point) => {
      const error = predictPoint(point, state) - point.y;
      return {
        weight: gradient.weight + scale * error * point.x,
        bias: gradient.bias + scale * error,
      };
    },
    { weight: 0, bias: 0 },
  );
}

export function numericalGradient(
  points: readonly OptimizationPoint[],
  state: OptimizationState,
  epsilon: number,
): GradientVector {
  if (!(epsilon > 0) || !Number.isFinite(epsilon)) {
    throw new Error("epsilon must be a positive finite number");
  }

  function centralDifference(field: "weight" | "bias"): number {
    const plus = { ...state, [field]: state[field] + epsilon };
    const minus = { ...state, [field]: state[field] - epsilon };
    return (meanSquaredError(points, plus) - meanSquaredError(points, minus)) / (2 * epsilon);
  }

  return {
    weight: centralDifference("weight"),
    bias: centralDifference("bias"),
  };
}

export function checkGradient(
  points: readonly OptimizationPoint[],
  state: OptimizationState,
  epsilon: number,
  tolerance = 1e-6,
): GradientCheck {
  const analytical = analyticalGradient(points, state);
  const numerical = numericalGradient(points, state, epsilon);
  const absoluteError = {
    weight: Math.abs(analytical.weight - numerical.weight),
    bias: Math.abs(analytical.bias - numerical.bias),
  };
  const relativeErrors = (["weight", "bias"] as const).map((field) => {
    const scale = Math.max(1, Math.abs(analytical[field]), Math.abs(numerical[field]));
    return absoluteError[field] / scale;
  });
  const maximumRelativeError = Math.max(...relativeErrors);

  return {
    analytical,
    numerical,
    absoluteError,
    maximumRelativeError,
    passes: maximumRelativeError <= tolerance,
  };
}

export function batchIndices(
  strategy: BatchStrategy,
  step: number,
  pointCount: number,
): number[] {
  if (!Number.isInteger(pointCount) || pointCount < 1) {
    throw new Error("pointCount must be a positive integer");
  }

  if (strategy === "full-batch") {
    return Array.from({ length: pointCount }, (_, index) => index);
  }
  if (strategy === "stochastic") {
    return [step % pointCount];
  }

  const start = (step * 2) % pointCount;
  return [start, (start + 1) % pointCount];
}

export function optimizationStep(
  points: readonly OptimizationPoint[],
  state: OptimizationState,
  learningRate: number,
  strategy: BatchStrategy,
): OptimizationTracePoint {
  if (!(learningRate > 0) || !Number.isFinite(learningRate)) {
    throw new Error("learningRate must be a positive finite number");
  }

  const indices = batchIndices(strategy, state.step, points.length);
  const batch = indices.map((index) => points[index]!);
  const gradient = analyticalGradient(batch, state);
  const next = {
    weight: state.weight - learningRate * gradient.weight,
    bias: state.bias - learningRate * gradient.bias,
    step: state.step + 1,
  };

  return {
    ...next,
    loss: meanSquaredError(points, next),
    batchIndices: indices,
  };
}

export function runOptimization(
  strategy: BatchStrategy,
  steps: number,
  learningRate: number,
  initialState: OptimizationState = DEFAULT_OPTIMIZATION_STATE,
  points: readonly OptimizationPoint[] = OPTIMIZATION_DATASET,
): OptimizationTracePoint[] {
  if (!Number.isInteger(steps) || steps < 0) {
    throw new Error("steps must be a non-negative integer");
  }

  const trace: OptimizationTracePoint[] = [{
    ...initialState,
    loss: meanSquaredError(points, initialState),
    batchIndices: [],
  }];
  let current = initialState;
  for (let index = 0; index < steps; index += 1) {
    const next = optimizationStep(points, current, learningRate, strategy);
    trace.push(next);
    current = next;
  }
  return trace;
}

export function sampleLossLandscape(
  points: readonly OptimizationPoint[],
  weightRange: readonly [number, number],
  biasRange: readonly [number, number],
  resolution: number,
): LandscapePoint[] {
  if (!Number.isInteger(resolution) || resolution < 2) {
    throw new Error("resolution must be an integer of at least two");
  }

  const samples: LandscapePoint[] = [];
  for (let row = 0; row < resolution; row += 1) {
    const bias = biasRange[0] + (biasRange[1] - biasRange[0]) * (row / (resolution - 1));
    for (let column = 0; column < resolution; column += 1) {
      const weight = weightRange[0] + (weightRange[1] - weightRange[0]) * (column / (resolution - 1));
      samples.push({
        weight,
        bias,
        loss: meanSquaredError(points, { weight, bias, step: 0 }),
        column,
        row,
      });
    }
  }
  return samples;
}
