/**
 * Tests for mv -- move (rename) files and directories.
 *
 * We test the exported `moveFile` and `moveMultiple` functions using
 * temporary directories. Each test gets a fresh temp directory.
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { moveFile, moveMultiple, MoveOptions } from "../src/mv.js";

// ---------------------------------------------------------------------------
// Helpers: temp directory management and default options.
// ---------------------------------------------------------------------------

let tmpDir: string;

function defaultOpts(overrides: Partial<MoveOptions> = {}): MoveOptions {
  return {
    force: false,
    noClobber: false,
    verbose: false,
    update: false,
    ...overrides,
  };
}

beforeEach(() => {
  tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "mv-test-"));
});

afterEach(() => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

// ---------------------------------------------------------------------------
// moveFile: basic rename.
// ---------------------------------------------------------------------------

describe("moveFile", () => {
  it("should rename a file to a new name", () => {
    const src = path.join(tmpDir, "old.txt");
    const dst = path.join(tmpDir, "new.txt");
    fs.writeFileSync(src, "hello");

    moveFile(src, dst, defaultOpts());

    expect(fs.existsSync(src)).toBe(false);
    expect(fs.existsSync(dst)).toBe(true);
    expect(fs.readFileSync(dst, "utf-8")).toBe("hello");
  });

  it("should preserve file contents exactly", () => {
    const src = path.join(tmpDir, "data.bin");
    const dst = path.join(tmpDir, "moved.bin");
    const data = Buffer.from([0x00, 0xff, 0x80, 0x7f]);
    fs.writeFileSync(src, data);

    moveFile(src, dst, defaultOpts());

    expect(Buffer.compare(fs.readFileSync(dst), data)).toBe(0);
    expect(fs.existsSync(src)).toBe(false);
  });

  it("should move a file into an existing directory", () => {
    const src = path.join(tmpDir, "file.txt");
    const dstDir = path.join(tmpDir, "subdir");
    fs.writeFileSync(src, "content");
    fs.mkdirSync(dstDir);

    moveFile(src, dstDir, defaultOpts());

    expect(fs.existsSync(src)).toBe(false);
    expect(fs.readFileSync(path.join(dstDir, "file.txt"), "utf-8")).toBe("content");
  });

  it("should throw if source does not exist", () => {
    const src = path.join(tmpDir, "nonexistent.txt");
    const dst = path.join(tmpDir, "dest.txt");

    expect(() => moveFile(src, dst, defaultOpts())).toThrow(
      /No such file or directory/
    );
  });

  // -------------------------------------------------------------------------
  // Overwrite behavior.
  // -------------------------------------------------------------------------

  it("should overwrite an existing file by default", () => {
    const src = path.join(tmpDir, "src.txt");
    const dst = path.join(tmpDir, "dst.txt");
    fs.writeFileSync(src, "new");
    fs.writeFileSync(dst, "old");

    moveFile(src, dst, defaultOpts());

    expect(fs.existsSync(src)).toBe(false);
    expect(fs.readFileSync(dst, "utf-8")).toBe("new");
  });

  // -------------------------------------------------------------------------
  // No-clobber mode (-n).
  // -------------------------------------------------------------------------

  it("should not overwrite when noClobber is true", () => {
    const src = path.join(tmpDir, "src.txt");
    const dst = path.join(tmpDir, "dst.txt");
    fs.writeFileSync(src, "new");
    fs.writeFileSync(dst, "old");

    moveFile(src, dst, defaultOpts({ noClobber: true }));

    // Source should still exist, destination unchanged.
    expect(fs.existsSync(src)).toBe(true);
    expect(fs.readFileSync(dst, "utf-8")).toBe("old");
  });

  it("should move when noClobber is true and destination does not exist", () => {
    const src = path.join(tmpDir, "src.txt");
    const dst = path.join(tmpDir, "dst.txt");
    fs.writeFileSync(src, "content");

    moveFile(src, dst, defaultOpts({ noClobber: true }));

    expect(fs.existsSync(src)).toBe(false);
    expect(fs.readFileSync(dst, "utf-8")).toBe("content");
  });

  // -------------------------------------------------------------------------
  // Update mode (-u).
  // -------------------------------------------------------------------------

  it("should skip move when update is true and source is older", () => {
    const src = path.join(tmpDir, "src.txt");
    const dst = path.join(tmpDir, "dst.txt");
    fs.writeFileSync(dst, "newer content");

    // Write source after a small delay (the file system may have
    // sub-second resolution, so we set mtime explicitly).
    fs.writeFileSync(src, "older content");

    // Make source older than destination.
    const pastTime = new Date(Date.now() - 10000);
    fs.utimesSync(src, pastTime, pastTime);

    moveFile(src, dst, defaultOpts({ update: true }));

    // Source should still exist, destination unchanged.
    expect(fs.existsSync(src)).toBe(true);
    expect(fs.readFileSync(dst, "utf-8")).toBe("newer content");
  });

  it("should move when update is true and source is newer", () => {
    const src = path.join(tmpDir, "src.txt");
    const dst = path.join(tmpDir, "dst.txt");

    fs.writeFileSync(dst, "old content");
    // Make destination older.
    const pastTime = new Date(Date.now() - 10000);
    fs.utimesSync(dst, pastTime, pastTime);

    fs.writeFileSync(src, "new content");

    moveFile(src, dst, defaultOpts({ update: true }));

    expect(fs.existsSync(src)).toBe(false);
    expect(fs.readFileSync(dst, "utf-8")).toBe("new content");
  });

  // -------------------------------------------------------------------------
  // Verbose mode (-v).
  // -------------------------------------------------------------------------

  it("should return verbose messages when verbose is true", () => {
    const src = path.join(tmpDir, "src.txt");
    const dst = path.join(tmpDir, "dst.txt");
    fs.writeFileSync(src, "content");

    const messages = moveFile(src, dst, defaultOpts({ verbose: true }));

    expect(messages.length).toBe(1);
    expect(messages[0]).toContain("renamed");
    expect(messages[0]).toContain("->");
  });

  it("should return empty messages when verbose is false", () => {
    const src = path.join(tmpDir, "src.txt");
    const dst = path.join(tmpDir, "dst.txt");
    fs.writeFileSync(src, "content");

    const messages = moveFile(src, dst, defaultOpts());

    expect(messages).toEqual([]);
  });

  // -------------------------------------------------------------------------
  // Directory rename.
  // -------------------------------------------------------------------------

  it("should rename a directory", () => {
    const srcDir = path.join(tmpDir, "olddir");
    const dstDir = path.join(tmpDir, "newdir");
    fs.mkdirSync(srcDir);
    fs.writeFileSync(path.join(srcDir, "file.txt"), "data");

    moveFile(srcDir, dstDir, defaultOpts());

    expect(fs.existsSync(srcDir)).toBe(false);
    expect(fs.existsSync(dstDir)).toBe(true);
    expect(fs.readFileSync(path.join(dstDir, "file.txt"), "utf-8")).toBe("data");
  });

  it("should move an empty file", () => {
    const src = path.join(tmpDir, "empty.txt");
    const dst = path.join(tmpDir, "moved-empty.txt");
    fs.writeFileSync(src, "");

    moveFile(src, dst, defaultOpts());

    expect(fs.existsSync(src)).toBe(false);
    expect(fs.existsSync(dst)).toBe(true);
    expect(fs.readFileSync(dst, "utf-8")).toBe("");
  });
});

// ---------------------------------------------------------------------------
// moveMultiple: move multiple sources into a directory.
// ---------------------------------------------------------------------------

describe("moveMultiple", () => {
  it("should move multiple files into a directory", () => {
    const dstDir = path.join(tmpDir, "dest");
    fs.mkdirSync(dstDir);

    const src1 = path.join(tmpDir, "a.txt");
    const src2 = path.join(tmpDir, "b.txt");
    fs.writeFileSync(src1, "alpha");
    fs.writeFileSync(src2, "beta");

    moveMultiple([src1, src2], dstDir, defaultOpts());

    expect(fs.existsSync(src1)).toBe(false);
    expect(fs.existsSync(src2)).toBe(false);
    expect(fs.readFileSync(path.join(dstDir, "a.txt"), "utf-8")).toBe("alpha");
    expect(fs.readFileSync(path.join(dstDir, "b.txt"), "utf-8")).toBe("beta");
  });

  it("should throw if target directory does not exist", () => {
    const src = path.join(tmpDir, "a.txt");
    fs.writeFileSync(src, "content");

    expect(() =>
      moveMultiple([src], path.join(tmpDir, "nope"), defaultOpts())
    ).toThrow(/not a directory/);
  });

  it("should throw if target is not a directory", () => {
    const src = path.join(tmpDir, "a.txt");
    const target = path.join(tmpDir, "target.txt");
    fs.writeFileSync(src, "content");
    fs.writeFileSync(target, "i am a file");

    expect(() =>
      moveMultiple([src], target, defaultOpts())
    ).toThrow(/not a directory/);
  });

  it("should return verbose messages for all moves", () => {
    const dstDir = path.join(tmpDir, "dest");
    fs.mkdirSync(dstDir);

    const src1 = path.join(tmpDir, "a.txt");
    const src2 = path.join(tmpDir, "b.txt");
    fs.writeFileSync(src1, "alpha");
    fs.writeFileSync(src2, "beta");

    const messages = moveMultiple(
      [src1, src2],
      dstDir,
      defaultOpts({ verbose: true })
    );

    expect(messages.length).toBe(2);
  });
});
