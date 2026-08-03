import catalogDocument from "../../../../specs/fixtures/neural-learning-rust-cabi-v1/catalog.json";
import sourceFixtureDocument from "../../../../specs/fixtures/neural-learning-v1/labs/00-weighted-neuron.json";

const PROBE_IDS = ["success", "null-input", "empty-input", "short-output", "non-finite", "overlapping-output"] as const;
const CANONICAL_FUNCTIONS = [
  "uint32_t neural_learning_abi_version(void)",
  "const char *neural_learning_status_message_v1(uint32_t status)",
  "uint32_t neural_learning_weighted_sum_f64_v1(const double *inputs, const double *weights, uint64_t input_count, double bias, double *contributions_out, uint64_t contributions_capacity, double *prediction_out)",
] as const;
const CANONICAL_RULES = [
  "all lengths and status values use fixed-width unsigned integers",
  "callers own every input and output buffer",
  "no Rust allocation or Rust type crosses the boundary",
  "mutable outputs do not overlap inputs or one another",
  "a Rust panic is caught and becomes status 6",
] as const;
const CANONICAL_STATUSES = [
  ["NEURAL_LEARNING_OK", "ok"],
  ["NEURAL_LEARNING_NULL_POINTER", "null pointer"],
  ["NEURAL_LEARNING_EMPTY_INPUT", "input count must be positive"],
  ["NEURAL_LEARNING_BUFFER_TOO_SMALL", "contribution buffer is too small"],
  ["NEURAL_LEARNING_VALUE_TOO_LARGE", "input count is too large"],
  ["NEURAL_LEARNING_NON_FINITE", "all inputs and arithmetic results must be finite"],
  ["NEURAL_LEARNING_PANIC", "Rust panic was contained"],
  ["NEURAL_LEARNING_OVERLAPPING_BUFFER", "mutable output buffers must not overlap other buffers"],
  ["NEURAL_LEARNING_MISALIGNED_POINTER", "pointer is not aligned for a double"],
] as const;

export type CAbiProbeId = typeof PROBE_IDS[number];

export interface CAbiStatus {
  readonly code: number;
  readonly symbol: string;
  readonly message: string;
}

export interface CAbiProbe {
  readonly id: Exclude<CAbiProbeId, "success">;
  readonly expectedStatus: number;
  readonly outputsUnchanged: true;
}

export interface CAbiCatalog {
  readonly title: string;
  readonly question: string;
  readonly versionNumber: 65536;
  readonly versionHex: "0x00010000";
  readonly header: "code/packages/rust/neural-learning-capi/include/neural_learning.h";
  readonly crate: "code/packages/rust/neural-learning-capi/Cargo.toml";
  readonly functions: readonly string[];
  readonly rules: readonly string[];
  readonly statuses: readonly CAbiStatus[];
  readonly inputs: readonly [number, number];
  readonly weights: readonly [number, number];
  readonly bias: number;
  readonly expectedContributions: readonly [number, number];
  readonly expectedPrediction: number;
  readonly probes: readonly CAbiProbe[];
}

export interface CAbiTrace {
  readonly catalog: CAbiCatalog;
  readonly probeId: CAbiProbeId;
  readonly contributions: readonly [number, number];
  readonly prediction: number;
  readonly status: CAbiStatus;
  readonly outputsWritten: boolean;
  readonly boundaryCheck: string;
}

function closedObject(value: unknown, keys: readonly string[], context: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(`${context}: expected object`);
  const result = value as Record<string, unknown>;
  const actual = Object.keys(result).sort();
  const expected = [...keys].sort();
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) throw new Error(`${context}: unexpected keys`);
  return result;
}

function finiteVector(value: unknown, context: string): [number, number] {
  if (!Array.isArray(value) || value.length !== 2 || value.some((entry) => typeof entry !== "number" || !Number.isFinite(entry))) throw new Error(`${context}: expected two finite numbers`);
  return [value[0] as number, value[1] as number];
}

function text(value: unknown, context: string): string {
  if (typeof value !== "string" || value.trim().length === 0 || value.length > 512 || /[\u0000-\u001f]/.test(value)) throw new Error(`${context}: invalid text`);
  return value;
}

export function parseCAbiCatalog(value: unknown): CAbiCatalog {
  const catalog = closedObject(value, ["schema_version", "id", "title", "question", "source_fixture", "abi", "statuses", "hand_check", "failure_probes"], "catalog");
  if (catalog.schema_version !== 1 || catalog.id !== "weighted-neuron-rust-c-abi" || catalog.source_fixture !== "code/specs/fixtures/neural-learning-v1/labs/00-weighted-neuron.json") throw new Error("catalog: wrong identity");
  const abi = closedObject(catalog.abi, ["version_number", "version_hex", "header", "crate", "library_base_name", "functions", "rules"], "abi");
  if (abi.version_number !== 65536 || abi.version_hex !== "0x00010000" || abi.header !== "code/packages/rust/neural-learning-capi/include/neural_learning.h" || abi.crate !== "code/packages/rust/neural-learning-capi/Cargo.toml" || abi.library_base_name !== "neural_learning_capi") throw new Error("abi: wrong identity");
  if (!Array.isArray(abi.functions) || JSON.stringify(abi.functions) !== JSON.stringify(CANONICAL_FUNCTIONS)) throw new Error("abi: wrong functions");
  if (!Array.isArray(abi.rules) || JSON.stringify(abi.rules) !== JSON.stringify(CANONICAL_RULES)) throw new Error("abi: wrong rules");
  const functions = abi.functions.map((entry, index) => text(entry, `functions[${index}]`));
  const rules = abi.rules.map((entry, index) => text(entry, `rules[${index}]`));

  if (!Array.isArray(catalog.statuses) || catalog.statuses.length !== 9) throw new Error("statuses: expected nine entries");
  const statuses = catalog.statuses.map((value, index) => {
    const status = closedObject(value, ["code", "symbol", "message"], `statuses[${index}]`);
    const canonical = CANONICAL_STATUSES[index];
    if (status.code !== index || !canonical || status.symbol !== canonical[0] || status.message !== canonical[1]) throw new Error(`statuses[${index}]: wrong identity`);
    return { code: index, symbol: canonical[0], message: canonical[1] };
  });

  const hand = closedObject(catalog.hand_check, ["inputs", "weights", "bias", "contributions_capacity", "expected_status", "expected_contributions", "expected_prediction", "absolute_tolerance"], "hand_check");
  const inputs = finiteVector(hand.inputs, "hand_check.inputs");
  const weights = finiteVector(hand.weights, "hand_check.weights");
  const expectedContributions = finiteVector(hand.expected_contributions, "hand_check.expected_contributions");
  if (typeof hand.bias !== "number" || !Number.isFinite(hand.bias) || typeof hand.expected_prediction !== "number" || !Number.isFinite(hand.expected_prediction) || hand.contributions_capacity !== 2 || hand.expected_status !== 0 || typeof hand.absolute_tolerance !== "number" || hand.absolute_tolerance <= 0) throw new Error("hand_check: wrong contract");
  const contributions = [inputs[0] * weights[0], inputs[1] * weights[1]] as const;
  const prediction = contributions[0] + contributions[1] + hand.bias;
  if (inputs[0] !== 2 || inputs[1] !== -1 || weights[0] !== 0.5 || weights[1] !== -0.25 || expectedContributions[0] !== contributions[0] || expectedContributions[1] !== contributions[1] || hand.expected_prediction !== prediction) throw new Error("hand_check: dishonest arithmetic");

  if (!Array.isArray(catalog.failure_probes) || catalog.failure_probes.length !== 5) throw new Error("failure_probes: expected five entries");
  const probes = catalog.failure_probes.map((value, index) => {
    const probe = closedObject(value, ["id", "expected_status", "outputs_unchanged"], `failure_probes[${index}]`);
    const expectedId = PROBE_IDS[index + 1];
    if (probe.id !== expectedId || probe.expected_status !== [1, 2, 3, 5, 7][index] || probe.outputs_unchanged !== true) throw new Error(`failure_probes[${index}]: wrong probe`);
    return { id: expectedId as CAbiProbe["id"], expectedStatus: probe.expected_status as number, outputsUnchanged: true as const };
  });
  return {
    title: text(catalog.title, "catalog.title"),
    question: text(catalog.question, "catalog.question"),
    versionNumber: 65536,
    versionHex: "0x00010000",
    header: abi.header,
    crate: abi.crate,
    functions,
    rules,
    statuses,
    inputs,
    weights,
    bias: hand.bias,
    expectedContributions,
    expectedPrediction: hand.expected_prediction,
    probes,
  };
}

export const cAbiCatalog = parseCAbiCatalog(catalogDocument);

function validateSourceFixture(value: unknown, catalog: CAbiCatalog): void {
  const fixture = closedObject(value, ["schema_version", "id", "title", "stage", "question", "concepts", "model", "dataset", "training", "expected"], "source fixture");
  if (fixture.schema_version !== 1 || fixture.id !== "weighted-neuron-forward" || fixture.stage !== "forward" || fixture.training !== null) throw new Error("source fixture: wrong identity");
  const model = closedObject(fixture.model, ["kind", "input_count", "layers"], "source model");
  if (model.kind !== "single-neuron" || model.input_count !== 2 || !Array.isArray(model.layers) || model.layers.length !== 1) throw new Error("source fixture: wrong model");
  const layer = closedObject(model.layers[0], ["name", "weights", "biases", "activation"], "source layer");
  if (layer.name !== "output" || layer.activation !== "identity" || JSON.stringify(layer.weights) !== JSON.stringify([[0.5], [-0.25]]) || JSON.stringify(layer.biases) !== JSON.stringify([0.1])) throw new Error("source fixture: wrong layer");
  const dataset = closedObject(fixture.dataset, ["input_labels", "target_labels", "rows"], "source dataset");
  if (!Array.isArray(dataset.rows) || dataset.rows.length !== 1) throw new Error("source fixture: wrong rows");
  const row = closedObject(dataset.rows[0], ["label", "input", "target"], "source row");
  const expected = closedObject(fixture.expected, ["absolute_tolerance", "forward", "first_step"], "source expected");
  if (!Array.isArray(expected.forward) || expected.forward.length !== 1 || expected.first_step !== null) throw new Error("source fixture: wrong expectation");
  const forward = closedObject(expected.forward[0], ["row", "prediction"], "source forward");
  if (row.label !== "worked example" || forward.row !== row.label || JSON.stringify(row.input) !== JSON.stringify(catalog.inputs) || JSON.stringify(forward.prediction) !== JSON.stringify([catalog.expectedPrediction]) || expected.absolute_tolerance !== 0.000001) throw new Error("source fixture: catalog disagreement");
}

validateSourceFixture(sourceFixtureDocument, cAbiCatalog);

const BOUNDARY_CHECKS: Readonly<Record<CAbiProbeId, string>> = {
  success: "validate buffers, calculate twice, then write outputs",
  "null-input": "reject a required null input pointer",
  "empty-input": "reject an input_count of zero",
  "short-output": "reject capacity smaller than input_count",
  "non-finite": "reject infinity before any output write",
  "overlapping-output": "reject mutable output overlapping an input buffer",
};

export function traceCAbi(probeId: CAbiProbeId = "success"): CAbiTrace {
  if (!PROBE_IDS.includes(probeId)) throw new Error(`unknown C ABI probe: ${probeId}`);
  const probe = probeId === "success" ? undefined : cAbiCatalog.probes.find((candidate) => candidate.id === probeId);
  const statusCode = probe?.expectedStatus ?? 0;
  const status = cAbiCatalog.statuses[statusCode];
  if (!status) throw new Error(`missing C ABI status ${statusCode}`);
  return {
    catalog: cAbiCatalog,
    probeId,
    contributions: cAbiCatalog.expectedContributions,
    prediction: cAbiCatalog.expectedPrediction,
    status,
    outputsWritten: probeId === "success",
    boundaryCheck: BOUNDARY_CHECKS[probeId],
  };
}
