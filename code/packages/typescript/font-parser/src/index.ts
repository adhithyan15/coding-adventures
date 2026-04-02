/**
 * @module @coding-adventures/font-parser
 *
 * Metrics-only OpenType/TrueType font parser.
 *
 * ## What this module does
 *
 * An OpenType font file is a binary *table database*. The first bytes are a
 * directory listing every named table and where to find it. This module reads
 * the subset of tables needed to **measure text** without touching the OS
 * font stack or browser APIs.
 *
 * | Table  | What we read                                       |
 * |--------|----------------------------------------------------|
 * | `head` | `unitsPerEm` — the coordinate system scale         |
 * | `hhea` | ascender / descender / lineGap / numberOfHMetrics  |
 * | `maxp` | `numGlyphs`                                        |
 * | `cmap` | Format 4 Unicode BMP → glyph ID mapping            |
 * | `hmtx` | advance width + left side bearing per glyph        |
 * | `kern` | Format 0 kerning pairs                             |
 * | `name` | family name, subfamily name (UTF-16 BE)            |
 * | `OS/2` | typographic metrics + xHeight/capHeight (v ≥ 2)    |
 *
 * ## Usage
 *
 * ```typescript
 * import { load, fontMetrics, glyphId, glyphMetrics, kerning } from "@coding-adventures/font-parser";
 * import { readFileSync } from "node:fs";
 *
 * const bytes = new Uint8Array(readFileSync("Inter-Regular.ttf"));
 * const font = load(bytes);
 *
 * const m = fontMetrics(font);
 * console.log(m.unitsPerEm);   // 2048
 * console.log(m.familyName);   // "Inter"
 *
 * const gidA = glyphId(font, 0x0041)!; // 'A'
 * const gidV = glyphId(font, 0x0056)!; // 'V'
 * console.log(kerning(font, gidA, gidV)); // negative for A+V
 * ```
 */

// ─────────────────────────────────────────────────────────────────────────────
// Error class
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Thrown when font bytes cannot be parsed.
 *
 * The `kind` discriminant lets callers handle specific failure modes without
 * string-matching on the message.
 */
export class FontError extends Error {
  constructor(
    public readonly kind:
      | "InvalidMagic"
      | "InvalidHeadMagic"
      | "TableNotFound"
      | "BufferTooShort"
      | "UnsupportedCmapFormat",
    message: string,
  ) {
    super(message);
    this.name = "FontError";
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public metric types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Global typographic metrics extracted from a font file.
 *
 * All integer fields are in **design units**. Convert to pixels:
 *
 * ```text
 * pixels = designUnits * fontSizePx / unitsPerEm
 * ```
 *
 * For Inter Regular at 16px: `16 / 2048 = 0.0078125 px per design unit`.
 */
export interface FontMetrics {
  /** Design units per em square. Inter Regular = 2048. */
  unitsPerEm: number;
  /** Distance from baseline to top of tallest glyph (positive). */
  ascender: number;
  /** Distance from baseline to bottom of deepest glyph (negative). */
  descender: number;
  /** Extra spacing between lines (often 0). */
  lineGap: number;
  /**
   * Height of lowercase 'x' above the baseline in design units.
   * `null` if the OS/2 table is absent or has version < 2.
   */
  xHeight: number | null;
  /**
   * Height of uppercase 'H' above the baseline in design units.
   * `null` if the OS/2 table is absent or has version < 2.
   */
  capHeight: number | null;
  /** Total number of glyphs in the font. */
  numGlyphs: number;
  /** Font family name, e.g. `"Inter"`. */
  familyName: string;
  /** Font subfamily / style name, e.g. `"Regular"`, `"Bold Italic"`. */
  subfamilyName: string;
}

/**
 * Per-glyph horizontal metrics, in design units.
 */
export interface GlyphMetrics {
  /**
   * Horizontal distance to advance the pen after this glyph.
   *
   * For proportional fonts each glyph has its own value.
   * For monospace fonts all glyphs share the same value.
   */
  advanceWidth: number;
  /**
   * Space between the pen position and the left ink edge.
   *
   * Positive = ink starts to the right of the pen (normal).
   * Negative = ink bleeds left of the pen (rare).
   */
  leftSideBearing: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal parsed table offsets
// ─────────────────────────────────────────────────────────────────────────────

/** Pre-parsed table directory — absolute byte offsets for each table. */
interface Tables {
  head: number;
  hhea: number;
  maxp: number;
  cmap: number;
  hmtx: number;
  kern: number | null;
  name: number | null;
  os2: number | null;
}

// ─────────────────────────────────────────────────────────────────────────────
// FontFile — the parsed font handle
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Opaque handle to a parsed font file.
 *
 * Created by {@link load}. Pass to the individual metric functions.
 * Holds a copy of the font bytes and the pre-parsed table directory.
 */
export interface FontFile {
  /** @internal */
  readonly _data: DataView;
  /** @internal */
  readonly _tables: Tables;
}

// ─────────────────────────────────────────────────────────────────────────────
// Big-endian reading helpers
// ─────────────────────────────────────────────────────────────────────────────
//
// All multi-byte integers in OpenType are big-endian (network byte order).
// DataView's methods already handle endianness — we just pass `false` (BE).
//
// Every helper bounds-checks via DataView's built-in range validation which
// throws a RangeError on out-of-bounds access.

function readU8(dv: DataView, offset: number): number {
  try {
    return dv.getUint8(offset);
  } catch {
    throw new FontError("BufferTooShort", `read u8 at offset ${offset} out of bounds`);
  }
}

/** Read a 16-bit big-endian unsigned integer. */
function readU16(dv: DataView, offset: number): number {
  try {
    return dv.getUint16(offset, false); // false = big-endian
  } catch {
    throw new FontError("BufferTooShort", `read u16 at offset ${offset} out of bounds`);
  }
}

/** Read a 16-bit big-endian signed integer (reinterpret the bits). */
function readI16(dv: DataView, offset: number): number {
  try {
    return dv.getInt16(offset, false);
  } catch {
    throw new FontError("BufferTooShort", `read i16 at offset ${offset} out of bounds`);
  }
}

/** Read a 32-bit big-endian unsigned integer. */
function readU32(dv: DataView, offset: number): number {
  try {
    return dv.getUint32(offset, false);
  } catch {
    throw new FontError("BufferTooShort", `read u32 at offset ${offset} out of bounds`);
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Table directory parsing
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Find the absolute byte offset of a named table.
 *
 * The table directory starts at byte 12 (after the 12-byte offset table).
 * Each record is 16 bytes: tag (4) + checksum (4) + offset (4) + length (4).
 *
 * We do a linear scan. For numTables ≈ 20–30 this is negligible.
 */
function findTable(
  dv: DataView,
  numTables: number,
  tag: string,
): number | null {
  const tagBytes = tag.split("").map((c) => c.charCodeAt(0));
  for (let i = 0; i < numTables; i++) {
    const rec = 12 + i * 16;
    if (
      dv.getUint8(rec) === tagBytes[0] &&
      dv.getUint8(rec + 1) === tagBytes[1] &&
      dv.getUint8(rec + 2) === tagBytes[2] &&
      dv.getUint8(rec + 3) === tagBytes[3]
    ) {
      return dv.getUint32(rec + 8, false); // absolute offset
    }
  }
  return null;
}

function requireTable(
  dv: DataView,
  numTables: number,
  tag: string,
): number {
  const off = findTable(dv, numTables, tag);
  if (off === null) {
    throw new FontError(
      "TableNotFound",
      `required table '${tag}' not found in font directory`,
    );
  }
  return off;
}

// ─────────────────────────────────────────────────────────────────────────────
// load — the entry point
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Parse raw font bytes and return a {@link FontFile} handle.
 *
 * @param bytes - Raw bytes from a `.ttf` or `.otf` file.
 * @throws {@link FontError} if the bytes are not a valid OpenType/TrueType font
 *   or if a required table is missing.
 *
 * @example
 * ```typescript
 * const bytes = new Uint8Array(await fs.readFile("Inter-Regular.ttf"));
 * const font = load(bytes);
 * ```
 */
export function load(bytes: Uint8Array): FontFile {
  if (bytes.length < 12) {
    throw new FontError("BufferTooShort", "font buffer is too small to be a valid font");
  }

  // Copy bytes into an ArrayBuffer so DataView can work on it.
  // This also owns the data, so the caller's buffer can be GC'd.
  const buf = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
  const dv = new DataView(buf);

  // Validate sfntVersion magic.
  const sfntVersion = readU32(dv, 0);
  if (sfntVersion !== 0x00010000 && sfntVersion !== 0x4f54544f) {
    throw new FontError(
      "InvalidMagic",
      `invalid sfntVersion 0x${sfntVersion.toString(16).padStart(8, "0")}; expected 0x00010000 or 0x4F54544F`,
    );
  }

  const numTables = readU16(dv, 4);

  const tables: Tables = {
    head: requireTable(dv, numTables, "head"),
    hhea: requireTable(dv, numTables, "hhea"),
    maxp: requireTable(dv, numTables, "maxp"),
    cmap: requireTable(dv, numTables, "cmap"),
    hmtx: requireTable(dv, numTables, "hmtx"),
    kern: findTable(dv, numTables, "kern"),
    name: findTable(dv, numTables, "name"),
    os2: findTable(dv, numTables, "OS/2"),
  };

  // Validate the head.magicNumber sentinel.
  // Offset within head table: 12 bytes in.
  const headMagic = readU32(dv, tables.head + 12);
  if (headMagic !== 0x5f0f3cf5) {
    throw new FontError(
      "InvalidHeadMagic",
      `invalid head.magicNumber 0x${headMagic.toString(16).padStart(8, "0")}; expected 0x5F0F3CF5`,
    );
  }

  return { _data: dv, _tables: tables };
}

// ─────────────────────────────────────────────────────────────────────────────
// fontMetrics
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Return global typographic metrics for the font.
 *
 * Prefers `OS/2` typographic values over `hhea` values when the OS/2 table
 * is present, matching the behaviour of modern renderers.
 */
export function fontMetrics(font: FontFile): FontMetrics {
  const dv = font._data;
  const t = font._tables;

  // ── head ─────────────────────────────────────────────────────────────────
  // unitsPerEm at offset 18 from table start.
  const unitsPerEm = readU16(dv, t.head + 18);

  // ── hhea ─────────────────────────────────────────────────────────────────
  // Fallback values when OS/2 is absent.
  const hheaAscender  = readI16(dv, t.hhea + 4);
  const hheaDescender = readI16(dv, t.hhea + 6);
  const hheaLineGap   = readI16(dv, t.hhea + 8);

  // ── maxp ─────────────────────────────────────────────────────────────────
  // numGlyphs at offset 4, regardless of maxp version.
  const numGlyphs = readU16(dv, t.maxp + 4);

  // ── OS/2 ─────────────────────────────────────────────────────────────────
  let ascender  = hheaAscender;
  let descender = hheaDescender;
  let lineGap   = hheaLineGap;
  let xHeight: number | null = null;
  let capHeight: number | null = null;

  if (t.os2 !== null) {
    const base = t.os2;
    const version = readU16(dv, base);
    ascender  = readI16(dv, base + 68);
    descender = readI16(dv, base + 70);
    lineGap   = readI16(dv, base + 72);
    if (version >= 2) {
      xHeight   = readI16(dv, base + 86);
      capHeight = readI16(dv, base + 88);
    }
  }

  // ── name ─────────────────────────────────────────────────────────────────
  const familyName    = readNameString(dv, t.name, 1) ?? "(unknown)";
  const subfamilyName = readNameString(dv, t.name, 2) ?? "(unknown)";

  return {
    unitsPerEm,
    ascender,
    descender,
    lineGap,
    xHeight,
    capHeight,
    numGlyphs,
    familyName,
    subfamilyName,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// glyphId — cmap Format 4 lookup
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Map a Unicode codepoint to a glyph ID.
 *
 * Only covers the Basic Multilingual Plane (codepoints 0x0000–0xFFFF).
 * Returns `null` if the codepoint is not in the font or is above 0xFFFF.
 *
 * ## Algorithm
 *
 * Format 4 stores BMP codepoints as segments `[startCode, endCode]`.
 * Binary-search `endCode[]` for the first entry ≥ `cp`, verify `startCode`,
 * then resolve via direct delta or the idRangeOffset pointer trick.
 */
export function glyphId(font: FontFile, codepoint: number): number | null {
  // Format 4 only covers BMP.
  if (codepoint > 0xffff || codepoint < 0) return null;
  const cp = codepoint;
  const dv = font._data;
  const cmapOff = font._tables.cmap;

  // ── Find the Format 4 subtable ──────────────────────────────────────────
  const numSubtables = readU16(dv, cmapOff + 2);
  let subtableAbs: number | null = null;

  for (let i = 0; i < numSubtables; i++) {
    const rec = cmapOff + 4 + i * 8;
    const platformId = readU16(dv, rec);
    const encodingId = readU16(dv, rec + 2);
    const subOffset  = readU32(dv, rec + 4);

    if (platformId === 3 && encodingId === 1) {
      subtableAbs = cmapOff + subOffset;
      break; // best match
    }
    if (platformId === 0 && subtableAbs === null) {
      subtableAbs = cmapOff + subOffset;
    }
  }

  if (subtableAbs === null) return null;

  // Verify Format 4.
  const format = readU16(dv, subtableAbs);
  if (format !== 4) return null;

  // ── Parse Format 4 header ───────────────────────────────────────────────
  const segCountX2 = readU16(dv, subtableAbs + 6);
  const segCount   = segCountX2 / 2;

  // Array absolute offsets:
  const endCodesBase       = subtableAbs + 14;
  const startCodesBase     = subtableAbs + 16 + segCount * 2;
  const idDeltaBase        = subtableAbs + 16 + segCount * 4;
  const idRangeOffsetBase  = subtableAbs + 16 + segCount * 6;

  // ── Binary search on endCode[] ──────────────────────────────────────────
  let lo = 0;
  let hi = segCount;
  while (lo < hi) {
    const mid = (lo + hi) >>> 1; // unsigned right shift = floor divide
    const endCode = readU16(dv, endCodesBase + mid * 2);
    if (endCode < cp) {
      lo = mid + 1;
    } else {
      hi = mid;
    }
  }

  if (lo >= segCount) return null;

  const endCode   = readU16(dv, endCodesBase   + lo * 2);
  const startCode = readU16(dv, startCodesBase + lo * 2);

  if (cp > endCode || cp < startCode) return null;

  const idDelta        = readI16(dv, idDeltaBase       + lo * 2);
  const idRangeOffset  = readU16(dv, idRangeOffsetBase + lo * 2);

  let glyph: number;
  if (idRangeOffset === 0) {
    // Direct delta. Use 32-bit arithmetic, then mask to 16 bits.
    //
    // JavaScript's bitwise operations are signed 32-bit, so we use
    // `& 0xFFFF` to wrap correctly regardless of sign.
    glyph = (cp + idDelta) & 0xffff;
  } else {
    // Indirect lookup. idRangeOffset is a byte offset from the address of
    // idRangeOffset[lo] itself, pointing into glyphIdArray.
    //
    // Absolute byte address of glyphIdArray[(cp - startCode)] entry:
    //   (idRangeOffsetBase + lo*2) + idRangeOffset + (cp - startCode)*2
    //
    // This mirrors the C pointer expression:
    //   *(idRangeOffset[i]/2 + (c - startCode[i]) + &idRangeOffset[i])
    const absOff = (idRangeOffsetBase + lo * 2) + idRangeOffset + (cp - startCode) * 2;
    glyph = readU16(dv, absOff);
  }

  return glyph === 0 ? null : glyph;
}

// ─────────────────────────────────────────────────────────────────────────────
// glyphMetrics — hmtx lookup
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Return horizontal metrics for a glyph ID.
 *
 * Returns `null` if `glyphId` is out of range (≥ numGlyphs).
 *
 * ## hmtx layout
 *
 * ```text
 * hMetrics[0 .. numberOfHMetrics]    — (advanceWidth: u16, lsb: i16) × N
 * leftSideBearings[0 .. numGlyphs - numberOfHMetrics]  — lsb (i16) only
 * ```
 *
 * Glyphs ≥ numberOfHMetrics share the last advance width from the first section.
 */
export function glyphMetrics(font: FontFile, gid: number): GlyphMetrics | null {
  const dv = font._data;
  const t  = font._tables;

  const numGlyphs    = readU16(dv, t.maxp + 4);
  const numHMetrics  = readU16(dv, t.hhea + 34);
  const hmtxOff      = t.hmtx;

  if (gid < 0 || gid >= numGlyphs) return null;

  if (gid < numHMetrics) {
    // Full record: (advanceWidth u16, lsb i16).
    const base = hmtxOff + gid * 4;
    return {
      advanceWidth:    readU16(dv, base),
      leftSideBearing: readI16(dv, base + 2),
    };
  } else {
    // Shared advance: last advance + per-glyph lsb.
    const lastAdvance = readU16(dv, hmtxOff + (numHMetrics - 1) * 4);
    const lsbOff = hmtxOff + numHMetrics * 4 + (gid - numHMetrics) * 2;
    return {
      advanceWidth:    lastAdvance,
      leftSideBearing: readI16(dv, lsbOff),
    };
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// kerning — kern Format 0 lookup
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Return the kerning adjustment (design units) for a glyph pair.
 *
 * Returns `0` if the font has no `kern` table or the pair is not found.
 * Negative = tighter spacing; positive = wider.
 *
 * ## Algorithm
 *
 * Format 0 stores pairs sorted ascending by a 32-bit composite key:
 * `(left << 16) | right`. Binary-search for the key.
 */
export function kerning(font: FontFile, left: number, right: number): number {
  const dv = font._data;

  if (font._tables.kern === null) return 0;
  const kernOff = font._tables.kern;

  const nTables = readU16(dv, kernOff + 2);

  let pos = kernOff + 4;
  for (let t = 0; t < nTables; t++) {
    if (pos + 6 > dv.byteLength) break;
    const length   = readU16(dv, pos + 2);
    const coverage = readU16(dv, pos + 4);
    const subFormat = coverage >> 8;

    if (subFormat === 0) {
      const nPairs   = readU16(dv, pos + 6);
      const pairsBase = pos + 14; // 6 (subtable hdr) + 8 (format0 hdr)

      // 32-bit composite key for binary search.
      // JavaScript numbers are 64-bit floats, but bit shifts are 32-bit signed.
      // `>>> 0` converts to unsigned 32-bit; we use multiplication to avoid
      // sign issues with large glyph IDs.
      const target = (left * 65536 + right) >>> 0;

      let lo = 0;
      let hi = nPairs;
      while (lo < hi) {
        const mid = (lo + hi) >>> 1;
        const pairOff  = pairsBase + mid * 6;
        const pairLeft  = readU16(dv, pairOff);
        const pairRight = readU16(dv, pairOff + 2);
        const key = (pairLeft * 65536 + pairRight) >>> 0;

        if (key === target) {
          return readI16(dv, pairOff + 4);
        } else if (key < target) {
          lo = mid + 1;
        } else {
          hi = mid;
        }
      }
    }

    pos += length;
  }

  return 0;
}

// ─────────────────────────────────────────────────────────────────────────────
// name table reading
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Read a string from the `name` table by nameID.
 *
 * Prefers platform 3 / encoding 1 (Windows Unicode BMP, UTF-16 BE).
 * Falls back to platform 0 (Unicode) if absent.
 *
 * Strings are decoded from UTF-16 BE. Unpaired surrogates become U+FFFD.
 */
function readNameString(
  dv: DataView,
  nameOff: number | null,
  nameId: number,
): string | null {
  if (nameOff === null) return null;
  const base = nameOff;

  const count        = readU16(dv, base + 2);
  const stringOffset = readU16(dv, base + 4);

  let best: { platformId: number; start: number; length: number } | null = null;

  for (let i = 0; i < count; i++) {
    const rec        = base + 6 + i * 12;
    const platformId = readU16(dv, rec);
    const encodingId = readU16(dv, rec + 2);
    const nid        = readU16(dv, rec + 6);
    const len        = readU16(dv, rec + 8);
    const strOff     = readU16(dv, rec + 10);

    if (nid !== nameId) continue;

    const absStart = base + stringOffset + strOff;

    if (platformId === 3 && encodingId === 1) {
      best = { platformId: 3, start: absStart, length: len };
      break; // best possible match
    }
    if (platformId === 0 && best === null) {
      best = { platformId: 0, start: absStart, length: len };
    }
  }

  if (best === null) return null;

  // Decode UTF-16 BE: pairs of bytes → u16 code units → string.
  //
  // TextDecoder is available in Node.js ≥ 11, all browsers, and
  // Deno. For environments without it, we provide a manual fallback.
  const raw = new Uint8Array(dv.buffer, best.start, best.length);
  return decodeUtf16Be(raw);
}

/**
 * Decode a UTF-16 Big-Endian byte array to a JavaScript string.
 *
 * In a browser or Node.js environment TextDecoder handles this in one call.
 * We provide a pure JS fallback for environments like bare WASM runtimes that
 * may not ship TextDecoder.
 */
function decodeUtf16Be(bytes: Uint8Array): string {
  // Fast path: use TextDecoder when available (browser, Node ≥ 11, Deno).
  if (typeof TextDecoder !== "undefined") {
    return new TextDecoder("utf-16be").decode(bytes);
  }

  // Fallback: manual UTF-16 BE decode.
  // Read pairs of bytes as big-endian u16 code units, then use
  // String.fromCharCode. Surrogate pairs are decoded to supplementary
  // characters. Unpaired surrogates become U+FFFD.
  const codeUnits: number[] = [];
  for (let i = 0; i + 1 < bytes.length; i += 2) {
    codeUnits.push((bytes[i] << 8) | bytes[i + 1]);
  }

  let result = "";
  let i = 0;
  while (i < codeUnits.length) {
    const cu = codeUnits[i];
    if (cu >= 0xd800 && cu <= 0xdbff) {
      // High surrogate — look for low surrogate.
      const next = codeUnits[i + 1];
      if (next !== undefined && next >= 0xdc00 && next <= 0xdfff) {
        const codePoint = 0x10000 + ((cu - 0xd800) << 10) + (next - 0xdc00);
        result += String.fromCodePoint(codePoint);
        i += 2;
      } else {
        result += "\uFFFD"; // unpaired surrogate
        i += 1;
      }
    } else {
      result += String.fromCharCode(cu);
      i += 1;
    }
  }
  return result;
}
