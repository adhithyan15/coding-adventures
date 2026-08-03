import {
  Circuit,
  ExpWaveform,
  PulseWaveform,
  PwlWaveform,
  SinWaveform,
  bjt,
  capacitorWithInitialVoltage,
  cccs,
  ccvs,
  currentSource,
  currentSourceWithAc,
  currentSourceWithWaveform,
  diode,
  acSweep,
  dcOp,
  dcSweep,
  defaultMosfetLevel1Params,
  inductorWithInitialCurrent,
  jfet,
  mosfet,
  mosfetFromModelCard,
  mutualInductor,
  normalizeModelCard,
  resistor,
  transmissionLine,
  vccs,
  vcvs,
  voltageSource,
  voltageSourceWithAc,
  voltageSourceWithWaveform,
  transient,
  type AdaptiveTransientOptions,
  type AcPoint,
  type Complex,
  type DcOpOptions,
  type DcResult,
  type DcSweepPoint,
  type Element,
  type JfetPolarity,
  type MosfetLevel1Params,
  type TransientPoint,
  type TransientMethod,
  type Waveform,
} from "@coding-adventures/spice-engine";

export class NetlistParseError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "NetlistParseError";
  }
}

export interface OpAnalysis {
  readonly kind: "op";
}

export interface TranAnalysis {
  readonly kind: "tran";
  readonly timeStep: number;
  readonly stopTime: number;
  readonly method?: TransientMethod;
}

export interface DcAnalysis {
  readonly kind: "dc";
  readonly sourceName: string;
  readonly start: number;
  readonly stop: number;
  readonly step: number;
}

export interface AcAnalysis {
  readonly kind: "ac";
  readonly mode: string;
  readonly points: number;
  readonly startHz: number;
  readonly stopHz: number;
}

export interface TfAnalysis {
  readonly kind: "tf";
  readonly outputNode: string;
  readonly inputSource: string;
}

export interface SensAnalysis {
  readonly kind: "sens";
  readonly outputNode: string;
}

export interface McAnalysis {
  readonly kind: "mc";
  readonly outputNode: string;
  readonly nTrials: number;
  readonly tolerance: number;
  readonly distribution: "gaussian" | "uniform";
  readonly seed?: number;
}

export interface NoiseAnalysis {
  readonly kind: "noise";
  readonly outputNode: string;
  readonly inputSource: string;
  readonly frequenciesHz: readonly number[];
  readonly temperature: number;
  readonly temperatureIsExplicit?: boolean;
}

export interface TempAnalysis {
  readonly kind: "temp";
  readonly temperaturesCelsius: readonly number[];
}

export interface OutputProbe {
  readonly kind: "voltage" | "current";
  readonly target: string;
}

export interface PrintAnalysis {
  readonly kind: "print";
  readonly analysis: string;
  readonly probes: readonly OutputProbe[];
}

export interface PlotAnalysis {
  readonly kind: "plot";
  readonly analysis: string;
  readonly probes: readonly OutputProbe[];
}

export interface SaveAnalysis {
  readonly kind: "save";
  readonly probes: readonly OutputProbe[];
}

export interface ProbeAnalysis {
  readonly kind: "probe";
  readonly analysis?: string;
  readonly probes: readonly OutputProbe[];
}

export type MeasureOperation = "find" | "max" | "min" | "avg" | "rms";

export interface MeasureAnalysis {
  readonly kind: "measure";
  readonly analysis: string;
  readonly name: string;
  readonly operation: MeasureOperation;
  readonly probe: OutputProbe;
  readonly at?: number;
  readonly start?: number;
  readonly stop?: number;
}

export interface FourAnalysis {
  readonly kind: "four";
  readonly frequencyHz: number;
  readonly probes: readonly OutputProbe[];
}

export interface DistortionAnalysis {
  readonly kind: "disto";
  readonly mode: string;
  readonly points: number;
  readonly startHz: number;
  readonly stopHz: number;
  readonly probes: readonly OutputProbe[];
}

export interface PoleZeroAnalysis {
  readonly kind: "pz";
  readonly outputNode: string;
  readonly inputSource: string;
  readonly poleZeroKind: "pole" | "zero" | "pz";
}

export type OptionValue = number | string | boolean;

export interface OptionsAnalysis {
  readonly kind: "options";
  readonly values: ReadonlyMap<string, OptionValue>;
}

export type Analysis =
  | OpAnalysis
  | TranAnalysis
  | DcAnalysis
  | AcAnalysis
  | TfAnalysis
  | SensAnalysis
  | McAnalysis
  | NoiseAnalysis
  | TempAnalysis
  | PrintAnalysis
  | PlotAnalysis
  | SaveAnalysis
  | ProbeAnalysis
  | MeasureAnalysis
  | FourAnalysis
  | DistortionAnalysis
  | PoleZeroAnalysis
  | OptionsAnalysis;

export type RunnableAnalysis = OpAnalysis | TranAnalysis | DcAnalysis | AcAnalysis;
export type AnalysisKind = RunnableAnalysis["kind"];
export type AnalysisResult =
  | DcResult
  | readonly DcSweepPoint[]
  | readonly AcPoint[]
  | readonly TransientPoint[];
export type SelectedOutputValue = number | Complex;

export interface AnalysisPlanStep {
  readonly index: number;
  readonly kind: AnalysisKind;
  readonly analysis: RunnableAnalysis;
}

export interface AnalysisExecutionResult {
  readonly index: number;
  readonly kind: AnalysisKind;
  readonly analysis: RunnableAnalysis;
  readonly result: AnalysisResult;
}

export interface SelectedOutputRow {
  readonly index: number;
  readonly axisName?: string;
  readonly axisValue?: number;
  readonly values: ReadonlyMap<string, SelectedOutputValue>;
}

export interface SelectedAnalysisOutput {
  readonly index: number;
  readonly kind: AnalysisKind;
  readonly probes: readonly OutputProbe[];
  readonly rows: readonly SelectedOutputRow[];
}

export interface MeasureResult {
  readonly analysisIndex: number;
  readonly analysis: string;
  readonly name: string;
  readonly operation: MeasureOperation;
  readonly probe: OutputProbe;
  readonly value: number;
}

export interface ModelCard {
  readonly name: string;
  readonly kind: string;
  readonly params: ReadonlyMap<string, number>;
}

export class ParsedNetlist {
  constructor(
    readonly circuit = new Circuit(),
    readonly analyses: Analysis[] = [],
    readonly models: ReadonlyMap<string, ModelCard> = new Map(),
    readonly title?: string,
  ) {}

  opCards(): OpAnalysis[] {
    return this.analyses.filter((analysis): analysis is OpAnalysis => analysis.kind === "op");
  }

  tranCards(): TranAnalysis[] {
    return this.analyses.filter((analysis): analysis is TranAnalysis => analysis.kind === "tran");
  }

  dcCards(): DcAnalysis[] {
    return this.analyses.filter((analysis): analysis is DcAnalysis => analysis.kind === "dc");
  }

  acCards(): AcAnalysis[] {
    return this.analyses.filter((analysis): analysis is AcAnalysis => analysis.kind === "ac");
  }

  tfCards(): TfAnalysis[] {
    return this.analyses.filter((analysis): analysis is TfAnalysis => analysis.kind === "tf");
  }

  sensCards(): SensAnalysis[] {
    return this.analyses.filter((analysis): analysis is SensAnalysis => analysis.kind === "sens");
  }

  mcCards(): McAnalysis[] {
    return this.analyses.filter((analysis): analysis is McAnalysis => analysis.kind === "mc");
  }

  noiseCards(): NoiseAnalysis[] {
    return this.analyses.filter(
      (analysis): analysis is NoiseAnalysis => analysis.kind === "noise",
    );
  }

  optionsCards(): OptionsAnalysis[] {
    return this.analyses.filter((analysis): analysis is OptionsAnalysis => analysis.kind === "options");
  }

  tempCards(): TempAnalysis[] {
    return this.analyses.filter((analysis): analysis is TempAnalysis => analysis.kind === "temp");
  }

  printCards(): PrintAnalysis[] {
    return this.analyses.filter((analysis): analysis is PrintAnalysis => analysis.kind === "print");
  }

  plotCards(): PlotAnalysis[] {
    return this.analyses.filter((analysis): analysis is PlotAnalysis => analysis.kind === "plot");
  }

  saveCards(): SaveAnalysis[] {
    return this.analyses.filter((analysis): analysis is SaveAnalysis => analysis.kind === "save");
  }

  probeCards(): ProbeAnalysis[] {
    return this.analyses.filter((analysis): analysis is ProbeAnalysis => analysis.kind === "probe");
  }

  measureCards(): MeasureAnalysis[] {
    return this.analyses.filter(
      (analysis): analysis is MeasureAnalysis => analysis.kind === "measure",
    );
  }

  fourCards(): FourAnalysis[] {
    return this.analyses.filter((analysis): analysis is FourAnalysis => analysis.kind === "four");
  }

  distortionCards(): DistortionAnalysis[] {
    return this.analyses.filter(
      (analysis): analysis is DistortionAnalysis => analysis.kind === "disto",
    );
  }

  poleZeroCards(): PoleZeroAnalysis[] {
    return this.analyses.filter((analysis): analysis is PoleZeroAnalysis => analysis.kind === "pz");
  }

  analysisPlan(): AnalysisPlanStep[] {
    return buildAnalysisPlan(this);
  }

  runAnalysisPlan(plan?: readonly AnalysisPlanStep[]): AnalysisExecutionResult[] {
    return runAnalysisPlan(this, plan);
  }

  selectOutputs(results?: readonly AnalysisExecutionResult[]): SelectedAnalysisOutput[] {
    return selectOutputs(this, results);
  }

  measureResults(results?: readonly AnalysisExecutionResult[]): MeasureResult[] {
    return measureResults(this, results);
  }

  transientMethod(tran?: TranAnalysis): TransientMethod | undefined {
    if (tran?.method !== undefined) {
      return tran.method;
    }
    for (const options of this.optionsCards()) {
      const value = options.values.get("method");
      if (typeof value === "string") {
        return parseTransientMethod(value, ".options method");
      }
    }
    return undefined;
  }

  dcOpOptions(): DcOpOptions {
    const values = this.mergedOptions();
    const options: {
      maxIterations?: number;
      tolerance?: number;
      pseudoTransientConductance?: number;
      pseudoTransientSteps?: number;
      pseudoTransientMaxIterations?: number;
    } = {};
    const tolerance = optionNumber(values, ["reltol", "tol"]);
    if (tolerance !== undefined) {
      options.tolerance = tolerance;
    }
    const maxIterations = optionInteger(values, ["itl1", "maxiter", "maxiters", "maxiterations"]);
    if (maxIterations !== undefined) {
      options.maxIterations = maxIterations;
    }
    const gmin = optionNumber(values, ["gmin"]);
    if (gmin !== undefined) {
      options.pseudoTransientConductance = gmin;
    }
    const pseudoSteps = optionInteger(values, ["srcsteps", "pseudotransientsteps"]);
    if (pseudoSteps !== undefined) {
      options.pseudoTransientSteps = pseudoSteps;
    }
    const pseudoIterations = optionInteger(values, ["itl6", "pseudotransientmaxiterations"]);
    if (pseudoIterations !== undefined) {
      options.pseudoTransientMaxIterations = pseudoIterations;
    }
    return options;
  }

  adaptiveTransientOptions(tran?: TranAnalysis): AdaptiveTransientOptions {
    const values = this.mergedOptions();
    const options: {
      method?: TransientMethod;
      tolerance?: number;
      minStep?: number;
      maxStep?: number;
    } = {};
    const method = this.transientMethod(tran);
    if (method !== undefined) {
      options.method = method;
    }
    const tolerance = optionNumber(values, ["trtol", "lte", "tollte"]);
    if (tolerance !== undefined) {
      options.tolerance = tolerance;
    }
    const minStep = optionNumber(values, ["minstep", "tmin"]);
    if (minStep !== undefined) {
      options.minStep = minStep;
    }
    const maxStep = optionNumber(values, ["maxstep", "tmax"]);
    if (maxStep !== undefined) {
      options.maxStep = maxStep;
    }
    return options;
  }

  operatingTemperatureKelvin(
    temperatureIndex = 0,
    defaultTemperatureKelvin = 300.0,
  ): number {
    if (!Number.isInteger(temperatureIndex) || temperatureIndex < 0) {
      throw new NetlistParseError("temperature index must be non-negative");
    }
    const temperaturesCelsius = this.tempCards().flatMap((card) => card.temperaturesCelsius);
    if (temperaturesCelsius.length === 0) {
      return defaultTemperatureKelvin;
    }
    const temperature = temperaturesCelsius[temperatureIndex];
    if (temperature === undefined) {
      throw new NetlistParseError(`temperature index ${temperatureIndex} exceeds .temp entries`);
    }
    return temperature + 273.15;
  }

  noiseTemperatureKelvin(
    noise?: NoiseAnalysis,
    temperatureIndex = 0,
    defaultTemperatureKelvin = 300.0,
  ): number {
    if (noise?.temperatureIsExplicit === true) {
      return noise.temperature;
    }
    return this.operatingTemperatureKelvin(temperatureIndex, defaultTemperatureKelvin);
  }

  private mergedOptions(): Map<string, OptionValue> {
    const values = new Map<string, OptionValue>();
    for (const options of this.optionsCards()) {
      for (const [key, value] of options.values) {
        values.set(key, value);
      }
    }
    return values;
  }
}

function optionNumber(
  values: ReadonlyMap<string, OptionValue>,
  keys: readonly string[],
): number | undefined {
  for (const key of keys) {
    const value = values.get(key);
    if (typeof value === "number") {
      return value;
    }
  }
  return undefined;
}

function optionInteger(
  values: ReadonlyMap<string, OptionValue>,
  keys: readonly string[],
): number | undefined {
  const value = optionNumber(values, keys);
  return value === undefined ? undefined : Math.trunc(value);
}

interface Statement {
  readonly lineNumber: number;
  readonly fields: string[];
}

interface SubcktDefinition {
  readonly name: string;
  readonly pins: string[];
  readonly body: Statement[];
  readonly lineNumber: number;
}

interface SourceSpec {
  readonly dcValue: number;
  readonly waveform?: Waveform;
  readonly ac?: {
    readonly magnitude: number;
    readonly phaseDegrees: number;
  };
}

const VALUE_RE = /^\s*([+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?)([a-zA-Z]*)\s*$/;
const SUFFIXES = new Map<string, number>([
  ["t", 1.0e12],
  ["g", 1.0e9],
  ["meg", 1.0e6],
  ["k", 1.0e3],
  ["", 1.0],
  ["m", 1.0e-3],
  ["u", 1.0e-6],
  ["n", 1.0e-9],
  ["p", 1.0e-12],
  ["f", 1.0e-15],
]);

export function parseNetlist(text: string): ParsedNetlist {
  const circuit = new Circuit();
  const analyses: Analysis[] = [];
  const models = new Map<string, ModelCard>();
  const statements: Statement[] = [];
  const subckts = new Map<string, SubcktDefinition>();
  let currentSubckt: SubcktDefinition | undefined;
  let title: string | undefined;
  let sawContent = false;

  const lines = text.split(/\r?\n/);
  for (let index = 0; index < lines.length; index++) {
    const lineNumber = index + 1;
    const rawLine = lines[index];
    const stripped = rawLine.trim();
    if (stripped.length === 0) {
      continue;
    }
    if (stripped.startsWith("*")) {
      if (!sawContent && title === undefined) {
        const candidate = stripped.slice(1).trim();
        title = candidate.length > 0 ? candidate : undefined;
      }
      continue;
    }
    sawContent = true;

    let fields: string[];
    try {
      fields = splitFields(stripInlineComment(rawLine));
    } catch (error) {
      throw lineError(lineNumber, error);
    }
    if (fields.length === 0) {
      continue;
    }
    const head = fields[0];
    const headLower = head.toLowerCase();

    try {
      if (currentSubckt !== undefined) {
        if (headLower === ".ends") {
          finishSubckt(currentSubckt, fields);
          subckts.set(currentSubckt.name.toLowerCase(), currentSubckt);
          currentSubckt = undefined;
        } else if (headLower === ".subckt") {
          throw new NetlistParseError("nested .subckt definitions are not supported");
        } else {
          currentSubckt.body.push({ lineNumber, fields });
        }
        continue;
      }
      if (headLower === ".subckt") {
        currentSubckt = startSubckt(fields, lineNumber, subckts);
        continue;
      }
      if (headLower === ".ends") {
        throw new NetlistParseError(".ends without matching .subckt");
      }
    } catch (error) {
      throw lineError(lineNumber, error);
    }

    if (headLower === ".end") {
      break;
    }
    statements.push({ lineNumber, fields });
  }

  if (currentSubckt !== undefined) {
    throw new NetlistParseError(
      `line ${currentSubckt.lineNumber}: .subckt ${JSON.stringify(currentSubckt.name)} is missing .ends`,
    );
  }

  for (const statement of statements) {
    if (statement.fields[0].toLowerCase() !== ".model") {
      continue;
    }
    try {
      const model = parseModelCard(statement.fields);
      const key = model.name.toLowerCase();
      if (models.has(key)) {
        throw new NetlistParseError(`duplicate .model definition ${JSON.stringify(model.name)}`);
      }
      models.set(key, model);
    } catch (error) {
      throw lineError(statement.lineNumber, error);
    }
  }

  for (const statement of statements) {
    try {
      if (statement.fields[0].toLowerCase() === ".model") {
        continue;
      }
      if (statement.fields[0].startsWith(".")) {
        analyses.push(parseDirective(statement.fields));
      } else if (statement.fields[0].toUpperCase().startsWith("X")) {
        for (const element of expandSubcktInstance(statement.fields, subckts, [], models)) {
          circuit.add(element);
        }
      } else {
        circuit.add(parseElement(statement.fields, models));
      }
    } catch (error) {
      throw lineError(statement.lineNumber, error);
    }
  }
  validateMutualInductors(circuit);
  validateTransmissionLines(circuit);

  return new ParsedNetlist(circuit, analyses, models, title);
}

export const parse = parseNetlist;

export function buildAnalysisPlan(parsed: ParsedNetlist): AnalysisPlanStep[] {
  return parsed.analyses.flatMap((analysis, index): AnalysisPlanStep[] => {
    const step = analysisPlanStep(index, analysis);
    return step === undefined ? [] : [step];
  });
}

export function runAnalysisPlan(
  parsed: ParsedNetlist,
  plan: readonly AnalysisPlanStep[] = buildAnalysisPlan(parsed),
): AnalysisExecutionResult[] {
  return plan.map((step) => ({
    index: step.index,
    kind: step.kind,
    analysis: step.analysis,
    result: executeAnalysisStep(parsed, step),
  }));
}

export function runNetlist(text: string): AnalysisExecutionResult[] {
  return runAnalysisPlan(parseNetlist(text));
}

export function selectOutputs(
  parsed: ParsedNetlist,
  results: readonly AnalysisExecutionResult[] = runAnalysisPlan(parsed),
): SelectedAnalysisOutput[] {
  const selected: SelectedAnalysisOutput[] = [];
  for (const result of results) {
    const probes = selectedOutputProbes(parsed, result.kind);
    if (probes.length === 0) {
      continue;
    }
    selected.push({
      index: result.index,
      kind: result.kind,
      probes,
      rows: selectedOutputRows(result, probes),
    });
  }
  return selected;
}

export function measureResults(
  parsed: ParsedNetlist,
  results: readonly AnalysisExecutionResult[] = runAnalysisPlan(parsed),
): MeasureResult[] {
  return parsed.measureCards().map((card) => {
    const execution = findMeasureExecutionResult(card, results);
    return {
      analysisIndex: execution.index,
      analysis: card.analysis,
      name: card.name,
      operation: card.operation,
      probe: card.probe,
      value: evaluateMeasure(card, execution),
    };
  });
}

export function parseValue(token: string): number {
  const match = VALUE_RE.exec(token);
  if (match === null) {
    throw new NetlistParseError(`expected numeric value, got ${JSON.stringify(token)}`);
  }
  const suffix = match[2].toLowerCase();
  const multiplier = SUFFIXES.get(suffix);
  if (multiplier === undefined) {
    throw new NetlistParseError(`unsupported numeric suffix ${JSON.stringify(match[2])}`);
  }
  return Number.parseFloat(match[1]) * multiplier;
}

function analysisPlanStep(index: number, analysis: Analysis): AnalysisPlanStep | undefined {
  if (
    analysis.kind === "op" ||
    analysis.kind === "tran" ||
    analysis.kind === "dc" ||
    analysis.kind === "ac"
  ) {
    return { index, kind: analysis.kind, analysis };
  }
  return undefined;
}

function executeAnalysisStep(parsed: ParsedNetlist, step: AnalysisPlanStep): AnalysisResult {
  const analysis = step.analysis;
  if (analysis.kind === "op") {
    return dcOp(parsed.circuit, parsed.dcOpOptions());
  }
  if (analysis.kind === "dc") {
    return dcSweep(
      parsed.circuit,
      analysis.sourceName,
      analysis.start,
      analysis.stop,
      analysis.step,
    );
  }
  if (analysis.kind === "ac") {
    return acSweep(
      parsed.circuit,
      analysis.startHz,
      analysis.stopHz,
      executableAcPointsPerDecade(analysis),
    );
  }
  if (analysis.kind === "tran") {
    return transient(
      parsed.circuit,
      analysis.timeStep,
      analysis.stopTime,
      parsed.transientMethod(analysis) ?? "euler",
    );
  }
  throw new NetlistParseError(`analysis card at index ${step.index} is not executable`);
}

function executableAcPointsPerDecade(analysis: AcAnalysis): number {
  if (analysis.mode === "dec" || analysis.mode === "log") {
    return analysis.points;
  }
  throw new NetlistParseError(
    `.ac mode ${JSON.stringify(analysis.mode)} is not executable; supported modes are "dec" and "log"`,
  );
}

function selectedOutputProbes(parsed: ParsedNetlist, kind: AnalysisKind): OutputProbe[] {
  const probes: OutputProbe[] = [];
  const seen = new Set<string>();
  const add = (newProbes: readonly OutputProbe[]) => {
    for (const probe of newProbes) {
      const key = `${probe.kind}:${probe.target.toLowerCase()}`;
      if (!seen.has(key)) {
        probes.push(probe);
        seen.add(key);
      }
    }
  };

  for (const card of parsed.analyses) {
    if (card.kind === "save") {
      add(card.probes);
    } else if (card.kind === "probe") {
      if (card.analysis === undefined || analysisNameMatches(card.analysis, kind)) {
        add(card.probes);
      }
    } else if (card.kind === "print" || card.kind === "plot") {
      if (analysisNameMatches(card.analysis, kind)) {
        add(card.probes);
      }
    }
  }
  return probes;
}

function analysisNameMatches(requested: string, kind: AnalysisKind): boolean {
  const aliases = new Map<string, AnalysisKind>([
    ["op", "op"],
    ["dcop", "op"],
    ["dc", "dc"],
    ["ac", "ac"],
    ["tran", "tran"],
    ["transient", "tran"],
  ]);
  return (aliases.get(requested.toLowerCase()) ?? requested.toLowerCase()) === kind;
}

function selectedOutputRows(
  execution: AnalysisExecutionResult,
  probes: readonly OutputProbe[],
): SelectedOutputRow[] {
  if (execution.kind === "op") {
    const result = execution.result as DcResult;
    return [
      {
        index: 0,
        values: selectedOutputValues(
          result.nodeVoltages,
          result.branchCurrents,
          probes,
          ".op output selection",
        ),
      },
    ];
  }
  if (execution.kind === "dc") {
    return (execution.result as readonly DcSweepPoint[]).map((point, index) => ({
      index,
      axisName: "source",
      axisValue: point.value,
      values: selectedOutputValues(
        point.result.nodeVoltages,
        point.result.branchCurrents,
        probes,
        ".dc output selection",
      ),
    }));
  }
  if (execution.kind === "ac") {
    return (execution.result as readonly AcPoint[]).map((point, index) => ({
      index,
      axisName: "frequency",
      axisValue: point.frequencyHz,
      values: selectedOutputValues(
        point.nodeVoltages,
        point.branchCurrents,
        probes,
        ".ac output selection",
      ),
    }));
  }
  return (execution.result as readonly TransientPoint[]).map((point, index) => ({
    index,
    axisName: "time",
    axisValue: point.time,
    values: selectedOutputValues(
      point.nodeVoltages,
      point.branchCurrents,
      probes,
      ".tran output selection",
    ),
  }));
}

function selectedOutputValues(
  nodeVoltages: ReadonlyMap<string, SelectedOutputValue>,
  branchCurrents: ReadonlyMap<string, SelectedOutputValue>,
  probes: readonly OutputProbe[],
  context: string,
): ReadonlyMap<string, SelectedOutputValue> {
  const values = new Map<string, SelectedOutputValue>();
  for (const probe of probes) {
    values.set(probeLabel(probe), probeValue(probe, nodeVoltages, branchCurrents, context));
  }
  return values;
}

function findMeasureExecutionResult(
  card: MeasureAnalysis,
  results: readonly AnalysisExecutionResult[],
): AnalysisExecutionResult {
  const result = results.find((candidate) => analysisNameMatches(card.analysis, candidate.kind));
  if (result === undefined) {
    throw new NetlistParseError(
      `.measure ${JSON.stringify(card.name)} references missing ${card.analysis} analysis`,
    );
  }
  return result;
}

function evaluateMeasure(card: MeasureAnalysis, execution: AnalysisExecutionResult): number {
  const samples = measureSamples(card, execution);
  if (samples.length === 0) {
    throw new NetlistParseError(`.measure ${JSON.stringify(card.name)} has no samples`);
  }
  if (card.operation === "find") {
    if (execution.kind === "op" && card.at === undefined) {
      return measureNumericValue(samples[0][1]);
    }
    if (card.at === undefined) {
      throw new NetlistParseError(`.measure ${JSON.stringify(card.name)} FIND requires AT=<value>`);
    }
    return measureNumericValue(interpolateMeasureValue(samples, card.at, card));
  }

  const ranged = rangeMeasureSamples(samples, card);
  const values = ranged.map((sample) => measureNumericValue(sample[1]));
  if (values.length === 0) {
    throw new NetlistParseError(`.measure ${JSON.stringify(card.name)} range has no samples`);
  }
  if (card.operation === "max") {
    return Math.max(...values);
  }
  if (card.operation === "min") {
    return Math.min(...values);
  }
  if (card.operation === "avg") {
    return averageMeasureValue(ranged);
  }
  return rmsMeasureValue(ranged);
}

type MeasureSample = readonly [number | undefined, SelectedOutputValue];

function measureSamples(card: MeasureAnalysis, execution: AnalysisExecutionResult): MeasureSample[] {
  if (execution.kind === "op") {
    const result = execution.result as DcResult;
    return [[
      undefined,
      probeValue(card.probe, result.nodeVoltages, result.branchCurrents, `.measure ${card.name}`),
    ]];
  }
  if (execution.kind === "dc") {
    return (execution.result as readonly DcSweepPoint[]).map((point) => [
      point.value,
      probeValue(
        card.probe,
        point.result.nodeVoltages,
        point.result.branchCurrents,
        `.measure ${card.name}`,
      ),
    ]);
  }
  if (execution.kind === "ac") {
    return (execution.result as readonly AcPoint[]).map((point) => [
      point.frequencyHz,
      probeValue(card.probe, point.nodeVoltages, point.branchCurrents, `.measure ${card.name}`),
    ]);
  }
  return (execution.result as readonly TransientPoint[]).map((point) => [
    point.time,
    probeValue(card.probe, point.nodeVoltages, point.branchCurrents, `.measure ${card.name}`),
  ]);
}

function rangeMeasureSamples(samples: readonly MeasureSample[], card: MeasureAnalysis): MeasureSample[] {
  if (samples.some((sample) => sample[0] === undefined)) {
    if (card.start !== undefined || card.stop !== undefined) {
      throw new NetlistParseError(`.measure ${JSON.stringify(card.name)} range requires swept samples`);
    }
    return [...samples];
  }
  const axisSamples = [...samples]
    .filter((sample): sample is readonly [number, SelectedOutputValue] => sample[0] !== undefined)
    .sort((left, right) => left[0] - right[0]);
  const lower = card.start ?? axisSamples[0][0];
  const upper = card.stop ?? axisSamples.at(-1)![0];
  if (lower > upper) {
    throw new NetlistParseError(`.measure ${JSON.stringify(card.name)} FROM must be <= TO`);
  }
  const ranged: MeasureSample[] = [];
  if (card.start !== undefined) {
    ranged.push([lower, interpolateMeasureValue(samples, lower, card)]);
  }
  for (const sample of axisSamples) {
    if (sample[0] >= lower && sample[0] <= upper && !axisAlreadyPresent(ranged, sample[0])) {
      ranged.push(sample);
    }
  }
  if (card.stop !== undefined && !axisAlreadyPresent(ranged, upper)) {
    ranged.push([upper, interpolateMeasureValue(samples, upper, card)]);
  }
  return ranged.sort((left, right) => (left[0] ?? Number.NEGATIVE_INFINITY) - (right[0] ?? Number.NEGATIVE_INFINITY));
}

function axisAlreadyPresent(samples: readonly MeasureSample[], axis: number): boolean {
  return samples.some((sample) => sample[0] !== undefined && Math.abs(sample[0] - axis) <= 1.0e-12);
}

function interpolateMeasureValue(
  samples: readonly MeasureSample[],
  target: number,
  card: MeasureAnalysis,
): SelectedOutputValue {
  const axisSamples = [...samples]
    .filter((sample): sample is readonly [number, SelectedOutputValue] => sample[0] !== undefined)
    .sort((left, right) => left[0] - right[0]);
  if (axisSamples.length === 0) {
    throw new NetlistParseError(`.measure ${JSON.stringify(card.name)} AT requires swept samples`);
  }
  if (target < axisSamples[0][0] || target > axisSamples.at(-1)![0]) {
    throw new NetlistParseError(`.measure ${JSON.stringify(card.name)} AT is outside the analysis range`);
  }
  for (const [axis, value] of axisSamples) {
    if (Math.abs(axis - target) <= 1.0e-12) {
      return value;
    }
  }
  for (let index = 0; index < axisSamples.length - 1; index += 1) {
    const [leftAxis, leftValue] = axisSamples[index];
    const [rightAxis, rightValue] = axisSamples[index + 1];
    if (leftAxis <= target && target <= rightAxis) {
      const fraction = (target - leftAxis) / (rightAxis - leftAxis);
      return interpolateOutputValues(leftValue, rightValue, fraction);
    }
  }
  return axisSamples.at(-1)![1];
}

function interpolateOutputValues(
  left: SelectedOutputValue,
  right: SelectedOutputValue,
  fraction: number,
): SelectedOutputValue {
  if (isComplex(left) || isComplex(right)) {
    const leftComplex = toComplex(left);
    const rightComplex = toComplex(right);
    return {
      real: leftComplex.real + (rightComplex.real - leftComplex.real) * fraction,
      imag: leftComplex.imag + (rightComplex.imag - leftComplex.imag) * fraction,
    };
  }
  return left + (right - left) * fraction;
}

function averageMeasureValue(samples: readonly MeasureSample[]): number {
  const numeric = samples.map((sample) => [sample[0], measureNumericValue(sample[1])] as const);
  if (numeric.length < 2 || numeric.some((sample) => sample[0] === undefined)) {
    return numeric.reduce((sum, sample) => sum + sample[1], 0.0) / numeric.length;
  }
  const span = numeric.at(-1)![0]! - numeric[0][0]!;
  if (span <= 0.0) {
    return numeric.reduce((sum, sample) => sum + sample[1], 0.0) / numeric.length;
  }
  let area = 0.0;
  for (let index = 0; index < numeric.length - 1; index += 1) {
    const [leftAxis, leftValue] = numeric[index];
    const [rightAxis, rightValue] = numeric[index + 1];
    area += 0.5 * (leftValue + rightValue) * (rightAxis! - leftAxis!);
  }
  return area / span;
}

function rmsMeasureValue(samples: readonly MeasureSample[]): number {
  const numeric = samples.map((sample) => [sample[0], measureNumericValue(sample[1])] as const);
  if (numeric.length < 2 || numeric.some((sample) => sample[0] === undefined)) {
    return Math.sqrt(
      numeric.reduce((sum, sample) => sum + sample[1] * sample[1], 0.0) / numeric.length,
    );
  }
  const span = numeric.at(-1)![0]! - numeric[0][0]!;
  if (span <= 0.0) {
    return Math.sqrt(
      numeric.reduce((sum, sample) => sum + sample[1] * sample[1], 0.0) / numeric.length,
    );
  }
  let area = 0.0;
  for (let index = 0; index < numeric.length - 1; index += 1) {
    const [leftAxis, leftValue] = numeric[index];
    const [rightAxis, rightValue] = numeric[index + 1];
    area += 0.5 * (leftValue * leftValue + rightValue * rightValue) * (rightAxis! - leftAxis!);
  }
  return Math.sqrt(area / span);
}

function measureNumericValue(value: SelectedOutputValue): number {
  return isComplex(value) ? Math.hypot(value.real, value.imag) : value;
}

function probeValue(
  probe: OutputProbe,
  nodeVoltages: ReadonlyMap<string, SelectedOutputValue>,
  branchCurrents: ReadonlyMap<string, SelectedOutputValue>,
  context: string,
): SelectedOutputValue {
  if (probe.kind === "voltage") {
    if (probe.target.toLowerCase() === "0" || probe.target.toLowerCase() === "gnd") {
      return containsComplexValues(nodeVoltages) ? { real: 0.0, imag: 0.0 } : 0.0;
    }
    const value = caseInsensitiveGet(nodeVoltages, probe.target);
    if (value === undefined) {
      throw new NetlistParseError(`${context}: missing voltage probe V(${probe.target})`);
    }
    return value;
  }
  const key = probe.target.toLowerCase().startsWith("i(") ? probe.target : `I(${probe.target})`;
  const value = caseInsensitiveGet(branchCurrents, key);
  if (value === undefined) {
    throw new NetlistParseError(`${context}: missing branch current probe I(${probe.target})`);
  }
  return value;
}

function containsComplexValues(values: ReadonlyMap<string, SelectedOutputValue>): boolean {
  return Array.from(values.values()).some(isComplex);
}

function caseInsensitiveGet(
  values: ReadonlyMap<string, SelectedOutputValue>,
  key: string,
): SelectedOutputValue | undefined {
  const exact = values.get(key);
  if (exact !== undefined) {
    return exact;
  }
  const lowerKey = key.toLowerCase();
  for (const [candidate, value] of values) {
    if (candidate.toLowerCase() === lowerKey) {
      return value;
    }
  }
  return undefined;
}

function probeLabel(probe: OutputProbe): string {
  return probe.kind === "voltage" ? `V(${probe.target})` : `I(${probe.target})`;
}

function isComplex(value: SelectedOutputValue): value is Complex {
  return typeof value === "object";
}

function toComplex(value: SelectedOutputValue): Complex {
  return isComplex(value) ? value : { real: value, imag: 0.0 };
}

function parseModelCard(fields: readonly string[]): ModelCard {
  requireMinFields(fields, 3, ".model");
  const tail = fields.slice(2).join(" ").trim();
  const match = /^([A-Za-z][A-Za-z0-9_]*)(?:\s*\((.*)\)|\s+(.*))?$/.exec(tail);
  if (match === null) {
    throw new NetlistParseError(`invalid .model kind ${JSON.stringify(tail)}`);
  }
  let paramsText = (match[2] ?? match[3] ?? "").trim();
  if (paramsText.startsWith("(") && paramsText.endsWith(")")) {
    paramsText = paramsText.slice(1, -1);
  }
  const kind = match[1].toUpperCase();
  const params = parseModelParams(paramsText);
  const diodeSaturationCurrent = params.get("IS") ?? params.get("JS");
  if (
    kind === "D" &&
    diodeSaturationCurrent !== undefined &&
    (!Number.isFinite(diodeSaturationCurrent) || diodeSaturationCurrent <= 0.0)
  ) {
    throw new NetlistParseError("diode IS must be finite and positive");
  }
  const gateSourceCapacitance = params.get("CGS") ?? params.get("CGS0");
  if (
    (kind === "NJF" || kind === "PJF") &&
    gateSourceCapacitance !== undefined &&
    (!Number.isFinite(gateSourceCapacitance) || gateSourceCapacitance < 0.0)
  ) {
    throw new NetlistParseError("JFET CGS must be finite and non-negative");
  }
  const gateDrainCapacitance = params.get("CGD") ?? params.get("CGD0");
  if (
    (kind === "NJF" || kind === "PJF") &&
    gateDrainCapacitance !== undefined &&
    (!Number.isFinite(gateDrainCapacitance) || gateDrainCapacitance < 0.0)
  ) {
    throw new NetlistParseError("JFET CGD must be finite and non-negative");
  }
  const level = params.get("LEVEL");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    level !== undefined &&
    (!Number.isFinite(level) || Math.abs(level - 1.0) > 1.0e-12)
  ) {
    throw new NetlistParseError("only MOS LEVEL=1 model cards are supported");
  }
  const oxideThickness = params.get("TOX");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    oxideThickness !== undefined &&
    (!Number.isFinite(oxideThickness) || oxideThickness <= 0.0)
  ) {
    throw new NetlistParseError("MOSFET TOX must be finite and positive");
  }
  const substrateDoping = params.get("N_SUB") ?? params.get("NSUB") ?? params.get("N");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    substrateDoping !== undefined &&
    (!Number.isFinite(substrateDoping) || substrateDoping <= 0.0)
  ) {
    throw new NetlistParseError("MOSFET NSUB must be finite and positive");
  }
  const surfaceStateDensity = params.get("NSS");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    surfaceStateDensity !== undefined &&
    (!Number.isFinite(surfaceStateDensity) || surfaceStateDensity < 0.0)
  ) {
    throw new NetlistParseError("MOSFET NSS must be finite and non-negative");
  }
  const gateType = params.get("TPG");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    gateType !== undefined &&
    gateType !== -1.0 &&
    gateType !== 0.0 &&
    gateType !== 1.0
  ) {
    throw new NetlistParseError("MOSFET TPG must be -1, 0, or 1");
  }
  const surfaceMobility = params.get("U0") ?? params.get("UO");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    surfaceMobility !== undefined &&
    (!Number.isFinite(surfaceMobility) || surfaceMobility < 0.0)
  ) {
    throw new NetlistParseError("MOSFET U0 must be finite and non-negative");
  }
  const transconductance = params.get("KP");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    transconductance !== undefined &&
    (!Number.isFinite(transconductance) || transconductance <= 0.0)
  ) {
    throw new NetlistParseError("MOSFET KP must be finite and positive");
  }
  const thresholdVoltage = params.get("VT0") ?? params.get("VTO") ?? params.get("VTH");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    thresholdVoltage !== undefined &&
    !Number.isFinite(thresholdVoltage)
  ) {
    throw new NetlistParseError("MOSFET VT0 must be finite");
  }
  const channelModulation = params.get("LAMBDA") ?? params.get("LAM");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    channelModulation !== undefined &&
    !Number.isFinite(channelModulation)
  ) {
    throw new NetlistParseError("MOSFET LAMBDA must be finite");
  }
  const bodyEffect = params.get("GAMMA");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    bodyEffect !== undefined &&
    (!Number.isFinite(bodyEffect) || bodyEffect < 0.0)
  ) {
    throw new NetlistParseError("MOSFET GAMMA must be finite and non-negative");
  }
  const surfacePotential = params.get("PHI");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    surfacePotential !== undefined &&
    (!Number.isFinite(surfacePotential) || surfacePotential <= 0.0)
  ) {
    throw new NetlistParseError("MOSFET PHI must be finite and positive");
  }
  const width = params.get("W");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    width !== undefined &&
    (!Number.isFinite(width) || width <= 0.0)
  ) {
    throw new NetlistParseError("MOSFET W must be finite and positive");
  }
  const length = params.get("L");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    length !== undefined &&
    (!Number.isFinite(length) || length <= 0.0)
  ) {
    throw new NetlistParseError("MOSFET L must be finite and positive");
  }
  const lateralDiffusion = params.get("LD");
  if (kind === "NMOS" || kind === "PMOS") {
    const effectiveLength = length ?? defaultMosfetLevel1Params().L;
    if (
      lateralDiffusion !== undefined &&
      (!Number.isFinite(lateralDiffusion) ||
        lateralDiffusion < 0.0 ||
        effectiveLength - 2.0 * lateralDiffusion <= 0.0)
    ) {
      throw new NetlistParseError(
        "MOSFET LD must be finite and non-negative with L - 2*LD > 0",
      );
    }
  }
  const saturationCurrent = params.get("IS");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    saturationCurrent !== undefined &&
    (!Number.isFinite(saturationCurrent) || saturationCurrent <= 0.0)
  ) {
    throw new NetlistParseError("MOSFET IS must be finite and positive");
  }
  const drainResistance = params.get("RD");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    drainResistance !== undefined &&
    (!Number.isFinite(drainResistance) || drainResistance < 0.0)
  ) {
    throw new NetlistParseError("MOSFET RD must be finite and non-negative");
  }
  const sourceResistance = params.get("RS");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    sourceResistance !== undefined &&
    (!Number.isFinite(sourceResistance) || sourceResistance < 0.0)
  ) {
    throw new NetlistParseError("MOSFET RS must be finite and non-negative");
  }
  const sheetResistance = params.get("RSH");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    sheetResistance !== undefined &&
    (!Number.isFinite(sheetResistance) || sheetResistance < 0.0)
  ) {
    throw new NetlistParseError("MOSFET RSH must be finite and non-negative");
  }
  const junctionCapacitance = params.get("CJ");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    junctionCapacitance !== undefined &&
    (!Number.isFinite(junctionCapacitance) || junctionCapacitance < 0.0)
  ) {
    throw new NetlistParseError("MOSFET CJ must be finite and non-negative");
  }
  const sidewallCapacitance = params.get("CJSW");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    sidewallCapacitance !== undefined &&
    (!Number.isFinite(sidewallCapacitance) || sidewallCapacitance < 0.0)
  ) {
    throw new NetlistParseError("MOSFET CJSW must be finite and non-negative");
  }
  for (const [name, canonical] of [
    ["CBS", "CBS"],
    ["CJS", "CBS"],
    ["CBD", "CBD"],
    ["CJD", "CBD"],
  ] as const) {
    const capacitance = params.get(name);
    if (
      (kind === "NMOS" || kind === "PMOS") &&
      capacitance !== undefined &&
      (!Number.isFinite(capacitance) || capacitance < 0.0)
    ) {
      throw new NetlistParseError(`MOSFET ${canonical} must be finite and non-negative`);
    }
  }
  const junctionCurrent = params.get("JS");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    junctionCurrent !== undefined &&
    (!Number.isFinite(junctionCurrent) || junctionCurrent < 0.0)
  ) {
    throw new NetlistParseError("MOSFET JS must be finite and non-negative");
  }
  const bulkPotential = params.get("PB");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    bulkPotential !== undefined &&
    (!Number.isFinite(bulkPotential) || bulkPotential <= 0.0)
  ) {
    throw new NetlistParseError("MOSFET PB must be finite and positive");
  }
  const gradingCoefficient = params.get("MJ");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    gradingCoefficient !== undefined &&
    (!Number.isFinite(gradingCoefficient) || gradingCoefficient < 0.0)
  ) {
    throw new NetlistParseError("MOSFET MJ must be finite and non-negative");
  }
  const sidewallGradingCoefficient = params.get("MJSW");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    sidewallGradingCoefficient !== undefined &&
    (!Number.isFinite(sidewallGradingCoefficient) || sidewallGradingCoefficient < 0.0)
  ) {
    throw new NetlistParseError("MOSFET MJSW must be finite and non-negative");
  }
  const forwardBiasCoefficient = params.get("FC");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    forwardBiasCoefficient !== undefined &&
    (!Number.isFinite(forwardBiasCoefficient) ||
      forwardBiasCoefficient < 0.0 ||
      forwardBiasCoefficient >= 1.0)
  ) {
    throw new NetlistParseError("MOSFET FC must be finite and in [0, 1)");
  }
  const flickerNoiseCoefficient = params.get("KF");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    flickerNoiseCoefficient !== undefined &&
    (!Number.isFinite(flickerNoiseCoefficient) || flickerNoiseCoefficient < 0.0)
  ) {
    throw new NetlistParseError("MOSFET KF must be finite and non-negative");
  }
  const flickerNoiseExponent = params.get("AF");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    flickerNoiseExponent !== undefined &&
    (!Number.isFinite(flickerNoiseExponent) || flickerNoiseExponent < 0.0)
  ) {
    throw new NetlistParseError("MOSFET AF must be finite and non-negative");
  }
  const nominalTemperature = params.get("T_NOM") ?? params.get("TNOM");
  if (
    (kind === "NMOS" || kind === "PMOS") &&
    nominalTemperature !== undefined &&
    (!Number.isFinite(nominalTemperature) || nominalTemperature <= 0.0)
  ) {
    throw new NetlistParseError("MOSFET TNOM must be finite and positive");
  }
  return {
    name: fields[1],
    kind,
    params,
  };
}

function parseModelParams(paramsText: string): ReadonlyMap<string, number> {
  const params = new Map<string, number>();
  if (paramsText.trim().length === 0) {
    return params;
  }
  const pattern = /([A-Za-z][A-Za-z0-9_]*)\s*=\s*([^,\s)]+)/g;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(paramsText)) !== null) {
    params.set(match[1].toUpperCase(), parseValue(match[2]));
  }
  const leftover = paramsText.replace(pattern, "").replace(/[,\s]/g, "");
  if (leftover.length > 0 || params.size === 0) {
    throw new NetlistParseError(
      `invalid .model parameter syntax ${JSON.stringify(paramsText)}`,
    );
  }
  return params;
}

function parseElementParams(tokens: readonly string[], label: string): ReadonlyMap<string, number> {
  const params = new Map<string, number>();
  for (const token of tokens) {
    const equals = token.indexOf("=");
    if (equals <= 0 || equals === token.length - 1) {
      throw new NetlistParseError(`invalid ${label} parameter syntax ${JSON.stringify(token)}`);
    }
    params.set(token.slice(0, equals).toUpperCase(), parseValue(token.slice(equals + 1)));
  }
  return params;
}

const MOSFET_PARAM_ALIASES = new Map<string, keyof MosfetLevel1Params>([
  ["VTO", "VT0"],
  ["VTH", "VT0"],
  ["LAM", "LAMBDA"],
  ["NSUB", "N_SUB"],
  ["N", "N_SUB"],
  ["TNOM", "T_NOM"],
  ["UO", "U0"],
  ["CJS", "CBS"],
  ["CJD", "CBD"],
]);

function mosfetParams(
  model: ModelCard,
  instanceParams: ReadonlyMap<string, number>,
): Partial<MosfetLevel1Params> {
  const modelParams = model.params;
  const params: Record<string, number> = {};
  for (const [name, value] of [...modelParams, ...instanceParams]) {
    const key = MOSFET_PARAM_ALIASES.get(name) ?? (name as keyof MosfetLevel1Params);
    if (isMosfetParam(key)) {
      params[key] = value;
    }
  }
  const canonicalParams = new Set(
    [...modelParams, ...instanceParams].map(([name]) => MOSFET_PARAM_ALIASES.get(name) ?? name),
  );
  if (modelParams.has("TOX")) {
    const derivationParams = Object.fromEntries(modelParams);
    derivationParams.U0 = params.U0 ?? 600.0;
    const normalized = normalizeModelCard(model.name, model.kind, derivationParams);
    const derived = mosfetFromModelCard("M", "d", "g", "s", "b", normalized).params;
    for (const key of ["KP", "VT0", "GAMMA", "PHI"] as const) {
      if (!canonicalParams.has(key)) params[key] = derived[key];
    }
  }
  return params;
}

function isMosfetParam(name: keyof MosfetLevel1Params): boolean {
  return [
    "VT0",
    "KP",
    "LAMBDA",
    "GAMMA",
    "PHI",
    "W",
    "L",
    "LD",
    "TOX",
    "U0",
    "RD",
    "RS",
    "RSH",
    "NRD",
    "NRS",
    "AD",
    "AS",
    "PD",
    "PS",
    "CJ",
    "CJSW",
    "JS",
    "PB",
    "MJ",
    "MJSW",
    "FC",
    "KF",
    "AF",
    "IS",
    "N_SUB",
    "T_NOM",
    "CGSO",
    "CGDO",
    "CGBO",
    "CBS",
    "CBD",
  ].includes(name);
}

function parseElement(fields: readonly string[], models: ReadonlyMap<string, ModelCard>): Element {
  const name = fields[0];
  const prefix = elementPrefix(name);
  if (prefix === "R") {
    requireFields(fields, 4, "resistor");
    return resistor(name, fields[1], fields[2], parseValue(fields[3]));
  }
  if (prefix === "C") {
    requireMinFields(fields, 4, "capacitor");
    const params = parseElementParams(fields.slice(4), "capacitor");
    for (const paramName of params.keys()) {
      if (paramName !== "IC") {
        throw new NetlistParseError(
          `unsupported capacitor parameter ${JSON.stringify(paramName)}`,
        );
      }
    }
    return capacitorWithInitialVoltage(
      name,
      fields[1],
      fields[2],
      parseValue(fields[3]),
      params.get("IC") ?? 0.0,
    );
  }
  if (prefix === "L") {
    requireMinFields(fields, 4, "inductor");
    const params = parseElementParams(fields.slice(4), "inductor");
    for (const paramName of params.keys()) {
      if (paramName !== "IC") {
        throw new NetlistParseError(
          `unsupported inductor parameter ${JSON.stringify(paramName)}`,
        );
      }
    }
    return inductorWithInitialCurrent(
      name,
      fields[1],
      fields[2],
      parseValue(fields[3]),
      params.get("IC") ?? 0.0,
    );
  }
  if (prefix === "K") {
    requireFields(fields, 4, "mutual inductor");
    return mutualInductor(name, fields[1], fields[2], parseValue(fields[3]));
  }
  if (prefix === "T") {
    requireMinFields(fields, 6, "transmission line");
    const params = parseElementParams(fields.slice(5), "transmission line");
    for (const paramName of params.keys()) {
      if (paramName !== "Z0" && paramName !== "TD") {
        throw new NetlistParseError(
          `unsupported transmission line parameter ${JSON.stringify(paramName)}`,
        );
      }
    }
    const characteristicImpedance = params.get("Z0");
    if (characteristicImpedance === undefined) {
      throw new NetlistParseError(`${name}: transmission line requires Z0`);
    }
    const delay = params.get("TD");
    if (delay === undefined) {
      throw new NetlistParseError(`${name}: transmission line requires TD`);
    }
    return transmissionLine(
      name,
      fields[1],
      fields[2],
      fields[3],
      fields[4],
      characteristicImpedance,
      delay,
    );
  }
  if (prefix === "V") {
    requireMinFields(fields, 4, "voltage source");
    const source = parseSourceValue(fields.slice(3));
    if (source.waveform !== undefined) {
      return voltageSourceWithWaveform(
        name,
        fields[1],
        fields[2],
        source.dcValue,
        source.waveform,
      );
    }
    return source.ac === undefined
      ? voltageSource(name, fields[1], fields[2], source.dcValue)
      : voltageSourceWithAc(
          name,
          fields[1],
          fields[2],
          source.dcValue,
          source.ac.magnitude,
          source.ac.phaseDegrees,
        );
  }
  if (prefix === "I") {
    requireMinFields(fields, 4, "current source");
    const source = parseSourceValue(fields.slice(3));
    if (source.waveform !== undefined) {
      return currentSourceWithWaveform(
        name,
        fields[1],
        fields[2],
        source.dcValue,
        source.waveform,
      );
    }
    return source.ac === undefined
      ? currentSource(name, fields[1], fields[2], source.dcValue)
      : currentSourceWithAc(
          name,
          fields[1],
          fields[2],
          source.dcValue,
          source.ac.magnitude,
          source.ac.phaseDegrees,
        );
  }
  if (prefix === "D") {
    requireFields(fields, 4, "diode");
    const model = models.get(fields[3].toLowerCase());
    if (model === undefined) {
      throw new NetlistParseError(
        `unknown model ${JSON.stringify(fields[3])} for diode ${JSON.stringify(name)}`,
      );
    }
    if (model.kind !== "D") {
      throw new NetlistParseError(
        `model ${JSON.stringify(model.name)} has kind ${JSON.stringify(model.kind)}, expected "D"`,
      );
    }
    return diode(
      name,
      fields[1],
      fields[2],
      model.params.get("IS") ?? model.params.get("JS") ?? 1.0e-15,
      model.params.get("VT") ?? 0.02585,
      model.params.get("N") ?? 1.0,
      model.params.get("BV"),
      model.params.get("IBV") ?? 1.0e-3,
      model.params.get("CJO") ?? model.params.get("CJ0") ?? 0.0,
      model.params.get("TT") ?? 0.0,
    );
  }
  if (prefix === "Q") {
    requireFields(fields, 5, "BJT");
    const model = models.get(fields[4].toLowerCase());
    if (model === undefined) {
      throw new NetlistParseError(
        `unknown model ${JSON.stringify(fields[4])} for BJT ${JSON.stringify(name)}`,
      );
    }
    if (model.kind !== "NPN" && model.kind !== "PNP") {
      throw new NetlistParseError(
        `model ${JSON.stringify(model.name)} has kind ${JSON.stringify(model.kind)}, expected "NPN" or "PNP"`,
      );
    }
    return bjt(
      name,
      fields[1],
      fields[2],
      fields[3],
      model.kind,
      model.params.get("IS") ?? 1.0e-14,
      model.params.get("BF") ?? model.params.get("BETA_F") ?? 100.0,
      model.params.get("VT") ?? 0.02585,
      model.params.get("CJE") ?? model.params.get("CBE") ?? 0.0,
      model.params.get("CJC") ?? model.params.get("CBC") ?? 0.0,
      model.params.get("TF") ?? 0.0,
      model.params.get("TR") ?? 0.0,
    );
  }
  if (prefix === "J") {
    requireFields(fields, 5, "JFET");
    const model = models.get(fields[4].toLowerCase());
    if (model === undefined) {
      throw new NetlistParseError(
        `unknown model ${JSON.stringify(fields[4])} for JFET ${JSON.stringify(name)}`,
      );
    }
    if (model.kind !== "NJF" && model.kind !== "PJF") {
      throw new NetlistParseError(
        `model ${JSON.stringify(model.name)} has kind ${JSON.stringify(model.kind)}, expected "NJF" or "PJF"`,
      );
    }
    const polarity = model.kind as JfetPolarity;
    return jfet(
      name,
      fields[1],
      fields[2],
      fields[3],
      polarity,
      model.params.get("BETA") ?? model.params.get("B") ?? 1.0e-4,
      model.params.get("VTO") ?? (polarity === "NJF" ? -2.0 : 2.0),
      model.params.get("LAMBDA") ?? 0.0,
      model.params.get("CGS") ?? model.params.get("CGS0") ?? 0.0,
      model.params.get("CGD") ?? model.params.get("CGD0") ?? 0.0,
    );
  }
  if (prefix === "M") {
    requireMinFields(fields, 6, "MOSFET");
    const model = models.get(fields[5].toLowerCase());
    if (model === undefined) {
      throw new NetlistParseError(
        `unknown model ${JSON.stringify(fields[5])} for MOSFET ${JSON.stringify(name)}`,
      );
    }
    if (model.kind !== "NMOS" && model.kind !== "PMOS") {
      throw new NetlistParseError(
        `model ${JSON.stringify(model.name)} has kind ${JSON.stringify(model.kind)}, expected "NMOS" or "PMOS"`,
      );
    }
    const instanceParams = parseElementParams(fields.slice(6), "MOSFET");
    const drainSquares = instanceParams.get("NRD");
    if (drainSquares !== undefined && (!Number.isFinite(drainSquares) || drainSquares < 0.0)) {
      throw new NetlistParseError("MOSFET NRD must be finite and non-negative");
    }
    const sourceSquares = instanceParams.get("NRS");
    if (
      sourceSquares !== undefined &&
      (!Number.isFinite(sourceSquares) || sourceSquares < 0.0)
    ) {
      throw new NetlistParseError("MOSFET NRS must be finite and non-negative");
    }
    const drainArea = instanceParams.get("AD");
    if (drainArea !== undefined && (!Number.isFinite(drainArea) || drainArea < 0.0)) {
      throw new NetlistParseError("MOSFET AD must be finite and non-negative");
    }
    const sourceArea = instanceParams.get("AS");
    if (sourceArea !== undefined && (!Number.isFinite(sourceArea) || sourceArea < 0.0)) {
      throw new NetlistParseError("MOSFET AS must be finite and non-negative");
    }
    const drainPerimeter = instanceParams.get("PD");
    if (
      drainPerimeter !== undefined &&
      (!Number.isFinite(drainPerimeter) || drainPerimeter < 0.0)
    ) {
      throw new NetlistParseError("MOSFET PD must be finite and non-negative");
    }
    const sourcePerimeter = instanceParams.get("PS");
    if (
      sourcePerimeter !== undefined &&
      (!Number.isFinite(sourcePerimeter) || sourcePerimeter < 0.0)
    ) {
      throw new NetlistParseError("MOSFET PS must be finite and non-negative");
    }
    return mosfet(
      name,
      fields[1],
      fields[2],
      fields[3],
      fields[4],
      model.kind,
      mosfetParams(model, instanceParams),
    );
  }
  if (prefix === "G") {
    requireFields(fields, 6, "VCCS");
    return vccs(name, fields[1], fields[2], fields[3], fields[4], parseValue(fields[5]));
  }
  if (prefix === "E") {
    requireFields(fields, 6, "VCVS");
    return vcvs(name, fields[1], fields[2], fields[3], fields[4], parseValue(fields[5]));
  }
  if (prefix === "F") {
    requireFields(fields, 5, "CCCS");
    return cccs(name, fields[1], fields[2], fields[3], parseValue(fields[4]));
  }
  if (prefix === "H") {
    requireFields(fields, 5, "CCVS");
    return ccvs(name, fields[1], fields[2], fields[3], parseValue(fields[4]));
  }
  throw new NetlistParseError(`unsupported element ${JSON.stringify(name)}`);
}

function validateMutualInductors(circuit: Circuit): void {
  const inductors = new Set<string>();
  for (const element of circuit.elements()) {
    if (element.kind === "inductor") {
      inductors.add(element.name);
    }
  }
  for (const element of circuit.elements()) {
    if (element.kind !== "mutual-inductor") {
      continue;
    }
    if (!Number.isFinite(element.coupling)) {
      throw new NetlistParseError(`${element.name}: coupling must be finite`);
    }
    if (Math.abs(element.coupling) >= 1.0) {
      throw new NetlistParseError(`${element.name}: coupling magnitude must be less than one`);
    }
    if (element.primary === element.secondary) {
      throw new NetlistParseError(`${element.name}: coupled inductors must be distinct`);
    }
    if (!inductors.has(element.primary)) {
      throw new NetlistParseError(`${element.name}: referenced inductor ${JSON.stringify(element.primary)} was not found`);
    }
    if (!inductors.has(element.secondary)) {
      throw new NetlistParseError(`${element.name}: referenced inductor ${JSON.stringify(element.secondary)} was not found`);
    }
  }
}

function validateTransmissionLines(circuit: Circuit): void {
  for (const element of circuit.elements()) {
    if (element.kind !== "transmission-line") {
      continue;
    }
    if (!Number.isFinite(element.characteristicImpedanceOhms)) {
      throw new NetlistParseError(`${element.name}: characteristic impedance must be finite`);
    }
    if (element.characteristicImpedanceOhms <= 0.0) {
      throw new NetlistParseError(`${element.name}: characteristic impedance must be positive`);
    }
    if (!Number.isFinite(element.delaySeconds)) {
      throw new NetlistParseError(`${element.name}: delay must be finite`);
    }
    if (element.delaySeconds <= 0.0) {
      throw new NetlistParseError(`${element.name}: delay must be positive`);
    }
  }
}

function startSubckt(
  fields: readonly string[],
  lineNumber: number,
  subckts: ReadonlyMap<string, SubcktDefinition>,
): SubcktDefinition {
  requireMinFields(fields, 3, ".subckt");
  const name = fields[1];
  if (subckts.has(name.toLowerCase())) {
    throw new NetlistParseError(`duplicate .subckt definition ${JSON.stringify(name)}`);
  }
  return { name, pins: fields.slice(2), body: [], lineNumber };
}

function finishSubckt(definition: SubcktDefinition, fields: readonly string[]): void {
  if (fields.length > 2) {
    throw new NetlistParseError(".ends expects at most a subcircuit name");
  }
  if (fields.length === 2 && fields[1].toLowerCase() !== definition.name.toLowerCase()) {
    throw new NetlistParseError(
      `.ends ${JSON.stringify(fields[1])} does not match .subckt ${JSON.stringify(definition.name)}`,
    );
  }
}

function expandSubcktInstance(
  fields: readonly string[],
  subckts: ReadonlyMap<string, SubcktDefinition>,
  stack: readonly string[],
  models: ReadonlyMap<string, ModelCard>,
): Element[] {
  requireMinFields(fields, 3, "subcircuit instance");
  const instanceName = fields[0];
  const subcktName = fields[fields.length - 1];
  const definition = subckts.get(subcktName.toLowerCase());
  if (definition === undefined) {
    throw new NetlistParseError(`unknown subcircuit ${JSON.stringify(subcktName)}`);
  }
  if (stack.includes(definition.name.toLowerCase())) {
    const cycle = [...stack, definition.name.toLowerCase()].join(" -> ");
    throw new NetlistParseError(`recursive subcircuit expansion is not supported: ${cycle}`);
  }

  const actualNodes = fields.slice(1, -1);
  if (actualNodes.length !== definition.pins.length) {
    throw new NetlistParseError(
      `subcircuit ${JSON.stringify(definition.name)} expects ${definition.pins.length} pins, got ${actualNodes.length}`,
    );
  }

  const nodeMap = new Map<string, string>();
  for (let index = 0; index < definition.pins.length; index++) {
    nodeMap.set(definition.pins[index], actualNodes[index]);
    nodeMap.set(definition.pins[index].toLowerCase(), actualNodes[index]);
  }

  const elements: Element[] = [];
  const nextStack = [...stack, definition.name.toLowerCase()];
  for (const statement of definition.body) {
    if (statement.fields[0].startsWith(".")) {
      throw new NetlistParseError(
        `line ${statement.lineNumber}: directives inside .subckt are not supported`,
      );
    }
    const localFields = mapSubcktFields(statement.fields, instanceName, nodeMap);
    if (elementPrefix(statement.fields[0]) === "X") {
      elements.push(...expandSubcktInstance(localFields, subckts, nextStack, models));
    } else {
      elements.push(parseElement(localFields, models));
    }
  }
  return elements;
}

function mapSubcktFields(
  fields: readonly string[],
  instanceName: string,
  nodeMap: ReadonlyMap<string, string>,
): string[] {
  const mapped = [`${instanceName}.${fields[0]}`, ...fields.slice(1)];
  const prefix = fields[0][0].toUpperCase();
  if (["R", "C", "L", "V", "I", "D"].includes(prefix)) {
    requireMinFields(fields, 3, "subcircuit element");
    mapped[1] = mapSubcktNode(fields[1], instanceName, nodeMap);
    mapped[2] = mapSubcktNode(fields[2], instanceName, nodeMap);
  } else if (prefix === "Q" || prefix === "J") {
    requireMinFields(fields, 4, prefix === "Q" ? "subcircuit BJT" : "subcircuit JFET");
    mapped[1] = mapSubcktNode(fields[1], instanceName, nodeMap);
    mapped[2] = mapSubcktNode(fields[2], instanceName, nodeMap);
    mapped[3] = mapSubcktNode(fields[3], instanceName, nodeMap);
  } else if (prefix === "M") {
    requireMinFields(fields, 5, "subcircuit MOSFET");
    for (let index = 1; index < 5; index++) {
      mapped[index] = mapSubcktNode(fields[index], instanceName, nodeMap);
    }
  } else if (prefix === "E" || prefix === "G") {
    requireMinFields(fields, 5, "subcircuit controlled source");
    for (let index = 1; index < 5; index++) {
      mapped[index] = mapSubcktNode(fields[index], instanceName, nodeMap);
    }
  } else if (prefix === "F" || prefix === "H") {
    requireMinFields(fields, 4, "subcircuit current-controlled source");
    mapped[1] = mapSubcktNode(fields[1], instanceName, nodeMap);
    mapped[2] = mapSubcktNode(fields[2], instanceName, nodeMap);
    mapped[3] = mapSubcktSourceRef(fields[3], instanceName);
  } else if (prefix === "K") {
    requireFields(fields, 4, "subcircuit mutual inductor");
    mapped[1] = mapSubcktSourceRef(fields[1], instanceName);
    mapped[2] = mapSubcktSourceRef(fields[2], instanceName);
  } else if (prefix === "T") {
    requireMinFields(fields, 6, "subcircuit transmission line");
    for (let index = 1; index < 5; index++) {
      mapped[index] = mapSubcktNode(fields[index], instanceName, nodeMap);
    }
  } else if (prefix === "X") {
    for (let index = 1; index < fields.length - 1; index++) {
      mapped[index] = mapSubcktNode(fields[index], instanceName, nodeMap);
    }
  }
  return mapped;
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

function elementPrefix(name: string): string {
  const localName = name.split(".").at(-1) ?? name;
  return localName[0].toUpperCase();
}

function parseSourceValue(fields: readonly string[]): SourceSpec {
  if (fields.length === 0) {
    throw new NetlistParseError("source is missing a value");
  }
  if (fields[0].toUpperCase() === "DC") {
    if (fields.length < 2) {
      throw new NetlistParseError("DC source form requires a value");
    }
    return {
      dcValue: parseValue(fields[1]),
      ac: parseSourceAc(fields.slice(2)),
    };
  }
  if (fields[0].toUpperCase() === "AC") {
    return {
      dcValue: 0.0,
      ac: parseSourceAc(fields),
    };
  }
  if (fields.length === 1 && fields[0].includes("(")) {
    const waveform = parseWaveform(fields[0]);
    return { dcValue: waveform.valueAt(0.0), waveform };
  }
  if (/^(PWL|SIN|PULSE|EXP)\(/i.test(fields[0])) {
    const waveform = parseWaveform(fields.join(" "));
    return { dcValue: waveform.valueAt(0.0), waveform };
  }
  return {
    dcValue: parseValue(fields[0]),
    ac: parseSourceAc(fields.slice(1)),
  };
}

function parseSourceAc(
  fields: readonly string[],
): { readonly magnitude: number; readonly phaseDegrees: number } | undefined {
  if (fields.length === 0) {
    return undefined;
  }
  if (fields[0].toUpperCase() !== "AC") {
    throw new NetlistParseError(
      `unsupported source suffix ${JSON.stringify(fields.join(" "))}`,
    );
  }
  if (fields.length < 2) {
    throw new NetlistParseError("AC source form requires a magnitude");
  }
  if (fields.length > 3) {
    throw new NetlistParseError("AC source form accepts magnitude and optional phase");
  }
  return {
    magnitude: parseValue(fields[1]),
    phaseDegrees: fields.length >= 3 ? parseValue(fields[2]) : 0.0,
  };
}

function parseWaveform(token: string): Waveform {
  const match = /^\s*([A-Za-z]+)\((.*)\)\s*$/.exec(token);
  if (match === null) {
    throw new NetlistParseError(`invalid source waveform ${JSON.stringify(token)}`);
  }
  const kind = match[1].toUpperCase();
  const values = match[2]
    .trim()
    .split(/[\s,]+/)
    .filter((part) => part.length > 0)
    .map(parseValue);
  if (kind === "PWL") {
    if (values.length < 4 || values.length % 2 !== 0) {
      throw new NetlistParseError("PWL requires time/value pairs");
    }
    const points: Array<readonly [number, number]> = [];
    for (let index = 0; index < values.length; index += 2) {
      points.push([values[index], values[index + 1]]);
    }
    return new PwlWaveform(points);
  }
  if (kind === "SIN") {
    const padded = pad(values, 5, 0.0);
    return new SinWaveform(
      padded[0],
      values.length >= 2 ? padded[1] : 1.0,
      values.length >= 3 ? padded[2] : 1.0,
      padded[3],
      padded[4],
    );
  }
  if (kind === "PULSE") {
    const padded = pad(values, 7, 0.0);
    return new PulseWaveform(
      padded[0],
      values.length >= 2 ? padded[1] : 1.0,
      padded[2],
      padded[3],
      padded[4],
      values.length >= 6 ? padded[5] : 0.5,
      values.length >= 7 ? padded[6] : 1.0,
    );
  }
  if (kind === "EXP") {
    const padded = pad(values, 6, 0.0);
    return new ExpWaveform(
      padded[0],
      values.length >= 2 ? padded[1] : 1.0,
      padded[2],
      values.length >= 4 ? padded[3] : 1.0,
      values.length >= 5 ? padded[4] : 1.0,
      values.length >= 6 ? padded[5] : 1.0,
    );
  }
  throw new NetlistParseError(`unsupported source waveform ${JSON.stringify(kind)}`);
}

function parseDirective(fields: readonly string[]): Analysis {
  const directive = fields[0].toLowerCase();
  if (directive === ".op") {
    requireFields(fields, 1, ".op");
    return { kind: "op" };
  }
  if (directive === ".tran") {
    requireMinFields(fields, 3, ".tran");
    const method = parseTranMethodOptions(fields.slice(3));
    const card: TranAnalysis = {
      kind: "tran",
      timeStep: parseValue(fields[1]),
      stopTime: parseValue(fields[2]),
    };
    return method === undefined ? card : { ...card, method };
  }
  if (directive === ".dc") {
    requireFields(fields, 5, ".dc");
    return {
      kind: "dc",
      sourceName: fields[1],
      start: parseValue(fields[2]),
      stop: parseValue(fields[3]),
      step: parseValue(fields[4]),
    };
  }
  if (directive === ".ac") {
    requireFields(fields, 5, ".ac");
    return {
      kind: "ac",
      mode: fields[1].toLowerCase(),
      points: Math.trunc(parseValue(fields[2])),
      startHz: parseValue(fields[3]),
      stopHz: parseValue(fields[4]),
    };
  }
  if (directive === ".tf") {
    requireFields(fields, 3, ".tf");
    return {
      kind: "tf",
      outputNode: parseVoltageProbe(fields[1], ".tf"),
      inputSource: fields[2],
    };
  }
  if (directive === ".sens") {
    requireFields(fields, 2, ".sens");
    return {
      kind: "sens",
      outputNode: parseVoltageProbe(fields[1], ".sens"),
    };
  }
  if (directive === ".mc") {
    requireMinFields(fields, 3, ".mc");
    requireMaxFields(fields, 6, ".mc");
    const distribution = fields.length >= 5 ? fields[4].toLowerCase() : "gaussian";
    if (distribution !== "gaussian" && distribution !== "uniform") {
      throw new NetlistParseError(
        `.mc distribution must be "gaussian" or "uniform", got ${JSON.stringify(fields[4])}`,
      );
    }
    return {
      kind: "mc",
      outputNode: parseVoltageProbe(fields[1], ".mc"),
      nTrials: Math.trunc(parseValue(fields[2])),
      tolerance: fields.length >= 4 ? parseValue(fields[3]) : 0.05,
      distribution,
      seed: fields.length >= 6 ? Math.trunc(parseValue(fields[5])) : undefined,
    };
  }
  if (directive === ".noise") {
    requireMinFields(fields, 3, ".noise");
    const frequenciesHz: number[] = [];
    let temperature = 300.0;
    let temperatureIsExplicit = false;
    let tailIndex = 3;
    while (tailIndex < fields.length) {
      const token = fields[tailIndex];
      const lowerToken = token.toLowerCase();
      if (lowerToken === "temp") {
        if (tailIndex + 1 >= fields.length) {
          throw new NetlistParseError(".noise temp requires a temperature value");
        }
        temperature = parseValue(fields[tailIndex + 1]);
        temperatureIsExplicit = true;
        tailIndex += 2;
      } else if (lowerToken.startsWith("temp=")) {
        temperature = parseValue(token.split("=", 2)[1]);
        temperatureIsExplicit = true;
        tailIndex += 1;
      } else {
        frequenciesHz.push(parseValue(token));
        tailIndex += 1;
      }
    }
    return {
      kind: "noise",
      outputNode: parseVoltageProbe(fields[1], ".noise"),
      inputSource: fields[2],
      frequenciesHz,
      temperature,
      ...(temperatureIsExplicit ? { temperatureIsExplicit: true } : {}),
    };
  }
  if (directive === ".temp") {
    requireMinFields(fields, 2, ".temp");
    return {
      kind: "temp",
      temperaturesCelsius: fields.slice(1).map(parseValue),
    };
  }
  if (directive === ".print") {
    requireMinFields(fields, 3, ".print");
    return {
      kind: "print",
      analysis: fields[1].toLowerCase(),
      probes: fields.slice(2).map((token) => parseOutputProbe(token, ".print")),
    };
  }
  if (directive === ".plot") {
    requireMinFields(fields, 3, ".plot");
    return {
      kind: "plot",
      analysis: fields[1].toLowerCase(),
      probes: fields.slice(2).map((token) => parseOutputProbe(token, ".plot")),
    };
  }
  if (directive === ".save") {
    requireMinFields(fields, 2, ".save");
    return {
      kind: "save",
      probes: fields.slice(1).map((token) => parseOutputProbe(token, ".save")),
    };
  }
  if (directive === ".probe") {
    return parseProbeCard(fields);
  }
  if (directive === ".measure" || directive === ".meas") {
    return parseMeasureCard(fields);
  }
  if (directive === ".four") {
    requireMinFields(fields, 3, ".four");
    return {
      kind: "four",
      frequencyHz: parseValue(fields[1]),
      probes: fields.slice(2).map((token) => parseOutputProbe(token, ".four")),
    };
  }
  if (directive === ".disto") {
    requireMinFields(fields, 6, ".disto");
    return {
      kind: "disto",
      mode: fields[1].toLowerCase(),
      points: Math.trunc(parseValue(fields[2])),
      startHz: parseValue(fields[3]),
      stopHz: parseValue(fields[4]),
      probes: fields.slice(5).map((token) => parseOutputProbe(token, ".disto")),
    };
  }
  if (directive === ".pz") {
    requireMinFields(fields, 3, ".pz");
    requireMaxFields(fields, 4, ".pz");
    const poleZeroKind = fields.length >= 4 ? fields[3].toLowerCase() : "pz";
    if (poleZeroKind !== "pole" && poleZeroKind !== "zero" && poleZeroKind !== "pz") {
      throw new NetlistParseError(
        `.pz kind must be "pole", "zero", or "pz", got ${JSON.stringify(fields[3])}`,
      );
    }
    return {
      kind: "pz",
      outputNode: parseVoltageProbe(fields[1], ".pz"),
      inputSource: fields[2],
      poleZeroKind,
    };
  }
  if (directive === ".options") {
    requireMinFields(fields, 2, ".options");
    return { kind: "options", values: parseOptions(fields.slice(1)) };
  }
  throw new NetlistParseError(`unsupported directive ${JSON.stringify(fields[0])}`);
}

function parseProbeCard(fields: readonly string[]): ProbeAnalysis {
  requireMinFields(fields, 2, ".probe");
  let analysis: string | undefined;
  let probeTokens = fields.slice(1);
  if (fields.length >= 3 && isAnalysisSelector(fields[1])) {
    analysis = fields[1].toLowerCase();
    probeTokens = fields.slice(2);
  }
  return {
    kind: "probe",
    ...(analysis === undefined ? {} : { analysis }),
    probes: probeTokens.map((token) => parseOutputProbe(token, ".probe")),
  };
}

function parseMeasureCard(fields: readonly string[]): MeasureAnalysis {
  const directive = fields[0].toLowerCase();
  requireMinFields(fields, 5, directive);
  const operation = parseMeasureOperation(fields[3], directive);
  const options = parseMeasureOptions(fields.slice(5), directive);
  if (operation === "find" && !options.has("at") && fields[1].toLowerCase() !== "op" && fields[1].toLowerCase() !== "dcop") {
    throw new NetlistParseError(`${directive} FIND requires AT=<value>`);
  }
  if (operation !== "find" && options.has("at")) {
    throw new NetlistParseError(`${directive} ${operation.toUpperCase()} does not support AT=<value>`);
  }
  return {
    kind: "measure",
    analysis: fields[1].toLowerCase(),
    name: fields[2],
    operation,
    probe: parseOutputProbe(fields[4], directive),
    ...(options.has("at") ? { at: options.get("at")! } : {}),
    ...(options.has("from") ? { start: options.get("from")! } : {}),
    ...(options.has("to") ? { stop: options.get("to")! } : {}),
  };
}

function parseMeasureOperation(token: string, directive: string): MeasureOperation {
  const operation = token.toLowerCase();
  if (
    operation !== "find" &&
    operation !== "max" &&
    operation !== "min" &&
    operation !== "avg" &&
    operation !== "rms"
  ) {
    throw new NetlistParseError(
      `${directive} operation must be FIND, MAX, MIN, AVG, or RMS, got ${JSON.stringify(token)}`,
    );
  }
  return operation;
}

function parseMeasureOptions(tokens: readonly string[], directive: string): ReadonlyMap<string, number> {
  const options = new Map<string, number>();
  for (const token of tokens) {
    if (!token.includes("=")) {
      throw new NetlistParseError(`${directive} option must be KEY=value, got ${JSON.stringify(token)}`);
    }
    const equalsIndex = token.indexOf("=");
    const key = token.slice(0, equalsIndex).trim().toLowerCase();
    const rawValue = token.slice(equalsIndex + 1);
    if (key !== "at" && key !== "from" && key !== "to") {
      throw new NetlistParseError(`${directive} unsupported option ${JSON.stringify(key)}`);
    }
    if (options.has(key)) {
      throw new NetlistParseError(`${directive} duplicate option ${JSON.stringify(key)}`);
    }
    if (rawValue === "") {
      throw new NetlistParseError(`${directive} option ${JSON.stringify(key)} requires a value`);
    }
    options.set(key, parseValue(rawValue));
  }
  return options;
}

function isAnalysisSelector(token: string): boolean {
  return ["op", "dcop", "dc", "ac", "tran", "transient"].includes(token.toLowerCase());
}

function parseOptions(tokens: readonly string[]): ReadonlyMap<string, OptionValue> {
  const values = new Map<string, OptionValue>();
  for (const token of tokens) {
    if (token.includes("=")) {
      const equalsIndex = token.indexOf("=");
      const rawKey = token.slice(0, equalsIndex);
      const rawValue = token.slice(equalsIndex + 1);
      const key = rawKey.trim().toLowerCase();
      if (key.length === 0) {
        throw new NetlistParseError(`.options contains empty option name in ${JSON.stringify(token)}`);
      }
      if (rawValue === "") {
        throw new NetlistParseError(`.options ${JSON.stringify(key)} requires a value`);
      }
      values.set(
        key,
        key === "method" ? parseTransientMethod(rawValue, ".options method") : parseOptionValue(rawValue),
      );
    } else {
      const key = token.trim().toLowerCase();
      if (key.length === 0) {
        throw new NetlistParseError(".options contains an empty flag");
      }
      values.set(key, true);
    }
  }
  return values;
}

function parseTranMethodOptions(tokens: readonly string[]): TransientMethod | undefined {
  let method: TransientMethod | undefined;
  for (const token of tokens) {
    if (!token.includes("=")) {
      throw new NetlistParseError(
        `.tran unsupported trailing option ${JSON.stringify(token)}; use method=<euler|trap|gear2>`,
      );
    }
    const equalsIndex = token.indexOf("=");
    const key = token.slice(0, equalsIndex).trim().toLowerCase();
    const rawValue = token.slice(equalsIndex + 1);
    if (key !== "method") {
      throw new NetlistParseError(`.tran unsupported option ${JSON.stringify(key)}`);
    }
    if (rawValue === "") {
      throw new NetlistParseError(".tran method requires a value");
    }
    method = parseTransientMethod(rawValue, ".tran method");
  }
  return method;
}

function parseTransientMethod(rawValue: string, context: string): TransientMethod {
  const method = rawValue.trim().toLowerCase();
  if (method === "euler" || method === "trap" || method === "gear2") {
    return method;
  }
  throw new NetlistParseError(
    `${context} must be euler, trap, or gear2, got ${JSON.stringify(rawValue)}`,
  );
}

function parseOptionValue(rawValue: string): OptionValue {
  try {
    return parseValue(rawValue);
  } catch {
    return rawValue;
  }
}

function parseVoltageProbe(token: string, directive: string): string {
  const match = /^v\(([^()\s]+)\)$/i.exec(token);
  if (match === null) {
    throw new NetlistParseError(
      `${directive} output must be a voltage probe V(node), got ${JSON.stringify(token)}`,
    );
  }
  return match[1];
}

function parseOutputProbe(token: string, directive: string): OutputProbe {
  const match = /^([vi])\(([^()\s]+)\)$/i.exec(token);
  if (match === null) {
    throw new NetlistParseError(
      `${directive} probe must be V(node) or I(source), got ${JSON.stringify(token)}`,
    );
  }
  return {
    kind: match[1].toLowerCase() === "v" ? "voltage" : "current",
    target: match[2],
  };
}

function splitFields(line: string): string[] {
  const fields: string[] = [];
  let current = "";
  let depth = 0;
  for (const char of line) {
    if (/\s/.test(char) && depth === 0) {
      if (current.length > 0) {
        fields.push(current);
        current = "";
      }
      continue;
    }
    if (char === "(") {
      depth += 1;
    } else if (char === ")") {
      depth -= 1;
      if (depth < 0) {
        throw new NetlistParseError("unmatched closing parenthesis");
      }
    }
    current += char;
  }
  if (depth !== 0) {
    throw new NetlistParseError("unclosed parenthesis");
  }
  if (current.length > 0) {
    fields.push(current);
  }
  return fields;
}

function stripInlineComment(line: string): string {
  return line.split(";", 1)[0];
}

function lineError(lineNumber: number, error: unknown): NetlistParseError {
  const message = error instanceof Error ? error.message : String(error);
  return new NetlistParseError(`line ${lineNumber}: ${message}`);
}

function requireFields(fields: readonly string[], count: number, label: string): void {
  if (fields.length !== count) {
    throw new NetlistParseError(`${label} expects ${count} fields, got ${fields.length}`);
  }
}

function requireMinFields(fields: readonly string[], count: number, label: string): void {
  if (fields.length < count) {
    throw new NetlistParseError(
      `${label} expects at least ${count} fields, got ${fields.length}`,
    );
  }
}

function requireMaxFields(fields: readonly string[], count: number, label: string): void {
  if (fields.length > count) {
    throw new NetlistParseError(`${label} expects at most ${count} fields, got ${fields.length}`);
  }
}

function pad(values: readonly number[], count: number, defaultValue: number): number[] {
  return [
    ...values,
    ...Array.from({ length: Math.max(0, count - values.length) }, () => defaultValue),
  ];
}

export { dcOp };
