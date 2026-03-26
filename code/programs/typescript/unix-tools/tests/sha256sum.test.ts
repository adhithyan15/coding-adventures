/**
 * Tests for sha256sum -- compute and check SHA256 message digest.
 *
 * We test the exported business logic functions: computeSha256,
 * formatChecksum, parseChecksumLine, and verifyChecksums.
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import {
  computeSha256,
  formatChecksum,
  parseChecksumLine,
  verifyChecksums,
} from "../src/sha256sum.js";

// ---------------------------------------------------------------------------
// computeSha256.
// ---------------------------------------------------------------------------

describe("computeSha256", () => {
  it("should compute SHA-256 of empty buffer", () => {
    // The SHA-256 of an empty message is well-known.
    const result = computeSha256(Buffer.from(""));
    expect(result).toBe(
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
  });

  it("should compute SHA-256 of 'hello world'", () => {
    const result = computeSha256(Buffer.from("hello world"));
    expect(result).toBe(
      "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    );
  });

  it("should return a 64-character hex string", () => {
    const result = computeSha256(Buffer.from("test"));
    expect(result).toHaveLength(64);
    expect(result).toMatch(/^[0-9a-f]{64}$/);
  });

  it("should produce different hashes for different inputs", () => {
    const hash1 = computeSha256(Buffer.from("hello"));
    const hash2 = computeSha256(Buffer.from("world"));
    expect(hash1).not.toBe(hash2);
  });

  it("should produce the same hash for the same input", () => {
    const hash1 = computeSha256(Buffer.from("consistent"));
    const hash2 = computeSha256(Buffer.from("consistent"));
    expect(hash1).toBe(hash2);
  });

  it("should handle binary data", () => {
    const data = Buffer.from([0x00, 0x01, 0x02, 0xff, 0xfe, 0xfd]);
    const result = computeSha256(data);
    expect(result).toHaveLength(64);
    expect(result).toMatch(/^[0-9a-f]{64}$/);
  });

  it("should compute correct hash for a single newline", () => {
    const result = computeSha256(Buffer.from("\n"));
    expect(result).toBe(
      "01ba4719c80b6fe911b091a7c05124b64eeece964e09c058ef8f9805daca546b"
    );
  });
});

// ---------------------------------------------------------------------------
// formatChecksum.
// ---------------------------------------------------------------------------

describe("formatChecksum", () => {
  it("should format text mode with two spaces", () => {
    const hash = "a".repeat(64);
    const result = formatChecksum(hash, "file.txt", false);
    expect(result).toBe(hash + "  file.txt");
  });

  it("should format binary mode with space+asterisk", () => {
    const hash = "b".repeat(64);
    const result = formatChecksum(hash, "file.txt", true);
    expect(result).toBe(hash + " *file.txt");
  });

  it("should handle filenames with spaces", () => {
    const hash = "c".repeat(64);
    const result = formatChecksum(hash, "my file.txt", false);
    expect(result).toBe(hash + "  my file.txt");
  });
});

// ---------------------------------------------------------------------------
// parseChecksumLine.
// ---------------------------------------------------------------------------

describe("parseChecksumLine", () => {
  it("should parse a text-mode line", () => {
    const hash =
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    const result = parseChecksumLine(`${hash}  empty.txt`);
    expect(result).toEqual({
      hash,
      filename: "empty.txt",
      binary: false,
    });
  });

  it("should parse a binary-mode line", () => {
    const hash =
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    const result = parseChecksumLine(`${hash} *binary.bin`);
    expect(result).toEqual({
      hash,
      filename: "binary.bin",
      binary: true,
    });
  });

  it("should return null for malformed lines", () => {
    expect(parseChecksumLine("not a checksum line")).toBeNull();
    expect(parseChecksumLine("")).toBeNull();
    // MD5-length hash (32 chars) should not match SHA-256 parser.
    expect(
      parseChecksumLine("d41d8cd98f00b204e9800998ecf8427e  file.txt")
    ).toBeNull();
  });

  it("should handle uppercase hex digits", () => {
    const hash =
      "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855";
    const result = parseChecksumLine(`${hash}  file.txt`);
    expect(result).not.toBeNull();
    expect(result!.hash).toBe(hash.toLowerCase());
  });

  it("should handle filenames with spaces", () => {
    const hash =
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    const result = parseChecksumLine(`${hash}  my file.txt`);
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
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "sha256-test-"));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it("should verify a correct checksum", () => {
    const filePath = path.join(tmpDir, "test.txt");
    fs.writeFileSync(filePath, "hello world");
    const hash = computeSha256(Buffer.from("hello world"));

    const checkContent = `${hash}  ${filePath}\n`;
    const result = verifyChecksums(checkContent, false, false, false, false);
    expect(result).toBe(true);
  });

  it("should detect an incorrect checksum", () => {
    const filePath = path.join(tmpDir, "test.txt");
    fs.writeFileSync(filePath, "hello world");

    const badHash = "0".repeat(64);
    const checkContent = `${badHash}  ${filePath}\n`;
    const result = verifyChecksums(checkContent, false, false, false, false);
    expect(result).toBe(false);
  });

  it("should handle missing files", () => {
    const hash =
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    const checkContent = `${hash}  /nonexistent/file.txt\n`;
    const result = verifyChecksums(checkContent, false, false, false, false);
    expect(result).toBe(false);
  });

  it("should skip empty lines", () => {
    const filePath = path.join(tmpDir, "test.txt");
    fs.writeFileSync(filePath, "");
    const hash = computeSha256(Buffer.from(""));

    const checkContent = `\n${hash}  ${filePath}\n\n`;
    const result = verifyChecksums(checkContent, false, false, false, false);
    expect(result).toBe(true);
  });
});
