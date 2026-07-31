const PIVOT_EPSILON = 1.0e-12;
const SPARSE_SOLVER_THRESHOLD = 30;
const DEFAULT_NEWTON_STEP_LIMIT = 5.0;
const TWO_PI = Math.PI * 2.0;
const BOLTZMANN = 1.380_649e-23;
const ELECTRON_CHARGE = 1.602_176_634e-19;
const MOSFET_CHANNEL_NOISE_GAMMA = 2.0 / 3.0;
const DIGITAL_BRIDGE_TIME_EPSILON = 1.0e-18;
const OXIDE_PERMITTIVITY = 3.453133e-11;
const SILICON_PERMITTIVITY = 11.70 * 8.854_214_871e-12;
const INTRINSIC_CARRIER_DENSITY_PER_CUBIC_METER = 1.45e16;
const CUBIC_CENTIMETERS_PER_CUBIC_METER = 1.0e6;

function siliconBandGapElectronVolts(temperatureKelvin: number): number {
  return 1.16 - (7.02e-4 * temperatureKelvin * temperatureKelvin) / (temperatureKelvin + 1108.0);
}

const SPICE_SUFFIX_FACTORS: Readonly<Record<string, number>> = Object.freeze({
  t: 1.0e12,
  g: 1.0e9,
  meg: 1.0e6,
  k: 1.0e3,
  m: 1.0e-3,
  mil: 25.4e-6,
  u: 1.0e-6,
  n: 1.0e-9,
  p: 1.0e-12,
  f: 1.0e-15,
});

export const BERKELEY_SPICE_GRAMMAR_NAME = "berkeley-spice-logical-card";
export const BERKELEY_SPICE_GRAMMAR_VERSION = 1;
export const BERKELEY_SPICE_TOKEN_GRAMMAR = String.raw`# Berkeley SPICE logical-card token grammar.
# @version 1
# @case_insensitive true
#
# This grammar targets normalized Berkeley SPICE logical cards: physical deck
# preprocessing owns title-line capture, column-1 comments, blank physical
# lines, and leading ${"`"}+${"`"} continuations. The token stream below is therefore
# card-oriented and deliberately preserves device/model atoms for the semantic
# lowerer instead of trying to encode all SPICE device arity in the lexer.

skip:
  WHITESPACE = /[ \t\r]+/

# Quoted and braced expressions must win before the generic ATOM token.
QUOTED_STRING = /"([^"\\\n]|\\.)*"/
BRACED_EXPR   = /\{[^}\n]*\}/

# Berkeley/SPICE-style scalar with optional engineering suffix. Semantic
# resolution owns suffix meaning (${ "`" }m${ "`" } vs ${ "`" }meg${ "`" }, temperature units, etc.).
NUMBER = /[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?[a-zA-Z]*/

# Known dot cards for the Berkeley compatibility base plus the already-present
# post-1970s analysis/output footholds. These appear before DOT so ${"`"}.op${"`"} is not
# tokenized as DOT + ATOM.
DOT_END     = ".end"
DOT_ENDS    = ".ends"
DOT_SUBCKT  = ".subckt"
DOT_MODEL   = ".model"
DOT_PARAM   = ".param"
DOT_FUNC    = ".func"
DOT_OPTIONS = ".options"
DOT_TEMP    = ".temp"
DOT_IC      = ".ic"
DOT_NODESET = ".nodeset"
DOT_OP      = ".op"
DOT_DC      = ".dc"
DOT_AC      = ".ac"
DOT_TRAN    = ".tran"
DOT_TF      = ".tf"
DOT_SENS    = ".sens"
DOT_NOISE   = ".noise"
DOT_DISTO   = ".disto"
DOT_PZ      = ".pz"
DOT_PRINT   = ".print"
DOT_PLOT    = ".plot"
DOT_SAVE    = ".save"
DOT_PROBE   = ".probe"
DOT_MEASURE = ".measure"
DOT_MEAS    = ".meas"
DOT_FOUR    = ".four"
DOT_INCLUDE = ".include"
DOT_LIB     = ".lib"
DOT_CONTROL = ".control"
DOT_ENDC    = ".endc"

LPAREN = "("
RPAREN = ")"
COMMA  = ","
EQUALS = "="
DOT    = "."

# Generic card atom. This is intentionally broad because SPICE node names,
# device names, model names, vector names, and source waveform arguments vary
# across dialects. Semantic lowering classifies atoms by card kind.
ATOM = /[^ \t\r\n()=,"{}]+/
`;
export const BERKELEY_SPICE_PARSER_GRAMMAR = String.raw`# Berkeley SPICE logical-card parser grammar.
# @version 1
#
# Input is a stream of normalized logical cards. The grammar recognizes the
# stable card shapes and leaves device arity, model parameter legality,
# expression evaluation, include/lib resolution, and dialect-specific behavior
# to semantic passes.

deck = { line } [ end_card ] EOF ;

line = blank_line
     | subckt_block
     | model_card
     | param_card
     | func_card
     | options_card
     | condition_card
     | analysis_card
     | output_card
     | source_card
     | control_card
     | unknown_directive_card
     | element_card
     ;

blank_line = NEWLINE ;

end_card = DOT_END NEWLINE ;

subckt_block = subckt_card { line } ends_card ;
subckt_card  = DOT_SUBCKT ATOM { card_item } NEWLINE ;
ends_card    = DOT_ENDS [ ATOM ] NEWLINE ;

model_card = DOT_MODEL ATOM ATOM [ parameter_list ] NEWLINE ;

param_card   = DOT_PARAM { assignment } NEWLINE ;
func_card    = DOT_FUNC function_signature card_value NEWLINE ;
options_card = DOT_OPTIONS { option_item } NEWLINE ;

condition_card = ( DOT_TEMP | DOT_IC | DOT_NODESET ) { option_item } NEWLINE ;

analysis_card = ( DOT_OP
                | DOT_DC
                | DOT_AC
                | DOT_TRAN
                | DOT_TF
                | DOT_SENS
                | DOT_NOISE
                | DOT_DISTO
                | DOT_PZ
                ) { card_item } NEWLINE ;

output_card = ( DOT_PRINT
              | DOT_PLOT
              | DOT_SAVE
              | DOT_PROBE
              | DOT_MEASURE
              | DOT_MEAS
              | DOT_FOUR
              ) { card_item } NEWLINE ;

source_card = ( DOT_INCLUDE | DOT_LIB ) { card_item } NEWLINE ;

control_card = DOT_CONTROL NEWLINE { control_line } DOT_ENDC NEWLINE ;
control_line = { card_item } NEWLINE ;

unknown_directive_card = DOT ATOM { card_item } NEWLINE ;

element_card = ATOM { card_item } NEWLINE ;

parameter_list = LPAREN [ option_item { [ COMMA ] option_item } ] RPAREN ;

option_item = assignment | card_value | waveform_call ;
assignment  = ATOM EQUALS card_value ;

function_signature = ATOM LPAREN [ ATOM { COMMA ATOM } ] RPAREN ;

waveform_call = ATOM LPAREN [ card_item { [ COMMA ] card_item } ] RPAREN ;

card_item = option_item
          | parameter_list
          | card_value
          ;

card_value = NUMBER
           | ATOM
           | QUOTED_STRING
           | BRACED_EXPR
           ;
`;

export interface BerkeleySourceSpan {
  readonly startLine: number;
  readonly startColumn: number;
  readonly endLine: number;
  readonly endColumn: number;
}

export type BerkeleyDiagnosticSeverity = "error" | "warning" | "note";

export interface BerkeleySyntaxDiagnostic {
  readonly code: string;
  readonly severity: BerkeleyDiagnosticSeverity;
  readonly message: string;
  readonly span?: BerkeleySourceSpan;
}

export type BerkeleyCardKind =
  | "element"
  | "model"
  | "subcktStart"
  | "subcktEnd"
  | "end"
  | "param"
  | "func"
  | "options"
  | "condition"
  | "analysis"
  | "output"
  | "source"
  | "controlStart"
  | "controlEnd"
  | "unknownDirective";

export interface BerkeleySyntaxToken {
  readonly kind: string;
  readonly text: string;
  readonly span: BerkeleySourceSpan;
}

export interface BerkeleyLogicalCard {
  readonly kind: BerkeleyCardKind;
  readonly head: string;
  readonly text: string;
  readonly span: BerkeleySourceSpan;
  readonly physicalLines: readonly number[];
  readonly tokens: readonly BerkeleySyntaxToken[];
}

export interface BerkeleyGrammarMetadata {
  readonly name: string;
  readonly version: number;
  readonly tokenGrammar: string;
  readonly parserGrammar: string;
}

export interface BerkeleyAnalysisInventoryEntry {
  readonly index: number;
  readonly directive: string;
  readonly analysis: string;
  readonly span: BerkeleySourceSpan;
}

export interface BerkeleySyntaxDeck {
  readonly grammar: BerkeleyGrammarMetadata;
  readonly title?: string;
  readonly cards: readonly BerkeleyLogicalCard[];
  readonly diagnostics: readonly BerkeleySyntaxDiagnostic[];
  hasErrors(): boolean;
  analysisInventory(): BerkeleyAnalysisInventoryEntry[];
}

interface BerkeleySourcePosition {
  readonly line: number;
  readonly column: number;
}

class BerkeleyLogicalCardBuilder {
  readonly positions: BerkeleySourcePosition[];
  readonly physicalLines: number[];
  text: string;

  constructor(lineNumber: number, text: string, startColumn: number) {
    this.text = text;
    this.positions = Array.from(text, (_, offset) => ({
      line: lineNumber,
      column: startColumn + offset,
    }));
    this.physicalLines = [lineNumber];
  }

  appendContinuation(lineNumber: number, text: string, startColumn: number): void {
    if (text.length === 0) {
      return;
    }
    const joinPosition = this.positions.at(-1) ?? {
      line: lineNumber,
      column: startColumn,
    };
    this.text += " ";
    this.positions.push({ line: joinPosition.line, column: joinPosition.column });
    for (const [offset, char] of Array.from(text).entries()) {
      this.text += char;
      this.positions.push({ line: lineNumber, column: startColumn + offset });
    }
    this.physicalLines.push(lineNumber);
  }

  span(): BerkeleySourceSpan {
    const start = this.positions[0];
    if (start === undefined) {
      return berkeleySourcePoint(1, 1);
    }
    const end = this.positions[this.positions.length - 1] ?? start;
    return {
      startLine: start.line,
      startColumn: start.column,
      endLine: end.line,
      endColumn: end.column + 1,
    };
  }
}

export function parseBerkeleySyntax(text: string): BerkeleySyntaxDeck {
  const builders: BerkeleyLogicalCardBuilder[] = [];
  const diagnostics: BerkeleySyntaxDiagnostic[] = [];
  let pending: BerkeleyLogicalCardBuilder | undefined;
  let title: string | undefined;
  let sawContent = false;

  const lines = text.split(/\r?\n/);
  if (lines.length > 0 && lines[lines.length - 1] === "") {
    lines.pop();
  }

  for (const [index, rawLine] of lines.entries()) {
    const lineNumber = index + 1;
    const withoutComment = stripBerkeleyInlineComment(rawLine);
    const trimmedInfo = berkeleyTrimmedWithColumn(withoutComment);
    if (trimmedInfo === undefined) {
      continue;
    }
    const { trimmed, startColumn } = trimmedInfo;
    if (trimmed.length === 0) {
      continue;
    }
    if (trimmed.startsWith("*")) {
      if (!sawContent && title === undefined) {
        const candidate = trimmed.slice(1).trim();
        if (candidate.length > 0) {
          title = candidate;
        }
      }
      continue;
    }
    if (trimmed.startsWith("+")) {
      const afterPlus = trimmed.slice(1);
      const continuation = afterPlus.trim();
      const continuationStartColumn =
        startColumn + 1 + afterPlus.length - afterPlus.trimStart().length;
      if (pending === undefined) {
        diagnostics.push(
          berkeleySyntaxError(
            "SPICE_SYNTAX_CONTINUATION_WITHOUT_CARD",
            "continuation line appears before any logical SPICE card",
            berkeleySourcePoint(lineNumber, startColumn),
          ),
        );
      } else {
        pending.appendContinuation(lineNumber, continuation, continuationStartColumn);
      }
      continue;
    }

    sawContent = true;
    if (pending !== undefined) {
      builders.push(pending);
    }
    pending = new BerkeleyLogicalCardBuilder(lineNumber, trimmed, startColumn);
  }

  if (pending !== undefined) {
    builders.push(pending);
  }

  const cards = builders.map((builder) => berkeleyLogicalCard(builder, diagnostics));
  return {
    grammar: berkeleyGrammarMetadata(),
    title,
    cards,
    diagnostics,
    hasErrors() {
      return this.diagnostics.some((diagnostic) => diagnostic.severity === "error");
    },
    analysisInventory() {
      return this.cards
        .map((card, index) => ({ card, index }))
        .filter(({ card }) => card.kind === "analysis")
        .map(({ card, index }) => ({
          index,
          directive: card.head,
          analysis: card.head.replace(/^\./, "").toLowerCase(),
          span: card.span,
        }));
    },
  };
}

function berkeleyGrammarMetadata(): BerkeleyGrammarMetadata {
  return {
    name: BERKELEY_SPICE_GRAMMAR_NAME,
    version: BERKELEY_SPICE_GRAMMAR_VERSION,
    tokenGrammar: BERKELEY_SPICE_TOKEN_GRAMMAR,
    parserGrammar: BERKELEY_SPICE_PARSER_GRAMMAR,
  };
}

function berkeleyLogicalCard(
  builder: BerkeleyLogicalCardBuilder,
  diagnostics: BerkeleySyntaxDiagnostic[],
): BerkeleyLogicalCard {
  const tokens = tokenizeBerkeleyCard(builder, diagnostics);
  const head = berkeleyCardHead(tokens);
  return {
    kind: classifyBerkeleyCard(head),
    head,
    text: builder.text,
    span: builder.span(),
    physicalLines: [...builder.physicalLines],
    tokens,
  };
}

function tokenizeBerkeleyCard(
  builder: BerkeleyLogicalCardBuilder,
  diagnostics: BerkeleySyntaxDiagnostic[],
): BerkeleySyntaxToken[] {
  const chars = Array.from(builder.text);
  const tokens: BerkeleySyntaxToken[] = [];
  let index = 0;
  let parenDepth = 0;

  while (index < chars.length) {
    const char = chars[index];
    if (/\s/.test(char)) {
      index += 1;
      continue;
    }

    if (char === "\"") {
      const start = index;
      index += 1;
      let escaped = false;
      let closed = false;
      while (index < chars.length) {
        const current = chars[index];
        if (escaped) {
          escaped = false;
        } else if (current === "\\") {
          escaped = true;
        } else if (current === "\"") {
          index += 1;
          closed = true;
          break;
        }
        index += 1;
      }
      if (!closed) {
        diagnostics.push(
          berkeleySyntaxError(
            "SPICE_SYNTAX_UNCLOSED_QUOTE",
            "quoted string is missing its closing quote",
            berkeleySpanForRange(builder.positions, start, index),
          ),
        );
      }
      tokens.push(berkeleyToken("QUOTED_STRING", chars, builder.positions, start, index));
      continue;
    }

    if (char === "{") {
      const start = index;
      index += 1;
      let closed = false;
      while (index < chars.length) {
        if (chars[index] === "}") {
          index += 1;
          closed = true;
          break;
        }
        index += 1;
      }
      if (!closed) {
        diagnostics.push(
          berkeleySyntaxError(
            "SPICE_SYNTAX_UNCLOSED_BRACED_EXPR",
            "braced expression is missing its closing brace",
            berkeleySpanForRange(builder.positions, start, index),
          ),
        );
      }
      tokens.push(berkeleyToken("BRACED_EXPR", chars, builder.positions, start, index));
      continue;
    }

    if (char === "(") {
      parenDepth += 1;
      tokens.push(berkeleyToken("LPAREN", chars, builder.positions, index, index + 1));
      index += 1;
      continue;
    }
    if (char === ")") {
      if (parenDepth === 0) {
        diagnostics.push(
          berkeleySyntaxError(
            "SPICE_SYNTAX_UNMATCHED_RPAREN",
            "closing parenthesis has no matching opening parenthesis",
            berkeleySpanForRange(builder.positions, index, index + 1),
          ),
        );
      } else {
        parenDepth -= 1;
      }
      tokens.push(berkeleyToken("RPAREN", chars, builder.positions, index, index + 1));
      index += 1;
      continue;
    }
    if (char === ",") {
      tokens.push(berkeleyToken("COMMA", chars, builder.positions, index, index + 1));
      index += 1;
      continue;
    }
    if (char === "=") {
      tokens.push(berkeleyToken("EQUALS", chars, builder.positions, index, index + 1));
      index += 1;
      continue;
    }

    if (char === ".") {
      const atomEnd = readBerkeleyAtomEnd(chars, index);
      const raw = chars.slice(index, atomEnd).join("");
      const kind = knownBerkeleyDotToken(raw);
      if (kind !== undefined) {
        tokens.push(berkeleyToken(kind, chars, builder.positions, index, atomEnd));
        index = atomEnd;
      } else {
        tokens.push(berkeleyToken("DOT", chars, builder.positions, index, index + 1));
        index += 1;
      }
      continue;
    }

    const start = index;
    index = readBerkeleyAtomEnd(chars, index);
    const raw = chars.slice(start, index).join("");
    tokens.push(
      berkeleyToken(
        isBerkeleyNumberToken(raw) ? "NUMBER" : "ATOM",
        chars,
        builder.positions,
        start,
        index,
      ),
    );
  }

  if (parenDepth > 0) {
    diagnostics.push(
      berkeleySyntaxError(
        "SPICE_SYNTAX_UNCLOSED_PAREN",
        "unclosed parenthesis: opening parenthesis is missing its closing parenthesis",
        builder.span(),
      ),
    );
  }

  return tokens;
}

function berkeleyToken(
  kind: string,
  chars: readonly string[],
  positions: readonly BerkeleySourcePosition[],
  start: number,
  end: number,
): BerkeleySyntaxToken {
  return {
    kind,
    text: chars.slice(start, end).join(""),
    span: berkeleySpanForRange(positions, start, end),
  };
}

function berkeleySpanForRange(
  positions: readonly BerkeleySourcePosition[],
  start: number,
  end: number,
): BerkeleySourceSpan {
  const first = positions[start] ?? positions[0] ?? { line: 1, column: 1 };
  const last = positions[Math.max(end - 1, 0)] ?? first;
  return {
    startLine: first.line,
    startColumn: first.column,
    endLine: last.line,
    endColumn: last.column + 1,
  };
}

function berkeleyCardHead(tokens: readonly BerkeleySyntaxToken[]): string {
  const first = tokens[0];
  if (first === undefined) {
    return "";
  }
  const second = tokens[1];
  if (first.kind === "DOT" && second?.kind === "ATOM") {
    return `.${second.text}`;
  }
  return first.text;
}

function classifyBerkeleyCard(head: string): BerkeleyCardKind {
  switch (head.toLowerCase()) {
    case ".model":
      return "model";
    case ".subckt":
      return "subcktStart";
    case ".ends":
      return "subcktEnd";
    case ".end":
      return "end";
    case ".param":
      return "param";
    case ".func":
      return "func";
    case ".options":
      return "options";
    case ".temp":
    case ".ic":
    case ".nodeset":
      return "condition";
    case ".op":
    case ".dc":
    case ".ac":
    case ".tran":
    case ".tf":
    case ".sens":
    case ".noise":
    case ".disto":
    case ".pz":
      return "analysis";
    case ".print":
    case ".plot":
    case ".save":
    case ".probe":
    case ".measure":
    case ".meas":
    case ".four":
      return "output";
    case ".include":
    case ".lib":
      return "source";
    case ".control":
      return "controlStart";
    case ".endc":
      return "controlEnd";
    default:
      return head.startsWith(".") ? "unknownDirective" : "element";
  }
}

function readBerkeleyAtomEnd(chars: readonly string[], index: number): number {
  while (index < chars.length) {
    const char = chars[index];
    if (/\s/.test(char) || ["(", ")", ",", "=", "\"", "{", "}"].includes(char)) {
      break;
    }
    index += 1;
  }
  return index;
}

function knownBerkeleyDotToken(raw: string): string | undefined {
  return new Map<string, string>([
    [".end", "DOT_END"],
    [".ends", "DOT_ENDS"],
    [".subckt", "DOT_SUBCKT"],
    [".model", "DOT_MODEL"],
    [".param", "DOT_PARAM"],
    [".func", "DOT_FUNC"],
    [".options", "DOT_OPTIONS"],
    [".temp", "DOT_TEMP"],
    [".ic", "DOT_IC"],
    [".nodeset", "DOT_NODESET"],
    [".op", "DOT_OP"],
    [".dc", "DOT_DC"],
    [".ac", "DOT_AC"],
    [".tran", "DOT_TRAN"],
    [".tf", "DOT_TF"],
    [".sens", "DOT_SENS"],
    [".noise", "DOT_NOISE"],
    [".disto", "DOT_DISTO"],
    [".pz", "DOT_PZ"],
    [".print", "DOT_PRINT"],
    [".plot", "DOT_PLOT"],
    [".save", "DOT_SAVE"],
    [".probe", "DOT_PROBE"],
    [".measure", "DOT_MEASURE"],
    [".meas", "DOT_MEAS"],
    [".four", "DOT_FOUR"],
    [".include", "DOT_INCLUDE"],
    [".lib", "DOT_LIB"],
    [".control", "DOT_CONTROL"],
    [".endc", "DOT_ENDC"],
  ]).get(raw.toLowerCase());
}

function isBerkeleyNumberToken(raw: string): boolean {
  let index = 0;
  if (raw[index] === "+" || raw[index] === "-") {
    index += 1;
  }

  let digitsBeforeDot = 0;
  while (/[0-9]/.test(raw[index] ?? "")) {
    digitsBeforeDot += 1;
    index += 1;
  }

  let digitsAfterDot = 0;
  if (raw[index] === ".") {
    index += 1;
    while (/[0-9]/.test(raw[index] ?? "")) {
      digitsAfterDot += 1;
      index += 1;
    }
  }

  if (digitsBeforeDot === 0 && digitsAfterDot === 0) {
    return false;
  }

  if (raw[index] === "e" || raw[index] === "E") {
    let probe = index + 1;
    if (raw[probe] === "+" || raw[probe] === "-") {
      probe += 1;
    }
    let exponentDigits = 0;
    while (/[0-9]/.test(raw[probe] ?? "")) {
      exponentDigits += 1;
      probe += 1;
    }
    if (exponentDigits > 0) {
      index = probe;
    }
  }

  return Array.from(raw.slice(index)).every((char) => /[A-Za-z]/.test(char));
}

function stripBerkeleyInlineComment(line: string): string {
  return line.split(";", 1)[0];
}

function berkeleyTrimmedWithColumn(
  line: string,
): { readonly trimmed: string; readonly startColumn: number } | undefined {
  const trimmedEnd = line.trimEnd();
  if (trimmedEnd.length === 0) {
    return undefined;
  }
  const trimmed = trimmedEnd.trimStart();
  return {
    trimmed,
    startColumn: trimmedEnd.length - trimmed.length + 1,
  };
}

function berkeleySourcePoint(line: number, column: number): BerkeleySourceSpan {
  return {
    startLine: line,
    startColumn: column,
    endLine: line,
    endColumn: column,
  };
}

function berkeleySyntaxError(
  code: string,
  message: string,
  span: BerkeleySourceSpan,
): BerkeleySyntaxDiagnostic {
  return { code, severity: "error", message, span };
}

export type Element =
  | Resistor
  | Capacitor
  | Inductor
  | MutualInductor
  | TransmissionLine
  | VoltageSource
  | CurrentSource
  | BSource
  | CustomModel
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

export interface CustomModelContext {
  readonly voltage: number;
  readonly temperatureKelvin: number;
  readonly parameters: Readonly<Record<string, number>>;
}

export interface CustomModelEvaluation {
  readonly currentAmps: number;
  readonly conductanceSiemens: number;
}

export type CustomModelEvaluator = (
  context: CustomModelContext,
) => CustomModelEvaluation;

export interface CustomModel {
  readonly kind: "custom-model";
  readonly name: string;
  readonly positive: string;
  readonly negative: string;
  readonly modelName: string;
  readonly parameters: Readonly<Record<string, number>>;
  readonly evaluator?: CustomModelEvaluator;
  readonly conductanceSiemens?: number;
  readonly currentOffsetAmps: number;
}

export interface CustomModelDiagnostic {
  readonly code: string;
  readonly message: string;
  readonly severity: "error" | "warning";
}

export interface CustomModelSourceAnalysis {
  readonly accepted: boolean;
  readonly subset: string;
  readonly moduleName?: string;
  readonly terminals: readonly string[];
  readonly contribution?: readonly [string, string];
  readonly diagnostics: readonly CustomModelDiagnostic[];
}

export interface CompatibilityOracle {
  readonly reference: string;
  readonly version: string;
  readonly source: string;
}

export interface CompatibilityGoldenValue {
  readonly name: string;
  readonly value: number;
  readonly unit: string;
  readonly absoluteTolerance: number;
  readonly relativeTolerance: number;
}

export interface CompatibilityDeck {
  readonly id: string;
  readonly title: string;
  readonly analysis: string;
  readonly netlist: string;
  readonly oracle: CompatibilityOracle;
  readonly goldenValues: readonly CompatibilityGoldenValue[];
  readonly knownIncompatibilities: readonly string[];
}

export interface DeckControlDiagnostic {
  readonly code: string;
  readonly directive: string;
  readonly lineNumber: number;
  readonly message: string;
  readonly severity: "error" | "warning";
}

export interface DeckControlSummary {
  readonly activeLines: readonly string[];
  readonly controlLines: readonly string[];
  readonly writeMarkers: readonly string[];
  readonly rawfileOptions: readonly string[];
  readonly terminated: boolean;
  readonly endLineNumber?: number;
  readonly diagnostics: readonly DeckControlDiagnostic[];
}

export interface DeckResolutionDiagnostic {
  readonly code: string;
  readonly directive: string;
  readonly source: string;
  readonly lineNumber: number;
  readonly message: string;
  readonly severity: "error" | "warning";
  readonly target?: string;
}

export interface DeckResolutionSummary {
  readonly activeLines: readonly string[];
  readonly terminated: boolean;
  readonly endLineNumber?: number;
  readonly diagnostics: readonly DeckResolutionDiagnostic[];
  readonly includedPaths: readonly string[];
  readonly librarySections: readonly string[];
}

export interface DeckParameterValue {
  readonly name: string;
  readonly value: number;
}

export interface DeckParameterDiagnostic {
  readonly code: string;
  readonly directive: string;
  readonly lineNumber: number;
  readonly message: string;
  readonly severity: "error" | "warning";
  readonly parameter?: string;
  readonly expression?: string;
}

export interface DeckParameterSummary {
  readonly activeLines: readonly string[];
  readonly terminated: boolean;
  readonly endLineNumber?: number;
  readonly parameters: readonly DeckParameterValue[];
  readonly diagnostics: readonly DeckParameterDiagnostic[];
}

export interface DeckNodeCondition {
  readonly directive: ".ic" | ".nodeset";
  readonly node: string;
  readonly value: number;
  readonly lineNumber: number;
}

export interface DeckInitialConditionDiagnostic {
  readonly code: string;
  readonly directive: ".ic" | ".nodeset";
  readonly lineNumber: number;
  readonly message: string;
  readonly severity: "error" | "warning";
  readonly token?: string;
}

export interface DeckInitialConditionSummary {
  readonly activeLines: readonly string[];
  readonly terminated: boolean;
  readonly endLineNumber?: number;
  readonly initialConditions: readonly DeckNodeCondition[];
  readonly nodesets: readonly DeckNodeCondition[];
  readonly diagnostics: readonly DeckInitialConditionDiagnostic[];
}

export interface DeckFunctionDefinition {
  readonly name: string;
  readonly arguments: readonly string[];
  readonly expression: string;
  readonly lineNumber: number;
}

export interface DeckFunctionDiagnostic {
  readonly code: string;
  readonly directive: ".func";
  readonly lineNumber: number;
  readonly message: string;
  readonly severity: "error" | "warning";
  readonly functionName?: string;
  readonly expression?: string;
}

export interface DeckFunctionSummary {
  readonly activeLines: readonly string[];
  readonly terminated: boolean;
  readonly endLineNumber?: number;
  readonly functions: readonly DeckFunctionDefinition[];
  readonly diagnostics: readonly DeckFunctionDiagnostic[];
}

export interface DeckMeasurementCard {
  readonly directive: ".measure" | ".meas";
  readonly analysis: "tran" | "transient" | "dc" | "ac";
  readonly name: string;
  readonly mode: string;
  readonly probe: string;
  readonly lineNumber: number;
  readonly fromValue?: number;
  readonly toValue?: number;
  readonly atValue?: number;
  readonly targetValue?: number;
  readonly crossingKind?: MeasurementCrossingKind;
  readonly crossingCount?: number;
  readonly triggerProbe?: string;
  readonly triggerValue?: number;
  readonly triggerCrossingKind?: MeasurementCrossingKind;
  readonly triggerCrossingCount?: number;
}

export type MeasurementCrossingKind = "rise" | "fall" | "cross";

export interface DeckMeasurementDiagnostic {
  readonly code: string;
  readonly directive: ".measure" | ".meas";
  readonly lineNumber: number;
  readonly message: string;
  readonly severity: "error" | "warning";
  readonly token?: string;
}

export interface DeckMeasurementSummary {
  readonly activeLines: readonly string[];
  readonly terminated: boolean;
  readonly endLineNumber?: number;
  readonly measurements: readonly DeckMeasurementCard[];
  readonly diagnostics: readonly DeckMeasurementDiagnostic[];
}

export interface DeckFourierCard {
  readonly directive: ".four";
  readonly fundamentalFrequencyHz: number;
  readonly probes: readonly string[];
  readonly lineNumber: number;
  readonly harmonics?: number;
  readonly fromValue?: number;
}

export interface DeckFourierDiagnostic {
  readonly code: string;
  readonly directive: ".four";
  readonly lineNumber: number;
  readonly message: string;
  readonly severity: "error" | "warning";
  readonly token?: string;
}

export interface DeckFourierSummary {
  readonly activeLines: readonly string[];
  readonly terminated: boolean;
  readonly endLineNumber?: number;
  readonly fourier: readonly DeckFourierCard[];
  readonly diagnostics: readonly DeckFourierDiagnostic[];
}

export interface DeckOutputSelection {
  readonly directive: ".save" | ".probe" | ".print" | ".plot";
  readonly analysis?: "op" | "dc" | "ac" | "tran";
  readonly probes: readonly string[];
  readonly lineNumber: number;
}

export interface DeckOutputDiagnostic {
  readonly code: string;
  readonly directive: ".save" | ".probe" | ".print" | ".plot";
  readonly lineNumber: number;
  readonly message: string;
  readonly severity: "error" | "warning";
  readonly token?: string;
}

export interface DeckOutputSummary {
  readonly activeLines: readonly string[];
  readonly terminated: boolean;
  readonly endLineNumber?: number;
  readonly selections: readonly DeckOutputSelection[];
  readonly diagnostics: readonly DeckOutputDiagnostic[];
}

export interface DeckAnalysisPlan {
  readonly directive: ".op" | ".dc" | ".ac" | ".tran" | ".tf" | ".sens" | ".noise";
  readonly analysis: "op" | "dc" | "ac" | "tran" | "tf" | "sens" | "noise";
  readonly lineNumber: number;
  readonly sourceName?: string;
  readonly outputNode?: string;
  readonly startValue?: number;
  readonly stopValue?: number;
  readonly stepValue?: number;
  readonly sweepKind?: "lin" | "dec" | "oct";
  readonly pointCount?: number;
  readonly startFrequencyHz?: number;
  readonly stopFrequencyHz?: number;
  readonly stepTime?: number;
  readonly stopTime?: number;
  readonly startTime?: number;
  readonly maxStep?: number;
  readonly useInitialConditions: boolean;
}

export interface DeckAnalysisDiagnostic {
  readonly code: string;
  readonly directive: ".op" | ".dc" | ".ac" | ".tran" | ".tf" | ".sens" | ".noise";
  readonly lineNumber: number;
  readonly message: string;
  readonly severity: "error" | "warning";
  readonly token?: string;
}

export interface DeckAnalysisSummary {
  readonly activeLines: readonly string[];
  readonly terminated: boolean;
  readonly endLineNumber?: number;
  readonly analyses: readonly DeckAnalysisPlan[];
  readonly diagnostics: readonly DeckAnalysisDiagnostic[];
}

export type DeckAnalysisExecutionResult =
  | DcResult
  | readonly DcSweepPoint[]
  | readonly AcPoint[]
  | readonly TransientPoint[]
  | TfResult
  | SensResult
  | NoiseResult;

export interface DeckRunArtifact {
  readonly analysis: DeckAnalysisPlan["analysis"];
  readonly directive: DeckAnalysisPlan["directive"];
  readonly analysisDirectiveCount: number;
  readonly analysisDirectives: readonly string[];
  readonly deckAnalysisKindCount: number;
  readonly deckAnalysisKinds: readonly string[];
  readonly deckAnalysisDirectiveCount: number;
  readonly deckAnalysisDirectives: readonly string[];
  readonly lineNumber: number;
  readonly sourceName?: string;
  readonly outputNode?: string;
  readonly sweepKind?: DeckAnalysisPlan["sweepKind"];
  readonly startValue?: number;
  readonly stopValue?: number;
  readonly stepValue?: number;
  readonly pointCount?: number;
  readonly startFrequencyHz?: number;
  readonly stopFrequencyHz?: number;
  readonly stepTime?: number;
  readonly stopTime?: number;
  readonly startTime?: number;
  readonly maxStep?: number;
  readonly useInitialConditions?: boolean;
  readonly resultRows: number;
  readonly resultColumnCount: number;
  readonly resultColumns: readonly string[];
  readonly tableCount: number;
  readonly tables: readonly string[];
  readonly outputProbeCount: number;
  readonly outputProbes: readonly string[];
  readonly outputDirectiveCount: number;
  readonly outputDirectives: readonly string[];
  readonly measurementCount: number;
  readonly measurementNames: readonly string[];
  readonly fourierCount: number;
  readonly fourierProbes: readonly string[];
  readonly controlLineCount: number;
  readonly controlLines: readonly string[];
  readonly writeMarkerCount: number;
  readonly writeMarkers: readonly string[];
  readonly rawfileOptionCount: number;
  readonly rawfileOptions: readonly string[];
  readonly controlPolicyArtifactCount: number;
  readonly controlPolicyCategories: readonly string[];
  readonly controlPolicyCodes: readonly string[];
  readonly controlPolicySeverities: readonly string[];
  readonly diagnosticCount: number;
  readonly diagnosticCodes: readonly string[];
}

export interface DeckTableArtifact {
  readonly name: string;
  readonly table: string;
  readonly csv: string;
  readonly json: string;
  readonly records: ReadonlyArray<Record<string, string>>;
}

export interface DeckOutputPlanArtifact {
  readonly analysis: DeckAnalysisPlan["analysis"];
  readonly directive: DeckAnalysisPlan["directive"];
  readonly lineNumber: number;
  readonly sourceName?: string;
  readonly outputNode?: string;
  readonly sweepKind?: DeckAnalysisPlan["sweepKind"];
  readonly startValue?: number;
  readonly stopValue?: number;
  readonly stepValue?: number;
  readonly pointCount?: number;
  readonly startFrequencyHz?: number;
  readonly stopFrequencyHz?: number;
  readonly stepTime?: number;
  readonly stopTime?: number;
  readonly startTime?: number;
  readonly maxStep?: number;
  readonly useInitialConditions?: boolean;
  readonly resultRowCount: number;
  readonly resultColumnCount: number;
  readonly resultColumns: readonly string[];
  readonly outputProbeCount: number;
  readonly outputProbes: readonly string[];
  readonly outputProbeLineCount: number;
  readonly outputProbeLines: readonly number[];
  readonly outputDirectiveCount: number;
  readonly outputDirectives: readonly string[];
  readonly outputDirectiveKindCount: number;
  readonly outputDirectiveKinds: readonly string[];
  readonly outputDirectiveAnalysisKindCount: number;
  readonly outputDirectiveAnalysisKinds: readonly string[];
  readonly outputDirectiveLineCount: number;
  readonly outputDirectiveLines: readonly number[];
  readonly tableCount: number;
  readonly tables: readonly string[];
}

export interface DeckControlPolicyArtifact {
  readonly lineNumber: number;
  readonly category: "script" | "workdir" | "control-flow" | "variable";
  readonly command: string;
  readonly code: string;
  readonly severity: DeckControlDiagnostic["severity"];
  readonly message: string;
}

export interface DeckControlPolicySummaryArtifact {
  readonly category: DeckControlPolicyArtifact["category"];
  readonly artifactCount: number;
  readonly lineNumbers: readonly number[];
  readonly commands: readonly string[];
  readonly codes: readonly string[];
  readonly severities: readonly string[];
}

export interface DeckRawfileArtifact {
  readonly target: string;
  readonly marker: string;
  readonly probeCount: number;
  readonly probes: readonly string[];
  readonly matchedProbeCount: number;
  readonly matchedProbes: readonly string[];
  readonly unmatchedProbeCount: number;
  readonly unmatchedProbes: readonly string[];
  readonly optionCount: number;
  readonly options: readonly string[];
  readonly rawfile: string;
}

export interface DeckWrdataArtifact {
  readonly target: string;
  readonly marker: string;
  readonly probeCount: number;
  readonly probes: readonly string[];
  readonly matchedProbeCount: number;
  readonly matchedProbes: readonly string[];
  readonly unmatchedProbeCount: number;
  readonly unmatchedProbes: readonly string[];
  readonly optionCount: number;
  readonly options: readonly string[];
  readonly datafile: string;
}

export interface DeckAnalysisExecution {
  readonly plan: DeckAnalysisPlan;
  readonly result: DeckAnalysisExecutionResult;
  readonly table: string;
  readonly outputProbes: readonly string[];
  readonly outputDirectives: readonly string[];
  readonly analysisDirectives: readonly string[];
  readonly deckAnalysisKindCount: number;
  readonly deckAnalysisKinds: readonly string[];
  readonly deckAnalysisDirectiveCount: number;
  readonly deckAnalysisDirectives: readonly string[];
  readonly outputPlanArtifactCount: number;
  readonly outputPlanArtifacts: readonly DeckOutputPlanArtifact[];
  readonly outputPlanArtifactTable: string;
  readonly outputPlanArtifactCsv: string;
  readonly outputPlanArtifactJson: string;
  readonly outputPlanArtifactRecords: ReadonlyArray<Record<string, string>>;
  readonly controlLineCount: number;
  readonly controlLines: readonly string[];
  readonly writeMarkerCount: number;
  readonly writeMarkers: readonly string[];
  readonly rawfileOptionCount: number;
  readonly rawfileOptions: readonly string[];
  readonly controlPolicyArtifactCount: number;
  readonly controlPolicyArtifacts: readonly DeckControlPolicyArtifact[];
  readonly controlPolicyArtifactTable: string;
  readonly controlPolicyArtifactCsv: string;
  readonly controlPolicyArtifactJson: string;
  readonly controlPolicyArtifactRecords: ReadonlyArray<Record<string, string>>;
  readonly controlPolicySummaryArtifactCount: number;
  readonly controlPolicySummaryArtifacts: readonly DeckControlPolicySummaryArtifact[];
  readonly controlPolicySummaryArtifactTable: string;
  readonly controlPolicySummaryArtifactCsv: string;
  readonly controlPolicySummaryArtifactJson: string;
  readonly controlPolicySummaryArtifactRecords: ReadonlyArray<Record<string, string>>;
  readonly rawfileArtifactCount: number;
  readonly rawfileArtifacts: readonly DeckRawfileArtifact[];
  readonly rawfileArtifactTable: string;
  readonly rawfileArtifactCsv: string;
  readonly rawfileArtifactJson: string;
  readonly rawfileArtifactRecords: ReadonlyArray<Record<string, string>>;
  readonly wrdataArtifactCount: number;
  readonly wrdataArtifacts: readonly DeckWrdataArtifact[];
  readonly wrdataArtifactTable: string;
  readonly wrdataArtifactCsv: string;
  readonly wrdataArtifactJson: string;
  readonly wrdataArtifactRecords: ReadonlyArray<Record<string, string>>;
  readonly diagnosticCount: number;
  readonly diagnosticCodes: readonly string[];
  readonly tableCount: number;
  readonly tables: readonly string[];
  readonly tableArtifacts: readonly DeckTableArtifact[];
  readonly measurements: readonly ProbeMeasurement[];
  readonly measurementTable: string;
  readonly fourier: readonly FourierResult[];
  readonly fourierTable: string;
  readonly runArtifacts: readonly DeckRunArtifact[];
  readonly runArtifactTable: string;
}

export interface DeckExecution {
  readonly executionCount: number;
  readonly analysisOrder: readonly string[];
  readonly analysisDirectives: readonly string[];
  readonly executions: readonly DeckAnalysisExecution[];
  readonly runArtifactCount: number;
  readonly runArtifacts: readonly DeckRunArtifact[];
  readonly runArtifactTable: string;
  readonly runArtifactCsv: string;
  readonly runArtifactJson: string;
  readonly runArtifactRecords: ReadonlyArray<Record<string, string>>;
}

export interface ReleaseReadinessIssue {
  readonly deckId: string;
  readonly field: string;
  readonly message: string;
}

export interface ReleaseReadinessReport {
  readonly passed: boolean;
  readonly deckCount: number;
  readonly analyses: readonly string[];
  readonly issues: readonly ReleaseReadinessIssue[];
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
  readonly junctionPotential: number;
  readonly gradingCoefficient: number;
  readonly forwardBiasDepletionCoefficient: number;
  readonly saturationCurrentTemperatureExponent: number;
  readonly energyGapElectronVolts: number;
  readonly seriesResistance: number;
  readonly flickerNoiseCoefficient: number;
  readonly flickerNoiseExponent: number;
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
  readonly gateSourceCapacitance: number;
  readonly gateDrainCapacitance: number;
  readonly flickerNoiseCoefficient: number;
  readonly flickerNoiseExponent: number;
  readonly junctionPotential: number;
  readonly forwardBiasDepletionCoefficient: number;
  readonly gateSaturationCurrent: number;
  readonly gateSaturationCurrentTemperatureExponent: number;
  readonly bandgapVoltage: number;
  readonly dopingTailParameter: number;
  readonly noiseEquationLevel: number;
  readonly channelNoiseCoefficient: number;
  readonly drainResistance: number;
  readonly sourceResistance: number;
  readonly thresholdVoltageTemperatureCoefficient: number;
  readonly alternativeThresholdVoltageTemperatureCoefficient?: number;
  readonly nominalTemperatureKelvin?: number;
  readonly mobilityTemperatureExponent: number;
  readonly mobilityTemperatureCoefficient?: number;
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
  readonly saturationCurrentTemperatureExponent: number;
  readonly energyGapElectronVolts: number;
  readonly forwardEarlyVoltage: number;
  readonly reverseEarlyVoltage: number;
  readonly forwardEmissionCoefficient: number;
  readonly reverseEmissionCoefficient: number;
  readonly baseEmitterJunctionPotential: number;
  readonly baseEmitterGradingCoefficient: number;
  readonly baseCollectorJunctionPotential: number;
  readonly baseCollectorGradingCoefficient: number;
  readonly forwardBiasDepletionCoefficient: number;
  readonly forwardBetaRolloffCurrent: number;
  readonly baseEmitterLeakageSaturationCurrent: number;
  readonly baseEmitterLeakageEmissionCoefficient: number;
  readonly baseCollectorLeakageSaturationCurrent: number;
  readonly baseCollectorLeakageEmissionCoefficient: number;
  readonly forwardBetaTemperatureExponent: number;
  readonly reverseBeta: number;
  readonly reverseBetaRolloffCurrent: number;
  readonly nominalTemperatureKelvin: number | undefined;
  readonly flickerNoiseCoefficient: number;
  readonly flickerNoiseExponent: number;
  readonly forwardExcessPhaseDegrees: number;
  readonly forwardTransitTimeBiasCoefficient: number;
  readonly forwardTransitTimeCurrent: number;
  readonly forwardTransitTimeVoltage: number;
  readonly emitterResistance: number;
  readonly collectorResistance: number;
  readonly baseResistance: number;
  readonly minimumBaseResistance: number | undefined;
  readonly baseResistanceHalfCurrent: number;
  readonly baseCollectorCapacitanceFraction: number;
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
  readonly LD: number;
  readonly TOX: number;
  readonly U0: number;
  readonly RD: number;
  readonly RS: number;
  readonly RSH: number;
  readonly NRD: number;
  readonly NRS: number;
  readonly AD: number;
  readonly AS: number;
  readonly PD: number;
  readonly PS: number;
  readonly CJ: number;
  readonly CJSW: number;
  readonly IS: number;
  readonly JS: number;
  readonly N_SUB: number;
  readonly T_NOM: number;
  readonly CGSO: number;
  readonly CGDO: number;
  readonly CGBO: number;
  readonly CBS: number;
  readonly CBD: number;
  readonly PB: number;
  readonly MJ: number;
  readonly MJSW: number;
  readonly FC: number;
  readonly KF: number;
  readonly AF: number;
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

export type ModelCardKind = "D" | "NPN" | "PNP" | "NJF" | "PJF" | "NMOS" | "PMOS";

export interface NormalizedModelCard {
  readonly name: string;
  readonly kind: ModelCardKind;
  readonly parameters: Readonly<Record<string, number>>;
  readonly unsupportedParameters: readonly string[];
}

export interface ModelCardUnsupportedParameterIssue {
  readonly modelName: string;
  readonly kind: ModelCardKind;
  readonly parameter: string;
  readonly message: string;
}

export interface ModelCardSupportedParameterCoverage {
  readonly kind: ModelCardKind;
  readonly canonicalParameter: string;
  readonly acceptedNames: readonly string[];
  readonly aliasCount: number;
}

export interface ModelCardSupportedParameterCoverageSummary {
  readonly kind: ModelCardKind;
  readonly canonicalParameterCount: number;
  readonly acceptedNameCount: number;
  readonly aliasedParameterCount: number;
  readonly maxAliasCount: number;
  readonly aliasedParameters: readonly string[];
}

export interface ModelCardSupportedParameterCoverageGateIssue {
  readonly kind: string;
  readonly field: string;
  readonly message: string;
}

export interface ModelCardSupportedParameterCoverageGateReport {
  readonly passed: boolean;
  readonly kindCount: number;
  readonly expectedKindCount: number;
  readonly canonicalParameterCount: number;
  readonly expectedCanonicalParameterCount: number;
  readonly acceptedNameCount: number;
  readonly aliasedParameterCount: number;
  readonly maxAliasCount: number;
  readonly issues: readonly ModelCardSupportedParameterCoverageGateIssue[];
}

export interface ModelCardSupportedParameterCoverageDashboardRow {
  readonly kind: ModelCardKind;
  readonly passed: boolean;
  readonly canonicalParameterCount: number;
  readonly expectedCanonicalParameterCount: number;
  readonly acceptedNameCount: number;
  readonly expectedAcceptedNameCount: number;
  readonly aliasedParameterCount: number;
  readonly expectedAliasedParameterCount: number;
  readonly maxAliasCount: number;
  readonly expectedMaxAliasCount: number;
  readonly issueCount: number;
  readonly issueFields: readonly string[];
}

export interface DeviceModelBehaviorFixture {
  readonly name: string;
  readonly kind: ModelCardKind;
  readonly model: NormalizedModelCard;
  readonly circuit: Circuit;
  readonly probeNode: string;
  readonly expectedMin: number;
  readonly expectedMax: number;
  readonly deckLines: readonly string[];
}

export interface DeviceModelTemperaturePoint {
  readonly temperatureKelvin: number;
  readonly expectedMin: number;
  readonly expectedMax: number;
}

export interface DeviceModelTemperatureBehaviorFixture {
  readonly name: string;
  readonly kind: ModelCardKind;
  readonly model: NormalizedModelCard;
  readonly circuit: Circuit;
  readonly probeNode: string;
  readonly nominalTemperatureKelvin: number;
  readonly energyGapElectronVolts: number;
  readonly temperatureBehavior: string;
  readonly temperaturePoints: readonly DeviceModelTemperaturePoint[];
  readonly deckLines: readonly string[];
}

export interface DeviceModelCapacitanceBehaviorFixture {
  readonly name: string;
  readonly kind: ModelCardKind;
  readonly model: NormalizedModelCard;
  readonly circuit: Circuit;
  readonly probeNode: string;
  readonly frequencyHz: number;
  readonly expectedMagnitudeMin: number;
  readonly expectedMagnitudeMax: number;
  readonly capacitanceBehavior: string;
  readonly deckLines: readonly string[];
}

export interface DeviceModelNoiseBehaviorFixture {
  readonly name: string;
  readonly kind: ModelCardKind;
  readonly model: NormalizedModelCard;
  readonly circuit: Circuit;
  readonly outputNode: string;
  readonly inputSource: string;
  readonly frequencyHz: number;
  readonly expectedNoiseElement: string;
  readonly expectedNoiseType: NoiseType;
  readonly expectedSourcePsdMin: number;
  readonly expectedSourcePsdMax: number;
  readonly expectedOutputPsdMin: number;
  readonly expectedOutputPsdMax: number;
  readonly noiseBehavior: string;
  readonly deckLines: readonly string[];
}

export interface DeviceModelChargeBehaviorFixture {
  readonly name: string;
  readonly kind: ModelCardKind;
  readonly model: NormalizedModelCard;
  readonly circuit: Circuit;
  readonly probeNode: string;
  readonly timeStepSeconds: number;
  readonly stopTimeSeconds: number;
  readonly storageCapacitanceFarads: number;
  readonly expectedInitialMin: number;
  readonly expectedInitialMax: number;
  readonly expectedFinalMin: number;
  readonly expectedFinalMax: number;
  readonly chargeBehavior: string;
  readonly deckLines: readonly string[];
}

export interface DeviceModelReferenceDeckAuditFixture {
  readonly name: string;
  readonly kind: ModelCardKind;
  readonly model: NormalizedModelCard;
  readonly analysis: string;
  readonly reference: string;
  readonly expectedBehavior: string;
  readonly deckLines: readonly string[];
}

export interface DeviceModelReferenceDeckAuditIssue {
  readonly fixtureName: string;
  readonly field: string;
  readonly message: string;
}

export interface DeviceModelReferenceDeckAuditGateReport {
  readonly passed: boolean;
  readonly fixtureCount: number;
  readonly expectedKinds: readonly ModelCardKind[];
  readonly expectedAnalyses: readonly string[];
  readonly issues: readonly DeviceModelReferenceDeckAuditIssue[];
}

export interface DeviceModelReferenceDeckAuditGateCoverageDigest {
  readonly passed: boolean;
  readonly fixtureCount: number;
  readonly expectedPairCount: number;
  readonly coveredPairCount: number;
  readonly missingPairCount: number;
  readonly issueCount: number;
  readonly issueFields: readonly string[];
}

export interface DeviceModelReferenceDeckAuditGateIssueSummary {
  readonly field: string;
  readonly issueCount: number;
  readonly fixtureNames: readonly string[];
  readonly messages: readonly string[];
}

export interface DeviceModelReferenceDeckAuditSummary {
  readonly kind: ModelCardKind | string;
  readonly fixtureCount: number;
  readonly analyses: readonly string[];
  readonly missingAnalyses: readonly string[];
  readonly deckLineCount: number;
  readonly references: readonly string[];
}

export interface DeviceModelReferenceDeckAuditAnalysisSummary {
  readonly analysis: string;
  readonly fixtureCount: number;
  readonly kinds: readonly (ModelCardKind | string)[];
  readonly missingKinds: readonly ModelCardKind[];
  readonly deckLineCount: number;
  readonly references: readonly string[];
}

export interface DeviceModelReferenceDeckAuditMatrixRow {
  readonly kind: ModelCardKind | string;
  readonly fixtureCount: number;
  readonly op: string;
  readonly temperature: string;
  readonly ac: string;
  readonly noise: string;
  readonly tran: string;
  readonly missingAnalyses: readonly string[];
  readonly extraAnalyses: readonly string[];
  readonly deckLineCount: number;
}

const REFERENCE_DECK_AUDIT_EXPECTED_KINDS: readonly ModelCardKind[] = [
  "D",
  "NPN",
  "NJF",
  "NMOS",
];
const REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES = [
  "op",
  "temperature",
  "ac",
  "noise",
  "tran",
] as const;
const MODEL_CARD_SUPPORTED_PARAMETER_COVERAGE_KINDS: readonly ModelCardKind[] = [
  "D",
  "NPN",
  "PNP",
  "NJF",
  "PJF",
  "NMOS",
  "PMOS",
];
const MODEL_CARD_SUPPORTED_PARAMETER_COVERAGE_EXPECTED_SUMMARIES: Readonly<
  Record<ModelCardKind, readonly [number, number, number, number]>
> = {
  D: [15, 21, 5, 3],
  NPN: [41, 58, 13, 4],
  PNP: [41, 58, 13, 4],
  NJF: [22, 30, 7, 3],
  PJF: [22, 30, 7, 3],
  NMOS: [33, 41, 7, 3],
  PMOS: [33, 41, 7, 3],
};

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
  readonly diagnostics: DcSolverDiagnostics;
  voltage(node: string): number | undefined;
  branchCurrent(sourceName: string): number | undefined;
}

export type DcConvergenceAid = "newton" | "gmin" | "source" | "pseudo_transient" | "none";

export type LinearSolverKind = "none" | "dense_real" | "sparse_real" | "dense_complex" | "sparse_complex";

export type LinearSolverBackend = "none" | "dense_gaussian" | "native_sparse_gaussian";

export interface LinearSolverProfile {
  readonly matrixSize: number;
  readonly solver: LinearSolverKind;
  readonly backend: LinearSolverBackend;
  readonly structuralNonzeros: number;
  readonly density: number;
  readonly fillInNonzeros: number;
  readonly fallbackReason?: string;
}

export interface DcSolverDiagnostics {
  readonly matrixSize: number;
  readonly solver: LinearSolverKind;
  readonly tolerance: number;
  readonly maxDelta: number;
  readonly convergenceAid: DcConvergenceAid;
  readonly newtonStepLimit?: number;
  readonly limitedNewtonSteps: number;
  readonly minimumDampingFactor: number;
  readonly solverProfile: LinearSolverProfile;
}

export interface DcOpOptions {
  readonly maxIterations?: number;
  readonly tolerance?: number;
  readonly convergenceAids?: boolean;
  readonly pseudoTransientSteps?: number;
  readonly pseudoTransientConductance?: number;
  readonly pseudoTransientMaxIterations?: number;
  readonly newtonStepLimit?: number | null;
}

export interface CornerOverride {
  readonly elementName: string;
  readonly parameter:
    | "resistance"
    | "capacitance"
    | "inductance"
    | "voltage"
    | "current"
    | "conductance";
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

export interface TemperatureDcPoint {
  readonly temperatureKelvin: number;
  readonly result: DcResult;
}

export interface TemperatureDcResult {
  readonly points: readonly TemperatureDcPoint[];
}

export interface CornerTemperatureDcPoint {
  readonly cornerName: string;
  readonly points: readonly TemperatureDcPoint[];
}

export interface CornerTemperatureDcResult {
  readonly points: readonly CornerTemperatureDcPoint[];
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

export type NoiseType = "thermal" | "shot" | "flicker";

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

export interface ProbeMeasurement {
  readonly name: string;
  readonly analysis: string;
  readonly probe: string;
  readonly mode: string;
  readonly value: number;
  readonly fromValue?: number;
  readonly toValue?: number;
}

export interface CornerTransientPoint {
  readonly cornerName: string;
  readonly points: readonly TransientPoint[];
}

export interface CornerTransientResult {
  readonly points: readonly CornerTransientPoint[];
}

export interface CornerAdaptiveTransientPoint {
  readonly cornerName: string;
  readonly result: AdaptiveTransientResult;
}

export interface CornerAdaptiveTransientResult {
  readonly points: readonly CornerAdaptiveTransientPoint[];
}

export type DigitalState = "low" | "high";

export interface DigitalEvent {
  readonly timeSeconds: number;
  readonly state: DigitalState;
}

export interface DigitalEventStream {
  readonly signalName: string;
  readonly events: readonly DigitalEvent[];
}

export interface DigitalTransientBridgeResult {
  readonly points: readonly TransientPoint[];
  readonly outputStreams: readonly DigitalEventStream[];
}

export interface CornerDigitalTransientBridgePoint {
  readonly cornerName: string;
  readonly result: DigitalTransientBridgeResult;
}

export interface CornerDigitalTransientBridgeResult {
  readonly points: readonly CornerDigitalTransientBridgePoint[];
}

export interface AdaptiveDigitalTransientBridgeResult {
  readonly result: AdaptiveTransientResult;
  readonly outputStreams: readonly DigitalEventStream[];
}

export interface CornerAdaptiveDigitalTransientBridgePoint {
  readonly cornerName: string;
  readonly result: AdaptiveDigitalTransientBridgeResult;
}

export interface CornerAdaptiveDigitalTransientBridgeResult {
  readonly points: readonly CornerAdaptiveDigitalTransientBridgePoint[];
}

export interface DigitalBridgeSchedule {
  readonly stopTime: number;
  readonly breakpoints: readonly number[];
}

export class DigitalLogicLevels {
  constructor(
    readonly lowVoltage: number,
    readonly highVoltage: number,
    readonly transitionSeconds: number,
  ) {}

  static cmos1v8(transitionSeconds: number): DigitalLogicLevels {
    return new DigitalLogicLevels(0.0, 1.8, transitionSeconds);
  }

  voltageFor(state: DigitalState): number {
    return normalizeDigitalState(state) === "low" ? this.lowVoltage : this.highVoltage;
  }
}

export class DigitalThresholds {
  constructor(
    readonly lowMaxVoltage: number,
    readonly highMinVoltage: number,
  ) {}

  static cmos1v8(): DigitalThresholds {
    return new DigitalThresholds(0.6, 1.2);
  }

  classify(voltage: number): DigitalState | undefined {
    if (voltage <= this.lowMaxVoltage) {
      return "low";
    }
    if (voltage >= this.highMinVoltage) {
      return "high";
    }
    return undefined;
  }
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

export interface CornerFourierPoint {
  readonly cornerName: string;
  readonly result: FourierResult;
}

export interface CornerFourierResult {
  readonly fundamentalFrequencyHz: number;
  readonly points: readonly CornerFourierPoint[];
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

export interface CornerDistortionPoint {
  readonly cornerName: string;
  readonly result: DistortionResult;
}

export interface CornerDistortionResult {
  readonly inputSource: string;
  readonly outputProbe: string;
  readonly points: readonly CornerDistortionPoint[];
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

export type PoleZeroTopology =
  | "rc-lowpass"
  | "rc-highpass"
  | "rlc-lowpass"
  | "rlc-highpass"
  | "rlc-bandpass"
  | "rlc-notch";

export interface CornerPoleZeroPoint {
  readonly cornerName: string;
  readonly result: PoleZeroResult;
}

export interface CornerPoleZeroResult {
  readonly inputSource: string;
  readonly outputNode: string;
  readonly topology: PoleZeroTopology;
  readonly points: readonly CornerPoleZeroPoint[];
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

export function poleZeroCorners(
  circuit: Circuit,
  inputSource: string,
  outputNode: string,
  topology: PoleZeroTopology,
  corners: readonly CornerSpec[],
): CornerPoleZeroResult {
  return {
    inputSource,
    outputNode,
    topology,
    points: corners.map((corner) => ({
      cornerName: corner.name,
      result: poleZeroForTopology(
        circuitWithCorner(circuit, corner),
        inputSource,
        outputNode,
        topology,
      ),
    })),
  };
}

function poleZeroForTopology(
  circuit: Circuit,
  inputSource: string,
  outputNode: string,
  topology: PoleZeroTopology,
): PoleZeroResult {
  switch (topology) {
    case "rc-lowpass":
      return poleZeroRcLowpass(circuit, inputSource, outputNode);
    case "rc-highpass":
      return poleZeroRcHighpass(circuit, inputSource, outputNode);
    case "rlc-lowpass":
      return poleZeroRlcLowpass(circuit, inputSource, outputNode);
    case "rlc-highpass":
      return poleZeroRlcHighpass(circuit, inputSource, outputNode);
    case "rlc-bandpass":
      return poleZeroRlcBandpass(circuit, inputSource, outputNode);
    case "rlc-notch":
      return poleZeroRlcNotch(circuit, inputSource, outputNode);
  }
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

export function distortionFromTransientCorners(
  circuit: Circuit,
  timeStep: number,
  stopTime: number,
  fundamentalFrequencyHz: number,
  inputSource: string,
  outputProbe: string,
  corners: readonly CornerSpec[],
  harmonics = 9,
  startTime?: number,
  method: TransientMethod = "euler",
): CornerDistortionResult {
  return {
    inputSource,
    outputProbe,
    points: corners.map((corner) => ({
      cornerName: corner.name,
      result: distortionFromTransient(
        transient(circuitWithCorner(circuit, corner), timeStep, stopTime, method),
        fundamentalFrequencyHz,
        inputSource,
        outputProbe,
        harmonics,
        startTime,
      ),
    })),
  };
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

export interface CornerPssPoint {
  readonly cornerName: string;
  readonly result: PssResult;
}

export interface CornerPssResult {
  readonly points: readonly CornerPssPoint[];
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
    case "custom-model":
      return { ...element, name, positive: mapSubcktNode(element.positive, instanceName, nodeMap), negative: mapSubcktNode(element.negative, instanceName, nodeMap) };
    case "diode":
      return diode(name, mapSubcktNode(element.anode, instanceName, nodeMap), mapSubcktNode(element.cathode, instanceName, nodeMap), element.saturationCurrent, element.thermalVoltage, element.emissionCoefficient, element.breakdownVoltage, element.breakdownCurrent, element.junctionCapacitance, element.transitTime, element.junctionPotential, element.gradingCoefficient, element.forwardBiasDepletionCoefficient, element.saturationCurrentTemperatureExponent, element.energyGapElectronVolts, element.seriesResistance, element.flickerNoiseCoefficient, element.flickerNoiseExponent);
    case "jfet":
      return jfet(name, mapSubcktNode(element.drain, instanceName, nodeMap), mapSubcktNode(element.gate, instanceName, nodeMap), mapSubcktNode(element.source, instanceName, nodeMap), element.polarity, element.beta, element.thresholdVoltage, element.channelLengthModulation, element.gateSourceCapacitance, element.gateDrainCapacitance, element.flickerNoiseCoefficient, element.flickerNoiseExponent, element.junctionPotential, element.forwardBiasDepletionCoefficient, element.gateSaturationCurrent, element.gateSaturationCurrentTemperatureExponent, element.bandgapVoltage, element.dopingTailParameter, element.noiseEquationLevel, element.channelNoiseCoefficient, element.drainResistance, element.sourceResistance, element.thresholdVoltageTemperatureCoefficient, element.alternativeThresholdVoltageTemperatureCoefficient, element.nominalTemperatureKelvin, element.mobilityTemperatureExponent, element.mobilityTemperatureCoefficient);
    case "bjt":
      return bjt(name, mapSubcktNode(element.collector, instanceName, nodeMap), mapSubcktNode(element.base, instanceName, nodeMap), mapSubcktNode(element.emitter, instanceName, nodeMap), element.polarity, element.saturationCurrent, element.forwardBeta, element.thermalVoltage, element.baseEmitterCapacitance, element.baseCollectorCapacitance, element.forwardTransitTime, element.reverseTransitTime, element.saturationCurrentTemperatureExponent, element.energyGapElectronVolts, element.forwardEarlyVoltage, element.forwardEmissionCoefficient, element.reverseEmissionCoefficient, element.baseEmitterJunctionPotential, element.baseEmitterGradingCoefficient, element.baseCollectorJunctionPotential, element.baseCollectorGradingCoefficient, element.forwardBiasDepletionCoefficient, element.reverseEarlyVoltage, element.forwardBetaRolloffCurrent, element.baseEmitterLeakageSaturationCurrent, element.baseEmitterLeakageEmissionCoefficient, element.baseCollectorLeakageSaturationCurrent, element.baseCollectorLeakageEmissionCoefficient, element.forwardBetaTemperatureExponent, element.reverseBeta, element.reverseBetaRolloffCurrent, element.nominalTemperatureKelvin, element.flickerNoiseCoefficient, element.flickerNoiseExponent, element.forwardExcessPhaseDegrees, element.forwardTransitTimeBiasCoefficient, element.forwardTransitTimeCurrent, element.forwardTransitTimeVoltage, element.emitterResistance, element.collectorResistance, element.baseResistance, element.minimumBaseResistance, element.baseResistanceHalfCurrent, element.baseCollectorCapacitanceFraction);
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

export function customLinearConductanceModel(
  name: string,
  positive: string,
  negative: string,
  conductanceSiemens: number,
  options: {
    readonly currentOffsetAmps?: number;
    readonly modelName?: string;
    readonly parameters?: Readonly<Record<string, number>>;
  } = {},
): CustomModel {
  return {
    kind: "custom-model",
    name,
    positive,
    negative,
    modelName: options.modelName ?? "linear_conductance",
    parameters: options.parameters ?? {},
    conductanceSiemens,
    currentOffsetAmps: options.currentOffsetAmps ?? 0.0,
  };
}

const CUSTOM_MODEL_SUBSET = "two-terminal-current-contribution-v0";
const CUSTOM_MODEL_FORBIDDEN_PATTERNS: readonly (readonly [string, string])[] = [
  ["ddt", "dynamic charge operators are not accepted in this custom-model subset"],
  ["idt", "dynamic integration operators are not accepted in this custom-model subset"],
  ["laplace", "Laplace-domain operators are not accepted in this custom-model subset"],
  ["cross", "event crossing operators are not accepted in this custom-model subset"],
  ["timer", "timer events are not accepted in this custom-model subset"],
  ["@(", "event controls are not accepted in this custom-model subset"],
  ["$finish", "system tasks are not accepted in this custom-model subset"],
  ["$stop", "system tasks are not accepted in this custom-model subset"],
  ["$display", "system tasks are not accepted in this custom-model subset"],
  ["initial", "procedural initial blocks are not accepted in this custom-model subset"],
  ["always", "procedural always blocks are not accepted in this custom-model subset"],
  ["analog function", "analog functions are not accepted in this custom-model subset"],
  ["discipline", "discipline declarations are not accepted in this custom-model subset"],
  ["branch ", "named branch declarations are not accepted in this custom-model subset"],
];

export function analyzeCustomModelSource(source: string): CustomModelSourceAnalysis {
  const diagnostics: CustomModelDiagnostic[] = [];
  const trimmed = source.trim();
  if (trimmed.length === 0) {
    return {
      accepted: false,
      subset: CUSTOM_MODEL_SUBSET,
      terminals: [],
      diagnostics: [{
        code: "CUSTOM_MODEL_EMPTY_SOURCE",
        message: "custom model source is empty",
        severity: "error",
      }],
    };
  }

  const lowered = trimmed.toLowerCase();
  for (const [token, message] of CUSTOM_MODEL_FORBIDDEN_PATTERNS) {
    if (lowered.includes(token)) {
      diagnostics.push({
        code: "CUSTOM_MODEL_FORBIDDEN_CONSTRUCT",
        message,
        severity: "error",
      });
    }
  }

  const moduleMatch = /\bmodule\s+([A-Za-z_][A-Za-z0-9_$]*)\s*\(([^)]*)\)\s*;/i.exec(trimmed);
  const moduleName = moduleMatch?.[1];
  const terminals = moduleMatch === null
    ? []
    : moduleMatch[2]
        .split(",")
        .map((port) => port.trim())
        .filter((port) => port.length > 0);
  if (moduleMatch === null) {
    diagnostics.push({
      code: "CUSTOM_MODEL_MISSING_MODULE",
      message: "custom model source must declare a module with a port list",
      severity: "error",
    });
  } else if (terminals.length < 2) {
    diagnostics.push({
      code: "CUSTOM_MODEL_PORT_COUNT",
      message: "custom model module must expose at least two terminals",
      severity: "error",
    });
  }

  const contributionMatch = /\bI\s*\(\s*([A-Za-z_][A-Za-z0-9_$]*)\s*,\s*([A-Za-z_][A-Za-z0-9_$]*)\s*\)\s*<\+/i.exec(trimmed);
  const contribution = contributionMatch === null
    ? undefined
    : [contributionMatch[1], contributionMatch[2]] as const;
  if (contribution === undefined) {
    diagnostics.push({
      code: "CUSTOM_MODEL_MISSING_CONTRIBUTION",
      message: "custom model source must contain a two-terminal I(p,n) <+ contribution",
      severity: "error",
    });
  } else if (
    terminals.length > 0 &&
    contribution.some((terminal) => !terminals.includes(terminal))
  ) {
    diagnostics.push({
      code: "CUSTOM_MODEL_UNKNOWN_TERMINAL",
      message: "current contribution terminals must be declared module ports",
      severity: "error",
    });
  }

  return {
    accepted: !diagnostics.some((diagnostic) => diagnostic.severity === "error"),
    subset: CUSTOM_MODEL_SUBSET,
    moduleName,
    terminals,
    contribution,
    diagnostics,
  };
}

const COMMON_KNOWN_INCOMPATIBILITIES = Object.freeze([
  "binary rawfile output is not part of this release gate",
  ".control blocks and vendor-specific directives are intentionally excluded",
  "golden values cover named probes, not byte-for-byte waveform dumps",
]);

const COMPATIBILITY_CORPUS: readonly CompatibilityDeck[] = Object.freeze([
  {
    id: "dc-op-resistive-divider",
    title: "DC operating point resistive divider",
    analysis: "op",
    netlist: `* dc-op-resistive-divider
V1 in 0 DC 10
R1 in out 10000
R2 out 0 10000
.op
.end
`,
    oracle: {
      reference: "closed-form",
      version: "divider-v1",
      source: "V(out)=V1*R2/(R1+R2); I(V1)=-V1/(R1+R2)",
    },
    goldenValues: [
      { name: "V(out)", value: 5.0, unit: "V", absoluteTolerance: 1.0e-9, relativeTolerance: 1.0e-9 },
      { name: "I(V1)", value: -5.0e-4, unit: "A", absoluteTolerance: 1.0e-12, relativeTolerance: 1.0e-9 },
    ],
    knownIncompatibilities: COMMON_KNOWN_INCOMPATIBILITIES,
  },
  {
    id: "dc-sweep-resistive-divider",
    title: "DC source sweep resistive divider",
    analysis: "dc",
    netlist: `* dc-sweep-resistive-divider
V1 in 0 DC 0
R1 in out 10000
R2 out 0 10000
.dc V1 0 10 5
.end
`,
    oracle: {
      reference: "closed-form",
      version: "divider-sweep-v1",
      source: "V(out)=V1*0.5 at each sweep point",
    },
    goldenValues: [
      { name: "points", value: 3.0, unit: "count", absoluteTolerance: 0.0, relativeTolerance: 0.0 },
      { name: "V(out)@V1=10", value: 5.0, unit: "V", absoluteTolerance: 1.0e-9, relativeTolerance: 1.0e-9 },
    ],
    knownIncompatibilities: COMMON_KNOWN_INCOMPATIBILITIES,
  },
  {
    id: "ac-rc-lowpass",
    title: "AC RC low-pass cutoff",
    analysis: "ac",
    netlist: `* ac-rc-lowpass
V1 in 0 DC 0 AC 1
R1 in out 1000
C1 out 0 1u
.ac dec 1 1 1k
.end
`,
    oracle: {
      reference: "closed-form",
      version: "rc-lowpass-v1",
      source: "|V(out)|=1/sqrt(1+(2*pi*f*R*C)^2)",
    },
    goldenValues: [
      { name: "f_c", value: 159.15494309189535, unit: "Hz", absoluteTolerance: 1.0e-9, relativeTolerance: 1.0e-9 },
      { name: "|V(out)|@f_c", value: 0.7071067811865475, unit: "V", absoluteTolerance: 1.0e-9, relativeTolerance: 1.0e-9 },
    ],
    knownIncompatibilities: COMMON_KNOWN_INCOMPATIBILITIES,
  },
  {
    id: "tran-rc-step",
    title: "Transient RC step response",
    analysis: "tran",
    netlist: `* tran-rc-step
V1 in 0 PULSE(0 1 0 1n 1n 1m 2m)
R1 in out 1000
C1 out 0 1u
.tran 0.0001 0.001
.end
`,
    oracle: {
      reference: "closed-form",
      version: "rc-step-v1",
      source: "V(out,t)=1-exp(-t/(R*C)) after an ideal 1 V step",
    },
    goldenValues: [
      { name: "V(out)@1ms", value: 0.6321205588285577, unit: "V", absoluteTolerance: 1.0e-6, relativeTolerance: 1.0e-6 },
    ],
    knownIncompatibilities: [
      ...COMMON_KNOWN_INCOMPATIBILITIES,
      "finite-edge pulse decks compare at the idealized step oracle point",
    ],
  },
  {
    id: "tf-resistive-divider",
    title: "Transfer-function resistive divider",
    analysis: "tf",
    netlist: `* tf-resistive-divider
V1 in 0 DC 10
R1 in out 10000
R2 out 0 10000
.tf V(out) V1
.end
`,
    oracle: {
      reference: "closed-form",
      version: "divider-tf-v1",
      source: "gain=R2/(R1+R2); input resistance=R1+R2",
    },
    goldenValues: [
      { name: "gain", value: 0.5, unit: "V/V", absoluteTolerance: 1.0e-9, relativeTolerance: 1.0e-9 },
      { name: "input_resistance", value: 20000.0, unit: "ohm", absoluteTolerance: 1.0e-6, relativeTolerance: 1.0e-9 },
    ],
    knownIncompatibilities: COMMON_KNOWN_INCOMPATIBILITIES,
  },
]);

const SUPPORTED_COMPATIBILITY_ANALYSES = new Set(["op", "dc", "ac", "tran", "tf"]);
const REQUIRED_COMPATIBILITY_ANALYSES = ["op", "dc", "ac", "tran"];
const UNSUPPORTED_DECK_CONTROL_DIRECTIVES = new Set([".include", ".lib", ".control"]);
const UNSUPPORTED_RESOLVED_DIRECTIVES = new Set([".control"]);
const UNSUPPORTED_PARAMETER_DIRECTIVES = new Set<string>();
const SUPPORTED_CONTROL_BLOCK_COMMANDS = new Set([
  "op",
  ".op",
  "dc",
  ".dc",
  "ac",
  ".ac",
  "tran",
  ".tran",
  "save",
  ".save",
  "probe",
  ".probe",
  "measure",
  ".measure",
  "meas",
  ".meas",
  "four",
  ".four",
  "fourier",
  ".fourier",
  "print",
  ".print",
  "plot",
  ".plot",
]);
const NOOP_CONTROL_BLOCK_COMMANDS = new Set([
  "display",
  ".display",
  "listing",
  ".listing",
  "show",
  ".show",
  "showmod",
  ".showmod",
  "status",
  ".status",
  "version",
  ".version",
  "help",
  ".help",
  "echo",
  ".echo",
  "rusage",
  ".rusage",
  "where",
  ".where",
  "run",
  ".run",
  "reset",
  ".reset",
  "quit",
  ".quit",
]);
const NOOP_CONTROL_BLOCK_ARGUMENT_COMMANDS = new Set(["write", ".write"]);
const NOOP_CONTROL_BLOCK_VECTOR_ARGUMENT_COMMANDS = new Set(["wrdata", ".wrdata"]);
const NOOP_CONTROL_BLOCK_SET_OPTIONS = new Set([
  "noaskquit",
  "filetype=ascii",
  "wr_vecnames",
  "wr_singlescale",
  "appendwrite",
]);
const SCRIPT_CONTROL_BLOCK_COMMANDS = new Set(["source", ".source", "shell", ".shell"]);
const WORKDIR_CONTROL_BLOCK_COMMANDS = new Set(["cd", ".cd"]);
const CONTROL_FLOW_CONTROL_BLOCK_COMMANDS = new Set([
  "if",
  ".if",
  "else",
  ".else",
  "end",
  ".end",
  "while",
  ".while",
  "foreach",
  ".foreach",
  "repeat",
  ".repeat",
  "dowhile",
  ".dowhile",
  "break",
  ".break",
  "continue",
  ".continue",
]);
const VARIABLE_CONTROL_BLOCK_COMMANDS = new Set([
  "let",
  ".let",
  "alter",
  ".alter",
  "alterparam",
  ".alterparam",
  "set",
  ".set",
  "unset",
  ".unset",
]);

export function compatibilityCorpus(): readonly CompatibilityDeck[] {
  return COMPATIBILITY_CORPUS;
}

export function analyzeDeckControls(netlist: string): DeckControlSummary {
  const activeLines: string[] = [];
  const controlLines: string[] = [];
  const writeMarkers: string[] = [];
  const rawfileOptions: string[] = [];
  const diagnostics: DeckControlDiagnostic[] = [];
  let endLineNumber: number | undefined;
  let inControlBlock = false;

  const lines = netlist.split(/\r?\n/);
  for (let index = 0; index < lines.length; index++) {
    const lineNumber = index + 1;
    const stripped = lines[index].trim();
    if (stripped.length === 0 || stripped.startsWith("*") || stripped.startsWith(";")) {
      continue;
    }
    const directive = deckDirective(stripped);
    if (inControlBlock) {
      if (directive === ".endc") {
        inControlBlock = false;
        continue;
      }
      const controlLine = controlBlockCommandAsDeckLine(stripped);
      if (controlLine !== undefined) {
        activeLines.push(controlLine);
        controlLines.push(controlLine);
        continue;
      }
      const writeMarker = controlBlockWriteMarker(stripped);
      if (writeMarker !== undefined) {
        writeMarkers.push(writeMarker);
        continue;
      }
      const rawfileOption = controlBlockRawfileOption(stripped);
      if (rawfileOption !== undefined) {
        rawfileOptions.push(rawfileOption);
        continue;
      }
      if (isNoopControlBlockCommand(stripped)) {
        continue;
      }
      if (isScriptControlBlockCommand(stripped)) {
        diagnostics.push({
          code: "SPICE_DECK_CONTROL_SCRIPT_COMMAND",
          directive: ".control",
          lineNumber,
          message: controlBlockScriptPolicyMessage(stripped),
          severity: "error",
        });
        continue;
      }
      if (isWorkdirControlBlockCommand(stripped)) {
        diagnostics.push({
          code: "SPICE_DECK_CONTROL_WORKDIR_COMMAND",
          directive: ".control",
          lineNumber,
          message: controlBlockWorkdirPolicyMessage(stripped),
          severity: "error",
        });
        continue;
      }
      if (isControlFlowControlBlockCommand(stripped)) {
        diagnostics.push({
          code: "SPICE_DECK_CONTROL_FLOW_COMMAND",
          directive: ".control",
          lineNumber,
          message: controlBlockFlowPolicyMessage(stripped),
          severity: "error",
        });
        continue;
      }
      if (isVariableControlBlockCommand(stripped)) {
        diagnostics.push({
          code: "SPICE_DECK_CONTROL_VARIABLE_COMMAND",
          directive: ".control",
          lineNumber,
          message: controlBlockVariablePolicyMessage(stripped),
          severity: "error",
        });
        continue;
      }
      diagnostics.push({
        code: "SPICE_DECK_CONTROL_COMMAND",
        directive: ".control",
        lineNumber,
        message: `${JSON.stringify(stripped)} inside .control is not executed by the deck execution foothold yet`,
        severity: "error",
      });
      continue;
    }
    if (directive === ".end") {
      endLineNumber = lineNumber;
      break;
    }
    if (directive !== undefined && UNSUPPORTED_DECK_CONTROL_DIRECTIVES.has(directive)) {
      diagnostics.push({
        code: "SPICE_DECK_UNSUPPORTED_DIRECTIVE",
        directive,
        lineNumber,
        message: `${directive} is not supported by the deck execution foothold yet`,
        severity: "error",
      });
      if (directive === ".control") {
        inControlBlock = true;
        continue;
      }
    }
    activeLines.push(stripped);
  }

  return {
    activeLines,
    controlLines,
    writeMarkers,
    rawfileOptions,
    terminated: endLineNumber !== undefined,
    endLineNumber,
    diagnostics,
  };
}

export function resolveDeckSources(
  netlist: string,
  sources: Readonly<Record<string, string>>,
): DeckResolutionSummary {
  const state: DeckResolutionState = {
    diagnostics: [],
    includedPaths: [],
    librarySections: [],
  };
  const resolved = resolveDeckLines(netlist, "<deck>", sources, state, []);

  return {
    activeLines: resolved.activeLines,
    terminated: resolved.terminated,
    endLineNumber: resolved.endLineNumber,
    diagnostics: state.diagnostics,
    includedPaths: state.includedPaths,
    librarySections: state.librarySections,
  };
}

export function resolveDeckParameters(netlist: string): DeckParameterSummary {
  const state = new DeckParameterState();
  collectParameterFunctions(netlist, state);
  const activeLines: string[] = [];
  let endLineNumber: number | undefined;

  const lines = netlist.split(/\r?\n/);
  for (let index = 0; index < lines.length; index++) {
    const lineNumber = index + 1;
    const stripped = lines[index].trim();
    if (stripped.length === 0 || stripped.startsWith("*") || stripped.startsWith(";")) {
      continue;
    }
    const directive = deckDirective(stripped);
    if (directive === ".end") {
      endLineNumber = lineNumber;
      break;
    }
    if (directive === ".param") {
      resolveParamLine(stripped, lineNumber, state);
      continue;
    }
    if (directive === ".func") {
      continue;
    }
    if (directive !== undefined && UNSUPPORTED_PARAMETER_DIRECTIVES.has(directive)) {
      addParameterDiagnostic(state, {
        code: "SPICE_DECK_UNSUPPORTED_DIRECTIVE",
        directive,
        lineNumber,
        message: `${directive} is not supported by the parameter resolver yet`,
      });
      activeLines.push(stripped);
      continue;
    }
    activeLines.push(rewriteParameterExpressions(stripped, lineNumber, state));
  }

  return {
    activeLines,
    terminated: endLineNumber !== undefined,
    endLineNumber,
    parameters: state.parameterValues(),
    diagnostics: state.diagnostics,
  };
}

export function resolveDeckInitialConditions(netlist: string): DeckInitialConditionSummary {
  const state = new DeckInitialConditionState();
  const activeLines: string[] = [];
  let endLineNumber: number | undefined;

  const lines = netlist.split(/\r?\n/);
  for (let index = 0; index < lines.length; index++) {
    const lineNumber = index + 1;
    const stripped = lines[index].trim();
    if (stripped.length === 0 || stripped.startsWith("*") || stripped.startsWith(";")) {
      continue;
    }
    const directive = deckDirective(stripped);
    if (directive === ".end") {
      endLineNumber = lineNumber;
      break;
    }
    if (directive === ".ic" || directive === ".nodeset") {
      resolveNodeConditionLine(stripped, lineNumber, directive, state);
      continue;
    }
    activeLines.push(stripped);
  }

  return {
    activeLines,
    terminated: endLineNumber !== undefined,
    endLineNumber,
    initialConditions: state.initialConditions,
    nodesets: state.nodesets,
    diagnostics: state.diagnostics,
  };
}

export function resolveDeckFunctions(netlist: string): DeckFunctionSummary {
  const state = new DeckFunctionState();
  const activeLines: string[] = [];
  let endLineNumber: number | undefined;

  const lines = netlist.split(/\r?\n/);
  for (let index = 0; index < lines.length; index++) {
    const lineNumber = index + 1;
    const stripped = lines[index].trim();
    if (stripped.length === 0 || stripped.startsWith("*") || stripped.startsWith(";")) {
      continue;
    }
    const directive = deckDirective(stripped);
    if (directive === ".end") {
      endLineNumber = lineNumber;
      break;
    }
    if (directive === ".func") {
      resolveFunctionLine(stripped, lineNumber, state);
      continue;
    }
    activeLines.push(stripped);
  }

  return {
    activeLines,
    terminated: endLineNumber !== undefined,
    endLineNumber,
    functions: state.functions,
    diagnostics: state.diagnostics,
  };
}

export function resolveDeckMeasurements(netlist: string): DeckMeasurementSummary {
  const state = new DeckMeasurementState();
  const activeLines: string[] = [];
  let endLineNumber: number | undefined;

  const lines = netlist.split(/\r?\n/);
  for (let index = 0; index < lines.length; index++) {
    const lineNumber = index + 1;
    const stripped = lines[index].trim();
    if (stripped.length === 0 || stripped.startsWith("*") || stripped.startsWith(";")) {
      continue;
    }
    const directive = deckDirective(stripped);
    if (directive === ".end") {
      endLineNumber = lineNumber;
      break;
    }
    if (directive === ".measure" || directive === ".meas") {
      resolveMeasurementLine(stripped, lineNumber, directive, state);
      continue;
    }
    activeLines.push(stripped);
  }

  return {
    activeLines,
    terminated: endLineNumber !== undefined,
    endLineNumber,
    measurements: state.measurements,
    diagnostics: state.diagnostics,
  };
}

export function resolveDeckFourier(netlist: string): DeckFourierSummary {
  const state = new DeckFourierState();
  const activeLines: string[] = [];
  let endLineNumber: number | undefined;

  const lines = netlist.split(/\r?\n/);
  for (let index = 0; index < lines.length; index++) {
    const lineNumber = index + 1;
    const stripped = lines[index].trim();
    if (stripped.length === 0 || stripped.startsWith("*") || stripped.startsWith(";")) {
      continue;
    }
    const directive = deckDirective(stripped);
    if (directive === ".end") {
      endLineNumber = lineNumber;
      break;
    }
    if (directive === ".four") {
      resolveFourierLine(stripped, lineNumber, state);
      continue;
    }
    activeLines.push(stripped);
  }

  return {
    activeLines,
    terminated: endLineNumber !== undefined,
    endLineNumber,
    fourier: state.fourier,
    diagnostics: state.diagnostics,
  };
}

export function resolveDeckOutputs(netlist: string): DeckOutputSummary {
  const state = new DeckOutputState();
  const activeLines: string[] = [];
  let endLineNumber: number | undefined;

  const lines = netlist.split(/\r?\n/);
  for (let index = 0; index < lines.length; index++) {
    const lineNumber = index + 1;
    const stripped = lines[index].trim();
    if (stripped.length === 0 || stripped.startsWith("*") || stripped.startsWith(";")) {
      continue;
    }
    const directive = deckDirective(stripped);
    if (directive === ".end") {
      endLineNumber = lineNumber;
      break;
    }
    if (directive === ".save" || directive === ".probe" || directive === ".print" || directive === ".plot") {
      resolveOutputLine(stripped, lineNumber, directive, state);
      continue;
    }
    activeLines.push(stripped);
  }

  return {
    activeLines,
    terminated: endLineNumber !== undefined,
    endLineNumber,
    selections: state.selections,
    diagnostics: state.diagnostics,
  };
}

export function selectDeckOutputProbes(netlist: string, analysis: string): string[] {
  const summary = resolveDeckOutputs(netlist);
  if (summary.diagnostics.length > 0) {
    const diagnostic = summary.diagnostics[0];
    throw invalidElement(
      "selectDeckOutputProbes",
      `line ${diagnostic.lineNumber}: ${diagnostic.message}`,
    );
  }
  const selected: string[] = [];
  const seen = new Set<string>();
  for (const selection of summary.selections) {
    if (selection.analysis !== undefined && !deckOutputAnalysisMatches(selection.analysis, analysis)) {
      continue;
    }
    for (const probe of selection.probes) {
      const key = deckOutputProbeKey(probe);
      if (seen.has(key)) {
        continue;
      }
      seen.add(key);
      selected.push(probe);
    }
  }
  return selected;
}

export function selectDeckOutputProbeLines(netlist: string, analysis: string): number[] {
  const summary = resolveDeckOutputs(netlist);
  if (summary.diagnostics.length > 0) {
    const diagnostic = summary.diagnostics[0];
    throw invalidElement(
      "selectDeckOutputProbeLines",
      `line ${diagnostic.lineNumber}: ${diagnostic.message}`,
    );
  }
  const selected: number[] = [];
  const seen = new Set<string>();
  for (const selection of summary.selections) {
    if (selection.analysis !== undefined && !deckOutputAnalysisMatches(selection.analysis, analysis)) {
      continue;
    }
    for (const probe of selection.probes) {
      const key = deckOutputProbeKey(probe);
      if (seen.has(key)) {
        continue;
      }
      seen.add(key);
      selected.push(selection.lineNumber);
    }
  }
  return selected;
}

export function selectDeckOutputDirectives(netlist: string, analysis: string): string[] {
  const summary = resolveDeckOutputs(netlist);
  if (summary.diagnostics.length > 0) {
    const diagnostic = summary.diagnostics[0];
    throw invalidElement(
      "selectDeckOutputDirectives",
      `line ${diagnostic.lineNumber}: ${diagnostic.message}`,
    );
  }
  const selected: string[] = [];
  const seen = new Set<string>();
  for (const selection of summary.selections) {
    if (selection.analysis !== undefined && !deckOutputAnalysisMatches(selection.analysis, analysis)) {
      continue;
    }
    if (seen.has(selection.directive)) {
      continue;
    }
    seen.add(selection.directive);
    selected.push(selection.directive);
  }
  return selected;
}

export function selectDeckOutputDirectiveAnalysisKinds(netlist: string, analysis: string): string[] {
  const summary = resolveDeckOutputs(netlist);
  if (summary.diagnostics.length > 0) {
    const diagnostic = summary.diagnostics[0];
    throw invalidElement(
      "selectDeckOutputDirectiveAnalysisKinds",
      `line ${diagnostic.lineNumber}: ${diagnostic.message}`,
    );
  }
  const selected: string[] = [];
  const seen = new Set<string>();
  for (const selection of summary.selections) {
    if (selection.analysis !== undefined && !deckOutputAnalysisMatches(selection.analysis, analysis)) {
      continue;
    }
    const analysisKind = selection.analysis ?? "global";
    if (seen.has(analysisKind)) {
      continue;
    }
    seen.add(analysisKind);
    selected.push(analysisKind);
  }
  return selected;
}

export function selectDeckOutputDirectiveLines(netlist: string, analysis: string): number[] {
  const summary = resolveDeckOutputs(netlist);
  if (summary.diagnostics.length > 0) {
    const diagnostic = summary.diagnostics[0];
    throw invalidElement(
      "selectDeckOutputDirectiveLines",
      `line ${diagnostic.lineNumber}: ${diagnostic.message}`,
    );
  }
  const selected: number[] = [];
  const seen = new Set<number>();
  for (const selection of summary.selections) {
    if (selection.analysis !== undefined && !deckOutputAnalysisMatches(selection.analysis, analysis)) {
      continue;
    }
    if (seen.has(selection.lineNumber)) {
      continue;
    }
    seen.add(selection.lineNumber);
    selected.push(selection.lineNumber);
  }
  return selected;
}

export function resolveDeckAnalyses(netlist: string): DeckAnalysisSummary {
  const state = new DeckAnalysisState();
  const activeLines: string[] = [];
  let endLineNumber: number | undefined;

  const lines = netlist.split(/\r?\n/);
  for (let index = 0; index < lines.length; index++) {
    const lineNumber = index + 1;
    const stripped = lines[index].trim();
    if (stripped.length === 0 || stripped.startsWith("*") || stripped.startsWith(";")) {
      continue;
    }
    const directive = deckDirective(stripped);
    if (directive === ".end") {
      endLineNumber = lineNumber;
      break;
    }
    if (
      directive === ".op" ||
      directive === ".dc" ||
      directive === ".ac" ||
      directive === ".tran" ||
      directive === ".tf" ||
      directive === ".sens" ||
      directive === ".noise"
    ) {
      resolveAnalysisLine(stripped, lineNumber, directive, state);
      continue;
    }
    activeLines.push(stripped);
  }

  return {
    activeLines,
    terminated: endLineNumber !== undefined,
    endLineNumber,
    analyses: state.analyses,
    diagnostics: state.diagnostics,
  };
}

export function selectDeckAnalysisPlan(netlist: string, analysis?: string): DeckAnalysisPlan {
  const summary = resolveDeckAnalyses(netlist);
  if (summary.diagnostics.length > 0) {
    const diagnostic = summary.diagnostics[0];
    throw invalidElement(
      "selectDeckAnalysisPlan",
      `line ${diagnostic.lineNumber}: ${diagnostic.message}`,
    );
  }

  const requestedAnalysis = analysis === undefined
    ? undefined
    : normalizeDeckAnalysisName(analysis);
  if (analysis !== undefined && requestedAnalysis === undefined) {
    throw invalidElement("selectDeckAnalysisPlan", `unsupported analysis ${JSON.stringify(analysis)}`);
  }

  let plans = [...summary.analyses];
  if (requestedAnalysis !== undefined) {
    plans = plans.filter((plan) => plan.analysis === requestedAnalysis);
    if (plans.length === 0) {
      throw invalidElement(
        "selectDeckAnalysisPlan",
        `no .${requestedAnalysis} analysis card found`,
      );
    }
    if (plans.length > 1) {
      throw invalidElement(
        "selectDeckAnalysisPlan",
        `multiple .${requestedAnalysis} analysis cards found`,
      );
    }
    return plans[0];
  }

  if (plans.length === 0) {
    return { directive: ".op", analysis: "op", lineNumber: 0, useInitialConditions: false };
  }
  if (plans.length > 1) {
    throw invalidElement(
      "selectDeckAnalysisPlan",
      "multiple analysis cards found; pass analysis to select one",
    );
  }
  return plans[0];
}

export function releaseReadinessGates(
  corpus: readonly CompatibilityDeck[] = COMPATIBILITY_CORPUS,
): ReleaseReadinessReport {
  const issues: ReleaseReadinessIssue[] = [];
  const seenIds = new Set<string>();
  const analyses: string[] = [];

  if (corpus.length === 0) {
    issues.push({
      deckId: "corpus",
      field: "deck_count",
      message: "compatibility corpus must contain at least one deck",
    });
  }

  for (const deck of corpus) {
    const deckId = deck.id || "<missing>";
    validateCompatibilityNonEmpty(deckId, "id", deck.id, issues);
    validateCompatibilityNonEmpty(deckId, "title", deck.title, issues);
    validateCompatibilityNonEmpty(deckId, "netlist", deck.netlist, issues);
    validateCompatibilityNonEmpty(deckId, "oracle.reference", deck.oracle.reference, issues);
    validateCompatibilityNonEmpty(deckId, "oracle.version", deck.oracle.version, issues);
    validateCompatibilityNonEmpty(deckId, "oracle.source", deck.oracle.source, issues);
    if (seenIds.has(deck.id)) {
      issues.push({ deckId, field: "id", message: "deck ids must be unique" });
    }
    seenIds.add(deck.id);
    if (!SUPPORTED_COMPATIBILITY_ANALYSES.has(deck.analysis)) {
      issues.push({
        deckId,
        field: "analysis",
        message: `unsupported analysis ${JSON.stringify(deck.analysis)}`,
      });
    } else if (!analyses.includes(deck.analysis)) {
      analyses.push(deck.analysis);
    }
    if (!deck.netlist.toLowerCase().includes(".end")) {
      issues.push({ deckId, field: "netlist", message: "deck must include .end" });
    }
    if (deck.goldenValues.length === 0) {
      issues.push({
        deckId,
        field: "goldenValues",
        message: "deck must include at least one golden value",
      });
    }
    deck.goldenValues.forEach((golden, index) => {
      const fieldPrefix = `goldenValues[${index}]`;
      validateCompatibilityNonEmpty(deckId, `${fieldPrefix}.name`, golden.name, issues);
      validateCompatibilityNonEmpty(deckId, `${fieldPrefix}.unit`, golden.unit, issues);
      if (!Number.isFinite(golden.value)) {
        issues.push({
          deckId,
          field: `${fieldPrefix}.value`,
          message: "golden value must be finite",
        });
      }
      if (
        !Number.isFinite(golden.absoluteTolerance) ||
        !Number.isFinite(golden.relativeTolerance) ||
        golden.absoluteTolerance < 0.0 ||
        golden.relativeTolerance < 0.0
      ) {
        issues.push({
          deckId,
          field: `${fieldPrefix}.tolerance`,
          message: "tolerances must be finite and non-negative",
        });
      }
      if (
        golden.absoluteTolerance === 0.0 &&
        golden.relativeTolerance === 0.0 &&
        golden.unit !== "count"
      ) {
        issues.push({
          deckId,
          field: `${fieldPrefix}.tolerance`,
          message: "non-count golden values need an absolute or relative tolerance",
        });
      }
    });
    if (deck.knownIncompatibilities.length === 0) {
      issues.push({
        deckId,
        field: "knownIncompatibilities",
        message: "deck must document known incompatibility boundaries",
      });
    }
  }

  for (const analysis of REQUIRED_COMPATIBILITY_ANALYSES) {
    if (!analyses.includes(analysis)) {
      issues.push({
        deckId: "corpus",
        field: "analysisCoverage",
        message: `missing required ${JSON.stringify(analysis)} compatibility deck`,
      });
    }
  }

  return {
    passed: issues.length === 0,
    deckCount: corpus.length,
    analyses,
    issues,
  };
}

export function formatCompatibilityCorpusTable(
  corpus: readonly CompatibilityDeck[] = COMPATIBILITY_CORPUS,
): string {
  const lines = ["id\tanalysis\toracle\tgolden_values\tknown_incompatibilities"];
  for (const deck of corpus) {
    const goldenValues = deck.goldenValues
      .map((entry) => `${entry.name}=${formatTableNumber(entry.value)}${entry.unit}`)
      .join(",");
    lines.push([
      deck.id,
      deck.analysis,
      `${deck.oracle.reference}@${deck.oracle.version}`,
      goldenValues,
      deck.knownIncompatibilities.length.toString(),
    ].join("\t"));
  }
  return lines.join("\n");
}

export function formatReleaseReadinessReport(report: ReleaseReadinessReport): string {
  const lines = [
    "passed\tdeck_count\tanalyses\tissue_count",
    `${String(report.passed)}\t${report.deckCount}\t${report.analyses.join(",")}\t${report.issues.length}`,
  ];
  if (report.issues.length > 0) {
    lines.push("deck_id\tfield\tmessage");
    for (const issue of report.issues) {
      lines.push(`${issue.deckId}\t${issue.field}\t${issue.message}`);
    }
  }
  return lines.join("\n");
}

interface DeckResolutionState {
  readonly diagnostics: DeckResolutionDiagnostic[];
  readonly includedPaths: string[];
  readonly librarySections: string[];
}

interface ResolvedDeckLines {
  readonly activeLines: string[];
  readonly terminated: boolean;
  readonly endLineNumber?: number;
}

function resolveDeckLines(
  netlist: string,
  source: string,
  sources: Readonly<Record<string, string>>,
  state: DeckResolutionState,
  stack: readonly string[],
): ResolvedDeckLines {
  const activeLines: string[] = [];
  let endLineNumber: number | undefined;
  let inControlBlock = false;

  const lines = netlist.split(/\r?\n/);
  for (let index = 0; index < lines.length; index++) {
    const lineNumber = index + 1;
    const stripped = lines[index].trim();
    if (stripped.length === 0 || stripped.startsWith("*") || stripped.startsWith(";")) {
      continue;
    }
    const directive = deckDirective(stripped);
    if (inControlBlock) {
      if (directive === ".endc") {
        inControlBlock = false;
        continue;
      }
      const controlLine = controlBlockCommandAsDeckLine(stripped);
      if (controlLine !== undefined) {
        activeLines.push(controlLine);
        continue;
      }
      if (isNoopControlBlockCommand(stripped)) {
        continue;
      }
      if (isScriptControlBlockCommand(stripped)) {
        state.diagnostics.push({
          code: "SPICE_DECK_CONTROL_SCRIPT_COMMAND",
          directive: ".control",
          source,
          lineNumber,
          message: controlBlockScriptPolicyMessage(stripped),
          severity: "error",
        });
        continue;
      }
      if (isWorkdirControlBlockCommand(stripped)) {
        state.diagnostics.push({
          code: "SPICE_DECK_CONTROL_WORKDIR_COMMAND",
          directive: ".control",
          source,
          lineNumber,
          message: controlBlockWorkdirPolicyMessage(stripped),
          severity: "error",
        });
        continue;
      }
      if (isControlFlowControlBlockCommand(stripped)) {
        state.diagnostics.push({
          code: "SPICE_DECK_CONTROL_FLOW_COMMAND",
          directive: ".control",
          source,
          lineNumber,
          message: controlBlockFlowPolicyMessage(stripped),
          severity: "error",
        });
        continue;
      }
      if (isVariableControlBlockCommand(stripped)) {
        state.diagnostics.push({
          code: "SPICE_DECK_CONTROL_VARIABLE_COMMAND",
          directive: ".control",
          source,
          lineNumber,
          message: controlBlockVariablePolicyMessage(stripped),
          severity: "error",
        });
        continue;
      }
      state.diagnostics.push({
        code: "SPICE_DECK_CONTROL_COMMAND",
        directive: ".control",
        source,
        lineNumber,
        message: `${JSON.stringify(stripped)} inside .control is not executed by the deck source resolver yet`,
        severity: "error",
      });
      continue;
    }
    if (directive === ".end") {
      endLineNumber = lineNumber;
      break;
    }
    if (directive === ".include") {
      activeLines.push(
        ...resolveIncludeDirective(stripped, source, lineNumber, sources, state, stack),
      );
      continue;
    }
    if (directive === ".lib") {
      activeLines.push(
        ...resolveLibraryDirective(stripped, source, lineNumber, sources, state, stack),
      );
      continue;
    }
    if (directive !== undefined && UNSUPPORTED_RESOLVED_DIRECTIVES.has(directive)) {
      state.diagnostics.push({
        code: "SPICE_DECK_UNSUPPORTED_DIRECTIVE",
        directive,
        source,
        lineNumber,
        message: `${directive} is not supported by the deck source resolver yet`,
        severity: "error",
      });
      if (directive === ".control") {
        inControlBlock = true;
        continue;
      }
    }
    activeLines.push(stripped);
  }

  return {
    activeLines,
    terminated: endLineNumber !== undefined,
    endLineNumber,
  };
}

function resolveIncludeDirective(
  line: string,
  source: string,
  lineNumber: number,
  sources: Readonly<Record<string, string>>,
  state: DeckResolutionState,
  stack: readonly string[],
): string[] {
  const tokens = directiveTokens(line);
  const target = tokens.length >= 2 ? unquoteToken(tokens[1]) : undefined;
  if (target === undefined || target.length === 0) {
    addResolutionDiagnostic(state, {
      code: "SPICE_DECK_INCLUDE_ARGUMENT",
      directive: ".include",
      source,
      lineNumber,
      message: ".include requires a source path",
    });
    return [];
  }
  if (stack.includes(target)) {
    addResolutionDiagnostic(state, {
      code: "SPICE_DECK_INCLUDE_CYCLE",
      directive: ".include",
      source,
      lineNumber,
      message: `.include cycle detected for ${target}`,
      target,
    });
    return [];
  }
  const content = sources[target];
  if (content === undefined) {
    addResolutionDiagnostic(state, {
      code: "SPICE_DECK_INCLUDE_NOT_FOUND",
      directive: ".include",
      source,
      lineNumber,
      message: `.include source ${JSON.stringify(target)} was not provided`,
      target,
    });
    return [];
  }

  state.includedPaths.push(target);
  return resolveDeckLines(content, target, sources, state, [...stack, target]).activeLines;
}

function resolveLibraryDirective(
  line: string,
  source: string,
  lineNumber: number,
  sources: Readonly<Record<string, string>>,
  state: DeckResolutionState,
  stack: readonly string[],
): string[] {
  const tokens = directiveTokens(line);
  const path = tokens.length >= 2 ? unquoteToken(tokens[1]) : undefined;
  const section = tokens.length >= 3 ? unquoteToken(tokens[2]) : undefined;
  if (path === undefined || path.length === 0 || section === undefined || section.length === 0) {
    addResolutionDiagnostic(state, {
      code: "SPICE_DECK_LIB_ARGUMENT",
      directive: ".lib",
      source,
      lineNumber,
      message: ".lib requires a source path and section name",
      target: path,
    });
    return [];
  }

  const target = `${path}:${section}`;
  const content = sources[path];
  if (content === undefined) {
    addResolutionDiagnostic(state, {
      code: "SPICE_DECK_LIB_NOT_FOUND",
      directive: ".lib",
      source,
      lineNumber,
      message: `.lib source ${JSON.stringify(path)} was not provided`,
      target,
    });
    return [];
  }
  if (stack.includes(target)) {
    addResolutionDiagnostic(state, {
      code: "SPICE_DECK_LIB_CYCLE",
      directive: ".lib",
      source,
      lineNumber,
      message: `.lib cycle detected for ${target}`,
      target,
    });
    return [];
  }

  const sectionLines = extractLibrarySection(content, path, section, source, lineNumber, state);
  if (sectionLines === undefined) {
    return [];
  }

  state.librarySections.push(target);
  return resolveDeckLines(sectionLines.join("\n"), target, sources, state, [...stack, target]).activeLines;
}

function extractLibrarySection(
  content: string,
  path: string,
  section: string,
  callSource: string,
  callLineNumber: number,
  state: DeckResolutionState,
): string[] | undefined {
  let inSection = false;
  let sectionStartLine: number | undefined;
  const sectionLines: string[] = [];
  const wanted = section.toLowerCase();
  const target = `${path}:${section}`;
  const lines = content.split(/\r?\n/);

  for (let index = 0; index < lines.length; index++) {
    const lineNumber = index + 1;
    const rawLine = lines[index];
    const stripped = rawLine.trim();
    if (stripped.length === 0 || stripped.startsWith("*") || stripped.startsWith(";")) {
      if (inSection) {
        sectionLines.push(rawLine);
      }
      continue;
    }
    const directive = deckDirective(stripped);
    const tokens = directiveTokens(stripped);
    if (!inSection) {
      if (
        directive === ".lib" &&
        tokens.length >= 2 &&
        unquoteToken(tokens[1]).toLowerCase() === wanted
      ) {
        inSection = true;
        sectionStartLine = lineNumber;
      }
      continue;
    }
    if (directive === ".endl" || directive === ".endlib") {
      return sectionLines;
    }
    sectionLines.push(rawLine);
  }

  if (!inSection) {
    addResolutionDiagnostic(state, {
      code: "SPICE_DECK_LIB_SECTION_NOT_FOUND",
      directive: ".lib",
      source: callSource,
      lineNumber: callLineNumber,
      message: `.lib section ${JSON.stringify(section)} was not found in ${JSON.stringify(path)}`,
      target,
    });
    return undefined;
  }

  addResolutionDiagnostic(state, {
    code: "SPICE_DECK_LIB_SECTION_UNTERMINATED",
    directive: ".lib",
    source: path,
    lineNumber: sectionStartLine ?? 1,
    message: `.lib section ${JSON.stringify(section)} in ${JSON.stringify(path)} is missing .endl`,
    target,
  });
  return undefined;
}

function addResolutionDiagnostic(
  state: DeckResolutionState,
  diagnostic: Omit<DeckResolutionDiagnostic, "severity"> & { readonly severity?: "error" | "warning" },
): void {
  state.diagnostics.push({
    ...diagnostic,
    severity: diagnostic.severity ?? "error",
  });
}

class DeckParameterState {
  readonly diagnostics: DeckParameterDiagnostic[] = [];
  private readonly parametersByName = new Map<string, DeckParameterValue>();
  private readonly functionsByName = new Map<string, DeckFunctionDefinition>();
  private readonly order: string[] = [];

  setParameter(name: string, value: number): void {
    const key = name.toLowerCase();
    if (!this.parametersByName.has(key)) {
      this.order.push(key);
    }
    this.parametersByName.set(key, { name, value });
  }

  getParameter(name: string): DeckParameterValue | undefined {
    return this.parametersByName.get(name.toLowerCase());
  }

  setFunction(definition: DeckFunctionDefinition): void {
    this.functionsByName.set(definition.name.toLowerCase(), definition);
  }

  getFunction(name: string): DeckFunctionDefinition | undefined {
    return this.functionsByName.get(name.toLowerCase());
  }

  parameterValues(): DeckParameterValue[] {
    return this.order.map((key) => this.parametersByName.get(key)!);
  }
}

class DeckInitialConditionState {
  readonly diagnostics: DeckInitialConditionDiagnostic[] = [];
  readonly initialConditions: DeckNodeCondition[] = [];
  readonly nodesets: DeckNodeCondition[] = [];
}

class DeckFunctionState {
  readonly diagnostics: DeckFunctionDiagnostic[] = [];
  readonly functions: DeckFunctionDefinition[] = [];
}

class DeckMeasurementState {
  readonly diagnostics: DeckMeasurementDiagnostic[] = [];
  readonly measurements: DeckMeasurementCard[] = [];
}

class DeckFourierState {
  readonly diagnostics: DeckFourierDiagnostic[] = [];
  readonly fourier: DeckFourierCard[] = [];
}

class DeckOutputState {
  readonly diagnostics: DeckOutputDiagnostic[] = [];
  readonly selections: DeckOutputSelection[] = [];
}

class DeckAnalysisState {
  readonly diagnostics: DeckAnalysisDiagnostic[] = [];
  readonly analyses: DeckAnalysisPlan[] = [];
}

function resolveNodeConditionLine(
  line: string,
  lineNumber: number,
  directive: ".ic" | ".nodeset",
  state: DeckInitialConditionState,
): void {
  const tokens = directiveTokens(line);
  if (tokens.length === 1) {
    addInitialConditionDiagnostic(state, {
      code: "SPICE_DECK_CONDITION_ARGUMENT",
      directive,
      lineNumber,
      message: `${directive} requires at least one V(node)=value assignment`,
    });
    return;
  }

  const emptyParameterState = new DeckParameterState();
  for (const token of tokens.slice(1)) {
    const equalsIndex = token.indexOf("=");
    if (equalsIndex < 0) {
      addInitialConditionDiagnostic(state, {
        code: "SPICE_DECK_CONDITION_ARGUMENT",
        directive,
        lineNumber,
        message: `${directive} assignment ${JSON.stringify(token)} must use V(node)=value syntax`,
        token,
      });
      continue;
    }
    const target = token.slice(0, equalsIndex).trim();
    const expression = stripExpressionDelimiters(token.slice(equalsIndex + 1).trim());
    const node = parseNodeConditionTarget(target);
    if (node === undefined) {
      addInitialConditionDiagnostic(state, {
        code: "SPICE_DECK_CONDITION_TARGET",
        directive,
        lineNumber,
        message: `${directive} target ${JSON.stringify(target)} must use V(node) syntax`,
        token,
      });
      continue;
    }
    try {
      const condition = {
        directive,
        node,
        value: evaluateParameterExpression(expression, emptyParameterState),
        lineNumber,
      };
      if (directive === ".ic") {
        state.initialConditions.push(condition);
      } else {
        state.nodesets.push(condition);
      }
    } catch (error) {
      addInitialConditionDiagnostic(state, {
        code: "SPICE_DECK_CONDITION_EXPRESSION",
        directive,
        lineNumber,
        message: error instanceof Error ? error.message : String(error),
        token,
      });
    }
  }
}

function resolveFunctionLine(line: string, lineNumber: number, state: DeckFunctionState): void {
  let restStart = 0;
  while (restStart < line.length && !isDirectiveWhitespace(line[restStart])) {
    restStart += 1;
  }
  while (restStart < line.length && isDirectiveWhitespace(line[restStart])) {
    restStart += 1;
  }
  const rest = line.slice(restStart).trim();
  if (rest.length === 0) {
    addFunctionDiagnostic(state, {
      code: "SPICE_DECK_FUNC_ARGUMENT",
      lineNumber,
      message: ".func requires a name(args) expression definition",
    });
    return;
  }

  const parsed = parseFunctionSignature(rest);
  if (parsed === undefined) {
    addFunctionDiagnostic(state, {
      code: "SPICE_DECK_FUNC_SIGNATURE",
      lineNumber,
      message: ".func definition must use name(args) expression syntax",
    });
    return;
  }
  const { name, argumentList, expression: rawExpression } = parsed;
  if (!isParameterName(name)) {
    addFunctionDiagnostic(state, {
      code: "SPICE_DECK_FUNC_SIGNATURE",
      lineNumber,
      message: `.func name ${JSON.stringify(name)} is not a valid identifier`,
      functionName: name,
    });
    return;
  }
  const invalidArgument = argumentList.find((argument) => !isParameterName(argument));
  if (invalidArgument !== undefined) {
    addFunctionDiagnostic(state, {
      code: "SPICE_DECK_FUNC_ARGUMENT",
      lineNumber,
      message: `.func argument ${JSON.stringify(invalidArgument)} is not a valid identifier`,
      functionName: name,
    });
    return;
  }
  if (new Set(argumentList.map((argument) => argument.toLowerCase())).size !== argumentList.length) {
    addFunctionDiagnostic(state, {
      code: "SPICE_DECK_FUNC_ARGUMENT",
      lineNumber,
      message: `.func ${JSON.stringify(name)} has duplicate argument names`,
      functionName: name,
    });
    return;
  }
  const expression = stripExpressionDelimiters(rawExpression.trim());
  if (expression.length === 0) {
    addFunctionDiagnostic(state, {
      code: "SPICE_DECK_FUNC_EXPRESSION",
      lineNumber,
      message: `.func ${JSON.stringify(name)} requires a non-empty expression`,
      functionName: name,
    });
    return;
  }
  state.functions.push({
    name,
    arguments: argumentList,
    expression,
    lineNumber,
  });
}

function resolveMeasurementLine(
  line: string,
  lineNumber: number,
  directive: ".measure" | ".meas",
  state: DeckMeasurementState,
): void {
  const tokens = directiveTokens(line);
  if (tokens.length < 5) {
    addMeasurementDiagnostic(state, {
      code: "SPICE_DECK_MEASURE_ARGUMENT",
      directive,
      lineNumber,
      message: `${directive} requires analysis, name, mode, and probe tokens`,
    });
    return;
  }

  const analysis = tokens[1].trim().toLowerCase();
  if (analysis !== "tran" && analysis !== "transient" && analysis !== "dc" && analysis !== "ac") {
    addMeasurementDiagnostic(state, {
      code: "SPICE_DECK_MEASURE_ANALYSIS",
      directive,
      lineNumber,
      message: `only transient, dc, and ac .measure cards are supported, got ${JSON.stringify(tokens[1])}`,
      token: tokens[1],
    });
    return;
  }
  const analysisName = analysis === "tran" || analysis === "dc" || analysis === "ac" ? analysis : "transient";

  const name = tokens[2].trim();
  if (!isParameterName(name)) {
    addMeasurementDiagnostic(state, {
      code: "SPICE_DECK_MEASURE_NAME",
      directive,
      lineNumber,
      message: `measurement name ${JSON.stringify(name)} is not a valid identifier`,
      token: name,
    });
    return;
  }

  if (tokens[3].trim().toLowerCase() === "trig") {
    resolveMeasurementDelayLine(tokens, lineNumber, directive, state, analysisName, name);
    return;
  }

  const mode = normalizeMeasurementModeToken(tokens[3]);
  if (mode === undefined) {
    addMeasurementDiagnostic(state, {
      code: "SPICE_DECK_MEASURE_MODE",
      directive,
      lineNumber,
      message: `unsupported measurement mode ${JSON.stringify(tokens[3])}`,
      token: tokens[3],
    });
    return;
  }

  const emptyParameterState = new DeckParameterState();
  let targetValue: number | undefined;
  const lineDiagnosticCount = state.diagnostics.length;
  const probe = mode === "when"
    ? (() => {
        const equalsIndex = tokens[4].indexOf("=");
        if (equalsIndex < 0) {
          addMeasurementDiagnostic(state, {
            code: "SPICE_DECK_MEASURE_ARGUMENT",
            directive,
            lineNumber,
            message: "WHEN measurements require probe=target syntax",
            token: tokens[4],
          });
          return "";
        }
        try {
          targetValue = evaluateParameterExpression(
            stripExpressionDelimiters(tokens[4].slice(equalsIndex + 1).trim()),
            emptyParameterState,
          );
        } catch (error) {
          addMeasurementDiagnostic(state, {
            code: "SPICE_DECK_MEASURE_EXPRESSION",
            directive,
            lineNumber,
            message: error instanceof Error ? error.message : String(error),
            token: tokens[4],
          });
          return "";
        }
        return unquoteToken(tokens[4].slice(0, equalsIndex).trim());
      })()
    : unquoteToken(tokens[4].trim());
  if (state.diagnostics.length !== lineDiagnosticCount) {
    return;
  }
  if (probe.length === 0) {
    addMeasurementDiagnostic(state, {
      code: "SPICE_DECK_MEASURE_PROBE",
      directive,
      lineNumber,
      message: "measurement probe must not be empty",
      token: tokens[4],
    });
    return;
  }

  let fromValue: number | undefined;
  let toValue: number | undefined;
  let atValue: number | undefined;
  let crossingKind: MeasurementCrossingKind | undefined;
  let crossingCount: number | undefined;
  const seenWindowTokens = new Set<string>();
  const diagnosticCount = state.diagnostics.length;
  for (const token of tokens.slice(5)) {
    const equalsIndex = token.indexOf("=");
    if (equalsIndex < 0) {
      addMeasurementDiagnostic(state, {
        code: "SPICE_DECK_MEASURE_ARGUMENT",
        directive,
        lineNumber,
        message: `measurement option ${JSON.stringify(token)} must use name=value syntax`,
        token,
      });
      continue;
    }
    const key = token.slice(0, equalsIndex).trim().toLowerCase();
    const expression = token.slice(equalsIndex + 1);
    if (key !== "from" && key !== "to" && key !== "at" && key !== "rise" && key !== "fall" &&
      key !== "cross") {
      addMeasurementDiagnostic(state, {
        code: "SPICE_DECK_MEASURE_ARGUMENT",
        directive,
        lineNumber,
        message: `unsupported measurement option ${JSON.stringify(key)}`,
        token,
      });
      continue;
    }
    if (seenWindowTokens.has(key)) {
      addMeasurementDiagnostic(state, {
        code: "SPICE_DECK_MEASURE_ARGUMENT",
        directive,
        lineNumber,
        message: `duplicate measurement option ${JSON.stringify(key)}`,
        token,
      });
      continue;
    }
    seenWindowTokens.add(key);
    if (key === "rise" || key === "fall" || key === "cross") {
      if (mode !== "when") {
        addMeasurementDiagnostic(state, {
          code: "SPICE_DECK_MEASURE_ARGUMENT",
          directive,
          lineNumber,
          message: "RISE, FALL, and CROSS options are only supported with WHEN mode",
          token,
        });
        continue;
      }
      if (crossingKind !== undefined) {
        addMeasurementDiagnostic(state, {
          code: "SPICE_DECK_MEASURE_ARGUMENT",
          directive,
          lineNumber,
          message: "only one of RISE, FALL, or CROSS may be specified",
          token,
        });
        continue;
      }
      try {
        const value = evaluateParameterExpression(
          stripExpressionDelimiters(expression.trim()),
          emptyParameterState,
        );
        if (!Number.isFinite(value) || value < 1 || !Number.isInteger(value)) {
          addMeasurementDiagnostic(state, {
            code: "SPICE_DECK_MEASURE_ARGUMENT",
            directive,
            lineNumber,
            message: "RISE, FALL, and CROSS counts must be positive integers",
            token,
          });
          continue;
        }
        crossingKind = key;
        crossingCount = value;
      } catch (error) {
        addMeasurementDiagnostic(state, {
          code: "SPICE_DECK_MEASURE_EXPRESSION",
          directive,
          lineNumber,
          message: error instanceof Error ? error.message : String(error),
          token,
        });
      }
      continue;
    }
    try {
      const value = evaluateParameterExpression(
        stripExpressionDelimiters(expression.trim()),
        emptyParameterState,
      );
      if (key === "from") {
        fromValue = value;
      } else if (key === "to") {
        toValue = value;
      } else {
        atValue = value;
      }
    } catch (error) {
      addMeasurementDiagnostic(state, {
        code: "SPICE_DECK_MEASURE_EXPRESSION",
        directive,
        lineNumber,
        message: error instanceof Error ? error.message : String(error),
        token,
      });
    }
  }

  if (mode === "find" && atValue === undefined) {
    addMeasurementDiagnostic(state, {
      code: "SPICE_DECK_MEASURE_ARGUMENT",
      directive,
      lineNumber,
      message: "FIND measurements require an AT value",
    });
  }
  if (mode === "when" && targetValue === undefined) {
    addMeasurementDiagnostic(state, {
      code: "SPICE_DECK_MEASURE_ARGUMENT",
      directive,
      lineNumber,
      message: "WHEN measurements require a target value",
    });
  }
  if (mode !== "find" && atValue !== undefined) {
    addMeasurementDiagnostic(state, {
      code: "SPICE_DECK_MEASURE_ARGUMENT",
      directive,
      lineNumber,
      message: "measurement AT value is only supported with FIND mode",
    });
  }
  if (atValue !== undefined && (fromValue !== undefined || toValue !== undefined)) {
    addMeasurementDiagnostic(state, {
      code: "SPICE_DECK_MEASURE_ARGUMENT",
      directive,
      lineNumber,
      message: "measurement AT value cannot be combined with FROM or TO",
    });
  }

  if (fromValue !== undefined && toValue !== undefined && fromValue > toValue) {
    addMeasurementDiagnostic(state, {
      code: "SPICE_DECK_MEASURE_WINDOW",
      directive,
      lineNumber,
      message: "measurement FROM value must be <= TO value",
    });
  }

  if (state.diagnostics.length !== diagnosticCount) {
    return;
  }

  state.measurements.push({
    directive,
    analysis: analysisName,
    name,
    mode,
    probe,
    lineNumber,
    fromValue,
    toValue,
    atValue,
    targetValue,
    crossingKind,
    crossingCount,
    triggerProbe: undefined,
    triggerValue: undefined,
    triggerCrossingKind: undefined,
    triggerCrossingCount: undefined,
  });
}

function resolveFourierLine(
  line: string,
  lineNumber: number,
  state: DeckFourierState,
): void {
  const tokens = directiveTokens(line);
  if (tokens.length < 3) {
    addFourierDiagnostic(state, {
      code: "SPICE_DECK_FOURIER_ARGUMENT",
      lineNumber,
      message: ".four requires a fundamental frequency and at least one probe",
    });
    return;
  }

  const emptyParameterState = new DeckParameterState();
  let fundamentalFrequencyHz: number;
  try {
    fundamentalFrequencyHz = evaluateParameterExpression(
      stripExpressionDelimiters(tokens[1].trim()),
      emptyParameterState,
    );
  } catch (error) {
    addFourierDiagnostic(state, {
      code: "SPICE_DECK_FOURIER_EXPRESSION",
      lineNumber,
      message: error instanceof Error ? error.message : String(error),
      token: tokens[1],
    });
    return;
  }
  if (!Number.isFinite(fundamentalFrequencyHz) || fundamentalFrequencyHz <= 0.0) {
    addFourierDiagnostic(state, {
      code: "SPICE_DECK_FOURIER_FREQUENCY",
      lineNumber,
      message: ".four fundamental frequency must be finite and positive",
      token: tokens[1],
    });
    return;
  }

  const probes: string[] = [];
  let harmonics: number | undefined;
  let fromValue: number | undefined;
  const seenOptions = new Set<string>();
  const diagnosticCount = state.diagnostics.length;
  for (const token of tokens.slice(2)) {
    const equalsIndex = token.indexOf("=");
    if (equalsIndex >= 0) {
      const key = token.slice(0, equalsIndex).trim().toLowerCase();
      const expression = token.slice(equalsIndex + 1);
      if (key !== "harmonics" && key !== "from") {
        addFourierDiagnostic(state, {
          code: "SPICE_DECK_FOURIER_ARGUMENT",
          lineNumber,
          message: `unsupported .four option ${JSON.stringify(key)}`,
          token,
        });
        continue;
      }
      if (seenOptions.has(key)) {
        addFourierDiagnostic(state, {
          code: "SPICE_DECK_FOURIER_ARGUMENT",
          lineNumber,
          message: `duplicate .four option ${JSON.stringify(key)}`,
          token,
        });
        continue;
      }
      seenOptions.add(key);
      try {
        const value = evaluateParameterExpression(
          stripExpressionDelimiters(expression.trim()),
          emptyParameterState,
        );
        if (key === "harmonics") {
          if (!Number.isFinite(value) || value < 1 || !Number.isInteger(value)) {
            addFourierDiagnostic(state, {
              code: "SPICE_DECK_FOURIER_ARGUMENT",
              lineNumber,
              message: ".four HARMONICS value must be a positive integer",
              token,
            });
            continue;
          }
          harmonics = value;
        } else {
          fromValue = value;
        }
      } catch (error) {
        addFourierDiagnostic(state, {
          code: "SPICE_DECK_FOURIER_EXPRESSION",
          lineNumber,
          message: error instanceof Error ? error.message : String(error),
          token,
        });
      }
      continue;
    }
    const probe = unquoteToken(token.trim());
    if (probe.length === 0) {
      addFourierDiagnostic(state, {
        code: "SPICE_DECK_FOURIER_PROBE",
        lineNumber,
        message: ".four probe must not be empty",
        token,
      });
      continue;
    }
    probes.push(probe);
  }

  if (probes.length === 0 && state.diagnostics.length === diagnosticCount) {
    addFourierDiagnostic(state, {
      code: "SPICE_DECK_FOURIER_PROBE",
      lineNumber,
      message: ".four requires at least one probe",
    });
  }
  if (fromValue !== undefined && !Number.isFinite(fromValue)) {
    addFourierDiagnostic(state, {
      code: "SPICE_DECK_FOURIER_WINDOW",
      lineNumber,
      message: ".four FROM value must be finite",
    });
  }

  if (state.diagnostics.length !== diagnosticCount) {
    return;
  }

  state.fourier.push({
    directive: ".four",
    fundamentalFrequencyHz,
    probes,
    lineNumber,
    harmonics,
    fromValue,
  });
}

function resolveOutputLine(
  line: string,
  lineNumber: number,
  directive: ".save" | ".probe" | ".print" | ".plot",
  state: DeckOutputState,
): void {
  const tokens = directiveTokens(line);
  if (tokens.length < 2) {
    const message = directive === ".print" || directive === ".plot"
      ? `${directive} requires an analysis token and at least one probe token`
      : `${directive} requires at least one probe token`;
    addOutputDiagnostic(state, {
      code: "SPICE_DECK_OUTPUT_ARGUMENT",
      directive,
      lineNumber,
      message,
    });
    return;
  }

  let analysis: DeckOutputSelection["analysis"];
  let probeTokens = tokens.slice(1);
  if (directive === ".print" || directive === ".plot") {
    if (tokens.length < 3) {
      addOutputDiagnostic(state, {
        code: "SPICE_DECK_OUTPUT_ARGUMENT",
        directive,
        lineNumber,
        message: `${directive} requires an analysis token and at least one probe token`,
      });
      return;
    }
    const normalizedAnalysis = normalizeDeckOutputAnalysis(tokens[1]);
    if (normalizedAnalysis === undefined) {
      addOutputDiagnostic(state, {
        code: "SPICE_DECK_OUTPUT_ANALYSIS",
        directive,
        lineNumber,
        message: `${directive} analysis must be op, dc, ac, or tran, got ${JSON.stringify(tokens[1])}`,
        token: tokens[1],
      });
      return;
    }
    analysis = normalizedAnalysis;
    probeTokens = tokens.slice(2);
  } else if (directive === ".probe") {
    const normalizedAnalysis = normalizeDeckOutputAnalysis(tokens[1]);
    if (normalizedAnalysis !== undefined) {
      analysis = normalizedAnalysis;
      probeTokens = tokens.slice(2);
    }
  }
  if (probeTokens.length === 0) {
    addOutputDiagnostic(state, {
      code: "SPICE_DECK_OUTPUT_ARGUMENT",
      directive,
      lineNumber,
      message: `${directive} requires at least one probe token`,
    });
    return;
  }

  const probes: string[] = [];
  for (const token of probeTokens) {
    const text = unquoteToken(token);
    const probe = normalizeDeckOutputProbe(text);
    if (probe === undefined) {
      addOutputDiagnostic(state, {
        code: "SPICE_DECK_OUTPUT_PROBE",
        directive,
        lineNumber,
        message: `${directive} probe must be V(node) or I(source), got ${JSON.stringify(text)}`,
        token: text,
      });
      continue;
    }
    probes.push(probe);
  }
  if (probes.length === 0) {
    return;
  }

  state.selections.push({
    directive,
    analysis,
    probes,
    lineNumber,
  });
}

function resolveAnalysisLine(
  line: string,
  lineNumber: number,
  directive: DeckAnalysisDiagnostic["directive"],
  state: DeckAnalysisState,
): void {
  const tokens = directiveTokens(line);
  switch (directive) {
    case ".op":
      resolveOpAnalysis(tokens, lineNumber, state);
      break;
    case ".dc":
      resolveDcAnalysis(tokens, lineNumber, state);
      break;
    case ".ac":
      resolveAcAnalysis(tokens, lineNumber, state);
      break;
    case ".tran":
      resolveTranAnalysis(tokens, lineNumber, state);
      break;
    case ".tf":
      resolveTfAnalysis(tokens, lineNumber, state);
      break;
    case ".sens":
      resolveSensAnalysis(tokens, lineNumber, state);
      break;
    case ".noise":
      resolveNoiseAnalysis(tokens, lineNumber, state);
      break;
  }
}

function resolveOpAnalysis(
  tokens: readonly string[],
  lineNumber: number,
  state: DeckAnalysisState,
): void {
  if (tokens.length !== 1) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_ARGUMENT",
      directive: ".op",
      lineNumber,
      message: ".op does not accept analysis arguments",
      token: tokens[1],
    });
    return;
  }
  state.analyses.push({ directive: ".op", analysis: "op", lineNumber, useInitialConditions: false });
}

function resolveDcAnalysis(
  tokens: readonly string[],
  lineNumber: number,
  state: DeckAnalysisState,
): void {
  if (tokens.length !== 5) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_ARGUMENT",
      directive: ".dc",
      lineNumber,
      message: ".dc requires source, start, stop, and step tokens",
    });
    return;
  }
  const sourceName = unquoteToken(tokens[1]).trim();
  if (sourceName.length === 0) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_ARGUMENT",
      directive: ".dc",
      lineNumber,
      message: ".dc source name must not be empty",
      token: tokens[1],
    });
    return;
  }
  const startValue = parseDeckAnalysisValue(tokens[2], ".dc", lineNumber, state);
  const stopValue = parseDeckAnalysisValue(tokens[3], ".dc", lineNumber, state);
  const stepValue = parseDeckAnalysisValue(tokens[4], ".dc", lineNumber, state);
  if (startValue === undefined || stopValue === undefined || stepValue === undefined) {
    return;
  }
  if (stepValue === 0.0) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_SWEEP",
      directive: ".dc",
      lineNumber,
      message: ".dc step value must be non-zero",
      token: tokens[4],
    });
    return;
  }
  if ((startValue < stopValue && stepValue < 0.0) || (startValue > stopValue && stepValue > 0.0)) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_SWEEP",
      directive: ".dc",
      lineNumber,
      message: ".dc step direction must move from start toward stop",
      token: tokens[4],
    });
    return;
  }
  state.analyses.push({
    directive: ".dc",
    analysis: "dc",
    lineNumber,
    sourceName,
    startValue,
    stopValue,
    stepValue,
    useInitialConditions: false,
  });
}

function resolveAcAnalysis(
  tokens: readonly string[],
  lineNumber: number,
  state: DeckAnalysisState,
): void {
  if (tokens.length !== 5) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_ARGUMENT",
      directive: ".ac",
      lineNumber,
      message: ".ac requires sweep kind, point count, start frequency, and stop frequency",
    });
    return;
  }
  const sweepKind = normalizeAcSweepKind(tokens[1]);
  if (sweepKind === undefined) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_MODE",
      directive: ".ac",
      lineNumber,
      message: `.ac sweep kind must be LIN, DEC, or OCT, got ${JSON.stringify(tokens[1])}`,
      token: tokens[1],
    });
    return;
  }
  const pointCount = parseDeckAnalysisInteger(tokens[2], ".ac", lineNumber, state);
  const startFrequencyHz = parseDeckAnalysisValue(tokens[3], ".ac", lineNumber, state);
  const stopFrequencyHz = parseDeckAnalysisValue(tokens[4], ".ac", lineNumber, state);
  if (pointCount === undefined || startFrequencyHz === undefined || stopFrequencyHz === undefined) {
    return;
  }
  if (pointCount < 1) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_SWEEP",
      directive: ".ac",
      lineNumber,
      message: ".ac point count must be a positive integer",
      token: tokens[2],
    });
    return;
  }
  if (startFrequencyHz <= 0.0 || stopFrequencyHz <= 0.0 || stopFrequencyHz < startFrequencyHz) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_SWEEP",
      directive: ".ac",
      lineNumber,
      message: ".ac frequencies must be positive and stop must be >= start",
    });
    return;
  }
  state.analyses.push({
    directive: ".ac",
    analysis: "ac",
    lineNumber,
    sweepKind,
    pointCount,
    startFrequencyHz,
    stopFrequencyHz,
    useInitialConditions: false,
  });
}

function resolveTranAnalysis(
  tokens: readonly string[],
  lineNumber: number,
  state: DeckAnalysisState,
): void {
  if (tokens.length < 3) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_ARGUMENT",
      directive: ".tran",
      lineNumber,
      message: ".tran requires step time and stop time",
    });
    return;
  }
  let useInitialConditions = false;
  const numericTokens: string[] = [];
  for (const token of tokens.slice(3)) {
    if (token.trim().toLowerCase() === "uic") {
      useInitialConditions = true;
      continue;
    }
    numericTokens.push(token);
  }
  if (numericTokens.length > 2) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_ARGUMENT",
      directive: ".tran",
      lineNumber,
      message: ".tran supports optional start time, max step, and UIC only",
      token: numericTokens[2],
    });
    return;
  }
  const stepTime = parseDeckAnalysisValue(tokens[1], ".tran", lineNumber, state);
  const stopTime = parseDeckAnalysisValue(tokens[2], ".tran", lineNumber, state);
  const startTime = numericTokens.length >= 1
    ? parseDeckAnalysisValue(numericTokens[0], ".tran", lineNumber, state)
    : undefined;
  const maxStep = numericTokens.length >= 2
    ? parseDeckAnalysisValue(numericTokens[1], ".tran", lineNumber, state)
    : undefined;
  if (stepTime === undefined || stopTime === undefined) {
    return;
  }
  if ((numericTokens.length >= 1 && startTime === undefined) || (numericTokens.length >= 2 && maxStep === undefined)) {
    return;
  }
  if (stepTime <= 0.0 || stopTime <= 0.0) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_INTERVAL",
      directive: ".tran",
      lineNumber,
      message: ".tran step time and stop time must be positive",
    });
    return;
  }
  if (startTime !== undefined && (startTime < 0.0 || startTime > stopTime)) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_INTERVAL",
      directive: ".tran",
      lineNumber,
      message: ".tran start time must be non-negative and <= stop time",
    });
    return;
  }
  if (maxStep !== undefined && maxStep <= 0.0) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_INTERVAL",
      directive: ".tran",
      lineNumber,
      message: ".tran max step must be positive",
    });
    return;
  }
  state.analyses.push({
    directive: ".tran",
    analysis: "tran",
    lineNumber,
    stepTime,
    stopTime,
    startTime,
    maxStep,
    useInitialConditions,
  });
}

function resolveTfAnalysis(
  tokens: readonly string[],
  lineNumber: number,
  state: DeckAnalysisState,
): void {
  if (tokens.length !== 3) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_ARGUMENT",
      directive: ".tf",
      lineNumber,
      message: ".tf requires output voltage probe and input source tokens",
    });
    return;
  }
  const outputProbe = normalizeDeckOutputProbe(unquoteToken(tokens[1]));
  if (outputProbe === undefined || !outputProbe.startsWith("V(")) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_ARGUMENT",
      directive: ".tf",
      lineNumber,
      message: `.tf output must be a voltage probe V(node), got ${JSON.stringify(tokens[1])}`,
      token: tokens[1],
    });
    return;
  }
  const inputSource = unquoteToken(tokens[2]).trim();
  if (inputSource.length === 0) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_ARGUMENT",
      directive: ".tf",
      lineNumber,
      message: ".tf input source name must not be empty",
      token: tokens[2],
    });
    return;
  }
  state.analyses.push({
    directive: ".tf",
    analysis: "tf",
    lineNumber,
    sourceName: inputSource,
    outputNode: outputProbe.slice(2, -1),
    useInitialConditions: false,
  });
}

function resolveSensAnalysis(
  tokens: readonly string[],
  lineNumber: number,
  state: DeckAnalysisState,
): void {
  if (tokens.length !== 2) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_ARGUMENT",
      directive: ".sens",
      lineNumber,
      message: ".sens requires one output voltage probe token",
    });
    return;
  }
  const outputProbe = normalizeDeckOutputProbe(unquoteToken(tokens[1]));
  if (outputProbe === undefined || !outputProbe.startsWith("V(")) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_ARGUMENT",
      directive: ".sens",
      lineNumber,
      message: `.sens output must be a voltage probe V(node), got ${JSON.stringify(tokens[1])}`,
      token: tokens[1],
    });
    return;
  }
  state.analyses.push({
    directive: ".sens",
    analysis: "sens",
    lineNumber,
    outputNode: outputProbe.slice(2, -1),
    useInitialConditions: false,
  });
}

function resolveNoiseAnalysis(
  tokens: readonly string[],
  lineNumber: number,
  state: DeckAnalysisState,
): void {
  if (tokens.length !== 3 && tokens.length !== 7) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_ARGUMENT",
      directive: ".noise",
      lineNumber,
      message: ".noise requires output voltage probe, input source, and optional sweep kind, point count, start frequency, and stop frequency tokens",
    });
    return;
  }
  const outputProbe = normalizeDeckOutputProbe(unquoteToken(tokens[1]));
  if (outputProbe === undefined || !outputProbe.startsWith("V(")) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_ARGUMENT",
      directive: ".noise",
      lineNumber,
      message: `.noise output must be a voltage probe V(node), got ${JSON.stringify(tokens[1])}`,
      token: tokens[1],
    });
    return;
  }
  const inputSource = unquoteToken(tokens[2]).trim();
  if (inputSource.length === 0) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_ARGUMENT",
      directive: ".noise",
      lineNumber,
      message: ".noise input source name must not be empty",
      token: tokens[2],
    });
    return;
  }

  let sweepKind: DeckAnalysisPlan["sweepKind"];
  let pointCount: number | undefined;
  let startFrequencyHz: number | undefined;
  let stopFrequencyHz: number | undefined;
  if (tokens.length === 7) {
    sweepKind = normalizeAcSweepKind(tokens[3]);
    if (sweepKind === undefined) {
      addAnalysisDiagnostic(state, {
        code: "SPICE_DECK_ANALYSIS_MODE",
        directive: ".noise",
        lineNumber,
        message: `.noise sweep kind must be LIN, DEC, or OCT, got ${JSON.stringify(tokens[3])}`,
        token: tokens[3],
      });
      return;
    }
    pointCount = parseDeckAnalysisInteger(tokens[4], ".noise", lineNumber, state);
    startFrequencyHz = parseDeckAnalysisValue(tokens[5], ".noise", lineNumber, state);
    stopFrequencyHz = parseDeckAnalysisValue(tokens[6], ".noise", lineNumber, state);
    if (
      pointCount === undefined ||
      startFrequencyHz === undefined ||
      stopFrequencyHz === undefined
    ) {
      return;
    }
    if (pointCount < 1) {
      addAnalysisDiagnostic(state, {
        code: "SPICE_DECK_ANALYSIS_SWEEP",
        directive: ".noise",
        lineNumber,
        message: ".noise point count must be a positive integer",
        token: tokens[4],
      });
      return;
    }
    if (startFrequencyHz <= 0.0 || stopFrequencyHz <= 0.0 || stopFrequencyHz < startFrequencyHz) {
      addAnalysisDiagnostic(state, {
        code: "SPICE_DECK_ANALYSIS_SWEEP",
        directive: ".noise",
        lineNumber,
        message: ".noise frequencies must be positive and stop must be >= start",
      });
      return;
    }
  }

  state.analyses.push({
    directive: ".noise",
    analysis: "noise",
    lineNumber,
    sourceName: inputSource,
    outputNode: outputProbe.slice(2, -1),
    sweepKind,
    pointCount,
    startFrequencyHz,
    stopFrequencyHz,
    useInitialConditions: false,
  });
}

interface ParsedMeasurementEdge {
  readonly probe: string;
  readonly value: number;
  readonly crossingKind?: MeasurementCrossingKind;
  readonly crossingCount?: number;
}

function resolveMeasurementDelayLine(
  tokens: readonly string[],
  lineNumber: number,
  directive: ".measure" | ".meas",
  state: DeckMeasurementState,
  analysis: "tran" | "transient" | "dc" | "ac",
  name: string,
): void {
  if (analysis !== "tran" && analysis !== "transient") {
    addMeasurementDiagnostic(state, {
      code: "SPICE_DECK_MEASURE_ARGUMENT",
      directive,
      lineNumber,
      message: "TRIG/TARG measurements are only supported for transient analysis",
      token: tokens[3],
    });
    return;
  }
  const targetIndex = tokens.findIndex((token, index) =>
    index >= 4 && token.trim().toLowerCase() === "targ"
  );
  if (targetIndex < 0) {
    addMeasurementDiagnostic(state, {
      code: "SPICE_DECK_MEASURE_ARGUMENT",
      directive,
      lineNumber,
      message: "TRIG measurements require a TARG section",
    });
    return;
  }
  const emptyParameterState = new DeckParameterState();
  const trigger = parseMeasurementDelayEdge(
    tokens.slice(4, targetIndex),
    "TRIG",
    directive,
    lineNumber,
    state,
    emptyParameterState,
  );
  if (trigger === undefined) {
    return;
  }
  const targetResult = parseMeasurementDelayTargetSection(
    tokens.slice(targetIndex + 1),
    directive,
    lineNumber,
    state,
    emptyParameterState,
  );
  if (targetResult === undefined) {
    return;
  }
  const [target, fromValue, toValue] = targetResult;
  if (fromValue !== undefined && toValue !== undefined && fromValue > toValue) {
    addMeasurementDiagnostic(state, {
      code: "SPICE_DECK_MEASURE_WINDOW",
      directive,
      lineNumber,
      message: "measurement FROM value must be <= TO value",
    });
    return;
  }
  state.measurements.push({
    directive,
    analysis,
    name,
    mode: "delay",
    probe: target.probe,
    lineNumber,
    fromValue,
    toValue,
    atValue: undefined,
    targetValue: target.value,
    crossingKind: target.crossingKind,
    crossingCount: target.crossingCount,
    triggerProbe: trigger.probe,
    triggerValue: trigger.value,
    triggerCrossingKind: trigger.crossingKind,
    triggerCrossingCount: trigger.crossingCount,
  });
}

function parseMeasurementDelayTargetSection(
  tokens: readonly string[],
  directive: ".measure" | ".meas",
  lineNumber: number,
  state: DeckMeasurementState,
  parameterState: DeckParameterState,
): readonly [ParsedMeasurementEdge, number | undefined, number | undefined] | undefined {
  const edgeTokens: string[] = [];
  let fromValue: number | undefined;
  let toValue: number | undefined;
  const seenWindowTokens = new Set<string>();
  for (const token of tokens) {
    const equalsIndex = token.indexOf("=");
    if (equalsIndex < 0) {
      edgeTokens.push(token);
      continue;
    }
    const key = token.slice(0, equalsIndex).trim().toLowerCase();
    if (key !== "from" && key !== "to") {
      edgeTokens.push(token);
      continue;
    }
    if (seenWindowTokens.has(key)) {
      addMeasurementDiagnostic(state, {
        code: "SPICE_DECK_MEASURE_ARGUMENT",
        directive,
        lineNumber,
        message: `duplicate measurement option ${JSON.stringify(key)}`,
        token,
      });
      return undefined;
    }
    seenWindowTokens.add(key);
    try {
      const value = evaluateParameterExpression(
        stripExpressionDelimiters(token.slice(equalsIndex + 1).trim()),
        parameterState,
      );
      if (key === "from") {
        fromValue = value;
      } else {
        toValue = value;
      }
    } catch (error) {
      addMeasurementDiagnostic(state, {
        code: "SPICE_DECK_MEASURE_EXPRESSION",
        directive,
        lineNumber,
        message: error instanceof Error ? error.message : String(error),
        token,
      });
      return undefined;
    }
  }
  const edge = parseMeasurementDelayEdge(
    edgeTokens,
    "TARG",
    directive,
    lineNumber,
    state,
    parameterState,
  );
  return edge === undefined ? undefined : [edge, fromValue, toValue];
}

function parseMeasurementDelayEdge(
  tokens: readonly string[],
  section: "TRIG" | "TARG",
  directive: ".measure" | ".meas",
  lineNumber: number,
  state: DeckMeasurementState,
  parameterState: DeckParameterState,
): ParsedMeasurementEdge | undefined {
  const first = tokens[0];
  if (first === undefined) {
    addMeasurementDiagnostic(state, {
      code: "SPICE_DECK_MEASURE_ARGUMENT",
      directive,
      lineNumber,
      message: `${section} measurements require a probe target`,
    });
    return undefined;
  }
  let value: number | undefined;
  const equalsIndex = first.indexOf("=");
  let probeExpressionFailed = false;
  const probe = equalsIndex >= 0
    ? (() => {
        try {
          value = evaluateParameterExpression(
            stripExpressionDelimiters(first.slice(equalsIndex + 1).trim()),
            parameterState,
          );
        } catch (error) {
          addMeasurementDiagnostic(state, {
            code: "SPICE_DECK_MEASURE_EXPRESSION",
            directive,
            lineNumber,
            message: error instanceof Error ? error.message : String(error),
            token: first,
          });
          probeExpressionFailed = true;
        }
        return unquoteToken(first.slice(0, equalsIndex).trim());
      })()
    : unquoteToken(first.trim());
  if (probeExpressionFailed) {
    return undefined;
  }
  if (probe.length === 0) {
    addMeasurementDiagnostic(state, {
      code: "SPICE_DECK_MEASURE_PROBE",
      directive,
      lineNumber,
      message: `${section} measurement probe must not be empty`,
      token: first,
    });
    return undefined;
  }

  let crossingKind: MeasurementCrossingKind | undefined;
  let crossingCount: number | undefined;
  const seenTokens = new Set<string>();
  for (const token of tokens.slice(1)) {
    const tokenEqualsIndex = token.indexOf("=");
    if (tokenEqualsIndex < 0) {
      addMeasurementDiagnostic(state, {
        code: "SPICE_DECK_MEASURE_ARGUMENT",
        directive,
        lineNumber,
        message: `${section} measurement option ${JSON.stringify(token)} must use name=value syntax`,
        token,
      });
      return undefined;
    }
    const key = token.slice(0, tokenEqualsIndex).trim().toLowerCase();
    if (key !== "val" && key !== "rise" && key !== "fall" && key !== "cross") {
      addMeasurementDiagnostic(state, {
        code: "SPICE_DECK_MEASURE_ARGUMENT",
        directive,
        lineNumber,
        message: `unsupported ${section} measurement option ${JSON.stringify(key)}`,
        token,
      });
      return undefined;
    }
    if (seenTokens.has(key)) {
      addMeasurementDiagnostic(state, {
        code: "SPICE_DECK_MEASURE_ARGUMENT",
        directive,
        lineNumber,
        message: `duplicate ${section} measurement option ${JSON.stringify(key)}`,
        token,
      });
      return undefined;
    }
    seenTokens.add(key);
    let parsed: number;
    try {
      parsed = evaluateParameterExpression(
        stripExpressionDelimiters(token.slice(tokenEqualsIndex + 1).trim()),
        parameterState,
      );
    } catch (error) {
      addMeasurementDiagnostic(state, {
        code: "SPICE_DECK_MEASURE_EXPRESSION",
        directive,
        lineNumber,
        message: error instanceof Error ? error.message : String(error),
        token,
      });
      return undefined;
    }
    if (key === "val") {
      value = parsed;
    } else {
      if (crossingKind !== undefined) {
        addMeasurementDiagnostic(state, {
          code: "SPICE_DECK_MEASURE_ARGUMENT",
          directive,
          lineNumber,
          message: `only one ${section} RISE, FALL, or CROSS option may be specified`,
          token,
        });
        return undefined;
      }
      if (!Number.isFinite(parsed) || parsed < 1 || !Number.isInteger(parsed)) {
        addMeasurementDiagnostic(state, {
          code: "SPICE_DECK_MEASURE_ARGUMENT",
          directive,
          lineNumber,
          message: `${section} RISE, FALL, and CROSS counts must be positive integers`,
          token,
        });
        return undefined;
      }
      crossingKind = key;
      crossingCount = parsed;
    }
  }
  if (value === undefined) {
    addMeasurementDiagnostic(state, {
      code: "SPICE_DECK_MEASURE_ARGUMENT",
      directive,
      lineNumber,
      message: `${section} measurements require a VAL value or probe=value target`,
    });
    return undefined;
  }
  return { probe, value, crossingKind, crossingCount };
}

function resolveParamLine(line: string, lineNumber: number, state: DeckParameterState): void {
  const tokens = directiveTokens(line);
  if (tokens.length === 1) {
    addParameterDiagnostic(state, {
      code: "SPICE_DECK_PARAM_ARGUMENT",
      directive: ".param",
      lineNumber,
      message: ".param requires at least one name=value assignment",
    });
    return;
  }

  for (const token of tokens.slice(1)) {
    const equalsIndex = token.indexOf("=");
    if (equalsIndex < 0) {
      addParameterDiagnostic(state, {
        code: "SPICE_DECK_PARAM_ARGUMENT",
        directive: ".param",
        lineNumber,
        message: `.param assignment ${JSON.stringify(token)} must use name=value syntax`,
        parameter: token,
      });
      continue;
    }
    const name = token.slice(0, equalsIndex).trim();
    const expression = stripExpressionDelimiters(token.slice(equalsIndex + 1).trim());
    if (!isParameterName(name)) {
      addParameterDiagnostic(state, {
        code: "SPICE_DECK_PARAM_NAME",
        directive: ".param",
        lineNumber,
        message: `.param name ${JSON.stringify(name)} is not a valid identifier`,
        parameter: name,
        expression,
      });
      continue;
    }
    try {
      const value = evaluateParameterExpression(expression, state);
      state.setParameter(name, value);
    } catch (error) {
      addParameterDiagnostic(state, {
        code: "SPICE_DECK_PARAM_EXPRESSION",
        directive: ".param",
        lineNumber,
        message: error instanceof Error ? error.message : String(error),
        parameter: name,
        expression,
      });
    }
  }
}

function collectParameterFunctions(netlist: string, state: DeckParameterState): void {
  const functionState = new DeckFunctionState();
  const lines = netlist.split(/\r?\n/);
  for (let index = 0; index < lines.length; index++) {
    const lineNumber = index + 1;
    const stripped = lines[index].trim();
    if (stripped.length === 0 || stripped.startsWith("*") || stripped.startsWith(";")) {
      continue;
    }
    const directive = deckDirective(stripped);
    if (directive === ".end") {
      break;
    }
    if (directive === ".func") {
      resolveFunctionLine(stripped, lineNumber, functionState);
    }
  }

  for (const definition of functionState.functions) {
    state.setFunction(definition);
  }
  for (const diagnostic of functionState.diagnostics) {
    addParameterDiagnostic(state, {
      code: diagnostic.code,
      directive: diagnostic.directive,
      lineNumber: diagnostic.lineNumber,
      message: diagnostic.message,
      parameter: diagnostic.functionName,
      expression: diagnostic.expression,
    });
  }
}

function rewriteParameterExpressions(
  line: string,
  lineNumber: number,
  state: DeckParameterState,
): string {
  const braced = replaceDelimitedParameterExpressions(line, "{", "}", lineNumber, state);
  return replaceDelimitedParameterExpressions(braced, "'", "'", lineNumber, state);
}

function replaceDelimitedParameterExpressions(
  line: string,
  openToken: string,
  closeToken: string,
  lineNumber: number,
  state: DeckParameterState,
): string {
  let result = "";
  let index = 0;
  while (index < line.length) {
    if (line[index] !== openToken) {
      result += line[index];
      index += 1;
      continue;
    }
    const closeIndex = line.indexOf(closeToken, index + 1);
    if (closeIndex < 0) {
      addParameterDiagnostic(state, {
        code: "SPICE_DECK_PARAM_UNTERMINATED",
        directive: ".param",
        lineNumber,
        message: `unterminated parameter expression starting at column ${index + 1}`,
      });
      result += line.slice(index);
      break;
    }
    const expression = line.slice(index + 1, closeIndex).trim();
    try {
      result += formatParameterNumber(evaluateParameterExpression(expression, state));
    } catch (error) {
      addParameterDiagnostic(state, {
        code: "SPICE_DECK_PARAM_UNRESOLVED",
        directive: ".param",
        lineNumber,
        message: error instanceof Error ? error.message : String(error),
        expression,
      });
      result += line.slice(index, closeIndex + 1);
    }
    index = closeIndex + 1;
  }
  return result;
}

function evaluateParameterExpression(expression: string, state: DeckParameterState): number {
  const value = new ParameterExpressionParser(expression, state).parse();
  if (!Number.isFinite(value)) {
    throw new Error(`parameter expression ${JSON.stringify(expression)} did not evaluate to a finite value`);
  }
  return value;
}

class ParameterExpressionParser {
  private index = 0;

  constructor(
    private readonly expression: string,
    private readonly state: DeckParameterState,
    private readonly localValues: ReadonlyMap<string, number> = new Map<string, number>(),
    private readonly callStack: readonly string[] = [],
  ) {}

  parse(): number {
    if (this.expression.length === 0) {
      throw new Error("parameter expression must not be empty");
    }
    const value = this.parseExpression();
    this.skipWhitespace();
    if (this.index !== this.expression.length) {
      throw new Error(`unexpected token ${JSON.stringify(this.expression[this.index])} in parameter expression`);
    }
    return value;
  }

  private parseExpression(): number {
    let value = this.parseTerm();
    while (true) {
      this.skipWhitespace();
      if (this.match("+")) {
        value += this.parseTerm();
      } else if (this.match("-")) {
        value -= this.parseTerm();
      } else {
        return value;
      }
    }
  }

  private parseTerm(): number {
    let value = this.parsePower();
    while (true) {
      this.skipWhitespace();
      if (this.match("*")) {
        value *= this.parsePower();
      } else if (this.match("/")) {
        const denominator = this.parsePower();
        if (denominator === 0.0) {
          throw new Error("division by zero in parameter expression");
        }
        value /= denominator;
      } else {
        return value;
      }
    }
  }

  private parsePower(): number {
    let value = this.parseUnary();
    this.skipWhitespace();
    if (this.match("^")) {
      value = value ** this.parsePower();
    }
    return value;
  }

  private parseUnary(): number {
    this.skipWhitespace();
    if (this.match("+")) {
      return this.parseUnary();
    }
    if (this.match("-")) {
      return -this.parseUnary();
    }
    return this.parsePrimary();
  }

  private parsePrimary(): number {
    this.skipWhitespace();
    if (this.match("(")) {
      const value = this.parseExpression();
      this.skipWhitespace();
      if (!this.match(")")) {
        throw new Error("missing ')' in parameter expression");
      }
      return value;
    }
    if (this.index >= this.expression.length) {
      throw new Error("unexpected end of parameter expression");
    }
    const char = this.expression[this.index];
    if (isDigit(char) || char === ".") {
      return this.parseNumber();
    }
    if (isAlpha(char) || char === "_") {
      return this.parseIdentifier();
    }
    throw new Error(`unexpected token ${JSON.stringify(char)} in parameter expression`);
  }

  private parseNumber(): number {
    const start = this.index;
    let sawDigit = false;
    while (this.index < this.expression.length && isDigit(this.expression[this.index])) {
      sawDigit = true;
      this.index += 1;
    }
    if (this.index < this.expression.length && this.expression[this.index] === ".") {
      this.index += 1;
      while (this.index < this.expression.length && isDigit(this.expression[this.index])) {
        sawDigit = true;
        this.index += 1;
      }
    }
    if (!sawDigit) {
      throw new Error("expected digit in numeric parameter expression");
    }
    if (
      this.index < this.expression.length &&
      (this.expression[this.index] === "e" || this.expression[this.index] === "E")
    ) {
      const exponentIndex = this.index;
      this.index += 1;
      if (
        this.index < this.expression.length &&
        (this.expression[this.index] === "+" || this.expression[this.index] === "-")
      ) {
        this.index += 1;
      }
      const exponentStart = this.index;
      while (this.index < this.expression.length && isDigit(this.expression[this.index])) {
        this.index += 1;
      }
      if (exponentStart === this.index) {
        this.index = exponentIndex;
      }
    }
    const numeric = Number.parseFloat(this.expression.slice(start, this.index));
    const suffixStart = this.index;
    while (this.index < this.expression.length && isAlpha(this.expression[this.index])) {
      this.index += 1;
    }
    const suffix = this.expression.slice(suffixStart, this.index).toLowerCase();
    if (suffix.length === 0) {
      return numeric;
    }
    const factor = SPICE_SUFFIX_FACTORS[suffix];
    if (factor === undefined) {
      throw new Error(`unsupported numeric suffix ${JSON.stringify(suffix)}`);
    }
    return numeric * factor;
  }

  private parseIdentifier(): number {
    const start = this.index;
    while (
      this.index < this.expression.length &&
      (isAlpha(this.expression[this.index]) ||
        isDigit(this.expression[this.index]) ||
        this.expression[this.index] === "_")
    ) {
      this.index += 1;
    }
    const name = this.expression.slice(start, this.index);
    this.skipWhitespace();
    if (this.index < this.expression.length && this.expression[this.index] === "(") {
      return this.evaluateFunctionCall(name, this.parseCallArguments());
    }
    const local = this.localValues.get(name.toLowerCase());
    if (local !== undefined) {
      return local;
    }
    if (name.toLowerCase() === "pi") {
      return Math.PI;
    }
    const parameter = this.state.getParameter(name);
    if (parameter === undefined) {
      throw new Error(`unknown parameter ${JSON.stringify(name)}`);
    }
    return parameter.value;
  }

  private parseCallArguments(): number[] {
    if (!this.match("(")) {
      throw new Error("expected '(' in function call");
    }
    this.skipWhitespace();
    if (this.match(")")) {
      return [];
    }
    const values: number[] = [];
    while (true) {
      values.push(this.parseExpression());
      this.skipWhitespace();
      if (this.match(",")) {
        continue;
      }
      if (this.match(")")) {
        return values;
      }
      throw new Error("missing ')' in function call");
    }
  }

  private evaluateFunctionCall(name: string, values: readonly number[]): number {
    const definition = this.state.getFunction(name);
    if (definition === undefined) {
      throw new Error(`unknown function ${JSON.stringify(name)}`);
    }
    if (values.length !== definition.arguments.length) {
      throw new Error(
        `function ${JSON.stringify(name)} expected ${definition.arguments.length} arguments but got ${values.length}`,
      );
    }
    const key = definition.name.toLowerCase();
    if (this.callStack.includes(key)) {
      throw new Error(`recursive function call ${JSON.stringify(name)}`);
    }
    const localValues = new Map(this.localValues);
    definition.arguments.forEach((argument, index) => {
      localValues.set(argument.toLowerCase(), values[index]);
    });
    return new ParameterExpressionParser(definition.expression, this.state, localValues, [
      ...this.callStack,
      key,
    ]).parse();
  }

  private skipWhitespace(): void {
    while (this.index < this.expression.length && /\s/.test(this.expression[this.index])) {
      this.index += 1;
    }
  }

  private match(token: string): boolean {
    if (this.expression.startsWith(token, this.index)) {
      this.index += token.length;
      return true;
    }
    return false;
  }
}

function addParameterDiagnostic(
  state: DeckParameterState,
  diagnostic: Omit<DeckParameterDiagnostic, "severity"> & { readonly severity?: "error" | "warning" },
): void {
  state.diagnostics.push({
    ...diagnostic,
    severity: diagnostic.severity ?? "error",
  });
}

function addInitialConditionDiagnostic(
  state: DeckInitialConditionState,
  diagnostic: Omit<DeckInitialConditionDiagnostic, "severity"> & { readonly severity?: "error" | "warning" },
): void {
  state.diagnostics.push({
    ...diagnostic,
    severity: diagnostic.severity ?? "error",
  });
}

function addFunctionDiagnostic(
  state: DeckFunctionState,
  diagnostic: Omit<DeckFunctionDiagnostic, "directive" | "severity"> & {
    readonly severity?: "error" | "warning";
  },
): void {
  state.diagnostics.push({
    ...diagnostic,
    directive: ".func",
    severity: diagnostic.severity ?? "error",
  });
}

function addMeasurementDiagnostic(
  state: DeckMeasurementState,
  diagnostic: Omit<DeckMeasurementDiagnostic, "severity"> & { readonly severity?: "error" | "warning" },
): void {
  state.diagnostics.push({
    ...diagnostic,
    severity: diagnostic.severity ?? "error",
  });
}

function addFourierDiagnostic(
  state: DeckFourierState,
  diagnostic: Omit<DeckFourierDiagnostic, "directive" | "severity"> & {
    readonly severity?: "error" | "warning";
  },
): void {
  state.diagnostics.push({
    ...diagnostic,
    directive: ".four",
    severity: diagnostic.severity ?? "error",
  });
}

function addOutputDiagnostic(
  state: DeckOutputState,
  diagnostic: Omit<DeckOutputDiagnostic, "severity"> & {
    readonly severity?: "error" | "warning";
  },
): void {
  state.diagnostics.push({
    ...diagnostic,
    severity: diagnostic.severity ?? "error",
  });
}

function addAnalysisDiagnostic(
  state: DeckAnalysisState,
  diagnostic: Omit<DeckAnalysisDiagnostic, "severity"> & {
    readonly severity?: "error" | "warning";
  },
): void {
  state.diagnostics.push({
    ...diagnostic,
    severity: diagnostic.severity ?? "error",
  });
}

function parseNodeConditionTarget(target: string): string | undefined {
  if (target.length < 4 || !target.toLowerCase().startsWith("v(") || !target.endsWith(")")) {
    return undefined;
  }
  const node = target.slice(2, -1).trim();
  return node.length > 0 ? node : undefined;
}

function isDirectiveWhitespace(char: string): boolean {
  return char === " " || char === "\t" || char === "\r" || char === "\n" || char === "\f";
}

function parseFunctionSignature(
  rest: string,
): { readonly name: string; readonly argumentList: readonly string[]; readonly expression: string } | undefined {
  const openIndex = rest.indexOf("(");
  if (openIndex < 0) {
    return undefined;
  }
  const closeIndex = rest.indexOf(")", openIndex + 1);
  if (closeIndex < 0) {
    return undefined;
  }
  const name = rest.slice(0, openIndex).trim();
  const argumentsRaw = rest.slice(openIndex + 1, closeIndex).trim();
  const expression = rest.slice(closeIndex + 1).trim();
  const argumentList = argumentsRaw.length === 0 ? [] : argumentsRaw.split(",").map((argument) => argument.trim());
  return { name, argumentList, expression };
}

function isParameterName(name: string): boolean {
  return /^[A-Za-z_][A-Za-z0-9_]*$/.test(name);
}

function normalizeMeasurementModeToken(mode: string): string | undefined {
  const normalized = mode.trim().toLowerCase().replace(/_/g, "-");
  switch (normalized) {
    case "max":
    case "min":
      return normalized;
    case "avg":
    case "average":
    case "mean":
      return "avg";
    case "rms":
    case "root-mean-square":
      return "rms";
    case "pp":
    case "p-p":
    case "p2p":
    case "peak-to-peak":
    case "peak2peak":
      return "pp";
    case "last":
    case "final":
      return "last";
    case "find":
      return "find";
    case "when":
      return "when";
    default:
      return undefined;
  }
}

function normalizeDeckOutputAnalysis(analysis: string): DeckOutputSelection["analysis"] | undefined {
  switch (analysis.trim().toLowerCase()) {
    case "op":
    case "dcop":
      return "op";
    case "dc":
      return "dc";
    case "ac":
      return "ac";
    case "tran":
    case "transient":
      return "tran";
    default:
      return undefined;
  }
}

function normalizeDeckAnalysisName(analysis: string): DeckAnalysisPlan["analysis"] | undefined {
  switch (analysis.trim().toLowerCase().replace(/^\./, "").replace(/_/g, "-")) {
    case "op":
    case "dcop":
    case "operating-point":
    case "operatingpoint":
      return "op";
    case "dc":
    case "dc-sweep":
    case "dcsweep":
      return "dc";
    case "ac":
    case "ac-sweep":
    case "acsweep":
      return "ac";
    case "tran":
    case "transient":
      return "tran";
    case "tf":
    case "transfer-function":
    case "transferfunction":
      return "tf";
    case "sens":
    case "sensitivity":
      return "sens";
    case "noise":
    case "ac-noise":
    case "noise-ac":
      return "noise";
    default:
      return undefined;
  }
}

function deckOutputAnalysisMatches(requested: string, analysis: string): boolean {
  return normalizeDeckOutputAnalysis(requested) === normalizeDeckOutputAnalysis(analysis);
}

function normalizeDeckOutputProbe(token: string): string | undefined {
  const text = token.trim();
  if (!text.endsWith(")")) {
    return undefined;
  }
  const lower = text.toLowerCase();
  let prefix: "V" | "I";
  if (lower.startsWith("v(")) {
    prefix = "V";
  } else if (lower.startsWith("i(")) {
    prefix = "I";
  } else {
    return undefined;
  }
  const target = text.slice(2, -1).trim();
  if (
    target.length === 0 ||
    target.includes("(") ||
    target.includes(")") ||
    target.includes(",") ||
    /\s/.test(target)
  ) {
    return undefined;
  }
  return `${prefix}(${target})`;
}

function deckOutputProbeKey(probe: string): string {
  return probe.toLowerCase();
}

function parseDeckAnalysisValue(
  token: string,
  directive: DeckAnalysisDiagnostic["directive"],
  lineNumber: number,
  state: DeckAnalysisState,
): number | undefined {
  try {
    return evaluateParameterExpression(
      stripExpressionDelimiters(unquoteToken(token).trim()),
      new DeckParameterState(),
    );
  } catch (error) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_EXPRESSION",
      directive,
      lineNumber,
      message: error instanceof Error ? error.message : String(error),
      token,
    });
    return undefined;
  }
}

function parseDeckAnalysisInteger(
  token: string,
  directive: DeckAnalysisDiagnostic["directive"],
  lineNumber: number,
  state: DeckAnalysisState,
): number | undefined {
  const value = parseDeckAnalysisValue(token, directive, lineNumber, state);
  if (value === undefined) {
    return undefined;
  }
  if (value < 0.0 || value % 1.0 !== 0.0) {
    addAnalysisDiagnostic(state, {
      code: "SPICE_DECK_ANALYSIS_ARGUMENT",
      directive,
      lineNumber,
      message: `${directive} point count must be an integer`,
      token,
    });
    return undefined;
  }
  return value;
}

function normalizeAcSweepKind(token: string): DeckAnalysisPlan["sweepKind"] | undefined {
  switch (token.trim().toLowerCase()) {
    case "lin":
      return "lin";
    case "dec":
      return "dec";
    case "oct":
      return "oct";
    default:
      return undefined;
  }
}

function stripExpressionDelimiters(expression: string): string {
  if (
    expression.length >= 2 &&
    ((expression[0] === "{" && expression[expression.length - 1] === "}") ||
      (expression[0] === "'" && expression[expression.length - 1] === "'"))
  ) {
    return expression.slice(1, -1).trim();
  }
  return expression;
}

function formatParameterNumber(value: number): string {
  if (value === 0.0) {
    return "0";
  }
  const absValue = Math.abs(value);
  if (absValue >= 1.0e-12 && absValue < 1.0e12) {
    const formatted = value.toFixed(12).replace(/0+$/, "").replace(/\.$/, "");
    return formatted === "-0" ? "0" : formatted;
  }
  const [mantissaRaw, exponentRaw] = value.toExponential(12).split("e");
  const mantissa = mantissaRaw.replace(/0+$/, "").replace(/\.$/, "");
  const exponent = Number.parseInt(exponentRaw, 10);
  return `${mantissa}e${exponent >= 0 ? "+" : ""}${exponent}`;
}

function isAlpha(char: string): boolean {
  return /^[A-Za-z]$/.test(char);
}

function isDigit(char: string): boolean {
  return /^[0-9]$/.test(char);
}

function directiveTokens(line: string): string[] {
  return line.split(/\s+/);
}

function unquoteToken(token: string): string {
  if (
    token.length >= 2 &&
    token[0] === token[token.length - 1] &&
    (token[0] === "'" || token[0] === '"')
  ) {
    return token.slice(1, -1);
  }
  return token;
}

function validateCompatibilityNonEmpty(
  deckId: string,
  field: string,
  value: string,
  issues: ReleaseReadinessIssue[],
): void {
  if (value.trim().length > 0) {
    return;
  }
  issues.push({
    deckId,
    field,
    message: "field must be documented and non-empty",
  });
}

function deckDirective(line: string): string | undefined {
  if (!line.startsWith(".")) {
    return undefined;
  }
  return line.split(/\s+/, 1)[0].toLowerCase();
}

function controlBlockCommandAsDeckLine(line: string): string | undefined {
  const parts = line.split(/\s+/, 1);
  const command = parts[0]?.toLowerCase();
  if (command === undefined || !SUPPORTED_CONTROL_BLOCK_COMMANDS.has(command)) {
    return undefined;
  }
  const directive =
    command === "four" || command === ".four" || command === "fourier" || command === ".fourier"
      ? ".four"
      : command.startsWith(".")
        ? command
        : `.${command}`;
  const rest = line.slice(parts[0].length).trimStart();
  return rest.length === 0 ? directive : `${directive} ${rest}`;
}

function controlBlockWriteMarker(line: string): string | undefined {
  const parts = line.split(/\s+/);
  const command = parts[0]?.toLowerCase();
  if (command === undefined) {
    return undefined;
  }
  if (NOOP_CONTROL_BLOCK_ARGUMENT_COMMANDS.has(command)) {
    const rest = parts.slice(1);
    return rest.length === 0 ? undefined : `${command.replace(/^\./, "")} ${rest.join(" ")}`;
  }
  if (NOOP_CONTROL_BLOCK_VECTOR_ARGUMENT_COMMANDS.has(command)) {
    const rest = parts.slice(1);
    return rest.length < 2 ? undefined : `${command.replace(/^\./, "")} ${rest.join(" ")}`;
  }
  return undefined;
}

function controlBlockRawfileOption(line: string): string | undefined {
  const parts = line.split(/\s+/);
  const command = parts[0]?.toLowerCase();
  if (command !== "set" && command !== ".set") {
    return undefined;
  }
  if (parts.length !== 2) {
    return undefined;
  }
  const option = parts[1]?.toLowerCase() ?? "";
  if (
    option === "filetype=ascii" ||
    option === "wr_vecnames" ||
    option === "wr_singlescale" ||
    option === "appendwrite"
  ) {
    return `set ${option}`;
  }
  return undefined;
}

function isNoopControlBlockCommand(line: string): boolean {
  const parts = line.split(/\s+/);
  const command = parts[0]?.toLowerCase();
  if (command === undefined) {
    return false;
  }
  if (NOOP_CONTROL_BLOCK_COMMANDS.has(command)) {
    return true;
  }
  if (NOOP_CONTROL_BLOCK_ARGUMENT_COMMANDS.has(command)) {
    return parts.length >= 2;
  }
  if (NOOP_CONTROL_BLOCK_VECTOR_ARGUMENT_COMMANDS.has(command)) {
    return parts.length >= 3;
  }
  return (
    (command === "set" || command === ".set") &&
    parts.length === 2 &&
    NOOP_CONTROL_BLOCK_SET_OPTIONS.has(parts[1]?.toLowerCase() ?? "")
  );
}

function isScriptControlBlockCommand(line: string): boolean {
  const command = line.split(/\s+/, 1)[0]?.toLowerCase();
  return command !== undefined && SCRIPT_CONTROL_BLOCK_COMMANDS.has(command);
}

function isWorkdirControlBlockCommand(line: string): boolean {
  const command = line.split(/\s+/, 1)[0]?.toLowerCase();
  return command !== undefined && WORKDIR_CONTROL_BLOCK_COMMANDS.has(command);
}

function isControlFlowControlBlockCommand(line: string): boolean {
  const command = line.split(/\s+/, 1)[0]?.toLowerCase();
  return command !== undefined && CONTROL_FLOW_CONTROL_BLOCK_COMMANDS.has(command);
}

function isVariableControlBlockCommand(line: string): boolean {
  const command = line.split(/\s+/, 1)[0]?.toLowerCase();
  return command !== undefined && VARIABLE_CONTROL_BLOCK_COMMANDS.has(command);
}

function controlBlockScriptPolicyMessage(line: string): string {
  return `${JSON.stringify(line)} inside .control is not executed because external script and shell commands are disabled by the deck execution policy`;
}

function controlBlockWorkdirPolicyMessage(line: string): string {
  return `${JSON.stringify(line)} inside .control is not executed because working-directory mutation is disabled by the deck execution policy`;
}

function controlBlockFlowPolicyMessage(line: string): string {
  return `${JSON.stringify(line)} inside .control is not executed because control-flow commands are disabled by the deck execution policy`;
}

function controlBlockVariablePolicyMessage(line: string): string {
  return `${JSON.stringify(line)} inside .control is not executed because control variables and circuit mutation commands are disabled by the deck execution policy`;
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
  junctionPotential = 1.0,
  gradingCoefficient = 0.5,
  forwardBiasDepletionCoefficient = 0.5,
  saturationCurrentTemperatureExponent = 3.0,
  energyGapElectronVolts = 1.11,
  seriesResistance = 0.0,
  flickerNoiseCoefficient = 0.0,
  flickerNoiseExponent = 1.0,
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
    junctionPotential,
    gradingCoefficient,
    forwardBiasDepletionCoefficient,
    saturationCurrentTemperatureExponent,
    energyGapElectronVolts,
    seriesResistance,
    flickerNoiseCoefficient,
    flickerNoiseExponent,
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
  if (!Number.isFinite(element.saturationCurrentTemperatureExponent)) {
    throw invalidElement(
      element.name,
      "saturation-current temperature exponent must be finite",
    );
  }
  const ratio = temperatureKelvin / nominalTemperatureKelvin;
  const exponent =
    (energyGapElectronVolts * ELECTRON_CHARGE) /
    (element.emissionCoefficient * BOLTZMANN) *
    (1.0 / nominalTemperatureKelvin - 1.0 / temperatureKelvin);
  const saturationScale =
    ratio ** element.saturationCurrentTemperatureExponent *
    Math.exp(Math.max(-100.0, Math.min(100.0, exponent)));
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
  nominalTemperatureKelvin = element.nominalTemperatureKelvin ?? nominalTemperatureKelvin;
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
  if (!Number.isFinite(element.saturationCurrentTemperatureExponent)) {
    throw invalidElement(element.name, "saturation-current temperature exponent must be finite");
  }
  if (!Number.isFinite(element.forwardBetaTemperatureExponent)) {
    throw invalidElement(element.name, "beta temperature exponent must be finite");
  }
  if (Number.isNaN(element.reverseBeta) || element.reverseBeta <= 0.0) {
    throw invalidElement(element.name, "reverse beta must be positive");
  }
  const saturationScale =
    ratio ** element.saturationCurrentTemperatureExponent *
    Math.exp(Math.max(-100.0, Math.min(100.0, exponent)));
  return {
    ...element,
    saturationCurrent: element.saturationCurrent * saturationScale,
    baseEmitterLeakageSaturationCurrent:
      element.baseEmitterLeakageSaturationCurrent * saturationScale,
    baseCollectorLeakageSaturationCurrent:
      element.baseCollectorLeakageSaturationCurrent * saturationScale,
    forwardBeta: element.forwardBeta * ratio ** element.forwardBetaTemperatureExponent,
    reverseBeta: element.reverseBeta * ratio ** element.forwardBetaTemperatureExponent,
    thermalVoltage: element.thermalVoltage * ratio,
  };
}

export function mosfetAtTemperature(
  element: Mosfet,
  temperatureKelvin: number,
  nominalTemperatureKelvin = 300.15,
  energyGapElectronVolts = 1.11,
): Mosfet {
  if (!Number.isFinite(temperatureKelvin) || temperatureKelvin <= 0.0) {
    throw invalidElement(element.name, "temperature must be finite and positive");
  }
  if (!Number.isFinite(nominalTemperatureKelvin) || nominalTemperatureKelvin <= 0.0) {
    throw invalidElement(element.name, "nominal temperature must be finite and positive");
  }
  if (!Number.isFinite(energyGapElectronVolts) || energyGapElectronVolts <= 0.0) {
    throw invalidElement(element.name, "energy gap must be finite and positive");
  }
  const referenceTemperatureKelvin = 300.15;
  const nominalTemperature =
    element.params.T_NOM !== referenceTemperatureKelvin
      ? element.params.T_NOM
      : nominalTemperatureKelvin;
  if (!Number.isFinite(nominalTemperature) || nominalTemperature <= 0.0) {
    throw invalidElement(element.name, "nominal temperature must be finite and positive");
  }
  const ratio = temperatureKelvin / nominalTemperature;
  const potentialCorrection = (temperature: number): number => {
    const thermalVoltage = (BOLTZMANN * temperature) / ELECTRON_CHARGE;
    const argument =
      (-siliconBandGapElectronVolts(temperature) * ELECTRON_CHARGE) /
        (2.0 * BOLTZMANN * temperature) +
      (1.115_087_7 * ELECTRON_CHARGE) /
        (2.0 * BOLTZMANN * referenceTemperatureKelvin);
    return (
      -2.0 *
      thermalVoltage *
      (1.5 * Math.log(temperature / referenceTemperatureKelvin) + argument)
    );
  };
  const nominalFactor = nominalTemperature / referenceTemperatureKelvin;
  const temperatureFactor = temperatureKelvin / referenceTemperatureKelvin;
  const nominalPhi =
    (element.params.PHI - potentialCorrection(nominalTemperature)) / nominalFactor;
  const temperaturePhi =
    temperatureFactor * nominalPhi + potentialCorrection(temperatureKelvin);
  const nominalBulkJunctionPotential =
    (element.params.PB - potentialCorrection(nominalTemperature)) / nominalFactor;
  const temperatureBulkJunctionPotential =
    temperatureFactor * nominalBulkJunctionPotential +
    potentialCorrection(temperatureKelvin);
  const nominalBulkPotentialShift =
    (element.params.PB - nominalBulkJunctionPotential) / nominalBulkJunctionPotential;
  const temperatureBulkPotentialShift =
    (temperatureBulkJunctionPotential - nominalBulkJunctionPotential) /
    nominalBulkJunctionPotential;
  const capacitanceScale = (gradingCoefficient: number): number => {
    const nominalScale =
      1.0 /
      (1.0 +
        gradingCoefficient *
          (4.0e-4 * (nominalTemperature - referenceTemperatureKelvin) -
            nominalBulkPotentialShift));
    const temperatureScale =
      1.0 +
      gradingCoefficient *
        (4.0e-4 * (temperatureKelvin - referenceTemperatureKelvin) -
          temperatureBulkPotentialShift);
    return nominalScale * temperatureScale;
  };
  const bottomCapacitanceScale = capacitanceScale(element.params.MJ);
  const sidewallCapacitanceScale = capacitanceScale(element.params.MJSW);
  const polarity = element.type === "NMOS" ? 1.0 : -1.0;
  const temperatureVbi =
    element.params.VT0 -
    polarity * element.params.GAMMA * Math.sqrt(element.params.PHI) +
    0.5 *
      (siliconBandGapElectronVolts(nominalTemperature) -
        siliconBandGapElectronVolts(temperatureKelvin)) +
    polarity * 0.5 * (temperaturePhi - element.params.PHI);
  const temperatureVt0 =
    temperatureVbi + polarity * element.params.GAMMA * Math.sqrt(temperaturePhi);
  const saturationExponent =
    (energyGapElectronVolts * ELECTRON_CHARGE) /
    BOLTZMANN *
    (1.0 / nominalTemperature - 1.0 / temperatureKelvin);
  const saturationScale =
    ratio ** 3 * Math.exp(Math.max(-100.0, Math.min(100.0, saturationExponent)));
  return {
    ...element,
    params: {
      ...element.params,
      VT0: temperatureVt0,
      PHI: temperaturePhi,
      PB: temperatureBulkJunctionPotential,
      CJ: element.params.CJ * bottomCapacitanceScale,
      CBS: element.params.CBS * bottomCapacitanceScale,
      CBD: element.params.CBD * bottomCapacitanceScale,
      CJSW: element.params.CJSW * sidewallCapacitanceScale,
      KP: element.params.KP * ratio ** -1.5,
      U0: element.params.U0 * ratio ** -1.5,
      IS: element.params.IS * saturationScale,
      JS: element.params.JS * saturationScale,
      T_NOM: temperatureKelvin,
    },
  };
}

export function jfetAtTemperature(
  element: Jfet,
  temperatureKelvin: number,
  nominalTemperatureKelvin = 300.15,
): Jfet {
  if (!Number.isFinite(temperatureKelvin) || temperatureKelvin <= 0.0) {
    throw invalidElement(element.name, "temperature must be finite and positive");
  }
  const nominalTemperature =
    element.nominalTemperatureKelvin ?? nominalTemperatureKelvin;
  if (!Number.isFinite(nominalTemperature) || nominalTemperature <= 0.0) {
    throw invalidElement(element.name, "nominal temperature must be finite and positive");
  }
  if (!Number.isFinite(element.thresholdVoltageTemperatureCoefficient)) {
    throw invalidElement(
      element.name,
      "threshold-voltage temperature coefficient must be finite",
    );
  }
  if (
    element.alternativeThresholdVoltageTemperatureCoefficient !== undefined &&
    !Number.isFinite(element.alternativeThresholdVoltageTemperatureCoefficient)
  ) {
    throw invalidElement(
      element.name,
      "alternative threshold-voltage temperature coefficient must be finite",
    );
  }
  if (!Number.isFinite(element.mobilityTemperatureExponent)) {
    throw invalidElement(element.name, "mobility temperature exponent must be finite");
  }
  if (!Number.isFinite(element.gateSaturationCurrentTemperatureExponent)) {
    throw invalidElement(
      element.name,
      "gate saturation-current temperature exponent must be finite",
    );
  }
  if (!Number.isFinite(element.bandgapVoltage) || element.bandgapVoltage <= 0.0) {
    throw invalidElement(element.name, "bandgap voltage must be finite and positive");
  }
  if (!Number.isFinite(element.dopingTailParameter)) {
    throw invalidElement(element.name, "doping-tail parameter must be finite");
  }
  if (
    !Number.isFinite(element.noiseEquationLevel) ||
    element.noiseEquationLevel < 1.0 ||
    !Number.isInteger(element.noiseEquationLevel)
  ) {
    throw invalidElement(
      element.name,
      "noise equation level must be a finite integer greater than or equal to 1",
    );
  }
  if (
    !Number.isFinite(element.channelNoiseCoefficient) ||
    element.channelNoiseCoefficient < 0.0
  ) {
    throw invalidElement(
      element.name,
      "channel noise coefficient must be finite and non-negative",
    );
  }
  if (
    element.mobilityTemperatureCoefficient !== undefined &&
    !Number.isFinite(element.mobilityTemperatureCoefficient)
  ) {
    throw invalidElement(element.name, "mobility temperature coefficient must be finite");
  }
  const temperatureRatio = temperatureKelvin / nominalTemperature;
  const saturationExponent =
    (element.bandgapVoltage * ELECTRON_CHARGE) /
    BOLTZMANN *
    (1.0 / nominalTemperature - 1.0 / temperatureKelvin);
  const saturationScale =
    temperatureRatio ** element.gateSaturationCurrentTemperatureExponent *
    Math.exp(Math.max(-100.0, Math.min(100.0, saturationExponent)));
  const betaScale =
    element.mobilityTemperatureCoefficient !== undefined
      ? 1.01 **
        (element.mobilityTemperatureCoefficient *
          (temperatureKelvin - nominalTemperature))
      : temperatureRatio ** element.mobilityTemperatureExponent;
  return {
    ...element,
    beta: element.beta * betaScale,
    gateSaturationCurrent: element.gateSaturationCurrent * saturationScale,
    thresholdVoltage:
      element.alternativeThresholdVoltageTemperatureCoefficient !== undefined
        ? element.thresholdVoltage +
          element.alternativeThresholdVoltageTemperatureCoefficient *
            (temperatureKelvin - nominalTemperature)
        : element.thresholdVoltage -
          element.thresholdVoltageTemperatureCoefficient *
            (temperatureKelvin - nominalTemperature),
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
          element.energyGapElectronVolts,
        ),
      );
    } else if (element.kind === "bjt") {
      adjusted.add(
        bjtAtTemperature(
          element,
          temperatureKelvin,
          nominalTemperatureKelvin,
          element.energyGapElectronVolts,
        ),
      );
    } else if (element.kind === "jfet") {
      adjusted.add(jfetAtTemperature(element, temperatureKelvin, nominalTemperatureKelvin));
    } else if (element.kind === "mosfet") {
      adjusted.add(
        mosfetAtTemperature(
          element,
          temperatureKelvin,
          nominalTemperatureKelvin,
          energyGapElectronVolts,
        ),
      );
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
  gateSourceCapacitance = 0.0,
  gateDrainCapacitance = 0.0,
  flickerNoiseCoefficient = 0.0,
  flickerNoiseExponent = 1.0,
  junctionPotential = 1.0,
  forwardBiasDepletionCoefficient = 0.5,
  gateSaturationCurrent = 1.0e-14,
  gateSaturationCurrentTemperatureExponent = 3.0,
  bandgapVoltage = 1.11,
  dopingTailParameter = 1.0,
  noiseEquationLevel = 1.0,
  channelNoiseCoefficient = 1.0,
  drainResistance = 0.0,
  sourceResistance = 0.0,
  thresholdVoltageTemperatureCoefficient = 0.0,
  alternativeThresholdVoltageTemperatureCoefficient: number | undefined = undefined,
  nominalTemperatureKelvin: number | undefined = undefined,
  mobilityTemperatureExponent = 0.0,
  mobilityTemperatureCoefficient: number | undefined = undefined,
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
    gateSourceCapacitance,
    gateDrainCapacitance,
    flickerNoiseCoefficient,
    flickerNoiseExponent,
    junctionPotential,
    forwardBiasDepletionCoefficient,
    gateSaturationCurrent,
    gateSaturationCurrentTemperatureExponent,
    bandgapVoltage,
    dopingTailParameter,
    noiseEquationLevel,
    channelNoiseCoefficient,
    drainResistance,
    sourceResistance,
    thresholdVoltageTemperatureCoefficient,
    alternativeThresholdVoltageTemperatureCoefficient,
    nominalTemperatureKelvin,
    mobilityTemperatureExponent,
    mobilityTemperatureCoefficient,
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
  saturationCurrentTemperatureExponent = 3.0,
  energyGapElectronVolts = 1.11,
  forwardEarlyVoltage = 0.0,
  forwardEmissionCoefficient = 1.0,
  reverseEmissionCoefficient = 1.0,
  baseEmitterJunctionPotential = 0.75,
  baseEmitterGradingCoefficient = 0.33,
  baseCollectorJunctionPotential = 0.75,
  baseCollectorGradingCoefficient = 0.33,
  forwardBiasDepletionCoefficient = 0.5,
  reverseEarlyVoltage = 0.0,
  forwardBetaRolloffCurrent = 0.0,
  baseEmitterLeakageSaturationCurrent = 0.0,
  baseEmitterLeakageEmissionCoefficient = 1.0,
  baseCollectorLeakageSaturationCurrent = 0.0,
  baseCollectorLeakageEmissionCoefficient = 2.0,
  forwardBetaTemperatureExponent = 0.0,
  reverseBeta = Number.POSITIVE_INFINITY,
  reverseBetaRolloffCurrent = 0.0,
  nominalTemperatureKelvin: number | undefined = undefined,
  flickerNoiseCoefficient = 0.0,
  flickerNoiseExponent = 1.0,
  forwardExcessPhaseDegrees = 0.0,
  forwardTransitTimeBiasCoefficient = 0.0,
  forwardTransitTimeCurrent = 0.0,
  forwardTransitTimeVoltage = 0.0,
  emitterResistance = 0.0,
  collectorResistance = 0.0,
  baseResistance = 0.0,
  minimumBaseResistance: number | undefined = undefined,
  baseResistanceHalfCurrent = 0.0,
  baseCollectorCapacitanceFraction = 1.0,
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
    saturationCurrentTemperatureExponent,
    energyGapElectronVolts,
    forwardEarlyVoltage,
    reverseEarlyVoltage,
    forwardEmissionCoefficient,
    reverseEmissionCoefficient,
    baseEmitterJunctionPotential,
    baseEmitterGradingCoefficient,
    baseCollectorJunctionPotential,
    baseCollectorGradingCoefficient,
    forwardBiasDepletionCoefficient,
    forwardBetaRolloffCurrent,
    baseEmitterLeakageSaturationCurrent,
    baseEmitterLeakageEmissionCoefficient,
    baseCollectorLeakageSaturationCurrent,
    baseCollectorLeakageEmissionCoefficient,
    forwardBetaTemperatureExponent,
    reverseBeta,
    reverseBetaRolloffCurrent,
    nominalTemperatureKelvin,
    flickerNoiseCoefficient,
    flickerNoiseExponent,
    forwardExcessPhaseDegrees,
    forwardTransitTimeBiasCoefficient,
    forwardTransitTimeCurrent,
    forwardTransitTimeVoltage,
    emitterResistance,
    collectorResistance,
    baseResistance,
    minimumBaseResistance,
    baseResistanceHalfCurrent,
    baseCollectorCapacitanceFraction,
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
    LD: 0.0,
    TOX: 1.0e-7,
    U0: 600.0,
    RD: 0.0,
    RS: 0.0,
    RSH: 0.0,
    NRD: 1.0,
    NRS: 1.0,
    AD: 0.0,
    AS: 0.0,
    PD: 0.0,
    PS: 0.0,
    CJ: 0.0,
    CJSW: 0.0,
    IS: 1.0e-15,
    JS: 0.0,
    N_SUB: 1.4,
    T_NOM: 300.15,
    CGSO: 0.0,
    CGDO: 0.0,
    CGBO: 0.0,
    CBS: 0.0,
    CBD: 0.0,
    PB: 0.8,
    MJ: 0.5,
    MJSW: 0.33,
    FC: 0.5,
    KF: 0.0,
    AF: 1.0,
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

const MODEL_TYPE_ALIASES: Readonly<Record<string, ModelCardKind>> = {
  D: "D",
  DIODE: "D",
  NPN: "NPN",
  PNP: "PNP",
  NJF: "NJF",
  NJFET: "NJF",
  NJ: "NJF",
  PJF: "PJF",
  PJFET: "PJF",
  PJ: "PJF",
  NMOS: "NMOS",
  NCH: "NMOS",
  PMOS: "PMOS",
  PCH: "PMOS",
};

const DIODE_PARAMETER_ALIASES: Readonly<Record<string, string>> = {
  IS: "IS",
  XTI: "XTI",
  JS: "IS",
  VT: "VT",
  V_T: "VT",
  N: "N",
  BV: "BV",
  IBV: "IBV",
  CJO: "CJO",
  CJ: "CJO",
  CJ0: "CJO",
  TT: "TT",
  VJ: "VJ",
  PB: "VJ",
  M: "M",
  MJ: "M",
  FC: "FC",
  EG: "EG",
  RS: "RS",
  KF: "KF",
  AF: "AF",
};

const BJT_PARAMETER_ALIASES: Readonly<Record<string, string>> = {
  IS: "IS",
  BF: "BF",
  BETA: "BF",
  BETA_F: "BF",
  HFE: "BF",
  VT: "VT",
  V_T: "VT",
  CJE: "CJE",
  CJE0: "CJE",
  CBE: "CJE",
  CJC: "CJC",
  CJC0: "CJC",
  CBC: "CJC",
  TF: "TF",
  TR: "TR",
  XTI: "XTI",
  EG: "EG",
  VAF: "VAF",
  VA: "VAF",
  VAR: "VAR",
  VB: "VAR",
  IKF: "IKF",
  IK: "IKF",
  IKR: "IKR",
  TNOM: "TNOM",
  T_NOM: "TNOM",
  KF: "KF",
  AF: "AF",
  PTF: "PTF",
  XTF: "XTF",
  ITF: "ITF",
  VTF: "VTF",
  RE: "RE",
  RC: "RC",
  RB: "RB",
  RBM: "RBM",
  IRB: "IRB",
  XCJC: "XCJC",
  ISE: "ISE",
  C2: "C2",
  NE: "NE",
  ISC: "ISC",
  C4: "C4",
  NC: "NC",
  XTB: "XTB",
  BR: "BR",
  BETA_R: "BR",
  NF: "NF",
  NR: "NR",
  VJE: "VJE",
  PE: "VJE",
  MJE: "MJE",
  ME: "MJE",
  VJC: "VJC",
  PC: "VJC",
  MJC: "MJC",
  MC: "MJC",
  FC: "FC",
};

const JFET_PARAMETER_ALIASES: Readonly<Record<string, string>> = {
  BETA: "BETA",
  BET: "BETA",
  VTO: "VTO",
  VT0: "VTO",
  VTH: "VTO",
  LAMBDA: "LAMBDA",
  LAM: "LAMBDA",
  CGS: "CGS",
  CGS0: "CGS",
  CGD: "CGD",
  CGD0: "CGD",
  KF: "KF",
  AF: "AF",
  PB: "PB",
  VJ: "PB",
  FC: "FC",
  IS: "IS",
  XTI: "XTI",
  EG: "EG",
  B: "B",
  NLEV: "NLEV",
  GDSNOI: "GDSNOI",
  RD: "RD",
  RS: "RS",
  TNOM: "TNOM",
  T_NOM: "TNOM",
  TCV: "TCV",
  VTOTC: "VTOTC",
  BEX: "BEX",
  BETATCE: "BETATCE",
};

const MOS_LEVEL1_PARAMETER_ALIASES: Readonly<Record<string, string>> = {
  LEVEL: "LEVEL",
  VT0: "VT0",
  VTO: "VT0",
  VTH: "VT0",
  KP: "KP",
  LAMBDA: "LAMBDA",
  LAM: "LAMBDA",
  GAMMA: "GAMMA",
  PHI: "PHI",
  W: "W",
  L: "L",
  LD: "LD",
  TOX: "TOX",
  U0: "U0",
  UO: "U0",
  RD: "RD",
  RS: "RS",
  RSH: "RSH",
  IS: "IS",
  JS: "JS",
  NSUB: "N_SUB",
  N_SUB: "N_SUB",
  NSS: "NSS",
  TPG: "TPG",
  TNOM: "T_NOM",
  T_NOM: "T_NOM",
  CGSO: "CGSO",
  CGDO: "CGDO",
  CGBO: "CGBO",
  CBS: "CBS",
  CJS: "CBS",
  CBD: "CBD",
  CJD: "CBD",
  CJ: "CJ",
  CJSW: "CJSW",
  PB: "PB",
  MJ: "MJ",
  MJSW: "MJSW",
  FC: "FC",
  KF: "KF",
  AF: "AF",
};

function modelTypeKey(text: string): string {
  return text.trim().toUpperCase().replace(/[-_]/g, "");
}

function parameterKey(text: string): string {
  return text.trim().toUpperCase().replace(/-/g, "_");
}

export function normalizeModelCardType(modelType: string): ModelCardKind {
  const kind = MODEL_TYPE_ALIASES[modelTypeKey(modelType)];
  if (kind === undefined) {
    throw invalidElement(modelType, "unsupported SPICE model type");
  }
  return kind;
}

function parameterAliases(kind: ModelCardKind): Readonly<Record<string, string>> {
  if (kind === "D") {
    return DIODE_PARAMETER_ALIASES;
  }
  if (kind === "NPN" || kind === "PNP") {
    return BJT_PARAMETER_ALIASES;
  }
  if (kind === "NJF" || kind === "PJF") {
    return JFET_PARAMETER_ALIASES;
  }
  return MOS_LEVEL1_PARAMETER_ALIASES;
}

export function normalizeModelCard(
  name: string,
  modelType: string,
  parameters: Readonly<Record<string, number>> = {},
): NormalizedModelCard {
  const kind = normalizeModelCardType(modelType);
  const aliases = parameterAliases(kind);
  const normalized: Record<string, number> = {};
  const unsupported: string[] = [];
  for (const [rawName, rawValue] of Object.entries(parameters)) {
    const key = parameterKey(rawName);
    const canonical = aliases[key];
    if (canonical === undefined) {
      if (!unsupported.includes(key)) {
        unsupported.push(key);
      }
      continue;
    }
    const value = Number(rawValue);
    if (canonical === "LEVEL") {
      if (Math.abs(value - 1.0) > 1.0e-12) {
        throw invalidElement(name, "only MOS LEVEL=1 model cards are supported");
      }
      normalized[canonical] = 1.0;
    } else if (canonical === "TPG" && value !== -1.0 && value !== 0.0 && value !== 1.0) {
      throw invalidElement(name, "MOSFET TPG must be -1, 0, or 1");
    } else if (canonical === "NSS" && (!Number.isFinite(value) || value < 0.0)) {
      throw invalidElement(name, "MOSFET NSS must be finite and non-negative");
    } else if (canonical === "T_NOM" && (!Number.isFinite(value) || value <= 0.0)) {
      throw invalidElement(name, "MOSFET TNOM must be finite and positive");
    } else if (canonical === "N_SUB" && (!Number.isFinite(value) || value <= 0.0)) {
      throw invalidElement(name, "MOSFET NSUB must be finite and positive");
    } else if (canonical === "TOX" && (!Number.isFinite(value) || value <= 0.0)) {
      throw invalidElement(name, "MOSFET TOX must be finite and positive");
    } else if (canonical === "U0" && (!Number.isFinite(value) || value < 0.0)) {
      throw invalidElement(name, "MOSFET U0 must be finite and non-negative");
    } else if (canonical === "KP" && (!Number.isFinite(value) || value <= 0.0)) {
      throw invalidElement(name, "MOSFET KP must be finite and positive");
    } else if (canonical === "VT0" && !Number.isFinite(value)) {
      throw invalidElement(name, "MOSFET VT0 must be finite");
    } else if (canonical === "LAMBDA" && !Number.isFinite(value)) {
      throw invalidElement(name, "MOSFET LAMBDA must be finite");
    } else if (
      canonical === "PHI" &&
      (!Number.isFinite(value) || value <= 0.0)
    ) {
      throw invalidElement(name, "MOSFET PHI must be finite and positive");
    } else if (
      canonical === "GAMMA" &&
      (!Number.isFinite(value) || value < 0.0)
    ) {
      throw invalidElement(name, "MOSFET GAMMA must be finite and non-negative");
    } else if (
      canonical === "PB" &&
      (!Number.isFinite(value) || value <= 0.0)
    ) {
      throw invalidElement(name, "MOSFET PB must be finite and positive");
    } else if (
      canonical === "MJ" &&
      (!Number.isFinite(value) || value < 0.0)
    ) {
      throw invalidElement(name, "MOSFET MJ must be finite and non-negative");
    } else if (
      canonical === "FC" &&
      (!Number.isFinite(value) || value < 0.0 || value >= 1.0)
    ) {
      throw invalidElement(name, "MOSFET FC must be finite and in [0, 1)");
    } else if (
      canonical === "MJSW" &&
      (!Number.isFinite(value) || value < 0.0)
    ) {
      throw invalidElement(name, "MOSFET MJSW must be finite and non-negative");
    } else if (
      canonical === "CJ" &&
      (!Number.isFinite(value) || value < 0.0)
    ) {
      throw invalidElement(name, "MOSFET CJ must be finite and non-negative");
    } else if (
      canonical === "CJSW" &&
      (!Number.isFinite(value) || value < 0.0)
    ) {
      throw invalidElement(name, "MOSFET CJSW must be finite and non-negative");
    } else if (
      canonical === "CBS" &&
      (!Number.isFinite(value) || value < 0.0)
    ) {
      throw invalidElement(name, "MOSFET CBS must be finite and non-negative");
    } else if (
      canonical === "CBD" &&
      (!Number.isFinite(value) || value < 0.0)
    ) {
      throw invalidElement(name, "MOSFET CBD must be finite and non-negative");
    } else if (
      canonical === "CGSO" &&
      (!Number.isFinite(value) || value < 0.0)
    ) {
      throw invalidElement(name, "MOSFET CGSO must be finite and non-negative");
    } else if (
      canonical === "CGDO" &&
      (!Number.isFinite(value) || value < 0.0)
    ) {
      throw invalidElement(name, "MOSFET CGDO must be finite and non-negative");
    } else if (
      canonical === "CGBO" &&
      (!Number.isFinite(value) || value < 0.0)
    ) {
      throw invalidElement(name, "MOSFET CGBO must be finite and non-negative");
    } else {
      normalized[canonical] = value;
    }
  }
  return { name, kind, parameters: normalized, unsupportedParameters: unsupported };
}

export function modelCardUnsupportedParameterIssues(
  model: NormalizedModelCard,
): readonly ModelCardUnsupportedParameterIssue[] {
  return model.unsupportedParameters.map((parameter) => ({
    modelName: model.name,
    kind: model.kind,
    parameter,
    message: `unsupported ${model.kind} model-card parameter ${parameter}`,
  }));
}

export function formatModelCardUnsupportedParameterIssueTable(
  model: NormalizedModelCard,
): string {
  return [
    "model_name\tkind\tparameter\tmessage",
    ...modelCardUnsupportedParameterIssues(model).map((issue) =>
      [issue.modelName, issue.kind, issue.parameter, issue.message].join("\t"),
    ),
  ].join("\n");
}

export function modelCardUnsupportedParameterIssueRecords(
  model: NormalizedModelCard,
): Array<Record<string, string>> {
  return deckTableRecords(formatModelCardUnsupportedParameterIssueTable(model));
}

export function formatModelCardUnsupportedParameterIssueCsv(
  model: NormalizedModelCard,
): string {
  return formatDeckTableCsv(formatModelCardUnsupportedParameterIssueTable(model));
}

export function formatModelCardUnsupportedParameterIssueJson(
  model: NormalizedModelCard,
): string {
  return formatDeckTableJson(formatModelCardUnsupportedParameterIssueTable(model));
}

export function modelCardSupportedParameterCoverage(): readonly ModelCardSupportedParameterCoverage[] {
  const rows: ModelCardSupportedParameterCoverage[] = [];
  for (const kind of MODEL_CARD_SUPPORTED_PARAMETER_COVERAGE_KINDS) {
    const grouped: Array<{ canonicalParameter: string; acceptedNames: string[] }> = [];
    for (const [acceptedName, canonical] of Object.entries(parameterAliases(kind))) {
      const existing = grouped.find((entry) => entry.canonicalParameter === canonical);
      if (existing === undefined) {
        grouped.push({ canonicalParameter: canonical, acceptedNames: [acceptedName] });
      } else {
        existing.acceptedNames.push(acceptedName);
      }
    }
    rows.push(
      ...grouped.map((entry) => ({
        kind,
        canonicalParameter: entry.canonicalParameter,
        acceptedNames: entry.acceptedNames,
        aliasCount: entry.acceptedNames.length,
      })),
    );
  }
  return rows;
}

export function formatModelCardSupportedParameterCoverageTable(): string {
  return [
    "kind\tcanonical_parameter\taccepted_names\talias_count",
    ...modelCardSupportedParameterCoverage().map((row) =>
      [row.kind, row.canonicalParameter, row.acceptedNames.join("|"), row.aliasCount].join("\t"),
    ),
  ].join("\n");
}

export function modelCardSupportedParameterCoverageRecords(): Array<Record<string, string>> {
  return deckTableRecords(formatModelCardSupportedParameterCoverageTable());
}

export function formatModelCardSupportedParameterCoverageCsv(): string {
  return formatDeckTableCsv(formatModelCardSupportedParameterCoverageTable());
}

export function formatModelCardSupportedParameterCoverageJson(): string {
  return formatDeckTableJson(formatModelCardSupportedParameterCoverageTable());
}

function modelCardSupportedParameterCoverageSummaryFrom(
  coverage: readonly ModelCardSupportedParameterCoverage[],
): readonly ModelCardSupportedParameterCoverageSummary[] {
  return MODEL_CARD_SUPPORTED_PARAMETER_COVERAGE_KINDS.map((kind) => {
    const rows = coverage.filter((row) => row.kind === kind);
    const aliasedParameters = rows
      .filter((row) => row.aliasCount > 1)
      .map((row) => row.canonicalParameter);
    return {
      kind,
      canonicalParameterCount: rows.length,
      acceptedNameCount: rows.reduce((total, row) => total + row.aliasCount, 0),
      aliasedParameterCount: aliasedParameters.length,
      maxAliasCount: rows.reduce((maximum, row) => Math.max(maximum, row.aliasCount), 0),
      aliasedParameters,
    };
  });
}

export function modelCardSupportedParameterCoverageSummary(): readonly ModelCardSupportedParameterCoverageSummary[] {
  return modelCardSupportedParameterCoverageSummaryFrom(modelCardSupportedParameterCoverage());
}

export function formatModelCardSupportedParameterCoverageSummaryTable(): string {
  return [
    "kind\tcanonical_parameter_count\taccepted_name_count\taliased_parameter_count\tmax_alias_count\taliased_parameters",
    ...modelCardSupportedParameterCoverageSummary().map((row) =>
      [
        row.kind,
        row.canonicalParameterCount,
        row.acceptedNameCount,
        row.aliasedParameterCount,
        row.maxAliasCount,
        row.aliasedParameters.join("|"),
      ].join("\t"),
    ),
  ].join("\n");
}

export function modelCardSupportedParameterCoverageSummaryRecords(): Array<Record<string, string>> {
  return deckTableRecords(formatModelCardSupportedParameterCoverageSummaryTable());
}

export function formatModelCardSupportedParameterCoverageSummaryCsv(): string {
  return formatDeckTableCsv(formatModelCardSupportedParameterCoverageSummaryTable());
}

export function formatModelCardSupportedParameterCoverageSummaryJson(): string {
  return formatDeckTableJson(formatModelCardSupportedParameterCoverageSummaryTable());
}

export function modelCardSupportedParameterCoverageGate(
  coverage: readonly ModelCardSupportedParameterCoverage[] = modelCardSupportedParameterCoverage(),
): ModelCardSupportedParameterCoverageGateReport {
  const issues: ModelCardSupportedParameterCoverageGateIssue[] = [];
  const actualKinds: string[] = [];
  for (const row of coverage) {
    if (!actualKinds.includes(row.kind)) {
      actualKinds.push(row.kind);
    }
  }
  if (actualKinds.join("\u0000") !== MODEL_CARD_SUPPORTED_PARAMETER_COVERAGE_KINDS.join("\u0000")) {
    issues.push({
      kind: "catalog",
      field: "kind_order",
      message: `expected model-card supported-parameter coverage kinds ${MODEL_CARD_SUPPORTED_PARAMETER_COVERAGE_KINDS.join(
        ",",
      )}, found ${actualKinds.join(",")}`,
    });
  }

  const summaries = modelCardSupportedParameterCoverageSummaryFrom(coverage);
  for (const kind of MODEL_CARD_SUPPORTED_PARAMETER_COVERAGE_KINDS) {
    const summary = summaries.find((candidate) => candidate.kind === kind)!;
    const [expectedCanonical, expectedAccepted, expectedAliased, expectedMaxAlias] =
      MODEL_CARD_SUPPORTED_PARAMETER_COVERAGE_EXPECTED_SUMMARIES[kind];
    if (summary.canonicalParameterCount !== expectedCanonical) {
      issues.push({
        kind,
        field: "canonical_parameter_count",
        message: `expected ${kind} to expose ${expectedCanonical} canonical supported parameters, found ${summary.canonicalParameterCount}`,
      });
    }
    if (summary.acceptedNameCount !== expectedAccepted) {
      issues.push({
        kind,
        field: "accepted_name_count",
        message: `expected ${kind} to expose ${expectedAccepted} accepted model-card names, found ${summary.acceptedNameCount}`,
      });
    }
    if (summary.aliasedParameterCount !== expectedAliased) {
      issues.push({
        kind,
        field: "aliased_parameter_count",
        message: `expected ${kind} to expose ${expectedAliased} alias-bearing parameters, found ${summary.aliasedParameterCount}`,
      });
    }
    if (summary.maxAliasCount !== expectedMaxAlias) {
      issues.push({
        kind,
        field: "max_alias_count",
        message: `expected ${kind} max alias count ${expectedMaxAlias}, found ${summary.maxAliasCount}`,
      });
    }
  }

  return {
    passed: issues.length === 0,
    kindCount: actualKinds.length,
    expectedKindCount: MODEL_CARD_SUPPORTED_PARAMETER_COVERAGE_KINDS.length,
    canonicalParameterCount: coverage.length,
    expectedCanonicalParameterCount: Object.values(
      MODEL_CARD_SUPPORTED_PARAMETER_COVERAGE_EXPECTED_SUMMARIES,
    ).reduce((total, [canonicalCount]) => total + canonicalCount, 0),
    acceptedNameCount: coverage.reduce((total, row) => total + row.aliasCount, 0),
    aliasedParameterCount: coverage.filter((row) => row.aliasCount > 1).length,
    maxAliasCount: coverage.reduce((maximum, row) => Math.max(maximum, row.aliasCount), 0),
    issues,
  };
}

export function formatModelCardSupportedParameterCoverageGateReport(
  report: ModelCardSupportedParameterCoverageGateReport = modelCardSupportedParameterCoverageGate(),
): string {
  const lines = [
    "passed\tkind_count\texpected_kind_count\tcanonical_parameter_count\texpected_canonical_parameter_count\taccepted_name_count\taliased_parameter_count\tmax_alias_count\tissue_count",
    [
      String(report.passed),
      report.kindCount,
      report.expectedKindCount,
      report.canonicalParameterCount,
      report.expectedCanonicalParameterCount,
      report.acceptedNameCount,
      report.aliasedParameterCount,
      report.maxAliasCount,
      report.issues.length,
    ].join("\t"),
  ];
  if (report.issues.length > 0) {
    lines.push("kind\tfield\tmessage");
    for (const issue of report.issues) {
      lines.push([issue.kind, issue.field, issue.message].join("\t"));
    }
  }
  return lines.join("\n");
}

export function formatModelCardSupportedParameterCoverageGateIssueTable(
  report: ModelCardSupportedParameterCoverageGateReport = modelCardSupportedParameterCoverageGate(),
): string {
  return [
    "kind\tfield\tmessage",
    ...report.issues.map((issue) => [issue.kind, issue.field, issue.message].join("\t")),
  ].join("\n");
}

export function modelCardSupportedParameterCoverageGateIssueRecords(
  report: ModelCardSupportedParameterCoverageGateReport = modelCardSupportedParameterCoverageGate(),
): Array<Record<string, string>> {
  return deckTableRecords(formatModelCardSupportedParameterCoverageGateIssueTable(report));
}

export function formatModelCardSupportedParameterCoverageGateIssueCsv(
  report: ModelCardSupportedParameterCoverageGateReport = modelCardSupportedParameterCoverageGate(),
): string {
  return formatDeckTableCsv(formatModelCardSupportedParameterCoverageGateIssueTable(report));
}

export function formatModelCardSupportedParameterCoverageGateIssueJson(
  report: ModelCardSupportedParameterCoverageGateReport = modelCardSupportedParameterCoverageGate(),
): string {
  return formatDeckTableJson(formatModelCardSupportedParameterCoverageGateIssueTable(report));
}

export function modelCardSupportedParameterCoverageDashboard(
  coverage: readonly ModelCardSupportedParameterCoverage[] = modelCardSupportedParameterCoverage(),
): readonly ModelCardSupportedParameterCoverageDashboardRow[] {
  const summaries = modelCardSupportedParameterCoverageSummaryFrom(coverage);
  const report = modelCardSupportedParameterCoverageGate(coverage);
  const globalIssueFields: string[] = [];
  for (const issue of report.issues.filter((candidate) => candidate.kind === "catalog")) {
    if (!globalIssueFields.includes(issue.field)) {
      globalIssueFields.push(issue.field);
    }
  }
  return MODEL_CARD_SUPPORTED_PARAMETER_COVERAGE_KINDS.map((kind) => {
    const summary = summaries.find((candidate) => candidate.kind === kind)!;
    const [expectedCanonical, expectedAccepted, expectedAliased, expectedMaxAlias] =
      MODEL_CARD_SUPPORTED_PARAMETER_COVERAGE_EXPECTED_SUMMARIES[kind];
    const issueFields = [...globalIssueFields];
    for (const issue of report.issues.filter((candidate) => candidate.kind === kind)) {
      if (!issueFields.includes(issue.field)) {
        issueFields.push(issue.field);
      }
    }
    return {
      kind,
      passed: issueFields.length === 0,
      canonicalParameterCount: summary.canonicalParameterCount,
      expectedCanonicalParameterCount: expectedCanonical,
      acceptedNameCount: summary.acceptedNameCount,
      expectedAcceptedNameCount: expectedAccepted,
      aliasedParameterCount: summary.aliasedParameterCount,
      expectedAliasedParameterCount: expectedAliased,
      maxAliasCount: summary.maxAliasCount,
      expectedMaxAliasCount: expectedMaxAlias,
      issueCount: issueFields.length,
      issueFields,
    };
  });
}

export function formatModelCardSupportedParameterCoverageDashboardTable(
  coverage: readonly ModelCardSupportedParameterCoverage[] = modelCardSupportedParameterCoverage(),
): string {
  return [
    "kind\tpassed\tcanonical_parameter_count\texpected_canonical_parameter_count\taccepted_name_count\texpected_accepted_name_count\taliased_parameter_count\texpected_aliased_parameter_count\tmax_alias_count\texpected_max_alias_count\tissue_count\tissue_fields",
    ...modelCardSupportedParameterCoverageDashboard(coverage).map((row) =>
      [
        row.kind,
        String(row.passed),
        row.canonicalParameterCount,
        row.expectedCanonicalParameterCount,
        row.acceptedNameCount,
        row.expectedAcceptedNameCount,
        row.aliasedParameterCount,
        row.expectedAliasedParameterCount,
        row.maxAliasCount,
        row.expectedMaxAliasCount,
        row.issueCount,
        row.issueFields.join("|"),
      ].join("\t"),
    ),
  ].join("\n");
}

export function modelCardSupportedParameterCoverageDashboardRecords(
  coverage: readonly ModelCardSupportedParameterCoverage[] = modelCardSupportedParameterCoverage(),
): Array<Record<string, string>> {
  return deckTableRecords(formatModelCardSupportedParameterCoverageDashboardTable(coverage));
}

export function formatModelCardSupportedParameterCoverageDashboardCsv(
  coverage: readonly ModelCardSupportedParameterCoverage[] = modelCardSupportedParameterCoverage(),
): string {
  return formatDeckTableCsv(formatModelCardSupportedParameterCoverageDashboardTable(coverage));
}

export function formatModelCardSupportedParameterCoverageDashboardJson(
  coverage: readonly ModelCardSupportedParameterCoverage[] = modelCardSupportedParameterCoverage(),
): string {
  return formatDeckTableJson(formatModelCardSupportedParameterCoverageDashboardTable(coverage));
}

export function diodeFromModelCard(
  name: string,
  anode: string,
  cathode: string,
  model: NormalizedModelCard,
): Diode {
  if (model.kind !== "D") {
    throw invalidElement(name, `expected diode model card, got ${model.kind}`);
  }
  const p = model.parameters;
  return diode(
    name,
    anode,
    cathode,
    p.IS ?? 1.0e-15,
    p.VT ?? 0.02585,
    p.N ?? 1.0,
    p.BV,
    p.IBV ?? 1.0e-3,
    p.CJO ?? 0.0,
    p.TT ?? 0.0,
    p.VJ ?? 1.0,
    p.M ?? 0.5,
    p.FC ?? 0.5,
    p.XTI ?? 3.0,
    p.EG ?? 1.11,
    p.RS ?? 0.0,
    p.KF ?? 0.0,
    p.AF ?? 1.0,
  );
}

export function bjtFromModelCard(
  name: string,
  collector: string,
  base: string,
  emitter: string,
  model: NormalizedModelCard,
): Bjt {
  if (model.kind !== "NPN" && model.kind !== "PNP") {
    throw invalidElement(name, `expected BJT model card, got ${model.kind}`);
  }
  const p = model.parameters;
  const saturationCurrent = p.IS ?? 1.0e-14;
  return bjt(
    name,
    collector,
    base,
    emitter,
    model.kind,
    saturationCurrent,
    p.BF ?? 100.0,
    p.VT ?? 0.02585,
    p.CJE ?? 0.0,
    p.CJC ?? 0.0,
    p.TF ?? 0.0,
    p.TR ?? 0.0,
    p.XTI ?? 3.0,
    p.EG ?? 1.11,
    p.VAF ?? 0.0,
    p.NF ?? 1.0,
    p.NR ?? 1.0,
    p.VJE ?? 0.75,
    p.MJE ?? 0.33,
    p.VJC ?? 0.75,
    p.MJC ?? 0.33,
    p.FC ?? 0.5,
    p.VAR ?? 0.0,
    p.IKF ?? 0.0,
    p.ISE ?? (p.C2 ?? 0.0) * saturationCurrent,
    p.NE ?? 1.0,
    p.ISC ?? (p.C4 ?? 0.0) * saturationCurrent,
    p.NC ?? 2.0,
    p.XTB ?? 0.0,
    p.BR ?? 1.0,
    p.IKR ?? 0.0,
    p.TNOM !== undefined ? p.TNOM + 273.15 : undefined,
    p.KF ?? 0.0,
    p.AF ?? 1.0,
    p.PTF ?? 0.0,
    p.XTF ?? 0.0,
    p.ITF ?? 0.0,
    p.VTF ?? 0.0,
    p.RE ?? 0.0,
    p.RC ?? 0.0,
    p.RB ?? 0.0,
    p.RBM,
    p.IRB ?? 0.0,
    p.XCJC ?? 1.0,
  );
}

export function jfetFromModelCard(
  name: string,
  drain: string,
  gate: string,
  source: string,
  model: NormalizedModelCard,
): Jfet {
  if (model.kind !== "NJF" && model.kind !== "PJF") {
    throw invalidElement(name, `expected JFET model card, got ${model.kind}`);
  }
  const p = model.parameters;
  return jfet(
    name,
    drain,
    gate,
    source,
    model.kind,
    p.BETA ?? 1.0e-4,
    p.VTO ?? (model.kind === "NJF" ? -2.0 : 2.0),
    p.LAMBDA ?? 0.0,
    p.CGS ?? 0.0,
    p.CGD ?? 0.0,
    p.KF ?? 0.0,
    p.AF ?? 1.0,
    p.PB ?? 1.0,
    p.FC ?? 0.5,
    p.IS ?? 1.0e-14,
    p.XTI ?? 3.0,
    p.EG ?? 1.11,
    p.B ?? 1.0,
    p.NLEV ?? 1.0,
    p.GDSNOI ?? 1.0,
    p.RD ?? 0.0,
    p.RS ?? 0.0,
    p.TCV ?? 0.0,
    p.VTOTC,
    p.TNOM !== undefined ? p.TNOM + 273.15 : undefined,
    p.BEX ?? 0.0,
    p.BETATCE,
  );
}

export function mosfetFromModelCard(
  name: string,
  drain: string,
  gate: string,
  source: string,
  body: string,
  model: NormalizedModelCard,
): Mosfet {
  if (model.kind !== "NMOS" && model.kind !== "PMOS") {
    throw invalidElement(name, `expected MOSFET model card, got ${model.kind}`);
  }
  const p = model.parameters;
  const surfaceMobility = p.U0 ?? 600.0;
  const transconductance =
    p.KP ??
    (p.TOX !== undefined && p.TOX > 0.0
      ? surfaceMobility * 1.0e-4 * OXIDE_PERMITTIVITY / p.TOX
      : undefined);
  let surfacePotential = p.PHI;
  let bodyEffectCoefficient = p.GAMMA;
  if (p.N_SUB !== undefined && p.TOX !== undefined) {
    const substrateDopingPerCubicMeter =
      p.N_SUB * CUBIC_CENTIMETERS_PER_CUBIC_METER;
    if (substrateDopingPerCubicMeter <= INTRINSIC_CARRIER_DENSITY_PER_CUBIC_METER) {
      throw invalidElement(name, "MOSFET NSUB must exceed the intrinsic carrier density");
    }
    if (p.TOX > 0.0) {
      if (surfacePotential === undefined) {
        const nominalTemperature = p.T_NOM ?? 300.15;
        const thermalVoltage = BOLTZMANN * nominalTemperature / ELECTRON_CHARGE;
        surfacePotential = Math.max(
          0.1,
          2.0 *
            thermalVoltage *
            Math.log(
              substrateDopingPerCubicMeter /
                INTRINSIC_CARRIER_DENSITY_PER_CUBIC_METER,
            ),
        );
      }
      if (bodyEffectCoefficient === undefined) {
        const oxideCapacitance = OXIDE_PERMITTIVITY / p.TOX;
        bodyEffectCoefficient =
          Math.sqrt(
            2.0 *
              SILICON_PERMITTIVITY *
              ELECTRON_CHARGE *
              substrateDopingPerCubicMeter,
          ) / oxideCapacitance;
      }
    }
  }
  const nominalTemperature = p.T_NOM ?? 300.15;
  const polarity = model.kind === "NMOS" ? 1.0 : -1.0;
  const bandGap = siliconBandGapElectronVolts(nominalTemperature);
  const gateType = p.TPG ?? 1.0;
  const substrateFermiPotential = polarity * 0.5 * (surfacePotential ?? 0.0);
  const gateWorkFunction =
    gateType === 0.0
      ? 3.2
      : 3.25 + 0.5 * bandGap - polarity * gateType * 0.5 * bandGap;
  const gateSubstrateWorkFunction =
    gateWorkFunction - (3.25 + 0.5 * bandGap + substrateFermiPotential);
  const surfaceStateShift =
    p.TOX !== undefined && p.TOX > 0.0
      ? ((p.NSS ?? 0.0) * 1.0e4 * ELECTRON_CHARGE) /
        (OXIDE_PERMITTIVITY / p.TOX)
      : 0.0;
  const thresholdVoltage =
    p.VT0 ??
    (p.N_SUB !== undefined && p.TOX !== undefined && p.TOX > 0.0
      ? gateSubstrateWorkFunction -
        surfaceStateShift +
        polarity *
          ((bodyEffectCoefficient ?? 0.0) * Math.sqrt(surfacePotential ?? 0.0) +
            (surfacePotential ?? 0.0))
      : undefined);
  const params: Partial<MosfetLevel1Params> = {
    ...(thresholdVoltage !== undefined ? { VT0: thresholdVoltage } : {}),
    ...(transconductance !== undefined ? { KP: transconductance } : {}),
    ...(p.LAMBDA !== undefined ? { LAMBDA: p.LAMBDA } : {}),
    ...(bodyEffectCoefficient !== undefined ? { GAMMA: bodyEffectCoefficient } : {}),
    ...(surfacePotential !== undefined ? { PHI: surfacePotential } : {}),
    ...(p.W !== undefined ? { W: p.W } : {}),
    ...(p.L !== undefined ? { L: p.L } : {}),
    ...(p.LD !== undefined ? { LD: p.LD } : {}),
    ...(p.TOX !== undefined ? { TOX: p.TOX } : {}),
    U0: surfaceMobility,
    ...(p.RD !== undefined ? { RD: p.RD } : {}),
    ...(p.RS !== undefined ? { RS: p.RS } : {}),
    ...(p.RSH !== undefined ? { RSH: p.RSH } : {}),
    ...(p.IS !== undefined ? { IS: p.IS } : {}),
    ...(p.JS !== undefined ? { JS: p.JS } : {}),
    ...(p.N_SUB !== undefined ? { N_SUB: p.N_SUB } : {}),
    ...(p.T_NOM !== undefined ? { T_NOM: p.T_NOM } : {}),
    ...(p.CGSO !== undefined ? { CGSO: p.CGSO } : {}),
    ...(p.CGDO !== undefined ? { CGDO: p.CGDO } : {}),
    ...(p.CGBO !== undefined ? { CGBO: p.CGBO } : {}),
    ...(p.CBS !== undefined ? { CBS: p.CBS } : {}),
    ...(p.CBD !== undefined ? { CBD: p.CBD } : {}),
    ...(p.CJ !== undefined ? { CJ: p.CJ } : {}),
    ...(p.CJSW !== undefined ? { CJSW: p.CJSW } : {}),
    ...(p.PB !== undefined ? { PB: p.PB } : {}),
    ...(p.MJ !== undefined ? { MJ: p.MJ } : {}),
    ...(p.MJSW !== undefined ? { MJSW: p.MJSW } : {}),
    ...(p.FC !== undefined ? { FC: p.FC } : {}),
    ...(p.KF !== undefined ? { KF: p.KF } : {}),
    ...(p.AF !== undefined ? { AF: p.AF } : {}),
  };
  return mosfet(name, drain, gate, source, body, model.kind, params);
}

export function deviceModelAuditFixtures(): readonly NormalizedModelCard[] {
  return [
    normalizeModelCard("Dfast", "diode", { JS: 2.0e-14, CJ: 1.5e-12, TT: 4.0e-9 }),
    normalizeModelCard("Qsmall", "npn", { BETA: 125.0, CBE: 2.0e-12, TF: 1.0e-10 }),
    normalizeModelCard("Jn", "njfet", { BET: 9.0e-4, VT0: -1.8, LAM: 0.02 }),
    normalizeModelCard("Mn", "nmos", {
      LEVEL: 1.0,
      VTO: 0.55,
      LAM: 0.04,
      NSUB: 1.6,
      CJD: 3.0e-13,
    }),
  ];
}

function modelCardByName(models: readonly NormalizedModelCard[]): Map<string, NormalizedModelCard> {
  return new Map(models.map((model) => [model.name, model]));
}

function requiredModel(
  models: ReadonlyMap<string, NormalizedModelCard>,
  name: string,
  helperName = "deviceModelBehaviorAuditFixtures",
): NormalizedModelCard {
  const model = models.get(name);
  if (model === undefined) {
    throw invalidElement(helperName, `missing ${name} model fixture`);
  }
  return model;
}

export function deviceModelBehaviorAuditFixtures(): readonly DeviceModelBehaviorFixture[] {
  const models = modelCardByName(deviceModelAuditFixtures());

  const diodeModel = requiredModel(models, "Dfast");
  const diodeCircuit = new Circuit();
  diodeCircuit.add(voltageSource("Vbias", "vin", "0", 0.8));
  diodeCircuit.add(resistor("Rlimit", "vin", "out", 1_000.0));
  diodeCircuit.add(diodeFromModelCard("D1", "out", "0", diodeModel));

  const bjtModel = requiredModel(models, "Qsmall");
  const bjtCircuit = new Circuit();
  bjtCircuit.add(voltageSource("Vcc", "vcc", "0", 5.0));
  bjtCircuit.add(voltageSource("Vbase", "base", "0", 0.72));
  bjtCircuit.add(resistor("Rload", "out", "0", 1_000.0));
  bjtCircuit.add(bjtFromModelCard("Q1", "vcc", "base", "out", bjtModel));

  const jfetModel = requiredModel(models, "Jn");
  const jfetCircuit = new Circuit();
  jfetCircuit.add(voltageSource("Vdd", "vdd", "0", 10.0));
  jfetCircuit.add(voltageSource("Vg", "gate", "0", 0.0));
  jfetCircuit.add(resistor("Rd", "vdd", "drain", 2_000.0));
  jfetCircuit.add(resistor("Rs", "source", "0", 1_000.0));
  jfetCircuit.add(jfetFromModelCard("J1", "drain", "gate", "source", jfetModel));

  const mosModel = requiredModel(models, "Mn");
  const mosCircuit = new Circuit();
  mosCircuit.add(voltageSource("Vdd", "vdd", "0", 1.8));
  mosCircuit.add(voltageSource("Vgate", "gate", "0", 1.8));
  mosCircuit.add(resistor("Rload", "vdd", "out", 1_000.0));
  mosCircuit.add(mosfetFromModelCard("M1", "out", "gate", "0", "0", mosModel));

  return [
    {
      name: "diode-forward-bias",
      kind: diodeModel.kind,
      model: diodeModel,
      circuit: diodeCircuit,
      probeNode: "out",
      expectedMin: 0.55,
      expectedMax: 0.65,
      deckLines: [
        "* device-model behavior fixture: diode-forward-bias",
        ".model Dfast D(IS=2e-14 CJO=1.5e-12 TT=4e-9)",
        "Vbias vin 0 0.8",
        "Rlimit vin out 1k",
        "D1 out 0 Dfast",
        ".op",
        ".save V(out)",
        ".end",
      ],
    },
    {
      name: "bjt-emitter-follower",
      kind: bjtModel.kind,
      model: bjtModel,
      circuit: bjtCircuit,
      probeNode: "out",
      expectedMin: 0.08,
      expectedMax: 0.18,
      deckLines: [
        "* device-model behavior fixture: bjt-emitter-follower",
        ".model Qsmall NPN(BF=125 CJE=2e-12 TF=1e-10)",
        "Vcc vcc 0 5",
        "Vbase base 0 0.72",
        "Q1 vcc base out Qsmall",
        "Rload out 0 1k",
        ".op",
        ".save V(out)",
        ".end",
      ],
    },
    {
      name: "jfet-source-bias",
      kind: jfetModel.kind,
      model: jfetModel,
      circuit: jfetCircuit,
      probeNode: "source",
      expectedMin: 0.80,
      expectedMax: 0.95,
      deckLines: [
        "* device-model behavior fixture: jfet-source-bias",
        ".model Jn NJF(BETA=9e-4 VTO=-1.8 LAMBDA=0.02)",
        "Vdd vdd 0 10",
        "Vg gate 0 0",
        "Rd vdd drain 2k",
        "Rs source 0 1k",
        "J1 drain gate source Jn",
        ".op",
        ".save V(source)",
        ".end",
      ],
    },
    {
      name: "mos-level1-common-source",
      kind: mosModel.kind,
      model: mosModel,
      circuit: mosCircuit,
      probeNode: "out",
      expectedMin: 0.55,
      expectedMax: 0.85,
      deckLines: [
        "* device-model behavior fixture: mos-level1-common-source",
        ".model Mn NMOS(LEVEL=1 VTO=0.55 LAMBDA=0.04 NSUB=1.6 CBD=3e-13)",
        "Vdd vdd 0 1.8",
        "Vgate gate 0 1.8",
        "Rload vdd out 1k",
        "M1 out gate 0 0 Mn",
        ".op",
        ".save V(out)",
        ".end",
      ],
    },
  ];
}

function deviceModelTemperaturePoints(name: string): readonly DeviceModelTemperaturePoint[] {
  const windows: Record<string, readonly [number, number, number][]> = {
    "diode-forward-bias": [
      [260.15, 0.63, 0.70],
      [300.15, 0.55, 0.65],
      [340.15, 0.49, 0.56],
    ],
    "bjt-emitter-follower": [
      [260.15, 0.03, 0.09],
      [300.15, 0.08, 0.18],
      [340.15, 0.15, 0.22],
    ],
    "jfet-source-bias": [
      [260.15, 0.86, 0.90],
      [300.15, 0.86, 0.90],
      [340.15, 0.86, 0.90],
    ],
    "mos-level1-common-source": [
      [260.15, 0.58, 0.68],
      [300.15, 0.55, 0.85],
      [340.15, 0.70, 0.82],
    ],
  };
  const fixtureWindows = windows[name];
  if (fixtureWindows === undefined) {
    throw invalidElement("deviceModelTemperatureAuditFixtures", `missing temperature windows for ${name}`);
  }
  return fixtureWindows.map(([temperatureKelvin, expectedMin, expectedMax]) => ({
    temperatureKelvin,
    expectedMin,
    expectedMax,
  }));
}

function deviceModelTemperatureBehavior(name: string): string {
  const behaviors: Record<string, string> = {
    "diode-forward-bias": "diode saturation current and thermal voltage scale with temperature",
    "bjt-emitter-follower": "BJT saturation current and thermal voltage scale with temperature",
    "jfet-source-bias": "JFET temperature scaling defaults to invariant; VTOTC overrides TCV for threshold-voltage scaling; BETATCE overrides BEX for beta scaling",
    "mos-level1-common-source": "Level-1 MOS threshold and transconductance scale with temperature",
  };
  const behavior = behaviors[name];
  if (behavior === undefined) {
    throw invalidElement("deviceModelTemperatureAuditFixtures", `missing temperature behavior for ${name}`);
  }
  return behavior;
}

function deviceModelTemperatureDeckLines(fixture: DeviceModelBehaviorFixture): readonly string[] {
  const lines = [...fixture.deckLines];
  lines[0] = `* device-model temperature fixture: ${fixture.name}`;
  const opIndex = lines.indexOf(".op");
  lines.splice(opIndex >= 0 ? opIndex : lines.length, 0, ".temp 260.15 300.15 340.15");
  return lines;
}

export function deviceModelTemperatureAuditFixtures(): readonly DeviceModelTemperatureBehaviorFixture[] {
  return deviceModelBehaviorAuditFixtures().map((fixture) => ({
    name: fixture.name,
    kind: fixture.kind,
    model: fixture.model,
    circuit: fixture.circuit,
    probeNode: fixture.probeNode,
    nominalTemperatureKelvin: 300.15,
    energyGapElectronVolts: 1.11,
    temperatureBehavior: deviceModelTemperatureBehavior(fixture.name),
    temperaturePoints: deviceModelTemperaturePoints(fixture.name),
    deckLines: deviceModelTemperatureDeckLines(fixture),
  }));
}

export function deviceModelCapacitanceAuditFixtures(): readonly DeviceModelCapacitanceBehaviorFixture[] {
  const helperName = "deviceModelCapacitanceAuditFixtures";
  const models = modelCardByName(deviceModelAuditFixtures());
  const frequencyHz = 100_000.0;

  const diodeModel = requiredModel(models, "Dfast", helperName);
  const diodeCircuit = new Circuit();
  diodeCircuit.add(voltageSourceWithAc("Vdrive", "in", "0", 0.0, 1.0, 0.0));
  diodeCircuit.add(resistor("Rin", "in", "out", 1_000_000.0));
  diodeCircuit.add(diodeFromModelCard("D1", "out", "0", diodeModel));

  const bjtModel = requiredModel(models, "Qsmall", helperName);
  const bjtCircuit = new Circuit();
  bjtCircuit.add(voltageSourceWithAc("Vdrive", "in", "0", 0.0, 1.0, 0.0));
  bjtCircuit.add(resistor("Rin", "in", "base", 1_000_000.0));
  bjtCircuit.add(resistor("Rc", "col", "0", 1_000.0));
  bjtCircuit.add(bjtFromModelCard("Q1", "col", "base", "0", bjtModel));

  const jfetModel = normalizeModelCard("Jn", "NJF", {
    BETA: 9.0e-4,
    VTO: -1.8,
    LAMBDA: 0.02,
    CGS: 2.0e-9,
    CGD: 1.0e-10,
  });
  const jfetCircuit = new Circuit();
  jfetCircuit.add(voltageSourceWithAc("Vdrive", "in", "0", 0.0, 1.0, 0.0));
  jfetCircuit.add(resistor("Rin", "in", "source", 1_000.0));
  jfetCircuit.add(resistor("Rd", "drain", "0", 2_000.0));
  jfetCircuit.add(voltageSource("Vgate", "gate", "0", 0.0));
  jfetCircuit.add(jfetFromModelCard("J1", "drain", "gate", "source", jfetModel));

  const mosModel = requiredModel(models, "Mn", helperName);
  const mosCircuit = new Circuit();
  mosCircuit.add(voltageSourceWithAc("Vdrive", "in", "0", 0.0, 1.0, 0.0));
  mosCircuit.add(resistor("Rin", "in", "drain", 5_000_000.0));
  mosCircuit.add(voltageSource("Vgate", "gate", "0", 0.0));
  mosCircuit.add(mosfetFromModelCard("M1", "drain", "gate", "0", "0", mosModel));

  return [
    {
      name: "diode-capacitance-ac",
      kind: diodeModel.kind,
      model: diodeModel,
      circuit: diodeCircuit,
      probeNode: "out",
      frequencyHz,
      expectedMagnitudeMin: 0.72,
      expectedMagnitudeMax: 0.74,
      capacitanceBehavior: "diode CJO and TT contribute high-frequency shunt capacitance",
      deckLines: [
        "* device-model capacitance fixture: diode-capacitance-ac",
        ".model Dfast D(IS=2e-14 CJO=1.5e-12 TT=4e-9)",
        "Vdrive in 0 0 AC 1",
        "Rin in out 1meg",
        "D1 out 0 Dfast",
        ".ac lin 1 100k 100k",
        ".save V(out)",
        ".end",
      ],
    },
    {
      name: "bjt-capacitance-ac",
      kind: bjtModel.kind,
      model: bjtModel,
      circuit: bjtCircuit,
      probeNode: "base",
      frequencyHz,
      expectedMagnitudeMin: 0.61,
      expectedMagnitudeMax: 0.64,
      capacitanceBehavior: "BJT CJE and TF contribute base-emitter AC capacitance",
      deckLines: [
        "* device-model capacitance fixture: bjt-capacitance-ac",
        ".model Qsmall NPN(BF=125 CJE=2e-12 TF=1e-10)",
        "Vdrive in 0 0 AC 1",
        "Rin in base 1meg",
        "Rc col 0 1k",
        "Q1 col base 0 Qsmall",
        ".ac lin 1 100k 100k",
        ".save V(base)",
        ".end",
      ],
    },
    {
      name: "jfet-capacitance-ac",
      kind: jfetModel.kind,
      model: jfetModel,
      circuit: jfetCircuit,
      probeNode: "source",
      frequencyHz,
      expectedMagnitudeMin: 0.50,
      expectedMagnitudeMax: 0.54,
      capacitanceBehavior: "JFET CGS/CGD contribute high-frequency gate-channel capacitance",
      deckLines: [
        "* device-model capacitance fixture: jfet-capacitance-ac",
        ".model Jn NJF(BETA=9e-4 VTO=-1.8 LAMBDA=0.02 CGS=2n CGD=100p)",
        "Vdrive in 0 0 AC 1",
        "Rin in source 1k",
        "Rd drain 0 2k",
        "Vgate gate 0 0",
        "J1 drain gate source Jn",
        ".ac lin 1 100k 100k",
        ".save V(source)",
        ".end",
      ],
    },
    {
      name: "mos-level1-capacitance-ac",
      kind: mosModel.kind,
      model: mosModel,
      circuit: mosCircuit,
      probeNode: "drain",
      frequencyHz,
      expectedMagnitudeMin: 0.72,
      expectedMagnitudeMax: 0.74,
      capacitanceBehavior: "Level-1 MOS CBD contributes drain-bulk AC capacitance",
      deckLines: [
        "* device-model capacitance fixture: mos-level1-capacitance-ac",
        ".model Mn NMOS(LEVEL=1 VTO=0.55 LAMBDA=0.04 NSUB=1.6 CBD=3e-13)",
        "Vdrive in 0 0 AC 1",
        "Rin in drain 5meg",
        "Vgate gate 0 0",
        "M1 drain gate 0 0 Mn",
        ".ac lin 1 100k 100k",
        ".save V(drain)",
        ".end",
      ],
    },
  ];
}

export function deviceModelNoiseAuditFixtures(): readonly DeviceModelNoiseBehaviorFixture[] {
  const helperName = "deviceModelNoiseAuditFixtures";
  const models = modelCardByName(deviceModelAuditFixtures());
  const frequencyHz = 1_000.0;

  const diodeModel = requiredModel(models, "Dfast", helperName);
  const diodeCircuit = new Circuit();
  diodeCircuit.add(voltageSource("Vbias", "vin", "0", 0.8));
  diodeCircuit.add(resistor("Rlimit", "vin", "out", 1_000.0));
  diodeCircuit.add(diodeFromModelCard("D1", "out", "0", diodeModel));

  const bjtModel = requiredModel(models, "Qsmall", helperName);
  const bjtCircuit = new Circuit();
  bjtCircuit.add(voltageSource("Vcc", "vcc", "0", 5.0));
  bjtCircuit.add(voltageSource("Vbase", "base", "0", 0.72));
  bjtCircuit.add(resistor("Rload", "out", "0", 1_000.0));
  bjtCircuit.add(bjtFromModelCard("Q1", "vcc", "base", "out", bjtModel));

  const jfetModel = requiredModel(models, "Jn", helperName);
  const jfetCircuit = new Circuit();
  jfetCircuit.add(voltageSource("Vdd", "vdd", "0", 10.0));
  jfetCircuit.add(voltageSource("Vg", "gate", "0", 0.0));
  jfetCircuit.add(resistor("Rd", "vdd", "drain", 2_000.0));
  jfetCircuit.add(resistor("Rs", "source", "0", 1_000.0));
  jfetCircuit.add(jfetFromModelCard("J1", "drain", "gate", "source", jfetModel));

  const mosModel = requiredModel(models, "Mn", helperName);
  const mosCircuit = new Circuit();
  mosCircuit.add(voltageSource("Vdd", "vdd", "0", 1.8));
  mosCircuit.add(voltageSource("Vgate", "gate", "0", 1.8));
  mosCircuit.add(resistor("Rload", "vdd", "out", 1_000.0));
  mosCircuit.add(mosfetFromModelCard("M1", "out", "gate", "0", "0", mosModel));

  return [
    {
      name: "diode-shot-noise",
      kind: diodeModel.kind,
      model: diodeModel,
      circuit: diodeCircuit,
      outputNode: "out",
      inputSource: "Vbias",
      frequencyHz,
      expectedNoiseElement: "D1",
      expectedNoiseType: "shot",
      expectedSourcePsdMin: 6.4e-23,
      expectedSourcePsdMax: 6.7e-23,
      expectedOutputPsdMin: 8.0e-19,
      expectedOutputPsdMax: 8.5e-19,
      noiseBehavior: "diode forward current contributes junction shot noise",
      deckLines: [
        "* device-model noise fixture: diode-shot-noise",
        ".model Dfast D(IS=2e-14 CJO=1.5e-12 TT=4e-9)",
        "Vbias vin 0 0.8",
        "Rlimit vin out 1k",
        "D1 out 0 Dfast",
        ".noise V(out) Vbias lin 1 1k 1k",
        ".save V(out)",
        ".end",
      ],
    },
    {
      name: "bjt-shot-noise",
      kind: bjtModel.kind,
      model: bjtModel,
      circuit: bjtCircuit,
      outputNode: "out",
      inputSource: "Vbase",
      frequencyHz,
      expectedNoiseElement: "Q1",
      expectedNoiseType: "shot",
      expectedSourcePsdMin: 3.7e-23,
      expectedSourcePsdMax: 3.9e-23,
      expectedOutputPsdMin: 1.1e-18,
      expectedOutputPsdMax: 1.3e-18,
      noiseBehavior: "BJT forward-active collector current contributes shot noise",
      deckLines: [
        "* device-model noise fixture: bjt-shot-noise",
        ".model Qsmall NPN(BF=125 CJE=2e-12 TF=1e-10)",
        "Vcc vcc 0 5",
        "Vbase base 0 0.72",
        "Q1 vcc base out Qsmall",
        "Rload out 0 1k",
        ".noise V(out) Vbase lin 1 1k 1k",
        ".save V(out)",
        ".end",
      ],
    },
    {
      name: "jfet-channel-noise",
      kind: jfetModel.kind,
      model: jfetModel,
      circuit: jfetCircuit,
      outputNode: "source",
      inputSource: "Vdd",
      frequencyHz,
      expectedNoiseElement: "J1",
      expectedNoiseType: "thermal",
      expectedSourcePsdMin: 2.0e-23,
      expectedSourcePsdMax: 2.2e-23,
      expectedOutputPsdMin: 2.3e-18,
      expectedOutputPsdMax: 2.5e-18,
      noiseBehavior: "JFET transconductance contributes long-channel channel thermal noise",
      deckLines: [
        "* device-model noise fixture: jfet-channel-noise",
        ".model Jn NJF(BETA=9e-4 VTO=-1.8 LAMBDA=0.02)",
        "Vdd vdd 0 10",
        "Vg gate 0 0",
        "Rd vdd drain 2k",
        "Rs source 0 1k",
        "J1 drain gate source Jn",
        ".noise V(source) Vdd lin 1 1k 1k",
        ".save V(source)",
        ".end",
      ],
    },
    {
      name: "mos-level1-channel-noise",
      kind: mosModel.kind,
      model: mosModel,
      circuit: mosCircuit,
      outputNode: "out",
      inputSource: "Vgate",
      frequencyHz,
      expectedNoiseElement: "M1",
      expectedNoiseType: "thermal",
      expectedSourcePsdMin: 1.3e-23,
      expectedSourcePsdMax: 1.4e-23,
      expectedOutputPsdMin: 3.3e-18,
      expectedOutputPsdMax: 3.5e-18,
      noiseBehavior: "Level-1 MOS gm contributes long-channel channel thermal noise",
      deckLines: [
        "* device-model noise fixture: mos-level1-channel-noise",
        ".model Mn NMOS(LEVEL=1 VTO=0.55 LAMBDA=0.04 NSUB=1.6 CBD=3e-13)",
        "Vdd vdd 0 1.8",
        "Vgate gate 0 1.8",
        "Rload vdd out 1k",
        "M1 out gate 0 0 Mn",
        ".noise V(out) Vgate lin 1 1k 1k",
        ".save V(out)",
        ".end",
      ],
    },
  ];
}

export function deviceModelChargeAuditFixtures(): readonly DeviceModelChargeBehaviorFixture[] {
  const helperName = "deviceModelChargeAuditFixtures";
  const models = modelCardByName(deviceModelAuditFixtures());
  const timeStepSeconds = 2.0e-8;
  const stopTimeSeconds = 2.0e-6;
  const storageCapacitanceFarads = 1.0e-10;

  const diodeModel = requiredModel(models, "Dfast", helperName);
  const diodeCircuit = new Circuit();
  diodeCircuit.add(voltageSource("Vbias", "vin", "0", 0.8));
  diodeCircuit.add(resistor("Rlimit", "vin", "out", 1_000.0));
  diodeCircuit.add(diodeFromModelCard("D1", "out", "0", diodeModel));
  diodeCircuit.add(capacitor("Cstore", "out", "0", storageCapacitanceFarads));

  const bjtModel = requiredModel(models, "Qsmall", helperName);
  const bjtCircuit = new Circuit();
  bjtCircuit.add(voltageSource("Vcc", "vcc", "0", 5.0));
  bjtCircuit.add(voltageSource("Vbase", "base", "0", 0.72));
  bjtCircuit.add(resistor("Rload", "out", "0", 1_000.0));
  bjtCircuit.add(bjtFromModelCard("Q1", "vcc", "base", "out", bjtModel));
  bjtCircuit.add(capacitor("Cstore", "out", "0", storageCapacitanceFarads));

  const jfetModel = normalizeModelCard("Jn", "NJF", {
    BETA: 9.0e-4,
    VTO: -1.8,
    LAMBDA: 0.02,
    CGS: 2.0e-11,
    CGD: 5.0e-12,
  });
  const jfetCircuit = new Circuit();
  jfetCircuit.add(voltageSource("Vdd", "vdd", "0", 10.0));
  jfetCircuit.add(voltageSource("Vg", "gate", "0", 0.0));
  jfetCircuit.add(resistor("Rd", "vdd", "drain", 2_000.0));
  jfetCircuit.add(resistor("Rs", "source", "0", 1_000.0));
  jfetCircuit.add(jfetFromModelCard("J1", "drain", "gate", "source", jfetModel));
  jfetCircuit.add(capacitor("Cstore", "source", "0", storageCapacitanceFarads));

  const mosModel = normalizeModelCard("Mn", "NMOS", {
    LEVEL: 1.0,
    VTO: 0.55,
    LAMBDA: 0.04,
    NSUB: 1.6,
    CGSO: 2.0e-11,
    CGDO: 5.0e-12,
    CGBO: 1.0e-12,
    CBS: 4.0e-13,
    CBD: 3.0e-13,
    PB: 0.9,
    MJ: 0.45,
  });
  const mosCircuit = new Circuit();
  mosCircuit.add(voltageSource("Vdd", "vdd", "0", 1.8));
  mosCircuit.add(voltageSource("Vgate", "gate", "0", 1.8));
  mosCircuit.add(resistor("Rload", "vdd", "out", 1_000.0));
  mosCircuit.add(mosfetFromModelCard("M1", "out", "gate", "0", "0", mosModel));
  mosCircuit.add(capacitor("Cstore", "out", "0", storageCapacitanceFarads));

  return [
    {
      name: "diode-storage-charge",
      kind: diodeModel.kind,
      model: diodeModel,
      circuit: diodeCircuit,
      probeNode: "out",
      timeStepSeconds,
      stopTimeSeconds,
      storageCapacitanceFarads,
      expectedInitialMin: -1.0e-9,
      expectedInitialMax: 1.0,
      expectedFinalMin: 0.58,
      expectedFinalMax: 0.61,
      chargeBehavior:
        "diode CJO/TT contribute transient anode-cathode storage; explicit Cstore keeps the fixture comparable with other charge audits",
      deckLines: [
        "* device-model charge fixture: diode-storage-charge",
        ".model Dfast D(IS=2e-14 CJO=1.5e-12 TT=4e-9)",
        "Vbias vin 0 0.8",
        "Rlimit vin out 1k",
        "D1 out 0 Dfast",
        "Cstore out 0 100p",
        ".tran 20n 2u",
        ".save V(out)",
        ".end",
      ],
    },
    {
      name: "bjt-storage-charge",
      kind: bjtModel.kind,
      model: bjtModel,
      circuit: bjtCircuit,
      probeNode: "out",
      timeStepSeconds,
      stopTimeSeconds,
      storageCapacitanceFarads,
      expectedInitialMin: -1.0e-9,
      expectedInitialMax: 1.0,
      expectedFinalMin: 0.10,
      expectedFinalMax: 0.14,
      chargeBehavior:
        "BJT CJE/CJC/TF/TR contribute transient base-emitter and base-collector storage; explicit Cstore keeps the fixture comparable with other charge audits",
      deckLines: [
        "* device-model charge fixture: bjt-storage-charge",
        ".model Qsmall NPN(BF=125 CJE=2e-12 TF=1e-10)",
        "Vcc vcc 0 5",
        "Vbase base 0 0.72",
        "Q1 vcc base out Qsmall",
        "Rload out 0 1k",
        "Cstore out 0 100p",
        ".tran 20n 2u",
        ".save V(out)",
        ".end",
      ],
    },
    {
      name: "jfet-storage-charge",
      kind: jfetModel.kind,
      model: jfetModel,
      circuit: jfetCircuit,
      probeNode: "source",
      timeStepSeconds,
      stopTimeSeconds,
      storageCapacitanceFarads,
      expectedInitialMin: -1.0e-9,
      expectedInitialMax: 1.0,
      expectedFinalMin: 0.86,
      expectedFinalMax: 0.90,
      chargeBehavior:
        "JFET CGS/CGD contribute transient gate-source and gate-drain storage; explicit Cstore keeps the fixture comparable with other charge audits",
      deckLines: [
        "* device-model charge fixture: jfet-storage-charge",
        ".model Jn NJF(BETA=9e-4 VTO=-1.8 LAMBDA=0.02 CGS=20p CGD=5p)",
        "Vdd vdd 0 10",
        "Vg gate 0 0",
        "Rd vdd drain 2k",
        "Rs source 0 1k",
        "J1 drain gate source Jn",
        "Cstore source 0 100p",
        ".tran 20n 2u",
        ".save V(source)",
        ".end",
      ],
    },
    {
      name: "mos-level1-storage-charge",
      kind: mosModel.kind,
      model: mosModel,
      circuit: mosCircuit,
      probeNode: "out",
      timeStepSeconds,
      stopTimeSeconds,
      storageCapacitanceFarads,
      expectedInitialMin: -1.0e-9,
      expectedInitialMax: 1.0,
      expectedFinalMin: 0.68,
      expectedFinalMax: 0.73,
      chargeBehavior:
        "Level-1 MOS CGSO/CGDO/CGBO plus CBS/CBD contribute transient gate-overlap and depletion-shaped bulk-junction storage; explicit Cstore keeps the fixture comparable with other charge audits",
      deckLines: [
        "* device-model charge fixture: mos-level1-storage-charge",
        ".model Mn NMOS(LEVEL=1 VTO=0.55 LAMBDA=0.04 NSUB=1.6 CGSO=20p CGDO=5p CGBO=1p CBS=4e-13 CBD=3e-13 PB=0.9 MJ=0.45)",
        "Vdd vdd 0 1.8",
        "Vgate gate 0 1.8",
        "Rload vdd out 1k",
        "M1 out gate 0 0 Mn",
        "Cstore out 0 100p",
        ".tran 20n 2u",
        ".save V(out)",
        ".end",
      ],
    },
  ];
}

export function deviceModelReferenceDeckAuditFixtures(): readonly DeviceModelReferenceDeckAuditFixture[] {
  const reference = "SPICE2/SPICE3-style local model-depth fixture";
  const rows: DeviceModelReferenceDeckAuditFixture[] = [];
  for (const fixture of deviceModelBehaviorAuditFixtures()) {
    rows.push({
      name: `${fixture.name}:op`,
      kind: fixture.kind,
      model: fixture.model,
      analysis: "op",
      reference,
      expectedBehavior: `DC probe ${fixture.probeNode} remains in [${fixture.expectedMin}, ${fixture.expectedMax}] V`,
      deckLines: fixture.deckLines,
    });
  }
  for (const fixture of deviceModelTemperatureAuditFixtures()) {
    rows.push({
      name: `${fixture.name}:temperature`,
      kind: fixture.kind,
      model: fixture.model,
      analysis: "temperature",
      reference,
      expectedBehavior: fixture.temperatureBehavior,
      deckLines: fixture.deckLines,
    });
  }
  for (const fixture of deviceModelCapacitanceAuditFixtures()) {
    rows.push({
      name: `${fixture.name}:ac`,
      kind: fixture.kind,
      model: fixture.model,
      analysis: "ac",
      reference,
      expectedBehavior: fixture.capacitanceBehavior,
      deckLines: fixture.deckLines,
    });
  }
  for (const fixture of deviceModelNoiseAuditFixtures()) {
    rows.push({
      name: `${fixture.name}:noise`,
      kind: fixture.kind,
      model: fixture.model,
      analysis: "noise",
      reference,
      expectedBehavior: fixture.noiseBehavior,
      deckLines: fixture.deckLines,
    });
  }
  for (const fixture of deviceModelChargeAuditFixtures()) {
    rows.push({
      name: `${fixture.name}:tran`,
      kind: fixture.kind,
      model: fixture.model,
      analysis: "tran",
      reference,
      expectedBehavior: fixture.chargeBehavior,
      deckLines: fixture.deckLines,
    });
  }
  return rows;
}

export function formatDeviceModelReferenceDeckAuditTable(
  fixtures: readonly DeviceModelReferenceDeckAuditFixture[] = deviceModelReferenceDeckAuditFixtures(),
): string {
  const lines = ["name\tkind\tanalysis\tmodel\treference\texpected_behavior\tdeck_lines"];
  for (const fixture of fixtures) {
    lines.push([
      fixture.name,
      fixture.kind,
      fixture.analysis,
      fixture.model.name,
      fixture.reference,
      fixture.expectedBehavior,
      fixture.deckLines.length.toString(),
    ].join("\t"));
  }
  return lines.join("\n");
}

export function deviceModelReferenceDeckAuditRecords(
  fixtures: readonly DeviceModelReferenceDeckAuditFixture[] = deviceModelReferenceDeckAuditFixtures(),
): Array<Record<string, string>> {
  return deckTableRecords(formatDeviceModelReferenceDeckAuditTable(fixtures));
}

export function formatDeviceModelReferenceDeckAuditCsv(
  fixtures: readonly DeviceModelReferenceDeckAuditFixture[] = deviceModelReferenceDeckAuditFixtures(),
): string {
  return formatDeckTableCsv(formatDeviceModelReferenceDeckAuditTable(fixtures));
}

export function formatDeviceModelReferenceDeckAuditJson(
  fixtures: readonly DeviceModelReferenceDeckAuditFixture[] = deviceModelReferenceDeckAuditFixtures(),
): string {
  return formatDeckTableJson(formatDeviceModelReferenceDeckAuditTable(fixtures));
}

export function deviceModelReferenceDeckAuditSummary(
  fixtures: readonly DeviceModelReferenceDeckAuditFixture[] = deviceModelReferenceDeckAuditFixtures(),
): readonly DeviceModelReferenceDeckAuditSummary[] {
  const expectedKinds = REFERENCE_DECK_AUDIT_EXPECTED_KINDS as readonly string[];
  const kinds = [...expectedKinds];
  for (const kind of [...new Set(fixtures.map((fixture) => fixture.kind))].sort()) {
    if (!kinds.includes(kind)) {
      kinds.push(kind);
    }
  }

  return kinds.map((kind) => {
    const rows = fixtures.filter((fixture) => fixture.kind === kind);
    const rowAnalyses = new Set(rows.map((fixture) => fixture.analysis));
    const analyses = [
      ...REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES.filter((analysis) => rowAnalyses.has(analysis)),
      ...[...rowAnalyses]
        .filter((analysis) => !(REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES as readonly string[]).includes(analysis))
        .sort(),
    ];
    const missingAnalyses = expectedKinds.includes(kind)
      ? REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES.filter((analysis) => !rowAnalyses.has(analysis))
      : [];
    const references: string[] = [];
    for (const fixture of rows) {
      if (fixture.reference.length > 0 && !references.includes(fixture.reference)) {
        references.push(fixture.reference);
      }
    }
    return {
      kind,
      fixtureCount: rows.length,
      analyses,
      missingAnalyses,
      deckLineCount: rows.reduce((total, fixture) => total + fixture.deckLines.length, 0),
      references,
    };
  });
}

export function formatDeviceModelReferenceDeckAuditSummaryTable(
  fixtures: readonly DeviceModelReferenceDeckAuditFixture[] = deviceModelReferenceDeckAuditFixtures(),
): string {
  const lines = ["kind\tfixture_count\tanalyses\tmissing_analyses\tdeck_lines\treferences"];
  for (const summary of deviceModelReferenceDeckAuditSummary(fixtures)) {
    lines.push([
      summary.kind,
      summary.fixtureCount.toString(),
      summary.analyses.join(","),
      summary.missingAnalyses.join(","),
      summary.deckLineCount.toString(),
      summary.references.join(","),
    ].join("\t"));
  }
  return lines.join("\n");
}

export function deviceModelReferenceDeckAuditSummaryRecords(
  fixtures: readonly DeviceModelReferenceDeckAuditFixture[] = deviceModelReferenceDeckAuditFixtures(),
): Array<Record<string, string>> {
  return deckTableRecords(formatDeviceModelReferenceDeckAuditSummaryTable(fixtures));
}

export function formatDeviceModelReferenceDeckAuditSummaryCsv(
  fixtures: readonly DeviceModelReferenceDeckAuditFixture[] = deviceModelReferenceDeckAuditFixtures(),
): string {
  return formatDeckTableCsv(formatDeviceModelReferenceDeckAuditSummaryTable(fixtures));
}

export function formatDeviceModelReferenceDeckAuditSummaryJson(
  fixtures: readonly DeviceModelReferenceDeckAuditFixture[] = deviceModelReferenceDeckAuditFixtures(),
): string {
  return formatDeckTableJson(formatDeviceModelReferenceDeckAuditSummaryTable(fixtures));
}

export function deviceModelReferenceDeckAuditAnalysisSummary(
  fixtures: readonly DeviceModelReferenceDeckAuditFixture[] = deviceModelReferenceDeckAuditFixtures(),
): readonly DeviceModelReferenceDeckAuditAnalysisSummary[] {
  const expectedKinds = REFERENCE_DECK_AUDIT_EXPECTED_KINDS as readonly string[];
  const expectedAnalyses = REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES as readonly string[];
  const analyses = [...expectedAnalyses];
  for (const analysis of [...new Set(fixtures.map((fixture) => fixture.analysis))].sort()) {
    if (!analyses.includes(analysis)) {
      analyses.push(analysis);
    }
  }

  return analyses.map((analysis) => {
    const rows = fixtures.filter((fixture) => fixture.analysis === analysis);
    const rowKinds = new Set(rows.map((fixture) => fixture.kind));
    const kinds = [
      ...REFERENCE_DECK_AUDIT_EXPECTED_KINDS.filter((kind) => rowKinds.has(kind)),
      ...[...rowKinds]
        .filter((kind) => !(REFERENCE_DECK_AUDIT_EXPECTED_KINDS as readonly string[]).includes(kind))
        .sort(),
    ];
    const missingKinds = expectedAnalyses.includes(analysis)
      ? REFERENCE_DECK_AUDIT_EXPECTED_KINDS.filter((kind) => !rowKinds.has(kind))
      : [];
    const references: string[] = [];
    for (const fixture of rows) {
      if (fixture.reference.length > 0 && !references.includes(fixture.reference)) {
        references.push(fixture.reference);
      }
    }
    return {
      analysis,
      fixtureCount: rows.length,
      kinds,
      missingKinds,
      deckLineCount: rows.reduce((total, fixture) => total + fixture.deckLines.length, 0),
      references,
    };
  });
}

export function formatDeviceModelReferenceDeckAuditAnalysisSummaryTable(
  fixtures: readonly DeviceModelReferenceDeckAuditFixture[] = deviceModelReferenceDeckAuditFixtures(),
): string {
  const lines = ["analysis\tfixture_count\tkinds\tmissing_kinds\tdeck_lines\treferences"];
  for (const summary of deviceModelReferenceDeckAuditAnalysisSummary(fixtures)) {
    lines.push([
      summary.analysis,
      summary.fixtureCount.toString(),
      summary.kinds.join(","),
      summary.missingKinds.join(","),
      summary.deckLineCount.toString(),
      summary.references.join(","),
    ].join("\t"));
  }
  return lines.join("\n");
}

export function deviceModelReferenceDeckAuditAnalysisSummaryRecords(
  fixtures: readonly DeviceModelReferenceDeckAuditFixture[] = deviceModelReferenceDeckAuditFixtures(),
): Array<Record<string, string>> {
  return deckTableRecords(formatDeviceModelReferenceDeckAuditAnalysisSummaryTable(fixtures));
}

export function formatDeviceModelReferenceDeckAuditAnalysisSummaryCsv(
  fixtures: readonly DeviceModelReferenceDeckAuditFixture[] = deviceModelReferenceDeckAuditFixtures(),
): string {
  return formatDeckTableCsv(formatDeviceModelReferenceDeckAuditAnalysisSummaryTable(fixtures));
}

export function formatDeviceModelReferenceDeckAuditAnalysisSummaryJson(
  fixtures: readonly DeviceModelReferenceDeckAuditFixture[] = deviceModelReferenceDeckAuditFixtures(),
): string {
  return formatDeckTableJson(formatDeviceModelReferenceDeckAuditAnalysisSummaryTable(fixtures));
}

export function deviceModelReferenceDeckAuditMatrix(
  fixtures: readonly DeviceModelReferenceDeckAuditFixture[] = deviceModelReferenceDeckAuditFixtures(),
): readonly DeviceModelReferenceDeckAuditMatrixRow[] {
  const expectedKinds = REFERENCE_DECK_AUDIT_EXPECTED_KINDS as readonly string[];
  const expectedAnalyses = REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES as readonly string[];
  const kinds = [...expectedKinds];
  for (const kind of [...new Set(fixtures.map((fixture) => fixture.kind))].sort()) {
    if (!kinds.includes(kind)) {
      kinds.push(kind);
    }
  }

  const namesFor = (
    rows: readonly DeviceModelReferenceDeckAuditFixture[],
    analysis: string,
  ): string => rows.filter((fixture) => fixture.analysis === analysis).map((fixture) => fixture.name).join(",");

  return kinds.map((kind) => {
    const rows = fixtures.filter((fixture) => fixture.kind === kind);
    const rowAnalyses = new Set(rows.map((fixture) => fixture.analysis));
    const missingAnalyses = expectedKinds.includes(kind)
      ? REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES.filter((analysis) => !rowAnalyses.has(analysis))
      : [];
    const extraAnalyses = [...rowAnalyses]
      .filter((analysis) => !expectedAnalyses.includes(analysis))
      .sort();

    return {
      kind,
      fixtureCount: rows.length,
      op: namesFor(rows, "op"),
      temperature: namesFor(rows, "temperature"),
      ac: namesFor(rows, "ac"),
      noise: namesFor(rows, "noise"),
      tran: namesFor(rows, "tran"),
      missingAnalyses,
      extraAnalyses,
      deckLineCount: rows.reduce((total, fixture) => total + fixture.deckLines.length, 0),
    };
  });
}

export function formatDeviceModelReferenceDeckAuditMatrixTable(
  fixtures: readonly DeviceModelReferenceDeckAuditFixture[] = deviceModelReferenceDeckAuditFixtures(),
): string {
  const lines = ["kind\tfixture_count\top\ttemperature\tac\tnoise\ttran\tmissing_analyses\textra_analyses\tdeck_lines"];
  for (const row of deviceModelReferenceDeckAuditMatrix(fixtures)) {
    lines.push([
      row.kind,
      row.fixtureCount.toString(),
      row.op,
      row.temperature,
      row.ac,
      row.noise,
      row.tran,
      row.missingAnalyses.join(","),
      row.extraAnalyses.join(","),
      row.deckLineCount.toString(),
    ].join("\t"));
  }
  return lines.join("\n");
}

export function deviceModelReferenceDeckAuditMatrixRecords(
  fixtures: readonly DeviceModelReferenceDeckAuditFixture[] = deviceModelReferenceDeckAuditFixtures(),
): Array<Record<string, string>> {
  return deckTableRecords(formatDeviceModelReferenceDeckAuditMatrixTable(fixtures));
}

export function formatDeviceModelReferenceDeckAuditMatrixCsv(
  fixtures: readonly DeviceModelReferenceDeckAuditFixture[] = deviceModelReferenceDeckAuditFixtures(),
): string {
  return formatDeckTableCsv(formatDeviceModelReferenceDeckAuditMatrixTable(fixtures));
}

export function formatDeviceModelReferenceDeckAuditMatrixJson(
  fixtures: readonly DeviceModelReferenceDeckAuditFixture[] = deviceModelReferenceDeckAuditFixtures(),
): string {
  return formatDeckTableJson(formatDeviceModelReferenceDeckAuditMatrixTable(fixtures));
}

export function deviceModelReferenceDeckAuditGate(
  fixtures: readonly DeviceModelReferenceDeckAuditFixture[] = deviceModelReferenceDeckAuditFixtures(),
): DeviceModelReferenceDeckAuditGateReport {
  const issues: DeviceModelReferenceDeckAuditIssue[] = [];
  const seenNames = new Set<string>();
  const seenPairs = new Set<string>();

  if (fixtures.length === 0) {
    issues.push({
      fixtureName: "audit_matrix",
      field: "fixture_count",
      message: "audit matrix must contain at least one reference-deck row",
    });
  }

  for (const fixture of fixtures) {
    const fixtureName = fixture.name.length === 0 ? "<missing>" : fixture.name;
    if (seenNames.has(fixture.name)) {
      issues.push({
        fixtureName,
        field: "name",
        message: "reference-deck audit fixture names must be unique",
      });
    }
    seenNames.add(fixture.name);
    if (fixture.name.trim().length === 0) {
      issues.push({
        fixtureName,
        field: "name",
        message: "field must be documented and non-empty",
      });
    }
    if (!REFERENCE_DECK_AUDIT_EXPECTED_KINDS.includes(fixture.kind)) {
      issues.push({
        fixtureName,
        field: "kind",
        message: `unsupported reference-deck audit kind ${JSON.stringify(fixture.kind)}`,
      });
    }
    if (
      !(REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES as readonly string[]).includes(fixture.analysis)
    ) {
      issues.push({
        fixtureName,
        field: "analysis",
        message: `unsupported reference-deck audit analysis ${JSON.stringify(fixture.analysis)}`,
      });
    }
    seenPairs.add(`${fixture.kind}:${fixture.analysis}`);
    if (fixture.model.name.trim().length === 0) {
      issues.push({
        fixtureName,
        field: "model.name",
        message: "field must be documented and non-empty",
      });
    }
    if (fixture.reference.trim().length === 0) {
      issues.push({
        fixtureName,
        field: "reference",
        message: "field must be documented and non-empty",
      });
    }
    if (fixture.expectedBehavior.trim().length === 0) {
      issues.push({
        fixtureName,
        field: "expected_behavior",
        message: "field must be documented and non-empty",
      });
    }
    if (fixture.deckLines.length === 0) {
      issues.push({
        fixtureName,
        field: "deck_lines",
        message: "reference deck must contain active deck lines",
      });
    } else {
      if (!fixture.deckLines[0]!.startsWith("* device-model ")) {
        issues.push({
          fixtureName,
          field: "deck_lines[0]",
          message: "reference deck must start with a device-model comment",
        });
      }
      if (!fixture.deckLines.some((line) => line.startsWith(".model "))) {
        issues.push({
          fixtureName,
          field: "deck_lines",
          message: "reference deck must include a .model card",
        });
      }
      if (fixture.deckLines.at(-1) !== ".end") {
        issues.push({
          fixtureName,
          field: "deck_lines[-1]",
          message: "reference deck must end with .end",
        });
      }
    }
  }

  for (const kind of REFERENCE_DECK_AUDIT_EXPECTED_KINDS) {
    for (const analysis of REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES) {
      if (!seenPairs.has(`${kind}:${analysis}`)) {
        issues.push({
          fixtureName: `${kind}:${analysis}`,
          field: "coverage",
          message: `missing required ${kind} ${analysis} reference-deck audit row`,
        });
      }
    }
  }

  return {
    passed: issues.length === 0,
    fixtureCount: fixtures.length,
    expectedKinds: REFERENCE_DECK_AUDIT_EXPECTED_KINDS,
    expectedAnalyses: REFERENCE_DECK_AUDIT_EXPECTED_ANALYSES,
    issues,
  };
}

export function formatDeviceModelReferenceDeckAuditGateReport(
  report: DeviceModelReferenceDeckAuditGateReport,
): string {
  const lines = [
    "passed\tfixture_count\texpected_kinds\texpected_analyses\tissue_count",
    [
      String(report.passed),
      report.fixtureCount.toString(),
      report.expectedKinds.join(","),
      report.expectedAnalyses.join(","),
      report.issues.length.toString(),
    ].join("\t"),
  ];
  if (report.issues.length > 0) {
    lines.push("fixture_name\tfield\tmessage");
    for (const issue of report.issues) {
      lines.push([issue.fixtureName, issue.field, issue.message].join("\t"));
    }
  }
  return lines.join("\n");
}

export function deviceModelReferenceDeckAuditGateCoverageDigest(
  report: DeviceModelReferenceDeckAuditGateReport = deviceModelReferenceDeckAuditGate(),
): DeviceModelReferenceDeckAuditGateCoverageDigest {
  const expectedPairCount = report.expectedKinds.length * report.expectedAnalyses.length;
  const missingPairCount = report.issues.filter((issue) => issue.field === "coverage").length;
  const issueFields = [...new Set(report.issues.map((issue) => issue.field))].sort();
  return {
    passed: report.passed,
    fixtureCount: report.fixtureCount,
    expectedPairCount,
    coveredPairCount: Math.max(expectedPairCount - missingPairCount, 0),
    missingPairCount,
    issueCount: report.issues.length,
    issueFields,
  };
}

export function formatDeviceModelReferenceDeckAuditGateCoverageDigestTable(
  report: DeviceModelReferenceDeckAuditGateReport = deviceModelReferenceDeckAuditGate(),
): string {
  const digest = deviceModelReferenceDeckAuditGateCoverageDigest(report);
  return [
    "passed\tfixture_count\texpected_pair_count\tcovered_pair_count\tmissing_pair_count\tissue_count\tissue_fields",
    [
      String(digest.passed),
      digest.fixtureCount.toString(),
      digest.expectedPairCount.toString(),
      digest.coveredPairCount.toString(),
      digest.missingPairCount.toString(),
      digest.issueCount.toString(),
      digest.issueFields.join(","),
    ].join("\t"),
  ].join("\n");
}

export function deviceModelReferenceDeckAuditGateCoverageDigestRecords(
  report: DeviceModelReferenceDeckAuditGateReport = deviceModelReferenceDeckAuditGate(),
): Array<Record<string, string>> {
  return deckTableRecords(formatDeviceModelReferenceDeckAuditGateCoverageDigestTable(report));
}

export function formatDeviceModelReferenceDeckAuditGateCoverageDigestCsv(
  report: DeviceModelReferenceDeckAuditGateReport = deviceModelReferenceDeckAuditGate(),
): string {
  return formatDeckTableCsv(formatDeviceModelReferenceDeckAuditGateCoverageDigestTable(report));
}

export function formatDeviceModelReferenceDeckAuditGateCoverageDigestJson(
  report: DeviceModelReferenceDeckAuditGateReport = deviceModelReferenceDeckAuditGate(),
): string {
  return formatDeckTableJson(formatDeviceModelReferenceDeckAuditGateCoverageDigestTable(report));
}

export function formatDeviceModelReferenceDeckAuditGateIssueTable(
  report: DeviceModelReferenceDeckAuditGateReport = deviceModelReferenceDeckAuditGate(),
): string {
  return [
    "fixture_name\tfield\tmessage",
    ...report.issues.map((issue) =>
      [issue.fixtureName, issue.field, issue.message].join("\t"),
    ),
  ].join("\n");
}

export function deviceModelReferenceDeckAuditGateIssueRecords(
  report: DeviceModelReferenceDeckAuditGateReport = deviceModelReferenceDeckAuditGate(),
): Array<Record<string, string>> {
  return deckTableRecords(formatDeviceModelReferenceDeckAuditGateIssueTable(report));
}

export function formatDeviceModelReferenceDeckAuditGateIssueCsv(
  report: DeviceModelReferenceDeckAuditGateReport = deviceModelReferenceDeckAuditGate(),
): string {
  return formatDeckTableCsv(formatDeviceModelReferenceDeckAuditGateIssueTable(report));
}

export function formatDeviceModelReferenceDeckAuditGateIssueJson(
  report: DeviceModelReferenceDeckAuditGateReport = deviceModelReferenceDeckAuditGate(),
): string {
  return formatDeckTableJson(formatDeviceModelReferenceDeckAuditGateIssueTable(report));
}

export function deviceModelReferenceDeckAuditGateIssueSummary(
  report: DeviceModelReferenceDeckAuditGateReport = deviceModelReferenceDeckAuditGate(),
): readonly DeviceModelReferenceDeckAuditGateIssueSummary[] {
  const groups = new Map<string, DeviceModelReferenceDeckAuditIssue[]>();
  for (const issue of report.issues) {
    const rows = groups.get(issue.field) ?? [];
    rows.push(issue);
    groups.set(issue.field, rows);
  }

  return [...groups.keys()].sort().map((field) => {
    const issues = groups.get(field) ?? [];
    const fixtureNames: string[] = [];
    const messages: string[] = [];
    for (const issue of issues) {
      if (!fixtureNames.includes(issue.fixtureName)) {
        fixtureNames.push(issue.fixtureName);
      }
      if (!messages.includes(issue.message)) {
        messages.push(issue.message);
      }
    }
    return {
      field,
      issueCount: issues.length,
      fixtureNames,
      messages,
    };
  });
}

export function formatDeviceModelReferenceDeckAuditGateIssueSummaryTable(
  report: DeviceModelReferenceDeckAuditGateReport = deviceModelReferenceDeckAuditGate(),
): string {
  return [
    "field\tissue_count\tfixture_names\tmessages",
    ...deviceModelReferenceDeckAuditGateIssueSummary(report).map((summary) =>
      [
        summary.field,
        summary.issueCount.toString(),
        summary.fixtureNames.join(","),
        summary.messages.join(","),
      ].join("\t"),
    ),
  ].join("\n");
}

export function deviceModelReferenceDeckAuditGateIssueSummaryRecords(
  report: DeviceModelReferenceDeckAuditGateReport = deviceModelReferenceDeckAuditGate(),
): Array<Record<string, string>> {
  return deckTableRecords(formatDeviceModelReferenceDeckAuditGateIssueSummaryTable(report));
}

export function formatDeviceModelReferenceDeckAuditGateIssueSummaryCsv(
  report: DeviceModelReferenceDeckAuditGateReport = deviceModelReferenceDeckAuditGate(),
): string {
  return formatDeckTableCsv(formatDeviceModelReferenceDeckAuditGateIssueSummaryTable(report));
}

export function formatDeviceModelReferenceDeckAuditGateIssueSummaryJson(
  report: DeviceModelReferenceDeckAuditGateReport = deviceModelReferenceDeckAuditGate(),
): string {
  return formatDeckTableJson(formatDeviceModelReferenceDeckAuditGateIssueSummaryTable(report));
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

export function measureTransientProbe(
  points: readonly TransientPoint[],
  name: string,
  probe: string,
  mode: string,
  fromTime?: number,
  toTime?: number,
): ProbeMeasurement {
  const normalizedMode = normalizeMeasurementMode(mode);
  if (fromTime !== undefined && !Number.isFinite(fromTime)) {
    throw invalidElement("measureTransientProbe", "fromTime must be finite");
  }
  if (toTime !== undefined && !Number.isFinite(toTime)) {
    throw invalidElement("measureTransientProbe", "toTime must be finite");
  }
  if (fromTime !== undefined && toTime !== undefined && fromTime > toTime) {
    throw invalidElement("measureTransientProbe", "fromTime must be <= toTime");
  }

  const selected = points.filter((point) =>
    (fromTime === undefined || point.time >= fromTime) &&
    (toTime === undefined || point.time <= toTime)
  );
  if (selected.length === 0) {
    throw invalidElement("measureTransientProbe", "no transient samples in window");
  }
  const values = selected.map((point) =>
    tableProbeValue(
      point.nodeVoltages,
      point.branchCurrents,
      probe,
      "measureTransientProbe",
    )
  );

  return {
    name,
    analysis: "tran",
    probe,
    mode: normalizedMode,
    value: measureValues(values, normalizedMode),
    fromValue: fromTime,
    toValue: toTime,
  };
}

export function measureTransientFindAtProbe(
  points: readonly TransientPoint[],
  name: string,
  probe: string,
  atTime: number,
): ProbeMeasurement {
  if (!Number.isFinite(atTime)) {
    throw invalidElement("measureTransientFindAtProbe", "atTime must be finite");
  }
  return {
    name,
    analysis: "tran",
    probe,
    mode: "find",
    value: transientProbeValueAt(points, probe, atTime, "measureTransientFindAtProbe"),
    fromValue: atTime,
    toValue: atTime,
  };
}

export function measureTransientWhenProbe(
  points: readonly TransientPoint[],
  name: string,
  probe: string,
  targetValue: number,
  fromTime?: number,
  toTime?: number,
): ProbeMeasurement {
  if (!Number.isFinite(targetValue)) {
    throw invalidElement("measureTransientWhenProbe", "targetValue must be finite");
  }
  if (fromTime !== undefined && !Number.isFinite(fromTime)) {
    throw invalidElement("measureTransientWhenProbe", "fromTime must be finite");
  }
  if (toTime !== undefined && !Number.isFinite(toTime)) {
    throw invalidElement("measureTransientWhenProbe", "toTime must be finite");
  }
  if (fromTime !== undefined && toTime !== undefined && fromTime > toTime) {
    throw invalidElement("measureTransientWhenProbe", "fromTime must be <= toTime");
  }
  return {
    name,
    analysis: "tran",
    probe,
    mode: "when",
    value: transientProbeCrossingTime(
      points,
      probe,
      targetValue,
      "cross",
      1,
      fromTime,
      toTime,
      "measureTransientWhenProbe",
    ),
    fromValue: fromTime,
    toValue: toTime,
  };
}

export function measureTransientWhenProbeCounted(
  points: readonly TransientPoint[],
  name: string,
  probe: string,
  targetValue: number,
  crossingKind: MeasurementCrossingKind,
  crossingCount: number,
  fromTime?: number,
  toTime?: number,
): ProbeMeasurement {
  const context = "measureTransientWhenProbeCounted";
  if (!Number.isFinite(targetValue)) {
    throw invalidElement(context, "targetValue must be finite");
  }
  const normalizedCrossingKind = normalizeTransientCrossingKind(crossingKind, context);
  if (!Number.isInteger(crossingCount) || crossingCount < 1) {
    throw invalidElement(context, "crossingCount must be a positive integer");
  }
  if (fromTime !== undefined && !Number.isFinite(fromTime)) {
    throw invalidElement(context, "fromTime must be finite");
  }
  if (toTime !== undefined && !Number.isFinite(toTime)) {
    throw invalidElement(context, "toTime must be finite");
  }
  if (fromTime !== undefined && toTime !== undefined && fromTime > toTime) {
    throw invalidElement(context, "fromTime must be <= toTime");
  }
  return {
    name,
    analysis: "tran",
    probe,
    mode: "when",
    value: transientProbeCrossingTime(
      points,
      probe,
      targetValue,
      normalizedCrossingKind,
      crossingCount,
      fromTime,
      toTime,
      context,
    ),
    fromValue: fromTime,
    toValue: toTime,
  };
}

export function measureTransientDelayBetweenProbes(
  points: readonly TransientPoint[],
  name: string,
  triggerProbe: string,
  triggerValue: number,
  triggerCrossingKind: MeasurementCrossingKind,
  triggerCrossingCount: number,
  targetProbe: string,
  targetValue: number,
  targetCrossingKind: MeasurementCrossingKind,
  targetCrossingCount: number,
  fromTime?: number,
  toTime?: number,
): ProbeMeasurement {
  const context = "measureTransientDelayBetweenProbes";
  if (!Number.isFinite(triggerValue)) {
    throw invalidElement(context, "triggerValue must be finite");
  }
  if (!Number.isFinite(targetValue)) {
    throw invalidElement(context, "targetValue must be finite");
  }
  const normalizedTriggerKind = normalizeTransientCrossingKind(triggerCrossingKind, context);
  const normalizedTargetKind = normalizeTransientCrossingKind(targetCrossingKind, context);
  if (!Number.isInteger(triggerCrossingCount) || !Number.isInteger(targetCrossingCount) ||
    triggerCrossingCount < 1 || targetCrossingCount < 1) {
    throw invalidElement(context, "crossing counts must be positive integers");
  }
  if (fromTime !== undefined && !Number.isFinite(fromTime)) {
    throw invalidElement(context, "fromTime must be finite");
  }
  if (toTime !== undefined && !Number.isFinite(toTime)) {
    throw invalidElement(context, "toTime must be finite");
  }
  if (fromTime !== undefined && toTime !== undefined && fromTime > toTime) {
    throw invalidElement(context, "fromTime must be <= toTime");
  }
  const triggerTime = transientProbeCrossingTime(
    points,
    triggerProbe,
    triggerValue,
    normalizedTriggerKind,
    triggerCrossingCount,
    fromTime,
    toTime,
    context,
  );
  const targetFromTime = Math.max(fromTime ?? triggerTime, triggerTime);
  const targetTime = transientProbeCrossingTime(
    points,
    targetProbe,
    targetValue,
    normalizedTargetKind,
    targetCrossingCount,
    targetFromTime,
    toTime,
    context,
  );
  return {
    name,
    analysis: "tran",
    probe: `${triggerProbe}->${targetProbe}`,
    mode: "delay",
    value: targetTime - triggerTime,
    fromValue: fromTime,
    toValue: toTime,
  };
}

function normalizeTransientCrossingKind(
  crossingKind: string,
  context: string,
): MeasurementCrossingKind {
  const normalized = crossingKind.trim().toLowerCase();
  if (normalized !== "rise" && normalized !== "fall" && normalized !== "cross") {
    throw invalidElement(context, "crossingKind must be rise, fall, or cross");
  }
  return normalized;
}

function transientProbeValueAt(
  points: readonly TransientPoint[],
  probe: string,
  atTime: number,
  context: string,
): number {
  let previous: readonly [number, number] | undefined;
  for (const point of points) {
    const value = tableProbeValue(point.nodeVoltages, point.branchCurrents, probe, context);
    if (point.time === atTime) {
      return value;
    }
    if (point.time > atTime) {
      if (previous === undefined) {
        throw invalidElement(context, "atTime is outside transient sample range");
      }
      const [previousTime, previousValue] = previous;
      if (point.time === previousTime) {
        throw invalidElement(context, "duplicate transient sample times around AT value");
      }
      const fraction = (atTime - previousTime) / (point.time - previousTime);
      return previousValue + (value - previousValue) * fraction;
    }
    previous = [point.time, value];
  }
  throw invalidElement(context, "atTime is outside transient sample range");
}

function transientProbeCrossingTime(
  points: readonly TransientPoint[],
  probe: string,
  targetValue: number,
  crossingKind: MeasurementCrossingKind,
  crossingCount: number,
  fromTime: number | undefined,
  toTime: number | undefined,
  context: string,
): number {
  let previous: readonly [number, number, number] | undefined;
  let selectedCount = 0;
  let matchedCount = 0;
  for (const point of points) {
    if ((fromTime !== undefined && point.time < fromTime) ||
      (toTime !== undefined && point.time > toTime)) {
      continue;
    }
    selectedCount += 1;
    const value = tableProbeValue(point.nodeVoltages, point.branchCurrents, probe, context);
    const delta = value - targetValue;
    let crossingTime: number | undefined;
    if (previous !== undefined) {
      const [previousTime, previousValue, previousDelta] = previous;
      if (delta === 0.0) {
        if (crossingKind === "cross") {
          crossingTime = point.time;
        } else if (crossingKind === "rise" && previousDelta < 0.0) {
          crossingTime = point.time;
        } else if (crossingKind === "fall" && previousDelta > 0.0) {
          crossingTime = point.time;
        }
      } else if ((previousDelta < 0.0 && delta > 0.0 && crossingKind !== "fall") ||
        (previousDelta > 0.0 && delta < 0.0 && crossingKind !== "rise")) {
        if (point.time === previousTime) {
          throw invalidElement(context, "duplicate transient sample times around WHEN crossing");
        }
        const fraction = (targetValue - previousValue) / (value - previousValue);
        crossingTime = previousTime + (point.time - previousTime) * fraction;
      }
    } else if (delta === 0.0 && crossingKind === "cross") {
      crossingTime = point.time;
    }
    if (crossingTime !== undefined) {
      matchedCount += 1;
      if (matchedCount === crossingCount) {
        return crossingTime;
      }
    }
    previous = [point.time, value, delta];
  }
  if (selectedCount === 0) {
    throw invalidElement(context, "no transient samples in window");
  }
  throw invalidElement(context, "no transient crossing in window");
}

export function measureTransientCards(
  points: readonly TransientPoint[],
  measurements: readonly DeckMeasurementCard[],
): ProbeMeasurement[] {
  return measurements.map((measurement) => {
    if (measurement.analysis !== "tran" && measurement.analysis !== "transient") {
      throw invalidElement("measureTransientCards", "only transient measurement cards are supported");
    }
    if (measurement.mode === "find") {
      if (measurement.atValue === undefined) {
        throw invalidElement("measureTransientCards", "FIND measurement cards require an AT value");
      }
      return measureTransientFindAtProbe(
        points,
        measurement.name,
        measurement.probe,
        measurement.atValue,
      );
    }
    if (measurement.mode === "when") {
      if (measurement.targetValue === undefined) {
        throw invalidElement("measureTransientCards", "WHEN measurement cards require a target value");
      }
      return measureTransientWhenProbeCounted(
        points,
        measurement.name,
        measurement.probe,
        measurement.targetValue,
        measurement.crossingKind ?? "cross",
        measurement.crossingCount ?? 1,
        measurement.fromValue,
        measurement.toValue,
      );
    }
    if (measurement.mode === "delay") {
      if (measurement.triggerProbe === undefined) {
        throw invalidElement("measureTransientCards", "delay measurement cards require a trigger probe");
      }
      if (measurement.triggerValue === undefined) {
        throw invalidElement("measureTransientCards", "delay measurement cards require a trigger value");
      }
      if (measurement.targetValue === undefined) {
        throw invalidElement("measureTransientCards", "delay measurement cards require a target value");
      }
      return measureTransientDelayBetweenProbes(
        points,
        measurement.name,
        measurement.triggerProbe,
        measurement.triggerValue,
        measurement.triggerCrossingKind ?? "cross",
        measurement.triggerCrossingCount ?? 1,
        measurement.probe,
        measurement.targetValue,
        measurement.crossingKind ?? "cross",
        measurement.crossingCount ?? 1,
        measurement.fromValue,
        measurement.toValue,
      );
    }
    return measureTransientProbe(
      points,
      measurement.name,
      measurement.probe,
      measurement.mode,
      measurement.fromValue,
      measurement.toValue,
    );
  });
}

export function measureTransientDeck(
  points: readonly TransientPoint[],
  netlist: string,
): ProbeMeasurement[] {
  const summary = resolveDeckMeasurements(netlist);
  if (summary.diagnostics.length > 0) {
    const diagnostic = summary.diagnostics[0]!;
    throw invalidElement(
      "measureTransientDeck",
      `line ${diagnostic.lineNumber}: ${diagnostic.message}`,
    );
  }
  return measureTransientCards(points, summary.measurements);
}

export function measureDcSweepProbe(
  points: readonly DcSweepPoint[],
  name: string,
  probe: string,
  mode: string,
  fromValue?: number,
  toValue?: number,
): ProbeMeasurement {
  const normalizedMode = normalizeMeasurementMode(mode, "measureDcSweepProbe");
  if (fromValue !== undefined && !Number.isFinite(fromValue)) {
    throw invalidElement("measureDcSweepProbe", "fromValue must be finite");
  }
  if (toValue !== undefined && !Number.isFinite(toValue)) {
    throw invalidElement("measureDcSweepProbe", "toValue must be finite");
  }
  if (fromValue !== undefined && toValue !== undefined && fromValue > toValue) {
    throw invalidElement("measureDcSweepProbe", "fromValue must be <= toValue");
  }

  const selected = points.filter((point) =>
    (fromValue === undefined || point.value >= fromValue) &&
    (toValue === undefined || point.value <= toValue)
  );
  if (selected.length === 0) {
    throw invalidElement("measureDcSweepProbe", "no dc sweep samples in window");
  }
  const values = selected.map((point) =>
    tableProbeValue(
      point.result.nodeVoltages,
      point.result.branchCurrents,
      probe,
      "measureDcSweepProbe",
    )
  );

  return {
    name,
    analysis: "dc",
    probe,
    mode: normalizedMode,
    value: measureValues(values, normalizedMode, "measureDcSweepProbe"),
    fromValue,
    toValue,
  };
}

export function measureDcSweepCards(
  points: readonly DcSweepPoint[],
  measurements: readonly DeckMeasurementCard[],
): ProbeMeasurement[] {
  return measurements.map((measurement) => {
    if (measurement.analysis !== "dc") {
      throw invalidElement("measureDcSweepCards", "only dc measurement cards are supported");
    }
    return measureDcSweepProbe(
      points,
      measurement.name,
      measurement.probe,
      measurement.mode,
      measurement.fromValue,
      measurement.toValue,
    );
  });
}

export function measureDcSweepDeck(
  points: readonly DcSweepPoint[],
  netlist: string,
): ProbeMeasurement[] {
  const summary = resolveDeckMeasurements(netlist);
  if (summary.diagnostics.length > 0) {
    const diagnostic = summary.diagnostics[0]!;
    throw invalidElement(
      "measureDcSweepDeck",
      `line ${diagnostic.lineNumber}: ${diagnostic.message}`,
    );
  }
  return measureDcSweepCards(points, summary.measurements);
}

export function measureAcSweepProbe(
  points: readonly AcPoint[],
  name: string,
  probe: string,
  mode: string,
  fromFrequency?: number,
  toFrequency?: number,
): ProbeMeasurement {
  const normalizedMode = normalizeMeasurementMode(mode, "measureAcSweepProbe");
  if (fromFrequency !== undefined && !Number.isFinite(fromFrequency)) {
    throw invalidElement("measureAcSweepProbe", "fromFrequency must be finite");
  }
  if (toFrequency !== undefined && !Number.isFinite(toFrequency)) {
    throw invalidElement("measureAcSweepProbe", "toFrequency must be finite");
  }
  if (fromFrequency !== undefined && toFrequency !== undefined && fromFrequency > toFrequency) {
    throw invalidElement("measureAcSweepProbe", "fromFrequency must be <= toFrequency");
  }

  const selected = points.filter((point) =>
    (fromFrequency === undefined || point.frequencyHz >= fromFrequency) &&
    (toFrequency === undefined || point.frequencyHz <= toFrequency)
  );
  if (selected.length === 0) {
    throw invalidElement("measureAcSweepProbe", "no ac sweep samples in window");
  }
  const values = selected.map((point) =>
    complexAbs(
      tableComplexProbeValue(
        point.nodeVoltages,
        point.branchCurrents,
        probe,
        "measureAcSweepProbe",
      ),
    )
  );

  return {
    name,
    analysis: "ac",
    probe,
    mode: normalizedMode,
    value: measureValues(values, normalizedMode, "measureAcSweepProbe"),
    fromValue: fromFrequency,
    toValue: toFrequency,
  };
}

export function measureAcSweepCards(
  points: readonly AcPoint[],
  measurements: readonly DeckMeasurementCard[],
): ProbeMeasurement[] {
  return measurements.map((measurement) => {
    if (measurement.analysis !== "ac") {
      throw invalidElement("measureAcSweepCards", "only ac measurement cards are supported");
    }
    return measureAcSweepProbe(
      points,
      measurement.name,
      measurement.probe,
      measurement.mode,
      measurement.fromValue,
      measurement.toValue,
    );
  });
}

export function measureAcSweepDeck(
  points: readonly AcPoint[],
  netlist: string,
): ProbeMeasurement[] {
  const summary = resolveDeckMeasurements(netlist);
  if (summary.diagnostics.length > 0) {
    const diagnostic = summary.diagnostics[0]!;
    throw invalidElement(
      "measureAcSweepDeck",
      `line ${diagnostic.lineNumber}: ${diagnostic.message}`,
    );
  }
  return measureAcSweepCards(points, summary.measurements);
}

export function formatMeasurementTable(measurements: readonly ProbeMeasurement[]): string {
  const rows = [["Name", "Analysis", "Probe", "Mode", "From", "To", "Value"].join("\t")];
  measurements.forEach((measurement) => {
    rows.push([
      measurement.name,
      measurement.analysis,
      measurement.probe,
      measurement.mode,
      formatOptionalTableNumber(measurement.fromValue),
      formatOptionalTableNumber(measurement.toValue),
      formatTableNumber(measurement.value),
    ].join("\t"));
  });
  rows.push("");
  return rows.join("\n");
}

function normalizeMeasurementMode(mode: string, context = "measureTransientProbe"): string {
  const normalized = mode.trim().toLowerCase().replace(/_/g, "-");
  switch (normalized) {
    case "max":
    case "min":
      return normalized;
    case "avg":
    case "average":
    case "mean":
      return "avg";
    case "rms":
    case "root-mean-square":
      return "rms";
    case "pp":
    case "p-p":
    case "p2p":
    case "peak-to-peak":
    case "peak2peak":
      return "pp";
    case "last":
    case "final":
      return "last";
    default:
      throw invalidElement(context, `unsupported mode ${JSON.stringify(mode)}`);
  }
}

function measureValues(values: readonly number[], mode: string, context = "measureTransientProbe"): number {
  switch (mode) {
    case "max":
      return values.reduce((max, value) => Math.max(max, value), Number.NEGATIVE_INFINITY);
    case "min":
      return values.reduce((min, value) => Math.min(min, value), Number.POSITIVE_INFINITY);
    case "avg":
      return values.reduce((sum, value) => sum + value, 0.0) / values.length;
    case "rms":
      return Math.sqrt(
        values.reduce((sum, value) => sum + value * value, 0.0) / values.length,
      );
    case "pp":
      return values.reduce((max, value) => Math.max(max, value), Number.NEGATIVE_INFINITY) -
        values.reduce((min, value) => Math.min(min, value), Number.POSITIVE_INFINITY);
    case "last":
      return values[values.length - 1]!;
    default:
      throw invalidElement(context, `unsupported mode ${JSON.stringify(mode)}`);
  }
}

function formatOptionalTableNumber(value: number | undefined): string {
  return value === undefined ? "" : formatTableNumber(value);
}

export function formatCornerDcTable(
  result: CornerSweepResult,
  probes?: readonly string[],
): string {
  const selectedProbes = probes === undefined || probes.length === 0 ? (
    result.points.length === 0 ? [] : defaultOutputProbes(
      result.points[0].result.nodeVoltages,
      result.points[0].result.branchCurrents,
    )
  ) : probes;
  const rows = [["Corner", "Index", ...selectedProbes].join("\t")];
  result.points.forEach((point, index) => {
    const values = selectedProbes.map((probe) =>
      formatTableNumber(
        tableProbeValue(
          point.result.nodeVoltages,
          point.result.branchCurrents,
          probe,
          "formatCornerDcTable",
        ),
      ),
    );
    rows.push([point.cornerName, String(index), ...values].join("\t"));
  });
  rows.push("");
  return rows.join("\n");
}

export function formatTemperatureDcTable(
  result: TemperatureDcResult,
  probes?: readonly string[],
): string {
  const selectedProbes = probes === undefined || probes.length === 0 ? (
    result.points.length === 0 ? [] : defaultOutputProbes(
      result.points[0].result.nodeVoltages,
      result.points[0].result.branchCurrents,
    )
  ) : probes;
  const rows = [["Index", "TemperatureKelvin", ...selectedProbes].join("\t")];
  result.points.forEach((point, index) => {
    const values = selectedProbes.map((probe) =>
      formatTableNumber(
        tableProbeValue(
          point.result.nodeVoltages,
          point.result.branchCurrents,
          probe,
          "formatTemperatureDcTable",
        ),
      ),
    );
    rows.push([
      String(index),
      formatTableNumber(point.temperatureKelvin),
      ...values,
    ].join("\t"));
  });
  rows.push("");
  return rows.join("\n");
}

export function formatCornerTemperatureDcTable(
  result: CornerTemperatureDcResult,
  probes?: readonly string[],
): string {
  const firstNonEmpty = result.points.find((corner) => corner.points.length > 0);
  const selectedProbes = probes === undefined || probes.length === 0 ? (
    firstNonEmpty === undefined ? [] : defaultOutputProbes(
      firstNonEmpty.points[0].result.nodeVoltages,
      firstNonEmpty.points[0].result.branchCurrents,
    )
  ) : probes;
  const rows = [["Corner", "Index", "TemperatureKelvin", ...selectedProbes].join("\t")];
  result.points.forEach((corner) => {
    corner.points.forEach((point, index) => {
      const values = selectedProbes.map((probe) =>
        formatTableNumber(
          tableProbeValue(
            point.result.nodeVoltages,
            point.result.branchCurrents,
            probe,
            "formatCornerTemperatureDcTable",
          ),
        ),
      );
      rows.push([
        corner.cornerName,
        String(index),
        formatTableNumber(point.temperatureKelvin),
        ...values,
      ].join("\t"));
    });
  });
  rows.push("");
  return rows.join("\n");
}

export function formatDcSweepTable(
  sourceName: string,
  points: readonly DcSweepPoint[],
  probes?: readonly string[],
): string {
  const selectedProbes = probes === undefined || probes.length === 0 ? (
    points.length === 0 ? [] : defaultOutputProbes(
      points[0].result.nodeVoltages,
      points[0].result.branchCurrents,
    )
  ) : probes;
  const rows = [["Index", "Source", "Value", ...selectedProbes].join("\t")];
  points.forEach((point, index) => {
    const values = selectedProbes.map((probe) =>
      formatTableNumber(
        tableProbeValue(
          point.result.nodeVoltages,
          point.result.branchCurrents,
          probe,
          "formatDcSweepTable",
        ),
      ),
    );
    rows.push([
      String(index),
      sourceName,
      formatTableNumber(point.value),
      ...values,
    ].join("\t"));
  });
  rows.push("");
  return rows.join("\n");
}

export function formatCornerDcSweepTable(
  result: CornerDcSweepResult,
  probes?: readonly string[],
): string {
  const firstNonEmpty = result.points.find((corner) => corner.points.length > 0);
  const selectedProbes = probes === undefined || probes.length === 0 ? (
    firstNonEmpty === undefined ? [] : defaultOutputProbes(
      firstNonEmpty.points[0].result.nodeVoltages,
      firstNonEmpty.points[0].result.branchCurrents,
    )
  ) : probes;
  const rows = [["Corner", "Index", "Source", "Value", ...selectedProbes].join("\t")];
  result.points.forEach((corner) => {
    corner.points.forEach((point, index) => {
      const values = selectedProbes.map((probe) =>
        formatTableNumber(
          tableProbeValue(
            point.result.nodeVoltages,
            point.result.branchCurrents,
            probe,
            "formatCornerDcSweepTable",
          ),
        ),
      );
      rows.push([
        corner.cornerName,
        String(index),
        result.sourceName,
        formatTableNumber(point.value),
        ...values,
      ].join("\t"));
    });
  });
  rows.push("");
  return rows.join("\n");
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

export function formatCornerTransientTable(
  result: CornerTransientResult,
  probes?: readonly string[],
): string {
  const firstNonEmpty = result.points.find((point) => point.points.length > 0);
  const selectedProbes = probes ?? (
    firstNonEmpty === undefined ? [] : defaultTransientOutputProbes(firstNonEmpty.points)
  );
  const rows = [["Corner", "Index", "Time", ...selectedProbes].join("\t")];
  result.points.forEach((corner) => {
    corner.points.forEach((point, index) => {
      const values = selectedProbes.map((probe) =>
        formatTableNumber(
          tableProbeValue(
            point.nodeVoltages,
            point.branchCurrents,
            probe,
            "formatCornerTransientTable",
          ),
        ),
      );
      rows.push([corner.cornerName, String(index), formatTableNumber(point.time), ...values].join("\t"));
    });
  });
  rows.push("");
  return rows.join("\n");
}

export function formatCornerAdaptiveTransientTable(
  result: CornerAdaptiveTransientResult,
  probes?: readonly string[],
): string {
  const firstNonEmpty = result.points.find((point) => point.result.points.length > 0);
  const selectedProbes = probes ?? (
    firstNonEmpty === undefined ? [] : defaultTransientOutputProbes(firstNonEmpty.result.points)
  );
  const rows = [[
    "Corner",
    "Method",
    "StepsRejected",
    "Converged",
    "Index",
    "Time",
    ...selectedProbes,
  ].join("\t")];
  result.points.forEach((corner) => {
    corner.result.points.forEach((point, index) => {
      const values = selectedProbes.map((probe) =>
        formatTableNumber(
          tableProbeValue(
            point.nodeVoltages,
            point.branchCurrents,
            probe,
            "formatCornerAdaptiveTransientTable",
          ),
        ),
      );
      rows.push([
        corner.cornerName,
        corner.result.method,
        String(corner.result.stepsRejected),
        String(corner.result.converged),
        String(index),
        formatTableNumber(point.time),
        ...values,
      ].join("\t"));
    });
  });
  rows.push("");
  return rows.join("\n");
}

export function formatPssTable(
  result: PssResult,
  probes?: readonly string[],
): string {
  const selectedProbes = probes ?? defaultTransientOutputProbes(result.steadyState);
  const rows = [[
    "Index",
    "Period",
    "TimeStep",
    "Converged",
    "Iterations",
    "ResidualL2",
    "Time",
    ...selectedProbes,
  ].join("\t")];
  result.steadyState.forEach((point, index) => {
    const values = selectedProbes.map((probe) =>
      formatTableNumber(
        tableProbeValue(
          point.nodeVoltages,
          point.branchCurrents,
          probe,
          "formatPssTable",
        ),
      ),
    );
    rows.push([
      String(index),
      formatTableNumber(result.periodSeconds),
      formatTableNumber(result.timeStepSeconds),
      String(result.converged),
      String(result.solve.iterationCount),
      formatTableNumber(result.solve.finalResidual.residualL2Norm),
      formatTableNumber(point.time),
      ...values,
    ].join("\t"));
  });
  rows.push("");
  return rows.join("\n");
}

export function formatCornerPssTable(
  result: CornerPssResult,
  probes?: readonly string[],
): string {
  const firstNonEmpty = result.points.find((point) => point.result.steadyState.length > 0);
  const selectedProbes = probes ?? (
    firstNonEmpty === undefined ? [] : defaultTransientOutputProbes(firstNonEmpty.result.steadyState)
  );
  const rows = [[
    "Corner",
    "Index",
    "Period",
    "TimeStep",
    "Converged",
    "Iterations",
    "ResidualL2",
    "Time",
    ...selectedProbes,
  ].join("\t")];
  result.points.forEach((corner) => {
    corner.result.steadyState.forEach((point, index) => {
      const values = selectedProbes.map((probe) =>
        formatTableNumber(
          tableProbeValue(
            point.nodeVoltages,
            point.branchCurrents,
            probe,
            "formatCornerPssTable",
          ),
        ),
      );
      rows.push([
        corner.cornerName,
        String(index),
        formatTableNumber(corner.result.periodSeconds),
        formatTableNumber(corner.result.timeStepSeconds),
        String(corner.result.converged),
        String(corner.result.solve.iterationCount),
        formatTableNumber(corner.result.solve.finalResidual.residualL2Norm),
        formatTableNumber(point.time),
        ...values,
      ].join("\t"));
    });
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

export function formatDeckOpTable(result: DcResult, netlist: string): string {
  const probes = selectDeckOutputProbes(netlist, "op");
  return probes.length === 0 ? formatDcTable(result) : formatDcTable(result, probes);
}

export function formatDeckDcSweepTable(
  sourceName: string,
  points: readonly DcSweepPoint[],
  netlist: string,
): string {
  const probes = selectDeckOutputProbes(netlist, "dc");
  return probes.length === 0
    ? formatDcSweepTable(sourceName, points)
    : formatDcSweepTable(sourceName, points, probes);
}

export function formatDeckAcTable(points: readonly AcPoint[], netlist: string): string {
  const probes = selectDeckOutputProbes(netlist, "ac");
  return probes.length === 0 ? formatAcTable(points) : formatAcTable(points, probes);
}

export function formatDeckTransientTable(
  points: readonly TransientPoint[],
  netlist: string,
): string {
  const probes = selectDeckOutputProbes(netlist, "tran");
  return probes.length === 0 ? formatTransientTable(points) : formatTransientTable(points, probes);
}

export function formatDeckTfTable(result: TfResult): string {
  return formatTfTable(result);
}

export function formatDeckSensTable(result: SensResult): string {
  return formatSensTable(result);
}

export function formatDeckNoiseTable(result: NoiseResult): string {
  return formatNoiseTable(result);
}

function deckResultRowCount(result: DeckAnalysisExecutionResult): number {
  return Array.isArray(result)
    ? result.length
    : "points" in result && Array.isArray(result.points)
      ? result.points.length
      : 1;
}

function deckRunArtifacts(
  plan: DeckAnalysisPlan,
  result: DeckAnalysisExecutionResult,
  resultColumns: readonly string[],
  outputProbes: readonly string[],
  outputDirectives: readonly string[],
  measurements: readonly ProbeMeasurement[],
  fourier: readonly FourierResult[],
  controlLines: readonly string[],
  writeMarkers: readonly string[],
  rawfileOptions: readonly string[],
  diagnosticCodes: readonly string[],
  controlPolicyArtifacts: readonly DeckControlPolicyArtifact[],
  deckAnalysisKinds: readonly string[],
  deckAnalysisDirectiveInventory: readonly string[],
): DeckRunArtifact[] {
  const isTransient = plan.analysis === "tran";
  const analysisDirectives = deckAnalysisDirectives(plan);
  const tables = deckStableTables(measurements, fourier, controlPolicyArtifacts);
  const controlPolicySummaries = deckControlPolicySummaryArtifacts(controlPolicyArtifacts);
  const controlPolicyCategories = controlPolicySummaries.map((artifact) => artifact.category);
  const controlPolicyCodes = controlPolicySummaries.flatMap((artifact) => artifact.codes);
  const controlPolicySeverities: string[] = [];
  for (const artifact of controlPolicySummaries) {
    for (const severity of artifact.severities) {
      pushUniqueString(controlPolicySeverities, severity);
    }
  }
  return [
    {
      analysis: plan.analysis,
      directive: plan.directive,
      analysisDirectiveCount: analysisDirectives.length,
      analysisDirectives,
      deckAnalysisKindCount: deckAnalysisKinds.length,
      deckAnalysisKinds: [...deckAnalysisKinds],
      deckAnalysisDirectiveCount: deckAnalysisDirectiveInventory.length,
      deckAnalysisDirectives: [...deckAnalysisDirectiveInventory],
      lineNumber: plan.lineNumber,
      sourceName: plan.sourceName,
      outputNode: plan.outputNode,
      sweepKind: plan.sweepKind,
      startValue: plan.startValue,
      stopValue: plan.stopValue,
      stepValue: plan.stepValue,
      pointCount: plan.pointCount,
      startFrequencyHz: plan.startFrequencyHz,
      stopFrequencyHz: plan.stopFrequencyHz,
      stepTime: isTransient ? plan.stepTime : undefined,
      stopTime: isTransient ? plan.stopTime : undefined,
      startTime: isTransient ? plan.startTime : undefined,
      maxStep: isTransient ? plan.maxStep : undefined,
      useInitialConditions: isTransient ? plan.useInitialConditions : undefined,
      resultRows: deckResultRowCount(result),
      resultColumnCount: resultColumns.length,
      resultColumns: [...resultColumns],
      tableCount: tables.length,
      tables,
      outputProbeCount: outputProbes.length,
      outputProbes: [...outputProbes],
      outputDirectiveCount: outputDirectives.length,
      outputDirectives: [...outputDirectives],
      measurementCount: measurements.length,
      measurementNames: measurements.map((measurement) => measurement.name),
      fourierCount: fourier.length,
      fourierProbes: fourier.flatMap((result) => result.probes.map((probe) => probe.probe)),
      controlLineCount: controlLines.length,
      controlLines: [...controlLines],
      writeMarkerCount: writeMarkers.length,
      writeMarkers: [...writeMarkers],
      rawfileOptionCount: rawfileOptions.length,
      rawfileOptions: [...rawfileOptions],
      controlPolicyArtifactCount: controlPolicyArtifacts.length,
      controlPolicyCategories,
      controlPolicyCodes,
      controlPolicySeverities,
      diagnosticCount: diagnosticCodes.length,
      diagnosticCodes: [...diagnosticCodes],
    },
  ];
}

function deckAnalysisDirectives(plan: DeckAnalysisPlan): string[] {
  return plan.directive.length === 0 ? [] : [plan.directive];
}

function deckAnalysisInventory(netlist: string): {
  analysisKinds: string[];
  directives: string[];
} {
  const analysisKinds: string[] = [];
  const directives: string[] = [];
  for (const plan of resolveDeckAnalyses(netlist).analyses) {
    if (plan.analysis.length > 0) {
      pushUniqueString(analysisKinds, plan.analysis);
    }
    if (plan.directive.length > 0) {
      directives.push(plan.directive);
    }
  }
  return { analysisKinds, directives };
}

function deckStableTables(
  measurements: readonly ProbeMeasurement[],
  fourier: readonly FourierResult[],
  controlPolicyArtifacts: readonly DeckControlPolicyArtifact[],
): string[] {
  const tables = ["result"];
  if (measurements.length > 0) {
    tables.push("measurement");
  }
  if (fourier.length > 0) {
    tables.push("fourier");
  }
  if (controlPolicyArtifacts.length > 0) {
    tables.push("control-policy", "control-policy-summary");
  }
  tables.push("output-plan");
  tables.push("run-artifact");
  return tables;
}

function deckAnalysisDiagnosticCodes(netlist: string, plan: DeckAnalysisPlan): string[] {
  return resolveDeckAnalyses(netlist).diagnostics
    .filter((diagnostic) =>
      diagnostic.lineNumber === plan.lineNumber && diagnostic.directive === plan.directive
    )
    .map((diagnostic) => diagnostic.code);
}

function deckControlDiagnosticCodes(netlist: string): string[] {
  return analyzeDeckControls(netlist).diagnostics
    .filter((diagnostic) => diagnostic.code.startsWith("SPICE_DECK_CONTROL_"))
    .map((diagnostic) => diagnostic.code);
}

function deckControlLines(netlist: string): string[] {
  return [...analyzeDeckControls(netlist).controlLines];
}

function deckControlWriteMarkers(netlist: string): string[] {
  return [...analyzeDeckControls(netlist).writeMarkers];
}

function deckControlRawfileOptions(netlist: string): string[] {
  return [...analyzeDeckControls(netlist).rawfileOptions];
}

function deckRunDiagnosticCodes(netlist: string, plan: DeckAnalysisPlan): string[] {
  return [
    ...deckAnalysisDiagnosticCodes(netlist, plan),
    ...deckControlDiagnosticCodes(netlist),
  ];
}

function formatDeckArtifactFloat(value: number | undefined): string {
  return value === undefined ? "" : formatTableNumber(value);
}

function formatDeckArtifactBoolean(value: boolean | undefined): string {
  return value === undefined ? "" : String(value);
}

const DECK_RUN_ARTIFACT_COLUMNS = [
  "Analysis",
  "Directive",
  "AnalysisDirectives",
  "AnalysisDirectiveList",
  "Line",
  "SourceName",
  "OutputNode",
  "SweepKind",
  "StartValue",
  "StopValue",
  "StepValue",
  "PointCount",
  "StartFrequencyHz",
  "StopFrequencyHz",
  "StepTime",
  "StopTime",
  "StartTime",
  "MaxStep",
  "UseInitialConditions",
  "ResultRows",
  "ResultColumns",
  "ResultColumnList",
  "Tables",
  "TableList",
  "OutputProbes",
  "OutputProbeList",
  "OutputDirectives",
  "OutputDirectiveList",
  "Measurements",
  "MeasurementList",
  "Fourier",
  "FourierList",
  "ControlLines",
  "ControlLineList",
  "WriteMarkers",
  "WriteMarkerList",
  "RawfileOptions",
  "RawfileOptionList",
  "ControlPolicyArtifacts",
  "ControlPolicyCategoryList",
  "ControlPolicyCodeList",
  "ControlPolicySeverityList",
  "Diagnostics",
  "DiagnosticCodeList",
  "DeckAnalysisKinds",
  "DeckAnalysisKindList",
  "DeckAnalysisDirectives",
  "DeckAnalysisDirectiveList",
] as const;

function deckRunArtifactCells(artifact: DeckRunArtifact): string[] {
  return [
    artifact.analysis,
    artifact.directive,
    String(artifact.analysisDirectiveCount),
    artifact.analysisDirectives.join(";"),
    String(artifact.lineNumber),
    artifact.sourceName ?? "",
    artifact.outputNode ?? "",
    artifact.sweepKind ?? "",
    formatDeckArtifactFloat(artifact.startValue),
    formatDeckArtifactFloat(artifact.stopValue),
    formatDeckArtifactFloat(artifact.stepValue),
    artifact.pointCount === undefined ? "" : String(artifact.pointCount),
    formatDeckArtifactFloat(artifact.startFrequencyHz),
    formatDeckArtifactFloat(artifact.stopFrequencyHz),
    formatDeckArtifactFloat(artifact.stepTime),
    formatDeckArtifactFloat(artifact.stopTime),
    formatDeckArtifactFloat(artifact.startTime),
    formatDeckArtifactFloat(artifact.maxStep),
    formatDeckArtifactBoolean(artifact.useInitialConditions),
    String(artifact.resultRows),
    String(artifact.resultColumnCount),
    artifact.resultColumns.join(";"),
    String(artifact.tableCount),
    artifact.tables.join(";"),
    String(artifact.outputProbeCount),
    artifact.outputProbes.join(";"),
    String(artifact.outputDirectiveCount),
    artifact.outputDirectives.join(";"),
    String(artifact.measurementCount),
    artifact.measurementNames.join(";"),
    String(artifact.fourierCount),
    artifact.fourierProbes.join(";"),
    String(artifact.controlLineCount),
    artifact.controlLines.join(";"),
    String(artifact.writeMarkerCount),
    artifact.writeMarkers.join(";"),
    String(artifact.rawfileOptionCount),
    artifact.rawfileOptions.join(";"),
    String(artifact.controlPolicyArtifactCount),
    artifact.controlPolicyCategories.join(";"),
    artifact.controlPolicyCodes.join(";"),
    artifact.controlPolicySeverities.join(";"),
    String(artifact.diagnosticCount),
    artifact.diagnosticCodes.join(";"),
    String(artifact.deckAnalysisKindCount),
    artifact.deckAnalysisKinds.join(";"),
    String(artifact.deckAnalysisDirectiveCount),
    artifact.deckAnalysisDirectives.join(";"),
  ];
}

type DeckRunArtifactRecord = Record<(typeof DECK_RUN_ARTIFACT_COLUMNS)[number], string>;

function deckRunArtifactRecord(artifact: DeckRunArtifact): DeckRunArtifactRecord {
  const cells = deckRunArtifactCells(artifact);
  return Object.fromEntries(
    DECK_RUN_ARTIFACT_COLUMNS.map((column, index) => [column, cells[index] ?? ""]),
  ) as DeckRunArtifactRecord;
}

export function deckRunArtifactRecords(
  artifacts: readonly DeckRunArtifact[],
): DeckRunArtifactRecord[] {
  return artifacts.map(deckRunArtifactRecord);
}

function deckOutputPlanArtifacts(
  plan: DeckAnalysisPlan,
  resultRowCount: number,
  resultColumns: readonly string[],
  outputProbes: readonly string[],
  outputProbeLines: readonly number[],
  outputDirectives: readonly string[],
  outputDirectiveAnalysisKinds: readonly string[],
  outputDirectiveLines: readonly number[],
  tables: readonly string[],
): DeckOutputPlanArtifact[] {
  const outputDirectiveKinds = deckOutputDirectiveKinds(outputDirectives);
  const isTransient = plan.analysis === "tran";
  return [
    {
      analysis: plan.analysis,
      directive: plan.directive,
      lineNumber: plan.lineNumber,
      sourceName: plan.sourceName,
      outputNode: plan.outputNode,
      sweepKind: plan.sweepKind,
      startValue: plan.startValue,
      stopValue: plan.stopValue,
      stepValue: plan.stepValue,
      pointCount: plan.pointCount,
      startFrequencyHz: plan.startFrequencyHz,
      stopFrequencyHz: plan.stopFrequencyHz,
      stepTime: isTransient ? plan.stepTime : undefined,
      stopTime: isTransient ? plan.stopTime : undefined,
      startTime: isTransient ? plan.startTime : undefined,
      maxStep: isTransient ? plan.maxStep : undefined,
      useInitialConditions: isTransient ? plan.useInitialConditions : undefined,
      resultRowCount,
      resultColumnCount: resultColumns.length,
      resultColumns: [...resultColumns],
      outputProbeCount: outputProbes.length,
      outputProbes: [...outputProbes],
      outputProbeLineCount: outputProbeLines.length,
      outputProbeLines: [...outputProbeLines],
      outputDirectiveCount: outputDirectives.length,
      outputDirectives: [...outputDirectives],
      outputDirectiveKindCount: outputDirectiveKinds.length,
      outputDirectiveKinds,
      outputDirectiveAnalysisKindCount: outputDirectiveAnalysisKinds.length,
      outputDirectiveAnalysisKinds: [...outputDirectiveAnalysisKinds],
      outputDirectiveLineCount: outputDirectiveLines.length,
      outputDirectiveLines: [...outputDirectiveLines],
      tableCount: tables.length,
      tables: [...tables],
    },
  ];
}

function deckOutputDirectiveKind(directive: string): string {
  const token = directive.trim().split(/\s+/u)[0]?.toLowerCase() ?? "";
  return token.startsWith(".") ? token.slice(1) : token;
}

function deckOutputDirectiveKinds(outputDirectives: readonly string[]): string[] {
  const selected: string[] = [];
  const seen = new Set<string>();
  for (const directive of outputDirectives) {
    const kind = deckOutputDirectiveKind(directive);
    if (kind.length === 0 || seen.has(kind)) {
      continue;
    }
    seen.add(kind);
    selected.push(kind);
  }
  return selected;
}

const DECK_OUTPUT_PLAN_ARTIFACT_COLUMNS = [
  "Analysis",
  "Directive",
  "Line",
  "SourceName",
  "OutputNode",
  "SweepKind",
  "StartValue",
  "StopValue",
  "StepValue",
  "PointCount",
  "StartFrequencyHz",
  "StopFrequencyHz",
  "StepTime",
  "StopTime",
  "StartTime",
  "MaxStep",
  "UseInitialConditions",
  "ResultRows",
  "ResultColumns",
  "ResultColumnList",
  "OutputProbes",
  "OutputProbeList",
  "OutputProbeLines",
  "OutputProbeLineList",
  "OutputDirectives",
  "OutputDirectiveList",
  "OutputDirectiveKinds",
  "OutputDirectiveKindList",
  "OutputDirectiveAnalysisKinds",
  "OutputDirectiveAnalysisKindList",
  "OutputDirectiveLines",
  "OutputDirectiveLineList",
  "Tables",
  "TableList",
] as const;

function deckOutputPlanArtifactCells(artifact: DeckOutputPlanArtifact): string[] {
  return [
    artifact.analysis,
    artifact.directive,
    String(artifact.lineNumber),
    artifact.sourceName ?? "",
    artifact.outputNode ?? "",
    artifact.sweepKind ?? "",
    formatDeckArtifactFloat(artifact.startValue),
    formatDeckArtifactFloat(artifact.stopValue),
    formatDeckArtifactFloat(artifact.stepValue),
    artifact.pointCount === undefined ? "" : String(artifact.pointCount),
    formatDeckArtifactFloat(artifact.startFrequencyHz),
    formatDeckArtifactFloat(artifact.stopFrequencyHz),
    formatDeckArtifactFloat(artifact.stepTime),
    formatDeckArtifactFloat(artifact.stopTime),
    formatDeckArtifactFloat(artifact.startTime),
    formatDeckArtifactFloat(artifact.maxStep),
    formatDeckArtifactBoolean(artifact.useInitialConditions),
    String(artifact.resultRowCount),
    String(artifact.resultColumnCount),
    artifact.resultColumns.join(";"),
    String(artifact.outputProbeCount),
    artifact.outputProbes.join(";"),
    String(artifact.outputProbeLineCount),
    artifact.outputProbeLines.map(String).join(";"),
    String(artifact.outputDirectiveCount),
    artifact.outputDirectives.join(";"),
    String(artifact.outputDirectiveKindCount),
    artifact.outputDirectiveKinds.join(";"),
    String(artifact.outputDirectiveAnalysisKindCount),
    artifact.outputDirectiveAnalysisKinds.join(";"),
    String(artifact.outputDirectiveLineCount),
    artifact.outputDirectiveLines.map(String).join(";"),
    String(artifact.tableCount),
    artifact.tables.join(";"),
  ];
}

export function deckOutputPlanArtifactRecords(
  artifacts: readonly DeckOutputPlanArtifact[],
): ReadonlyArray<Record<string, string>> {
  return artifacts.map((artifact) => {
    const cells = deckOutputPlanArtifactCells(artifact);
    return Object.fromEntries(
      DECK_OUTPUT_PLAN_ARTIFACT_COLUMNS.map((column, index) => [
        column,
        cells[index] ?? "",
      ]),
    );
  });
}

export function formatDeckOutputPlanArtifactTable(
  artifacts: readonly DeckOutputPlanArtifact[],
): string {
  const rows = [DECK_OUTPUT_PLAN_ARTIFACT_COLUMNS.join("\t")];
  for (const artifact of artifacts) {
    rows.push(deckOutputPlanArtifactCells(artifact).join("\t"));
  }
  return `${rows.join("\n")}\n`;
}

export function formatDeckOutputPlanArtifactCsv(
  artifacts: readonly DeckOutputPlanArtifact[],
): string {
  const rows = [DECK_OUTPUT_PLAN_ARTIFACT_COLUMNS.join(",")];
  for (const artifact of artifacts) {
    rows.push(deckOutputPlanArtifactCells(artifact).map(formatCsvCell).join(","));
  }
  return `${rows.join("\n")}\n`;
}

export function formatDeckOutputPlanArtifactJson(
  artifacts: readonly DeckOutputPlanArtifact[],
): string {
  return `${JSON.stringify(deckOutputPlanArtifactRecords(artifacts))}\n`;
}

function deckOutputPlanArtifactBundle(
  plan: DeckAnalysisPlan,
  resultTable: string,
  outputProbes: readonly string[],
  outputProbeLines: readonly number[],
  outputDirectives: readonly string[],
  outputDirectiveAnalysisKinds: readonly string[],
  outputDirectiveLines: readonly number[],
  tables: readonly string[],
): {
  artifacts: DeckOutputPlanArtifact[];
  table: string;
  csv: string;
  json: string;
  records: ReadonlyArray<Record<string, string>>;
} {
  const artifacts = deckOutputPlanArtifacts(
    plan,
    deckTableRowCount(resultTable),
    deckTableColumns(resultTable),
    outputProbes,
    outputProbeLines,
    outputDirectives,
    outputDirectiveAnalysisKinds,
    outputDirectiveLines,
    tables,
  );
  return {
    artifacts,
    table: formatDeckOutputPlanArtifactTable(artifacts),
    csv: formatDeckOutputPlanArtifactCsv(artifacts),
    json: formatDeckOutputPlanArtifactJson(artifacts),
    records: deckOutputPlanArtifactRecords(artifacts),
  };
}

function deckTableColumns(table: string): string[] {
  const header = table.split("\n", 1)[0] ?? "";
  return header.length === 0 ? [] : header.split("\t");
}

function deckTableRowCount(table: string): number {
  const rows = deckTableRows(table);
  return rows.length === 0 ? 0 : rows.length - 1;
}

export function formatDeckRunArtifactTable(artifacts: readonly DeckRunArtifact[]): string {
  const rows = [DECK_RUN_ARTIFACT_COLUMNS.join("\t")];
  for (const artifact of artifacts) {
    rows.push(deckRunArtifactCells(artifact).join("\t"));
  }
  return `${rows.join("\n")}\n`;
}

function formatCsvCell(value: string): string {
  if (/[",\n\r]/u.test(value)) {
    return `"${value.replace(/"/gu, '""')}"`;
  }
  return value;
}

function deckTableRows(table: string): string[] {
  const rows = table.split(/\r?\n/u);
  if (rows[rows.length - 1] === "") {
    rows.pop();
  }
  return rows;
}

export function formatDeckTableCsv(table: string): string {
  const rows = deckTableRows(table);
  if (rows.length === 0) {
    return "";
  }
  return `${rows.map((row) => row.split("\t").map(formatCsvCell).join(",")).join("\n")}\n`;
}

export function deckTableRecords(table: string): Array<Record<string, string>> {
  const rows = deckTableRows(table);
  if (rows.length === 0) {
    return [];
  }
  const columns = rows[0]!.split("\t");
  return rows.slice(1).map((row) => {
    const cells = row.split("\t");
    return Object.fromEntries(columns.map((column, index) => [column, cells[index] ?? ""]));
  });
}

export function formatDeckTableJson(table: string): string {
  const records = deckTableRecords(table);
  return `${JSON.stringify(records)}\n`;
}

function deckTableArtifact(name: string, table: string): DeckTableArtifact {
  return {
    name,
    table,
    csv: formatDeckTableCsv(table),
    json: formatDeckTableJson(table),
    records: deckTableRecords(table),
  };
}

function deckTableArtifacts(
  plan: DeckAnalysisPlan,
  resultTable: string,
  measurementTable: string,
  fourierTable: string,
  runArtifactTable: string,
  measurements: readonly ProbeMeasurement[],
  fourier: readonly FourierResult[],
  controlPolicyArtifacts: readonly DeckControlPolicyArtifact[],
  controlPolicyArtifactTable: string,
  controlPolicySummaryArtifacts: readonly DeckControlPolicySummaryArtifact[],
  controlPolicySummaryArtifactTable: string,
  outputProbes: readonly string[],
  outputProbeLines: readonly number[],
  outputDirectives: readonly string[],
  outputDirectiveAnalysisKinds: readonly string[],
  outputDirectiveLines: readonly number[],
  tables: readonly string[],
): DeckTableArtifact[] {
  const artifacts = [deckTableArtifact("result", resultTable)];
  if (measurements.length > 0) {
    artifacts.push(deckTableArtifact("measurement", measurementTable));
  }
  if (fourier.length > 0) {
    artifacts.push(deckTableArtifact("fourier", fourierTable));
  }
  if (controlPolicyArtifacts.length > 0) {
    artifacts.push(deckTableArtifact("control-policy", controlPolicyArtifactTable));
  }
  if (controlPolicySummaryArtifacts.length > 0) {
    artifacts.push(
      deckTableArtifact("control-policy-summary", controlPolicySummaryArtifactTable),
    );
  }
  const outputPlanArtifactTable = deckOutputPlanArtifactBundle(
    plan,
    resultTable,
    outputProbes,
    outputProbeLines,
    outputDirectives,
    outputDirectiveAnalysisKinds,
    outputDirectiveLines,
    tables,
  ).table;
  artifacts.push(deckTableArtifact("output-plan", outputPlanArtifactTable));
  artifacts.push(deckTableArtifact("run-artifact", runArtifactTable));
  return artifacts;
}

export function formatDeckRawfileAscii(
  table: string,
  analysis: DeckAnalysisPlan["analysis"],
  rawfileOptions: readonly string[] = [],
): string {
  return formatDeckRawfileAsciiForProbes(table, analysis, rawfileOptions, []);
}

function formatDeckRawfileAsciiForProbes(
  table: string,
  analysis: DeckAnalysisPlan["analysis"],
  rawfileOptions: readonly string[],
  probes: readonly string[],
): string {
  const rows = deckTableRows(table);
  if (rows.length === 0) {
    return "";
  }
  const projectedRows = deckRawfileProjectRows(rows, probes);
  const columns = projectedRows[0]!.split("\t");
  const dataRows = projectedRows.slice(1).map((row) => row.split("\t"));
  const lines = [
    `Title: SPICE deck ${analysis} result`,
    "Date: deterministic",
    `Plotname: ${analysis}`,
    "Flags: real",
    `No. Variables: ${columns.length}`,
    `No. Points: ${dataRows.length}`,
    `Options: ${rawfileOptions.join(";")}`,
    "Variables:",
  ];
  columns.forEach((column, index) => {
    lines.push(`\t${index}\t${column}\treal`);
  });
  lines.push("Values:");
  dataRows.forEach((row, index) => {
    const padded = [...row, ...Array<string>(Math.max(0, columns.length - row.length)).fill("")];
    lines.push(`${index}\t${padded.slice(0, columns.length).join("\t")}`);
  });
  return `${lines.join("\n")}\n`;
}

function deckRawfileProjectRows(rows: readonly string[], probes: readonly string[]): string[] {
  const columns = rows[0]!.split("\t");
  if (probes.length === 0) {
    return [...rows];
  }
  const { selectedIndices } = deckRawfileProbeInventory(columns, probes);
  return rows.map((row) => {
    const cells = row.split("\t");
    return selectedIndices.map((index) => cells[index] ?? "").join("\t");
  });
}

function deckRawfileProbeInventory(
  columns: readonly string[],
  probes: readonly string[],
): {
  selectedIndices: number[];
  matchedProbes: string[];
  unmatchedProbes: string[];
} {
  const selectedIndices: number[] = [];
  const matchedProbes: string[] = [];
  const unmatchedProbes: string[] = [];
  if (columns.length > 0) {
    selectedIndices.push(0);
  }
  const normalizedColumns = columns.map((column) => column.toLowerCase());
  for (const probe of probes) {
    const index = normalizedColumns.indexOf(probe.toLowerCase());
    if (index !== -1 && !selectedIndices.includes(index)) {
      selectedIndices.push(index);
      matchedProbes.push(columns[index]!);
    } else if (index === -1) {
      unmatchedProbes.push(probe);
    }
  }
  return { selectedIndices, matchedProbes, unmatchedProbes };
}

function deckWriteMarkerParts(marker: string): { target: string; probes: string[] } | undefined {
  const parts = marker.trim().split(/\s+/u);
  if (parts.length < 2 || parts[0] !== "write") {
    return undefined;
  }
  return {
    target: parts[1]!,
    probes: parts.slice(2),
  };
}

const DECK_CONTROL_POLICY_CATEGORIES: Record<
  string,
  DeckControlPolicyArtifact["category"]
> = {
  SPICE_DECK_CONTROL_SCRIPT_COMMAND: "script",
  SPICE_DECK_CONTROL_WORKDIR_COMMAND: "workdir",
  SPICE_DECK_CONTROL_FLOW_COMMAND: "control-flow",
  SPICE_DECK_CONTROL_VARIABLE_COMMAND: "variable",
};

function deckControlPolicyArtifacts(netlist: string): DeckControlPolicyArtifact[] {
  const summary = analyzeDeckControls(netlist);
  const lines = netlist.split(/\r?\n/u);
  return summary.diagnostics.flatMap((diagnostic) => {
    const category = DECK_CONTROL_POLICY_CATEGORIES[diagnostic.code];
    if (category === undefined) {
      return [];
    }
    return [{
      lineNumber: diagnostic.lineNumber,
      category,
      command: lines[diagnostic.lineNumber - 1]?.trim() ?? "",
      code: diagnostic.code,
      severity: diagnostic.severity,
      message: diagnostic.message,
    }];
  });
}

const DECK_CONTROL_POLICY_ARTIFACT_COLUMNS = [
  "Line",
  "Category",
  "Command",
  "Code",
  "Severity",
  "Message",
] as const;

function deckControlPolicyArtifactCells(artifact: DeckControlPolicyArtifact): string[] {
  return [
    String(artifact.lineNumber),
    artifact.category,
    artifact.command,
    artifact.code,
    artifact.severity,
    artifact.message,
  ];
}

export function deckControlPolicyArtifactRecords(
  artifacts: readonly DeckControlPolicyArtifact[],
): Array<Record<string, string>> {
  return artifacts.map((artifact) => {
    const cells = deckControlPolicyArtifactCells(artifact);
    return Object.fromEntries(
      DECK_CONTROL_POLICY_ARTIFACT_COLUMNS.map((column, index) => [
        column,
        cells[index] ?? "",
      ]),
    );
  });
}

export function formatDeckControlPolicyArtifactTable(
  artifacts: readonly DeckControlPolicyArtifact[],
): string {
  const rows = [DECK_CONTROL_POLICY_ARTIFACT_COLUMNS.join("\t")];
  for (const artifact of artifacts) {
    rows.push(deckControlPolicyArtifactCells(artifact).join("\t"));
  }
  return `${rows.join("\n")}\n`;
}

export function formatDeckControlPolicyArtifactCsv(
  artifacts: readonly DeckControlPolicyArtifact[],
): string {
  const rows = [DECK_CONTROL_POLICY_ARTIFACT_COLUMNS.join(",")];
  for (const artifact of artifacts) {
    rows.push(deckControlPolicyArtifactCells(artifact).map(formatCsvCell).join(","));
  }
  return `${rows.join("\n")}\n`;
}

export function formatDeckControlPolicyArtifactJson(
  artifacts: readonly DeckControlPolicyArtifact[],
): string {
  return `${JSON.stringify(deckControlPolicyArtifactRecords(artifacts))}\n`;
}

function pushUniqueString(values: string[], value: string): void {
  if (!values.includes(value)) {
    values.push(value);
  }
}

function deckControlPolicySummaryArtifacts(
  artifacts: readonly DeckControlPolicyArtifact[],
): DeckControlPolicySummaryArtifact[] {
  const summaries: Array<{
    category: DeckControlPolicyArtifact["category"];
    artifactCount: number;
    lineNumbers: number[];
    commands: string[];
    codes: string[];
    severities: string[];
  }> = [];
  for (const artifact of artifacts) {
    const summary = summaries.find((candidate) => candidate.category === artifact.category);
    if (summary === undefined) {
      summaries.push({
        category: artifact.category,
        artifactCount: 1,
        lineNumbers: [artifact.lineNumber],
        commands: [artifact.command],
        codes: [artifact.code],
        severities: [artifact.severity],
      });
      continue;
    }
    summary.lineNumbers.push(artifact.lineNumber);
    summary.commands.push(artifact.command);
    pushUniqueString(summary.codes, artifact.code);
    pushUniqueString(summary.severities, artifact.severity);
    summary.artifactCount += 1;
  }
  return summaries;
}

const DECK_CONTROL_POLICY_SUMMARY_ARTIFACT_COLUMNS = [
  "Category",
  "Artifacts",
  "LineList",
  "CommandList",
  "CodeList",
  "SeverityList",
] as const;

function deckControlPolicySummaryArtifactCells(
  artifact: DeckControlPolicySummaryArtifact,
): string[] {
  return [
    artifact.category,
    String(artifact.artifactCount),
    artifact.lineNumbers.join(";"),
    artifact.commands.join(";"),
    artifact.codes.join(";"),
    artifact.severities.join(";"),
  ];
}

export function deckControlPolicySummaryArtifactRecords(
  artifacts: readonly DeckControlPolicySummaryArtifact[],
): Array<Record<string, string>> {
  return artifacts.map((artifact) => {
    const cells = deckControlPolicySummaryArtifactCells(artifact);
    return Object.fromEntries(
      DECK_CONTROL_POLICY_SUMMARY_ARTIFACT_COLUMNS.map((column, index) => [
        column,
        cells[index] ?? "",
      ]),
    );
  });
}

export function formatDeckControlPolicySummaryArtifactTable(
  artifacts: readonly DeckControlPolicySummaryArtifact[],
): string {
  const rows = [DECK_CONTROL_POLICY_SUMMARY_ARTIFACT_COLUMNS.join("\t")];
  for (const artifact of artifacts) {
    rows.push(deckControlPolicySummaryArtifactCells(artifact).join("\t"));
  }
  return `${rows.join("\n")}\n`;
}

export function formatDeckControlPolicySummaryArtifactCsv(
  artifacts: readonly DeckControlPolicySummaryArtifact[],
): string {
  const rows = [DECK_CONTROL_POLICY_SUMMARY_ARTIFACT_COLUMNS.join(",")];
  for (const artifact of artifacts) {
    rows.push(deckControlPolicySummaryArtifactCells(artifact).map(formatCsvCell).join(","));
  }
  return `${rows.join("\n")}\n`;
}

export function formatDeckControlPolicySummaryArtifactJson(
  artifacts: readonly DeckControlPolicySummaryArtifact[],
): string {
  return `${JSON.stringify(deckControlPolicySummaryArtifactRecords(artifacts))}\n`;
}

function deckRawfileArtifacts(
  plan: DeckAnalysisPlan,
  table: string,
  writeMarkers: readonly string[],
  rawfileOptions: readonly string[],
): DeckRawfileArtifact[] {
  const rows = deckTableRows(table);
  const columns = rows[0]?.split("\t") ?? [];
  return writeMarkers.flatMap((marker) => {
    const parts = deckWriteMarkerParts(marker);
    if (parts === undefined) {
      return [];
    }
    const { matchedProbes, unmatchedProbes } = deckRawfileProbeInventory(
      columns,
      parts.probes,
    );
    return [{
      target: parts.target,
      marker,
      probeCount: parts.probes.length,
      probes: [...parts.probes],
      matchedProbeCount: matchedProbes.length,
      matchedProbes,
      unmatchedProbeCount: unmatchedProbes.length,
      unmatchedProbes,
      optionCount: rawfileOptions.length,
      options: [...rawfileOptions],
      rawfile: formatDeckRawfileAsciiForProbes(
        table,
        plan.analysis,
        rawfileOptions,
        parts.probes,
      ),
    }];
  });
}

const DECK_RAWFILE_ARTIFACT_COLUMNS = [
  "Target",
  "Marker",
  "Probes",
  "ProbeList",
  "MatchedProbes",
  "MatchedProbeList",
  "UnmatchedProbes",
  "UnmatchedProbeList",
  "Options",
  "RawfileOptionList",
  "Bytes",
] as const;

function deckRawfileArtifactCells(artifact: DeckRawfileArtifact): string[] {
  return [
    artifact.target,
    artifact.marker,
    String(artifact.probeCount),
    artifact.probes.join(";"),
    String(artifact.matchedProbeCount),
    artifact.matchedProbes.join(";"),
    String(artifact.unmatchedProbeCount),
    artifact.unmatchedProbes.join(";"),
    String(artifact.optionCount),
    artifact.options.join(";"),
    String(artifact.rawfile.length),
  ];
}

export function deckRawfileArtifactRecords(
  artifacts: readonly DeckRawfileArtifact[],
): Array<Record<string, string>> {
  return artifacts.map((artifact) => {
    const cells = deckRawfileArtifactCells(artifact);
    return Object.fromEntries(
      DECK_RAWFILE_ARTIFACT_COLUMNS.map((column, index) => [column, cells[index] ?? ""]),
    );
  });
}

export function formatDeckRawfileArtifactTable(
  artifacts: readonly DeckRawfileArtifact[],
): string {
  const rows = [DECK_RAWFILE_ARTIFACT_COLUMNS.join("\t")];
  for (const artifact of artifacts) {
    rows.push(deckRawfileArtifactCells(artifact).join("\t"));
  }
  return `${rows.join("\n")}\n`;
}

export function formatDeckRawfileArtifactCsv(
  artifacts: readonly DeckRawfileArtifact[],
): string {
  const rows = [DECK_RAWFILE_ARTIFACT_COLUMNS.join(",")];
  for (const artifact of artifacts) {
    rows.push(deckRawfileArtifactCells(artifact).map(formatCsvCell).join(","));
  }
  return `${rows.join("\n")}\n`;
}

export function formatDeckRawfileArtifactJson(
  artifacts: readonly DeckRawfileArtifact[],
): string {
  return `${JSON.stringify(deckRawfileArtifactRecords(artifacts))}\n`;
}

export function formatDeckWrdataAscii(
  table: string,
  probes: readonly string[] = [],
  rawfileOptions: readonly string[] = [],
): string {
  const rows = deckTableRows(table);
  if (rows.length === 0) {
    return "";
  }
  const projectedRows = deckWrdataProjectRows(rows, probes);
  const columns = projectedRows[0]!.split("\t");
  const lines = [
    "# SPICE deck wrdata artifact",
    `Probes: ${probes.join(";")}`,
  ];
  if (rawfileOptions.length > 0) {
    lines.push(`Options: ${rawfileOptions.join(";")}`);
  }
  const normalizedOptions = new Set(rawfileOptions.map((option) => option.toLowerCase()));
  if (normalizedOptions.has("set wr_vecnames")) {
    lines.push(`VectorNames: ${columns.join(";")}`);
  }
  if (normalizedOptions.has("set wr_singlescale") && columns.length > 0) {
    lines.push(`Scale: ${columns[0]}`);
  }
  lines.push(...projectedRows);
  return `${lines.join("\n")}\n`;
}

function deckWrdataProjectRows(rows: readonly string[], probes: readonly string[]): string[] {
  const columns = rows[0]!.split("\t");
  if (probes.length === 0) {
    return [...rows];
  }
  const { selectedIndices } = deckWrdataProbeInventory(columns, probes);
  return rows.map((row) => {
    const cells = row.split("\t");
    return selectedIndices.map((index) => cells[index] ?? "").join("\t");
  });
}

function deckWrdataProbeInventory(
  columns: readonly string[],
  probes: readonly string[],
): {
  selectedIndices: number[];
  matchedProbes: string[];
  unmatchedProbes: string[];
} {
  const selectedIndices: number[] = [];
  const matchedProbes: string[] = [];
  const unmatchedProbes: string[] = [];
  if (columns.length > 0) {
    selectedIndices.push(0);
  }
  const normalizedColumns = columns.map((column) => column.toLowerCase());
  for (const probe of probes) {
    const index = normalizedColumns.indexOf(probe.toLowerCase());
    if (index !== -1 && !selectedIndices.includes(index)) {
      selectedIndices.push(index);
      matchedProbes.push(columns[index]!);
    } else if (index === -1) {
      unmatchedProbes.push(probe);
    }
  }
  return { selectedIndices, matchedProbes, unmatchedProbes };
}

function deckWrdataMarkerParts(marker: string): { target: string; probes: string[] } | undefined {
  const parts = marker.trim().split(/\s+/u);
  if (parts.length < 2 || parts[0] !== "wrdata") {
    return undefined;
  }
  return {
    target: parts[1]!,
    probes: parts.slice(2),
  };
}

function deckWrdataArtifacts(
  table: string,
  writeMarkers: readonly string[],
  rawfileOptions: readonly string[],
): DeckWrdataArtifact[] {
  const rows = deckTableRows(table);
  const columns = rows[0]?.split("\t") ?? [];
  return writeMarkers.flatMap((marker) => {
    const parts = deckWrdataMarkerParts(marker);
    if (parts === undefined) {
      return [];
    }
    const { matchedProbes, unmatchedProbes } = deckWrdataProbeInventory(
      columns,
      parts.probes,
    );
    return [{
      target: parts.target,
      marker,
      probeCount: parts.probes.length,
      probes: [...parts.probes],
      matchedProbeCount: matchedProbes.length,
      matchedProbes,
      unmatchedProbeCount: unmatchedProbes.length,
      unmatchedProbes,
      optionCount: rawfileOptions.length,
      options: [...rawfileOptions],
      datafile: formatDeckWrdataAscii(table, parts.probes, rawfileOptions),
    }];
  });
}

const DECK_WRDATA_ARTIFACT_COLUMNS = [
  "Target",
  "Marker",
  "Probes",
  "ProbeList",
  "MatchedProbes",
  "MatchedProbeList",
  "UnmatchedProbes",
  "UnmatchedProbeList",
  "Options",
  "RawfileOptionList",
  "Bytes",
] as const;

function deckWrdataArtifactCells(artifact: DeckWrdataArtifact): string[] {
  return [
    artifact.target,
    artifact.marker,
    String(artifact.probeCount),
    artifact.probes.join(";"),
    String(artifact.matchedProbeCount),
    artifact.matchedProbes.join(";"),
    String(artifact.unmatchedProbeCount),
    artifact.unmatchedProbes.join(";"),
    String(artifact.optionCount),
    artifact.options.join(";"),
    String(artifact.datafile.length),
  ];
}

export function deckWrdataArtifactRecords(
  artifacts: readonly DeckWrdataArtifact[],
): Array<Record<string, string>> {
  return artifacts.map((artifact) => {
    const cells = deckWrdataArtifactCells(artifact);
    return Object.fromEntries(
      DECK_WRDATA_ARTIFACT_COLUMNS.map((column, index) => [column, cells[index] ?? ""]),
    );
  });
}

export function formatDeckWrdataArtifactTable(
  artifacts: readonly DeckWrdataArtifact[],
): string {
  const rows = [DECK_WRDATA_ARTIFACT_COLUMNS.join("\t")];
  for (const artifact of artifacts) {
    rows.push(deckWrdataArtifactCells(artifact).join("\t"));
  }
  return `${rows.join("\n")}\n`;
}

export function formatDeckWrdataArtifactCsv(
  artifacts: readonly DeckWrdataArtifact[],
): string {
  const rows = [DECK_WRDATA_ARTIFACT_COLUMNS.join(",")];
  for (const artifact of artifacts) {
    rows.push(deckWrdataArtifactCells(artifact).map(formatCsvCell).join(","));
  }
  return `${rows.join("\n")}\n`;
}

export function formatDeckWrdataArtifactJson(
  artifacts: readonly DeckWrdataArtifact[],
): string {
  return `${JSON.stringify(deckWrdataArtifactRecords(artifacts))}\n`;
}

export function formatDeckRunArtifactCsv(artifacts: readonly DeckRunArtifact[]): string {
  const rows = [DECK_RUN_ARTIFACT_COLUMNS.join(",")];
  for (const artifact of artifacts) {
    rows.push(deckRunArtifactCells(artifact).map(formatCsvCell).join(","));
  }
  return `${rows.join("\n")}\n`;
}

export function formatDeckRunArtifactJson(artifacts: readonly DeckRunArtifact[]): string {
  return `${JSON.stringify(deckRunArtifactRecords(artifacts))}\n`;
}

function selectDeckMeasurementCardsForAnalysis(
  netlist: string,
  analysis: DeckAnalysisPlan["analysis"],
): DeckMeasurementCard[] {
  const summary = resolveDeckMeasurements(netlist);
  if (summary.diagnostics.length > 0) {
    const diagnostic = summary.diagnostics[0]!;
    throw invalidElement(
      "runDeckAnalysis",
      `line ${diagnostic.lineNumber}: ${diagnostic.message}`,
    );
  }
  return summary.measurements.filter((measurement) =>
    measurement.analysis === analysis ||
    (analysis === "tran" && measurement.analysis === "transient")
  );
}

function selectDeckFourierCardsForAnalysis(
  netlist: string,
  analysis: DeckAnalysisPlan["analysis"],
): DeckFourierCard[] {
  const summary = resolveDeckFourier(netlist);
  if (summary.diagnostics.length > 0) {
    const diagnostic = summary.diagnostics[0]!;
    throw invalidElement(
      "runDeckAnalysis",
      `line ${diagnostic.lineNumber}: ${diagnostic.message}`,
    );
  }
  return analysis === "tran" ? [...summary.fourier] : [];
}

export function runDeckAnalysis(
  circuit: Circuit,
  netlist: string,
  analysis?: string,
): DeckAnalysisExecution {
  const plan = selectDeckAnalysisPlan(netlist, analysis);
  return runDeckAnalysisPlan(circuit, netlist, plan);
}

export function runDeck(circuit: Circuit, netlist: string): DeckExecution {
  const plans = deckAnalysisPlansForExecution(netlist, "runDeck");
  const executions = plans.map((plan) => runDeckAnalysisPlan(circuit, netlist, plan));
  const runArtifacts = executions.flatMap((execution) => execution.runArtifacts);
  return {
    executionCount: executions.length,
    analysisOrder: plans.map((plan) => plan.analysis),
    analysisDirectives: plans.map((plan) => plan.directive),
    executions,
    runArtifactCount: runArtifacts.length,
    runArtifacts,
    runArtifactTable: formatDeckRunArtifactTable(runArtifacts),
    runArtifactCsv: formatDeckRunArtifactCsv(runArtifacts),
    runArtifactJson: formatDeckRunArtifactJson(runArtifacts),
    runArtifactRecords: deckRunArtifactRecords(runArtifacts),
  };
}

function deckAnalysisPlansForExecution(netlist: string, context: string): DeckAnalysisPlan[] {
  const summary = resolveDeckAnalyses(netlist);
  if (summary.diagnostics.length > 0) {
    const diagnostic = summary.diagnostics[0]!;
    throw invalidElement(
      context,
      `line ${diagnostic.lineNumber}: ${diagnostic.message}`,
    );
  }
  if (summary.analyses.length === 0) {
    return [{
      directive: ".op",
      analysis: "op",
      lineNumber: 0,
      useInitialConditions: false,
    }];
  }
  return [...summary.analyses];
}

function runDeckAnalysisPlan(
  circuit: Circuit,
  netlist: string,
  plan: DeckAnalysisPlan,
): DeckAnalysisExecution {
  const diagnosticCodes = deckRunDiagnosticCodes(netlist, plan);
  const controlLines = deckControlLines(netlist);
  const writeMarkers = deckControlWriteMarkers(netlist);
  const rawfileOptions = deckControlRawfileOptions(netlist);
  const controlPolicyArtifacts = deckControlPolicyArtifacts(netlist);
  const controlPolicyArtifactTable = formatDeckControlPolicyArtifactTable(controlPolicyArtifacts);
  const controlPolicyArtifactCsv = formatDeckControlPolicyArtifactCsv(controlPolicyArtifacts);
  const controlPolicyArtifactJson = formatDeckControlPolicyArtifactJson(controlPolicyArtifacts);
  const controlPolicyArtifactRecords = deckControlPolicyArtifactRecords(controlPolicyArtifacts);
  const controlPolicySummaryArtifacts = deckControlPolicySummaryArtifacts(controlPolicyArtifacts);
  const controlPolicySummaryArtifactTable = formatDeckControlPolicySummaryArtifactTable(
    controlPolicySummaryArtifacts,
  );
  const controlPolicySummaryArtifactCsv = formatDeckControlPolicySummaryArtifactCsv(
    controlPolicySummaryArtifacts,
  );
  const controlPolicySummaryArtifactJson = formatDeckControlPolicySummaryArtifactJson(
    controlPolicySummaryArtifacts,
  );
  const controlPolicySummaryArtifactRecords = deckControlPolicySummaryArtifactRecords(
    controlPolicySummaryArtifacts,
  );
  const analysisDirectives = deckAnalysisDirectives(plan);
  const {
    analysisKinds: deckAnalysisKinds,
    directives: deckAnalysisDirectiveInventory,
  } = deckAnalysisInventory(netlist);
  if (plan.analysis === "op") {
    const result = dcOp(circuit);
    const table = formatDeckOpTable(result, netlist);
    selectDeckMeasurementCardsForAnalysis(netlist, plan.analysis);
    selectDeckFourierCardsForAnalysis(netlist, plan.analysis);
    const measurements: ProbeMeasurement[] = [];
    const fourier: FourierResult[] = [];
    const outputProbes = selectDeckOutputProbes(netlist, plan.analysis);
    const outputProbeLines = selectDeckOutputProbeLines(netlist, plan.analysis);
    const outputDirectives = selectDeckOutputDirectives(netlist, plan.analysis);
    const outputDirectiveAnalysisKinds = selectDeckOutputDirectiveAnalysisKinds(
      netlist,
      plan.analysis,
    );
    const outputDirectiveLines = selectDeckOutputDirectiveLines(netlist, plan.analysis);
    const runArtifacts = deckRunArtifacts(
      plan,
      result,
      deckTableColumns(table),
      outputProbes,
      outputDirectives,
      measurements,
      fourier,
      controlLines,
      writeMarkers,
      rawfileOptions,
      diagnosticCodes,
      controlPolicyArtifacts,
      deckAnalysisKinds,
      deckAnalysisDirectiveInventory,
    );
    const tables = deckStableTables(measurements, fourier, controlPolicyArtifacts);
    const outputPlanArtifactBundle = deckOutputPlanArtifactBundle(
      plan,
      table,
      outputProbes,
      outputProbeLines,
      outputDirectives,
      outputDirectiveAnalysisKinds,
      outputDirectiveLines,
      tables,
    );
    const measurementTable = formatMeasurementTable(measurements);
    const fourierTable = formatDeckFourierTable(fourier);
    const runArtifactTable = formatDeckRunArtifactTable(runArtifacts);
    const tableArtifacts = deckTableArtifacts(
      plan,
      table,
      measurementTable,
      fourierTable,
      runArtifactTable,
      measurements,
      fourier,
      controlPolicyArtifacts,
      controlPolicyArtifactTable,
      controlPolicySummaryArtifacts,
      controlPolicySummaryArtifactTable,
      outputProbes,
      outputProbeLines,
      outputDirectives,
      outputDirectiveAnalysisKinds,
      outputDirectiveLines,
      tables,
    );
    const rawfileArtifacts = deckRawfileArtifacts(plan, table, writeMarkers, rawfileOptions);
    const rawfileArtifactTable = formatDeckRawfileArtifactTable(rawfileArtifacts);
    const rawfileArtifactCsv = formatDeckRawfileArtifactCsv(rawfileArtifacts);
    const rawfileArtifactJson = formatDeckRawfileArtifactJson(rawfileArtifacts);
    const rawfileArtifactRecords = deckRawfileArtifactRecords(rawfileArtifacts);
    const wrdataArtifacts = deckWrdataArtifacts(table, writeMarkers, rawfileOptions);
    const wrdataArtifactTable = formatDeckWrdataArtifactTable(wrdataArtifacts);
    const wrdataArtifactCsv = formatDeckWrdataArtifactCsv(wrdataArtifacts);
    const wrdataArtifactJson = formatDeckWrdataArtifactJson(wrdataArtifacts);
    const wrdataArtifactRecords = deckWrdataArtifactRecords(wrdataArtifacts);
    return {
      plan,
      result,
      table,
      outputProbes,
      outputDirectives,
      analysisDirectives,
      deckAnalysisKindCount: deckAnalysisKinds.length,
      deckAnalysisKinds: [...deckAnalysisKinds],
      deckAnalysisDirectiveCount: deckAnalysisDirectiveInventory.length,
      deckAnalysisDirectives: [...deckAnalysisDirectiveInventory],
      outputPlanArtifactCount: outputPlanArtifactBundle.artifacts.length,
      outputPlanArtifacts: outputPlanArtifactBundle.artifacts,
      outputPlanArtifactTable: outputPlanArtifactBundle.table,
      outputPlanArtifactCsv: outputPlanArtifactBundle.csv,
      outputPlanArtifactJson: outputPlanArtifactBundle.json,
      outputPlanArtifactRecords: outputPlanArtifactBundle.records,
      controlLineCount: controlLines.length,
      controlLines: [...controlLines],
      writeMarkerCount: writeMarkers.length,
      writeMarkers: [...writeMarkers],
      rawfileOptionCount: rawfileOptions.length,
      rawfileOptions: [...rawfileOptions],
      controlPolicyArtifactCount: controlPolicyArtifacts.length,
      controlPolicyArtifacts,
      controlPolicyArtifactTable,
      controlPolicyArtifactCsv,
      controlPolicyArtifactJson,
      controlPolicyArtifactRecords,
      controlPolicySummaryArtifactCount: controlPolicySummaryArtifacts.length,
      controlPolicySummaryArtifacts,
      controlPolicySummaryArtifactTable,
      controlPolicySummaryArtifactCsv,
      controlPolicySummaryArtifactJson,
      controlPolicySummaryArtifactRecords,
      rawfileArtifactCount: rawfileArtifacts.length,
      rawfileArtifacts,
      rawfileArtifactTable,
      rawfileArtifactCsv,
      rawfileArtifactJson,
      rawfileArtifactRecords,
      wrdataArtifactCount: wrdataArtifacts.length,
      wrdataArtifacts,
      wrdataArtifactTable,
      wrdataArtifactCsv,
      wrdataArtifactJson,
      wrdataArtifactRecords,
      diagnosticCount: diagnosticCodes.length,
      diagnosticCodes: [...diagnosticCodes],
      tableCount: tables.length,
      tables,
      tableArtifacts,
      measurements,
      measurementTable,
      fourier,
      fourierTable,
      runArtifacts,
      runArtifactTable,
    };
  }
  if (plan.analysis === "dc") {
    const sourceName = requireDeckPlanString(plan.sourceName, plan, "sourceName");
    const start = requireDeckPlanNumber(plan.startValue, plan, "startValue");
    const stop = requireDeckPlanNumber(plan.stopValue, plan, "stopValue");
    const step = requireDeckPlanNumber(plan.stepValue, plan, "stepValue");
    const result = dcSweep(circuit, sourceName, start, stop, step);
    const table = formatDeckDcSweepTable(sourceName, result, netlist);
    const measurements = measureDcSweepCards(
      result,
      selectDeckMeasurementCardsForAnalysis(netlist, plan.analysis),
    );
    selectDeckFourierCardsForAnalysis(netlist, plan.analysis);
    const fourier: FourierResult[] = [];
    const outputProbes = selectDeckOutputProbes(netlist, plan.analysis);
    const outputProbeLines = selectDeckOutputProbeLines(netlist, plan.analysis);
    const outputDirectives = selectDeckOutputDirectives(netlist, plan.analysis);
    const outputDirectiveAnalysisKinds = selectDeckOutputDirectiveAnalysisKinds(
      netlist,
      plan.analysis,
    );
    const outputDirectiveLines = selectDeckOutputDirectiveLines(netlist, plan.analysis);
    const runArtifacts = deckRunArtifacts(
      plan,
      result,
      deckTableColumns(table),
      outputProbes,
      outputDirectives,
      measurements,
      fourier,
      controlLines,
      writeMarkers,
      rawfileOptions,
      diagnosticCodes,
      controlPolicyArtifacts,
      deckAnalysisKinds,
      deckAnalysisDirectiveInventory,
    );
    const tables = deckStableTables(measurements, fourier, controlPolicyArtifacts);

    const outputPlanArtifactBundle = deckOutputPlanArtifactBundle(
      plan,
      table,
      outputProbes,
      outputProbeLines,
      outputDirectives,
      outputDirectiveAnalysisKinds,
      outputDirectiveLines,
      tables,
    );
    const measurementTable = formatMeasurementTable(measurements);
    const fourierTable = formatDeckFourierTable(fourier);
    const runArtifactTable = formatDeckRunArtifactTable(runArtifacts);
    const tableArtifacts = deckTableArtifacts(
      plan,
      table,
      measurementTable,
      fourierTable,
      runArtifactTable,
      measurements,
      fourier,
      controlPolicyArtifacts,
      controlPolicyArtifactTable,
      controlPolicySummaryArtifacts,
      controlPolicySummaryArtifactTable,
      outputProbes,
      outputProbeLines,
      outputDirectives,
      outputDirectiveAnalysisKinds,
      outputDirectiveLines,
      tables,
    );
    const rawfileArtifacts = deckRawfileArtifacts(plan, table, writeMarkers, rawfileOptions);
    const rawfileArtifactTable = formatDeckRawfileArtifactTable(rawfileArtifacts);
    const rawfileArtifactCsv = formatDeckRawfileArtifactCsv(rawfileArtifacts);
    const rawfileArtifactJson = formatDeckRawfileArtifactJson(rawfileArtifacts);
    const rawfileArtifactRecords = deckRawfileArtifactRecords(rawfileArtifacts);
    const wrdataArtifacts = deckWrdataArtifacts(table, writeMarkers, rawfileOptions);
    const wrdataArtifactTable = formatDeckWrdataArtifactTable(wrdataArtifacts);
    const wrdataArtifactCsv = formatDeckWrdataArtifactCsv(wrdataArtifacts);
    const wrdataArtifactJson = formatDeckWrdataArtifactJson(wrdataArtifacts);
    const wrdataArtifactRecords = deckWrdataArtifactRecords(wrdataArtifacts);
    return {
      plan,
      result,
      table,
      outputProbes,
      outputDirectives,
      analysisDirectives,
      deckAnalysisKindCount: deckAnalysisKinds.length,
      deckAnalysisKinds: [...deckAnalysisKinds],
      deckAnalysisDirectiveCount: deckAnalysisDirectiveInventory.length,
      deckAnalysisDirectives: [...deckAnalysisDirectiveInventory],
      outputPlanArtifactCount: outputPlanArtifactBundle.artifacts.length,
      outputPlanArtifacts: outputPlanArtifactBundle.artifacts,
      outputPlanArtifactTable: outputPlanArtifactBundle.table,
      outputPlanArtifactCsv: outputPlanArtifactBundle.csv,
      outputPlanArtifactJson: outputPlanArtifactBundle.json,
      outputPlanArtifactRecords: outputPlanArtifactBundle.records,
      controlLineCount: controlLines.length,
      controlLines: [...controlLines],
      writeMarkerCount: writeMarkers.length,
      writeMarkers: [...writeMarkers],
      rawfileOptionCount: rawfileOptions.length,
      rawfileOptions: [...rawfileOptions],
      controlPolicyArtifactCount: controlPolicyArtifacts.length,
      controlPolicyArtifacts,
      controlPolicyArtifactTable,
      controlPolicyArtifactCsv,
      controlPolicyArtifactJson,
      controlPolicyArtifactRecords,
      controlPolicySummaryArtifactCount: controlPolicySummaryArtifacts.length,
      controlPolicySummaryArtifacts,
      controlPolicySummaryArtifactTable,
      controlPolicySummaryArtifactCsv,
      controlPolicySummaryArtifactJson,
      controlPolicySummaryArtifactRecords,
      rawfileArtifactCount: rawfileArtifacts.length,
      rawfileArtifacts,
      rawfileArtifactTable,
      rawfileArtifactCsv,
      rawfileArtifactJson,
      rawfileArtifactRecords,
      wrdataArtifactCount: wrdataArtifacts.length,
      wrdataArtifacts,
      wrdataArtifactTable,
      wrdataArtifactCsv,
      wrdataArtifactJson,
      wrdataArtifactRecords,
      diagnosticCount: diagnosticCodes.length,
      diagnosticCodes: [...diagnosticCodes],
      tableCount: tables.length,
      tables,
      tableArtifacts,
      measurements,
      measurementTable,
      fourier,
      fourierTable,
      runArtifacts,
      runArtifactTable,
    };
  }
  if (plan.analysis === "ac") {
    const sweepKind = requireDeckPlanString(plan.sweepKind, plan, "sweepKind");
    const pointCount = requireDeckPlanInteger(plan.pointCount, plan, "pointCount");
    const startFrequencyHz = requireDeckPlanNumber(
      plan.startFrequencyHz,
      plan,
      "startFrequencyHz",
    );
    const stopFrequencyHz = requireDeckPlanNumber(plan.stopFrequencyHz, plan, "stopFrequencyHz");
    const result = runDeckAcSweep(
      circuit,
      plan,
      sweepKind,
      pointCount,
      startFrequencyHz,
      stopFrequencyHz,
    );
    const table = formatDeckAcTable(result, netlist);
    const measurements = measureAcSweepCards(
      result,
      selectDeckMeasurementCardsForAnalysis(netlist, plan.analysis),
    );
    selectDeckFourierCardsForAnalysis(netlist, plan.analysis);
    const fourier: FourierResult[] = [];
    const outputProbes = selectDeckOutputProbes(netlist, plan.analysis);
    const outputProbeLines = selectDeckOutputProbeLines(netlist, plan.analysis);
    const outputDirectives = selectDeckOutputDirectives(netlist, plan.analysis);
    const outputDirectiveAnalysisKinds = selectDeckOutputDirectiveAnalysisKinds(
      netlist,
      plan.analysis,
    );
    const outputDirectiveLines = selectDeckOutputDirectiveLines(netlist, plan.analysis);
    const runArtifacts = deckRunArtifacts(
      plan,
      result,
      deckTableColumns(table),
      outputProbes,
      outputDirectives,
      measurements,
      fourier,
      controlLines,
      writeMarkers,
      rawfileOptions,
      diagnosticCodes,
      controlPolicyArtifacts,
      deckAnalysisKinds,
      deckAnalysisDirectiveInventory,
    );
    const tables = deckStableTables(measurements, fourier, controlPolicyArtifacts);

    const outputPlanArtifactBundle = deckOutputPlanArtifactBundle(
      plan,
      table,
      outputProbes,
      outputProbeLines,
      outputDirectives,
      outputDirectiveAnalysisKinds,
      outputDirectiveLines,
      tables,
    );
    const measurementTable = formatMeasurementTable(measurements);
    const fourierTable = formatDeckFourierTable(fourier);
    const runArtifactTable = formatDeckRunArtifactTable(runArtifacts);
    const tableArtifacts = deckTableArtifacts(
      plan,
      table,
      measurementTable,
      fourierTable,
      runArtifactTable,
      measurements,
      fourier,
      controlPolicyArtifacts,
      controlPolicyArtifactTable,
      controlPolicySummaryArtifacts,
      controlPolicySummaryArtifactTable,
      outputProbes,
      outputProbeLines,
      outputDirectives,
      outputDirectiveAnalysisKinds,
      outputDirectiveLines,
      tables,
    );
    const rawfileArtifacts = deckRawfileArtifacts(plan, table, writeMarkers, rawfileOptions);
    const rawfileArtifactTable = formatDeckRawfileArtifactTable(rawfileArtifacts);
    const rawfileArtifactCsv = formatDeckRawfileArtifactCsv(rawfileArtifacts);
    const rawfileArtifactJson = formatDeckRawfileArtifactJson(rawfileArtifacts);
    const rawfileArtifactRecords = deckRawfileArtifactRecords(rawfileArtifacts);
    const wrdataArtifacts = deckWrdataArtifacts(table, writeMarkers, rawfileOptions);
    const wrdataArtifactTable = formatDeckWrdataArtifactTable(wrdataArtifacts);
    const wrdataArtifactCsv = formatDeckWrdataArtifactCsv(wrdataArtifacts);
    const wrdataArtifactJson = formatDeckWrdataArtifactJson(wrdataArtifacts);
    const wrdataArtifactRecords = deckWrdataArtifactRecords(wrdataArtifacts);
    return {
      plan,
      result,
      table,
      outputProbes,
      outputDirectives,
      analysisDirectives,
      deckAnalysisKindCount: deckAnalysisKinds.length,
      deckAnalysisKinds: [...deckAnalysisKinds],
      deckAnalysisDirectiveCount: deckAnalysisDirectiveInventory.length,
      deckAnalysisDirectives: [...deckAnalysisDirectiveInventory],
      outputPlanArtifactCount: outputPlanArtifactBundle.artifacts.length,
      outputPlanArtifacts: outputPlanArtifactBundle.artifacts,
      outputPlanArtifactTable: outputPlanArtifactBundle.table,
      outputPlanArtifactCsv: outputPlanArtifactBundle.csv,
      outputPlanArtifactJson: outputPlanArtifactBundle.json,
      outputPlanArtifactRecords: outputPlanArtifactBundle.records,
      controlLineCount: controlLines.length,
      controlLines: [...controlLines],
      writeMarkerCount: writeMarkers.length,
      writeMarkers: [...writeMarkers],
      rawfileOptionCount: rawfileOptions.length,
      rawfileOptions: [...rawfileOptions],
      controlPolicyArtifactCount: controlPolicyArtifacts.length,
      controlPolicyArtifacts,
      controlPolicyArtifactTable,
      controlPolicyArtifactCsv,
      controlPolicyArtifactJson,
      controlPolicyArtifactRecords,
      controlPolicySummaryArtifactCount: controlPolicySummaryArtifacts.length,
      controlPolicySummaryArtifacts,
      controlPolicySummaryArtifactTable,
      controlPolicySummaryArtifactCsv,
      controlPolicySummaryArtifactJson,
      controlPolicySummaryArtifactRecords,
      rawfileArtifactCount: rawfileArtifacts.length,
      rawfileArtifacts,
      rawfileArtifactTable,
      rawfileArtifactCsv,
      rawfileArtifactJson,
      rawfileArtifactRecords,
      wrdataArtifactCount: wrdataArtifacts.length,
      wrdataArtifacts,
      wrdataArtifactTable,
      wrdataArtifactCsv,
      wrdataArtifactJson,
      wrdataArtifactRecords,
      diagnosticCount: diagnosticCodes.length,
      diagnosticCodes: [...diagnosticCodes],
      tableCount: tables.length,
      tables,
      tableArtifacts,
      measurements,
      measurementTable,
      fourier,
      fourierTable,
      runArtifacts,
      runArtifactTable,
    };
  }
  if (plan.analysis === "tran") {
    const stepTime = requireDeckPlanNumber(plan.stepTime, plan, "stepTime");
    const stopTime = requireDeckPlanNumber(plan.stopTime, plan, "stopTime");
    const runStep = plan.maxStep !== undefined ? Math.min(stepTime, plan.maxStep) : stepTime;
    const result = sampleTransientPointsPrintStep(
      transient(circuit, runStep, stopTime),
      stepTime,
      plan.startTime,
      stopTime,
    );
    const measurements = measureTransientCards(
      result,
      selectDeckMeasurementCardsForAnalysis(netlist, plan.analysis),
    );
    const table = formatDeckTransientTable(result, netlist);
    const fourier = fourierTransientCards(
      result,
      selectDeckFourierCardsForAnalysis(netlist, plan.analysis),
    );
    const outputProbes = selectDeckOutputProbes(netlist, plan.analysis);
    const outputProbeLines = selectDeckOutputProbeLines(netlist, plan.analysis);
    const outputDirectives = selectDeckOutputDirectives(netlist, plan.analysis);
    const outputDirectiveAnalysisKinds = selectDeckOutputDirectiveAnalysisKinds(
      netlist,
      plan.analysis,
    );
    const outputDirectiveLines = selectDeckOutputDirectiveLines(netlist, plan.analysis);
    const runArtifacts = deckRunArtifacts(
      plan,
      result,
      deckTableColumns(table),
      outputProbes,
      outputDirectives,
      measurements,
      fourier,
      controlLines,
      writeMarkers,
      rawfileOptions,
      diagnosticCodes,
      controlPolicyArtifacts,
      deckAnalysisKinds,
      deckAnalysisDirectiveInventory,
    );
    const tables = deckStableTables(measurements, fourier, controlPolicyArtifacts);

    const outputPlanArtifactBundle = deckOutputPlanArtifactBundle(
      plan,
      table,
      outputProbes,
      outputProbeLines,
      outputDirectives,
      outputDirectiveAnalysisKinds,
      outputDirectiveLines,
      tables,
    );
    const measurementTable = formatMeasurementTable(measurements);
    const fourierTable = formatDeckFourierTable(fourier);
    const runArtifactTable = formatDeckRunArtifactTable(runArtifacts);
    const tableArtifacts = deckTableArtifacts(
      plan,
      table,
      measurementTable,
      fourierTable,
      runArtifactTable,
      measurements,
      fourier,
      controlPolicyArtifacts,
      controlPolicyArtifactTable,
      controlPolicySummaryArtifacts,
      controlPolicySummaryArtifactTable,
      outputProbes,
      outputProbeLines,
      outputDirectives,
      outputDirectiveAnalysisKinds,
      outputDirectiveLines,
      tables,
    );
    const rawfileArtifacts = deckRawfileArtifacts(plan, table, writeMarkers, rawfileOptions);
    const rawfileArtifactTable = formatDeckRawfileArtifactTable(rawfileArtifacts);
    const rawfileArtifactCsv = formatDeckRawfileArtifactCsv(rawfileArtifacts);
    const rawfileArtifactJson = formatDeckRawfileArtifactJson(rawfileArtifacts);
    const rawfileArtifactRecords = deckRawfileArtifactRecords(rawfileArtifacts);
    const wrdataArtifacts = deckWrdataArtifacts(table, writeMarkers, rawfileOptions);
    const wrdataArtifactTable = formatDeckWrdataArtifactTable(wrdataArtifacts);
    const wrdataArtifactCsv = formatDeckWrdataArtifactCsv(wrdataArtifacts);
    const wrdataArtifactJson = formatDeckWrdataArtifactJson(wrdataArtifacts);
    const wrdataArtifactRecords = deckWrdataArtifactRecords(wrdataArtifacts);
    return {
      plan,
      result,
      table,
      outputProbes,
      outputDirectives,
      analysisDirectives,
      deckAnalysisKindCount: deckAnalysisKinds.length,
      deckAnalysisKinds: [...deckAnalysisKinds],
      deckAnalysisDirectiveCount: deckAnalysisDirectiveInventory.length,
      deckAnalysisDirectives: [...deckAnalysisDirectiveInventory],
      outputPlanArtifactCount: outputPlanArtifactBundle.artifacts.length,
      outputPlanArtifacts: outputPlanArtifactBundle.artifacts,
      outputPlanArtifactTable: outputPlanArtifactBundle.table,
      outputPlanArtifactCsv: outputPlanArtifactBundle.csv,
      outputPlanArtifactJson: outputPlanArtifactBundle.json,
      outputPlanArtifactRecords: outputPlanArtifactBundle.records,
      controlLineCount: controlLines.length,
      controlLines: [...controlLines],
      writeMarkerCount: writeMarkers.length,
      writeMarkers: [...writeMarkers],
      rawfileOptionCount: rawfileOptions.length,
      rawfileOptions: [...rawfileOptions],
      controlPolicyArtifactCount: controlPolicyArtifacts.length,
      controlPolicyArtifacts,
      controlPolicyArtifactTable,
      controlPolicyArtifactCsv,
      controlPolicyArtifactJson,
      controlPolicyArtifactRecords,
      controlPolicySummaryArtifactCount: controlPolicySummaryArtifacts.length,
      controlPolicySummaryArtifacts,
      controlPolicySummaryArtifactTable,
      controlPolicySummaryArtifactCsv,
      controlPolicySummaryArtifactJson,
      controlPolicySummaryArtifactRecords,
      rawfileArtifactCount: rawfileArtifacts.length,
      rawfileArtifacts,
      rawfileArtifactTable,
      rawfileArtifactCsv,
      rawfileArtifactJson,
      rawfileArtifactRecords,
      wrdataArtifactCount: wrdataArtifacts.length,
      wrdataArtifacts,
      wrdataArtifactTable,
      wrdataArtifactCsv,
      wrdataArtifactJson,
      wrdataArtifactRecords,
      diagnosticCount: diagnosticCodes.length,
      diagnosticCodes: [...diagnosticCodes],
      tableCount: tables.length,
      tables,
      tableArtifacts,
      measurements,
      measurementTable,
      fourier,
      fourierTable,
      runArtifacts,
      runArtifactTable,
    };
  }
  if (plan.analysis === "tf") {
    const outputNode = requireDeckPlanString(plan.outputNode, plan, "outputNode");
    const inputSource = requireDeckPlanString(plan.sourceName, plan, "sourceName");
    const result = tf(circuit, outputNode, inputSource);
    selectDeckMeasurementCardsForAnalysis(netlist, plan.analysis);
    selectDeckFourierCardsForAnalysis(netlist, plan.analysis);
    const measurements: ProbeMeasurement[] = [];
    const fourier: FourierResult[] = [];
    const outputProbes = [`V(${outputNode})`];
    const outputProbeLines: number[] = [];
    const outputDirectives: string[] = [];
    const outputDirectiveAnalysisKinds: string[] = [];
    const outputDirectiveLines: number[] = [];
    const table = formatDeckTfTable(result);
    const runArtifacts = deckRunArtifacts(
      plan,
      result,
      deckTableColumns(table),
      outputProbes,
      outputDirectives,
      measurements,
      fourier,
      controlLines,
      writeMarkers,
      rawfileOptions,
      diagnosticCodes,
      controlPolicyArtifacts,
      deckAnalysisKinds,
      deckAnalysisDirectiveInventory,
    );
    const tables = deckStableTables(measurements, fourier, controlPolicyArtifacts);

    const outputPlanArtifactBundle = deckOutputPlanArtifactBundle(
      plan,
      table,
      outputProbes,
      outputProbeLines,
      outputDirectives,
      outputDirectiveAnalysisKinds,
      outputDirectiveLines,
      tables,
    );
    const measurementTable = formatMeasurementTable(measurements);
    const fourierTable = formatDeckFourierTable(fourier);
    const runArtifactTable = formatDeckRunArtifactTable(runArtifacts);
    const tableArtifacts = deckTableArtifacts(
      plan,
      table,
      measurementTable,
      fourierTable,
      runArtifactTable,
      measurements,
      fourier,
      controlPolicyArtifacts,
      controlPolicyArtifactTable,
      controlPolicySummaryArtifacts,
      controlPolicySummaryArtifactTable,
      outputProbes,
      outputProbeLines,
      outputDirectives,
      outputDirectiveAnalysisKinds,
      outputDirectiveLines,
      tables,
    );
    const rawfileArtifacts = deckRawfileArtifacts(plan, table, writeMarkers, rawfileOptions);
    const rawfileArtifactTable = formatDeckRawfileArtifactTable(rawfileArtifacts);
    const rawfileArtifactCsv = formatDeckRawfileArtifactCsv(rawfileArtifacts);
    const rawfileArtifactJson = formatDeckRawfileArtifactJson(rawfileArtifacts);
    const rawfileArtifactRecords = deckRawfileArtifactRecords(rawfileArtifacts);
    const wrdataArtifacts = deckWrdataArtifacts(table, writeMarkers, rawfileOptions);
    const wrdataArtifactTable = formatDeckWrdataArtifactTable(wrdataArtifacts);
    const wrdataArtifactCsv = formatDeckWrdataArtifactCsv(wrdataArtifacts);
    const wrdataArtifactJson = formatDeckWrdataArtifactJson(wrdataArtifacts);
    const wrdataArtifactRecords = deckWrdataArtifactRecords(wrdataArtifacts);
    return {
      plan,
      result,
      table,
      outputProbes,
      outputDirectives,
      analysisDirectives,
      deckAnalysisKindCount: deckAnalysisKinds.length,
      deckAnalysisKinds: [...deckAnalysisKinds],
      deckAnalysisDirectiveCount: deckAnalysisDirectiveInventory.length,
      deckAnalysisDirectives: [...deckAnalysisDirectiveInventory],
      outputPlanArtifactCount: outputPlanArtifactBundle.artifacts.length,
      outputPlanArtifacts: outputPlanArtifactBundle.artifacts,
      outputPlanArtifactTable: outputPlanArtifactBundle.table,
      outputPlanArtifactCsv: outputPlanArtifactBundle.csv,
      outputPlanArtifactJson: outputPlanArtifactBundle.json,
      outputPlanArtifactRecords: outputPlanArtifactBundle.records,
      controlLineCount: controlLines.length,
      controlLines: [...controlLines],
      writeMarkerCount: writeMarkers.length,
      writeMarkers: [...writeMarkers],
      rawfileOptionCount: rawfileOptions.length,
      rawfileOptions: [...rawfileOptions],
      controlPolicyArtifactCount: controlPolicyArtifacts.length,
      controlPolicyArtifacts,
      controlPolicyArtifactTable,
      controlPolicyArtifactCsv,
      controlPolicyArtifactJson,
      controlPolicyArtifactRecords,
      controlPolicySummaryArtifactCount: controlPolicySummaryArtifacts.length,
      controlPolicySummaryArtifacts,
      controlPolicySummaryArtifactTable,
      controlPolicySummaryArtifactCsv,
      controlPolicySummaryArtifactJson,
      controlPolicySummaryArtifactRecords,
      rawfileArtifactCount: rawfileArtifacts.length,
      rawfileArtifacts,
      rawfileArtifactTable,
      rawfileArtifactCsv,
      rawfileArtifactJson,
      rawfileArtifactRecords,
      wrdataArtifactCount: wrdataArtifacts.length,
      wrdataArtifacts,
      wrdataArtifactTable,
      wrdataArtifactCsv,
      wrdataArtifactJson,
      wrdataArtifactRecords,
      diagnosticCount: diagnosticCodes.length,
      diagnosticCodes: [...diagnosticCodes],
      tableCount: tables.length,
      tables,
      tableArtifacts,
      measurements,
      measurementTable,
      fourier,
      fourierTable,
      runArtifacts,
      runArtifactTable,
    };
  }
  if (plan.analysis === "sens") {
    const outputNode = requireDeckPlanString(plan.outputNode, plan, "outputNode");
    const result = sensDc(circuit, outputNode);
    selectDeckMeasurementCardsForAnalysis(netlist, plan.analysis);
    selectDeckFourierCardsForAnalysis(netlist, plan.analysis);
    const measurements: ProbeMeasurement[] = [];
    const fourier: FourierResult[] = [];
    const outputProbes = [`V(${outputNode})`];
    const outputProbeLines: number[] = [];
    const outputDirectives: string[] = [];
    const outputDirectiveAnalysisKinds: string[] = [];
    const outputDirectiveLines: number[] = [];
    const table = formatDeckSensTable(result);
    const runArtifacts = deckRunArtifacts(
      plan,
      result,
      deckTableColumns(table),
      outputProbes,
      outputDirectives,
      measurements,
      fourier,
      controlLines,
      writeMarkers,
      rawfileOptions,
      diagnosticCodes,
      controlPolicyArtifacts,
      deckAnalysisKinds,
      deckAnalysisDirectiveInventory,
    );
    const tables = deckStableTables(measurements, fourier, controlPolicyArtifacts);

    const outputPlanArtifactBundle = deckOutputPlanArtifactBundle(
      plan,
      table,
      outputProbes,
      outputProbeLines,
      outputDirectives,
      outputDirectiveAnalysisKinds,
      outputDirectiveLines,
      tables,
    );
    const measurementTable = formatMeasurementTable(measurements);
    const fourierTable = formatDeckFourierTable(fourier);
    const runArtifactTable = formatDeckRunArtifactTable(runArtifacts);
    const tableArtifacts = deckTableArtifacts(
      plan,
      table,
      measurementTable,
      fourierTable,
      runArtifactTable,
      measurements,
      fourier,
      controlPolicyArtifacts,
      controlPolicyArtifactTable,
      controlPolicySummaryArtifacts,
      controlPolicySummaryArtifactTable,
      outputProbes,
      outputProbeLines,
      outputDirectives,
      outputDirectiveAnalysisKinds,
      outputDirectiveLines,
      tables,
    );
    const rawfileArtifacts = deckRawfileArtifacts(plan, table, writeMarkers, rawfileOptions);
    const rawfileArtifactTable = formatDeckRawfileArtifactTable(rawfileArtifacts);
    const rawfileArtifactCsv = formatDeckRawfileArtifactCsv(rawfileArtifacts);
    const rawfileArtifactJson = formatDeckRawfileArtifactJson(rawfileArtifacts);
    const rawfileArtifactRecords = deckRawfileArtifactRecords(rawfileArtifacts);
    const wrdataArtifacts = deckWrdataArtifacts(table, writeMarkers, rawfileOptions);
    const wrdataArtifactTable = formatDeckWrdataArtifactTable(wrdataArtifacts);
    const wrdataArtifactCsv = formatDeckWrdataArtifactCsv(wrdataArtifacts);
    const wrdataArtifactJson = formatDeckWrdataArtifactJson(wrdataArtifacts);
    const wrdataArtifactRecords = deckWrdataArtifactRecords(wrdataArtifacts);
    return {
      plan,
      result,
      table,
      outputProbes,
      outputDirectives,
      analysisDirectives,
      deckAnalysisKindCount: deckAnalysisKinds.length,
      deckAnalysisKinds: [...deckAnalysisKinds],
      deckAnalysisDirectiveCount: deckAnalysisDirectiveInventory.length,
      deckAnalysisDirectives: [...deckAnalysisDirectiveInventory],
      outputPlanArtifactCount: outputPlanArtifactBundle.artifacts.length,
      outputPlanArtifacts: outputPlanArtifactBundle.artifacts,
      outputPlanArtifactTable: outputPlanArtifactBundle.table,
      outputPlanArtifactCsv: outputPlanArtifactBundle.csv,
      outputPlanArtifactJson: outputPlanArtifactBundle.json,
      outputPlanArtifactRecords: outputPlanArtifactBundle.records,
      controlLineCount: controlLines.length,
      controlLines: [...controlLines],
      writeMarkerCount: writeMarkers.length,
      writeMarkers: [...writeMarkers],
      rawfileOptionCount: rawfileOptions.length,
      rawfileOptions: [...rawfileOptions],
      controlPolicyArtifactCount: controlPolicyArtifacts.length,
      controlPolicyArtifacts,
      controlPolicyArtifactTable,
      controlPolicyArtifactCsv,
      controlPolicyArtifactJson,
      controlPolicyArtifactRecords,
      controlPolicySummaryArtifactCount: controlPolicySummaryArtifacts.length,
      controlPolicySummaryArtifacts,
      controlPolicySummaryArtifactTable,
      controlPolicySummaryArtifactCsv,
      controlPolicySummaryArtifactJson,
      controlPolicySummaryArtifactRecords,
      rawfileArtifactCount: rawfileArtifacts.length,
      rawfileArtifacts,
      rawfileArtifactTable,
      rawfileArtifactCsv,
      rawfileArtifactJson,
      rawfileArtifactRecords,
      wrdataArtifactCount: wrdataArtifacts.length,
      wrdataArtifacts,
      wrdataArtifactTable,
      wrdataArtifactCsv,
      wrdataArtifactJson,
      wrdataArtifactRecords,
      diagnosticCount: diagnosticCodes.length,
      diagnosticCodes: [...diagnosticCodes],
      tableCount: tables.length,
      tables,
      tableArtifacts,
      measurements,
      measurementTable,
      fourier,
      fourierTable,
      runArtifacts,
      runArtifactTable,
    };
  }
  if (plan.analysis === "noise") {
    const outputNode = requireDeckPlanString(plan.outputNode, plan, "outputNode");
    const inputSource = requireDeckPlanString(plan.sourceName, plan, "sourceName");
    const frequenciesHz = plan.sweepKind === undefined
      ? undefined
      : deckAcFrequencies(
          plan,
          plan.sweepKind,
          requireDeckPlanInteger(plan.pointCount, plan, "pointCount"),
          requireDeckPlanNumber(plan.startFrequencyHz, plan, "startFrequencyHz"),
          requireDeckPlanNumber(plan.stopFrequencyHz, plan, "stopFrequencyHz"),
        );
    const result = noiseAc(circuit, outputNode, inputSource, frequenciesHz);
    selectDeckMeasurementCardsForAnalysis(netlist, plan.analysis);
    selectDeckFourierCardsForAnalysis(netlist, plan.analysis);
    const measurements: ProbeMeasurement[] = [];
    const fourier: FourierResult[] = [];
    const outputProbes = [`V(${outputNode})`];
    const outputProbeLines: number[] = [];
    const outputDirectives: string[] = [];
    const outputDirectiveAnalysisKinds: string[] = [];
    const outputDirectiveLines: number[] = [];
    const table = formatDeckNoiseTable(result);
    const runArtifacts = deckRunArtifacts(
      plan,
      result,
      deckTableColumns(table),
      outputProbes,
      outputDirectives,
      measurements,
      fourier,
      controlLines,
      writeMarkers,
      rawfileOptions,
      diagnosticCodes,
      controlPolicyArtifacts,
      deckAnalysisKinds,
      deckAnalysisDirectiveInventory,
    );
    const tables = deckStableTables(measurements, fourier, controlPolicyArtifacts);

    const outputPlanArtifactBundle = deckOutputPlanArtifactBundle(
      plan,
      table,
      outputProbes,
      outputProbeLines,
      outputDirectives,
      outputDirectiveAnalysisKinds,
      outputDirectiveLines,
      tables,
    );
    const measurementTable = formatMeasurementTable(measurements);
    const fourierTable = formatDeckFourierTable(fourier);
    const runArtifactTable = formatDeckRunArtifactTable(runArtifacts);
    const tableArtifacts = deckTableArtifacts(
      plan,
      table,
      measurementTable,
      fourierTable,
      runArtifactTable,
      measurements,
      fourier,
      controlPolicyArtifacts,
      controlPolicyArtifactTable,
      controlPolicySummaryArtifacts,
      controlPolicySummaryArtifactTable,
      outputProbes,
      outputProbeLines,
      outputDirectives,
      outputDirectiveAnalysisKinds,
      outputDirectiveLines,
      tables,
    );
    const rawfileArtifacts = deckRawfileArtifacts(plan, table, writeMarkers, rawfileOptions);
    const rawfileArtifactTable = formatDeckRawfileArtifactTable(rawfileArtifacts);
    const rawfileArtifactCsv = formatDeckRawfileArtifactCsv(rawfileArtifacts);
    const rawfileArtifactJson = formatDeckRawfileArtifactJson(rawfileArtifacts);
    const rawfileArtifactRecords = deckRawfileArtifactRecords(rawfileArtifacts);
    const wrdataArtifacts = deckWrdataArtifacts(table, writeMarkers, rawfileOptions);
    const wrdataArtifactTable = formatDeckWrdataArtifactTable(wrdataArtifacts);
    const wrdataArtifactCsv = formatDeckWrdataArtifactCsv(wrdataArtifacts);
    const wrdataArtifactJson = formatDeckWrdataArtifactJson(wrdataArtifacts);
    const wrdataArtifactRecords = deckWrdataArtifactRecords(wrdataArtifacts);
    return {
      plan,
      result,
      table,
      outputProbes,
      outputDirectives,
      analysisDirectives,
      deckAnalysisKindCount: deckAnalysisKinds.length,
      deckAnalysisKinds: [...deckAnalysisKinds],
      deckAnalysisDirectiveCount: deckAnalysisDirectiveInventory.length,
      deckAnalysisDirectives: [...deckAnalysisDirectiveInventory],
      outputPlanArtifactCount: outputPlanArtifactBundle.artifacts.length,
      outputPlanArtifacts: outputPlanArtifactBundle.artifacts,
      outputPlanArtifactTable: outputPlanArtifactBundle.table,
      outputPlanArtifactCsv: outputPlanArtifactBundle.csv,
      outputPlanArtifactJson: outputPlanArtifactBundle.json,
      outputPlanArtifactRecords: outputPlanArtifactBundle.records,
      controlLineCount: controlLines.length,
      controlLines: [...controlLines],
      writeMarkerCount: writeMarkers.length,
      writeMarkers: [...writeMarkers],
      rawfileOptionCount: rawfileOptions.length,
      rawfileOptions: [...rawfileOptions],
      controlPolicyArtifactCount: controlPolicyArtifacts.length,
      controlPolicyArtifacts,
      controlPolicyArtifactTable,
      controlPolicyArtifactCsv,
      controlPolicyArtifactJson,
      controlPolicyArtifactRecords,
      controlPolicySummaryArtifactCount: controlPolicySummaryArtifacts.length,
      controlPolicySummaryArtifacts,
      controlPolicySummaryArtifactTable,
      controlPolicySummaryArtifactCsv,
      controlPolicySummaryArtifactJson,
      controlPolicySummaryArtifactRecords,
      rawfileArtifactCount: rawfileArtifacts.length,
      rawfileArtifacts,
      rawfileArtifactTable,
      rawfileArtifactCsv,
      rawfileArtifactJson,
      rawfileArtifactRecords,
      wrdataArtifactCount: wrdataArtifacts.length,
      wrdataArtifacts,
      wrdataArtifactTable,
      wrdataArtifactCsv,
      wrdataArtifactJson,
      wrdataArtifactRecords,
      diagnosticCount: diagnosticCodes.length,
      diagnosticCodes: [...diagnosticCodes],
      tableCount: tables.length,
      tables,
      tableArtifacts,
      measurements,
      measurementTable,
      fourier,
      fourierTable,
      runArtifacts,
      runArtifactTable,
    };
  }
  throw invalidElement("runDeckAnalysis", `unsupported analysis ${JSON.stringify(plan.analysis)}`);
}

function requireDeckPlanString(
  value: string | undefined,
  plan: DeckAnalysisPlan,
  fieldName: string,
): string {
  if (value !== undefined && value.length > 0) {
    return value;
  }
  throw invalidElement(
    "runDeckAnalysis",
    `line ${plan.lineNumber}: ${plan.directive} analysis missing ${fieldName}`,
  );
}

function requireDeckPlanNumber(
  value: number | undefined,
  plan: DeckAnalysisPlan,
  fieldName: string,
): number {
  if (value !== undefined && Number.isFinite(value)) {
    return value;
  }
  throw invalidElement(
    "runDeckAnalysis",
    `line ${plan.lineNumber}: ${plan.directive} analysis missing ${fieldName}`,
  );
}

function requireDeckPlanInteger(
  value: number | undefined,
  plan: DeckAnalysisPlan,
  fieldName: string,
): number {
  if (value !== undefined && Number.isInteger(value)) {
    return value;
  }
  throw invalidElement(
    "runDeckAnalysis",
    `line ${plan.lineNumber}: ${plan.directive} analysis missing ${fieldName}`,
  );
}

function sampleTransientPointsPrintStep(
  points: readonly TransientPoint[],
  printStep: number,
  startTime: number | undefined,
  stopTime: number,
): TransientPoint[] {
  if (points.length === 0) {
    return [...points];
  }
  const epsilon = Math.max(Math.abs(stopTime), Math.abs(printStep), 1.0) * 1.0e-12;
  const reportStart = startTime !== undefined && startTime > 0.0
    ? startTime
    : Math.abs(points[0]?.time ?? 0.0) <= epsilon
      ? 0.0
      : printStep;
  const sampled: TransientPoint[] = [];
  for (let index = 0; ; index += 1) {
    const sampleTime = reportStart + index * printStep;
    if (sampleTime > stopTime + epsilon) {
      break;
    }
    sampled.push(interpolateTransientPoint(points, sampleTime));
  }
  return sampled;
}

function interpolateTransientPoint(
  points: readonly TransientPoint[],
  time: number,
): TransientPoint {
  const epsilon = Math.max(Math.abs(time), 1.0) * 1.0e-12;
  for (const point of points) {
    if (Math.abs(point.time - time) <= epsilon) {
      return makeTransientPoint(time, new Map(point.nodeVoltages), new Map(point.branchCurrents));
    }
  }
  for (let index = 0; index + 1 < points.length; index += 1) {
    const left = points[index]!;
    const right = points[index + 1]!;
    if (left.time - epsilon <= time && time <= right.time + epsilon) {
      const span = right.time - left.time;
      if (span <= 0.0) {
        return makeTransientPoint(time, new Map(left.nodeVoltages), new Map(left.branchCurrents));
      }
      const alpha = (time - left.time) / span;
      return makeTransientPoint(
        time,
        interpolateValueMap(left.nodeVoltages, right.nodeVoltages, alpha),
        interpolateValueMap(left.branchCurrents, right.branchCurrents, alpha),
      );
    }
  }
  throw invalidElement("runDeckAnalysis", "transient print point is outside output");
}

function interpolateValueMap(
  left: ReadonlyMap<string, number>,
  right: ReadonlyMap<string, number>,
  alpha: number,
): Map<string, number> {
  const values = new Map<string, number>();
  for (const [key, leftValue] of left) {
    const rightValue = right.get(key) ?? leftValue;
    values.set(key, (1.0 - alpha) * leftValue + alpha * rightValue);
  }
  for (const [key, rightValue] of right) {
    if (!values.has(key)) {
      values.set(key, rightValue);
    }
  }
  return values;
}

function runDeckAcSweep(
  circuit: Circuit,
  plan: DeckAnalysisPlan,
  sweepKind: string,
  pointCount: number,
  startFrequencyHz: number,
  stopFrequencyHz: number,
): AcPoint[] {
  return deckAcFrequencies(plan, sweepKind, pointCount, startFrequencyHz, stopFrequencyHz).map(
    (frequencyHz) => {
      const point = acSweep(circuit, frequencyHz, frequencyHz, 1)[0];
      if (point === undefined) {
        throw invalidElement(
          "runDeckAnalysis",
          `line ${plan.lineNumber}: .ac ${sweepKind.toUpperCase()} produced no samples`,
        );
      }
      return point;
    },
  );
}

function deckAcFrequencies(
  plan: DeckAnalysisPlan,
  sweepKind: string,
  pointCount: number,
  startFrequencyHz: number,
  stopFrequencyHz: number,
): number[] {
  if (pointCount <= 0) {
    throw invalidElement(
      "runDeckAnalysis",
      `line ${plan.lineNumber}: .ac pointCount must be positive`,
    );
  }
  if (sweepKind === "lin") {
    if (pointCount === 1) {
      return [startFrequencyHz];
    }
    const step = (stopFrequencyHz - startFrequencyHz) / (pointCount - 1);
    return Array.from({ length: pointCount }, (_value, index) => startFrequencyHz + index * step);
  }
  if (sweepKind === "dec" || sweepKind === "oct") {
    const base = sweepKind === "dec" ? 10.0 : 2.0;
    const ratio = base ** (1.0 / pointCount);
    const epsilon = stopFrequencyHz * 1.0e-12;
    const frequencies: number[] = [];
    for (
      let frequencyHz = startFrequencyHz;
      frequencyHz <= stopFrequencyHz + epsilon;
      frequencyHz *= ratio
    ) {
      frequencies.push(frequencyHz);
    }
    return frequencies;
  }
  throw invalidElement(
    "runDeckAnalysis",
    `line ${plan.lineNumber}: .ac ${sweepKind.toUpperCase()} execution is not supported yet`,
  );
}

export function formatCornerAcTable(
  result: CornerAcSweepResult,
  probes?: readonly string[],
): string {
  const firstNonEmpty = result.points.find((point) => point.points.length > 0);
  const selectedProbes = probes === undefined || probes.length === 0 ? (
    firstNonEmpty === undefined ? [] : defaultAcOutputProbes(firstNonEmpty.points)
  ) : probes;
  const rows = [["Corner", "Index", "Frequency", "Probe", "Real", "Imaginary", "Magnitude", "Phase"].join("\t")];
  result.points.forEach((corner) => {
    corner.points.forEach((point, index) => {
      selectedProbes.forEach((probe) => {
        const value = tableComplexProbeValue(
          point.nodeVoltages,
          point.branchCurrents,
          probe,
          "formatCornerAcTable",
        );
        rows.push([
          corner.cornerName,
          String(index),
          formatTableNumber(point.frequencyHz),
          probe,
          formatTableNumber(value.real),
          formatTableNumber(value.imag),
          formatTableNumber(complexAbs(value)),
          formatTableNumber(complexPhase(value) * 180.0 / Math.PI),
        ].join("\t"));
      });
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

export function formatCornerTfTable(result: CornerTfResult): string {
  const rows = [["Corner", "TransferRatio", "InputImpedance", "OutputImpedance"].join("\t")];
  result.points.forEach((point) => {
    rows.push([
      point.cornerName,
      formatTableNumber(point.result.transferRatio),
      formatTableNumber(point.result.inputImpedanceOhms),
      formatTableNumber(point.result.outputImpedanceOhms),
    ].join("\t"));
  });
  rows.push("");
  return rows.join("\n");
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

export function formatCornerPoleZeroTable(result: CornerPoleZeroResult): string {
  const rows = [["Corner", "Index", "Kind", "Real", "Imaginary", "Frequency", "Damping"].join("\t")];
  result.points.forEach((corner) => {
    corner.result.entries.forEach((entry, index) => {
      rows.push(
        [
          corner.cornerName,
          String(index),
          entry.kind,
          formatTableNumber(entry.real),
          formatTableNumber(entry.imaginary),
          formatTableNumber(entry.frequencyHz),
          formatTableNumber(entry.damping),
        ].join("\t"),
      );
    });
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

export function formatCornerDistortionTable(result: CornerDistortionResult): string {
  const rows = [["Corner", "Frequency", "Input", "Output", "Harmonic", "Magnitude", "Phase", "THD"].join("\t")];
  result.points.forEach((corner) => {
    corner.result.points.forEach((point) => {
      point.harmonics.forEach((harmonic) => {
        rows.push(
          [
            corner.cornerName,
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

export function formatDeckFourierTable(results: readonly FourierResult[]): string {
  return results.map((result) => formatFourierTable(result)).join("\n");
}

export function formatCornerFourierTable(result: CornerFourierResult): string {
  const rows = [["Corner", "Probe", "Harmonic", "Frequency", "Cosine", "Sine", "Magnitude", "Phase", "DC", "THD"].join("\t")];
  result.points.forEach((corner) => {
    corner.result.probes.forEach((probe) => {
      probe.harmonics.forEach((harmonic) => {
        rows.push(
          [
            corner.cornerName,
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

function realSolverKind(matrixSize: number): LinearSolverKind {
  if (matrixSize === 0) {
    return "none";
  }
  return matrixSize >= SPARSE_SOLVER_THRESHOLD ? "sparse_real" : "dense_real";
}

function complexSolverKind(matrixSize: number): LinearSolverKind {
  if (matrixSize === 0) {
    return "none";
  }
  return matrixSize >= SPARSE_SOLVER_THRESHOLD ? "sparse_complex" : "dense_complex";
}

function emptySolverProfile(matrixSize = 0): LinearSolverProfile {
  return {
    matrixSize,
    solver: realSolverKind(matrixSize),
    backend: "none",
    structuralNonzeros: 0,
    density: 0.0,
    fillInNonzeros: 0,
  };
}

function realMatrixNonzeros(matrix: readonly (readonly number[])[]): number {
  return matrix.reduce(
    (count, row) => count + row.filter((value) => value !== 0.0).length,
    0,
  );
}

function realMatrixDensity(matrixSize: number, structuralNonzeros: number): number {
  if (matrixSize === 0) {
    return 0.0;
  }
  return structuralNonzeros / (matrixSize * matrixSize);
}

function realSolverProfile(
  matrix: readonly (readonly number[])[],
  backend: LinearSolverBackend,
  fillInNonzeros = 0,
  fallbackReason?: string,
): LinearSolverProfile {
  const matrixSize = matrix.length;
  const structuralNonzeros = realMatrixNonzeros(matrix);
  return {
    matrixSize,
    solver: realSolverKind(matrixSize),
    backend,
    structuralNonzeros,
    density: realMatrixDensity(matrixSize, structuralNonzeros),
    fillInNonzeros,
    ...(fallbackReason === undefined ? {} : { fallbackReason }),
  };
}

function dcDiagnosticsFromLinearSolution(
  solution: LinearSolution,
  convergenceAid: DcConvergenceAid,
  tolerance: number,
): DcSolverDiagnostics {
  const matrixSize = solution.vector.length;
  return {
    matrixSize,
    solver: realSolverKind(matrixSize),
    tolerance,
    maxDelta: solution.maxDelta,
    convergenceAid,
    ...(solution.newtonStepLimit === undefined
      ? {}
      : { newtonStepLimit: solution.newtonStepLimit }),
    limitedNewtonSteps: solution.limitedNewtonSteps,
    minimumDampingFactor: solution.minimumDampingFactor,
    solverProfile: solution.solverProfile,
  };
}

export function dcOp(
  circuit: Circuit,
  options: DcOpOptions = {},
): DcResult {
  const solveOptions = validatedDcOpOptions(options);
  return dcOpFromInitialVector(circuit, solveOptions);
}

export function dcOpWithInitialVector(
  circuit: Circuit,
  initialVector: readonly number[],
  options: DcOpOptions = {},
): DcResult {
  const solveOptions = validatedDcOpOptions(options);
  validateDcInitialVector(circuit, initialVector);
  return dcOpFromInitialVector(circuit, solveOptions, initialVector);
}

export function dcOpWithInitialConditions(
  circuit: Circuit,
  summary: DeckInitialConditionSummary,
  options: DcOpOptions = {},
): DcResult {
  return dcOpWithInitialVector(
    circuit,
    dcInitialVectorFromConditions(circuit, summary.initialConditions, summary.nodesets),
    options,
  );
}

export function dcInitialVectorFromConditions(
  circuit: Circuit,
  initialConditions: readonly DeckNodeCondition[],
  nodesets: readonly DeckNodeCondition[] = [],
): number[] {
  const nodeIndices = collectNodeIndices(circuit);
  const voltageSources = collectVoltageSources(circuit, []);
  const vector = Array.from({ length: nodeIndices.size + voltageSources.size }, () => 0.0);
  for (const condition of nodesets) {
    applyNodeConditionToInitialVector(condition, nodeIndices, vector);
  }
  for (const condition of initialConditions) {
    applyNodeConditionToInitialVector(condition, nodeIndices, vector);
  }
  return vector;
}

function dcOpFromInitialVector(
  circuit: Circuit,
  solveOptions: ResolvedDcOpOptions,
  initialVector?: readonly number[],
): DcResult {
  const solution = solveDcNewton(circuit, solveOptions, initialVector);
  if (solution.converged) {
    return makeDcResult(
      solution.nodeVoltages,
      solution.branchCurrents,
      solution.iterations,
      solution.converged,
      "newton",
      dcDiagnosticsFromLinearSolution(solution, "newton", solveOptions.tolerance),
    );
  }
  if (!solveOptions.convergenceAids) {
    return makeDcResult(
      solution.nodeVoltages,
      solution.branchCurrents,
      solution.iterations,
      false,
      "none",
      dcDiagnosticsFromLinearSolution(solution, "none", solveOptions.tolerance),
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
    dcDiagnosticsFromLinearSolution(finalSolution, convergenceAid, solveOptions.tolerance),
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

export function dcTemperatureSweep(
  circuit: Circuit,
  temperaturesKelvin: readonly number[],
  options: DcOpOptions = {},
  nominalTemperatureKelvin = 300.15,
  energyGapElectronVolts = 1.11,
): TemperatureDcResult {
  return {
    points: temperaturesKelvin.map((temperatureKelvin) => ({
      temperatureKelvin,
      result: dcOp(
        circuitAtTemperature(
          circuit,
          temperatureKelvin,
          nominalTemperatureKelvin,
          energyGapElectronVolts,
        ),
        options,
      ),
    })),
  };
}

export function dcTemperatureSweepCorners(
  circuit: Circuit,
  temperaturesKelvin: readonly number[],
  corners: readonly CornerSpec[],
  options: DcOpOptions = {},
  nominalTemperatureKelvin = 300.15,
  energyGapElectronVolts = 1.11,
): CornerTemperatureDcResult {
  return {
    points: corners.map((corner) => ({
      cornerName: corner.name,
      points: dcTemperatureSweep(
        circuitWithCorner(circuit, corner),
        temperaturesKelvin,
        options,
        nominalTemperatureKelvin,
        energyGapElectronVolts,
      ).points,
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
      return makeNoisePoint(
        frequencyHz,
        0.0,
        0.0,
        zeroNoiseEntries(noiseSources, frequencyHz),
      );
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
          zeroNoiseEntries(noiseSources, frequencyHz),
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
      const sourcePsd = source.sourcePsd / frequencyHz ** source.frequencyExponent;
      return {
        elementName: source.elementName,
        noiseType: source.noiseType,
        sourcePsd,
        outputPsd: complexAbs(transfer) ** 2 * sourcePsd,
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
  seedDeviceCapacitorStates(circuit, initialSolution.nodeVoltages, capacitorStates);
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

export function transientCorners(
  circuit: Circuit,
  timeStep: number,
  stopTime: number,
  corners: readonly CornerSpec[],
  method: TransientMethod = "euler",
): CornerTransientResult {
  return {
    points: corners.map((corner) => ({
      cornerName: corner.name,
      points: transient(circuitWithCorner(circuit, corner), timeStep, stopTime, method),
    })),
  };
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

export function fourierTransientCards(
  points: readonly TransientPoint[],
  fourierCards: readonly DeckFourierCard[],
): FourierResult[] {
  return fourierCards.map((card) =>
    fourier(
      points,
      card.fundamentalFrequencyHz,
      card.probes,
      card.harmonics ?? 9,
      card.fromValue,
    )
  );
}

export function fourierTransientDeck(
  points: readonly TransientPoint[],
  netlist: string,
): FourierResult[] {
  const summary = resolveDeckFourier(netlist);
  if (summary.diagnostics.length > 0) {
    const diagnostic = summary.diagnostics[0];
    throw invalidElement(
      "fourierTransientDeck",
      `line ${diagnostic.lineNumber}: ${diagnostic.message}`,
    );
  }
  return fourierTransientCards(points, summary.fourier);
}

export function fourierCorners(
  circuit: Circuit,
  timeStep: number,
  stopTime: number,
  fundamentalFrequencyHz: number,
  probes: readonly string[],
  corners: readonly CornerSpec[],
  harmonics = 9,
  startTime?: number,
  method: TransientMethod = "euler",
): CornerFourierResult {
  return {
    fundamentalFrequencyHz,
    points: corners.map((corner) => ({
      cornerName: corner.name,
      result: fourier(
        transient(circuitWithCorner(circuit, corner), timeStep, stopTime, method),
        fundamentalFrequencyHz,
        probes,
        harmonics,
        startTime,
      ),
    })),
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
  seedDeviceCapacitorStates(circuit, initialSolution.nodeVoltages, capacitorStates);
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

export function transientAdaptiveCorners(
  circuit: Circuit,
  timeStep: number,
  stopTime: number,
  corners: readonly CornerSpec[],
  options: AdaptiveTransientOptions = {},
): CornerAdaptiveTransientResult {
  return {
    points: corners.map((corner) => ({
      cornerName: corner.name,
      result: transientAdaptive(
        circuitWithCorner(circuit, corner),
        timeStep,
        stopTime,
        options,
      ),
    })),
  };
}

export function digitalEventsToPwlWaveform(
  events: readonly DigitalEvent[],
  levels: DigitalLogicLevels,
): PwlWaveform {
  validateDigitalLogicLevels(levels, "digitalEvents");
  if (events.length === 0) {
    throw invalidElement("digitalEvents", "at least one digital event is required");
  }

  let previousTime = Number.NEGATIVE_INFINITY;
  for (const event of events) {
    validateDigitalEventTime(event.timeSeconds, previousTime, "digitalEvents");
    normalizeDigitalState(event.state);
    previousTime = event.timeSeconds;
  }

  const points: [number, number][] = [];
  let currentState = normalizeDigitalState(events[0].state);
  points.push([events[0].timeSeconds, levels.voltageFor(currentState)]);

  for (const event of events.slice(1)) {
    const eventState = normalizeDigitalState(event.state);
    if (eventState === currentState) {
      continue;
    }
    const startTime = event.timeSeconds;
    const endTime = startTime + levels.transitionSeconds;
    const lastTime = points[points.length - 1][0];
    if (startTime <= lastTime) {
      throw invalidElement("digitalEvents", "digital transition overlaps the previous transition");
    }
    points.push([startTime, levels.voltageFor(currentState)]);
    points.push([endTime, levels.voltageFor(eventState)]);
    currentState = eventState;
  }

  if (points.length === 1) {
    points.push([points[0][0] + levels.transitionSeconds, levels.voltageFor(currentState)]);
  }

  return new PwlWaveform(points);
}

export function digitalEventsToVoltageSource(
  name: string,
  positive: string,
  negative: string,
  events: readonly DigitalEvent[],
  levels: DigitalLogicLevels,
): VoltageSource {
  if (events.length === 0) {
    throw invalidElement("digitalEvents", "at least one digital event is required");
  }
  return voltageSourceWithWaveform(
    name,
    positive,
    negative,
    levels.voltageFor(events[0].state),
    digitalEventsToPwlWaveform(events, levels),
  );
}

export function digitalEventStreamsToVoltageSources(
  streams: readonly DigitalEventStream[],
  negative: string,
  levels: DigitalLogicLevels,
): VoltageSource[] {
  const negativeNode = negative.trim();
  if (negativeNode.length === 0) {
    throw invalidElement("digitalEventStreams", "digital event stream negative node must not be empty");
  }
  const seenSignalNames = new Set<string>();
  return streams.map((stream) => {
    const signalName = validateDigitalEventStreamName(stream, seenSignalNames);
    return digitalEventsToVoltageSource(
      `V${signalName}`,
      signalName,
      negativeNode,
      stream.events,
      levels,
    );
  });
}

export function digitalEventStreamsToBridgeSchedule(
  streams: readonly DigitalEventStream[],
  levels: DigitalLogicLevels,
): DigitalBridgeSchedule {
  validateDigitalLogicLevels(levels, "digitalBridgeSchedule");
  const seenSignalNames = new Set<string>();
  const breakpoints: number[] = [];
  let stopTime = 0.0;
  for (const stream of streams) {
    validateDigitalEventStreamName(stream, seenSignalNames);
    digitalEventsToPwlWaveform(stream.events, levels);
    let currentState = normalizeDigitalState(stream.events[0].state);
    stream.events.forEach((event, index) => {
      const eventState = normalizeDigitalState(event.state);
      breakpoints.push(event.timeSeconds);
      stopTime = Math.max(stopTime, event.timeSeconds);
      if (index > 0 && eventState !== currentState) {
        const transitionEnd = event.timeSeconds + levels.transitionSeconds;
        breakpoints.push(transitionEnd);
        stopTime = Math.max(stopTime, transitionEnd);
        currentState = eventState;
      }
    });
  }

  breakpoints.sort((left, right) => left - right);
  const deduped: number[] = [];
  for (const breakpoint of breakpoints) {
    if (deduped.length === 0 || Math.abs(breakpoint - deduped[deduped.length - 1]) > DIGITAL_BRIDGE_TIME_EPSILON) {
      deduped.push(breakpoint);
    }
  }
  return { stopTime, breakpoints: deduped };
}

export function transientWithDigitalEventStreams(
  circuit: Circuit,
  inputStreams: readonly DigitalEventStream[],
  negative: string,
  levels: DigitalLogicLevels,
  timeStep: number,
  stopTime: number,
  outputProbes: readonly (readonly [string, string])[],
  thresholds: DigitalThresholds,
  method: TransientMethod = "euler",
): DigitalTransientBridgeResult {
  const bridged = circuitWithExtraVoltageSources(
    circuit,
    digitalEventStreamsToVoltageSources(inputStreams, negative, levels),
  );
  const points = transient(bridged, timeStep, stopTime, method);
  return {
    points,
    outputStreams: sampleTransientProbesAsDigitalEventStreams(points, outputProbes, thresholds),
  };
}

export function transientWithDigitalEventStreamsCorners(
  circuit: Circuit,
  inputStreams: readonly DigitalEventStream[],
  negative: string,
  levels: DigitalLogicLevels,
  timeStep: number,
  stopTime: number,
  outputProbes: readonly (readonly [string, string])[],
  thresholds: DigitalThresholds,
  corners: readonly CornerSpec[],
  method: TransientMethod = "euler",
): CornerDigitalTransientBridgeResult {
  return {
    points: corners.map((corner) => ({
      cornerName: corner.name,
      result: transientWithDigitalEventStreams(
        circuitWithCorner(circuit, corner),
        inputStreams,
        negative,
        levels,
        timeStep,
        stopTime,
        outputProbes,
        thresholds,
        method,
      ),
    })),
  };
}

export function transientAdaptiveWithDigitalEventStreams(
  circuit: Circuit,
  inputStreams: readonly DigitalEventStream[],
  negative: string,
  levels: DigitalLogicLevels,
  timeStep: number,
  stopTime: number,
  options: AdaptiveTransientOptions,
  outputProbes: readonly (readonly [string, string])[],
  thresholds: DigitalThresholds,
): AdaptiveDigitalTransientBridgeResult {
  const bridged = circuitWithExtraVoltageSources(
    circuit,
    digitalEventStreamsToVoltageSources(inputStreams, negative, levels),
  );
  const result = transientAdaptive(bridged, timeStep, stopTime, options);
  return {
    result,
    outputStreams: sampleTransientProbesAsDigitalEventStreams(result.points, outputProbes, thresholds),
  };
}

export function transientAdaptiveWithDigitalEventStreamsCorners(
  circuit: Circuit,
  inputStreams: readonly DigitalEventStream[],
  negative: string,
  levels: DigitalLogicLevels,
  timeStep: number,
  stopTime: number,
  options: AdaptiveTransientOptions,
  outputProbes: readonly (readonly [string, string])[],
  thresholds: DigitalThresholds,
  corners: readonly CornerSpec[],
): CornerAdaptiveDigitalTransientBridgeResult {
  return {
    points: corners.map((corner) => ({
      cornerName: corner.name,
      result: transientAdaptiveWithDigitalEventStreams(
        circuitWithCorner(circuit, corner),
        inputStreams,
        negative,
        levels,
        timeStep,
        stopTime,
        options,
        outputProbes,
        thresholds,
      ),
    })),
  };
}

export function sampleTransientProbeAsDigitalEvents(
  points: readonly TransientPoint[],
  probe: string,
  thresholds: DigitalThresholds,
): DigitalEvent[] {
  validateDigitalThresholds(thresholds);
  const events: DigitalEvent[] = [];
  let currentState: DigitalState | undefined;
  for (const point of points) {
    if (point.time <= DIGITAL_BRIDGE_TIME_EPSILON) {
      continue;
    }
    const voltage = tableProbeValue(
      point.nodeVoltages,
      point.branchCurrents,
      probe,
      "sampleTransientProbeAsDigitalEvents",
    );
    const state = thresholds.classify(voltage);
    if (state === undefined) {
      continue;
    }
    if (currentState !== state) {
      events.push({ timeSeconds: point.time, state });
      currentState = state;
    }
  }
  return events;
}

export function sampleTransientProbesAsDigitalEventStreams(
  points: readonly TransientPoint[],
  outputProbes: readonly (readonly [string, string])[],
  thresholds: DigitalThresholds,
): DigitalEventStream[] {
  const seenSignalNames = new Set<string>();
  return outputProbes.map(([signalName, probe]) => {
    const streamName = signalName.trim();
    if (streamName.length === 0) {
      throw invalidElement("digitalEventStream", "digital event stream signal name must not be empty");
    }
    if (seenSignalNames.has(streamName)) {
      throw invalidElement(streamName, "digital event stream signal names must be unique");
    }
    seenSignalNames.add(streamName);
    return {
      signalName: streamName,
      events: sampleTransientProbeAsDigitalEvents(points, probe, thresholds),
    };
  });
}

export function formatDigitalEventTable(events: readonly DigitalEvent[]): string {
  const rows = [["Index", "Time", "State"].join("\t")];
  let previousTime = Number.NEGATIVE_INFINITY;
  events.forEach((event, index) => {
    validateDigitalEventTime(event.timeSeconds, previousTime, "digitalEvent");
    previousTime = event.timeSeconds;
    rows.push([
      String(index),
      formatTableNumber(event.timeSeconds),
      normalizeDigitalState(event.state),
    ].join("\t"));
  });
  rows.push("");
  return rows.join("\n");
}

export function formatDigitalEventStreamTable(streams: readonly DigitalEventStream[]): string {
  const rows = [["Signal", "Index", "Time", "State"].join("\t")];
  for (const stream of streams) {
    if (stream.signalName.trim().length === 0) {
      throw invalidElement("digitalEventStream", "digital event stream signal name must not be empty");
    }
    let previousTime = Number.NEGATIVE_INFINITY;
    stream.events.forEach((event, index) => {
      validateDigitalEventTime(event.timeSeconds, previousTime, stream.signalName);
      previousTime = event.timeSeconds;
      rows.push([
        stream.signalName,
        String(index),
        formatTableNumber(event.timeSeconds),
        normalizeDigitalState(event.state),
      ].join("\t"));
    });
  }
  rows.push("");
  return rows.join("\n");
}

export function formatCornerDigitalEventStreamTable(
  result: CornerDigitalTransientBridgeResult,
): string {
  const rows = [["Corner", "Signal", "Index", "Time", "State"].join("\t")];
  for (const corner of result.points) {
    for (const stream of corner.result.outputStreams) {
      if (stream.signalName.trim().length === 0) {
        throw invalidElement("digitalEventStream", "digital event stream signal name must not be empty");
      }
      let previousTime = Number.NEGATIVE_INFINITY;
      stream.events.forEach((event, index) => {
        validateDigitalEventTime(event.timeSeconds, previousTime, stream.signalName);
        previousTime = event.timeSeconds;
        rows.push([
          corner.cornerName,
          stream.signalName,
          String(index),
          formatTableNumber(event.timeSeconds),
          normalizeDigitalState(event.state),
        ].join("\t"));
      });
    }
  }
  rows.push("");
  return rows.join("\n");
}

export function formatAdaptiveDigitalEventStreamTable(
  result: AdaptiveDigitalTransientBridgeResult,
): string {
  const rows = [["Method", "StepsRejected", "Converged", "Signal", "Index", "Time", "State"].join("\t")];
  for (const stream of result.outputStreams) {
    if (stream.signalName.trim().length === 0) {
      throw invalidElement("digitalEventStream", "digital event stream signal name must not be empty");
    }
    let previousTime = Number.NEGATIVE_INFINITY;
    stream.events.forEach((event, index) => {
      validateDigitalEventTime(event.timeSeconds, previousTime, stream.signalName);
      previousTime = event.timeSeconds;
      rows.push([
        result.result.method,
        String(result.result.stepsRejected),
        String(result.result.converged),
        stream.signalName,
        String(index),
        formatTableNumber(event.timeSeconds),
        normalizeDigitalState(event.state),
      ].join("\t"));
    });
  }
  rows.push("");
  return rows.join("\n");
}

export function formatCornerAdaptiveDigitalEventStreamTable(
  result: CornerAdaptiveDigitalTransientBridgeResult,
): string {
  const rows = [["Corner", "Method", "StepsRejected", "Converged", "Signal", "Index", "Time", "State"].join("\t")];
  for (const corner of result.points) {
    for (const stream of corner.result.outputStreams) {
      if (stream.signalName.trim().length === 0) {
        throw invalidElement("digitalEventStream", "digital event stream signal name must not be empty");
      }
      let previousTime = Number.NEGATIVE_INFINITY;
      stream.events.forEach((event, index) => {
        validateDigitalEventTime(event.timeSeconds, previousTime, stream.signalName);
        previousTime = event.timeSeconds;
        rows.push([
          corner.cornerName,
          corner.result.result.method,
          String(corner.result.result.stepsRejected),
          String(corner.result.result.converged),
          stream.signalName,
          String(index),
          formatTableNumber(event.timeSeconds),
          normalizeDigitalState(event.state),
        ].join("\t"));
      });
    }
  }
  rows.push("");
  return rows.join("\n");
}

export function formatDigitalBridgeScheduleTable(schedule: DigitalBridgeSchedule): string {
  if (!Number.isFinite(schedule.stopTime) || schedule.stopTime < 0.0) {
    throw invalidElement("digitalBridgeSchedule", "digital bridge stop time must be finite and non-negative");
  }
  const rows = [["Index", "Time", "StopTime"].join("\t")];
  let previousTime = Number.NEGATIVE_INFINITY;
  schedule.breakpoints.forEach((timeSeconds, index) => {
    validateDigitalEventTime(timeSeconds, previousTime, "digitalBridgeSchedule");
    if (timeSeconds > schedule.stopTime) {
      throw invalidElement("digitalBridgeSchedule", "digital bridge breakpoint must not exceed stop time");
    }
    previousTime = timeSeconds;
    rows.push([
      String(index),
      formatTableNumber(timeSeconds),
      formatTableNumber(schedule.stopTime),
    ].join("\t"));
  });
  rows.push("");
  return rows.join("\n");
}

export function formatDigitalEventStreamVcd(
  streams: readonly DigitalEventStream[],
  options: { readonly moduleName?: string; readonly timescale?: string } = {},
): string {
  const moduleName = (options.moduleName ?? "spice_bridge").trim();
  const timescale = options.timescale ?? "1ps";
  if (timescale !== "1ps") {
    throw invalidElement("digitalEventStreamVcd", "only 1ps timescale is supported");
  }
  if (moduleName.length === 0) {
    throw invalidElement("digitalEventStreamVcd", "module name must not be empty");
  }

  const seenSignalNames = new Set<string>();
  const signalIds = new Map<string, string>();
  streams.forEach((stream, index) => {
    const signalName = validateDigitalEventStreamName(stream, seenSignalNames);
    signalIds.set(signalName, vcdIdentifier(index));
    let previousTime = Number.NEGATIVE_INFINITY;
    for (const event of stream.events) {
      validateDigitalEventTime(event.timeSeconds, previousTime, signalName);
      normalizeDigitalState(event.state);
      previousTime = event.timeSeconds;
    }
  });

  const rows = [
    "$version coding-adventures spice-engine mixed-signal bridge $end",
    `$timescale ${timescale} $end`,
    `$scope module ${moduleName} $end`,
  ];
  for (const stream of streams) {
    const signalName = stream.signalName.trim();
    rows.push(`$var wire 1 ${signalIds.get(signalName)!} ${signalName} $end`);
  }
  rows.push("$upscope $end", "$enddefinitions $end", "$dumpvars");
  for (const stream of streams) {
    if (stream.events.length > 0) {
      rows.push(`${vcdStateValue(stream.events[0].state)}${signalIds.get(stream.signalName.trim())!}`);
    }
  }
  rows.push("$end");

  const eventsByTick = new Map<number, [string, DigitalState][]>();
  for (const stream of streams) {
    const signalId = signalIds.get(stream.signalName.trim())!;
    for (const event of stream.events) {
      const tick = vcdTick(event.timeSeconds);
      const existing = eventsByTick.get(tick) ?? [];
      existing.push([signalId, event.state]);
      eventsByTick.set(tick, existing);
    }
  }
  for (const tick of Array.from(eventsByTick.keys()).sort((left, right) => left - right)) {
    rows.push(`#${tick}`);
    for (const [signalId, state] of eventsByTick.get(tick)!) {
      rows.push(`${vcdStateValue(state)}${signalId}`);
    }
  }
  rows.push("");
  return rows.join("\n");
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

export function pssCorners(
  circuit: Circuit,
  corners: readonly CornerSpec[],
  stepsPerPeriod = 64,
  residualTolerance = 1.0e-6,
  perturbation = 1.0e-6,
  maxNewtonIterations = 8,
): CornerPssResult | undefined {
  const points: CornerPssPoint[] = [];
  for (const corner of corners) {
    const result = pss(
      circuitWithCorner(circuit, corner),
      stepsPerPeriod,
      residualTolerance,
      perturbation,
      maxNewtonIterations,
    );
    if (result === undefined) {
      return undefined;
    }
    points.push({ cornerName: corner.name, result });
  }
  return { points };
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
    case "custom-model":
      if (element.conductanceSiemens === undefined) {
        return undefined;
      }
      return {
        elementName: element.name,
        parameter: "conductanceSiemens",
        nominalValue: element.conductanceSiemens,
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
    case "b-source":
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
      case "custom-model":
        perturbed.add(element.conductanceSiemens === undefined
          ? element
          : { ...element, conductanceSiemens: element.conductanceSiemens + delta });
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
      case "b-source":
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
    case "custom-model":
      return element.conductanceSiemens === undefined
        ? element
        : {
            ...element,
            conductanceSiemens: randomizedValue(
              element.conductanceSiemens,
              tolerance,
              distribution,
              rng,
            ),
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
    case "capacitor":
    case "inductor":
    case "mutual-inductor":
    case "transmission-line":
    case "b-source":
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
  readonly maxDelta: number;
  readonly newtonStepLimit?: number;
  readonly limitedNewtonSteps: number;
  readonly minimumDampingFactor: number;
  readonly solverProfile: LinearSolverProfile;
}

interface LinearSystemSolve {
  readonly solution: readonly number[];
  readonly profile: LinearSolverProfile;
}

interface LinearSolveOptions {
  readonly maxIterations: number;
  readonly tolerance: number;
  readonly initialVector?: readonly number[];
  readonly returnSingularAsUnconverged?: boolean;
  readonly newtonStepLimit?: number;
}

interface ResolvedDcOpOptions {
  readonly maxIterations: number;
  readonly tolerance: number;
  readonly convergenceAids: boolean;
  readonly pseudoTransientSteps: number;
  readonly pseudoTransientConductance: number;
  readonly pseudoTransientMaxIterations: number;
  readonly newtonStepLimit?: number;
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
  readonly frequencyExponent: number;
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
      newtonStepLimit: undefined,
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
      newtonStepLimit: options.newtonStepLimit,
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
    element.kind === "b-source" ||
    element.kind === "custom-model"
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
      maxDelta: 0.0,
      limitedNewtonSteps: 0,
      minimumDampingFactor: 1.0,
      solverProfile: emptySolverProfile(0),
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
  const activeStepLimit = hasNonlinearElement ? options.newtonStepLimit : undefined;
  let limitedNewtonSteps = 0;
  let minimumDampingFactor = 1.0;
  const applyNewtonStepLimit = (candidate: LinearSolution): LinearSolution => {
    if (!candidate.converged) {
      return {
        ...candidate,
        newtonStepLimit: activeStepLimit,
        limitedNewtonSteps,
        minimumDampingFactor,
      };
    }
    const step = limitNewtonStep(operatingPoint, candidate.vector, activeStepLimit);
    if (step.limited) {
      limitedNewtonSteps += 1;
      minimumDampingFactor = Math.min(minimumDampingFactor, step.dampingFactor);
    }
    return {
      ...linearSolutionFromVector(
        circuit,
        inductorStates,
        nodeIndices,
        voltageSources,
        nodeCount,
        step.vector,
        candidate.converged,
        step.maxDelta,
        candidate.solverProfile,
      ),
      newtonStepLimit: activeStepLimit,
      limitedNewtonSteps,
      minimumDampingFactor,
    };
  };
  if (!hasNonlinearElement) {
    return { ...solution, iterations: 1, converged: solution.converged };
  }
  solution = applyNewtonStepLimit(solution);

  let iterations = 1;
  while (iterations < options.maxIterations) {
    if (!solution.converged) {
      return { ...solution, iterations, converged: false, maxDelta: Number.POSITIVE_INFINITY };
    }
    const delta = solution.maxDelta;
    operatingPoint = [...solution.vector];
    if (delta < options.tolerance) {
      return { ...solution, iterations, converged: true, maxDelta: delta };
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
    solution = applyNewtonStepLimit(solution);
    iterations += 1;
  }

  const delta = solution.maxDelta;
  return { ...solution, iterations, converged: delta < options.tolerance, maxDelta: delta };
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
    return { ...solution, iterations: 1, converged: true, maxDelta: 0.0 };
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
        Number.POSITIVE_INFINITY,
        emptySolverProfile(matrixSize),
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
  const newtonStepLimit =
    options.newtonStepLimit === null
      ? undefined
      : options.newtonStepLimit ?? DEFAULT_NEWTON_STEP_LIMIT;
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
  if (
    newtonStepLimit !== undefined &&
    (!Number.isFinite(newtonStepLimit) || newtonStepLimit <= 0.0)
  ) {
    throw invalidElement("dcOp", "newtonStepLimit must be finite and positive");
  }
  return {
    maxIterations,
    tolerance,
    convergenceAids: options.convergenceAids ?? true,
    pseudoTransientSteps,
    pseudoTransientConductance,
    pseudoTransientMaxIterations,
    newtonStepLimit,
  };
}

function validateDcInitialVector(circuit: Circuit, initialVector: readonly number[]): void {
  const nodeIndices = collectNodeIndices(circuit);
  const voltageSources = collectVoltageSources(circuit, []);
  const expectedLength = nodeIndices.size + voltageSources.size;
  if (initialVector.length !== expectedLength) {
    throw invalidElement(
      "dcInitialVector",
      `expected ${expectedLength} entries for circuit MNA ordering, got ${initialVector.length}`,
    );
  }
  if (initialVector.some((value) => !Number.isFinite(value))) {
    throw invalidElement("dcInitialVector", "all entries must be finite");
  }
}

function applyNodeConditionToInitialVector(
  condition: DeckNodeCondition,
  nodeIndices: ReadonlyMap<string, number>,
  vector: number[],
): void {
  if (!Number.isFinite(condition.value)) {
    throw invalidElement(condition.directive, `V(${condition.node}) must be finite`);
  }
  if (isGround(condition.node)) {
    if (condition.value !== 0.0) {
      throw invalidElement(condition.directive, `V(${condition.node}) conflicts with ground`);
    }
    return;
  }
  const index = nodeIndices.get(condition.node);
  if (index === undefined) {
    throw invalidElement(
      condition.directive,
      `references unknown node ${JSON.stringify(condition.node)}`,
    );
  }
  vector[index] = condition.value;
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

function circuitWithExtraVoltageSources(
  circuit: Circuit,
  sources: readonly VoltageSource[],
): Circuit {
  const bridged = new Circuit();
  for (const element of circuit.elements()) {
    bridged.add(element);
  }
  for (const source of sources) {
    bridged.add(source);
  }
  return bridged;
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

function normalizeDigitalState(state: DigitalState | string): DigitalState {
  const text = state.trim().toLowerCase();
  if (text === "low") {
    return "low";
  }
  if (text === "high") {
    return "high";
  }
  throw invalidElement("digitalEvent", `unsupported digital state ${state}`);
}

function validateDigitalLogicLevels(levels: DigitalLogicLevels, context: string): void {
  if (
    !Number.isFinite(levels.lowVoltage) ||
    !Number.isFinite(levels.highVoltage) ||
    !Number.isFinite(levels.transitionSeconds)
  ) {
    throw invalidElement(context, "digital logic levels must be finite");
  }
  if (levels.highVoltage <= levels.lowVoltage) {
    throw invalidElement(context, "digital high voltage must be greater than low voltage");
  }
  if (levels.transitionSeconds <= 0.0) {
    throw invalidElement(context, "digital transition time must be finite and positive");
  }
}

function validateDigitalThresholds(thresholds: DigitalThresholds): void {
  if (
    !Number.isFinite(thresholds.lowMaxVoltage) ||
    !Number.isFinite(thresholds.highMinVoltage)
  ) {
    throw invalidElement("digitalThresholds", "digital thresholds must be finite");
  }
  if (thresholds.highMinVoltage <= thresholds.lowMaxVoltage) {
    throw invalidElement("digitalThresholds", "digital high threshold must be greater than low threshold");
  }
}

function validateDigitalEventStreamName(
  stream: DigitalEventStream,
  seenSignalNames: Set<string>,
): string {
  const signalName = stream.signalName.trim();
  if (signalName.length === 0) {
    throw invalidElement("digitalEventStream", "digital event stream signal name must not be empty");
  }
  if (seenSignalNames.has(signalName)) {
    throw invalidElement(signalName, "digital event stream signal names must be unique");
  }
  seenSignalNames.add(signalName);
  return signalName;
}

function validateDigitalEventTime(
  timeSeconds: number,
  previousTime: number,
  context: string,
): void {
  if (!Number.isFinite(timeSeconds) || timeSeconds < 0.0) {
    throw invalidElement(context, "digital event times must be finite and non-negative");
  }
  if (timeSeconds <= previousTime) {
    throw invalidElement(context, "digital event times must be strictly increasing");
  }
}

function vcdIdentifier(index: number): string {
  return `s${index}`;
}

function vcdTick(timeSeconds: number): number {
  if (!Number.isFinite(timeSeconds) || timeSeconds < 0.0) {
    throw invalidElement("digitalEventStreamVcd", "event times must be finite and non-negative");
  }
  return Math.round(timeSeconds / 1.0e-12);
}

function vcdStateValue(state: DigitalState): string {
  return normalizeDigitalState(state) === "low" ? "0" : "1";
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
    case "custom-model":
      if (override.parameter === "conductance") {
        return { ...element, conductanceSiemens: override.value };
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
      case "custom-model":
        stampCustomModel(element, nodeIndices, matrix, rhs, operatingPoint);
        break;
      case "diode":
        stampDiode(element, capacitorStates, nodeIndices, matrix, rhs, operatingPoint);
        break;
      case "jfet":
        stampJfet(element, capacitorStates, nodeIndices, matrix, rhs, operatingPoint);
        break;
      case "bjt":
        stampBjt(element, capacitorStates, nodeIndices, matrix, rhs, operatingPoint);
        break;
      case "mosfet":
        stampMosfet(element, capacitorStates, nodeIndices, matrix, rhs, operatingPoint);
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

  const solved = solveLinearSystemWithProfile(matrix, rhs);
  return linearSolutionFromVector(
    circuit,
    inductorStates,
    nodeIndices,
    voltageSources,
    nodeCount,
    solved.solution,
    true,
    0.0,
    solved.profile,
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
  maxDelta: number,
  solverProfile: LinearSolverProfile,
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
    maxDelta,
    limitedNewtonSteps: 0,
    minimumDampingFactor: 1.0,
    solverProfile,
  };
}

function maxVectorDelta(left: readonly number[], right: readonly number[]): number {
  let max = 0.0;
  for (let index = 0; index < left.length; index++) {
    max = Math.max(max, Math.abs(left[index] - right[index]));
  }
  return max;
}

interface NewtonStepLimitResult {
  readonly vector: readonly number[];
  readonly maxDelta: number;
  readonly dampingFactor: number;
  readonly limited: boolean;
}

function limitNewtonStep(
  previous: readonly number[],
  candidate: readonly number[],
  stepLimit?: number,
): NewtonStepLimitResult {
  if (stepLimit === undefined) {
    return {
      vector: [...candidate],
      maxDelta: maxVectorDelta(previous, candidate),
      dampingFactor: 1.0,
      limited: false,
    };
  }

  const rawDelta = maxVectorDelta(previous, candidate);
  if (rawDelta <= stepLimit) {
    return {
      vector: [...candidate],
      maxDelta: rawDelta,
      dampingFactor: 1.0,
      limited: false,
    };
  }

  if (!Number.isFinite(rawDelta)) {
    return {
      vector: candidate.map((value, index) => {
        const delta = value - previous[index];
        if (!Number.isFinite(delta)) {
          return previous[index];
        }
        return previous[index] + Math.sign(delta) * stepLimit;
      }),
      maxDelta: stepLimit,
      dampingFactor: 0.0,
      limited: true,
    };
  }

  const dampingFactor = stepLimit / rawDelta;
  return {
    vector: candidate.map(
      (value, index) => previous[index] + (value - previous[index]) * dampingFactor,
    ),
    maxDelta: stepLimit,
    dampingFactor,
    limited: true,
  };
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
      case "custom-model":
        stampConductance(
          matrix,
          nodeIndex(nodeIndices, element.positive),
          nodeIndex(nodeIndices, element.negative),
          customModelConductance(element, nodeIndices, operatingPoint),
        );
        break;
      case "diode":
        validateDiode(element);
        const diodeAnode = diodeIntrinsicAnodeNode(element);
        const diodeVoltage = vectorVoltage(operatingPoint, nodeIndex(nodeIndices, diodeAnode)) -
          vectorVoltage(operatingPoint, nodeIndex(nodeIndices, element.cathode));
        const [, diodeConductance] = diodeCurrentConductance(element, diodeVoltage);
        stampConductance(
          matrix,
          nodeIndex(nodeIndices, diodeAnode),
          nodeIndex(nodeIndices, element.cathode),
          diodeConductance,
        );
        if (element.seriesResistance > 0.0) {
          stampConductance(
            matrix,
            nodeIndex(nodeIndices, element.anode),
            nodeIndex(nodeIndices, diodeAnode),
            1.0 / element.seriesResistance,
          );
        }
        break;
      case "jfet":
        stampJfetSmallSignal(element, nodeIndices, matrix, operatingPoint);
        break;
      case "bjt":
        stampBjtSmallSignal(element, nodeIndices, matrix, operatingPoint);
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
      case "custom-model":
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
      case "custom-model":
        stampComplexConductance(
          matrix,
          nodeIndex(nodeIndices, element.positive),
          nodeIndex(nodeIndices, element.negative),
          complex(customModelConductance(element, nodeIndices, operatingPoint), 0.0),
        );
        break;
      case "diode":
        validateDiode(element);
        const diodeAnode = diodeIntrinsicAnodeNode(element);
        const diodeVoltage = vectorVoltage(operatingPoint, nodeIndex(nodeIndices, diodeAnode)) -
          vectorVoltage(operatingPoint, nodeIndex(nodeIndices, element.cathode));
        const [, diodeConductance] = diodeCurrentConductance(element, diodeVoltage);
        const diodeCapacitance = diodeDynamicCapacitance(element, diodeVoltage);
        stampComplexConductance(
          matrix,
          nodeIndex(nodeIndices, diodeAnode),
          nodeIndex(nodeIndices, element.cathode),
          complex(diodeConductance, omega * diodeCapacitance),
        );
        if (element.seriesResistance > 0.0) {
          stampComplexConductance(
            matrix,
            nodeIndex(nodeIndices, element.anode),
            nodeIndex(nodeIndices, diodeAnode),
            complex(1.0 / element.seriesResistance, 0.0),
          );
        }
        break;
      case "jfet":
        stampAcJfetSmallSignal(element, nodeIndices, matrix, operatingPoint, omega);
        break;
      case "bjt":
        stampAcBjtSmallSignal(element, nodeIndices, matrix, operatingPoint, omega);
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
  diagnostics: DcSolverDiagnostics = {
    matrixSize: 0,
    solver: "none",
    tolerance: 0.0,
    maxDelta: 0.0,
    convergenceAid,
    limitedNewtonSteps: 0,
    minimumDampingFactor: 1.0,
    solverProfile: emptySolverProfile(0),
  },
): DcResult {
  return {
    nodeVoltages,
    branchCurrents,
    iterations,
    converged,
    convergenceAid,
    diagnostics,
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
      case "custom-model":
        insertNode(names, element.positive);
        insertNode(names, element.negative);
        break;
      case "diode":
        insertNode(names, element.anode);
        insertNode(names, element.cathode);
        if (element.seriesResistance > 0.0) {
          insertNode(names, diodeIntrinsicAnodeNode(element));
        }
        break;
      case "jfet":
        insertNode(names, element.drain);
        insertNode(names, element.gate);
        insertNode(names, element.source);
        if (element.drainResistance > 0.0) {
          insertNode(names, jfetIntrinsicDrainNode(element));
        }
        if (element.sourceResistance > 0.0) {
          insertNode(names, jfetIntrinsicSourceNode(element));
        }
        break;
      case "bjt":
        insertNode(names, element.collector);
        insertNode(names, element.base);
        insertNode(names, element.emitter);
        if (element.emitterResistance > 0.0) {
          insertNode(names, bjtIntrinsicEmitterNode(element));
        }
        if (element.collectorResistance > 0.0) {
          insertNode(names, bjtIntrinsicCollectorNode(element));
        }
        if (element.baseResistance > 0.0) {
          insertNode(names, bjtIntrinsicBaseNode(element));
        }
        break;
      case "mosfet":
        insertNode(names, element.drain);
        insertNode(names, element.gate);
        insertNode(names, element.source);
        insertNode(names, element.body);
        if (mosfetDrainResistance(element) > 0.0) {
          insertNode(names, mosfetIntrinsicDrainNode(element));
        }
        if (mosfetSourceResistance(element) > 0.0) {
          insertNode(names, mosfetIntrinsicSourceNode(element));
        }
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
        element.kind === "custom-model" ||
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

function jfetChannelNoiseConductance(
  element: Jfet,
  vgs: number,
  vds: number,
  gm: number,
): number {
  if (element.noiseEquationLevel < 3.0) {
    return MOSFET_CHANNEL_NOISE_GAMMA * Math.abs(gm);
  }
  const normalizedVgs = element.polarity === "NJF" ? vgs : -vgs;
  const normalizedVds = element.polarity === "NJF" ? vds : -vds;
  const thresholdVoltage =
    element.polarity === "NJF" ? element.thresholdVoltage : -element.thresholdVoltage;
  const overdrive = normalizedVgs - thresholdVoltage;
  if (overdrive <= 0.0 || normalizedVds < 0.0) {
    return 0.0;
  }
  const alpha =
    overdrive >= normalizedVds ? 1.0 - normalizedVds / overdrive : 0.0;
  return (
    MOSFET_CHANNEL_NOISE_GAMMA *
    element.beta *
    overdrive *
    (1.0 + alpha + alpha * alpha) /
    (1.0 + alpha) *
    element.channelNoiseCoefficient
  );
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
        frequencyExponent: 0.0,
      });
    } else if (element.kind === "diode") {
      validateDiode(element);
      const intrinsicAnode = diodeIntrinsicAnodeNode(element);
      const anode = nodeIndex(nodeIndices, intrinsicAnode);
      const cathode = nodeIndex(nodeIndices, element.cathode);
      const anodeVoltage = vectorVoltage(operatingPoint, anode);
      const cathodeVoltage = vectorVoltage(operatingPoint, cathode);
      const [current] = diodeCurrentConductance(element, anodeVoltage - cathodeVoltage);
      sources.push({
        elementName: element.name,
        noiseType: "shot",
        positive: anode,
        negative: cathode,
        sourcePsd: 2.0 * ELECTRON_CHARGE * Math.abs(current),
        frequencyExponent: 0.0,
      });
      if (element.flickerNoiseCoefficient > 0.0) {
        sources.push({
          elementName: element.name,
          noiseType: "flicker",
          positive: anode,
          negative: cathode,
          sourcePsd:
            element.flickerNoiseCoefficient *
            Math.abs(current) ** element.flickerNoiseExponent,
          frequencyExponent: 1.0,
        });
      }
      if (element.seriesResistance > 0.0) {
        sources.push({
          elementName: `${element.name}:RS`,
          noiseType: "thermal",
          positive: nodeIndex(nodeIndices, element.anode),
          negative: anode,
          sourcePsd: 4.0 * BOLTZMANN * temperatureKelvin / element.seriesResistance,
          frequencyExponent: 0.0,
        });
      }
    } else if (element.kind === "bjt") {
      validateBjt(element);
      const emitterNode = bjtIntrinsicEmitterNode(element);
      const collectorNode = bjtIntrinsicCollectorNode(element);
      const baseNode = bjtIntrinsicBaseNode(element);
      const base = nodeIndex(nodeIndices, baseNode);
      const emitter = nodeIndex(nodeIndices, emitterNode);
      const collector = nodeIndex(nodeIndices, collectorNode);
      const baseVoltage = vectorVoltage(operatingPoint, base);
      const emitterVoltage = vectorVoltage(operatingPoint, emitter);
      const collectorVoltage = vectorVoltage(operatingPoint, collector);
      if (element.emitterResistance > 0.0) {
        sources.push({
          elementName: `${element.name}:RE`,
          noiseType: "thermal",
          positive: nodeIndex(nodeIndices, element.emitter),
          negative: nodeIndex(nodeIndices, emitterNode),
          sourcePsd:
            4.0 * BOLTZMANN * temperatureKelvin / element.emitterResistance,
          frequencyExponent: 0.0,
        });
      }
      if (element.collectorResistance > 0.0) {
        sources.push({
          elementName: `${element.name}:RC`,
          noiseType: "thermal",
          positive: nodeIndex(nodeIndices, element.collector),
          negative: nodeIndex(nodeIndices, collectorNode),
          sourcePsd:
            4.0 * BOLTZMANN * temperatureKelvin / element.collectorResistance,
          frequencyExponent: 0.0,
        });
      }
      if (element.baseResistance > 0.0) {
        const baseResistance = bjtEffectiveBaseResistance(
          element,
          baseVoltage,
          emitterVoltage,
          collectorVoltage,
        );
        sources.push({
          elementName: `${element.name}:RB`,
          noiseType: "thermal",
          positive: nodeIndex(nodeIndices, element.base),
          negative: nodeIndex(nodeIndices, baseNode),
          sourcePsd:
            4.0 * BOLTZMANN * temperatureKelvin / baseResistance,
          frequencyExponent: 0.0,
        });
      }
      const junctionVoltage =
        element.polarity === "NPN"
          ? baseVoltage - emitterVoltage
          : emitterVoltage - baseVoltage;
      const reverseJunctionVoltage = element.polarity === "NPN"
        ? baseVoltage - collectorVoltage
        : collectorVoltage - baseVoltage;
      const forwardThermalVoltage = element.thermalVoltage * element.forwardEmissionCoefficient;
      const exponent = Math.max(-40.0, Math.min(40.0, junctionVoltage / forwardThermalVoltage));
      const outputVoltage = element.polarity === "NPN"
        ? collectorVoltage - emitterVoltage
        : emitterVoltage - collectorVoltage;
      const earlyFactor = bjtEarlyFactor(element, junctionVoltage, outputVoltage);
      const expValue = Math.exp(exponent);
      const baseCollectorCurrent = element.saturationCurrent * (expValue - 1.0);
      const baseTransconductance =
        element.saturationCurrent / forwardThermalVoltage * expValue;
      const collectorCurrent = bjtForwardTransport(
        element,
        baseCollectorCurrent,
        baseTransconductance,
        earlyFactor,
      ).collectorCurrent;
      const leakageCurrent = bjtBaseEmitterLeakage(element, junctionVoltage).current;
      const collectorLeakageCurrent =
        bjtBaseCollectorLeakage(element, reverseJunctionVoltage).current;
      const reverseBaseCurrent =
        bjtReverseBaseCurrent(element, reverseJunctionVoltage).current;
      sources.push({
        elementName: element.name,
        noiseType: "shot",
        positive: element.polarity === "NPN" ? base : emitter,
        negative: element.polarity === "NPN" ? emitter : base,
        sourcePsd:
          2.0 * ELECTRON_CHARGE *
          (Math.abs(collectorCurrent) + Math.abs(leakageCurrent) +
            Math.abs(collectorLeakageCurrent) + Math.abs(reverseBaseCurrent)),
        frequencyExponent: 0.0,
      });
      if (element.flickerNoiseCoefficient > 0.0) {
        const baseCurrent =
          baseCollectorCurrent / element.forwardBeta + leakageCurrent;
        sources.push({
          elementName: element.name,
          noiseType: "flicker",
          positive: element.polarity === "NPN" ? base : emitter,
          negative: element.polarity === "NPN" ? emitter : base,
          sourcePsd:
            element.flickerNoiseCoefficient * Math.abs(baseCurrent) ** element.flickerNoiseExponent,
          frequencyExponent: 1.0,
        });
      }
    } else if (element.kind === "jfet") {
      validateJfet(element);
      const intrinsicDrain = jfetIntrinsicDrainNode(element);
      const intrinsicSource = jfetIntrinsicSourceNode(element);
      const drain = nodeIndex(nodeIndices, intrinsicDrain);
      const gate = nodeIndex(nodeIndices, element.gate);
      const source = nodeIndex(nodeIndices, intrinsicSource);
      const drainVoltage = vectorVoltage(operatingPoint, drain);
      const gateVoltage = vectorVoltage(operatingPoint, gate);
      const sourceVoltage = vectorVoltage(operatingPoint, source);
      const result = evaluateJfet(
        element,
        gateVoltage - sourceVoltage,
        drainVoltage - sourceVoltage,
      );
      const gm = Math.max(0.0, result.gm);
      const noiseConductance = jfetChannelNoiseConductance(
        element,
        gateVoltage - sourceVoltage,
        drainVoltage - sourceVoltage,
        gm,
      );
      if (noiseConductance > 0.0) {
        sources.push({
          elementName: element.name,
          noiseType: "thermal",
          positive: drain,
          negative: source,
          sourcePsd: 4.0 * BOLTZMANN * temperatureKelvin * noiseConductance,
          frequencyExponent: 0.0,
        });
      }
      const [gateSourceCurrent] = jfetGateJunctionCurrentConductance(
        element,
        gateVoltage - sourceVoltage,
      );
      const [gateDrainCurrent] = jfetGateJunctionCurrentConductance(
        element,
        gateVoltage - drainVoltage,
      );
      sources.push({
        elementName: `${element.name}:IGS`,
        noiseType: "shot",
        positive: gate,
        negative: source,
        sourcePsd: 2.0 * ELECTRON_CHARGE * Math.abs(gateSourceCurrent),
        frequencyExponent: 0.0,
      });
      sources.push({
        elementName: `${element.name}:IGD`,
        noiseType: "shot",
        positive: gate,
        negative: drain,
        sourcePsd: 2.0 * ELECTRON_CHARGE * Math.abs(gateDrainCurrent),
        frequencyExponent: 0.0,
      });
      if (element.flickerNoiseCoefficient > 0.0) {
        sources.push({
          elementName: element.name,
          noiseType: "flicker",
          positive: drain,
          negative: source,
          sourcePsd:
            element.flickerNoiseCoefficient *
            Math.abs(result.drainCurrent) ** element.flickerNoiseExponent,
          frequencyExponent: 1.0,
        });
      }
      if (element.drainResistance > 0.0) {
        sources.push({
          elementName: `${element.name}:RD`,
          noiseType: "thermal",
          positive: nodeIndex(nodeIndices, element.drain),
          negative: drain,
          sourcePsd:
            4.0 * BOLTZMANN * temperatureKelvin / element.drainResistance,
          frequencyExponent: 0.0,
        });
      }
      if (element.sourceResistance > 0.0) {
        sources.push({
          elementName: `${element.name}:RS`,
          noiseType: "thermal",
          positive: nodeIndex(nodeIndices, element.source),
          negative: source,
          sourcePsd:
            4.0 * BOLTZMANN * temperatureKelvin / element.sourceResistance,
          frequencyExponent: 0.0,
        });
      }
    } else if (element.kind === "mosfet") {
      validateMosfet(element);
      const drain = nodeIndex(nodeIndices, mosfetIntrinsicDrainNode(element));
      const gate = nodeIndex(nodeIndices, element.gate);
      const source = nodeIndex(nodeIndices, mosfetIntrinsicSourceNode(element));
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
          frequencyExponent: 0.0,
        });
      }
      if (element.params.KF > 0.0) {
        sources.push({
          elementName: element.name,
          noiseType: "flicker",
          positive: drain,
          negative: source,
          sourcePsd:
            element.params.KF *
            Math.abs(result.drainCurrent) ** element.params.AF,
          frequencyExponent: 1.0,
        });
      }
      const [sourceBulkCurrent] = mosfetBulkJunctionCurrentConductance(
        element,
        sourceVoltage,
        bodyVoltage,
        element.params.AS,
      );
      const [drainBulkCurrent] = mosfetBulkJunctionCurrentConductance(
        element,
        drainVoltage,
        bodyVoltage,
        element.params.AD,
      );
      const isNmos = element.type === "NMOS";
      sources.push({
        elementName: `${element.name}:IBS`,
        noiseType: "shot",
        positive: isNmos ? body : source,
        negative: isNmos ? source : body,
        sourcePsd: 2.0 * ELECTRON_CHARGE * Math.abs(sourceBulkCurrent),
        frequencyExponent: 0.0,
      });
      sources.push({
        elementName: `${element.name}:IBD`,
        noiseType: "shot",
        positive: isNmos ? body : drain,
        negative: isNmos ? drain : body,
        sourcePsd: 2.0 * ELECTRON_CHARGE * Math.abs(drainBulkCurrent),
        frequencyExponent: 0.0,
      });
      const drainResistance = mosfetDrainResistance(element);
      if (drainResistance > 0.0) {
        sources.push({
          elementName: `${element.name}:RD`,
          noiseType: "thermal",
          positive: nodeIndex(nodeIndices, element.drain),
          negative: drain,
          sourcePsd: 4.0 * BOLTZMANN * temperatureKelvin / drainResistance,
          frequencyExponent: 0.0,
        });
      }
      const sourceResistance = mosfetSourceResistance(element);
      if (sourceResistance > 0.0) {
        sources.push({
          elementName: `${element.name}:RS`,
          noiseType: "thermal",
          positive: nodeIndex(nodeIndices, element.source),
          negative: source,
          sourcePsd: 4.0 * BOLTZMANN * temperatureKelvin / sourceResistance,
          frequencyExponent: 0.0,
        });
      }
    }
  }
  return sources;
}

function zeroNoiseEntries(
  sources: readonly NoiseSource[],
  frequencyHz: number,
): NoiseEntry[] {
  return sources.map((source) => ({
    elementName: source.elementName,
    noiseType: source.noiseType,
    sourcePsd: source.sourcePsd / frequencyHz ** source.frequencyExponent,
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

function validateCustomModel(element: CustomModel): void {
  if (!Number.isFinite(element.currentOffsetAmps)) {
    throw invalidElement(element.name, "custom-model current offset must be finite");
  }
  for (const [name, value] of Object.entries(element.parameters)) {
    if (!Number.isFinite(value)) {
      throw invalidElement(element.name, `custom-model parameter ${name} must be finite`);
    }
  }
  if (element.evaluator === undefined) {
    if (
      element.conductanceSiemens === undefined ||
      !Number.isFinite(element.conductanceSiemens)
    ) {
      throw invalidElement(
        element.name,
        "custom-model conductance must be finite when no evaluator is supplied",
      );
    }
  } else if (
    element.conductanceSiemens !== undefined &&
    !Number.isFinite(element.conductanceSiemens)
  ) {
    throw invalidElement(element.name, "custom-model conductance must be finite");
  }
}

function customModelVoltage(
  element: CustomModel,
  nodeIndices: ReadonlyMap<string, number>,
  operatingPoint: readonly number[],
): number {
  return (
    vectorVoltage(operatingPoint, nodeIndex(nodeIndices, element.positive)) -
    vectorVoltage(operatingPoint, nodeIndex(nodeIndices, element.negative))
  );
}

function evaluateCustomModel(
  element: CustomModel,
  voltage: number,
): CustomModelEvaluation {
  validateCustomModel(element);
  const evaluation = element.evaluator === undefined
    ? {
        currentAmps:
          element.conductanceSiemens! * voltage + element.currentOffsetAmps,
        conductanceSiemens: element.conductanceSiemens!,
      }
    : element.evaluator({
        voltage,
        temperatureKelvin: 300.15,
        parameters: element.parameters,
      });
  if (!Number.isFinite(evaluation.currentAmps)) {
    throw invalidElement(element.name, "custom-model current must be finite");
  }
  if (!Number.isFinite(evaluation.conductanceSiemens)) {
    throw invalidElement(element.name, "custom-model conductance must be finite");
  }
  return evaluation;
}

function stampCustomModel(
  element: CustomModel,
  nodeIndices: ReadonlyMap<string, number>,
  matrix: number[][],
  rhs: number[],
  operatingPoint: readonly number[],
): void {
  const positive = nodeIndex(nodeIndices, element.positive);
  const negative = nodeIndex(nodeIndices, element.negative);
  const voltage = customModelVoltage(element, nodeIndices, operatingPoint);
  const evaluation = evaluateCustomModel(element, voltage);
  const equivalentCurrent =
    evaluation.currentAmps - evaluation.conductanceSiemens * voltage;

  stampConductance(matrix, positive, negative, evaluation.conductanceSiemens);
  if (positive !== undefined) {
    rhs[positive] -= equivalentCurrent;
  }
  if (negative !== undefined) {
    rhs[negative] += equivalentCurrent;
  }
}

function customModelConductance(
  element: CustomModel,
  nodeIndices: ReadonlyMap<string, number>,
  operatingPoint: readonly number[],
): number {
  return evaluateCustomModel(
    element,
    customModelVoltage(element, nodeIndices, operatingPoint),
  ).conductanceSiemens;
}

function stampDiode(
  element: Diode,
  capacitorStates: readonly CapacitorState[],
  nodeIndices: ReadonlyMap<string, number>,
  matrix: number[][],
  rhs: number[],
  operatingPoint: readonly number[],
): void {
  validateDiode(element);
  const intrinsicAnode = diodeIntrinsicAnodeNode(element);
  const anode = nodeIndex(nodeIndices, intrinsicAnode);
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
  if (element.seriesResistance > 0.0) {
    stampConductance(
      matrix,
      nodeIndex(nodeIndices, element.anode),
      anode,
      1.0 / element.seriesResistance,
    );
  }
  stampDiodeCharge(element, capacitorStates, nodeIndices, matrix, rhs);
}

function stampDiodeCharge(
  element: Diode,
  capacitorStates: readonly CapacitorState[],
  nodeIndices: ReadonlyMap<string, number>,
  matrix: number[][],
  rhs: number[],
): void {
  const state = capacitorStates.find(
    (candidate) => candidate.name === diodeChargeStateName(element),
  );
  if (state === undefined) {
    return;
  }
  const capacitance = diodeDynamicCapacitance(element, state.previousVoltage);
  if (capacitance <= 0.0) {
    return;
  }
  const conductance =
    state.method === "trap"
      ? (2.0 * capacitance) / state.timeStep
      : state.method === "gear2"
        ? (3.0 * capacitance) / (2.0 * state.timeStep)
        : capacitance / state.timeStep;
  const historyCurrent =
    state.method === "trap"
      ? conductance * state.previousVoltage + state.previousCurrent
      : state.method === "gear2"
        ? capacitance *
          (4.0 * state.previousVoltage - state.previousPreviousVoltage) /
          (2.0 * state.timeStep)
        : conductance * state.previousVoltage;
  const anode = nodeIndex(nodeIndices, diodeIntrinsicAnodeNode(element));
  const cathode = nodeIndex(nodeIndices, element.cathode);
  stampConductance(matrix, anode, cathode, conductance);
  if (anode !== undefined) {
    rhs[anode] += historyCurrent;
  }
  if (cathode !== undefined) {
    rhs[cathode] -= historyCurrent;
  }
}

function bjtEarlyFactor(
  element: Bjt,
  junctionVoltage: number,
  outputVoltage: number,
): number {
  const forwardTerm = element.forwardEarlyVoltage === 0.0
    ? 0.0
    : outputVoltage / element.forwardEarlyVoltage;
  const reverseTerm = element.reverseEarlyVoltage === 0.0
    ? 0.0
    : junctionVoltage / element.reverseEarlyVoltage;
  return 1.0 + forwardTerm - reverseTerm;
}

function bjtForwardTransconductance(
  element: Bjt,
  baseCollectorCurrent: number,
  baseTransconductance: number,
  earlyFactor: number,
): number {
  const reverseEarlyConductance = element.reverseEarlyVoltage === 0.0
    ? 0.0
    : baseCollectorCurrent / element.reverseEarlyVoltage;
  return baseTransconductance * earlyFactor - reverseEarlyConductance;
}

function bjtForwardTransport(
  element: Bjt,
  baseCollectorCurrent: number,
  baseTransconductance: number,
  earlyFactor: number,
): { collectorCurrent: number; transconductance: number; chargeFactor: number } {
  const lowCurrentTransconductance = bjtForwardTransconductance(
    element,
    baseCollectorCurrent,
    baseTransconductance,
    earlyFactor,
  );
  if (element.forwardBetaRolloffCurrent === 0.0 || baseCollectorCurrent <= 0.0) {
    return {
      collectorCurrent: baseCollectorCurrent * earlyFactor,
      transconductance: lowCurrentTransconductance,
      chargeFactor: 1.0,
    };
  }
  const root = Math.sqrt(
    1.0 + 4.0 * baseCollectorCurrent / element.forwardBetaRolloffCurrent,
  );
  const chargeFactor = 0.5 * (1.0 + root);
  const chargeDerivative =
    baseTransconductance / (element.forwardBetaRolloffCurrent * root);
  return {
    collectorCurrent: baseCollectorCurrent * earlyFactor / chargeFactor,
    transconductance:
      lowCurrentTransconductance / chargeFactor
      - baseCollectorCurrent * earlyFactor * chargeDerivative / chargeFactor ** 2,
    chargeFactor,
  };
}

function bjtBaseEmitterLeakage(
  element: Bjt,
  junctionVoltage: number,
): { current: number; conductance: number } {
  if (element.baseEmitterLeakageSaturationCurrent === 0.0) {
    return { current: 0.0, conductance: 0.0 };
  }
  const thermalVoltage =
    element.thermalVoltage * element.baseEmitterLeakageEmissionCoefficient;
  const exponent = Math.max(-40.0, Math.min(40.0, junctionVoltage / thermalVoltage));
  const expValue = Math.exp(exponent);
  return {
    current: element.baseEmitterLeakageSaturationCurrent * (expValue - 1.0),
    conductance:
      element.baseEmitterLeakageSaturationCurrent / thermalVoltage * expValue,
  };
}

function bjtBaseCollectorLeakage(
  element: Bjt,
  junctionVoltage: number,
): { current: number; conductance: number } {
  if (element.baseCollectorLeakageSaturationCurrent === 0.0) {
    return { current: 0.0, conductance: 0.0 };
  }
  const thermalVoltage =
    element.thermalVoltage * element.baseCollectorLeakageEmissionCoefficient;
  const exponent = Math.max(-40.0, Math.min(40.0, junctionVoltage / thermalVoltage));
  const expValue = Math.exp(exponent);
  return {
    current: element.baseCollectorLeakageSaturationCurrent * (expValue - 1.0),
    conductance:
      element.baseCollectorLeakageSaturationCurrent / thermalVoltage * expValue,
  };
}

function bjtReverseBaseCurrent(
  element: Bjt,
  junctionVoltage: number,
): { current: number; conductance: number } {
  if (element.reverseBeta === Number.POSITIVE_INFINITY) {
    return { current: 0.0, conductance: 0.0 };
  }
  const thermalVoltage = element.thermalVoltage * element.reverseEmissionCoefficient;
  const exponent = Math.max(-40.0, Math.min(40.0, junctionVoltage / thermalVoltage));
  const expValue = Math.exp(exponent);
  const diffusionCurrent = element.saturationCurrent * (expValue - 1.0);
  const diffusionConductance = element.saturationCurrent / thermalVoltage * expValue;
  if (element.reverseBetaRolloffCurrent === 0.0 || diffusionCurrent <= 0.0) {
    return {
      current: diffusionCurrent / element.reverseBeta,
      conductance: diffusionConductance / element.reverseBeta,
    };
  }
  const root = Math.sqrt(
    1.0 + 4.0 * diffusionCurrent / element.reverseBetaRolloffCurrent,
  );
  const chargeFactor = 0.5 * (1.0 + root);
  const chargeDerivative =
    diffusionConductance / (element.reverseBetaRolloffCurrent * root);
  return {
    current: diffusionCurrent * chargeFactor / element.reverseBeta,
    conductance:
      (diffusionConductance * chargeFactor + diffusionCurrent * chargeDerivative)
      / element.reverseBeta,
  };
}

function bjtEffectiveBaseResistance(
  element: Bjt,
  baseVoltage: number,
  emitterVoltage: number,
  collectorVoltage: number,
): number {
  const minimum = element.minimumBaseResistance ?? element.baseResistance;
  if (minimum === element.baseResistance) {
    return element.baseResistance;
  }
  const junctionVoltage = element.polarity === "NPN"
    ? baseVoltage - emitterVoltage
    : emitterVoltage - baseVoltage;
  const reverseVoltage = element.polarity === "NPN"
    ? baseVoltage - collectorVoltage
    : collectorVoltage - baseVoltage;
  const outputVoltage = element.polarity === "NPN"
    ? collectorVoltage - emitterVoltage
    : emitterVoltage - collectorVoltage;
  const forwardThermalVoltage =
    element.thermalVoltage * element.forwardEmissionCoefficient;
  const exponent = Math.max(
    -40.0,
    Math.min(40.0, junctionVoltage / forwardThermalVoltage),
  );
  const expValue = Math.exp(exponent);
  const diffusionCurrent = element.saturationCurrent * (expValue - 1.0);
  const diffusionConductance =
    element.saturationCurrent / forwardThermalVoltage * expValue;
  const earlyFactor = bjtEarlyFactor(element, junctionVoltage, outputVoltage);
  const transport = bjtForwardTransport(
    element,
    diffusionCurrent,
    diffusionConductance,
    earlyFactor,
  );
  const leakage = bjtBaseEmitterLeakage(element, junctionVoltage);
  const collectorLeakage = bjtBaseCollectorLeakage(element, reverseVoltage);
  const reverseBase = bjtReverseBaseCurrent(element, reverseVoltage);
  const baseCurrent = diffusionCurrent / element.forwardBeta
    + leakage.current
    + collectorLeakage.current
    + reverseBase.current;
  const variableResistance = element.baseResistance - minimum;
  if (element.baseResistanceHalfCurrent === 0.0) {
    return minimum + variableResistance / transport.chargeFactor;
  }
  const ratio = Math.max(baseCurrent / element.baseResistanceHalfCurrent, 1.0e-9);
  const angle =
    (-1.0 + Math.sqrt(1.0 + 14.59025 * ratio))
    / (2.4317 * Math.sqrt(ratio));
  const tangent = Math.tan(angle);
  const transition =
    3.0 * (tangent - angle) / (angle * tangent * tangent);
  return minimum + variableResistance * transition;
}

function stampBjt(
  element: Bjt,
  capacitorStates: readonly CapacitorState[],
  nodeIndices: ReadonlyMap<string, number>,
  matrix: number[][],
  rhs: number[],
  operatingPoint: readonly number[],
): void {
  validateBjt(element);
  const chargeElement = element;
  if (element.emitterResistance > 0.0) {
    const intrinsicEmitter = bjtIntrinsicEmitterNode(element);
    stampConductance(
      matrix,
      nodeIndex(nodeIndices, element.emitter),
      nodeIndex(nodeIndices, intrinsicEmitter),
      1.0 / element.emitterResistance,
    );
    element = { ...element, emitter: intrinsicEmitter, emitterResistance: 0.0 };
  }
  if (element.collectorResistance > 0.0) {
    const intrinsicCollector = bjtIntrinsicCollectorNode(element);
    stampConductance(
      matrix,
      nodeIndex(nodeIndices, element.collector),
      nodeIndex(nodeIndices, intrinsicCollector),
      1.0 / element.collectorResistance,
    );
    element = { ...element, collector: intrinsicCollector, collectorResistance: 0.0 };
  }
  if (element.baseResistance > 0.0) {
    const intrinsicBase = bjtIntrinsicBaseNode(element);
    const intrinsicBaseIndex = nodeIndex(nodeIndices, intrinsicBase);
    const baseVoltage = vectorVoltage(operatingPoint, intrinsicBaseIndex);
    const emitterVoltage = vectorVoltage(
      operatingPoint,
      nodeIndex(nodeIndices, element.emitter),
    );
    const collectorVoltage = vectorVoltage(
      operatingPoint,
      nodeIndex(nodeIndices, element.collector),
    );
    const baseResistance = bjtEffectiveBaseResistance(
      element,
      baseVoltage,
      emitterVoltage,
      collectorVoltage,
    );
    stampConductance(
      matrix,
      nodeIndex(nodeIndices, element.base),
      intrinsicBaseIndex,
      1.0 / baseResistance,
    );
    element = {
      ...element,
      base: intrinsicBase,
      baseResistance: 0.0,
      minimumBaseResistance: undefined,
      baseResistanceHalfCurrent: 0.0,
    };
  }
  const collector = nodeIndex(nodeIndices, element.collector);
  const base = nodeIndex(nodeIndices, element.base);
  const emitter = nodeIndex(nodeIndices, element.emitter);
  const baseVoltage = base === undefined ? 0.0 : operatingPoint[base];
  const emitterVoltage = emitter === undefined ? 0.0 : operatingPoint[emitter];
  const collectorVoltage = collector === undefined ? 0.0 : operatingPoint[collector];

  const junctionVoltage =
    element.polarity === "NPN"
      ? baseVoltage - emitterVoltage
      : emitterVoltage - baseVoltage;
  const reverseJunctionVoltage = element.polarity === "NPN"
    ? baseVoltage - collectorVoltage
    : collectorVoltage - baseVoltage;
  const forwardThermalVoltage = element.thermalVoltage * element.forwardEmissionCoefficient;
  const exponent = Math.max(-40.0, Math.min(40.0, junctionVoltage / forwardThermalVoltage));
  const expValue = Math.exp(exponent);
  const baseCollectorCurrent = element.saturationCurrent * (expValue - 1.0);
  const baseTransconductance = element.saturationCurrent / forwardThermalVoltage * expValue;
  const outputVoltage = element.polarity === "NPN"
    ? collectorVoltage - emitterVoltage
    : emitterVoltage - collectorVoltage;
  const earlyFactor = bjtEarlyFactor(element, junctionVoltage, outputVoltage);
  const transport = bjtForwardTransport(
    element,
    baseCollectorCurrent,
    baseTransconductance,
    earlyFactor,
  );
  const outputConductance = element.forwardEarlyVoltage === 0.0
    ? 0.0
    : baseCollectorCurrent / element.forwardEarlyVoltage / transport.chargeFactor;
  const collectorCurrent = transport.collectorCurrent;
  const transconductance = transport.transconductance;
  const leakage = bjtBaseEmitterLeakage(element, junctionVoltage);
  const junctionConductance =
    baseTransconductance / element.forwardBeta + leakage.conductance;
  const baseCurrent = baseCollectorCurrent / element.forwardBeta + leakage.current;
  const equivalentCollectorCurrent =
    collectorCurrent - transconductance * junctionVoltage - outputConductance * outputVoltage;
  const equivalentBaseCurrent =
    baseCurrent - junctionConductance * junctionVoltage;
  const collectorLeakage = bjtBaseCollectorLeakage(element, reverseJunctionVoltage);
  const reverseBase = bjtReverseBaseCurrent(element, reverseJunctionVoltage);
  const baseCollectorJunctionCurrent = collectorLeakage.current + reverseBase.current;
  const baseCollectorConductance = collectorLeakage.conductance + reverseBase.conductance;
  const equivalentCollectorLeakageCurrent =
    baseCollectorJunctionCurrent - baseCollectorConductance * reverseJunctionVoltage;

  stampConductance(matrix, collector, emitter, outputConductance);
  stampConductance(matrix, base, collector, baseCollectorConductance);
  if (element.polarity === "NPN") {
    stampConductance(matrix, base, emitter, junctionConductance);
    stampTransconductance(matrix, collector, emitter, base, emitter, transconductance);
    stampCurrentSourceEquivalent(rhs, base, emitter, equivalentBaseCurrent);
    stampCurrentSourceEquivalent(rhs, collector, emitter, equivalentCollectorCurrent);
    stampCurrentSourceEquivalent(rhs, base, collector, equivalentCollectorLeakageCurrent);
  } else {
    stampConductance(matrix, emitter, base, junctionConductance);
    stampTransconductance(matrix, emitter, collector, emitter, base, transconductance);
    stampCurrentSourceEquivalent(rhs, emitter, base, equivalentBaseCurrent);
    stampCurrentSourceEquivalent(rhs, emitter, collector, equivalentCollectorCurrent);
    stampCurrentSourceEquivalent(rhs, collector, base, equivalentCollectorLeakageCurrent);
  }
  stampBjtCharge(chargeElement, capacitorStates, nodeIndices, matrix, rhs);
}

function stampBjtCharge(
  element: Bjt,
  capacitorStates: readonly CapacitorState[],
  nodeIndices: ReadonlyMap<string, number>,
  matrix: number[][],
  rhs: number[],
): void {
  const reverseJunctionVoltage =
    capacitorStates.find(
      (state) => state.name === bjtBaseCollectorChargeStateName(element),
    )?.previousVoltage ?? 0.0;
  for (const spec of bjtChargeStateSpecs(element)) {
    const state = capacitorStates.find((candidate) => candidate.name === spec.name);
    if (state === undefined) {
      continue;
    }
    const capacitance = bjtChargeDynamicCapacitance(
      element,
      spec.kind,
      state.previousVoltage,
      reverseJunctionVoltage,
    );
    if (capacitance <= 0.0) {
      continue;
    }
    const conductance =
      state.method === "trap"
        ? (2.0 * capacitance) / state.timeStep
        : state.method === "gear2"
          ? (3.0 * capacitance) / (2.0 * state.timeStep)
          : capacitance / state.timeStep;
    const historyCurrent =
      state.method === "trap"
        ? conductance * state.previousVoltage + state.previousCurrent
        : state.method === "gear2"
          ? capacitance *
            (4.0 * state.previousVoltage - state.previousPreviousVoltage) /
            (2.0 * state.timeStep)
          : conductance * state.previousVoltage;
    const positive = nodeIndex(nodeIndices, spec.positive);
    const negative = nodeIndex(nodeIndices, spec.negative);
    stampConductance(matrix, positive, negative, conductance);
    if (positive !== undefined) {
      rhs[positive] += historyCurrent;
    }
    if (negative !== undefined) {
      rhs[negative] -= historyCurrent;
    }
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

function mosfetBulkJunctionCapacitance(
  zeroBiasCapacitance: number,
  junctionVoltage: number,
  junctionPotential: number,
  gradingCoefficient: number,
  forwardBiasCoefficient: number,
): number {
  if (zeroBiasCapacitance <= 0.0) {
    return zeroBiasCapacitance;
  }
  if (junctionPotential <= 0.0 || gradingCoefficient === 0.0) {
    return zeroBiasCapacitance;
  }
  const normalizedVoltage = junctionVoltage / junctionPotential;
  if (normalizedVoltage < forwardBiasCoefficient) {
    return zeroBiasCapacitance / ((1.0 - normalizedVoltage) ** gradingCoefficient);
  }
  const denominator = (1.0 - forwardBiasCoefficient) ** (1.0 + gradingCoefficient);
  const continuation =
    1.0 -
    forwardBiasCoefficient * (1.0 + gradingCoefficient) +
    gradingCoefficient * normalizedVoltage;
  return zeroBiasCapacitance * continuation / denominator;
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
  if (!Number.isFinite(element.gateSourceCapacitance) || element.gateSourceCapacitance < 0.0) {
    throw invalidElement(element.name, "gate-source capacitance must be finite and non-negative");
  }
  if (!Number.isFinite(element.gateDrainCapacitance) || element.gateDrainCapacitance < 0.0) {
    throw invalidElement(element.name, "gate-drain capacitance must be finite and non-negative");
  }
  if (!Number.isFinite(element.flickerNoiseCoefficient) || element.flickerNoiseCoefficient < 0.0) {
    throw invalidElement(
      element.name,
      "flicker-noise coefficient must be finite and non-negative",
    );
  }
  if (!Number.isFinite(element.flickerNoiseExponent) || element.flickerNoiseExponent < 0.0) {
    throw invalidElement(
      element.name,
      "flicker-noise exponent must be finite and non-negative",
    );
  }
  if (!Number.isFinite(element.junctionPotential) || element.junctionPotential <= 0.0) {
    throw invalidElement(element.name, "junction potential must be finite and positive");
  }
  if (!Number.isFinite(element.forwardBiasDepletionCoefficient) ||
      element.forwardBiasDepletionCoefficient < 0.0 ||
      element.forwardBiasDepletionCoefficient >= 1.0) {
    throw invalidElement(
      element.name,
      "forward-bias depletion coefficient must be finite and in [0, 1)",
    );
  }
  if (!Number.isFinite(element.gateSaturationCurrent) ||
      element.gateSaturationCurrent < 0.0) {
    throw invalidElement(
      element.name,
      "gate saturation current must be finite and non-negative",
    );
  }
  if (!Number.isFinite(element.gateSaturationCurrentTemperatureExponent)) {
    throw invalidElement(
      element.name,
      "gate saturation-current temperature exponent must be finite",
    );
  }
  if (!Number.isFinite(element.bandgapVoltage) || element.bandgapVoltage <= 0.0) {
    throw invalidElement(element.name, "bandgap voltage must be finite and positive");
  }
  if (!Number.isFinite(element.dopingTailParameter)) {
    throw invalidElement(element.name, "doping-tail parameter must be finite");
  }
  if (
    !Number.isFinite(element.noiseEquationLevel) ||
    element.noiseEquationLevel < 1.0 ||
    !Number.isInteger(element.noiseEquationLevel)
  ) {
    throw invalidElement(
      element.name,
      "noise equation level must be a finite integer greater than or equal to 1",
    );
  }
  if (
    !Number.isFinite(element.channelNoiseCoefficient) ||
    element.channelNoiseCoefficient < 0.0
  ) {
    throw invalidElement(
      element.name,
      "channel noise coefficient must be finite and non-negative",
    );
  }
  const effectiveThreshold =
    element.polarity === "NJF" ? element.thresholdVoltage : -element.thresholdVoltage;
  if (
    element.dopingTailParameter !== 1.0 &&
    element.junctionPotential === effectiveThreshold
  ) {
    throw invalidElement(
      element.name,
      "junction potential minus effective threshold voltage must be non-zero when doping-tail parameter differs from 1",
    );
  }
  if (!Number.isFinite(element.drainResistance) || element.drainResistance < 0.0) {
    throw invalidElement(
      element.name,
      "drain resistance must be finite and non-negative",
    );
  }
  if (!Number.isFinite(element.sourceResistance) || element.sourceResistance < 0.0) {
    throw invalidElement(
      element.name,
      "source resistance must be finite and non-negative",
    );
  }
  if (!Number.isFinite(element.thresholdVoltageTemperatureCoefficient)) {
    throw invalidElement(
      element.name,
      "threshold-voltage temperature coefficient must be finite",
    );
  }
  if (element.nominalTemperatureKelvin !== undefined &&
      (!Number.isFinite(element.nominalTemperatureKelvin) ||
       element.nominalTemperatureKelvin <= 0.0)) {
    throw invalidElement(element.name, "nominal temperature must be finite and positive");
  }
  if (
    element.alternativeThresholdVoltageTemperatureCoefficient !== undefined &&
    !Number.isFinite(element.alternativeThresholdVoltageTemperatureCoefficient)
  ) {
    throw invalidElement(
      element.name,
      "alternative threshold-voltage temperature coefficient must be finite",
    );
  }
  if (!Number.isFinite(element.mobilityTemperatureExponent)) {
    throw invalidElement(element.name, "mobility temperature exponent must be finite");
  }
  if (
    element.mobilityTemperatureCoefficient !== undefined &&
    !Number.isFinite(element.mobilityTemperatureCoefficient)
  ) {
    throw invalidElement(element.name, "mobility temperature coefficient must be finite");
  }
}

const JFET_THERMAL_VOLTAGE = 0.02585;

function jfetGateJunctionCurrentConductance(
  element: Jfet,
  gateVoltage: number,
): readonly [number, number] {
  const junctionVoltage = element.polarity === "NJF" ? gateVoltage : -gateVoltage;
  const exponent = Math.max(
    -40.0,
    Math.min(40.0, junctionVoltage / JFET_THERMAL_VOLTAGE),
  );
  const expValue = Math.exp(exponent);
  return [
    element.gateSaturationCurrent * (expValue - 1.0),
    element.gateSaturationCurrent / JFET_THERMAL_VOLTAGE * expValue,
  ];
}

function stampJfetGateJunction(
  element: Jfet,
  gate: number | undefined,
  terminal: number | undefined,
  gateVoltage: number,
  matrix: number[][],
  rhs: number[],
): void {
  const [current, conductance] =
    jfetGateJunctionCurrentConductance(element, gateVoltage);
  const junctionVoltage = element.polarity === "NJF" ? gateVoltage : -gateVoltage;
  const equivalentCurrent = current - conductance * junctionVoltage;
  if (element.polarity === "NJF") {
    stampConductance(matrix, gate, terminal, conductance);
    stampCurrentSourceEquivalent(rhs, gate, terminal, equivalentCurrent);
  } else {
    stampConductance(matrix, terminal, gate, conductance);
    stampCurrentSourceEquivalent(rhs, terminal, gate, equivalentCurrent);
  }
}

function stampJfet(
  element: Jfet,
  capacitorStates: readonly CapacitorState[],
  nodeIndices: ReadonlyMap<string, number>,
  matrix: number[][],
  rhs: number[],
  operatingPoint: readonly number[],
): void {
  validateJfet(element);
  const intrinsicDrain = jfetIntrinsicDrainNode(element);
  const intrinsicSource = jfetIntrinsicSourceNode(element);
  const drain = nodeIndex(nodeIndices, intrinsicDrain);
  const gate = nodeIndex(nodeIndices, element.gate);
  const source = nodeIndex(nodeIndices, intrinsicSource);
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
  stampJfetGateJunction(
    element,
    gate,
    source,
    gateVoltage - sourceVoltage,
    matrix,
    rhs,
  );
  stampJfetGateJunction(
    element,
    gate,
    drain,
    gateVoltage - drainVoltage,
    matrix,
    rhs,
  );
  stampJfetCharge(element, capacitorStates, nodeIndices, matrix, rhs);
  if (element.drainResistance > 0.0) {
    stampConductance(
      matrix,
      nodeIndex(nodeIndices, element.drain),
      drain,
      1.0 / element.drainResistance,
    );
  }
  if (element.sourceResistance > 0.0) {
    stampConductance(
      matrix,
      nodeIndex(nodeIndices, element.source),
      source,
      1.0 / element.sourceResistance,
    );
  }
}

function stampJfetCharge(
  element: Jfet,
  capacitorStates: readonly CapacitorState[],
  nodeIndices: ReadonlyMap<string, number>,
  matrix: number[][],
  rhs: number[],
): void {
  for (const spec of jfetChargeStateSpecs(element)) {
    const state = capacitorStates.find((candidate) => candidate.name === spec.name);
    if (state === undefined || spec.capacitance <= 0.0) {
      continue;
    }
    const capacitance = jfetChargeDynamicCapacitance(
      element,
      spec.capacitance,
      state.previousVoltage,
    );
    const conductance =
      state.method === "trap"
        ? (2.0 * capacitance) / state.timeStep
        : state.method === "gear2"
          ? (3.0 * capacitance) / (2.0 * state.timeStep)
          : capacitance / state.timeStep;
    const historyCurrent =
      state.method === "trap"
        ? conductance * state.previousVoltage + state.previousCurrent
        : state.method === "gear2"
          ? (capacitance *
              (4.0 * state.previousVoltage - state.previousPreviousVoltage)) /
            (2.0 * state.timeStep)
          : conductance * state.previousVoltage;
    const positive = nodeIndex(nodeIndices, spec.positive);
    const negative = nodeIndex(nodeIndices, spec.negative);
    stampConductance(matrix, positive, negative, conductance);
    if (positive !== undefined) {
      rhs[positive] += historyCurrent;
    }
    if (negative !== undefined) {
      rhs[negative] -= historyCurrent;
    }
  }
}

function evaluateJfet(element: Jfet, vgs: number, vds: number): JfetDcResult {
  if (element.polarity === "PJF") {
    const result = evaluateNjf(
      -vgs,
      -vds,
      -element.thresholdVoltage,
      element.beta,
      element.channelLengthModulation,
      element.junctionPotential,
      element.dopingTailParameter,
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
    element.junctionPotential,
    element.dopingTailParameter,
  );
}

function evaluateNjf(
  vgs: number,
  vds: number,
  thresholdVoltage: number,
  beta: number,
  channelLengthModulation: number,
  junctionPotential: number,
  dopingTailParameter: number,
): JfetDcResult {
  const overdrive = vgs - thresholdVoltage;
  if (overdrive <= 0.0 || vds < 0.0) {
    return { drainCurrent: 0.0, gm: 0.0, gds: 0.0 };
  }
  const tailFactor =
    dopingTailParameter === 1.0
      ? 0.0
      : (1.0 - dopingTailParameter) / (junctionPotential - thresholdVoltage);
  const modulation = 1.0 + channelLengthModulation * vds;
  if (vds < overdrive) {
    const slope =
      2.0 * dopingTailParameter + 3.0 * tailFactor * (overdrive - vds);
    const channel =
      vds *
      (vds * (tailFactor * vds - dopingTailParameter) + overdrive * slope);
    return {
      drainCurrent: beta * channel * modulation,
      gm: beta * modulation * vds * (slope + 3.0 * tailFactor * overdrive),
      gds:
        beta * modulation * (overdrive - vds) * slope +
        beta * channel * channelLengthModulation,
    };
  }
  const channel =
    overdrive * overdrive * (dopingTailParameter + overdrive * tailFactor);
  return {
    drainCurrent: beta * channel * modulation,
    gm:
      beta *
      modulation *
      overdrive *
      (2.0 * dopingTailParameter + 3.0 * overdrive * tailFactor),
    gds: beta * channel * channelLengthModulation,
  };
}

function stampMosfet(
  element: Mosfet,
  capacitorStates: readonly CapacitorState[],
  nodeIndices: ReadonlyMap<string, number>,
  matrix: number[][],
  rhs: number[],
  operatingPoint: readonly number[],
): void {
  validateMosfet(element);
  const drain = nodeIndex(nodeIndices, mosfetIntrinsicDrainNode(element));
  const gate = nodeIndex(nodeIndices, element.gate);
  const source = nodeIndex(nodeIndices, mosfetIntrinsicSourceNode(element));
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
  stampMosfetBulkJunction(
    element,
    source,
    body,
    sourceVoltage,
    bodyVoltage,
    element.params.AS,
    matrix,
    rhs,
  );
  stampMosfetBulkJunction(
    element,
    drain,
    body,
    drainVoltage,
    bodyVoltage,
    element.params.AD,
    matrix,
    rhs,
  );
  stampMosfetCharge(element, capacitorStates, nodeIndices, matrix, rhs);
  const drainResistance = mosfetDrainResistance(element);
  if (drainResistance > 0.0) {
    stampConductance(
      matrix,
      nodeIndex(nodeIndices, element.drain),
      drain,
      1.0 / drainResistance,
    );
  }
  const sourceResistance = mosfetSourceResistance(element);
  if (sourceResistance > 0.0) {
    stampConductance(
      matrix,
      nodeIndex(nodeIndices, element.source),
      source,
      1.0 / sourceResistance,
    );
  }
}

function mosfetBulkJunctionCurrentConductance(
  element: Mosfet,
  terminalVoltage: number,
  bodyVoltage: number,
  terminalArea: number,
): readonly [number, number] {
  const saturationCurrent =
    element.params.JS > 0.0 && element.params.AD > 0.0 && element.params.AS > 0.0
      ? element.params.JS * terminalArea
      : element.params.IS;
  const junctionVoltage =
    element.type === "NMOS"
      ? bodyVoltage - terminalVoltage
      : terminalVoltage - bodyVoltage;
  const thermalVoltage = BOLTZMANN * element.params.T_NOM / ELECTRON_CHARGE;
  const normalizedVoltage = junctionVoltage / thermalVoltage;
  const limitedExp = Math.exp(Math.max(-40.0, Math.min(40.0, normalizedVoltage)));
  const currentFactor =
    normalizedVoltage > 40.0
      ? limitedExp * (1.0 + normalizedVoltage - 40.0)
      : limitedExp;
  const conductanceFactor = limitedExp;
  return [
    saturationCurrent * (currentFactor - 1.0),
    (saturationCurrent / thermalVoltage) * conductanceFactor,
  ];
}

function stampMosfetBulkJunction(
    element: Mosfet,
    terminal: number | undefined,
    body: number | undefined,
    terminalVoltage: number,
    bodyVoltage: number,
    terminalArea: number,
    matrix: number[][],
    rhs: number[],
): void {
  const [current, conductance] = mosfetBulkJunctionCurrentConductance(
    element,
    terminalVoltage,
    bodyVoltage,
    terminalArea,
  );
  const isNmos = element.type === "NMOS";
  const junctionVoltage = isNmos
    ? bodyVoltage - terminalVoltage
    : terminalVoltage - bodyVoltage;
  const positive = isNmos ? body : terminal;
  const negative = isNmos ? terminal : body;
  stampConductance(matrix, positive, negative, conductance);
  stampCurrentSourceEquivalent(
    rhs,
    positive,
    negative,
    current - conductance * junctionVoltage,
  );
}

function stampMosfetCharge(
  element: Mosfet,
  capacitorStates: readonly CapacitorState[],
  nodeIndices: ReadonlyMap<string, number>,
  matrix: number[][],
  rhs: number[],
): void {
  for (const spec of mosfetChargeStateSpecs(element)) {
    const state = capacitorStates.find((candidate) => candidate.name === spec.name);
    if (state === undefined) {
      continue;
    }
    const capacitance = mosfetChargeDynamicCapacitance(element, spec, state.previousVoltage);
    if (capacitance <= 0.0) {
      continue;
    }
    const conductance =
      state.method === "trap"
        ? (2.0 * capacitance) / state.timeStep
        : state.method === "gear2"
          ? (3.0 * capacitance) / (2.0 * state.timeStep)
          : capacitance / state.timeStep;
    const historyCurrent =
      state.method === "trap"
        ? conductance * state.previousVoltage + state.previousCurrent
        : state.method === "gear2"
          ? (capacitance *
              (4.0 * state.previousVoltage - state.previousPreviousVoltage)) /
            (2.0 * state.timeStep)
          : conductance * state.previousVoltage;
    const positive = nodeIndex(nodeIndices, spec.positive);
    const negative = nodeIndex(nodeIndices, spec.negative);
    stampConductance(matrix, positive, negative, conductance);
    if (positive !== undefined) {
      rhs[positive] += historyCurrent;
    }
    if (negative !== undefined) {
      rhs[negative] -= historyCurrent;
    }
  }
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
  const effectiveLength = params.L - 2.0 * params.LD;
  const beta = params.KP * (params.W / effectiveLength);
  const cgsOverlap = params.CGSO * params.W;
  const cgdOverlap = params.CGDO * params.W;
  const cgbOverlap = params.CGBO * effectiveLength;
  const channelCapacitance =
    params.W * effectiveLength * (OXIDE_PERMITTIVITY / params.TOX);
  const cbsBulk = mosfetBulkJunctionCapacitance(
    params.CBS + params.CJ * params.AS,
    vbs, params.PB, params.MJ, params.FC,
  ) + mosfetBulkJunctionCapacitance(
    params.CJSW * params.PS,
    vbs, params.PB, params.MJSW, params.FC,
  );
  const cbdBulk = mosfetBulkJunctionCapacitance(
    params.CBD + params.CJ * params.AD,
    vbs - vds, params.PB, params.MJ, params.FC,
  ) + mosfetBulkJunctionCapacitance(
    params.CJSW * params.PD,
    vbs - vds, params.PB, params.MJSW, params.FC,
  );
  const capacitances = {
    cgs: cgsOverlap + channelCapacitance,
    cgd: cgdOverlap,
    cgb: cgbOverlap,
    cbs: cbsBulk,
    cbd: cbdBulk,
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
      cgs: cgsOverlap + channelCapacitance / 2.0,
      cgd: cgdOverlap,
      cgb: cgbOverlap,
      cbs: cbsBulk,
      cbd: cbdBulk,
    };
  }
  const current = 0.5 * beta * overdrive * overdrive * (1.0 + params.LAMBDA * vds);
  const gm = beta * overdrive * (1.0 + params.LAMBDA * vds);
  return {
    drainCurrent: current,
    gm,
    gds: 0.5 * beta * overdrive * overdrive * params.LAMBDA,
    gmb: gm * bodyFactor,
    cgs: cgsOverlap + (2.0 / 3.0) * channelCapacitance,
    cgd: cgdOverlap,
    cgb: cgbOverlap,
    cbs: cbsBulk,
    cbd: cbdBulk,
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
  if (
    !Number.isFinite(element.flickerNoiseExponent) ||
    element.flickerNoiseExponent < 0.0
  ) {
    throw invalidElement(
      element.name,
      "flicker-noise exponent must be finite and non-negative",
    );
  }
  if (
    !Number.isFinite(element.flickerNoiseCoefficient) ||
    element.flickerNoiseCoefficient < 0.0
  ) {
    throw invalidElement(
      element.name,
      "flicker-noise coefficient must be finite and non-negative",
    );
  }
  if (!Number.isFinite(element.seriesResistance) || element.seriesResistance < 0.0) {
    throw invalidElement(element.name, "series resistance must be finite and non-negative");
  }
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
  if (!Number.isFinite(element.junctionPotential) || element.junctionPotential <= 0.0) {
    throw invalidElement(element.name, "junction potential must be finite and positive");
  }
  if (!Number.isFinite(element.gradingCoefficient) || element.gradingCoefficient < 0.0) {
    throw invalidElement(element.name, "grading coefficient must be finite and non-negative");
  }
  if (
    !Number.isFinite(element.forwardBiasDepletionCoefficient) ||
    element.forwardBiasDepletionCoefficient < 0.0 ||
    element.forwardBiasDepletionCoefficient >= 1.0
  ) {
    throw invalidElement(
      element.name,
      "forward-bias depletion coefficient must be finite and in [0, 1)",
    );
  }
  if (!Number.isFinite(element.saturationCurrentTemperatureExponent)) {
    throw invalidElement(
      element.name,
      "saturation-current temperature exponent must be finite",
    );
  }
  if (!Number.isFinite(element.energyGapElectronVolts) || element.energyGapElectronVolts <= 0.0) {
    throw invalidElement(element.name, "energy gap must be finite and positive");
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

function diodeChargeStateName(element: Diode): string {
  return `_D_${element.name}_charge`;
}

function diodeIntrinsicAnodeNode(element: Diode): string {
  return element.seriesResistance === 0.0
    ? element.anode
    : `_D_${element.name}_anode`;
}

function diodeHasChargeStorage(element: Diode): boolean {
  return element.junctionCapacitance > 0.0 || element.transitTime > 0.0;
}

function diodeDynamicCapacitance(element: Diode, voltage: number): number {
  const [, conductance] = diodeCurrentConductance(element, voltage);
  return diodeDepletionCapacitance(element, voltage) + element.transitTime * conductance;
}

function diodeDepletionCapacitance(element: Diode, voltage: number): number {
  if (element.junctionCapacitance <= 0.0 || element.gradingCoefficient === 0.0) {
    return element.junctionCapacitance;
  }
  const normalizedVoltage = voltage / element.junctionPotential;
  if (normalizedVoltage < element.forwardBiasDepletionCoefficient) {
    return element.junctionCapacitance /
      ((1.0 - normalizedVoltage) ** element.gradingCoefficient);
  }
  const coefficient = element.forwardBiasDepletionCoefficient;
  const transitionScale = (1.0 - coefficient) ** (1.0 + element.gradingCoefficient);
  const continuation = 1.0 - coefficient * (1.0 + element.gradingCoefficient) +
    element.gradingCoefficient * normalizedVoltage;
  return element.junctionCapacitance * continuation / transitionScale;
}

function diodeChargeVoltage(element: Diode, nodeVoltages: ReadonlyMap<string, number>): number {
  return voltageAt(nodeVoltages, diodeIntrinsicAnodeNode(element)) -
    voltageAt(nodeVoltages, element.cathode);
}

type BjtChargeStateKind =
  | "base-emitter"
  | "base-collector"
  | "external-base-collector";

interface BjtChargeStateSpec {
  readonly name: string;
  readonly positive: string;
  readonly negative: string;
  readonly kind: BjtChargeStateKind;
}

function bjtBaseEmitterChargeStateName(element: Bjt): string {
  return `_Q_${element.name}_be_charge`;
}

function bjtBaseCollectorChargeStateName(element: Bjt): string {
  return `_Q_${element.name}_bc_charge`;
}

function bjtExternalBaseCollectorChargeStateName(element: Bjt): string {
  return `_Q_${element.name}_bx_charge`;
}

function bjtIntrinsicEmitterNode(element: Bjt): string {
  return element.emitterResistance === 0.0
    ? element.emitter
    : `__spice_${element.name}_emitter`;
}

function bjtIntrinsicCollectorNode(element: Bjt): string {
  return element.collectorResistance === 0.0
    ? element.collector
    : `__spice_${element.name}_collector`;
}

function bjtIntrinsicBaseNode(element: Bjt): string {
  return element.baseResistance === 0.0
    ? element.base
    : `__spice_${element.name}_base`;
}

function bjtJunctionTransconductance(
  element: Bjt,
  voltage: number,
  emissionCoefficient: number,
): number {
  const effectiveThermalVoltage = element.thermalVoltage * emissionCoefficient;
  const exponent = Math.max(-40.0, Math.min(40.0, voltage / effectiveThermalVoltage));
  return (element.saturationCurrent / effectiveThermalVoltage) * Math.exp(exponent);
}

function bjtForwardTransitTimeScale(
  element: Bjt,
  voltage: number,
  reverseJunctionVoltage: number,
): number {
  const effectiveThermalVoltage =
    element.thermalVoltage * element.forwardEmissionCoefficient;
  const exponent = Math.max(-40.0, Math.min(40.0, voltage / effectiveThermalVoltage));
  const forwardCurrent = Math.max(
    element.saturationCurrent * (Math.exp(exponent) - 1.0),
    0.0,
  );
  let currentFactor = 1.0;
  if (element.forwardTransitTimeCurrent > 0.0) {
    const ratio = forwardCurrent / (forwardCurrent + element.forwardTransitTimeCurrent);
    currentFactor = ratio * ratio;
  }
  const voltageFactor = element.forwardTransitTimeVoltage === 0.0
    ? 1.0
    : Math.exp(Math.max(
        -40.0,
        Math.min(40.0, reverseJunctionVoltage / (1.44 * element.forwardTransitTimeVoltage)),
      ));
  return 1.0 +
    element.forwardTransitTimeBiasCoefficient * currentFactor * voltageFactor;
}

function bjtChargeDynamicCapacitance(
  element: Bjt,
  kind: BjtChargeStateKind,
  voltage: number,
  reverseJunctionVoltage: number,
): number {
  if (kind === "base-emitter") {
    const conductance = bjtJunctionTransconductance(
      element,
      voltage,
      element.forwardEmissionCoefficient,
    );
    return bjtBaseEmitterDepletionCapacitance(element, voltage) +
      element.forwardTransitTime *
        bjtForwardTransitTimeScale(element, voltage, reverseJunctionVoltage) *
        conductance;
  }
  const depletionCapacitance = bjtBaseCollectorDepletionCapacitance(element, voltage);
  if (kind === "external-base-collector") {
    return (1.0 - element.baseCollectorCapacitanceFraction) * depletionCapacitance;
  }
  const conductance = bjtJunctionTransconductance(
    element,
    voltage,
    element.reverseEmissionCoefficient,
  );
  return element.baseCollectorCapacitanceFraction * depletionCapacitance +
    element.reverseTransitTime * conductance;
}

function bjtBaseEmitterDepletionCapacitance(element: Bjt, voltage: number): number {
  if (element.baseEmitterCapacitance <= 0.0 || element.baseEmitterGradingCoefficient === 0.0) {
    return element.baseEmitterCapacitance;
  }
  const normalizedVoltage = voltage / element.baseEmitterJunctionPotential;
  const coefficient = element.forwardBiasDepletionCoefficient;
  if (normalizedVoltage < coefficient) {
    return element.baseEmitterCapacitance /
      ((1.0 - normalizedVoltage) ** element.baseEmitterGradingCoefficient);
  }
  const transitionScale = (1.0 - coefficient) **
    (1.0 + element.baseEmitterGradingCoefficient);
  const continuation = 1.0 - coefficient * (1.0 + element.baseEmitterGradingCoefficient) +
    element.baseEmitterGradingCoefficient * normalizedVoltage;
  return element.baseEmitterCapacitance * continuation / transitionScale;
}

function bjtBaseCollectorDepletionCapacitance(element: Bjt, voltage: number): number {
  if (element.baseCollectorCapacitance <= 0.0 || element.baseCollectorGradingCoefficient === 0.0) {
    return element.baseCollectorCapacitance;
  }
  const normalizedVoltage = voltage / element.baseCollectorJunctionPotential;
  const coefficient = element.forwardBiasDepletionCoefficient;
  if (normalizedVoltage < coefficient) {
    return element.baseCollectorCapacitance /
      ((1.0 - normalizedVoltage) ** element.baseCollectorGradingCoefficient);
  }
  const transitionScale = (1.0 - coefficient) **
    (1.0 + element.baseCollectorGradingCoefficient);
  const continuation = 1.0 - coefficient * (1.0 + element.baseCollectorGradingCoefficient) +
    element.baseCollectorGradingCoefficient * normalizedVoltage;
  return element.baseCollectorCapacitance * continuation / transitionScale;
}

function bjtChargeStateSpecs(element: Bjt): BjtChargeStateSpec[] {
  const specs: BjtChargeStateSpec[] = [];
  const emitter = bjtIntrinsicEmitterNode(element);
  const collector = bjtIntrinsicCollectorNode(element);
  const base = bjtIntrinsicBaseNode(element);
  if (element.baseEmitterCapacitance > 0.0 || element.forwardTransitTime > 0.0) {
    const [positive, negative] =
      element.polarity === "NPN"
        ? [base, emitter]
        : [emitter, base];
    specs.push({
      name: bjtBaseEmitterChargeStateName(element),
      positive,
      negative,
      kind: "base-emitter",
    });
  }
  if (
    element.baseCollectorCapacitance > 0.0 ||
    element.reverseTransitTime > 0.0 ||
    (
      element.forwardTransitTime > 0.0 &&
      element.forwardTransitTimeBiasCoefficient > 0.0 &&
      element.forwardTransitTimeVoltage > 0.0
    )
  ) {
    const [positive, negative] =
      element.polarity === "NPN"
        ? [base, collector]
        : [collector, base];
    specs.push({
      name: bjtBaseCollectorChargeStateName(element),
      positive,
      negative,
      kind: "base-collector",
    });
  }
  if (
    element.baseCollectorCapacitance > 0.0 &&
    element.baseCollectorCapacitanceFraction < 1.0
  ) {
    const [positive, negative] =
      element.polarity === "NPN"
        ? [element.base, collector]
        : [collector, element.base];
    specs.push({
      name: bjtExternalBaseCollectorChargeStateName(element),
      positive,
      negative,
      kind: "external-base-collector",
    });
  }
  return specs;
}

function bjtChargeStateVoltage(
  spec: BjtChargeStateSpec,
  nodeVoltages: ReadonlyMap<string, number>,
): number {
  return voltageAt(nodeVoltages, spec.positive) - voltageAt(nodeVoltages, spec.negative);
}

interface JfetChargeStateSpec {
  readonly name: string;
  readonly positive: string;
  readonly negative: string;
  readonly capacitance: number;
}

function jfetGateSourceChargeStateName(element: Jfet): string {
  return `_J_${element.name}_gs_charge`;
}

function jfetGateDrainChargeStateName(element: Jfet): string {
  return `_J_${element.name}_gd_charge`;
}

function jfetChargeStateSpecs(element: Jfet): JfetChargeStateSpec[] {
  const specs: JfetChargeStateSpec[] = [];
  if (element.gateSourceCapacitance > 0.0) {
    specs.push({
      name: jfetGateSourceChargeStateName(element),
      positive: element.gate,
      negative: jfetIntrinsicSourceNode(element),
      capacitance: element.gateSourceCapacitance,
    });
  }
  if (element.gateDrainCapacitance > 0.0) {
    specs.push({
      name: jfetGateDrainChargeStateName(element),
      positive: element.gate,
      negative: jfetIntrinsicDrainNode(element),
      capacitance: element.gateDrainCapacitance,
    });
  }
  return specs;
}

function jfetIntrinsicDrainNode(element: Jfet): string {
  return element.drainResistance === 0.0
    ? element.drain
    : `__spice_${element.name}_drain`;
}

function jfetIntrinsicSourceNode(element: Jfet): string {
  return element.sourceResistance === 0.0
    ? element.source
    : `__spice_${element.name}_source`;
}

function mosfetIntrinsicDrainNode(element: Mosfet): string {
  const drainResistance = mosfetDrainResistance(element);
  return !Number.isFinite(drainResistance) || drainResistance <= 0.0
    ? element.drain
    : `__spice_${element.name}_drain`;
}

function mosfetIntrinsicSourceNode(element: Mosfet): string {
  const sourceResistance = mosfetSourceResistance(element);
  return !Number.isFinite(sourceResistance) || sourceResistance <= 0.0
    ? element.source
    : `__spice_${element.name}_source`;
}

function mosfetDrainResistance(element: Mosfet): number {
  return element.params.RD > 0.0
    ? element.params.RD
    : element.params.RSH * element.params.NRD;
}

function mosfetSourceResistance(element: Mosfet): number {
  return element.params.RS > 0.0
    ? element.params.RS
    : element.params.RSH * element.params.NRS;
}

function jfetChargeStateVoltage(
  spec: JfetChargeStateSpec,
  nodeVoltages: ReadonlyMap<string, number>,
): number {
  return voltageAt(nodeVoltages, spec.positive) - voltageAt(nodeVoltages, spec.negative);
}

function jfetChargeDynamicCapacitance(
  element: Jfet,
  zeroBiasCapacitance: number,
  junctionVoltage: number,
): number {
  const gradingCoefficient = 0.5;
  const orientedVoltage = element.polarity === "PJF" ? -junctionVoltage : junctionVoltage;
  const normalizedVoltage = orientedVoltage / element.junctionPotential;
  if (normalizedVoltage < element.forwardBiasDepletionCoefficient) {
    return zeroBiasCapacitance / ((1.0 - normalizedVoltage) ** gradingCoefficient);
  }
  const transitionScale =
    (1.0 - element.forwardBiasDepletionCoefficient) ** (1.0 + gradingCoefficient);
  const continuation =
    1.0 -
    element.forwardBiasDepletionCoefficient * (1.0 + gradingCoefficient) +
    gradingCoefficient * normalizedVoltage;
  return (zeroBiasCapacitance * continuation) / transitionScale;
}

interface MosfetChargeStateSpec {
  readonly name: string;
  readonly positive: string;
  readonly negative: string;
  readonly capacitance: number;
  readonly kind: "gate-overlap" | "source-body" | "drain-body";
}

function mosfetGateSourceChargeStateName(element: Mosfet): string {
  return `_M_${element.name}_gs_charge`;
}

function mosfetGateDrainChargeStateName(element: Mosfet): string {
  return `_M_${element.name}_gd_charge`;
}

function mosfetGateBodyChargeStateName(element: Mosfet): string {
  return `_M_${element.name}_gb_charge`;
}

function mosfetSourceBodyChargeStateName(element: Mosfet): string {
  return `_M_${element.name}_sb_charge`;
}

function mosfetDrainBodyChargeStateName(element: Mosfet): string {
  return `_M_${element.name}_db_charge`;
}

function mosfetChargeStateSpecs(element: Mosfet): MosfetChargeStateSpec[] {
  const specs: MosfetChargeStateSpec[] = [];
  const gateSourceCapacitance = element.params.CGSO * element.params.W;
  const gateDrainCapacitance = element.params.CGDO * element.params.W;
  const gateBodyCapacitance = element.params.CGBO * element.params.L;
  const sourceBodyCapacitance =
    element.params.CBS
    + element.params.CJ * element.params.AS
    + element.params.CJSW * element.params.PS;
  const drainBodyCapacitance =
    element.params.CBD
    + element.params.CJ * element.params.AD
    + element.params.CJSW * element.params.PD;
  if (gateSourceCapacitance > 0.0) {
    specs.push({
      name: mosfetGateSourceChargeStateName(element),
      positive: element.gate,
      negative: mosfetIntrinsicSourceNode(element),
      capacitance: gateSourceCapacitance,
      kind: "gate-overlap",
    });
  }
  if (gateDrainCapacitance > 0.0) {
    specs.push({
      name: mosfetGateDrainChargeStateName(element),
      positive: element.gate,
      negative: mosfetIntrinsicDrainNode(element),
      capacitance: gateDrainCapacitance,
      kind: "gate-overlap",
    });
  }
  if (gateBodyCapacitance > 0.0) {
    specs.push({
      name: mosfetGateBodyChargeStateName(element),
      positive: element.gate,
      negative: element.body,
      capacitance: gateBodyCapacitance,
      kind: "gate-overlap",
    });
  }
  if (sourceBodyCapacitance > 0.0) {
    specs.push({
      name: mosfetSourceBodyChargeStateName(element),
      positive: mosfetIntrinsicSourceNode(element),
      negative: element.body,
      capacitance: sourceBodyCapacitance,
      kind: "source-body",
    });
  }
  if (drainBodyCapacitance > 0.0) {
    specs.push({
      name: mosfetDrainBodyChargeStateName(element),
      positive: mosfetIntrinsicDrainNode(element),
      negative: element.body,
      capacitance: drainBodyCapacitance,
      kind: "drain-body",
    });
  }
  return specs;
}

function mosfetChargeStateVoltage(
  spec: MosfetChargeStateSpec,
  nodeVoltages: ReadonlyMap<string, number>,
): number {
  return voltageAt(nodeVoltages, spec.positive) - voltageAt(nodeVoltages, spec.negative);
}

function mosfetChargeDynamicCapacitance(
  element: Mosfet,
  spec: MosfetChargeStateSpec,
  stateVoltage: number,
): number {
  if (spec.kind !== "source-body" && spec.kind !== "drain-body") {
    return spec.capacitance;
  }
  const junctionVoltage = element.type === "PMOS" ? stateVoltage : -stateVoltage;
  const bottomCapacitance = spec.kind === "source-body"
    ? element.params.CBS + element.params.CJ * element.params.AS
    : element.params.CBD + element.params.CJ * element.params.AD;
  const sidewallCapacitance = spec.kind === "source-body"
    ? element.params.CJSW * element.params.PS
    : element.params.CJSW * element.params.PD;
  return mosfetBulkJunctionCapacitance(
    bottomCapacitance,
    junctionVoltage,
    element.params.PB,
    element.params.MJ,
    element.params.FC,
  ) + mosfetBulkJunctionCapacitance(
    sidewallCapacitance,
    junctionVoltage,
    element.params.PB,
    element.params.MJSW,
    element.params.FC,
  );
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
  if (Number.isNaN(element.reverseBeta) || element.reverseBeta <= 0.0) {
    throw invalidElement(element.name, "reverse beta must be positive");
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
  if (!Number.isFinite(element.saturationCurrentTemperatureExponent)) {
    throw invalidElement(element.name, "saturation-current temperature exponent must be finite");
  }
  if (!Number.isFinite(element.forwardBetaTemperatureExponent)) {
    throw invalidElement(element.name, "forward-beta temperature exponent must be finite");
  }
  if (!Number.isFinite(element.energyGapElectronVolts) || element.energyGapElectronVolts <= 0.0) {
    throw invalidElement(element.name, "energy gap must be finite and positive");
  }
  if (!Number.isFinite(element.forwardEarlyVoltage) || element.forwardEarlyVoltage < 0.0) {
    throw invalidElement(element.name, "forward Early voltage must be finite and non-negative");
  }
  if (!Number.isFinite(element.reverseEarlyVoltage) || element.reverseEarlyVoltage < 0.0) {
    throw invalidElement(element.name, "reverse Early voltage must be finite and non-negative");
  }
  if (!Number.isFinite(element.forwardBetaRolloffCurrent) || element.forwardBetaRolloffCurrent < 0.0) {
    throw invalidElement(element.name, "forward beta roll-off current must be finite and non-negative");
  }
  if (!Number.isFinite(element.reverseBetaRolloffCurrent) || element.reverseBetaRolloffCurrent < 0.0) {
    throw invalidElement(element.name, "reverse beta roll-off current must be finite and non-negative");
  }
  if (element.nominalTemperatureKelvin !== undefined &&
      (!Number.isFinite(element.nominalTemperatureKelvin) || element.nominalTemperatureKelvin <= 0.0)) {
    throw invalidElement(element.name, "nominal temperature must be finite and positive");
  }
  if (!Number.isFinite(element.flickerNoiseCoefficient) || element.flickerNoiseCoefficient < 0.0) {
    throw invalidElement(element.name, "flicker noise coefficient must be finite and non-negative");
  }
  if (!Number.isFinite(element.flickerNoiseExponent) || element.flickerNoiseExponent < 0.0) {
    throw invalidElement(element.name, "flicker noise exponent must be finite and non-negative");
  }
  if (!Number.isFinite(element.forwardExcessPhaseDegrees) || element.forwardExcessPhaseDegrees < 0.0) {
    throw invalidElement(element.name, "forward excess phase must be finite and non-negative");
  }
  if (!Number.isFinite(element.forwardTransitTimeBiasCoefficient) || element.forwardTransitTimeBiasCoefficient < 0.0) {
    throw invalidElement(element.name, "forward transit-time bias coefficient must be finite and non-negative");
  }
  if (!Number.isFinite(element.forwardTransitTimeCurrent) || element.forwardTransitTimeCurrent < 0.0) {
    throw invalidElement(element.name, "forward transit-time current must be finite and non-negative");
  }
  if (!Number.isFinite(element.forwardTransitTimeVoltage) || element.forwardTransitTimeVoltage < 0.0) {
    throw invalidElement(element.name, "forward transit-time voltage must be finite and non-negative");
  }
  if (!Number.isFinite(element.emitterResistance) || element.emitterResistance < 0.0) {
    throw invalidElement(element.name, "emitter resistance must be finite and non-negative");
  }
  if (!Number.isFinite(element.collectorResistance) || element.collectorResistance < 0.0) {
    throw invalidElement(element.name, "collector resistance must be finite and non-negative");
  }
  if (!Number.isFinite(element.baseResistance) || element.baseResistance < 0.0) {
    throw invalidElement(element.name, "base resistance must be finite and non-negative");
  }
  if (element.minimumBaseResistance !== undefined &&
      (!Number.isFinite(element.minimumBaseResistance) || element.minimumBaseResistance < 0.0)) {
    throw invalidElement(
      element.name,
      "minimum base resistance must be finite and non-negative",
    );
  }
  if (!Number.isFinite(element.baseResistanceHalfCurrent) ||
      element.baseResistanceHalfCurrent < 0.0) {
    throw invalidElement(
      element.name,
      "base-resistance half-current must be finite and non-negative",
    );
  }
  if (!Number.isFinite(element.baseCollectorCapacitanceFraction) ||
      element.baseCollectorCapacitanceFraction < 0.0 ||
      element.baseCollectorCapacitanceFraction > 1.0) {
    throw invalidElement(
      element.name,
      "base-collector capacitance fraction must be between zero and one",
    );
  }
  if (!Number.isFinite(element.baseEmitterLeakageSaturationCurrent) || element.baseEmitterLeakageSaturationCurrent < 0.0) {
    throw invalidElement(element.name, "base-emitter leakage saturation current must be finite and non-negative");
  }
  if (!Number.isFinite(element.baseEmitterLeakageEmissionCoefficient) || element.baseEmitterLeakageEmissionCoefficient <= 0.0) {
    throw invalidElement(element.name, "base-emitter leakage emission coefficient must be finite and positive");
  }
  if (!Number.isFinite(element.baseCollectorLeakageSaturationCurrent) || element.baseCollectorLeakageSaturationCurrent < 0.0) {
    throw invalidElement(element.name, "base-collector leakage saturation current must be finite and non-negative");
  }
  if (!Number.isFinite(element.baseCollectorLeakageEmissionCoefficient) || element.baseCollectorLeakageEmissionCoefficient <= 0.0) {
    throw invalidElement(element.name, "base-collector leakage emission coefficient must be finite and positive");
  }
  if (!Number.isFinite(element.forwardEmissionCoefficient) || element.forwardEmissionCoefficient <= 0.0) {
    throw invalidElement(element.name, "forward emission coefficient must be finite and positive");
  }
  if (!Number.isFinite(element.reverseEmissionCoefficient) || element.reverseEmissionCoefficient <= 0.0) {
    throw invalidElement(element.name, "reverse emission coefficient must be finite and positive");
  }
  if (!Number.isFinite(element.baseEmitterJunctionPotential) || element.baseEmitterJunctionPotential <= 0.0) {
    throw invalidElement(element.name, "base-emitter junction potential must be finite and positive");
  }
  if (!Number.isFinite(element.baseEmitterGradingCoefficient) ||
      element.baseEmitterGradingCoefficient < 0.0 ||
      element.baseEmitterGradingCoefficient >= 1.0) {
    throw invalidElement(element.name, "base-emitter grading coefficient must be finite and in [0, 1)");
  }
  if (!Number.isFinite(element.baseCollectorJunctionPotential) || element.baseCollectorJunctionPotential <= 0.0) {
    throw invalidElement(element.name, "base-collector junction potential must be finite and positive");
  }
  if (!Number.isFinite(element.baseCollectorGradingCoefficient) ||
      element.baseCollectorGradingCoefficient < 0.0 ||
      element.baseCollectorGradingCoefficient >= 1.0) {
    throw invalidElement(element.name, "base-collector grading coefficient must be finite and in [0, 1)");
  }
  if (!Number.isFinite(element.forwardBiasDepletionCoefficient) ||
      element.forwardBiasDepletionCoefficient < 0.0 ||
      element.forwardBiasDepletionCoefficient >= 1.0) {
    throw invalidElement(element.name, "forward-bias depletion coefficient must be finite and in [0, 1)");
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
  if (params.LD < 0.0 || params.L - 2.0 * params.LD <= 0.0) {
    throw invalidElement(element.name, "MOSFET LD must be non-negative with L - 2*LD > 0");
  }
  if (params.RD < 0.0) {
    throw invalidElement(element.name, "MOSFET RD must be non-negative");
  }
  if (params.RS < 0.0) {
    throw invalidElement(element.name, "MOSFET RS must be non-negative");
  }
  if (params.RSH < 0.0) {
    throw invalidElement(element.name, "MOSFET RSH must be non-negative");
  }
  if (params.NRD < 0.0) {
    throw invalidElement(element.name, "MOSFET NRD must be non-negative");
  }
  if (params.NRS < 0.0) {
    throw invalidElement(element.name, "MOSFET NRS must be non-negative");
  }
  if (params.AD < 0.0) {
    throw invalidElement(element.name, "MOSFET AD must be non-negative");
  }
  if (params.AS < 0.0) {
    throw invalidElement(element.name, "MOSFET AS must be non-negative");
  }
  if (params.CJ < 0.0) {
    throw invalidElement(element.name, "MOSFET CJ must be non-negative");
  }
  if (params.PD < 0.0) {
    throw invalidElement(element.name, "MOSFET PD must be non-negative");
  }
  if (params.PS < 0.0) {
    throw invalidElement(element.name, "MOSFET PS must be non-negative");
  }
  if (params.CJSW < 0.0) {
    throw invalidElement(element.name, "MOSFET CJSW must be non-negative");
  }
  if (params.TOX <= 0.0) {
    throw invalidElement(element.name, "MOSFET TOX must be positive");
  }
  if (params.U0 < 0.0) {
    throw invalidElement(element.name, "MOSFET U0 must be non-negative");
  }
  if (params.PHI <= 0.0) {
    throw invalidElement(element.name, "MOSFET PHI must be positive");
  }
  if (params.IS <= 0.0 || params.N_SUB <= 0.0 || params.T_NOM <= 0.0) {
    throw invalidElement(element.name, "MOSFET IS, N_SUB, and T_NOM must be positive");
  }
  if (params.JS < 0.0) {
    throw invalidElement(element.name, "MOSFET JS must be non-negative");
  }
  if (params.PB <= 0.0 || params.MJ < 0.0) {
    throw invalidElement(element.name, "MOSFET PB must be positive and MJ must be non-negative");
  }
  if (params.MJSW < 0.0) {
    throw invalidElement(element.name, "MOSFET MJSW must be non-negative");
  }
  if (params.FC < 0.0 || params.FC >= 1.0) {
    throw invalidElement(element.name, "MOSFET FC must be in [0, 1)");
  }
  if (params.KF < 0.0) {
    throw invalidElement(element.name, "MOSFET KF must be non-negative");
  }
  if (params.AF < 0.0) {
    throw invalidElement(element.name, "MOSFET AF must be non-negative");
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
    } else if (element.kind === "diode" && diodeHasChargeStorage(element)) {
      states.push({
        name: diodeChargeStateName(element),
        previousVoltage: 0.0,
        previousPreviousVoltage: 0.0,
        previousCurrent: 0.0,
        timeStep,
        method,
      });
    } else if (element.kind === "bjt") {
      for (const spec of bjtChargeStateSpecs(element)) {
        states.push({
          name: spec.name,
          previousVoltage: 0.0,
          previousPreviousVoltage: 0.0,
          previousCurrent: 0.0,
          timeStep,
          method,
        });
      }
    } else if (element.kind === "jfet") {
      for (const spec of jfetChargeStateSpecs(element)) {
        states.push({
          name: spec.name,
          previousVoltage: 0.0,
          previousPreviousVoltage: 0.0,
          previousCurrent: 0.0,
          timeStep,
          method,
        });
      }
    } else if (element.kind === "mosfet") {
      for (const spec of mosfetChargeStateSpecs(element)) {
        states.push({
          name: spec.name,
          previousVoltage: 0.0,
          previousPreviousVoltage: 0.0,
          previousCurrent: 0.0,
          timeStep,
          method,
        });
      }
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
    } else if (element.kind === "diode" && diodeHasChargeStorage(element)) {
      voltages.set(diodeChargeStateName(element), diodeChargeVoltage(element, nodeVoltages));
    } else if (element.kind === "bjt") {
      for (const spec of bjtChargeStateSpecs(element)) {
        voltages.set(spec.name, bjtChargeStateVoltage(spec, nodeVoltages));
      }
    } else if (element.kind === "jfet") {
      for (const spec of jfetChargeStateSpecs(element)) {
        voltages.set(spec.name, jfetChargeStateVoltage(spec, nodeVoltages));
      }
    } else if (element.kind === "mosfet") {
      for (const spec of mosfetChargeStateSpecs(element)) {
        voltages.set(spec.name, mosfetChargeStateVoltage(spec, nodeVoltages));
      }
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
      if (element.kind === "diode" && diodeHasChargeStorage(element)) {
        const stateName = diodeChargeStateName(element);
        const current = currentVoltages.get(stateName) ?? 0.0;
        const previous = previousVoltages.get(stateName) ?? 0.0;
        const previousPrevious = previousPreviousVoltages.get(stateName) ?? 0.0;
        maxLte = Math.max(maxLte, Math.abs(current - 2.0 * previous + previousPrevious) / 2.0);
      } else if (element.kind === "bjt") {
        for (const spec of bjtChargeStateSpecs(element)) {
          const current = currentVoltages.get(spec.name) ?? 0.0;
          const previous = previousVoltages.get(spec.name) ?? 0.0;
          const previousPrevious = previousPreviousVoltages.get(spec.name) ?? 0.0;
          maxLte = Math.max(maxLte, Math.abs(current - 2.0 * previous + previousPrevious) / 2.0);
        }
      } else if (element.kind === "jfet") {
        for (const spec of jfetChargeStateSpecs(element)) {
          const current = currentVoltages.get(spec.name) ?? 0.0;
          const previous = previousVoltages.get(spec.name) ?? 0.0;
          const previousPrevious = previousPreviousVoltages.get(spec.name) ?? 0.0;
          maxLte = Math.max(maxLte, Math.abs(current - 2.0 * previous + previousPrevious) / 2.0);
        }
      } else if (element.kind === "mosfet") {
        for (const spec of mosfetChargeStateSpecs(element)) {
          const current = currentVoltages.get(spec.name) ?? 0.0;
          const previous = previousVoltages.get(spec.name) ?? 0.0;
          const previousPrevious = previousPreviousVoltages.get(spec.name) ?? 0.0;
          maxLte = Math.max(maxLte, Math.abs(current - 2.0 * previous + previousPrevious) / 2.0);
        }
      }
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
  const previousVoltages = new Map(
    capacitorStates.map((state) => [state.name, state.previousVoltage]),
  );
  for (const state of capacitorStates) {
    const capacitorElement = circuit
      .elements()
      .find(
        (candidate): candidate is Capacitor =>
          candidate.kind === "capacitor" && candidate.name === state.name,
      );
    const diodeElement =
      capacitorElement === undefined
        ? circuit
            .elements()
            .find(
              (candidate): candidate is Diode =>
                candidate.kind === "diode" && diodeChargeStateName(candidate) === state.name,
            )
        : undefined;
    const bjtElement =
      capacitorElement === undefined && diodeElement === undefined
        ? circuit
            .elements()
            .find(
              (candidate): candidate is Bjt =>
                candidate.kind === "bjt" &&
                bjtChargeStateSpecs(candidate).some((spec) => spec.name === state.name),
            )
        : undefined;
    const bjtSpec =
      bjtElement === undefined
        ? undefined
        : bjtChargeStateSpecs(bjtElement).find((spec) => spec.name === state.name);
    const jfetElement =
      capacitorElement === undefined && diodeElement === undefined && bjtSpec === undefined
        ? circuit
            .elements()
            .find(
              (candidate): candidate is Jfet =>
                candidate.kind === "jfet" &&
                jfetChargeStateSpecs(candidate).some((spec) => spec.name === state.name),
            )
        : undefined;
    const jfetSpec =
      jfetElement === undefined
        ? undefined
        : jfetChargeStateSpecs(jfetElement).find((spec) => spec.name === state.name);
    const mosfetElement =
      capacitorElement === undefined &&
      diodeElement === undefined &&
      bjtSpec === undefined &&
      jfetSpec === undefined
        ? circuit
            .elements()
            .find(
              (candidate): candidate is Mosfet =>
                candidate.kind === "mosfet" &&
                mosfetChargeStateSpecs(candidate).some((spec) => spec.name === state.name),
            )
        : undefined;
    const mosfetSpec =
      mosfetElement === undefined
        ? undefined
        : mosfetChargeStateSpecs(mosfetElement).find((spec) => spec.name === state.name);
    if (
      capacitorElement === undefined &&
      diodeElement === undefined &&
      bjtSpec === undefined &&
      jfetSpec === undefined &&
      mosfetSpec === undefined
    ) {
      continue;
    }
    const previousVoltage = state.previousVoltage;
    const previousCurrent = state.previousCurrent;
    const voltage =
      capacitorElement !== undefined
        ? voltageAt(nodeVoltages, capacitorElement.n1) - voltageAt(nodeVoltages, capacitorElement.n2)
        : diodeElement !== undefined
          ? diodeChargeVoltage(diodeElement, nodeVoltages)
          : bjtSpec !== undefined
            ? bjtChargeStateVoltage(bjtSpec, nodeVoltages)
            : jfetSpec !== undefined
              ? jfetChargeStateVoltage(jfetSpec, nodeVoltages)
              : mosfetChargeStateVoltage(mosfetSpec!, nodeVoltages);
    const capacitance =
      capacitorElement !== undefined
        ? capacitorElement.capacitanceFarads
        : diodeElement !== undefined
          ? diodeDynamicCapacitance(diodeElement, previousVoltage)
          : bjtSpec !== undefined
            ? bjtChargeDynamicCapacitance(
                bjtElement!,
                bjtSpec.kind,
                previousVoltage,
                previousVoltages.get(bjtBaseCollectorChargeStateName(bjtElement!)) ?? 0.0,
              )
            : jfetSpec !== undefined
              ? jfetChargeDynamicCapacitance(
                  jfetElement!,
                  jfetSpec.capacitance,
                  state.previousVoltage,
                )
              : mosfetChargeDynamicCapacitance(
                  mosfetElement!,
                  mosfetSpec!,
                  state.previousVoltage,
                );
    if (state.method === "trap") {
      const conductance = (2.0 * capacitance) / state.timeStep;
      state.previousCurrent = conductance * (voltage - previousVoltage) - previousCurrent;
    } else if (state.method === "gear2") {
      state.previousCurrent =
        capacitance *
        (3.0 * voltage - 4.0 * previousVoltage + state.previousPreviousVoltage) /
        (2.0 * state.timeStep);
    } else {
      state.previousCurrent =
        (capacitance / state.timeStep) * (voltage - previousVoltage);
    }
    state.previousVoltage = voltage;
    state.previousPreviousVoltage = previousVoltage;
  }
}

function seedDeviceCapacitorStates(
  circuit: Circuit,
  nodeVoltages: ReadonlyMap<string, number>,
  capacitorStates: CapacitorState[],
): void {
  for (const element of circuit.elements()) {
    if (element.kind === "diode") {
      const state = capacitorStates.find(
        (candidate) => candidate.name === diodeChargeStateName(element),
      );
      if (state === undefined) {
        continue;
      }
      const voltage = diodeChargeVoltage(element, nodeVoltages);
      state.previousVoltage = voltage;
      state.previousPreviousVoltage = voltage;
      state.previousCurrent = 0.0;
    } else if (element.kind === "bjt") {
      for (const spec of bjtChargeStateSpecs(element)) {
        const state = capacitorStates.find((candidate) => candidate.name === spec.name);
        if (state === undefined) {
          continue;
        }
        const voltage = bjtChargeStateVoltage(spec, nodeVoltages);
        state.previousVoltage = voltage;
        state.previousPreviousVoltage = voltage;
        state.previousCurrent = 0.0;
      }
    } else if (element.kind === "jfet") {
      for (const spec of jfetChargeStateSpecs(element)) {
        const state = capacitorStates.find((candidate) => candidate.name === spec.name);
        if (state === undefined) {
          continue;
        }
        const voltage = jfetChargeStateVoltage(spec, nodeVoltages);
        state.previousVoltage = voltage;
        state.previousPreviousVoltage = voltage;
        state.previousCurrent = 0.0;
      }
    } else if (element.kind === "mosfet") {
      for (const spec of mosfetChargeStateSpecs(element)) {
        const state = capacitorStates.find((candidate) => candidate.name === spec.name);
        if (state === undefined) {
          continue;
        }
        const voltage = mosfetChargeStateVoltage(spec, nodeVoltages);
        state.previousVoltage = voltage;
        state.previousPreviousVoltage = voltage;
        state.previousCurrent = 0.0;
      }
    }
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
  operatingPoint: readonly number[],
): void {
  validateBjt(element);
  if (element.emitterResistance > 0.0) {
    const intrinsicEmitter = bjtIntrinsicEmitterNode(element);
    stampConductance(
      matrix,
      nodeIndex(nodeIndices, element.emitter),
      nodeIndex(nodeIndices, intrinsicEmitter),
      1.0 / element.emitterResistance,
    );
    element = { ...element, emitter: intrinsicEmitter, emitterResistance: 0.0 };
  }
  if (element.collectorResistance > 0.0) {
    const intrinsicCollector = bjtIntrinsicCollectorNode(element);
    stampConductance(
      matrix,
      nodeIndex(nodeIndices, element.collector),
      nodeIndex(nodeIndices, intrinsicCollector),
      1.0 / element.collectorResistance,
    );
    element = { ...element, collector: intrinsicCollector, collectorResistance: 0.0 };
  }
  if (element.baseResistance > 0.0) {
    const intrinsicBase = bjtIntrinsicBaseNode(element);
    const intrinsicBaseIndex = nodeIndex(nodeIndices, intrinsicBase);
    const baseVoltage = vectorVoltage(operatingPoint, intrinsicBaseIndex);
    const emitterVoltage = vectorVoltage(
      operatingPoint,
      nodeIndex(nodeIndices, element.emitter),
    );
    const collectorVoltage = vectorVoltage(
      operatingPoint,
      nodeIndex(nodeIndices, element.collector),
    );
    const baseResistance = bjtEffectiveBaseResistance(
      element,
      baseVoltage,
      emitterVoltage,
      collectorVoltage,
    );
    stampConductance(
      matrix,
      nodeIndex(nodeIndices, element.base),
      intrinsicBaseIndex,
      1.0 / baseResistance,
    );
    element = {
      ...element,
      base: intrinsicBase,
      baseResistance: 0.0,
      minimumBaseResistance: undefined,
      baseResistanceHalfCurrent: 0.0,
    };
  }
  const collector = nodeIndex(nodeIndices, element.collector);
  const base = nodeIndex(nodeIndices, element.base);
  const emitter = nodeIndex(nodeIndices, element.emitter);
  const baseVoltage = vectorVoltage(operatingPoint, base);
  const emitterVoltage = vectorVoltage(operatingPoint, emitter);
  const collectorVoltage = vectorVoltage(operatingPoint, collector);
  const junctionVoltage =
    element.polarity === "NPN"
      ? baseVoltage - emitterVoltage
      : emitterVoltage - baseVoltage;
  const reverseJunctionVoltage = element.polarity === "NPN"
    ? baseVoltage - collectorVoltage
    : collectorVoltage - baseVoltage;
  const forwardThermalVoltage = element.thermalVoltage * element.forwardEmissionCoefficient;
  const exponent = Math.max(-40.0, Math.min(40.0, junctionVoltage / forwardThermalVoltage));
  const expValue = Math.exp(exponent);
  const baseCollectorCurrent = element.saturationCurrent * (expValue - 1.0);
  const baseTransconductance = element.saturationCurrent / forwardThermalVoltage * expValue;
  const outputVoltage = element.polarity === "NPN"
    ? collectorVoltage - emitterVoltage
    : emitterVoltage - collectorVoltage;
  const earlyFactor = bjtEarlyFactor(element, junctionVoltage, outputVoltage);
  const transport = bjtForwardTransport(
    element,
    baseCollectorCurrent,
    baseTransconductance,
    earlyFactor,
  );
  const outputConductance = element.forwardEarlyVoltage === 0.0
    ? 0.0
    : baseCollectorCurrent / element.forwardEarlyVoltage / transport.chargeFactor;
  const transconductance = transport.transconductance;
  const leakage = bjtBaseEmitterLeakage(element, junctionVoltage);
  const junctionConductance =
    baseTransconductance / element.forwardBeta + leakage.conductance;
  const collectorLeakage = bjtBaseCollectorLeakage(element, reverseJunctionVoltage);
  const reverseBase = bjtReverseBaseCurrent(element, reverseJunctionVoltage);
  stampConductance(matrix, collector, emitter, outputConductance);
  stampConductance(
    matrix,
    base,
    collector,
    collectorLeakage.conductance + reverseBase.conductance,
  );
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
  const drain = nodeIndex(nodeIndices, mosfetIntrinsicDrainNode(element));
  const gate = nodeIndex(nodeIndices, element.gate);
  const source = nodeIndex(nodeIndices, mosfetIntrinsicSourceNode(element));
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
  const [, sourceBulkConductance] = mosfetBulkJunctionCurrentConductance(
    element,
    sourceVoltage,
    bodyVoltage,
    element.params.AS,
  );
  const [, drainBulkConductance] = mosfetBulkJunctionCurrentConductance(
    element,
    drainVoltage,
    bodyVoltage,
    element.params.AD,
  );
  stampConductance(matrix, drain, source, result.gds);
  stampConductance(matrix, body, source, sourceBulkConductance);
  stampConductance(matrix, body, drain, drainBulkConductance);
  stampTransconductance(matrix, drain, source, gate, source, result.gm);
  stampTransconductance(matrix, drain, source, body, source, result.gmb);
  const drainResistance = mosfetDrainResistance(element);
  if (drainResistance > 0.0) {
    stampConductance(
      matrix,
      nodeIndex(nodeIndices, element.drain),
      drain,
      1.0 / drainResistance,
    );
  }
  const sourceResistance = mosfetSourceResistance(element);
  if (sourceResistance > 0.0) {
    stampConductance(
      matrix,
      nodeIndex(nodeIndices, element.source),
      source,
      1.0 / sourceResistance,
    );
  }
}

function stampJfetSmallSignal(
  element: Jfet,
  nodeIndices: ReadonlyMap<string, number>,
  matrix: number[][],
  operatingPoint: readonly number[],
): void {
  validateJfet(element);
  const intrinsicDrain = jfetIntrinsicDrainNode(element);
  const intrinsicSource = jfetIntrinsicSourceNode(element);
  const drain = nodeIndex(nodeIndices, intrinsicDrain);
  const gate = nodeIndex(nodeIndices, element.gate);
  const source = nodeIndex(nodeIndices, intrinsicSource);
  const drainVoltage = vectorVoltage(operatingPoint, drain);
  const gateVoltage = vectorVoltage(operatingPoint, gate);
  const sourceVoltage = vectorVoltage(operatingPoint, source);
  const result = evaluateJfet(
    element,
    gateVoltage - sourceVoltage,
    drainVoltage - sourceVoltage,
  );
  const [, gateSourceConductance] =
    jfetGateJunctionCurrentConductance(element, gateVoltage - sourceVoltage);
  const [, gateDrainConductance] =
    jfetGateJunctionCurrentConductance(element, gateVoltage - drainVoltage);
  stampConductance(matrix, drain, source, result.gds);
  stampConductance(matrix, gate, source, gateSourceConductance);
  stampConductance(matrix, gate, drain, gateDrainConductance);
  stampTransconductance(matrix, drain, source, gate, source, result.gm);
  if (element.drainResistance > 0.0) {
    stampConductance(
      matrix,
      nodeIndex(nodeIndices, element.drain),
      drain,
      1.0 / element.drainResistance,
    );
  }
  if (element.sourceResistance > 0.0) {
    stampConductance(
      matrix,
      nodeIndex(nodeIndices, element.source),
      source,
      1.0 / element.sourceResistance,
    );
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
  operatingPoint: readonly number[],
  omega: number,
): void {
  validateBjt(element);
  const externalBase = nodeIndex(nodeIndices, element.base);
  if (element.emitterResistance > 0.0) {
    const intrinsicEmitter = bjtIntrinsicEmitterNode(element);
    stampComplexConductance(
      matrix,
      nodeIndex(nodeIndices, element.emitter),
      nodeIndex(nodeIndices, intrinsicEmitter),
      complex(1.0 / element.emitterResistance, 0.0),
    );
    element = { ...element, emitter: intrinsicEmitter, emitterResistance: 0.0 };
  }
  if (element.collectorResistance > 0.0) {
    const intrinsicCollector = bjtIntrinsicCollectorNode(element);
    stampComplexConductance(
      matrix,
      nodeIndex(nodeIndices, element.collector),
      nodeIndex(nodeIndices, intrinsicCollector),
      complex(1.0 / element.collectorResistance, 0.0),
    );
    element = { ...element, collector: intrinsicCollector, collectorResistance: 0.0 };
  }
  if (element.baseResistance > 0.0) {
    const intrinsicBase = bjtIntrinsicBaseNode(element);
    const intrinsicBaseIndex = nodeIndex(nodeIndices, intrinsicBase);
    const baseVoltage = vectorVoltage(operatingPoint, intrinsicBaseIndex);
    const emitterVoltage = vectorVoltage(
      operatingPoint,
      nodeIndex(nodeIndices, element.emitter),
    );
    const collectorVoltage = vectorVoltage(
      operatingPoint,
      nodeIndex(nodeIndices, element.collector),
    );
    const baseResistance = bjtEffectiveBaseResistance(
      element,
      baseVoltage,
      emitterVoltage,
      collectorVoltage,
    );
    stampComplexConductance(
      matrix,
      nodeIndex(nodeIndices, element.base),
      intrinsicBaseIndex,
      complex(1.0 / baseResistance, 0.0),
    );
    element = {
      ...element,
      base: intrinsicBase,
      baseResistance: 0.0,
      minimumBaseResistance: undefined,
      baseResistanceHalfCurrent: 0.0,
    };
  }
  const collector = nodeIndex(nodeIndices, element.collector);
  const base = nodeIndex(nodeIndices, element.base);
  const emitter = nodeIndex(nodeIndices, element.emitter);
  const collectorVoltage = vectorVoltage(operatingPoint, collector);
  const baseVoltage = vectorVoltage(operatingPoint, base);
  const emitterVoltage = vectorVoltage(operatingPoint, emitter);
  const junctionVoltage =
    element.polarity === "NPN"
      ? baseVoltage - emitterVoltage
      : emitterVoltage - baseVoltage;
  const reverseJunctionVoltage =
    element.polarity === "NPN"
      ? baseVoltage - collectorVoltage
      : collectorVoltage - baseVoltage;
  const forwardThermalVoltage = element.thermalVoltage * element.forwardEmissionCoefficient;
  const exponent = Math.max(-40.0, Math.min(40.0, junctionVoltage / forwardThermalVoltage));
  const reverseThermalVoltage = element.thermalVoltage * element.reverseEmissionCoefficient;
  const reverseExponent = Math.max(
    -40.0,
    Math.min(40.0, reverseJunctionVoltage / reverseThermalVoltage),
  );
  const expValue = Math.exp(exponent);
  const baseCollectorCurrent = element.saturationCurrent * (expValue - 1.0);
  const baseTransconductance = element.saturationCurrent / forwardThermalVoltage * expValue;
  const outputVoltage = element.polarity === "NPN"
    ? collectorVoltage - emitterVoltage
    : emitterVoltage - collectorVoltage;
  const earlyFactor = bjtEarlyFactor(element, junctionVoltage, outputVoltage);
  const transport = bjtForwardTransport(
    element,
    baseCollectorCurrent,
    baseTransconductance,
    earlyFactor,
  );
  const outputConductance = element.forwardEarlyVoltage === 0.0
    ? 0.0
    : baseCollectorCurrent / element.forwardEarlyVoltage / transport.chargeFactor;
  const transconductance = transport.transconductance;
  const leakage = bjtBaseEmitterLeakage(element, junctionVoltage);
  const collectorLeakage = bjtBaseCollectorLeakage(element, reverseJunctionVoltage);
  const reverseBase = bjtReverseBaseCurrent(element, reverseJunctionVoltage);
  const junctionConductance =
    baseTransconductance / element.forwardBeta + leakage.conductance;
  const diffusionCapacitance =
    element.forwardTransitTime *
    bjtForwardTransitTimeScale(element, junctionVoltage, reverseJunctionVoltage) *
    transconductance;
  const excessPhase =
    omega * element.forwardTransitTime * element.forwardExcessPhaseDegrees * Math.PI / 180.0;
  const acTransconductance = complex(
    transconductance * Math.cos(excessPhase),
    -transconductance * Math.sin(excessPhase),
  );
  const reverseTransconductance =
    element.saturationCurrent / reverseThermalVoltage * Math.exp(reverseExponent);
  const reverseDiffusionCapacitance = element.reverseTransitTime * reverseTransconductance;
  const baseEmitterAdmittance = complex(
    junctionConductance,
    omega * (
      bjtBaseEmitterDepletionCapacitance(element, junctionVoltage) + diffusionCapacitance
    ),
  );
  const baseCollectorDepletion =
    bjtBaseCollectorDepletionCapacitance(element, reverseJunctionVoltage);
  const baseCollectorAdmittance = complex(
    collectorLeakage.conductance + reverseBase.conductance,
    omega * (
      element.baseCollectorCapacitanceFraction * baseCollectorDepletion +
      reverseDiffusionCapacitance
    ),
  );
  const externalBaseCollectorAdmittance = complex(
    0.0,
    omega * (1.0 - element.baseCollectorCapacitanceFraction) *
      baseCollectorDepletion,
  );
  stampComplexConductance(matrix, collector, emitter, complex(outputConductance, 0.0));
  if (externalBaseCollectorAdmittance.imag !== 0.0) {
    stampComplexConductance(
      matrix,
      externalBase,
      collector,
      externalBaseCollectorAdmittance,
    );
  }
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
      acTransconductance,
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
      acTransconductance,
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
  const drain = nodeIndex(nodeIndices, mosfetIntrinsicDrainNode(element));
  const gate = nodeIndex(nodeIndices, element.gate);
  const source = nodeIndex(nodeIndices, mosfetIntrinsicSourceNode(element));
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
  const [, sourceBulkConductance] = mosfetBulkJunctionCurrentConductance(
    element,
    sourceVoltage,
    bodyVoltage,
    element.params.AS,
  );
  const [, drainBulkConductance] = mosfetBulkJunctionCurrentConductance(
    element,
    drainVoltage,
    bodyVoltage,
    element.params.AD,
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
  stampComplexConductance(
    matrix,
    body,
    source,
    complex(sourceBulkConductance, omega * result.cbs),
  );
  stampComplexConductance(
    matrix,
    body,
    drain,
    complex(drainBulkConductance, omega * result.cbd),
  );
  stampComplexTransconductance(
    matrix,
    drain,
    source,
    body,
    source,
    complex(result.gmb, 0.0),
  );
  const drainResistance = mosfetDrainResistance(element);
  if (drainResistance > 0.0) {
    stampComplexConductance(
      matrix,
      nodeIndex(nodeIndices, element.drain),
      drain,
      complex(1.0 / drainResistance, 0.0),
    );
  }
  const sourceResistance = mosfetSourceResistance(element);
  if (sourceResistance > 0.0) {
    stampComplexConductance(
      matrix,
      nodeIndex(nodeIndices, element.source),
      source,
      complex(1.0 / sourceResistance, 0.0),
    );
  }
}

function stampAcJfetSmallSignal(
  element: Jfet,
  nodeIndices: ReadonlyMap<string, number>,
  matrix: Complex[][],
  operatingPoint: readonly number[],
  omega: number,
): void {
  validateJfet(element);
  const intrinsicDrain = jfetIntrinsicDrainNode(element);
  const intrinsicSource = jfetIntrinsicSourceNode(element);
  const drain = nodeIndex(nodeIndices, intrinsicDrain);
  const gate = nodeIndex(nodeIndices, element.gate);
  const source = nodeIndex(nodeIndices, intrinsicSource);
  const drainVoltage = vectorVoltage(operatingPoint, drain);
  const gateVoltage = vectorVoltage(operatingPoint, gate);
  const sourceVoltage = vectorVoltage(operatingPoint, source);
  const result = evaluateJfet(
    element,
    gateVoltage - sourceVoltage,
    drainVoltage - sourceVoltage,
  );
  const gateSourceCapacitance = jfetChargeDynamicCapacitance(
    element,
    element.gateSourceCapacitance,
    gateVoltage - sourceVoltage,
  );
  const gateDrainCapacitance = jfetChargeDynamicCapacitance(
    element,
    element.gateDrainCapacitance,
    gateVoltage - drainVoltage,
  );
  const [, gateSourceConductance] =
    jfetGateJunctionCurrentConductance(element, gateVoltage - sourceVoltage);
  const [, gateDrainConductance] =
    jfetGateJunctionCurrentConductance(element, gateVoltage - drainVoltage);
  stampComplexConductance(matrix, drain, source, complex(result.gds, 0.0));
  stampComplexConductance(
    matrix,
    gate,
    source,
    complex(gateSourceConductance, omega * gateSourceCapacitance),
  );
  stampComplexConductance(
    matrix,
    gate,
    drain,
    complex(gateDrainConductance, omega * gateDrainCapacitance),
  );
  stampComplexTransconductance(
    matrix,
    drain,
    source,
    gate,
    source,
    complex(result.gm, 0.0),
  );
  if (element.drainResistance > 0.0) {
    stampComplexConductance(
      matrix,
      nodeIndex(nodeIndices, element.drain),
      drain,
      complex(1.0 / element.drainResistance, 0.0),
    );
  }
  if (element.sourceResistance > 0.0) {
    stampComplexConductance(
      matrix,
      nodeIndex(nodeIndices, element.source),
      source,
      complex(1.0 / element.sourceResistance, 0.0),
    );
  }
}

function solveLinearSystem(matrix: number[][], rhs: number[]): number[] {
  return [...solveLinearSystemWithProfile(matrix, rhs).solution];
}

function solveLinearSystemWithProfile(matrix: number[][], rhs: number[]): LinearSystemSolve {
  if (rhs.length >= SPARSE_SOLVER_THRESHOLD) {
    return solveSparseLinearSystemWithProfile(matrix, rhs);
  }
  const profile = realSolverProfile(matrix, "dense_gaussian");
  return {
    solution: solveDenseLinearSystem(matrix, rhs),
    profile,
  };
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
  return [...solveSparseLinearSystemWithProfile(matrix, rhs).solution];
}

function solveSparseLinearSystemWithProfile(matrix: number[][], rhs: number[]): LinearSystemSolve {
  const n = rhs.length;
  const initialNonzeros = realMatrixNonzeros(matrix);
  let peakNonzeros = initialNonzeros;
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
    peakNonzeros = Math.max(
      peakNonzeros,
      rows.reduce((count, row) => count + row.size, 0),
    );
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
  return {
    solution,
    profile: realSolverProfile(
      matrix,
      "native_sparse_gaussian",
      Math.max(0, peakNonzeros - initialNonzeros),
    ),
  };
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
  if (complexSolverKind(rhs.length) === "sparse_complex") {
    return solveSparseComplexLinearSystem(matrix, rhs);
  }
  return solveDenseComplexLinearSystem(matrix, rhs);
}

function solveDenseComplexLinearSystem(matrix: Complex[][], rhs: Complex[]): Complex[] {
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

function solveSparseComplexLinearSystem(matrix: Complex[][], rhs: Complex[]): Complex[] {
  const n = rhs.length;
  const rows = matrix.map((row) => {
    const entries = new Map<number, Complex>();
    row.forEach((value, col) => {
      if (value.real !== 0.0 || value.imag !== 0.0) {
        entries.set(col, value);
      }
    });
    return entries;
  });
  const sparseRhs = [...rhs];

  for (let pivotCol = 0; pivotCol < n; pivotCol++) {
    let pivotRow = pivotCol;
    let pivotAbs = complexAbs(rows[pivotCol].get(pivotCol) ?? complex(0.0, 0.0));
    for (let row = pivotCol + 1; row < n; row++) {
      const candidateAbs = complexAbs(rows[row].get(pivotCol) ?? complex(0.0, 0.0));
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
      const value = rows[row].get(pivotCol) ?? complex(0.0, 0.0);
      if (value.real === 0.0 && value.imag === 0.0) {
        continue;
      }
      const factor = complexDiv(value, pivot);
      rows[row].delete(pivotCol);
      for (const [col, pivotValue] of pivotEntries) {
        const nextValue = complexSub(
          rows[row].get(col) ?? complex(0.0, 0.0),
          complexMul(factor, pivotValue),
        );
        if (complexAbs(nextValue) < PIVOT_EPSILON) {
          rows[row].delete(col);
        } else {
          rows[row].set(col, nextValue);
        }
      }
      sparseRhs[row] = complexSub(sparseRhs[row], complexMul(factor, sparseRhs[pivotCol]));
    }
  }

  const solution = Array.from({ length: n }, () => complex(0.0, 0.0));
  for (let row = n - 1; row >= 0; row--) {
    const diagonal = rows[row].get(row) ?? complex(0.0, 0.0);
    if (complexAbs(diagonal) < PIVOT_EPSILON) {
      throw new SpiceError("circuit matrix is singular", "SINGULAR_MATRIX");
    }
    let value = sparseRhs[row];
    for (const [col, entry] of rows[row].entries()) {
      if (col > row) {
        value = complexSub(value, complexMul(entry, solution[col]));
      }
    }
    solution[row] = complexDiv(value, diagonal);
    if (!Number.isFinite(solution[row].real) || !Number.isFinite(solution[row].imag)) {
      throw new SpiceError("circuit matrix is singular", "SINGULAR_MATRIX");
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
