const PIVOT_EPSILON = 1.0e-12;

export type Element = Resistor | Capacitor | Inductor | VoltageSource | CurrentSource;

export interface Resistor {
  readonly kind: "resistor";
  readonly name: string;
  readonly n1: string;
  readonly n2: string;
  readonly resistanceOhms: number;
}

export interface Capacitor {
  readonly kind: "capacitor";
  readonly name: string;
  readonly n1: string;
  readonly n2: string;
  readonly capacitanceFarads: number;
  readonly initialVoltage: number;
}

export interface Inductor {
  readonly kind: "inductor";
  readonly name: string;
  readonly n1: string;
  readonly n2: string;
  readonly inductanceHenrys: number;
  readonly initialCurrent: number;
}

export interface VoltageSource {
  readonly kind: "voltage-source";
  readonly name: string;
  readonly positive: string;
  readonly negative: string;
  readonly voltage: number;
}

export interface CurrentSource {
  readonly kind: "current-source";
  readonly name: string;
  readonly positive: string;
  readonly negative: string;
  readonly current: number;
}

export interface DcResult {
  readonly nodeVoltages: ReadonlyMap<string, number>;
  readonly branchCurrents: ReadonlyMap<string, number>;
  voltage(node: string): number | undefined;
  branchCurrent(sourceName: string): number | undefined;
}

export interface TransientPoint {
  readonly time: number;
  readonly nodeVoltages: ReadonlyMap<string, number>;
  readonly branchCurrents: ReadonlyMap<string, number>;
  voltage(node: string): number | undefined;
  branchCurrent(sourceName: string): number | undefined;
}

export class SpiceError extends Error {
  constructor(
    message: string,
    readonly code: "INVALID_ELEMENT" | "SINGULAR_MATRIX",
    readonly elementName?: string,
  ) {
    super(message);
    this.name = "SpiceError";
  }
}

export class Circuit {
  private readonly _elements: Element[] = [];

  add(element: Element): void {
    this._elements.push(element);
  }

  elements(): readonly Element[] {
    return this._elements;
  }
}

export function resistor(
  name: string,
  n1: string,
  n2: string,
  resistanceOhms: number,
): Resistor {
  return { kind: "resistor", name, n1, n2, resistanceOhms };
}

export function capacitor(
  name: string,
  n1: string,
  n2: string,
  capacitanceFarads: number,
): Capacitor {
  return capacitorWithInitialVoltage(name, n1, n2, capacitanceFarads, 0.0);
}

export function capacitorWithInitialVoltage(
  name: string,
  n1: string,
  n2: string,
  capacitanceFarads: number,
  initialVoltage: number,
): Capacitor {
  return {
    kind: "capacitor",
    name,
    n1,
    n2,
    capacitanceFarads,
    initialVoltage,
  };
}

export function inductor(
  name: string,
  n1: string,
  n2: string,
  inductanceHenrys: number,
): Inductor {
  return inductorWithInitialCurrent(name, n1, n2, inductanceHenrys, 0.0);
}

export function inductorWithInitialCurrent(
  name: string,
  n1: string,
  n2: string,
  inductanceHenrys: number,
  initialCurrent: number,
): Inductor {
  return {
    kind: "inductor",
    name,
    n1,
    n2,
    inductanceHenrys,
    initialCurrent,
  };
}

export function voltageSource(
  name: string,
  positive: string,
  negative: string,
  voltage: number,
): VoltageSource {
  return { kind: "voltage-source", name, positive, negative, voltage };
}

export function currentSource(
  name: string,
  positive: string,
  negative: string,
  current: number,
): CurrentSource {
  return { kind: "current-source", name, positive, negative, current };
}

export function dcOp(circuit: Circuit): DcResult {
  const solution = solveLinearCircuit(circuit, [], []);
  return makeDcResult(solution.nodeVoltages, solution.branchCurrents);
}

export function transient(
  circuit: Circuit,
  timeStep: number,
  stopTime: number,
): TransientPoint[] {
  if (!Number.isFinite(timeStep) || timeStep <= 0.0) {
    throw invalidElement("transient", "time step must be finite and positive");
  }
  if (!Number.isFinite(stopTime) || stopTime < 0.0) {
    throw invalidElement("transient", "stop time must be finite and non-negative");
  }

  validateReactiveElements(circuit);

  const capacitorStates = initialCapacitorStates(circuit, timeStep);
  const inductorStates = initialInductorStates(circuit, timeStep);
  const points: TransientPoint[] = [];
  for (let time = timeStep; time <= stopTime + timeStep * 1.0e-9; time += timeStep) {
    const solution = solveLinearCircuit(circuit, capacitorStates, inductorStates);
    updateCapacitorStates(circuit, solution.nodeVoltages, capacitorStates);
    updateInductorStates(circuit, solution.nodeVoltages, inductorStates);
    points.push(
      makeTransientPoint(time, solution.nodeVoltages, solution.branchCurrents),
    );
  }
  return points;
}

interface CapacitorState {
  readonly name: string;
  previousVoltage: number;
  readonly timeStep: number;
}

interface InductorState {
  readonly name: string;
  previousCurrent: number;
  readonly timeStep: number;
}

interface LinearSolution {
  readonly nodeVoltages: ReadonlyMap<string, number>;
  readonly branchCurrents: ReadonlyMap<string, number>;
}

function solveLinearCircuit(
  circuit: Circuit,
  capacitorStates: readonly CapacitorState[],
  inductorStates: readonly InductorState[],
): LinearSolution {
  const nodeIndices = collectNodeIndices(circuit);
  const voltageSources = collectVoltageSources(circuit, inductorStates);
  const nodeCount = nodeIndices.size;
  const branchCount = voltageSources.size;
  const matrixSize = nodeCount + branchCount;

  if (matrixSize === 0) {
    return { nodeVoltages: new Map(), branchCurrents: new Map() };
  }

  const matrix = Array.from({ length: matrixSize }, () =>
    Array.from({ length: matrixSize }, () => 0.0),
  );
  const rhs = Array.from({ length: matrixSize }, () => 0.0);

  for (const element of circuit.elements()) {
    switch (element.kind) {
      case "resistor":
        stampResistor(element, nodeIndices, matrix);
        break;
      case "capacitor":
        stampCapacitor(element, capacitorStates, nodeIndices, matrix, rhs);
        break;
      case "inductor":
        stampInductor(
          element,
          inductorStates,
          nodeIndices,
          voltageSources,
          nodeCount,
          matrix,
          rhs,
        );
        break;
      case "voltage-source":
        stampVoltageSource(
          element,
          nodeIndices,
          voltageSources,
          nodeCount,
          matrix,
          rhs,
        );
        break;
      case "current-source":
        stampCurrentSource(element, nodeIndices, rhs);
        break;
    }
  }

  const solution = solveLinearSystem(matrix, rhs);
  const nodeVoltages = new Map<string, number>();
  const nodesByIndex = Array.from(nodeIndices.entries()).sort(
    ([, a], [, b]) => a - b,
  );
  for (const [node, index] of nodesByIndex) {
    nodeVoltages.set(node, solution[index]);
  }

  const branchCurrents = new Map<string, number>();
  for (const [sourceName, branchIndex] of voltageSources.entries()) {
    branchCurrents.set(`I(${sourceName})`, solution[nodeCount + branchIndex]);
  }
  insertTransientInductorCurrents(
    circuit,
    inductorStates,
    nodeVoltages,
    branchCurrents,
  );

  return { nodeVoltages, branchCurrents };
}

function makeDcResult(
  nodeVoltages: ReadonlyMap<string, number>,
  branchCurrents: ReadonlyMap<string, number>,
): DcResult {
  return {
    nodeVoltages,
    branchCurrents,
    voltage(node: string): number | undefined {
      return isGround(node) ? 0.0 : nodeVoltages.get(node);
    },
    branchCurrent(sourceName: string): number | undefined {
      const key = sourceName.startsWith("I(")
        ? sourceName
        : `I(${sourceName})`;
      return branchCurrents.get(key);
    },
  };
}

function makeTransientPoint(
  time: number,
  nodeVoltages: ReadonlyMap<string, number>,
  branchCurrents: ReadonlyMap<string, number>,
): TransientPoint {
  return {
    time,
    nodeVoltages,
    branchCurrents,
    voltage(node: string): number | undefined {
      return isGround(node) ? 0.0 : nodeVoltages.get(node);
    },
    branchCurrent(sourceName: string): number | undefined {
      const key = sourceName.startsWith("I(")
        ? sourceName
        : `I(${sourceName})`;
      return branchCurrents.get(key);
    },
  };
}

function collectNodeIndices(circuit: Circuit): Map<string, number> {
  const names = new Set<string>();
  for (const element of circuit.elements()) {
    switch (element.kind) {
      case "resistor":
      case "capacitor":
      case "inductor":
        insertNode(names, element.n1);
        insertNode(names, element.n2);
        break;
      case "voltage-source":
      case "current-source":
        insertNode(names, element.positive);
        insertNode(names, element.negative);
        break;
    }
  }

  const nodeIndices = new Map<string, number>();
  for (const node of Array.from(names).sort()) {
    nodeIndices.set(node, nodeIndices.size);
  }
  return nodeIndices;
}

function collectVoltageSources(
  circuit: Circuit,
  inductorStates: readonly InductorState[],
): Map<string, number> {
  const sources = new Map<string, number>();
  for (const element of circuit.elements()) {
    if (element.kind === "voltage-source") {
      insertBranchName(sources, element.name, "duplicate voltage source name");
    } else if (element.kind === "inductor") {
      if (sources.has(element.name)) {
        throw invalidElement(element.name, "duplicate branch element name");
      }
      if (!inductorStates.some((state) => state.name === element.name)) {
        sources.set(element.name, sources.size);
      }
    }
  }
  return sources;
}

function insertBranchName(
  sources: Map<string, number>,
  name: string,
  duplicateReason: string,
): void {
  if (sources.has(name)) {
    throw invalidElement(name, duplicateReason);
  }
  sources.set(name, sources.size);
}

function insertNode(nodes: Set<string>, node: string): void {
  if (!isGround(node)) {
    nodes.add(node);
  }
}

function isGround(node: string): boolean {
  return node === "0" || node.toLowerCase() === "gnd";
}

function nodeIndex(
  nodeIndices: ReadonlyMap<string, number>,
  node: string,
): number | undefined {
  return isGround(node) ? undefined : nodeIndices.get(node);
}

function stampResistor(
  element: Resistor,
  nodeIndices: ReadonlyMap<string, number>,
  matrix: number[][],
): void {
  if (!Number.isFinite(element.resistanceOhms) || element.resistanceOhms <= 0) {
    throw invalidElement(element.name, "resistance must be finite and positive");
  }

  const conductance = 1.0 / element.resistanceOhms;
  const n1 = nodeIndex(nodeIndices, element.n1);
  const n2 = nodeIndex(nodeIndices, element.n2);
  stampConductance(matrix, n1, n2, conductance);
}

function stampCapacitor(
  element: Capacitor,
  capacitorStates: readonly CapacitorState[],
  nodeIndices: ReadonlyMap<string, number>,
  matrix: number[][],
  rhs: number[],
): void {
  validateCapacitor(element);
  const state = capacitorStates.find((candidate) => candidate.name === element.name);
  if (state === undefined) {
    return;
  }

  const conductance = element.capacitanceFarads / state.timeStep;
  const n1 = nodeIndex(nodeIndices, element.n1);
  const n2 = nodeIndex(nodeIndices, element.n2);
  stampConductance(matrix, n1, n2, conductance);

  const historyCurrent = conductance * state.previousVoltage;
  if (n1 !== undefined) {
    rhs[n1] += historyCurrent;
  }
  if (n2 !== undefined) {
    rhs[n2] -= historyCurrent;
  }
}

function stampInductor(
  element: Inductor,
  inductorStates: readonly InductorState[],
  nodeIndices: ReadonlyMap<string, number>,
  voltageSources: ReadonlyMap<string, number>,
  nodeCount: number,
  matrix: number[][],
  rhs: number[],
): void {
  validateInductor(element);
  const n1 = nodeIndex(nodeIndices, element.n1);
  const n2 = nodeIndex(nodeIndices, element.n2);
  const state = inductorStates.find((candidate) => candidate.name === element.name);
  if (state === undefined) {
    stampZeroVoltageBranch(element.name, voltageSources, nodeCount, matrix, n1, n2);
    return;
  }

  const conductance = state.timeStep / element.inductanceHenrys;
  stampConductance(matrix, n1, n2, conductance);
  if (n1 !== undefined) {
    rhs[n1] -= state.previousCurrent;
  }
  if (n2 !== undefined) {
    rhs[n2] += state.previousCurrent;
  }
}

function stampZeroVoltageBranch(
  name: string,
  voltageSources: ReadonlyMap<string, number>,
  nodeCount: number,
  matrix: number[][],
  positive: number | undefined,
  negative: number | undefined,
): void {
  const sourceIndex = voltageSources.get(name);
  if (sourceIndex === undefined) {
    throw invalidElement(name, "branch element was not indexed");
  }
  const branch = nodeCount + sourceIndex;
  stampBranchMatrix(matrix, branch, positive, negative);
}

function stampConductance(
  matrix: number[][],
  n1: number | undefined,
  n2: number | undefined,
  conductance: number,
): void {
  if (n1 !== undefined) {
    matrix[n1][n1] += conductance;
  }
  if (n2 !== undefined) {
    matrix[n2][n2] += conductance;
  }
  if (n1 !== undefined && n2 !== undefined) {
    matrix[n1][n2] -= conductance;
    matrix[n2][n1] -= conductance;
  }
}

function validateReactiveElements(circuit: Circuit): void {
  for (const element of circuit.elements()) {
    if (element.kind === "capacitor") {
      validateCapacitor(element);
    } else if (element.kind === "inductor") {
      validateInductor(element);
    }
  }
}

function validateCapacitor(element: Capacitor): void {
  if (!Number.isFinite(element.capacitanceFarads) || element.capacitanceFarads <= 0.0) {
    throw invalidElement(element.name, "capacitance must be finite and positive");
  }
  if (!Number.isFinite(element.initialVoltage)) {
    throw invalidElement(element.name, "initial voltage must be finite");
  }
}

function validateInductor(element: Inductor): void {
  if (!Number.isFinite(element.inductanceHenrys) || element.inductanceHenrys <= 0.0) {
    throw invalidElement(element.name, "inductance must be finite and positive");
  }
  if (!Number.isFinite(element.initialCurrent)) {
    throw invalidElement(element.name, "initial current must be finite");
  }
}

function initialCapacitorStates(
  circuit: Circuit,
  timeStep: number,
): CapacitorState[] {
  const states: CapacitorState[] = [];
  for (const element of circuit.elements()) {
    if (element.kind === "capacitor") {
      states.push({
        name: element.name,
        previousVoltage: element.initialVoltage,
        timeStep,
      });
    }
  }
  return states;
}

function initialInductorStates(
  circuit: Circuit,
  timeStep: number,
): InductorState[] {
  const states: InductorState[] = [];
  for (const element of circuit.elements()) {
    if (element.kind === "inductor") {
      states.push({
        name: element.name,
        previousCurrent: element.initialCurrent,
        timeStep,
      });
    }
  }
  return states;
}

function updateCapacitorStates(
  circuit: Circuit,
  nodeVoltages: ReadonlyMap<string, number>,
  capacitorStates: CapacitorState[],
): void {
  for (const state of capacitorStates) {
    const element = circuit
      .elements()
      .find(
        (candidate): candidate is Capacitor =>
          candidate.kind === "capacitor" && candidate.name === state.name,
      );
    if (element === undefined) {
      continue;
    }
    state.previousVoltage =
      voltageAt(nodeVoltages, element.n1) - voltageAt(nodeVoltages, element.n2);
  }
}

function updateInductorStates(
  circuit: Circuit,
  nodeVoltages: ReadonlyMap<string, number>,
  inductorStates: InductorState[],
): void {
  for (const state of inductorStates) {
    const element = circuit
      .elements()
      .find(
        (candidate): candidate is Inductor =>
          candidate.kind === "inductor" && candidate.name === state.name,
      );
    if (element === undefined) {
      continue;
    }
    state.previousCurrent = inductorCurrent(element, state, nodeVoltages);
  }
}

function insertTransientInductorCurrents(
  circuit: Circuit,
  inductorStates: readonly InductorState[],
  nodeVoltages: ReadonlyMap<string, number>,
  branchCurrents: Map<string, number>,
): void {
  for (const state of inductorStates) {
    const element = circuit
      .elements()
      .find(
        (candidate): candidate is Inductor =>
          candidate.kind === "inductor" && candidate.name === state.name,
      );
    if (element === undefined) {
      continue;
    }
    branchCurrents.set(
      `I(${element.name})`,
      inductorCurrent(element, state, nodeVoltages),
    );
  }
}

function inductorCurrent(
  element: Inductor,
  state: InductorState,
  nodeVoltages: ReadonlyMap<string, number>,
): number {
  const conductance = state.timeStep / element.inductanceHenrys;
  const voltage = voltageAt(nodeVoltages, element.n1) - voltageAt(nodeVoltages, element.n2);
  return state.previousCurrent + conductance * voltage;
}

function voltageAt(
  nodeVoltages: ReadonlyMap<string, number>,
  node: string,
): number {
  return isGround(node) ? 0.0 : nodeVoltages.get(node) ?? 0.0;
}

function stampVoltageSource(
  element: VoltageSource,
  nodeIndices: ReadonlyMap<string, number>,
  voltageSources: ReadonlyMap<string, number>,
  nodeCount: number,
  matrix: number[][],
  rhs: number[],
): void {
  if (!Number.isFinite(element.voltage)) {
    throw invalidElement(element.name, "voltage must be finite");
  }

  const sourceIndex = voltageSources.get(element.name);
  if (sourceIndex === undefined) {
    throw invalidElement(element.name, "voltage source was not indexed");
  }

  const branch = nodeCount + sourceIndex;
  const positive = nodeIndex(nodeIndices, element.positive);
  const negative = nodeIndex(nodeIndices, element.negative);

  stampBranchMatrix(matrix, branch, positive, negative);
  rhs[branch] += element.voltage;
}

function stampBranchMatrix(
  matrix: number[][],
  branch: number,
  positive: number | undefined,
  negative: number | undefined,
): void {
  if (positive !== undefined) {
    matrix[positive][branch] += 1.0;
    matrix[branch][positive] += 1.0;
  }
  if (negative !== undefined) {
    matrix[negative][branch] -= 1.0;
    matrix[branch][negative] -= 1.0;
  }
}

function stampCurrentSource(
  element: CurrentSource,
  nodeIndices: ReadonlyMap<string, number>,
  rhs: number[],
): void {
  if (!Number.isFinite(element.current)) {
    throw invalidElement(element.name, "current must be finite");
  }

  const positive = nodeIndex(nodeIndices, element.positive);
  const negative = nodeIndex(nodeIndices, element.negative);
  if (positive !== undefined) {
    rhs[positive] -= element.current;
  }
  if (negative !== undefined) {
    rhs[negative] += element.current;
  }
}

function solveLinearSystem(matrix: number[][], rhs: number[]): number[] {
  const n = rhs.length;
  for (let pivotCol = 0; pivotCol < n; pivotCol++) {
    let pivotRow = pivotCol;
    let pivotAbs = Math.abs(matrix[pivotCol][pivotCol]);
    for (let row = pivotCol + 1; row < n; row++) {
      const candidateAbs = Math.abs(matrix[row][pivotCol]);
      if (candidateAbs > pivotAbs) {
        pivotAbs = candidateAbs;
        pivotRow = row;
      }
    }

    if (pivotAbs < PIVOT_EPSILON) {
      throw new SpiceError(
        "circuit matrix is singular",
        "SINGULAR_MATRIX",
      );
    }

    [matrix[pivotCol], matrix[pivotRow]] = [
      matrix[pivotRow],
      matrix[pivotCol],
    ];
    [rhs[pivotCol], rhs[pivotRow]] = [rhs[pivotRow], rhs[pivotCol]];

    const pivot = matrix[pivotCol][pivotCol];
    for (let row = pivotCol + 1; row < n; row++) {
      const factor = matrix[row][pivotCol] / pivot;
      if (factor === 0.0) {
        continue;
      }
      matrix[row][pivotCol] = 0.0;
      for (let col = pivotCol + 1; col < n; col++) {
        matrix[row][col] -= factor * matrix[pivotCol][col];
      }
      rhs[row] -= factor * rhs[pivotCol];
    }
  }

  const solution = Array.from({ length: n }, () => 0.0);
  for (let row = n - 1; row >= 0; row--) {
    let tailSum = 0.0;
    for (let col = row + 1; col < n; col++) {
      tailSum += matrix[row][col] * solution[col];
    }
    solution[row] = (rhs[row] - tailSum) / matrix[row][row];
  }
  return solution;
}

function invalidElement(name: string, reason: string): SpiceError {
  return new SpiceError(`invalid element ${name}: ${reason}`, "INVALID_ELEMENT", name);
}
