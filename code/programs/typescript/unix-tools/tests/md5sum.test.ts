/**
 * Tests for md5sum -- compute and check MD5 message digest.
 *
 * We test the exported business logic functions: computeMd5,
 * formatChecksum, parseChecksumLine, and verifyChecksums.
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import {
  computeMd5,
  formatChecksum,
  parseChecksumLine,
  verifyChecksums,
} from "../src/md5sum.js";

// ---------------------------------------------------------------------------
// computeMd5.
// ---------------------------------------------------------------------------

describe("computeMd5", () => {
  it("should compute MD5 of empty buffer", () => {
    // The MD5 of an empty message is well-known.
    const result = computeMd5(Buffer.from(""));
    expect(result).toBe("d41d8cd98f00b204e9800998ecf8427e");
  });

  it("should compute MD5 of 'hello world'", () => {
    const result = computeMd5(Buffer.from("hello world"));
    expect(result).toBe("5eb63bbbe01eeed093cb22bb8f5acdc3");
  });

  it("should return a 32-character hex string", () => {
    const result = computeMd5(Buffer.from("test"));
    expect(result).toHaveLength(32);
    expect(result).toMatch(/^[0-9a-f]{32}$/);
  });

  it("should produce different hashes for different inputs", () => {
    const hash1 = computeMd5(Buffer.from("hello"));
    const hash2 = computeMd5(Buffer.from("world"));
    expect(hash1).not.toBe(hash2);
  });

  it("should produce the same hash for the same input", () => {
    const hash1 = computeMd5(Buffer.from("consistent"));
    const hash2 = computeMd5(Buffer.from("consistent"));
    expect(hash1).toBe(hash2);
  });

  it("should handle binary data", () => {
    const data = Buffer.from([0x00, 0x01, 0x02, 0xff, 0xfe, 0xfd]);
    const result = computeMd5(data);
    expect(result).toHaveLength(32);
    expect(result).toMatch(/^[0-9a-f]{32}$/);
  });

  it("should compute correct hash for a newline", () => {
    const result = computeMd5(Buffer.from("\n"));
    expect(result).toBe("68b329da9893e34099c7d8ad5cb9c940");
  });
});

// ---------------------------------------------------------------------------
// formatChecksum.
// ---------------------------------------------------------------------------

describe("formatChecksum", () => {
  it("should format text mode with two spaces", () => {
    const result = formatChecksum("abc123", "file.txt", false);
    expect(result).toBe("abc123  file.txt");
  });

  it("should format binary mode with space+asterisk", () => {
    const result = formatChecksum("abc123", "file.txt", true);
    expect(result).toBe("abc123 *file.txt");
  });

  it("should handle filenames with spaces", () => {
    const result = formatChecksum("abc123", "my file.txt", false);
    expect(result).toBe("abc123  my file.txt");
  });

  it("should handle stdin filename", () => {
    const result = formatChecksum("abc123", "-", false);
    expect(result).toBe("abc123  -");
  });
});

// ---------------------------------------------------------------------------
// parseChecksumLine.
// ---------------------------------------------------------------------------

describe("parseChecksumLine", () => {
  it("should parse a text-mode line", () => {
    const result = parseChecksumLine(
      "d41d8cd98f00b204e9800998ecf8427e  empty.txt"
    );
    expect(result).toEqual({
      hash: "d41d8cd98f00b204e9800998ecf8427e",
      filename: "empty.txt",
      binary: false,
    });
  });

  it("should parse a binary-mode line", () => {
    const result = parseChecksumLine(
      "d41d8cd98f00b204e9800998ecf8427e *binary.bin"
    );
    expect(result).toEqual({
      hash: "d41d8cd98f00b204e9800998ecf8427e",
      filename: "binary.bin",
      binary: true,
    });
  });

  it("should return null for malformed lines", () => {
    expect(parseChecksumLine("not a checksum line")).toBeNull();
    expect(parseChecksumLine("")).toBeNull();
    expect(parseChecksumLine("abc  file.txt")).toBeNull();
  });

  it("should handle uppercase hex digits", () => {
    const result = parseChecksumLine(
      "D41D8CD98F00B204E9800998ECF8427E  file.txt"
    );
    expect(result).not.toBeNull();
    expect(result!.hash).toBe("d41d8cd98f00b204e9800998ecf8427e");
  });

  it("should handle filenames with spaces", () => {
    const result = parseChecksumLine(
      "d41d8cd98f00b204e9800998ecf8427e  my file.txt"
    );
    expect(result).not.toBeNull();
    expect(result!.filename).toBe("my file.txt");
  });
});

// ---------------------------------------------------------------------------
// verifyChecksums (integration test with temp files).
// ---------------------------------------------------------------------------

describe("verifyChecksums", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "md5-test-"));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it("should verify a correct checksum", () => {
    const filePath = path.join(tmpDir, "test.txt");
    fs.writeFileSync(filePath, "hello world");
    const hash = computeMd5(Buffer.from("hello world"));

    const checkContent = `${hash}  ${filePath}\n`;
    const result = verifyChecksums(checkContent, false, false, false, false);
    expect(result).toBe(true);
  });

  it("should detect an incorrect checksum", () => {
    const filePath = path.join(tmpDir, "test.txt");
    fs.writeFileSync(filePath, "hello world");

    const checkContent = `00000000000000000000000000000000  ${filePath}\n`;
    const result = verifyChecksums(checkContent, false, false, false, false);
    expect(result).toBe(false);
  });

  it("should handle missing files", () => {
    const checkContent =
      "d41d8cd98f00b204e9800998ecf8427e  /nonexistent/file.txt\n";
    const result = verifyChecksums(checkContent, false, false, false, false);
    expect(result).toBe(false);
  });

  it("should skip empty lines", () => {
    const filePath = path.join(tmpDir, "test.txt");
    fs.writeFileSync(filePath, "");
    const hash = computeMd5(Buffer.from(""));

    const checkContent = `\n${hash}  ${filePath}\n\n`;
    const result = verifyChecksums(checkContent, false, false, false, false);
    expect(result).toBe(true);
  });
});
