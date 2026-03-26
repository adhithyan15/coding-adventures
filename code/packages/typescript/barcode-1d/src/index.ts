/**
 * @coding-adventures/barcode-1d
 *
 * This package is the shared geometry layer for linear barcodes.
 *
 * Barcode symbologies still own the interesting domain rules:
 * - which inputs are valid
 * - how checksums work
 * - which symbol table to use
 * - where start/stop or guard patterns belong
 *
 * Once a symbology has answered those questions, however, most 1D formats look
 * the same: they become a left-to-right stream of bars and spaces measured in
 * modules. This package owns that seam.
 */
import {
  createScene,
  drawRect,
  drawText,
  type DrawMetadata,
  type DrawScene,
} from "@coding-adventures/draw-instructions";

export const VERSION = "0.1.0";

export type Barcode1DRunColor = "bar" | "space";
export type Barcode1DRunRole =
  | "data"
  | "start"
  | "stop"
  | "guard"
  | "check"
  | "inter-character-gap";

export interface Barcode1DRun {
  color: Barcode1DRunColor;
  modules: number;
  sourceLabel: string;
  sourceIndex: number;
  role: Barcode1DRunRole;
}

export type Barcode1DSymbolRole = Exclude<Barcode1DRunRole, "inter-character-gap">;

export interface Barcode1DSymbolLayout {
  label: string;
  startModule: number;
  endModule: number;
  sourceIndex: number;
  role: Barcode1DSymbolRole;
}

export interface Barcode1DLayout {
  leftQuietZoneModules: number;
  rightQuietZoneModules: number;
  contentModules: number;
  totalModules: number;
  symbolLayouts: Barcode1DSymbolLayout[];
}

export interface Barcode1DSymbolDescriptor {
  label: string;
  modules: number;
  sourceIndex: number;
  role: Barcode1DSymbolRole;
}

export interface Barcode1DRenderConfig {
  moduleWidth: number;
  barHeight: number;
  quietZoneModules: number;
  includeHumanReadableText: boolean;
  textFontSize: number;
  textMargin: number;
  foreground: string;
  background: string;
}

export interface DrawBarcode1DOptions {
  renderConfig?: Partial<Barcode1DRenderConfig>;
  humanReadableText?: string | null;
  metadata?: DrawMetadata;
  label?: string;
  symbols?: Barcode1DSymbolDescriptor[];
}

const TEXT_DESCENT_ALLOWANCE = 4;

export const DEFAULT_BARCODE_1D_RENDER_CONFIG: Barcode1DRenderConfig = {
  moduleWidth: 4,
  barHeight: 120,
  quietZoneModules: 10,
  includeHumanReadableText: true,
  textFontSize: 16,
  textMargin: 8,
  foreground: "#000000",
  background: "#ffffff",
};

export class Barcode1DError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "Barcode1DError";
  }
}

export class InvalidBarcode1DConfigurationError extends Barcode1DError {
  constructor(message: string) {
    super(message);
    this.name = "InvalidBarcode1DConfigurationError";
  }
}

function assertPositiveInteger(value: number, name: string): void {
  if (!Number.isInteger(value) || value <= 0) {
    throw new InvalidBarcode1DConfigurationError(`${name} must be a positive integer`);
  }
}

function validateRenderConfig(config: Barcode1DRenderConfig): void {
  assertPositiveInteger(config.moduleWidth, "moduleWidth");
  assertPositiveInteger(config.barHeight, "barHeight");
  assertPositiveInteger(config.quietZoneModules, "quietZoneModules");
  assertPositiveInteger(config.textFontSize, "textFontSize");

  if (config.textMargin < 0) {
    throw new InvalidBarcode1DConfigurationError("textMargin must be zero or greater");
  }
}

function mergeRenderConfig(
  override: Partial<Barcode1DRenderConfig> | undefined,
): Barcode1DRenderConfig {
  const config = { ...DEFAULT_BARCODE_1D_RENDER_CONFIG, ...override };
  validateRenderConfig(config);
  return config;
}

export function totalModules(runs: Barcode1DRun[]): number {
  return runs.reduce((sum, run) => sum + run.modules, 0);
}

function validateRuns(runs: Barcode1DRun[]): void {
  runs.forEach((run, index) => {
    assertPositiveInteger(run.modules, `runs[${index}].modules`);

    if (index > 0 && runs[index - 1].color === run.color) {
      throw new Barcode1DError("Runs must alternate between bars and spaces");
    }
  });
}

export function computeBarcode1DLayout(
  runs: Barcode1DRun[],
  options: { quietZoneModules?: number; symbols?: Barcode1DSymbolDescriptor[] } = {},
): Barcode1DLayout {
  validateRuns(runs);

  const quietZoneModules = options.quietZoneModules ?? DEFAULT_BARCODE_1D_RENDER_CONFIG.quietZoneModules;
  assertPositiveInteger(quietZoneModules, "quietZoneModules");

  const contentModules = totalModules(runs);
  const symbolLayouts: Barcode1DSymbolLayout[] = [];

  if (options.symbols !== undefined) {
    let cursor = 0;

    for (const symbol of options.symbols) {
      assertPositiveInteger(symbol.modules, `symbol "${symbol.label}" modules`);
      symbolLayouts.push({
        label: symbol.label,
        startModule: cursor,
        endModule: cursor + symbol.modules,
        sourceIndex: symbol.sourceIndex,
        role: symbol.role,
      });
      cursor += symbol.modules;
    }

    if (cursor !== contentModules) {
      throw new Barcode1DError("Symbol descriptors must add up to the same total width as the run stream");
    }
  } else {
    let cursor = 0;
    let currentStart = 0;
    let currentLabel: string | null = null;
    let currentSourceIndex = -1;
    let currentRole: Barcode1DSymbolRole | null = null;

    const flush = (): void => {
      if (currentLabel === null || currentRole === null) {
        return;
      }

      symbolLayouts.push({
        label: currentLabel,
        startModule: currentStart,
        endModule: cursor,
        sourceIndex: currentSourceIndex,
        role: currentRole,
      });
    };

    for (const run of runs) {
      if (run.role !== "inter-character-gap") {
        const isSameSymbol =
          currentLabel === run.sourceLabel &&
          currentSourceIndex === run.sourceIndex &&
          currentRole === run.role;

        if (!isSameSymbol) {
          flush();
          currentStart = cursor;
          currentLabel = run.sourceLabel;
          currentSourceIndex = run.sourceIndex;
          currentRole = run.role;
        }
      }

      cursor += run.modules;
    }

    flush();
  }

  return {
    leftQuietZoneModules: quietZoneModules,
    rightQuietZoneModules: quietZoneModules,
    contentModules,
    totalModules: quietZoneModules + contentModules + quietZoneModules,
    symbolLayouts,
  };
}

export interface RunsFromBinaryPatternOptions {
  sourceLabel: string;
  sourceIndex: number;
  role: Barcode1DRunRole;
}

/**
 * Convert a binary string such as `101` or `110100101` into module runs.
 *
 * The first bit is assumed to describe a bar. This matches the barcode formats
 * we currently implement.
 */
export function runsFromBinaryPattern(
  pattern: string,
  options: RunsFromBinaryPatternOptions,
): Barcode1DRun[] {
  if (!/^[01]+$/.test(pattern)) {
    throw new Barcode1DError(`Binary pattern must contain only 0 or 1, got "${pattern}"`);
  }

  const runs: Barcode1DRun[] = [];
  let currentBit = pattern[0];
  let width = 1;

  for (let index = 1; index < pattern.length; index += 1) {
    const bit = pattern[index];

    if (bit === currentBit) {
      width += 1;
      continue;
    }

    runs.push({
      color: currentBit === "1" ? "bar" : "space",
      modules: width,
      sourceLabel: options.sourceLabel,
      sourceIndex: options.sourceIndex,
      role: options.role,
    });

    currentBit = bit;
    width = 1;
  }

  runs.push({
    color: currentBit === "1" ? "bar" : "space",
    modules: width,
    sourceLabel: options.sourceLabel,
    sourceIndex: options.sourceIndex,
    role: options.role,
  });

  return runs;
}

export interface RunsFromWidthPatternOptions extends RunsFromBinaryPatternOptions {
  narrowModules?: number;
  wideModules?: number;
  narrowMarker?: string;
  wideMarker?: string;
  startingColor?: Barcode1DRunColor;
}

/**
 * Convert a symbolic width pattern such as `NWNNW` into numeric module runs.
 */
export function runsFromWidthPattern(
  pattern: string,
  options: RunsFromWidthPatternOptions,
): Barcode1DRun[] {
  const narrowModules = options.narrowModules ?? 1;
  const wideModules = options.wideModules ?? 3;
  const narrowMarker = options.narrowMarker ?? "N";
  const wideMarker = options.wideMarker ?? "W";
  const startingColor = options.startingColor ?? "bar";

  assertPositiveInteger(narrowModules, "narrowModules");
  assertPositiveInteger(wideModules, "wideModules");

  const runs: Barcode1DRun[] = [];
  let color: Barcode1DRunColor = startingColor;

  for (const marker of pattern) {
    let modules: number;

    if (marker === narrowMarker) {
      modules = narrowModules;
    } else if (marker === wideMarker) {
      modules = wideModules;
    } else {
      throw new Barcode1DError(`Unknown width marker "${marker}" in pattern "${pattern}"`);
    }

    runs.push({
      color,
      modules,
      sourceLabel: options.sourceLabel,
      sourceIndex: options.sourceIndex,
      role: options.role,
    });

    color = color === "bar" ? "space" : "bar";
  }

  return runs;
}

export function drawBarcode1D(
  runs: Barcode1DRun[],
  options: DrawBarcode1DOptions = {},
): DrawScene {
  const config = mergeRenderConfig(options.renderConfig);
  const layout = computeBarcode1DLayout(runs, {
    quietZoneModules: config.quietZoneModules,
    symbols: options.symbols,
  });
  const instructions = [];
  let moduleCursor = layout.leftQuietZoneModules;

  for (const run of runs) {
    const x = moduleCursor * config.moduleWidth;
    const width = run.modules * config.moduleWidth;

    if (run.color === "bar") {
      instructions.push(
        drawRect(x, 0, width, config.barHeight, config.foreground, {
          sourceLabel: run.sourceLabel,
          sourceIndex: run.sourceIndex,
          role: run.role,
          moduleStart: moduleCursor,
          moduleEnd: moduleCursor + run.modules,
        }),
      );
    }

    moduleCursor += run.modules;
  }

  const hasText =
    config.includeHumanReadableText && options.humanReadableText !== undefined && options.humanReadableText !== null;

  const textBlockHeight = hasText
    ? config.textMargin + config.textFontSize + TEXT_DESCENT_ALLOWANCE
    : 0;

  if (hasText) {
    instructions.push(
      drawText(
        (layout.totalModules * config.moduleWidth) / 2,
        config.barHeight + config.textMargin + config.textFontSize - 2,
        options.humanReadableText ?? "",
        {
          fontSize: config.textFontSize,
          fill: config.foreground,
          metadata: { role: "human-readable-text" },
        },
      ),
    );
  }

  return createScene(
    layout.totalModules * config.moduleWidth,
    config.barHeight + textBlockHeight,
    instructions,
    {
      background: config.background,
      metadata: {
        ...options.metadata,
        label: options.label ?? "1D barcode",
        leftQuietZoneModules: layout.leftQuietZoneModules,
        rightQuietZoneModules: layout.rightQuietZoneModules,
        contentModules: layout.contentModules,
        totalModules: layout.totalModules,
      },
    },
  );
}
