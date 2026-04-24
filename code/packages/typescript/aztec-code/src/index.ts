/**
 * @module aztec-code
 *
 * Aztec Code encoder — ISO/IEC 24778:2008 compliant.
 *
 * Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995 and
 * published as a patent-free format. Unlike QR Code (which uses three square
 * finder patterns at three corners), Aztec Code places a single **bullseye
 * finder pattern at the center** of the symbol. The scanner finds the center
 * first, then reads outward in a spiral — no large quiet zone is needed.
 *
 * ## Where Aztec Code is used today
 *
 * - **IATA boarding passes** — the barcode on every airline boarding pass
 * - **Eurostar and Amtrak rail tickets** — printed and on-screen tickets
 * - **PostNL, Deutsche Post, La Poste** — European postal routing
 * - **US military ID cards**
 *
 * ## Symbol variants
 *
 * ```
 * Compact: 1-4 layers,  size = 11 + 4*layers  (15x15 to 27x27)
 * Full:    1-32 layers, size = 15 + 4*layers  (19x19 to 143x143)
 * ```
 *
 * ## Encoding pipeline (v0.1.0 — byte-mode only)
 *
 * ```
 * input string / bytes
 *   -> Binary-Shift codewords from Upper mode
 *   -> symbol size selection (smallest compact then full that fits at 23% ECC)
 *   -> pad to exact codeword count
 *   -> GF(256)/0x12D Reed-Solomon ECC (poly 0x12D, b=1 roots alpha^1..alpha^n)
 *   -> bit stuffing (insert complement after 4 consecutive identical bits)
 *   -> GF(16) mode message (layers + codeword count + 5 or 6 RS nibbles)
 *   -> ModuleGrid  (bullseye -> orientation marks -> mode msg -> data spiral)
 * ```
 *
 * ## v0.1.0 simplifications
 *
 * 1. Byte-mode only — all input encoded via Binary-Shift from Upper mode.
 *    Multi-mode (Digit/Upper/Lower/Mixed/Punct) optimization is v0.2.0.
 * 2. 8-bit codewords -> GF(256) RS (same polynomial as Data Matrix: 0x12D).
 *    GF(16) and GF(32) RS for 4-bit/5-bit codewords are v0.2.0.
 * 3. Default ECC = 23%.
 * 4. Auto-select compact vs full (force-compact option is v0.2.0).
 */

import {
  type ModuleGrid,
  type Barcode2DLayoutConfig,
  type PaintScene,
  type AnnotatedModuleGrid,
  layout,
} from "@coding-adventures/barcode-2d";

import { renderToSvgString } from "@coding-adventures/paint-vm-svg";

export type { ModuleGrid, Barcode2DLayoutConfig, PaintScene, AnnotatedModuleGrid };

// -----------------------------------------------------------------------------
// Package version
// -----------------------------------------------------------------------------

export const VERSION = "0.1.0";

// -----------------------------------------------------------------------------
// Public types
// -----------------------------------------------------------------------------

/**
 * Options for Aztec Code encoding.
 */
export interface AztecOptions {
  /**
   * Minimum error-correction percentage (default: 23, range: 10-90).
   */
  minEccPercent?: number;
}

/**
 * Base error class for Aztec Code failures.
 */
export class AztecError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "AztecError";
  }
}

/**
 * Thrown when the input is too long to fit in a 32-layer full Aztec symbol.
 */
export class InputTooLongError extends AztecError {
  constructor(message: string) {
    super(message);
    this.name = "InputTooLongError";
  }
}

// -----------------------------------------------------------------------------
// GF(16) arithmetic — for mode message Reed-Solomon
// -----------------------------------------------------------------------------
//
// GF(16) is the finite field with 16 elements, built from the primitive
// polynomial:
//
//   p(x) = x^4 + x + 1   (binary: 10011 = 0x13)
//
// Every non-zero element can be written as a power of the primitive element
// alpha. alpha is the root of p(x), so alpha^4 = alpha + 1.
//
// The log table maps a field element (1..15) to its discrete log (0..14).
// The antilog (exponentiation) table maps a log value to its element.
//
// alpha^0=1, alpha^1=2, alpha^2=4, alpha^3=8,
// alpha^4=3, alpha^5=6, alpha^6=12, alpha^7=11,
// alpha^8=5, alpha^9=10, alpha^10=7, alpha^11=14,
// alpha^12=15, alpha^13=13, alpha^14=9, alpha^15=1 (period=15)

/** GF(16) discrete logarithm: LOG16[e] = i means alpha^i = e. */
const LOG16: ReadonlyArray<number> = [
  -1,  // log(0) = undefined
   0,  // log(1) = 0
   1,  // log(2) = 1
   4,  // log(3) = 4
   2,  // log(4) = 2
   8,  // log(5) = 8
   5,  // log(6) = 5
  10,  // log(7) = 10
   3,  // log(8) = 3
  14,  // log(9) = 14
   9,  // log(10) = 9
   7,  // log(11) = 7
   6,  // log(12) = 6
  13,  // log(13) = 13
  11,  // log(14) = 11
  12,  // log(15) = 12
];

/** GF(16) antilogarithm: ALOG16[i] = alpha^i. */
const ALOG16: ReadonlyArray<number> = [
   1, 2, 4, 8, 3, 6, 12, 11, 5, 10, 7, 14, 15, 13, 9, 1,
];

/**
 * Multiply two GF(16) elements.
 *
 * Uses log/antilog: a*b = ALOG16[(LOG16[a] + LOG16[b]) mod 15].
 * Returns 0 if either operand is 0.
 */
function gf16Mul(a: number, b: number): number {
  if (a === 0 || b === 0) return 0;
  return ALOG16[((LOG16[a] as number) + (LOG16[b] as number)) % 15] as number;
}

/**
 * Build the GF(16) RS generator polynomial with roots alpha^1 through alpha^n.
 *
 * Returns [g_0, g_1, ..., g_n] where g_n = 1 (monic).
 */
function buildGf16Generator(n: number): number[] {
  let g: number[] = [1];
  for (let i = 1; i <= n; i++) {
    const ai = ALOG16[i % 15] as number;
    const next: number[] = new Array(g.length + 1).fill(0);
    for (let j = 0; j < g.length; j++) {
      next[j + 1] ^= g[j];
      next[j] ^= gf16Mul(ai, g[j]);
    }
    g = next;
  }
  return g;
}

/**
 * Compute n GF(16) RS check nibbles for the given data nibbles.
 *
 * Uses the LFSR polynomial division algorithm.
 */
function gf16RsEncode(data: number[], n: number): number[] {
  const g = buildGf16Generator(n);
  const rem: number[] = new Array(n).fill(0);
  for (const byte of data) {
    const fb = byte ^ (rem[0] as number);
    for (let i = 0; i < n - 1; i++) {
      rem[i] = (rem[i + 1] as number) ^ gf16Mul(g[i + 1] as number, fb);
    }
    rem[n - 1] = gf16Mul(g[n] as number, fb);
  }
  return rem;
}

// -----------------------------------------------------------------------------
// GF(256)/0x12D arithmetic — for 8-bit data codewords
// -----------------------------------------------------------------------------
//
// Aztec Code uses GF(256) with primitive polynomial:
//   p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  =  0x12D
//
// This is the SAME polynomial as Data Matrix ECC200, but DIFFERENT from
// QR Code (0x11D). We implement it inline since the repo's gf256 package
// uses 0x11D.
//
// Generator convention: b=1, roots alpha^1..alpha^n (MA02 style).

const GF256_POLY = 0x12d;

/** EXP_12D[i] = alpha^i in GF(256)/0x12D, doubled for fast multiply. */
const EXP_12D = new Uint8Array(512);
/** LOG_12D[e] = discrete log of e in GF(256)/0x12D. */
const LOG_12D = new Uint8Array(256);

// Build tables at module load time. The primitive element is alpha = 2.
(function buildGf256Tables(): void {
  let x = 1;
  for (let i = 0; i < 255; i++) {
    EXP_12D[i] = x;
    EXP_12D[i + 255] = x;
    LOG_12D[x] = i;
    x = x << 1;
    if (x & 0x100) x ^= GF256_POLY;
    x &= 0xff;
  }
  EXP_12D[255] = 1;
})();

/**
 * Multiply two GF(256)/0x12D elements.
 *
 * Uses log/antilog lookup with the doubled EXP table.
 */
function gf256Mul(a: number, b: number): number {
  if (a === 0 || b === 0) return 0;
  return EXP_12D[(LOG_12D[a] as number) + (LOG_12D[b] as number)] as number;
}

/**
 * Build the GF(256)/0x12D RS generator polynomial with roots alpha^1..alpha^n.
 *
 * Returns big-endian coefficients (highest degree first).
 */
function buildGf256Generator(n: number): number[] {
  let g: number[] = [1];
  for (let i = 1; i <= n; i++) {
    const ai = EXP_12D[i] as number;
    const next: number[] = new Array(g.length + 1).fill(0);
    for (let j = 0; j < g.length; j++) {
      next[j] ^= g[j];
      next[j + 1] ^= gf256Mul(g[j], ai);
    }
    g = next;
  }
  return g;
}

/**
 * Compute n_check GF(256)/0x12D RS check bytes for the given data bytes.
 */
function gf256RsEncode(data: number[], nCheck: number): number[] {
  const g = buildGf256Generator(nCheck);
  const n = g.length - 1;
  const rem: number[] = new Array(n).fill(0);
  for (const b of data) {
    const fb = b ^ (rem[0] as number);
    for (let i = 0; i < n - 1; i++) {
      rem[i] = (rem[i + 1] as number) ^ gf256Mul(g[i + 1] as number, fb);
    }
    rem[n - 1] = gf256Mul(g[n] as number, fb);
  }
  return rem;
}

// -----------------------------------------------------------------------------
// Aztec Code capacity tables
// -----------------------------------------------------------------------------
//
// Derived from ISO/IEC 24778:2008 Table 1.
// Each entry: totalBits (total data+ECC bit positions), maxBytes8 (8-bit cw slots).

const COMPACT_CAPACITY: ReadonlyArray<{ totalBits: number; maxBytes8: number }> = [
  { totalBits: 0,   maxBytes8: 0  }, // index 0 unused
  { totalBits:  72, maxBytes8:  9 }, // 1 layer, 15x15
  { totalBits: 200, maxBytes8: 25 }, // 2 layers, 19x19
  { totalBits: 392, maxBytes8: 49 }, // 3 layers, 23x23
  { totalBits: 648, maxBytes8: 81 }, // 4 layers, 27x27
];

const FULL_CAPACITY: ReadonlyArray<{ totalBits: number; maxBytes8: number }> = [
  { totalBits: 0,      maxBytes8: 0    }, // index 0 unused
  { totalBits:    88,  maxBytes8:   11 }, //  1 layer
  { totalBits:   216,  maxBytes8:   27 }, //  2 layers
  { totalBits:   360,  maxBytes8:   45 }, //  3 layers
  { totalBits:   520,  maxBytes8:   65 }, //  4 layers
  { totalBits:   696,  maxBytes8:   87 }, //  5 layers
  { totalBits:   888,  maxBytes8:  111 }, //  6 layers
  { totalBits:  1096,  maxBytes8:  137 }, //  7 layers
  { totalBits:  1320,  maxBytes8:  165 }, //  8 layers
  { totalBits:  1560,  maxBytes8:  195 }, //  9 layers
  { totalBits:  1816,  maxBytes8:  227 }, // 10 layers
  { totalBits:  2088,  maxBytes8:  261 }, // 11 layers
  { totalBits:  2376,  maxBytes8:  297 }, // 12 layers
  { totalBits:  2680,  maxBytes8:  335 }, // 13 layers
  { totalBits:  3000,  maxBytes8:  375 }, // 14 layers
  { totalBits:  3336,  maxBytes8:  417 }, // 15 layers
  { totalBits:  3688,  maxBytes8:  461 }, // 16 layers
  { totalBits:  4056,  maxBytes8:  507 }, // 17 layers
  { totalBits:  4440,  maxBytes8:  555 }, // 18 layers
  { totalBits:  4840,  maxBytes8:  605 }, // 19 layers
  { totalBits:  5256,  maxBytes8:  657 }, // 20 layers
  { totalBits:  5688,  maxBytes8:  711 }, // 21 layers
  { totalBits:  6136,  maxBytes8:  767 }, // 22 layers
  { totalBits:  6600,  maxBytes8:  825 }, // 23 layers
  { totalBits:  7080,  maxBytes8:  885 }, // 24 layers
  { totalBits:  7576,  maxBytes8:  947 }, // 25 layers
  { totalBits:  8088,  maxBytes8: 1011 }, // 26 layers
  { totalBits:  8616,  maxBytes8: 1077 }, // 27 layers
  { totalBits:  9160,  maxBytes8: 1145 }, // 28 layers
  { totalBits:  9720,  maxBytes8: 1215 }, // 29 layers
  { totalBits: 10296,  maxBytes8: 1287 }, // 30 layers
  { totalBits: 10888,  maxBytes8: 1361 }, // 31 layers
  { totalBits: 11496,  maxBytes8: 1437 }, // 32 layers
];

// -----------------------------------------------------------------------------
// Data encoding — Binary-Shift from Upper mode (v0.1.0 byte-mode path)
// -----------------------------------------------------------------------------
//
// All input is wrapped in a single Binary-Shift block from Upper mode:
//   1. Emit 5 bits = 0b11111 (Binary-Shift escape in Upper mode)
//   2. If len <= 31: 5 bits for length
//      If len > 31:  5 bits = 0b00000, then 11 bits for length
//   3. Each byte as 8 bits, MSB first

/**
 * Encode input bytes as a flat bit array using the Binary-Shift escape.
 *
 * Returns an array of 0/1 values, MSB first.
 */
function encodeBytesAsBits(input: Uint8Array): number[] {
  const bits: number[] = [];

  const writeBits = (value: number, count: number): void => {
    for (let i = count - 1; i >= 0; i--) {
      bits.push((value >> i) & 1);
    }
  };

  const len = input.length;
  writeBits(31, 5); // Binary-Shift escape

  if (len <= 31) {
    writeBits(len, 5);
  } else {
    writeBits(0, 5);
    writeBits(len, 11);
  }

  for (const byte of input) {
    writeBits(byte, 8);
  }

  return bits;
}

// -----------------------------------------------------------------------------
// Symbol size selection
// -----------------------------------------------------------------------------

interface SymbolSpec {
  compact: boolean;
  layers: number;
  dataCwCount: number;
  eccCwCount: number;
  totalBits: number;
}

/**
 * Select the smallest symbol that can hold dataBitCount bits at minEccPct.
 *
 * Tries compact 1-4, then full 1-32. Adds 20% conservative stuffing overhead.
 *
 * @throws InputTooLongError if no symbol fits.
 */
function selectSymbol(dataBitCount: number, minEccPct: number): SymbolSpec {
  const stuffedBitCount = Math.ceil(dataBitCount * 1.2);

  for (let layers = 1; layers <= 4; layers++) {
    const cap = COMPACT_CAPACITY[layers];
    if (!cap) continue;
    const totalBytes = cap.maxBytes8;
    const eccCwCount = Math.ceil((minEccPct / 100) * totalBytes);
    const dataCwCount = totalBytes - eccCwCount;
    if (dataCwCount <= 0) continue;
    if (Math.ceil(stuffedBitCount / 8) <= dataCwCount) {
      return { compact: true, layers, dataCwCount, eccCwCount, totalBits: cap.totalBits };
    }
  }

  for (let layers = 1; layers <= 32; layers++) {
    const cap = FULL_CAPACITY[layers];
    if (!cap) continue;
    const totalBytes = cap.maxBytes8;
    const eccCwCount = Math.ceil((minEccPct / 100) * totalBytes);
    const dataCwCount = totalBytes - eccCwCount;
    if (dataCwCount <= 0) continue;
    if (Math.ceil(stuffedBitCount / 8) <= dataCwCount) {
      return { compact: false, layers, dataCwCount, eccCwCount, totalBits: cap.totalBits };
    }
  }

  throw new InputTooLongError(
    `Input is too long to fit in any Aztec Code symbol (${dataBitCount} bits needed)`
  );
}

// -----------------------------------------------------------------------------
// Padding
// -----------------------------------------------------------------------------

function padToBytes(bits: number[], targetBytes: number): number[] {
  const out = [...bits];
  while (out.length % 8 !== 0) out.push(0);
  while (out.length < targetBytes * 8) out.push(0);
  return out.slice(0, targetBytes * 8);
}

// -----------------------------------------------------------------------------
// Bit stuffing
// -----------------------------------------------------------------------------
//
// After every 4 consecutive identical bits (all 0 or all 1), insert one
// complement bit. Applies only to the data+ECC bit stream.
//
// Example:
//   Input:  1 1 1 1 0 0 0 0
//   After 4 ones: insert 0  -> [1,1,1,1,0]
//   After 4 zeros: insert 1 -> [1,1,1,1,0, 0,0,0,1,0]

/**
 * Apply Aztec bit stuffing to the data+ECC bit stream.
 *
 * Inserts a complement bit after every run of 4 identical bits.
 */
function stuffBits(bits: ReadonlyArray<number>): number[] {
  const stuffed: number[] = [];
  let runVal = -1;
  let runLen = 0;

  for (const bit of bits) {
    if (bit === runVal) {
      runLen++;
    } else {
      runVal = bit;
      runLen = 1;
    }

    stuffed.push(bit);

    if (runLen === 4) {
      const stuffBit = 1 - bit;
      stuffed.push(stuffBit);
      runVal = stuffBit;
      runLen = 1;
    }
  }

  return stuffed;
}

// -----------------------------------------------------------------------------
// Mode message encoding
// -----------------------------------------------------------------------------
//
// The mode message encodes layer count and data codeword count, protected by
// GF(16) RS.
//
// Compact (28 bits = 7 nibbles):
//   m = ((layers-1) << 6) | (dataCwCount-1)
//   2 data nibbles + 5 ECC nibbles
//
// Full (40 bits = 10 nibbles):
//   m = ((layers-1) << 11) | (dataCwCount-1)
//   4 data nibbles + 6 ECC nibbles

/**
 * Encode the mode message as a flat bit array (28 bits compact, 40 bits full).
 */
function encodeModeMessage(compact: boolean, layers: number, dataCwCount: number): number[] {
  let dataNibbles: number[];
  let numEcc: number;

  if (compact) {
    const m = ((layers - 1) << 6) | (dataCwCount - 1);
    dataNibbles = [m & 0xf, (m >> 4) & 0xf];
    numEcc = 5;
  } else {
    const m = ((layers - 1) << 11) | (dataCwCount - 1);
    dataNibbles = [m & 0xf, (m >> 4) & 0xf, (m >> 8) & 0xf, (m >> 12) & 0xf];
    numEcc = 6;
  }

  const eccNibbles = gf16RsEncode(dataNibbles, numEcc);
  const allNibbles = [...dataNibbles, ...eccNibbles];

  const bits: number[] = [];
  for (const nibble of allNibbles) {
    for (let i = 3; i >= 0; i--) {
      bits.push((nibble >> i) & 1);
    }
  }

  return bits;
}

// -----------------------------------------------------------------------------
// Grid construction
// -----------------------------------------------------------------------------

/** Symbol size: compact = 11+4*layers, full = 15+4*layers. */
function symbolSize(compact: boolean, layers: number): number {
  return compact ? 11 + 4 * layers : 15 + 4 * layers;
}

/** Bullseye radius: compact = 5, full = 7. */
function bullseyeRadius(compact: boolean): number {
  return compact ? 5 : 7;
}

/**
 * Draw the bullseye finder pattern.
 *
 * Color at Chebyshev distance d from center:
 *   d <= 1: DARK  (solid 3x3 inner core)
 *   d > 1, d even: LIGHT
 *   d > 1, d odd:  DARK
 */
function drawBullseye(
  modules: boolean[][],
  reserved: boolean[][],
  cx: number,
  cy: number,
  compact: boolean
): void {
  const br = bullseyeRadius(compact);
  for (let row = cy - br; row <= cy + br; row++) {
    for (let col = cx - br; col <= cx + br; col++) {
      const d = Math.max(Math.abs(col - cx), Math.abs(row - cy));
      const dark = d <= 1 ? true : d % 2 === 1;
      modules[row][col] = dark;
      reserved[row][col] = true;
    }
  }
}

/**
 * Draw reference grid for full Aztec symbols.
 *
 * Grid lines at rows/cols that are multiples of 16 from center.
 * Module value alternates dark/light from center.
 */
function drawReferenceGrid(
  modules: boolean[][],
  reserved: boolean[][],
  cx: number,
  cy: number,
  size: number
): void {
  for (let row = 0; row < size; row++) {
    for (let col = 0; col < size; col++) {
      const onH = (cy - row) % 16 === 0;
      const onV = (cx - col) % 16 === 0;
      if (!onH && !onV) continue;

      let dark: boolean;
      if (onH && onV) {
        dark = true;
      } else if (onH) {
        dark = (cx - col) % 2 === 0;
      } else {
        dark = (cy - row) % 2 === 0;
      }

      modules[row][col] = dark;
      reserved[row][col] = true;
    }
  }
}

/**
 * Place orientation marks and mode message bits.
 *
 * The mode message ring is the perimeter at Chebyshev radius (bullseyeRadius+1).
 * The 4 corners are orientation marks (DARK). The remaining non-corner positions
 * carry mode message bits clockwise from TL+1.
 *
 * Returns positions in the ring after the mode message bits (for data bits).
 */
function drawOrientationAndModeMessage(
  modules: boolean[][],
  reserved: boolean[][],
  cx: number,
  cy: number,
  compact: boolean,
  modeMessageBits: number[]
): Array<[number, number]> {
  const r = bullseyeRadius(compact) + 1;

  // Enumerate non-corner perimeter positions clockwise from TL+1.
  const nonCorner: Array<[number, number]> = [];

  // Top edge (skip both corners)
  for (let col = cx - r + 1; col <= cx + r - 1; col++) {
    nonCorner.push([col, cy - r]);
  }
  // Right edge (skip both corners)
  for (let row = cy - r + 1; row <= cy + r - 1; row++) {
    nonCorner.push([cx + r, row]);
  }
  // Bottom edge: right to left (skip both corners)
  for (let col = cx + r - 1; col >= cx - r + 1; col--) {
    nonCorner.push([col, cy + r]);
  }
  // Left edge: bottom to top (skip both corners)
  for (let row = cy + r - 1; row >= cy - r + 1; row--) {
    nonCorner.push([cx - r, row]);
  }

  // Place 4 orientation mark corners as DARK
  const corners: Array<[number, number]> = [
    [cx - r, cy - r],
    [cx + r, cy - r],
    [cx + r, cy + r],
    [cx - r, cy + r],
  ];
  for (const [col, row] of corners) {
    modules[row][col] = true;
    reserved[row][col] = true;
  }

  // Place mode message bits
  for (let i = 0; i < modeMessageBits.length && i < nonCorner.length; i++) {
    const [col, row] = nonCorner[i]!;
    modules[row][col] = modeMessageBits[i] === 1;
    reserved[row][col] = true;
  }

  // Return remaining positions for data bits
  return nonCorner.slice(modeMessageBits.length);
}

// -----------------------------------------------------------------------------
// Data layer spiral placement
// -----------------------------------------------------------------------------
//
// Bits are placed in a clockwise spiral starting from the innermost data layer.
// Each layer band is 2 modules wide. Pairs: outer row/col first, then inner.
//
// For compact: d_inner of first layer = bullseyeRadius + 2 = 7
// For full:    d_inner of first layer = bullseyeRadius + 2 = 9

/**
 * Place all data bits using the clockwise layer spiral.
 *
 * Fills the mode ring remaining positions first, then spirals outward.
 */
function placeDataBits(
  modules: boolean[][],
  reserved: boolean[][],
  bits: number[],
  cx: number,
  cy: number,
  compact: boolean,
  layers: number,
  modeRingRemainingPositions: Array<[number, number]>
): void {
  const size = modules.length;
  let bitIndex = 0;

  const placeBit = (col: number, row: number): void => {
    if (row < 0 || row >= size || col < 0 || col >= size) return;
    if (reserved[row]?.[col] !== true) {
      (modules[row] as boolean[])[col] = (bits[bitIndex] ?? 0) === 1;
      bitIndex++;
    }
  };

  // Fill remaining mode ring positions first
  for (const [col, row] of modeRingRemainingPositions) {
    modules[row][col] = (bits[bitIndex] ?? 0) === 1;
    bitIndex++;
  }

  // Spiral through data layers
  const br = bullseyeRadius(compact);
  const dStart = br + 2; // mode msg ring at br+1, first data layer at br+2

  for (let L = 0; L < layers; L++) {
    const dI = dStart + 2 * L; // inner radius
    const dO = dI + 1;          // outer radius

    // Top edge: left to right
    for (let col = cx - dI + 1; col <= cx + dI; col++) {
      placeBit(col, cy - dO);
      placeBit(col, cy - dI);
    }
    // Right edge: top to bottom
    for (let row = cy - dI + 1; row <= cy + dI; row++) {
      placeBit(cx + dO, row);
      placeBit(cx + dI, row);
    }
    // Bottom edge: right to left
    for (let col = cx + dI; col >= cx - dI + 1; col--) {
      placeBit(col, cy + dO);
      placeBit(col, cy + dI);
    }
    // Left edge: bottom to top
    for (let row = cy + dI; row >= cy - dI + 1; row--) {
      placeBit(cx - dO, row);
      placeBit(cx - dI, row);
    }
  }
}

// -----------------------------------------------------------------------------
// Main encode function
// -----------------------------------------------------------------------------

/**
 * Encode data as an Aztec Code symbol.
 *
 * Returns a ModuleGrid where modules[row][col] === true means a dark module.
 * The grid origin (0,0) is the top-left corner.
 *
 * Steps:
 * 1. Encode input via Binary-Shift from Upper mode.
 * 2. Select the smallest symbol at the requested ECC level.
 * 3. Pad the data codeword sequence.
 * 4. Compute GF(256)/0x12D RS ECC.
 * 5. Apply bit stuffing.
 * 6. Compute GF(16) mode message.
 * 7. Initialize grid with structural patterns.
 * 8. Place data+ECC bits in the clockwise layer spiral.
 *
 * @throws InputTooLongError if the data exceeds max symbol capacity.
 */
export function encode(data: string | Uint8Array, options?: AztecOptions): ModuleGrid {
  const minEccPct = options?.minEccPercent ?? 23;
  const input: Uint8Array =
    typeof data === "string" ? new TextEncoder().encode(data) : data;

  // Step 1: encode data
  const dataBits = encodeBytesAsBits(input);

  // Step 2: select symbol
  const spec = selectSymbol(dataBits.length, minEccPct);
  const { compact, layers, dataCwCount, eccCwCount } = spec;

  // Step 3: pad to dataCwCount bytes
  const paddedBits = padToBytes(dataBits, dataCwCount);

  const dataBytes: number[] = [];
  for (let i = 0; i < dataCwCount; i++) {
    let byte = 0;
    for (let b = 0; b < 8; b++) {
      byte = (byte << 1) | (paddedBits[i * 8 + b] ?? 0);
    }
    // All-zero codeword avoidance: last codeword 0x00 -> 0xFF
    if (byte === 0 && i === dataCwCount - 1) byte = 0xff;
    dataBytes.push(byte);
  }

  // Step 4: compute RS ECC
  const eccBytes = gf256RsEncode(dataBytes, eccCwCount);

  // Step 5: build bit stream + stuff
  const allBytes = [...dataBytes, ...eccBytes];
  const rawBits: number[] = [];
  for (const byte of allBytes) {
    for (let i = 7; i >= 0; i--) {
      rawBits.push((byte >> i) & 1);
    }
  }
  const stuffedBits = stuffBits(rawBits);

  // Step 6: mode message
  const modeMsg = encodeModeMessage(compact, layers, dataCwCount);

  // Step 7: initialize grid
  const size = symbolSize(compact, layers);
  const cx = Math.floor(size / 2);
  const cy = Math.floor(size / 2);

  const modules: boolean[][] = Array.from({ length: size }, () =>
    new Array<boolean>(size).fill(false)
  );
  const reserved: boolean[][] = Array.from({ length: size }, () =>
    new Array<boolean>(size).fill(false)
  );

  // Reference grid first (full only), then bullseye overwrites
  if (!compact) {
    drawReferenceGrid(modules, reserved, cx, cy, size);
  }
  drawBullseye(modules, reserved, cx, cy, compact);

  const modeRingRemainingPositions = drawOrientationAndModeMessage(
    modules, reserved, cx, cy, compact, modeMsg
  );

  // Step 8: place data spiral
  placeDataBits(
    modules, reserved, stuffedBits, cx, cy, compact, layers, modeRingRemainingPositions
  );

  return {
    modules: modules.map((row) => [...row]),
    moduleShape: "square",
    rows: size,
    cols: size,
  };
}

// -----------------------------------------------------------------------------
// Convenience functions
// -----------------------------------------------------------------------------

/**
 * Encode data and convert the module grid to a PaintScene.
 */
export function encodeAndLayout(
  data: string | Uint8Array,
  options?: AztecOptions,
  config?: Barcode2DLayoutConfig
): PaintScene {
  const grid = encode(data, options);
  return layout(grid, config);
}

/**
 * Encode data and render to an SVG string.
 */
export function renderSvg(
  data: string | Uint8Array,
  options?: AztecOptions,
  config?: Barcode2DLayoutConfig
): string {
  const scene = encodeAndLayout(data, options, config);
  return renderToSvgString(scene);
}

/**
 * Encode data and return an annotated module grid.
 *
 * In v0.1.0, annotations are not fully populated; returns a plain ModuleGrid
 * cast to AnnotatedModuleGrid. Full annotation support is v0.2.0.
 */
export function explain(
  data: string | Uint8Array,
  options?: AztecOptions
): AnnotatedModuleGrid {
  const grid = encode(data, options);
  return grid as AnnotatedModuleGrid;
}
