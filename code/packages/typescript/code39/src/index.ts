import {
  createScene,
  drawRect,
  drawText,
  type DrawRenderer,
  type DrawScene,
} from "@coding-adventures/draw-instructions";

export const VERSION = "0.1.0";

export type RunColor = "bar" | "space";
export type RunWidth = "narrow" | "wide";

export interface EncodedCharacter {
  char: string;
  isStartStop: boolean;
  pattern: string;
}

export interface BarcodeRun {
  color: RunColor;
  width: RunWidth;
  sourceChar: string;
  sourceIndex: number;
  isInterCharacterGap: boolean;
}

export interface RenderConfig {
  narrowUnit: number;
  wideUnit: number;
  barHeight: number;
  quietZoneUnits: number;
  includeHumanReadableText: boolean;
}

export const DEFAULT_RENDER_CONFIG: RenderConfig = {
  narrowUnit: 4,
  wideUnit: 12,
  barHeight: 120,
  quietZoneUnits: 10,
  includeHumanReadableText: true,
};

const TEXT_MARGIN = 8;
const TEXT_FONT_SIZE = 16;
const TEXT_BLOCK_HEIGHT = TEXT_MARGIN + TEXT_FONT_SIZE + 4;

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

function assertPositiveInteger(value: number, name: string): void {
  if (!Number.isInteger(value) || value <= 0) {
    throw new InvalidConfigurationError(`${name} must be a positive integer`);
  }
}

function validateRenderConfig(config: RenderConfig): void {
  assertPositiveInteger(config.narrowUnit, "narrowUnit");
  assertPositiveInteger(config.wideUnit, "wideUnit");
  assertPositiveInteger(config.barHeight, "barHeight");
  assertPositiveInteger(config.quietZoneUnits, "quietZoneUnits");

  if (config.wideUnit <= config.narrowUnit) {
    throw new InvalidConfigurationError("wideUnit must be greater than narrowUnit");
  }
}

function unitWidth(width: RunWidth, config: RenderConfig): number {
  return width === "wide" ? config.wideUnit : config.narrowUnit;
}

export class BarcodeError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "BarcodeError";
  }
}

export class InvalidCharacterError extends BarcodeError {
  constructor(message: string) {
    super(message);
    this.name = "InvalidCharacterError";
  }
}

export class InvalidConfigurationError extends BarcodeError {
  constructor(message: string) {
    super(message);
    this.name = "InvalidConfigurationError";
  }
}

export function normalizeCode39(data: string): string {
  const normalized = data.toUpperCase();

  for (const char of normalized) {
    if (char === "*") {
      throw new InvalidCharacterError('Input must not contain "*" because it is reserved for start/stop');
    }

    if (!(char in CODE39_BAR_SPACE_PATTERNS) || char === "*") {
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
    const colorSequence: RunColor[] = ["bar", "space", "bar", "space", "bar", "space", "bar", "space", "bar"];

    encodedChar.pattern.split("").forEach((element, elementIndex) => {
      runs.push({
        color: colorSequence[elementIndex],
        width: element === "W" ? "wide" : "narrow",
        sourceChar: encodedChar.char,
        sourceIndex,
        isInterCharacterGap: false,
      });
    });

    if (sourceIndex < encoded.length - 1) {
      runs.push({
        color: "space",
        width: "narrow",
        sourceChar: encodedChar.char,
        sourceIndex,
        isInterCharacterGap: true,
      });
    }
  });

  return runs;
}

export function drawOneDimensionalBarcode(
  runs: BarcodeRun[],
  textValue: string | null,
  config: RenderConfig = DEFAULT_RENDER_CONFIG,
): DrawScene {
  validateRenderConfig(config);

  const quietZoneWidth = config.quietZoneUnits * config.narrowUnit;
  const instructions = [];
  let cursorX = quietZoneWidth;

  for (const run of runs) {
    const width = unitWidth(run.width, config);

    if (run.color === "bar") {
      instructions.push(
        drawRect(cursorX, 0, width, config.barHeight, "#000000", {
          char: run.sourceChar,
          index: run.sourceIndex,
          interGap: run.isInterCharacterGap,
        }),
      );
    }

    cursorX += width;
  }

  if (config.includeHumanReadableText && textValue !== null) {
    instructions.push(
      drawText(
        (cursorX + quietZoneWidth) / 2,
        config.barHeight + TEXT_MARGIN + TEXT_FONT_SIZE - 2,
        textValue,
        { fontSize: TEXT_FONT_SIZE, metadata: { role: "label" } },
      ),
    );
  }

  return createScene(cursorX + quietZoneWidth, config.barHeight + (config.includeHumanReadableText ? TEXT_BLOCK_HEIGHT : 0), instructions, {
    metadata: {
      label: textValue === null ? "Code 39 barcode" : `Code 39 barcode for ${textValue}`,
      symbology: "code39",
    },
  });
}

export function drawCode39(
  data: string,
  config: RenderConfig = DEFAULT_RENDER_CONFIG,
): DrawScene {
  const normalized = normalizeCode39(data);
  return drawOneDimensionalBarcode(expandCode39Runs(normalized), normalized, config);
}

export function renderCode39<Output>(
  data: string,
  renderer: DrawRenderer<Output>,
  config: RenderConfig = DEFAULT_RENDER_CONFIG,
): Output {
  return renderer.render(drawCode39(data, config));
}
