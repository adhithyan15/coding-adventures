import { mkdtemp, rm } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { validateAuthoringProject } from "@coding-adventures/forme-authoring-core";
import { runProductWorker } from "../src/product-worker.js";

const project = validateAuthoringProject({
  schemaVersion: 1,
  projectId: "01952c0d-7e63-7000-8000-000000000068",
  title: "Worker product fixture",
  site: { baseUrl: null, themeId: "forme-classless" },
  workflow: { lastPublication: null },
  activeDocumentId: "01952c0d-7e63-7000-8000-000000000069",
  documents: [{
    id: "01952c0d-7e63-7000-8000-000000000069",
    slug: "welcome",
    title: "Welcome",
    status: "draft",
    body: {
      type: "document",
      children: [{
        type: "paragraph",
        children: [{ type: "text", value: "Bundled worker" }],
      }],
    },
  }],
});

describe("bundled product worker", () => {
  it("runs the exact product and returns only checked artifact metadata", async () => {
    const output = await mkdtemp(join(tmpdir(), "forme-product-worker-"));
    try {
      const response = await runProductWorker(new TextEncoder().encode(JSON.stringify({
        schemaVersion: 1,
        project,
        revision: "worker-revision",
        output,
      })));
      const decoded = JSON.parse(new TextDecoder().decode(response));
      expect(decoded.schemaVersion).toBe(1);
      expect(decoded.revision).toBe("worker-revision");
      expect(decoded.buildId).toMatch(/^blake2b:/);
      expect(decoded.manifestSha256).toMatch(/^[A-Za-z0-9+/]{43}=$/);
      expect(decoded.manifest).toMatchObject({ version: 1, fileCount: 1 });
      expect(decoded.files).toEqual([expect.objectContaining({
        path: "welcome.html",
        size: expect.any(Number),
        sha256: expect.stringMatching(/^[0-9a-f]{64}$/),
      })]);
      expect(JSON.stringify(decoded)).not.toContain(output);
    } finally {
      await rm(output, { recursive: true, force: true });
    }
  });

  it("maps known build failure to one fixed code", async () => {
    const request = new TextEncoder().encode(JSON.stringify({
      schemaVersion: 1,
      project,
      revision: "worker-revision",
      output: "/unused",
    }));
    const response = await runProductWorker(request, {
      async build() { throw new Error("sensitive native detail"); },
    });
    expect(JSON.parse(new TextDecoder().decode(response))).toEqual({
      schemaVersion: 1,
      error: { code: "BUILD_FAILED" },
    });
    expect(new TextDecoder().decode(response)).not.toContain("sensitive");
  });

  it("bundles as a silent one-record subprocess", async () => {
    const output = await mkdtemp(join(tmpdir(), "forme-product-worker-process-"));
    try {
      const worker = join(import.meta.dirname, "..", "dist", "worker", "forme-product-worker.cjs");
      const request = JSON.stringify({
        schemaVersion: 1,
        project,
        revision: "subprocess-revision",
        output,
      });
      const result = spawnSync(process.execPath, [worker], {
        input: request,
        encoding: "utf8",
        maxBuffer: 9 * 1024 * 1024,
      });
      expect(result.status).toBe(0);
      expect(result.stderr).toBe("");
      expect(JSON.parse(result.stdout)).toEqual(expect.objectContaining({
        schemaVersion: 1,
        revision: "subprocess-revision",
      }));

      const invalid = spawnSync(process.execPath, [worker], {
        input: "{}",
        encoding: "utf8",
      });
      expect(invalid.status).toBe(2);
      expect(invalid.stdout).toBe("");
      expect(invalid.stderr).toBe("");
    } finally {
      await rm(output, { recursive: true, force: true });
    }
  });
});
