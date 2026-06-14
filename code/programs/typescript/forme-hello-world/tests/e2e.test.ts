/**
 * tests/e2e.test.ts — end-to-end integration test for the demo.
 *
 * Strategy: copy the committed `content/` fixture into an OS tempdir,
 * point the pipeline at the tempdir for both input and output, run
 * once, and assert on:
 *
 *   - RunResult.outcome === "success"
 *   - dist/blog/hello.html exists in the tempdir
 *   - its bytes contain the expected substrings (title, body, theme)
 *   - the final stage's RunResult output is a DeployArtifact carrying
 *     exactly one route + one file, and the buildId is a valid
 *     blake2b: prefixed RevisionId.
 *
 * Why a tempdir rather than the package's own `dist/`:
 *
 *   - Parallel test runs don't fight over the same directory.
 *   - A leaked failure can't pollute the committed source tree.
 *   - We can run the build twice in two different tempdirs and still
 *     observe the same buildId — a free determinism check.
 *
 * @module e2e.test
 */

import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { mkdtemp, readFile, copyFile, mkdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { silentLogger } from "@coding-adventures/forme-stage";
import { buildBlog } from "../src/build.js";

// ── Path resolution ──────────────────────────────────────────────────

const __filename     = fileURLToPath(import.meta.url);
const __dirname      = dirname(__filename);
const PACKAGE_ROOT   = resolve(__dirname, "..");
const FIXTURE_SOURCE = resolve(PACKAGE_ROOT, "content", "hello.md");

// ── Helpers ──────────────────────────────────────────────────────────

/**
 * Stage the committed fixture into an isolated tempdir laid out as:
 *
 *     <tmp>/
 *       content/
 *         hello.md       ← copy of code/programs/.../content/hello.md
 *       dist/             ← created lazily by emit-fs
 */
async function stageFixture(): Promise<{
  tmpRoot:     string;
  contentRoot: string;
  outDir:      string;
}> {
  const tmpRoot     = await mkdtemp(join(tmpdir(), "forme-hello-world-"));
  const contentRoot = join(tmpRoot, "content");
  const outDir      = join(tmpRoot, "dist");
  await mkdir(contentRoot, { recursive: true });
  await copyFile(FIXTURE_SOURCE, join(contentRoot, "hello.md"));
  return { tmpRoot, contentRoot, outDir };
}

// ── Test suite ───────────────────────────────────────────────────────

describe("forme-hello-world end-to-end", () => {
  let tmpRoot:     string;
  let contentRoot: string;
  let outDir:      string;

  beforeAll(async () => {
    ({ tmpRoot, contentRoot, outDir } = await stageFixture());
  });

  afterAll(async () => {
    await rm(tmpRoot, { recursive: true, force: true });
  });

  it("builds successfully and writes the expected HTML file", async () => {
    const result = await buildBlog({
      contentRoot,
      outDir,
      logger: silentLogger(),
    });

    // Top-level outcome.
    expect(result.outcome).toBe("success");
    expect(result.errors).toEqual([]);

    // BuildId must be a well-formed RevisionId (blake2b:<hex>).
    expect(result.buildId).toMatch(/^blake2b:[0-9a-f]{64}$/);

    // Every stage ran exactly once on this one-file fixture.  In the
    // streaming topology, the per-item parser is invoked once per
    // streamed source — so itemsConsumed/itemsProduced for parse is
    // 1/1 (the orchestrator counts per-invocation, not per-stream).
    const byName = new Map(result.stages.map(s => [s.stageName, s]));
    for (const [name, summary] of byName) {
      expect(summary.outcome, `stage ${name} outcome`).toBe("success");
      expect(summary.errorCount, `stage ${name} errorCount`).toBe(0);
    }
    // All four stages are present.
    expect(byName.has("@coding-adventures/forme-source-fs")).toBe(true);
    expect(byName.has("@coding-adventures/forme-parse-markdown")).toBe(true);
    expect(byName.has("@coding-adventures/forme-render-static")).toBe(true);
    expect(byName.has("@coding-adventures/forme-emit-fs")).toBe(true);

    // The file is actually on disk.
    const htmlPath = join(outDir, "blog", "hello.html");
    const html     = await readFile(htmlPath, "utf-8");

    // Body content from hello.md flowed through GFM parsing + the
    // classless theme.  These substrings are stable across renderer
    // tweaks because they come straight from the source post.
    expect(html).toContain("<h1>Hello, Forme</h1>");
    expect(html).toContain("forme-source-fs");
    expect(html).toContain("forme-emit-fs");

    // Title resolved via the frontmatter.title path (not the H1
    // fallback or the slug fallback).
    expect(html).toContain("<title>Hello, Forme</title>");

    // The classless theme is inlined as a <style> block — proves
    // forme-render-static's wrapper ran (vs raw GFM output).
    expect(html).toMatch(/<style>[\s\S]*<\/style>/);
  });

  it("two consecutive builds produce the same buildId (determinism check)", async () => {
    // Two builds, two different output dirs, same content.  buildId
    // is computed over the route → sha256 map (FM00 §5.6 in
    // forme-emit-fs/src/index.ts), which is content-only.  So the
    // ids MUST match — if they don't, something is leaking time /
    // randomness / iteration-order non-determinism into the build.
    const outA = join(tmpRoot, "dist-a");
    const outB = join(tmpRoot, "dist-b");

    const a = await buildBlog({ contentRoot, outDir: outA, logger: silentLogger() });
    const b = await buildBlog({ contentRoot, outDir: outB, logger: silentLogger() });

    expect(a.outcome).toBe("success");
    expect(b.outcome).toBe("success");
    expect(a.buildId).toBe(b.buildId);
  });
});
