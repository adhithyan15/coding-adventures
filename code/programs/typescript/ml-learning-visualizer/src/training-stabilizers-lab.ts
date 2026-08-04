export type TrainingStabilizerRouteId = "plain" | "normalization" | "dropout" | "residual";

export interface TrainingStabilizerRouteDefinition {
  id: TrainingStabilizerRouteId;
  label: string;
  summary: string;
}

export interface TrainingNormalizationTrace {
  mean: number;
  centered: number[];
  variance: number;
  standardDeviation: number;
  normalized: number[];
  upstreamSum: number;
  upstreamDotNormalized: number;
}

export interface TrainingDropoutTrace {
  scaledMask: number[];
  evaluationOutput: number[];
  trainingExpectation: number[];
}

export interface TrainingStabilizerRouteTrace {
  id: TrainingStabilizerRouteId;
  output: number[];
  score: number;
  branchGradient: number[];
  skipGradient: number[];
  inputGradient: number[];
  weightGradient: number;
  finiteDifferenceInputGradient: number[];
  finiteDifferenceWeightGradient: number;
  inputGradientAbsoluteError: number[];
  weightGradientAbsoluteError: number;
}

export interface TrainingStabilizerTrace {
  input: number[];
  branchWeight: number;
  upstreamGradient: number[];
  dropoutMask: number[];
  keepProbability: number;
  branch: number[];
  normalization: TrainingNormalizationTrace;
  dropout: TrainingDropoutTrace;
  routes: TrainingStabilizerRouteTrace[];
}

export const TRAINING_STABILIZER_ROUTES: readonly TrainingStabilizerRouteDefinition[] = [
  { id: "plain", label: "Plain branch", summary: "The learned branch is the control" },
  { id: "normalization", label: "Layer normalization", summary: "Coordinates share mean and variance" },
  { id: "dropout", label: "Inverted dropout", summary: "A pinned training mask drops and rescales" },
  { id: "residual", label: "Identity residual", summary: "A short skip bypasses the learned branch" },
];

export const DEFAULT_STABILIZER_INPUT = [1, 1, 3, 3] as const;
export const DEFAULT_STABILIZER_UPSTREAM = [1, 0, 0, -1] as const;
export const DEFAULT_STABILIZER_DROPOUT_MASK = [1, 0, 1, 0] as const;

function cleanZero(value: number): number {
  return Math.abs(value) < 1e-12 ? 0 : value;
}

function dot(left: readonly number[], right: readonly number[]): number {
  return cleanZero(left.reduce((sum, value, index) => sum + value * right[index]!, 0));
}

function normalize(
  values: readonly number[],
  upstream: readonly number[],
  epsilon: number,
): TrainingNormalizationTrace {
  const mean = values.reduce((sum, value) => sum + value, 0) / values.length;
  const centered = values.map((value) => cleanZero(value - mean));
  const variance = centered.reduce((sum, value) => sum + value ** 2, 0) / values.length;
  const standardDeviation = Math.sqrt(variance + epsilon);
  if (standardDeviation === 0) {
    throw new Error("NN25 normalization variance must be positive.");
  }
  const normalized = centered.map((value) => cleanZero(value / standardDeviation));
  return {
    mean,
    centered,
    variance,
    standardDeviation,
    normalized,
    upstreamSum: cleanZero(upstream.reduce((sum, value) => sum + value, 0)),
    upstreamDotNormalized: dot(upstream, normalized),
  };
}

function routeOutput(
  routeId: TrainingStabilizerRouteId,
  input: readonly number[],
  branchWeight: number,
  dropoutMask: readonly number[],
  keepProbability: number,
  normalizationEpsilon: number,
): number[] {
  const branch = input.map((value) => cleanZero(branchWeight * value));
  if (routeId === "plain") return branch;
  if (routeId === "normalization") {
    return normalize(branch, [0, 0, 0, 0], normalizationEpsilon).normalized;
  }
  if (routeId === "dropout") {
    return branch.map((value, index) => cleanZero(
      value * dropoutMask[index]! / keepProbability,
    ));
  }
  return branch.map((value, index) => cleanZero(value + input[index]!));
}

function traceRoute(
  routeId: TrainingStabilizerRouteId,
  input: readonly number[],
  branchWeight: number,
  upstream: readonly number[],
  dropoutMask: readonly number[],
  keepProbability: number,
  normalizationEpsilon: number,
  finiteDifferenceEpsilon: number,
  normalization: TrainingNormalizationTrace,
): TrainingStabilizerRouteTrace {
  const output = routeOutput(
    routeId,
    input,
    branchWeight,
    dropoutMask,
    keepProbability,
    normalizationEpsilon,
  );
  let branchGradient: number[];
  if (routeId === "normalization") {
    const count = input.length;
    const denominator = count * normalization.standardDeviation;
    branchGradient = upstream.map((value, index) => cleanZero((
      count * value
      - normalization.upstreamSum
      - normalization.normalized[index]! * normalization.upstreamDotNormalized
    ) / denominator));
  } else if (routeId === "dropout") {
    branchGradient = upstream.map((value, index) => cleanZero(
      value * dropoutMask[index]! / keepProbability,
    ));
  } else {
    branchGradient = [...upstream];
  }
  const skipGradient = routeId === "residual" ? [...upstream] : input.map(() => 0);
  const inputGradient = branchGradient.map((value, index) => cleanZero(
    branchWeight * value + skipGradient[index]!,
  ));
  const weightGradient = dot(branchGradient, input);
  const score = dot(upstream, output);
  const finiteDifferenceInputGradient = input.map((_, coordinate) => {
    const positive = [...input];
    const negative = [...input];
    positive[coordinate] = positive[coordinate]! + finiteDifferenceEpsilon;
    negative[coordinate] = negative[coordinate]! - finiteDifferenceEpsilon;
    const positiveScore = dot(upstream, routeOutput(
      routeId,
      positive,
      branchWeight,
      dropoutMask,
      keepProbability,
      normalizationEpsilon,
    ));
    const negativeScore = dot(upstream, routeOutput(
      routeId,
      negative,
      branchWeight,
      dropoutMask,
      keepProbability,
      normalizationEpsilon,
    ));
    return (positiveScore - negativeScore) / (2 * finiteDifferenceEpsilon);
  });
  const positiveWeightScore = dot(upstream, routeOutput(
    routeId,
    input,
    branchWeight + finiteDifferenceEpsilon,
    dropoutMask,
    keepProbability,
    normalizationEpsilon,
  ));
  const negativeWeightScore = dot(upstream, routeOutput(
    routeId,
    input,
    branchWeight - finiteDifferenceEpsilon,
    dropoutMask,
    keepProbability,
    normalizationEpsilon,
  ));
  const finiteDifferenceWeightGradient = (
    positiveWeightScore - negativeWeightScore
  ) / (2 * finiteDifferenceEpsilon);
  return {
    id: routeId,
    output,
    score,
    branchGradient,
    skipGradient,
    inputGradient,
    weightGradient,
    finiteDifferenceInputGradient,
    finiteDifferenceWeightGradient,
    inputGradientAbsoluteError: inputGradient.map((value, index) => (
      Math.abs(value - finiteDifferenceInputGradient[index]!)
    )),
    weightGradientAbsoluteError: Math.abs(
      weightGradient - finiteDifferenceWeightGradient,
    ),
  };
}

export function traceTrainingStabilizers(
  input: readonly number[] = DEFAULT_STABILIZER_INPUT,
  branchWeight = 0.5,
  upstreamGradient: readonly number[] = DEFAULT_STABILIZER_UPSTREAM,
  dropoutMask: readonly number[] = DEFAULT_STABILIZER_DROPOUT_MASK,
  keepProbability = 0.5,
  normalizationEpsilon = 0,
  finiteDifferenceEpsilon = 1e-6,
): TrainingStabilizerTrace {
  if (
    input.length !== 4
    || upstreamGradient.length !== 4
    || dropoutMask.length !== 4
    || !input.every(Number.isFinite)
    || !upstreamGradient.every(Number.isFinite)
    || !dropoutMask.every((value) => value === 0 || value === 1)
    || !Number.isFinite(branchWeight)
    || !Number.isFinite(keepProbability)
    || keepProbability <= 0
    || keepProbability > 1
    || !Number.isFinite(normalizationEpsilon)
    || normalizationEpsilon < 0
    || !Number.isFinite(finiteDifferenceEpsilon)
    || finiteDifferenceEpsilon <= 0
  ) {
    throw new Error(
      "NN25 needs four finite coordinates, a binary mask, valid probability, and valid epsilon values.",
    );
  }
  const branch = input.map((value) => cleanZero(branchWeight * value));
  const normalization = normalize(branch, upstreamGradient, normalizationEpsilon);
  const scaledMask = dropoutMask.map((value) => cleanZero(value / keepProbability));
  const dropout: TrainingDropoutTrace = {
    scaledMask,
    evaluationOutput: [...branch],
    trainingExpectation: [...branch],
  };
  const routes = TRAINING_STABILIZER_ROUTES.map((route) => traceRoute(
    route.id,
    input,
    branchWeight,
    upstreamGradient,
    dropoutMask,
    keepProbability,
    normalizationEpsilon,
    finiteDifferenceEpsilon,
    normalization,
  ));
  return {
    input: [...input],
    branchWeight,
    upstreamGradient: [...upstreamGradient],
    dropoutMask: [...dropoutMask],
    keepProbability,
    branch,
    normalization,
    dropout,
    routes,
  };
}
