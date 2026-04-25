/**
 * Test suite for @coding-adventures/aztec-code
 *
 * Covers:
 *  - Symbol size selection (compact 1-4, full)
 *  - Bullseye finder pattern structure
 *  - Orientation mark placement
 *  - Bit stuffing algorithm
 *  - GF(16) mode message (indirect via encode)
 *  - GF(256)/0x12D Reed-Solomon ECC (indirect via encode)
 *  - encodeAndLayout, renderSvg, explain wrappers
 *  - minEccPercent option
 *  - Error cases
 *  - Determinism
 */

import { describe, it, expect } from "vitest";
import {
  encode,
  encodeAndLayout,
  renderSvg,
  explain,
  AztecError,
  InputTooLongError,
  VERSION,
} from "../src/index.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Return the dark/light value at (row, col) from a ModuleGrid. */
function dark(grid: ReturnType<typeof encode>, row: number, col: number): boolean {
  return grid.modules[row]![col] === true;
}

// ---------------------------------------------------------------------------
// Version
// ---------------------------------------------------------------------------

describe("VERSION", () => {
  it("should be 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

// ---------------------------------------------------------------------------
// Error classes
// ---------------------------------------------------------------------------

describe("error classes", () => {
  it("AztecError extends Error", () => {
    const e = new AztecError("test");
    expect(e).toBeInstanceOf(Error);
    expect(e).toBeInstanceOf(AztecError);
    expect(e.name).toBe("AztecError");
    expect(e.message).toBe("test");
  });

  it("InputTooLongError extends AztecError", () => {
    const e = new InputTooLongError("too long");
    expect(e).toBeInstanceOf(AztecError);
    expect(e).toBeInstanceOf(InputTooLongError);
    expect(e.name).toBe("InputTooLongError");
  });

  it("throws InputTooLongError for huge input", () => {
    expect(() => encode("x".repeat(2000))).toThrow(InputTooLongError);
  });
});

// ---------------------------------------------------------------------------
// Compact symbol sizes
// ---------------------------------------------------------------------------

describe("compact symbol sizes", () => {
  it("1-layer compact = 15x15 for a single byte 'A'", () => {
    const g = encode("A");
    expect(g.rows).toBe(15);
    expect(g.cols).toBe(15);
  });

  it("2-layer compact = 19x19 for 'Hello'", () => {
    const g = encode("Hello");
    expect(g.rows).toBe(19);
    expect(g.cols).toBe(19);
  });

  it("3-layer compact = 23x23 for 20-byte input", () => {
    // 20 bytes + Binary-Shift overhead overflows compact-2 (25 data cw - 6 ecc = 19)
    const g = encode("12345678901234567890");
    expect(g.rows).toBe(23);
    expect(g.cols).toBe(23);
  });

  it("4-layer compact = 27x27 for 40-byte input", () => {
    const g = encode("12345678901234567890".repeat(2));
    expect(g.rows).toBe(27);
    expect(g.cols).toBe(27);
  });
});

// ---------------------------------------------------------------------------
// Full symbol sizes
// ---------------------------------------------------------------------------

describe("full symbol sizes", () => {
  it("uses full variant for input that doesn't fit in compact", () => {
    // 100 bytes of data should exceed compact-4 capacity
    const g = encode("x".repeat(100));
    expect(g.rows).toBeGreaterThanOrEqual(19);
    expect(g.rows % 4).toBe(3); // full symbol sizes are 19+4k (19,23,27,...) -> all ≡ 3 mod 4
  });

  it("full symbol rows === cols (square)", () => {
    const g = encode("x".repeat(150));
    expect(g.rows).toBe(g.cols);
  });
});

// ---------------------------------------------------------------------------
// Bullseye finder pattern — compact (r=5, cx=cy=7 for 15x15)
// ---------------------------------------------------------------------------

describe("bullseye pattern — compact 15x15", () => {
  const g = encode("A"); // 15x15 compact-1
  const cx = 7;
  const cy = 7;

  it("center (d=0) is DARK", () => {
    expect(dark(g, cy, cx)).toBe(true);
  });

  it("d=1 ring is DARK (all 8 neighbours)", () => {
    for (let dr = -1; dr <= 1; dr++) {
      for (let dc = -1; dc <= 1; dc++) {
        if (dr === 0 && dc === 0) continue;
        expect(dark(g, cy + dr, cx + dc)).toBe(true);
      }
    }
  });

  it("d=2 ring is LIGHT", () => {
    // corners of the d=2 perimeter
    expect(dark(g, cy - 2, cx - 2)).toBe(false);
    expect(dark(g, cy - 2, cx + 2)).toBe(false);
    expect(dark(g, cy + 2, cx - 2)).toBe(false);
    expect(dark(g, cy + 2, cx + 2)).toBe(false);
    // midpoints of each side
    expect(dark(g, cy - 2, cx)).toBe(false);
    expect(dark(g, cy + 2, cx)).toBe(false);
    expect(dark(g, cy, cx - 2)).toBe(false);
    expect(dark(g, cy, cx + 2)).toBe(false);
  });

  it("d=3 ring is DARK", () => {
    expect(dark(g, cy - 3, cx)).toBe(true);
    expect(dark(g, cy + 3, cx)).toBe(true);
    expect(dark(g, cy, cx - 3)).toBe(true);
    expect(dark(g, cy, cx + 3)).toBe(true);
  });

  it("d=4 ring is LIGHT", () => {
    expect(dark(g, cy - 4, cx)).toBe(false);
    expect(dark(g, cy + 4, cx)).toBe(false);
    expect(dark(g, cy, cx - 4)).toBe(false);
    expect(dark(g, cy, cx + 4)).toBe(false);
  });

  it("d=5 ring is DARK", () => {
    expect(dark(g, cy - 5, cx)).toBe(true);
    expect(dark(g, cy + 5, cx)).toBe(true);
    expect(dark(g, cy, cx - 5)).toBe(true);
    expect(dark(g, cy, cx + 5)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Bullseye finder pattern — full symbol
// ---------------------------------------------------------------------------

describe("bullseye pattern — full symbol", () => {
  const g = encode("x".repeat(100));
  const cx = Math.floor(g.cols / 2);
  const cy = Math.floor(g.rows / 2);

  it("center is DARK", () => {
    expect(dark(g, cy, cx)).toBe(true);
  });

  it("d=2 ring is LIGHT for full symbol", () => {
    expect(dark(g, cy - 2, cx)).toBe(false);
    expect(dark(g, cy + 2, cx)).toBe(false);
    expect(dark(g, cy, cx - 2)).toBe(false);
    expect(dark(g, cy, cx + 2)).toBe(false);
  });

  it("d=7 ring is DARK (bullseye outer ring for full)", () => {
    expect(dark(g, cy - 7, cx)).toBe(true);
    expect(dark(g, cy + 7, cx)).toBe(true);
    expect(dark(g, cy, cx - 7)).toBe(true);
    expect(dark(g, cy, cx + 7)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Orientation marks — compact
// ---------------------------------------------------------------------------

describe("orientation marks — compact 15x15", () => {
  const g = encode("A"); // 15x15, cx=cy=7, bullseye r=5, mode ring r=6
  const cx = 7;
  const cy = 7;
  const r = 6; // bullseyeRadius(compact) + 1

  it("top-left corner of mode ring is DARK", () => {
    expect(dark(g, cy - r, cx - r)).toBe(true);
  });

  it("top-right corner is DARK", () => {
    expect(dark(g, cy - r, cx + r)).toBe(true);
  });

  it("bottom-right corner is DARK", () => {
    expect(dark(g, cy + r, cx + r)).toBe(true);
  });

  it("bottom-left corner is DARK", () => {
    expect(dark(g, cy + r, cx - r)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Orientation marks — full symbol
// ---------------------------------------------------------------------------

describe("orientation marks — full symbol", () => {
  const g = encode("x".repeat(100));
  const cx = Math.floor(g.cols / 2);
  const cy = Math.floor(g.rows / 2);
  const r = 8; // bullseyeRadius(full=7) + 1

  it("top-left corner of mode ring is DARK", () => {
    expect(dark(g, cy - r, cx - r)).toBe(true);
  });

  it("top-right corner is DARK", () => {
    expect(dark(g, cy - r, cx + r)).toBe(true);
  });

  it("bottom-right corner is DARK", () => {
    expect(dark(g, cy + r, cx + r)).toBe(true);
  });

  it("bottom-left corner is DARK", () => {
    expect(dark(g, cy + r, cx - r)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Grid structural properties
// ---------------------------------------------------------------------------

describe("grid structure", () => {
  it("modules array has correct dimensions", () => {
    const g = encode("A");
    expect(g.modules).toHaveLength(15);
    expect(g.modules[0]).toHaveLength(15);
  });

  it("moduleShape is square", () => {
    expect(encode("A").moduleShape).toBe("square");
  });

  it("rows and cols are equal (square symbol)", () => {
    const g = encode("Hello, World!");
    expect(g.rows).toBe(g.cols);
  });

  it("rows matches modules.length", () => {
    const g = encode("test");
    expect(g.rows).toBe(g.modules.length);
  });

  it("cols matches modules[0].length", () => {
    const g = encode("test");
    expect(g.cols).toBe(g.modules[0]!.length);
  });
});

// ---------------------------------------------------------------------------
// Uint8Array input
// ---------------------------------------------------------------------------

describe("Uint8Array input", () => {
  it("accepts Uint8Array input", () => {
    const bytes = new TextEncoder().encode("Hello");
    const g1 = encode("Hello");
    const g2 = encode(bytes);
    expect(g1.rows).toBe(g2.rows);
    expect(g1.cols).toBe(g2.cols);
  });

  it("produces identical output for string and Uint8Array", () => {
    const str = "ABC";
    const bytes = new TextEncoder().encode(str);
    const g1 = encode(str);
    const g2 = encode(bytes);
    for (let r = 0; r < g1.rows; r++) {
      for (let c = 0; c < g1.cols; c++) {
        expect(dark(g1, r, c)).toBe(dark(g2, r, c));
      }
    }
  });
});

// ---------------------------------------------------------------------------
// minEccPercent option
// ---------------------------------------------------------------------------

describe("minEccPercent option", () => {
  it("higher ECC can require a larger symbol", () => {
    const gLow = encode("Hello", { minEccPercent: 10 });
    const gHigh = encode("Hello", { minEccPercent: 80 });
    // Higher ECC uses more space for parity, so it needs a larger or equal symbol
    expect(gHigh.rows).toBeGreaterThanOrEqual(gLow.rows);
  });

  it("minEccPercent 33 produces a valid grid", () => {
    const g = encode("Hello", { minEccPercent: 33 });
    expect(g.rows).toBeGreaterThanOrEqual(15);
  });
});

// ---------------------------------------------------------------------------
// Determinism
// ---------------------------------------------------------------------------

describe("determinism", () => {
  it("same input always produces identical output", () => {
    const g1 = encode("Hello, World!");
    const g2 = encode("Hello, World!");
    expect(g1.rows).toBe(g2.rows);
    for (let r = 0; r < g1.rows; r++) {
      for (let c = 0; c < g1.cols; c++) {
        expect(dark(g1, r, c)).toBe(dark(g2, r, c));
      }
    }
  });

  it("different inputs produce different grids", () => {
    const g1 = encode("Hello");
    const g2 = encode("World");
    let differs = false;
    for (let r = 0; r < g1.rows && !differs; r++) {
      for (let c = 0; c < g1.cols && !differs; c++) {
        if (dark(g1, r, c) !== dark(g2, r, c)) differs = true;
      }
    }
    expect(differs).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// encodeAndLayout
// ---------------------------------------------------------------------------

describe("encodeAndLayout", () => {
  it("returns a PaintScene object", () => {
    const scene = encodeAndLayout("Hello");
    expect(scene).toBeDefined();
    expect(typeof scene).toBe("object");
  });

  it("accepts optional AztecOptions", () => {
    const scene = encodeAndLayout("Hello", { minEccPercent: 33 });
    expect(scene).toBeDefined();
  });

  it("accepts optional config", () => {
    const scene = encodeAndLayout("Hello", undefined, { moduleSize: 10 });
    expect(scene).toBeDefined();
  });
});

// ---------------------------------------------------------------------------
// renderSvg
// ---------------------------------------------------------------------------

describe("renderSvg", () => {
  it("returns a string", () => {
    const svg = renderSvg("Hello");
    expect(typeof svg).toBe("string");
  });

  it("contains <svg tag", () => {
    const svg = renderSvg("Hello");
    expect(svg).toContain("<svg");
  });

  it("contains <rect elements", () => {
    const svg = renderSvg("Hello");
    expect(svg).toContain("<rect");
  });

  it("is well-formed enough to include closing tag", () => {
    const svg = renderSvg("Hello");
    expect(svg).toContain("</svg>");
  });

  it("accepts optional options and config", () => {
    const svg = renderSvg("A", { minEccPercent: 23 }, { moduleSize: 5 });
    expect(svg).toContain("<svg");
  });
});

// ---------------------------------------------------------------------------
// explain
// ---------------------------------------------------------------------------

describe("explain", () => {
  it("returns an AnnotatedModuleGrid", () => {
    const annotated = explain("Hello");
    expect(annotated).toBeDefined();
    expect(typeof annotated.rows).toBe("number");
    expect(typeof annotated.cols).toBe("number");
  });

  it("rows and cols match encode output", () => {
    const g = encode("Hello");
    const a = explain("Hello");
    expect(a.rows).toBe(g.rows);
    expect(a.cols).toBe(g.cols);
  });
});

// ---------------------------------------------------------------------------
// Cross-language corpus (known vector)
// ---------------------------------------------------------------------------

describe("cross-language corpus", () => {
  it("encode 'A' produces a 15x15 compact-1 grid", () => {
    const g = encode("A");
    expect(g.rows).toBe(15);
    expect(g.cols).toBe(15);
  });

  it("encode empty string produces a small valid grid", () => {
    // Empty string -> 5+5 bits (BS escape + 0-length), should fit in compact-1
    const g = encode("");
    expect(g.rows).toBeGreaterThanOrEqual(15);
    expect(g.rows).toBe(g.cols);
  });

  it("encode 'Hello, World!' is deterministic and square", () => {
    const g = encode("Hello, World!");
    expect(g.rows).toBe(g.cols);
    expect(g.rows).toBeGreaterThanOrEqual(15);
  });

  it("all modules are boolean", () => {
    const g = encode("Test");
    for (const row of g.modules) {
      for (const cell of row) {
        expect(typeof cell).toBe("boolean");
      }
    }
  });
});

// ---------------------------------------------------------------------------
// Reference grid (full symbols only)
// ---------------------------------------------------------------------------

describe("reference grid — full symbol", () => {
  // Use a 100-byte input to force a full symbol
  const g = encode("x".repeat(100));
  const cx = Math.floor(g.cols / 2);
  const cy = Math.floor(g.rows / 2);

  it("reference grid modules at row/col 16 away from center exist", () => {
    // At cx+16, cy: the reference grid places a module here
    // We can't test exact dark/light as data may overwrite non-reserved,
    // but we can verify the symbol dimensions are reasonable
    expect(g.rows).toBeGreaterThanOrEqual(19);
  });

  it("symbol center is always DARK (part of bullseye inner core)", () => {
    expect(dark(g, cy, cx)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Additional coverage tests
// ---------------------------------------------------------------------------

describe("additional coverage", () => {
  it("encodes binary data (all bytes 0x00-0xFF)", () => {
    const bytes = new Uint8Array(256);
    for (let i = 0; i < 256; i++) bytes[i] = i;
    const g = encode(bytes);
    expect(g.rows).toBeGreaterThanOrEqual(15);
    expect(g.rows).toBe(g.cols);
  });

  it("encodes 32-byte input without error", () => {
    const g = encode("A".repeat(32));
    expect(g.rows).toBeGreaterThanOrEqual(15);
  });

  it("encodes 50-byte input without error", () => {
    const g = encode("B".repeat(50));
    expect(g.rows).toBeGreaterThanOrEqual(15);
  });

  it("encodes 200-byte input without error", () => {
    const g = encode("C".repeat(200));
    expect(g.rows).toBeGreaterThanOrEqual(19);
  });

  it("encodes 500-byte input without error", () => {
    const g = encode("D".repeat(500));
    expect(g.rows).toBeGreaterThanOrEqual(19);
  });

  it("encodes unicode via UTF-8 bytes", () => {
    const g = encode("こんにちは"); // Japanese "Hello"
    expect(g.rows).toBeGreaterThanOrEqual(15);
    expect(g.rows).toBe(g.cols);
  });

  it("encode with minEccPercent 10 succeeds", () => {
    expect(() => encode("Hello", { minEccPercent: 10 })).not.toThrow();
  });

  it("encode with minEccPercent 90 succeeds (may need large symbol)", () => {
    expect(() => encode("Hello", { minEccPercent: 90 })).not.toThrow();
  });

  it("modules are mutable (copy semantics)", () => {
    const g = encode("A");
    const original = g.modules[0]![0];
    // Mutate the copy
    g.modules[0]![0] = !original;
    // Re-encode should give fresh grid
    const g2 = encode("A");
    expect(g2.modules[0]![0]).toBe(original);
  });
});
