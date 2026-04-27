/**
 * utf16.test.ts -- UTF-16 offset conversion tests
 *
 * This is the most important correctness test in the entire package. If the
 * position conversion functions are wrong, every feature that depends on cursor
 * position will be wrong: hover, go-to-definition, references, completion,
 * rename, signature help.
 *
 * # Why UTF-16 Matters
 *
 * LSP's "character" offset is measured in UTF-16 code units. In JavaScript,
 * strings are already UTF-16 internally, so the conversion is simpler than
 * in Go or Rust. But we still need to test edge cases with emoji (surrogate
 * pairs) and multi-byte UTF-8 characters to ensure byte offset conversion
 * is correct.
 */

import { describe, it, expect } from "vitest";
import {
  convertPositionToStringIndex,
  convertUTF16OffsetToByteOffset,
} from "../src/document-manager.js";

describe("convertPositionToStringIndex", () => {
  it("ASCII simple", () => {
    // "hello world" -- "world" starts at character 6
    expect(convertPositionToStringIndex("hello world", { line: 0, character: 6 })).toBe(6);
  });

  it("start of file", () => {
    expect(convertPositionToStringIndex("abc", { line: 0, character: 0 })).toBe(0);
  });

  it("end of short string", () => {
    expect(convertPositionToStringIndex("abc", { line: 0, character: 3 })).toBe(3);
  });

  it("second line", () => {
    // "hello\nworld" -- line 1 starts at string index 6
    expect(convertPositionToStringIndex("hello\nworld", { line: 1, character: 0 })).toBe(6);
  });

  it("emoji: guitar takes 2 UTF-16 units", () => {
    // "A\u{1F3B8}B"
    // UTF-16 units: A (1 unit) + guitar (2 units) + B (1 unit) = 4 units
    // "B" is at UTF-16 character 3, string index 3 (JS strings ARE UTF-16)
    const text = "A\u{1F3B8}B";
    expect(text.length).toBe(4); // JS .length is UTF-16 code units
    expect(convertPositionToStringIndex(text, { line: 0, character: 3 })).toBe(3);
  });

  it("emoji at start", () => {
    // "\u{1F3B8}hello"
    // guitar = 2 UTF-16 units, "h" is at character 2
    const text = "\u{1F3B8}hello";
    expect(convertPositionToStringIndex(text, { line: 0, character: 2 })).toBe(2);
  });

  it("multiline with emoji", () => {
    // line 0: "A\u{1F3B8}B\n"  (A=1, guitar=2, B=1, \n=1 = 5 UTF-16 units)
    // line 1: "hello"
    // "hello" starts at string index 5 on line 1
    const text = "A\u{1F3B8}B\nhello";
    expect(convertPositionToStringIndex(text, { line: 1, character: 0 })).toBe(5);
  });

  it("beyond line end clamps to newline", () => {
    // If character is past the end of the line, we stop at the newline.
    expect(convertPositionToStringIndex("ab\ncd", { line: 0, character: 100 })).toBe(2);
  });

  it("line beyond file end clamps to end", () => {
    expect(convertPositionToStringIndex("abc", { line: 5, character: 0 })).toBe(3);
  });

  it("Chinese characters (BMP codepoints)", () => {
    // Each Chinese character is 1 UTF-16 code unit (BMP)
    const text = "\u4e2d\u6587"; // "zhong wen"
    expect(convertPositionToStringIndex(text, { line: 0, character: 1 })).toBe(1);
  });
});

describe("convertUTF16OffsetToByteOffset", () => {
  it("ASCII simple", () => {
    expect(convertUTF16OffsetToByteOffset("hello world", 0, 6)).toBe(6);
  });

  it("emoji: guitar is 4 UTF-8 bytes but 2 UTF-16 units", () => {
    // "A\u{1F3B8}B"
    // UTF-8: A(1) + guitar(4) + B(1) = 6 bytes
    // UTF-16: A(1) + guitar(2) + B(1) = 4 units
    // "B" is at UTF-16 char 3, byte offset 5
    const text = "A\u{1F3B8}B";
    expect(convertUTF16OffsetToByteOffset(text, 0, 3)).toBe(5);
  });

  it("emoji at start", () => {
    // "\u{1F3B8}hello"
    // guitar = 4 UTF-8 bytes, "h" starts at byte 4
    const text = "\u{1F3B8}hello";
    expect(convertUTF16OffsetToByteOffset(text, 0, 2)).toBe(4);
  });

  it("2-byte UTF-8 (BMP codepoint: e-acute)", () => {
    // "cafe-acute!" -- e-acute (U+00E9) is 2 UTF-8 bytes, 1 UTF-16 unit
    // c(1) + a(1) + f(1) + e-acute(2) = 5 bytes for "cafe-acute"
    // "!" is at UTF-16 char 4, byte offset 5
    const text = "caf\u00e9!";
    expect(convertUTF16OffsetToByteOffset(text, 0, 4)).toBe(5);
  });

  it("multiline with emoji", () => {
    // line 0: "A\u{1F3B8}B\n"  (1 + 4 + 1 + 1 = 7 UTF-8 bytes)
    // line 1: "hello"
    // "hello" starts at byte 7
    const text = "A\u{1F3B8}B\nhello";
    expect(convertUTF16OffsetToByteOffset(text, 1, 0)).toBe(7);
  });

  it("Chinese character (3-byte UTF-8, 1 UTF-16 unit)", () => {
    // "zhong wen" -- each character is 3 UTF-8 bytes, 1 UTF-16 unit
    // Second character starts at byte 3
    const text = "\u4e2d\u6587";
    expect(convertUTF16OffsetToByteOffset(text, 0, 1)).toBe(3);
  });

  it("beyond line end clamps", () => {
    expect(convertUTF16OffsetToByteOffset("ab\ncd", 0, 100)).toBe(2);
  });
});
