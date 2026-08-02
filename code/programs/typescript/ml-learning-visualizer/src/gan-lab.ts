export interface ScalarAffineParameters {
  weight: number;
  bias: number;
}

export interface GanParameters {
  generator: ScalarAffineParameters;
  discriminator: ScalarAffineParameters;
}

export interface GanState {
  generatorProduct: number;
  fakeSample: number;
  realLogit: number;
  realProbability: number;
  fakeLogit: number;
  fakeProbability: number;
  discriminatorLoss: number;
  generatorLoss: number;
}

export interface GradientAudit {
  epsilon: number;
  parameterOrder: string[];
  analytical: number[];
  numerical: number[];
  maxAbsoluteError: number;
}

export interface OneDimensionalGanTrace {
  realSample: number;
  savedNoise: number;
  discriminatorLearningRate: number;
  generatorLearningRate: number;
  parameters: GanParameters;
  initial: GanState;
  discriminatorStep: {
    backward: {
      realLogitGradient: number;
      fakeLogitGradient: number;
      weightGradient: number;
      biasGradient: number;
      fakeSampleGradient: number;
    };
    updatedParameters: ScalarAffineParameters;
    state: GanState;
    gradientCheck: GradientAudit;
  };
  generatorStep: {
    backward: {
      fakeLogitGradient: number;
      fakeSampleGradient: number;
      weightGradient: number;
      biasGradient: number;
    };
    updatedParameters: ScalarAffineParameters;
    state: GanState;
    gradientCheck: GradientAudit;
  };
}

export const DEFAULT_GAN_PARAMETERS: GanParameters = {
  generator: { weight: 0.2, bias: 0 },
  discriminator: { weight: 1, bias: 0 },
};
export const DEFAULT_GAN_REAL_SAMPLE = 1;
export const DEFAULT_GAN_SAVED_NOISE = 1;
export const DEFAULT_DISCRIMINATOR_LEARNING_RATE = 0.5;
export const DEFAULT_GENERATOR_LEARNING_RATE = 0.25;

function cleanZero(value: number): number {
  return Math.abs(value) < 1e-12 ? 0 : value;
}

function sigmoid(value: number): number {
  if (value >= 0) {
    return 1 / (1 + Math.exp(-value));
  }
  const exponential = Math.exp(value);
  return exponential / (1 + exponential);
}

function forwardGan(
  realSample: number,
  savedNoise: number,
  parameters: GanParameters,
): GanState {
  const generatorProduct = cleanZero(
    savedNoise * parameters.generator.weight,
  );
  const fakeSample = cleanZero(
    generatorProduct + parameters.generator.bias,
  );
  const realLogit = cleanZero(
    realSample * parameters.discriminator.weight
      + parameters.discriminator.bias,
  );
  const fakeLogit = cleanZero(
    fakeSample * parameters.discriminator.weight
      + parameters.discriminator.bias,
  );
  const realProbability = sigmoid(realLogit);
  const fakeProbability = sigmoid(fakeLogit);
  const discriminatorLoss = -0.5 * (
    Math.log(realProbability) + Math.log(1 - fakeProbability)
  );
  const generatorLoss = -Math.log(fakeProbability);
  return {
    generatorProduct,
    fakeSample,
    realLogit,
    realProbability,
    fakeLogit,
    fakeProbability,
    discriminatorLoss,
    generatorLoss,
  };
}

function finiteDifference(
  values: readonly number[],
  lossFunction: (candidate: readonly number[]) => number,
): Omit<GradientAudit, "parameterOrder" | "analytical"> {
  const epsilon = 1e-6;
  const numerical = values.map((_, parameterIndex) => {
    const plus = [...values];
    const minus = [...values];
    plus[parameterIndex]! += epsilon;
    minus[parameterIndex]! -= epsilon;
    return (lossFunction(plus) - lossFunction(minus)) / (2 * epsilon);
  });
  return { epsilon, numerical, maxAbsoluteError: 0 };
}

function maxAbsoluteError(
  analytical: readonly number[],
  numerical: readonly number[],
): number {
  return Math.max(
    ...analytical.map((value, index) => (
      Math.abs(value - numerical[index]!)
    )),
  );
}

export function traceOneDimensionalGan(
  realSample = DEFAULT_GAN_REAL_SAMPLE,
  savedNoise = DEFAULT_GAN_SAVED_NOISE,
  discriminatorLearningRate = DEFAULT_DISCRIMINATOR_LEARNING_RATE,
  generatorLearningRate = DEFAULT_GENERATOR_LEARNING_RATE,
  parameters: GanParameters = DEFAULT_GAN_PARAMETERS,
): OneDimensionalGanTrace {
  const scalarInputs = [
    realSample,
    savedNoise,
    discriminatorLearningRate,
    generatorLearningRate,
    parameters.generator.weight,
    parameters.generator.bias,
    parameters.discriminator.weight,
    parameters.discriminator.bias,
  ];
  if (
    !scalarInputs.every(Number.isFinite)
    || discriminatorLearningRate <= 0
    || generatorLearningRate <= 0
  ) {
    throw new Error(
      "NN18 V1 needs finite scalar samples and parameters, plus positive learning rates.",
    );
  }

  const parameterCopy: GanParameters = {
    generator: { ...parameters.generator },
    discriminator: { ...parameters.discriminator },
  };
  const initial = forwardGan(realSample, savedNoise, parameterCopy);

  const realLogitGradient = 0.5 * (initial.realProbability - 1);
  const fakeLogitGradient = 0.5 * initial.fakeProbability;
  const discriminatorWeightGradient = cleanZero(
    realLogitGradient * realSample
      + fakeLogitGradient * initial.fakeSample,
  );
  const discriminatorBiasGradient = cleanZero(
    realLogitGradient + fakeLogitGradient,
  );
  const discriminatorAnalytical = [
    discriminatorWeightGradient,
    discriminatorBiasGradient,
  ];
  const discriminatorNumerical = finiteDifference(
    [parameterCopy.discriminator.weight, parameterCopy.discriminator.bias],
    ([weight, bias]) => {
      const realProbability = sigmoid(realSample * weight! + bias!);
      const fakeProbability = sigmoid(initial.fakeSample * weight! + bias!);
      return -0.5 * (
        Math.log(realProbability) + Math.log(1 - fakeProbability)
      );
    },
  );
  const updatedDiscriminator = {
    weight: cleanZero(
      parameterCopy.discriminator.weight
        - discriminatorLearningRate * discriminatorWeightGradient,
    ),
    bias: cleanZero(
      parameterCopy.discriminator.bias
        - discriminatorLearningRate * discriminatorBiasGradient,
    ),
  };
  const afterDiscriminator = forwardGan(realSample, savedNoise, {
    generator: parameterCopy.generator,
    discriminator: updatedDiscriminator,
  });

  const generatorFakeLogitGradient = afterDiscriminator.fakeProbability - 1;
  const generatorFakeSampleGradient = cleanZero(
    generatorFakeLogitGradient * updatedDiscriminator.weight,
  );
  const generatorWeightGradient = cleanZero(
    generatorFakeSampleGradient * savedNoise,
  );
  const generatorBiasGradient = generatorFakeSampleGradient;
  const generatorAnalytical = [
    generatorWeightGradient,
    generatorBiasGradient,
  ];
  const generatorNumerical = finiteDifference(
    [parameterCopy.generator.weight, parameterCopy.generator.bias],
    ([weight, bias]) => {
      const fakeSample = savedNoise * weight! + bias!;
      const fakeProbability = sigmoid(
        fakeSample * updatedDiscriminator.weight + updatedDiscriminator.bias,
      );
      return -Math.log(fakeProbability);
    },
  );
  const updatedGenerator = {
    weight: cleanZero(
      parameterCopy.generator.weight
        - generatorLearningRate * generatorWeightGradient,
    ),
    bias: cleanZero(
      parameterCopy.generator.bias
        - generatorLearningRate * generatorBiasGradient,
    ),
  };
  const afterGenerator = forwardGan(realSample, savedNoise, {
    generator: updatedGenerator,
    discriminator: updatedDiscriminator,
  });

  return {
    realSample,
    savedNoise,
    discriminatorLearningRate,
    generatorLearningRate,
    parameters: parameterCopy,
    initial,
    discriminatorStep: {
      backward: {
        realLogitGradient,
        fakeLogitGradient,
        weightGradient: discriminatorWeightGradient,
        biasGradient: discriminatorBiasGradient,
        fakeSampleGradient: 0,
      },
      updatedParameters: updatedDiscriminator,
      state: afterDiscriminator,
      gradientCheck: {
        epsilon: discriminatorNumerical.epsilon,
        parameterOrder: ["discriminator.weight", "discriminator.bias"],
        analytical: discriminatorAnalytical,
        numerical: discriminatorNumerical.numerical,
        maxAbsoluteError: maxAbsoluteError(
          discriminatorAnalytical,
          discriminatorNumerical.numerical,
        ),
      },
    },
    generatorStep: {
      backward: {
        fakeLogitGradient: generatorFakeLogitGradient,
        fakeSampleGradient: generatorFakeSampleGradient,
        weightGradient: generatorWeightGradient,
        biasGradient: generatorBiasGradient,
      },
      updatedParameters: updatedGenerator,
      state: afterGenerator,
      gradientCheck: {
        epsilon: generatorNumerical.epsilon,
        parameterOrder: ["generator.weight", "generator.bias"],
        analytical: generatorAnalytical,
        numerical: generatorNumerical.numerical,
        maxAbsoluteError: maxAbsoluteError(
          generatorAnalytical,
          generatorNumerical.numerical,
        ),
      },
    },
  };
}
