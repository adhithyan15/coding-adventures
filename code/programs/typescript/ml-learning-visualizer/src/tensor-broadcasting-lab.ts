export type TensorBroadcastScenarioId =
  | "outer-grid"
  | "row-over-batch"
  | "scalar-over-matrix"
  | "incompatible-tail";

export interface TensorOperand {
  shape: readonly number[];
  values: readonly number[];
}

export interface TensorBroadcastScenario {
  id: TensorBroadcastScenarioId;
  title: string;
  summary: string;
  left: TensorOperand;
  right: TensorOperand;
  upstream: TensorOperand | null;
}

export interface BroadcastMapping {
  outputIndex: number[];
  outputFlatIndex: number;
  leftIndex: number[];
  leftFlatIndex: number;
  rightIndex: number[];
  rightFlatIndex: number;
  leftValue: number;
  rightValue: number;
  outputValue: number;
  upstream: number;
}

interface BroadcastTraceBase {
  compatible: boolean;
  left: TensorOperand;
  right: TensorOperand;
  paddedLeftShape: number[];
  paddedRightShape: number[];
}

export interface CompatibleBroadcastTrace extends BroadcastTraceBase {
  compatible: true;
  upstream: TensorOperand;
  outputShape: number[];
  leftExpandedAxes: number[];
  rightExpandedAxes: number[];
  outputValues: number[];
  mappings: BroadcastMapping[];
  leftGradient: number[];
  rightGradient: number[];
  finiteDifferenceLeftGradient: number[];
  finiteDifferenceRightGradient: number[];
  maxGradientAbsoluteError: number;
}

export interface IncompatibleBroadcastTrace extends BroadcastTraceBase {
  compatible: false;
  upstream: null;
  mismatchAxis: number;
  leftDimension: number;
  rightDimension: number;
  error: string;
}

export type BroadcastTrace = CompatibleBroadcastTrace | IncompatibleBroadcastTrace;
export type ScenarioBroadcastTrace = BroadcastTrace & Pick<TensorBroadcastScenario, "id" | "title" | "summary">;

const MAX_RANK = 4;
const MAX_DIMENSION = 8;
const MAX_VALUES = 64;
const MAX_ABSOLUTE_VALUE = 1e6;

export const TENSOR_BROADCAST_SCENARIOS: readonly TensorBroadcastScenario[] = [
  {
    id: "outer-grid",
    title: "Column + row",
    summary: "Both inputs expand along one axis.",
    left: { shape: [2, 1], values: [1, 2] },
    right: { shape: [1, 3], values: [10, 20, 30] },
    upstream: { shape: [2, 3], values: [1, 2, 3, 4, 5, 6] },
  },
  {
    id: "row-over-batch",
    title: "Matrix + rank-one row",
    summary: "Right alignment turns [3] into [1, 3].",
    left: { shape: [2, 3], values: [1, 2, 3, 4, 5, 6] },
    right: { shape: [3], values: [10, 20, 30] },
    upstream: { shape: [2, 3], values: [1, 1, 1, 1, 1, 1] },
  },
  {
    id: "scalar-over-matrix",
    title: "Scalar + matrix",
    summary: "A rank-zero value reaches every output cell.",
    left: { shape: [], values: [2] },
    right: { shape: [2, 2], values: [1, 2, 3, 4] },
    upstream: { shape: [2, 2], values: [1, -1, 2, -2] },
  },
  {
    id: "incompatible-tail",
    title: "Mismatch",
    summary: "Trailing dimensions 3 and 2 cannot align.",
    left: { shape: [2, 3], values: [1, 2, 3, 4, 5, 6] },
    right: { shape: [2], values: [10, 20] },
    upstream: null,
  },
] as const;

function elementCount(shape: readonly number[]): number {
  return shape.reduce((count, dimension) => count * dimension, 1);
}

function validateTensor(tensor: TensorOperand, name: string): void {
  if (typeof tensor !== "object" || tensor === null
    || !Array.isArray(tensor.shape) || !Array.isArray(tensor.values)) {
    throw new Error(`${name} must contain shape and values arrays`);
  }
  if (tensor.shape.length > MAX_RANK) {
    throw new Error(`${name} shape must contain at most ${MAX_RANK} dimensions`);
  }
  tensor.shape.forEach((dimension) => {
    if (!Number.isInteger(dimension) || dimension <= 0 || dimension > MAX_DIMENSION) {
      throw new Error(`${name} dimensions must be positive integers up to ${MAX_DIMENSION}`);
    }
  });
  const count = elementCount(tensor.shape);
  if (count > MAX_VALUES || tensor.values.length !== count) {
    throw new Error(`${name} values must match its bounded shape`);
  }
  if (!tensor.values.every((value) => (
    Number.isFinite(value) && Math.abs(value) <= MAX_ABSOLUTE_VALUE
  ))) {
    throw new Error(`${name} values must be finite and bounded`);
  }
}

function requireFinite(value: number, context: string): number {
  if (!Number.isFinite(value)) {
    throw new Error(`${context} must remain finite`);
  }
  return value;
}

function rowMajorStrides(shape: readonly number[]): number[] {
  const result = new Array<number>(shape.length).fill(0);
  let stride = 1;
  for (let index = shape.length - 1; index >= 0; index -= 1) {
    result[index] = stride;
    stride *= shape[index]!;
  }
  return result;
}

function unravel(flatIndex: number, shape: readonly number[]): number[] {
  return rowMajorStrides(shape).map((stride) => {
    const coordinate = Math.floor(flatIndex / stride);
    flatIndex %= stride;
    return coordinate;
  });
}

function flatten(index: readonly number[], shape: readonly number[]): number {
  return index.reduce(
    (flatIndex, coordinate, axis) => flatIndex + coordinate * rowMajorStrides(shape)[axis]!,
    0,
  );
}

function padShapes(leftShape: readonly number[], rightShape: readonly number[]): [number[], number[]] {
  const rank = Math.max(leftShape.length, rightShape.length);
  return [
    [...new Array<number>(rank - leftShape.length).fill(1), ...leftShape],
    [...new Array<number>(rank - rightShape.length).fill(1), ...rightShape],
  ];
}

function score(
  leftValues: readonly number[],
  rightValues: readonly number[],
  mappings: readonly BroadcastMapping[],
): number {
  let total = 0;
  mappings.forEach((mapping) => {
    const output = requireFinite(
      leftValues[mapping.leftFlatIndex]! + rightValues[mapping.rightFlatIndex]!,
      "broadcast score output",
    );
    const contribution = requireFinite(mapping.upstream * output, "broadcast score contribution");
    total = requireFinite(total + contribution, "broadcast score");
  });
  return total;
}

export function traceBroadcastAdd(
  left: TensorOperand,
  right: TensorOperand,
  upstream: TensorOperand | null,
  finiteDifferenceEpsilon = 1e-5,
): BroadcastTrace {
  validateTensor(left, "left tensor");
  validateTensor(right, "right tensor");
  if (!Number.isFinite(finiteDifferenceEpsilon)
    || finiteDifferenceEpsilon < 1e-12 || finiteDifferenceEpsilon > 1) {
    throw new Error("finite-difference epsilon must be finite and in [1e-12, 1]");
  }
  const [paddedLeftShape, paddedRightShape] = padShapes(left.shape, right.shape);
  const outputShape: number[] = [];
  for (let axis = 0; axis < paddedLeftShape.length; axis += 1) {
    const leftDimension = paddedLeftShape[axis]!;
    const rightDimension = paddedRightShape[axis]!;
    if (leftDimension !== rightDimension && leftDimension !== 1 && rightDimension !== 1) {
      return {
        compatible: false,
        left,
        right,
        upstream: null,
        paddedLeftShape,
        paddedRightShape,
        mismatchAxis: axis,
        leftDimension,
        rightDimension,
        error: `axis ${axis}: dimensions ${leftDimension} and ${rightDimension} are incompatible`,
      };
    }
    outputShape.push(Math.max(leftDimension, rightDimension));
  }

  if (upstream === null) {
    throw new Error("compatible shapes require an upstream tensor");
  }
  validateTensor(upstream, "upstream tensor");
  if (upstream.shape.length !== outputShape.length
    || upstream.shape.some((dimension, axis) => dimension !== outputShape[axis])) {
    throw new Error(`upstream shape must equal output shape [${outputShape.join(", ")}]`);
  }

  const rank = outputShape.length;
  const leftLeadingAxes = rank - left.shape.length;
  const rightLeadingAxes = rank - right.shape.length;
  const mappings: BroadcastMapping[] = [];
  for (let outputFlatIndex = 0; outputFlatIndex < elementCount(outputShape); outputFlatIndex += 1) {
    const outputIndex = unravel(outputFlatIndex, outputShape);
    const paddedLeftIndex = outputIndex.map((coordinate, axis) => (
      paddedLeftShape[axis] === 1 ? 0 : coordinate
    ));
    const paddedRightIndex = outputIndex.map((coordinate, axis) => (
      paddedRightShape[axis] === 1 ? 0 : coordinate
    ));
    const leftIndex = paddedLeftIndex.slice(leftLeadingAxes);
    const rightIndex = paddedRightIndex.slice(rightLeadingAxes);
    const leftFlatIndex = flatten(leftIndex, left.shape);
    const rightFlatIndex = flatten(rightIndex, right.shape);
    const leftValue = left.values[leftFlatIndex]!;
    const rightValue = right.values[rightFlatIndex]!;
    const outputValue = requireFinite(leftValue + rightValue, "broadcast output");
    mappings.push({
      outputIndex,
      outputFlatIndex,
      leftIndex,
      leftFlatIndex,
      rightIndex,
      rightFlatIndex,
      leftValue,
      rightValue,
      outputValue,
      upstream: upstream.values[outputFlatIndex]!,
    });
  }

  const leftGradient = new Array<number>(left.values.length).fill(0);
  const rightGradient = new Array<number>(right.values.length).fill(0);
  mappings.forEach((mapping) => {
    leftGradient[mapping.leftFlatIndex] = requireFinite(
      leftGradient[mapping.leftFlatIndex]! + mapping.upstream,
      "left broadcast gradient",
    );
    rightGradient[mapping.rightFlatIndex] = requireFinite(
      rightGradient[mapping.rightFlatIndex]! + mapping.upstream,
      "right broadcast gradient",
    );
  });
  const finiteDifferenceLeftGradient = left.values.map((_, index) => {
    const positive = [...left.values];
    const negative = [...left.values];
    positive[index]! += finiteDifferenceEpsilon;
    negative[index]! -= finiteDifferenceEpsilon;
    return requireFinite(
      (score(positive, right.values, mappings) - score(negative, right.values, mappings))
        / (2 * finiteDifferenceEpsilon),
      "left finite-difference gradient",
    );
  });
  const finiteDifferenceRightGradient = right.values.map((_, index) => {
    const positive = [...right.values];
    const negative = [...right.values];
    positive[index]! += finiteDifferenceEpsilon;
    negative[index]! -= finiteDifferenceEpsilon;
    return requireFinite(
      (score(left.values, positive, mappings) - score(left.values, negative, mappings))
        / (2 * finiteDifferenceEpsilon),
      "right finite-difference gradient",
    );
  });
  const errors = [
    ...leftGradient.map((value, index) => Math.abs(value - finiteDifferenceLeftGradient[index]!)),
    ...rightGradient.map((value, index) => Math.abs(value - finiteDifferenceRightGradient[index]!)),
  ];
  const maxGradientAbsoluteError = requireFinite(Math.max(...errors, 0), "gradient error");
  return {
    compatible: true,
    left,
    right,
    upstream,
    paddedLeftShape,
    paddedRightShape,
    outputShape,
    leftExpandedAxes: paddedLeftShape.flatMap((dimension, axis) => (
      dimension === 1 && outputShape[axis]! > 1 ? [axis] : []
    )),
    rightExpandedAxes: paddedRightShape.flatMap((dimension, axis) => (
      dimension === 1 && outputShape[axis]! > 1 ? [axis] : []
    )),
    outputValues: mappings.map((mapping) => mapping.outputValue),
    mappings,
    leftGradient,
    rightGradient,
    finiteDifferenceLeftGradient,
    finiteDifferenceRightGradient,
    maxGradientAbsoluteError,
  };
}

export function traceTensorBroadcasting(
  scenarioId: TensorBroadcastScenarioId = "outer-grid",
): ScenarioBroadcastTrace {
  const scenario = TENSOR_BROADCAST_SCENARIOS.find((item) => item.id === scenarioId);
  if (scenario === undefined) {
    throw new Error(`unknown tensor broadcasting scenario: ${scenarioId}`);
  }
  return {
    id: scenario.id,
    title: scenario.title,
    summary: scenario.summary,
    ...traceBroadcastAdd(scenario.left, scenario.right, scenario.upstream),
  } as ScenarioBroadcastTrace;
}
