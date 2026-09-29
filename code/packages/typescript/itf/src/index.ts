/**
 * @coding-adventures/itf
 *
 * Interleaved 2 of 5 is the first barcode in this repository where two digits
 * share one visual block: one digit drives the bar widths and the next digit
 * drives the space widths.
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

export interface EncodedPair {
  pair: string;
  barPattern: string;
  spacePattern: string;
  binaryPattern: string;
  sourceIndex: number;
}

export class ItfError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ItfError";
  }
}

export type ItfErrorId = "input-too-long" | "invalid-length" | "invalid-character";

export class InvalidItfInputError extends ItfError {
  readonly errorId?: ItfErrorId;

  constructor(message: string, errorId?: ItfErrorId) {
    super(message);
    this.name = "InvalidItfInputError";
    this.errorId = errorId;
  }
}

const START_PATTERN = "1010";
const STOP_PATTERN = "11101";

const DIGIT_PATTERNS = [
  "00110",
  "10001",
  "01001",
  "11000",
  "00101",
  "10100",
  "01100",
  "00011",
  "10010",
  "01010",
];

export function normalizeItf(data: string): string {
  const scalars = Array.from(data);
  if (scalars.some((scalar) => {
    const codePoint = scalar.codePointAt(0)!;
    return codePoint >= 0xd800 && codePoint <= 0xdfff;
  })) {
    throw new TypeError("ITF input must contain valid Unicode scalar values");
  }
  if (scalars.length > 4096) {
    throw new InvalidItfInputError(
      "ITF input must contain at most 4096 characters",
      "input-too-long",
    );
  }
  if (scalars.length === 0 || scalars.length % 2 !== 0) {
    throw new InvalidItfInputError(
      "ITF input must contain an even number of digits",
      "invalid-length",
    );
  }
  if (scalars.some((scalar) => scalar < "0" || scalar > "9")) {
    throw new InvalidItfInputError("ITF input must contain digits only", "invalid-character");
  }

  return data;
}

function encodePair(pair: string, sourceIndex: number): EncodedPair {
  const barPattern = DIGIT_PATTERNS[Number(pair[0])];
  const spacePattern = DIGIT_PATTERNS[Number(pair[1])];
  const binaryPattern = barPattern
    .split("")
    .map((barMarker, index) => {
      const spaceMarker = spacePattern[index];
      return `${barMarker === "1" ? "111" : "1"}${spaceMarker === "1" ? "000" : "0"}`;
    })
    .join("");

  return { pair, barPattern, spacePattern, binaryPattern, sourceIndex };
}

export function encodeItf(data: string): EncodedPair[] {
  const normalized = normalizeItf(data);
  return normalized.match(/.{2}/g)?.map((pair, index) => encodePair(pair, index)) ?? [];
}

function buildSymbols(encodedPairs: EncodedPair[]): Barcode1DSymbolDescriptor[] {
  return [
    { label: "start", modules: START_PATTERN.length, sourceIndex: -1, role: "start" },
    ...encodedPairs.map((pair) => ({
      label: pair.pair,
      modules: pair.binaryPattern.length,
      sourceIndex: pair.sourceIndex,
      role: "data" as const,
    })),
    { label: "stop", modules: STOP_PATTERN.length, sourceIndex: -2, role: "stop" },
  ];
}

export function expandItfRuns(data: string): Barcode1DRun[] {
  const encodedPairs = encodeItf(data);
  const runs: Barcode1DRun[] = [];

  runs.push(...runsFromBinaryPattern(START_PATTERN, { sourceLabel: "start", sourceIndex: -1, role: "start" }));

  encodedPairs.forEach((pair) => {
    runs.push(
      ...runsFromBinaryPattern(pair.binaryPattern, {
        sourceLabel: pair.pair,
        sourceIndex: pair.sourceIndex,
        role: "data",
      }),
    );
  });

  runs.push(...runsFromBinaryPattern(STOP_PATTERN, { sourceLabel: "stop", sourceIndex: -2, role: "stop" }));
  return runs;
}

export function layoutItf(
  data: string,
  options: Omit<PaintBarcode1DOptions, "symbols" | "humanReadableText" | "label" | "metadata"> & {
    metadata?: Record<string, string | number | boolean>;
    label?: string;
  } = {},
): PaintScene {
  const normalized = normalizeItf(data);
  const encodedPairs = encodeItf(normalized);

  return layoutBarcode1D(expandItfRuns(normalized), {
    ...options,
    symbols: buildSymbols(encodedPairs),
    label: options.label ?? `ITF barcode for ${normalized}`,
    metadata: {
      ...options.metadata,
      symbology: "itf",
      pairCount: encodedPairs.length,
    },
  });
}

export function drawItf(
  data: string,
  options: Omit<PaintBarcode1DOptions, "symbols" | "humanReadableText" | "label" | "metadata"> & {
    metadata?: Record<string, string | number | boolean>;
    label?: string;
  } = {},
): PaintScene {
  return layoutItf(data, options);
}
