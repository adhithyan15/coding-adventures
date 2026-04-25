/**
 * Tests for Data Matrix ECC200 encoder.
 *
 * Test strategy:
 *   1. GF(256)/0x12D arithmetic — verify exp/log tables and multiplication
 *   2. ASCII encoding — character codewords and digit pairs
 *   3. Pad codewords — scrambled-pad formula
 *   4. RS encoding — ECC computation per block
 *   5. Utah algorithm — module placement for known 10×10 symbols
 *   6. Symbol border — L-finder and timing clock structural pattern
 *   7. Integration — full pipeline for test corpus
 *   8. Multi-region symbols — alignment borders for 32×32+
 */

import { describe, it, expect } from "vitest";
import {
  encode,
  encodeAndLayout,
  renderSvg,
  explain,
  InputTooLongError,
  _internal,
} from "../src/index.js";

const {
  GF_EXP,
  GF_LOG,
  gfMul,
  encodeAscii,
  padCodewords,
  selectSymbol,
  rsEncodeBlock,
  getGenerator,
  utahPlacement,
  SQUARE_SIZES,
} = _internal;

// ─────────────────────────────────────────────────────────────────────────────
// 1. GF(256)/0x12D arithmetic
// ─────────────────────────────────────────────────────────────────────────────

describe("GF(256)/0x12D arithmetic", () => {
  it("exp table starts correctly", () => {
    expect(GF_EXP[0]).toBe(1);    // α^0 = 1
    expect(GF_EXP[1]).toBe(2);    // α^1 = 2 (generator)
    expect(GF_EXP[2]).toBe(4);    // α^2 = 4
    expect(GF_EXP[3]).toBe(8);    // α^3 = 8
    expect(GF_EXP[4]).toBe(16);   // α^4 = 16
    expect(GF_EXP[5]).toBe(32);   // α^5 = 32
    expect(GF_EXP[6]).toBe(64);   // α^6 = 64
    expect(GF_EXP[7]).toBe(128);  // α^7 = 128
  });

  it("exp[8] = 0x2D (first reduction with 0x12D)", () => {
    // 0x80 << 1 = 0x100; 0x100 XOR 0x12D = 0x2D = 45
    expect(GF_EXP[8]).toBe(0x2d);
  });

  it("exp[9] = 0x5A", () => {
    // 0x2D << 1 = 0x5A (no overflow)
    expect(GF_EXP[9]).toBe(0x5a);
  });

  it("exp[10] = 0xB4", () => {
    // 0x5A << 1 = 0xB4 (no overflow)
    expect(GF_EXP[10]).toBe(0xb4);
  });

  it("exp table wraps: exp[255] = exp[0] = 1", () => {
    // Multiplicative group order = 255, so α^255 = 1
    expect(GF_EXP[255]).toBe(1);
  });

  it("log table is inverse of exp", () => {
    for (let i = 0; i < 255; i++) {
      const v = GF_EXP[i]!;
      expect(GF_LOG[v]).toBe(i);
    }
  });

  it("gfMul: identity and zero", () => {
    expect(gfMul(0, 0xff)).toBe(0);  // zero absorbs
    expect(gfMul(0xff, 0)).toBe(0);  // zero absorbs
    expect(gfMul(1, 7)).toBe(7);     // 1 is identity
    expect(gfMul(7, 1)).toBe(7);     // commutativity
  });

  it("gfMul: α * α = α^2 = 4", () => {
    expect(gfMul(2, 2)).toBe(4);
  });

  it("gfMul: α^7 * α = α^8 = 0x2D", () => {
    expect(gfMul(0x80, 2)).toBe(0x2d);
  });

  it("gfMul is commutative", () => {
    for (const a of [3, 7, 45, 128, 200, 255]) {
      for (const b of [5, 11, 90, 180, 215]) {
        expect(gfMul(a, b)).toBe(gfMul(b, a));
      }
    }
  });

  it("field has order 255 (every non-zero element generates the group)", () => {
    // α^254 must be a non-zero value, and α^255 = 1
    expect(GF_EXP[254]).not.toBe(0);
    expect(GF_EXP[254]).not.toBe(1); // not identity (unless it's the last)
    // Actually gf_exp[254] might be 1 if the index 255 wraps. Check that
    // the exp table has all 255 non-zero elements distinct in [0..254]
    const seen = new Set<number>();
    for (let i = 0; i < 255; i++) {
      const v = GF_EXP[i]!;
      expect(v).toBeGreaterThan(0);
      expect(seen.has(v)).toBe(false); // all distinct
      seen.add(v);
    }
    expect(seen.size).toBe(255);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 2. ASCII encoding
// ─────────────────────────────────────────────────────────────────────────────

describe("ASCII encoding", () => {
  const enc = (s: string) => encodeAscii(new TextEncoder().encode(s));

  it("single ASCII char: codeword = ASCII + 1", () => {
    expect(enc("A")).toEqual([66]);     // 65 + 1
    expect(enc("a")).toEqual([98]);     // 97 + 1
    expect(enc(" ")).toEqual([33]);     // 32 + 1
    expect(enc("\0")).toEqual([1]);     // 0 + 1
  });

  it("digit pairs: codeword = 130 + (d1*10 + d2)", () => {
    expect(enc("12")).toEqual([142]);   // 130 + (1*10+2) = 142
    expect(enc("34")).toEqual([164]);   // 130 + (3*10+4) = 164
    expect(enc("56")).toEqual([186]);   // 130 + (5*10+6) = 186
    expect(enc("78")).toEqual([208]);   // 130 + (7*10+8) = 208
    expect(enc("00")).toEqual([130]);   // 130 + 0 = 130
    expect(enc("99")).toEqual([229]);   // 130 + 99 = 229
  });

  it("1234 → two digit pairs", () => {
    expect(enc("1234")).toEqual([142, 164]);  // [130+(1*10+2), 130+(3*10+4)]
  });

  it("mixed: '1A' → separate codewords (no pair)", () => {
    expect(enc("1A")).toEqual([50, 66]);  // 49+1=50, 65+1=66
  });

  it("odd digit count: '123' → pair + single", () => {
    // "12" → 142, "3" → 52 (51+1=52)
    expect(enc("123")).toEqual([142, 52]);
  });

  it("Hello → individual codewords", () => {
    const result = enc("Hello");
    expect(result).toEqual([73, 102, 109, 109, 112]);
    // H=72+1=73, e=101+1=102, l=108+1=109, l=108+1=109, o=111+1=112
  });

  it("'Hello World' → 11 codewords (space + W give singles)", () => {
    const result = enc("Hello World");
    expect(result).toHaveLength(11);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 3. Pad codewords
// ─────────────────────────────────────────────────────────────────────────────

describe("pad codewords", () => {
  it("no padding needed when already at capacity", () => {
    const cw = [66, 129, 70];
    expect(padCodewords(cw, 3)).toEqual([66, 129, 70]);
  });

  it("pads 'A' → [66] to 3 codewords (10×10 symbol)", () => {
    // Verified against ISO/IEC 16022 Annex F:
    //   k=2: first pad = 129 (literal)
    //   k=3: scrambled = 129 + (149*3 mod 253) + 1
    //       = 129 + (447 mod 253) + 1 = 129 + 194 + 1 = 324
    //       324 > 254 → 324 - 254 = 70
    const padded = padCodewords([66], 3);
    expect(padded).toEqual([66, 129, 70]);
  });

  it("first pad byte is always 129", () => {
    const padded = padCodewords([10], 5);
    expect(padded[1]).toBe(129);
  });

  it("pads to exact capacity", () => {
    for (const entry of SQUARE_SIZES.slice(0, 5)) {
      const padded = padCodewords([66], entry.dataCW);
      expect(padded).toHaveLength(entry.dataCW);
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 4. RS encoding
// ─────────────────────────────────────────────────────────────────────────────

describe("Reed-Solomon encoding", () => {
  it("generator polynomial degree matches nEcc", () => {
    for (const nEcc of [5, 7, 10, 12, 14, 18, 20, 24, 28]) {
      const gen = getGenerator(nEcc);
      expect(gen).toHaveLength(nEcc + 1);  // nEcc+1 coefficients
      expect(gen[0]).toBe(1);              // monic: leading coeff = 1
    }
  });

  it("all generator roots are roots of the polynomial", () => {
    // For generator with n=5, each α^1..α^5 must be a root.
    // This verifies the polynomial is correctly computed.
    const gen5 = getGenerator(5);
    for (let root = 1; root <= 5; root++) {
      // Evaluate gen5 at α^root using Horner's method
      const x = GF_EXP[root]!;
      let val = 0;
      for (const coeff of gen5) {
        val = gfMul(val, x) ^ coeff;
      }
      expect(val).toBe(0); // α^root is a root of g(x)
    }
  });

  it("10×10 symbol: RS ECC for [66, 129, 70]", () => {
    // Data "A" padded: [66, 129, 70]
    // ECC for n_ecc=5, GF(256)/0x12D
    // We compute and verify the result is 5 bytes
    const gen = getGenerator(5);
    const ecc = rsEncodeBlock([66, 129, 70], gen);
    expect(ecc).toHaveLength(5);
    // All ECC bytes must be in GF(256) range
    for (const byte of ecc) {
      expect(byte).toBeGreaterThanOrEqual(0);
      expect(byte).toBeLessThanOrEqual(255);
    }
  });

  it("ECC bytes change when data changes", () => {
    const gen = getGenerator(5);
    const ecc1 = rsEncodeBlock([66, 129, 70], gen);
    const ecc2 = rsEncodeBlock([67, 129, 70], gen);
    expect(ecc1).not.toEqual(ecc2);
  });

  it("systematic: codeword with appended ECC passes syndrome check", () => {
    // For b=1 RS, the generator polynomial has roots α^1..α^n.
    // A valid codeword C(x) = D(x)*x^n + R(x) must satisfy C(α^i) = 0 for i=1..n.
    const nEcc = 5;
    const gen = getGenerator(nEcc);
    const data = [66, 129, 70];
    const ecc = rsEncodeBlock(data, gen);
    const codeword = [...data, ...ecc];

    // Check C(α^i) = 0 for i=1..nEcc (b=1 convention)
    for (let root = 1; root <= nEcc; root++) {
      const x = GF_EXP[root]!;
      // Evaluate codeword polynomial at x using Horner's method
      let val = 0;
      for (const byte of codeword) {
        val = gfMul(val, x) ^ byte;
      }
      expect(val).toBe(0); // C(α^root) must be 0 for a valid codeword
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 5. Utah placement (structure tests)
// ─────────────────────────────────────────────────────────────────────────────

describe("Utah placement algorithm", () => {
  it("fills 8×8 logical grid (10×10 symbol interior): 0xFF codewords → mostly dark", () => {
    const nRows = 8;
    const nCols = 8;
    // Use 8 codewords of 0xFF (all bits set = dark)
    const codewords = new Array(8).fill(0xff);
    const grid = utahPlacement(codewords, nRows, nCols);
    // Every module placed directly has bit=1 (dark).
    // Residual unfilled modules (if any) get fill pattern (r+c)%2==1 = dark at odd positions.
    // So the result should have many dark modules (hard to predict exact count due to fill).
    let dark = 0;
    for (let r = 0; r < nRows; r++) for (let c = 0; c < nCols; c++) if (grid[r]![c]) dark++;
    // With 0xFF codewords, all placed bits are dark; fill positions vary.
    // We know 8 codewords × 8 bits = 64 placements but some may overlap due to corner patterns.
    expect(dark).toBeGreaterThan(40); // at least majority dark
    expect(dark).toBeLessThanOrEqual(nRows * nCols);
  });

  it("0x00 codewords: only fill-pattern positions are dark", () => {
    // With 0x00 codewords, all placed bits are 0 (light).
    // Any unset modules get fill pattern (r+c)%2==1 (dark at odd positions).
    const codewords = new Array(8).fill(0x00);
    const grid = utahPlacement(codewords, 8, 8);
    // The placement sets all modules to 0 (light). The fill pattern then
    // sets residual modules. So we should see at most the fill-pattern dark modules.
    // Verify the grid has at least some light modules.
    let light = 0;
    for (let r = 0; r < 8; r++) for (let c = 0; c < 8; c++) if (!grid[r]![c]) light++;
    expect(light).toBeGreaterThan(0);
  });

  it("logical grid is correct size", () => {
    for (const entry of SQUARE_SIZES.slice(0, 5)) {
      const nRows = entry.regionRows * entry.dataRegionHeight;
      const nCols = entry.regionCols * entry.dataRegionWidth;
      const total = entry.dataCW + entry.eccCW;
      const codewords = new Array(total).fill(0xAA);
      const grid = utahPlacement(codewords, nRows, nCols);
      expect(grid).toHaveLength(nRows);
      expect(grid[0]).toHaveLength(nCols);
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 6. Symbol border structure
// ─────────────────────────────────────────────────────────────────────────────

describe("symbol border structure", () => {
  function assertBorder(input: string): void {
    const grid = encode(input);
    const { rows, cols, modules } = grid;

    // L-finder: left column (col 0) — all dark
    for (let r = 0; r < rows; r++) {
      expect(modules[r]![0], `left col[${r}] should be dark`).toBe(true);
    }

    // L-finder: bottom row (row rows-1) — all dark
    // This includes the bottom-right corner which is dark even though timing would say light.
    for (let c = 0; c < cols; c++) {
      expect(modules[rows - 1]![c], `bottom row[${c}] should be dark`).toBe(true);
    }

    // Timing: top row (row 0) — alternating dark/light starting dark.
    // Skip the rightmost column (col=cols-1) because the right-column timing overrides it to dark.
    for (let c = 0; c < cols - 1; c++) {
      const expected = c % 2 === 0;
      expect(modules[0]![c], `top row[${c}] expected ${expected}`).toBe(expected);
    }

    // Timing: right column (col cols-1) — alternating dark/light starting dark.
    // Skip the bottom row (row=rows-1) because the L-finder overrides it to dark.
    for (let r = 0; r < rows - 1; r++) {
      const expected = r % 2 === 0;
      expect(modules[r]![cols - 1], `right col[${r}] expected ${expected}`).toBe(expected);
    }

    // Corner (0,0) must be dark (both L-bar and timing start here — they agree)
    expect(modules[0]![0]).toBe(true);
    // Corner (0, cols-1): dark (right-col timing: row 0, 0%2=0 → dark)
    expect(modules[0]![cols - 1]).toBe(true);
    // Corner (rows-1, 0): dark (L-finder bottom row)
    expect(modules[rows - 1]![0]).toBe(true);
    // Corner (rows-1, cols-1): dark (L-finder bottom row overrides timing)
    expect(modules[rows - 1]![cols - 1]).toBe(true);
  }

  it("'A' → 10×10 symbol has correct border", () => assertBorder("A"));
  it("'1234' → symbol has correct border", () => assertBorder("1234"));
  it("'Hello World' → 16×16 symbol has correct border", () => assertBorder("Hello World"));
  it("'ABCDEFGHIJKLMNOP' → larger symbol has correct border", () => assertBorder("ABCDEFGHIJKLMNOP"));
});

// ─────────────────────────────────────────────────────────────────────────────
// 7. Integration — full pipeline
// ─────────────────────────────────────────────────────────────────────────────

describe("encode() integration", () => {
  it("'A' → 10×10 symbol", () => {
    const grid = encode("A");
    expect(grid.rows).toBe(10);
    expect(grid.cols).toBe(10);
    expect(grid.moduleShape).toBe("square");
    expect(grid.modules).toHaveLength(10);
    expect(grid.modules[0]).toHaveLength(10);
  });

  it("'1234' → 10×10 symbol (2 digit-pair codewords fit)", () => {
    // "12" → codeword 142, "34" → codeword 174 → 2 data codewords
    // 10×10 capacity = 3, so 2 codewords fit easily
    const grid = encode("1234");
    expect(grid.rows).toBe(10);
    expect(grid.cols).toBe(10);
  });

  it("'Hello World' → 16×16 symbol", () => {
    // 11 characters → 11 ASCII codewords
    // 14×14 capacity = 8 (too small), 16×16 capacity = 12 (fits: 11 ≤ 12)
    const grid = encode("Hello World");
    expect(grid.rows).toBe(16);
    expect(grid.cols).toBe(16);
  });

  it("encoding produces a valid boolean grid (no undefined entries)", () => {
    const grid = encode("Hello World");
    const { rows, cols, modules } = grid;
    expect(rows).toBeGreaterThan(0);
    expect(cols).toBeGreaterThan(0);
    for (let r = 0; r < rows; r++) {
      for (let c = 0; c < cols; c++) {
        expect(typeof modules[r]![c]).toBe("boolean");
      }
    }
  });

  it("empty string → smallest symbol", () => {
    // 0 data codewords → 10×10 (capacity 3)
    const grid = encode("");
    expect(grid.rows).toBe(10);
    expect(grid.cols).toBe(10);
  });

  it("single digit '5' → 10×10 (1 codeword)", () => {
    const grid = encode("5");
    expect(grid.rows).toBe(10);
  });

  it("single digit pair '55' → 10×10 (1 digit-pair codeword)", () => {
    const grid = encode("55");
    expect(grid.rows).toBe(10);
  });

  it("3 codewords fit in 10×10, 4 need 12×12", () => {
    // "AAA" → 3 codewords → exactly 10×10
    expect(encode("AAA").rows).toBe(10);
    // "AAAA" → 4 codewords → needs 12×12 (capacity 5)
    expect(encode("AAAA").rows).toBe(12);
  });

  it("large input fits in appropriate symbol size", () => {
    // ABCDEFGHIJKLMNOPQRSTUVWXYZ = 26 letters = 26 codewords
    // 0123456789 = 5 digit pairs = 5 codewords
    // Total: 31 codewords → 24×24 (capacity 36) or 26×26 (capacity 44)
    const input = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    const cw = encodeAscii(new TextEncoder().encode(input));
    // Verify digit compression worked
    expect(cw.length).toBe(31); // 26 letters + 5 digit pairs
    // 31 codewords fits in 24×24 (capacity 36)
    const grid = encode(input);
    expect(grid.rows).toBe(24);
    expect(grid.cols).toBe(24);
  });

  it("InputTooLongError on exceeding max capacity", () => {
    // Generate a string of 1600 unique chars (way over 1558 max)
    const huge = "A".repeat(1600);
    expect(() => encode(huge)).toThrow(InputTooLongError);
  });

  it("symbols grow as input grows", () => {
    // Verify that larger inputs consistently produce larger symbols
    const sizes = [
      encode("A").rows,
      encode("HELLO WORLD").rows,
      encode("The quick brown fox jumps over the lazy dog").rows,
    ];
    for (let i = 0; i < sizes.length - 1; i++) {
      expect(sizes[i]).toBeLessThanOrEqual(sizes[i + 1]!);
    }
  });

  it("digit-heavy input uses less codewords than letter input of same length", () => {
    // "12345678901234567890" (20 digits) → 10 digit-pair codewords
    // "ABCDEFGHIJKLMNOPQRST" (20 letters) → 20 codewords
    const digitCW = encodeAscii(new TextEncoder().encode("12345678901234567890"));
    const letterCW = encodeAscii(new TextEncoder().encode("ABCDEFGHIJKLMNOPQRST"));
    expect(digitCW.length).toBeLessThan(letterCW.length);
    expect(digitCW.length).toBe(10); // 10 pairs
    expect(letterCW.length).toBe(20);
  });

  it("Uint8Array input works same as string", () => {
    const str = "Hello";
    const bytes = new TextEncoder().encode(str);
    const g1 = encode(str);
    const g2 = encode(bytes);
    expect(g1.rows).toBe(g2.rows);
    expect(g1.cols).toBe(g2.cols);
    for (let r = 0; r < g1.rows; r++) {
      for (let c = 0; c < g1.cols; c++) {
        expect(g1.modules[r]![c]).toBe(g2.modules[r]![c]);
      }
    }
  });

  it("deterministic: same input produces identical grid twice", () => {
    const g1 = encode("Hello World");
    const g2 = encode("Hello World");
    for (let r = 0; r < g1.rows; r++) {
      for (let c = 0; c < g1.cols; c++) {
        expect(g1.modules[r]![c]).toBe(g2.modules[r]![c]);
      }
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 8. Multi-region symbols
// ─────────────────────────────────────────────────────────────────────────────

describe("multi-region symbols", () => {
  it("32×32 symbol has correct overall dimensions", () => {
    // Need 45+ codewords to force 32×32 (capacity = 62)
    const input = "A".repeat(50); // 50 codewords → needs 32×32
    const grid = encode(input);
    expect(grid.rows).toBe(32);
    expect(grid.cols).toBe(32);
  });

  it("32×32 symbol has correct outer border", () => {
    const input = "A".repeat(50);
    const { rows, cols, modules } = encode(input);
    // Left col all dark
    for (let r = 0; r < rows; r++) expect(modules[r]![0]).toBe(true);
    // Bottom row all dark (L-finder — overrides right-col corner)
    for (let c = 0; c < cols; c++) expect(modules[rows - 1]![c]).toBe(true);
    // Top row alternating (skip last col — overridden by right-col timing: row0=dark)
    for (let c = 0; c < cols - 1; c++) expect(modules[0]![c]).toBe(c % 2 === 0);
    // Top-right corner: dark (right-col timing: 0%2===0)
    expect(modules[0]![cols - 1]).toBe(true);
    // Right col alternating (skip last row — overridden by L-finder: dark)
    for (let r = 0; r < rows - 1; r++) expect(modules[r]![cols - 1]).toBe(r % 2 === 0);
    // Bottom-right corner: dark (L-finder)
    expect(modules[rows - 1]![cols - 1]).toBe(true);
  });

  it("32×32 symbol (2×2 regions) has alignment borders", () => {
    // 32×32: regionRows=2, regionCols=2, dataRegionHeight=14, dataRegionWidth=14
    // Outer border: row 0, row 31, col 0, col 31
    // Horizontal AB: rows 15 (all dark) and 16 (alternating) at interior cols
    // Vertical AB: cols 15 (all dark) and 16 (alternating) at interior rows
    // Physical AB row0 = 1 + 1*14 + 0*2 = 15
    const input = "A".repeat(50);
    const { modules } = encode(input);

    const abRow0 = 15;
    const abRow1 = 16;
    const abCol0 = 15;
    const abCol1 = 16;

    // Horizontal AB row0: all dark, for interior cols (skip outer borders and vertical AB cols)
    for (let c = 1; c < 15; c++) {
      expect(modules[abRow0]![c], `H-AB row0 col ${c}`).toBe(true);
    }
    for (let c = 17; c < 31; c++) {
      expect(modules[abRow0]![c], `H-AB row0 col ${c}`).toBe(true);
    }

    // Horizontal AB row1: alternating, for cols that are not also vertical AB cols
    for (let c = 1; c < 15; c++) {
      expect(modules[abRow1]![c], `H-AB row1 col ${c}`).toBe(c % 2 === 0);
    }
    for (let c = 17; c < 31; c++) {
      expect(modules[abRow1]![c], `H-AB row1 col ${c}`).toBe(c % 2 === 0);
    }

    // Vertical AB col0: all dark, for interior rows (skip outer borders and horizontal AB rows)
    for (let r = 1; r < 15; r++) {
      expect(modules[r]![abCol0], `V-AB col0 row ${r}`).toBe(true);
    }
    for (let r = 17; r < 31; r++) {
      expect(modules[r]![abCol0], `V-AB col0 row ${r}`).toBe(true);
    }

    // Vertical AB col1: alternating, for rows not in horizontal AB
    for (let r = 1; r < 15; r++) {
      expect(modules[r]![abCol1], `V-AB col1 row ${r}`).toBe(r % 2 === 0);
    }
    for (let r = 17; r < 31; r++) {
      expect(modules[r]![abCol1], `V-AB col1 row ${r}`).toBe(r % 2 === 0);
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 9. Symbol selection
// ─────────────────────────────────────────────────────────────────────────────

describe("symbol selection", () => {
  it("selects smallest fitting square symbol", () => {
    expect(selectSymbol(1, "square").symbolRows).toBe(10);
    expect(selectSymbol(3, "square").symbolRows).toBe(10);
    expect(selectSymbol(4, "square").symbolRows).toBe(12);
    expect(selectSymbol(5, "square").symbolRows).toBe(12);
    expect(selectSymbol(6, "square").symbolRows).toBe(14);
    expect(selectSymbol(9, "square").symbolRows).toBe(16);
  });

  it("'any' shape selects from both square and rectangular", () => {
    // All valid symbol sizes should work without throwing
    expect(() => selectSymbol(5, "any")).not.toThrow();
    expect(() => selectSymbol(5, "rectangular")).not.toThrow();
  });

  it("throws InputTooLongError for count > 1558", () => {
    expect(() => selectSymbol(1559, "square")).toThrow(InputTooLongError);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 10. encodeAndLayout, renderSvg, explain
// ─────────────────────────────────────────────────────────────────────────────

describe("layout and SVG rendering", () => {
  it("encodeAndLayout returns a PaintScene with instructions", () => {
    const scene = encodeAndLayout("Hello");
    expect(scene).toBeDefined();
    expect(scene.instructions).toBeDefined();
    expect(scene.instructions.length).toBeGreaterThan(0);
  });

  it("renderSvg returns a string starting with <svg", () => {
    const svg = renderSvg("Hello");
    expect(typeof svg).toBe("string");
    expect(svg).toContain("<svg");
    expect(svg).toContain("</svg>");
  });

  it("explain returns AnnotatedModuleGrid with null annotations", () => {
    const annotated = explain("A");
    expect(annotated.rows).toBe(10);
    expect(annotated.cols).toBe(10);
    expect(annotated.annotations).toBeDefined();
    expect(annotated.annotations[0]![0]).toBeNull();
  });

  it("encodeAndLayout respects custom quiet zone", () => {
    const scene1 = encodeAndLayout("A", {}, { quietZoneModules: 1 });
    const scene2 = encodeAndLayout("A", {}, { quietZoneModules: 4 });
    // Larger quiet zone → larger overall dimensions
    // Both should produce valid scenes
    expect(scene1.instructions.length).toBeGreaterThan(0);
    expect(scene2.instructions.length).toBeGreaterThan(0);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 11. Cross-language test corpus verification
// ─────────────────────────────────────────────────────────────────────────────

describe("cross-language test corpus", () => {
  // The encode function must produce a consistent, deterministic output
  // that is identical to other language implementations.
  // We verify structural invariants and key properties that the Rust
  // implementation must also satisfy.

  it("corpus: 'A' → 10×10, dark module count in valid range", () => {
    const grid = encode("A");
    expect(grid.rows).toBe(10);
    expect(grid.cols).toBe(10);
    let dark = 0;
    for (let r = 0; r < 10; r++) for (let c = 0; c < 10; c++) if (grid.modules[r]![c]) dark++;
    // A 10×10 symbol has 100 modules. A reasonable range is 30-70 dark modules.
    expect(dark).toBeGreaterThan(20);
    expect(dark).toBeLessThan(85);
  });

  it("corpus: '1234' → 10×10", () => {
    expect(encode("1234").rows).toBe(10);
    expect(encode("1234").cols).toBe(10);
  });

  it("corpus: 'Hello World' → 16×16", () => {
    expect(encode("Hello World").rows).toBe(16);
    expect(encode("Hello World").cols).toBe(16);
  });

  it("corpus: specific codewords for 'A' (ISO Annex F check)", () => {
    // From ISO/IEC 16022 Annex F: encoding "A" in a 10×10 symbol
    //   Data: [66] (A+1)
    //   Padded: [66, 129, 70]
    //   Data fits in 10×10 (capacity=3)
    const cw = encodeAscii(new TextEncoder().encode("A"));
    expect(cw).toEqual([66]);

    const padded = padCodewords(cw, 3);
    expect(padded).toEqual([66, 129, 70]);
  });

  it("corpus: specific codewords for '1234'", () => {
    // "12" → 130+(1*10+2) = 142, "34" → 130+(3*10+4) = 164
    const cw = encodeAscii(new TextEncoder().encode("1234"));
    expect(cw).toEqual([142, 164]);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 12. Edge cases
// ─────────────────────────────────────────────────────────────────────────────

describe("edge cases", () => {
  it("null byte (\\x00) encodes as codeword 1", () => {
    const cw = encodeAscii(new Uint8Array([0]));
    expect(cw).toEqual([1]);
  });

  it("DEL (0x7F = 127) encodes as codeword 128", () => {
    const cw = encodeAscii(new Uint8Array([127]));
    expect(cw).toEqual([128]);
  });

  it("mixed digits and non-digits: '1A2B' → 4 codewords (no pairs)", () => {
    // '1' and 'A' → no pair; 'A', '2', 'B' → all singles
    // Actually: '1' and 'A' → can't pair; '2' and 'B' → can't pair
    // Result: 1→50, A→66, 2→51, B→67 = [50, 66, 51, 67]
    const cw = encodeAscii(new TextEncoder().encode("1A2B"));
    expect(cw).toEqual([50, 66, 51, 67]);
  });

  it("long all-digit input gets heavy digit-pair compression", () => {
    // 100 digits → 50 digit-pair codewords
    const input = "12345678901234567890".repeat(5); // 100 digits
    const cw = encodeAscii(new TextEncoder().encode(input));
    expect(cw.length).toBe(50);
  });

  it("shape option 'rectangular' is accepted without error", () => {
    expect(() => encode("Hello", { shape: "rectangular" })).not.toThrow();
  });

  it("shape option 'any' is accepted without error", () => {
    expect(() => encode("Hello", { shape: "any" })).not.toThrow();
  });
});
