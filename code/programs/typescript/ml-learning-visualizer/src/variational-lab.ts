export interface ScalarAffineParameters {
  weight: number;
  bias: number;
}

export interface VariationalParameters {
  encoder: {
    mean: ScalarAffineParameters;
    logVariance: ScalarAffineParameters;
  };
  decoder: ScalarAffineParameters;
}

export interface VariationalForwardTrace {
  meanProduct: number;
  mean: number;
  logVarianceProduct: number;
  logVariance: number;
  variance: number;
  standardDeviation: number;
  epsilon: number;
  noiseContribution: number;
  latent: number;
  decoderProduct: number;
  reconstruction: number;
  error: number;
  reconstructionLoss: number;
  meanSquared: number;
  kl: number;
  weightedKl: number;
  totalLoss: number;
}

export interface VariationalBackwardTrace {
  reconstructionGradient: number;
  decoderWeightGradient: number;
  decoderBiasGradient: number;
  latentGradient: number;
  reconstructionMeanGradient: number;
  reconstructionLogVarianceGradient: number;
  klMeanGradient: number;
  klLogVarianceGradient: number;
  weightedKlMeanGradient: number;
  weightedKlLogVarianceGradient: number;
  meanGradient: number;
  logVarianceGradient: number;
  meanWeightGradient: number;
  meanBiasGradient: number;
  logVarianceWeightGradient: number;
  logVarianceBiasGradient: number;
}

export interface VariationalTrace {
  input: number;
  beta: number;
  samplingEpsilon: number;
  learningRate: number;
  parameters: VariationalParameters;
  forward: VariationalForwardTrace;
  backward: VariationalBackwardTrace;
  gradientCheck: {
    epsilon: number;
    parameterOrder: string[];
    analytical: number[];
    numerical: number[];
    maxAbsoluteError: number;
  };
  updatedParameters: VariationalParameters;
  postUpdate: VariationalForwardTrace;
}

export const DEFAULT_VARIATIONAL_PARAMETERS: VariationalParameters = {
  encoder: {
    mean: { weight: 0.4, bias: 0 },
    logVariance: { weight: 0, bias: 0 },
  },
  decoder: { weight: 1, bias: 0 },
};
export const DEFAULT_VARIATIONAL_INPUT = 1;
export const DEFAULT_VARIATIONAL_EPSILON = 0.5;
export const DEFAULT_VARIATIONAL_BETA = 0.1;
export const DEFAULT_VARIATIONAL_LEARNING_RATE = 0.1;
export const VARIATIONAL_PARAMETER_ORDER = [
  "encoder.mean.weight",
  "encoder.mean.bias",
  "encoder.log_variance.weight",
  "encoder.log_variance.bias",
  "decoder.weight",
  "decoder.bias",
];

function cleanZero(value: number): number {
  return Math.abs(value) < 1e-12 ? 0 : value;
}

function copyParameters(parameters: VariationalParameters): VariationalParameters {
  return {
    encoder: {
      mean: { ...parameters.encoder.mean },
      logVariance: { ...parameters.encoder.logVariance },
    },
    decoder: { ...parameters.decoder },
  };
}

function forwardVariational(
  input: number,
  parameters: VariationalParameters,
  epsilon: number,
  beta: number,
): VariationalForwardTrace {
  const meanProduct = cleanZero(input * parameters.encoder.mean.weight);
  const mean = cleanZero(meanProduct + parameters.encoder.mean.bias);
  const logVarianceProduct = cleanZero(
    input * parameters.encoder.logVariance.weight,
  );
  const logVariance = cleanZero(
    logVarianceProduct + parameters.encoder.logVariance.bias,
  );
  const variance = Math.exp(logVariance);
  const standardDeviation = Math.exp(0.5 * logVariance);
  const noiseContribution = cleanZero(standardDeviation * epsilon);
  const latent = cleanZero(mean + noiseContribution);
  const decoderProduct = cleanZero(latent * parameters.decoder.weight);
  const reconstruction = cleanZero(decoderProduct + parameters.decoder.bias);
  const error = cleanZero(reconstruction - input);
  const reconstructionLoss = 0.5 * error * error;
  const meanSquared = mean * mean;
  const kl = 0.5 * (meanSquared + variance - 1 - logVariance);
  const weightedKl = beta * kl;
  return {
    meanProduct,
    mean,
    logVarianceProduct,
    logVariance,
    variance,
    standardDeviation,
    epsilon,
    noiseContribution,
    latent,
    decoderProduct,
    reconstruction,
    error,
    reconstructionLoss,
    meanSquared,
    kl,
    weightedKl,
    totalLoss: reconstructionLoss + weightedKl,
  };
}

function flattenParameters(parameters: VariationalParameters): number[] {
  return [
    parameters.encoder.mean.weight,
    parameters.encoder.mean.bias,
    parameters.encoder.logVariance.weight,
    parameters.encoder.logVariance.bias,
    parameters.decoder.weight,
    parameters.decoder.bias,
  ];
}

function parametersFromFlat(values: readonly number[]): VariationalParameters {
  return {
    encoder: {
      mean: { weight: values[0]!, bias: values[1]! },
      logVariance: { weight: values[2]!, bias: values[3]! },
    },
    decoder: { weight: values[4]!, bias: values[5]! },
  };
}

export function traceScalarVariationalAutoencoder(
  beta = DEFAULT_VARIATIONAL_BETA,
  epsilon = DEFAULT_VARIATIONAL_EPSILON,
  learningRate = DEFAULT_VARIATIONAL_LEARNING_RATE,
  input = DEFAULT_VARIATIONAL_INPUT,
  parameters: VariationalParameters = DEFAULT_VARIATIONAL_PARAMETERS,
): VariationalTrace {
  const flatInput = flattenParameters(parameters);
  if (
    !Number.isFinite(beta)
    || beta < 0
    || !Number.isFinite(epsilon)
    || !Number.isFinite(learningRate)
    || learningRate <= 0
    || !Number.isFinite(input)
    || !flatInput.every(Number.isFinite)
  ) {
    throw new Error(
      "NN17 V1 needs finite scalar parameters, input and epsilon, non-negative beta, and a positive learning rate.",
    );
  }

  const parameterCopy = copyParameters(parameters);
  const forward = forwardVariational(input, parameterCopy, epsilon, beta);
  if (
    !Number.isFinite(forward.variance)
    || !Number.isFinite(forward.standardDeviation)
    || !Number.isFinite(forward.totalLoss)
  ) {
    throw new Error("NN17 V1 produced a non-finite Gaussian or objective.");
  }

  const reconstructionGradient = forward.error;
  const decoderWeightGradient = cleanZero(
    reconstructionGradient * forward.latent,
  );
  const decoderBiasGradient = reconstructionGradient;
  const latentGradient = cleanZero(
    reconstructionGradient * parameterCopy.decoder.weight,
  );
  const reconstructionMeanGradient = latentGradient;
  const reconstructionLogVarianceGradient = cleanZero(
    latentGradient * 0.5 * forward.standardDeviation * epsilon,
  );
  const klMeanGradient = forward.mean;
  const klLogVarianceGradient = cleanZero(0.5 * (forward.variance - 1));
  const weightedKlMeanGradient = cleanZero(beta * klMeanGradient);
  const weightedKlLogVarianceGradient = cleanZero(
    beta * klLogVarianceGradient,
  );
  const meanGradient = cleanZero(
    reconstructionMeanGradient + weightedKlMeanGradient,
  );
  const logVarianceGradient = cleanZero(
    reconstructionLogVarianceGradient + weightedKlLogVarianceGradient,
  );
  const meanWeightGradient = cleanZero(meanGradient * input);
  const meanBiasGradient = meanGradient;
  const logVarianceWeightGradient = cleanZero(logVarianceGradient * input);
  const logVarianceBiasGradient = logVarianceGradient;
  const backward: VariationalBackwardTrace = {
    reconstructionGradient,
    decoderWeightGradient,
    decoderBiasGradient,
    latentGradient,
    reconstructionMeanGradient,
    reconstructionLogVarianceGradient,
    klMeanGradient,
    klLogVarianceGradient,
    weightedKlMeanGradient,
    weightedKlLogVarianceGradient,
    meanGradient,
    logVarianceGradient,
    meanWeightGradient,
    meanBiasGradient,
    logVarianceWeightGradient,
    logVarianceBiasGradient,
  };

  const analytical = [
    meanWeightGradient,
    meanBiasGradient,
    logVarianceWeightGradient,
    logVarianceBiasGradient,
    decoderWeightGradient,
    decoderBiasGradient,
  ];
  const auditEpsilon = 1e-6;
  const numerical = flatInput.map((_, parameterIndex) => {
    const plus = [...flatInput];
    const minus = [...flatInput];
    plus[parameterIndex]! += auditEpsilon;
    minus[parameterIndex]! -= auditEpsilon;
    return (
      forwardVariational(
        input,
        parametersFromFlat(plus),
        epsilon,
        beta,
      ).totalLoss
      - forwardVariational(
        input,
        parametersFromFlat(minus),
        epsilon,
        beta,
      ).totalLoss
    ) / (2 * auditEpsilon);
  });
  const maxAbsoluteError = Math.max(...analytical.map((value, index) => (
    Math.abs(value - numerical[index]!)
  )));

  const updatedParameters = parametersFromFlat(
    flatInput.map((value, index) => (
      value - learningRate * analytical[index]!
    )),
  );
  const postUpdate = forwardVariational(
    input,
    updatedParameters,
    epsilon,
    beta,
  );

  return {
    input,
    beta,
    samplingEpsilon: epsilon,
    learningRate,
    parameters: parameterCopy,
    forward,
    backward,
    gradientCheck: {
      epsilon: auditEpsilon,
      parameterOrder: [...VARIATIONAL_PARAMETER_ORDER],
      analytical,
      numerical,
      maxAbsoluteError,
    },
    updatedParameters,
    postUpdate,
  };
}
