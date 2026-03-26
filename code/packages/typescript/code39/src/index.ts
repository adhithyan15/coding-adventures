/**
 * @coding-adventures/code39
 *
 * This package implements Code 39 from first principles and stops at a
 * backend-neutral draw scene.
 *
 * The pipeline is intentionally explicit:
 *
 *   user input
 *     -> normalize into Code 39 standard mode
 *     -> encode each character to a narrow/wide pattern
 *     -> expand patterns into bar/space runs
 *     -> translate runs into generic draw instructions
 *     -> hand the scene to some renderer
 *
 * Keeping these stages separate makes the code easier to learn from and easier
 * to visualize later.
 */
import {
  createScene,
  drawRect,
  drawText,
  type DrawRenderer,
  type DrawScene,
} from "@coding-adventures/draw-instructions";

export const VERSION = "0.1.0";

/** Code 39 is a 1D symbology, so every run is either a black bar or white space. */
export type RunColor = "bar" | "space";
export type RunWidth = "narrow" | "wide";

/** One encoded symbol, including the inserted start/stop characters. */
export interface EncodedCharacter {
  char: string;
  isStartStop: boolean;
  pattern: string;
}

/**
 * One run in the left-to-right barcode stream.
 *
 * This is the most useful intermediate structure for a teaching visualizer.
 * It lets us answer questions such as:
 * - which source character produced this run?
 * - is this run a bar or a space?
 * - is it narrow or wide?
 * - is this the inter-character gap?
 */
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

/** Reasonable defaults for an on-screen educational rendering. */
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

/**
 * Canonical Code 39 reference table.
 *
 * We store the traditional bar/space encoding using:
 * - uppercase letters = wide element
 * - lowercase letters = narrow element
 *
 * The pattern always starts with a bar and alternates bar/space for 9
 * elements total.
 */
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

/** Convert the mixed-case encoding into a width-only string like `WNNNNWNNW`. */
function widthPatternFromBarSpacePattern(barSpacePattern: string): string {
  return barSpacePattern
    .split("")
    .map((element) => (element === element.toUpperCase() ? "W" : "N"))
    .join("");
}

/** Rendering config values are geometry parameters, so they must be positive integers. */
function assertPositiveInteger(value: number, name: string): void {
  if (!Number.isInteger(value) || value <= 0) {
    throw new InvalidConfigurationError(`${name} must be a positive integer`);
  }
}

/** Validate the geometry contract before we start placing bars. */
function validateRenderConfig(config: RenderConfig): void {
  assertPositiveInteger(config.narrowUnit, "narrowUnit");
  assertPositiveInteger(config.wideUnit, "wideUnit");
  assertPositiveInteger(config.barHeight, "barHeight");
  assertPositiveInteger(config.quietZoneUnits, "quietZoneUnits");

  if (config.wideUnit <= config.narrowUnit) {
    throw new InvalidConfigurationError("wideUnit must be greater than narrowUnit");
  }
}

/** Turn symbolic width classes into concrete scene units. */
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

/**
 * Normalize user input into standard Code 39 mode.
 *
 * Standard Code 39 only supports uppercase letters, so lowercase input is
 * promoted rather than rejected. The asterisk is rejected because it is
 * reserved for the start/stop symbol.
 */
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

/** Encode a single supported character to its 9-element width pattern. */
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

/** Wrap user data with start/stop markers and encode the full symbol sequence. */
export function encodeCode39(data: string): EncodedCharacter[] {
  const normalized = normalizeCode39(data);
  return `*${normalized}*`.split("").map((char) => encodeCode39Char(char));
}

/**
 * Expand each encoded symbol into alternating bar/space runs.
 *
 * Why keep runs separate from draw instructions?
 * Because runs are still barcode-domain data. They are the bridge between
 * symbology rules and generic geometry. That makes them a perfect place for
 * debugging and visualization.
 */
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

/**
 * Translate a 1D run stream into generic draw instructions.
 *
 * This is the abstraction boundary:
 * - before this point, we still speak in barcode terms
 * - after this point, everything is generic scene geometry
 *
 * Only black bars become rectangles. White spaces are represented implicitly by
 * advancing the x cursor.
 */
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

  // Human-readable text is not part of the barcode signal itself, but it is
  // part of many rendered barcode labels and is useful in visualizations.
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

/** Full convenience pipeline: input string -> generic draw scene. */
export function drawCode39(
  data: string,
  config: RenderConfig = DEFAULT_RENDER_CONFIG,
): DrawScene {
  const normalized = normalizeCode39(data);
  return drawOneDimensionalBarcode(expandCode39Runs(normalized), normalized, config);
}

/** Delegate final output generation to an external renderer package. */
export function renderCode39<Output>(
  data: string,
  renderer: DrawRenderer<Output>,
  config: RenderConfig = DEFAULT_RENDER_CONFIG,
): Output {
  return renderer.render(drawCode39(data, config));
}
