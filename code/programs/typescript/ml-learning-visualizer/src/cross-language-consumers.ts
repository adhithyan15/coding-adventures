import catalogDocument from "../../../../specs/fixtures/cross-language-consumers-v1/catalog.json";
import fixtureDocument from "../../../../specs/fixtures/neural-learning-v1/labs/00-weighted-neuron.json";

const LANE_IDS = ["go-native", "ruby-native", "rust-native"] as const;
const RECEIPT_KEYS = [
  "schema_version",
  "lane_id",
  "fixture_id",
  "row",
  "contributions",
  "bias",
  "preactivation",
  "prediction",
  "maximum_absolute_error",
  "passes",
] as const;

export type ConsumerLaneId = typeof LANE_IDS[number];

export interface ConsumerLane {
  readonly id: ConsumerLaneId;
  readonly language: "Go" | "Ruby" | "Rust";
  readonly family: "compiled-garbage-collected" | "dynamic-interpreted" | "systems-native";
  readonly execution: "native";
  readonly workingDirectory: string;
  readonly command: readonly string[];
  readonly source: string;
}

export interface ConsumerHandCheck {
  readonly input: readonly [number, number];
  readonly weights: readonly [number, number];
  readonly contributions: readonly [number, number];
  readonly bias: number;
  readonly preactivation: number;
  readonly activation: "identity";
  readonly prediction: number;
  readonly absoluteTolerance: number;
}

export interface ConsumerCatalog {
  readonly id: "weighted-neuron-language-consumers";
  readonly title: string;
  readonly question: string;
  readonly sourceFixture: "code/specs/fixtures/neural-learning-v1/labs/00-weighted-neuron.json";
  readonly command: "python code/scripts/validate_cross_language_fixture_consumers.py";
  readonly steps: readonly string[];
  readonly receiptKeys: readonly string[];
  readonly handCheck: ConsumerHandCheck;
  readonly lanes: readonly ConsumerLane[];
}

export interface ConsumerTrace {
  readonly catalog: ConsumerCatalog;
  readonly lane: ConsumerLane;
  readonly row: "worked example";
  readonly contributions: readonly [number, number];
  readonly preactivation: number;
  readonly prediction: number;
  readonly maximumAbsoluteError: number;
  readonly passes: boolean;
}

const CANONICAL_LANES: Readonly<Record<ConsumerLaneId, Omit<ConsumerLane, "id">>> = {
  "go-native": {
    language: "Go",
    family: "compiled-garbage-collected",
    execution: "native",
    workingDirectory: "code/programs/go/neural-fixture-consumer",
    command: ["go", "run", ".", "--fixture", "{fixture}"],
    source: "code/programs/go/neural-fixture-consumer/main.go",
  },
  "ruby-native": {
    language: "Ruby",
    family: "dynamic-interpreted",
    execution: "native",
    workingDirectory: ".",
    command: ["ruby", "code/programs/ruby/neural-fixture-consumer/main.rb", "--fixture", "{fixture}"],
    source: "code/programs/ruby/neural-fixture-consumer/main.rb",
  },
  "rust-native": {
    language: "Rust",
    family: "systems-native",
    execution: "native",
    workingDirectory: ".",
    command: ["cargo", "run", "--quiet", "--manifest-path", "code/programs/rust/neural-fixture-consumer/Cargo.toml", "--", "--fixture", "{fixture}"],
    source: "code/programs/rust/neural-fixture-consumer/src/main.rs",
  },
};

function object(value: unknown, keys: readonly string[], context: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(`${context}: expected object`);
  const record = value as Record<string, unknown>;
  const actual = Object.keys(record).sort();
  const expected = [...keys].sort();
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) throw new Error(`${context}: unexpected keys`);
  return record;
}

function text(value: unknown, context: string, maximum = 512): string {
  if (typeof value !== "string" || value.trim().length === 0 || value.length > maximum || /[\u0000-\u001f]/.test(value)) throw new Error(`${context}: invalid text`);
  return value;
}

function number(value: unknown, context: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) throw new Error(`${context}: expected finite number`);
  return value;
}

function vector(value: unknown, length: number, context: string): number[] {
  if (!Array.isArray(value) || value.length !== length) throw new Error(`${context}: expected ${length} values`);
  return value.map((entry, index) => number(entry, `${context}[${index}]`));
}

function sameStrings(actual: unknown, expected: readonly string[], context: string): string[] {
  if (!Array.isArray(actual) || actual.length !== expected.length || actual.some((entry, index) => entry !== expected[index])) throw new Error(`${context}: wrong values`);
  return actual as string[];
}

function parseLane(value: unknown, index: number): ConsumerLane {
  const lane = object(value, ["id", "language", "family", "execution", "working_directory", "command", "source"], `lanes[${index}]`);
  if (!LANE_IDS.includes(lane.id as ConsumerLaneId)) throw new Error(`lanes[${index}]: unknown lane`);
  const id = lane.id as ConsumerLaneId;
  const canonical = CANONICAL_LANES[id];
  const command = sameStrings(lane.command, canonical.command, `lanes[${index}].command`);
  if (lane.language !== canonical.language || lane.family !== canonical.family || lane.execution !== "native" || lane.working_directory !== canonical.workingDirectory || lane.source !== canonical.source) throw new Error(`lanes[${index}]: non-canonical lane`);
  return { id, ...canonical, command };
}

export function parseConsumerCatalog(value: unknown): ConsumerCatalog {
  const catalog = object(value, ["schema_version", "id", "title", "question", "source_fixture", "protocol", "hand_check", "lanes"], "catalog");
  if (catalog.schema_version !== 1 || catalog.id !== "weighted-neuron-language-consumers" || catalog.source_fixture !== "code/specs/fixtures/neural-learning-v1/labs/00-weighted-neuron.json") throw new Error("catalog: wrong identity");
  const protocol = object(catalog.protocol, ["command", "success_exit_code", "steps", "receipt_keys"], "protocol");
  if (protocol.command !== "python code/scripts/validate_cross_language_fixture_consumers.py" || protocol.success_exit_code !== 0) throw new Error("protocol: wrong command");
  if (!Array.isArray(protocol.steps) || protocol.steps.length !== 4) throw new Error("protocol: expected four steps");
  const steps = protocol.steps.map((step, index) => text(step, `protocol.steps[${index}]`, 240));
  const receiptKeys = sameStrings(protocol.receipt_keys, RECEIPT_KEYS, "protocol.receipt_keys");

  const hand = object(catalog.hand_check, ["input", "weights", "contributions", "bias", "preactivation", "activation", "prediction", "absolute_tolerance"], "hand_check");
  const input = vector(hand.input, 2, "hand_check.input") as [number, number];
  const weights = vector(hand.weights, 2, "hand_check.weights") as [number, number];
  const contributions = vector(hand.contributions, 2, "hand_check.contributions") as [number, number];
  const bias = number(hand.bias, "hand_check.bias");
  const preactivation = number(hand.preactivation, "hand_check.preactivation");
  const prediction = number(hand.prediction, "hand_check.prediction");
  const absoluteTolerance = number(hand.absolute_tolerance, "hand_check.absolute_tolerance");
  const recomputed = [input[0] * weights[0], input[1] * weights[1]] as const;
  if (input[0] !== 2 || input[1] !== -1 || weights[0] !== 0.5 || weights[1] !== -0.25 || contributions[0] !== recomputed[0] || contributions[1] !== recomputed[1] || preactivation !== recomputed[0] + recomputed[1] + bias || prediction !== preactivation || hand.activation !== "identity" || absoluteTolerance <= 0) throw new Error("hand_check: dishonest arithmetic");

  if (!Array.isArray(catalog.lanes) || catalog.lanes.length !== 3) throw new Error("catalog: expected three lanes");
  const lanes = catalog.lanes.map(parseLane);
  if (lanes.some((lane, index) => lane.id !== LANE_IDS[index])) throw new Error("catalog: lane order mismatch");
  return {
    id: catalog.id,
    title: text(catalog.title, "catalog.title", 180),
    question: text(catalog.question, "catalog.question", 240),
    sourceFixture: catalog.source_fixture,
    command: protocol.command,
    steps,
    receiptKeys,
    handCheck: { input, weights, contributions, bias, preactivation, activation: "identity", prediction, absoluteTolerance },
    lanes,
  };
}

function parseSourceFixture(value: unknown): { readonly row: "worked example"; readonly storedPrediction: number } {
  const fixture = object(value, ["schema_version", "id", "title", "stage", "question", "concepts", "model", "dataset", "training", "expected"], "source fixture");
  if (fixture.schema_version !== 1 || fixture.id !== "weighted-neuron-forward" || fixture.stage !== "forward" || fixture.training !== null) throw new Error("source fixture: wrong identity");
  const model = object(fixture.model, ["kind", "input_count", "layers"], "source fixture model");
  if (model.kind !== "single-neuron" || model.input_count !== 2 || !Array.isArray(model.layers) || model.layers.length !== 1) throw new Error("source fixture: wrong model");
  const layer = object(model.layers[0], ["name", "weights", "biases", "activation"], "source fixture layer");
  if (layer.name !== "output" || layer.activation !== "identity" || !Array.isArray(layer.weights) || layer.weights.length !== 2 || !Array.isArray(layer.biases) || layer.biases.length !== 1) throw new Error("source fixture: wrong layer");
  const weights = layer.weights.map((entry, index) => vector(entry, 1, `source fixture weight[${index}]`)[0]);
  const bias = number(layer.biases[0], "source fixture bias");
  const dataset = object(fixture.dataset, ["input_labels", "target_labels", "rows"], "source fixture dataset");
  if (!Array.isArray(dataset.rows) || dataset.rows.length !== 1) throw new Error("source fixture: wrong rows");
  const row = object(dataset.rows[0], ["label", "input", "target"], "source fixture row");
  const input = vector(row.input, 2, "source fixture input");
  const expected = object(fixture.expected, ["absolute_tolerance", "forward", "first_step"], "source fixture expected");
  if (!Array.isArray(expected.forward) || expected.forward.length !== 1 || expected.first_step !== null) throw new Error("source fixture: wrong expectation");
  const forward = object(expected.forward[0], ["row", "prediction"], "source fixture forward");
  const storedPrediction = vector(forward.prediction, 1, "source fixture prediction")[0]!;
  const catalog = parseConsumerCatalog(catalogDocument);
  if (row.label !== "worked example" || forward.row !== row.label || input[0] !== catalog.handCheck.input[0] || input[1] !== catalog.handCheck.input[1] || weights[0] !== catalog.handCheck.weights[0] || weights[1] !== catalog.handCheck.weights[1] || bias !== catalog.handCheck.bias || storedPrediction !== catalog.handCheck.prediction || expected.absolute_tolerance !== catalog.handCheck.absoluteTolerance) throw new Error("source fixture: catalog disagrees with NN03");
  return { row: "worked example", storedPrediction };
}

export const consumerCatalog = parseConsumerCatalog(catalogDocument);
const sourceFixture = parseSourceFixture(fixtureDocument);

export function traceLanguageConsumer(laneId: ConsumerLaneId = "go-native"): ConsumerTrace {
  const lane = consumerCatalog.lanes.find((candidate) => candidate.id === laneId);
  if (!lane) throw new Error(`unknown consumer lane: ${laneId}`);
  const contributions = [
    consumerCatalog.handCheck.input[0] * consumerCatalog.handCheck.weights[0],
    consumerCatalog.handCheck.input[1] * consumerCatalog.handCheck.weights[1],
  ] as const;
  const preactivation = contributions[0] + contributions[1] + consumerCatalog.handCheck.bias;
  const prediction = preactivation;
  const maximumAbsoluteError = Math.abs(prediction - sourceFixture.storedPrediction);
  return {
    catalog: consumerCatalog,
    lane,
    row: sourceFixture.row,
    contributions,
    preactivation,
    prediction,
    maximumAbsoluteError,
    passes: maximumAbsoluteError <= consumerCatalog.handCheck.absoluteTolerance,
  };
}
