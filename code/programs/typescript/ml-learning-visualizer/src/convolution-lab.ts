export interface ConvolutionPositionTrace {
  outputIndex: number;
  startIndex: number;
  window: number[];
  products: number[];
  accumulator: number[];
  output: number;
}

export interface ConvolutionGradientContribution {
  outputIndex: number;
  window: number[];
  outputGradient: number;
  kernelGradient: number[];
}

export interface ConvolutionTrainingTrace {
  outputs: number[];
  errors: number[];
  loss: number;
  outputGradients: number[];
  contributions: ConvolutionGradientContribution[];
  kernelGradient: number[];
}

export interface ConvolutionStepProposal {
  nextKernel: number[];
  nextOutputs: number[];
  nextLoss: number;
}

export const DEFAULT_CONVOLUTION_SIGNAL = [2, 1, 3, 0, 4, 2] as const;
export const DEFAULT_CONVOLUTION_KERNEL = [1, -1, 2] as const;
export const DEFAULT_CONVOLUTION_TARGETS = [6, -2, 10, 0] as const;
export const DEFAULT_CONVOLUTION_LEARNING_RATE = 0.02;
export const DEFAULT_CONVOLUTION_EPSILON = 0.000001;

function cleanZero(value: number): number {
  return value === 0 ? 0 : value;
}

export function traceValidCorrelation(
  signal: readonly number[],
  kernel: readonly number[],
): ConvolutionPositionTrace[] {
  if (signal.length === 0 || kernel.length === 0) {
    throw new Error("Signal and kernel must contain at least one number.");
  }
  if (kernel.length > signal.length) {
    throw new Error("The kernel cannot be longer than the signal in valid mode.");
  }
  if (![...signal, ...kernel].every(Number.isFinite)) {
    throw new Error("Signal and kernel values must be finite numbers.");
  }

  return Array.from({ length: signal.length - kernel.length + 1 }, (_, startIndex) => {
    const window = signal.slice(startIndex, startIndex + kernel.length);
    const products = window.map((value, index) => {
      const product = value * kernel[index]!;
      return cleanZero(product);
    });
    const accumulator = products.reduce<number[]>(
      (values, product) => [...values, values[values.length - 1]! + product],
      [0],
    );
    return {
      outputIndex: startIndex,
      startIndex,
      window,
      products,
      accumulator,
      output: accumulator[accumulator.length - 1]!,
    };
  });
}

export function meanSquaredError(
  outputs: readonly number[],
  targets: readonly number[],
): number {
  if (outputs.length === 0 || outputs.length !== targets.length) {
    throw new Error("Outputs and targets must have the same non-zero length.");
  }
  return outputs.reduce(
    (total, output, index) => total + (output - targets[index]!) ** 2,
    0,
  ) / outputs.length;
}

export function traceConvolutionTraining(
  signal: readonly number[],
  kernel: readonly number[],
  targets: readonly number[],
): ConvolutionTrainingTrace {
  const positionTraces = traceValidCorrelation(signal, kernel);
  const outputs = positionTraces.map((position) => position.output);
  if (targets.length !== outputs.length || !targets.every(Number.isFinite)) {
    throw new Error(`Expected ${outputs.length} finite target values.`);
  }
  const errors = outputs.map((output, index) => cleanZero(output - targets[index]!));
  const outputGradients = errors.map((error) => cleanZero((2 * error) / errors.length));
  const kernelGradient = kernel.map(() => 0);
  const contributions = positionTraces.map((position, outputIndex) => {
    const outputGradient = outputGradients[outputIndex]!;
    const contribution = position.window.map((value, kernelIndex) => {
      const gradient = cleanZero(outputGradient * value);
      kernelGradient[kernelIndex] = cleanZero(kernelGradient[kernelIndex]! + gradient);
      return gradient;
    });
    return {
      outputIndex,
      window: position.window,
      outputGradient,
      kernelGradient: contribution,
    };
  });
  return {
    outputs,
    errors,
    loss: meanSquaredError(outputs, targets),
    outputGradients,
    contributions,
    kernelGradient,
  };
}

export function numericalKernelGradient(
  signal: readonly number[],
  kernel: readonly number[],
  targets: readonly number[],
  epsilon = DEFAULT_CONVOLUTION_EPSILON,
): number[] {
  if (!Number.isFinite(epsilon) || epsilon <= 0) {
    throw new Error("Finite-difference epsilon must be positive.");
  }
  return kernel.map((_, index) => {
    const plus = [...kernel];
    const minus = [...kernel];
    plus[index] += epsilon;
    minus[index] -= epsilon;
    const plusOutputs = traceValidCorrelation(signal, plus).map((position) => position.output);
    const minusOutputs = traceValidCorrelation(signal, minus).map((position) => position.output);
    return (
      meanSquaredError(plusOutputs, targets) - meanSquaredError(minusOutputs, targets)
    ) / (2 * epsilon);
  });
}

export function proposeConvolutionStep(
  signal: readonly number[],
  kernel: readonly number[],
  targets: readonly number[],
  learningRate: number,
): ConvolutionStepProposal {
  if (!Number.isFinite(learningRate) || learningRate <= 0) {
    throw new Error("Learning rate must be positive.");
  }
  const training = traceConvolutionTraining(signal, kernel, targets);
  const nextKernel = kernel.map(
    (value, index) => cleanZero(value - learningRate * training.kernelGradient[index]!),
  );
  const nextOutputs = traceValidCorrelation(signal, nextKernel).map(
    (position) => position.output,
  );
  return {
    nextKernel,
    nextOutputs,
    nextLoss: meanSquaredError(nextOutputs, targets),
  };
}

export function parseNumberList(text: string): number[] | null {
  const pieces = text.split(",").map((piece) => piece.trim());
  if (pieces.length === 0 || pieces.some((piece) => piece === "")) {
    return null;
  }
  const values = pieces.map(Number);
  return values.every(Number.isFinite) ? values : null;
}
