/**
 * Tests for split -- split a file into pieces.
 *
 * We test the exported business logic functions: generateSuffix,
 * generateFilename, splitByLines, splitByBytes, splitByChunks,
 * and parseByteSize.
 *
 * All tests operate on in-memory data, so no filesystem access is
 * needed for most tests.
 */

import { describe, it, expect } from "vitest";
import {
  generateSuffix,
  generateFilename,
  splitByLines,
  splitByBytes,
  splitByChunks,
  parseByteSize,
  SuffixOptions,
} from "../src/split.js";

// ---------------------------------------------------------------------------
// Helper: default suffix options.
// ---------------------------------------------------------------------------

function defaultSuffixOpts(
  overrides: Partial<SuffixOptions> = {}
): SuffixOptions {
  return {
    suffixLength: 2,
    numeric: false,
    hex: false,
    additionalSuffix: "",
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// generateSuffix: alphabetic, numeric, and hex suffix generation.
// ---------------------------------------------------------------------------

describe("generateSuffix", () => {
  // -------------------------------------------------------------------------
  // Alphabetic suffixes (default).
  // -------------------------------------------------------------------------

  it("should generate alphabetic suffixes starting with 'aa'", () => {
    expect(generateSuffix(0, defaultSuffixOpts())).toBe("aa");
  });

  it("should generate sequential alphabetic suffixes", () => {
    expect(generateSuffix(1, defaultSuffixOpts())).toBe("ab");
    expect(generateSuffix(25, defaultSuffixOpts())).toBe("az");
    expect(generateSuffix(26, defaultSuffixOpts())).toBe("ba");
  });

  it("should generate 'zz' as the last 2-char alphabetic suffix", () => {
    // 26*26 - 1 = 675
    expect(generateSuffix(675, defaultSuffixOpts())).toBe("zz");
  });

  it("should throw when alphabetic suffixes are exhausted", () => {
    // 26^2 = 676 possible suffixes
    expect(() => generateSuffix(676, defaultSuffixOpts())).toThrow(
      /suffixes exhausted/
    );
  });

  it("should handle suffix length 3", () => {
    const opts = defaultSuffixOpts({ suffixLength: 3 });
    expect(generateSuffix(0, opts)).toBe("aaa");
    expect(generateSuffix(1, opts)).toBe("aab");
  });

  it("should handle suffix length 1", () => {
    const opts = defaultSuffixOpts({ suffixLength: 1 });
    expect(generateSuffix(0, opts)).toBe("a");
    expect(generateSuffix(25, opts)).toBe("z");
    expect(() => generateSuffix(26, opts)).toThrow(/suffixes exhausted/);
  });

  // -------------------------------------------------------------------------
  // Numeric suffixes (-d).
  // -------------------------------------------------------------------------

  it("should generate numeric suffixes starting with '00'", () => {
    expect(generateSuffix(0, defaultSuffixOpts({ numeric: true }))).toBe("00");
  });

  it("should generate sequential numeric suffixes", () => {
    expect(generateSuffix(1, defaultSuffixOpts({ numeric: true }))).toBe("01");
    expect(generateSuffix(9, defaultSuffixOpts({ numeric: true }))).toBe("09");
    expect(generateSuffix(10, defaultSuffixOpts({ numeric: true }))).toBe("10");
    expect(generateSuffix(99, defaultSuffixOpts({ numeric: true }))).toBe("99");
  });

  it("should throw when numeric suffixes are exhausted", () => {
    expect(() =>
      generateSuffix(100, defaultSuffixOpts({ numeric: true }))
    ).toThrow(/suffixes exhausted/);
  });

  it("should handle numeric suffix length 3", () => {
    const opts = defaultSuffixOpts({ numeric: true, suffixLength: 3 });
    expect(generateSuffix(0, opts)).toBe("000");
    expect(generateSuffix(999, opts)).toBe("999");
    expect(() => generateSuffix(1000, opts)).toThrow(/suffixes exhausted/);
  });

  // -------------------------------------------------------------------------
  // Hex suffixes (-x).
  // -------------------------------------------------------------------------

  it("should generate hex suffixes starting with '00'", () => {
    expect(generateSuffix(0, defaultSuffixOpts({ hex: true }))).toBe("00");
  });

  it("should generate sequential hex suffixes", () => {
    expect(generateSuffix(10, defaultSuffixOpts({ hex: true }))).toBe("0a");
    expect(generateSuffix(15, defaultSuffixOpts({ hex: true }))).toBe("0f");
    expect(generateSuffix(16, defaultSuffixOpts({ hex: true }))).toBe("10");
    expect(generateSuffix(255, defaultSuffixOpts({ hex: true }))).toBe("ff");
  });

  it("should throw when hex suffixes are exhausted", () => {
    // 16^2 = 256
    expect(() =>
      generateSuffix(256, defaultSuffixOpts({ hex: true }))
    ).toThrow(/suffixes exhausted/);
  });
});

// ---------------------------------------------------------------------------
// generateFilename: full output filename.
// ---------------------------------------------------------------------------

describe("generateFilename", () => {
  it("should combine prefix and suffix", () => {
    expect(generateFilename("x", 0, defaultSuffixOpts())).toBe("xaa");
    expect(generateFilename("x", 1, defaultSuffixOpts())).toBe("xab");
  });

  it("should use custom prefix", () => {
    expect(generateFilename("output_", 0, defaultSuffixOpts())).toBe(
      "output_aa"
    );
  });

  it("should append additional suffix", () => {
    const opts = defaultSuffixOpts({ additionalSuffix: ".txt" });
    expect(generateFilename("x", 0, opts)).toBe("xaa.txt");
    expect(generateFilename("x", 1, opts)).toBe("xab.txt");
  });

  it("should work with numeric suffixes and additional suffix", () => {
    const opts = defaultSuffixOpts({
      numeric: true,
      additionalSuffix: ".dat",
    });
    expect(generateFilename("part", 5, opts)).toBe("part05.dat");
  });
});

// ---------------------------------------------------------------------------
// parseByteSize: parsing size strings.
// ---------------------------------------------------------------------------

describe("parseByteSize", () => {
  it("should parse plain numbers", () => {
    expect(parseByteSize("1024")).toBe(1024);
    expect(parseByteSize("0")).toBe(0);
    expect(parseByteSize("100")).toBe(100);
  });

  it("should parse K suffix", () => {
    expect(parseByteSize("1K")).toBe(1024);
    expect(parseByteSize("2k")).toBe(2048);
    expect(parseByteSize("1KB")).toBe(1024);
  });

  it("should parse M suffix", () => {
    expect(parseByteSize("1M")).toBe(1048576);
    expect(parseByteSize("2m")).toBe(2097152);
    expect(parseByteSize("1MB")).toBe(1048576);
  });

  it("should parse G suffix", () => {
    expect(parseByteSize("1G")).toBe(1073741824);
    expect(parseByteSize("1GB")).toBe(1073741824);
  });

  it("should throw for invalid size strings", () => {
    expect(() => parseByteSize("abc")).toThrow(/invalid number of bytes/);
    expect(() => parseByteSize("")).toThrow(/invalid number of bytes/);
  });
});

// ---------------------------------------------------------------------------
// splitByLines: splitting text by line count.
// ---------------------------------------------------------------------------

describe("splitByLines", () => {
  it("should split content into chunks of N lines", () => {
    const content = "a\nb\nc\nd\ne\n";
    const result = splitByLines(content, 2, "x", defaultSuffixOpts());

    expect(result.length).toBe(3);
    expect(result[0]).toEqual(["xaa", "a\nb\n"]);
    expect(result[1]).toEqual(["xab", "c\nd\n"]);
    expect(result[2]).toEqual(["xac", "e\n"]);
  });

  it("should handle content that divides evenly", () => {
    const content = "a\nb\nc\nd\n";
    const result = splitByLines(content, 2, "x", defaultSuffixOpts());

    expect(result.length).toBe(2);
    expect(result[0]).toEqual(["xaa", "a\nb\n"]);
    expect(result[1]).toEqual(["xab", "c\nd\n"]);
  });

  it("should handle single-line chunks", () => {
    const content = "a\nb\nc\n";
    const result = splitByLines(content, 1, "x", defaultSuffixOpts());

    expect(result.length).toBe(3);
    expect(result[0]).toEqual(["xaa", "a\n"]);
    expect(result[1]).toEqual(["xab", "b\n"]);
    expect(result[2]).toEqual(["xac", "c\n"]);
  });

  it("should handle all lines in one chunk", () => {
    const content = "a\nb\nc\n";
    const result = splitByLines(content, 100, "x", defaultSuffixOpts());

    expect(result.length).toBe(1);
    expect(result[0]).toEqual(["xaa", "a\nb\nc\n"]);
  });

  it("should handle empty content", () => {
    const result = splitByLines("", 10, "x", defaultSuffixOpts());
    expect(result).toEqual([]);
  });

  it("should use custom prefix", () => {
    const content = "a\nb\n";
    const result = splitByLines(content, 1, "out_", defaultSuffixOpts());

    expect(result[0][0]).toBe("out_aa");
    expect(result[1][0]).toBe("out_ab");
  });

  it("should use numeric suffixes when specified", () => {
    const content = "a\nb\n";
    const result = splitByLines(
      content,
      1,
      "x",
      defaultSuffixOpts({ numeric: true })
    );

    expect(result[0][0]).toBe("x00");
    expect(result[1][0]).toBe("x01");
  });
});

// ---------------------------------------------------------------------------
// splitByBytes: splitting content by byte count.
// ---------------------------------------------------------------------------

describe("splitByBytes", () => {
  it("should split buffer into chunks of N bytes", () => {
    const content = Buffer.from("abcdefghij");
    const result = splitByBytes(content, 3, "x", defaultSuffixOpts());

    expect(result.length).toBe(4);
    expect(result[0][0]).toBe("xaa");
    expect(result[0][1].toString()).toBe("abc");
    expect(result[1][0]).toBe("xab");
    expect(result[1][1].toString()).toBe("def");
    expect(result[2][0]).toBe("xac");
    expect(result[2][1].toString()).toBe("ghi");
    expect(result[3][0]).toBe("xad");
    expect(result[3][1].toString()).toBe("j");
  });

  it("should handle exact division", () => {
    const content = Buffer.from("abcdef");
    const result = splitByBytes(content, 3, "x", defaultSuffixOpts());

    expect(result.length).toBe(2);
    expect(result[0][1].toString()).toBe("abc");
    expect(result[1][1].toString()).toBe("def");
  });

  it("should handle empty content", () => {
    const result = splitByBytes(Buffer.from(""), 10, "x", defaultSuffixOpts());
    expect(result).toEqual([]);
  });

  it("should handle single chunk", () => {
    const content = Buffer.from("hello");
    const result = splitByBytes(content, 100, "x", defaultSuffixOpts());

    expect(result.length).toBe(1);
    expect(result[0][1].toString()).toBe("hello");
  });

  it("should handle byte-at-a-time splitting", () => {
    const content = Buffer.from("abc");
    const result = splitByBytes(content, 1, "x", defaultSuffixOpts());

    expect(result.length).toBe(3);
    expect(result[0][1].toString()).toBe("a");
    expect(result[1][1].toString()).toBe("b");
    expect(result[2][1].toString()).toBe("c");
  });
});

// ---------------------------------------------------------------------------
// splitByChunks: splitting into N equal chunks.
// ---------------------------------------------------------------------------

describe("splitByChunks", () => {
  it("should split into N equal chunks", () => {
    const content = Buffer.from("abcdef");
    const result = splitByChunks(content, 3, "x", defaultSuffixOpts());

    expect(result.length).toBe(3);
    expect(result[0][1].toString()).toBe("ab");
    expect(result[1][1].toString()).toBe("cd");
    expect(result[2][1].toString()).toBe("ef");
  });

  it("should handle uneven division (last chunk smaller)", () => {
    const content = Buffer.from("abcde");
    const result = splitByChunks(content, 3, "x", defaultSuffixOpts());

    expect(result.length).toBe(3);
    expect(result[0][1].toString()).toBe("ab");
    expect(result[1][1].toString()).toBe("cd");
    expect(result[2][1].toString()).toBe("e");
  });

  it("should handle more chunks than bytes", () => {
    const content = Buffer.from("ab");
    const result = splitByChunks(content, 5, "x", defaultSuffixOpts());

    // Should produce only 2 chunks (one byte each).
    expect(result.length).toBe(2);
    expect(result[0][1].toString()).toBe("a");
    expect(result[1][1].toString()).toBe("b");
  });

  it("should handle single chunk", () => {
    const content = Buffer.from("hello");
    const result = splitByChunks(content, 1, "x", defaultSuffixOpts());

    expect(result.length).toBe(1);
    expect(result[0][1].toString()).toBe("hello");
  });

  it("should handle empty content", () => {
    const result = splitByChunks(Buffer.from(""), 3, "x", defaultSuffixOpts());
    expect(result).toEqual([]);
  });

  it("should handle zero chunks", () => {
    const result = splitByChunks(
      Buffer.from("hello"),
      0,
      "x",
      defaultSuffixOpts()
    );
    expect(result).toEqual([]);
  });

  it("should use correct filenames", () => {
    const content = Buffer.from("abcd");
    const result = splitByChunks(content, 2, "part_", defaultSuffixOpts());

    expect(result[0][0]).toBe("part_aa");
    expect(result[1][0]).toBe("part_ab");
  });
});
