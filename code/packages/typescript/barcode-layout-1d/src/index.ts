/**
 * @coding-adventures/barcode-layout-1d
 *
 * Shared geometry layer for linear barcodes.
 *
 * Barcode symbologies still own the interesting domain rules:
 * - which inputs are valid
 * - how checksums work
 * - which symbol table to use
 * - where start/stop or guard patterns belong
 *
 * Once a symbology has answered those questions, most 1D formats reduce to a
 * left-to-right stream of bars and spaces measured in modules. This package
 * owns the layout step from those runs into a rect-only PaintScene.
 */
import {
  paintRect,
  paintScene,
  type PaintInstruction,
  type PaintScene,
} from "@coding-adventures/paint-instructions";

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

export interface PaintBarcode1DOptions {
  renderConfig?: Partial<Barcode1DRenderConfig>;
  humanReadableText?: string | null;
  metadata?: Record<string, string | number | boolean>;
  label?: string;
  symbols?: Barcode1DSymbolDescriptor[];
}

export type DrawBarcode1DOptions = PaintBarcode1DOptions;
export type LayoutBarcode1DOptions = PaintBarcode1DOptions;

export const DEFAULT_BARCODE_1D_RENDER_CONFIG: Barcode1DRenderConfig = {
  moduleWidth: 4,
  barHeight: 120,
  quietZoneModules: 10,
  includeHumanReadableText: false,
  textFontSize: 16,
  textMargin: 8,
  foreground: "#000000",
  background: "#ffffff",
};

export class Barcode1DError extends Error {
  constructor(public readonly errorId: string) {
    super(errorId);
    this.name = "Barcode1DError";
  }
}

export class InvalidBarcode1DConfigurationError extends Barcode1DError {
  constructor(errorId: string) {
    super(errorId);
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

  if (config.includeHumanReadableText) {
    throw new InvalidBarcode1DConfigurationError(
      "Human-readable text is disabled for barcode-layout-1d until text metrics and glyph shaping are finished",
    );
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

export function layoutBarcode1D(
  runs: Barcode1DRun[],
  options: LayoutBarcode1DOptions = {},
): PaintScene {
  const config = mergeRenderConfig(options.renderConfig);

  if (options.humanReadableText !== undefined && options.humanReadableText !== null) {
    throw new InvalidBarcode1DConfigurationError(
      "Human-readable text is disabled for barcode-layout-1d until text metrics and glyph shaping are finished",
    );
  }

  const layout = computeBarcode1DLayout(runs, {
    quietZoneModules: config.quietZoneModules,
    symbols: options.symbols,
  });
  const instructions: PaintInstruction[] = [];
  let moduleCursor = layout.leftQuietZoneModules;

  for (const run of runs) {
    const x = moduleCursor * config.moduleWidth;
    const width = run.modules * config.moduleWidth;

    if (run.color === "bar") {
      instructions.push(
        paintRect(x, 0, width, config.barHeight, {
          fill: config.foreground,
          metadata: {
            sourceLabel: run.sourceLabel,
            sourceIndex: run.sourceIndex,
            role: run.role,
            moduleStart: moduleCursor,
            moduleEnd: moduleCursor + run.modules,
          },
        }),
      );
    }

    moduleCursor += run.modules;
  }

  return paintScene(
    layout.totalModules * config.moduleWidth,
    config.barHeight,
    config.background,
    instructions,
    {
      metadata: {
        ...options.metadata,
        label: options.label ?? "1D barcode",
        leftQuietZoneModules: layout.leftQuietZoneModules,
        rightQuietZoneModules: layout.rightQuietZoneModules,
        contentModules: layout.contentModules,
        totalModules: layout.totalModules,
        moduleWidthPx: config.moduleWidth,
        barHeightPx: config.barHeight,
      },
    },
  );
}

export function drawBarcode1D(
  runs: Barcode1DRun[],
  options: DrawBarcode1DOptions = {},
): PaintScene {
  return layoutBarcode1D(runs, options);
}

const V1_LIMITS = {
  patternScalars: 65_567,
  runs: 40_979,
  contentModules: 65_567,
  quietZoneModules: 4_096,
  symbols: 40_979,
  labelScalars: 4_096,
  metadataEntries: 64,
  metadataKeyScalars: 128,
  metadataValueScalars: 4_096,
  metadataUtf8Bytes: 65_536,
  renderDimension: 8_192,
  colorScalars: 128,
} as const;

const V1_RUN_COLORS = new Set<string>(["bar", "space"]);
const V1_RUN_ROLES = new Set<string>([
  "data",
  "start",
  "stop",
  "guard",
  "check",
  "inter-character-gap",
]);
const V1_SYMBOL_ROLES = new Set<string>(["data", "start", "stop", "guard", "check"]);

function validRunColorV1(value: unknown): value is Barcode1DRunColor {
  return typeof value === "string" && V1_RUN_COLORS.has(value);
}

function validRunRoleV1(value: unknown): value is Barcode1DRunRole {
  return typeof value === "string" && V1_RUN_ROLES.has(value);
}

function validSymbolRoleV1(value: unknown): value is Barcode1DSymbolRole {
  return typeof value === "string" && V1_SYMBOL_ROLES.has(value);
}

function scalarLengthV1(value: string): number {
  for (let index = 0; index < value.length; index += 1) {
    const unit = value.charCodeAt(index);
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(index + 1);
      if (!(next >= 0xdc00 && next <= 0xdfff)) {
        throw new Barcode1DError("invalid-source-attribution");
      }
      index += 1;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      throw new Barcode1DError("invalid-source-attribution");
    }
  }
  return Array.from(value).length;
}

function patternScalarsV1(value: string, errorId: "invalid-binary-token" | "invalid-width-token"): number {
  try {
    return scalarLengthV1(value);
  } catch {
    throw new Barcode1DError(errorId);
  }
}

function validateSourceV1(label: string, sourceIndex: number): void {
  if (
    scalarLengthV1(label) > V1_LIMITS.labelScalars ||
    !Number.isInteger(sourceIndex) ||
    sourceIndex < -2_147_483_648 ||
    sourceIndex > 2_147_483_647
  ) {
    throw new Barcode1DError("invalid-source-attribution");
  }
}

function checkedAddV1(left: number, right: number, errorId: string): number {
  const result = left + right;
  if (!Number.isSafeInteger(result)) throw new Barcode1DError(errorId);
  return result;
}

function validateRunsV1(runs: Barcode1DRun[]): number {
  if (runs.length > V1_LIMITS.runs) throw new Barcode1DError("too-many-runs");
  let content = 0;
  runs.forEach((run, index) => {
    if (!validRunColorV1(run.color) || !validRunRoleV1(run.role)) {
      throw new Barcode1DError("invalid-source-attribution");
    }
    validateSourceV1(run.sourceLabel, run.sourceIndex);
    if (!Number.isInteger(run.modules) || run.modules <= 0) {
      throw new Barcode1DError("invalid-module-count");
    }
    if (index > 0 && runs[index - 1].color === run.color) {
      throw new Barcode1DError("non-alternating-runs");
    }
    content = checkedAddV1(content, run.modules, "content-too-wide");
    if (content > V1_LIMITS.contentModules) {
      throw new Barcode1DError("content-too-wide");
    }
  });
  return content;
}

/** Strict language-neutral v1 binary expansion entry point. */
export function expandBinaryV1(
  pattern: string,
  options: RunsFromBinaryPatternOptions,
): Barcode1DRun[] {
  const scalarCount = patternScalarsV1(pattern, "invalid-binary-token");
  if (scalarCount > V1_LIMITS.patternScalars) throw new Barcode1DError("pattern-too-long");
  if (scalarCount === 0) throw new Barcode1DError("empty-pattern");
  validateSourceV1(options.sourceLabel, options.sourceIndex);
  if (!validRunRoleV1(options.role)) throw new Barcode1DError("invalid-source-attribution");
  const tokens = Array.from(pattern);
  const runs: Barcode1DRun[] = [];
  let current = tokens[0];
  let width = 1;
  for (const token of tokens.slice(1)) {
    if (token === current) {
      width = checkedAddV1(width, 1, "content-too-wide");
      continue;
    }
    if (current !== "0" && current !== "1") throw new Barcode1DError("invalid-binary-token");
    if (runs.length === V1_LIMITS.runs) throw new Barcode1DError("too-many-runs");
    runs.push({
      color: current === "1" ? "bar" : "space",
      modules: width,
      sourceLabel: options.sourceLabel,
      sourceIndex: options.sourceIndex,
      role: options.role,
    });
    current = token;
    width = 1;
  }
  if (current !== "0" && current !== "1") throw new Barcode1DError("invalid-binary-token");
  if (runs.length === V1_LIMITS.runs) throw new Barcode1DError("too-many-runs");
  runs.push({
    color: current === "1" ? "bar" : "space",
    modules: width,
    sourceLabel: options.sourceLabel,
    sourceIndex: options.sourceIndex,
    role: options.role,
  });
  validateRunsV1(runs);
  return runs;
}

/** Strict language-neutral v1 width expansion entry point. */
export function expandWidthV1(
  pattern: string,
  options: RunsFromWidthPatternOptions,
): Barcode1DRun[] {
  const scalarCount = patternScalarsV1(pattern, "invalid-width-token");
  if (scalarCount > V1_LIMITS.patternScalars) throw new Barcode1DError("pattern-too-long");
  if (scalarCount === 0) throw new Barcode1DError("empty-pattern");
  const narrowMarker = options.narrowMarker ?? "N";
  const wideMarker = options.wideMarker ?? "W";
  let narrowMarkerLength: number;
  let wideMarkerLength: number;
  try {
    narrowMarkerLength = scalarLengthV1(narrowMarker);
    wideMarkerLength = scalarLengthV1(wideMarker);
  } catch {
    throw new Barcode1DError("invalid-marker-configuration");
  }
  if (narrowMarkerLength !== 1 || wideMarkerLength !== 1 || narrowMarker === wideMarker) {
    throw new Barcode1DError("invalid-marker-configuration");
  }
  const narrowModules = options.narrowModules ?? 1;
  const wideModules = options.wideModules ?? 3;
  if (!Number.isInteger(narrowModules) || narrowModules <= 0 || !Number.isInteger(wideModules) || wideModules <= 0) {
    throw new Barcode1DError("invalid-module-count");
  }
  validateSourceV1(options.sourceLabel, options.sourceIndex);
  if (!validRunRoleV1(options.role)) throw new Barcode1DError("invalid-source-attribution");
  if (options.startingColor !== undefined && !validRunColorV1(options.startingColor)) {
    throw new Barcode1DError("invalid-marker-configuration");
  }
  let color: Barcode1DRunColor = options.startingColor ?? "bar";
  let content = 0;
  const runs: Barcode1DRun[] = [];
  for (const marker of pattern) {
    const modules = marker === narrowMarker ? narrowModules : marker === wideMarker ? wideModules : undefined;
    if (modules === undefined) throw new Barcode1DError("invalid-width-token");
    if (runs.length === V1_LIMITS.runs) throw new Barcode1DError("too-many-runs");
    runs.push({ color, modules, sourceLabel: options.sourceLabel, sourceIndex: options.sourceIndex, role: options.role });
    content = checkedAddV1(content, modules, "content-too-wide");
    if (content > V1_LIMITS.contentModules) throw new Barcode1DError("content-too-wide");
    color = color === "bar" ? "space" : "bar";
  }
  return runs;
}

/** Strict language-neutral v1 module-space layout entry point. */
export function computeLayoutV1(
  runs: Barcode1DRun[],
  options: { quietZoneModules: number; symbols?: Barcode1DSymbolDescriptor[] },
): Barcode1DLayout {
  const contentModules = validateRunsV1(runs);
  const quiet = options.quietZoneModules;
  if (!Number.isInteger(quiet) || quiet < 1 || quiet > V1_LIMITS.quietZoneModules) {
    throw new Barcode1DError("invalid-quiet-zone");
  }
  const totalModules = checkedAddV1(checkedAddV1(quiet, contentModules, "content-too-wide"), quiet, "content-too-wide");
  const symbolLayouts: Barcode1DSymbolLayout[] = [];
  if (options.symbols !== undefined) {
    if (options.symbols.length > V1_LIMITS.symbols) throw new Barcode1DError("too-many-symbols");
    let cursor = 0;
    for (const symbol of options.symbols) {
      if (!Number.isInteger(symbol.modules) || symbol.modules <= 0) throw new Barcode1DError("invalid-module-count");
      validateSourceV1(symbol.label, symbol.sourceIndex);
      if (!validSymbolRoleV1(symbol.role)) throw new Barcode1DError("invalid-source-attribution");
      const endModule = checkedAddV1(cursor, symbol.modules, "symbol-width-mismatch");
      symbolLayouts.push({ label: symbol.label, startModule: cursor, endModule, sourceIndex: symbol.sourceIndex, role: symbol.role });
      cursor = endModule;
    }
    if (cursor !== contentModules) throw new Barcode1DError("symbol-width-mismatch");
  } else {
    let cursor = 0;
    let current: Barcode1DSymbolLayout | undefined;
    for (const run of runs) {
      if (run.role !== "inter-character-gap") {
        const same = current !== undefined && current.label === run.sourceLabel && current.sourceIndex === run.sourceIndex && current.role === run.role;
        if (!same) {
          if (current !== undefined) symbolLayouts.push(current);
          current = { label: run.sourceLabel, startModule: cursor, endModule: cursor, sourceIndex: run.sourceIndex, role: run.role };
        }
      }
      cursor = checkedAddV1(cursor, run.modules, "content-too-wide");
      if (current !== undefined) current.endModule = cursor;
    }
    if (current !== undefined) symbolLayouts.push(current);
    if (symbolLayouts.length > V1_LIMITS.symbols) throw new Barcode1DError("too-many-symbols");
  }
  return { leftQuietZoneModules: quiet, rightQuietZoneModules: quiet, contentModules, totalModules, symbolLayouts };
}

export interface PaintBarcode1DV1Options {
  renderConfig?: Partial<Barcode1DRenderConfig>;
  quietZoneModules?: number;
  humanReadableText?: string | null;
  metadata?: Record<string, string>;
  label?: string;
  symbols?: Barcode1DSymbolDescriptor[];
}

function positiveRenderIntegerV1(value: number): boolean {
  return Number.isInteger(value) && value >= 1 && value <= V1_LIMITS.renderDimension;
}

/** Strict language-neutral v1 rectangle-scene entry point. */
export function projectSceneV1(
  runs: Barcode1DRun[],
  options: PaintBarcode1DV1Options = {},
): PaintScene {
  if (options.renderConfig?.includeHumanReadableText === true || options.humanReadableText != null) {
    throw new Barcode1DError("human-readable-text-unsupported");
  }
  const config = { ...DEFAULT_BARCODE_1D_RENDER_CONFIG, ...options.renderConfig };
  if (!positiveRenderIntegerV1(config.moduleWidth) || !positiveRenderIntegerV1(config.barHeight)) {
    throw new Barcode1DError("invalid-render-config");
  }
  try {
    if (scalarLengthV1(config.foreground) > V1_LIMITS.colorScalars || scalarLengthV1(config.background) > V1_LIMITS.colorScalars) {
      throw new Barcode1DError("invalid-render-config");
    }
  } catch (error) {
    if (error instanceof Barcode1DError && error.errorId === "invalid-render-config") throw error;
    throw new Barcode1DError("invalid-render-config");
  }
  const quietZoneModules = options.quietZoneModules ?? options.renderConfig?.quietZoneModules ?? DEFAULT_BARCODE_1D_RENDER_CONFIG.quietZoneModules;
  const layout = computeLayoutV1(runs, { quietZoneModules, symbols: options.symbols });
  const metadata = options.metadata ?? {};
  if (Object.keys(metadata).length > V1_LIMITS.metadataEntries) throw new Barcode1DError("metadata-too-large");
  const label = options.label ?? "1D barcode";
  try {
    if (scalarLengthV1(label) > V1_LIMITS.labelScalars) throw new Barcode1DError("metadata-too-large");
  } catch {
    throw new Barcode1DError("metadata-too-large");
  }
  let metadataBytes = 0;
  const encoder = new TextEncoder();
  for (const [key, value] of Object.entries(metadata)) {
    try {
      if (scalarLengthV1(key) > V1_LIMITS.metadataKeyScalars || scalarLengthV1(value) > V1_LIMITS.metadataValueScalars) {
        throw new Barcode1DError("metadata-too-large");
      }
    } catch {
      throw new Barcode1DError("metadata-too-large");
    }
    metadataBytes = checkedAddV1(metadataBytes, encoder.encode(key).length + encoder.encode(value).length, "metadata-too-large");
    if (metadataBytes > V1_LIMITS.metadataUtf8Bytes) throw new Barcode1DError("metadata-too-large");
  }
  const sceneWidth = checkedAddV1(0, layout.totalModules * config.moduleWidth, "invalid-render-config");
  let moduleCursor = layout.leftQuietZoneModules;
  const instructions: PaintInstruction[] = [];
  for (const run of runs) {
    const moduleEnd = checkedAddV1(moduleCursor, run.modules, "content-too-wide");
    if (run.color === "bar") {
      instructions.push(paintRect(moduleCursor * config.moduleWidth, 0, run.modules * config.moduleWidth, config.barHeight, {
        fill: config.foreground,
        metadata: {
          sourceLabel: run.sourceLabel,
          sourceIndex: String(run.sourceIndex),
          role: run.role,
          moduleStart: String(moduleCursor),
          moduleEnd: String(moduleEnd),
        },
      }));
    }
    moduleCursor = moduleEnd;
  }
  return paintScene(sceneWidth, config.barHeight, config.background, instructions, {
    metadata: {
      ...metadata,
      label,
      leftQuietZoneModules: String(layout.leftQuietZoneModules),
      rightQuietZoneModules: String(layout.rightQuietZoneModules),
      contentModules: String(layout.contentModules),
      totalModules: String(layout.totalModules),
      moduleWidthPx: String(config.moduleWidth),
      barHeightPx: String(config.barHeight),
      sceneWidthPx: String(sceneWidth),
      sceneHeightPx: String(config.barHeight),
      symbolCount: String(layout.symbolLayouts.length),
    },
  });
}
