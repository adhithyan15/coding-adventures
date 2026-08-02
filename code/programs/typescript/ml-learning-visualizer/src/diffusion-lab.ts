export interface DiffusionScheduleStep {
  t: number;
  beta: number;
  normalizedT: number;
}

export interface DiffusionDenoiserParameters {
  sampleWeight: number;
  timestepWeight: number;
  bias: number;
}

export interface DiffusionForwardStep extends DiffusionScheduleStep {
  alpha: number;
  alphaBar: number;
  signalScale: number;
  noiseScale: number;
  signalContribution: number;
  noiseContribution: number;
  noisySample: number;
}

export interface DiffusionPredictionStep {
  t: number;
  noisySample: number;
  normalizedT: number;
  predictedNoise: number;
  targetNoise: number;
  error: number;
  loss: number;
}

export interface DiffusionReverseStep {
  t: number;
  inputSample: number;
  normalizedT: number;
  predictedNoise: number;
  noiseCoefficient: number;
  scaledNoiseCorrection: number;
  correctedSample: number;
  alphaScale: number;
  outputMean: number;
}

export interface OneDimensionalDiffusionTrace {
  cleanSample: number;
  savedNoise: number;
  learningRate: number;
  schedule: DiffusionScheduleStep[];
  denoiser: DiffusionDenoiserParameters;
  forwardSteps: DiffusionForwardStep[];
  initialDenoising: DiffusionPredictionStep[];
  initialMeanLoss: number;
  backward: {
    perStep: Array<{
      t: number;
      predictionGradient: number;
      sampleWeightContribution: number;
      timestepWeightContribution: number;
      biasContribution: number;
    }>;
    sampleWeightGradient: number;
    timestepWeightGradient: number;
    biasGradient: number;
  };
  gradientCheck: {
    epsilon: number;
    parameterOrder: string[];
    analytical: number[];
    numerical: number[];
    maxAbsoluteError: number;
  };
  updatedDenoiser: DiffusionDenoiserParameters;
  postUpdateDenoising: DiffusionPredictionStep[];
  postUpdateMeanLoss: number;
  reverseSteps: DiffusionReverseStep[];
  finalReconstruction: number;
  finalAbsoluteError: number;
}

export const DEFAULT_DIFFUSION_CLEAN_SAMPLE = 1;
export const DEFAULT_DIFFUSION_SAVED_NOISE = -0.5;
export const DEFAULT_DIFFUSION_SCHEDULE: DiffusionScheduleStep[] = [
  { t: 1, beta: 0.36, normalizedT: 0.5 },
  { t: 2, beta: 0.4375, normalizedT: 1 },
];
export const DEFAULT_DIFFUSION_DENOISER: DiffusionDenoiserParameters = {
  sampleWeight: 0,
  timestepWeight: 0,
  bias: 0,
};
export const DEFAULT_DIFFUSION_LEARNING_RATE = 0.5;
export const DIFFUSION_PARAMETER_ORDER = [
  "denoiser.sample_weight",
  "denoiser.timestep_weight",
  "denoiser.bias",
];

function copyDenoiser(
  parameters: DiffusionDenoiserParameters,
): DiffusionDenoiserParameters {
  return { ...parameters };
}

function forwardDiffusion(
  cleanSample: number,
  savedNoise: number,
  schedule: readonly DiffusionScheduleStep[],
): DiffusionForwardStep[] {
  let alphaBar = 1;
  return schedule.map((step) => {
    const alpha = 1 - step.beta;
    alphaBar *= alpha;
    const signalScale = Math.sqrt(alphaBar);
    const noiseScale = Math.sqrt(1 - alphaBar);
    const signalContribution = signalScale * cleanSample;
    const noiseContribution = noiseScale * savedNoise;
    return {
      ...step,
      alpha,
      alphaBar,
      signalScale,
      noiseScale,
      signalContribution,
      noiseContribution,
      noisySample: signalContribution + noiseContribution,
    };
  });
}

function predictNoise(
  forwardSteps: readonly DiffusionForwardStep[],
  savedNoise: number,
  denoiser: DiffusionDenoiserParameters,
): { rows: DiffusionPredictionStep[]; meanLoss: number } {
  const rows = forwardSteps.map((step) => {
    const predictedNoise = denoiser.sampleWeight * step.noisySample
      + denoiser.timestepWeight * step.normalizedT
      + denoiser.bias;
    const error = predictedNoise - savedNoise;
    return {
      t: step.t,
      noisySample: step.noisySample,
      normalizedT: step.normalizedT,
      predictedNoise,
      targetNoise: savedNoise,
      error,
      loss: 0.5 * error * error,
    };
  });
  return {
    rows,
    meanLoss: rows.reduce((sum, row) => sum + row.loss, 0) / rows.length,
  };
}

function reverseDiffusion(
  forwardSteps: readonly DiffusionForwardStep[],
  denoiser: DiffusionDenoiserParameters,
): DiffusionReverseStep[] {
  let currentSample = forwardSteps[forwardSteps.length - 1]!.noisySample;
  return [...forwardSteps].reverse().map((step) => {
    const predictedNoise = denoiser.sampleWeight * currentSample
      + denoiser.timestepWeight * step.normalizedT
      + denoiser.bias;
    const noiseCoefficient = step.beta / step.noiseScale;
    const scaledNoiseCorrection = noiseCoefficient * predictedNoise;
    const correctedSample = currentSample - scaledNoiseCorrection;
    const alphaScale = Math.sqrt(step.alpha);
    const outputMean = correctedSample / alphaScale;
    const trace = {
      t: step.t,
      inputSample: currentSample,
      normalizedT: step.normalizedT,
      predictedNoise,
      noiseCoefficient,
      scaledNoiseCorrection,
      correctedSample,
      alphaScale,
      outputMean,
    };
    currentSample = outputMean;
    return trace;
  });
}

export function traceOneDimensionalDiffusion(
  cleanSample = DEFAULT_DIFFUSION_CLEAN_SAMPLE,
  savedNoise = DEFAULT_DIFFUSION_SAVED_NOISE,
  learningRate = DEFAULT_DIFFUSION_LEARNING_RATE,
  denoiser: DiffusionDenoiserParameters = DEFAULT_DIFFUSION_DENOISER,
  schedule: readonly DiffusionScheduleStep[] = DEFAULT_DIFFUSION_SCHEDULE,
): OneDimensionalDiffusionTrace {
  const scalarInputs = [
    cleanSample,
    savedNoise,
    learningRate,
    denoiser.sampleWeight,
    denoiser.timestepWeight,
    denoiser.bias,
    ...schedule.flatMap((step) => [step.t, step.beta, step.normalizedT]),
  ];
  if (
    !scalarInputs.every(Number.isFinite)
    || learningRate <= 0
    || schedule.length < 2
    || schedule.some((step, index) => (
      !Number.isInteger(step.t)
      || step.t !== index + 1
      || step.beta <= 0
      || step.beta >= 1
      || step.normalizedT <= (schedule[index - 1]?.normalizedT ?? 0)
      || step.normalizedT > 1
    ))
    || Math.abs(schedule[schedule.length - 1]!.normalizedT - 1) > 1e-12
  ) {
    throw new Error(
      "NN19 V1 needs finite scalars, a positive learning rate, and consecutive increasing diffusion steps ending at normalized time 1.",
    );
  }

  const parameterCopy = copyDenoiser(denoiser);
  const scheduleCopy = schedule.map((step) => ({ ...step }));
  const forwardSteps = forwardDiffusion(
    cleanSample,
    savedNoise,
    scheduleCopy,
  );
  const initial = predictNoise(forwardSteps, savedNoise, parameterCopy);
  const count = initial.rows.length;
  const perStep = initial.rows.map((row) => {
    const predictionGradient = row.error / count;
    return {
      t: row.t,
      predictionGradient,
      sampleWeightContribution: predictionGradient * row.noisySample,
      timestepWeightContribution: predictionGradient * row.normalizedT,
      biasContribution: predictionGradient,
    };
  });
  const sampleWeightGradient = perStep.reduce(
    (sum, row) => sum + row.sampleWeightContribution,
    0,
  );
  const timestepWeightGradient = perStep.reduce(
    (sum, row) => sum + row.timestepWeightContribution,
    0,
  );
  const biasGradient = perStep.reduce(
    (sum, row) => sum + row.biasContribution,
    0,
  );
  const analytical = [
    sampleWeightGradient,
    timestepWeightGradient,
    biasGradient,
  ];
  const initialParameters = [
    parameterCopy.sampleWeight,
    parameterCopy.timestepWeight,
    parameterCopy.bias,
  ];
  const auditEpsilon = 1e-6;
  const numerical = initialParameters.map((_, parameterIndex) => {
    const plus = [...initialParameters];
    const minus = [...initialParameters];
    plus[parameterIndex]! += auditEpsilon;
    minus[parameterIndex]! -= auditEpsilon;
    const lossFor = (values: readonly number[]) => predictNoise(
      forwardSteps,
      savedNoise,
      {
        sampleWeight: values[0]!,
        timestepWeight: values[1]!,
        bias: values[2]!,
      },
    ).meanLoss;
    return (lossFor(plus) - lossFor(minus)) / (2 * auditEpsilon);
  });
  const maxAbsoluteError = Math.max(
    ...analytical.map((value, index) => (
      Math.abs(value - numerical[index]!)
    )),
  );
  const updatedDenoiser = {
    sampleWeight: parameterCopy.sampleWeight
      - learningRate * sampleWeightGradient,
    timestepWeight: parameterCopy.timestepWeight
      - learningRate * timestepWeightGradient,
    bias: parameterCopy.bias - learningRate * biasGradient,
  };
  const postUpdate = predictNoise(
    forwardSteps,
    savedNoise,
    updatedDenoiser,
  );
  const reverseSteps = reverseDiffusion(forwardSteps, updatedDenoiser);
  const finalReconstruction = reverseSteps[reverseSteps.length - 1]!.outputMean;

  return {
    cleanSample,
    savedNoise,
    learningRate,
    schedule: scheduleCopy,
    denoiser: parameterCopy,
    forwardSteps,
    initialDenoising: initial.rows,
    initialMeanLoss: initial.meanLoss,
    backward: {
      perStep,
      sampleWeightGradient,
      timestepWeightGradient,
      biasGradient,
    },
    gradientCheck: {
      epsilon: auditEpsilon,
      parameterOrder: [...DIFFUSION_PARAMETER_ORDER],
      analytical,
      numerical,
      maxAbsoluteError,
    },
    updatedDenoiser,
    postUpdateDenoising: postUpdate.rows,
    postUpdateMeanLoss: postUpdate.meanLoss,
    reverseSteps,
    finalReconstruction,
    finalAbsoluteError: Math.abs(finalReconstruction - cleanSample),
  };
}
