/**
 * @coding-adventures/ean-13
 *
 * EAN-13 builds directly on the retail-module model introduced by UPC-A, but
 * it adds one especially interesting twist: the first digit is encoded
 * indirectly through the parity pattern of the next six digits.
 */
import {
  layoutBarcode1D,
  runsFromBinaryPattern,
  type Barcode1DRun,
  type Barcode1DSymbolDescriptor,
  type PaintBarcode1DOptions,
} from "@coding-adventures/barcode-layout-1d";
import type { PaintScene } from "@coding-adventures/paint-instructions";

export const VERSION = "0.1.0";

type EanEncoding = "L" | "G" | "R";

export interface EncodedDigit {
  digit: string;
  encoding: EanEncoding;
  pattern: string;
  sourceIndex: number;
  role: "data" | "check";
}

export class Ean13Error extends Error {
  constructor(message: string) {
    super(message);
    this.name = "Ean13Error";
  }
}

export class InvalidEan13InputError extends Ean13Error {
  constructor(message: string) {
    super(message);
    this.name = "InvalidEan13InputError";
  }
}

export class InvalidEan13CheckDigitError extends Ean13Error {
  constructor(message: string) {
    super(message);
    this.name = "InvalidEan13CheckDigitError";
  }
}

const SIDE_GUARD = "101";
const CENTER_GUARD = "01010";

const DIGIT_PATTERNS: Record<EanEncoding, string[]> = {
  L: [
    "0001101",
    "0011001",
    "0010011",
    "0111101",
    "0100011",
    "0110001",
    "0101111",
    "0111011",
    "0110111",
    "0001011",
  ],
  G: [
    "0100111",
    "0110011",
    "0011011",
    "0100001",
    "0011101",
    "0111001",
    "0000101",
    "0010001",
    "0001001",
    "0010111",
  ],
  R: [
    "1110010",
    "1100110",
    "1101100",
    "1000010",
    "1011100",
    "1001110",
    "1010000",
    "1000100",
    "1001000",
    "1110100",
  ],
};

const LEFT_PARITY_PATTERNS = [
  "LLLLLL",
  "LLGLGG",
  "LLGGLG",
  "LLGGGL",
  "LGLLGG",
  "LGGLLG",
  "LGGGLL",
  "LGLGLG",
  "LGLGGL",
  "LGGLGL",
] as const;

function assertDigits(data: string, expectedLengths: number[]): void {
  if (!/^\d+$/.test(data)) {
    throw new InvalidEan13InputError("EAN-13 input must contain digits only");
  }

  if (!expectedLengths.includes(data.length)) {
    throw new InvalidEan13InputError("EAN-13 input must contain 12 digits or 13 digits");
  }
}

export function computeEan13CheckDigit(payload12: string): string {
  assertDigits(payload12, [12]);

  const total = payload12
    .split("")
    .reverse()
    .reduce((sum, digit, index) => sum + Number(digit) * (index % 2 === 0 ? 3 : 1), 0);

  return String((10 - (total % 10)) % 10);
}

export function normalizeEan13(data: string): string {
  assertDigits(data, [12, 13]);

  if (data.length === 12) {
    return `${data}${computeEan13CheckDigit(data)}`;
  }

  const expected = computeEan13CheckDigit(data.slice(0, 12));
  const actual = data[12];

  if (expected !== actual) {
    throw new InvalidEan13CheckDigitError(`Invalid EAN-13 check digit: expected ${expected} but received ${actual}`);
  }

  return data;
}

export function leftParityPattern(data: string): string {
  const normalized = normalizeEan13(data);
  return LEFT_PARITY_PATTERNS[Number(normalized[0])];
}

export function encodeEan13(data: string): EncodedDigit[] {
  const normalized = normalizeEan13(data);
  const parity = LEFT_PARITY_PATTERNS[Number(normalized[0])];
  const digits = normalized.split("");

  const leftDigits = digits.slice(1, 7).map((digit, offset) => {
    const encoding = parity[offset] as "L" | "G";
    return {
      digit,
      encoding,
      pattern: DIGIT_PATTERNS[encoding][Number(digit)],
      sourceIndex: offset + 1,
      role: "data" as const,
    };
  });

  const rightDigits = digits.slice(7).map((digit, offset) => ({
    digit,
    encoding: "R" as const,
    pattern: DIGIT_PATTERNS.R[Number(digit)],
    sourceIndex: offset + 7,
    role: offset === 5 ? "check" as const : "data" as const,
  }));

  return [...leftDigits, ...rightDigits];
}

function buildEan13Symbols(normalized: string, encodedDigits: EncodedDigit[]): Barcode1DSymbolDescriptor[] {
  return [
    { label: normalized[0], modules: 0, sourceIndex: 0, role: "data" },
    { label: "start", modules: 3, sourceIndex: -1, role: "guard" },
    ...encodedDigits.slice(0, 6).map((entry) => ({
      label: entry.digit,
      modules: 7,
      sourceIndex: entry.sourceIndex,
      role: entry.role,
    })),
    { label: "center", modules: 5, sourceIndex: -2, role: "guard" },
    ...encodedDigits.slice(6).map((entry) => ({
      label: entry.digit,
      modules: 7,
      sourceIndex: entry.sourceIndex,
      role: entry.role,
    })),
    { label: "end", modules: 3, sourceIndex: -3, role: "guard" },
  ];
}

export function expandEan13Runs(data: string): Barcode1DRun[] {
  const encodedDigits = encodeEan13(data);
  const runs: Barcode1DRun[] = [];

  runs.push(...runsFromBinaryPattern(SIDE_GUARD, { sourceLabel: "start", sourceIndex: -1, role: "guard" }));

  encodedDigits.slice(0, 6).forEach((entry) => {
    runs.push(
      ...runsFromBinaryPattern(entry.pattern, {
        sourceLabel: entry.digit,
        sourceIndex: entry.sourceIndex,
        role: entry.role,
      }),
    );
  });

  runs.push(...runsFromBinaryPattern(CENTER_GUARD, { sourceLabel: "center", sourceIndex: -2, role: "guard" }));

  encodedDigits.slice(6).forEach((entry) => {
    runs.push(
      ...runsFromBinaryPattern(entry.pattern, {
        sourceLabel: entry.digit,
        sourceIndex: entry.sourceIndex,
        role: entry.role,
      }),
    );
  });

  runs.push(...runsFromBinaryPattern(SIDE_GUARD, { sourceLabel: "end", sourceIndex: -3, role: "guard" }));
  return runs;
}

export function layoutEan13(
  data: string,
  options: Omit<PaintBarcode1DOptions, "symbols" | "humanReadableText" | "label" | "metadata"> & {
    metadata?: Record<string, string | number | boolean>;
    label?: string;
  } = {},
): PaintScene {
  const normalized = normalizeEan13(data);
  const encodedDigits = encodeEan13(normalized);

  return layoutBarcode1D(expandEan13Runs(normalized), {
    ...options,
    symbols: buildEan13Symbols(normalized, encodedDigits).filter((symbol) => symbol.modules > 0),
    label: options.label ?? `EAN-13 barcode for ${normalized}`,
    metadata: {
      ...options.metadata,
      symbology: "ean-13",
      leadingDigit: normalized[0],
      leftParity: LEFT_PARITY_PATTERNS[Number(normalized[0])],
    },
  });
}

export function drawEan13(
  data: string,
  options: Omit<PaintBarcode1DOptions, "symbols" | "humanReadableText" | "label" | "metadata"> & {
    metadata?: Record<string, string | number | boolean>;
    label?: string;
  } = {},
): PaintScene {
  return layoutEan13(data, options);
}
