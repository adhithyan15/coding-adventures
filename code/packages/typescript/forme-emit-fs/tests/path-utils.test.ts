/**
 * path-utils.test.ts — route → on-disk path mapping safety tests.
 */

import { describe, it, expect } from "vitest";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { routeToOutPath } from "../src/path-utils.js";

let outDir: string;
async function freshOutDir() {
  outDir = await mkdtemp(join(tmpdir(), "forme-emit-fs-path-"));
  return outDir;
}
async function cleanOutDir() {
  if (outDir) await rm(outDir, { recursive: true, force: true });
}

describe("routeToOutPath", () => {
  it("strips a single leading slash", async () => {
    const dir = await freshOutDir();
    try {
      expect(routeToOutPath(dir, "/blog/hello.html"))
        .toBe(resolve(dir, "blog/hello.html"));
    } finally { await cleanOutDir(); }
  });

  it("works without a leading slash", async () => {
    const dir = await freshOutDir();
    try {
      expect(routeToOutPath(dir, "blog/hello.html"))
        .toBe(resolve(dir, "blog/hello.html"));
    } finally { await cleanOutDir(); }
  });

  it("rejects an empty route", async () => {
    const dir = await freshOutDir();
    try {
      expect(() => routeToOutPath(dir, ""))
        .toThrow(/empty route/);
    } finally { await cleanOutDir(); }
  });

  it("rejects a bare slash route", async () => {
    const dir = await freshOutDir();
    try {
      expect(() => routeToOutPath(dir, "/"))
        .toThrow(/no filename component/);
    } finally { await cleanOutDir(); }
  });

  it("rejects multiple leading slashes", async () => {
    const dir = await freshOutDir();
    try {
      expect(() => routeToOutPath(dir, "//etc/passwd"))
        .toThrow(/multiple slashes/);
    } finally { await cleanOutDir(); }
  });

  it("rejects parent-directory traversal", async () => {
    const dir = await freshOutDir();
    try {
      expect(() => routeToOutPath(dir, "/../../../etc/passwd"))
        .toThrow(/escape outDir/);
    } finally { await cleanOutDir(); }
  });

  it("rejects an embedded .. that escapes outDir", async () => {
    const dir = await freshOutDir();
    try {
      expect(() => routeToOutPath(dir, "/blog/../../escape.html"))
        .toThrow(/escape outDir/);
    } finally { await cleanOutDir(); }
  });

  it("allows nested directories", async () => {
    const dir = await freshOutDir();
    try {
      expect(routeToOutPath(dir, "/blog/2026/05/post.html"))
        .toBe(resolve(dir, "blog/2026/05/post.html"));
    } finally { await cleanOutDir(); }
  });
});
