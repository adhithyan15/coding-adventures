const PIVOT_EPSILON = 1.0e-12;
const TWO_PI = Math.PI * 2.0;

export type Element =
  | Resistor
  | Capacitor
  | Inductor
  | VoltageSource
  | CurrentSource
  | Vccs;

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

export interface Vccs {
  readonly kind: "vccs";
  readonly name: string;
  readonly positive: string;
  readonly negative: string;
  readonly controlPositive: string;
  readonly controlNegative: string;
  readonly transconductanceSiemens: number;
}

export interface DcResult {
  readonly nodeVoltages: ReadonlyMap<string, number>;
  readonly branchCurrents: ReadonlyMap<string, number>;
  voltage(node: string): number | undefined;
  branchCurrent(sourceName: string): number | undefined;
}

export interface DcSweepPoint {
  readonly value: number;
  readonly result: DcResult;
}

export interface TfResult {
  readonly transferRatio: number;
  readonly inputImpedanceOhms: number;
  readonly outputImpedanceOhms: number;
  gain(): number;
}

export interface SensEntry {
  readonly elementName: string;
  readonly parameter: string;
  readonly nominalValue: number;
  readonly sensitivity: number;
  readonly relativeSensitivity: number;
}

export interface SensResult {
  readonly outputNode: string;
  readonly nominalVoltage: number;
  readonly entries: readonly SensEntry[];
  entry(elementName: string, parameter: string): SensEntry | undefined;
}

export interface Complex {
  readonly real: number;
  readonly imag: number;
}

export interface AcPoint {
  readonly frequencyHz: number;
  readonly nodeVoltages: ReadonlyMap<string, Complex>;
  readonly branchCurrents: ReadonlyMap<string, Complex>;
  voltage(node: string): Complex | undefined;
  branchCurrent(sourceName: string): Complex | undefined;
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

export function vccs(
  name: string,
  positive: string,
  negative: string,
  controlPositive: string,
  controlNegative: string,
  transconductanceSiemens: number,
): Vccs {
  return {
    kind: "vccs",
    name,
    positive,
    negative,
    controlPositive,
    controlNegative,
    transconductanceSiemens,
  };
}

export function complexAbs(value: Complex): number {
  return Math.hypot(value.real, value.imag);
}

export function complexPhase(value: Complex): number {
  return Math.atan2(value.imag, value.real);
}

export function dcOp(circuit: Circuit): DcResult {
  const solution = solveLinearCircuit(circuit, [], []);
  return makeDcResult(solution.nodeVoltages, solution.branchCurrents);
}

export function dcSweep(
  circuit: Circuit,
  sourceName: string,
  start: number,
  stop: number,
  step: number,
): DcSweepPoint[] {
  validateSweep(sourceName, start, stop, step);

  const points: DcSweepPoint[] = [];
  const epsilon = Math.abs(step) * 1.0e-9;
  for (
    let value = start;
    sweepIncludes(value, stop, step, epsilon);
    value += step
  ) {
    const swept = circuitWithSweptSource(circuit, sourceName, value);
    points.push({ value, result: dcOp(swept) });
  }
  return points;
}

export function tf(
  circuit: Circuit,
  outputNode: string,
  inputSource: string,
): TfResult {
  const nodeIndices = collectNodeIndices(circuit);
  if (!isGround(outputNode) && !nodeIndices.has(outputNode)) {
    throw invalidElement(outputNode, "output node was not found in circuit");
  }

  const input = findInputSource(circuit, inputSource);
  const voltageSources = collectAcVoltageSources(circuit);
  const nodeCount = nodeIndices.size;
  const matrix = buildSmallSignalMatrix(circuit, nodeIndices, voltageSources);
  const size = matrix.length;
  const outputIndex = nodeIndex(nodeIndices, outputNode);

  const forwardRhs = Array.from({ length: size }, () => 0.0);
  if (input.kind === "voltage-source") {
    const sourceIndex = voltageSources.get(input.name);
    if (sourceIndex === undefined) {
      throw invalidElement(input.name, "voltage source was not indexed");
    }
    forwardRhs[nodeCount + sourceIndex] = 1.0;
  } else {
    const positive = nodeIndex(nodeIndices, input.positive);
    const negative = nodeIndex(nodeIndices, input.negative);
    if (positive !== undefined) {
      forwardRhs[positive] -= 1.0;
    }
    if (negative !== undefined) {
      forwardRhs[negative] += 1.0;
    }
  }

  const forward = solveLinearSystem(cloneMatrix(matrix), forwardRhs);
  const transferRatio = outputIndex === undefined ? 0.0 : forward[outputIndex];
  const inputImpedanceOhms =
    input.kind === "voltage-source"
      ? voltageSourceInputImpedance(input, voltageSources, nodeCount, forward)
      : currentSourceInputImpedance(input, nodeIndices, forward);

  const outputRhs = Array.from({ length: size }, () => 0.0);
  if (outputIndex !== undefined) {
    outputRhs[outputIndex] = 1.0;
  }
  const output = solveLinearSystem(matrix, outputRhs);
  const outputImpedanceOhms =
    outputIndex === undefined ? 0.0 : output[outputIndex];

  return makeTfResult(transferRatio, inputImpedanceOhms, outputImpedanceOhms);
}

export function sensDc(circuit: Circuit, outputNode: string): SensResult {
  const nodeIndices = collectNodeIndices(circuit);
  if (!isGround(outputNode) && !nodeIndices.has(outputNode)) {
    throw invalidElement(outputNode, "output node was not found in circuit");
  }

  const nominal = dcOp(circuit);
  const nominalVoltage = nominal.voltage(outputNode) ?? 0.0;
  const entries: SensEntry[] = [];

  for (let elementIndex = 0; elementIndex < circuit.elements().length; elementIndex++) {
    const element = circuit.elements()[elementIndex];
    const parameter = elementParameter(element);
    if (parameter === undefined) {
      continue;
    }

    const delta = perturbationFor(parameter.nominalValue);
    const perturbed = circuitWithPerturbedElement(circuit, elementIndex, delta);
    const perturbedResult = dcOp(perturbed);
    const perturbedVoltage = perturbedResult.voltage(outputNode) ?? 0.0;
    const sensitivity = (perturbedVoltage - nominalVoltage) / delta;
    const relativeSensitivity =
      Math.abs(nominalVoltage) > 1.0e-30
        ? sensitivity * parameter.nominalValue / nominalVoltage
        : 0.0;

    entries.push({
      elementName: parameter.elementName,
      parameter: parameter.parameter,
      nominalValue: parameter.nominalValue,
      sensitivity,
      relativeSensitivity,
    });
  }

  entries.sort(
    (left, right) =>
      Math.abs(right.relativeSensitivity) - Math.abs(left.relativeSensitivity) ||
      left.elementName.localeCompare(right.elementName) ||
      left.parameter.localeCompare(right.parameter),
  );

  return makeSensResult(outputNode, nominalVoltage, entries);
}

export function acSweep(
  circuit: Circuit,
  startHz: number,
  stopHz: number,
  pointsPerDecade: number,
): AcPoint[] {
  if (!Number.isFinite(startHz) || !Number.isFinite(stopHz) || startHz <= 0.0 || stopHz <= 0.0) {
    throw invalidElement("acSweep", "frequency bounds must be finite and positive");
  }
  if (stopHz < startHz) {
    throw invalidElement(
      "acSweep",
      "stop frequency must be greater than or equal to start frequency",
    );
  }
  if (!Number.isInteger(pointsPerDecade) || pointsPerDecade <= 0) {
    throw invalidElement("acSweep", "points per decade must be positive");
  }

  validateReactiveElements(circuit);

  const points: AcPoint[] = [];
  const ratio = 10.0 ** (1.0 / pointsPerDecade);
  const epsilon = stopHz * 1.0e-12;
  for (
    let frequency = startHz;
    frequency <= stopHz + epsilon;
    frequency *= ratio
  ) {
    const solution = solveAcCircuit(circuit, TWO_PI * frequency);
    points.push(
      makeAcPoint(frequency, solution.nodeVoltages, solution.branchCurrents),
    );
  }
  return points;
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

function validateSweep(
  sourceName: string,
  start: number,
  stop: number,
  step: number,
): void {
  if (sourceName.length === 0) {
    throw invalidElement("dcSweep", "source name must not be empty");
  }
  if (
    !Number.isFinite(start) ||
    !Number.isFinite(stop) ||
    !Number.isFinite(step) ||
    step === 0.0
  ) {
    throw invalidElement(
      sourceName,
      "sweep bounds and step must be finite, with non-zero step",
    );
  }
  if (Math.sign(stop - start) !== Math.sign(step) && start !== stop) {
    throw invalidElement(
      sourceName,
      "sweep step direction must move from start toward stop",
    );
  }
}

function sweepIncludes(
  value: number,
  stop: number,
  step: number,
  epsilon: number,
): boolean {
  return step > 0.0 ? value <= stop + epsilon : value >= stop - epsilon;
}

function circuitWithSweptSource(
  circuit: Circuit,
  sourceName: string,
  value: number,
): Circuit {
  let found = false;
  const swept = new Circuit();
  for (const element of circuit.elements()) {
    if (element.kind === "voltage-source" && element.name === sourceName) {
      swept.add({ ...element, voltage: value });
      found = true;
    } else if (element.kind === "current-source" && element.name === sourceName) {
      swept.add({ ...element, current: value });
      found = true;
    } else {
      swept.add(element);
    }
  }
  if (!found) {
    throw invalidElement(
      sourceName,
      "sweep source must be an independent voltage or current source",
    );
  }
  return swept;
}

interface ElementParameter {
  readonly elementName: string;
  readonly parameter: string;
  readonly nominalValue: number;
}

function elementParameter(element: Element): ElementParameter | undefined {
  switch (element.kind) {
    case "resistor":
      return {
        elementName: element.name,
        parameter: "resistanceOhms",
        nominalValue: element.resistanceOhms,
      };
    case "voltage-source":
      return {
        elementName: element.name,
        parameter: "voltage",
        nominalValue: element.voltage,
      };
    case "current-source":
      return {
        elementName: element.name,
        parameter: "current",
        nominalValue: element.current,
      };
    case "vccs":
      return {
        elementName: element.name,
        parameter: "transconductanceSiemens",
        nominalValue: element.transconductanceSiemens,
      };
    case "capacitor":
    case "inductor":
      return undefined;
  }
}

function perturbationFor(value: number): number {
  return Math.max(Math.abs(value) * 1.0e-6, 1.0e-9);
}

function circuitWithPerturbedElement(
  circuit: Circuit,
  elementIndex: number,
  delta: number,
): Circuit {
  const perturbed = new Circuit();
  circuit.elements().forEach((element, index) => {
    if (index !== elementIndex) {
      perturbed.add(element);
      return;
    }

    switch (element.kind) {
      case "resistor":
        perturbed.add({
          ...element,
          resistanceOhms: element.resistanceOhms + delta,
        });
        break;
      case "voltage-source":
        perturbed.add({ ...element, voltage: element.voltage + delta });
        break;
      case "current-source":
        perturbed.add({ ...element, current: element.current + delta });
        break;
      case "vccs":
        perturbed.add({
          ...element,
          transconductanceSiemens: element.transconductanceSiemens + delta,
        });
        break;
      case "capacitor":
      case "inductor":
        perturbed.add(element);
        break;
    }
  });
  return perturbed;
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

interface AcSolution {
  readonly nodeVoltages: ReadonlyMap<string, Complex>;
  readonly branchCurrents: ReadonlyMap<string, Complex>;
}

type InputSource = VoltageSource | CurrentSource;

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
      case "vccs":
        stampVccs(element, nodeIndices, matrix);
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

function buildSmallSignalMatrix(
  circuit: Circuit,
  nodeIndices: ReadonlyMap<string, number>,
  voltageSources: ReadonlyMap<string, number>,
): number[][] {
  const nodeCount = nodeIndices.size;
  const matrixSize = nodeCount + voltageSources.size;
  const matrix = Array.from({ length: matrixSize }, () =>
    Array.from({ length: matrixSize }, () => 0.0),
  );

  for (const element of circuit.elements()) {
    switch (element.kind) {
      case "resistor":
        stampResistor(element, nodeIndices, matrix);
        break;
      case "capacitor":
        validateCapacitor(element);
        break;
      case "inductor": {
        validateInductor(element);
        const n1 = nodeIndex(nodeIndices, element.n1);
        const n2 = nodeIndex(nodeIndices, element.n2);
        stampConductance(matrix, n1, n2, 1.0e12);
        break;
      }
      case "voltage-source": {
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
        break;
      }
      case "current-source":
        if (!Number.isFinite(element.current)) {
          throw invalidElement(element.name, "current must be finite");
        }
        break;
      case "vccs":
        stampVccs(element, nodeIndices, matrix);
        break;
    }
  }

  return matrix;
}

function solveAcCircuit(circuit: Circuit, omega: number): AcSolution {
  const nodeIndices = collectNodeIndices(circuit);
  const voltageSources = collectAcVoltageSources(circuit);
  const nodeCount = nodeIndices.size;
  const branchCount = voltageSources.size;
  const matrixSize = nodeCount + branchCount;

  if (matrixSize === 0) {
    return { nodeVoltages: new Map(), branchCurrents: new Map() };
  }

  const matrix = Array.from({ length: matrixSize }, () =>
    Array.from({ length: matrixSize }, () => complex(0.0, 0.0)),
  );
  const rhs = Array.from({ length: matrixSize }, () => complex(0.0, 0.0));

  for (const element of circuit.elements()) {
    switch (element.kind) {
      case "resistor":
        stampAcResistor(element, nodeIndices, matrix);
        break;
      case "capacitor":
        stampAcCapacitor(element, omega, nodeIndices, matrix);
        break;
      case "inductor":
        stampAcInductor(element, omega, nodeIndices, matrix);
        break;
      case "voltage-source":
        stampAcVoltageSource(
          element,
          nodeIndices,
          voltageSources,
          nodeCount,
          matrix,
          rhs,
        );
        break;
      case "current-source":
        stampAcCurrentSource(element, nodeIndices, rhs);
        break;
      case "vccs":
        stampAcVccs(element, nodeIndices, matrix);
        break;
    }
  }

  const solution = solveComplexLinearSystem(matrix, rhs);
  const nodeVoltages = new Map<string, Complex>();
  const nodesByIndex = Array.from(nodeIndices.entries()).sort(
    ([, a], [, b]) => a - b,
  );
  for (const [node, index] of nodesByIndex) {
    nodeVoltages.set(node, solution[index]);
  }

  const branchCurrents = new Map<string, Complex>();
  for (const [sourceName, branchIndex] of voltageSources.entries()) {
    branchCurrents.set(`I(${sourceName})`, solution[nodeCount + branchIndex]);
  }
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

function makeTfResult(
  transferRatio: number,
  inputImpedanceOhms: number,
  outputImpedanceOhms: number,
): TfResult {
  return {
    transferRatio,
    inputImpedanceOhms,
    outputImpedanceOhms,
    gain(): number {
      return transferRatio;
    },
  };
}

function makeSensResult(
  outputNode: string,
  nominalVoltage: number,
  entries: readonly SensEntry[],
): SensResult {
  return {
    outputNode,
    nominalVoltage,
    entries,
    entry(elementName: string, parameter: string): SensEntry | undefined {
      return entries.find(
        (candidate) =>
          candidate.elementName === elementName &&
          candidate.parameter === parameter,
      );
    },
  };
}

function makeAcPoint(
  frequencyHz: number,
  nodeVoltages: ReadonlyMap<string, Complex>,
  branchCurrents: ReadonlyMap<string, Complex>,
): AcPoint {
  return {
    frequencyHz,
    nodeVoltages,
    branchCurrents,
    voltage(node: string): Complex | undefined {
      return isGround(node) ? complex(0.0, 0.0) : nodeVoltages.get(node);
    },
    branchCurrent(sourceName: string): Complex | undefined {
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
      case "vccs":
        insertNode(names, element.positive);
        insertNode(names, element.negative);
        insertNode(names, element.controlPositive);
        insertNode(names, element.controlNegative);
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

function collectAcVoltageSources(circuit: Circuit): Map<string, number> {
  const sources = new Map<string, number>();
  for (const element of circuit.elements()) {
    if (element.kind === "voltage-source") {
      insertBranchName(sources, element.name, "duplicate voltage source name");
    }
  }
  return sources;
}

function findInputSource(circuit: Circuit, inputSource: string): InputSource {
  for (const element of circuit.elements()) {
    if (
      (element.kind === "voltage-source" || element.kind === "current-source") &&
      element.name === inputSource
    ) {
      return element;
    }
    if (
      (element.kind === "resistor" ||
        element.kind === "capacitor" ||
        element.kind === "inductor" ||
        element.kind === "vccs") &&
      element.name === inputSource
    ) {
      throw invalidElement(
        inputSource,
        `input element must be an independent voltage or current source, got ${element.kind}`,
      );
    }
  }
  throw invalidElement(inputSource, "input source was not found");
}

function voltageSourceInputImpedance(
  source: VoltageSource,
  voltageSources: ReadonlyMap<string, number>,
  nodeCount: number,
  forward: readonly number[],
): number {
  const sourceIndex = voltageSources.get(source.name);
  if (sourceIndex === undefined) {
    throw invalidElement(source.name, "voltage source was not indexed");
  }
  const branchCurrent = forward[nodeCount + sourceIndex];
  return Math.abs(branchCurrent) > 1.0e-30
    ? -1.0 / branchCurrent
    : Number.POSITIVE_INFINITY;
}

function currentSourceInputImpedance(
  source: CurrentSource,
  nodeIndices: ReadonlyMap<string, number>,
  forward: readonly number[],
): number {
  const positive = nodeIndex(nodeIndices, source.positive);
  const negative = nodeIndex(nodeIndices, source.negative);
  const vPlus = positive === undefined ? 0.0 : forward[positive];
  const vMinus = negative === undefined ? 0.0 : forward[negative];
  return vMinus - vPlus;
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

function stampVccs(
  element: Vccs,
  nodeIndices: ReadonlyMap<string, number>,
  matrix: number[][],
): void {
  if (!Number.isFinite(element.transconductanceSiemens)) {
    throw invalidElement(element.name, "transconductance must be finite");
  }

  const positive = nodeIndex(nodeIndices, element.positive);
  const negative = nodeIndex(nodeIndices, element.negative);
  const controlPositive = nodeIndex(nodeIndices, element.controlPositive);
  const controlNegative = nodeIndex(nodeIndices, element.controlNegative);
  stampTransconductance(
    matrix,
    positive,
    negative,
    controlPositive,
    controlNegative,
    element.transconductanceSiemens,
  );
}

function stampTransconductance(
  matrix: number[][],
  positive: number | undefined,
  negative: number | undefined,
  controlPositive: number | undefined,
  controlNegative: number | undefined,
  transconductance: number,
): void {
  if (positive !== undefined && controlPositive !== undefined) {
    matrix[positive][controlPositive] += transconductance;
  }
  if (positive !== undefined && controlNegative !== undefined) {
    matrix[positive][controlNegative] -= transconductance;
  }
  if (negative !== undefined && controlPositive !== undefined) {
    matrix[negative][controlPositive] -= transconductance;
  }
  if (negative !== undefined && controlNegative !== undefined) {
    matrix[negative][controlNegative] += transconductance;
  }
}

function stampAcResistor(
  element: Resistor,
  nodeIndices: ReadonlyMap<string, number>,
  matrix: Complex[][],
): void {
  if (!Number.isFinite(element.resistanceOhms) || element.resistanceOhms <= 0) {
    throw invalidElement(element.name, "resistance must be finite and positive");
  }

  const conductance = complex(1.0 / element.resistanceOhms, 0.0);
  const n1 = nodeIndex(nodeIndices, element.n1);
  const n2 = nodeIndex(nodeIndices, element.n2);
  stampComplexConductance(matrix, n1, n2, conductance);
}

function stampAcCapacitor(
  element: Capacitor,
  omega: number,
  nodeIndices: ReadonlyMap<string, number>,
  matrix: Complex[][],
): void {
  validateCapacitor(element);
  const admittance = complex(0.0, omega * element.capacitanceFarads);
  const n1 = nodeIndex(nodeIndices, element.n1);
  const n2 = nodeIndex(nodeIndices, element.n2);
  stampComplexConductance(matrix, n1, n2, admittance);
}

function stampAcInductor(
  element: Inductor,
  omega: number,
  nodeIndices: ReadonlyMap<string, number>,
  matrix: Complex[][],
): void {
  validateInductor(element);
  const admittance = complex(0.0, -1.0 / (omega * element.inductanceHenrys));
  const n1 = nodeIndex(nodeIndices, element.n1);
  const n2 = nodeIndex(nodeIndices, element.n2);
  stampComplexConductance(matrix, n1, n2, admittance);
}

function stampComplexConductance(
  matrix: Complex[][],
  n1: number | undefined,
  n2: number | undefined,
  conductance: Complex,
): void {
  if (n1 !== undefined) {
    matrix[n1][n1] = complexAdd(matrix[n1][n1], conductance);
  }
  if (n2 !== undefined) {
    matrix[n2][n2] = complexAdd(matrix[n2][n2], conductance);
  }
  if (n1 !== undefined && n2 !== undefined) {
    matrix[n1][n2] = complexSub(matrix[n1][n2], conductance);
    matrix[n2][n1] = complexSub(matrix[n2][n1], conductance);
  }
}

function stampAcVoltageSource(
  element: VoltageSource,
  nodeIndices: ReadonlyMap<string, number>,
  voltageSources: ReadonlyMap<string, number>,
  nodeCount: number,
  matrix: Complex[][],
  rhs: Complex[],
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

  stampComplexBranchMatrix(matrix, branch, positive, negative);
  rhs[branch] = complexAdd(rhs[branch], complex(element.voltage, 0.0));
}

function stampComplexBranchMatrix(
  matrix: Complex[][],
  branch: number,
  positive: number | undefined,
  negative: number | undefined,
): void {
  const one = complex(1.0, 0.0);
  if (positive !== undefined) {
    matrix[positive][branch] = complexAdd(matrix[positive][branch], one);
    matrix[branch][positive] = complexAdd(matrix[branch][positive], one);
  }
  if (negative !== undefined) {
    matrix[negative][branch] = complexSub(matrix[negative][branch], one);
    matrix[branch][negative] = complexSub(matrix[branch][negative], one);
  }
}

function stampAcCurrentSource(
  element: CurrentSource,
  nodeIndices: ReadonlyMap<string, number>,
  rhs: Complex[],
): void {
  if (!Number.isFinite(element.current)) {
    throw invalidElement(element.name, "current must be finite");
  }

  const current = complex(element.current, 0.0);
  const positive = nodeIndex(nodeIndices, element.positive);
  const negative = nodeIndex(nodeIndices, element.negative);
  if (positive !== undefined) {
    rhs[positive] = complexSub(rhs[positive], current);
  }
  if (negative !== undefined) {
    rhs[negative] = complexAdd(rhs[negative], current);
  }
}

function stampAcVccs(
  element: Vccs,
  nodeIndices: ReadonlyMap<string, number>,
  matrix: Complex[][],
): void {
  if (!Number.isFinite(element.transconductanceSiemens)) {
    throw invalidElement(element.name, "transconductance must be finite");
  }

  const positive = nodeIndex(nodeIndices, element.positive);
  const negative = nodeIndex(nodeIndices, element.negative);
  const controlPositive = nodeIndex(nodeIndices, element.controlPositive);
  const controlNegative = nodeIndex(nodeIndices, element.controlNegative);
  stampComplexTransconductance(
    matrix,
    positive,
    negative,
    controlPositive,
    controlNegative,
    complex(element.transconductanceSiemens, 0.0),
  );
}

function stampComplexTransconductance(
  matrix: Complex[][],
  positive: number | undefined,
  negative: number | undefined,
  controlPositive: number | undefined,
  controlNegative: number | undefined,
  transconductance: Complex,
): void {
  if (positive !== undefined && controlPositive !== undefined) {
    matrix[positive][controlPositive] = complexAdd(
      matrix[positive][controlPositive],
      transconductance,
    );
  }
  if (positive !== undefined && controlNegative !== undefined) {
    matrix[positive][controlNegative] = complexSub(
      matrix[positive][controlNegative],
      transconductance,
    );
  }
  if (negative !== undefined && controlPositive !== undefined) {
    matrix[negative][controlPositive] = complexSub(
      matrix[negative][controlPositive],
      transconductance,
    );
  }
  if (negative !== undefined && controlNegative !== undefined) {
    matrix[negative][controlNegative] = complexAdd(
      matrix[negative][controlNegative],
      transconductance,
    );
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

function cloneMatrix(matrix: readonly (readonly number[])[]): number[][] {
  return matrix.map((row) => [...row]);
}

function solveComplexLinearSystem(matrix: Complex[][], rhs: Complex[]): Complex[] {
  const n = rhs.length;
  for (let pivotCol = 0; pivotCol < n; pivotCol++) {
    let pivotRow = pivotCol;
    let pivotAbs = complexAbs(matrix[pivotCol][pivotCol]);
    for (let row = pivotCol + 1; row < n; row++) {
      const candidateAbs = complexAbs(matrix[row][pivotCol]);
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
      const factor = complexDiv(matrix[row][pivotCol], pivot);
      if (factor.real === 0.0 && factor.imag === 0.0) {
        continue;
      }
      matrix[row][pivotCol] = complex(0.0, 0.0);
      for (let col = pivotCol + 1; col < n; col++) {
        matrix[row][col] = complexSub(
          matrix[row][col],
          complexMul(factor, matrix[pivotCol][col]),
        );
      }
      rhs[row] = complexSub(rhs[row], complexMul(factor, rhs[pivotCol]));
    }
  }

  const solution = Array.from({ length: n }, () => complex(0.0, 0.0));
  for (let row = n - 1; row >= 0; row--) {
    let tailSum = complex(0.0, 0.0);
    for (let col = row + 1; col < n; col++) {
      tailSum = complexAdd(tailSum, complexMul(matrix[row][col], solution[col]));
    }
    solution[row] = complexDiv(complexSub(rhs[row], tailSum), matrix[row][row]);
    if (!Number.isFinite(solution[row].real) || !Number.isFinite(solution[row].imag)) {
      throw new SpiceError(
        "circuit matrix is singular",
        "SINGULAR_MATRIX",
      );
    }
  }
  return solution;
}

function complex(real: number, imag: number): Complex {
  return { real, imag };
}

function complexAdd(left: Complex, right: Complex): Complex {
  return complex(left.real + right.real, left.imag + right.imag);
}

function complexSub(left: Complex, right: Complex): Complex {
  return complex(left.real - right.real, left.imag - right.imag);
}

function complexMul(left: Complex, right: Complex): Complex {
  return complex(
    left.real * right.real - left.imag * right.imag,
    left.real * right.imag + left.imag * right.real,
  );
}

function complexDiv(left: Complex, right: Complex): Complex {
  const denominator = right.real * right.real + right.imag * right.imag;
  return complex(
    (left.real * right.real + left.imag * right.imag) / denominator,
    (left.imag * right.real - left.real * right.imag) / denominator,
  );
}

function invalidElement(name: string, reason: string): SpiceError {
  return new SpiceError(`invalid element ${name}: ${reason}`, "INVALID_ELEMENT", name);
}
