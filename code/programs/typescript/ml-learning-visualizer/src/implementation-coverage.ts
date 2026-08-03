import catalogDocument from "../../../../specs/fixtures/neural-learning-implementation-coverage-v1/catalog.json";
import sourceFixtureDocument from "../../../../specs/fixtures/neural-learning-v1/labs/00-weighted-neuron.json";

const IMPLEMENTATIONS = ["native", "rust-core-binding"] as const;
const CANONICAL_CONTRACTS = {
  native_catalog: "code/specs/fixtures/cross-language-consumers-v1/catalog.json",
  rust_c_abi_catalog: "code/specs/fixtures/neural-learning-rust-cabi-v1/catalog.json",
  native_validator: "code/scripts/validate_cross_language_fixture_consumers.py",
  binding_validator: "code/scripts/validate_neural_learning_rust_cabi.py",
  coverage_validator: "code/scripts/validate_neural_learning_implementation_coverage.py",
} as const;
const CANONICAL_LANES = [
  {
    id: "go-native",
    language: "Go",
    implementation: "native",
    arithmetic_owner: "Go",
    interface: "fixture JSON to Go arithmetic",
    evidence: "code/programs/go/neural-fixture-consumer/main.go",
    validator: CANONICAL_CONTRACTS.native_validator,
  },
  {
    id: "ruby-native",
    language: "Ruby",
    implementation: "native",
    arithmetic_owner: "Ruby",
    interface: "fixture JSON to Ruby arithmetic",
    evidence: "code/programs/ruby/neural-fixture-consumer/main.rb",
    validator: CANONICAL_CONTRACTS.native_validator,
  },
  {
    id: "rust-native",
    language: "Rust",
    implementation: "native",
    arithmetic_owner: "Rust",
    interface: "fixture JSON to Rust arithmetic",
    evidence: "code/programs/rust/neural-fixture-consumer/src/main.rs",
    validator: CANONICAL_CONTRACTS.native_validator,
  },
  {
    id: "python-ctypes-rust-core",
    language: "Python",
    implementation: "rust-core-binding",
    arithmetic_owner: "Rust",
    interface: "Python ctypes to versioned C ABI",
    evidence: "code/scripts/validate_neural_learning_rust_cabi.py",
    validator: CANONICAL_CONTRACTS.binding_validator,
  },
] as const;
const CANONICAL_RULES = [
  "native means the language lane owns the weighted-neuron arithmetic",
  "rust-core-binding means the caller crosses the stable C ABI and Rust owns the arithmetic",
  "a registered lane counts as verified only after its executable validator passes",
  "coverage counts implementation paths, not code quality, speed, or curriculum mastery",
] as const;

export type ImplementationKind = typeof IMPLEMENTATIONS[number];
export type CoverageLaneId = typeof CANONICAL_LANES[number]["id"];

export interface CoverageLane {
  readonly id: CoverageLaneId;
  readonly language: string;
  readonly implementation: ImplementationKind;
  readonly arithmeticOwner: string;
  readonly interface: string;
  readonly evidence: string;
  readonly validator: string;
}

export interface ImplementationCoverageCatalog {
  readonly title: string;
  readonly question: string;
  readonly sourceFixture: string;
  readonly contracts: typeof CANONICAL_CONTRACTS;
  readonly handCheck: {
    readonly inputs: readonly [number, number];
    readonly weights: readonly [number, number];
    readonly contributions: readonly [number, number];
    readonly bias: number;
    readonly prediction: number;
    readonly nativeImplementations: 3;
    readonly rustCoreBindings: 1;
    readonly totalVerifiedLanes: 4;
  };
  readonly lanes: readonly CoverageLane[];
  readonly rules: readonly string[];
}

export interface ImplementationCoverageTrace {
  readonly catalog: ImplementationCoverageCatalog;
  readonly lane: CoverageLane;
  readonly nativeLaneIds: readonly CoverageLaneId[];
  readonly bindingLaneIds: readonly CoverageLaneId[];
  readonly nativeFraction: "3 / 4";
  readonly bindingFraction: "1 / 4";
}

function closedObject(value: unknown, keys: readonly string[], context: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(`${context}: expected object`);
  const result = value as Record<string, unknown>;
  const actual = Object.keys(result).sort();
  const expected = [...keys].sort();
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) throw new Error(`${context}: unexpected keys`);
  return result;
}

function nonemptyText(value: unknown, context: string): string {
  if (typeof value !== "string" || value.trim().length === 0 || value.length > 512 || /[\u0000-\u001f]/.test(value)) throw new Error(`${context}: invalid text`);
  return value;
}

function pair(value: unknown, context: string): [number, number] {
  if (!Array.isArray(value) || value.length !== 2 || value.some((entry) => typeof entry !== "number" || !Number.isFinite(entry))) throw new Error(`${context}: expected two finite numbers`);
  return [value[0] as number, value[1] as number];
}

function singleton(value: unknown, context: string): number {
  if (!Array.isArray(value) || value.length !== 1 || typeof value[0] !== "number" || !Number.isFinite(value[0])) throw new Error(`${context}: expected one finite number`);
  return value[0];
}

function parseSourceFixtureHand(value: unknown) {
  const fixture = closedObject(value, ["schema_version", "id", "title", "stage", "question", "concepts", "model", "dataset", "training", "expected"], "source fixture");
  if (fixture.schema_version !== 1 || fixture.id !== "weighted-neuron-forward" || fixture.stage !== "forward" || fixture.training !== null) throw new Error("source fixture: wrong identity");
  const model = closedObject(fixture.model, ["kind", "input_count", "layers"], "source fixture model");
  if (model.kind !== "single-neuron" || model.input_count !== 2 || !Array.isArray(model.layers) || model.layers.length !== 1) throw new Error("source fixture: wrong model");
  const layer = closedObject(model.layers[0], ["name", "weights", "biases", "activation"], "source fixture layer");
  if (layer.name !== "output" || layer.activation !== "identity" || !Array.isArray(layer.weights) || layer.weights.length !== 2) throw new Error("source fixture: wrong layer");
  const weights: [number, number] = [
    singleton(layer.weights[0], "source fixture weights[0]"),
    singleton(layer.weights[1], "source fixture weights[1]"),
  ];
  const bias = singleton(layer.biases, "source fixture biases");
  const dataset = closedObject(fixture.dataset, ["input_labels", "target_labels", "rows"], "source fixture dataset");
  if (!Array.isArray(dataset.rows) || dataset.rows.length !== 1) throw new Error("source fixture: wrong rows");
  const row = closedObject(dataset.rows[0], ["label", "input", "target"], "source fixture row");
  const inputs = pair(row.input, "source fixture input");
  const expected = closedObject(fixture.expected, ["absolute_tolerance", "forward", "first_step"], "source fixture expected");
  if (!Array.isArray(expected.forward) || expected.forward.length !== 1 || expected.first_step !== null) throw new Error("source fixture: wrong expectation");
  const forward = closedObject(expected.forward[0], ["row", "prediction"], "source fixture forward");
  const prediction = singleton(forward.prediction, "source fixture prediction");
  if (row.label !== "worked example" || forward.row !== row.label || inputs[0] !== 2 || inputs[1] !== -1 || weights[0] !== 0.5 || weights[1] !== -0.25 || bias !== 0.1 || prediction !== 1.35) throw new Error("source fixture: wrong canonical hand example");
  return { inputs, weights, bias, prediction };
}

const sourceFixtureHand = parseSourceFixtureHand(sourceFixtureDocument);

export function parseImplementationCoverageCatalog(value: unknown): ImplementationCoverageCatalog {
  const catalog = closedObject(value, ["schema_version", "id", "title", "question", "source_fixture", "contracts", "hand_check", "lanes", "rules"], "catalog");
  if (catalog.schema_version !== 1 || catalog.id !== "weighted-neuron-implementation-coverage" || catalog.source_fixture !== "code/specs/fixtures/neural-learning-v1/labs/00-weighted-neuron.json") throw new Error("catalog: wrong identity");
  const contracts = closedObject(catalog.contracts, Object.keys(CANONICAL_CONTRACTS), "contracts");
  if (JSON.stringify(contracts) !== JSON.stringify(CANONICAL_CONTRACTS)) throw new Error("contracts: wrong values");

  const hand = closedObject(catalog.hand_check, ["inputs", "weights", "contributions", "bias", "prediction", "native_implementations", "rust_core_bindings", "total_verified_lanes"], "hand_check");
  const inputs = pair(hand.inputs, "hand_check.inputs");
  const weights = pair(hand.weights, "hand_check.weights");
  const contributions = pair(hand.contributions, "hand_check.contributions");
  const bias = hand.bias;
  const prediction = hand.prediction;
  if (typeof bias !== "number" || !Number.isFinite(bias) || typeof prediction !== "number" || !Number.isFinite(prediction)) throw new Error("hand_check: expected finite arithmetic");
  const recomputed = [inputs[0] * weights[0], inputs[1] * weights[1]] as const;
  if (contributions[0] !== recomputed[0] || contributions[1] !== recomputed[1] || prediction !== recomputed[0] + recomputed[1] + bias) throw new Error("hand_check: dishonest arithmetic");
  if (inputs[0] !== sourceFixtureHand.inputs[0] || inputs[1] !== sourceFixtureHand.inputs[1] || weights[0] !== sourceFixtureHand.weights[0] || weights[1] !== sourceFixtureHand.weights[1] || bias !== sourceFixtureHand.bias || prediction !== sourceFixtureHand.prediction) throw new Error("hand_check: catalog disagrees with NN03 source fixture");
  if (hand.native_implementations !== 3 || hand.rust_core_bindings !== 1 || hand.total_verified_lanes !== 4) throw new Error("hand_check: dishonest coverage count");

  if (!Array.isArray(catalog.lanes) || JSON.stringify(catalog.lanes) !== JSON.stringify(CANONICAL_LANES)) throw new Error("lanes: wrong values");
  const lanes = catalog.lanes.map((raw, index) => {
    const lane = closedObject(raw, ["id", "language", "implementation", "arithmetic_owner", "interface", "evidence", "validator"], `lanes[${index}]`);
    return {
      id: lane.id as CoverageLaneId,
      language: nonemptyText(lane.language, `lanes[${index}].language`),
      implementation: lane.implementation as ImplementationKind,
      arithmeticOwner: nonemptyText(lane.arithmetic_owner, `lanes[${index}].arithmetic_owner`),
      interface: nonemptyText(lane.interface, `lanes[${index}].interface`),
      evidence: nonemptyText(lane.evidence, `lanes[${index}].evidence`),
      validator: nonemptyText(lane.validator, `lanes[${index}].validator`),
    };
  });
  if (!Array.isArray(catalog.rules) || JSON.stringify(catalog.rules) !== JSON.stringify(CANONICAL_RULES)) throw new Error("rules: wrong values");

  return {
    title: nonemptyText(catalog.title, "catalog.title"),
    question: nonemptyText(catalog.question, "catalog.question"),
    sourceFixture: catalog.source_fixture as string,
    contracts: CANONICAL_CONTRACTS,
    handCheck: {
      inputs,
      weights,
      contributions,
      bias,
      prediction,
      nativeImplementations: 3,
      rustCoreBindings: 1,
      totalVerifiedLanes: 4,
    },
    lanes,
    rules: [...CANONICAL_RULES],
  };
}

export const implementationCoverageCatalog = parseImplementationCoverageCatalog(catalogDocument);

export function traceImplementationCoverage(laneId: CoverageLaneId = "go-native"): ImplementationCoverageTrace {
  const lane = implementationCoverageCatalog.lanes.find((item) => item.id === laneId);
  if (lane === undefined) throw new Error(`unknown coverage lane: ${laneId}`);
  return {
    catalog: implementationCoverageCatalog,
    lane,
    nativeLaneIds: implementationCoverageCatalog.lanes.filter((item) => item.implementation === "native").map((item) => item.id),
    bindingLaneIds: implementationCoverageCatalog.lanes.filter((item) => item.implementation === "rust-core-binding").map((item) => item.id),
    nativeFraction: "3 / 4",
    bindingFraction: "1 / 4",
  };
}
