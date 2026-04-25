/**
 * Test suite for the @coding-adventures/pdf417 TypeScript package.
 *
 * We verify:
 *  - GF(929) arithmetic (tables and operations)
 *  - Byte compaction
 *  - Reed-Solomon ECC generation (sanity checks)
 *  - Row indicator computation (LRI and RRI)
 *  - Start / stop pattern expansion
 *  - Module grid dimensions (width formula, row height scaling)
 *  - Every row starts with the start pattern and ends with the stop pattern
 *  - Integration tests (encode hello world, all-256-bytes, empty input)
 *  - Error cases (invalid ECC level, invalid columns, input too long)
 *  - Determinism (same input → same bits every time)
 *  - Structural correctness (repeated rows match when rowHeight > 1)
 *  - encode_and_layout returns a valid PaintScene
 */

import { describe, it, expect } from "vitest";

import {
  encode,
  encodeAndLayout,
  computeLRI,
  computeRRI,
  PDF417Error,
  InputTooLongError,
  InvalidDimensionsError,
  InvalidECCLevelError,
  _testing,
} from "../src/index.js";

const { gfMul, gfAdd, GF_EXP, GF_LOG, byteCompact, expandWidths, expandPattern, chooseDimensions, autoEccLevel } = _testing;

import { CLUSTER_TABLES, START_PATTERN, STOP_PATTERN } from "../src/cluster-tables.js";

// ─────────────────────────────────────────────────────────────────────────────
// Helper: read a module row as a bit string
// ─────────────────────────────────────────────────────────────────────────────

function rowBits(grid: { rows: number; cols: number; modules: ReadonlyArray<ReadonlyArray<boolean>> }, row: number): string {
  return grid.modules[row].map((d) => (d ? "1" : "0")).join("");
}

const START_BITS = "11111111010101000";
const STOP_BITS  = "111111101000101001";

// ─────────────────────────────────────────────────────────────────────────────
// Cluster tables
// ─────────────────────────────────────────────────────────────────────────────

describe("cluster tables", () => {
  it("has 3 clusters", () => {
    expect(CLUSTER_TABLES.length).toBe(3);
  });

  it("each cluster has 929 entries", () => {
    for (const cluster of CLUSTER_TABLES) {
      expect(cluster.length).toBe(929);
    }
  });

  it("all entries are non-zero u32 values", () => {
    for (const cluster of CLUSTER_TABLES) {
      for (const entry of cluster) {
        expect(entry).toBeGreaterThan(0);
        expect(entry).toBeLessThanOrEqual(0xFFFFFFFF);
      }
    }
  });

  it("expanding any cluster-table entry yields exactly 17 modules", () => {
    // Spot-check 10 entries from each cluster.
    for (let ci = 0; ci < 3; ci++) {
      for (let cw = 0; cw < 929; cw += 93) {
        const modules: boolean[] = [];
        expandPattern(CLUSTER_TABLES[ci][cw] as number, modules);
        expect(modules.length).toBe(17);
      }
    }
  });

  it("start pattern decodes to 17 modules matching START_BITS", () => {
    const modules: boolean[] = [];
    expandWidths(START_PATTERN, modules);
    expect(modules.length).toBe(17);
    expect(modules.map((d) => (d ? "1" : "0")).join("")).toBe(START_BITS);
  });

  it("stop pattern decodes to 18 modules matching STOP_BITS", () => {
    const modules: boolean[] = [];
    expandWidths(STOP_PATTERN, modules);
    expect(modules.length).toBe(18);
    expect(modules.map((d) => (d ? "1" : "0")).join("")).toBe(STOP_BITS);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// GF(929) arithmetic
// ─────────────────────────────────────────────────────────────────────────────

describe("GF(929) tables", () => {
  it("GF_EXP[0] = 1 (α^0 = 1)", () => {
    expect(GF_EXP[0]).toBe(1);
  });

  it("GF_EXP[1] = 3 (α^1 = 3)", () => {
    expect(GF_EXP[1]).toBe(3);
  });

  it("GF_EXP[2] = 9 (α^2 = 9)", () => {
    expect(GF_EXP[2]).toBe(9);
  });

  it("GF_EXP[3] = 27 (α^3 = 27)", () => {
    expect(GF_EXP[3]).toBe(27);
  });

  it("GF_EXP[928] = 1 (Fermat: α^{928} ≡ 1 mod 929)", () => {
    expect(GF_EXP[928]).toBe(1);
  });

  it("GF_LOG[1] = 0 (log_α(1) = 0)", () => {
    expect(GF_LOG[1]).toBe(0);
  });

  it("GF_LOG[3] = 1 (log_α(3) = 1)", () => {
    expect(GF_LOG[3]).toBe(1);
  });
});

describe("GF(929) arithmetic operations", () => {
  it("gfAdd: (100 + 900) mod 929 = 71", () => {
    expect(gfAdd(100, 900)).toBe(71);
  });

  it("gfAdd: 928 + 1 = 0 (mod 929)", () => {
    expect(gfAdd(928, 1)).toBe(0);
  });

  it("gfAdd: identity 0 + 500 = 500", () => {
    expect(gfAdd(0, 500)).toBe(500);
  });

  it("gfMul: 3 × 3 = 9", () => {
    expect(gfMul(3, 3)).toBe(9);
  });

  it("gfMul: 3 × 310 = 1 (310 is the multiplicative inverse of 3)", () => {
    // 3 × 310 = 930 ≡ 1 (mod 929)
    expect(gfMul(3, 310)).toBe(1);
  });

  it("gfMul: 0 × anything = 0", () => {
    expect(gfMul(0, 500)).toBe(0);
    expect(gfMul(500, 0)).toBe(0);
  });

  it("gfMul: 1 × 928 = 928 (identity)", () => {
    expect(gfMul(1, 928)).toBe(928);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Byte compaction
// ─────────────────────────────────────────────────────────────────────────────

describe("byteCompact", () => {
  it("empty input → just the latch codeword [924]", () => {
    expect(byteCompact(new Uint8Array([]))).toEqual([924]);
  });

  it("single byte → [924, byte]", () => {
    const result = byteCompact(new Uint8Array([65]));
    expect(result[0]).toBe(924);
    expect(result[1]).toBe(65);
    expect(result.length).toBe(2);
  });

  it("6 bytes → latch + 5 base-900 codewords", () => {
    const result = byteCompact(new Uint8Array([0x41, 0x42, 0x43, 0x44, 0x45, 0x46]));
    expect(result[0]).toBe(924);
    expect(result.length).toBe(6);

    // Verify round-trip manually.
    const n = BigInt("0x41") * 256n ** 5n
            + BigInt("0x42") * 256n ** 4n
            + BigInt("0x43") * 256n ** 3n
            + BigInt("0x44") * 256n ** 2n
            + BigInt("0x45") * 256n
            + BigInt("0x46");
    const expected = [];
    let rem = n;
    for (let j = 4; j >= 0; j--) {
      expected[j] = Number(rem % 900n);
      rem = rem / 900n;
    }
    for (let i = 0; i < 5; i++) {
      expect(result[i + 1]).toBe(expected[i]);
    }
  });

  it("7 bytes → latch + 5 codewords (6-byte group) + 1 direct", () => {
    const result = byteCompact(new Uint8Array([65, 66, 67, 68, 69, 70, 71]));
    expect(result[0]).toBe(924);
    expect(result.length).toBe(7);
    expect(result[6]).toBe(71);
  });

  it("12 bytes → latch + 5 + 5 codewords (two complete 6-byte groups)", () => {
    const result = byteCompact(new Uint8Array(12).fill(65));
    expect(result[0]).toBe(924);
    expect(result.length).toBe(11); // 1 latch + 5 + 5
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Row indicators
// ─────────────────────────────────────────────────────────────────────────────

describe("row indicators (LRI / RRI)", () => {
  // R=4, C=3, L=2 → R_info=1, C_info=2, L_info=6
  // Cluster 0: LRI=R_info=1, RRI=C_info=2
  // Cluster 1: LRI=L_info=6, RRI=R_info=1
  // Cluster 2: LRI=C_info=2, RRI=L_info=6
  // Row 3 (cluster 0): rowGroup=1 → LRI=31, RRI=32
  it("row 0 cluster 0: LRI=R_info=1, RRI=C_info=2", () => {
    expect(computeLRI(0, 4, 3, 2)).toBe(1);
    expect(computeRRI(0, 4, 3, 2)).toBe(2);
  });

  it("row 1 cluster 1: LRI=L_info=6, RRI=R_info=1", () => {
    expect(computeLRI(1, 4, 3, 2)).toBe(6);
    expect(computeRRI(1, 4, 3, 2)).toBe(1);
  });

  it("row 2 cluster 2: LRI=C_info=2, RRI=L_info=6", () => {
    expect(computeLRI(2, 4, 3, 2)).toBe(2);
    expect(computeRRI(2, 4, 3, 2)).toBe(6);
  });

  it("row 3 cluster 0 row_group=1: LRI=31, RRI=32", () => {
    expect(computeLRI(3, 4, 3, 2)).toBe(31);
    expect(computeRRI(3, 4, 3, 2)).toBe(32);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Dimension heuristic
// ─────────────────────────────────────────────────────────────────────────────

describe("chooseDimensions", () => {
  it("minimum rows is always 3", () => {
    const { rows } = chooseDimensions(1);
    expect(rows).toBeGreaterThanOrEqual(3);
  });

  it("minimum cols is always 1", () => {
    const { cols } = chooseDimensions(1);
    expect(cols).toBeGreaterThanOrEqual(1);
  });

  it("cols × rows >= total for typical inputs", () => {
    for (const total of [1, 10, 50, 100, 500]) {
      const { cols, rows } = chooseDimensions(total);
      expect(cols * rows).toBeGreaterThanOrEqual(total);
    }
  });
});

describe("autoEccLevel", () => {
  it("≤40 data codewords → level 2", () => {
    expect(autoEccLevel(10)).toBe(2);
    expect(autoEccLevel(40)).toBe(2);
  });

  it("41–160 → level 3", () => {
    expect(autoEccLevel(41)).toBe(3);
    expect(autoEccLevel(160)).toBe(3);
  });

  it("161–320 → level 4", () => {
    expect(autoEccLevel(161)).toBe(4);
    expect(autoEccLevel(320)).toBe(4);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Module width formula
// ─────────────────────────────────────────────────────────────────────────────

describe("symbol dimensions", () => {
  it("module_width = 69 + 17 * cols for various column counts", () => {
    for (const c of [1, 3, 5, 10, 30]) {
      const grid = encode(new TextEncoder().encode("HELLO WORLD HELLO WORLD"), { columns: c, rowHeight: 1 });
      expect(grid.cols).toBe(69 + 17 * c);
    }
  });

  it("minimum logical rows enforced (≥3 logical rows)", () => {
    const grid = encode(new TextEncoder().encode("A"), { rowHeight: 1 });
    // With rowHeight=1, grid.rows == logical rows.
    expect(grid.rows).toBeGreaterThanOrEqual(3);
  });

  it("row_height=6 produces twice the module rows as row_height=3", () => {
    const g3 = encode(new TextEncoder().encode("A"), { rowHeight: 3 });
    const g6 = encode(new TextEncoder().encode("A"), { rowHeight: 6 });
    expect(g6.rows).toBe(g3.rows * 2);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Start / stop pattern in every row
// ─────────────────────────────────────────────────────────────────────────────

describe("every row starts and ends correctly", () => {
  it("every module row starts with the start pattern", () => {
    const grid = encode(new TextEncoder().encode("TEST"), { columns: 3, rowHeight: 1 });
    for (let r = 0; r < grid.rows; r++) {
      const bits = rowBits(grid, r);
      expect(bits.slice(0, 17)).toBe(START_BITS);
    }
  });

  it("every module row ends with the stop pattern", () => {
    const grid = encode(new TextEncoder().encode("TEST"), { columns: 3, rowHeight: 1 });
    for (let r = 0; r < grid.rows; r++) {
      const bits = rowBits(grid, r);
      expect(bits.slice(-18)).toBe(STOP_BITS);
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Integration tests
// ─────────────────────────────────────────────────────────────────────────────

describe("encode() integration", () => {
  it("encodes a single byte without error", () => {
    const grid = encode(new TextEncoder().encode("A"));
    expect(grid.rows).toBeGreaterThanOrEqual(3);
    expect(grid.cols).toBeGreaterThanOrEqual(69 + 17);
  });

  it("encodes 'HELLO WORLD' with correct start/stop in every row", () => {
    const grid = encode(new TextEncoder().encode("HELLO WORLD"), { rowHeight: 1 });
    for (let r = 0; r < grid.rows; r++) {
      const bits = rowBits(grid, r);
      expect(bits.slice(0, 17)).toBe(START_BITS);
      expect(bits.slice(-18)).toBe(STOP_BITS);
    }
  });

  it("encodes all 256 byte values without error", () => {
    const bytes = new Uint8Array(256);
    for (let i = 0; i < 256; i++) bytes[i] = i;
    const grid = encode(bytes);
    expect(grid.rows).toBeGreaterThanOrEqual(3);
  });

  it("encodes repeated 0xFF bytes without error", () => {
    const bytes = new Uint8Array(256).fill(0xff);
    const grid = encode(bytes);
    expect(grid.rows).toBeGreaterThanOrEqual(3);
  });

  it("encodes empty input without error", () => {
    const grid = encode(new Uint8Array(0));
    expect(grid.rows).toBeGreaterThanOrEqual(3);
  });

  it("is deterministic: same input yields identical grids", () => {
    const text = new TextEncoder().encode("PDF417 TEST");
    const g1 = encode(text);
    const g2 = encode(text);
    expect(g1.rows).toBe(g2.rows);
    expect(g1.cols).toBe(g2.cols);
    for (let r = 0; r < g1.rows; r++) {
      for (let c = 0; c < g1.cols; c++) {
        expect(g1.modules[r][c]).toBe(g2.modules[r][c]);
      }
    }
  });

  it("different inputs produce different grids", () => {
    const g1 = encode(new TextEncoder().encode("AAA"), { rowHeight: 1 });
    const g2 = encode(new TextEncoder().encode("BBB"), { rowHeight: 1 });
    let differ = false;
    for (let r = 0; r < Math.min(g1.rows, g2.rows); r++) {
      for (let c = 0; c < Math.min(g1.cols, g2.cols); c++) {
        if (g1.modules[r][c] !== g2.modules[r][c]) {
          differ = true;
          break;
        }
      }
      if (differ) break;
    }
    expect(differ).toBe(true);
  });

  it("row repetition: each logical row repeats rowHeight times", () => {
    const rowHeight = 4;
    const grid = encode(new TextEncoder().encode("HELLO"), { rowHeight, columns: 3 });
    const logicalRows = grid.rows / rowHeight;
    for (let lr = 0; lr < logicalRows; lr++) {
      for (let h = 1; h < rowHeight; h++) {
        for (let c = 0; c < grid.cols; c++) {
          expect(grid.modules[lr * rowHeight][c]).toBe(grid.modules[lr * rowHeight + h][c]);
        }
      }
    }
  });

  it("higher ECC level produces a larger or equal symbol", () => {
    const text = new TextEncoder().encode("HELLO WORLD");
    const g2 = encode(text, { eccLevel: 2 });
    const g4 = encode(text, { eccLevel: 4 });
    expect(g4.rows * g4.cols).toBeGreaterThanOrEqual(g2.rows * g2.cols);
  });

  it("ECC level 0 is accepted", () => {
    expect(() => encode(new TextEncoder().encode("A"), { eccLevel: 0 })).not.toThrow();
  });

  it("ECC level 8 is accepted", () => {
    expect(() => encode(new TextEncoder().encode("A"), { eccLevel: 8 })).not.toThrow();
  });

  it("accepts a number[] instead of Uint8Array", () => {
    expect(() => encode([65, 66, 67])).not.toThrow();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Error cases
// ─────────────────────────────────────────────────────────────────────────────

describe("error cases", () => {
  it("ECC level 9 throws InvalidECCLevelError", () => {
    expect(() => encode(new TextEncoder().encode("A"), { eccLevel: 9 }))
      .toThrow(InvalidECCLevelError);
  });

  it("ECC level -1 throws InvalidECCLevelError", () => {
    expect(() => encode(new TextEncoder().encode("A"), { eccLevel: -1 }))
      .toThrow(InvalidECCLevelError);
  });

  it("columns 0 throws InvalidDimensionsError", () => {
    expect(() => encode(new TextEncoder().encode("A"), { columns: 0 }))
      .toThrow(InvalidDimensionsError);
  });

  it("columns 31 throws InvalidDimensionsError", () => {
    expect(() => encode(new TextEncoder().encode("A"), { columns: 31 }))
      .toThrow(InvalidDimensionsError);
  });

  it("too much data with columns=1 throws InputTooLongError", () => {
    const huge = new Uint8Array(3000).fill(65);
    expect(() => encode(huge, { columns: 1 })).toThrow(InputTooLongError);
  });

  it("all error classes extend PDF417Error", () => {
    expect(new InputTooLongError("x")).toBeInstanceOf(PDF417Error);
    expect(new InvalidDimensionsError("x")).toBeInstanceOf(PDF417Error);
    expect(new InvalidECCLevelError("x")).toBeInstanceOf(PDF417Error);
  });

  it("all error classes extend Error", () => {
    expect(new PDF417Error("x")).toBeInstanceOf(Error);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// encodeAndLayout
// ─────────────────────────────────────────────────────────────────────────────

describe("encodeAndLayout", () => {
  it("returns a PaintScene with positive width and height", () => {
    const scene = encodeAndLayout(new TextEncoder().encode("HELLO"));
    expect(scene.width).toBeGreaterThan(0);
    expect(scene.height).toBeGreaterThan(0);
  });

  it("returns a PaintScene with at least 2 instructions (background + dark modules)", () => {
    const scene = encodeAndLayout(new TextEncoder().encode("HELLO"));
    expect(scene.instructions.length).toBeGreaterThan(1);
  });
});
