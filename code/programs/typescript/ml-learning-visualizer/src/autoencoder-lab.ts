export interface AutoencoderParameters {
  encoder: {
    weights: number[];
    bias: number;
  };
  decoder: {
    weights: number[];
    bias: number[];
  };
}

export interface AutoencoderForwardTrace {
  encoderProducts: number[];
  bottleneck: number;
  decoderProducts: number[];
  reconstruction: number[];
  errors: number[];
  squaredErrors: number[];
  loss: number;
}

export interface AutoencoderBackwardTrace {
  reconstructionGradients: number[];
  decoderWeightGradients: number[];
  decoderBiasGradients: number[];
  bottleneckGradientContributions: number[];
  bottleneckGradient: number;
  encoderWeightGradients: number[];
  encoderBiasGradient: number;
}

export interface AutoencoderTrace {
  input: number[];
  learningRate: number;
  parameters: AutoencoderParameters;
  forward: AutoencoderForwardTrace;
  backward: AutoencoderBackwardTrace;
  gradientCheck: {
    epsilon: number;
    parameterOrder: string[];
    analytical: number[];
    numerical: number[];
    maxAbsoluteError: number;
  };
  updatedParameters: AutoencoderParameters;
  postUpdate: AutoencoderForwardTrace;
}

export const DEFAULT_AUTOENCODER_INPUT = [2, -1];
export const DEFAULT_AUTOENCODER_PARAMETERS: AutoencoderParameters = {
  encoder: {
    weights: [0.5, -0.25],
    bias: 0,
  },
  decoder: {
    weights: [1.2, -0.8],
    bias: [0.1, -0.2],
  },
};
export const DEFAULT_AUTOENCODER_LEARNING_RATE = 0.1;
export const AUTOENCODER_PARAMETER_ORDER = [
  "encoder.weights[0]",
  "encoder.weights[1]",
  "encoder.bias",
  "decoder.weights[0]",
  "decoder.weights[1]",
  "decoder.bias[0]",
  "decoder.bias[1]",
];

function cleanZero(value: number): number {
  return Math.abs(value) < 1e-12 ? 0 : value;
}

function validVector(values: readonly number[], length: number): boolean {
  return values.length === length && values.every(Number.isFinite);
}

function copyParameters(parameters: AutoencoderParameters): AutoencoderParameters {
  return {
    encoder: {
      weights: [...parameters.encoder.weights],
      bias: parameters.encoder.bias,
    },
    decoder: {
      weights: [...parameters.decoder.weights],
      bias: [...parameters.decoder.bias],
    },
  };
}

function forwardAutoencoder(
  input: readonly number[],
  parameters: AutoencoderParameters,
): AutoencoderForwardTrace {
  const encoderProducts = input.map((value, index) => cleanZero(
    value * parameters.encoder.weights[index]!,
  ));
  const bottleneck = cleanZero(
    encoderProducts.reduce((sum, value) => sum + value, 0)
    + parameters.encoder.bias,
  );
  const decoderProducts = parameters.decoder.weights.map((weight) => (
    cleanZero(bottleneck * weight)
  ));
  const reconstruction = decoderProducts.map((product, index) => cleanZero(
    product + parameters.decoder.bias[index]!,
  ));
  const errors = reconstruction.map((value, index) => cleanZero(
    value - input[index]!,
  ));
  const squaredErrors = errors.map((value) => value * value);
  return {
    encoderProducts,
    bottleneck,
    decoderProducts,
    reconstruction,
    errors,
    squaredErrors,
    loss: squaredErrors.reduce((sum, value) => sum + value, 0) / 2,
  };
}

function flattenParameters(parameters: AutoencoderParameters): number[] {
  return [
    ...parameters.encoder.weights,
    parameters.encoder.bias,
    ...parameters.decoder.weights,
    ...parameters.decoder.bias,
  ];
}

function parametersFromFlat(values: readonly number[]): AutoencoderParameters {
  return {
    encoder: {
      weights: values.slice(0, 2),
      bias: values[2]!,
    },
    decoder: {
      weights: values.slice(3, 5),
      bias: values.slice(5, 7),
    },
  };
}

export function traceTwoNumberAutoencoder(
  learningRate = DEFAULT_AUTOENCODER_LEARNING_RATE,
  input: readonly number[] = DEFAULT_AUTOENCODER_INPUT,
  parameters: AutoencoderParameters = DEFAULT_AUTOENCODER_PARAMETERS,
): AutoencoderTrace {
  if (
    !Number.isFinite(learningRate)
    || learningRate <= 0
    || !validVector(input, 2)
    || !validVector(parameters.encoder.weights, 2)
    || !Number.isFinite(parameters.encoder.bias)
    || !validVector(parameters.decoder.weights, 2)
    || !validVector(parameters.decoder.bias, 2)
  ) {
    throw new Error(
      "NN16 V1 needs a two-number input, 2 -> 1 -> 2 finite parameters, and a positive learning rate.",
    );
  }

  const parameterCopy = copyParameters(parameters);
  const forward = forwardAutoencoder(input, parameterCopy);
  const reconstructionGradients = [...forward.errors];
  const decoderWeightGradients = reconstructionGradients.map((gradient) => (
    cleanZero(gradient * forward.bottleneck)
  ));
  const decoderBiasGradients = [...reconstructionGradients];
  const bottleneckGradientContributions = reconstructionGradients.map(
    (gradient, index) => cleanZero(
      gradient * parameterCopy.decoder.weights[index]!,
    ),
  );
  const bottleneckGradient = cleanZero(
    bottleneckGradientContributions.reduce((sum, value) => sum + value, 0),
  );
  const encoderWeightGradients = input.map((value) => cleanZero(
    bottleneckGradient * value,
  ));
  const encoderBiasGradient = bottleneckGradient;
  const backward: AutoencoderBackwardTrace = {
    reconstructionGradients,
    decoderWeightGradients,
    decoderBiasGradients,
    bottleneckGradientContributions,
    bottleneckGradient,
    encoderWeightGradients,
    encoderBiasGradient,
  };

  const analytical = [
    ...encoderWeightGradients,
    encoderBiasGradient,
    ...decoderWeightGradients,
    ...decoderBiasGradients,
  ];
  const flatParameters = flattenParameters(parameterCopy);
  const epsilon = 1e-6;
  const numerical = flatParameters.map((_, parameterIndex) => {
    const plus = [...flatParameters];
    const minus = [...flatParameters];
    plus[parameterIndex]! += epsilon;
    minus[parameterIndex]! -= epsilon;
    return (
      forwardAutoencoder(input, parametersFromFlat(plus)).loss
      - forwardAutoencoder(input, parametersFromFlat(minus)).loss
    ) / (2 * epsilon);
  });
  const maxAbsoluteError = Math.max(...analytical.map((value, index) => (
    Math.abs(value - numerical[index]!)
  )));

  const updatedParameters: AutoencoderParameters = {
    encoder: {
      weights: parameterCopy.encoder.weights.map((value, index) => (
        value - learningRate * encoderWeightGradients[index]!
      )),
      bias: parameterCopy.encoder.bias - learningRate * encoderBiasGradient,
    },
    decoder: {
      weights: parameterCopy.decoder.weights.map((value, index) => (
        value - learningRate * decoderWeightGradients[index]!
      )),
      bias: parameterCopy.decoder.bias.map((value, index) => (
        value - learningRate * decoderBiasGradients[index]!
      )),
    },
  };
  const postUpdate = forwardAutoencoder(input, updatedParameters);

  return {
    input: [...input],
    learningRate,
    parameters: parameterCopy,
    forward,
    backward,
    gradientCheck: {
      epsilon,
      parameterOrder: [...AUTOENCODER_PARAMETER_ORDER],
      analytical,
      numerical,
      maxAbsoluteError,
    },
    updatedParameters,
    postUpdate,
  };
}
