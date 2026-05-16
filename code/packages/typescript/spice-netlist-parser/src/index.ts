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
  dcOp,
  inductorWithInitialCurrent,
  mosfet,
  resistor,
  vccs,
  vcvs,
  voltageSource,
  voltageSourceWithAc,
  voltageSourceWithWaveform,
  type Element,
  type MosfetLevel1Params,
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
}

export type Analysis =
  | OpAnalysis
  | TranAnalysis
  | DcAnalysis
  | AcAnalysis
  | TfAnalysis
  | SensAnalysis
  | McAnalysis
  | NoiseAnalysis;

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

  return new ParsedNetlist(circuit, analyses, models, title);
}

export const parse = parseNetlist;

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
  return {
    name: fields[1],
    kind: match[1].toUpperCase(),
    params: parseModelParams(paramsText),
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
  ["NSUB", "N_SUB"],
  ["N", "N_SUB"],
  ["TNOM", "T_NOM"],
]);

function mosfetParams(
  modelParams: ReadonlyMap<string, number>,
  instanceParams: ReadonlyMap<string, number>,
): Partial<MosfetLevel1Params> {
  const params: Record<string, number> = {};
  for (const [name, value] of [...modelParams, ...instanceParams]) {
    const key = MOSFET_PARAM_ALIASES.get(name) ?? (name as keyof MosfetLevel1Params);
    if (isMosfetParam(key)) {
      params[key] = value;
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
    "IS",
    "N_SUB",
    "T_NOM",
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
      model.params.get("IS") ?? 1.0e-15,
      model.params.get("VT") ?? 0.02585,
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
    return mosfet(
      name,
      fields[1],
      fields[2],
      fields[3],
      fields[4],
      model.kind,
      mosfetParams(model.params, parseElementParams(fields.slice(6), "MOSFET")),
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
  } else if (prefix === "Q") {
    requireMinFields(fields, 4, "subcircuit BJT");
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
    requireFields(fields, 3, ".tran");
    return { kind: "tran", timeStep: parseValue(fields[1]), stopTime: parseValue(fields[2]) };
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
    let tailIndex = 3;
    while (tailIndex < fields.length) {
      const token = fields[tailIndex];
      const lowerToken = token.toLowerCase();
      if (lowerToken === "temp") {
        if (tailIndex + 1 >= fields.length) {
          throw new NetlistParseError(".noise temp requires a temperature value");
        }
        temperature = parseValue(fields[tailIndex + 1]);
        tailIndex += 2;
      } else if (lowerToken.startsWith("temp=")) {
        temperature = parseValue(token.split("=", 2)[1]);
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
    };
  }
  throw new NetlistParseError(`unsupported directive ${JSON.stringify(fields[0])}`);
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
