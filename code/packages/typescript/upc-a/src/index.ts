/**
 * @coding-adventures/upc-a
 *
 * UPC-A is the first retail barcode in the repository's barcode track.
 *
 * It teaches a different lesson than Code 39:
 * - fixed-width numeric digits
 * - start, center, and end guards
 * - a required modulo-10 check digit
 *
 * The package keeps those rules explicit, then hands the geometry to the
 * shared linear-barcode package.
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

type UpcEncoding = "L" | "R";

export interface EncodedDigit {
  digit: string;
  encoding: UpcEncoding;
  pattern: string;
  sourceIndex: number;
  role: "data" | "check";
}

export class UpcAError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "UpcAError";
  }
}

export class InvalidUpcAInputError extends UpcAError {
  constructor(message: string) {
    super(message);
    this.name = "InvalidUpcAInputError";
  }
}

export class InvalidUpcACheckDigitError extends UpcAError {
  constructor(message: string) {
    super(message);
    this.name = "InvalidUpcACheckDigitError";
  }
}

const SIDE_GUARD = "101";
const CENTER_GUARD = "01010";

const DIGIT_PATTERNS: Record<UpcEncoding, string[]> = {
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

function assertDigits(data: string, expectedLengths: number[]): void {
  if (!/^\d+$/.test(data)) {
    throw new InvalidUpcAInputError("UPC-A input must contain digits only");
  }

  if (!expectedLengths.includes(data.length)) {
    throw new InvalidUpcAInputError("UPC-A input must contain 11 digits or 12 digits");
  }
}

export function computeUpcACheckDigit(payload11: string): string {
  assertDigits(payload11, [11]);

  let oddSum = 0;
  let evenSum = 0;

  payload11.split("").forEach((digit, index) => {
    const value = Number(digit);
    if (index % 2 === 0) {
      oddSum += value;
    } else {
      evenSum += value;
    }
  });

  const total = oddSum * 3 + evenSum;
  return String((10 - (total % 10)) % 10);
}

export function normalizeUpcA(data: string): string {
  assertDigits(data, [11, 12]);

  if (data.length === 11) {
    return `${data}${computeUpcACheckDigit(data)}`;
  }

  const expected = computeUpcACheckDigit(data.slice(0, 11));
  const actual = data[11];

  if (expected !== actual) {
    throw new InvalidUpcACheckDigitError(`Invalid UPC-A check digit: expected ${expected} but received ${actual}`);
  }

  return data;
}

export function encodeUpcA(data: string): EncodedDigit[] {
  const normalized = normalizeUpcA(data);

  return normalized.split("").map((digit, index) => ({
    digit,
    encoding: index < 6 ? "L" : "R",
    pattern: index < 6 ? DIGIT_PATTERNS.L[Number(digit)] : DIGIT_PATTERNS.R[Number(digit)],
    sourceIndex: index,
    role: index === 11 ? "check" : "data",
  }));
}

function buildUpcASymbols(encodedDigits: EncodedDigit[]): Barcode1DSymbolDescriptor[] {
  return [
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

export function expandUpcARuns(data: string): Barcode1DRun[] {
  const encodedDigits = encodeUpcA(data);
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

export function layoutUpcA(
  data: string,
  options: Omit<PaintBarcode1DOptions, "symbols" | "humanReadableText" | "label" | "metadata"> & {
    metadata?: Record<string, string | number | boolean>;
    label?: string;
  } = {},
): PaintScene {
  const normalized = normalizeUpcA(data);
  const encodedDigits = encodeUpcA(normalized);

  return layoutBarcode1D(expandUpcARuns(normalized), {
    ...options,
    symbols: buildUpcASymbols(encodedDigits),
    label: options.label ?? `UPC-A barcode for ${normalized}`,
    metadata: {
      ...options.metadata,
      symbology: "upc-a",
    },
  });
}

export function drawUpcA(
  data: string,
  options: Omit<PaintBarcode1DOptions, "symbols" | "humanReadableText" | "label" | "metadata"> & {
    metadata?: Record<string, string | number | boolean>;
    label?: string;
  } = {},
): PaintScene {
  return layoutUpcA(data, options);
}
