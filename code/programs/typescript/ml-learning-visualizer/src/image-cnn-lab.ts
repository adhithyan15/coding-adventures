export type NumberMatrix = number[][];

export interface ImageInputChannel {
  name: string;
  values: NumberMatrix;
}

export interface ImageFilter {
  name: string;
  kernels: NumberMatrix[];
  bias: number;
}

export interface ImagePositionTrace {
  filterIndex: number;
  row: number;
  column: number;
  windows: NumberMatrix[];
  products: NumberMatrix[];
  channelSums: number[];
  preBiasSum: number;
  output: number;
}

export interface ImageNormalizationTrace {
  means: number[];
  variances: number[];
  denominators: number[];
  maps: NumberMatrix[];
}

export interface ImagePoolingTrace {
  values: number[];
  argmax: [number, number][];
}

export interface TinyImageCnnTrace {
  positions: ImagePositionTrace[][][];
  channelContributions: NumberMatrix[][];
  convolution: NumberMatrix[];
  normalization: ImageNormalizationTrace;
  activation: NumberMatrix[];
  pooling: ImagePoolingTrace;
}

export const DEFAULT_IMAGE_CHANNELS: ImageInputChannel[] = [
  {
    name: "vertical-position",
    values: [[0, 0, 0], [1, 1, 1], [2, 2, 2]],
  },
  {
    name: "horizontal-position",
    values: [[0, 1, 2], [0, 1, 2], [0, 1, 2]],
  },
];

export const DEFAULT_IMAGE_FILTERS: ImageFilter[] = [
  {
    name: "toward-bottom-right",
    kernels: [
      [[4, 0], [0, 0]],
      [[2, 0], [0, 0]],
    ],
    bias: 0,
  },
  {
    name: "toward-top-left",
    kernels: [
      [[-4, 0], [0, 0]],
      [[-2, 0], [0, 0]],
    ],
    bias: 6,
  },
];

export const DEFAULT_IMAGE_EPSILON = 4;
export const DEFAULT_IMAGE_GAMMA = [1, 1] as const;
export const DEFAULT_IMAGE_BETA = [0, 0] as const;

function cleanZero(value: number): number {
  return value === 0 ? 0 : value;
}

function matrixShape(matrix: readonly (readonly number[])[]): [number, number] {
  if (matrix.length === 0 || matrix[0]!.length === 0) {
    throw new Error("Matrices must contain at least one value.");
  }
  const width = matrix[0]!.length;
  if (matrix.some((row) => row.length !== width || !row.every(Number.isFinite))) {
    throw new Error("Matrices must be rectangular and contain finite numbers.");
  }
  return [matrix.length, width];
}

function spatialNormalize(
  maps: readonly NumberMatrix[],
  epsilon: number,
  gamma: readonly number[],
  beta: readonly number[],
): ImageNormalizationTrace {
  if (!Number.isFinite(epsilon) || epsilon <= 0) {
    throw new Error("Normalization epsilon must be positive.");
  }
  if (gamma.length !== maps.length || beta.length !== maps.length) {
    throw new Error("Gamma and beta must match the output channel count.");
  }
  const means: number[] = [];
  const variances: number[] = [];
  const denominators: number[] = [];
  const normalizedMaps = maps.map((featureMap, filterIndex) => {
    const values = featureMap.flat();
    const mean = values.reduce((total, value) => total + value, 0) / values.length;
    const variance = values.reduce(
      (total, value) => total + (value - mean) ** 2,
      0,
    ) / values.length;
    const denominator = Math.sqrt(variance + epsilon);
    means.push(mean);
    variances.push(variance);
    denominators.push(denominator);
    return featureMap.map((row) => row.map((value) => cleanZero(
      gamma[filterIndex]! * (value - mean) / denominator + beta[filterIndex]!,
    )));
  });
  return { means, variances, denominators, maps: normalizedMaps };
}

function maxPoolEntireMaps(maps: readonly NumberMatrix[]): ImagePoolingTrace {
  const values: number[] = [];
  const argmax: [number, number][] = [];
  for (const featureMap of maps) {
    let bestValue = Number.NEGATIVE_INFINITY;
    let bestPosition: [number, number] = [0, 0];
    for (const [rowIndex, row] of featureMap.entries()) {
      for (const [columnIndex, value] of row.entries()) {
        if (value > bestValue) {
          bestValue = value;
          bestPosition = [rowIndex, columnIndex];
        }
      }
    }
    values.push(bestValue);
    argmax.push(bestPosition);
  }
  return { values, argmax };
}

export function traceTinyImageCnn(
  channels: readonly ImageInputChannel[] = DEFAULT_IMAGE_CHANNELS,
  filters: readonly ImageFilter[] = DEFAULT_IMAGE_FILTERS,
  epsilon = DEFAULT_IMAGE_EPSILON,
  gamma: readonly number[] = DEFAULT_IMAGE_GAMMA,
  beta: readonly number[] = DEFAULT_IMAGE_BETA,
): TinyImageCnnTrace {
  if (channels.length === 0 || filters.length === 0) {
    throw new Error("The image and filter bank must be non-empty.");
  }
  const [inputHeight, inputWidth] = matrixShape(channels[0]!.values);
  if (channels.some((channel) => {
    const shape = matrixShape(channel.values);
    return shape[0] !== inputHeight || shape[1] !== inputWidth;
  })) {
    throw new Error("Every input channel must have the same image shape.");
  }

  const positions: ImagePositionTrace[][][] = [];
  const channelContributions: NumberMatrix[][] = [];
  const convolution: NumberMatrix[] = [];
  for (const [filterIndex, filter] of filters.entries()) {
    if (!Number.isFinite(filter.bias) || filter.kernels.length !== channels.length) {
      throw new Error("Every filter needs a finite bias and one kernel per input channel.");
    }
    const [kernelHeight, kernelWidth] = matrixShape(filter.kernels[0]!);
    if (filter.kernels.some((kernel) => {
      const shape = matrixShape(kernel);
      return shape[0] !== kernelHeight || shape[1] !== kernelWidth;
    })) {
      throw new Error("Every kernel in one filter must have the same shape.");
    }
    if (kernelHeight > inputHeight || kernelWidth > inputWidth) {
      throw new Error("Kernels must fit inside the image in valid mode.");
    }
    const outputHeight = inputHeight - kernelHeight + 1;
    const outputWidth = inputWidth - kernelWidth + 1;
    const filterContributions = channels.map(
      () => Array.from({ length: outputHeight }, () => Array(outputWidth).fill(0) as number[]),
    );
    const filterPositions: ImagePositionTrace[][] = [];
    const outputMap: NumberMatrix = [];
    for (let row = 0; row < outputHeight; row += 1) {
      const positionRow: ImagePositionTrace[] = [];
      const outputRow: number[] = [];
      for (let column = 0; column < outputWidth; column += 1) {
        const windows = channels.map((channel) => (
          Array.from({ length: kernelHeight }, (_, kernelRow) => (
            channel.values[row + kernelRow]!.slice(column, column + kernelWidth)
          ))
        ));
        const products = windows.map((window, channelIndex) => (
          window.map((windowRow, kernelRow) => (
            windowRow.map((value, kernelColumn) => cleanZero(
              value * filter.kernels[channelIndex]![kernelRow]![kernelColumn]!,
            ))
          ))
        ));
        const channelSums = products.map((product) => cleanZero(
          product.flat().reduce((total, value) => total + value, 0),
        ));
        const preBiasSum = cleanZero(
          channelSums.reduce((total, value) => total + value, 0),
        );
        const output = cleanZero(preBiasSum + filter.bias);
        channelSums.forEach((sum, channelIndex) => {
          filterContributions[channelIndex]![row]![column] = sum;
        });
        positionRow.push({
          filterIndex,
          row,
          column,
          windows,
          products,
          channelSums,
          preBiasSum,
          output,
        });
        outputRow.push(output);
      }
      filterPositions.push(positionRow);
      outputMap.push(outputRow);
    }
    positions.push(filterPositions);
    channelContributions.push(filterContributions);
    convolution.push(outputMap);
  }

  const normalization = spatialNormalize(convolution, epsilon, gamma, beta);
  const activation = normalization.maps.map((featureMap) => (
    featureMap.map((row) => row.map((value) => Math.max(0, value)))
  ));
  return {
    positions,
    channelContributions,
    convolution,
    normalization,
    activation,
    pooling: maxPoolEntireMaps(activation),
  };
}
