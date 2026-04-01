/**
 * Tests for @coding-adventures/font-parser
 *
 * Tests fall into two categories:
 * 1. Integration tests against Inter Regular v4.0 (real font, known values)
 * 2. Unit tests against synthetic byte buffers (verifies kern/cmap logic
 *    without depending on a specific font's optional tables)
 */

import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, it, expect } from "vitest";

import {
  load,
  fontMetrics,
  glyphId,
  glyphMetrics,
  kerning,
  FontError,
  type FontFile,
} from "../src/index.js";

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Load Inter Regular from the shared fixtures directory.
 *
 * Inter is at: code/fixtures/fonts/Inter-Regular.ttf
 * This test file is at: code/packages/typescript/font-parser/tests/
 * So we go up 4 levels.
 */
function loadInterRegular(): Uint8Array {
  const thisDir = dirname(fileURLToPath(import.meta.url));
  const fontPath = join(thisDir, "../../../../fixtures/fonts/Inter-Regular.ttf");
  return new Uint8Array(readFileSync(fontPath));
}

/** Write a u16 big-endian into a DataView. */
function writeU16(buf: DataView, offset: number, value: number): void {
  buf.setUint16(offset, value, false);
}
/** Write an i16 big-endian into a DataView. */
function writeI16(buf: DataView, offset: number, value: number): void {
  buf.setInt16(offset, value, false);
}
/** Write a u32 big-endian into a DataView. */
function writeU32(buf: DataView, offset: number, value: number): void {
  buf.setUint32(offset, value, false);
}
/** Write 4 ASCII bytes as a tag. */
function writeTag(dv: DataView, offset: number, tag: string): void {
  for (let i = 0; i < 4; i++) dv.setUint8(offset + i, tag.charCodeAt(i));
}

/**
 * Build a minimal valid synthetic font with a kern Format 0 table.
 *
 * Tables present: head, hhea, maxp, cmap (Format 4, single sentinel segment),
 * hmtx (5 full records), kern (Format 0 with the given pairs).
 *
 * This lets us test kern binary search without an external font file.
 */
function buildSyntheticFont(pairs: Array<[left: number, right: number, value: number]>): Uint8Array {
  // 6 tables: cmap, head, hhea, hmtx, kern, maxp
  const numTables = 6;
  const dirSize = 12 + numTables * 16; // offset table + table records

  // Fixed table sizes
  const headLen = 54;
  const hheaLen = 36;
  const maxpLen = 6;
  // cmap: 4 (index hdr) + 8 (enc record) + 24 (Format 4, segCount=1) = 36
  const cmapLen = 36;
  const hmtxLen = 5 * 4; // 5 full hMetric records
  // kern: 4 (kern hdr) + 6 (subtable hdr) + 8 (format0 hdr) + pairs*6
  const nPairs = pairs.length;
  const kernLen = 4 + 6 + 8 + nPairs * 6;

  // Absolute table offsets
  const headOff = dirSize;
  const hheaOff = headOff + headLen;
  const maxpOff = hheaOff + hheaLen;
  const cmapOff = maxpOff + maxpLen;
  const hmtxOff = cmapOff + cmapLen;
  const kernOff = hmtxOff + hmtxLen;

  const totalSize = kernOff + kernLen;
  const ab = new ArrayBuffer(totalSize);
  const dv = new DataView(ab);

  // ── Offset Table ─────────────────────────────────────────────────────────
  writeU32(dv, 0,  0x00010000);  // sfntVersion
  writeU16(dv, 4,  numTables);
  writeU16(dv, 6,  64);          // searchRange (placeholder)
  writeU16(dv, 8,  2);           // entrySelector
  writeU16(dv, 10, 32);          // rangeShift

  // ── Table Records (sorted by tag: cmap < head < hhea < hmtx < kern < maxp) ─
  let rec = 12;
  const writeRecord = (tag: string, off: number, len: number) => {
    writeTag(dv, rec, tag);
    writeU32(dv, rec + 4, 0);   // checksum (skip)
    writeU32(dv, rec + 8, off);
    writeU32(dv, rec + 12, len);
    rec += 16;
  };
  writeRecord("cmap", cmapOff, cmapLen);
  writeRecord("head", headOff, headLen);
  writeRecord("hhea", hheaOff, hheaLen);
  writeRecord("hmtx", hmtxOff, hmtxLen);
  writeRecord("kern", kernOff, kernLen);
  writeRecord("maxp", maxpOff, maxpLen);

  // ── head table ───────────────────────────────────────────────────────────
  let p = headOff;
  writeU32(dv, p,      0x00010000); // version
  writeU32(dv, p + 4,  0x00010000); // fontRevision
  writeU32(dv, p + 8,  0);          // checksumAdjustment
  writeU32(dv, p + 12, 0x5f0f3cf5); // magicNumber ← sentinel
  writeU16(dv, p + 16, 0);           // flags
  writeU16(dv, p + 18, 1000);        // unitsPerEm
  // created + modified (8 bytes each) = zeros
  writeU16(dv, p + 44, 0);  // macStyle
  writeU16(dv, p + 46, 8);  // lowestRecPPEM
  writeI16(dv, p + 48, 2);  // fontDirectionHint
  writeI16(dv, p + 50, 0);  // indexToLocFormat
  writeI16(dv, p + 52, 0);  // glyphDataFormat

  // ── hhea table ───────────────────────────────────────────────────────────
  p = hheaOff;
  writeU32(dv, p,      0x00010000); // version
  writeI16(dv, p + 4,  800);        // ascender
  writeI16(dv, p + 6,  -200);       // descender
  writeI16(dv, p + 8,  0);          // lineGap
  writeU16(dv, p + 10, 1000);       // advanceWidthMax
  // rest zeros through offset 32
  writeI16(dv, p + 32, 0);          // metricDataFormat
  writeU16(dv, p + 34, 5);          // numberOfHMetrics

  // ── maxp table ───────────────────────────────────────────────────────────
  p = maxpOff;
  writeU32(dv, p,     0x00005000); // version 0.5
  writeU16(dv, p + 4, 5);          // numGlyphs

  // ── cmap table ───────────────────────────────────────────────────────────
  // Index header: version=0, numSubtables=1
  p = cmapOff;
  writeU16(dv, p, 0);               // version
  writeU16(dv, p + 2, 1);          // numSubtables
  // Encoding record: platform 3, encoding 1, subtable at offset 12
  writeU16(dv, p + 4, 3);           // platformID
  writeU16(dv, p + 6, 1);           // encodingID
  writeU32(dv, p + 8, 12);          // subtable offset from cmap start
  // Format 4 subtable with segCount=1 (just the sentinel segment)
  const sub = p + 12;
  writeU16(dv, sub,      4);        // format
  writeU16(dv, sub + 2,  24);       // length (24 bytes for segCount=1)
  writeU16(dv, sub + 4,  0);        // language
  writeU16(dv, sub + 6,  2);        // segCountX2 = 1*2
  writeU16(dv, sub + 8,  2);        // searchRange
  writeU16(dv, sub + 10, 0);        // entrySelector
  writeU16(dv, sub + 12, 0);        // rangeShift
  writeU16(dv, sub + 14, 0xffff);   // endCode[0] sentinel
  writeU16(dv, sub + 16, 0);        // reservedPad
  writeU16(dv, sub + 18, 0xffff);   // startCode[0] sentinel
  writeI16(dv, sub + 20, 1);        // idDelta[0]
  writeU16(dv, sub + 22, 0);        // idRangeOffset[0]

  // ── hmtx table ───────────────────────────────────────────────────────────
  p = hmtxOff;
  for (let i = 0; i < 5; i++) {
    writeU16(dv, p + i * 4,     600);  // advanceWidth
    writeI16(dv, p + i * 4 + 2, 50);   // lsb
  }

  // ── kern table ───────────────────────────────────────────────────────────
  p = kernOff;
  writeU16(dv, p, 0);               // version
  writeU16(dv, p + 2, 1);          // nTables
  // Subtable header
  const subLen = 6 + 8 + nPairs * 6;
  writeU16(dv, p + 4, 0);           // subtable version
  writeU16(dv, p + 6, subLen);      // subtable length
  writeU16(dv, p + 8, 0x0001);      // coverage: format 0, horizontal
  // Format 0 header
  writeU16(dv, p + 10, nPairs);     // nPairs
  writeU16(dv, p + 12, 0);          // searchRange
  writeU16(dv, p + 14, 0);          // entrySelector
  writeU16(dv, p + 16, 0);          // rangeShift
  // Kern pairs sorted by composite key
  const sorted = [...pairs].sort(([la, ra], [lb, rb]) =>
    (la * 65536 + ra) - (lb * 65536 + rb),
  );
  let pairOff = p + 18;
  for (const [left, right, value] of sorted) {
    writeU16(dv, pairOff,     left);
    writeU16(dv, pairOff + 2, right);
    writeI16(dv, pairOff + 4, value);
    pairOff += 6;
  }

  return new Uint8Array(ab);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: load()
// ─────────────────────────────────────────────────────────────────────────────

describe("load()", () => {
  it("throws BufferTooShort for an empty buffer", () => {
    expect(() => load(new Uint8Array(0))).toThrowError(FontError);
    try {
      load(new Uint8Array(0));
    } catch (e) {
      expect(e).toBeInstanceOf(FontError);
      expect((e as FontError).kind).toBe("BufferTooShort");
    }
  });

  it("throws InvalidMagic for a buffer with wrong sfntVersion", () => {
    const buf = new Uint8Array(256).fill(0);
    new DataView(buf.buffer).setUint32(0, 0xdeadbeef, false);
    try {
      load(buf);
      expect.fail("should have thrown");
    } catch (e) {
      expect(e).toBeInstanceOf(FontError);
      expect((e as FontError).kind).toBe("InvalidMagic");
    }
  });

  it("loads Inter Regular without error", () => {
    const bytes = loadInterRegular();
    expect(() => load(bytes)).not.toThrow();
  });

  it("loads the synthetic font without error", () => {
    const bytes = buildSyntheticFont([[1, 2, -140]]);
    expect(() => load(bytes)).not.toThrow();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Tests: fontMetrics()
// ─────────────────────────────────────────────────────────────────────────────

describe("fontMetrics()", () => {
  let font: FontFile;
  beforeEach(() => {
    font = load(loadInterRegular());
  });

  it("returns unitsPerEm = 2048 for Inter Regular", () => {
    expect(fontMetrics(font).unitsPerEm).toBe(2048);
  });

  it("returns familyName = 'Inter'", () => {
    expect(fontMetrics(font).familyName).toBe("Inter");
  });

  it("returns subfamilyName = 'Regular'", () => {
    expect(fontMetrics(font).subfamilyName).toBe("Regular");
  });

  it("returns positive ascender", () => {
    expect(fontMetrics(font).ascender).toBeGreaterThan(0);
  });

  it("returns non-positive descender", () => {
    expect(fontMetrics(font).descender).toBeLessThanOrEqual(0);
  });

  it("returns positive numGlyphs", () => {
    expect(fontMetrics(font).numGlyphs).toBeGreaterThan(100);
  });

  it("returns non-null xHeight (OS/2 version ≥ 2)", () => {
    expect(fontMetrics(font).xHeight).not.toBeNull();
    expect(fontMetrics(font).xHeight!).toBeGreaterThan(0);
  });

  it("returns non-null capHeight (OS/2 version ≥ 2)", () => {
    expect(fontMetrics(font).capHeight).not.toBeNull();
    expect(fontMetrics(font).capHeight!).toBeGreaterThan(0);
  });

  it("returns '(unknown)' family name when name table absent (synthetic)", () => {
    const bytes = buildSyntheticFont([]);
    const f = load(bytes);
    expect(fontMetrics(f).familyName).toBe("(unknown)");
  });

  it("returns unitsPerEm = 1000 from synthetic font", () => {
    const bytes = buildSyntheticFont([]);
    const f = load(bytes);
    expect(fontMetrics(f).unitsPerEm).toBe(1000);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Tests: glyphId()
// ─────────────────────────────────────────────────────────────────────────────

describe("glyphId()", () => {
  let font: FontFile;
  beforeEach(() => {
    font = load(loadInterRegular());
  });

  it("returns a glyph ID for 'A' (U+0041)", () => {
    expect(glyphId(font, 0x0041)).not.toBeNull();
    expect(glyphId(font, 0x0041)).toBeGreaterThan(0);
  });

  it("returns a glyph ID for 'V' (U+0056)", () => {
    expect(glyphId(font, 0x0056)).not.toBeNull();
  });

  it("returns a glyph ID for space (U+0020)", () => {
    expect(glyphId(font, 0x0020)).not.toBeNull();
  });

  it("returns different IDs for 'A' and 'V'", () => {
    expect(glyphId(font, 0x0041)).not.toBe(glyphId(font, 0x0056));
  });

  it("returns null for codepoints above U+FFFF", () => {
    expect(glyphId(font, 0x10000)).toBeNull();
  });

  it("returns null for negative codepoints", () => {
    expect(glyphId(font, -1)).toBeNull();
  });

  it("does not throw for U+FFFF (sentinel region)", () => {
    expect(() => glyphId(font, 0xffff)).not.toThrow();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Tests: glyphMetrics()
// ─────────────────────────────────────────────────────────────────────────────

describe("glyphMetrics()", () => {
  let font: FontFile;
  beforeEach(() => {
    font = load(loadInterRegular());
  });

  it("returns positive advance width for 'A'", () => {
    const gid = glyphId(font, 0x0041)!;
    const gm = glyphMetrics(font, gid);
    expect(gm).not.toBeNull();
    expect(gm!.advanceWidth).toBeGreaterThan(0);
  });

  it("returns advance width in expected range (100–2400 design units for 'A')", () => {
    const gid = glyphId(font, 0x0041)!;
    const gm = glyphMetrics(font, gid)!;
    expect(gm.advanceWidth).toBeGreaterThanOrEqual(100);
    expect(gm.advanceWidth).toBeLessThanOrEqual(2400);
  });

  it("returns null for glyph ID out of range", () => {
    const m = fontMetrics(font);
    expect(glyphMetrics(font, m.numGlyphs)).toBeNull();
  });

  it("returns null for negative glyph ID", () => {
    expect(glyphMetrics(font, -1)).toBeNull();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Tests: kerning()
// ─────────────────────────────────────────────────────────────────────────────

describe("kerning()", () => {
  it("returns 0 for Inter Regular (no kern table — uses GPOS)", () => {
    // Inter v4.0 uses GPOS for kerning. The legacy kern table is absent.
    // FNT00 only parses the kern table; GPOS support is FNT01 scope.
    const font = load(loadInterRegular());
    const gidA = glyphId(font, 0x0041)!;
    const gidV = glyphId(font, 0x0056)!;
    expect(kerning(font, gidA, gidV)).toBe(0);
  });

  it("finds kern pair (1, 2) → -140 in synthetic font", () => {
    const font = load(buildSyntheticFont([[1, 2, -140], [3, 4, 80]]));
    expect(kerning(font, 1, 2)).toBe(-140);
  });

  it("finds kern pair (3, 4) → 80 in synthetic font", () => {
    const font = load(buildSyntheticFont([[1, 2, -140], [3, 4, 80]]));
    expect(kerning(font, 3, 4)).toBe(80);
  });

  it("returns 0 for absent pair in synthetic font", () => {
    const font = load(buildSyntheticFont([[1, 2, -140], [3, 4, 80]]));
    expect(kerning(font, 1, 4)).toBe(0);
  });

  it("returns 0 for absent pair (reversed) in synthetic font", () => {
    const font = load(buildSyntheticFont([[1, 2, -140]]));
    expect(kerning(font, 2, 1)).toBe(0);
  });

  it("returns 0 when no kern table present", () => {
    const font = load(loadInterRegular());
    expect(kerning(font, 0, 0)).toBe(0);
  });
});

// Need beforeEach import
import { beforeEach } from "vitest";
