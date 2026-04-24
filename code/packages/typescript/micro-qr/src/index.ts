/**
 * @module micro-qr
 *
 * Micro QR Code encoder — ISO/IEC 18004:2015 Annex E compliant.
 *
 * Micro QR Code is the compact variant of QR Code, designed for applications
 * where even the smallest standard QR (21×21) is too large. Think
 * surface-mount component labels, circuit board markings, and miniature
 * industrial tags.
 *
 * ## What makes Micro QR different from regular QR?
 *
 * Regular QR Code uses three identical 7×7 finder patterns (squares at
 * top-left, top-right, and bottom-left corners) so a scanner can identify
 * orientation from any angle. Micro QR uses only **one** finder, in the
 * top-left. Because there is only one, orientation is always unambiguous —
 * the data area is always to the bottom-right of the single finder. This
 * saves enormous space at the cost of needing a controlled scanning
 * environment.
 *
 * ## Symbol sizes
 *
 * ```
 * M1: 11×11   M2: 13×13   M3: 15×15   M4: 17×17
 * formula: size = 2 × version_number + 9
 * ```
 *
 * ## Encoding pipeline
 *
 * ```
 * input string
 *   → auto-select smallest symbol (M1..M4) and mode (numeric/alphanumeric/byte)
 *   → build bit stream (mode indicator + char count + data + terminator + padding)
 *   → Reed-Solomon ECC (GF(256)/0x11D, b=0, single block)
 *   → initialize grid (finder, L-shaped separator, timing at row0/col0, format reserved)
 *   → zigzag data placement (two-column snake from bottom-right)
 *   → evaluate 4 mask patterns, pick lowest penalty
 *   → write format information (15 bits, single copy, XOR 0x4445)
 *   → ModuleGrid
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

export type { ModuleGrid, Barcode2DLayoutConfig, PaintScene, AnnotatedModuleGrid };

export const VERSION = "0.1.0";

// ─────────────────────────────────────────────────────────────────────────────
// Public types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Micro QR symbol designator (M1 through M4).
 *
 * Each step up adds two rows/columns (size = 2×version+9), increasing
 * data capacity. M1 is the smallest (11×11) and supports only numeric mode
 * with no real error correction. M4 (17×17) supports all four modes and
 * reaches 35 numeric digits at ECC-L.
 */
export type MicroQRVersion = "M1" | "M2" | "M3" | "M4";

/**
 * Error correction level.
 *
 * Unlike regular QR which has L/M/Q/H, Micro QR has a reduced set:
 *
 * | Level     | Available in | Recovery |
 * |-----------|-------------|---------|
 * | DETECTION | M1 only     | detects errors only (no correction) |
 * | L         | M2, M3, M4  | ~7% of codewords |
 * | M         | M2, M3, M4  | ~15% of codewords |
 * | Q         | M4 only     | ~25% of codewords |
 *
 * Level H (30%) is not available in any Micro QR symbol — the symbols
 * are too small to afford that much redundancy.
 */
export type MicroQREccLevel = "DETECTION" | "L" | "M" | "Q";

/**
 * Error hierarchy for the Micro QR encoder.
 *
 * All errors extend MicroQRError so callers can catch the whole family
 * with a single `instanceof MicroQRError` guard.
 */
export class MicroQRError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "MicroQRError";
  }
}

/** Input is too long for any M1–M4 symbol at any ECC level. */
export class InputTooLongError extends MicroQRError {
  constructor(message: string) {
    super(message);
    this.name = "InputTooLongError";
  }
}

/** The requested encoding mode is not available for the chosen symbol. */
export class UnsupportedModeError extends MicroQRError {
  constructor(message: string) {
    super(message);
    this.name = "UnsupportedModeError";
  }
}

/** A character cannot be encoded in the selected mode. */
export class InvalidCharacterError extends MicroQRError {
  constructor(message: string) {
    super(message);
    this.name = "InvalidCharacterError";
  }
}

/** The requested ECC level is not available for the chosen symbol. */
export class ECCNotAvailableError extends MicroQRError {
  constructor(message: string) {
    super(message);
    this.name = "ECCNotAvailableError";
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Capacity and codeword tables
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Symbol+ECC configuration.
 *
 * There are exactly 8 valid (version, ECC) combinations in Micro QR.
 * We use a tuple index 0..7 derived from the symbol_indicator field
 * in the format information (the standard's own numbering).
 *
 * symbol_indicator → version+ECC:
 *   0 = M1/DETECTION   4 = M3/M
 *   1 = M2/L           5 = M4/L
 *   2 = M2/M           6 = M4/M
 *   3 = M3/L           7 = M4/Q
 */
interface SymbolConfig {
  version: MicroQRVersion;
  ecc: MicroQREccLevel;
  symbolIndicator: number;    // 0..7, used in format information
  size: number;               // symbol side length in modules
  dataCW: number;             // data codewords (full bytes)
  eccCW: number;              // ECC codewords
  numericCap: number;         // max numeric characters
  alphaCap: number;           // max alphanumeric chars (-1 = not supported)
  byteCap: number;            // max byte chars (-1 = not supported)
  kanjiCap: number;           // max kanji chars (-1 = not supported)
  terminatorBits: number;     // 3/5/7/9 zero bits appended after data
  modeIndicatorBits: number;  // 0/1/2/3 bits for the mode indicator field
  charCountBitsNumeric: number;    // char-count field width for numeric
  charCountBitsAlpha: number;      // char-count field width for alphanumeric
  charCountBitsByte: number;       // char-count field width for byte
  charCountBitsKanji: number;      // char-count field width for kanji
  m1HalfCW: boolean;          // true for M1: last data "codeword" is 4 bits
}

/**
 * All 8 valid Micro QR symbol configurations from ISO 18004:2015 Annex E.
 *
 * The data capacities, codeword counts, and field widths are mandatory
 * compile-time constants — do not compute them at runtime.
 */
const SYMBOL_CONFIGS: SymbolConfig[] = [
  // M1 / DETECTION
  {
    version: "M1", ecc: "DETECTION", symbolIndicator: 0, size: 11,
    dataCW: 3, eccCW: 2,
    numericCap: 5, alphaCap: -1, byteCap: -1, kanjiCap: -1,
    terminatorBits: 3, modeIndicatorBits: 0,
    charCountBitsNumeric: 3, charCountBitsAlpha: 0, charCountBitsByte: 0, charCountBitsKanji: 0,
    m1HalfCW: true,
  },
  // M2 / L
  {
    version: "M2", ecc: "L", symbolIndicator: 1, size: 13,
    dataCW: 5, eccCW: 5,
    numericCap: 10, alphaCap: 6, byteCap: 4, kanjiCap: -1,
    terminatorBits: 5, modeIndicatorBits: 1,
    charCountBitsNumeric: 4, charCountBitsAlpha: 3, charCountBitsByte: 4, charCountBitsKanji: 0,
    m1HalfCW: false,
  },
  // M2 / M
  {
    version: "M2", ecc: "M", symbolIndicator: 2, size: 13,
    dataCW: 4, eccCW: 6,
    numericCap: 8, alphaCap: 5, byteCap: 3, kanjiCap: -1,
    terminatorBits: 5, modeIndicatorBits: 1,
    charCountBitsNumeric: 4, charCountBitsAlpha: 3, charCountBitsByte: 4, charCountBitsKanji: 0,
    m1HalfCW: false,
  },
  // M3 / L
  {
    version: "M3", ecc: "L", symbolIndicator: 3, size: 15,
    dataCW: 11, eccCW: 6,
    numericCap: 23, alphaCap: 14, byteCap: 9, kanjiCap: -1,
    terminatorBits: 7, modeIndicatorBits: 2,
    charCountBitsNumeric: 5, charCountBitsAlpha: 4, charCountBitsByte: 4, charCountBitsKanji: 0,
    m1HalfCW: false,
  },
  // M3 / M
  {
    version: "M3", ecc: "M", symbolIndicator: 4, size: 15,
    dataCW: 9, eccCW: 8,
    numericCap: 18, alphaCap: 11, byteCap: 7, kanjiCap: -1,
    terminatorBits: 7, modeIndicatorBits: 2,
    charCountBitsNumeric: 5, charCountBitsAlpha: 4, charCountBitsByte: 4, charCountBitsKanji: 0,
    m1HalfCW: false,
  },
  // M4 / L
  {
    version: "M4", ecc: "L", symbolIndicator: 5, size: 17,
    dataCW: 16, eccCW: 8,
    numericCap: 35, alphaCap: 21, byteCap: 15, kanjiCap: 9,
    terminatorBits: 9, modeIndicatorBits: 3,
    charCountBitsNumeric: 6, charCountBitsAlpha: 5, charCountBitsByte: 5, charCountBitsKanji: 4,
    m1HalfCW: false,
  },
  // M4 / M
  {
    version: "M4", ecc: "M", symbolIndicator: 6, size: 17,
    dataCW: 14, eccCW: 10,
    numericCap: 30, alphaCap: 18, byteCap: 13, kanjiCap: 8,
    terminatorBits: 9, modeIndicatorBits: 3,
    charCountBitsNumeric: 6, charCountBitsAlpha: 5, charCountBitsByte: 5, charCountBitsKanji: 4,
    m1HalfCW: false,
  },
  // M4 / Q
  {
    version: "M4", ecc: "Q", symbolIndicator: 7, size: 17,
    dataCW: 10, eccCW: 14,
    numericCap: 21, alphaCap: 13, byteCap: 9, kanjiCap: 6,
    terminatorBits: 9, modeIndicatorBits: 3,
    charCountBitsNumeric: 6, charCountBitsAlpha: 5, charCountBitsByte: 5, charCountBitsKanji: 4,
    m1HalfCW: false,
  },
];

// ─────────────────────────────────────────────────────────────────────────────
// RS generator polynomials (compile-time constants)
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Monic RS generator polynomials over GF(256)/0x11D, b=0 convention.
 *
 * These are the polynomials g(x) = (x+α⁰)(x+α¹)···(x+α^{n-1}).
 * Coefficients listed highest-degree first; leading monic term (1) included
 * so the array has n+1 entries.
 *
 * We only need counts {2, 5, 6, 8, 10, 14} for Micro QR.
 */
const RS_GENERATORS: ReadonlyMap<number, ReadonlyArray<number>> = new Map([
  // 2 ECC codewords (M1 detection)
  // g(x) = (x+1)(x+α) = x² + 3x + 2
  [2, [0x01, 0x03, 0x02]],

  // 5 ECC codewords (M2-L)
  [5, [0x01, 0x1f, 0xf6, 0x44, 0xd9, 0x68]],

  // 6 ECC codewords (M2-M, M3-L)
  [6, [0x01, 0x3f, 0x4e, 0x17, 0x9b, 0x05, 0x37]],

  // 8 ECC codewords (M3-M, M4-L)
  [8, [0x01, 0x63, 0x0d, 0x60, 0x6d, 0x5b, 0x10, 0xa2, 0xa3]],

  // 10 ECC codewords (M4-M)
  [10, [0x01, 0xf6, 0x75, 0xa8, 0xd0, 0xc3, 0xe3, 0x36, 0xe1, 0x3c, 0x45]],

  // 14 ECC codewords (M4-Q)
  [14, [0x01, 0xf6, 0x9a, 0x60, 0x97, 0x8a, 0xf1, 0xa4, 0xa1, 0x8e, 0xfc, 0x7a, 0x52, 0xad, 0xac]],
]);

// ─────────────────────────────────────────────────────────────────────────────
// Pre-computed format information table
// ─────────────────────────────────────────────────────────────────────────────

/**
 * All 32 pre-computed format information words (after XOR with 0x4445).
 *
 * Indexed as FORMAT_TABLE[symbolIndicator][maskPattern].
 *
 * The 15-bit format word encodes:
 *   [symbol_indicator (3 bits)] [mask_pattern (2 bits)] [BCH-10 remainder]
 * then XOR-masked with 0x4445 (not 0x5412 like regular QR).
 *
 * Source: computed via the BCH(15,5) procedure with G(x)=0x537.
 *
 * | Symbol+ECC | Mask 0 | Mask 1 | Mask 2 | Mask 3 |
 * |-----------|--------|--------|--------|--------|
 * | M1 (000)  | 0x4445 | 0x4172 | 0x4E2B | 0x4B1C |
 * | M2-L(001) | 0x5528 | 0x501F | 0x5F46 | 0x5A71 |
 * | M2-M(010) | 0x6649 | 0x637E | 0x6C27 | 0x6910 |
 * | M3-L(011) | 0x7764 | 0x7253 | 0x7D0A | 0x783D |
 * | M3-M(100) | 0x06DE | 0x03E9 | 0x0CB0 | 0x0987 |
 * | M4-L(101) | 0x17F3 | 0x12C4 | 0x1D9D | 0x18AA |
 * | M4-M(110) | 0x24B2 | 0x2185 | 0x2EDC | 0x2BEB |
 * | M4-Q(111) | 0x359F | 0x30A8 | 0x3FF1 | 0x3AC6 |
 */
const FORMAT_TABLE: ReadonlyArray<ReadonlyArray<number>> = [
  [0x4445, 0x4172, 0x4E2B, 0x4B1C],  // M1
  [0x5528, 0x501F, 0x5F46, 0x5A71],  // M2-L
  [0x6649, 0x637E, 0x6C27, 0x6910],  // M2-M
  [0x7764, 0x7253, 0x7D0A, 0x783D],  // M3-L
  [0x06DE, 0x03E9, 0x0CB0, 0x0987],  // M3-M
  [0x17F3, 0x12C4, 0x1D9D, 0x18AA],  // M4-L
  [0x24B2, 0x2185, 0x2EDC, 0x2BEB],  // M4-M
  [0x359F, 0x30A8, 0x3FF1, 0x3AC6],  // M4-Q
];

// ─────────────────────────────────────────────────────────────────────────────
// Encoding mode
// ─────────────────────────────────────────────────────────────────────────────

/**
 * The 45-character alphanumeric set used in QR and Micro QR.
 *
 * Pairs pack into 11 bits: (first×45 + second).
 * A trailing single character uses 6 bits.
 */
const ALPHANUM_CHARS = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

type EncodingMode = "numeric" | "alphanumeric" | "byte";

/**
 * Mode indicator values, keyed by (mode, symbol version).
 *
 * M1: no indicator (0 bits — only numeric mode exists)
 * M2: 1 bit  — 0=numeric, 1=alphanumeric
 * M3: 2 bits — 00=numeric, 01=alphanumeric, 10=byte
 * M4: 3 bits — 000=numeric, 001=alphanumeric, 010=byte, 011=kanji
 *
 * The fact that mode indicator widths grow with version is deliberate:
 * larger symbols can fit more modes, so they need more bits to distinguish them.
 */
function modeIndicatorValue(mode: EncodingMode, cfg: SymbolConfig): number {
  if (cfg.modeIndicatorBits === 0) return 0; // M1, no indicator
  if (cfg.modeIndicatorBits === 1) return mode === "numeric" ? 0 : 1;
  if (cfg.modeIndicatorBits === 2) {
    if (mode === "numeric") return 0b00;
    if (mode === "alphanumeric") return 0b01;
    return 0b10; // byte
  }
  // 3 bits (M4)
  if (mode === "numeric") return 0b000;
  if (mode === "alphanumeric") return 0b001;
  return 0b010; // byte (kanji = 0b011 but handled separately)
}

function charCountBits(mode: EncodingMode, cfg: SymbolConfig): number {
  if (mode === "numeric") return cfg.charCountBitsNumeric;
  if (mode === "alphanumeric") return cfg.charCountBitsAlpha;
  return cfg.charCountBitsByte;
}

/**
 * Determine the most compact encoding mode that covers the full input
 * and is supported by the given symbol configuration.
 *
 * Selection order (most compact to least):
 *   1. numeric   — all chars are 0–9
 *   2. alphanumeric — all chars in the 45-char set
 *   3. byte      — raw UTF-8 bytes
 *
 * Kanji mode is handled separately (future extension).
 */
function selectMode(input: string, cfg: SymbolConfig): EncodingMode {
  const isNumeric = input === "" || /^\d+$/.test(input);
  if (isNumeric && cfg.charCountBitsNumeric > 0) return "numeric";

  const isAlpha = [...input].every((c) => ALPHANUM_CHARS.includes(c));
  if (isAlpha && cfg.alphaCap > 0) return "alphanumeric";

  if (cfg.byteCap > 0) return "byte";

  throw new UnsupportedModeError(
    `Input cannot be encoded in any mode supported by ${cfg.version}-${cfg.ecc}. ` +
    `Use a higher version or switch to byte mode.`
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Bit-writer utility
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Accumulates individual bits, then flushes them as a byte array.
 *
 * The bit stream for Micro QR is MSB-first within each codeword:
 * the first bit written ends up as the most-significant bit of byte 0.
 *
 * This mirrors a serial data bus where the most significant bit is
 * transmitted first ("big-endian bit order").
 */
class BitWriter {
  private readonly _bits: number[] = [];

  /** Append `count` bits from `value`, MSB first. */
  write(value: number, count: number): void {
    for (let i = count - 1; i >= 0; i--) {
      this._bits.push((value >> i) & 1);
    }
  }

  get bitLength(): number { return this._bits.length; }

  /**
   * Return all accumulated bits as a packed byte array.
   * If the bit count is not a multiple of 8, the last byte is
   * zero-padded on the right (least-significant bits are 0).
   */
  toBytes(): number[] {
    const bytes: number[] = [];
    for (let i = 0; i < this._bits.length; i += 8) {
      let b = 0;
      for (let j = 0; j < 8; j++) b = (b << 1) | (this._bits[i + j] ?? 0);
      bytes.push(b);
    }
    return bytes;
  }

  toBits(): number[] { return [...this._bits]; }
}

// ─────────────────────────────────────────────────────────────────────────────
// Data encoding helpers
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Encode a numeric string into the bit writer.
 *
 * Groups of three digits map to 10 bits (values 0–999).
 * A remaining pair maps to 7 bits (0–99).
 * A single trailing digit maps to 4 bits (0–9).
 *
 * Example: "12345" → "123" (10b=0001111011) + "45" (7b=0101101) = 17 bits.
 */
function encodeNumeric(input: string, w: BitWriter): void {
  let i = 0;
  while (i + 2 < input.length) {
    w.write(parseInt(input.slice(i, i + 3), 10), 10);
    i += 3;
  }
  if (i + 1 < input.length) {
    w.write(parseInt(input.slice(i, i + 2), 10), 7);
    i += 2;
  }
  if (i < input.length) {
    w.write(parseInt(input[i]!, 10), 4);
  }
}

/**
 * Encode an alphanumeric string into the bit writer.
 *
 * Pairs encode as (firstIndex × 45 + secondIndex) in 11 bits.
 * A trailing single character uses 6 bits.
 */
function encodeAlphanumeric(input: string, w: BitWriter): void {
  let i = 0;
  while (i + 1 < input.length) {
    const a = ALPHANUM_CHARS.indexOf(input[i]!);
    const b = ALPHANUM_CHARS.indexOf(input[i + 1]!);
    if (a < 0 || b < 0) throw new InvalidCharacterError(
      `Character not in alphanumeric set: '${a < 0 ? input[i] : input[i + 1]}'`
    );
    w.write(a * 45 + b, 11);
    i += 2;
  }
  if (i < input.length) {
    const a = ALPHANUM_CHARS.indexOf(input[i]!);
    if (a < 0) throw new InvalidCharacterError(
      `Character not in alphanumeric set: '${input[i]}'`
    );
    w.write(a, 6);
  }
}

/**
 * Encode a byte-mode string by writing each UTF-8 byte as 8 bits.
 *
 * For ASCII / ISO-8859-1 inputs this is identical to the raw byte values.
 * For multi-byte UTF-8 characters, each byte of the UTF-8 encoding is
 * written individually — so a 3-byte UTF-8 code point uses 3 byte-count
 * units and 24 bits. Scanners that understand UTF-8 will reconstruct the
 * original character; older scanners see the raw bytes.
 */
function encodeByte(input: string, w: BitWriter): void {
  const bytes = new TextEncoder().encode(input);
  for (const b of bytes) w.write(b, 8);
}

// ─────────────────────────────────────────────────────────────────────────────
// Reed-Solomon encoder
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Compute n ECC bytes using GF(256)/0x11D polynomial division.
 *
 * This is the LFSR (Linear Feedback Shift Register) implementation
 * of polynomial remainder division:
 *
 * ```
 * ecc = [0] × n
 * for each data byte b:
 *   feedback = b XOR ecc[0]
 *   shift ecc left (drop ecc[0], append 0)
 *   for i in 0..n-1:
 *     ecc[i] ^= G[i+1] × feedback   (GF multiplication)
 * ```
 *
 * The resulting `ecc` array is the remainder of D(x)·x^n mod G(x).
 * For b=0 convention, the first root of G is α^0 = 1, so the standard
 * syndrome check S_j = R(α^j) for j=0..n-1 should yield all zeros for
 * a valid codeword.
 */
function rsEncode(data: number[], eccCount: number): number[] {
  const gen = RS_GENERATORS.get(eccCount);
  if (!gen) throw new MicroQRError(`No generator polynomial for eccCount=${eccCount}`);
  const n = eccCount;
  const rem: number[] = new Array(n).fill(0);
  for (const b of data) {
    const fb = b ^ rem[0]!;
    for (let i = 0; i < n - 1; i++) rem[i] = rem[i + 1]!;
    rem[n - 1] = 0;
    if (fb !== 0) {
      for (let i = 0; i < n; i++) rem[i]! != null && (rem[i] ^= gfMul(gen[i + 1] as number, fb));
    }
  }
  return rem;
}

// ─────────────────────────────────────────────────────────────────────────────
// Data codeword assembly
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Build the complete data codeword byte sequence for the given input and config.
 *
 * For all symbols except M1:
 *   [mode indicator] [char count] [data bits] [terminator] [byte-align pad] [0xEC/0x11 fill]
 *   Total: exactly cfg.dataCW bytes.
 *
 * For M1 (m1HalfCW = true):
 *   Total data capacity is 20 bits (2 full bytes + 4-bit nibble).
 *   The RS encoder receives 3 bytes where byte[2] has data in the upper 4 bits
 *   and 0000 in the lower 4 bits.
 *   No 0xEC/0x11 padding is used for M1.
 *
 * The terminator is `terminatorBits` zero bits, truncated if capacity is full.
 */
function buildDataCodewords(input: string, cfg: SymbolConfig): number[] {
  const mode = selectMode(input, cfg);

  // Total capacity in bits
  const totalBits = cfg.m1HalfCW
    ? cfg.dataCW * 8 - 4     // M1: 3×8 − 4 = 20 usable bits
    : cfg.dataCW * 8;

  const w = new BitWriter();

  // Mode indicator (0 bits for M1, otherwise 1/2/3 bits)
  if (cfg.modeIndicatorBits > 0) {
    w.write(modeIndicatorValue(mode, cfg), cfg.modeIndicatorBits);
  }

  // Character count
  const ccBits = charCountBits(mode, cfg);
  const byteInput = new TextEncoder().encode(input);
  const charCount = mode === "byte" ? byteInput.length : input.length;
  w.write(charCount, ccBits);

  // Encoded data
  if (mode === "numeric")      encodeNumeric(input, w);
  else if (mode === "alphanumeric") encodeAlphanumeric(input, w);
  else                         encodeByte(input, w);

  // Terminator: up to terminatorBits zero bits (truncated if capacity exhausted)
  const remaining = totalBits - w.bitLength;
  if (remaining > 0) w.write(0, Math.min(cfg.terminatorBits, remaining));

  if (cfg.m1HalfCW) {
    // M1: pad to exactly 20 bits with zeros, then pack into 3 bytes
    // (the last byte has data in the upper 4 bits, lower 4 bits = 0000)
    const bits = w.toBits();
    while (bits.length < 20) bits.push(0);
    // Truncate to 20 bits (terminator might overshoot in theory but shouldn't)
    bits.splice(20);
    // Pack into 3 bytes: byte0 = bits[0..7], byte1 = bits[8..15],
    // byte2 = bits[16..19] << 4 (upper nibble only)
    const b0 = (bits[0]! << 7) | (bits[1]! << 6) | (bits[2]! << 5) | (bits[3]! << 4) |
               (bits[4]! << 3) | (bits[5]! << 2) | (bits[6]! << 1) | bits[7]!;
    const b1 = (bits[8]! << 7) | (bits[9]! << 6) | (bits[10]! << 5) | (bits[11]! << 4) |
               (bits[12]! << 3) | (bits[13]! << 2) | (bits[14]! << 1) | bits[15]!;
    const b2 = (bits[16]! << 7) | (bits[17]! << 6) | (bits[18]! << 5) | (bits[19]! << 4);
    return [b0, b1, b2];
  }

  // Pad to byte boundary
  const rem = w.bitLength % 8;
  if (rem !== 0) w.write(0, 8 - rem);

  // Fill remaining data codewords with alternating 0xEC / 0x11
  const bytes = w.toBytes();
  let padByte = 0xec;
  while (bytes.length < cfg.dataCW) {
    bytes.push(padByte);
    padByte = padByte === 0xec ? 0x11 : 0xec;
  }
  return bytes;
}

// ─────────────────────────────────────────────────────────────────────────────
// Symbol / version selection
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Find the smallest symbol configuration that can hold the given input.
 *
 * The auto-selection algorithm:
 *   For each config in SYMBOL_CONFIGS (M1→M4-Q order):
 *     If the requested ECC level matches (or is "any"):
 *       Determine the encoding mode for this symbol.
 *       If the character count fits within the symbol's capacity:
 *         Return this config.
 *
 * The capacity check uses the pre-computed character limits (numericCap,
 * alphaCap, byteCap) rather than re-computing from bit counts. This
 * matches the standard's own capacity tables exactly.
 */
function selectConfig(
  input: string,
  version?: MicroQRVersion,
  ecc?: MicroQREccLevel,
): SymbolConfig {
  const candidates = SYMBOL_CONFIGS.filter((c) => {
    if (version && c.version !== version) return false;
    if (ecc && c.ecc !== ecc) return false;
    return true;
  });

  if (candidates.length === 0) {
    throw new ECCNotAvailableError(
      `No symbol configuration matches version=${version ?? "any"} ecc=${ecc ?? "any"}.`
    );
  }

  for (const cfg of candidates) {
    try {
      const mode = selectMode(input, cfg);
      const byteLen = new TextEncoder().encode(input).length;
      const len = mode === "byte" ? byteLen : input.length;
      const cap = mode === "numeric" ? cfg.numericCap :
                  mode === "alphanumeric" ? cfg.alphaCap :
                  cfg.byteCap;
      if (cap >= 0 && len <= cap) return cfg;
    } catch {
      // Mode not supported or input doesn't fit — try next config
    }
  }

  throw new InputTooLongError(
    `Input "${input.length > 20 ? input.slice(0, 20) + "…" : input}" ` +
    `(length ${input.length}) does not fit in any Micro QR symbol ` +
    `(version=${version ?? "any"}, ecc=${ecc ?? "any"}). ` +
    `Maximum is 35 numeric characters in M4-L.`
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Grid construction
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Internal working grid — tracks both module values (dark/light) and
 * which positions are "reserved" (structural) so the data placement
 * and masking steps know which modules to skip.
 */
interface WorkGrid {
  size: number;
  modules: boolean[][];   // true = dark
  reserved: boolean[][];  // true = structural (finder/separator/timing/format)
}

function makeWorkGrid(size: number): WorkGrid {
  return {
    size,
    modules: Array.from({ length: size }, () => new Array<boolean>(size).fill(false)),
    reserved: Array.from({ length: size }, () => new Array<boolean>(size).fill(false)),
  };
}

function setMod(g: WorkGrid, r: number, c: number, dark: boolean, reserve = false): void {
  g.modules[r]![c] = dark;
  if (reserve) g.reserved[r]![c] = true;
}

/**
 * Place the 7×7 finder pattern at the top-left corner (rows 0–6, cols 0–6).
 *
 * The pattern:
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
 * Dark modules form the outer border and a 3×3 inner core.
 * This 1:1:3:1:1 dark:light ratio is what scanners look for.
 */
function placeFinder(g: WorkGrid): void {
  for (let dr = 0; dr < 7; dr++) {
    for (let dc = 0; dc < 7; dc++) {
      const onBorder = dr === 0 || dr === 6 || dc === 0 || dc === 6;
      const inCore   = dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4;
      setMod(g, dr, dc, onBorder || inCore, true);
    }
  }
}

/**
 * Place the L-shaped separator (light modules bordering the finder on its
 * bottom and right sides).
 *
 * In regular QR, each of the three finder patterns is surrounded on all
 * four sides by light separators. In Micro QR, the finder is in the
 * top-left corner, so the top and left edges are the symbol boundary —
 * only the bottom and right need separators:
 *
 * ```
 * Row 7, cols 0–7  (bottom of finder + corner)
 * Col 7, rows 0–7  (right of finder + corner)
 * ```
 */
function placeSeparator(g: WorkGrid): void {
  for (let i = 0; i <= 7; i++) {
    setMod(g, 7, i, false, true);  // bottom row
    setMod(g, i, 7, false, true);  // right column
  }
}

/**
 * Place the timing pattern extensions.
 *
 * The timing pattern in Micro QR runs along row 0 and col 0. The first
 * 7 positions (0–6) are already covered by the finder pattern. Position 7
 * is the separator (always light). Positions 8 onward alternate dark/light:
 *
 * ```
 * Dark at even col index: col 8, 10, 12, ...
 * Dark at even row index: row 8, 10, 12, ...
 * ```
 *
 * Unlike regular QR, there is no "hop over col 6" because the timing
 * column is col 0, not col 6.
 */
function placeTiming(g: WorkGrid): void {
  const sz = g.size;
  // Row 0: timing from col 8 to sz-1
  for (let c = 8; c < sz; c++) setMod(g, 0, c, c % 2 === 0, true);
  // Col 0: timing from row 8 to sz-1
  for (let r = 8; r < sz; r++) setMod(g, r, 0, r % 2 === 0, true);
}

/**
 * Reserve format information module positions.
 *
 * The 15 format modules form an L-shape:
 *   Row 8, cols 1–8  → 8 modules (hold bits f14..f7, MSB first)
 *   Col 8, rows 1–7  → 7 modules (hold bits f6..f0, where f0 is at row 1)
 *
 * These are temporarily set to light (false) and marked reserved.
 * Actual format bits are written after mask selection.
 *
 * Note: unlike regular QR which has TWO copies of format info, Micro QR
 * has only ONE. This means a scanner cannot recover from damage to
 * these modules — but the small symbol size means they're a smaller
 * fraction of the total and less likely to be hit.
 */
function reserveFormatInfo(g: WorkGrid): void {
  // Row 8, cols 1–8 (f14..f7)
  for (let c = 1; c <= 8; c++) { g.modules[8]![c] = false; g.reserved[8]![c] = true; }
  // Col 8, rows 1–7 (f6..f0)
  for (let r = 1; r <= 7; r++) { g.modules[r]![8] = false; g.reserved[r]![8] = true; }
}

/**
 * Write format information bits into the reserved positions.
 *
 * Placement (f14 = MSB):
 *   Row 8, col 1  ← f14
 *   Row 8, col 2  ← f13
 *   ...
 *   Row 8, col 8  ← f7
 *   Col 8, row 7  ← f6
 *   Col 8, row 6  ← f5
 *   ...
 *   Col 8, row 1  ← f0  (LSB)
 *
 * The row-8 strip goes left-to-right (MSB first).
 * The col-8 strip goes upward (row 7 → row 1), so the LSB is nearest
 * the finder corner.
 */
function writeFormatInfo(g: WorkGrid, fmtBits: number): void {
  // Row 8, cols 1–8: bits f14 down to f7
  for (let i = 0; i < 8; i++) {
    g.modules[8]![1 + i] = ((fmtBits >> (14 - i)) & 1) === 1;
  }
  // Col 8, rows 7 down to 1: bits f6 down to f0
  for (let i = 0; i < 7; i++) {
    g.modules[7 - i]![8] = ((fmtBits >> (6 - i)) & 1) === 1;
  }
}

/**
 * Initialize the grid with all structural modules.
 *
 * Order matters: each function assumes earlier ones have run.
 * reserved[][] is the authoritative mask for data placement.
 */
function buildGrid(cfg: SymbolConfig): WorkGrid {
  const g = makeWorkGrid(cfg.size);
  placeFinder(g);
  placeSeparator(g);
  placeTiming(g);
  reserveFormatInfo(g);
  return g;
}

// ─────────────────────────────────────────────────────────────────────────────
// Data placement (two-column zigzag)
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Place the final codeword stream into the grid using the two-column zigzag.
 *
 * The zigzag scans from the bottom-right corner, moving left two columns
 * at a time, alternating upward and downward direction:
 *
 * ```
 * col = size-1, dir = up
 *   scan (size-1, col), (size-1, col-1)   ← rightmost cell of current row
 *   scan (size-2, col), (size-2, col-1)
 *   ...
 *   scan (0, col),      (0, col-1)
 * flip direction, col -= 2
 * col = size-3, dir = down
 *   scan (0, col), ...
 * ```
 *
 * Reserved modules (finder/separator/timing/format) are skipped.
 * Remaining unset modules after all bits are placed receive 0 (remainder bits).
 *
 * Key difference from regular QR: no timing column skip at col 6.
 * Micro QR's timing is at col 0, which is already reserved everywhere,
 * so the skip happens automatically through the reserved check.
 */
function placeBits(g: WorkGrid, bits: boolean[]): void {
  const sz = g.size;
  let bitIdx = 0;
  let up = true;

  for (let col = sz - 1; col >= 1; col -= 2) {
    const rows = up
      ? Array.from({ length: sz }, (_, i) => sz - 1 - i)  // sz-1 down to 0
      : Array.from({ length: sz }, (_, i) => i);           // 0 up to sz-1

    for (const row of rows) {
      for (const dc of [0, 1]) {
        const c = col - dc;
        if (g.reserved[row]![c]) continue;
        g.modules[row]![c] = bitIdx < bits.length ? bits[bitIdx++]! : false;
      }
    }
    up = !up;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Masking
// ─────────────────────────────────────────────────────────────────────────────

/**
 * The 4 mask conditions for Micro QR (patterns 0–3).
 *
 * These are the same as the first four patterns in regular QR's set of 8.
 * If the condition is true for a data/ECC module, that module is flipped.
 *
 * | Pattern | Condition |
 * |---------|-----------|
 * | 0       | (row + col) mod 2 == 0 |
 * | 1       | row mod 2 == 0 |
 * | 2       | col mod 3 == 0 |
 * | 3       | (row + col) mod 3 == 0 |
 */
const MASK_CONDITIONS: ReadonlyArray<(r: number, c: number) => boolean> = [
  (r, c) => (r + c) % 2 === 0,
  (r, _c) => r % 2 === 0,
  (_r, c) => c % 3 === 0,
  (r, c) => (r + c) % 3 === 0,
];

/**
 * Apply mask pattern `maskIdx` to all non-reserved modules.
 *
 * Returns a new module array; the original is not modified (important
 * because we evaluate all 4 masks to pick the best one).
 */
function applyMask(
  modules: boolean[][], reserved: boolean[][], sz: number, maskIdx: number,
): boolean[][] {
  const cond = MASK_CONDITIONS[maskIdx]!;
  return modules.map((row, r) =>
    row.map((dark, c) => reserved[r]![c] ? dark : dark !== cond(r, c))
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Penalty scoring
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Compute the 4-rule penalty score for a module grid.
 *
 * The four rules (same as regular QR):
 *
 * **Rule 1** — Adjacent same-color runs of 5+ modules in any row or column.
 *   Score += (run_length - 2) for each run of length ≥ 5.
 *   A run of 5 scores 3, a run of 7 scores 5, etc.
 *
 * **Rule 2** — 2×2 all-same-color blocks.
 *   Score += 3 for each 2×2 square where all four modules share a color.
 *   Overlapping blocks each count separately.
 *
 * **Rule 3** — Finder-pattern-like sequences.
 *   Score += 40 for each occurrence of:
 *     1 0 1 1 1 0 1 0 0 0 0   (looks like a finder in one scan direction)
 *   or its reverse:
 *     0 0 0 0 1 0 1 1 1 0 1
 *   in any row or column. These sequences can confuse scanners trying to
 *   locate the finder pattern.
 *
 * **Rule 4** — Dark/light proportion.
 *   score += min(|prev5 - 50|, |next5 - 50|) / 5 × 10
 *   where prev5 = largest multiple of 5 ≤ dark_percent,
 *         next5 = prev5 + 5.
 *   This penalizes symbols that are significantly darker or lighter than 50%.
 */
function computePenalty(modules: boolean[][], sz: number): number {
  let penalty = 0;

  // Rule 1 — adjacent same-color runs of ≥ 5
  for (let a = 0; a < sz; a++) {
    for (const horiz of [true, false]) {
      let run = 1;
      let prev = horiz ? modules[a]![0]! : modules[0]![a]!;
      for (let i = 1; i < sz; i++) {
        const cur = horiz ? modules[a]![i]! : modules[i]![a]!;
        if (cur === prev) {
          run++;
        } else {
          if (run >= 5) penalty += run - 2;
          run = 1;
          prev = cur;
        }
      }
      if (run >= 5) penalty += run - 2;
    }
  }

  // Rule 2 — 2×2 same-color blocks
  for (let r = 0; r < sz - 1; r++) {
    for (let c = 0; c < sz - 1; c++) {
      const d = modules[r]![c]!;
      if (d === modules[r]![c + 1]! && d === modules[r + 1]![c]! && d === modules[r + 1]![c + 1]!) {
        penalty += 3;
      }
    }
  }

  // Rule 3 — finder-like sequences
  const P1 = [1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0];
  const P2 = [0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1];
  for (let a = 0; a < sz; a++) {
    for (let b = 0; b <= sz - 11; b++) {
      let mH1 = true, mH2 = true, mV1 = true, mV2 = true;
      for (let k = 0; k < 11; k++) {
        const bH = modules[a]![b + k]! ? 1 : 0;
        const bV = modules[b + k]![a]! ? 1 : 0;
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

  // Rule 4 — dark proportion deviation
  let dark = 0;
  for (let r = 0; r < sz; r++) for (let c = 0; c < sz; c++) if (modules[r]![c]!) dark++;
  const darkPct = (dark / (sz * sz)) * 100;
  const prev5 = Math.floor(darkPct / 5) * 5;
  penalty += Math.min(Math.abs(prev5 - 50), Math.abs(prev5 + 5 - 50)) / 5 * 10;

  return penalty;
}

// ─────────────────────────────────────────────────────────────────────────────
// Full encode pipeline
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Encode an input string to a Micro QR Code ModuleGrid.
 *
 * The auto-selection logic (when version/ecc are omitted) picks the
 * smallest symbol+ECC combination that fits the input. This means:
 *
 * - "1" → M1 (11×11, detection only)
 * - "12345" → M1 (exactly fills 5-digit numeric capacity)
 * - "HELLO" → M2-L (5 alphanumeric chars)
 * - "hello" → M3-L (5 bytes, lowercase not in alphanumeric set)
 * - "https://a.b" → M4-L (11 bytes)
 *
 * The returned grid is ready for `layout()` or `renderSvg()`.
 */
export function encode(input: string, options?: {
  version?: MicroQRVersion;
  ecc?: MicroQREccLevel;
}): ModuleGrid {
  const cfg = selectConfig(input, options?.version, options?.ecc);

  // 1. Build data codewords
  const dataCW = buildDataCodewords(input, cfg);

  // 2. Compute RS ECC
  const eccCW = rsEncode(dataCW, cfg.eccCW);

  // 3. Flatten to bit stream
  // For M1: data bytes are [b0, b1, b2] where b2 has data only in upper nibble.
  // We emit 7.5 bytes = 60 bits but only the first 20 data bits matter;
  // the remaining bits are ECC.
  const finalCW = [...dataCW, ...eccCW];
  const bits: boolean[] = [];
  for (let cwIdx = 0; cwIdx < finalCW.length; cwIdx++) {
    const cw = finalCW[cwIdx]!;
    // For M1: last data codeword (index 2) contributes only 4 bits
    const bitsInCW = cfg.m1HalfCW && cwIdx === cfg.dataCW - 1 ? 4 : 8;
    for (let b = bitsInCW - 1; b >= 0; b--) {
      bits.push(((cw >> (b + (8 - bitsInCW))) & 1) === 1);
    }
  }

  // 4. Build grid
  const grid = buildGrid(cfg);

  // 5. Place bits
  placeBits(grid, bits);

  // 6. Evaluate all 4 masks, pick lowest penalty
  let bestMask = 0;
  let bestPenalty = Infinity;
  for (let m = 0; m < 4; m++) {
    const masked = applyMask(grid.modules, grid.reserved, cfg.size, m);
    const fmtBits = FORMAT_TABLE[cfg.symbolIndicator]![m]!;
    // Write format info into a temporary copy for penalty evaluation
    const tmpModules = masked.map((row) => [...row]);
    const tmpGrid: WorkGrid = { size: cfg.size, modules: tmpModules, reserved: grid.reserved };
    writeFormatInfo(tmpGrid, fmtBits);
    const p = computePenalty(tmpGrid.modules, cfg.size);
    if (p < bestPenalty) {
      bestPenalty = p;
      bestMask = m;
    }
  }

  // 7. Apply best mask and write final format info
  const finalModules = applyMask(grid.modules, grid.reserved, cfg.size, bestMask);
  const finalGrid: WorkGrid = { size: cfg.size, modules: finalModules, reserved: grid.reserved };
  writeFormatInfo(finalGrid, FORMAT_TABLE[cfg.symbolIndicator]![bestMask]!);

  return {
    rows: cfg.size,
    cols: cfg.size,
    modules: finalModules,
    moduleShape: "square",
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Layout and rendering helpers
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Convert a ModuleGrid to a pixel-resolved PaintScene.
 *
 * Delegates to `barcode-2d`'s `layout()`. The quiet zone defaults to 2
 * modules (half of regular QR's 4-module quiet zone) because Micro QR
 * only requires 2 modules on all sides.
 *
 * Override with `config.quietZone` if needed.
 */
export function mqrLayout(
  grid: ModuleGrid,
  config?: Partial<Barcode2DLayoutConfig>,
): PaintScene {
  return layout(grid, { quietZoneModules: 2, ...config });
}

/**
 * Encode a string and immediately convert to a PaintScene.
 *
 * Convenience wrapper for the common case of encoding and then rendering.
 */
export function encodeAndLayout(
  input: string,
  options?: { version?: MicroQRVersion; ecc?: MicroQREccLevel },
  config?: Partial<Barcode2DLayoutConfig>,
): PaintScene {
  return mqrLayout(encode(input, options), config);
}

/**
 * Encode with per-module role annotations (for interactive visualizers).
 *
 * v0.1.0: returns the encoded grid with null annotations.
 */
export function explain(
  input: string,
  options?: { version?: MicroQRVersion; ecc?: MicroQREccLevel },
): AnnotatedModuleGrid {
  const grid = encode(input, options);
  return {
    ...grid,
    annotations: Array.from({ length: grid.rows }, () => new Array(grid.cols).fill(null)),
  };
}
