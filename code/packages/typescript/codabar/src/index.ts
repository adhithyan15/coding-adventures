/**
 * @coding-adventures/codabar
 *
 * Codabar stays close to Code 39 in spirit, but it is a better teaching
 * example for configurable start/stop symbols.
 */
import {
  layoutBarcode1D,
  runsFromBinaryPattern,
  type Barcode1DRun,
  type PaintBarcode1DOptions,
} from "@coding-adventures/barcode-layout-1d";
import type { PaintScene } from "@coding-adventures/paint-instructions";

export const VERSION = "0.1.0";

export type CodabarGuard = "A" | "B" | "C" | "D";

export interface CodabarOptions {
  start?: CodabarGuard;
  stop?: CodabarGuard;
}

export interface EncodedCodabarSymbol {
  char: string;
  pattern: string;
  sourceIndex: number;
  role: "data" | "start" | "stop";
}

export class CodabarError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "CodabarError";
  }
}

export class InvalidCodabarInputError extends CodabarError {
  constructor(message: string) {
    super(message);
    this.name = "InvalidCodabarInputError";
  }
}

const GUARDS = new Set<CodabarGuard>(["A", "B", "C", "D"]);

const PATTERNS: Record<string, string> = {
  "0": "101010011",
  "1": "101011001",
  "2": "101001011",
  "3": "110010101",
  "4": "101101001",
  "5": "110101001",
  "6": "100101011",
  "7": "100101101",
  "8": "100110101",
  "9": "110100101",
  "-": "101001101",
  $: "101100101",
  ":": "1101011011",
  "/": "1101101011",
  ".": "1101101101",
  "+": "1011011011",
  A: "1011001001",
  B: "1001001011",
  C: "1010010011",
  D: "1010011001",
};

function isGuard(char: string): char is CodabarGuard {
  return GUARDS.has(char as CodabarGuard);
}

function assertBodyChars(body: string): void {
  for (const char of body) {
    if (!(char in PATTERNS) || isGuard(char)) {
      throw new InvalidCodabarInputError(`Invalid Codabar body character "${char}"`);
    }
  }
}

export function normalizeCodabar(data: string, options: CodabarOptions = {}): string {
  const normalized = data.toUpperCase();

  if (normalized.length >= 2 && isGuard(normalized[0]) && isGuard(normalized[normalized.length - 1])) {
    const body = normalized.slice(1, -1);
    assertBodyChars(body);
    return normalized;
  }

  assertBodyChars(normalized);
  return `${options.start ?? "A"}${normalized}${options.stop ?? "A"}`;
}

export function encodeCodabar(data: string, options: CodabarOptions = {}): EncodedCodabarSymbol[] {
  const normalized = normalizeCodabar(data, options);

  return normalized.split("").map((char, index) => ({
    char,
    pattern: PATTERNS[char],
    sourceIndex: index,
    role: index === 0 ? "start" : index === normalized.length - 1 ? "stop" : "data",
  }));
}

export function expandCodabarRuns(data: string, options: CodabarOptions = {}): Barcode1DRun[] {
  const encoded = encodeCodabar(data, options);
  const runs: Barcode1DRun[] = [];

  encoded.forEach((entry, index) => {
    runs.push(
      ...runsFromBinaryPattern(entry.pattern, {
        sourceLabel: entry.char,
        sourceIndex: entry.sourceIndex,
        role: entry.role === "data" ? "data" : entry.role,
      }),
    );

    if (index < encoded.length - 1) {
      runs.push({
        color: "space",
        modules: 1,
        sourceLabel: entry.char,
        sourceIndex: entry.sourceIndex,
        role: "inter-character-gap",
      });
    }
  });

  return runs;
}

export function layoutCodabar(
  data: string,
  options: CodabarOptions &
    Omit<PaintBarcode1DOptions, "symbols" | "humanReadableText" | "label" | "metadata"> & {
      metadata?: Record<string, string | number | boolean>;
      label?: string;
    } = {},
): PaintScene {
  const { start, stop, ...drawOptions } = options;
  const normalized = normalizeCodabar(data, { start, stop });

  return layoutBarcode1D(expandCodabarRuns(normalized), {
    ...drawOptions,
    label: drawOptions.label ?? `Codabar barcode for ${normalized}`,
    metadata: {
      ...drawOptions.metadata,
      symbology: "codabar",
      start: normalized[0],
      stop: normalized[normalized.length - 1],
    },
  });
}

export function drawCodabar(
  data: string,
  options: CodabarOptions &
    Omit<PaintBarcode1DOptions, "symbols" | "humanReadableText" | "label" | "metadata"> & {
      metadata?: Record<string, string | number | boolean>;
      label?: string;
    } = {},
): PaintScene {
  return layoutCodabar(data, options);
}
