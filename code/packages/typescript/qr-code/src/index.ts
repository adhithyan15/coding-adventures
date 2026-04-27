/**
 * @module qr-code
 *
 * QR Code encoder — ISO/IEC 18004:2015 compliant.
 *
 * QR Code (Quick Response) was invented by Masahiro Hara at Denso Wave in
 * 1994 to track automotive parts. It is now the most widely deployed 2D
 * barcode on earth.  This encoder produces valid, scannable QR Codes from
 * any UTF-8 string.
 *
 * ## Encoding pipeline
 *
 * ```
 * input string
 *   → mode selection    (numeric / alphanumeric / byte)
 *   → version selection (smallest version that fits at the chosen ECC level)
 *   → bit stream        (mode indicator + char count + data + padding)
 *   → blocks + RS ECC   (GF(256) b=0 convention, poly 0x11D)
 *   → interleave        (data CWs interleaved, then ECC CWs)
 *   → grid init         (finder, separator, timing, alignment, format, dark)
 *   → zigzag placement  (two-column snake from bottom-right corner)
 *   → mask evaluation   (8 patterns, lowest 4-rule penalty wins)
 *   → finalize          (format info + version info v7+)
 *   → ModuleGrid        (abstract boolean grid, true = dark)
 * ```
 */

import {
  type ModuleGrid,
  type Barcode2DLayoutConfig,
  type PaintScene,
  type AnnotatedModuleGrid,
  layout,
} from "@coding-adventures/barcode-2d";

import { multiply as gfMul, ALOG } from "@coding-adventures/gf256";

import { renderToSvgString } from "@coding-adventures/paint-vm-svg";

export type { ModuleGrid, Barcode2DLayoutConfig, PaintScene, AnnotatedModuleGrid };

// ─────────────────────────────────────────────────────────────────────────────
// Public types
// ─────────────────────────────────────────────────────────────────────────────

export const VERSION = "0.1.0";

/**
 * Error correction level.
 *
 * | Level | Recovery | Use case                         |
 * |-------|----------|----------------------------------|
 * | L     | ~7%      | Maximum data density             |
 * | M     | ~15%     | General-purpose (common default) |
 * | Q     | ~25%     | Moderate noise/damage expected   |
 * | H     | ~30%     | High damage risk, logo overlaid  |
 */
export type EccLevel = "L" | "M" | "Q" | "H";

export class QRCodeError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "QRCodeError";
  }
}

export class InputTooLongError extends QRCodeError {
  constructor(message: string) {
    super(message);
    this.name = "InputTooLongError";
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/**
 * 2-bit ECC level indicator placed in format information.
 * L=01, M=00, Q=11, H=10 — deliberately not alphabetical order.
 */
const ECC_INDICATOR: Record<EccLevel, number> = { L: 0b01, M: 0b00, Q: 0b11, H: 0b10 };

/** Index 0=L, 1=M, 2=Q, 3=H for table lookups. */
const ECC_IDX: Record<EccLevel, number> = { L: 0, M: 1, Q: 2, H: 3 };

// ─────────────────────────────────────────────────────────────────────────────
// ISO 18004:2015 — Capacity tables (Table 9)
// ─────────────────────────────────────────────────────────────────────────────

/**
 * ECC codewords per block, indexed [eccIdx][version].
 * Index 0 is a placeholder; versions run 1–40.
 */
const ECC_CODEWORDS_PER_BLOCK: ReadonlyArray<ReadonlyArray<number>> = [
  // L:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
  [-1,  7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28, 30, 30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
  // M:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
  [-1, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28],
  // Q:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
  [-1, 13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30, 24, 28, 28, 26, 30, 28, 30, 30, 30, 30, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
  // H:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
  [-1, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 30, 24, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
];

/** Number of error correction blocks, indexed [eccIdx][version]. */
const NUM_BLOCKS: ReadonlyArray<ReadonlyArray<number>> = [
  // L:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
  [-1,  1,  1,  1,  1,  1,  2,  2,  2,  2,  4,  4,  4,  4,  4,  6,  6,  6,  6,  7,  8,  8,  9,  9, 10, 12, 12, 12, 13, 14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25],
  // M:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
  [-1,  1,  1,  1,  2,  2,  4,  4,  4,  5,  5,  5,  8,  9,  9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21, 23, 25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49],
  // Q:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
  [-1,  1,  1,  2,  2,  4,  4,  6,  6,  8,  8,  8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27, 29, 34, 34, 35, 38, 40, 43, 45, 48, 51, 53, 56, 59, 62, 65, 68],
  // H:   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
  [-1,  1,  1,  2,  4,  4,  4,  5,  6,  8,  8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32, 35, 37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 80],
];

/**
 * Alignment pattern center coordinates, indexed by [version - 1].
 * The crossproduct of these values (excluding finder overlaps) gives all
 * alignment pattern positions.  Source: ISO 18004:2015 Annex E.
 */
const ALIGNMENT_POSITIONS: ReadonlyArray<ReadonlyArray<number>> = [
  [],                             // v1  — none
  [6, 18],                        // v2
  [6, 22],                        // v3
  [6, 26],                        // v4
  [6, 30],                        // v5
  [6, 34],                        // v6
  [6, 22, 38],                    // v7
  [6, 24, 42],                    // v8
  [6, 26, 46],                    // v9
  [6, 28, 50],                    // v10
  [6, 30, 54],                    // v11
  [6, 32, 58],                    // v12
  [6, 34, 62],                    // v13
  [6, 26, 46, 66],                // v14
  [6, 26, 48, 70],                // v15
  [6, 26, 50, 74],                // v16
  [6, 30, 54, 78],                // v17
  [6, 30, 56, 82],                // v18
  [6, 30, 58, 86],                // v19
  [6, 34, 62, 90],                // v20
  [6, 28, 50, 72, 94],            // v21
  [6, 26, 50, 74, 98],            // v22
  [6, 30, 54, 78, 102],           // v23
  [6, 28, 54, 80, 106],           // v24
  [6, 32, 58, 84, 110],           // v25
  [6, 30, 58, 86, 114],           // v26
  [6, 34, 62, 90, 118],           // v27
  [6, 26, 50, 74, 98, 122],       // v28
  [6, 30, 54, 78, 102, 126],      // v29
  [6, 26, 52, 78, 104, 130],      // v30
  [6, 30, 56, 82, 108, 134],      // v31
  [6, 34, 60, 86, 112, 138],      // v32
  [6, 30, 58, 86, 114, 142],      // v33
  [6, 34, 62, 90, 118, 146],      // v34
  [6, 30, 54, 78, 102, 126, 150], // v35
  [6, 24, 50, 76, 102, 128, 154], // v36
  [6, 28, 54, 80, 106, 132, 158], // v37
  [6, 32, 58, 84, 110, 136, 162], // v38
  [6, 26, 54, 82, 110, 138, 166], // v39
  [6, 30, 58, 86, 114, 142, 170], // v40
];

// ─────────────────────────────────────────────────────────────────────────────
// Grid geometry
// ─────────────────────────────────────────────────────────────────────────────

/** Symbol size: (4 × version + 17). V1=21, V40=177. */
function symbolSize(version: number): number {
  return 4 * version + 17;
}

/**
 * Total raw data+ECC bits available in the symbol.
 * Derived by subtracting all function module areas from the total count.
 * Formula from Nayuki's reference implementation (public domain).
 */
function numRawDataModules(version: number): number {
  let result = (16 * version + 128) * version + 64;
  if (version >= 2) {
    const numAlign = Math.floor(version / 7) + 2;
    result -= (25 * numAlign - 10) * numAlign - 55;
    if (version >= 7) result -= 36;
  }
  return result;
}

/** Total data codewords (message + padding, no ECC). */
function numDataCodewords(version: number, ecc: EccLevel): number {
  const e = ECC_IDX[ecc];
  return (
    Math.floor(numRawDataModules(version) / 8) -
    NUM_BLOCKS[e][version] * ECC_CODEWORDS_PER_BLOCK[e][version]
  );
}

/** Remainder bits appended after interleaved codewords (0, 3, 4, or 7). */
function numRemainderBits(version: number): number {
  return numRawDataModules(version) % 8;
}

// ─────────────────────────────────────────────────────────────────────────────
// Reed-Solomon (b=0 convention)
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Compute the monic RS generator of degree n using b=0: g(x) = ∏(x + αⁱ) i=0..n-1.
 *
 * Start with g=[1], then for each i multiply by [1, αⁱ]:
 *   new[j] = old[j-1] ⊕ (αⁱ · old[j])
 */
function buildGenerator(n: number): number[] {
  let g: number[] = [1];
  for (let i = 0; i < n; i++) {
    const ai = ALOG[i] as number;
    const next: number[] = new Array(g.length + 1).fill(0);
    for (let j = 0; j < g.length; j++) {
      next[j] ^= g[j];
      next[j + 1] ^= gfMul(g[j], ai);
    }
    g = next;
  }
  return g;
}

const GENERATORS = new Map<number, number[]>();
// Pre-build all generators used by QR tables
for (const n of [7, 10, 13, 15, 16, 17, 18, 20, 22, 24, 26, 28, 30]) {
  GENERATORS.set(n, buildGenerator(n));
}

function getGenerator(n: number): number[] {
  if (!GENERATORS.has(n)) GENERATORS.set(n, buildGenerator(n));
  return GENERATORS.get(n)!;
}

/**
 * Compute ECC bytes: R(x) = D(x)·xⁿ mod G(x) via LFSR division.
 *
 * Shift register approach:
 *   for each data byte b:
 *     feedback = b ⊕ R[0]
 *     shift R left (R[i] ← R[i+1])
 *     R[i] ^= G[i+1] · feedback   for i=0..n-1
 */
function rsEncode(data: number[], generator: number[]): number[] {
  const n = generator.length - 1;
  const rem: number[] = new Array(n).fill(0);
  for (const b of data) {
    const fb = b ^ rem[0];
    for (let i = 0; i < n - 1; i++) rem[i] = rem[i + 1];
    rem[n - 1] = 0;
    if (fb !== 0) {
      for (let i = 0; i < n; i++) rem[i] ^= gfMul(generator[i + 1], fb);
    }
  }
  return rem;
}

// ─────────────────────────────────────────────────────────────────────────────
// Data encoding modes
// ─────────────────────────────────────────────────────────────────────────────

/**
 * 45-character alphanumeric alphabet with their QR code indices (0–44).
 * Pairs encode as: (first × 45 + second) into 11 bits.
 * Trailing single character encodes into 6 bits.
 */
const ALPHANUM_CHARS = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

type EncodingMode = "numeric" | "alphanumeric" | "byte";

const MODE_INDICATOR: Record<EncodingMode, number> = {
  numeric: 0b0001,
  alphanumeric: 0b0010,
  byte: 0b0100,
};

/** Select the most compact mode covering the entire input. */
function selectMode(input: string): EncodingMode {
  if (/^\d*$/.test(input)) return "numeric";
  if (input.split("").every((c) => ALPHANUM_CHARS.includes(c))) return "alphanumeric";
  return "byte";
}

/** Width of the character-count field (version- and mode-dependent). */
function charCountBits(mode: EncodingMode, version: number): number {
  if (mode === "numeric") return version <= 9 ? 10 : version <= 26 ? 12 : 14;
  if (mode === "alphanumeric") return version <= 9 ? 9 : version <= 26 ? 11 : 13;
  return version <= 9 ? 8 : 16; // byte
}

/** Bit-writer: accumulates bits and flushes as bytes. */
class BitWriter {
  private bits: number[] = [];

  write(value: number, count: number): void {
    for (let i = count - 1; i >= 0; i--) this.bits.push((value >> i) & 1);
  }

  get bitLength(): number { return this.bits.length; }

  toBytes(): number[] {
    const bytes: number[] = [];
    for (let i = 0; i < this.bits.length; i += 8) {
      let byte = 0;
      for (let j = 0; j < 8; j++) byte = (byte << 1) | (this.bits[i + j] ?? 0);
      bytes.push(byte);
    }
    return bytes;
  }
}

/** Groups of 3 digits → 10 bits; pairs → 7 bits; single → 4 bits. */
function encodeNumeric(input: string, w: BitWriter): void {
  let i = 0;
  while (i + 2 < input.length) { w.write(parseInt(input.slice(i, i + 3), 10), 10); i += 3; }
  if (i + 1 < input.length) { w.write(parseInt(input.slice(i, i + 2), 10), 7); i += 2; }
  if (i < input.length) w.write(parseInt(input[i], 10), 4);
}

/** Pairs encode as (idx1 × 45 + idx2) → 11 bits; trailing single → 6 bits. */
function encodeAlphanumeric(input: string, w: BitWriter): void {
  let i = 0;
  while (i + 1 < input.length) {
    const idx0 = ALPHANUM_CHARS.indexOf(input[i]);
    const idx1 = ALPHANUM_CHARS.indexOf(input[i + 1]);
    // Guard: selectMode() must have confirmed every character is in the
    // alphanumeric set before this function is called.  indexOf returning -1
    // would silently produce a corrupt bit stream, so we fail fast here.
    if (idx0 < 0 || idx1 < 0) throw new QRCodeError(
      `encodeAlphanumeric: character not in QR alphanumeric set (precondition violated)`
    );
    w.write(idx0 * 45 + idx1, 11);
    i += 2;
  }
  if (i < input.length) {
    const idx = ALPHANUM_CHARS.indexOf(input[i]);
    if (idx < 0) throw new QRCodeError(
      `encodeAlphanumeric: character not in QR alphanumeric set (precondition violated)`
    );
    w.write(idx, 6);
  }
}

/** Each UTF-8 byte → 8 bits. */
function encodeByte(input: string, w: BitWriter): void {
  for (const b of new TextEncoder().encode(input)) w.write(b, 8);
}

/**
 * Assemble the full data codeword sequence.
 *
 * Format: [mode 4b][char count][data bits][terminator ≤4b][byte pad][0xEC/0x11 pad bytes]
 * Output is exactly numDataCodewords(version, ecc) bytes.
 */
function buildDataCodewords(input: string, version: number, ecc: EccLevel): number[] {
  const mode = selectMode(input);
  const capacity = numDataCodewords(version, ecc);
  const w = new BitWriter();

  w.write(MODE_INDICATOR[mode], 4);

  const charCount = mode === "byte"
    ? new TextEncoder().encode(input).length
    : input.length;
  w.write(charCount, charCountBits(mode, version));

  if (mode === "numeric") encodeNumeric(input, w);
  else if (mode === "alphanumeric") encodeAlphanumeric(input, w);
  else encodeByte(input, w);

  // Terminator: up to 4 zero bits
  const termLen = Math.min(4, capacity * 8 - w.bitLength);
  if (termLen > 0) w.write(0, termLen);
  // Pad to byte boundary
  const rem = w.bitLength % 8;
  if (rem !== 0) w.write(0, 8 - rem);

  // Fill remaining capacity with alternating 0xEC / 0x11
  const bytes = w.toBytes();
  let pad = 0xec;
  while (bytes.length < capacity) { bytes.push(pad); pad = pad === 0xec ? 0x11 : 0xec; }
  return bytes;
}

// ─────────────────────────────────────────────────────────────────────────────
// Block processing
// ─────────────────────────────────────────────────────────────────────────────

interface Block { data: number[]; ecc: number[]; }

function computeBlocks(data: number[], version: number, ecc: EccLevel): Block[] {
  const e = ECC_IDX[ecc];
  const totalBlocks = NUM_BLOCKS[e][version];
  const eccLen = ECC_CODEWORDS_PER_BLOCK[e][version];
  const totalData = numDataCodewords(version, ecc);
  const shortLen = Math.floor(totalData / totalBlocks);
  const numLong = totalData % totalBlocks;
  const gen = getGenerator(eccLen);
  const blocks: Block[] = [];
  let offset = 0;

  const g1Count = totalBlocks - numLong;
  for (let i = 0; i < g1Count; i++) {
    const d = data.slice(offset, offset + shortLen);
    blocks.push({ data: d, ecc: rsEncode(d, gen) });
    offset += shortLen;
  }
  for (let i = 0; i < numLong; i++) {
    const d = data.slice(offset, offset + shortLen + 1);
    blocks.push({ data: d, ecc: rsEncode(d, gen) });
    offset += shortLen + 1;
  }
  return blocks;
}

/**
 * Interleave codewords across blocks:
 *   round-robin data CWs, then round-robin ECC CWs.
 *
 * Interleaving spreads a burst error across all blocks, so each block's
 * RS decoder only loses a few codewords — well within its correction budget.
 */
function interleaveBlocks(blocks: Block[]): number[] {
  const result: number[] = [];
  const maxData = Math.max(...blocks.map((b) => b.data.length));
  const maxEcc  = Math.max(...blocks.map((b) => b.ecc.length));
  for (let i = 0; i < maxData; i++) for (const b of blocks) if (i < b.data.length) result.push(b.data[i]);
  for (let i = 0; i < maxEcc;  i++) for (const b of blocks) if (i < b.ecc.length)  result.push(b.ecc[i]);
  return result;
}

// ─────────────────────────────────────────────────────────────────────────────
// Grid construction
// ─────────────────────────────────────────────────────────────────────────────

interface WorkGrid {
  size: number;
  modules: boolean[][];   // true = dark
  reserved: boolean[][];  // true = structural (don't touch during data/mask)
}

function makeWorkGrid(size: number): WorkGrid {
  return {
    size,
    modules:  Array.from({ length: size }, () => new Array<boolean>(size).fill(false)),
    reserved: Array.from({ length: size }, () => new Array<boolean>(size).fill(false)),
  };
}

function setMod(g: WorkGrid, r: number, c: number, dark: boolean, reserve = false): void {
  g.modules[r][c] = dark;
  if (reserve) g.reserved[r][c] = true;
}

/**
 * 7×7 finder pattern centred at (topRow, topCol).
 *
 * ```
 * ■ ■ ■ ■ ■ ■ ■
 * ■ □ □ □ □ □ ■
 * ■ □ ■ ■ ■ □ ■
 * ■ □ ■ ■ ■ □ ■
 * ■ □ ■ ■ ■ □ ■
 * ■ □ □ □ □ □ ■
 * ■ ■ ■ ■ ■ ■ ■
 * ```
 *
 * The 1:1:3:1:1 dark:light ratio in every scan direction lets any decoder
 * locate and orient the symbol even under partial occlusion or rotation.
 */
function placeFinder(g: WorkGrid, topRow: number, topCol: number): void {
  for (let dr = 0; dr < 7; dr++) {
    for (let dc = 0; dc < 7; dc++) {
      const onBorder = dr === 0 || dr === 6 || dc === 0 || dc === 6;
      const inCore   = dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4;
      setMod(g, topRow + dr, topCol + dc, onBorder || inCore, true);
    }
  }
}

/**
 * 5×5 alignment pattern centred at (row, col).
 *
 * ```
 * ■ ■ ■ ■ ■
 * ■ □ □ □ ■
 * ■ □ ■ □ ■
 * ■ □ □ □ ■
 * ■ ■ ■ ■ ■
 * ```
 *
 * Appears in versions 2+ at tabulated positions, helping decoders correct
 * for perspective distortion and barrel/pincushion warping.
 */
function placeAlignment(g: WorkGrid, row: number, col: number): void {
  for (let dr = -2; dr <= 2; dr++) {
    for (let dc = -2; dc <= 2; dc++) {
      const onBorder = Math.abs(dr) === 2 || Math.abs(dc) === 2;
      const isCenter = dr === 0 && dc === 0;
      setMod(g, row + dr, col + dc, onBorder || isCenter, true);
    }
  }
}

/**
 * Place all alignment patterns for the version.
 *
 * All crossproduct pairs of ALIGNMENT_POSITIONS[version-1] are considered.
 * Any whose centre falls on an already-reserved module (finder/separator/timing)
 * is skipped — this naturally excludes the three finder-overlap positions.
 */
function placeAllAlignments(g: WorkGrid, version: number): void {
  for (const row of ALIGNMENT_POSITIONS[version - 1]) {
    for (const col of ALIGNMENT_POSITIONS[version - 1]) {
      if (g.reserved[row][col]) continue; // overlaps finder/timing
      placeAlignment(g, row, col);
    }
  }
}

/**
 * Alternating dark/light timing strips.
 * Row 6, cols 8..size-9 (horizontal).
 * Col 6, rows 8..size-9 (vertical).
 * Dark when index is even (starts and ends dark).
 */
function placeTimingStrips(g: WorkGrid): void {
  const sz = g.size;
  for (let c = 8; c <= sz - 9; c++) setMod(g, 6, c, c % 2 === 0, true);
  for (let r = 8; r <= sz - 9; r++) setMod(g, r, 6, r % 2 === 0, true);
}

/**
 * Reserve format information module positions (15 modules × 2 copies).
 * Writes placeholder (false) values so the positions are known to data
 * placement; actual bits are written after mask selection.
 *
 * Copy 1 — adjacent to top-left finder:
 *   (8, 0..5), (8, 7), (8, 8), (7, 8), (5..0, 8)
 *
 * Copy 2:
 *   (size-1..size-7, 8)  and  (8, size-8..size-1)
 */
function reserveFormatInfo(g: WorkGrid): void {
  const sz = g.size;
  // Copy 1 horizontal strip: row 8, cols 0..8 (skip col 6 = timing)
  for (let c = 0; c <= 8; c++) if (c !== 6) g.reserved[8][c] = true;
  // Copy 1 vertical strip: col 8, rows 0..8 (skip row 6 = timing)
  for (let r = 0; r <= 8; r++) if (r !== 6) g.reserved[r][8] = true;
  // Copy 2 bottom-left: col 8, rows size-7..size-1
  for (let r = sz - 7; r < sz; r++) g.reserved[r][8] = true;
  // Copy 2 top-right: row 8, cols size-8..size-1
  for (let c = sz - 8; c < sz; c++) g.reserved[8][c] = true;
}

/**
 * Compute the 15-bit format information string.
 *
 * 1. 5-bit data = [ECC level (2b)] [mask pattern (3b)]
 * 2. BCH(15,5): remainder of (data × x^10) mod G(x), G(x) = 0x537
 * 3. Concatenate data and 10-bit remainder
 * 4. XOR with 0x5412 to prevent all-zero format info
 *
 * G(x) = x^10 + x^8 + x^5 + x^4 + x^2 + x + 1 = 0x537.
 */
function computeFormatBits(ecc: EccLevel, mask: number): number {
  const data = (ECC_INDICATOR[ecc] << 3) | mask;
  let rem = data << 10;
  for (let i = 14; i >= 10; i--) {
    if ((rem >> i) & 1) rem ^= 0x537 << (i - 10);
  }
  return ((data << 10) | (rem & 0x3ff)) ^ 0x5412;
}

/**
 * Write 15-bit format information into both copy locations.
 *
 * Bit positions (bit 0 = LSB of the 15-bit string):
 *
 * Copy 1:
 *   bits 0–5  → (8, 0..5)
 *   bit  6    → (8, 7)       [skip (8,6) = timing]
 *   bit  7    → (8, 8)       [corner]
 *   bit  8    → (7, 8)       [skip (6,8) = timing]
 *   bits 9–14 → (5..0, 8)
 *
 * Copy 2:
 *   bits 0–6  → (size-1..size-7, 8)
 *   bits 7–14 → (8, size-8..size-1)
 */
function writeFormatInfo(g: WorkGrid, fmtBits: number): void {
  const sz = g.size;
  // Copy 1
  for (let i = 0; i <= 5; i++) g.modules[8][i] = ((fmtBits >> i) & 1) === 1;
  g.modules[8][7] = ((fmtBits >> 6) & 1) === 1;   // bit 6, skip col 6
  g.modules[8][8] = ((fmtBits >> 7) & 1) === 1;   // bit 7, corner
  g.modules[7][8] = ((fmtBits >> 8) & 1) === 1;   // bit 8, skip row 6
  for (let i = 9; i <= 14; i++) g.modules[14 - i][8] = ((fmtBits >> i) & 1) === 1;
  // Copy 2
  for (let i = 0; i <= 6; i++) g.modules[sz - 1 - i][8] = ((fmtBits >> i) & 1) === 1;
  for (let i = 7; i <= 14; i++) g.modules[8][sz - 15 + i] = ((fmtBits >> i) & 1) === 1;
}

/**
 * Reserve version information positions (v7+): two 6×3 blocks.
 * Near top-right: rows 0..5, cols size-11..size-9.
 * Near bottom-left: rows size-11..size-9, cols 0..5.
 */
function reserveVersionInfo(g: WorkGrid, version: number): void {
  if (version < 7) return;
  const sz = g.size;
  for (let r = 0; r < 6; r++) for (let dc = 0; dc < 3; dc++) g.reserved[r][sz - 11 + dc] = true;
  for (let dr = 0; dr < 3; dr++) for (let c = 0; c < 6; c++) g.reserved[sz - 11 + dr][c] = true;
}

/**
 * Compute 18-bit version information (v7+).
 *
 * 1. 6-bit version number
 * 2. BCH(18,6): remainder of (version × x^12) mod G(x), G(x) = 0x1F25
 * 3. Concatenate for 18 bits
 *
 * G(x) = x^12+x^11+x^10+x^9+x^8+x^5+x^2+1 = 0x1F25.
 */
function computeVersionBits(version: number): number {
  let rem = version << 12;
  for (let i = 17; i >= 12; i--) {
    if ((rem >> i) & 1) rem ^= 0x1f25 << (i - 12);
  }
  return (version << 12) | (rem & 0xfff);
}

/**
 * Write version information into both 6×3 blocks (v7+).
 *
 * Top-right block: bit i → (5 − ⌊i/3⌋, size−9−(i%3))
 * Bottom-left block (transposed): bit i → (size−9−(i%3), 5−⌊i/3⌋)
 */
function writeVersionInfo(g: WorkGrid, version: number): void {
  if (version < 7) return;
  const sz = g.size;
  const bits = computeVersionBits(version);
  for (let i = 0; i < 18; i++) {
    const dark = ((bits >> i) & 1) === 1;
    const a = 5 - Math.floor(i / 3);
    const b = sz - 9 - (i % 3);
    g.modules[a][b] = dark;
    g.modules[b][a] = dark;
  }
}

/**
 * The always-dark module at (4V+9, 8).
 * Set once; not masked; not part of data.
 */
function placeDarkModule(g: WorkGrid, version: number): void {
  setMod(g, 4 * version + 9, 8, true, true);
}

/**
 * Place the interleaved codeword stream using the two-column zigzag scan.
 *
 * Scans from column size−1 leftward in 2-column strips, alternating
 * upward/downward:
 *   - Column 6 (vertical timing strip) is always skipped.
 *   - After decrementing to col=6, jump to col=5.
 *   - Reserved modules are skipped; data bits fill the rest.
 */
function placeBits(g: WorkGrid, codewords: number[], version: number): void {
  const sz = g.size;

  // Flatten codewords to a bit array (MSB first)
  const bits: boolean[] = [];
  for (const cw of codewords) for (let b = 7; b >= 0; b--) bits.push(((cw >> b) & 1) === 1);
  for (let i = 0; i < numRemainderBits(version); i++) bits.push(false);

  let bitIdx = 0;
  let up = true;       // true = bottom→top, false = top→bottom
  let col = sz - 1;    // leading column of current 2-column strip

  while (col >= 1) {
    for (let vi = 0; vi < sz; vi++) {
      const row = up ? sz - 1 - vi : vi;
      for (const dc of [0, 1]) {
        const c = col - dc;
        if (c === 6) continue;        // timing column
        if (g.reserved[row][c]) continue;
        g.modules[row][c] = bitIdx < bits.length ? bits[bitIdx++] : false;
      }
    }
    up = !up;
    col -= 2;
    if (col === 6) col = 5; // hop over the vertical timing strip
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Masking
// ─────────────────────────────────────────────────────────────────────────────

/**
 * The 8 mask conditions from ISO 18004 Table 10.
 *
 * If condition(row, col) is true, the module is flipped (dark↔light).
 * Applied only to non-reserved (data/ECC) modules.
 */
const MASK_CONDS: ReadonlyArray<(r: number, c: number) => boolean> = [
  (r, c) => (r + c) % 2 === 0,
  (r, _c) => r % 2 === 0,
  (_r, c) => c % 3 === 0,
  (r, c) => (r + c) % 3 === 0,
  (r, c) => (Math.floor(r / 2) + Math.floor(c / 3)) % 2 === 0,
  (r, c) => (r * c) % 2 + (r * c) % 3 === 0,
  (r, c) => ((r * c) % 2 + (r * c) % 3) % 2 === 0,
  (r, c) => ((r + c) % 2 + (r * c) % 3) % 2 === 0,
];

/** Return a new module array with mask applied to all non-reserved cells. */
function applyMask(
  modules: boolean[][], reserved: boolean[][], sz: number, maskIdx: number,
): boolean[][] {
  const cond = MASK_CONDS[maskIdx];
  return modules.map((row, r) =>
    row.map((dark, c) => reserved[r][c] ? dark : dark !== cond(r, c)),
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Penalty scoring (ISO 18004 Section 7.8.3)
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Compute the 4-rule penalty score for a (masked) module array.
 *
 * Rule 1: runs of ≥5 same-color modules in a row/col → score += run−2
 * Rule 2: 2×2 same-color blocks → score += 3 per block
 * Rule 3: finder-like patterns (1,0,1,1,1,0,1,0,0,0,0) → score += 40 per match
 * Rule 4: dark proportion deviation from 50% → score based on 5% steps
 */
function computePenalty(modules: boolean[][], sz: number): number {
  let penalty = 0;

  // Rule 1 — adjacent same-color runs ≥ 5
  for (let r = 0; r < sz; r++) {
    for (const horiz of [true, false]) {
      let run = 1;
      let prev = horiz ? modules[r][0] : modules[0][r];
      for (let i = 1; i < sz; i++) {
        const cur = horiz ? modules[r][i] : modules[i][r];
        if (cur === prev) { run++; }
        else { if (run >= 5) penalty += run - 2; run = 1; prev = cur; }
      }
      if (run >= 5) penalty += run - 2;
    }
  }

  // Rule 2 — 2×2 same-color blocks
  for (let r = 0; r < sz - 1; r++)
    for (let c = 0; c < sz - 1; c++) {
      const d = modules[r][c];
      if (d === modules[r][c+1] && d === modules[r+1][c] && d === modules[r+1][c+1]) penalty += 3;
    }

  // Rule 3 — finder-pattern-like sequences (horizontal and vertical)
  // Pattern and its reverse, each adds 40 when found.
  const P1 = [1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0];
  const P2 = [0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1];
  for (let a = 0; a < sz; a++) {
    for (let b = 0; b <= sz - 11; b++) {
      let mH1 = true, mH2 = true, mV1 = true, mV2 = true;
      for (let k = 0; k < 11; k++) {
        const bH = modules[a][b + k] ? 1 : 0;
        const bV = modules[b + k][a] ? 1 : 0;
        if (bH !== P1[k]) mH1 = false;
        if (bH !== P2[k]) mH2 = false;
        if (bV !== P1[k]) mV1 = false;
        if (bV !== P2[k]) mV2 = false;
      }
      if (mH1) penalty += 40;
      if (mH2) penalty += 40;
      if (mV1) penalty += 40;
      if (mV2) penalty += 40;
    }
  }

  // Rule 4 — dark module ratio deviation
  let dark = 0;
  for (let r = 0; r < sz; r++) for (let c = 0; c < sz; c++) if (modules[r][c]) dark++;
  const ratio = (dark / (sz * sz)) * 100;
  const prev5 = Math.floor(ratio / 5) * 5;
  penalty += Math.min(Math.abs(prev5 - 50), Math.abs(prev5 + 5 - 50)) / 5 * 10;

  return penalty;
}

// ─────────────────────────────────────────────────────────────────────────────
// Version selection
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Find the minimum version (1–40) whose data codeword capacity fits the input.
 *
 * Uses the exact bit count for the selected mode, accounting for mode indicator
 * and character-count field widths that vary with version.
 */
function selectVersion(input: string, ecc: EccLevel): number {
  const mode = selectMode(input);
  const byteLen = new TextEncoder().encode(input).length;

  for (let v = 1; v <= 40; v++) {
    const capacity = numDataCodewords(v, ecc);
    const dataBits =
      mode === "byte" ? byteLen * 8 :
      mode === "numeric" ? Math.ceil(input.length * 10 / 3) :
      Math.ceil(input.length * 11 / 2);
    const bitsNeeded = 4 + charCountBits(mode, v) + dataBits;
    if (Math.ceil(bitsNeeded / 8) <= capacity) return v;
  }
  throw new InputTooLongError(
    `Input (${input.length} chars, ECC=${ecc}) exceeds version 40 capacity.`,
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Grid initialisation
// ─────────────────────────────────────────────────────────────────────────────

function buildGrid(version: number): WorkGrid {
  const sz = symbolSize(version);
  const g = makeWorkGrid(sz);

  // Three finder patterns at three corners
  placeFinder(g, 0, 0);           // top-left
  placeFinder(g, 0, sz - 7);      // top-right
  placeFinder(g, sz - 7, 0);      // bottom-left

  // Separators (1-module light border just outside each finder)
  // Top-left: row 7 and col 7
  for (let i = 0; i <= 7; i++) { setMod(g, 7, i, false, true); setMod(g, i, 7, false, true); }
  // Top-right: row 7 and col sz-8
  for (let i = 0; i <= 7; i++) { setMod(g, 7, sz-1-i, false, true); setMod(g, i, sz-8, false, true); }
  // Bottom-left: row sz-8 and col 7
  for (let i = 0; i <= 7; i++) { setMod(g, sz-8, i, false, true); setMod(g, sz-1-i, 7, false, true); }

  placeTimingStrips(g);      // must come before alignments (sets row/col 6 reserved)
  placeAllAlignments(g, version);  // uses reserved-check to skip finder/timing overlaps

  reserveFormatInfo(g);      // reserve 15 positions × 2 copies
  reserveVersionInfo(g, version); // v7+: reserve 6×3 × 2 copies

  placeDarkModule(g, version); // always-dark at (4V+9, 8)

  return g;
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Encode a UTF-8 string into a QR Code ModuleGrid.
 *
 * Returns a `(4V+17) × (4V+17)` boolean grid, `true` = dark module.
 * Pass the result to `layout()` to obtain pixel coordinates, or to
 * `renderSvg()` for a one-shot SVG string.
 *
 * @throws InputTooLongError if the input exceeds version-40 capacity.
 *
 * @example
 * ```typescript
 * const grid = encode("https://example.com", "M");
 * // grid.rows === grid.cols === 29 for "https://example.com" at M
 * ```
 */
export function encode(input: string, eccLevel: EccLevel): ModuleGrid {
  // Early-exit guard: QR Code v40 holds at most 7089 numeric characters
  // (~2953 bytes in byte mode).  Without this guard, selectVersion() would
  // call `new TextEncoder().encode(input)` up to 40 times for a huge input,
  // allocating O(n) memory 40 times before finally throwing InputTooLongError.
  // In a server-side Node.js context this is a cheap DoS amplifier.
  if (input.length > 7089) {
    throw new InputTooLongError(
      `Input length ${input.length} exceeds 7089 (QR Code v40 numeric-mode maximum).`
    );
  }
  const version = selectVersion(input, eccLevel);
  const sz      = symbolSize(version);

  const dataCW     = buildDataCodewords(input, version, eccLevel);
  const blocks     = computeBlocks(dataCW, version, eccLevel);
  const interleaved = interleaveBlocks(blocks);

  const grid = buildGrid(version);
  placeBits(grid, interleaved, version);

  // Evaluate all 8 masks; pick the one with lowest penalty
  let bestMask = 0;
  let bestPenalty = Infinity;
  for (let m = 0; m < 8; m++) {
    const masked = applyMask(grid.modules, grid.reserved, sz, m);
    const fmtBits = computeFormatBits(eccLevel, m);
    const testG: WorkGrid = { size: sz, modules: masked, reserved: grid.reserved };
    writeFormatInfo(testG, fmtBits);
    const p = computePenalty(masked, sz);
    if (p < bestPenalty) { bestPenalty = p; bestMask = m; }
  }

  // Finalize with best mask
  const finalMods = applyMask(grid.modules, grid.reserved, sz, bestMask);
  const finalG: WorkGrid = { size: sz, modules: finalMods, reserved: grid.reserved };
  writeFormatInfo(finalG, computeFormatBits(eccLevel, bestMask));
  writeVersionInfo(finalG, version);

  return { rows: sz, cols: sz, modules: finalMods, moduleShape: "square" };
}

/**
 * Encode and convert to a pixel-resolved PaintScene.
 *
 * Delegates pixel geometry (module size, quiet zone, colours) to
 * `barcode-2d`'s `layout()`.
 */
export function encodeAndLayout(
  input: string,
  eccLevel: EccLevel,
  config?: Partial<Barcode2DLayoutConfig>,
): PaintScene {
  return layout(encode(input, eccLevel), config);
}

/**
 * Encode and render directly to an SVG string.
 *
 * Returns a complete `<svg>…</svg>` document.
 *
 * @security Do NOT inject the returned string via `innerHTML` or `outerHTML`.
 * Use `DOMParser` + `appendChild` instead, or a trusted HTML sanitizer:
 * ```typescript
 * const parser = new DOMParser();
 * const svgDoc = parser.parseFromString(svg, "image/svg+xml");
 * document.body.appendChild(svgDoc.documentElement);
 * ```
 *
 * @example
 * ```typescript
 * const svg = renderSvg("https://example.com", "M");
 * // Safe: parse, then append
 * const parser = new DOMParser();
 * document.body.appendChild(
 *   parser.parseFromString(svg, "image/svg+xml").documentElement
 * );
 * ```
 */
export function renderSvg(
  input: string,
  eccLevel: EccLevel,
  config?: Partial<Barcode2DLayoutConfig>,
): string {
  return renderToSvgString(encodeAndLayout(input, eccLevel, config));
}

/**
 * Encode with per-module role annotations (for interactive visualizers).
 *
 * v0.1.0: returns the encoded grid with null annotations.
 * Full annotation support (finder/timing/data/ECC roles per module) is v0.2.0.
 */
export function explain(input: string, eccLevel: EccLevel): AnnotatedModuleGrid {
  const grid = encode(input, eccLevel);
  return {
    ...grid,
    annotations: Array.from({ length: grid.rows }, () => new Array(grid.cols).fill(null)),
  };
}
