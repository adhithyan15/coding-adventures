const PIVOT_EPSILON = 1.0e-12;
const SPARSE_SOLVER_THRESHOLD = 30;
const TWO_PI = Math.PI * 2.0;
const BOLTZMANN = 1.380_649e-23;
const ELECTRON_CHARGE = 1.602_176_634e-19;
const MOSFET_CHANNEL_NOISE_GAMMA = 2.0 / 3.0;

export type Element =
  | Resistor
  | Capacitor
  | Inductor
  | MutualInductor
  | TransmissionLine
  | VoltageSource
  | CurrentSource
  | BSource
  | Diode
  | Jfet
  | Bjt
  | Mosfet
  | Vccs
  | Vcvs
  | Cccs
  | Ccvs;

export type TransientMethod = "euler" | "trap" | "gear2";

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

export interface MutualInductor {
  readonly kind: "mutual-inductor";
  readonly name: string;
  readonly primary: string;
  readonly secondary: string;
  readonly coupling: number;
}

export interface TransmissionLine {
  readonly kind: "transmission-line";
  readonly name: string;
  readonly n1: string;
  readonly n2: string;
  readonly n3: string;
  readonly n4: string;
  readonly characteristicImpedanceOhms: number;
  readonly delaySeconds: number;
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

export function waveformPeriod(waveform: Waveform): number | undefined {
  if (waveform instanceof SinWaveform) {
    if (
      Number.isFinite(waveform.frequencyHz) &&
      waveform.frequencyHz > 0.0 &&
      waveform.damping === 0.0
    ) {
      return 1.0 / waveform.frequencyHz;
    }
    return undefined;
  }
  if (waveform instanceof PulseWaveform) {
    if (Number.isFinite(waveform.periodSeconds) && waveform.periodSeconds > 0.0) {
      return waveform.periodSeconds;
    }
  }
  return undefined;
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

export type SubcircuitElement = Element | XInstance;

export interface SubcircuitDefinition {
  readonly name: string;
  readonly pins: readonly string[];
  readonly elements: readonly SubcircuitElement[];
  readonly parameters?: Readonly<Record<string, number>>;
}

export interface XInstance {
  readonly kind: "x-instance";
  readonly name: string;
  readonly nodes: readonly string[];
  readonly subckt: string;
  readonly parameters?: Readonly<Record<string, number>>;
}

export interface Diode {
  readonly kind: "diode";
  readonly name: string;
  readonly anode: string;
  readonly cathode: string;
  readonly saturationCurrent: number;
  readonly thermalVoltage: number;
  readonly emissionCoefficient: number;
  readonly breakdownVoltage?: number;
  readonly breakdownCurrent: number;
  readonly junctionCapacitance: number;
  readonly transitTime: number;
}

export type JfetPolarity = "NJF" | "PJF";

export interface Jfet {
  readonly kind: "jfet";
  readonly name: string;
  readonly drain: string;
  readonly gate: string;
  readonly source: string;
  readonly polarity: JfetPolarity;
  readonly beta: number;
  readonly thresholdVoltage: number;
  readonly channelLengthModulation: number;
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
  readonly baseEmitterCapacitance: number;
  readonly baseCollectorCapacitance: number;
  readonly forwardTransitTime: number;
  readonly reverseTransitTime: number;
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
  readonly CGSO: number;
  readonly CGDO: number;
  readonly CGBO: number;
  readonly CBS: number;
  readonly CBD: number;
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
  readonly convergenceAid: DcConvergenceAid;
  voltage(node: string): number | undefined;
  branchCurrent(sourceName: string): number | undefined;
}

export type DcConvergenceAid = "newton" | "gmin" | "source" | "pseudo_transient" | "none";

export interface DcOpOptions {
  readonly maxIterations?: number;
  readonly tolerance?: number;
  readonly convergenceAids?: boolean;
  readonly pseudoTransientSteps?: number;
  readonly pseudoTransientConductance?: number;
  readonly pseudoTransientMaxIterations?: number;
}

export interface CornerOverride {
  readonly elementName: string;
  readonly parameter: "resistance" | "capacitance" | "inductance" | "voltage" | "current";
  readonly value: number;
}

export interface CornerSpec {
  readonly name: string;
  readonly overrides: readonly CornerOverride[];
}

export interface CornerPoint {
  readonly cornerName: string;
  readonly result: DcResult;
}

export interface CornerSweepResult {
  readonly points: readonly CornerPoint[];
}

export interface DcSweepPoint {
  readonly value: number;
  readonly result: DcResult;
}

export interface CornerDcSweepPoint {
  readonly cornerName: string;
  readonly points: readonly DcSweepPoint[];
}

export interface CornerDcSweepResult {
  readonly sourceName: string;
  readonly points: readonly CornerDcSweepPoint[];
}

export interface CornerAcSweepPoint {
  readonly cornerName: string;
  readonly points: readonly AcPoint[];
}

export interface CornerAcSweepResult {
  readonly points: readonly CornerAcSweepPoint[];
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

export interface CornerMcPoint {
  readonly cornerName: string;
  readonly result: McResult;
}

export interface CornerMcResult {
  readonly outputNode: string;
  readonly points: readonly CornerMcPoint[];
}

export interface TfResult {
  readonly transferRatio: number;
  readonly inputImpedanceOhms: number;
  readonly outputImpedanceOhms: number;
  gain(): number;
}

export interface CornerTfPoint {
  readonly cornerName: string;
  readonly result: TfResult;
}

export interface CornerTfResult {
  readonly inputSource: string;
  readonly outputNode: string;
  readonly points: readonly CornerTfPoint[];
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

export interface CornerSensPoint {
  readonly cornerName: string;
  readonly result: SensResult;
}

export interface CornerSensResult {
  readonly outputNode: string;
  readonly points: readonly CornerSensPoint[];
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

export interface SParameterPoint {
  readonly frequencyHz: number;
  readonly s11: Complex;
  readonly s21: Complex;
  readonly s12: Complex;
  readonly s22: Complex;
}

export interface SParameterResult {
  readonly port1Source: string;
  readonly port2Source: string;
  readonly referenceImpedanceOhms: number;
  readonly points: readonly SParameterPoint[];
}

export interface CornerSParameterPoint {
  readonly cornerName: string;
  readonly result: SParameterResult;
}

export interface CornerSParameterResult {
  readonly port1Source: string;
  readonly port2Source: string;
  readonly referenceImpedanceOhms: number;
  readonly points: readonly CornerSParameterPoint[];
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

export interface CornerNoisePoint {
  readonly cornerName: string;
  readonly result: NoiseResult;
}

export interface CornerNoiseResult {
  readonly outputNode: string;
  readonly inputSource: string;
  readonly points: readonly CornerNoisePoint[];
}

export interface TransientPoint {
  readonly time: number;
  readonly nodeVoltages: ReadonlyMap<string, number>;
  readonly branchCurrents: ReadonlyMap<string, number>;
  voltage(node: string): number | undefined;
  branchCurrent(sourceName: string): number | undefined;
}

export interface FourierHarmonic {
  readonly harmonic: number;
  readonly frequencyHz: number;
  readonly cosine: number;
  readonly sine: number;
  readonly magnitude: number;
  readonly phaseDegrees: number;
}

export interface FourierProbeResult {
  readonly probe: string;
  readonly dc: number;
  readonly harmonics: readonly FourierHarmonic[];
  readonly totalHarmonicDistortion: number;
}

export interface FourierResult {
  readonly fundamentalFrequencyHz: number;
  readonly startTime: number;
  readonly endTime: number;
  readonly probes: readonly FourierProbeResult[];
}

export interface DistortionHarmonic {
  readonly harmonic: number;
  readonly frequencyHz: number;
  readonly magnitude: number;
  readonly phaseDegrees: number;
}

export interface DistortionPoint {
  readonly frequencyHz: number;
  readonly fundamentalMagnitude: number;
  readonly harmonics: readonly DistortionHarmonic[];
  readonly totalHarmonicDistortion: number;
}

export interface DistortionResult {
  readonly inputSource: string;
  readonly outputProbe: string;
  readonly points: readonly DistortionPoint[];
}

export interface PoleZeroEntry {
  readonly kind: "pole" | "zero";
  readonly real: number;
  readonly imaginary: number;
  readonly frequencyHz: number;
  readonly damping: number;
}

export interface PoleZeroResult {
  readonly inputSource: string;
  readonly outputNode: string;
  readonly entries: readonly PoleZeroEntry[];
}

export function poleZeroRcLowpass(
  circuit: Circuit,
  inputSource: string,
  outputNode: string,
): PoleZeroResult {
  const source = circuit
    .elements()
    .find((element): element is VoltageSource => element.kind === "voltage-source" && element.name === inputSource);
  if (source === undefined) {
    throw new SpiceError(`poleZeroRcLowpass: missing input source ${JSON.stringify(inputSource)}`, "INVALID_ELEMENT", inputSource);
  }
  if (!isGround(source.negative)) {
    throw new SpiceError("poleZeroRcLowpass: input source negative terminal must be ground", "INVALID_ELEMENT", inputSource);
  }

  const resistor = circuit
    .elements()
    .find(
      (element): element is Resistor =>
        element.kind === "resistor" &&
        ((element.n1 === source.positive && element.n2 === outputNode) ||
          (element.n2 === source.positive && element.n1 === outputNode)),
    );
  const capacitor = circuit
    .elements()
    .find(
      (element): element is Capacitor =>
        element.kind === "capacitor" &&
        (element.n1 === outputNode || element.n2 === outputNode) &&
        (isGround(element.n1) || isGround(element.n2)),
    );
  if (resistor === undefined || capacitor === undefined) {
    throw new SpiceError(
      "poleZeroRcLowpass: expected one resistor from input to output and one grounded output capacitor",
      "INVALID_ELEMENT",
      outputNode,
    );
  }
  if (!Number.isFinite(resistor.resistanceOhms) || resistor.resistanceOhms <= 0.0) {
    throw new SpiceError("poleZeroRcLowpass: resistance must be finite and positive", "INVALID_ELEMENT", resistor.name);
  }
  if (!Number.isFinite(capacitor.capacitanceFarads) || capacitor.capacitanceFarads <= 0.0) {
    throw new SpiceError("poleZeroRcLowpass: capacitance must be finite and positive", "INVALID_ELEMENT", capacitor.name);
  }

  const real = -1.0 / (resistor.resistanceOhms * capacitor.capacitanceFarads);
  return {
    inputSource,
    outputNode,
    entries: [
      {
        kind: "pole",
        real,
        imaginary: 0.0,
        frequencyHz: Math.abs(real) / TWO_PI,
        damping: 1.0,
      },
    ],
  };
}

export function poleZeroRcHighpass(
  circuit: Circuit,
  inputSource: string,
  outputNode: string,
): PoleZeroResult {
  const source = circuit
    .elements()
    .find((element): element is VoltageSource => element.kind === "voltage-source" && element.name === inputSource);
  if (source === undefined) {
    throw new SpiceError(`poleZeroRcHighpass: missing input source ${JSON.stringify(inputSource)}`, "INVALID_ELEMENT", inputSource);
  }
  if (!isGround(source.negative)) {
    throw new SpiceError("poleZeroRcHighpass: input source negative terminal must be ground", "INVALID_ELEMENT", inputSource);
  }

  const capacitor = circuit
    .elements()
    .find(
      (element): element is Capacitor =>
        element.kind === "capacitor" &&
        ((element.n1 === source.positive && element.n2 === outputNode) ||
          (element.n2 === source.positive && element.n1 === outputNode)),
    );
  const resistor = circuit
    .elements()
    .find(
      (element): element is Resistor =>
        element.kind === "resistor" &&
        (element.n1 === outputNode || element.n2 === outputNode) &&
        (isGround(element.n1) || isGround(element.n2)),
    );
  if (capacitor === undefined || resistor === undefined) {
    throw new SpiceError(
      "poleZeroRcHighpass: expected one capacitor from input to output and one grounded output resistor",
      "INVALID_ELEMENT",
      outputNode,
    );
  }
  if (!Number.isFinite(resistor.resistanceOhms) || resistor.resistanceOhms <= 0.0) {
    throw new SpiceError("poleZeroRcHighpass: resistance must be finite and positive", "INVALID_ELEMENT", resistor.name);
  }
  if (!Number.isFinite(capacitor.capacitanceFarads) || capacitor.capacitanceFarads <= 0.0) {
    throw new SpiceError("poleZeroRcHighpass: capacitance must be finite and positive", "INVALID_ELEMENT", capacitor.name);
  }

  const real = -1.0 / (resistor.resistanceOhms * capacitor.capacitanceFarads);
  return {
    inputSource,
    outputNode,
    entries: [
      {
        kind: "zero",
        real: 0.0,
        imaginary: 0.0,
        frequencyHz: 0.0,
        damping: 1.0,
      },
      {
        kind: "pole",
        real,
        imaginary: 0.0,
        frequencyHz: Math.abs(real) / TWO_PI,
        damping: 1.0,
      },
    ],
  };
}

export function poleZeroRlcLowpass(
  circuit: Circuit,
  inputSource: string,
  outputNode: string,
): PoleZeroResult {
  const source = circuit
    .elements()
    .find((element): element is VoltageSource => element.kind === "voltage-source" && element.name === inputSource);
  if (source === undefined) {
    throw new SpiceError(`poleZeroRlcLowpass: missing input source ${JSON.stringify(inputSource)}`, "INVALID_ELEMENT", inputSource);
  }
  if (!isGround(source.negative)) {
    throw new SpiceError("poleZeroRlcLowpass: input source negative terminal must be ground", "INVALID_ELEMENT", inputSource);
  }

  let resistor: Resistor | undefined;
  let intermediate: string | undefined;
  for (const element of circuit.elements()) {
    if (element.kind === "resistor" && (element.n1 === source.positive || element.n2 === source.positive)) {
      const other = element.n1 === source.positive ? element.n2 : element.n1;
      if (other !== outputNode && !isGround(other)) {
        resistor = element;
        intermediate = other;
        break;
      }
    }
  }
  const inductor = circuit
    .elements()
    .find(
      (element): element is Inductor =>
        element.kind === "inductor" &&
        intermediate !== undefined &&
        ((element.n1 === intermediate && element.n2 === outputNode) ||
          (element.n2 === intermediate && element.n1 === outputNode)),
    );
  const capacitor = circuit
    .elements()
    .find(
      (element): element is Capacitor =>
        element.kind === "capacitor" &&
        (element.n1 === outputNode || element.n2 === outputNode) &&
        (isGround(element.n1) || isGround(element.n2)),
    );
  if (resistor === undefined || inductor === undefined || capacitor === undefined) {
    throw new SpiceError(
      "poleZeroRlcLowpass: expected series resistor and inductor from input to output plus one grounded output capacitor",
      "INVALID_ELEMENT",
      outputNode,
    );
  }
  if (!Number.isFinite(resistor.resistanceOhms) || resistor.resistanceOhms <= 0.0) {
    throw new SpiceError("poleZeroRlcLowpass: resistance must be finite and positive", "INVALID_ELEMENT", resistor.name);
  }
  if (!Number.isFinite(inductor.inductanceHenrys) || inductor.inductanceHenrys <= 0.0) {
    throw new SpiceError("poleZeroRlcLowpass: inductance must be finite and positive", "INVALID_ELEMENT", inductor.name);
  }
  if (!Number.isFinite(capacitor.capacitanceFarads) || capacitor.capacitanceFarads <= 0.0) {
    throw new SpiceError("poleZeroRlcLowpass: capacitance must be finite and positive", "INVALID_ELEMENT", capacitor.name);
  }

  const alpha = resistor.resistanceOhms / (2.0 * inductor.inductanceHenrys);
  const omega0 = 1.0 / Math.sqrt(inductor.inductanceHenrys * capacitor.capacitanceFarads);
  const discriminant = alpha * alpha - omega0 * omega0;
  let entries: readonly PoleZeroEntry[];
  if (discriminant >= 0.0) {
    const root = Math.sqrt(discriminant);
    const first = -alpha + root;
    const second = -alpha - root;
    entries = [
      {
        kind: "pole",
        real: first,
        imaginary: 0.0,
        frequencyHz: Math.abs(first) / TWO_PI,
        damping: 1.0,
      },
      {
        kind: "pole",
        real: second,
        imaginary: 0.0,
        frequencyHz: Math.abs(second) / TWO_PI,
        damping: 1.0,
      },
    ];
  } else {
    const imaginary = Math.sqrt(-discriminant);
    entries = [
      {
        kind: "pole",
        real: -alpha,
        imaginary,
        frequencyHz: omega0 / TWO_PI,
        damping: alpha / omega0,
      },
      {
        kind: "pole",
        real: -alpha,
        imaginary: -imaginary,
        frequencyHz: omega0 / TWO_PI,
        damping: alpha / omega0,
      },
    ];
  }
  return {
    inputSource,
    outputNode,
    entries,
  };
}

export function poleZeroRlcHighpass(
  circuit: Circuit,
  inputSource: string,
  outputNode: string,
): PoleZeroResult {
  const source = circuit
    .elements()
    .find((element): element is VoltageSource => element.kind === "voltage-source" && element.name === inputSource);
  if (source === undefined) {
    throw new SpiceError(`poleZeroRlcHighpass: missing input source ${JSON.stringify(inputSource)}`, "INVALID_ELEMENT", inputSource);
  }
  if (!isGround(source.negative)) {
    throw new SpiceError("poleZeroRlcHighpass: input source negative terminal must be ground", "INVALID_ELEMENT", inputSource);
  }

  let resistor: Resistor | undefined;
  let intermediate: string | undefined;
  for (const element of circuit.elements()) {
    if (element.kind === "resistor" && (element.n1 === source.positive || element.n2 === source.positive)) {
      const other = element.n1 === source.positive ? element.n2 : element.n1;
      if (other !== outputNode && !isGround(other)) {
        resistor = element;
        intermediate = other;
        break;
      }
    }
  }
  const capacitor = circuit
    .elements()
    .find(
      (element): element is Capacitor =>
        element.kind === "capacitor" &&
        intermediate !== undefined &&
        ((element.n1 === intermediate && element.n2 === outputNode) ||
          (element.n2 === intermediate && element.n1 === outputNode)),
    );
  const inductor = circuit
    .elements()
    .find(
      (element): element is Inductor =>
        element.kind === "inductor" &&
        (element.n1 === outputNode || element.n2 === outputNode) &&
        (isGround(element.n1) || isGround(element.n2)),
    );
  if (resistor === undefined || capacitor === undefined || inductor === undefined) {
    throw new SpiceError(
      "poleZeroRlcHighpass: expected series resistor and capacitor from input to output plus one grounded output inductor",
      "INVALID_ELEMENT",
      outputNode,
    );
  }
  if (!Number.isFinite(resistor.resistanceOhms) || resistor.resistanceOhms <= 0.0) {
    throw new SpiceError("poleZeroRlcHighpass: resistance must be finite and positive", "INVALID_ELEMENT", resistor.name);
  }
  if (!Number.isFinite(capacitor.capacitanceFarads) || capacitor.capacitanceFarads <= 0.0) {
    throw new SpiceError("poleZeroRlcHighpass: capacitance must be finite and positive", "INVALID_ELEMENT", capacitor.name);
  }
  if (!Number.isFinite(inductor.inductanceHenrys) || inductor.inductanceHenrys <= 0.0) {
    throw new SpiceError("poleZeroRlcHighpass: inductance must be finite and positive", "INVALID_ELEMENT", inductor.name);
  }

  const alpha = resistor.resistanceOhms / (2.0 * inductor.inductanceHenrys);
  const omega0 = 1.0 / Math.sqrt(inductor.inductanceHenrys * capacitor.capacitanceFarads);
  const discriminant = alpha * alpha - omega0 * omega0;
  const entries: PoleZeroEntry[] = [
    {
      kind: "zero",
      real: 0.0,
      imaginary: 0.0,
      frequencyHz: 0.0,
      damping: 1.0,
    },
    {
      kind: "zero",
      real: 0.0,
      imaginary: 0.0,
      frequencyHz: 0.0,
      damping: 1.0,
    },
  ];
  if (discriminant >= 0.0) {
    const root = Math.sqrt(discriminant);
    const first = -alpha + root;
    const second = -alpha - root;
    entries.push(
      {
        kind: "pole",
        real: first,
        imaginary: 0.0,
        frequencyHz: Math.abs(first) / TWO_PI,
        damping: 1.0,
      },
      {
        kind: "pole",
        real: second,
        imaginary: 0.0,
        frequencyHz: Math.abs(second) / TWO_PI,
        damping: 1.0,
      },
    );
  } else {
    const imaginary = Math.sqrt(-discriminant);
    entries.push(
      {
        kind: "pole",
        real: -alpha,
        imaginary,
        frequencyHz: omega0 / TWO_PI,
        damping: alpha / omega0,
      },
      {
        kind: "pole",
        real: -alpha,
        imaginary: -imaginary,
        frequencyHz: omega0 / TWO_PI,
        damping: alpha / omega0,
      },
    );
  }
  return {
    inputSource,
    outputNode,
    entries,
  };
}

export function poleZeroRlcBandpass(
  circuit: Circuit,
  inputSource: string,
  outputNode: string,
): PoleZeroResult {
  const source = circuit
    .elements()
    .find((element): element is VoltageSource => element.kind === "voltage-source" && element.name === inputSource);
  if (source === undefined) {
    throw new SpiceError(`poleZeroRlcBandpass: missing input source ${JSON.stringify(inputSource)}`, "INVALID_ELEMENT", inputSource);
  }
  if (!isGround(source.negative)) {
    throw new SpiceError("poleZeroRlcBandpass: input source negative terminal must be ground", "INVALID_ELEMENT", inputSource);
  }

  let inductor: Inductor | undefined;
  let intermediate: string | undefined;
  for (const element of circuit.elements()) {
    if (element.kind === "inductor" && (element.n1 === source.positive || element.n2 === source.positive)) {
      const other = element.n1 === source.positive ? element.n2 : element.n1;
      if (other !== outputNode && !isGround(other)) {
        inductor = element;
        intermediate = other;
        break;
      }
    }
  }
  const capacitor = circuit
    .elements()
    .find(
      (element): element is Capacitor =>
        element.kind === "capacitor" &&
        intermediate !== undefined &&
        ((element.n1 === intermediate && element.n2 === outputNode) ||
          (element.n2 === intermediate && element.n1 === outputNode)),
    );
  const resistor = circuit
    .elements()
    .find(
      (element): element is Resistor =>
        element.kind === "resistor" &&
        (element.n1 === outputNode || element.n2 === outputNode) &&
        (isGround(element.n1) || isGround(element.n2)),
    );
  if (inductor === undefined || capacitor === undefined || resistor === undefined) {
    throw new SpiceError(
      "poleZeroRlcBandpass: expected series inductor and capacitor from input to output plus one grounded output resistor",
      "INVALID_ELEMENT",
      outputNode,
    );
  }
  if (!Number.isFinite(inductor.inductanceHenrys) || inductor.inductanceHenrys <= 0.0) {
    throw new SpiceError("poleZeroRlcBandpass: inductance must be finite and positive", "INVALID_ELEMENT", inductor.name);
  }
  if (!Number.isFinite(capacitor.capacitanceFarads) || capacitor.capacitanceFarads <= 0.0) {
    throw new SpiceError("poleZeroRlcBandpass: capacitance must be finite and positive", "INVALID_ELEMENT", capacitor.name);
  }
  if (!Number.isFinite(resistor.resistanceOhms) || resistor.resistanceOhms <= 0.0) {
    throw new SpiceError("poleZeroRlcBandpass: resistance must be finite and positive", "INVALID_ELEMENT", resistor.name);
  }

  const alpha = resistor.resistanceOhms / (2.0 * inductor.inductanceHenrys);
  const omega0 = 1.0 / Math.sqrt(inductor.inductanceHenrys * capacitor.capacitanceFarads);
  const discriminant = alpha * alpha - omega0 * omega0;
  const entries: PoleZeroEntry[] = [
    {
      kind: "zero",
      real: 0.0,
      imaginary: 0.0,
      frequencyHz: 0.0,
      damping: 1.0,
    },
  ];
  if (discriminant >= 0.0) {
    const root = Math.sqrt(discriminant);
    const first = -alpha + root;
    const second = -alpha - root;
    entries.push(
      {
        kind: "pole",
        real: first,
        imaginary: 0.0,
        frequencyHz: Math.abs(first) / TWO_PI,
        damping: 1.0,
      },
      {
        kind: "pole",
        real: second,
        imaginary: 0.0,
        frequencyHz: Math.abs(second) / TWO_PI,
        damping: 1.0,
      },
    );
  } else {
    const imaginary = Math.sqrt(-discriminant);
    entries.push(
      {
        kind: "pole",
        real: -alpha,
        imaginary,
        frequencyHz: omega0 / TWO_PI,
        damping: alpha / omega0,
      },
      {
        kind: "pole",
        real: -alpha,
        imaginary: -imaginary,
        frequencyHz: omega0 / TWO_PI,
        damping: alpha / omega0,
      },
    );
  }
  return {
    inputSource,
    outputNode,
    entries,
  };
}

export function poleZeroRlcNotch(
  circuit: Circuit,
  inputSource: string,
  outputNode: string,
): PoleZeroResult {
  const source = circuit
    .elements()
    .find((element): element is VoltageSource => element.kind === "voltage-source" && element.name === inputSource);
  if (source === undefined) {
    throw new SpiceError(`poleZeroRlcNotch: missing input source ${JSON.stringify(inputSource)}`, "INVALID_ELEMENT", inputSource);
  }
  if (!isGround(source.negative)) {
    throw new SpiceError("poleZeroRlcNotch: input source negative terminal must be ground", "INVALID_ELEMENT", inputSource);
  }

  const resistor = circuit
    .elements()
    .find(
      (element): element is Resistor =>
        element.kind === "resistor" &&
        (element.n1 === source.positive || element.n2 === source.positive) &&
        (element.n1 === outputNode || element.n2 === outputNode),
    );
  let inductor: Inductor | undefined;
  let intermediate: string | undefined;
  for (const element of circuit.elements()) {
    if (element.kind === "inductor" && (element.n1 === outputNode || element.n2 === outputNode)) {
      const other = element.n1 === outputNode ? element.n2 : element.n1;
      if (!isGround(other)) {
        inductor = element;
        intermediate = other;
        break;
      }
    }
  }
  const capacitor = circuit
    .elements()
    .find(
      (element): element is Capacitor =>
        element.kind === "capacitor" &&
        intermediate !== undefined &&
        (element.n1 === intermediate || element.n2 === intermediate) &&
        (isGround(element.n1) || isGround(element.n2)),
    );
  if (resistor === undefined || inductor === undefined || capacitor === undefined) {
    throw new SpiceError(
      "poleZeroRlcNotch: expected series resistor from input to output plus a grounded series inductor-capacitor branch at output",
      "INVALID_ELEMENT",
      outputNode,
    );
  }
  if (!Number.isFinite(resistor.resistanceOhms) || resistor.resistanceOhms <= 0.0) {
    throw new SpiceError("poleZeroRlcNotch: resistance must be finite and positive", "INVALID_ELEMENT", resistor.name);
  }
  if (!Number.isFinite(inductor.inductanceHenrys) || inductor.inductanceHenrys <= 0.0) {
    throw new SpiceError("poleZeroRlcNotch: inductance must be finite and positive", "INVALID_ELEMENT", inductor.name);
  }
  if (!Number.isFinite(capacitor.capacitanceFarads) || capacitor.capacitanceFarads <= 0.0) {
    throw new SpiceError("poleZeroRlcNotch: capacitance must be finite and positive", "INVALID_ELEMENT", capacitor.name);
  }

  const alpha = resistor.resistanceOhms / (2.0 * inductor.inductanceHenrys);
  const omega0 = 1.0 / Math.sqrt(inductor.inductanceHenrys * capacitor.capacitanceFarads);
  const discriminant = alpha * alpha - omega0 * omega0;
  const entries: PoleZeroEntry[] = [
    {
      kind: "zero",
      real: 0.0,
      imaginary: omega0,
      frequencyHz: omega0 / TWO_PI,
      damping: 0.0,
    },
    {
      kind: "zero",
      real: 0.0,
      imaginary: -omega0,
      frequencyHz: omega0 / TWO_PI,
      damping: 0.0,
    },
  ];
  if (discriminant >= 0.0) {
    const root = Math.sqrt(discriminant);
    const first = -alpha + root;
    const second = -alpha - root;
    entries.push(
      {
        kind: "pole",
        real: first,
        imaginary: 0.0,
        frequencyHz: Math.abs(first) / TWO_PI,
        damping: 1.0,
      },
      {
        kind: "pole",
        real: second,
        imaginary: 0.0,
        frequencyHz: Math.abs(second) / TWO_PI,
        damping: 1.0,
      },
    );
  } else {
    const imaginary = Math.sqrt(-discriminant);
    entries.push(
      {
        kind: "pole",
        real: -alpha,
        imaginary,
        frequencyHz: omega0 / TWO_PI,
        damping: alpha / omega0,
      },
      {
        kind: "pole",
        real: -alpha,
        imaginary: -imaginary,
        frequencyHz: omega0 / TWO_PI,
        damping: alpha / omega0,
      },
    );
  }
  return {
    inputSource,
    outputNode,
    entries,
  };
}

export function distortionFromFourier(
  result: FourierResult,
  inputSource: string,
  outputProbe: string,
): DistortionResult {
  const probe = result.probes.find((candidate) => candidate.probe === outputProbe);
  if (probe === undefined) {
    throw new SpiceError(`distortionFromFourier: missing probe ${JSON.stringify(outputProbe)}`, "INVALID_ELEMENT", outputProbe);
  }
  const fundamental = probe.harmonics[0];
  if (fundamental === undefined) {
    throw new SpiceError("distortionFromFourier: Fourier result has no harmonics", "INVALID_ELEMENT", outputProbe);
  }
  return {
    inputSource,
    outputProbe,
    points: [
      {
        frequencyHz: fundamental.frequencyHz,
        fundamentalMagnitude: fundamental.magnitude,
        harmonics: probe.harmonics.slice(1).map((harmonic) => ({
          harmonic: harmonic.harmonic,
          frequencyHz: harmonic.frequencyHz,
          magnitude: harmonic.magnitude,
          phaseDegrees: harmonic.phaseDegrees,
        })),
        totalHarmonicDistortion: probe.totalHarmonicDistortion,
      },
    ],
  };
}

export function distortionFromTransient(
  points: readonly TransientPoint[],
  fundamentalFrequencyHz: number,
  inputSource: string,
  outputProbe: string,
  harmonics = 9,
  startTime?: number,
): DistortionResult {
  return distortionFromFourier(
    fourier(points, fundamentalFrequencyHz, [outputProbe], harmonics, startTime),
    inputSource,
    outputProbe,
  );
}

export interface AdaptiveTransientOptions {
  readonly method?: TransientMethod;
  readonly tolerance?: number;
  readonly minStep?: number;
  readonly maxStep?: number;
}

export interface AdaptiveTransientResult {
  readonly points: readonly TransientPoint[];
  readonly method: TransientMethod;
  readonly stepsRejected: number;
  readonly converged: boolean;
}

export interface PssResidualEntry {
  readonly kind: "node" | "branch_current";
  readonly name: string;
  readonly value: number;
}

export interface PssResidualResult {
  readonly periodSeconds: number;
  readonly timeStepSeconds: number;
  readonly nodeResiduals: ReadonlyMap<string, number>;
  readonly branchResiduals: ReadonlyMap<string, number>;
  readonly residualVector: readonly PssResidualEntry[];
  readonly maxAbsBranchResidual: number;
  readonly maxAbsResidual: number;
  readonly residualL2Norm: number;
  readonly residualRmsNorm: number;
  readonly residualTolerance: number;
  readonly withinTolerance: boolean;
}

export interface PssStateEntry {
  readonly kind: "capacitor_voltage" | "inductor_current";
  readonly name: string;
  readonly value: number;
}

export interface PssResidualJacobianColumn {
  readonly state: PssStateEntry;
  readonly residualDerivatives: readonly PssResidualEntry[];
}

export interface PssResidualJacobianResult {
  readonly residual: PssResidualResult;
  readonly stateVector: readonly PssStateEntry[];
  readonly perturbation: number;
  readonly columns: readonly PssResidualJacobianColumn[];
  readonly jacobian: readonly (readonly number[])[];
}

export interface PssNewtonUpdateResult {
  readonly jacobian: PssResidualJacobianResult;
  readonly stateUpdates: readonly PssStateEntry[];
  readonly nextStateVector: readonly PssStateEntry[];
  readonly updateL2Norm: number;
}

export interface PssNewtonCandidateResult {
  readonly update: PssNewtonUpdateResult;
  readonly candidateCircuit: Circuit;
  readonly candidateStateVector: readonly PssStateEntry[];
  readonly candidateResidual: PssResidualResult;
}

export interface PssNewtonIterationResult {
  readonly candidate: PssNewtonCandidateResult;
  readonly accepted: boolean;
  readonly residualL2Reduction: number;
  readonly residualL2Ratio: number;
  readonly nextCircuit: Circuit;
  readonly nextStateVector: readonly PssStateEntry[];
  readonly nextResidual: PssResidualResult;
  readonly converged: boolean;
}

export interface PssNewtonSolveResult {
  readonly iterations: readonly PssNewtonIterationResult[];
  readonly finalCircuit: Circuit;
  readonly finalStateVector: readonly PssStateEntry[];
  readonly finalResidual: PssResidualResult;
  readonly converged: boolean;
  readonly iterationCount: number;
}

export interface PssResult {
  readonly solve: PssNewtonSolveResult;
  readonly steadyState: readonly TransientPoint[];
  readonly periodSeconds: number;
  readonly timeStepSeconds: number;
  readonly converged: boolean;
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
  private readonly _subcircuits = new Map<string, SubcircuitDefinition>();

  add(element: Element | XInstance): void {
    if (element.kind === "x-instance") {
      this.instantiate(element);
      return;
    }
    this._elements.push(element);
  }

  elements(): readonly Element[] {
    return this._elements;
  }

  subcircuits(): ReadonlyMap<string, SubcircuitDefinition> {
    return this._subcircuits;
  }

  defineSubcircuit(definition: SubcircuitDefinition): void {
    const key = definition.name.toLowerCase();
    if (this._subcircuits.has(key)) {
      throw new SpiceError(
        `duplicate subcircuit definition ${JSON.stringify(definition.name)}`,
        "INVALID_ELEMENT",
        definition.name,
      );
    }
    this._subcircuits.set(key, definition);
  }

  instantiate(instance: XInstance): void {
    this._elements.push(...expandXInstance(instance, this._subcircuits, []));
  }
}

function isIntegerMultiple(
  candidate: number,
  period: number,
  tolerance: number,
): boolean {
  const ratio = candidate / period;
  const nearest = Math.round(ratio);
  return (
    nearest >= 1 &&
    Math.abs(ratio - nearest) <= tolerance * Math.max(1.0, Math.abs(ratio))
  );
}

export function estimatePeriod(
  circuit: Circuit,
  tolerance = 1.0e-9,
): number | undefined {
  const periods: number[] = [];
  for (const element of circuit.elements()) {
    if (
      (element.kind === "voltage-source" || element.kind === "current-source") &&
      element.waveform !== undefined
    ) {
      const period = waveformPeriod(element.waveform);
      if (period === undefined) {
        return undefined;
      }
      periods.push(period);
    }
  }
  if (periods.length === 0) {
    return undefined;
  }

  const candidate = Math.max(...periods);
  if (!Number.isFinite(candidate) || candidate <= 0.0) {
    return undefined;
  }
  return periods.every((period) => isIntegerMultiple(candidate, period, tolerance))
    ? candidate
    : undefined;
}

function expandXInstance(
  instance: XInstance,
  subcircuits: ReadonlyMap<string, SubcircuitDefinition>,
  stack: readonly string[],
): Element[] {
  const definition = subcircuits.get(instance.subckt.toLowerCase());
  if (definition === undefined) {
    throw new SpiceError(
      `unknown subcircuit ${JSON.stringify(instance.subckt)}`,
      "INVALID_ELEMENT",
      instance.name,
    );
  }
  const definitionKey = definition.name.toLowerCase();
  if (stack.includes(definitionKey)) {
    throw new SpiceError(
      `recursive subcircuit expansion is not supported: ${[...stack, definitionKey].join(" -> ")}`,
      "INVALID_ELEMENT",
      instance.name,
    );
  }
  if (instance.nodes.length !== definition.pins.length) {
    throw new SpiceError(
      `subcircuit ${JSON.stringify(definition.name)} expects ${definition.pins.length} pins, got ${instance.nodes.length}`,
      "INVALID_ELEMENT",
      instance.name,
    );
  }

  const nodeMap = new Map<string, string>();
  for (let index = 0; index < definition.pins.length; index++) {
    nodeMap.set(definition.pins[index], instance.nodes[index]);
    nodeMap.set(definition.pins[index].toLowerCase(), instance.nodes[index]);
  }
  const expanded: Element[] = [];
  const nextStack = [...stack, definitionKey];
  for (const element of definition.elements) {
    if (element.kind === "x-instance") {
      expanded.push(
        ...expandXInstance(
          {
            ...element,
            name: `${instance.name}.${element.name}`,
            nodes: element.nodes.map((node) =>
              mapSubcktNode(node, instance.name, nodeMap),
            ),
          },
          subcircuits,
          nextStack,
        ),
      );
    } else {
      expanded.push(cloneSubcktElement(element, instance.name, nodeMap));
    }
  }
  return expanded;
}

function mapSubcktNode(
  node: string,
  instanceName: string,
  nodeMap: ReadonlyMap<string, string>,
): string {
  if (node.toLowerCase() === "0" || node.toLowerCase() === "gnd") {
    return node;
  }
  return nodeMap.get(node) ?? nodeMap.get(node.toLowerCase()) ?? `${instanceName}.${node}`;
}

function mapSubcktSourceRef(sourceName: string, instanceName: string): string {
  return sourceName.includes(".") ? sourceName : `${instanceName}.${sourceName}`;
}

function mapBSourceExprNodes(
  expr: string | undefined,
  instanceName: string,
  nodeMap: ReadonlyMap<string, string>,
): string | undefined {
  if (expr === undefined) {
    return undefined;
  }
  let result = "";
  let index = 0;
  while (index < expr.length) {
    if (expr[index] === "V" && expr[index + 1] === "(") {
      const close = expr.indexOf(")", index + 2);
      if (close !== -1) {
        const args = expr.slice(index + 2, close).split(",");
        if (args.length >= 1 && args.length <= 2) {
          result += `V(${args
            .map((arg) => mapSubcktNode(arg.trim(), instanceName, nodeMap))
            .join(",")})`;
          index = close + 1;
          continue;
        }
      }
    }
    result += expr[index];
    index++;
  }
  return result;
}

function cloneSubcktElement(
  element: Element,
  instanceName: string,
  nodeMap: ReadonlyMap<string, string>,
): Element {
  const name = `${instanceName}.${element.name}`;
  switch (element.kind) {
    case "resistor":
      return resistor(name, mapSubcktNode(element.n1, instanceName, nodeMap), mapSubcktNode(element.n2, instanceName, nodeMap), element.resistanceOhms);
    case "capacitor":
      return capacitorWithInitialVoltage(name, mapSubcktNode(element.n1, instanceName, nodeMap), mapSubcktNode(element.n2, instanceName, nodeMap), element.capacitanceFarads, element.initialVoltage);
    case "inductor":
      return inductorWithInitialCurrent(name, mapSubcktNode(element.n1, instanceName, nodeMap), mapSubcktNode(element.n2, instanceName, nodeMap), element.inductanceHenrys, element.initialCurrent);
    case "mutual-inductor":
      return mutualInductor(name, mapSubcktSourceRef(element.primary, instanceName), mapSubcktSourceRef(element.secondary, instanceName), element.coupling);
    case "transmission-line":
      return transmissionLine(name, mapSubcktNode(element.n1, instanceName, nodeMap), mapSubcktNode(element.n2, instanceName, nodeMap), mapSubcktNode(element.n3, instanceName, nodeMap), mapSubcktNode(element.n4, instanceName, nodeMap), element.characteristicImpedanceOhms, element.delaySeconds);
    case "voltage-source":
      return { ...element, name, positive: mapSubcktNode(element.positive, instanceName, nodeMap), negative: mapSubcktNode(element.negative, instanceName, nodeMap) };
    case "current-source":
      return { ...element, name, positive: mapSubcktNode(element.positive, instanceName, nodeMap), negative: mapSubcktNode(element.negative, instanceName, nodeMap) };
    case "b-source":
      return { ...element, name, positive: mapSubcktNode(element.positive, instanceName, nodeMap), negative: mapSubcktNode(element.negative, instanceName, nodeMap), voltageExpr: mapBSourceExprNodes(element.voltageExpr, instanceName, nodeMap), currentExpr: mapBSourceExprNodes(element.currentExpr, instanceName, nodeMap) };
    case "diode":
      return diode(name, mapSubcktNode(element.anode, instanceName, nodeMap), mapSubcktNode(element.cathode, instanceName, nodeMap), element.saturationCurrent, element.thermalVoltage, element.emissionCoefficient, element.breakdownVoltage, element.breakdownCurrent, element.junctionCapacitance, element.transitTime);
    case "jfet":
      return jfet(name, mapSubcktNode(element.drain, instanceName, nodeMap), mapSubcktNode(element.gate, instanceName, nodeMap), mapSubcktNode(element.source, instanceName, nodeMap), element.polarity, element.beta, element.thresholdVoltage, element.channelLengthModulation);
    case "bjt":
      return bjt(name, mapSubcktNode(element.collector, instanceName, nodeMap), mapSubcktNode(element.base, instanceName, nodeMap), mapSubcktNode(element.emitter, instanceName, nodeMap), element.polarity, element.saturationCurrent, element.forwardBeta, element.thermalVoltage, element.baseEmitterCapacitance, element.baseCollectorCapacitance, element.forwardTransitTime, element.reverseTransitTime);
    case "mosfet":
      return mosfet(name, mapSubcktNode(element.drain, instanceName, nodeMap), mapSubcktNode(element.gate, instanceName, nodeMap), mapSubcktNode(element.source, instanceName, nodeMap), mapSubcktNode(element.body, instanceName, nodeMap), element.type, element.params);
    case "vccs":
      return vccs(name, mapSubcktNode(element.positive, instanceName, nodeMap), mapSubcktNode(element.negative, instanceName, nodeMap), mapSubcktNode(element.controlPositive, instanceName, nodeMap), mapSubcktNode(element.controlNegative, instanceName, nodeMap), element.transconductanceSiemens);
    case "vcvs":
      return vcvs(name, mapSubcktNode(element.positive, instanceName, nodeMap), mapSubcktNode(element.negative, instanceName, nodeMap), mapSubcktNode(element.controlPositive, instanceName, nodeMap), mapSubcktNode(element.controlNegative, instanceName, nodeMap), element.gain);
    case "cccs":
      return cccs(name, mapSubcktNode(element.positive, instanceName, nodeMap), mapSubcktNode(element.negative, instanceName, nodeMap), mapSubcktSourceRef(element.controlSource, instanceName), element.gain);
    case "ccvs":
      return ccvs(name, mapSubcktNode(element.positive, instanceName, nodeMap), mapSubcktNode(element.negative, instanceName, nodeMap), mapSubcktSourceRef(element.controlSource, instanceName), element.transresistanceOhms);
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

export function mutualInductor(
  name: string,
  primary: string,
  secondary: string,
  coupling: number,
): MutualInductor {
  return {
    kind: "mutual-inductor",
    name,
    primary,
    secondary,
    coupling,
  };
}

export function transmissionLine(
  name: string,
  n1: string,
  n2: string,
  n3: string,
  n4: string,
  characteristicImpedanceOhms: number,
  delaySeconds: number,
): TransmissionLine {
  return {
    kind: "transmission-line",
    name,
    n1,
    n2,
    n3,
    n4,
    characteristicImpedanceOhms,
    delaySeconds,
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

export function subcircuitDefinition(
  name: string,
  pins: readonly string[],
  elements: readonly SubcircuitElement[],
  parameters?: Readonly<Record<string, number>>,
): SubcircuitDefinition {
  return { name, pins, elements, parameters };
}

export function xInstance(
  name: string,
  nodes: readonly string[],
  subckt: string,
  parameters?: Readonly<Record<string, number>>,
): XInstance {
  return { kind: "x-instance", name, nodes, subckt, parameters };
}

export function diode(
  name: string,
  anode: string,
  cathode: string,
  saturationCurrent = 1.0e-15,
  thermalVoltage = 0.02585,
  emissionCoefficient = 1.0,
  breakdownVoltage?: number,
  breakdownCurrent = 1.0e-3,
  junctionCapacitance = 0.0,
  transitTime = 0.0,
): Diode {
  return {
    kind: "diode",
    name,
    anode,
    cathode,
    saturationCurrent,
    thermalVoltage,
    emissionCoefficient,
    breakdownVoltage,
    breakdownCurrent,
    junctionCapacitance,
    transitTime,
  };
}

export function diodeAtTemperature(
  element: Diode,
  temperatureKelvin: number,
  nominalTemperatureKelvin = 300.15,
  energyGapElectronVolts = 1.11,
): Diode {
  if (!Number.isFinite(temperatureKelvin) || temperatureKelvin <= 0.0) {
    throw invalidElement(element.name, "temperature must be finite and positive");
  }
  if (!Number.isFinite(nominalTemperatureKelvin) || nominalTemperatureKelvin <= 0.0) {
    throw invalidElement(element.name, "nominal temperature must be finite and positive");
  }
  if (!Number.isFinite(energyGapElectronVolts) || energyGapElectronVolts <= 0.0) {
    throw invalidElement(element.name, "energy gap must be finite and positive");
  }
  if (!Number.isFinite(element.emissionCoefficient) || element.emissionCoefficient <= 0.0) {
    throw invalidElement(element.name, "emission coefficient must be finite and positive");
  }
  const ratio = temperatureKelvin / nominalTemperatureKelvin;
  const exponent =
    (energyGapElectronVolts * ELECTRON_CHARGE) /
    (element.emissionCoefficient * BOLTZMANN) *
    (1.0 / nominalTemperatureKelvin - 1.0 / temperatureKelvin);
  const saturationScale =
    ratio ** 3 * Math.exp(Math.max(-100.0, Math.min(100.0, exponent)));
  return {
    ...element,
    saturationCurrent: element.saturationCurrent * saturationScale,
    thermalVoltage: element.thermalVoltage * ratio,
  };
}

export function bjtAtTemperature(
  element: Bjt,
  temperatureKelvin: number,
  nominalTemperatureKelvin = 300.15,
  energyGapElectronVolts = 1.11,
): Bjt {
  if (!Number.isFinite(temperatureKelvin) || temperatureKelvin <= 0.0) {
    throw invalidElement(element.name, "temperature must be finite and positive");
  }
  if (!Number.isFinite(nominalTemperatureKelvin) || nominalTemperatureKelvin <= 0.0) {
    throw invalidElement(element.name, "nominal temperature must be finite and positive");
  }
  if (!Number.isFinite(energyGapElectronVolts) || energyGapElectronVolts <= 0.0) {
    throw invalidElement(element.name, "energy gap must be finite and positive");
  }
  const ratio = temperatureKelvin / nominalTemperatureKelvin;
  const exponent =
    (energyGapElectronVolts * ELECTRON_CHARGE) /
    BOLTZMANN *
    (1.0 / nominalTemperatureKelvin - 1.0 / temperatureKelvin);
  const saturationScale =
    ratio ** 3 * Math.exp(Math.max(-100.0, Math.min(100.0, exponent)));
  return {
    ...element,
    saturationCurrent: element.saturationCurrent * saturationScale,
    thermalVoltage: element.thermalVoltage * ratio,
  };
}

export function mosfetAtTemperature(
  element: Mosfet,
  temperatureKelvin: number,
  nominalTemperatureKelvin = 300.15,
): Mosfet {
  if (!Number.isFinite(temperatureKelvin) || temperatureKelvin <= 0.0) {
    throw invalidElement(element.name, "temperature must be finite and positive");
  }
  if (!Number.isFinite(nominalTemperatureKelvin) || nominalTemperatureKelvin <= 0.0) {
    throw invalidElement(element.name, "nominal temperature must be finite and positive");
  }
  const ratio = temperatureKelvin / nominalTemperatureKelvin;
  const thresholdShift = -2.0e-3 * (temperatureKelvin - nominalTemperatureKelvin);
  return {
    ...element,
    params: {
      ...element.params,
      VT0: element.params.VT0 + thresholdShift,
      KP: element.params.KP * ratio ** -1.5,
      T_NOM: temperatureKelvin,
    },
  };
}

export function circuitAtTemperature(
  circuit: Circuit,
  temperatureKelvin: number,
  nominalTemperatureKelvin = 300.15,
  energyGapElectronVolts = 1.11,
): Circuit {
  const adjusted = new Circuit();
  for (const definition of circuit.subcircuits().values()) {
    adjusted.defineSubcircuit(definition);
  }
  for (const element of circuit.elements()) {
    if (element.kind === "diode") {
      adjusted.add(
        diodeAtTemperature(
          element,
          temperatureKelvin,
          nominalTemperatureKelvin,
          energyGapElectronVolts,
        ),
      );
    } else if (element.kind === "bjt") {
      adjusted.add(
        bjtAtTemperature(
          element,
          temperatureKelvin,
          nominalTemperatureKelvin,
          energyGapElectronVolts,
        ),
      );
    } else if (element.kind === "mosfet") {
      adjusted.add(mosfetAtTemperature(element, temperatureKelvin, nominalTemperatureKelvin));
    } else {
      adjusted.add(element);
    }
  }
  return adjusted;
}

export function jfet(
  name: string,
  drain: string,
  gate: string,
  source: string,
  polarity: JfetPolarity = "NJF",
  beta = 1.0e-4,
  thresholdVoltage = polarity === "NJF" ? -2.0 : 2.0,
  channelLengthModulation = 0.0,
): Jfet {
  return {
    kind: "jfet",
    name,
    drain,
    gate,
    source,
    polarity,
    beta,
    thresholdVoltage,
    channelLengthModulation,
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
  baseEmitterCapacitance = 0.0,
  baseCollectorCapacitance = 0.0,
  forwardTransitTime = 0.0,
  reverseTransitTime = 0.0,
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
    baseEmitterCapacitance,
    baseCollectorCapacitance,
    forwardTransitTime,
    reverseTransitTime,
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
    CGSO: 0.0,
    CGDO: 0.0,
    CGBO: 0.0,
    CBS: 0.0,
    CBD: 0.0,
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

export function formatDcTable(
  result: DcResult,
  probes?: readonly string[],
): string {
  const selectedProbes = probes ?? defaultOutputProbes(
    result.nodeVoltages,
    result.branchCurrents,
  );
  const values = selectedProbes.map((probe) =>
    formatTableNumber(
      tableProbeValue(
        result.nodeVoltages,
        result.branchCurrents,
        probe,
        "formatDcTable",
      ),
    ),
  );
  return [
    ["Index", ...selectedProbes].join("\t"),
    ["0", ...values].join("\t"),
    "",
  ].join("\n");
}

export function formatTransientTable(
  points: readonly TransientPoint[],
  probes?: readonly string[],
): string {
  const selectedProbes = probes ?? defaultTransientOutputProbes(points);
  const rows = [["Index", "Time", ...selectedProbes].join("\t")];
  points.forEach((point, index) => {
    const values = selectedProbes.map((probe) =>
      formatTableNumber(
        tableProbeValue(
          point.nodeVoltages,
          point.branchCurrents,
          probe,
          "formatTransientTable",
        ),
      ),
    );
    rows.push([String(index), formatTableNumber(point.time), ...values].join("\t"));
  });
  rows.push("");
  return rows.join("\n");
}

export function formatAcTable(
  points: readonly AcPoint[],
  probes?: readonly string[],
): string {
  const selectedProbes = probes ?? defaultAcOutputProbes(points);
  const rows = [["Index", "Frequency", "Probe", "Real", "Imaginary", "Magnitude", "Phase"].join("\t")];
  points.forEach((point, index) => {
    selectedProbes.forEach((probe) => {
      const value = tableComplexProbeValue(
        point.nodeVoltages,
        point.branchCurrents,
        probe,
        "formatAcTable",
      );
      rows.push(
        [
          String(index),
          formatTableNumber(point.frequencyHz),
          probe,
          formatTableNumber(value.real),
          formatTableNumber(value.imag),
          formatTableNumber(complexAbs(value)),
          formatTableNumber(complexPhase(value) * 180.0 / Math.PI),
        ].join("\t"),
      );
    });
  });
  rows.push("");
  return rows.join("\n");
}

export function formatTfTable(result: TfResult): string {
  return [
    ["TransferRatio", "InputImpedance", "OutputImpedance"].join("\t"),
    [
      formatTableNumber(result.transferRatio),
      formatTableNumber(result.inputImpedanceOhms),
      formatTableNumber(result.outputImpedanceOhms),
    ].join("\t"),
    "",
  ].join("\n");
}

export function formatMcTable(result: McResult): string {
  const rows = [["Trial", "OutputNode", "OutputValue", "Mean", "StdDev", "Converged"].join("\t")];
  result.points.forEach((point) => {
    const outputValue = point.converged
      ? formatTableNumber(point.voltage(result.outputNode) ?? 0.0)
      : "";
    rows.push(
      [
        String(point.trial),
        result.outputNode,
        outputValue,
        formatTableNumber(result.mean),
        formatTableNumber(result.stdDev),
        String(point.converged),
      ].join("\t"),
    );
  });
  rows.push("");
  return rows.join("\n");
}

export function formatCornerMcTable(result: CornerMcResult): string {
  const rows = [["Corner", "Trial", "OutputNode", "OutputValue", "Mean", "StdDev", "Converged"].join("\t")];
  result.points.forEach((corner) => {
    corner.result.points.forEach((point) => {
      const outputValue = point.converged
        ? formatTableNumber(point.voltage(result.outputNode) ?? 0.0)
        : "";
      rows.push(
        [
          corner.cornerName,
          String(point.trial),
          result.outputNode,
          outputValue,
          formatTableNumber(corner.result.mean),
          formatTableNumber(corner.result.stdDev),
          String(point.converged),
        ].join("\t"),
      );
    });
  });
  rows.push("");
  return rows.join("\n");
}

export function formatSensTable(result: SensResult): string {
  const rows = [[
    "OutputNode",
    "NominalVoltage",
    "Element",
    "Parameter",
    "NominalValue",
    "Sensitivity",
    "RelativeSensitivity",
  ].join("\t")];
  result.entries.forEach((entry) => {
    rows.push(
      [
        result.outputNode,
        formatTableNumber(result.nominalVoltage),
        entry.elementName,
        entry.parameter,
        formatTableNumber(entry.nominalValue),
        formatTableNumber(entry.sensitivity),
        formatTableNumber(entry.relativeSensitivity),
      ].join("\t"),
    );
  });
  rows.push("");
  return rows.join("\n");
}

export function formatCornerSensTable(result: CornerSensResult): string {
  const rows = [[
    "Corner",
    "OutputNode",
    "NominalVoltage",
    "Element",
    "Parameter",
    "NominalValue",
    "Sensitivity",
    "RelativeSensitivity",
  ].join("\t")];
  result.points.forEach((corner) => {
    corner.result.entries.forEach((entry) => {
      rows.push(
        [
          corner.cornerName,
          result.outputNode,
          formatTableNumber(corner.result.nominalVoltage),
          entry.elementName,
          entry.parameter,
          formatTableNumber(entry.nominalValue),
          formatTableNumber(entry.sensitivity),
          formatTableNumber(entry.relativeSensitivity),
        ].join("\t"),
      );
    });
  });
  rows.push("");
  return rows.join("\n");
}

function sParameterValues(point: SParameterPoint): readonly (readonly [string, Complex])[] {
  return [
    ["S11", point.s11],
    ["S21", point.s21],
    ["S12", point.s12],
    ["S22", point.s22],
  ];
}

export function formatSParameterTable(result: SParameterResult): string {
  const rows = [[
    "Index",
    "Frequency",
    "Port1",
    "Port2",
    "Parameter",
    "Real",
    "Imaginary",
    "Magnitude",
    "Phase",
  ].join("\t")];
  result.points.forEach((point, index) => {
    sParameterValues(point).forEach(([parameter, value]) => {
      rows.push(
        [
          String(index),
          formatTableNumber(point.frequencyHz),
          result.port1Source,
          result.port2Source,
          parameter,
          formatTableNumber(value.real),
          formatTableNumber(value.imag),
          formatTableNumber(complexAbs(value)),
          formatTableNumber(complexPhase(value) * 180.0 / Math.PI),
        ].join("\t"),
      );
    });
  });
  rows.push("");
  return rows.join("\n");
}

export function formatCornerSParameterTable(result: CornerSParameterResult): string {
  const rows = [[
    "Corner",
    "Index",
    "Frequency",
    "Port1",
    "Port2",
    "Parameter",
    "Real",
    "Imaginary",
    "Magnitude",
    "Phase",
  ].join("\t")];
  result.points.forEach((corner) => {
    corner.result.points.forEach((point, index) => {
      sParameterValues(point).forEach(([parameter, value]) => {
        rows.push(
          [
            corner.cornerName,
            String(index),
            formatTableNumber(point.frequencyHz),
            result.port1Source,
            result.port2Source,
            parameter,
            formatTableNumber(value.real),
            formatTableNumber(value.imag),
            formatTableNumber(complexAbs(value)),
            formatTableNumber(complexPhase(value) * 180.0 / Math.PI),
          ].join("\t"),
        );
      });
    });
  });
  rows.push("");
  return rows.join("\n");
}

export function formatNoiseTable(result: NoiseResult): string {
  const rows = [[
    "Index",
    "Frequency",
    "OutputNode",
    "InputSource",
    "OutputPSD",
    "InputReferredPSD",
    "Element",
    "Type",
    "SourcePSD",
    "ContributionPSD",
  ].join("\t")];
  result.points.forEach((point, index) => {
    if (point.entries.length === 0) {
      rows.push([
        String(index),
        formatTableNumber(point.frequencyHz),
        result.outputNode,
        result.inputSource,
        formatTableNumber(point.outputPsd),
        formatTableNumber(point.inputReferredPsd),
        "",
        "",
        "",
        "",
      ].join("\t"));
      return;
    }
    point.entries.forEach((entry) => {
      rows.push(
        [
          String(index),
          formatTableNumber(point.frequencyHz),
          result.outputNode,
          result.inputSource,
          formatTableNumber(point.outputPsd),
          formatTableNumber(point.inputReferredPsd),
          entry.elementName,
          entry.noiseType,
          formatTableNumber(entry.sourcePsd),
          formatTableNumber(entry.outputPsd),
        ].join("\t"),
      );
    });
  });
  rows.push("");
  return rows.join("\n");
}

export function formatCornerNoiseTable(result: CornerNoiseResult): string {
  const rows = [[
    "Corner",
    "Index",
    "Frequency",
    "OutputNode",
    "InputSource",
    "OutputPSD",
    "InputReferredPSD",
    "Element",
    "Type",
    "SourcePSD",
    "ContributionPSD",
  ].join("\t")];
  result.points.forEach((corner) => {
    corner.result.points.forEach((point, index) => {
      if (point.entries.length === 0) {
        rows.push([
          corner.cornerName,
          String(index),
          formatTableNumber(point.frequencyHz),
          result.outputNode,
          result.inputSource,
          formatTableNumber(point.outputPsd),
          formatTableNumber(point.inputReferredPsd),
          "",
          "",
          "",
          "",
        ].join("\t"));
        return;
      }
      point.entries.forEach((entry) => {
        rows.push(
          [
            corner.cornerName,
            String(index),
            formatTableNumber(point.frequencyHz),
            result.outputNode,
            result.inputSource,
            formatTableNumber(point.outputPsd),
            formatTableNumber(point.inputReferredPsd),
            entry.elementName,
            entry.noiseType,
            formatTableNumber(entry.sourcePsd),
            formatTableNumber(entry.outputPsd),
          ].join("\t"),
        );
      });
    });
  });
  rows.push("");
  return rows.join("\n");
}

export function formatPoleZeroTable(result: PoleZeroResult): string {
  const rows = [["Index", "Kind", "Real", "Imaginary", "Frequency", "Damping"].join("\t")];
  result.entries.forEach((entry, index) => {
    rows.push(
      [
        String(index),
        entry.kind,
        formatTableNumber(entry.real),
        formatTableNumber(entry.imaginary),
        formatTableNumber(entry.frequencyHz),
        formatTableNumber(entry.damping),
      ].join("\t"),
    );
  });
  rows.push("");
  return rows.join("\n");
}

export function formatDistortionTable(result: DistortionResult): string {
  const rows = [["Frequency", "Input", "Output", "Harmonic", "Magnitude", "Phase", "THD"].join("\t")];
  result.points.forEach((point) => {
    point.harmonics.forEach((harmonic) => {
      rows.push(
        [
          formatTableNumber(point.frequencyHz),
          result.inputSource,
          result.outputProbe,
          String(harmonic.harmonic),
          formatTableNumber(harmonic.magnitude),
          formatTableNumber(harmonic.phaseDegrees),
          formatTableNumber(point.totalHarmonicDistortion),
        ].join("\t"),
      );
    });
  });
  rows.push("");
  return rows.join("\n");
}

export function formatFourierTable(result: FourierResult): string {
  const rows = [["Probe", "Harmonic", "Frequency", "Cosine", "Sine", "Magnitude", "Phase", "DC", "THD"].join("\t")];
  result.probes.forEach((probe) => {
    probe.harmonics.forEach((harmonic) => {
      rows.push(
        [
          probe.probe,
          String(harmonic.harmonic),
          formatTableNumber(harmonic.frequencyHz),
          formatTableNumber(harmonic.cosine),
          formatTableNumber(harmonic.sine),
          formatTableNumber(harmonic.magnitude),
          formatTableNumber(harmonic.phaseDegrees),
          formatTableNumber(probe.dc),
          formatTableNumber(probe.totalHarmonicDistortion),
        ].join("\t"),
      );
    });
  });
  rows.push("");
  return rows.join("\n");
}

function defaultOutputProbes(
  nodeVoltages: ReadonlyMap<string, number>,
  branchCurrents: ReadonlyMap<string, number>,
): string[] {
  return [
    ...Array.from(nodeVoltages.keys()).sort().map((name) => `V(${name})`),
    ...Array.from(branchCurrents.keys()).sort(),
  ];
}

function defaultTransientOutputProbes(points: readonly TransientPoint[]): string[] {
  const nodeNames = new Set<string>();
  const branchNames = new Set<string>();
  for (const point of points) {
    for (const name of point.nodeVoltages.keys()) {
      nodeNames.add(name);
    }
    for (const name of point.branchCurrents.keys()) {
      branchNames.add(name);
    }
  }
  return [
    ...Array.from(nodeNames).sort().map((name) => `V(${name})`),
    ...Array.from(branchNames).sort(),
  ];
}

function defaultAcOutputProbes(points: readonly AcPoint[]): string[] {
  const nodeNames = new Set<string>();
  const branchNames = new Set<string>();
  for (const point of points) {
    for (const name of point.nodeVoltages.keys()) {
      nodeNames.add(name);
    }
    for (const name of point.branchCurrents.keys()) {
      branchNames.add(name);
    }
  }
  return [
    ...Array.from(nodeNames).sort().map((name) => `V(${name})`),
    ...Array.from(branchNames).sort(),
  ];
}

function formatTableNumber(value: number): string {
  const [mantissa, exponentText] = value.toExponential(6).split("e");
  const exponent = Number.parseInt(exponentText, 10);
  const sign = exponent < 0 ? "-" : "+";
  const magnitude = Math.abs(exponent).toString().padStart(2, "0");
  return `${mantissa}e${sign}${magnitude}`;
}

function tableProbeValue(
  nodeVoltages: ReadonlyMap<string, number>,
  branchCurrents: ReadonlyMap<string, number>,
  probe: string,
  context: string,
): number {
  const text = probe.trim();
  const lower = text.toLowerCase();
  if (lower.startsWith("v(") && text.endsWith(")")) {
    const args = text.slice(2, -1).split(",").map((arg) => arg.trim());
    if (args.length === 1) {
      return tableVoltage(nodeVoltages, args[0], context);
    }
    if (args.length === 2) {
      return tableVoltage(nodeVoltages, args[0], context) -
        tableVoltage(nodeVoltages, args[1], context);
    }
  }
  if (lower.startsWith("i(") && text.endsWith(")")) {
    const key = `I(${text.slice(2, -1).trim()})`;
    const value = branchCurrents.get(key);
    if (value === undefined) {
      throw invalidElement(context, `missing branch current probe ${probe}`);
    }
    return value;
  }
  if (text.length > 0) {
    return tableVoltage(nodeVoltages, text, context);
  }
  throw invalidElement(context, "empty probe");
}

function tableComplexProbeValue(
  nodeVoltages: ReadonlyMap<string, Complex>,
  branchCurrents: ReadonlyMap<string, Complex>,
  probe: string,
  context: string,
): Complex {
  const text = probe.trim();
  const lower = text.toLowerCase();
  if (lower.startsWith("v(") && text.endsWith(")")) {
    const args = text.slice(2, -1).split(",").map((arg) => arg.trim());
    if (args.length === 1) {
      return tableComplexVoltage(nodeVoltages, args[0], context);
    }
    if (args.length === 2) {
      const positive = tableComplexVoltage(nodeVoltages, args[0], context);
      const negative = tableComplexVoltage(nodeVoltages, args[1], context);
      return { real: positive.real - negative.real, imag: positive.imag - negative.imag };
    }
  }
  if (lower.startsWith("i(") && text.endsWith(")")) {
    const key = `I(${text.slice(2, -1).trim()})`;
    const value = branchCurrents.get(key);
    if (value === undefined) {
      throw invalidElement(context, `missing branch current probe ${probe}`);
    }
    return value;
  }
  if (text.length > 0) {
    return tableComplexVoltage(nodeVoltages, text, context);
  }
  throw invalidElement(context, "empty probe");
}

function tableComplexVoltage(
  nodeVoltages: ReadonlyMap<string, Complex>,
  node: string,
  context: string,
): Complex {
  if (isGround(node)) {
    return { real: 0.0, imag: 0.0 };
  }
  const value = nodeVoltages.get(node);
  if (value === undefined) {
    throw invalidElement(context, `missing node voltage ${node}`);
  }
  return value;
}

function tableVoltage(
  nodeVoltages: ReadonlyMap<string, number>,
  node: string,
  context: string,
): number {
  if (isGround(node)) {
    return 0.0;
  }
  const value = nodeVoltages.get(node);
  if (value === undefined) {
    throw invalidElement(context, `missing node voltage ${node}`);
  }
  return value;
}

export function dcOp(
  circuit: Circuit,
  options: DcOpOptions = {},
): DcResult {
  const solveOptions = validatedDcOpOptions(options);
  const solution = solveDcNewton(circuit, solveOptions);
  if (solution.converged) {
    return makeDcResult(
      solution.nodeVoltages,
      solution.branchCurrents,
      solution.iterations,
      solution.converged,
      "newton",
    );
  }
  if (!solveOptions.convergenceAids) {
    return makeDcResult(
      solution.nodeVoltages,
      solution.branchCurrents,
      solution.iterations,
      false,
      "none",
    );
  }

  const gminSolution = solveDcWithGminStepping(circuit, solveOptions, solution.vector);
  const sourceSolution =
    gminSolution === undefined ? solveDcWithSourceStepping(circuit, solveOptions) : undefined;
  const pseudoSolution =
    gminSolution === undefined && sourceSolution === undefined
      ? solveDcWithPseudoTransient(circuit, solveOptions)
      : undefined;
  const finalSolution = gminSolution ?? sourceSolution ?? pseudoSolution ?? solution;
  const convergenceAid: DcConvergenceAid =
    gminSolution !== undefined
      ? "gmin"
      : sourceSolution !== undefined
        ? "source"
        : pseudoSolution !== undefined
          ? "pseudo_transient"
          : "none";
  return makeDcResult(
    finalSolution.nodeVoltages,
    finalSolution.branchCurrents,
    finalSolution.iterations,
    finalSolution.converged,
    convergenceAid,
  );
}

export function dcCorners(
  circuit: Circuit,
  corners: readonly CornerSpec[],
  options: DcOpOptions = {},
): CornerSweepResult {
  return {
    points: corners.map((corner) => ({
      cornerName: corner.name,
      result: dcOp(circuitWithCorner(circuit, corner), options),
    })),
  };
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

export function dcSweepCorners(
  circuit: Circuit,
  sourceName: string,
  start: number,
  stop: number,
  step: number,
  corners: readonly CornerSpec[],
): CornerDcSweepResult {
  return {
    sourceName,
    points: corners.map((corner) => ({
      cornerName: corner.name,
      points: dcSweep(circuitWithCorner(circuit, corner), sourceName, start, stop, step),
    })),
  };
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

export function mcDcCorners(
  circuit: Circuit,
  outputNode: string,
  corners: readonly CornerSpec[],
  nTrials = 100,
  options: McOptions = {},
): CornerMcResult {
  return {
    outputNode,
    points: corners.map((corner) => ({
      cornerName: corner.name,
      result: mcDc(
        circuitWithCorner(circuit, corner),
        outputNode,
        nTrials,
        options,
      ),
    })),
  };
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
  const operatingPoint = solveDcOperatingPointForSmallSignal(
    circuit,
    nodeIndices,
    voltageSources,
  );
  const matrix = buildSmallSignalMatrix(
    circuit,
    nodeIndices,
    voltageSources,
    operatingPoint,
  );
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

export function tfCorners(
  circuit: Circuit,
  outputNode: string,
  inputSource: string,
  corners: readonly CornerSpec[],
): CornerTfResult {
  return {
    inputSource,
    outputNode,
    points: corners.map((corner) => ({
      cornerName: corner.name,
      result: tf(circuitWithCorner(circuit, corner), outputNode, inputSource),
    })),
  };
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

export function sensDcCorners(
  circuit: Circuit,
  outputNode: string,
  corners: readonly CornerSpec[],
): CornerSensResult {
  return {
    outputNode,
    points: corners.map((corner) => ({
      cornerName: corner.name,
      result: sensDc(circuitWithCorner(circuit, corner), outputNode),
    })),
  };
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

export function acSweepCorners(
  circuit: Circuit,
  startHz: number,
  stopHz: number,
  pointsPerDecade: number,
  corners: readonly CornerSpec[],
): CornerAcSweepResult {
  return {
    points: corners.map((corner) => ({
      cornerName: corner.name,
      points: acSweep(
        circuitWithCorner(circuit, corner),
        startHz,
        stopHz,
        pointsPerDecade,
      ),
    })),
  };
}

export function sParameters(
  circuit: Circuit,
  port1Source: string,
  port2Source: string,
  frequenciesHz: readonly number[],
  referenceImpedanceOhms = 50.0,
): SParameterResult {
  if (!Number.isFinite(referenceImpedanceOhms) || referenceImpedanceOhms <= 0.0) {
    throw invalidElement("sParameters", "reference impedance must be finite and positive");
  }
  for (const frequency of frequenciesHz) {
    if (!Number.isFinite(frequency) || frequency <= 0.0) {
      throw invalidElement("sParameters", "frequencies must be finite and positive");
    }
  }

  const ports = [port1Source, port2Source] as const;
  validateSParameterPorts(circuit, ports);

  const points = frequenciesHz.map((frequencyHz) => {
    const columns = ports.map((drivenSource) => {
      const drivenCircuit = circuitWithSParameterDrive(circuit, ports, drivenSource);
      const point = acSweep(drivenCircuit, frequencyHz, frequencyHz, 1)[0];
      return [
        branchCurrentIntoNetwork(point, port1Source),
        branchCurrentIntoNetwork(point, port2Source),
      ] as const;
    });
    const [s11, s21, s12, s22] = yToS2Port(
      columns[0][0],
      columns[0][1],
      columns[1][0],
      columns[1][1],
      referenceImpedanceOhms,
    );
    return { frequencyHz, s11, s21, s12, s22 };
  });

  return {
    port1Source,
    port2Source,
    referenceImpedanceOhms,
    points,
  };
}

export function sParametersCorners(
  circuit: Circuit,
  port1Source: string,
  port2Source: string,
  frequenciesHz: readonly number[],
  corners: readonly CornerSpec[],
  referenceImpedanceOhms = 50.0,
): CornerSParameterResult {
  return {
    port1Source,
    port2Source,
    referenceImpedanceOhms,
    points: corners.map((corner) => ({
      cornerName: corner.name,
      result: sParameters(
        circuitWithCorner(circuit, corner),
        port1Source,
        port2Source,
        frequenciesHz,
        referenceImpedanceOhms,
      ),
    })),
  };
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
  const operatingPoint = solveDcOperatingPointForSmallSignal(
    circuit,
    nodeIndices,
    voltageSources,
  );
  const noiseSources = collectNoiseSources(
    circuit,
    nodeIndices,
    operatingPoint,
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
      operatingPoint,
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

export function noiseAcCorners(
  circuit: Circuit,
  outputNode: string,
  inputSource: string,
  corners: readonly CornerSpec[],
  frequenciesHz: readonly number[] = defaultNoiseFrequencies(),
  temperatureKelvin = 300.0,
): CornerNoiseResult {
  return {
    outputNode,
    inputSource,
    points: corners.map((corner) => ({
      cornerName: corner.name,
      result: noiseAc(
        circuitWithCorner(circuit, corner),
        outputNode,
        inputSource,
        frequenciesHz,
        temperatureKelvin,
      ),
    })),
  };
}

export function transient(
  circuit: Circuit,
  timeStep: number,
  stopTime: number,
  method: TransientMethod = "euler",
): TransientPoint[] {
  if (!Number.isFinite(timeStep) || timeStep <= 0.0) {
    throw invalidElement("transient", "time step must be finite and positive");
  }
  if (!Number.isFinite(stopTime) || stopTime < 0.0) {
    throw invalidElement("transient", "stop time must be finite and non-negative");
  }
  if (method !== "euler" && method !== "trap" && method !== "gear2") {
    throw invalidElement("transient", "method must be euler, trap, or gear2");
  }

  validateReactiveElements(circuit);

  const capacitorStates = initialCapacitorStates(circuit, timeStep, method);
  const inductorStates = initialInductorStates(circuit, timeStep, method);
  const lineStates = initialTransmissionLineStates(circuit);
  const initialCircuit = circuitWithTransmissionLineCompanions(circuit, lineStates, 0.0);
  const initialSolution = solveLinearCircuit(
    initialCircuit,
    capacitorStates,
    inductorStates,
    0.0,
  );
  updateTransmissionLineStates(circuit, initialSolution.nodeVoltages, lineStates, 0.0);
  const points: TransientPoint[] = [];
  for (let time = timeStep; time <= stopTime + timeStep * 1.0e-9; time += timeStep) {
    const stepMethod = method === "gear2" && points.length === 0 ? "euler" : method;
    setReactiveStateMethod(capacitorStates, inductorStates, stepMethod);
    const companionCircuit = circuitWithTransmissionLineCompanions(circuit, lineStates, time);
    const solution = solveLinearCircuit(
      companionCircuit,
      capacitorStates,
      inductorStates,
      time,
    );
    updateCapacitorStates(circuit, solution.nodeVoltages, capacitorStates);
    updateInductorStates(circuit, solution.nodeVoltages, inductorStates);
    const lineCurrents = updateTransmissionLineStates(
      circuit,
      solution.nodeVoltages,
      lineStates,
      time,
    );
    const branchCurrents = new Map(solution.branchCurrents);
    for (const [name, current] of lineCurrents) {
      branchCurrents.set(name, current);
    }
    points.push(
      makeTransientPoint(time, solution.nodeVoltages, branchCurrents),
    );
  }
  return points;
}

export function fourier(
  points: readonly TransientPoint[],
  fundamentalFrequencyHz: number,
  probes: readonly string[],
  harmonics = 9,
  startTime?: number,
): FourierResult {
  if (!Number.isFinite(fundamentalFrequencyHz) || fundamentalFrequencyHz <= 0.0) {
    throw invalidElement("fourier", "fundamental frequency must be finite and positive");
  }
  if (!Number.isInteger(harmonics) || harmonics < 1) {
    throw invalidElement("fourier", "harmonics must be a positive integer");
  }
  if (probes.length === 0) {
    throw invalidElement("fourier", "at least one probe is required");
  }
  if (points.length < 2) {
    throw invalidElement("fourier", "at least two transient points are required");
  }
  const sortedPoints = [...points].sort((left, right) => left.time - right.time);
  const period = 1.0 / fundamentalFrequencyHz;
  const endTime = sortedPoints[sortedPoints.length - 1].time;
  const windowStart = startTime ?? endTime - period;
  if (!Number.isFinite(windowStart) || windowStart < sortedPoints[0].time) {
    throw invalidElement("fourier", "transient output does not contain a full analysis window");
  }
  if (windowStart >= endTime) {
    throw invalidElement("fourier", "analysis window must have positive duration");
  }

  return {
    fundamentalFrequencyHz,
    startTime: windowStart,
    endTime,
    probes: probes.map((probe) =>
      fourierProbe(sortedPoints, probe, fundamentalFrequencyHz, harmonics, windowStart, endTime),
    ),
  };
}

function fourierProbe(
  points: readonly TransientPoint[],
  probe: string,
  fundamentalFrequencyHz: number,
  harmonics: number,
  startTime: number,
  endTime: number,
): FourierProbeResult {
  const samples: [number, number][] = [
    [startTime, interpolateProbe(points, probe, startTime)],
  ];
  for (const point of points) {
    if (startTime < point.time && point.time < endTime) {
      samples.push([point.time, probeValue(point, probe)]);
    }
  }
  samples.push([endTime, interpolateProbe(points, probe, endTime)]);
  samples.sort((left, right) => left[0] - right[0]);

  const duration = endTime - startTime;
  const dc = integrateSamples(samples, () => 1.0) / duration;
  const omega = 2.0 * Math.PI * fundamentalFrequencyHz;
  const components: FourierHarmonic[] = [];
  for (let harmonic = 1; harmonic <= harmonics; harmonic += 1) {
    const cosine =
      (2.0 / duration) *
      integrateSamples(samples, (time) => Math.cos(harmonic * omega * time));
    const sine =
      (2.0 / duration) *
      integrateSamples(samples, (time) => Math.sin(harmonic * omega * time));
    components.push({
      harmonic,
      frequencyHz: harmonic * fundamentalFrequencyHz,
      cosine,
      sine,
      magnitude: Math.hypot(cosine, sine),
      phaseDegrees: (Math.atan2(cosine, sine) * 180.0) / Math.PI,
    });
  }
  const fundamental = components[0].magnitude;
  const distortion = Math.hypot(...components.slice(1).map((component) => component.magnitude));
  const totalHarmonicDistortion =
    fundamental === 0.0 ? (distortion > 0.0 ? Number.POSITIVE_INFINITY : 0.0) : distortion / fundamental;
  return { probe, dc, harmonics: components, totalHarmonicDistortion };
}

function integrateSamples(
  samples: readonly (readonly [number, number])[],
  weight: (time: number) => number,
): number {
  let total = 0.0;
  for (let index = 1; index < samples.length; index += 1) {
    const [leftTime, leftValue] = samples[index - 1];
    const [rightTime, rightValue] = samples[index];
    total +=
      0.5 *
      (rightTime - leftTime) *
      (leftValue * weight(leftTime) + rightValue * weight(rightTime));
  }
  return total;
}

function interpolateProbe(
  points: readonly TransientPoint[],
  probe: string,
  time: number,
): number {
  for (const point of points) {
    if (Math.abs(point.time - time) <= 1.0e-15) {
      return probeValue(point, probe);
    }
  }
  for (let index = 1; index < points.length; index += 1) {
    const left = points[index - 1];
    const right = points[index];
    if (left.time <= time && time <= right.time) {
      const span = right.time - left.time;
      if (span <= 0.0) {
        return probeValue(left, probe);
      }
      const alpha = (time - left.time) / span;
      return (1.0 - alpha) * probeValue(left, probe) + alpha * probeValue(right, probe);
    }
  }
  throw invalidElement("fourier", "analysis window is outside transient output");
}

function probeValue(point: TransientPoint, probe: string): number {
  const text = probe.trim();
  const lower = text.toLowerCase();
  if (lower.startsWith("v(") && text.endsWith(")")) {
    const args = text.slice(2, -1).split(",").map((arg) => arg.trim());
    if (args.length === 1) {
      return pointVoltage(point, args[0]);
    }
    if (args.length === 2) {
      return pointVoltage(point, args[0]) - pointVoltage(point, args[1]);
    }
  }
  if (lower.startsWith("i(") && text.endsWith(")")) {
    const value = point.branchCurrent(text.slice(2, -1).trim());
    if (value === undefined) {
      throw invalidElement("fourier", `missing branch current probe ${probe}`);
    }
    return value;
  }
  if (text.length > 0) {
    return pointVoltage(point, text);
  }
  throw invalidElement("fourier", "empty probe");
}

function pointVoltage(point: TransientPoint, node: string): number {
  const value = point.voltage(node);
  if (value === undefined) {
    throw invalidElement("fourier", `missing node voltage ${node}`);
  }
  return value;
}

export function transientAdaptive(
  circuit: Circuit,
  timeStep: number,
  stopTime: number,
  options: AdaptiveTransientOptions = {},
): AdaptiveTransientResult {
  if (!Number.isFinite(timeStep) || timeStep <= 0.0) {
    throw invalidElement("transient", "time step must be finite and positive");
  }
  if (!Number.isFinite(stopTime) || stopTime < 0.0) {
    throw invalidElement("transient", "stop time must be finite and non-negative");
  }
  const method = options.method ?? "trap";
  if (method !== "euler" && method !== "trap" && method !== "gear2") {
    throw invalidElement("transient", "method must be euler, trap, or gear2");
  }
  const tolerance = options.tolerance ?? 1.0e-4;
  const minStep = options.minStep ?? timeStep / 1_000.0;
  const maxStep = options.maxStep ?? timeStep * 10.0;
  if (!Number.isFinite(tolerance) || tolerance < 0.0) {
    throw invalidElement("transient", "adaptive tolerance must be finite and non-negative");
  }
  if (!Number.isFinite(minStep) || minStep <= 0.0) {
    throw invalidElement("transient", "minimum step must be finite and positive");
  }
  if (!Number.isFinite(maxStep) || maxStep < minStep) {
    throw invalidElement("transient", "maximum step must be finite and at least the minimum step");
  }

  validateReactiveElements(circuit);

  const capacitorStates = initialCapacitorStates(circuit, timeStep, method);
  const inductorStates = initialInductorStates(circuit, timeStep, method);
  const lineStates = initialTransmissionLineStates(circuit);
  const initialCircuit = circuitWithTransmissionLineCompanions(circuit, lineStates, 0.0);
  const initialSolution = solveLinearCircuit(
    initialCircuit,
    capacitorStates,
    inductorStates,
    0.0,
  );
  updateTransmissionLineStates(circuit, initialSolution.nodeVoltages, lineStates, 0.0);

  const points: TransientPoint[] = [];
  let stepsRejected = 0;
  let currentTime = 0.0;
  let step = Math.min(timeStep, maxStep);
  let previousCapVoltages = capacitorVoltages(circuit, initialSolution.nodeVoltages);
  let previousPreviousCapVoltages = new Map(previousCapVoltages);

  while (currentTime < stopTime - timeStep * 1.0e-12) {
    const remaining = stopTime - currentTime;
    const proposedStep = remaining <= minStep ? remaining : Math.max(minStep, Math.min(step, remaining));
    const proposedTime = currentTime + proposedStep;
    const stepMethod = method === "gear2" && points.length === 0 ? "euler" : method;
    setReactiveStateMethod(capacitorStates, inductorStates, stepMethod);
    setReactiveStateStep(capacitorStates, inductorStates, proposedStep);
    const companionCircuit = circuitWithTransmissionLineCompanions(
      circuit,
      lineStates,
      proposedTime,
    );
    const solution = solveLinearCircuit(
      companionCircuit,
      capacitorStates,
      inductorStates,
      proposedTime,
    );
    const proposedCapVoltages = capacitorVoltages(circuit, solution.nodeVoltages);
    const canEstimateLte = method !== "euler" && points.length >= 1;
    const lte = canEstimateLte
      ? transientLteEstimate(
        circuit,
        proposedCapVoltages,
        previousCapVoltages,
        previousPreviousCapVoltages,
      )
      : 0.0;
    if (canEstimateLte && lte > tolerance && proposedStep > minStep + 1.0e-20) {
      step = Math.max(proposedStep / 2.0, minStep);
      stepsRejected += 1;
      continue;
    }

    updateCapacitorStates(circuit, solution.nodeVoltages, capacitorStates);
    updateInductorStates(circuit, solution.nodeVoltages, inductorStates);
    const lineCurrents = updateTransmissionLineStates(
      circuit,
      solution.nodeVoltages,
      lineStates,
      proposedTime,
    );
    const branchCurrents = new Map(solution.branchCurrents);
    for (const [name, current] of lineCurrents) {
      branchCurrents.set(name, current);
    }
    points.push(makeTransientPoint(proposedTime, solution.nodeVoltages, branchCurrents));
    currentTime = proposedTime;
    previousPreviousCapVoltages = previousCapVoltages;
    previousCapVoltages = proposedCapVoltages;
    step = canEstimateLte && lte < tolerance / 8.0
      ? Math.min(proposedStep * 2.0, maxStep)
      : proposedStep;
  }

  return { points, method, stepsRejected, converged: true };
}

export function pssResidual(
  circuit: Circuit,
  stepsPerPeriod = 64,
  residualTolerance = 1.0e-6,
): PssResidualResult | undefined {
  const period = estimatePeriod(circuit);
  if (period === undefined) {
    return undefined;
  }
  if (!Number.isInteger(stepsPerPeriod) || stepsPerPeriod <= 0) {
    throw invalidElement(
      "pssResidual",
      "steps per period must be a positive integer",
    );
  }
  if (!Number.isFinite(residualTolerance) || residualTolerance < 0.0) {
    throw invalidElement(
      "pssResidual",
      "residual tolerance must be finite and non-negative",
    );
  }

  const timeStep = period / stepsPerPeriod;
  validateReactiveElements(circuit);
  const initialSolution = solveLinearCircuit(
    circuit,
    initialCapacitorStates(circuit, timeStep, "euler"),
    initialInductorStates(circuit, timeStep, "euler"),
    0.0,
  );
  const points = transient(circuit, timeStep, period);
  if (points.length === 0) {
    return {
      periodSeconds: period,
      timeStepSeconds: timeStep,
      nodeResiduals: new Map(),
      branchResiduals: new Map(),
      residualVector: [],
      maxAbsBranchResidual: 0.0,
      maxAbsResidual: 0.0,
      residualL2Norm: 0.0,
      residualRmsNorm: 0.0,
      residualTolerance,
      withinTolerance: false,
    };
  }

  const last = points[points.length - 1];
  const nodes = new Set<string>([
    ...initialSolution.nodeVoltages.keys(),
    ...last.nodeVoltages.keys(),
  ]);
  const nodeResiduals = new Map<string, number>();
  const residualVector: PssResidualEntry[] = [];
  let maxAbsResidual = 0.0;
  for (const node of [...nodes].sort()) {
    const residual =
      (last.nodeVoltages.get(node) ?? 0.0) -
      (initialSolution.nodeVoltages.get(node) ?? 0.0);
    nodeResiduals.set(node, residual);
    residualVector.push({ kind: "node", name: node, value: residual });
    maxAbsResidual = Math.max(maxAbsResidual, Math.abs(residual));
  }
  const branches = new Set<string>([
    ...initialSolution.branchCurrents.keys(),
    ...last.branchCurrents.keys(),
  ]);
  const branchResiduals = new Map<string, number>();
  let maxAbsBranchResidual = 0.0;
  for (const branch of [...branches].sort()) {
    const residual =
      (last.branchCurrents.get(branch) ?? 0.0) -
      (initialSolution.branchCurrents.get(branch) ?? 0.0);
    branchResiduals.set(branch, residual);
    residualVector.push({
      kind: "branch_current",
      name: branch,
      value: residual,
    });
    maxAbsBranchResidual = Math.max(maxAbsBranchResidual, Math.abs(residual));
  }
  maxAbsResidual = Math.max(maxAbsResidual, maxAbsBranchResidual);
  const residualL2Norm = Math.sqrt(
    residualVector.reduce((sum, entry) => sum + entry.value * entry.value, 0.0),
  );
  const residualRmsNorm =
    residualVector.length > 0
      ? residualL2Norm / Math.sqrt(residualVector.length)
      : 0.0;
  return {
    periodSeconds: period,
    timeStepSeconds: timeStep,
    nodeResiduals,
    branchResiduals,
    residualVector,
    maxAbsBranchResidual,
    maxAbsResidual,
    residualL2Norm,
    residualRmsNorm,
    residualTolerance,
    withinTolerance: maxAbsResidual <= residualTolerance,
  };
}

function pssStateVector(circuit: Circuit): PssStateEntry[] {
  const stateVector: PssStateEntry[] = [];
  for (const element of circuit.elements()) {
    if (element.kind === "capacitor") {
      stateVector.push({
        kind: "capacitor_voltage",
        name: element.name,
        value: element.initialVoltage,
      });
    } else if (element.kind === "inductor") {
      stateVector.push({
        kind: "inductor_current",
        name: element.name,
        value: element.initialCurrent,
      });
    }
  }
  return stateVector;
}

function withPerturbedPssState(
  circuit: Circuit,
  target: PssStateEntry,
  perturbation: number,
): Circuit {
  const perturbed = new Circuit();
  for (const element of circuit.elements()) {
    if (
      target.kind === "capacitor_voltage" &&
      element.kind === "capacitor" &&
      element.name === target.name
    ) {
      perturbed.add({
        ...element,
        initialVoltage: element.initialVoltage + perturbation,
      });
    } else if (
      target.kind === "inductor_current" &&
      element.kind === "inductor" &&
      element.name === target.name
    ) {
      perturbed.add({
        ...element,
        initialCurrent: element.initialCurrent + perturbation,
      });
    } else {
      perturbed.add(element);
    }
  }
  return perturbed;
}

function withPssStateVector(
  circuit: Circuit,
  stateVector: readonly PssStateEntry[],
): Circuit {
  const targetByKey = new Map(
    stateVector.map((state) => [`${state.kind}\u0000${state.name}`, state.value]),
  );
  const candidate = new Circuit();
  for (const element of circuit.elements()) {
    if (element.kind === "capacitor") {
      const value = targetByKey.get(`capacitor_voltage\u0000${element.name}`);
      candidate.add(
        value === undefined ? element : { ...element, initialVoltage: value },
      );
    } else if (element.kind === "inductor") {
      const value = targetByKey.get(`inductor_current\u0000${element.name}`);
      candidate.add(
        value === undefined ? element : { ...element, initialCurrent: value },
      );
    } else {
      candidate.add(element);
    }
  }
  return candidate;
}

export function pssResidualJacobian(
  circuit: Circuit,
  stepsPerPeriod = 64,
  residualTolerance = 1.0e-6,
  perturbation = 1.0e-6,
): PssResidualJacobianResult | undefined {
  if (!Number.isFinite(perturbation) || perturbation <= 0.0) {
    throw invalidElement(
      "pssResidualJacobian",
      "perturbation must be finite and positive",
    );
  }

  const residual = pssResidual(circuit, stepsPerPeriod, residualTolerance);
  if (residual === undefined) {
    return undefined;
  }

  const stateVector = pssStateVector(circuit);
  const columns: PssResidualJacobianColumn[] = [];
  for (const state of stateVector) {
    const perturbed = pssResidual(
      withPerturbedPssState(circuit, state, perturbation),
      stepsPerPeriod,
      residualTolerance,
    );
    if (perturbed === undefined) {
      throw invalidElement(
        "pssResidualJacobian",
        "perturbed circuit no longer has an estimated period",
      );
    }
    if (perturbed.residualVector.length !== residual.residualVector.length) {
      throw invalidElement(
        "pssResidualJacobian",
        "perturbed residual vector changed shape",
      );
    }
    const residualDerivatives = residual.residualVector.map((baseEntry, index) => {
      const perturbedEntry = perturbed.residualVector[index];
      if (
        perturbedEntry.kind !== baseEntry.kind ||
        perturbedEntry.name !== baseEntry.name
      ) {
        throw invalidElement(
          "pssResidualJacobian",
          "perturbed residual vector changed ordering",
        );
      }
      return {
        kind: baseEntry.kind,
        name: baseEntry.name,
        value: (perturbedEntry.value - baseEntry.value) / perturbation,
      };
    });
    columns.push({ state, residualDerivatives });
  }

  const jacobian = residual.residualVector.map((_entry, rowIndex) =>
    columns.map((column) => column.residualDerivatives[rowIndex].value),
  );
  return {
    residual,
    stateVector,
    perturbation,
    columns,
    jacobian,
  };
}

function solvePssNormalEquations(jacobian: PssResidualJacobianResult): number[] {
  const columnCount = jacobian.stateVector.length;
  if (columnCount === 0) {
    return [];
  }

  const normalMatrix = Array.from({ length: columnCount }, () =>
    Array.from({ length: columnCount }, () => 0.0),
  );
  const normalRhs = Array.from({ length: columnCount }, () => 0.0);
  for (const [rowIndex, row] of jacobian.jacobian.entries()) {
    const residualValue = jacobian.residual.residualVector[rowIndex].value;
    for (let col = 0; col < columnCount; col++) {
      normalRhs[col] -= row[col] * residualValue;
      for (let otherCol = 0; otherCol < columnCount; otherCol++) {
        normalMatrix[col][otherCol] += row[col] * row[otherCol];
      }
    }
  }
  return solveLinearSystem(normalMatrix, normalRhs);
}

export function pssNewtonUpdate(
  circuit: Circuit,
  stepsPerPeriod = 64,
  residualTolerance = 1.0e-6,
  perturbation = 1.0e-6,
): PssNewtonUpdateResult | undefined {
  const jacobian = pssResidualJacobian(
    circuit,
    stepsPerPeriod,
    residualTolerance,
    perturbation,
  );
  if (jacobian === undefined) {
    return undefined;
  }

  const updateValues = solvePssNormalEquations(jacobian);
  const stateUpdates = jacobian.stateVector.map((state, index) => ({
    kind: state.kind,
    name: state.name,
    value: updateValues[index],
  }));
  const nextStateVector = jacobian.stateVector.map((state, index) => ({
    kind: state.kind,
    name: state.name,
    value: state.value + updateValues[index],
  }));
  const updateL2Norm = Math.sqrt(
    updateValues.reduce((sum, value) => sum + value * value, 0.0),
  );
  return {
    jacobian,
    stateUpdates,
    nextStateVector,
    updateL2Norm,
  };
}

export function pssNewtonCandidate(
  circuit: Circuit,
  stepsPerPeriod = 64,
  residualTolerance = 1.0e-6,
  perturbation = 1.0e-6,
): PssNewtonCandidateResult | undefined {
  const update = pssNewtonUpdate(
    circuit,
    stepsPerPeriod,
    residualTolerance,
    perturbation,
  );
  if (update === undefined) {
    return undefined;
  }

  const candidateCircuit = withPssStateVector(circuit, update.nextStateVector);
  const candidateResidual = pssResidual(
    candidateCircuit,
    stepsPerPeriod,
    residualTolerance,
  );
  if (candidateResidual === undefined) {
    throw invalidElement(
      "pssNewtonCandidate",
      "candidate circuit no longer has an estimated period",
    );
  }

  return {
    update,
    candidateCircuit,
    candidateStateVector: pssStateVector(candidateCircuit),
    candidateResidual,
  };
}

export function pssNewtonIteration(
  circuit: Circuit,
  stepsPerPeriod = 64,
  residualTolerance = 1.0e-6,
  perturbation = 1.0e-6,
): PssNewtonIterationResult | undefined {
  const candidate = pssNewtonCandidate(
    circuit,
    stepsPerPeriod,
    residualTolerance,
    perturbation,
  );
  if (candidate === undefined) {
    return undefined;
  }

  const baseResidual = candidate.update.jacobian.residual;
  const candidateResidual = candidate.candidateResidual;
  const baseNorm = baseResidual.residualL2Norm;
  const candidateNorm = candidateResidual.residualL2Norm;
  const accepted = candidateNorm <= baseNorm;
  const nextResidual = accepted ? candidateResidual : baseResidual;

  return {
    candidate,
    accepted,
    residualL2Reduction: baseNorm - candidateNorm,
    residualL2Ratio: baseNorm > 0.0 ? candidateNorm / baseNorm : 0.0,
    nextCircuit: accepted ? candidate.candidateCircuit : circuit,
    nextStateVector: accepted
      ? candidate.candidateStateVector
      : candidate.update.jacobian.stateVector,
    nextResidual,
    converged: nextResidual.withinTolerance,
  };
}

export function pssNewtonSolve(
  circuit: Circuit,
  stepsPerPeriod = 64,
  residualTolerance = 1.0e-6,
  perturbation = 1.0e-6,
  maxNewtonIterations = 8,
): PssNewtonSolveResult | undefined {
  if (!Number.isInteger(maxNewtonIterations) || maxNewtonIterations <= 0) {
    throw invalidElement(
      "pssNewtonSolve",
      "max Newton iterations must be a positive integer",
    );
  }

  let currentCircuit = circuit;
  const iterations: PssNewtonIterationResult[] = [];
  for (let index = 0; index < maxNewtonIterations; index++) {
    const iteration = pssNewtonIteration(
      currentCircuit,
      stepsPerPeriod,
      residualTolerance,
      perturbation,
    );
    if (iteration === undefined) {
      return undefined;
    }

    iterations.push(iteration);
    currentCircuit = iteration.nextCircuit;
    if (iteration.converged || !iteration.accepted) {
      break;
    }
  }

  const finalIteration = iterations[iterations.length - 1];
  return {
    iterations,
    finalCircuit: finalIteration.nextCircuit,
    finalStateVector: finalIteration.nextStateVector,
    finalResidual: finalIteration.nextResidual,
    converged: finalIteration.nextResidual.withinTolerance,
    iterationCount: iterations.length,
  };
}

export function pss(
  circuit: Circuit,
  stepsPerPeriod = 64,
  residualTolerance = 1.0e-6,
  perturbation = 1.0e-6,
  maxNewtonIterations = 8,
): PssResult | undefined {
  const solve = pssNewtonSolve(
    circuit,
    stepsPerPeriod,
    residualTolerance,
    perturbation,
    maxNewtonIterations,
  );
  if (solve === undefined) {
    return undefined;
  }

  const steadyState = transient(
    solve.finalCircuit,
    solve.finalResidual.timeStepSeconds,
    solve.finalResidual.periodSeconds,
  );
  return {
    solve,
    steadyState,
    periodSeconds: solve.finalResidual.periodSeconds,
    timeStepSeconds: solve.finalResidual.timeStepSeconds,
    converged: solve.converged,
  };
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
    case "jfet":
      return {
        elementName: element.name,
        parameter: "beta",
        nominalValue: element.beta,
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
    case "mutual-inductor":
    case "transmission-line":
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
      case "jfet":
        perturbed.add({ ...element, beta: element.beta + delta });
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
      case "mutual-inductor":
      case "transmission-line":
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
    case "jfet":
      return {
        ...element,
        beta: randomizedValue(element.beta, tolerance, distribution, rng),
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
    case "mutual-inductor":
    case "transmission-line":
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
  previousPreviousVoltage: number;
  previousCurrent: number;
  timeStep: number;
  method: TransientMethod;
}

interface InductorState {
  readonly name: string;
  previousCurrent: number;
  previousPreviousCurrent: number;
  previousVoltage: number;
  timeStep: number;
  method: TransientMethod;
}

interface TransmissionLineState {
  readonly name: string;
  samples: Array<{
    readonly time: number;
    readonly port1Voltage: number;
    readonly port1Current: number;
    readonly port2Voltage: number;
    readonly port2Current: number;
  }>;
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
  readonly pseudoTransientSteps: number;
  readonly pseudoTransientConductance: number;
  readonly pseudoTransientMaxIterations: number;
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

function solveDcOperatingPointForSmallSignal(
  circuit: Circuit,
  nodeIndices: ReadonlyMap<string, number>,
  voltageSources: ReadonlyMap<string, number>,
): readonly number[] {
  const matrixSize = nodeIndices.size + voltageSources.size;
  if (matrixSize === 0 || !circuit.elements().some(isNonlinearElement)) {
    return Array.from({ length: matrixSize }, () => 0.0);
  }

  const options = validatedDcOpOptions({});
  const solution = solveDcNewton(circuit, options);
  if (solution.converged || !options.convergenceAids) {
    return solution.vector;
  }

  const aided =
    solveDcWithGminStepping(circuit, options, solution.vector) ??
    solveDcWithSourceStepping(circuit, options);
  return (aided ?? solution).vector;
}

function isNonlinearElement(element: Element): boolean {
  return (
    element.kind === "diode" ||
    element.kind === "jfet" ||
    element.kind === "bjt" ||
    element.kind === "mosfet" ||
    element.kind === "b-source"
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

  const hasNonlinearElement = circuit.elements().some(isNonlinearElement);
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
  const pseudoTransientSteps = options.pseudoTransientSteps ?? 20;
  const pseudoTransientConductance = options.pseudoTransientConductance ?? 1.0e-3;
  const pseudoTransientMaxIterations = options.pseudoTransientMaxIterations ?? maxIterations;
  if (!Number.isInteger(maxIterations) || maxIterations < 1) {
    throw invalidElement("dcOp", "maxIterations must be a positive integer");
  }
  if (!Number.isFinite(tolerance) || tolerance <= 0.0) {
    throw invalidElement("dcOp", "tolerance must be finite and positive");
  }
  if (!Number.isInteger(pseudoTransientSteps) || pseudoTransientSteps < 0) {
    throw invalidElement("dcOp", "pseudoTransientSteps must be a non-negative integer");
  }
  if (!Number.isFinite(pseudoTransientConductance) || pseudoTransientConductance <= 0.0) {
    throw invalidElement("dcOp", "pseudoTransientConductance must be finite and positive");
  }
  if (!Number.isInteger(pseudoTransientMaxIterations) || pseudoTransientMaxIterations < 1) {
    throw invalidElement("dcOp", "pseudoTransientMaxIterations must be a positive integer");
  }
  return {
    maxIterations,
    tolerance,
    convergenceAids: options.convergenceAids ?? true,
    pseudoTransientSteps,
    pseudoTransientConductance,
    pseudoTransientMaxIterations,
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

function solveDcWithPseudoTransient(
  circuit: Circuit,
  options: ResolvedDcOpOptions,
): LinearSolution | undefined {
  if (options.pseudoTransientSteps === 0) {
    return undefined;
  }

  const nodeIndices = collectNodeIndices(circuit);
  const nodesByIndex = Array.from(nodeIndices.entries()).sort(
    ([, left], [, right]) => left - right,
  );
  if (nodesByIndex.length === 0) {
    return undefined;
  }

  let previousNodeVoltages = new Map<string, number>(
    nodesByIndex.map(([node]) => [node, 0.0]),
  );
  let warmStart: readonly number[] | undefined;
  let lastSolution: LinearSolution | undefined;
  const pseudoOptions: ResolvedDcOpOptions = {
    ...options,
    maxIterations: options.pseudoTransientMaxIterations,
  };

  for (let step = 0; step < options.pseudoTransientSteps; step++) {
    const pseudoCircuit = circuitWithPseudoTransientCompanions(
      circuit,
      nodesByIndex.map(([node]) => node),
      previousNodeVoltages,
      options.pseudoTransientConductance,
      step,
    );
    const solution = solveDcNewton(pseudoCircuit, pseudoOptions, warmStart);
    if (!solution.converged) {
      return undefined;
    }

    let delta = 0.0;
    const nextNodeVoltages = new Map<string, number>();
    for (const [node] of nodesByIndex) {
      const next = solution.nodeVoltages.get(node) ?? 0.0;
      delta = Math.max(delta, Math.abs(next - (previousNodeVoltages.get(node) ?? 0.0)));
      nextNodeVoltages.set(node, next);
    }
    previousNodeVoltages = nextNodeVoltages;
    warmStart = solution.vector;
    lastSolution = solution;
    if (delta < options.tolerance) {
      break;
    }
  }

  if (lastSolution === undefined) {
    return undefined;
  }

  const finalSolution = solveDcNewton(circuit, pseudoOptions, warmStart);
  return finalSolution.converged ? finalSolution : undefined;
}

function circuitWithPseudoTransientCompanions(
  circuit: Circuit,
  nodes: readonly string[],
  previousNodeVoltages: ReadonlyMap<string, number>,
  conductance: number,
  step: number,
): Circuit {
  const pseudoCircuit = circuitFromElements(circuit.elements());
  for (const node of nodes) {
    pseudoCircuit.add(resistor(`__ptran_g_${step}_${node}`, node, "0", 1.0 / conductance));
    const historyCurrent = conductance * (previousNodeVoltages.get(node) ?? 0.0);
    if (historyCurrent !== 0.0) {
      pseudoCircuit.add(currentSource(`__ptran_i_${step}_${node}`, "0", node, historyCurrent));
    }
  }
  return pseudoCircuit;
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

function circuitWithCorner(circuit: Circuit, corner: CornerSpec): Circuit {
  const overridesByName = new Map<string, CornerOverride[]>();
  for (const override of corner.overrides) {
    const existing = overridesByName.get(override.elementName) ?? [];
    existing.push(override);
    overridesByName.set(override.elementName, existing);
  }

  const seen = new Set<string>();
  const elements = circuit.elements().map((element) => {
    const overrides = overridesByName.get(element.name);
    if (overrides === undefined) {
      return element;
    }
    seen.add(element.name);
    return overrides.reduce((current, override) => applyCornerOverride(current, override), element);
  });

  for (const elementName of overridesByName.keys()) {
    if (!seen.has(elementName)) {
      throw invalidElement("dcCorners", `missing element for corner override ${elementName}`);
    }
  }

  return circuitFromElements(elements);
}

function applyCornerOverride(element: Element, override: CornerOverride): Element {
  if (!Number.isFinite(override.value)) {
    throw invalidElement("dcCorners", "override values must be finite");
  }
  switch (element.kind) {
    case "resistor":
      if (override.parameter === "resistance") {
        if (override.value <= 0.0) {
          throw invalidElement("dcCorners", "resistance overrides must be positive");
        }
        return { ...element, resistanceOhms: override.value };
      }
      break;
    case "capacitor":
      if (override.parameter === "capacitance") {
        if (override.value <= 0.0) {
          throw invalidElement("dcCorners", "capacitance overrides must be positive");
        }
        return { ...element, capacitanceFarads: override.value };
      }
      break;
    case "inductor":
      if (override.parameter === "inductance") {
        if (override.value <= 0.0) {
          throw invalidElement("dcCorners", "inductance overrides must be positive");
        }
        return { ...element, inductanceHenrys: override.value };
      }
      break;
    case "voltage-source":
      if (override.parameter === "voltage") {
        return { ...element, voltage: override.value };
      }
      break;
    case "current-source":
      if (override.parameter === "current") {
        return { ...element, current: override.value };
      }
      break;
  }
  throw invalidElement(
    "dcCorners",
    `unsupported override ${override.elementName}.${override.parameter}`,
  );
}

function validateSParameterPorts(
  circuit: Circuit,
  ports: readonly [string, string],
): void {
  for (const port of ports) {
    const element = circuit.elements().find(
      (candidate) => candidate.kind === "voltage-source" && candidate.name === port,
    );
    if (element === undefined) {
      throw invalidElement("sParameters", `missing voltage-source port ${JSON.stringify(port)}`);
    }
  }
}

function circuitWithSParameterDrive(
  circuit: Circuit,
  ports: readonly [string, string],
  drivenSource: string,
): Circuit {
  const portNames = new Set<string>(ports);
  return circuitFromElements(
    circuit.elements().map((element) => {
      if (element.kind !== "voltage-source" || !portNames.has(element.name)) {
        return element;
      }
      return {
        ...element,
        ac: acSource(element.name === drivenSource ? 1.0 : 0.0),
      };
    }),
  );
}

function branchCurrentIntoNetwork(point: AcPoint, sourceName: string): Complex {
  const current = point.branchCurrent(sourceName);
  if (current === undefined) {
    throw invalidElement("sParameters", `missing branch current for ${JSON.stringify(sourceName)}`);
  }
  return complexScale(current, -1.0);
}

function yToS2Port(
  y11: Complex,
  y21: Complex,
  y12: Complex,
  y22: Complex,
  z0: number,
): [Complex, Complex, Complex, Complex] {
  const a11 = complexSub(complex(1.0, 0.0), complexScale(y11, z0));
  const a12 = complexScale(y12, -z0);
  const a21 = complexScale(y21, -z0);
  const a22 = complexSub(complex(1.0, 0.0), complexScale(y22, z0));

  const b11 = complexAdd(complex(1.0, 0.0), complexScale(y11, z0));
  const b12 = complexScale(y12, z0);
  const b21 = complexScale(y21, z0);
  const b22 = complexAdd(complex(1.0, 0.0), complexScale(y22, z0));
  const det = complexSub(complexMul(b11, b22), complexMul(b12, b21));
  if (complexAbs(det) < 1.0e-18) {
    throw invalidElement("sParameters", "singular Y-to-S conversion");
  }

  const invB11 = complexDiv(b22, det);
  const invB12 = complexDiv(complexScale(b12, -1.0), det);
  const invB21 = complexDiv(complexScale(b21, -1.0), det);
  const invB22 = complexDiv(b11, det);

  return [
    complexAdd(complexMul(a11, invB11), complexMul(a12, invB21)),
    complexAdd(complexMul(a21, invB11), complexMul(a22, invB21)),
    complexAdd(complexMul(a11, invB12), complexMul(a12, invB22)),
    complexAdd(complexMul(a21, invB12), complexMul(a22, invB22)),
  ];
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
  const inductors = inductorByName(circuit);
  const coupledNames = coupledInductorNames(circuit);
  const hasTransientInductorStates = inductorStates.length > 0;

  for (const element of circuit.elements()) {
    switch (element.kind) {
      case "resistor":
        stampResistor(element, nodeIndices, matrix);
        break;
      case "capacitor":
        stampCapacitor(element, capacitorStates, nodeIndices, matrix, rhs);
        break;
      case "inductor":
        if (!hasTransientInductorStates || !coupledNames.has(element.name)) {
          stampInductor(
            element,
            inductorStates,
            nodeIndices,
            voltageSources,
            nodeCount,
            matrix,
            rhs,
          );
        }
        break;
      case "mutual-inductor":
        if (hasTransientInductorStates) {
          stampTransientMutualInductor(
            element,
            inductors,
            inductorStates,
            nodeIndices,
            matrix,
            rhs,
          );
        }
        break;
      case "transmission-line":
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
      case "jfet":
        stampJfet(element, nodeIndices, matrix, rhs, operatingPoint);
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
  operatingPoint: readonly number[],
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
        const diodeVoltage = vectorVoltage(operatingPoint, nodeIndex(nodeIndices, element.anode)) -
          vectorVoltage(operatingPoint, nodeIndex(nodeIndices, element.cathode));
        const [, diodeConductance] = diodeCurrentConductance(element, diodeVoltage);
        stampConductance(
          matrix,
          nodeIndex(nodeIndices, element.anode),
          nodeIndex(nodeIndices, element.cathode),
          diodeConductance,
        );
        break;
      case "jfet":
        stampJfetSmallSignal(element, nodeIndices, matrix, operatingPoint);
        break;
      case "bjt":
        stampBjtSmallSignal(element, nodeIndices, matrix);
        break;
      case "mosfet":
        stampMosfetSmallSignal(element, nodeIndices, matrix, operatingPoint);
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

  const operatingPoint = solveDcOperatingPointForSmallSignal(
    circuit,
    nodeIndices,
    voltageSources,
  );
  const matrix = buildAcMatrix(
    circuit,
    omega,
    nodeIndices,
    voltageSources,
    operatingPoint,
  );
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
      case "mutual-inductor":
      case "transmission-line":
      case "diode":
      case "jfet":
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
  operatingPoint: readonly number[],
): Complex[][] {
  const nodeCount = nodeIndices.size;
  const matrixSize = nodeCount + voltageSources.size;
  const matrix = Array.from({ length: matrixSize }, () =>
    Array.from({ length: matrixSize }, () => complex(0.0, 0.0)),
  );
  const inductors = inductorByName(circuit);
  const coupledNames = coupledInductorNames(circuit);

  for (const element of circuit.elements()) {
    switch (element.kind) {
      case "resistor":
        stampAcResistor(element, nodeIndices, matrix);
        break;
      case "capacitor":
        stampAcCapacitor(element, omega, nodeIndices, matrix);
        break;
      case "inductor":
        if (coupledNames.has(element.name)) {
          break;
        }
        stampAcInductor(element, omega, nodeIndices, matrix);
        break;
      case "mutual-inductor":
        stampAcMutualInductor(element, inductors, omega, nodeIndices, matrix);
        break;
      case "transmission-line":
        stampAcTransmissionLine(element, omega, nodeIndices, matrix);
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
        const diodeVoltage = vectorVoltage(operatingPoint, nodeIndex(nodeIndices, element.anode)) -
          vectorVoltage(operatingPoint, nodeIndex(nodeIndices, element.cathode));
        const [, diodeConductance] = diodeCurrentConductance(element, diodeVoltage);
        const diffusionCapacitance = element.transitTime * diodeConductance;
        stampComplexConductance(
          matrix,
          nodeIndex(nodeIndices, element.anode),
          nodeIndex(nodeIndices, element.cathode),
          complex(diodeConductance, omega * (element.junctionCapacitance + diffusionCapacitance)),
        );
        break;
      case "jfet":
        stampAcJfetSmallSignal(element, nodeIndices, matrix, operatingPoint);
        break;
      case "bjt":
        stampAcBjtSmallSignal(element, nodeIndices, matrix, omega);
        break;
      case "mosfet":
        stampAcMosfetSmallSignal(element, nodeIndices, matrix, operatingPoint, omega);
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
  convergenceAid: DcConvergenceAid = converged ? "newton" : "none",
): DcResult {
  return {
    nodeVoltages,
    branchCurrents,
    iterations,
    converged,
    convergenceAid,
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
      case "transmission-line":
        insertNode(names, element.n1);
        insertNode(names, element.n2);
        insertNode(names, element.n3);
        insertNode(names, element.n4);
        break;
      case "mutual-inductor":
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
      case "jfet":
        insertNode(names, element.drain);
        insertNode(names, element.gate);
        insertNode(names, element.source);
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
        element.kind === "jfet" ||
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
  operatingPoint: readonly number[],
  temperatureKelvin: number,
): NoiseSource[] {
  const sources: NoiseSource[] = [];
  for (const element of circuit.elements()) {
    if (element.kind === "resistor") {
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
    } else if (element.kind === "mosfet") {
      validateMosfet(element);
      const drain = nodeIndex(nodeIndices, element.drain);
      const gate = nodeIndex(nodeIndices, element.gate);
      const source = nodeIndex(nodeIndices, element.source);
      const body = nodeIndex(nodeIndices, element.body);
      const drainVoltage = vectorVoltage(operatingPoint, drain);
      const gateVoltage = vectorVoltage(operatingPoint, gate);
      const sourceVoltage = vectorVoltage(operatingPoint, source);
      const bodyVoltage = vectorVoltage(operatingPoint, body);
      const result = evaluateMosfetLevel1(
        element,
        gateVoltage - sourceVoltage,
        drainVoltage - sourceVoltage,
        bodyVoltage - sourceVoltage,
      );
      const gm = Math.max(0.0, result.gm);
      if (gm > 0.0) {
        sources.push({
          elementName: element.name,
          noiseType: "thermal",
          positive: drain,
          negative: source,
          sourcePsd: 4.0 * BOLTZMANN * temperatureKelvin * MOSFET_CHANNEL_NOISE_GAMMA * gm,
        });
      }
    }
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

  const conductance =
    state.method === "trap"
      ? (2.0 * element.capacitanceFarads) / state.timeStep
      : state.method === "gear2"
        ? (3.0 * element.capacitanceFarads) / (2.0 * state.timeStep)
        : element.capacitanceFarads / state.timeStep;
  const n1 = nodeIndex(nodeIndices, element.n1);
  const n2 = nodeIndex(nodeIndices, element.n2);
  stampConductance(matrix, n1, n2, conductance);

  const historyCurrent =
    state.method === "trap"
      ? conductance * state.previousVoltage + state.previousCurrent
      : state.method === "gear2"
      ? element.capacitanceFarads *
        (4.0 * state.previousVoltage - state.previousPreviousVoltage) /
        (2.0 * state.timeStep)
      : conductance * state.previousVoltage;
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

  const conductance =
    state.method === "trap"
      ? state.timeStep / (2.0 * element.inductanceHenrys)
      : state.method === "gear2"
        ? (2.0 * state.timeStep) / (3.0 * element.inductanceHenrys)
        : state.timeStep / element.inductanceHenrys;
  stampConductance(matrix, n1, n2, conductance);
  const historyCurrent =
    state.method === "trap"
      ? state.previousCurrent + conductance * state.previousVoltage
      : state.method === "gear2"
      ? (4.0 * state.previousCurrent - state.previousPreviousCurrent) / 3.0
      : state.previousCurrent;
  if (n1 !== undefined) {
    rhs[n1] -= historyCurrent;
  }
  if (n2 !== undefined) {
    rhs[n2] += historyCurrent;
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
  const [current, conductance] = diodeCurrentConductance(element, voltage);
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
  readonly cgs: number;
  readonly cgd: number;
  readonly cgb: number;
  readonly cbs: number;
  readonly cbd: number;
}

interface JfetDcResult {
  readonly drainCurrent: number;
  readonly gm: number;
  readonly gds: number;
}

function validateJfet(element: Jfet): void {
  if (!Number.isFinite(element.beta) || element.beta <= 0.0) {
    throw invalidElement(element.name, "beta must be finite and positive");
  }
  if (!Number.isFinite(element.thresholdVoltage)) {
    throw invalidElement(element.name, "threshold voltage must be finite");
  }
  if (!Number.isFinite(element.channelLengthModulation)) {
    throw invalidElement(element.name, "channel length modulation must be finite");
  }
}

function stampJfet(
  element: Jfet,
  nodeIndices: ReadonlyMap<string, number>,
  matrix: number[][],
  rhs: number[],
  operatingPoint: readonly number[],
): void {
  validateJfet(element);
  const drain = nodeIndex(nodeIndices, element.drain);
  const gate = nodeIndex(nodeIndices, element.gate);
  const source = nodeIndex(nodeIndices, element.source);
  const drainVoltage = vectorVoltage(operatingPoint, drain);
  const gateVoltage = vectorVoltage(operatingPoint, gate);
  const sourceVoltage = vectorVoltage(operatingPoint, source);
  const vgs = gateVoltage - sourceVoltage;
  const vds = drainVoltage - sourceVoltage;
  const result = evaluateJfet(element, vgs, vds);
  const equivalentCurrent =
    result.drainCurrent - result.gm * vgs - result.gds * vds;

  stampConductance(matrix, drain, source, result.gds);
  stampTransconductance(matrix, drain, source, gate, source, result.gm);
  stampCurrentSourceEquivalent(rhs, drain, source, equivalentCurrent);
}

function evaluateJfet(element: Jfet, vgs: number, vds: number): JfetDcResult {
  if (element.polarity === "PJF") {
    const result = evaluateNjf(
      -vgs,
      -vds,
      -element.thresholdVoltage,
      element.beta,
      element.channelLengthModulation,
    );
    return {
      drainCurrent: -result.drainCurrent,
      gm: result.gm,
      gds: result.gds,
    };
  }
  return evaluateNjf(
    vgs,
    vds,
    element.thresholdVoltage,
    element.beta,
    element.channelLengthModulation,
  );
}

function evaluateNjf(
  vgs: number,
  vds: number,
  thresholdVoltage: number,
  beta: number,
  channelLengthModulation: number,
): JfetDcResult {
  const overdrive = vgs - thresholdVoltage;
  if (overdrive <= 0.0 || vds < 0.0) {
    return { drainCurrent: 0.0, gm: 0.0, gds: 0.0 };
  }
  if (vds < overdrive) {
    const channel = 2.0 * overdrive * vds - vds * vds;
    const modulation = 1.0 + channelLengthModulation * vds;
    return {
      drainCurrent: beta * channel * modulation,
      gm: 2.0 * beta * vds * modulation,
      gds:
        beta * (2.0 * overdrive - 2.0 * vds) * modulation +
        beta * channel * channelLengthModulation,
    };
  }
  return {
    drainCurrent:
      beta * overdrive * overdrive * (1.0 + channelLengthModulation * vds),
    gm: 2.0 * beta * overdrive * (1.0 + channelLengthModulation * vds),
    gds: beta * overdrive * overdrive * channelLengthModulation,
  };
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
      cgs: result.cgs,
      cgd: result.cgd,
      cgb: result.cgb,
      cbs: result.cbs,
      cbd: result.cbd,
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
  const cgsOverlap = params.CGSO * params.W;
  const cgdOverlap = params.CGDO * params.W;
  const cgbOverlap = params.CGBO * params.L;
  const cgsIntrinsic = (2.0 / 3.0) * params.W * params.L * params.KP;
  const capacitances = {
    cgs: cgsOverlap + cgsIntrinsic,
    cgd: cgdOverlap,
    cgb: cgbOverlap,
    cbs: params.CBS,
    cbd: params.CBD,
  };
  const threshold =
    params.PHI - vbs >= 0.0
      ? params.VT0 + params.GAMMA * (Math.sqrt(params.PHI - vbs) - Math.sqrt(params.PHI))
      : params.VT0;
  const overdrive = vgs - threshold;
  if (overdrive <= 0.0) {
    return { drainCurrent: 0.0, gm: 0.0, gds: 0.0, gmb: 0.0, ...capacitances };
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
      cgs: cgsOverlap + cgsIntrinsic / 2.0,
      cgd: cgdOverlap,
      cgb: cgbOverlap,
      cbs: params.CBS,
      cbd: params.CBD,
    };
  }
  const current = 0.5 * beta * overdrive * overdrive * (1.0 + params.LAMBDA * vds);
  const gm = beta * overdrive * (1.0 + params.LAMBDA * vds);
  return {
    drainCurrent: current,
    gm,
    gds: 0.5 * beta * overdrive * overdrive * params.LAMBDA,
    gmb: gm * bodyFactor,
    cgs: cgsOverlap + (2.0 / 3.0) * cgsIntrinsic,
    cgd: cgdOverlap,
    cgb: cgbOverlap,
    cbs: params.CBS,
    cbd: params.CBD,
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
    } else if (element.kind === "transmission-line") {
      validateTransmissionLine(element);
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
  if (!Number.isFinite(element.emissionCoefficient) || element.emissionCoefficient <= 0.0) {
    throw invalidElement(element.name, "emission coefficient must be finite and positive");
  }
  if (
    element.breakdownVoltage !== undefined &&
    (!Number.isFinite(element.breakdownVoltage) || element.breakdownVoltage <= 0.0)
  ) {
    throw invalidElement(element.name, "breakdown voltage must be finite and positive");
  }
  if (!Number.isFinite(element.breakdownCurrent) || element.breakdownCurrent <= 0.0) {
    throw invalidElement(element.name, "breakdown current must be finite and positive");
  }
  if (!Number.isFinite(element.junctionCapacitance) || element.junctionCapacitance < 0.0) {
    throw invalidElement(element.name, "junction capacitance must be finite and non-negative");
  }
  if (!Number.isFinite(element.transitTime) || element.transitTime < 0.0) {
    throw invalidElement(element.name, "transit time must be finite and non-negative");
  }
}

function diodeEffectiveThermalVoltage(element: Diode): number {
  return element.thermalVoltage * element.emissionCoefficient;
}

function diodeCurrentConductance(element: Diode, voltage: number): [number, number] {
  const vtEff = diodeEffectiveThermalVoltage(element);
  const forwardVoltage = Math.min(voltage, 0.7 * element.emissionCoefficient);
  const exponent = Math.max(-40.0, Math.min(40.0, forwardVoltage / vtEff));
  const expValue = Math.exp(exponent);
  let current = element.saturationCurrent * (expValue - 1.0);
  let conductance = element.saturationCurrent / vtEff * expValue;
  if (element.breakdownVoltage !== undefined && voltage <= -element.breakdownVoltage) {
    const breakdownExponent = Math.max(
      -40.0,
      Math.min(40.0, (-voltage - element.breakdownVoltage) / vtEff),
    );
    const breakdownExpValue = Math.exp(breakdownExponent);
    current -= element.breakdownCurrent * breakdownExpValue;
    conductance += element.breakdownCurrent / vtEff * breakdownExpValue;
  }
  return [current, conductance];
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
  if (!Number.isFinite(element.baseEmitterCapacitance) || element.baseEmitterCapacitance < 0.0) {
    throw invalidElement(element.name, "base-emitter capacitance must be finite and non-negative");
  }
  if (!Number.isFinite(element.baseCollectorCapacitance) || element.baseCollectorCapacitance < 0.0) {
    throw invalidElement(element.name, "base-collector capacitance must be finite and non-negative");
  }
  if (!Number.isFinite(element.forwardTransitTime) || element.forwardTransitTime < 0.0) {
    throw invalidElement(element.name, "forward transit time must be finite and non-negative");
  }
  if (!Number.isFinite(element.reverseTransitTime) || element.reverseTransitTime < 0.0) {
    throw invalidElement(element.name, "reverse transit time must be finite and non-negative");
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
  if (
    params.CGSO < 0.0 ||
    params.CGDO < 0.0 ||
    params.CGBO < 0.0 ||
    params.CBS < 0.0 ||
    params.CBD < 0.0
  ) {
    throw invalidElement(element.name, "MOSFET capacitances must be non-negative");
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
  method: TransientMethod,
): CapacitorState[] {
  const states: CapacitorState[] = [];
  for (const element of circuit.elements()) {
    if (element.kind === "capacitor") {
      states.push({
        name: element.name,
        previousVoltage: element.initialVoltage,
        previousPreviousVoltage: element.initialVoltage,
        previousCurrent: 0.0,
        timeStep,
        method,
      });
    }
  }
  return states;
}

function initialInductorStates(
  circuit: Circuit,
  timeStep: number,
  method: TransientMethod,
): InductorState[] {
  const states: InductorState[] = [];
  for (const element of circuit.elements()) {
    if (element.kind === "inductor") {
      states.push({
        name: element.name,
        previousCurrent: element.initialCurrent,
        previousPreviousCurrent: element.initialCurrent,
        previousVoltage: 0.0,
        timeStep,
        method,
      });
    }
  }
  return states;
}

function setReactiveStateMethod(
  capacitorStates: CapacitorState[],
  inductorStates: InductorState[],
  method: TransientMethod,
): void {
  for (const state of capacitorStates) {
    state.method = method;
  }
  for (const state of inductorStates) {
    state.method = method;
  }
}

function setReactiveStateStep(
  capacitorStates: CapacitorState[],
  inductorStates: InductorState[],
  timeStep: number,
): void {
  for (const state of capacitorStates) {
    state.timeStep = timeStep;
  }
  for (const state of inductorStates) {
    state.timeStep = timeStep;
  }
}

function capacitorVoltages(
  circuit: Circuit,
  nodeVoltages: ReadonlyMap<string, number>,
): Map<string, number> {
  const voltages = new Map<string, number>();
  for (const element of circuit.elements()) {
    if (element.kind === "capacitor") {
      voltages.set(
        element.name,
        voltageAt(nodeVoltages, element.n1) - voltageAt(nodeVoltages, element.n2),
      );
    }
  }
  return voltages;
}

function transientLteEstimate(
  circuit: Circuit,
  currentVoltages: ReadonlyMap<string, number>,
  previousVoltages: ReadonlyMap<string, number>,
  previousPreviousVoltages: ReadonlyMap<string, number>,
): number {
  let maxLte = 0.0;
  for (const element of circuit.elements()) {
    if (element.kind !== "capacitor") {
      continue;
    }
    const current = currentVoltages.get(element.name) ?? element.initialVoltage;
    const previous = previousVoltages.get(element.name) ?? element.initialVoltage;
    const previousPrevious =
      previousPreviousVoltages.get(element.name) ?? element.initialVoltage;
    maxLte = Math.max(maxLte, Math.abs(current - 2.0 * previous + previousPrevious) / 2.0);
  }
  return maxLte;
}

function initialTransmissionLineStates(circuit: Circuit): TransmissionLineState[] {
  return circuit
    .elements()
    .filter((element): element is TransmissionLine => element.kind === "transmission-line")
    .map((element) => ({ name: element.name, samples: [] }));
}

function validateTransmissionLine(element: TransmissionLine): void {
  if (!Number.isFinite(element.characteristicImpedanceOhms)) {
    throw invalidElement(element.name, "characteristic impedance must be finite");
  }
  if (element.characteristicImpedanceOhms <= 0.0) {
    throw invalidElement(element.name, "characteristic impedance must be positive");
  }
  if (!Number.isFinite(element.delaySeconds)) {
    throw invalidElement(element.name, "delay must be finite");
  }
  if (element.delaySeconds <= 0.0) {
    throw invalidElement(element.name, "delay must be positive");
  }
}

function transmissionLineStateAt(
  state: TransmissionLineState | undefined,
  targetTime: number,
): {
  readonly port1Voltage: number;
  readonly port1Current: number;
  readonly port2Voltage: number;
  readonly port2Current: number;
} {
  const samples = state?.samples ?? [];
  if (samples.length === 0 || targetTime < samples[0].time - 1.0e-18) {
    return { port1Voltage: 0.0, port1Current: 0.0, port2Voltage: 0.0, port2Current: 0.0 };
  }
  if (targetTime <= samples[0].time) {
    return samples[0];
  }
  for (let index = 0; index + 1 < samples.length; index += 1) {
    const left = samples[index];
    const right = samples[index + 1];
    if (targetTime <= right.time) {
      const span = right.time - left.time;
      if (span <= 0.0) {
        return right;
      }
      const alpha = (targetTime - left.time) / span;
      return {
        port1Voltage: left.port1Voltage + alpha * (right.port1Voltage - left.port1Voltage),
        port1Current: left.port1Current + alpha * (right.port1Current - left.port1Current),
        port2Voltage: left.port2Voltage + alpha * (right.port2Voltage - left.port2Voltage),
        port2Current: left.port2Current + alpha * (right.port2Current - left.port2Current),
      };
    }
  }
  return samples[samples.length - 1];
}

function transmissionLineHistoryTerms(
  element: TransmissionLine,
  lineStates: readonly TransmissionLineState[],
  time: number,
): readonly [number, number] {
  validateTransmissionLine(element);
  const delayed = transmissionLineStateAt(
    lineStates.find((state) => state.name === element.name),
    time - element.delaySeconds,
  );
  return [
    delayed.port2Voltage / element.characteristicImpedanceOhms + delayed.port2Current,
    delayed.port1Voltage / element.characteristicImpedanceOhms + delayed.port1Current,
  ];
}

function circuitWithTransmissionLineCompanions(
  circuit: Circuit,
  lineStates: readonly TransmissionLineState[],
  time: number,
): Circuit {
  const companion = new Circuit();
  for (const element of circuit.elements()) {
    if (element.kind !== "transmission-line") {
      companion.add(element);
      continue;
    }
    const [history1, history2] = transmissionLineHistoryTerms(element, lineStates, time);
    companion.add(resistor(`_T_${element.name}_P1_R`, element.n1, element.n2, element.characteristicImpedanceOhms));
    companion.add(resistor(`_T_${element.name}_P2_R`, element.n3, element.n4, element.characteristicImpedanceOhms));
    companion.add(currentSource(`_T_${element.name}_P1_I`, element.n1, element.n2, -history1));
    companion.add(currentSource(`_T_${element.name}_P2_I`, element.n3, element.n4, -history2));
  }
  return companion;
}

function transmissionLinePortVoltage(
  element: TransmissionLine,
  nodeVoltages: ReadonlyMap<string, number>,
  firstPort: boolean,
): number {
  if (firstPort) {
    return voltageAt(nodeVoltages, element.n1) - voltageAt(nodeVoltages, element.n2);
  }
  return voltageAt(nodeVoltages, element.n3) - voltageAt(nodeVoltages, element.n4);
}

function updateTransmissionLineStates(
  circuit: Circuit,
  nodeVoltages: ReadonlyMap<string, number>,
  lineStates: TransmissionLineState[],
  time: number,
): Map<string, number> {
  const currents = new Map<string, number>();
  for (const element of circuit.elements()) {
    if (element.kind !== "transmission-line") {
      continue;
    }
    const [history1, history2] = transmissionLineHistoryTerms(element, lineStates, time);
    const port1Voltage = transmissionLinePortVoltage(element, nodeVoltages, true);
    const port2Voltage = transmissionLinePortVoltage(element, nodeVoltages, false);
    const port1Current = port1Voltage / element.characteristicImpedanceOhms - history1;
    const port2Current = port2Voltage / element.characteristicImpedanceOhms - history2;
    currents.set(`I(${element.name}:1)`, port1Current);
    currents.set(`I(${element.name}:2)`, port2Current);
    const state = lineStates.find((candidate) => candidate.name === element.name);
    if (state !== undefined) {
      state.samples.push({ time, port1Voltage, port1Current, port2Voltage, port2Current });
    }
  }
  return currents;
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
    const previousVoltage = state.previousVoltage;
    const previousCurrent = state.previousCurrent;
    const voltage =
      voltageAt(nodeVoltages, element.n1) - voltageAt(nodeVoltages, element.n2);
    if (state.method === "trap") {
      const conductance = (2.0 * element.capacitanceFarads) / state.timeStep;
      state.previousCurrent = conductance * (voltage - previousVoltage) - previousCurrent;
    } else if (state.method === "gear2") {
      state.previousCurrent =
        element.capacitanceFarads *
        (3.0 * voltage - 4.0 * previousVoltage + state.previousPreviousVoltage) /
        (2.0 * state.timeStep);
    } else {
      state.previousCurrent =
        (element.capacitanceFarads / state.timeStep) * (voltage - previousVoltage);
    }
    state.previousVoltage = voltage;
    state.previousPreviousVoltage = previousVoltage;
  }
}

function updateInductorStates(
  circuit: Circuit,
  nodeVoltages: ReadonlyMap<string, number>,
  inductorStates: InductorState[],
): void {
  const inductors = inductorByName(circuit);
  const coupledCurrents = coupledTransientInductorCurrents(
    circuit,
    inductors,
    inductorStates,
    nodeVoltages,
  );
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
    const previousCurrent = state.previousCurrent;
    const voltage = voltageAt(nodeVoltages, element.n1) - voltageAt(nodeVoltages, element.n2);
    state.previousCurrent =
      coupledCurrents.get(element.name) ?? inductorCurrent(element, state, nodeVoltages);
    state.previousPreviousCurrent = previousCurrent;
    state.previousVoltage = voltage;
  }
}

function insertTransientInductorCurrents(
  circuit: Circuit,
  inductorStates: readonly InductorState[],
  nodeVoltages: ReadonlyMap<string, number>,
  branchCurrents: Map<string, number>,
): void {
  const inductors = inductorByName(circuit);
  const coupledCurrents = coupledTransientInductorCurrents(
    circuit,
    inductors,
    inductorStates,
    nodeVoltages,
  );
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
      coupledCurrents.get(element.name) ?? inductorCurrent(element, state, nodeVoltages),
    );
  }
}

function inductorCurrent(
  element: Inductor,
  state: InductorState,
  nodeVoltages: ReadonlyMap<string, number>,
): number {
  const voltage = voltageAt(nodeVoltages, element.n1) - voltageAt(nodeVoltages, element.n2);
  if (state.method === "trap") {
    const conductance = state.timeStep / (2.0 * element.inductanceHenrys);
    return state.previousCurrent + conductance * state.previousVoltage + conductance * voltage;
  }
  if (state.method === "gear2") {
    return (
      (2.0 * state.timeStep * voltage) / (3.0 * element.inductanceHenrys) +
      (4.0 * state.previousCurrent - state.previousPreviousCurrent) / 3.0
    );
  }
  return state.previousCurrent + (state.timeStep / element.inductanceHenrys) * voltage;
}

function stampTransientMutualInductor(
  element: MutualInductor,
  inductors: ReadonlyMap<string, Inductor>,
  inductorStates: readonly InductorState[],
  nodeIndices: ReadonlyMap<string, number>,
  matrix: number[][],
  rhs: number[],
): void {
  const { primary, secondary, mutualInductance } = validateMutualInductor(element, inductors);
  const primaryState = inductorStates.find((state) => state.name === primary.name);
  const secondaryState = inductorStates.find((state) => state.name === secondary.name);
  if (primaryState === undefined || secondaryState === undefined) {
    return;
  }
  const { g11, g12, g22 } = transientMutualConductances(
    element,
    primary,
    secondary,
    mutualInductance,
    primaryState.timeStep,
    primaryState.method,
  );
  const p1 = nodeIndex(nodeIndices, primary.n1);
  const p2 = nodeIndex(nodeIndices, primary.n2);
  const s1 = nodeIndex(nodeIndices, secondary.n1);
  const s2 = nodeIndex(nodeIndices, secondary.n2);
  stampConductance(matrix, p1, p2, g11);
  stampConductance(matrix, s1, s2, g22);
  stampTransconductance(matrix, p1, p2, s1, s2, g12);
  stampTransconductance(matrix, s1, s2, p1, p2, g12);
  let primaryHistoryCurrent = primaryState.previousCurrent;
  let secondaryHistoryCurrent = secondaryState.previousCurrent;
  if (primaryState.method === "trap") {
    primaryHistoryCurrent +=
      g11 * primaryState.previousVoltage + g12 * secondaryState.previousVoltage;
    secondaryHistoryCurrent +=
      g12 * primaryState.previousVoltage + g22 * secondaryState.previousVoltage;
  }
  stampCurrentSourceEquivalent(rhs, p1, p2, primaryHistoryCurrent);
  stampCurrentSourceEquivalent(rhs, s1, s2, secondaryHistoryCurrent);
}

function coupledTransientInductorCurrents(
  circuit: Circuit,
  inductors: ReadonlyMap<string, Inductor>,
  inductorStates: readonly InductorState[],
  nodeVoltages: ReadonlyMap<string, number>,
): Map<string, number> {
  const currents = new Map<string, number>();
  for (const element of circuit.elements()) {
    if (element.kind !== "mutual-inductor") {
      continue;
    }
    const { primary, secondary, mutualInductance } = validateMutualInductor(element, inductors);
    const primaryState = inductorStates.find((state) => state.name === primary.name);
    const secondaryState = inductorStates.find((state) => state.name === secondary.name);
    if (primaryState === undefined || secondaryState === undefined) {
      continue;
    }
    const { g11, g12, g22 } = transientMutualConductances(
      element,
      primary,
      secondary,
      mutualInductance,
      primaryState.timeStep,
      primaryState.method,
    );
    const primaryVoltage = voltageAt(nodeVoltages, primary.n1) - voltageAt(nodeVoltages, primary.n2);
    const secondaryVoltage = voltageAt(nodeVoltages, secondary.n1) - voltageAt(nodeVoltages, secondary.n2);
    let primaryHistoryCurrent = primaryState.previousCurrent;
    let secondaryHistoryCurrent = secondaryState.previousCurrent;
    if (primaryState.method === "trap") {
      primaryHistoryCurrent +=
        g11 * primaryState.previousVoltage + g12 * secondaryState.previousVoltage;
      secondaryHistoryCurrent +=
        g12 * primaryState.previousVoltage + g22 * secondaryState.previousVoltage;
    }
    currents.set(
      primary.name,
      primaryHistoryCurrent + g11 * primaryVoltage + g12 * secondaryVoltage,
    );
    currents.set(
      secondary.name,
      secondaryHistoryCurrent + g12 * primaryVoltage + g22 * secondaryVoltage,
    );
  }
  return currents;
}

function transientMutualConductances(
  element: MutualInductor,
  primary: Inductor,
  secondary: Inductor,
  mutualInductance: number,
  timeStep: number,
  method: TransientMethod,
): { g11: number; g12: number; g22: number } {
  const determinant = primary.inductanceHenrys * secondary.inductanceHenrys - mutualInductance ** 2;
  if (!Number.isFinite(determinant) || determinant <= 0.0) {
    throw invalidElement(element.name, "coupled inductance matrix is singular");
  }
  const scale = method === "trap" ? timeStep / (2.0 * determinant) : timeStep / determinant;
  return {
    g11: secondary.inductanceHenrys * scale,
    g12: -mutualInductance * scale,
    g22: primary.inductanceHenrys * scale,
  };
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
  let index = 0;
  while (index < expression.length) {
    if (expression[index] !== "V") {
      index += 1;
      continue;
    }

    let cursor = index + 1;
    while (/\s/.test(expression[cursor] ?? "")) {
      cursor += 1;
    }
    if (expression[cursor] !== "(") {
      index += 1;
      continue;
    }

    const argsStart = cursor + 1;
    let argsEnd = argsStart;
    while (argsEnd < expression.length && expression[argsEnd] !== ")") {
      argsEnd += 1;
    }
    if (argsEnd >= expression.length) {
      break;
    }

    for (const node of expression.slice(argsStart, argsEnd).split(",")) {
      const trimmed = node.trim();
      if (trimmed !== "" && !isGround(trimmed)) {
        nodes.push(trimmed);
      }
    }
    index = argsEnd + 1;
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
  const result = evaluateMosfetLevel1(
    element,
    gateVoltage - sourceVoltage,
    drainVoltage - sourceVoltage,
    bodyVoltage - sourceVoltage,
  );
  stampConductance(matrix, drain, source, result.gds);
  stampTransconductance(matrix, drain, source, gate, source, result.gm);
  stampTransconductance(matrix, drain, source, body, source, result.gmb);
}

function stampJfetSmallSignal(
  element: Jfet,
  nodeIndices: ReadonlyMap<string, number>,
  matrix: number[][],
  operatingPoint: readonly number[],
): void {
  validateJfet(element);
  const drain = nodeIndex(nodeIndices, element.drain);
  const gate = nodeIndex(nodeIndices, element.gate);
  const source = nodeIndex(nodeIndices, element.source);
  const drainVoltage = vectorVoltage(operatingPoint, drain);
  const gateVoltage = vectorVoltage(operatingPoint, gate);
  const sourceVoltage = vectorVoltage(operatingPoint, source);
  const result = evaluateJfet(
    element,
    gateVoltage - sourceVoltage,
    drainVoltage - sourceVoltage,
  );
  stampConductance(matrix, drain, source, result.gds);
  stampTransconductance(matrix, drain, source, gate, source, result.gm);
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

function inductorByName(circuit: Circuit): Map<string, Inductor> {
  const inductors = new Map<string, Inductor>();
  for (const element of circuit.elements()) {
    if (element.kind === "inductor") {
      inductors.set(element.name, element);
    }
  }
  return inductors;
}

function coupledInductorNames(circuit: Circuit): Set<string> {
  const names = new Set<string>();
  for (const element of circuit.elements()) {
    if (element.kind === "mutual-inductor") {
      names.add(element.primary);
      names.add(element.secondary);
    }
  }
  return names;
}

function validateMutualInductor(
  element: MutualInductor,
  inductors: ReadonlyMap<string, Inductor>,
): { primary: Inductor; secondary: Inductor; mutualInductance: number } {
  if (!Number.isFinite(element.coupling)) {
    throw invalidElement(element.name, "coupling must be finite");
  }
  if (Math.abs(element.coupling) >= 1.0) {
    throw invalidElement(element.name, "coupling magnitude must be less than one");
  }
  if (element.primary === element.secondary) {
    throw invalidElement(element.name, "coupled inductors must be distinct");
  }
  const primary = inductors.get(element.primary);
  if (primary === undefined) {
    throw invalidElement(element.name, `referenced inductor ${JSON.stringify(element.primary)} was not found`);
  }
  const secondary = inductors.get(element.secondary);
  if (secondary === undefined) {
    throw invalidElement(element.name, `referenced inductor ${JSON.stringify(element.secondary)} was not found`);
  }
  validateInductor(primary);
  validateInductor(secondary);
  return {
    primary,
    secondary,
    mutualInductance: element.coupling * Math.sqrt(primary.inductanceHenrys * secondary.inductanceHenrys),
  };
}

function stampAcMutualInductor(
  element: MutualInductor,
  inductors: ReadonlyMap<string, Inductor>,
  omega: number,
  nodeIndices: ReadonlyMap<string, number>,
  matrix: Complex[][],
): void {
  const { primary, secondary, mutualInductance } = validateMutualInductor(element, inductors);
  if (omega === 0.0) {
    stampComplexConductance(matrix, nodeIndex(nodeIndices, primary.n1), nodeIndex(nodeIndices, primary.n2), complex(1.0e12, 0.0));
    stampComplexConductance(matrix, nodeIndex(nodeIndices, secondary.n1), nodeIndex(nodeIndices, secondary.n2), complex(1.0e12, 0.0));
    return;
  }

  const determinant = primary.inductanceHenrys * secondary.inductanceHenrys - mutualInductance ** 2;
  if (!Number.isFinite(determinant) || determinant <= 0.0) {
    throw invalidElement(element.name, "coupled inductance matrix is singular");
  }

  const scale = complex(0.0, -1.0 / (omega * determinant));
  const y11 = complexScale(scale, secondary.inductanceHenrys);
  const y12 = complexScale(scale, -mutualInductance);
  const y22 = complexScale(scale, primary.inductanceHenrys);
  const p1 = nodeIndex(nodeIndices, primary.n1);
  const p2 = nodeIndex(nodeIndices, primary.n2);
  const s1 = nodeIndex(nodeIndices, secondary.n1);
  const s2 = nodeIndex(nodeIndices, secondary.n2);
  stampComplexConductance(matrix, p1, p2, y11);
  stampComplexConductance(matrix, s1, s2, y22);
  stampComplexTransconductance(matrix, p1, p2, s1, s2, y12);
  stampComplexTransconductance(matrix, s1, s2, p1, p2, y12);
}

function stampAcTransmissionLine(
  element: TransmissionLine,
  omega: number,
  nodeIndices: ReadonlyMap<string, number>,
  matrix: Complex[][],
): void {
  if (!Number.isFinite(element.characteristicImpedanceOhms)) {
    throw invalidElement(element.name, "characteristic impedance must be finite");
  }
  if (element.characteristicImpedanceOhms <= 0.0) {
    throw invalidElement(element.name, "characteristic impedance must be positive");
  }
  if (!Number.isFinite(element.delaySeconds)) {
    throw invalidElement(element.name, "delay must be finite");
  }
  if (element.delaySeconds <= 0.0) {
    throw invalidElement(element.name, "delay must be positive");
  }
  const phase = omega * element.delaySeconds;
  const sinPhase = Math.sin(phase);
  if (Math.abs(sinPhase) < 1.0e-12) {
    throw invalidElement(element.name, "transmission line phase is singular at this frequency");
  }
  const cosPhase = Math.cos(phase);
  const y11 = complex(0.0, -cosPhase / (element.characteristicImpedanceOhms * sinPhase));
  const y12 = complex(0.0, 1.0 / (element.characteristicImpedanceOhms * sinPhase));
  const n1 = nodeIndex(nodeIndices, element.n1);
  const n2 = nodeIndex(nodeIndices, element.n2);
  const n3 = nodeIndex(nodeIndices, element.n3);
  const n4 = nodeIndex(nodeIndices, element.n4);
  stampComplexConductance(matrix, n1, n2, y11);
  stampComplexConductance(matrix, n3, n4, y11);
  stampComplexTransconductance(matrix, n1, n2, n3, n4, y12);
  stampComplexTransconductance(matrix, n3, n4, n1, n2, y12);
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
  omega: number,
): void {
  validateBjt(element);
  const collector = nodeIndex(nodeIndices, element.collector);
  const base = nodeIndex(nodeIndices, element.base);
  const emitter = nodeIndex(nodeIndices, element.emitter);
  const transconductance = element.saturationCurrent / element.thermalVoltage;
  const junctionConductance = transconductance / element.forwardBeta;
  const diffusionCapacitance = element.forwardTransitTime * transconductance;
  const reverseDiffusionCapacitance = element.reverseTransitTime * transconductance;
  const baseEmitterAdmittance = complex(
    junctionConductance,
    omega * (element.baseEmitterCapacitance + diffusionCapacitance),
  );
  const baseCollectorAdmittance = complex(
    0.0,
    omega * (element.baseCollectorCapacitance + reverseDiffusionCapacitance),
  );
  if (element.polarity === "NPN") {
    stampComplexConductance(
      matrix,
      base,
      emitter,
      baseEmitterAdmittance,
    );
    stampComplexConductance(matrix, base, collector, baseCollectorAdmittance);
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
      baseEmitterAdmittance,
    );
    stampComplexConductance(matrix, base, collector, baseCollectorAdmittance);
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
  operatingPoint: readonly number[],
  omega: number,
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
  const result = evaluateMosfetLevel1(
    element,
    gateVoltage - sourceVoltage,
    drainVoltage - sourceVoltage,
    bodyVoltage - sourceVoltage,
  );
  stampComplexConductance(matrix, drain, source, complex(result.gds, 0.0));
  stampComplexTransconductance(
    matrix,
    drain,
    source,
    gate,
    source,
    complex(result.gm, 0.0),
  );
  stampComplexConductance(matrix, gate, source, complex(0.0, omega * result.cgs));
  stampComplexConductance(matrix, gate, drain, complex(0.0, omega * result.cgd));
  stampComplexConductance(matrix, gate, body, complex(0.0, omega * result.cgb));
  stampComplexConductance(matrix, body, source, complex(0.0, omega * result.cbs));
  stampComplexConductance(matrix, body, drain, complex(0.0, omega * result.cbd));
  stampComplexTransconductance(
    matrix,
    drain,
    source,
    body,
    source,
    complex(result.gmb, 0.0),
  );
}

function stampAcJfetSmallSignal(
  element: Jfet,
  nodeIndices: ReadonlyMap<string, number>,
  matrix: Complex[][],
  operatingPoint: readonly number[],
): void {
  validateJfet(element);
  const drain = nodeIndex(nodeIndices, element.drain);
  const gate = nodeIndex(nodeIndices, element.gate);
  const source = nodeIndex(nodeIndices, element.source);
  const drainVoltage = vectorVoltage(operatingPoint, drain);
  const gateVoltage = vectorVoltage(operatingPoint, gate);
  const sourceVoltage = vectorVoltage(operatingPoint, source);
  const result = evaluateJfet(
    element,
    gateVoltage - sourceVoltage,
    drainVoltage - sourceVoltage,
  );
  stampComplexConductance(matrix, drain, source, complex(result.gds, 0.0));
  stampComplexTransconductance(
    matrix,
    drain,
    source,
    gate,
    source,
    complex(result.gm, 0.0),
  );
}

function solveLinearSystem(matrix: number[][], rhs: number[]): number[] {
  if (rhs.length >= SPARSE_SOLVER_THRESHOLD) {
    return solveSparseLinearSystem(matrix, rhs);
  }
  return solveDenseLinearSystem(matrix, rhs);
}

function solveDenseLinearSystem(matrix: number[][], rhs: number[]): number[] {
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

function solveSparseLinearSystem(matrix: number[][], rhs: number[]): number[] {
  const n = rhs.length;
  const rows = matrix.map((row) => {
    const entries = new Map<number, number>();
    row.forEach((value, col) => {
      if (value !== 0.0) {
        entries.set(col, value);
      }
    });
    return entries;
  });
  const sparseRhs = [...rhs];

  for (let pivotCol = 0; pivotCol < n; pivotCol++) {
    let pivotRow = pivotCol;
    let pivotAbs = Math.abs(rows[pivotCol].get(pivotCol) ?? 0.0);
    for (let row = pivotCol + 1; row < n; row++) {
      const candidateAbs = Math.abs(rows[row].get(pivotCol) ?? 0.0);
      if (candidateAbs > pivotAbs) {
        pivotAbs = candidateAbs;
        pivotRow = row;
      }
    }

    if (pivotAbs < PIVOT_EPSILON) {
      throw new SpiceError("circuit matrix is singular", "SINGULAR_MATRIX");
    }

    [rows[pivotCol], rows[pivotRow]] = [rows[pivotRow], rows[pivotCol]];
    [sparseRhs[pivotCol], sparseRhs[pivotRow]] = [
      sparseRhs[pivotRow],
      sparseRhs[pivotCol],
    ];

    const pivot = rows[pivotCol].get(pivotCol)!;
    const pivotEntries = [...rows[pivotCol].entries()].filter(
      ([col]) => col > pivotCol,
    );
    for (let row = pivotCol + 1; row < n; row++) {
      const value = rows[row].get(pivotCol) ?? 0.0;
      if (value === 0.0) {
        continue;
      }
      const factor = value / pivot;
      rows[row].delete(pivotCol);
      for (const [col, pivotValue] of pivotEntries) {
        const nextValue = (rows[row].get(col) ?? 0.0) - factor * pivotValue;
        if (Math.abs(nextValue) < PIVOT_EPSILON) {
          rows[row].delete(col);
        } else {
          rows[row].set(col, nextValue);
        }
      }
      sparseRhs[row] -= factor * sparseRhs[pivotCol];
    }
  }

  const solution = Array.from({ length: n }, () => 0.0);
  for (let row = n - 1; row >= 0; row--) {
    const diagonal = rows[row].get(row) ?? 0.0;
    if (Math.abs(diagonal) < PIVOT_EPSILON) {
      throw new SpiceError("circuit matrix is singular", "SINGULAR_MATRIX");
    }
    let value = sparseRhs[row];
    for (const [col, entry] of rows[row].entries()) {
      if (col > row) {
        value -= entry * solution[col];
      }
    }
    solution[row] = value / diagonal;
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

function complexScale(value: Complex, scale: number): Complex {
  return complex(value.real * scale, value.imag * scale);
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
