import fixtureDocument from "../../../../specs/fixtures/precision-residency-v1/labs/00-tiny-affine.json";

const MAX_TEXT = 512;
const MAX_ABSOLUTE_NUMBER = 1e6;
const FORMAT_IDS = ["binary32", "binary16", "symmetric_int8"] as const;
const STRATEGY_IDS = ["eager", "resident"] as const;

export type PrecisionFormatId = typeof FORMAT_IDS[number];
export type ResidencyStrategyId = typeof STRATEGY_IDS[number];

const FORMAT_TITLES: Record<PrecisionFormatId, string> = {
  binary32: "IEEE-754 binary32",
  binary16: "IEEE-754 binary16",
  symmetric_int8: "Symmetric signed int8",
};
const INPUT_PAYLOADS: Record<PrecisionFormatId, string> = {
  binary32: "../payloads/00-input-x.f32le.hex",
  binary16: "../payloads/00-input-x.f16le.hex",
  symmetric_int8: "../payloads/00-input-x.i8.hex",
};
const STRATEGY_TITLES: Record<ResidencyStrategyId, string> = {
  eager: "Eager copies",
  resident: "Resident buffers",
};
const STRATEGY_STEPS: Record<ResidencyStrategyId, readonly string[]> = {
  eager: ["upload x, w, and b", "run affine neuron", "download y", "discard device buffers"],
  resident: ["upload x, w, and b once", "run affine neuron three times", "keep x, w, b, and y on device", "download final y once"],
};

export interface PrecisionFormat {
  readonly id: PrecisionFormatId;
  readonly title: string;
  readonly storageBytesPerValue: number;
  readonly accumulatorStorageBytes?: number;
  readonly encodedInputs: readonly number[];
  readonly encodedWeight: number;
  readonly accumulators: readonly number[];
  readonly outputs: readonly number[];
  readonly maximumAbsoluteError: number;
  readonly inputScale?: number;
  readonly weightScale?: number;
  readonly zeroPoint?: number;
}

export interface ResidencyStrategy {
  readonly id: ResidencyStrategyId;
  readonly title: string;
  readonly steps: readonly string[];
  readonly uploadCount: number;
  readonly downloadCount: number;
  readonly totalTransferBytes: number;
}

export interface PrecisionResidencyFixture {
  readonly id: "tiny-affine-precision-residency";
  readonly title: string;
  readonly question: string;
  readonly graph: {
    readonly equation: "y = x * w + b";
    readonly weight: number;
    readonly bias: number;
  };
  readonly scenario: {
    readonly inputs: readonly number[];
    readonly referenceOutputs: readonly number[];
  };
  readonly formats: readonly PrecisionFormat[];
  readonly residency: {
    readonly dtype: "binary32";
    readonly repeatCount: number;
    readonly uploadBytesPerCopy: number;
    readonly downloadBytesPerCopy: number;
    readonly strategies: readonly ResidencyStrategy[];
  };
}

export interface PrecisionRowTrace {
  readonly input: number;
  readonly encodedInput: number;
  readonly encodedWeight: number;
  readonly accumulator: number;
  readonly output: number;
  readonly referenceOutput: number;
  readonly absoluteError: number;
}

export interface PrecisionResidencyTrace {
  readonly fixture: PrecisionResidencyFixture;
  readonly format: PrecisionFormat;
  readonly strategy: ResidencyStrategy;
  readonly rows: readonly PrecisionRowTrace[];
  readonly repeatCount: number;
  readonly uploadCount: number;
  readonly downloadCount: number;
  readonly transferBytes: number;
  readonly bytesSavedAgainstEager: number;
}

function object(value: unknown, keys: readonly string[], context: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`${context} must be an object`);
  }
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  if (actual.join(",") !== expected.join(",")) {
    throw new Error(`${context} has unexpected fields`);
  }
  return value as Record<string, unknown>;
}

function text(value: unknown, context: string): string {
  if (typeof value !== "string" || value.length < 1 || value.length > MAX_TEXT) {
    throw new Error(`${context} must be bounded text`);
  }
  return value;
}

function number(value: unknown, context: string): number {
  if (typeof value !== "number" || !Number.isFinite(value) || Math.abs(value) > MAX_ABSOLUTE_NUMBER) {
    throw new Error(`${context} must be a finite bounded number`);
  }
  return value;
}

function integer(value: unknown, minimum: number, maximum: number, context: string): number {
  const result = number(value, context);
  if (!Number.isInteger(result) || result < minimum || result > maximum) {
    throw new Error(`${context} must be a bounded integer`);
  }
  return result;
}

function numbers(value: unknown, length: number, context: string): number[] {
  if (!Array.isArray(value) || value.length !== length) {
    throw new Error(`${context} must contain ${length} numbers`);
  }
  return value.map((item, index) => number(item, `${context}[${index}]`));
}

function strings(value: unknown, length: number, context: string): string[] {
  if (!Array.isArray(value) || value.length !== length) {
    throw new Error(`${context} must contain ${length} strings`);
  }
  return value.map((item, index) => text(item, `${context}[${index}]`));
}

function sameNumbers(actual: readonly number[], expected: readonly number[], context: string): void {
  if (actual.length !== expected.length || actual.some((value, index) => value !== expected[index])) {
    throw new Error(`${context} does not match the arithmetic oracle`);
  }
}

export function roundTiesToEven(value: number): number {
  const lower = Math.floor(value);
  const fraction = value - lower;
  if (fraction < 0.5) return lower;
  if (fraction > 0.5) return lower + 1;
  return lower % 2 === 0 ? lower : lower + 1;
}

export function roundToBinary16(value: number): number {
  if (!Number.isFinite(value)) {
    throw new Error("binary16 input must be finite and representable");
  }
  if (value === 0) return value;
  const sign = value < 0 ? -1 : 1;
  const magnitude = Math.abs(value);
  const roundedMagnitude = magnitude < 2 ** -14
    ? roundTiesToEven(magnitude / 2 ** -24) * 2 ** -24
    : (() => {
        const exponent = Math.floor(Math.log2(magnitude));
        const quantum = 2 ** (exponent - 10);
        return roundTiesToEven(magnitude / quantum) * quantum;
      })();
  if (roundedMagnitude > 65504) {
    throw new Error("binary16 input must be finite and representable");
  }
  return sign * roundedMagnitude;
}

function traceFloat(
  inputs: readonly number[],
  weight: number,
  bias: number,
  roundValue: (value: number) => number,
) {
  const encodedInputs = inputs.map(roundValue);
  const encodedWeight = roundValue(weight);
  const encodedBias = roundValue(bias);
  const accumulators = encodedInputs.map((value) => roundValue(value * encodedWeight));
  const outputs = accumulators.map((value) => roundValue(value + encodedBias));
  return { encodedInputs, encodedWeight, accumulators, outputs };
}

function maxError(outputs: readonly number[], reference: readonly number[]): number {
  return Math.max(...outputs.map((value, index) => Math.abs(value - reference[index]!)));
}

function normalizeFormat(
  raw: unknown,
  expectedId: PrecisionFormatId,
  inputs: readonly number[],
  weight: number,
  bias: number,
  referenceOutputs: readonly number[],
): PrecisionFormat {
  const floatKeys = [
    "id", "title", "storage_bytes_per_value", "input_payload_file", "output_payload_file",
    "encoded_inputs", "encoded_weight", "accumulators", "outputs", "maximum_absolute_error",
  ];
  const int8Keys = [
    "id", "title", "storage_bytes_per_value", "input_payload_file", "weight_payload_file",
    "accumulator_storage_bytes", "input_scale", "weight_scale", "zero_point", "encoded_inputs", "encoded_weight",
    "accumulators", "outputs", "maximum_absolute_error",
  ];
  const item = object(raw, expectedId === "symmetric_int8" ? int8Keys : floatKeys, `format ${expectedId}`);
  if (item.id !== expectedId) throw new Error("precision format roster is not canonical");
  if (text(item.title, "format title") !== FORMAT_TITLES[expectedId]) {
    throw new Error("precision format title is not canonical");
  }
  if (item.input_payload_file !== INPUT_PAYLOADS[expectedId]) {
    throw new Error("input payload reference is not canonical");
  }
  if (expectedId === "binary32" && item.output_payload_file !== "../payloads/00-output-y.f32le.hex") {
    throw new Error("output payload reference is not canonical");
  }
  if (expectedId === "binary16" && item.output_payload_file !== "../payloads/00-output-y.f16le.hex") {
    throw new Error("output payload reference is not canonical");
  }
  if (expectedId === "symmetric_int8" && item.weight_payload_file !== "../payloads/00-weight-w.i8.hex") {
    throw new Error("weight payload reference is not canonical");
  }
  const encodedInputs = numbers(item.encoded_inputs, 2, "encoded inputs");
  const encodedWeight = number(item.encoded_weight, "encoded weight");
  const accumulators = numbers(item.accumulators, 2, "accumulators");
  const outputs = numbers(item.outputs, 2, "outputs");
  let oracle: { encodedInputs: readonly number[]; encodedWeight: number; accumulators: readonly number[]; outputs: readonly number[] };
  let inputScale: number | undefined;
  let weightScale: number | undefined;
  let zeroPoint: number | undefined;
  let accumulatorStorageBytes: number | undefined;
  if (expectedId === "symmetric_int8") {
    inputScale = number(item.input_scale, "input scale");
    weightScale = number(item.weight_scale, "weight scale");
    zeroPoint = integer(item.zero_point, -128, 127, "zero point");
    accumulatorStorageBytes = integer(item.accumulator_storage_bytes, 1, 8, "accumulator storage width");
    if (inputScale !== 0.01 || weightScale !== 0.5 || zeroPoint !== 0 || accumulatorStorageBytes !== 4) {
      throw new Error("int8 quantization parameters are not canonical");
    }
    const quantizedInputs = inputs.map((value) => roundTiesToEven(value / inputScale!));
    const quantizedWeight = roundTiesToEven(weight / weightScale);
    const integerAccumulators = quantizedInputs.map((value) => value * quantizedWeight);
    oracle = {
      encodedInputs: quantizedInputs,
      encodedWeight: quantizedWeight,
      accumulators: integerAccumulators,
      outputs: integerAccumulators.map((value) => value * inputScale! * weightScale!),
    };
  } else {
    oracle = traceFloat(inputs, weight, bias, expectedId === "binary32" ? Math.fround : roundToBinary16);
  }
  sameNumbers(encodedInputs, oracle.encodedInputs, "encoded inputs");
  if (encodedWeight !== oracle.encodedWeight) throw new Error("encoded weight does not match the arithmetic oracle");
  sameNumbers(accumulators, oracle.accumulators, "accumulators");
  sameNumbers(outputs, oracle.outputs, "outputs");
  const maximumAbsoluteError = number(item.maximum_absolute_error, "maximum absolute error");
  if (maximumAbsoluteError !== maxError(outputs, referenceOutputs)) {
    throw new Error("maximum absolute error does not match the arithmetic oracle");
  }
  const storageBytesPerValue = integer(item.storage_bytes_per_value, 1, 8, "storage width");
  const expectedWidth = expectedId === "binary32" ? 4 : expectedId === "binary16" ? 2 : 1;
  if (storageBytesPerValue !== expectedWidth) throw new Error("storage width is not canonical");
  return {
    id: expectedId,
    title: FORMAT_TITLES[expectedId],
    storageBytesPerValue,
    encodedInputs,
    encodedWeight,
    accumulators,
    outputs,
    maximumAbsoluteError,
    ...(accumulatorStorageBytes === undefined ? {} : { accumulatorStorageBytes }),
    ...(inputScale === undefined ? {} : { inputScale, weightScale, zeroPoint }),
  };
}

export function normalizePrecisionResidencyFixture(value: unknown): PrecisionResidencyFixture {
  const lab = object(value, ["schema_version", "id", "title", "question", "graph", "scenario", "formats", "residency"], "precision fixture");
  if (lab.schema_version !== 1 || lab.id !== "tiny-affine-precision-residency") {
    throw new Error("precision fixture identity is not canonical");
  }
  const graph = object(lab.graph, ["equation", "weight", "bias"], "graph");
  const weight = number(graph.weight, "weight");
  const bias = number(graph.bias, "bias");
  if (graph.equation !== "y = x * w + b" || weight !== 2 || bias !== 0) {
    throw new Error("affine graph is not canonical");
  }
  const scenario = object(lab.scenario, ["inputs", "reference_outputs"], "scenario");
  const inputs = numbers(scenario.inputs, 2, "inputs");
  const referenceOutputs = numbers(scenario.reference_outputs, 2, "reference outputs");
  if (inputs.join(",") !== "1.0004,1.0006") throw new Error("input scenario is not canonical");
  sameNumbers(referenceOutputs, inputs.map((input) => input * weight + bias), "reference outputs");
  const rawFormats = lab.formats;
  if (!Array.isArray(rawFormats) || rawFormats.length !== 3) throw new Error("precision fixture must contain three formats");
  const formats = FORMAT_IDS.map((id, index) => normalizeFormat(rawFormats[index], id, inputs, weight, bias, referenceOutputs));

  const residency = object(lab.residency, ["dtype", "repeat_count", "upload_bytes_per_copy", "download_bytes_per_copy", "strategies"], "residency");
  const repeatCount = integer(residency.repeat_count, 1, 16, "repeat count");
  const uploadBytesPerCopy = integer(residency.upload_bytes_per_copy, 1, 1024, "upload bytes");
  const downloadBytesPerCopy = integer(residency.download_bytes_per_copy, 1, 1024, "download bytes");
  if (residency.dtype !== "binary32" || repeatCount !== 3 || uploadBytesPerCopy !== 16 || downloadBytesPerCopy !== 8) {
    throw new Error("residency byte contract is not canonical");
  }
  const rawStrategies = residency.strategies;
  if (!Array.isArray(rawStrategies) || rawStrategies.length !== 2) throw new Error("residency strategy roster is not canonical");
  const strategies = STRATEGY_IDS.map((id, index): ResidencyStrategy => {
    const raw = object(rawStrategies[index], ["id", "title", "steps", "upload_count", "download_count", "total_transfer_bytes"], `strategy ${id}`);
    const uploads = id === "eager" ? repeatCount : 1;
    const downloads = id === "eager" ? repeatCount : 1;
    const total = id === "eager" ? (uploadBytesPerCopy + downloadBytesPerCopy) * repeatCount : uploadBytesPerCopy + downloadBytesPerCopy;
    const steps = strings(raw.steps, 4, "strategy steps");
    if (raw.id !== id || raw.title !== STRATEGY_TITLES[id] || steps.join("\u0000") !== STRATEGY_STEPS[id].join("\u0000") || raw.upload_count !== uploads || raw.download_count !== downloads || raw.total_transfer_bytes !== total) {
      throw new Error("residency transfer oracle is not canonical");
    }
    return {
      id,
      title: STRATEGY_TITLES[id],
      steps,
      uploadCount: uploads,
      downloadCount: downloads,
      totalTransferBytes: total,
    };
  });
  return deepFreeze({
    id: "tiny-affine-precision-residency" as const,
    title: text(lab.title, "fixture title"),
    question: text(lab.question, "fixture question"),
    graph: { equation: "y = x * w + b" as const, weight, bias },
    scenario: { inputs, referenceOutputs },
    formats,
    residency: { dtype: "binary32" as const, repeatCount, uploadBytesPerCopy, downloadBytesPerCopy, strategies },
  });
}

export const PRECISION_RESIDENCY_FIXTURE = normalizePrecisionResidencyFixture(fixtureDocument);

export function tracePrecisionResidency(
  formatId: PrecisionFormatId = "binary16",
  strategyId: ResidencyStrategyId = "resident",
  repeatCount = PRECISION_RESIDENCY_FIXTURE.residency.repeatCount,
): PrecisionResidencyTrace {
  if (!Number.isInteger(repeatCount) || repeatCount < 1 || repeatCount > 8) {
    throw new Error("repeat count must be an integer from 1 through 8");
  }
  const fixture = PRECISION_RESIDENCY_FIXTURE;
  const format = fixture.formats.find((item) => item.id === formatId);
  const strategyTemplate = fixture.residency.strategies.find((item) => item.id === strategyId);
  if (format === undefined || strategyTemplate === undefined) throw new Error("unknown precision or residency selection");
  const rows = fixture.scenario.inputs.map((input, index): PrecisionRowTrace => ({
    input,
    encodedInput: format.encodedInputs[index]!,
    encodedWeight: format.encodedWeight,
    accumulator: format.accumulators[index]!,
    output: format.outputs[index]!,
    referenceOutput: fixture.scenario.referenceOutputs[index]!,
    absoluteError: Math.abs(format.outputs[index]! - fixture.scenario.referenceOutputs[index]!),
  }));
  const eagerBytes = (fixture.residency.uploadBytesPerCopy + fixture.residency.downloadBytesPerCopy) * repeatCount;
  const uploadCount = strategyId === "eager" ? repeatCount : 1;
  const downloadCount = strategyId === "eager" ? repeatCount : 1;
  const transferBytes = strategyId === "eager" ? eagerBytes : fixture.residency.uploadBytesPerCopy + fixture.residency.downloadBytesPerCopy;
  const strategy = deepFreeze({
    ...strategyTemplate,
    steps: strategyTemplate.steps.map((step, index) => (
      index === 1 ? `run affine neuron ${repeatCount} ${repeatCount === 1 ? "time" : "times"}` : step
    )),
    uploadCount,
    downloadCount,
    totalTransferBytes: transferBytes,
  });
  return deepFreeze({
    fixture,
    format,
    strategy,
    rows,
    repeatCount,
    uploadCount,
    downloadCount,
    transferBytes,
    bytesSavedAgainstEager: eagerBytes - transferBytes,
  });
}

function deepFreeze<T>(value: T): T {
  if (typeof value !== "object" || value === null || Object.isFrozen(value)) return value;
  Object.freeze(value);
  Object.values(value).forEach((child) => deepFreeze(child));
  return value;
}
