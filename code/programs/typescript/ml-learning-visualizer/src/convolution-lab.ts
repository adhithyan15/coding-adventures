export interface ConvolutionPositionTrace {
  outputIndex: number;
  startIndex: number;
  window: number[];
  products: number[];
  accumulator: number[];
  output: number;
}

export const DEFAULT_CONVOLUTION_SIGNAL = [2, 1, 3, 0, 4, 2] as const;
export const DEFAULT_CONVOLUTION_KERNEL = [1, -1, 2] as const;

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
      return product === 0 ? 0 : product;
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

export function parseNumberList(text: string): number[] | null {
  const pieces = text.split(",").map((piece) => piece.trim());
  if (pieces.length === 0 || pieces.some((piece) => piece === "")) {
    return null;
  }
  const values = pieces.map(Number);
  return values.every(Number.isFinite) ? values : null;
}
