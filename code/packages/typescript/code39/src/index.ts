/**
 * @coding-adventures/code39
 *
 * Dependency-free Code 39 encoder that expands into shared barcode runs and
 * then hands layout to @coding-adventures/barcode-layout-1d.
 */
import {
  DEFAULT_BARCODE_1D_RENDER_CONFIG,
  Barcode1DError,
  InvalidBarcode1DConfigurationError,
  layoutBarcode1D,
  runsFromWidthPattern,
  type Barcode1DRenderConfig,
  type Barcode1DRun,
  type PaintBarcode1DOptions,
} from "@coding-adventures/barcode-layout-1d";
import type { PaintScene } from "@coding-adventures/paint-instructions";

export const VERSION = "0.1.0";

export interface EncodedCharacter {
  char: string;
  isStartStop: boolean;
  pattern: string;
}

export type BarcodeRun = Barcode1DRun;
export type RunColor = BarcodeRun["color"];
export type RenderConfig = Barcode1DRenderConfig;
export type LayoutCode39Options = Omit<PaintBarcode1DOptions, "symbols" | "humanReadableText" | "label" | "metadata"> & {
  metadata?: Record<string, string | number | boolean>;
  label?: string;
};

export const DEFAULT_RENDER_CONFIG: RenderConfig = DEFAULT_BARCODE_1D_RENDER_CONFIG;
export { Barcode1DError as BarcodeError, InvalidBarcode1DConfigurationError as InvalidConfigurationError };

const CODE39_BAR_SPACE_PATTERNS: Record<string, string> = {
  "0": "bwbWBwBwb",
  "1": "BwbWbwbwB",
  "2": "bwBWbwbwB",
  "3": "BwBWbwbwb",
  "4": "bwbWBwbwB",
  "5": "BwbWBwbwb",
  "6": "bwBWBwbwb",
  "7": "bwbWbwBwB",
  "8": "BwbWbwBwb",
  "9": "bwBWbwBwb",
  A: "BwbwbWbwB",
  B: "bwBwbWbwB",
  C: "BwBwbWbwb",
  D: "bwbwBWbwB",
  E: "BwbwBWbwb",
  F: "bwBwBWbwb",
  G: "bwbwbWBwB",
  H: "BwbwbWBwb",
  I: "bwBwbWBwb",
  J: "bwbwBWBwb",
  K: "BwbwbwbWB",
  L: "bwBwbwbWB",
  M: "BwBwbwbWb",
  N: "bwbwBwbWB",
  O: "BwbwBwbWb",
  P: "bwBwBwbWb",
  Q: "bwbwbwBWB",
  R: "BwbwbwBWb",
  S: "bwBwbwBWb",
  T: "bwbwBwBWb",
  U: "BWbwbwbwB",
  V: "bWBwbwbwB",
  W: "BWBwbwbwb",
  X: "bWbwBwbwB",
  Y: "BWbwBwbwb",
  Z: "bWBwBwbwb",
  "-": "bWbwbwBwB",
  ".": "BWbwbwBwb",
  " ": "bWBwbwBwb",
  $: "bWbWbWbwb",
  "/": "bWbWbwbWb",
  "+": "bWbwbWbWb",
  "%": "bwbWbWbWb",
  "*": "bWbwBwBwb",
};

function widthPatternFromBarSpacePattern(barSpacePattern: string): string {
  return barSpacePattern
    .split("")
    .map((element) => (element === element.toUpperCase() ? "W" : "N"))
    .join("");
}

export class InvalidCharacterError extends Barcode1DError {
  constructor(message: string) {
    super(message);
    this.name = "InvalidCharacterError";
  }
}

function roleForEncodedChar(encodedChar: EncodedCharacter, sourceIndex: number, encodedLength: number): BarcodeRun["role"] {
  if (!encodedChar.isStartStop) {
    return "data";
  }

  if (sourceIndex === 0) {
    return "start";
  }

  if (sourceIndex === encodedLength - 1) {
    return "stop";
  }

  return "guard";
}

export function normalizeCode39(data: string): string {
  const normalized = data.toUpperCase();

  for (const char of normalized) {
    if (char === "*") {
      throw new InvalidCharacterError('Input must not contain "*" because it is reserved for start/stop');
    }

    if (!(char in CODE39_BAR_SPACE_PATTERNS)) {
      throw new InvalidCharacterError(`Invalid character: "${char}" is not supported by Code 39`);
    }
  }

  return normalized;
}

export function encodeCode39Char(char: string): EncodedCharacter {
  if (!(char in CODE39_BAR_SPACE_PATTERNS)) {
    throw new InvalidCharacterError(`Invalid character: "${char}" is not supported by Code 39`);
  }

  return {
    char,
    isStartStop: char === "*",
    pattern: widthPatternFromBarSpacePattern(CODE39_BAR_SPACE_PATTERNS[char]),
  };
}

export function encodeCode39(data: string): EncodedCharacter[] {
  const normalized = normalizeCode39(data);
  return `*${normalized}*`.split("").map((char) => encodeCode39Char(char));
}

export function expandCode39Runs(data: string): BarcodeRun[] {
  const encoded = encodeCode39(data);
  const runs: BarcodeRun[] = [];

  encoded.forEach((encodedChar, sourceIndex) => {
    runs.push(
      ...runsFromWidthPattern(encodedChar.pattern, {
        sourceLabel: encodedChar.char,
        sourceIndex,
        role: roleForEncodedChar(encodedChar, sourceIndex, encoded.length),
      }),
    );

    if (sourceIndex < encoded.length - 1) {
      runs.push({
        color: "space",
        modules: 1,
        sourceLabel: encodedChar.char,
        sourceIndex,
        role: "inter-character-gap",
      });
    }
  });

  return runs;
}

export function layoutCode39(
  data: string,
  options: LayoutCode39Options = {},
): PaintScene {
  const normalized = normalizeCode39(data);
  return layoutBarcode1D(expandCode39Runs(normalized), {
    ...options,
    label: options.label ?? (normalized.length === 0 ? "Code 39 barcode" : `Code 39 barcode for ${normalized}`),
    metadata: {
      ...options.metadata,
      symbology: "code39",
      encodedText: normalized,
    },
  });
}

export function drawCode39(
  data: string,
  options: LayoutCode39Options = {},
): PaintScene {
  return layoutCode39(data, options);
}
