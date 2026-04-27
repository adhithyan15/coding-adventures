/**
 * @coding-adventures/code128
 *
 * This package implements Code 128 Code Set B in a deliberately explicit way.
 *
 * Code Set B is a good first slice because it covers printable ASCII and still
 * exposes the interesting parts of Code 128:
 * - start codes
 * - symbol values
 * - modulo-103 checksum
 * - a stop pattern with a different width than ordinary data symbols
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

export interface EncodedCode128Symbol {
  label: string;
  value: number;
  pattern: string;
  sourceIndex: number;
  role: "start" | "data" | "check" | "stop";
}

export class Code128Error extends Error {
  constructor(message: string) {
    super(message);
    this.name = "Code128Error";
  }
}

export class InvalidCode128InputError extends Code128Error {
  constructor(message: string) {
    super(message);
    this.name = "InvalidCode128InputError";
  }
}

const START_B = 104;
const STOP = 106;

// Values 0-105 use one six-width pattern. Value 106 is the stop pattern.
const PATTERNS = [
  "11011001100", "11001101100", "11001100110", "10010011000", "10010001100",
  "10001001100", "10011001000", "10011000100", "10001100100", "11001001000",
  "11001000100", "11000100100", "10110011100", "10011011100", "10011001110",
  "10111001100", "10011101100", "10011100110", "11001110010", "11001011100",
  "11001001110", "11011100100", "11001110100", "11101101110", "11101001100",
  "11100101100", "11100100110", "11101100100", "11100110100", "11100110010",
  "11011011000", "11011000110", "11000110110", "10100011000", "10001011000",
  "10001000110", "10110001000", "10001101000", "10001100010", "11010001000",
  "11000101000", "11000100010", "10110111000", "10110001110", "10001101110",
  "10111011000", "10111000110", "10001110110", "11101110110", "11010001110",
  "11000101110", "11011101000", "11011100010", "11011101110", "11101011000",
  "11101000110", "11100010110", "11101101000", "11101100010", "11100011010",
  "11101111010", "11001000010", "11110001010", "10100110000", "10100001100",
  "10010110000", "10010000110", "10000101100", "10000100110", "10110010000",
  "10110000100", "10011010000", "10011000010", "10000110100", "10000110010",
  "11000010010", "11001010000", "11110111010", "11000010100", "10001111010",
  "10100111100", "10010111100", "10010011110", "10111100100", "10011110100",
  "10011110010", "11110100100", "11110010100", "11110010010", "11011011110",
  "11011110110", "11110110110", "10101111000", "10100011110", "10001011110",
  "10111101000", "10111100010", "11110101000", "11110100010", "10111011110",
  "10111101110", "11101011110", "11110101110", "11010000100", "11010010000",
  "11010011100", "1100011101011",
] as const;

export function normalizeCode128B(data: string): string {
  for (const char of data) {
    const code = char.charCodeAt(0);
    if (code < 32 || code > 126) {
      throw new InvalidCode128InputError("Code 128 Code Set B supports printable ASCII characters only");
    }
  }

  return data;
}

function valueForCode128BChar(char: string): number {
  return char.charCodeAt(0) - 32;
}

export function computeCode128Checksum(values: number[]): number {
  const weightedSum = values.reduce((sum, value, index) => sum + value * (index + 1), START_B);
  return weightedSum % 103;
}

export function encodeCode128B(data: string): EncodedCode128Symbol[] {
  const normalized = normalizeCode128B(data);
  const dataSymbols = normalized.split("").map((char, index) => {
    const value = valueForCode128BChar(char);
    return {
      label: char,
      value,
      pattern: PATTERNS[value],
      sourceIndex: index,
      role: "data" as const,
    };
  });
  const checksum = computeCode128Checksum(dataSymbols.map((symbol) => symbol.value));

  return [
    { label: "Start B", value: START_B, pattern: PATTERNS[START_B], sourceIndex: -1, role: "start" },
    ...dataSymbols,
    { label: `Checksum ${checksum}`, value: checksum, pattern: PATTERNS[checksum], sourceIndex: normalized.length, role: "check" },
    { label: "Stop", value: STOP, pattern: PATTERNS[STOP], sourceIndex: normalized.length + 1, role: "stop" },
  ];
}

function buildSymbols(encoded: EncodedCode128Symbol[]): Barcode1DSymbolDescriptor[] {
  return encoded.map((entry) => ({
    label: entry.label,
    modules: entry.role === "stop" ? 13 : 11,
    sourceIndex: entry.sourceIndex,
    role: entry.role,
  }));
}

export function expandCode128Runs(data: string): Barcode1DRun[] {
  return encodeCode128B(data).flatMap((entry) =>
    runsFromBinaryPattern(entry.pattern, {
      sourceLabel: entry.label,
      sourceIndex: entry.sourceIndex,
      role: entry.role,
    }),
  );
}

export function layoutCode128(
  data: string,
  options: Omit<PaintBarcode1DOptions, "symbols" | "humanReadableText" | "label" | "metadata"> & {
    metadata?: Record<string, string | number | boolean>;
    label?: string;
  } = {},
): PaintScene {
  const normalized = normalizeCode128B(data);
  const encoded = encodeCode128B(normalized);
  const checksum = encoded[encoded.length - 2]?.value ?? 0;

  return layoutBarcode1D(expandCode128Runs(normalized), {
    ...options,
    symbols: buildSymbols(encoded),
    label: options.label ?? `Code 128 barcode for ${normalized}`,
    metadata: {
      ...options.metadata,
      symbology: "code128",
      codeSet: "B",
      checksum,
    },
  });
}

export function drawCode128(
  data: string,
  options: Omit<PaintBarcode1DOptions, "symbols" | "humanReadableText" | "label" | "metadata"> & {
    metadata?: Record<string, string | number | boolean>;
    label?: string;
  } = {},
): PaintScene {
  return layoutCode128(data, options);
}
