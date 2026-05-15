/**
 * forme-source-fs — walker tests
 */

import { afterEach, beforeEach, describe, it, expect } from "vitest";
import { mkdir, mkdtemp, rm, writeFile, symlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { parseGlob, walkFiles } from "../src/walker.js";

let root: string;

beforeEach(async () => {
  root = await mkdtemp(join(tmpdir(), "forme-source-fs-test-"));
});

afterEach(async () => {
  await rm(root, { recursive: true, force: true });
});

async function collect(iter: AsyncIterable<string>): Promise<string[]> {
  const out: string[] = [];
  for await (const v of iter) out.push(v);
  return out;
}

describe("parseGlob", () => {
  it("accepts the canonical **/*.<ext> shape", () => {
    expect(parseGlob("**/*.md").ext).toBe(".md");
    expect(parseGlob("**/*.html").ext).toBe(".html");
  });

  it("normalises extension to lower-case", () => {
    expect(parseGlob("**/*.MD").ext).toBe(".md");
  });

  it("rejects unsupported patterns", () => {
    expect(() => parseGlob("*.md")).toThrow(/not supported/);
    expect(() => parseGlob("posts/**/*.md")).toThrow(/not supported/);
    expect(() => parseGlob("**/*.{md,mdx}")).toThrow(/not supported/);
    expect(() => parseGlob("**/*.md.txt")).toThrow(/not supported/);
    expect(() => parseGlob("")).toThrow(/not supported/);
  });
});

describe("walkFiles", () => {
  it("returns nothing on a missing directory (no throw)", async () => {
    const files = await collect(walkFiles(join(root, "absent"), ".md"));
    expect(files).toEqual([]);
  });

  it("yields matching files in deterministic sorted order", async () => {
    await writeFile(join(root, "b.md"), "b");
    await writeFile(join(root, "a.md"), "a");
    await writeFile(join(root, "c.md"), "c");
    const files = await collect(walkFiles(root, ".md"));
    expect(files).toEqual([
      join(root, "a.md"),
      join(root, "b.md"),
      join(root, "c.md"),
    ]);
  });

  it("recurses into subdirectories", async () => {
    await mkdir(join(root, "posts", "2026"), { recursive: true });
    await writeFile(join(root, "top.md"), "x");
    await writeFile(join(root, "posts", "hello.md"), "x");
    await writeFile(join(root, "posts", "2026", "deep.md"), "x");
    const files = await collect(walkFiles(root, ".md"));
    expect(files.length).toBe(3);
    expect(files.some(f => f.endsWith("/top.md"))).toBe(true);
    expect(files.some(f => f.endsWith("/hello.md"))).toBe(true);
    expect(files.some(f => f.endsWith("/deep.md"))).toBe(true);
  });

  it("filters by extension", async () => {
    await writeFile(join(root, "post.md"), "x");
    await writeFile(join(root, "image.png"), "x");
    await writeFile(join(root, "data.json"), "x");
    const md = await collect(walkFiles(root, ".md"));
    expect(md.length).toBe(1);
    const png = await collect(walkFiles(root, ".png"));
    expect(png.length).toBe(1);
  });

  it("ignores dotfiles and dot-directories", async () => {
    await writeFile(join(root, ".hidden.md"), "x");
    await mkdir(join(root, ".git"));
    await writeFile(join(root, ".git", "config.md"), "x");
    await writeFile(join(root, "visible.md"), "x");
    const files = await collect(walkFiles(root, ".md"));
    expect(files).toEqual([join(root, "visible.md")]);
  });

  it("skips symlinks (cycle hazard)", async () => {
    await writeFile(join(root, "real.md"), "x");
    try {
      await symlink(join(root, "real.md"), join(root, "link.md"));
    } catch {
      // Some CI systems disallow symlinks; skip gracefully.
      return;
    }
    const files = await collect(walkFiles(root, ".md"));
    expect(files).toEqual([join(root, "real.md")]);
  });

  it("case-insensitive extension match", async () => {
    await writeFile(join(root, "post.MD"), "x");
    const files = await collect(walkFiles(root, ".md"));
    expect(files.length).toBe(1);
  });

  it("does not match files with no extension", async () => {
    await writeFile(join(root, "README"), "x");
    const files = await collect(walkFiles(root, ".md"));
    expect(files).toEqual([]);
  });
});
