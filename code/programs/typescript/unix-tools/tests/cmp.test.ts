/**
 * Tests for cmp -- compare two files byte by byte.
 *
 * We test the exported business logic functions: compareBuffers,
 * formatByte, formatDefaultDiff, formatListDiff, and formatEof.
 */

import { describe, it, expect } from "vitest";
import {
  compareBuffers,
  formatByte,
  formatDefaultDiff,
  formatListDiff,
  formatEof,
  CmpOptions,
  ByteDifference,
} from "../src/cmp.js";

// ---------------------------------------------------------------------------
// Helper: default cmp options (no flags set).
// ---------------------------------------------------------------------------

function defaultOpts(overrides: Partial<CmpOptions> = {}): CmpOptions {
  return {
    list: false,
    silent: false,
    printBytes: false,
    skipBytes: 0,
    maxBytes: 0,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// compareBuffers: identical files.
// ---------------------------------------------------------------------------

describe("compareBuffers", () => {
  it("should report identical buffers", () => {
    const buf = Buffer.from("hello world");
    const result = compareBuffers(buf, buf, defaultOpts());
    expect(result.identical).toBe(true);
    expect(result.firstDiff).toBeUndefined();
    expect(result.allDiffs).toEqual([]);
    expect(result.eofReached).toBe(false);
  });

  it("should report identical empty buffers", () => {
    const result = compareBuffers(Buffer.alloc(0), Buffer.alloc(0), defaultOpts());
    expect(result.identical).toBe(true);
  });

  it("should find the first difference", () => {
    const a = Buffer.from("hello");
    const b = Buffer.from("heLlo");
    const result = compareBuffers(a, b, defaultOpts());

    expect(result.identical).toBe(false);
    expect(result.firstDiff).toBeDefined();
    expect(result.firstDiff!.byteNumber).toBe(3);
    expect(result.firstDiff!.lineNumber).toBe(1);
    expect(result.firstDiff!.byte1).toBe("l".charCodeAt(0));
    expect(result.firstDiff!.byte2).toBe("L".charCodeAt(0));
  });

  it("should track line numbers across newlines", () => {
    const a = Buffer.from("line1\nline2\nfoo");
    const b = Buffer.from("line1\nline2\nbar");
    const result = compareBuffers(a, b, defaultOpts());

    expect(result.identical).toBe(false);
    expect(result.firstDiff!.lineNumber).toBe(3);
    // "foo" vs "bar" differs at the 'f'/'b' byte.
    expect(result.firstDiff!.byteNumber).toBe(13);
  });

  it("should detect EOF on shorter file (file 1 shorter)", () => {
    const a = Buffer.from("hello");
    const b = Buffer.from("hello world");
    const result = compareBuffers(a, b, defaultOpts());

    expect(result.identical).toBe(false);
    expect(result.eofReached).toBe(true);
    expect(result.eofFile).toBe(1);
  });

  it("should detect EOF on shorter file (file 2 shorter)", () => {
    const a = Buffer.from("hello world");
    const b = Buffer.from("hello");
    const result = compareBuffers(a, b, defaultOpts());

    expect(result.identical).toBe(false);
    expect(result.eofReached).toBe(true);
    expect(result.eofFile).toBe(2);
  });

  // --- skipBytes (-i) --------------------------------------------------

  it("should skip initial bytes when skipBytes is set", () => {
    const a = Buffer.from("XXXhello");
    const b = Buffer.from("YYYhello");
    const result = compareBuffers(a, b, defaultOpts({ skipBytes: 3 }));

    expect(result.identical).toBe(true);
  });

  it("should find differences after skip offset", () => {
    const a = Buffer.from("XXXhello");
    const b = Buffer.from("YYYheLlo");
    const result = compareBuffers(a, b, defaultOpts({ skipBytes: 3 }));

    expect(result.identical).toBe(false);
    expect(result.firstDiff!.byteNumber).toBe(6); // position 6 (1-based)
  });

  // --- maxBytes (-n) ---------------------------------------------------

  it("should only compare up to maxBytes", () => {
    const a = Buffer.from("hello world");
    const b = Buffer.from("hello WORLD");
    // First 5 bytes are the same, difference at byte 6.
    const result = compareBuffers(a, b, defaultOpts({ maxBytes: 5 }));

    expect(result.identical).toBe(true);
  });

  it("should find differences within maxBytes range", () => {
    const a = Buffer.from("heLlo world");
    const b = Buffer.from("hello WORLD");
    const result = compareBuffers(a, b, defaultOpts({ maxBytes: 5 }));

    expect(result.identical).toBe(false);
    expect(result.firstDiff!.byteNumber).toBe(3);
  });

  // --- list mode (-l) --------------------------------------------------

  it("should collect all differences in list mode", () => {
    const a = Buffer.from("abcde");
    const b = Buffer.from("aXcYe");
    const result = compareBuffers(a, b, defaultOpts({ list: true }));

    expect(result.identical).toBe(false);
    expect(result.allDiffs).toHaveLength(2);
    expect(result.allDiffs[0].byteNumber).toBe(2);
    expect(result.allDiffs[0].byte1).toBe("b".charCodeAt(0));
    expect(result.allDiffs[0].byte2).toBe("X".charCodeAt(0));
    expect(result.allDiffs[1].byteNumber).toBe(4);
  });

  it("should collect no differences for identical buffers in list mode", () => {
    const buf = Buffer.from("hello");
    const result = compareBuffers(buf, buf, defaultOpts({ list: true }));

    expect(result.identical).toBe(true);
    expect(result.allDiffs).toEqual([]);
  });

  // --- silent mode (-s) ------------------------------------------------

  it("should stop at first difference in silent mode", () => {
    const a = Buffer.from("abcde");
    const b = Buffer.from("aXcYe");
    const result = compareBuffers(a, b, defaultOpts({ silent: true }));

    expect(result.identical).toBe(false);
    expect(result.firstDiff!.byteNumber).toBe(2);
    // In silent mode we don't collect all diffs.
    expect(result.allDiffs).toEqual([]);
  });

  // --- combined: skip + maxBytes ---------------------------------------

  it("should handle skipBytes and maxBytes together", () => {
    const a = Buffer.from("XXXhelloWORLD");
    const b = Buffer.from("YYYhelloworld");
    // Skip 3, compare at most 5 bytes => compare "hello" vs "hello".
    const result = compareBuffers(a, b, defaultOpts({ skipBytes: 3, maxBytes: 5 }));

    expect(result.identical).toBe(true);
  });

  // --- EOF with no content differences ---------------------------------

  it("should report EOF even when shared content is identical", () => {
    const a = Buffer.from("abc");
    const b = Buffer.from("abcdef");
    const result = compareBuffers(a, b, defaultOpts());

    expect(result.identical).toBe(false);
    expect(result.eofReached).toBe(true);
    expect(result.eofFile).toBe(1);
    expect(result.firstDiff).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// formatByte: byte display formatting.
// ---------------------------------------------------------------------------

describe("formatByte", () => {
  it("should display printable ASCII characters", () => {
    expect(formatByte(65)).toBe("A");
    expect(formatByte(97)).toBe("a");
    expect(formatByte(48)).toBe("0");
    expect(formatByte(32)).toBe(" ");
    expect(formatByte(126)).toBe("~");
  });

  it("should display special characters by name", () => {
    expect(formatByte(0)).toBe("\\0");
    expect(formatByte(7)).toBe("\\a");
    expect(formatByte(8)).toBe("\\b");
    expect(formatByte(9)).toBe("\\t");
    expect(formatByte(10)).toBe("\\n");
    expect(formatByte(11)).toBe("\\v");
    expect(formatByte(12)).toBe("\\f");
    expect(formatByte(13)).toBe("\\r");
  });

  it("should display non-printable bytes in octal", () => {
    expect(formatByte(128)).toBe("\\200");
    expect(formatByte(255)).toBe("\\377");
    expect(formatByte(1)).toBe("\\001");
  });
});

// ---------------------------------------------------------------------------
// formatDefaultDiff: default-mode output.
// ---------------------------------------------------------------------------

describe("formatDefaultDiff", () => {
  const diff: ByteDifference = {
    byteNumber: 42,
    lineNumber: 3,
    byte1: 65,  // 'A'
    byte2: 97,  // 'a'
  };

  it("should format without byte characters", () => {
    const result = formatDefaultDiff("file1", "file2", diff, false);
    expect(result).toBe("file1 file2 differ: byte 42, line 3");
  });

  it("should format with byte characters", () => {
    const result = formatDefaultDiff("file1", "file2", diff, true);
    expect(result).toContain("byte 42, line 3");
    expect(result).toContain("A");
    expect(result).toContain("a");
  });
});

// ---------------------------------------------------------------------------
// formatListDiff: list-mode output.
// ---------------------------------------------------------------------------

describe("formatListDiff", () => {
  const diff: ByteDifference = {
    byteNumber: 5,
    lineNumber: 1,
    byte1: 65,
    byte2: 97,
  };

  it("should format without byte characters", () => {
    const result = formatListDiff(diff, false);
    expect(result).toContain("5");
    expect(result).toContain("101"); // octal for 65
    expect(result).toContain("141"); // octal for 97
  });

  it("should format with byte characters", () => {
    const result = formatListDiff(diff, true);
    expect(result).toContain("A");
    expect(result).toContain("a");
  });
});

// ---------------------------------------------------------------------------
// formatEof: EOF message formatting.
// ---------------------------------------------------------------------------

describe("formatEof", () => {
  it("should format an EOF message", () => {
    const result = formatEof("file1.txt", 100, 5);
    expect(result).toBe("cmp: EOF on file1.txt after byte 100, line 5");
  });

  it("should handle single byte and line", () => {
    const result = formatEof("a.bin", 1, 1);
    expect(result).toBe("cmp: EOF on a.bin after byte 1, line 1");
  });
});
