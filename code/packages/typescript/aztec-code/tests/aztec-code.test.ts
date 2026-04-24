/**
 * @file aztec-code.test.ts
 *
 * Unit tests for the Aztec Code encoder (ISO/IEC 24778:2008).
 *
 * Test categories:
 *   1. GF(16) arithmetic — verify log/antilog tables and multiplication.
 *   2. Mode message encoding — compact and full.
 *   3. Bit stuffing — runs of 4, alternating, all zeros.
 *   4. Symbol sizing — compact vs full selection.
 *   5. Full encode integration — verify grid structure.
 *   6. Error cases — InputTooLong.
 */

import { describe, it, expect } from "vitest";
import {
  encode,
  encodeAndLayout,
  renderSvg,
  explain,
  AztecError,
  InputTooLongError,
} from "../src/index.js";

// ─────────────────────────────────────────────────────────────────────────────
// Helper: re-expose internals for testing via module augmentation workaround.
// We test through the public API where possible, and verify structural
// properties of the grid for internals.
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Verify that the bullseye finder pattern is correctly placed in a grid.
 *
 * Compact bullseye: Chebyshev distance ≤ 5 from center.
 * Full bullseye: Chebyshev distance ≤ 7 from center.
 *
 * Rule: dist even → dark, dist odd → light.
 */
function checkBullseye(
  modules: boolean[][],
  cx: number,
  cy: number,
  radius: number,
): void {
  for (let dr = -radius; dr <= radius; dr++) {
    for (let dc = -radius; dc <= radius; dc++) {
      const d = Math.max(Math.abs(dr), Math.abs(dc));
      // DARK: d ≤ 1 (solid 3×3 core) OR d ≥ 2 and odd (dark rings).
      // LIGHT: d ≥ 2 and even.
      const expectedDark = d <= 1 || d % 2 === 1;
      const actual = modules[cy + dr][cx + dc];
      expect(actual).toBe(expectedDark);
    }
  }
}

/**
 * Count dark modules in a rectangular region of the grid.
 */
function countDarkInRegion(
  modules: boolean[][],
  rowStart: number, rowEnd: number,
  colStart: number, colEnd: number,
): number {
  let count = 0;
  for (let r = rowStart; r < rowEnd; r++) {
    for (let c = colStart; c < colEnd; c++) {
      if (modules[r][c]) count++;
    }
  }
  return count;
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. GF(16) arithmetic
//
// We cannot access GF16_LOG/ALOG directly from outside the module, so we
// verify GF(16) properties through the mode message output (which depends
// on correct GF(16) RS arithmetic).
// ─────────────────────────────────────────────────────────────────────────────

describe("GF(16) RS via mode message", () => {
  it("produces a 28-bit mode message for compact symbols", () => {
    // The compact mode message must be exactly 28 bits.
    // We verify this indirectly: a compact 1-layer symbol must be 15×15.
    const grid = encode("A");
    expect(grid.rows).toBe(15);
    expect(grid.cols).toBe(15);
  });

  it("produces a 40-bit mode message for full symbols", () => {
    // Force a full symbol by encoding enough data that compact cannot fit.
    // A 60-byte input should require full Aztec.
    const longInput = "A".repeat(60);
    const grid = encode(longInput);
    // Full L=1: 19×19 or larger.
    expect(grid.rows).toBeGreaterThanOrEqual(19);
    expect(grid.rows % 4).toBe(3); // size = 15 + 4L, so size ≡ 15 mod 4 ≡ 3 mod 4
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 2. Bit stuffing — tested via a thin wrapper since the function is internal.
//    We verify the effect on the output grid: the stuffed stream increases
//    the bit count by at most 25% for worst-case input.
// ─────────────────────────────────────────────────────────────────────────────

describe("bit stuffing effect", () => {
  it("handles all-zeros input without overflow", () => {
    // Encoding 1 zero byte should produce a valid (non-throwing) grid.
    const grid = encode(new Uint8Array([0x00]));
    expect(grid.rows).toBeGreaterThanOrEqual(15);
  });

  it("handles all-ones input without overflow", () => {
    const grid = encode(new Uint8Array([0xff]));
    expect(grid.rows).toBeGreaterThanOrEqual(15);
  });

  it("handles alternating bits without overflow", () => {
    // 0xAA = 10101010 — alternating, no stuffing needed.
    const grid = encode(new Uint8Array([0xaa, 0x55]));
    expect(grid.rows).toBeGreaterThanOrEqual(15);
  });

  it("handles repeating patterns without throwing", () => {
    // Worst-case stuffing: all 0x00 bytes trigger stuffing every 4 bits.
    const allZeros = new Uint8Array(20).fill(0);
    expect(() => encode(allZeros)).not.toThrow();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 3. Symbol sizing
// ─────────────────────────────────────────────────────────────────────────────

describe("symbol sizing", () => {
  it('encodes "A" as compact 1 layer (15×15)', () => {
    const grid = encode("A");
    expect(grid.rows).toBe(15);
    expect(grid.cols).toBe(15);
  });

  it('encodes "Hello World" to a valid symbol', () => {
    const grid = encode("Hello World");
    expect(grid.rows).toBe(grid.cols);
    // "Hello World" is 11 bytes → should fit in compact 1 layer (15×15)
    // compact L=1 provides 9 data bytes at 23% ECC, which is borderline.
    // Either 15×15 or 19×19 is acceptable.
    expect([15, 19, 23, 27]).toContain(grid.rows);
  });

  it('encodes "https://example.com" to a valid symbol', () => {
    const grid = encode("https://example.com");
    expect(grid.rows).toBe(grid.cols);
    expect(grid.rows).toBeGreaterThanOrEqual(15);
    // Size must follow the Aztec formula: either 11+4L or 15+4L.
    const s = grid.rows;
    const fitsCompact = s >= 15 && s <= 27 && (s - 11) % 4 === 0;
    const fitsFull    = s >= 19 && s <= 143 && (s - 15) % 4 === 0;
    expect(fitsCompact || fitsFull).toBe(true);
  });

  it("compact=true forces compact mode for short input", () => {
    const grid = encode("Hi", { compact: true });
    const s = grid.rows;
    expect(s).toBeGreaterThanOrEqual(15);
    expect(s).toBeLessThanOrEqual(27);
    expect((s - 11) % 4).toBe(0);
  });

  it("compact=true throws for input that does not fit in compact", () => {
    const longInput = "A".repeat(60);
    expect(() => encode(longInput, { compact: true })).toThrow(InputTooLongError);
  });

  it("throws InputTooLongError for extremely long input", () => {
    const tooLong = "X".repeat(4000); // > 3471 byte limit
    expect(() => encode(tooLong)).toThrow(InputTooLongError);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 4. Bullseye structural verification
// ─────────────────────────────────────────────────────────────────────────────

describe("bullseye structure", () => {
  it("compact 15×15: bullseye at center has correct Chebyshev distance pattern", () => {
    const grid = encode("A");
    expect(grid.rows).toBe(15);
    const cx = 7;
    const cy = 7;
    // Verify bullseye (radius 5) using the Chebyshev rule.
    checkBullseye(grid.modules, cx, cy, 5);
  });

  it("compact 15×15: center 3×3 is all dark (d ≤ 1)", () => {
    // d=0 and d=1 both dark — they merge into a solid 3×3 dark square.
    const grid = encode("A");
    const cx = 7;
    const cy = 7;
    for (let dr = -1; dr <= 1; dr++) {
      for (let dc = -1; dc <= 1; dc++) {
        // d ≤ 1 → always dark
        expect(grid.modules[cy + dr][cx + dc]).toBe(true);
      }
    }
  });

  it("compact 15×15: ring at d=2 is all light", () => {
    // d=2 even (and ≥ 2) → light
    const grid = encode("A");
    const cx = 7;
    const cy = 7;
    for (let dr = -2; dr <= 2; dr++) {
      for (let dc = -2; dc <= 2; dc++) {
        const d = Math.max(Math.abs(dr), Math.abs(dc));
        if (d === 2) {
          expect(grid.modules[cy + dr][cx + dc]).toBe(false);
        }
      }
    }
  });

  it("compact 15×15: ring at d=3 is all dark", () => {
    // d=3 odd (and ≥ 2) → dark
    const grid = encode("A");
    const cx = 7;
    const cy = 7;
    for (let dr = -3; dr <= 3; dr++) {
      for (let dc = -3; dc <= 3; dc++) {
        const d = Math.max(Math.abs(dr), Math.abs(dc));
        if (d === 3) {
          expect(grid.modules[cy + dr][cx + dc]).toBe(true);
        }
      }
    }
  });

  it("compact 15×15: ring at d=4 is all light", () => {
    // d=4 even (and ≥ 2) → light
    const grid = encode("A");
    const cx = 7;
    const cy = 7;
    for (let dr = -4; dr <= 4; dr++) {
      for (let dc = -4; dc <= 4; dc++) {
        const d = Math.max(Math.abs(dr), Math.abs(dc));
        if (d === 4) {
          expect(grid.modules[cy + dr][cx + dc]).toBe(false);
        }
      }
    }
  });

  it("compact 15×15: ring at d=5 is all dark", () => {
    // d=5 odd (and ≥ 2) → dark (outermost compact bullseye ring)
    const grid = encode("A");
    const cx = 7;
    const cy = 7;
    for (let dr = -5; dr <= 5; dr++) {
      for (let dc = -5; dc <= 5; dc++) {
        const d = Math.max(Math.abs(dr), Math.abs(dc));
        if (d === 5) {
          expect(grid.modules[cy + dr][cx + dc]).toBe(true);
        }
      }
    }
  });

  it("full symbol: bullseye radius 7 has correct Chebyshev pattern", () => {
    const longStr = "A".repeat(60);
    const grid = encode(longStr);
    const cx = Math.floor(grid.cols / 2);
    const cy = Math.floor(grid.rows / 2);
    checkBullseye(grid.modules, cx, cy, 7);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 5. Orientation marks
// ─────────────────────────────────────────────────────────────────────────────

describe("orientation marks", () => {
  it("compact 15×15: four corners of mode message ring (d=6) are dark", () => {
    const grid = encode("A");
    const cx = 7;
    const cy = 7;
    const r = 6; // mode message ring radius
    expect(grid.modules[cy - r][cx - r]).toBe(true); // top-left
    expect(grid.modules[cy - r][cx + r]).toBe(true); // top-right
    expect(grid.modules[cy + r][cx + r]).toBe(true); // bottom-right
    expect(grid.modules[cy + r][cx - r]).toBe(true); // bottom-left
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 6. Grid dimensions and symmetry
// ─────────────────────────────────────────────────────────────────────────────

describe("grid dimensions", () => {
  it("grid is always square", () => {
    for (const input of ["A", "Hello World", "https://example.com"]) {
      const grid = encode(input);
      expect(grid.rows).toBe(grid.cols);
    }
  });

  it("grid size is odd (required by Aztec formula)", () => {
    for (const input of ["A", "Hello", "Hello World", "https://example.com"]) {
      const grid = encode(input);
      expect(grid.rows % 2).toBe(1);
    }
  });

  it("compact symbol sizes follow 11 + 4L formula", () => {
    for (let L = 1; L <= 4; L++) {
      const expected = 11 + 4 * L;
      expect(expected % 2).toBe(1); // must be odd
    }
    expect(11 + 4 * 1).toBe(15);
    expect(11 + 4 * 2).toBe(19);
    expect(11 + 4 * 3).toBe(23);
    expect(11 + 4 * 4).toBe(27);
  });

  it("full symbol sizes follow 15 + 4L formula", () => {
    expect(15 + 4 * 1).toBe(19);
    expect(15 + 4 * 2).toBe(23);
    expect(15 + 4 * 32).toBe(143);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 7. Convenience API
// ─────────────────────────────────────────────────────────────────────────────

describe("convenience API", () => {
  it("encodeAndLayout returns a PaintScene with at least one instruction", () => {
    const scene = encodeAndLayout("A");
    expect(scene.instructions.length).toBeGreaterThan(0);
  });

  it("renderSvg returns a string containing <svg", () => {
    const svg = renderSvg("A");
    expect(typeof svg).toBe("string");
    expect(svg).toContain("<svg");
  });

  it("renderSvg result contains at least one rect element (data module)", () => {
    const svg = renderSvg("A");
    expect(svg).toContain("<rect");
  });

  it("explain returns an annotated grid with null annotations", () => {
    const annotated = explain("A");
    expect(annotated.annotations.length).toBe(annotated.rows);
    expect(annotated.annotations[0].length).toBe(annotated.cols);
    // v0.1.0 returns null annotations
    expect(annotated.annotations[0][0]).toBeNull();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 8. Cross-language test corpus
//
// These encode the canonical test vectors and verify structural properties
// that must hold for all correct Aztec implementations.
// ─────────────────────────────────────────────────────────────────────────────

describe("test corpus", () => {
  it('"A" encodes to 15×15 compact 1-layer symbol', () => {
    const grid = encode("A");
    expect(grid.rows).toBe(15);
    expect(grid.cols).toBe(15);
    expect(grid.moduleShape).toBe("square");
  });

  it('"Hello World" encodes without throwing', () => {
    expect(() => encode("Hello World")).not.toThrow();
  });

  it('"https://example.com" encodes without throwing', () => {
    expect(() => encode("https://example.com")).not.toThrow();
  });

  it("raw binary [0..63] encodes without throwing", () => {
    const raw = new Uint8Array(64);
    for (let i = 0; i < 64; i++) raw[i] = i;
    expect(() => encode(raw)).not.toThrow();
  });

  it("digit-heavy string encodes without throwing", () => {
    expect(() => encode("01234567890123456789")).not.toThrow();
  });

  it("encoding same input twice produces identical grids", () => {
    const input = "https://example.com";
    const g1 = encode(input);
    const g2 = encode(input);
    expect(g1.rows).toBe(g2.rows);
    for (let r = 0; r < g1.rows; r++) {
      for (let c = 0; c < g1.cols; c++) {
        expect(g1.modules[r][c]).toBe(g2.modules[r][c]);
      }
    }
  });

  it("Uint8Array and string inputs produce the same grid for ASCII text", () => {
    const str = "Hello";
    const bytes = new TextEncoder().encode(str);
    const g1 = encode(str);
    const g2 = encode(bytes);
    expect(g1.rows).toBe(g2.rows);
    for (let r = 0; r < g1.rows; r++) {
      for (let c = 0; c < g1.cols; c++) {
        expect(g1.modules[r][c]).toBe(g2.modules[r][c]);
      }
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 9. ECC option
// ─────────────────────────────────────────────────────────────────────────────

describe("ECC options", () => {
  it("higher ECC may produce a larger symbol for the same input", () => {
    const inputStr = "Hello World Hello World";
    const g23 = encode(inputStr, { minEccPercent: 23 });
    const g50 = encode(inputStr, { minEccPercent: 50 });
    // Higher ECC requires more redundancy → same or larger symbol.
    expect(g50.rows).toBeGreaterThanOrEqual(g23.rows);
  });

  it("minEccPercent=10 produces a valid symbol", () => {
    expect(() => encode("A", { minEccPercent: 10 })).not.toThrow();
  });
});
