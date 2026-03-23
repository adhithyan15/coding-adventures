/**
 * Tests for chown -- change file owner and group.
 *
 * We test the exported business logic functions: parseOwnerSpec,
 * chownFile, and chownRecursive.
 *
 * Note: Most chown operations require root privileges, so we test
 * the parsing and logic layer. The actual ownership change tests
 * expect permission errors when not running as root.
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import {
  parseOwnerSpec,
  chownFile,
  chownRecursive,
  ChownOptions,
  OwnerSpec,
} from "../src/chown.js";

// ---------------------------------------------------------------------------
// Helpers: temp directory management and default options.
// ---------------------------------------------------------------------------

let tmpDir: string;

function defaultOpts(overrides: Partial<ChownOptions> = {}): ChownOptions {
  return {
    recursive: false,
    verbose: false,
    changes: false,
    silent: false,
    noDereference: false,
    ...overrides,
  };
}

beforeEach(() => {
  tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "chown-test-"));
});

afterEach(() => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

// ---------------------------------------------------------------------------
// parseOwnerSpec: parsing OWNER[:GROUP] strings.
// ---------------------------------------------------------------------------

describe("parseOwnerSpec", () => {
  it("should parse owner only", () => {
    const result = parseOwnerSpec("1000");
    expect(result.uid).toBe(1000);
    expect(result.gid).toBeNull();
  });

  it("should parse owner:group", () => {
    const result = parseOwnerSpec("1000:100");
    expect(result.uid).toBe(1000);
    expect(result.gid).toBe(100);
  });

  it("should parse :group only", () => {
    const result = parseOwnerSpec(":100");
    expect(result.uid).toBeNull();
    expect(result.gid).toBe(100);
  });

  it("should parse owner: (owner only, with colon)", () => {
    const result = parseOwnerSpec("1000:");
    expect(result.uid).toBe(1000);
    expect(result.gid).toBeNull();
  });

  it("should parse legacy dot separator", () => {
    const result = parseOwnerSpec("1000.100");
    expect(result.uid).toBe(1000);
    expect(result.gid).toBe(100);
  });

  it("should parse zero as a valid ID", () => {
    const result = parseOwnerSpec("0:0");
    expect(result.uid).toBe(0);
    expect(result.gid).toBe(0);
  });

  it("should throw for invalid owner", () => {
    expect(() => parseOwnerSpec("abc")).toThrow("invalid user or group");
  });

  it("should throw for negative IDs", () => {
    expect(() => parseOwnerSpec("-1")).toThrow("invalid user or group");
  });

  it("should throw for invalid group after colon", () => {
    expect(() => parseOwnerSpec("1000:abc")).toThrow("invalid user or group");
  });
});

// ---------------------------------------------------------------------------
// chownFile: applying ownership changes.
// ---------------------------------------------------------------------------

describe("chownFile", () => {
  it("should stat the file and return current ownership", () => {
    const filePath = path.join(tmpDir, "test.txt");
    fs.writeFileSync(filePath, "hello");

    // Use current user's uid/gid so chown is a no-op.
    const stat = fs.statSync(filePath);
    const spec: OwnerSpec = { uid: stat.uid, gid: stat.gid };

    const result = chownFile(filePath, spec, defaultOpts());

    expect(result.filePath).toBe(filePath);
    expect(result.oldUid).toBe(stat.uid);
    expect(result.oldGid).toBe(stat.gid);
    expect(result.changed).toBe(false);
    expect(result.error).toBe(false);
  });

  it("should report 'retained' message when ownership doesn't change", () => {
    const filePath = path.join(tmpDir, "test.txt");
    fs.writeFileSync(filePath, "hello");

    const stat = fs.statSync(filePath);
    const spec: OwnerSpec = { uid: stat.uid, gid: stat.gid };

    const result = chownFile(filePath, spec, defaultOpts());

    expect(result.message).toContain("retained");
  });

  it("should keep current uid when spec.uid is null", () => {
    const filePath = path.join(tmpDir, "test.txt");
    fs.writeFileSync(filePath, "hello");

    const stat = fs.statSync(filePath);
    const spec: OwnerSpec = { uid: null, gid: stat.gid };

    const result = chownFile(filePath, spec, defaultOpts());

    expect(result.newUid).toBe(stat.uid);
    expect(result.changed).toBe(false);
  });

  it("should keep current gid when spec.gid is null", () => {
    const filePath = path.join(tmpDir, "test.txt");
    fs.writeFileSync(filePath, "hello");

    const stat = fs.statSync(filePath);
    const spec: OwnerSpec = { uid: stat.uid, gid: null };

    const result = chownFile(filePath, spec, defaultOpts());

    expect(result.newGid).toBe(stat.gid);
    expect(result.changed).toBe(false);
  });

  it("should report error when changing to different owner (no root)", () => {
    const filePath = path.join(tmpDir, "test.txt");
    fs.writeFileSync(filePath, "hello");

    // Try to change to a different UID -- should fail without root.
    const spec: OwnerSpec = { uid: 9999, gid: null };

    const result = chownFile(filePath, spec, defaultOpts());

    // On non-root systems, this will fail with EPERM.
    expect(result.error).toBe(true);
    expect(result.message).toContain("changing ownership");
  });

  it("should handle noDereference option for regular files", () => {
    const filePath = path.join(tmpDir, "test.txt");
    fs.writeFileSync(filePath, "hello");

    const stat = fs.lstatSync(filePath);
    const spec: OwnerSpec = { uid: stat.uid, gid: stat.gid };

    const result = chownFile(filePath, spec, defaultOpts({ noDereference: true }));

    expect(result.error).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// chownRecursive: recursive ownership changes.
// ---------------------------------------------------------------------------

describe("chownRecursive", () => {
  it("should process directory and all contents", () => {
    // Create directory structure.
    const subDir = path.join(tmpDir, "subdir");
    fs.mkdirSync(subDir);
    fs.writeFileSync(path.join(tmpDir, "file1.txt"), "hello");
    fs.writeFileSync(path.join(subDir, "file2.txt"), "world");

    // Use current ownership (no-op change).
    const stat = fs.statSync(tmpDir);
    const spec: OwnerSpec = { uid: stat.uid, gid: stat.gid };

    const results = chownRecursive(tmpDir, spec, defaultOpts());

    // Should have results for tmpDir, file1, subdir, file2.
    expect(results.length).toBeGreaterThanOrEqual(3);
  });

  it("should return results even if some files fail", () => {
    const filePath = path.join(tmpDir, "test.txt");
    fs.writeFileSync(filePath, "hello");

    // Try to change to a different owner recursively.
    const spec: OwnerSpec = { uid: 9999, gid: null };

    const results = chownRecursive(tmpDir, spec, defaultOpts());

    // Should have at least one result.
    expect(results.length).toBeGreaterThanOrEqual(1);
  });

  it("should handle empty directories", () => {
    const spec: OwnerSpec = { uid: null, gid: null };
    const results = chownRecursive(tmpDir, spec, defaultOpts());

    // Should have result for the directory itself.
    expect(results.length).toBe(1);
  });
});
