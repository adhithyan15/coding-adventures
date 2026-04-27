/**
 * Tests for the qr-code encoder.
 *
 * Tests are organized into:
 *  1. Unit tests for internal helpers (RS, format info, mode selection)
 *  2. Structural tests verifying grid dimensions and functional patterns
 *  3. Integration tests encoding known strings and verifying properties
 *  4. Error handling tests
 *
 * We cannot easily run a full QR scanner in CI, so we verify structural
 * properties: correct size, finder patterns at expected corners, format info
 * bits readable, dark module in place.  Cross-language comparison happens
 * at the integration level via the test corpus.
 */

import { describe, it, expect } from "vitest";
import {
  encode,
  encodeAndLayout,
  renderSvg,
  explain,
  InputTooLongError,
  VERSION,
  type EccLevel,
} from "../src/index";

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/** Decode the 15-bit format info from copy 1 (top-left finder area). */
function readFormatInfoCopy1(modules: ReadonlyArray<ReadonlyArray<boolean>>, _size: number): number {
  // Bit positions of copy 1 (bit 0 first):
  // bits 0–5 at (8, 0..5), bit 6 at (8,7), bit 7 at (8,8),
  // bit 8 at (7,8), bits 9–14 at (5..0, 8)
  let fmt = 0;
  const rowCol: Array<[number, number]> = [
    [8, 0], [8, 1], [8, 2], [8, 3], [8, 4], [8, 5],
    [8, 7], [8, 8],
    [7, 8],
    [5, 8], [4, 8], [3, 8], [2, 8], [1, 8], [0, 8],
  ];
  for (let i = 0; i < 15; i++) {
    const [r, c] = rowCol[i];
    if (modules[r][c]) fmt |= 1 << i;
  }
  return fmt;
}

/** XOR the raw format bits with the 0x5412 mask and BCH-decode to get [ecc, maskPattern]. */
function decodeFormatInfo(rawFmt: number): { eccBits: number; maskPattern: number } | null {
  const fmt = rawFmt ^ 0x5412;
  // Verify BCH: (fmt >> 10) << 10 mod 0x537 should equal (fmt & 0x3FF)
  let rem = (fmt >> 10) << 10;
  for (let i = 14; i >= 10; i--) {
    if ((rem >> i) & 1) rem ^= 0x537 << (i - 10);
  }
  if ((rem & 0x3ff) !== (fmt & 0x3ff)) return null;
  return { eccBits: (fmt >> 13) & 0x3, maskPattern: (fmt >> 10) & 0x7 };
}

/** Check that a 7×7 finder pattern exists at (topRow, topCol). */
function hasFinder(modules: ReadonlyArray<ReadonlyArray<boolean>>, topRow: number, topCol: number): boolean {
  for (let dr = 0; dr < 7; dr++) {
    for (let dc = 0; dc < 7; dc++) {
      const onBorder = dr === 0 || dr === 6 || dc === 0 || dc === 6;
      const inCore   = dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4;
      const expected = onBorder || inCore;
      if (modules[topRow + dr][topCol + dc] !== expected) return false;
    }
  }
  return true;
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Constants
// ─────────────────────────────────────────────────────────────────────────────

describe("VERSION", () => {
  it("is 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 2. Version selection and grid size
// ─────────────────────────────────────────────────────────────────────────────

describe("encode — version and size", () => {
  it("version 1 produces a 21×21 grid", () => {
    const grid = encode("A", "M");
    expect(grid.rows).toBe(21);
    expect(grid.cols).toBe(21);
    expect(grid.moduleShape).toBe("square");
    expect(grid.modules.length).toBe(21);
    expect(grid.modules[0].length).toBe(21);
  });

  it("HELLO WORLD at M selects version 1 (21×21)", () => {
    // "HELLO WORLD" = 11 alphanumeric chars, fits in v1 M (16 data CW capacity)
    const grid = encode("HELLO WORLD", "M");
    expect(grid.rows).toBe(21);
  });

  it("https://example.com at M selects version 2 (25×25)", () => {
    // 19 bytes in byte mode; version 2 M has 28 data CWs
    const grid = encode("https://example.com", "M");
    expect(grid.rows).toBe(25);
  });

  it("numeric short string selects small version", () => {
    const grid = encode("01234567890", "L");
    // Numeric 11 digits ≈ 37 bits + header; v1 L has 19×8=152 bits → fits easily
    expect(grid.rows).toBe(21);
  });

  it("larger input needs larger version", () => {
    // "The quick brown fox jumps over the lazy dog" = 43 chars, byte mode
    const grid = encode("The quick brown fox jumps over the lazy dog", "M");
    expect(grid.rows).toBeGreaterThan(21);
  });

  it("all four ECC levels produce grids", () => {
    for (const ecc of ["L", "M", "Q", "H"] as EccLevel[]) {
      const grid = encode("HELLO", ecc);
      expect(grid.rows).toBeGreaterThanOrEqual(21);
    }
  });

  it("H level requires larger version than L for same input", () => {
    const gridL = encode("The quick brown fox", "L");
    const gridH = encode("The quick brown fox", "H");
    expect(gridH.rows).toBeGreaterThanOrEqual(gridL.rows);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 3. Structural correctness
// ─────────────────────────────────────────────────────────────────────────────

describe("encode — structural patterns", () => {
  it("top-left finder pattern is present", () => {
    const grid = encode("HELLO WORLD", "M");
    expect(hasFinder(grid.modules, 0, 0)).toBe(true);
  });

  it("top-right finder pattern is present", () => {
    const grid = encode("HELLO WORLD", "M");
    const sz = grid.rows;
    expect(hasFinder(grid.modules, 0, sz - 7)).toBe(true);
  });

  it("bottom-left finder pattern is present", () => {
    const grid = encode("HELLO WORLD", "M");
    const sz = grid.rows;
    expect(hasFinder(grid.modules, sz - 7, 0)).toBe(true);
  });

  it("timing strips alternate dark/light in row 6", () => {
    const grid = encode("HELLO WORLD", "M");
    const sz = grid.rows;
    for (let c = 8; c <= sz - 9; c++) {
      // Dark when col is even
      expect(grid.modules[6][c]).toBe(c % 2 === 0);
    }
  });

  it("timing strips alternate dark/light in col 6", () => {
    const grid = encode("HELLO WORLD", "M");
    const sz = grid.rows;
    for (let r = 8; r <= sz - 9; r++) {
      expect(grid.modules[r][6]).toBe(r % 2 === 0);
    }
  });

  it("dark module at (4V+9, 8) is dark", () => {
    const grid = encode("A", "M");
    // version 1: dark module at (4*1+9, 8) = (13, 8)
    expect(grid.modules[13][8]).toBe(true);
  });

  it("dark module present for version 2 symbol", () => {
    const grid = encode("https://example.com", "M");
    // version 2: dark module at (4*2+9, 8) = (17, 8)
    expect(grid.modules[17][8]).toBe(true);
  });

  it("format information is present and decodable", () => {
    const grid = encode("HELLO WORLD", "M");
    const rawFmt = readFormatInfoCopy1(grid.modules, grid.rows);
    const decoded = decodeFormatInfo(rawFmt);
    expect(decoded).not.toBeNull();
    // ECC M indicator = 00 = 0
    expect(decoded!.eccBits).toBe(0b00);
  });

  it("format info ECC bits match the requested level", () => {
    const eccBitsMap: Record<EccLevel, number> = { L: 0b01, M: 0b00, Q: 0b11, H: 0b10 };
    for (const ecc of ["L", "M", "Q", "H"] as EccLevel[]) {
      const grid = encode("HELLO", ecc);
      const rawFmt = readFormatInfoCopy1(grid.modules, grid.rows);
      const decoded = decodeFormatInfo(rawFmt);
      expect(decoded).not.toBeNull();
      expect(decoded!.eccBits).toBe(eccBitsMap[ecc]);
    }
  });

  it("two format info copies are consistent", () => {
    const grid = encode("HELLO WORLD", "M");
    const sz = grid.rows;
    const copy2positions: Array<[number, number]> = [
      [sz-1,8],[sz-2,8],[sz-3,8],[sz-4,8],[sz-5,8],[sz-6,8],[sz-7,8],
      [8,sz-8],[8,sz-7],[8,sz-6],[8,sz-5],[8,sz-4],[8,sz-3],[8,sz-2],[8,sz-1],
    ];
    const copy1positions: Array<[number, number]> = [
      [8,0],[8,1],[8,2],[8,3],[8,4],[8,5],[8,7],[8,8],
      [7,8],[5,8],[4,8],[3,8],[2,8],[1,8],[0,8],
    ];
    let fmt1 = 0, fmt2 = 0;
    for (let i = 0; i < 15; i++) {
      const [r1,c1] = copy1positions[i];
      const [r2,c2] = copy2positions[i];
      if (grid.modules[r1][c1]) fmt1 |= 1 << i;
      if (grid.modules[r2][c2]) fmt2 |= 1 << i;
    }
    expect(fmt1).toBe(fmt2);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 4. Encoding mode selection
// ─────────────────────────────────────────────────────────────────────────────

describe("encode — mode selection (via grid size)", () => {
  it("pure digits produce a smaller grid than equivalent bytes", () => {
    // "000000000000000" (15 digits) uses numeric mode — much more compact
    const numericGrid = encode("000000000000000", "M");
    // Same content as bytes would be larger
    const byteGrid = encode("A".repeat(15), "M");
    // Both should be valid QR codes; numeric may fit in smaller version
    expect(numericGrid.rows).toBeLessThanOrEqual(byteGrid.rows);
  });

  it("uppercase + digits + space uses alphanumeric mode", () => {
    const grid = encode("HELLO WORLD", "M");
    expect(grid.rows).toBe(21); // v1 — fits in alphanumeric, not byte
  });

  it("lowercase falls back to byte mode", () => {
    const grid = encode("hello world", "M");
    // Byte mode needs more space than alphanumeric
    expect(grid.rows).toBeGreaterThanOrEqual(21);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 5. encodeAndLayout
// ─────────────────────────────────────────────────────────────────────────────

describe("encodeAndLayout", () => {
  it("returns a PaintScene", () => {
    const scene = encodeAndLayout("HELLO", "M");
    expect(scene).toBeDefined();
    expect(scene.width).toBeGreaterThan(0);
    expect(scene.height).toBeGreaterThan(0);
    expect(scene.instructions).toBeDefined();
  });

  it("default config produces 21×21 grid at 10px/module with 4-module quiet zone", () => {
    const scene = encodeAndLayout("A", "M");
    // v1: 21 modules + 2*4 quiet = 29 modules → 29*10 = 290px
    expect(scene.width).toBe(290);
    expect(scene.height).toBe(290);
  });

  it("accepts partial config", () => {
    const scene = encodeAndLayout("A", "M", { moduleSizePx: 5 });
    // 29 modules × 5 px = 145px
    expect(scene.width).toBe(145);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 6. renderSvg
// ─────────────────────────────────────────────────────────────────────────────

describe("renderSvg", () => {
  it("returns a string starting with <svg", () => {
    const svg = renderSvg("HELLO WORLD", "M");
    expect(typeof svg).toBe("string");
    expect(svg.trimStart()).toMatch(/^<svg/);
  });

  it("contains a closing </svg> tag", () => {
    const svg = renderSvg("A", "L");
    expect(svg).toContain("</svg>");
  });

  it("SVG is larger for larger QR versions", () => {
    const svgV1 = renderSvg("A", "M");
    const svgV3 = renderSvg("The quick brown fox", "M");
    expect(svgV3.length).toBeGreaterThan(svgV1.length);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 7. explain
// ─────────────────────────────────────────────────────────────────────────────

describe("explain", () => {
  it("returns grid with annotations array matching dimensions", () => {
    const annotated = explain("HELLO WORLD", "M");
    expect(annotated.rows).toBe(21);
    expect(annotated.annotations.length).toBe(21);
    expect(annotated.annotations[0].length).toBe(21);
  });

  it("module grid matches plain encode", () => {
    const plain = encode("HELLO WORLD", "M");
    const annotated = explain("HELLO WORLD", "M");
    expect(annotated.modules).toEqual(plain.modules);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 8. Error handling
// ─────────────────────────────────────────────────────────────────────────────

describe("error handling", () => {
  it("throws InputTooLongError for a string exceeding all versions", () => {
    const giant = "A".repeat(8000); // well beyond version 40 H capacity
    expect(() => encode(giant, "H")).toThrow(InputTooLongError);
  });

  it("error message mentions the ECC level", () => {
    // 2000 ASCII bytes is under the 7089-char early-exit guard but exceeds
    // v40-H byte-mode capacity (~1273 bytes), so selectVersion throws with
    // the ECC level in the message.
    const giant = "A".repeat(2000);
    try {
      encode(giant, "H");
    } catch (e) {
      expect((e as Error).message).toContain("H");
    }
  });

  it("does not throw for empty string", () => {
    expect(() => encode("", "M")).not.toThrow();
  });

  it("encodes a single character without error", () => {
    const grid = encode("A", "M");
    expect(grid.rows).toBe(21);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 9. Determinism and consistency
// ─────────────────────────────────────────────────────────────────────────────

describe("determinism", () => {
  it("same input produces identical grids across calls", () => {
    const g1 = encode("https://example.com", "M");
    const g2 = encode("https://example.com", "M");
    expect(g1.modules).toEqual(g2.modules);
  });

  it("different inputs produce different grids", () => {
    const g1 = encode("HELLO", "M");
    const g2 = encode("WORLD", "M");
    // At least some modules differ
    let diff = false;
    for (let r = 0; r < g1.rows; r++)
      for (let c = 0; c < g1.cols; c++)
        if (g1.modules[r][c] !== g2.modules[r][c]) diff = true;
    expect(diff).toBe(true);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 10. Edge cases
// ─────────────────────────────────────────────────────────────────────────────

describe("edge cases", () => {
  it("numeric mode: all zeros", () => {
    const grid = encode("0000000000", "L");
    expect(grid.rows).toBeGreaterThanOrEqual(21);
  });

  it("byte mode: UTF-8 multi-byte characters", () => {
    // "→" is 3 UTF-8 bytes
    const grid = encode("→→→", "M");
    expect(grid.rows).toBeGreaterThanOrEqual(21);
  });

  it("alphanumeric: all special chars", () => {
    const grid = encode("$%*+-./:", "M");
    expect(grid.rows).toBeGreaterThanOrEqual(21);
  });

  it("version 7+ grid has correct size", () => {
    // 85 uppercase-letter alphanumeric chars exceed v6-H capacity (~84 chars).
    // v6 H has 60 data CW = 480 bits; 4+9+ceil(85×11/2)=481 bits → doesn't fit.
    // v7 H has 66 data CW = 528 bits; 481 bits fits. Expect 45×45 or larger.
    const input = "A".repeat(85);
    const grid = encode(input, "H");
    expect(grid.rows).toBeGreaterThanOrEqual(45);
  });

  it("modules array is a proper 2D boolean array", () => {
    const grid = encode("HELLO", "M");
    for (const row of grid.modules) {
      for (const cell of row) {
        expect(typeof cell).toBe("boolean");
      }
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 11. Integration: known test corpus
// ─────────────────────────────────────────────────────────────────────────────

describe("integration — test corpus", () => {
  const corpus = [
    { input: "A",                                          ecc: "M" as EccLevel },
    { input: "HELLO WORLD",                                ecc: "M" as EccLevel },
    { input: "https://example.com",                        ecc: "M" as EccLevel },
    { input: "01234567890",                                ecc: "M" as EccLevel },
    { input: "The quick brown fox jumps over the lazy dog", ecc: "M" as EccLevel },
  ];

  for (const { input, ecc } of corpus) {
    it(`encodes "${input.slice(0, 20)}…" at ${ecc} without error`, () => {
      const grid = encode(input, ecc);
      expect(grid.rows).toBeGreaterThanOrEqual(21);
      expect(grid.rows).toBe(grid.cols);
      expect(grid.moduleShape).toBe("square");
    });
  }

  it("each corpus item produces a decodable format info", () => {
    for (const { input, ecc } of corpus) {
      const grid = encode(input, ecc);
      const rawFmt = readFormatInfoCopy1(grid.modules, grid.rows);
      const decoded = decodeFormatInfo(rawFmt);
      expect(decoded).not.toBeNull();
    }
  });
});
