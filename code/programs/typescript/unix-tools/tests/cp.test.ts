/**
 * Tests for cp -- copy files and directories.
 *
 * We test the exported `copyFile` and `copyMultiple` functions using
 * temporary directories created with `fs.mkdtempSync`. Each test gets
 * a fresh temp directory that is cleaned up after the test.
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { copyFile, copyMultiple, CopyOptions } from "../src/cp.js";

// ---------------------------------------------------------------------------
// Helpers: temp directory management and default options.
// ---------------------------------------------------------------------------

let tmpDir: string;

function defaultOpts(overrides: Partial<CopyOptions> = {}): CopyOptions {
  return {
    force: false,
    noClobber: false,
    recursive: false,
    verbose: false,
    ...overrides,
  };
}

beforeEach(() => {
  tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "cp-test-"));
});

afterEach(() => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

// ---------------------------------------------------------------------------
// copyFile: basic file copy.
// ---------------------------------------------------------------------------

describe("copyFile", () => {
  it("should copy a regular file to a new destination", () => {
    const src = path.join(tmpDir, "source.txt");
    const dst = path.join(tmpDir, "dest.txt");
    fs.writeFileSync(src, "hello world");

    copyFile(src, dst, defaultOpts());

    expect(fs.existsSync(dst)).toBe(true);
    expect(fs.readFileSync(dst, "utf-8")).toBe("hello world");
  });

  it("should preserve file contents exactly", () => {
    const src = path.join(tmpDir, "binary.bin");
    const dst = path.join(tmpDir, "copy.bin");
    const data = Buffer.from([0x00, 0x01, 0xff, 0xfe, 0x80]);
    fs.writeFileSync(src, data);

    copyFile(src, dst, defaultOpts());

    expect(Buffer.compare(fs.readFileSync(dst), data)).toBe(0);
  });

  it("should copy an empty file", () => {
    const src = path.join(tmpDir, "empty.txt");
    const dst = path.join(tmpDir, "empty-copy.txt");
    fs.writeFileSync(src, "");

    copyFile(src, dst, defaultOpts());

    expect(fs.existsSync(dst)).toBe(true);
    expect(fs.readFileSync(dst, "utf-8")).toBe("");
  });

  it("should throw if source does not exist", () => {
    const src = path.join(tmpDir, "nonexistent.txt");
    const dst = path.join(tmpDir, "dest.txt");

    expect(() => copyFile(src, dst, defaultOpts())).toThrow(
      /No such file or directory/
    );
  });

  // -------------------------------------------------------------------------
  // Copy into a directory.
  // -------------------------------------------------------------------------

  it("should copy into a directory when destination is an existing directory", () => {
    const src = path.join(tmpDir, "file.txt");
    const dstDir = path.join(tmpDir, "subdir");
    fs.writeFileSync(src, "content");
    fs.mkdirSync(dstDir);

    copyFile(src, dstDir, defaultOpts());

    const expectedPath = path.join(dstDir, "file.txt");
    expect(fs.existsSync(expectedPath)).toBe(true);
    expect(fs.readFileSync(expectedPath, "utf-8")).toBe("content");
  });

  // -------------------------------------------------------------------------
  // Overwrite behavior (default).
  // -------------------------------------------------------------------------

  it("should overwrite an existing file by default", () => {
    const src = path.join(tmpDir, "src.txt");
    const dst = path.join(tmpDir, "dst.txt");
    fs.writeFileSync(src, "new content");
    fs.writeFileSync(dst, "old content");

    copyFile(src, dst, defaultOpts());

    expect(fs.readFileSync(dst, "utf-8")).toBe("new content");
  });

  // -------------------------------------------------------------------------
  // No-clobber mode (-n).
  // -------------------------------------------------------------------------

  it("should not overwrite when noClobber is true", () => {
    const src = path.join(tmpDir, "src.txt");
    const dst = path.join(tmpDir, "dst.txt");
    fs.writeFileSync(src, "new content");
    fs.writeFileSync(dst, "old content");

    copyFile(src, dst, defaultOpts({ noClobber: true }));

    expect(fs.readFileSync(dst, "utf-8")).toBe("old content");
  });

  it("should copy when noClobber is true and destination does not exist", () => {
    const src = path.join(tmpDir, "src.txt");
    const dst = path.join(tmpDir, "dst.txt");
    fs.writeFileSync(src, "content");

    copyFile(src, dst, defaultOpts({ noClobber: true }));

    expect(fs.readFileSync(dst, "utf-8")).toBe("content");
  });

  // -------------------------------------------------------------------------
  // Force mode (-f).
  // -------------------------------------------------------------------------

  it("should remove and re-copy when force is true", () => {
    const src = path.join(tmpDir, "src.txt");
    const dst = path.join(tmpDir, "dst.txt");
    fs.writeFileSync(src, "new content");
    fs.writeFileSync(dst, "old content");

    copyFile(src, dst, defaultOpts({ force: true }));

    expect(fs.readFileSync(dst, "utf-8")).toBe("new content");
  });

  // -------------------------------------------------------------------------
  // Verbose mode (-v).
  // -------------------------------------------------------------------------

  it("should return verbose messages when verbose is true", () => {
    const src = path.join(tmpDir, "src.txt");
    const dst = path.join(tmpDir, "dst.txt");
    fs.writeFileSync(src, "content");

    const messages = copyFile(src, dst, defaultOpts({ verbose: true }));

    expect(messages.length).toBe(1);
    expect(messages[0]).toContain("src.txt");
    expect(messages[0]).toContain("dst.txt");
    expect(messages[0]).toContain("->");
  });

  it("should return empty messages when verbose is false", () => {
    const src = path.join(tmpDir, "src.txt");
    const dst = path.join(tmpDir, "dst.txt");
    fs.writeFileSync(src, "content");

    const messages = copyFile(src, dst, defaultOpts({ verbose: false }));

    expect(messages).toEqual([]);
  });

  // -------------------------------------------------------------------------
  // Recursive mode (-R).
  // -------------------------------------------------------------------------

  it("should refuse to copy a directory without recursive flag", () => {
    const srcDir = path.join(tmpDir, "srcdir");
    const dst = path.join(tmpDir, "dstdir");
    fs.mkdirSync(srcDir);
    fs.writeFileSync(path.join(srcDir, "file.txt"), "data");

    expect(() => copyFile(srcDir, dst, defaultOpts())).toThrow(
      /omitting directory/
    );
  });

  it("should copy a directory recursively with recursive flag", () => {
    const srcDir = path.join(tmpDir, "srcdir");
    const dstDir = path.join(tmpDir, "dstdir");
    fs.mkdirSync(srcDir);
    fs.writeFileSync(path.join(srcDir, "a.txt"), "alpha");

    const nestedDir = path.join(srcDir, "nested");
    fs.mkdirSync(nestedDir);
    fs.writeFileSync(path.join(nestedDir, "b.txt"), "beta");

    copyFile(srcDir, dstDir, defaultOpts({ recursive: true }));

    expect(fs.existsSync(path.join(dstDir, "a.txt"))).toBe(true);
    expect(fs.readFileSync(path.join(dstDir, "a.txt"), "utf-8")).toBe("alpha");
    expect(fs.existsSync(path.join(dstDir, "nested", "b.txt"))).toBe(true);
    expect(fs.readFileSync(path.join(dstDir, "nested", "b.txt"), "utf-8")).toBe("beta");
  });

  it("should not overwrite directory when noClobber is true and recursive", () => {
    const srcDir = path.join(tmpDir, "srcdir");
    const dstDir = path.join(tmpDir, "dstdir");
    fs.mkdirSync(srcDir);
    fs.mkdirSync(dstDir);
    fs.writeFileSync(path.join(srcDir, "a.txt"), "new");
    fs.writeFileSync(path.join(dstDir, "existing.txt"), "old");

    copyFile(srcDir, dstDir, defaultOpts({ recursive: true, noClobber: true }));

    // Destination should remain unchanged.
    expect(fs.existsSync(path.join(dstDir, "existing.txt"))).toBe(true);
    expect(fs.existsSync(path.join(dstDir, "a.txt"))).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// copyMultiple: copy multiple sources into a directory.
// ---------------------------------------------------------------------------

describe("copyMultiple", () => {
  it("should copy multiple files into a directory", () => {
    const dstDir = path.join(tmpDir, "dest");
    fs.mkdirSync(dstDir);

    const src1 = path.join(tmpDir, "a.txt");
    const src2 = path.join(tmpDir, "b.txt");
    fs.writeFileSync(src1, "alpha");
    fs.writeFileSync(src2, "beta");

    copyMultiple([src1, src2], dstDir, defaultOpts());

    expect(fs.readFileSync(path.join(dstDir, "a.txt"), "utf-8")).toBe("alpha");
    expect(fs.readFileSync(path.join(dstDir, "b.txt"), "utf-8")).toBe("beta");
  });

  it("should throw if target directory does not exist", () => {
    const src = path.join(tmpDir, "a.txt");
    fs.writeFileSync(src, "content");

    expect(() =>
      copyMultiple([src], path.join(tmpDir, "nope"), defaultOpts())
    ).toThrow(/not a directory/);
  });

  it("should throw if target is not a directory", () => {
    const src = path.join(tmpDir, "a.txt");
    const target = path.join(tmpDir, "target.txt");
    fs.writeFileSync(src, "content");
    fs.writeFileSync(target, "i am a file");

    expect(() =>
      copyMultiple([src], target, defaultOpts())
    ).toThrow(/not a directory/);
  });

  it("should return verbose messages for all copies", () => {
    const dstDir = path.join(tmpDir, "dest");
    fs.mkdirSync(dstDir);

    const src1 = path.join(tmpDir, "a.txt");
    const src2 = path.join(tmpDir, "b.txt");
    fs.writeFileSync(src1, "alpha");
    fs.writeFileSync(src2, "beta");

    const messages = copyMultiple(
      [src1, src2],
      dstDir,
      defaultOpts({ verbose: true })
    );

    expect(messages.length).toBe(2);
  });
});
