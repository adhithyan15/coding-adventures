const PIVOT_EPSILON = 1.0e-12;
const TWO_PI = Math.PI * 2.0;
const BOLTZMANN = 1.380_649e-23;

export type Element =
  | Resistor
  | Capacitor
  | Inductor
  | VoltageSource
  | CurrentSource
  | BSource
  | Diode
  | Bjt
  | Mosfet
  | Vccs
  | Vcvs
  | Cccs
  | Ccvs;

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

export type Waveform =
  | PwlWaveform
  | SinWaveform
  | PulseWaveform
  | ExpWaveform;

export interface AcSource {
  readonly magnitude: number;
  readonly phaseDegrees: number;
}

export class PwlWaveform {
  constructor(readonly points: readonly (readonly [number, number])[]) {}

  valueAt(timeSeconds: number): number {
    if (this.points.length === 0) {
      return Number.NaN;
    }
    if (timeSeconds <= this.points[0][0]) {
      return this.points[0][1];
    }
    const last = this.points[this.points.length - 1];
    if (timeSeconds >= last[0]) {
      return last[1];
    }

    for (let index = 0; index < this.points.length - 1; index++) {
      const [leftTime, leftValue] = this.points[index];
      const [rightTime, rightValue] = this.points[index + 1];
      if (timeSeconds <= rightTime) {
        const phase = (timeSeconds - leftTime) / (rightTime - leftTime);
        return leftValue + (rightValue - leftValue) * phase;
      }
    }
    return last[1];
  }

  validate(): string | undefined {
    if (this.points.length < 2) {
      return "PWL waveform requires at least two points";
    }
    let previousTime = Number.NEGATIVE_INFINITY;
    for (const [time, value] of this.points) {
      if (!Number.isFinite(time) || !Number.isFinite(value)) {
        return "PWL waveform times and values must be finite";
      }
      if (time <= previousTime) {
        return "PWL waveform times must be strictly increasing";
      }
      previousTime = time;
    }
    return undefined;
  }
}

export class SinWaveform {
  constructor(
    readonly offset = 0.0,
    readonly amplitude = 1.0,
    readonly frequencyHz = 1.0,
    readonly delaySeconds = 0.0,
    readonly damping = 0.0,
  ) {}

  valueAt(timeSeconds: number): number {
    if (timeSeconds < this.delaySeconds) {
      return this.offset;
    }
    const shiftedTime = timeSeconds - this.delaySeconds;
    const envelope =
      this.damping === 0.0 ? 1.0 : Math.exp(-this.damping * shiftedTime);
    return (
      this.offset +
      this.amplitude *
        Math.sin(TWO_PI * this.frequencyHz * shiftedTime) *
        envelope
    );
  }

  validate(): string | undefined {
    if (
      !Number.isFinite(this.offset) ||
      !Number.isFinite(this.amplitude) ||
      !Number.isFinite(this.frequencyHz) ||
      !Number.isFinite(this.delaySeconds) ||
      !Number.isFinite(this.damping)
    ) {
      return "SIN waveform parameters must be finite";
    }
    if (this.frequencyHz < 0.0) {
      return "SIN waveform frequency must be non-negative";
    }
    if (this.delaySeconds < 0.0) {
      return "SIN waveform delay must be non-negative";
    }
    return undefined;
  }
}

export class PulseWaveform {
  constructor(
    readonly initialValue = 0.0,
    readonly pulsedValue = 1.0,
    readonly delaySeconds = 0.0,
    readonly riseTimeSeconds = 0.0,
    readonly fallTimeSeconds = 0.0,
    readonly pulseWidthSeconds = 0.5,
    readonly periodSeconds = 1.0,
  ) {}

  valueAt(timeSeconds: number): number {
    if (timeSeconds < this.delaySeconds) {
      return this.initialValue;
    }
    const elapsed =
      (timeSeconds - this.delaySeconds) % this.periodSeconds;
    if (this.riseTimeSeconds > 0.0 && elapsed < this.riseTimeSeconds) {
      const phase = elapsed / this.riseTimeSeconds;
      return this.initialValue + (this.pulsedValue - this.initialValue) * phase;
    }
    if (elapsed < this.riseTimeSeconds + this.pulseWidthSeconds) {
      return this.pulsedValue;
    }
    const fallStart = this.riseTimeSeconds + this.pulseWidthSeconds;
    if (this.fallTimeSeconds > 0.0 && elapsed < fallStart + this.fallTimeSeconds) {
      const phase = (elapsed - fallStart) / this.fallTimeSeconds;
      return this.pulsedValue + (this.initialValue - this.pulsedValue) * phase;
    }
    return this.initialValue;
  }

  validate(): string | undefined {
    if (
      !Number.isFinite(this.initialValue) ||
      !Number.isFinite(this.pulsedValue) ||
      !Number.isFinite(this.delaySeconds) ||
      !Number.isFinite(this.riseTimeSeconds) ||
      !Number.isFinite(this.fallTimeSeconds) ||
      !Number.isFinite(this.pulseWidthSeconds) ||
      !Number.isFinite(this.periodSeconds)
    ) {
      return "PULSE waveform parameters must be finite";
    }
    if (
      this.delaySeconds < 0.0 ||
      this.riseTimeSeconds < 0.0 ||
      this.fallTimeSeconds < 0.0 ||
      this.pulseWidthSeconds < 0.0 ||
      this.periodSeconds <= 0.0
    ) {
      return "PULSE waveform timing values must be non-negative and period positive";
    }
    if (
      this.riseTimeSeconds + this.pulseWidthSeconds + this.fallTimeSeconds >
      this.periodSeconds
    ) {
      return "PULSE waveform high interval must fit within the period";
    }
    return undefined;
  }
}

export class ExpWaveform {
  constructor(
    readonly initialValue = 0.0,
    readonly pulsedValue = 1.0,
    readonly riseDelaySeconds = 0.0,
    readonly riseTimeConstantSeconds = 1.0,
    readonly fallDelaySeconds = 1.0,
    readonly fallTimeConstantSeconds = 1.0,
  ) {}

  valueAt(timeSeconds: number): number {
    if (timeSeconds <= this.riseDelaySeconds) {
      return this.initialValue;
    }
    let value =
      this.initialValue +
      (this.pulsedValue - this.initialValue) *
        (1.0 -
          Math.exp(
            -(timeSeconds - this.riseDelaySeconds) /
              this.riseTimeConstantSeconds,
          ));
    if (timeSeconds >= this.fallDelaySeconds) {
      value +=
        (this.initialValue - this.pulsedValue) *
        (1.0 -
          Math.exp(
            -(timeSeconds - this.fallDelaySeconds) /
              this.fallTimeConstantSeconds,
          ));
    }
    return value;
  }

  validate(): string | undefined {
    if (
      !Number.isFinite(this.initialValue) ||
      !Number.isFinite(this.pulsedValue) ||
      !Number.isFinite(this.riseDelaySeconds) ||
      !Number.isFinite(this.riseTimeConstantSeconds) ||
      !Number.isFinite(this.fallDelaySeconds) ||
      !Number.isFinite(this.fallTimeConstantSeconds)
    ) {
      return "EXP waveform parameters must be finite";
    }
    if (
      this.riseDelaySeconds < 0.0 ||
      this.fallDelaySeconds < 0.0 ||
      this.riseTimeConstantSeconds <= 0.0 ||
      this.fallTimeConstantSeconds <= 0.0
    ) {
      return "EXP waveform delays must be non-negative and time constants positive";
    }
    return undefined;
  }
}

export interface VoltageSource {
  readonly kind: "voltage-source";
  readonly name: string;
  readonly positive: string;
  readonly negative: string;
  readonly voltage: number;
  readonly ac?: AcSource;
  readonly waveform?: Waveform;
}

export interface CurrentSource {
  readonly kind: "current-source";
  readonly name: string;
  readonly positive: string;
  readonly negative: string;
  readonly current: number;
  readonly ac?: AcSource;
  readonly waveform?: Waveform;
}

export interface BSource {
  readonly kind: "b-source";
  readonly name: string;
  readonly positive: string;
  readonly negative: string;
  readonly voltageExpr?: string;
  readonly currentExpr?: string;
}

export interface Diode {
  readonly kind: "diode";
  readonly name: string;
  readonly anode: string;
  readonly cathode: string;
  readonly saturationCurrent: number;
  readonly thermalVoltage: number;
}

export type BjtPolarity = "NPN" | "PNP";

export interface Bjt {
  readonly kind: "bjt";
  readonly name: string;
  readonly collector: string;
  readonly base: string;
  readonly emitter: string;
  readonly polarity: BjtPolarity;
  readonly saturationCurrent: number;
  readonly forwardBeta: number;
  readonly thermalVoltage: number;
}

export type MosfetType = "NMOS" | "PMOS";

export interface MosfetLevel1Params {
  readonly VT0: number;
  readonly KP: number;
  readonly LAMBDA: number;
  readonly GAMMA: number;
  readonly PHI: number;
  readonly W: number;
  readonly L: number;
  readonly IS: number;
  readonly N_SUB: number;
  readonly T_NOM: number;
}

export interface Mosfet {
  readonly kind: "mosfet";
  readonly name: string;
  readonly drain: string;
  readonly gate: string;
  readonly source: string;
  readonly body: string;
  readonly type: MosfetType;
  readonly model: "level1";
  readonly params: MosfetLevel1Params;
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

export interface Vcvs {
  readonly kind: "vcvs";
  readonly name: string;
  readonly positive: string;
  readonly negative: string;
  readonly controlPositive: string;
  readonly controlNegative: string;
  readonly gain: number;
}

export interface Cccs {
  readonly kind: "cccs";
  readonly name: string;
  readonly positive: string;
  readonly negative: string;
  readonly controlSource: string;
  readonly gain: number;
}

export interface Ccvs {
  readonly kind: "ccvs";
  readonly name: string;
  readonly positive: string;
  readonly negative: string;
  readonly controlSource: string;
  readonly transresistanceOhms: number;
}

export interface DcResult {
  readonly nodeVoltages: ReadonlyMap<string, number>;
  readonly branchCurrents: ReadonlyMap<string, number>;
  readonly iterations: number;
  readonly converged: boolean;
  voltage(node: string): number | undefined;
  branchCurrent(sourceName: string): number | undefined;
}

export interface DcOpOptions {
  readonly maxIterations?: number;
  readonly tolerance?: number;
  readonly convergenceAids?: boolean;
}

export interface DcSweepPoint {
  readonly value: number;
  readonly result: DcResult;
}

export type McDistribution = "gaussian" | "uniform";

export interface McOptions {
  readonly tolerance?: number;
  readonly distribution?: McDistribution;
  readonly seed?: number;
}

export interface McPoint {
  readonly trial: number;
  readonly nodeVoltages: ReadonlyMap<string, number>;
  readonly branchCurrents: ReadonlyMap<string, number>;
  readonly converged: boolean;
  voltage(node: string): number | undefined;
  branchCurrent(sourceName: string): number | undefined;
}

export interface McResult {
  readonly outputNode: string;
  readonly points: readonly McPoint[];
  readonly nTrials: number;
  readonly mean: number;
  readonly stdDev: number;
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

export type NoiseType = "thermal";

export interface NoiseEntry {
  readonly elementName: string;
  readonly noiseType: NoiseType;
  readonly sourcePsd: number;
  readonly outputPsd: number;
}

export interface NoisePoint {
  readonly frequencyHz: number;
  readonly outputPsd: number;
  readonly inputReferredPsd: number;
  readonly entries: readonly NoiseEntry[];
}

export interface NoiseResult {
  readonly outputNode: string;
  readonly inputSource: string;
  readonly temperatureKelvin: number;
  readonly points: readonly NoisePoint[];
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

export function acSource(magnitude: number, phaseDegrees = 0.0): AcSource {
  return { magnitude, phaseDegrees };
}

export function voltageSourceWithAc(
  name: string,
  positive: string,
  negative: string,
  voltage: number,
  magnitude: number,
  phaseDegrees = 0.0,
): VoltageSource {
  return {
    kind: "voltage-source",
    name,
    positive,
    negative,
    voltage,
    ac: acSource(magnitude, phaseDegrees),
  };
}

export function voltageSourceWithWaveform(
  name: string,
  positive: string,
  negative: string,
  voltage: number,
  waveform: Waveform,
): VoltageSource {
  return {
    kind: "voltage-source",
    name,
    positive,
    negative,
    voltage,
    waveform,
  };
}

export function currentSource(
  name: string,
  positive: string,
  negative: string,
  current: number,
): CurrentSource {
  return { kind: "current-source", name, positive, negative, current };
}

export function currentSourceWithAc(
  name: string,
  positive: string,
  negative: string,
  current: number,
  magnitude: number,
  phaseDegrees = 0.0,
): CurrentSource {
  return {
    kind: "current-source",
    name,
    positive,
    negative,
    current,
    ac: acSource(magnitude, phaseDegrees),
  };
}

export function currentSourceWithWaveform(
  name: string,
  positive: string,
  negative: string,
  current: number,
  waveform: Waveform,
): CurrentSource {
  return {
    kind: "current-source",
    name,
    positive,
    negative,
    current,
    waveform,
  };
}

export function bSourceCurrent(
  name: string,
  positive: string,
  negative: string,
  currentExpr: string,
): BSource {
  return { kind: "b-source", name, positive, negative, currentExpr };
}

export function bSourceVoltage(
  name: string,
  positive: string,
  negative: string,
  voltageExpr: string,
): BSource {
  return { kind: "b-source", name, positive, negative, voltageExpr };
}

export function diode(
  name: string,
  anode: string,
  cathode: string,
  saturationCurrent = 1.0e-15,
  thermalVoltage = 0.02585,
): Diode {
  return {
    kind: "diode",
    name,
    anode,
    cathode,
    saturationCurrent,
    thermalVoltage,
  };
}

export function bjt(
  name: string,
  collector: string,
  base: string,
  emitter: string,
  polarity: BjtPolarity = "NPN",
  saturationCurrent = 1.0e-14,
  forwardBeta = 100.0,
  thermalVoltage = 0.02585,
): Bjt {
  return {
    kind: "bjt",
    name,
    collector,
    base,
    emitter,
    polarity,
    saturationCurrent,
    forwardBeta,
    thermalVoltage,
  };
}

export function defaultMosfetLevel1Params(): MosfetLevel1Params {
  return {
    VT0: 0.42,
    KP: 220.0e-6,
    LAMBDA: 0.05,
    GAMMA: 0.27,
    PHI: 0.84,
    W: 1.0e-6,
    L: 130.0e-9,
    IS: 1.0e-15,
    N_SUB: 1.4,
    T_NOM: 300.15,
  };
}

export function mosfet(
  name: string,
  drain: string,
  gate: string,
  source: string,
  body: string,
  type: MosfetType = "NMOS",
  params: Partial<MosfetLevel1Params> = {},
): Mosfet {
  return {
    kind: "mosfet",
    name,
    drain,
    gate,
    source,
    body,
    type,
    model: "level1",
    params: { ...defaultMosfetLevel1Params(), ...params },
  };
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

export function vcvs(
  name: string,
  positive: string,
  negative: string,
  controlPositive: string,
  controlNegative: string,
  gain: number,
): Vcvs {
  return {
    kind: "vcvs",
    name,
    positive,
    negative,
    controlPositive,
    controlNegative,
    gain,
  };
}

export function cccs(
  name: string,
  positive: string,
  negative: string,
  controlSource: string,
  gain: number,
): Cccs {
  return {
    kind: "cccs",
    name,
    positive,
    negative,
    controlSource,
    gain,
  };
}

export function ccvs(
  name: string,
  positive: string,
  negative: string,
  controlSource: string,
  transresistanceOhms: number,
): Ccvs {
  return {
    kind: "ccvs",
    name,
    positive,
    negative,
    controlSource,
    transresistanceOhms,
  };
}

export function complexAbs(value: Complex): number {
  return Math.hypot(value.real, value.imag);
}

export function complexPhase(value: Complex): number {
  return Math.atan2(value.imag, value.real);
}

export function dcOp(
  circuit: Circuit,
  options: DcOpOptions = {},
): DcResult {
  const solveOptions = validatedDcOpOptions(options);
  const solution = solveDcNewton(circuit, solveOptions);
  if (solution.converged || !solveOptions.convergenceAids) {
    return makeDcResult(
      solution.nodeVoltages,
      solution.branchCurrents,
      solution.iterations,
      solution.converged,
    );
  }

  const aided =
    solveDcWithGminStepping(circuit, solveOptions, solution.vector) ??
    solveDcWithSourceStepping(circuit, solveOptions);
  const finalSolution = aided ?? solution;
  return makeDcResult(
    finalSolution.nodeVoltages,
    finalSolution.branchCurrents,
    finalSolution.iterations,
    finalSolution.converged,
  );
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

export function mcDc(
  circuit: Circuit,
  outputNode: string,
  nTrials = 100,
  options: McOptions = {},
): McResult {
  const nodeIndices = collectNodeIndices(circuit);
  if (!isGround(outputNode) && !nodeIndices.has(outputNode)) {
    throw invalidElement(outputNode, "output node was not found in circuit");
  }
  if (!Number.isInteger(nTrials) || nTrials < 1) {
    throw invalidElement("mcDc", "nTrials must be a positive integer");
  }

  const tolerance = options.tolerance ?? 0.05;
  const distribution = options.distribution ?? "gaussian";
  if (!Number.isFinite(tolerance) || tolerance < 0.0) {
    throw invalidElement("mcDc", "tolerance must be finite and non-negative");
  }
  if (distribution !== "gaussian" && distribution !== "uniform") {
    throw invalidElement(
      "mcDc",
      "distribution must be 'gaussian' or 'uniform'",
    );
  }

  const rng = seededRandom(options.seed);
  const points: McPoint[] = [];

  for (let trial = 0; trial < nTrials; trial++) {
    const trialCircuit = circuitWithRandomizedElements(
      circuit,
      tolerance,
      distribution,
      rng,
    );

    try {
      const result = dcOp(trialCircuit);
      points.push(
        makeMcPoint(
          trial,
          result.nodeVoltages,
          result.branchCurrents,
          true,
        ),
      );
    } catch (error) {
      if (error instanceof SpiceError && error.code === "SINGULAR_MATRIX") {
        points.push(makeMcPoint(trial, new Map(), new Map(), false));
        continue;
      }
      throw error;
    }
  }

  const convergedVoltages = points
    .filter((point) => point.converged)
    .map((point) => point.voltage(outputNode) ?? 0.0);

  return makeMcResult(
    outputNode,
    points,
    nTrials,
    sampleMean(convergedVoltages),
    sampleStdDev(convergedVoltages),
  );
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

export function noiseAc(
  circuit: Circuit,
  outputNode: string,
  inputSource: string,
  frequenciesHz: readonly number[] = defaultNoiseFrequencies(),
  temperatureKelvin = 300.0,
): NoiseResult {
  if (!Number.isFinite(temperatureKelvin) || temperatureKelvin <= 0.0) {
    throw invalidElement("noiseAc", "temperature must be finite and positive");
  }
  for (const frequency of frequenciesHz) {
    if (!Number.isFinite(frequency) || frequency <= 0.0) {
      throw invalidElement(
        "noiseAc",
        "frequencies must be finite and positive",
      );
    }
  }

  validateReactiveElements(circuit);

  const nodeIndices = collectNodeIndices(circuit);
  if (!isGround(outputNode) && !nodeIndices.has(outputNode)) {
    throw invalidElement(outputNode, "output node was not found in circuit");
  }

  const input = findInputSource(circuit, inputSource);
  const voltageSources = collectAcVoltageSources(circuit);
  const nodeCount = nodeIndices.size;
  const matrixSize = nodeCount + voltageSources.size;
  const outputIndex = nodeIndex(nodeIndices, outputNode);
  const noiseSources = collectNoiseSources(
    circuit,
    nodeIndices,
    temperatureKelvin,
  );

  const points = frequenciesHz.map((frequencyHz) => {
    if (outputIndex === undefined || matrixSize === 0) {
      return makeNoisePoint(frequencyHz, 0.0, 0.0, zeroNoiseEntries(noiseSources));
    }

    const matrix = buildAcMatrix(
      circuit,
      TWO_PI * frequencyHz,
      nodeIndices,
      voltageSources,
    );
    const rhs = Array.from({ length: matrixSize }, () => complex(0.0, 0.0));
    rhs[outputIndex] = complex(1.0, 0.0);

    let adjoint: Complex[];
    try {
      adjoint = solveComplexLinearSystem(transposeComplexMatrix(matrix), rhs);
    } catch (error) {
      if (error instanceof SpiceError && error.code === "SINGULAR_MATRIX") {
        return makeNoisePoint(
          frequencyHz,
          0.0,
          0.0,
          zeroNoiseEntries(noiseSources),
        );
      }
      throw error;
    }

    const entries = noiseSources.map((source) => {
      const hPositive =
        source.positive === undefined ? complex(0.0, 0.0) : adjoint[source.positive];
      const hNegative =
        source.negative === undefined ? complex(0.0, 0.0) : adjoint[source.negative];
      const transfer = complexSub(hPositive, hNegative);
      return {
        elementName: source.elementName,
        noiseType: source.noiseType,
        sourcePsd: source.sourcePsd,
        outputPsd: complexAbs(transfer) ** 2 * source.sourcePsd,
      };
    });
    entries.sort(
      (left, right) =>
        right.outputPsd - left.outputPsd ||
        left.elementName.localeCompare(right.elementName),
    );

    const outputPsd = entries.reduce((sum, entry) => sum + entry.outputPsd, 0.0);
    const inputGain = adjointInputGain(
      input,
      adjoint,
      nodeIndices,
      voltageSources,
      nodeCount,
    );
    const gainSquared = complexAbs(inputGain) ** 2;
    const inputReferredPsd =
      gainSquared > 1.0e-100 ? outputPsd / gainSquared : 0.0;

    return makeNoisePoint(frequencyHz, outputPsd, inputReferredPsd, entries);
  });

  return {
    outputNode,
    inputSource,
    temperatureKelvin,
    points,
  };
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
    const solution = solveLinearCircuit(
      circuit,
      capacitorStates,
      inductorStates,
      time,
    );
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
    case "diode":
      return {
        elementName: element.name,
        parameter: "saturationCurrent",
        nominalValue: element.saturationCurrent,
      };
    case "bjt":
      return {
        elementName: element.name,
        parameter: "saturationCurrent",
        nominalValue: element.saturationCurrent,
      };
    case "mosfet":
      return {
        elementName: element.name,
        parameter: "KP",
        nominalValue: element.params.KP,
      };
    case "vccs":
      return {
        elementName: element.name,
        parameter: "transconductanceSiemens",
        nominalValue: element.transconductanceSiemens,
      };
    case "vcvs":
      return {
        elementName: element.name,
        parameter: "gain",
        nominalValue: element.gain,
      };
    case "cccs":
      return {
        elementName: element.name,
        parameter: "gain",
        nominalValue: element.gain,
      };
    case "ccvs":
      return {
        elementName: element.name,
        parameter: "transresistanceOhms",
        nominalValue: element.transresistanceOhms,
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
      case "diode":
        perturbed.add({
          ...element,
          saturationCurrent: element.saturationCurrent + delta,
        });
        break;
      case "bjt":
        perturbed.add({
          ...element,
          saturationCurrent: element.saturationCurrent + delta,
        });
        break;
      case "mosfet":
        perturbed.add({
          ...element,
          params: { ...element.params, KP: element.params.KP + delta },
        });
        break;
      case "vccs":
        perturbed.add({
          ...element,
          transconductanceSiemens: element.transconductanceSiemens + delta,
        });
        break;
      case "vcvs":
        perturbed.add({ ...element, gain: element.gain + delta });
        break;
      case "cccs":
        perturbed.add({ ...element, gain: element.gain + delta });
        break;
      case "ccvs":
        perturbed.add({
          ...element,
          transresistanceOhms: element.transresistanceOhms + delta,
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

type RandomSource = () => number;

function seededRandom(seed: number | undefined): RandomSource {
  if (seed === undefined) {
    return Math.random;
  }
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let value = state;
    value = Math.imul(value ^ (value >>> 15), value | 1);
    value ^= value + Math.imul(value ^ (value >>> 7), value | 61);
    return ((value ^ (value >>> 14)) >>> 0) / 4_294_967_296;
  };
}

function randomizedValue(
  nominalValue: number,
  tolerance: number,
  distribution: McDistribution,
  rng: RandomSource,
): number {
  if (tolerance === 0.0) {
    return nominalValue;
  }
  if (distribution === "uniform") {
    return nominalValue * (1.0 + tolerance * (2.0 * rng() - 1.0));
  }
  return nominalValue * (1.0 + gaussian(rng) * tolerance / 3.0);
}

function gaussian(rng: RandomSource): number {
  const u1 = Math.max(rng(), Number.MIN_VALUE);
  const u2 = rng();
  return Math.sqrt(-2.0 * Math.log(u1)) * Math.cos(TWO_PI * u2);
}

function circuitWithRandomizedElements(
  circuit: Circuit,
  tolerance: number,
  distribution: McDistribution,
  rng: RandomSource,
): Circuit {
  const randomized = new Circuit();
  for (const element of circuit.elements()) {
    randomized.add(randomizedElement(element, tolerance, distribution, rng));
  }
  return randomized;
}

function randomizedElement(
  element: Element,
  tolerance: number,
  distribution: McDistribution,
  rng: RandomSource,
): Element {
  switch (element.kind) {
    case "resistor":
      return {
        ...element,
        resistanceOhms: randomizedValue(
          element.resistanceOhms,
          tolerance,
          distribution,
          rng,
        ),
      };
    case "voltage-source":
      return {
        ...element,
        voltage: randomizedValue(element.voltage, tolerance, distribution, rng),
      };
    case "current-source":
      return {
        ...element,
        current: randomizedValue(element.current, tolerance, distribution, rng),
      };
    case "diode":
      return {
        ...element,
        saturationCurrent: randomizedValue(
          element.saturationCurrent,
          tolerance,
          distribution,
          rng,
        ),
      };
    case "bjt":
      return {
        ...element,
        saturationCurrent: randomizedValue(
          element.saturationCurrent,
          tolerance,
          distribution,
          rng,
        ),
      };
    case "mosfet":
      return {
        ...element,
        params: {
          ...element.params,
          KP: randomizedValue(element.params.KP, tolerance, distribution, rng),
        },
      };
    case "vccs":
      return {
        ...element,
        transconductanceSiemens: randomizedValue(
          element.transconductanceSiemens,
          tolerance,
          distribution,
          rng,
        ),
      };
    case "vcvs":
      return {
        ...element,
        gain: randomizedValue(element.gain, tolerance, distribution, rng),
      };
    case "cccs":
      return {
        ...element,
        gain: randomizedValue(element.gain, tolerance, distribution, rng),
      };
    case "ccvs":
      return {
        ...element,
        transresistanceOhms: randomizedValue(
          element.transresistanceOhms,
          tolerance,
          distribution,
          rng,
        ),
      };
    case "b-source":
      return element;
    case "capacitor":
    case "inductor":
      return element;
  }
}

function sampleMean(values: readonly number[]): number {
  if (values.length === 0) {
    return 0.0;
  }
  return values.reduce((sum, value) => sum + value, 0.0) / values.length;
}

function sampleStdDev(values: readonly number[]): number {
  if (values.length < 2) {
    return 0.0;
  }
  const mean = sampleMean(values);
  const variance =
    values.reduce((sum, value) => sum + (value - mean) ** 2, 0.0) /
    (values.length - 1);
  return Math.sqrt(variance);
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
  readonly vector: readonly number[];
  readonly iterations: number;
  readonly converged: boolean;
}

interface LinearSolveOptions {
  readonly maxIterations: number;
  readonly tolerance: number;
  readonly initialVector?: readonly number[];
  readonly returnSingularAsUnconverged?: boolean;
}

interface ResolvedDcOpOptions {
  readonly maxIterations: number;
  readonly tolerance: number;
  readonly convergenceAids: boolean;
}

interface AcSolution {
  readonly nodeVoltages: ReadonlyMap<string, Complex>;
  readonly branchCurrents: ReadonlyMap<string, Complex>;
}

interface NoiseSource {
  readonly elementName: string;
  readonly noiseType: NoiseType;
  readonly positive: number | undefined;
  readonly negative: number | undefined;
  readonly sourcePsd: number;
}

type InputSource = VoltageSource | CurrentSource;

function solveLinearCircuit(
  circuit: Circuit,
  capacitorStates: readonly CapacitorState[],
  inductorStates: readonly InductorState[],
  sourceTime: number | undefined,
): LinearSolution {
  return solveLinearCircuitWithOptions(
    circuit,
    capacitorStates,
    inductorStates,
    sourceTime,
    {
      maxIterations: 80,
      tolerance: 1.0e-9,
    },
  );
}

function solveDcNewton(
  circuit: Circuit,
  options: ResolvedDcOpOptions,
  initialVector?: readonly number[],
): LinearSolution {
  return solveLinearCircuitWithOptions(
    circuit,
    [],
    [],
    undefined,
    {
      maxIterations: options.maxIterations,
      tolerance: options.tolerance,
      initialVector,
      returnSingularAsUnconverged: true,
    },
  );
}

function solveLinearCircuitWithOptions(
  circuit: Circuit,
  capacitorStates: readonly CapacitorState[],
  inductorStates: readonly InductorState[],
  sourceTime: number | undefined,
  options: LinearSolveOptions,
): LinearSolution {
  const nodeIndices = collectNodeIndices(circuit);
  const voltageSources = collectVoltageSources(circuit, inductorStates);
  const nodeCount = nodeIndices.size;
  const branchCount = voltageSources.size;
  const matrixSize = nodeCount + branchCount;

  if (matrixSize === 0) {
    return {
      nodeVoltages: new Map(),
      branchCurrents: new Map(),
      vector: [],
      iterations: 0,
      converged: true,
    };
  }

  const hasNonlinearElement = circuit
    .elements()
    .some(
      (element) =>
        element.kind === "diode" ||
        element.kind === "bjt" ||
        element.kind === "mosfet" ||
        element.kind === "b-source",
    );
  const returnSingularAsUnconverged =
    (options.returnSingularAsUnconverged ?? false) && hasNonlinearElement;
  let operatingPoint =
    options.initialVector?.length === matrixSize
      ? [...options.initialVector]
      : Array.from({ length: matrixSize }, () => 0.0);
  let solution = solveLinearCircuitAtOperatingPointOrFailure(
    circuit,
    capacitorStates,
    inductorStates,
    sourceTime,
    nodeIndices,
    voltageSources,
    nodeCount,
    matrixSize,
    operatingPoint,
    returnSingularAsUnconverged,
  );
  if (!hasNonlinearElement) {
    return { ...solution, iterations: 1, converged: solution.converged };
  }

  let iterations = 1;
  while (iterations < options.maxIterations) {
    if (!solution.converged) {
      return { ...solution, iterations, converged: false };
    }
    const delta = maxVectorDelta(solution.vector, operatingPoint);
    operatingPoint = [...solution.vector];
    if (delta < options.tolerance) {
      return { ...solution, iterations, converged: true };
    }
    solution = solveLinearCircuitAtOperatingPointOrFailure(
      circuit,
      capacitorStates,
      inductorStates,
      sourceTime,
      nodeIndices,
      voltageSources,
      nodeCount,
      matrixSize,
      operatingPoint,
      returnSingularAsUnconverged,
    );
    iterations += 1;
  }

  const delta = maxVectorDelta(solution.vector, operatingPoint);
  return { ...solution, iterations, converged: delta < options.tolerance };
}

function solveLinearCircuitAtOperatingPointOrFailure(
  circuit: Circuit,
  capacitorStates: readonly CapacitorState[],
  inductorStates: readonly InductorState[],
  sourceTime: number | undefined,
  nodeIndices: ReadonlyMap<string, number>,
  voltageSources: ReadonlyMap<string, number>,
  nodeCount: number,
  matrixSize: number,
  operatingPoint: readonly number[],
  returnSingularAsUnconverged: boolean,
): LinearSolution {
  try {
    const solution = solveLinearCircuitAtOperatingPoint(
      circuit,
      capacitorStates,
      inductorStates,
      sourceTime,
      nodeIndices,
      voltageSources,
      nodeCount,
      matrixSize,
      operatingPoint,
    );
    return { ...solution, iterations: 1, converged: true };
  } catch (error) {
    if (
      returnSingularAsUnconverged &&
      error instanceof SpiceError &&
      error.code === "SINGULAR_MATRIX"
    ) {
      return linearSolutionFromVector(
        circuit,
        inductorStates,
        nodeIndices,
        voltageSources,
        nodeCount,
        operatingPoint,
        false,
      );
    }
    throw error;
  }
}

function validatedDcOpOptions(options: DcOpOptions): ResolvedDcOpOptions {
  const maxIterations = options.maxIterations ?? 80;
  const tolerance = options.tolerance ?? 1.0e-9;
  if (!Number.isInteger(maxIterations) || maxIterations < 1) {
    throw invalidElement("dcOp", "maxIterations must be a positive integer");
  }
  if (!Number.isFinite(tolerance) || tolerance <= 0.0) {
    throw invalidElement("dcOp", "tolerance must be finite and positive");
  }
  return {
    maxIterations,
    tolerance,
    convergenceAids: options.convergenceAids ?? true,
  };
}

function solveDcWithGminStepping(
  circuit: Circuit,
  options: ResolvedDcOpOptions,
  initialVector: readonly number[],
): LinearSolution | undefined {
  let warmStart: readonly number[] | undefined = initialVector;
  let finalSolution: LinearSolution | undefined;

  for (const gmin of dcGminSequence()) {
    const steppedCircuit =
      gmin === 0.0 ? circuit : circuitWithGmin(circuit, gmin);
    const solution = solveDcNewton(steppedCircuit, options, warmStart);
    if (!solution.converged) {
      return undefined;
    }
    warmStart = solution.vector;
    finalSolution = solution;
  }

  return finalSolution;
}

function solveDcWithSourceStepping(
  circuit: Circuit,
  options: ResolvedDcOpOptions,
): LinearSolution | undefined {
  let warmStart: readonly number[] | undefined;
  let finalSolution: LinearSolution | undefined;

  for (let step = 0; step <= 10; step++) {
    const scale = step / 10.0;
    const steppedCircuit =
      scale === 1.0 ? circuit : circuitWithScaledIndependentSources(circuit, scale);
    const solution = solveDcNewton(steppedCircuit, options, warmStart);
    if (!solution.converged) {
      return undefined;
    }
    warmStart = solution.vector;
    finalSolution = solution;
  }

  return finalSolution;
}

function dcGminSequence(): number[] {
  const sequence: number[] = [];
  for (let exponent = -3; exponent >= -12; exponent--) {
    sequence.push(10.0 ** exponent);
  }
  sequence.push(0.0);
  return sequence;
}

function circuitWithGmin(circuit: Circuit, gminSiemens: number): Circuit {
  const aided = circuitFromElements(circuit.elements());
  for (const node of collectNodeIndices(circuit).keys()) {
    aided.add(resistor(`__gmin_${node}`, node, "0", 1.0 / gminSiemens));
  }
  return aided;
}

function circuitWithScaledIndependentSources(
  circuit: Circuit,
  scale: number,
): Circuit {
  const scaled = new Circuit();
  for (const element of circuit.elements()) {
    if (element.kind === "voltage-source") {
      scaled.add({ ...element, voltage: element.voltage * scale });
    } else if (element.kind === "current-source") {
      scaled.add({ ...element, current: element.current * scale });
    } else {
      scaled.add(element);
    }
  }
  return scaled;
}

function circuitFromElements(elements: readonly Element[]): Circuit {
  const circuit = new Circuit();
  for (const element of elements) {
    circuit.add(element);
  }
  return circuit;
}

function solveLinearCircuitAtOperatingPoint(
  circuit: Circuit,
  capacitorStates: readonly CapacitorState[],
  inductorStates: readonly InductorState[],
  sourceTime: number | undefined,
  nodeIndices: ReadonlyMap<string, number>,
  voltageSources: ReadonlyMap<string, number>,
  nodeCount: number,
  matrixSize: number,
  operatingPoint: readonly number[],
): LinearSolution {

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
          sourceTime,
        );
        break;
      case "current-source":
        stampCurrentSource(element, nodeIndices, rhs, sourceTime);
        break;
      case "b-source":
        stampBSource(
          element,
          nodeIndices,
          voltageSources,
          nodeCount,
          matrix,
          rhs,
          operatingPoint,
        );
        break;
      case "diode":
        stampDiode(element, nodeIndices, matrix, rhs, operatingPoint);
        break;
      case "bjt":
        stampBjt(element, nodeIndices, matrix, rhs, operatingPoint);
        break;
      case "mosfet":
        stampMosfet(element, nodeIndices, matrix, rhs, operatingPoint);
        break;
      case "vccs":
        stampVccs(element, nodeIndices, matrix);
        break;
      case "vcvs":
        stampVcvs(
          element,
          nodeIndices,
          voltageSources,
          nodeCount,
          matrix,
        );
        break;
      case "cccs":
        stampCccs(element, nodeIndices, voltageSources, matrix);
        break;
      case "ccvs":
        stampCcvs(
          element,
          nodeIndices,
          voltageSources,
          nodeCount,
          matrix,
        );
        break;
    }
  }

  const solution = solveLinearSystem(matrix, rhs);
  return linearSolutionFromVector(
    circuit,
    inductorStates,
    nodeIndices,
    voltageSources,
    nodeCount,
    solution,
    true,
  );
}

function linearSolutionFromVector(
  circuit: Circuit,
  inductorStates: readonly InductorState[],
  nodeIndices: ReadonlyMap<string, number>,
  voltageSources: ReadonlyMap<string, number>,
  nodeCount: number,
  solution: readonly number[],
  converged: boolean,
): LinearSolution {
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

  return {
    nodeVoltages,
    branchCurrents,
    vector: [...solution],
    iterations: 1,
    converged,
  };
}

function maxVectorDelta(left: readonly number[], right: readonly number[]): number {
  let max = 0.0;
  for (let index = 0; index < left.length; index++) {
    max = Math.max(max, Math.abs(left[index] - right[index]));
  }
  return max;
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
      case "diode":
        validateDiode(element);
        stampConductance(
          matrix,
          nodeIndex(nodeIndices, element.anode),
          nodeIndex(nodeIndices, element.cathode),
          element.saturationCurrent / element.thermalVoltage,
        );
        break;
      case "bjt":
        stampBjtSmallSignal(element, nodeIndices, matrix);
        break;
      case "mosfet":
        stampMosfetSmallSignal(element, nodeIndices, matrix);
        break;
      case "vccs":
        stampVccs(element, nodeIndices, matrix);
        break;
      case "vcvs":
        stampVcvs(
          element,
          nodeIndices,
          voltageSources,
          nodeCount,
          matrix,
        );
        break;
      case "cccs":
        stampCccs(element, nodeIndices, voltageSources, matrix);
        break;
      case "ccvs":
        stampCcvs(
          element,
          nodeIndices,
          voltageSources,
          nodeCount,
          matrix,
        );
        break;
    }
  }

  return matrix;
}

function solveAcCircuit(circuit: Circuit, omega: number): AcSolution {
  const nodeIndices = collectNodeIndices(circuit);
  const voltageSources = collectAcVoltageSources(circuit);
  const explicitAcSources = usesExplicitAcSources(circuit);
  const nodeCount = nodeIndices.size;
  const branchCount = voltageSources.size;
  const matrixSize = nodeCount + branchCount;

  if (matrixSize === 0) {
    return { nodeVoltages: new Map(), branchCurrents: new Map() };
  }

  const matrix = buildAcMatrix(circuit, omega, nodeIndices, voltageSources);
  const rhs = Array.from({ length: matrixSize }, () => complex(0.0, 0.0));

  for (const element of circuit.elements()) {
    switch (element.kind) {
      case "voltage-source":
        stampAcVoltageSourceRhs(
          element,
          voltageSources,
          nodeCount,
          explicitAcSources,
          rhs,
        );
        break;
      case "current-source":
        stampAcCurrentSource(element, nodeIndices, explicitAcSources, rhs);
        break;
      case "resistor":
      case "capacitor":
      case "inductor":
      case "diode":
      case "bjt":
      case "mosfet":
      case "vccs":
      case "vcvs":
      case "cccs":
      case "ccvs":
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

function buildAcMatrix(
  circuit: Circuit,
  omega: number,
  nodeIndices: ReadonlyMap<string, number>,
  voltageSources: ReadonlyMap<string, number>,
): Complex[][] {
  const nodeCount = nodeIndices.size;
  const matrixSize = nodeCount + voltageSources.size;
  const matrix = Array.from({ length: matrixSize }, () =>
    Array.from({ length: matrixSize }, () => complex(0.0, 0.0)),
  );

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
        stampAcVoltageSourceMatrix(
          element,
          nodeIndices,
          voltageSources,
          nodeCount,
          matrix,
        );
        break;
      case "current-source":
        if (!Number.isFinite(element.current)) {
          throw invalidElement(element.name, "current must be finite");
        }
        break;
      case "diode":
        validateDiode(element);
        stampComplexConductance(
          matrix,
          nodeIndex(nodeIndices, element.anode),
          nodeIndex(nodeIndices, element.cathode),
          complex(element.saturationCurrent / element.thermalVoltage, 0.0),
        );
        break;
      case "bjt":
        stampAcBjtSmallSignal(element, nodeIndices, matrix);
        break;
      case "mosfet":
        stampAcMosfetSmallSignal(element, nodeIndices, matrix);
        break;
      case "vccs":
        stampAcVccs(element, nodeIndices, matrix);
        break;
      case "vcvs":
        stampAcVcvs(
          element,
          nodeIndices,
          voltageSources,
          nodeCount,
          matrix,
        );
        break;
      case "cccs":
        stampAcCccs(element, nodeIndices, voltageSources, matrix);
        break;
      case "ccvs":
        stampAcCcvs(
          element,
          nodeIndices,
          voltageSources,
          nodeCount,
          matrix,
        );
        break;
    }
  }

  return matrix;
}

function makeDcResult(
  nodeVoltages: ReadonlyMap<string, number>,
  branchCurrents: ReadonlyMap<string, number>,
  iterations = 1,
  converged = true,
): DcResult {
  return {
    nodeVoltages,
    branchCurrents,
    iterations,
    converged,
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

function makeMcPoint(
  trial: number,
  nodeVoltages: ReadonlyMap<string, number>,
  branchCurrents: ReadonlyMap<string, number>,
  converged: boolean,
): McPoint {
  return {
    trial,
    nodeVoltages,
    branchCurrents,
    converged,
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

function makeMcResult(
  outputNode: string,
  points: readonly McPoint[],
  nTrials: number,
  mean: number,
  stdDev: number,
): McResult {
  return {
    outputNode,
    points,
    nTrials,
    mean,
    stdDev,
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

function makeNoisePoint(
  frequencyHz: number,
  outputPsd: number,
  inputReferredPsd: number,
  entries: readonly NoiseEntry[],
): NoisePoint {
  return {
    frequencyHz,
    outputPsd,
    inputReferredPsd,
    entries,
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
      case "b-source":
        insertNode(names, element.positive);
        insertNode(names, element.negative);
        for (const node of bSourceExprNodes(
          element.voltageExpr ?? element.currentExpr ?? "",
        )) {
          insertNode(names, node);
        }
        break;
      case "diode":
        insertNode(names, element.anode);
        insertNode(names, element.cathode);
        break;
      case "bjt":
        insertNode(names, element.collector);
        insertNode(names, element.base);
        insertNode(names, element.emitter);
        break;
      case "mosfet":
        insertNode(names, element.drain);
        insertNode(names, element.gate);
        insertNode(names, element.source);
        insertNode(names, element.body);
        break;
      case "vccs":
        insertNode(names, element.positive);
        insertNode(names, element.negative);
        insertNode(names, element.controlPositive);
        insertNode(names, element.controlNegative);
        break;
      case "vcvs":
        insertNode(names, element.positive);
        insertNode(names, element.negative);
        insertNode(names, element.controlPositive);
        insertNode(names, element.controlNegative);
        break;
      case "cccs":
        insertNode(names, element.positive);
        insertNode(names, element.negative);
        break;
      case "ccvs":
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
    if (
      element.kind === "voltage-source" ||
      element.kind === "vcvs" ||
      element.kind === "ccvs" ||
      (element.kind === "b-source" && element.voltageExpr !== undefined)
    ) {
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
    if (
      element.kind === "voltage-source" ||
      element.kind === "vcvs" ||
      element.kind === "ccvs" ||
      (element.kind === "b-source" && element.voltageExpr !== undefined)
    ) {
      insertBranchName(sources, element.name, "duplicate voltage source name");
    }
  }
  return sources;
}

function usesExplicitAcSources(circuit: Circuit): boolean {
  return circuit.elements().some(
    (element) =>
      (element.kind === "voltage-source" || element.kind === "current-source") &&
      element.ac !== undefined,
  );
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
        element.kind === "diode" ||
        element.kind === "bjt" ||
        element.kind === "mosfet" ||
        element.kind === "vccs" ||
        element.kind === "vcvs" ||
        element.kind === "cccs" ||
        element.kind === "ccvs" ||
        element.kind === "b-source") &&
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

function collectNoiseSources(
  circuit: Circuit,
  nodeIndices: ReadonlyMap<string, number>,
  temperatureKelvin: number,
): NoiseSource[] {
  const sources: NoiseSource[] = [];
  for (const element of circuit.elements()) {
    if (element.kind !== "resistor") {
      continue;
    }
    if (!Number.isFinite(element.resistanceOhms) || element.resistanceOhms <= 0) {
      throw invalidElement(element.name, "resistance must be finite and positive");
    }
    sources.push({
      elementName: element.name,
      noiseType: "thermal",
      positive: nodeIndex(nodeIndices, element.n1),
      negative: nodeIndex(nodeIndices, element.n2),
      sourcePsd: 4.0 * BOLTZMANN * temperatureKelvin / element.resistanceOhms,
    });
  }
  return sources;
}

function zeroNoiseEntries(sources: readonly NoiseSource[]): NoiseEntry[] {
  return sources.map((source) => ({
    elementName: source.elementName,
    noiseType: source.noiseType,
    sourcePsd: source.sourcePsd,
    outputPsd: 0.0,
  }));
}

function defaultNoiseFrequencies(): number[] {
  const count = 50;
  return Array.from({ length: count }, (_unused, index) => {
    const exponent = 6.0 * index / (count - 1);
    return 10.0 ** exponent;
  });
}

function adjointInputGain(
  input: InputSource,
  adjoint: readonly Complex[],
  nodeIndices: ReadonlyMap<string, number>,
  voltageSources: ReadonlyMap<string, number>,
  nodeCount: number,
): Complex {
  if (input.kind === "voltage-source") {
    const sourceIndex = voltageSources.get(input.name);
    if (sourceIndex === undefined) {
      throw invalidElement(input.name, "voltage source was not indexed");
    }
    return adjoint[nodeCount + sourceIndex];
  }

  const positive = nodeIndex(nodeIndices, input.positive);
  const negative = nodeIndex(nodeIndices, input.negative);
  const hPositive = positive === undefined ? complex(0.0, 0.0) : adjoint[positive];
  const hNegative = negative === undefined ? complex(0.0, 0.0) : adjoint[negative];
  return complexSub(hNegative, hPositive);
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

function stampDiode(
  element: Diode,
  nodeIndices: ReadonlyMap<string, number>,
  matrix: number[][],
  rhs: number[],
  operatingPoint: readonly number[],
): void {
  validateDiode(element);
  const anode = nodeIndex(nodeIndices, element.anode);
  const cathode = nodeIndex(nodeIndices, element.cathode);
  const voltage =
    (anode === undefined ? 0.0 : operatingPoint[anode]) -
    (cathode === undefined ? 0.0 : operatingPoint[cathode]);
  const exponent = Math.max(-40.0, Math.min(40.0, voltage / element.thermalVoltage));
  const expValue = Math.exp(exponent);
  const current = element.saturationCurrent * (expValue - 1.0);
  const conductance = element.saturationCurrent / element.thermalVoltage * expValue;
  const equivalentCurrent = current - conductance * voltage;

  stampConductance(matrix, anode, cathode, conductance);
  if (anode !== undefined) {
    rhs[anode] -= equivalentCurrent;
  }
  if (cathode !== undefined) {
    rhs[cathode] += equivalentCurrent;
  }
}

function stampBjt(
  element: Bjt,
  nodeIndices: ReadonlyMap<string, number>,
  matrix: number[][],
  rhs: number[],
  operatingPoint: readonly number[],
): void {
  validateBjt(element);
  const collector = nodeIndex(nodeIndices, element.collector);
  const base = nodeIndex(nodeIndices, element.base);
  const emitter = nodeIndex(nodeIndices, element.emitter);
  const baseVoltage = base === undefined ? 0.0 : operatingPoint[base];
  const emitterVoltage = emitter === undefined ? 0.0 : operatingPoint[emitter];

  const junctionVoltage =
    element.polarity === "NPN"
      ? baseVoltage - emitterVoltage
      : emitterVoltage - baseVoltage;
  const exponent = Math.max(-40.0, Math.min(40.0, junctionVoltage / element.thermalVoltage));
  const expValue = Math.exp(exponent);
  const collectorCurrent = element.saturationCurrent * (expValue - 1.0);
  const transconductance = element.saturationCurrent / element.thermalVoltage * expValue;
  const junctionConductance = transconductance / element.forwardBeta;
  const baseCurrent = collectorCurrent / element.forwardBeta;
  const equivalentCollectorCurrent =
    collectorCurrent - transconductance * junctionVoltage;
  const equivalentBaseCurrent =
    baseCurrent - junctionConductance * junctionVoltage;

  if (element.polarity === "NPN") {
    stampConductance(matrix, base, emitter, junctionConductance);
    stampTransconductance(matrix, collector, emitter, base, emitter, transconductance);
    stampCurrentSourceEquivalent(rhs, base, emitter, equivalentBaseCurrent);
    stampCurrentSourceEquivalent(rhs, collector, emitter, equivalentCollectorCurrent);
  } else {
    stampConductance(matrix, emitter, base, junctionConductance);
    stampTransconductance(matrix, emitter, collector, emitter, base, transconductance);
    stampCurrentSourceEquivalent(rhs, emitter, base, equivalentBaseCurrent);
    stampCurrentSourceEquivalent(rhs, emitter, collector, equivalentCollectorCurrent);
  }
}

interface MosfetDcResult {
  readonly drainCurrent: number;
  readonly gm: number;
  readonly gds: number;
  readonly gmb: number;
}

function stampMosfet(
  element: Mosfet,
  nodeIndices: ReadonlyMap<string, number>,
  matrix: number[][],
  rhs: number[],
  operatingPoint: readonly number[],
): void {
  validateMosfet(element);
  const drain = nodeIndex(nodeIndices, element.drain);
  const gate = nodeIndex(nodeIndices, element.gate);
  const source = nodeIndex(nodeIndices, element.source);
  const body = nodeIndex(nodeIndices, element.body);
  const drainVoltage = vectorVoltage(operatingPoint, drain);
  const gateVoltage = vectorVoltage(operatingPoint, gate);
  const sourceVoltage = vectorVoltage(operatingPoint, source);
  const bodyVoltage = vectorVoltage(operatingPoint, body);
  const vgs = gateVoltage - sourceVoltage;
  const vds = drainVoltage - sourceVoltage;
  const vbs = bodyVoltage - sourceVoltage;
  const result = evaluateMosfetLevel1(element, vgs, vds, vbs);
  const equivalentCurrent =
    result.drainCurrent - result.gm * vgs - result.gds * vds - result.gmb * vbs;

  stampConductance(matrix, drain, source, result.gds);
  stampTransconductance(matrix, drain, source, gate, source, result.gm);
  stampTransconductance(matrix, drain, source, body, source, result.gmb);
  stampCurrentSourceEquivalent(rhs, drain, source, equivalentCurrent);
}

function evaluateMosfetLevel1(
  element: Mosfet,
  vgs: number,
  vds: number,
  vbs: number,
): MosfetDcResult {
  if (element.type === "PMOS") {
    const result = evaluateNmosLevel1(element.params, -vgs, -vds, -vbs);
    return {
      drainCurrent: -result.drainCurrent,
      gm: result.gm,
      gds: result.gds,
      gmb: result.gmb,
    };
  }
  return evaluateNmosLevel1(element.params, vgs, vds, vbs);
}

function evaluateNmosLevel1(
  params: MosfetLevel1Params,
  vgs: number,
  vds: number,
  vbs: number,
): MosfetDcResult {
  const beta = params.KP * (params.W / params.L);
  const threshold =
    params.PHI - vbs >= 0.0
      ? params.VT0 + params.GAMMA * (Math.sqrt(params.PHI - vbs) - Math.sqrt(params.PHI))
      : params.VT0;
  const overdrive = vgs - threshold;
  if (overdrive <= 0.0) {
    return { drainCurrent: 0.0, gm: 0.0, gds: 0.0, gmb: 0.0 };
  }
  const bodyFactor =
    params.PHI - vbs > 0.0
      ? params.GAMMA / (2.0 * Math.sqrt(params.PHI - vbs))
      : 0.0;
  if (vds < overdrive) {
    const channel = overdrive * vds - 0.5 * vds * vds;
    const modulation = 1.0 + params.LAMBDA * vds;
    const gm = beta * vds * modulation;
    return {
      drainCurrent: beta * channel * modulation,
      gm,
      gds: beta * (overdrive - vds) * modulation + beta * channel * params.LAMBDA,
      gmb: gm * bodyFactor,
    };
  }
  const current = 0.5 * beta * overdrive * overdrive * (1.0 + params.LAMBDA * vds);
  const gm = beta * overdrive * (1.0 + params.LAMBDA * vds);
  return {
    drainCurrent: current,
    gm,
    gds: 0.5 * beta * overdrive * overdrive * params.LAMBDA,
    gmb: gm * bodyFactor,
  };
}

function vectorVoltage(vector: readonly number[], index: number | undefined): number {
  return index === undefined ? 0.0 : vector[index];
}

function stampCurrentSourceEquivalent(
  rhs: number[],
  positive: number | undefined,
  negative: number | undefined,
  current: number,
): void {
  if (positive !== undefined) {
    rhs[positive] -= current;
  }
  if (negative !== undefined) {
    rhs[negative] += current;
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

function validateDiode(element: Diode): void {
  if (!Number.isFinite(element.saturationCurrent) || element.saturationCurrent <= 0.0) {
    throw invalidElement(element.name, "saturation current must be finite and positive");
  }
  if (!Number.isFinite(element.thermalVoltage) || element.thermalVoltage <= 0.0) {
    throw invalidElement(element.name, "thermal voltage must be finite and positive");
  }
}

function validateBjt(element: Bjt): void {
  if (element.polarity !== "NPN" && element.polarity !== "PNP") {
    throw invalidElement(element.name, "BJT polarity must be NPN or PNP");
  }
  if (!Number.isFinite(element.saturationCurrent) || element.saturationCurrent <= 0.0) {
    throw invalidElement(element.name, "saturation current must be finite and positive");
  }
  if (!Number.isFinite(element.forwardBeta) || element.forwardBeta <= 0.0) {
    throw invalidElement(element.name, "forward beta must be finite and positive");
  }
  if (!Number.isFinite(element.thermalVoltage) || element.thermalVoltage <= 0.0) {
    throw invalidElement(element.name, "thermal voltage must be finite and positive");
  }
}

function validateMosfet(element: Mosfet): void {
  if (element.type !== "NMOS" && element.type !== "PMOS") {
    throw invalidElement(element.name, "MOSFET type must be NMOS or PMOS");
  }
  const params = element.params;
  for (const [name, value] of Object.entries(params)) {
    if (!Number.isFinite(value)) {
      throw invalidElement(element.name, `MOSFET ${name} must be finite`);
    }
  }
  if (params.KP <= 0.0) {
    throw invalidElement(element.name, "MOSFET KP must be positive");
  }
  if (params.W <= 0.0 || params.L <= 0.0) {
    throw invalidElement(element.name, "MOSFET W and L must be positive");
  }
  if (params.PHI <= 0.0) {
    throw invalidElement(element.name, "MOSFET PHI must be positive");
  }
  if (params.IS <= 0.0 || params.N_SUB <= 0.0 || params.T_NOM <= 0.0) {
    throw invalidElement(element.name, "MOSFET IS, N_SUB, and T_NOM must be positive");
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

class BSourceExpressionParser {
  private position = 0;

  constructor(
    private readonly expression: string,
    private readonly resolver: (node: string) => number,
  ) {}

  parse(): number {
    const value = this.parseExpression();
    this.skipWhitespace();
    if (this.position !== this.expression.length) {
      throw new Error("unexpected expression input");
    }
    return value;
  }

  private parseExpression(): number {
    let value = this.parseTerm();
    while (true) {
      this.skipWhitespace();
      if (this.consume("+")) {
        value += this.parseTerm();
      } else if (this.consume("-")) {
        value -= this.parseTerm();
      } else {
        return value;
      }
    }
  }

  private parseTerm(): number {
    let value = this.parseFactor();
    while (true) {
      this.skipWhitespace();
      if (this.consume("*")) {
        value *= this.parseFactor();
      } else if (this.consume("/")) {
        value /= this.parseFactor();
      } else {
        return value;
      }
    }
  }

  private parseFactor(): number {
    this.skipWhitespace();
    if (this.consume("+")) {
      return this.parseFactor();
    }
    if (this.consume("-")) {
      return -this.parseFactor();
    }
    if (this.consume("(")) {
      const value = this.parseExpression();
      this.expect(")");
      return value;
    }
    if (this.peekIdentifier("V")) {
      this.position += 1;
      this.expect("(");
      const first = this.parseNodeName();
      this.skipWhitespace();
      if (this.consume(",")) {
        const second = this.parseNodeName();
        this.expect(")");
        return this.resolver(first) - this.resolver(second);
      }
      this.expect(")");
      return this.resolver(first);
    }
    return this.parseNumber();
  }

  private parseNumber(): number {
    this.skipWhitespace();
    const start = this.position;
    if (this.expression[this.position] === ".") {
      this.position += 1;
    }
    while (/[0-9]/.test(this.expression[this.position] ?? "")) {
      this.position += 1;
    }
    if (this.expression[this.position] === ".") {
      this.position += 1;
      while (/[0-9]/.test(this.expression[this.position] ?? "")) {
        this.position += 1;
      }
    }
    if (/[eE]/.test(this.expression[this.position] ?? "")) {
      this.position += 1;
      if (/[+-]/.test(this.expression[this.position] ?? "")) {
        this.position += 1;
      }
      while (/[0-9]/.test(this.expression[this.position] ?? "")) {
        this.position += 1;
      }
    }
    if (this.position === start) {
      throw new Error("expected number");
    }
    const value = Number(this.expression.slice(start, this.position));
    if (!Number.isFinite(value)) {
      throw new Error("number must be finite");
    }
    return value;
  }

  private parseNodeName(): string {
    this.skipWhitespace();
    const start = this.position;
    while (/[A-Za-z0-9_.$:-]/.test(this.expression[this.position] ?? "")) {
      this.position += 1;
    }
    if (this.position === start) {
      throw new Error("expected node name");
    }
    this.skipWhitespace();
    return this.expression.slice(start, this.position);
  }

  private consume(token: string): boolean {
    this.skipWhitespace();
    if (this.expression.startsWith(token, this.position)) {
      this.position += token.length;
      return true;
    }
    return false;
  }

  private expect(token: string): void {
    if (!this.consume(token)) {
      throw new Error(`expected ${token}`);
    }
  }

  private peekIdentifier(name: string): boolean {
    this.skipWhitespace();
    return this.expression.startsWith(name, this.position);
  }

  private skipWhitespace(): void {
    while (/\s/.test(this.expression[this.position] ?? "")) {
      this.position += 1;
    }
  }
}

function bSourceExprNodes(expression: string): string[] {
  const nodes: string[] = [];
  const pattern = /V\s*\(([^)]*)\)/g;
  for (const match of expression.matchAll(pattern)) {
    for (const node of match[1].split(",")) {
      const trimmed = node.trim();
      if (trimmed !== "" && !isGround(trimmed)) {
        nodes.push(trimmed);
      }
    }
  }
  return nodes;
}

function evalBSourceExpression(
  expression: string,
  nodeIndices: ReadonlyMap<string, number>,
  operatingPoint: readonly number[],
): number {
  const parser = new BSourceExpressionParser(expression, (node) => {
    const index = nodeIndex(nodeIndices, node);
    return index === undefined ? 0.0 : operatingPoint[index];
  });
  const value = parser.parse();
  if (!Number.isFinite(value)) {
    throw new Error("expression produced a non-finite value");
  }
  return value;
}

function bSourceLinearization(
  expression: string,
  nodeIndices: ReadonlyMap<string, number>,
  operatingPoint: readonly number[],
): { derivatives: Map<string, number>; offset: number } {
  const value = evalBSourceExpression(expression, nodeIndices, operatingPoint);
  const derivatives = new Map<string, number>();
  for (const [node, index] of nodeIndices) {
    const h = Math.max(1.0e-6, Math.abs(operatingPoint[index]) * 1.0e-6);
    const plus = [...operatingPoint];
    const minus = [...operatingPoint];
    plus[index] += h;
    minus[index] -= h;
    derivatives.set(
      node,
      (evalBSourceExpression(expression, nodeIndices, plus) -
        evalBSourceExpression(expression, nodeIndices, minus)) /
        (2.0 * h),
    );
  }
  let linearPart = 0.0;
  for (const [node, derivative] of derivatives) {
    linearPart += derivative * operatingPoint[nodeIndices.get(node)!];
  }
  return { derivatives, offset: value - linearPart };
}

function stampVoltageSource(
  element: VoltageSource,
  nodeIndices: ReadonlyMap<string, number>,
  voltageSources: ReadonlyMap<string, number>,
  nodeCount: number,
  matrix: number[][],
  rhs: number[],
  sourceTime: number | undefined,
): void {
  const voltage = sourceVoltageAt(element, sourceTime);
  if (!Number.isFinite(voltage)) {
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
  rhs[branch] += voltage;
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
  sourceTime: number | undefined,
): void {
  const current = sourceCurrentAt(element, sourceTime);
  if (!Number.isFinite(current)) {
    throw invalidElement(element.name, "current must be finite");
  }

  const positive = nodeIndex(nodeIndices, element.positive);
  const negative = nodeIndex(nodeIndices, element.negative);
  if (positive !== undefined) {
    rhs[positive] -= current;
  }
  if (negative !== undefined) {
    rhs[negative] += current;
  }
}

function stampBSource(
  element: BSource,
  nodeIndices: ReadonlyMap<string, number>,
  voltageSources: ReadonlyMap<string, number>,
  nodeCount: number,
  matrix: number[][],
  rhs: number[],
  operatingPoint: readonly number[],
): void {
  const hasVoltage = element.voltageExpr !== undefined;
  const hasCurrent = element.currentExpr !== undefined;
  if (hasVoltage === hasCurrent) {
    throw invalidElement(
      element.name,
      "B-source must define exactly one voltageExpr or currentExpr",
    );
  }

  const positive = nodeIndex(nodeIndices, element.positive);
  const negative = nodeIndex(nodeIndices, element.negative);
  try {
    if (element.currentExpr !== undefined) {
      const { derivatives, offset } = bSourceLinearization(
        element.currentExpr,
        nodeIndices,
        operatingPoint,
      );
      if (positive !== undefined) {
        for (const [node, derivative] of derivatives) {
          matrix[positive][nodeIndices.get(node)!] += derivative;
        }
        rhs[positive] -= offset;
      }
      if (negative !== undefined) {
        for (const [node, derivative] of derivatives) {
          matrix[negative][nodeIndices.get(node)!] -= derivative;
        }
        rhs[negative] += offset;
      }
      return;
    }

    const sourceIndex = voltageSources.get(element.name);
    if (sourceIndex === undefined || element.voltageExpr === undefined) {
      throw invalidElement(element.name, "voltage B-source was not indexed");
    }
    const branch = nodeCount + sourceIndex;
    const { derivatives, offset } = bSourceLinearization(
      element.voltageExpr,
      nodeIndices,
      operatingPoint,
    );
    stampBranchMatrix(matrix, branch, positive, negative);
    for (const [node, derivative] of derivatives) {
      matrix[branch][nodeIndices.get(node)!] -= derivative;
    }
    rhs[branch] += offset;
  } catch (error) {
    if (error instanceof SpiceError) {
      throw error;
    }
    throw invalidElement(
      element.name,
      error instanceof Error ? error.message : "invalid behavioral expression",
    );
  }
}

function sourceVoltageAt(
  source: VoltageSource,
  sourceTime: number | undefined,
): number {
  if (sourceTime !== undefined && source.waveform !== undefined) {
    const reason = source.waveform.validate();
    if (reason !== undefined) {
      throw invalidElement(source.name, reason);
    }
    return source.waveform.valueAt(sourceTime);
  }
  return source.voltage;
}

function sourceCurrentAt(
  source: CurrentSource,
  sourceTime: number | undefined,
): number {
  if (sourceTime !== undefined && source.waveform !== undefined) {
    const reason = source.waveform.validate();
    if (reason !== undefined) {
      throw invalidElement(source.name, reason);
    }
    return source.waveform.valueAt(sourceTime);
  }
  return source.current;
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

function stampVcvs(
  element: Vcvs,
  nodeIndices: ReadonlyMap<string, number>,
  voltageSources: ReadonlyMap<string, number>,
  nodeCount: number,
  matrix: number[][],
): void {
  if (!Number.isFinite(element.gain)) {
    throw invalidElement(element.name, "gain must be finite");
  }

  const sourceIndex = voltageSources.get(element.name);
  if (sourceIndex === undefined) {
    throw invalidElement(element.name, "voltage source was not indexed");
  }

  const branch = nodeCount + sourceIndex;
  const positive = nodeIndex(nodeIndices, element.positive);
  const negative = nodeIndex(nodeIndices, element.negative);
  const controlPositive = nodeIndex(nodeIndices, element.controlPositive);
  const controlNegative = nodeIndex(nodeIndices, element.controlNegative);
  stampBranchMatrix(matrix, branch, positive, negative);
  stampControlledVoltageRow(
    matrix,
    branch,
    controlPositive,
    controlNegative,
    element.gain,
  );
}

function stampCccs(
  element: Cccs,
  nodeIndices: ReadonlyMap<string, number>,
  voltageSources: ReadonlyMap<string, number>,
  matrix: number[][],
): void {
  if (!Number.isFinite(element.gain)) {
    throw invalidElement(element.name, "gain must be finite");
  }

  const sourceIndex = voltageSources.get(element.controlSource);
  if (sourceIndex === undefined) {
    throw invalidElement(element.name, "control source was not indexed");
  }

  const positive = nodeIndex(nodeIndices, element.positive);
  const negative = nodeIndex(nodeIndices, element.negative);
  stampCurrentControlledCurrent(
    matrix,
    positive,
    negative,
    sourceIndex + nodeIndices.size,
    element.gain,
  );
}

function stampCcvs(
  element: Ccvs,
  nodeIndices: ReadonlyMap<string, number>,
  voltageSources: ReadonlyMap<string, number>,
  nodeCount: number,
  matrix: number[][],
): void {
  if (!Number.isFinite(element.transresistanceOhms)) {
    throw invalidElement(element.name, "transresistance must be finite");
  }

  const sourceIndex = voltageSources.get(element.name);
  if (sourceIndex === undefined) {
    throw invalidElement(element.name, "voltage source was not indexed");
  }
  const controlSourceIndex = voltageSources.get(element.controlSource);
  if (controlSourceIndex === undefined) {
    throw invalidElement(element.name, "control source was not indexed");
  }

  const branch = nodeCount + sourceIndex;
  const controlBranch = nodeCount + controlSourceIndex;
  const positive = nodeIndex(nodeIndices, element.positive);
  const negative = nodeIndex(nodeIndices, element.negative);
  stampBranchMatrix(matrix, branch, positive, negative);
  matrix[branch][controlBranch] -= element.transresistanceOhms;
}

function stampCurrentControlledCurrent(
  matrix: number[][],
  positive: number | undefined,
  negative: number | undefined,
  controlBranch: number,
  gain: number,
): void {
  if (positive !== undefined) {
    matrix[positive][controlBranch] += gain;
  }
  if (negative !== undefined) {
    matrix[negative][controlBranch] -= gain;
  }
}

function stampControlledVoltageRow(
  matrix: number[][],
  branch: number,
  controlPositive: number | undefined,
  controlNegative: number | undefined,
  gain: number,
): void {
  if (controlPositive !== undefined) {
    matrix[branch][controlPositive] -= gain;
  }
  if (controlNegative !== undefined) {
    matrix[branch][controlNegative] += gain;
  }
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

function stampBjtSmallSignal(
  element: Bjt,
  nodeIndices: ReadonlyMap<string, number>,
  matrix: number[][],
): void {
  validateBjt(element);
  const collector = nodeIndex(nodeIndices, element.collector);
  const base = nodeIndex(nodeIndices, element.base);
  const emitter = nodeIndex(nodeIndices, element.emitter);
  const transconductance = element.saturationCurrent / element.thermalVoltage;
  const junctionConductance = transconductance / element.forwardBeta;
  if (element.polarity === "NPN") {
    stampConductance(matrix, base, emitter, junctionConductance);
    stampTransconductance(matrix, collector, emitter, base, emitter, transconductance);
  } else {
    stampConductance(matrix, emitter, base, junctionConductance);
    stampTransconductance(matrix, emitter, collector, emitter, base, transconductance);
  }
}

function stampMosfetSmallSignal(
  element: Mosfet,
  nodeIndices: ReadonlyMap<string, number>,
  matrix: number[][],
): void {
  validateMosfet(element);
  const drain = nodeIndex(nodeIndices, element.drain);
  const gate = nodeIndex(nodeIndices, element.gate);
  const source = nodeIndex(nodeIndices, element.source);
  const body = nodeIndex(nodeIndices, element.body);
  const result = evaluateMosfetLevel1(element, 0.0, 0.0, 0.0);
  stampConductance(matrix, drain, source, result.gds);
  stampTransconductance(matrix, drain, source, gate, source, result.gm);
  stampTransconductance(matrix, drain, source, body, source, result.gmb);
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

function stampAcVoltageSourceRhs(
  element: VoltageSource,
  voltageSources: ReadonlyMap<string, number>,
  nodeCount: number,
  explicitAcSources: boolean,
  rhs: Complex[],
): void {
  if (!Number.isFinite(element.voltage)) {
    throw invalidElement(element.name, "voltage must be finite");
  }

  const sourceIndex = voltageSources.get(element.name);
  if (sourceIndex === undefined) {
    throw invalidElement(element.name, "voltage source was not indexed");
  }

  const phasor = voltageSourceAcPhasor(element, explicitAcSources);
  rhs[nodeCount + sourceIndex] = complexAdd(
    rhs[nodeCount + sourceIndex],
    phasor,
  );
}

function stampAcVoltageSourceMatrix(
  element: VoltageSource,
  nodeIndices: ReadonlyMap<string, number>,
  voltageSources: ReadonlyMap<string, number>,
  nodeCount: number,
  matrix: Complex[][],
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
  explicitAcSources: boolean,
  rhs: Complex[],
): void {
  if (!Number.isFinite(element.current)) {
    throw invalidElement(element.name, "current must be finite");
  }

  const current = currentSourceAcPhasor(element, explicitAcSources);
  const positive = nodeIndex(nodeIndices, element.positive);
  const negative = nodeIndex(nodeIndices, element.negative);
  if (positive !== undefined) {
    rhs[positive] = complexSub(rhs[positive], current);
  }
  if (negative !== undefined) {
    rhs[negative] = complexAdd(rhs[negative], current);
  }
}

function voltageSourceAcPhasor(
  element: VoltageSource,
  explicitAcSources: boolean,
): Complex {
  return sourceAcPhasor(element.name, element.ac, element.voltage, explicitAcSources);
}

function currentSourceAcPhasor(
  element: CurrentSource,
  explicitAcSources: boolean,
): Complex {
  return sourceAcPhasor(element.name, element.ac, element.current, explicitAcSources);
}

function sourceAcPhasor(
  elementName: string,
  ac: AcSource | undefined,
  legacyValue: number,
  explicitAcSources: boolean,
): Complex {
  if (ac === undefined) {
    return explicitAcSources ? complex(0.0, 0.0) : complex(legacyValue, 0.0);
  }
  validateAcSource(elementName, ac);
  const phaseRadians = (ac.phaseDegrees * Math.PI) / 180.0;
  return complex(
    ac.magnitude * Math.cos(phaseRadians),
    ac.magnitude * Math.sin(phaseRadians),
  );
}

function validateAcSource(elementName: string, ac: AcSource): void {
  if (!Number.isFinite(ac.magnitude) || !Number.isFinite(ac.phaseDegrees)) {
    throw invalidElement(elementName, "AC magnitude and phase must be finite");
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

function stampAcVcvs(
  element: Vcvs,
  nodeIndices: ReadonlyMap<string, number>,
  voltageSources: ReadonlyMap<string, number>,
  nodeCount: number,
  matrix: Complex[][],
): void {
  if (!Number.isFinite(element.gain)) {
    throw invalidElement(element.name, "gain must be finite");
  }

  const sourceIndex = voltageSources.get(element.name);
  if (sourceIndex === undefined) {
    throw invalidElement(element.name, "voltage source was not indexed");
  }

  const branch = nodeCount + sourceIndex;
  const positive = nodeIndex(nodeIndices, element.positive);
  const negative = nodeIndex(nodeIndices, element.negative);
  const controlPositive = nodeIndex(nodeIndices, element.controlPositive);
  const controlNegative = nodeIndex(nodeIndices, element.controlNegative);
  stampComplexBranchMatrix(matrix, branch, positive, negative);
  stampComplexControlledVoltageRow(
    matrix,
    branch,
    controlPositive,
    controlNegative,
    complex(element.gain, 0.0),
  );
}

function stampAcCccs(
  element: Cccs,
  nodeIndices: ReadonlyMap<string, number>,
  voltageSources: ReadonlyMap<string, number>,
  matrix: Complex[][],
): void {
  if (!Number.isFinite(element.gain)) {
    throw invalidElement(element.name, "gain must be finite");
  }

  const sourceIndex = voltageSources.get(element.controlSource);
  if (sourceIndex === undefined) {
    throw invalidElement(element.name, "control source was not indexed");
  }

  const positive = nodeIndex(nodeIndices, element.positive);
  const negative = nodeIndex(nodeIndices, element.negative);
  stampComplexCurrentControlledCurrent(
    matrix,
    positive,
    negative,
    sourceIndex + nodeIndices.size,
    complex(element.gain, 0.0),
  );
}

function stampAcCcvs(
  element: Ccvs,
  nodeIndices: ReadonlyMap<string, number>,
  voltageSources: ReadonlyMap<string, number>,
  nodeCount: number,
  matrix: Complex[][],
): void {
  if (!Number.isFinite(element.transresistanceOhms)) {
    throw invalidElement(element.name, "transresistance must be finite");
  }

  const sourceIndex = voltageSources.get(element.name);
  if (sourceIndex === undefined) {
    throw invalidElement(element.name, "voltage source was not indexed");
  }
  const controlSourceIndex = voltageSources.get(element.controlSource);
  if (controlSourceIndex === undefined) {
    throw invalidElement(element.name, "control source was not indexed");
  }

  const branch = nodeCount + sourceIndex;
  const controlBranch = nodeCount + controlSourceIndex;
  const positive = nodeIndex(nodeIndices, element.positive);
  const negative = nodeIndex(nodeIndices, element.negative);
  stampComplexBranchMatrix(matrix, branch, positive, negative);
  matrix[branch][controlBranch] = complexSub(
    matrix[branch][controlBranch],
    complex(element.transresistanceOhms, 0.0),
  );
}

function stampComplexCurrentControlledCurrent(
  matrix: Complex[][],
  positive: number | undefined,
  negative: number | undefined,
  controlBranch: number,
  gain: Complex,
): void {
  if (positive !== undefined) {
    matrix[positive][controlBranch] = complexAdd(
      matrix[positive][controlBranch],
      gain,
    );
  }
  if (negative !== undefined) {
    matrix[negative][controlBranch] = complexSub(
      matrix[negative][controlBranch],
      gain,
    );
  }
}

function stampComplexControlledVoltageRow(
  matrix: Complex[][],
  branch: number,
  controlPositive: number | undefined,
  controlNegative: number | undefined,
  gain: Complex,
): void {
  if (controlPositive !== undefined) {
    matrix[branch][controlPositive] = complexSub(
      matrix[branch][controlPositive],
      gain,
    );
  }
  if (controlNegative !== undefined) {
    matrix[branch][controlNegative] = complexAdd(
      matrix[branch][controlNegative],
      gain,
    );
  }
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

function stampAcBjtSmallSignal(
  element: Bjt,
  nodeIndices: ReadonlyMap<string, number>,
  matrix: Complex[][],
): void {
  validateBjt(element);
  const collector = nodeIndex(nodeIndices, element.collector);
  const base = nodeIndex(nodeIndices, element.base);
  const emitter = nodeIndex(nodeIndices, element.emitter);
  const transconductance = element.saturationCurrent / element.thermalVoltage;
  const junctionConductance = transconductance / element.forwardBeta;
  if (element.polarity === "NPN") {
    stampComplexConductance(
      matrix,
      base,
      emitter,
      complex(junctionConductance, 0.0),
    );
    stampComplexTransconductance(
      matrix,
      collector,
      emitter,
      base,
      emitter,
      complex(transconductance, 0.0),
    );
  } else {
    stampComplexConductance(
      matrix,
      emitter,
      base,
      complex(junctionConductance, 0.0),
    );
    stampComplexTransconductance(
      matrix,
      emitter,
      collector,
      emitter,
      base,
      complex(transconductance, 0.0),
    );
  }
}

function stampAcMosfetSmallSignal(
  element: Mosfet,
  nodeIndices: ReadonlyMap<string, number>,
  matrix: Complex[][],
): void {
  validateMosfet(element);
  const drain = nodeIndex(nodeIndices, element.drain);
  const gate = nodeIndex(nodeIndices, element.gate);
  const source = nodeIndex(nodeIndices, element.source);
  const body = nodeIndex(nodeIndices, element.body);
  const result = evaluateMosfetLevel1(element, 0.0, 0.0, 0.0);
  stampComplexConductance(matrix, drain, source, complex(result.gds, 0.0));
  stampComplexTransconductance(
    matrix,
    drain,
    source,
    gate,
    source,
    complex(result.gm, 0.0),
  );
  stampComplexTransconductance(
    matrix,
    drain,
    source,
    body,
    source,
    complex(result.gmb, 0.0),
  );
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

function transposeComplexMatrix(matrix: readonly (readonly Complex[])[]): Complex[][] {
  return matrix.map((row, rowIndex) =>
    row.map((_value, colIndex) => matrix[colIndex][rowIndex]),
  );
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
