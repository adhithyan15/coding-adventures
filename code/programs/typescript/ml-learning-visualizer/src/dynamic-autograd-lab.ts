export type DynamicAutogradScenarioId =
  | "multiply_add_square"
  | "negative_branch"
  | "saved_snapshot";

export type DeclaredAutogradOperation =
  | "multiply"
  | "add"
  | "square"
  | "negate"
  | "branch_nonnegative";

export type ExecutedAutogradOperation =
  | "input"
  | "multiply"
  | "add"
  | "square"
  | "negate"
  | "identity";

export interface AutogradInput {
  id: string;
  value: number;
  requiresGradient: true;
}

export interface AutogradStep {
  id: string;
  operation: DeclaredAutogradOperation;
  inputs: readonly string[];
}

export interface DynamicAutogradScenario {
  id: DynamicAutogradScenarioId;
  title: string;
  summary: string;
  expression: string;
  inputs: readonly AutogradInput[];
  steps: readonly AutogradStep[];
  output: string;
  mutationsAfterForward: Readonly<Record<string, number>>;
}

export interface SavedAutogradValue {
  name: "left" | "right" | "input";
  sourceId: string;
  value: number;
}

export interface AutogradNodeTrace {
  id: string;
  operation: ExecutedAutogradOperation;
  parents: string[];
  forwardValue: number;
  savedValues: SavedAutogradValue[];
}

export interface AutogradDerivativeTrace {
  parentId: string;
  value: number;
  source: string;
}

export interface AutogradContributionTrace {
  parentId: string;
  value: number;
}

export interface AutogradBackwardStep {
  nodeId: string;
  operation: Exclude<ExecutedAutogradOperation, "input">;
  upstreamGradient: number;
  localDerivatives: AutogradDerivativeTrace[];
  parentContributions: AutogradContributionTrace[];
}

export interface DynamicAutogradTrace {
  scenario: DynamicAutogradScenario;
  nodes: AutogradNodeTrace[];
  topologicalOrder: string[];
  backwardOrder: string[];
  branchChoices: Record<string, "nonnegative" | "negative">;
  liveInputValues: Record<string, number>;
  backwardSteps: AutogradBackwardStep[];
  gradients: Record<string, number>;
  finiteDifferenceGradients: Record<string, number>;
  gradientAbsoluteErrors: Record<string, number>;
  maxGradientAbsoluteError: number;
}

const MAX_INPUTS = 4;
const MAX_STEPS = 12;
const MAX_ABSOLUTE_INPUT = 1e6;
const IDENTIFIER = /^[a-z][a-z0-9_]{0,31}$/;

export const DYNAMIC_AUTOGRAD_SCENARIOS: readonly DynamicAutogradScenario[] = [
  {
    id: "multiply_add_square",
    title: "Complete graph",
    summary: "Multiply, add, and square with every saved value visible.",
    expression: "loss = (x × w + b)²",
    inputs: [
      { id: "x", value: 2, requiresGradient: true },
      { id: "w", value: 3, requiresGradient: true },
      { id: "b", value: 1, requiresGradient: true },
    ],
    steps: [
      { id: "m", operation: "multiply", inputs: ["x", "w"] },
      { id: "z", operation: "add", inputs: ["m", "b"] },
      { id: "loss", operation: "square", inputs: ["z"] },
    ],
    output: "loss",
    mutationsAfterForward: {},
  },
  {
    id: "negative_branch",
    title: "Runtime branch",
    summary: "A negative input records negate, not the unexecuted identity path.",
    expression: "loss = abs(x)², x < 0",
    inputs: [{ id: "x", value: -2, requiresGradient: true }],
    steps: [
      { id: "abs_x", operation: "branch_nonnegative", inputs: ["x"] },
      { id: "loss", operation: "square", inputs: ["abs_x"] },
    ],
    output: "loss",
    mutationsAfterForward: {},
  },
  {
    id: "saved_snapshot",
    title: "Mutation snapshot",
    summary: "Live w becomes 100; backward still reads saved forward w = 3.",
    expression: "product = x × w; then live w ← 100",
    inputs: [
      { id: "x", value: 2, requiresGradient: true },
      { id: "w", value: 3, requiresGradient: true },
    ],
    steps: [{ id: "product", operation: "multiply", inputs: ["x", "w"] }],
    output: "product",
    mutationsAfterForward: { w: 100 },
  },
] as const;

function requireFinite(value: number, context: string): number {
  if (!Number.isFinite(value)) throw new Error(`${context} must remain finite`);
  return value;
}

function validateIdentifier(value: unknown, context: string): asserts value is string {
  if (typeof value !== "string" || !IDENTIFIER.test(value)) {
    throw new Error(`${context} must be a bounded identifier`);
  }
}

function validateInputValue(
  value: unknown,
  context: string,
  allowedHeadroom = 0,
): asserts value is number {
  if (typeof value !== "number" || !Number.isFinite(value)
    || Math.abs(value) > MAX_ABSOLUTE_INPUT + allowedHeadroom) {
    throw new Error(`${context} must be finite and bounded`);
  }
}

function snapshotScenario(scenario: DynamicAutogradScenario): DynamicAutogradScenario {
  const mutations: Record<string, number> = Object.create(null);
  Object.entries(scenario.mutationsAfterForward).forEach(([id, value]) => {
    mutations[id] = value;
  });
  const inputs = scenario.inputs.map((input) => Object.freeze({ ...input }));
  const steps = scenario.steps.map((step) => Object.freeze({
    ...step,
    inputs: Object.freeze([...step.inputs]),
  }));
  return Object.freeze({
    id: scenario.id,
    title: scenario.title,
    summary: scenario.summary,
    expression: scenario.expression,
    inputs: Object.freeze(inputs),
    steps: Object.freeze(steps),
    output: scenario.output,
    mutationsAfterForward: Object.freeze(mutations),
  });
}

function expectedArity(operation: DeclaredAutogradOperation): number {
  return operation === "multiply" || operation === "add" ? 2 : 1;
}

function validateScenario(scenario: DynamicAutogradScenario): void {
  if (typeof scenario !== "object" || scenario === null
    || !Array.isArray(scenario.inputs) || !Array.isArray(scenario.steps)
    || typeof scenario.mutationsAfterForward !== "object"
    || scenario.mutationsAfterForward === null
    || Array.isArray(scenario.mutationsAfterForward)) {
    throw new Error("autograd scenario must contain bounded arrays and mutation object");
  }
  if (scenario.inputs.length < 1 || scenario.inputs.length > MAX_INPUTS
    || scenario.steps.length < 1 || scenario.steps.length > MAX_STEPS) {
    throw new Error("autograd scenario exceeds the bounded graph size");
  }
  const known = new Set<string>();
  scenario.inputs.forEach((input, index) => {
    if (typeof input !== "object" || input === null) throw new Error("input must be an object");
    validateIdentifier(input.id, `input ${index} id`);
    validateInputValue(input.value, `input ${input.id}`);
    if (input.requiresGradient !== true || known.has(input.id)) {
      throw new Error("inputs must require gradients and have unique ids");
    }
    known.add(input.id);
  });
  scenario.steps.forEach((step, index) => {
    if (typeof step !== "object" || step === null || !Array.isArray(step.inputs)) {
      throw new Error("step must contain an inputs array");
    }
    validateIdentifier(step.id, `step ${index} id`);
    if (known.has(step.id) || ![
      "multiply", "add", "square", "negate", "branch_nonnegative",
    ].includes(step.operation)) {
      throw new Error("step id or operation is invalid");
    }
    if (step.inputs.length !== expectedArity(step.operation)) {
      throw new Error(`${step.operation} has invalid arity`);
    }
    step.inputs.forEach((parentId) => {
      validateIdentifier(parentId, `step ${step.id} parent`);
      if (!known.has(parentId)) throw new Error(`step ${step.id} parent must already exist`);
    });
    known.add(step.id);
  });
  if (scenario.output !== scenario.steps.at(-1)!.id) {
    throw new Error("autograd output must be the final executed step");
  }
  const inputIds = new Set(scenario.inputs.map((input) => input.id));
  const mutationEntries = Object.entries(scenario.mutationsAfterForward);
  if (mutationEntries.length > MAX_INPUTS) throw new Error("too many live mutations");
  mutationEntries.forEach(([id, value]) => {
    validateIdentifier(id, "mutation id");
    validateInputValue(value, `mutation ${id}`);
    if (!inputIds.has(id)) throw new Error(`mutation ${id} must target an input`);
  });
}

function forwardGraph(
  scenario: DynamicAutogradScenario,
  overrides: Readonly<Record<string, number>> = {},
  allowedInputHeadroom = 0,
): { nodes: AutogradNodeTrace[]; branches: Record<string, "nonnegative" | "negative"> } {
  const nodes: AutogradNodeTrace[] = [];
  const byId = new Map<string, AutogradNodeTrace>();
  const branches: Record<string, "nonnegative" | "negative"> = Object.create(null);
  scenario.inputs.forEach((input) => {
    const value = Object.prototype.hasOwnProperty.call(overrides, input.id)
      ? overrides[input.id]!
      : input.value;
    validateInputValue(value, `input ${input.id}`, allowedInputHeadroom);
    const node: AutogradNodeTrace = {
      id: input.id,
      operation: "input",
      parents: [],
      forwardValue: value,
      savedValues: [],
    };
    nodes.push(node);
    byId.set(node.id, node);
  });
  scenario.steps.forEach((step) => {
    const parents = step.inputs.map((id) => byId.get(id)!);
    const values = parents.map((parent) => parent.forwardValue);
    let operation: ExecutedAutogradOperation = step.operation;
    let forwardValue: number;
    let savedValues: SavedAutogradValue[] = [];
    if (step.operation === "multiply") {
      forwardValue = requireFinite(values[0]! * values[1]!, `${step.id} product`);
      savedValues = [
        { name: "left", sourceId: parents[0]!.id, value: values[0]! },
        { name: "right", sourceId: parents[1]!.id, value: values[1]! },
      ];
    } else if (step.operation === "add") {
      forwardValue = requireFinite(values[0]! + values[1]!, `${step.id} sum`);
    } else if (step.operation === "square") {
      forwardValue = requireFinite(values[0]! * values[0]!, `${step.id} square`);
      savedValues = [{ name: "input", sourceId: parents[0]!.id, value: values[0]! }];
    } else if (step.operation === "negate") {
      forwardValue = requireFinite(-values[0]!, `${step.id} negation`);
    } else if (values[0]! >= 0) {
      operation = "identity";
      branches[step.id] = "nonnegative";
      forwardValue = values[0]!;
    } else {
      operation = "negate";
      branches[step.id] = "negative";
      forwardValue = requireFinite(-values[0]!, `${step.id} branch negation`);
    }
    const node: AutogradNodeTrace = {
      id: step.id,
      operation,
      parents: [...step.inputs],
      forwardValue,
      savedValues,
    };
    nodes.push(node);
    byId.set(node.id, node);
  });
  return { nodes, branches };
}

function topologicalOrder(nodes: readonly AutogradNodeTrace[], output: string): string[] {
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const visited = new Set<string>();
  const order: string[] = [];
  function visit(id: string): void {
    if (visited.has(id)) return;
    visited.add(id);
    byId.get(id)!.parents.forEach(visit);
    order.push(id);
  }
  visit(output);
  return order;
}

function saved(node: AutogradNodeTrace, name: SavedAutogradValue["name"]): number {
  const match = node.savedValues.find((item) => item.name === name);
  if (!match) throw new Error(`${node.id} is missing saved ${name}`);
  return match.value;
}

function localDerivatives(node: AutogradNodeTrace): AutogradDerivativeTrace[] {
  if (node.operation === "multiply") return [
    { parentId: node.parents[0]!, value: saved(node, "right"), source: "saved:right" },
    { parentId: node.parents[1]!, value: saved(node, "left"), source: "saved:left" },
  ];
  if (node.operation === "add") return [
    { parentId: node.parents[0]!, value: 1, source: "constant:1" },
    { parentId: node.parents[1]!, value: 1, source: "constant:1" },
  ];
  if (node.operation === "square") return [{
    parentId: node.parents[0]!,
    value: requireFinite(2 * saved(node, "input"), `${node.id} derivative`),
    source: "saved:input",
  }];
  if (node.operation === "negate") return [
    { parentId: node.parents[0]!, value: -1, source: "constant:-1" },
  ];
  if (node.operation === "identity") return [
    { parentId: node.parents[0]!, value: 1, source: "constant:1" },
  ];
  throw new Error(`cannot differentiate ${node.operation}`);
}

function outputFor(
  scenario: DynamicAutogradScenario,
  overrides: Readonly<Record<string, number>>,
  finiteDifferenceEpsilon: number,
): number {
  return forwardGraph(scenario, overrides, finiteDifferenceEpsilon).nodes.at(-1)!.forwardValue;
}

export function traceDynamicAutogradProgram(
  scenario: DynamicAutogradScenario,
  finiteDifferenceEpsilon = 1e-5,
  applyMutations = true,
): DynamicAutogradTrace {
  validateScenario(scenario);
  if (!Number.isFinite(finiteDifferenceEpsilon)
    || finiteDifferenceEpsilon < 1e-12 || finiteDifferenceEpsilon > 1) {
    throw new Error("finite-difference epsilon must be finite and in [1e-12, 1]");
  }
  const scenarioSnapshot = snapshotScenario(scenario);
  const { nodes, branches } = forwardGraph(scenarioSnapshot);
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const topo = topologicalOrder(nodes, scenarioSnapshot.output);
  const backwardOrder = [...topo].reverse();
  const gradients: Record<string, number> = Object.create(null);
  gradients[scenarioSnapshot.output] = 1;
  const backwardSteps: AutogradBackwardStep[] = [];
  backwardOrder.forEach((nodeId) => {
    const node = byId.get(nodeId)!;
    const upstream = gradients[nodeId];
    if (upstream === undefined || node.operation === "input") return;
    const derivatives = localDerivatives(node);
    const contributions = derivatives.map((derivative) => {
      const value = requireFinite(
        upstream * derivative.value,
        `${node.id} parent contribution`,
      );
      gradients[derivative.parentId] = requireFinite(
        (gradients[derivative.parentId] ?? 0) + value,
        `${derivative.parentId} accumulated gradient`,
      );
      return { parentId: derivative.parentId, value };
    });
    backwardSteps.push({
      nodeId,
      operation: node.operation,
      upstreamGradient: upstream,
      localDerivatives: derivatives,
      parentContributions: contributions,
    });
  });

  const original = Object.fromEntries(
    scenarioSnapshot.inputs.map((input) => [input.id, input.value]),
  );
  const finiteDifferenceGradients: Record<string, number> = Object.create(null);
  const gradientAbsoluteErrors: Record<string, number> = Object.create(null);
  scenarioSnapshot.inputs.forEach((input) => {
    const plus = { ...original, [input.id]: input.value + finiteDifferenceEpsilon };
    const minus = { ...original, [input.id]: input.value - finiteDifferenceEpsilon };
    const numerical = requireFinite(
      (outputFor(scenarioSnapshot, plus, finiteDifferenceEpsilon)
        - outputFor(scenarioSnapshot, minus, finiteDifferenceEpsilon))
        / (2 * finiteDifferenceEpsilon),
      `${input.id} finite difference`,
    );
    finiteDifferenceGradients[input.id] = numerical;
    gradientAbsoluteErrors[input.id] = Math.abs(gradients[input.id]! - numerical);
  });
  return {
    scenario: scenarioSnapshot,
    nodes,
    topologicalOrder: topo,
    backwardOrder,
    branchChoices: branches,
    liveInputValues: applyMutations
      ? { ...original, ...scenarioSnapshot.mutationsAfterForward }
      : original,
    backwardSteps,
    gradients,
    finiteDifferenceGradients,
    gradientAbsoluteErrors,
    maxGradientAbsoluteError: Math.max(...Object.values(gradientAbsoluteErrors), 0),
  };
}

export function traceDynamicAutograd(
  scenarioId: DynamicAutogradScenarioId,
  applyMutations = true,
): DynamicAutogradTrace {
  const scenario = DYNAMIC_AUTOGRAD_SCENARIOS.find((item) => item.id === scenarioId);
  if (!scenario) throw new Error(`unknown dynamic autograd scenario: ${scenarioId}`);
  return traceDynamicAutogradProgram(scenario, 1e-5, applyMutations);
}
