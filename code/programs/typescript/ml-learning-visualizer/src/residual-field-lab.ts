export interface HiddenPathTrace {
  hiddenIndex: number;
  inputIndices: number[];
  inputValues: number[];
  subtotal: number;
}

export interface ResidualOutputTrace {
  outputIndex: number;
  hiddenIndices: number[];
  hiddenValues: number[];
  hiddenPaths: HiddenPathTrace[];
  inputPathCounts: number[];
  inputContributions: number[];
  receptiveFieldIndices: number[];
  mainOutput: number;
  skipContribution: number;
  residualSum: number;
  output: number;
}

export interface ResidualBlockTrace {
  hidden: number[];
  main: number[];
  skip: number[];
  residualSum: number[];
  output: number[];
  traces: ResidualOutputTrace[];
}

export const DEFAULT_RESIDUAL_INPUT = [1, 0, 2, 0, 1] as const;
export const DEFAULT_RESIDUAL_KERNELS = [
  [1, 1, 1],
  [1, 1, 1],
] as const;

function cleanZero(value: number): number {
  return value === 0 ? 0 : value;
}

export function sameCorrelation(
  signal: readonly number[],
  kernel: readonly number[],
): number[] {
  if (
    signal.length === 0
    || kernel.length === 0
    || kernel.length % 2 === 0
    || ![...signal, ...kernel].every(Number.isFinite)
  ) {
    throw new Error("Same correlation needs a finite signal and an odd kernel.");
  }
  const radius = Math.floor(kernel.length / 2);
  return signal.map((_, index) => cleanZero(
    kernel.reduce((total, weight, kernelIndex) => {
      const signalIndex = index + kernelIndex - radius;
      const signalValue = signalIndex >= 0 && signalIndex < signal.length
        ? signal[signalIndex]!
        : 0;
      return total + signalValue * weight;
    }, 0),
  ));
}

export function traceResidualBlock(
  input: readonly number[] = DEFAULT_RESIDUAL_INPUT,
  kernels: readonly (readonly number[])[] = DEFAULT_RESIDUAL_KERNELS,
): ResidualBlockTrace {
  if (
    kernels.length !== 2
    || kernels.some((kernel) => (
      kernel.length !== 3 || kernel.some((value) => value !== 1)
    ))
  ) {
    throw new Error("NN08 V1 uses two [1, 1, 1] kernels.");
  }
  const hidden = sameCorrelation(input, kernels[0]!);
  const main = sameCorrelation(hidden, kernels[1]!);
  const skip = [...input];
  const residualSum = main.map((value, index) => cleanZero(value + skip[index]!));
  const output = residualSum.map((value) => Math.max(0, value));
  const traces = input.map((_, outputIndex): ResidualOutputTrace => {
    const hiddenIndices = [outputIndex - 1, outputIndex, outputIndex + 1]
      .filter((index) => index >= 0 && index < input.length);
    const inputPathCounts = input.map(() => 0);
    const hiddenPaths = hiddenIndices.map((hiddenIndex): HiddenPathTrace => {
      const inputIndices = [hiddenIndex - 1, hiddenIndex, hiddenIndex + 1]
        .filter((index) => index >= 0 && index < input.length);
      inputIndices.forEach((inputIndex) => {
        inputPathCounts[inputIndex] = inputPathCounts[inputIndex]! + 1;
      });
      return {
        hiddenIndex,
        inputIndices,
        inputValues: inputIndices.map((inputIndex) => input[inputIndex]!),
        subtotal: hidden[hiddenIndex]!,
      };
    });
    return {
      outputIndex,
      hiddenIndices,
      hiddenValues: hiddenIndices.map((hiddenIndex) => hidden[hiddenIndex]!),
      hiddenPaths,
      inputPathCounts,
      inputContributions: input.map(
        (value, inputIndex) => cleanZero(value * inputPathCounts[inputIndex]!),
      ),
      receptiveFieldIndices: inputPathCounts
        .map((count, inputIndex) => ({ count, inputIndex }))
        .filter(({ count }) => count > 0)
        .map(({ inputIndex }) => inputIndex),
      mainOutput: main[outputIndex]!,
      skipContribution: skip[outputIndex]!,
      residualSum: residualSum[outputIndex]!,
      output: output[outputIndex]!,
    };
  });
  return { hidden, main, skip, residualSum, output, traces };
}
