/**
 * # pdf417
 *
 * PDF417 stacked linear barcode encoder — ISO/IEC 15438:2015 compliant.
 *
 * PDF417 (Portable Data File 417) was invented by Ynjiun P. Wang at Symbol
 * Technologies in 1991. The name encodes its geometry: each codeword has
 * exactly **4** bars and **4** spaces (8 elements), and every codeword
 * occupies exactly **17** modules of horizontal space.
 *
 * ## Where PDF417 is deployed
 *
 * | Application | Detail |
 * |---|---|
 * | AAMVA | North American driver's licences and government IDs |
 * | IATA BCBP | Airline boarding passes |
 * | USPS | Domestic shipping labels |
 * | US immigration | Form I-94, customs declarations |
 * | Healthcare | Patient wristbands, medication labels |
 *
 * ## Encoding pipeline
 *
 * ```
 * raw bytes
 *   → byte compaction     (codeword 924 latch + 6-bytes-to-5-codewords base-900)
 *   → length descriptor   (codeword 0 = total codewords in symbol)
 *   → RS ECC              (GF(929) Reed-Solomon, b=3 convention, α=3)
 *   → dimension selection (auto: roughly square symbol)
 *   → padding             (codeword 900 fills unused slots)
 *   → row indicators      (LRI + RRI per row, encode R/C/ECC level)
 *   → cluster table lookup (codeword → 17-module bar/space pattern)
 *   → start/stop patterns (fixed per row)
 *   → ModuleGrid          (abstract boolean grid)
 * ```
 *
 * ## v0.1.0 scope
 *
 * This release implements **byte compaction only**. Text and numeric
 * compaction are planned for v0.2.0.
 */

import type { ModuleGrid, Barcode2DLayoutConfig, PaintScene } from "@coding-adventures/barcode-2d";
import { makeModuleGrid, setModule, layout } from "@coding-adventures/barcode-2d";

import { CLUSTER_TABLES, START_PATTERN, STOP_PATTERN } from "./cluster-tables.js";

export const VERSION = "0.1.0";

// ─────────────────────────────────────────────────────────────────────────────
// Public error types
// ─────────────────────────────────────────────────────────────────────────────

/** Base class for all PDF417 encoding errors. */
export class PDF417Error extends Error {
  constructor(message: string) {
    super(message);
    this.name = "PDF417Error";
  }
}

/** Input data is too long to fit in any valid PDF417 symbol. */
export class InputTooLongError extends PDF417Error {
  constructor(message: string) {
    super(message);
    this.name = "InputTooLongError";
  }
}

/** User-supplied rows or columns are out of the valid range. */
export class InvalidDimensionsError extends PDF417Error {
  constructor(message: string) {
    super(message);
    this.name = "InvalidDimensionsError";
  }
}

/** ECC level is outside the valid range 0–8. */
export class InvalidECCLevelError extends PDF417Error {
  constructor(message: string) {
    super(message);
    this.name = "InvalidECCLevelError";
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Options
// ─────────────────────────────────────────────────────────────────────────────

/** Options controlling how the PDF417 symbol is encoded. */
export interface PDF417Options {
  /**
   * Reed-Solomon error correction level (0–8).
   * Higher levels use more ECC codewords. Default: auto-selected.
   */
  eccLevel?: number;

  /**
   * Number of data columns (1–30).
   * Default: auto-selected to produce a roughly square symbol.
   */
  columns?: number;

  /**
   * Module-rows per logical PDF417 row (1–10).
   * Larger values produce taller symbols. Default: 3.
   */
  rowHeight?: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/** GF(929) prime modulus. */
const GF929_PRIME = 929;

/** Generator element α = 3 (primitive root mod 929). */
const GF929_ALPHA = 3;

/** Multiplicative group order = PRIME - 1 = 928. */
const GF929_ORDER = 928;

/** Latch-to-byte-compaction codeword (alternate form, any length). */
const LATCH_BYTE = 924;

/** Padding codeword (neutral filler). */
const PADDING_CW = 900;

const MIN_ROWS = 3;
const MAX_ROWS = 90;
const MIN_COLS = 1;
const MAX_COLS = 30;

// ─────────────────────────────────────────────────────────────────────────────
// GF(929) arithmetic
// ─────────────────────────────────────────────────────────────────────────────
//
// GF(929) is the integers modulo 929. Since 929 is prime every non-zero
// element has a multiplicative inverse. We use log/antilog lookup tables for
// O(1) multiplication, built once at module load time.
//
// The tables take ~3.7 KB total (929 entries × 2 bytes × 2 arrays) and are
// built in ~0.1 ms — negligible compared to any barcode render time.

const GF_EXP: number[] = new Array<number>(929).fill(0);
const GF_LOG: number[] = new Array<number>(929).fill(0);

(function buildGFTables(): void {
  // GF_EXP[i] = α^i mod 929   (α = 3)
  // GF_LOG[v] = i  such that α^i = v
  let val = 1;
  for (let i = 0; i < GF929_ORDER; i++) {
    GF_EXP[i] = val;
    GF_LOG[val] = i;
    val = (val * GF929_ALPHA) % GF929_PRIME;
  }
  // GF_EXP[928] = GF_EXP[0] = 1  for wrap-around convenience in gfMul.
  GF_EXP[GF929_ORDER] = GF_EXP[0];
})();

/** GF(929) multiply using log/antilog tables. Returns 0 if either operand is 0. */
function gfMul(a: number, b: number): number {
  if (a === 0 || b === 0) return 0;
  return GF_EXP[(GF_LOG[a] + GF_LOG[b]) % GF929_ORDER];
}

/** GF(929) add: (a + b) mod 929. */
function gfAdd(a: number, b: number): number {
  return (a + b) % GF929_PRIME;
}

// ─────────────────────────────────────────────────────────────────────────────
// Reed-Solomon generator polynomial
// ─────────────────────────────────────────────────────────────────────────────
//
// For ECC level L, k = 2^(L+1) ECC codewords. The generator polynomial uses
// the b=3 convention: roots are α^3, α^4, ..., α^{k+2}.
//
//   g(x) = (x − α^3)(x − α^4) ··· (x − α^{k+2})
//
// We build g iteratively by multiplying in each linear factor (x − α^j).

/**
 * Build the RS generator polynomial for ECC level `eccLevel`.
 *
 * Returns k+1 coefficients [g_k, g_{k-1}, ..., g_1, g_0] where
 * k = 2^(eccLevel+1) and g_k = 1 (leading coefficient).
 */
function buildGenerator(eccLevel: number): number[] {
  const k = 1 << (eccLevel + 1); // 2^(eccLevel+1)
  let g: number[] = [1];

  for (let j = 3; j <= k + 2; j++) {
    const root = GF_EXP[j % GF929_ORDER]; // α^j
    const negRoot = GF929_PRIME - root;   // −α^j in GF(929)

    const newG: number[] = new Array<number>(g.length + 1).fill(0);
    for (let i = 0; i < g.length; i++) {
      newG[i] = gfAdd(newG[i], g[i]);
      newG[i + 1] = gfAdd(newG[i + 1], gfMul(g[i], negRoot));
    }
    g = newG;
  }

  return g;
}

// ─────────────────────────────────────────────────────────────────────────────
// Reed-Solomon encoder
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Compute `k` RS ECC codewords for `data` over GF(929) with b=3 convention.
 *
 * Uses the standard shift-register (LFSR) polynomial long-division algorithm.
 * No interleaving — all data feeds a single RS encoder (simpler than QR Code).
 */
function rsEncode(data: number[], eccLevel: number): number[] {
  const g = buildGenerator(eccLevel);
  const k = g.length - 1;
  const ecc: number[] = new Array<number>(k).fill(0);

  for (const d of data) {
    const feedback = gfAdd(d, ecc[0]);
    // Shift register left.
    for (let i = 0; i < k - 1; i++) {
      ecc[i] = ecc[i + 1];
    }
    ecc[k - 1] = 0;
    // Add feedback × generator coefficient to each cell.
    for (let i = 0; i < k; i++) {
      ecc[i] = gfAdd(ecc[i], gfMul(g[k - i], feedback));
    }
  }

  return ecc;
}

// ─────────────────────────────────────────────────────────────────────────────
// Byte compaction
// ─────────────────────────────────────────────────────────────────────────────
//
// 6 bytes → 5 codewords by treating the 6 bytes as a 48-bit big-endian integer
// and expressing it in base 900. Remaining 1–5 bytes are encoded directly.

/**
 * Encode raw bytes using byte compaction mode (codeword 924 latch).
 *
 * Returns [924, c1, c2, ...] where c_i are byte-compacted codewords.
 */
function byteCompact(bytes: Uint8Array): number[] {
  const codewords: number[] = [LATCH_BYTE];

  let i = 0;
  const len = bytes.length;

  // Process full 6-byte groups → 5 codewords each.
  // We use BigInt for the 48-bit arithmetic to avoid JS precision loss.
  while (i + 6 <= len) {
    let n = BigInt(0);
    for (let j = 0; j < 6; j++) {
      n = n * BigInt(256) + BigInt(bytes[i + j]);
    }
    // Convert n to base 900 → 5 codewords, most-significant first.
    const group: number[] = new Array<number>(5).fill(0);
    for (let j = 4; j >= 0; j--) {
      group[j] = Number(n % BigInt(900));
      n = n / BigInt(900);
    }
    codewords.push(...group);
    i += 6;
  }

  // Remaining bytes: 1 codeword per byte.
  while (i < len) {
    codewords.push(bytes[i]);
    i++;
  }

  return codewords;
}

// ─────────────────────────────────────────────────────────────────────────────
// ECC level auto-selection
// ─────────────────────────────────────────────────────────────────────────────

/** Select the minimum recommended ECC level based on data codeword count. */
function autoEccLevel(dataCount: number): number {
  if (dataCount <= 40) return 2;
  if (dataCount <= 160) return 3;
  if (dataCount <= 320) return 4;
  if (dataCount <= 863) return 5;
  return 6;
}

// ─────────────────────────────────────────────────────────────────────────────
// Dimension selection
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Choose the number of columns and rows for the symbol.
 *
 * Heuristic: c = ceil(sqrt(total / 3)), clamped to 1–30.
 * Then r = ceil(total / c), clamped to 3–90.
 */
function chooseDimensions(total: number): { cols: number; rows: number } {
  let c = Math.max(MIN_COLS, Math.min(MAX_COLS, Math.ceil(Math.sqrt(total / 3))));
  let r = Math.max(MIN_ROWS, Math.ceil(total / c));

  if (r < MIN_ROWS) {
    r = MIN_ROWS;
    c = Math.max(MIN_COLS, Math.min(MAX_COLS, Math.ceil(total / r)));
    r = Math.max(MIN_ROWS, Math.ceil(total / c));
  }

  r = Math.min(MAX_ROWS, r);
  return { cols: c, rows: r };
}

// ─────────────────────────────────────────────────────────────────────────────
// Row indicator computation
// ─────────────────────────────────────────────────────────────────────────────
//
// Each row carries two row indicator codewords that together encode:
//   R_info = (R-1) / 3      (total rows info)
//   C_info = C - 1          (columns info)
//   L_info = 3*L + (R-1)%3  (ECC level + row parity)
//
// For row r (0-indexed), cluster = r % 3:
//   Cluster 0: LRI = 30*(r/3) + R_info,  RRI = 30*(r/3) + C_info
//   Cluster 1: LRI = 30*(r/3) + L_info,  RRI = 30*(r/3) + R_info
//   Cluster 2: LRI = 30*(r/3) + C_info,  RRI = 30*(r/3) + L_info
//
// Note: the RRI formula here (Cluster 0 → C_info, Cluster 1 → R_info,
// Cluster 2 → L_info) follows the Python pdf417 library rather than the
// original spec text. The Python library produces verified scannable symbols.

/** Compute the Left Row Indicator codeword for row `r`. */
export function computeLRI(r: number, rows: number, cols: number, eccLevel: number): number {
  const rInfo = Math.floor((rows - 1) / 3);
  const cInfo = cols - 1;
  const lInfo = 3 * eccLevel + (rows - 1) % 3;
  const rowGroup = Math.floor(r / 3);
  const cluster = r % 3;

  if (cluster === 0) return 30 * rowGroup + rInfo;
  if (cluster === 1) return 30 * rowGroup + lInfo;
  return 30 * rowGroup + cInfo;
}

/** Compute the Right Row Indicator codeword for row `r`. */
export function computeRRI(r: number, rows: number, cols: number, eccLevel: number): number {
  const rInfo = Math.floor((rows - 1) / 3);
  const cInfo = cols - 1;
  const lInfo = 3 * eccLevel + (rows - 1) % 3;
  const rowGroup = Math.floor(r / 3);
  const cluster = r % 3;

  if (cluster === 0) return 30 * rowGroup + cInfo;
  if (cluster === 1) return 30 * rowGroup + rInfo;
  return 30 * rowGroup + lInfo;
}

// ─────────────────────────────────────────────────────────────────────────────
// Codeword → modules expansion
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Expand a packed bar/space pattern into 17 boolean module values.
 *
 * The 8 element widths are stored as 4 bits each in the packed u32:
 *   bits 31..28 = b1, bits 27..24 = s1, ..., bits 3..0 = s4.
 *
 * We alternate: bar (dark=true), space (dark=false), bar, space, ...
 */
function expandPattern(packed: number, modules: boolean[]): void {
  const b1 = (packed >>> 28) & 0xf;
  const s1 = (packed >>> 24) & 0xf;
  const b2 = (packed >>> 20) & 0xf;
  const s2 = (packed >>> 16) & 0xf;
  const b3 = (packed >>> 12) & 0xf;
  const s3 = (packed >>>  8) & 0xf;
  const b4 = (packed >>>  4) & 0xf;
  const s4 =  packed        & 0xf;

  for (let i = 0; i < b1; i++) modules.push(true);
  for (let i = 0; i < s1; i++) modules.push(false);
  for (let i = 0; i < b2; i++) modules.push(true);
  for (let i = 0; i < s2; i++) modules.push(false);
  for (let i = 0; i < b3; i++) modules.push(true);
  for (let i = 0; i < s3; i++) modules.push(false);
  for (let i = 0; i < b4; i++) modules.push(true);
  for (let i = 0; i < s4; i++) modules.push(false);
}

/**
 * Expand a bar/space width array into boolean module values.
 *
 * The first element is always a bar (dark = true). Each subsequent element
 * alternates between space (false) and bar (true).
 */
function expandWidths(widths: readonly number[], modules: boolean[]): void {
  let dark = true;
  for (const w of widths) {
    for (let i = 0; i < w; i++) modules.push(dark);
    dark = !dark;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main encoder: encode()
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Encode `data` bytes as a PDF417 symbol and return the `ModuleGrid`.
 *
 * @throws {InvalidECCLevelError} if options.eccLevel is not in 0–8.
 * @throws {InvalidDimensionsError} if options.columns is out of range.
 * @throws {InputTooLongError} if data exceeds the symbol's capacity.
 */
export function encode(data: Uint8Array | number[], options: PDF417Options = {}): ModuleGrid {
  const bytes = data instanceof Uint8Array ? data : new Uint8Array(data);

  // ── Validate ECC level ────────────────────────────────────────────────────
  if (options.eccLevel !== undefined && (options.eccLevel < 0 || options.eccLevel > 8)) {
    throw new InvalidECCLevelError(`ECC level must be 0–8, got ${options.eccLevel}`);
  }

  // ── Byte compaction ───────────────────────────────────────────────────────
  const dataCwords = byteCompact(bytes);

  // ── Auto-select ECC level ─────────────────────────────────────────────────
  const eccLevel = options.eccLevel ?? autoEccLevel(dataCwords.length + 1);
  const eccCount = 1 << (eccLevel + 1); // 2^(eccLevel+1)

  // ── Length descriptor ─────────────────────────────────────────────────────
  // The length descriptor is the very first codeword; it counts itself +
  // all data codewords + all ECC codewords (but NOT padding).
  const lengthDesc = 1 + dataCwords.length + eccCount;

  // Full data array for RS encoding: [lengthDesc, ...dataCwords]
  const fullData: number[] = [lengthDesc, ...dataCwords];

  // ── RS ECC ────────────────────────────────────────────────────────────────
  const eccCwords = rsEncode(fullData, eccLevel);

  // ── Choose dimensions ──────────────────────────────────────────────────────
  const totalCwords = fullData.length + eccCwords.length;

  let cols: number;
  let rows: number;

  if (options.columns !== undefined) {
    if (options.columns < MIN_COLS || options.columns > MAX_COLS) {
      throw new InvalidDimensionsError(`columns must be 1–30, got ${options.columns}`);
    }
    cols = options.columns;
    rows = Math.max(MIN_ROWS, Math.ceil(totalCwords / cols));
    if (rows > MAX_ROWS) {
      throw new InputTooLongError(
        `Data requires ${rows} rows (max 90) with ${cols} columns.`
      );
    }
  } else {
    const dims = chooseDimensions(totalCwords);
    cols = dims.cols;
    rows = dims.rows;
  }

  // Verify capacity.
  if (cols * rows < totalCwords) {
    throw new InputTooLongError(
      `Cannot fit ${totalCwords} codewords in ${rows}×${cols} grid.`
    );
  }

  // ── Pad to fill grid exactly ───────────────────────────────────────────────
  const paddingCount = cols * rows - totalCwords;
  const paddedData = [...fullData, ...Array<number>(paddingCount).fill(PADDING_CW)];

  // Full codeword sequence: [data+padding, ecc]
  const fullSequence = [...paddedData, ...eccCwords];

  // ── Rasterize ─────────────────────────────────────────────────────────────
  const rowHeight = Math.max(1, options.rowHeight ?? 3);
  return rasterize(fullSequence, rows, cols, eccLevel, rowHeight);
}

// ─────────────────────────────────────────────────────────────────────────────
// Rasterization
// ─────────────────────────────────────────────────────────────────────────────

/** Convert the flat codeword sequence to a `ModuleGrid`. */
function rasterize(
  sequence: number[],
  rows: number,
  cols: number,
  eccLevel: number,
  rowHeight: number,
): ModuleGrid {
  // Each row = start(17) + LRI(17) + data×cols(17 each) + RRI(17) + stop(18)
  const moduleWidth = 69 + 17 * cols;
  const moduleHeight = rows * rowHeight;

  let grid = makeModuleGrid(moduleHeight, moduleWidth, "square");

  // Precompute start and stop module sequences (same for every row).
  const startModules: boolean[] = [];
  expandWidths(START_PATTERN, startModules);

  const stopModules: boolean[] = [];
  expandWidths(STOP_PATTERN, stopModules);

  for (let r = 0; r < rows; r++) {
    const cluster = r % 3;
    const clusterTable = CLUSTER_TABLES[cluster];

    const rowModules: boolean[] = [];

    // 1. Start pattern (17 modules).
    rowModules.push(...startModules);

    // 2. Left Row Indicator (17 modules).
    const lri = computeLRI(r, rows, cols, eccLevel);
    expandPattern(clusterTable[lri] as number, rowModules);

    // 3. Data codewords (17 modules each).
    for (let j = 0; j < cols; j++) {
      const cw = sequence[r * cols + j];
      expandPattern(clusterTable[cw] as number, rowModules);
    }

    // 4. Right Row Indicator (17 modules).
    const rri = computeRRI(r, rows, cols, eccLevel);
    expandPattern(clusterTable[rri] as number, rowModules);

    // 5. Stop pattern (18 modules).
    rowModules.push(...stopModules);

    if (rowModules.length !== moduleWidth) {
      throw new Error(
        `Internal error: row ${r} has ${rowModules.length} modules, expected ${moduleWidth}`
      );
    }

    // Write this module row `rowHeight` times into the grid.
    const moduleRowBase = r * rowHeight;
    for (let h = 0; h < rowHeight; h++) {
      const moduleRow = moduleRowBase + h;
      for (let col = 0; col < moduleWidth; col++) {
        if (rowModules[col]) {
          grid = setModule(grid, moduleRow, col, true);
        }
      }
    }
  }

  return grid;
}

// ─────────────────────────────────────────────────────────────────────────────
// Convenience functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Encode `data` and pass the `ModuleGrid` through the layout pipeline,
 * producing a `PaintScene` ready for a render backend.
 */
export function encodeAndLayout(
  data: Uint8Array | number[],
  options: PDF417Options = {},
  config?: Partial<Barcode2DLayoutConfig>,
): PaintScene {
  const grid = encode(data, options);
  const layoutConfig: Barcode2DLayoutConfig = {
    moduleSizePx: 10,
    quietZoneModules: 2,
    foreground: "#000000",
    background: "#ffffff",
    ...config,
  };
  return layout(grid, layoutConfig);
}

// ─────────────────────────────────────────────────────────────────────────────
// Re-export GF arithmetic for testing
// ─────────────────────────────────────────────────────────────────────────────

/** Exported for testing only. Not part of the public API. */
export const _testing = {
  gfMul,
  gfAdd,
  GF_EXP,
  GF_LOG,
  byteCompact,
  rsEncode,
  buildGenerator,
  expandWidths,
  expandPattern,
  chooseDimensions,
  autoEccLevel,
};
