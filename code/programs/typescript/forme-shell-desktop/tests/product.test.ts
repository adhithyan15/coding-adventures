import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { validateAuthoringProject } from "@coding-adventures/forme-authoring-core";
import type { DeployArtifact } from "@coding-adventures/forme-types";
import { buildAuthoringProduct, previewAuthoringProduct } from "../src/product.js";

const project = validateAuthoringProject({
  schemaVersion: 1,
  projectId: "01952c0d-7e63-7000-8000-000000000068",
  title: "My Forme site",
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
        type: "heading",
        level: 1,
        children: [{ type: "text", value: "Hello from Forme" }],
      }],
    },
  }],
});

describe("desktop authoring product pipeline", () => {
  it("runs the real Forme pipeline and returns its static artifact", async () => {
    const output = await mkdtemp(join(tmpdir(), "forme-desktop-product-"));
    try {
      const result = await buildAuthoringProduct({
        project,
        revision: "sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        output,
      });
      expect(result.outcome).toBe("success");
      const artifact = result.outputs.site as DeployArtifact;
      expect(artifact.variant).toEqual({ kind: "dist-tree" });
      expect(Object.keys(artifact.files)).toEqual(["welcome.html"]);
      const html = new TextDecoder().decode(artifact.files["welcome.html"]);
      expect(html).toContain("<h1>Hello from Forme</h1>");
      expect(await readFile(join(output, "welcome.html"), "utf8")).toBe(html);
    } finally {
      await rm(output, { recursive: true, force: true });
    }
  });

  it("is byte-identical and build-identical across clean materializations", async () => {
    const first = await mkdtemp(join(tmpdir(), "forme-desktop-product-a-"));
    const second = await mkdtemp(join(tmpdir(), "forme-desktop-product-b-"));
    try {
      const a = await buildAuthoringProduct({ project, revision: "revision-1", output: first });
      const b = await buildAuthoringProduct({ project, revision: "revision-1", output: second });
      const aa = a.outputs.site as DeployArtifact;
      const bb = b.outputs.site as DeployArtifact;
      expect(a.outcome).toBe("success");
      expect(b.outcome).toBe("success");
      expect(a.buildId).toBe(b.buildId);
      expect(aa.manifest.buildId).toBe(bb.manifest.buildId);
      expect(aa.files["welcome.html"]).toEqual(bb.files["welcome.html"]);
    } finally {
      await Promise.all([
        rm(first, { recursive: true, force: true }),
        rm(second, { recursive: true, force: true }),
      ]);
    }
  });

  it("rejects an unreviewed theme before constructing a pipeline", async () => {
    await expect(buildAuthoringProduct({
      project: { ...project, site: { ...project.site, themeId: "unreviewed" } },
      revision: "revision-1",
      output: "/unused",
    })).rejects.toThrow(/reviewed desktop theme/);
  });

  it("rejects an empty persisted revision before constructing a pipeline", async () => {
    await expect(buildAuthoringProduct({
      project,
      revision: "",
      output: "/unused",
    })).rejects.toThrow(/revision is invalid/);
  });

  it("rejects an empty output directory before constructing a pipeline", async () => {
    await expect(buildAuthoringProduct({
      project,
      revision: "revision-1",
      output: "",
    })).rejects.toThrow(/output directory is invalid/);
  });

  it("publishes the same product through the real FM03 watch and FM07 snapshot path", async () => {
    const preview = await previewAuthoringProduct({ project, revision: "preview-revision" });
    expect(preview.attempt).toEqual({
      outcome: "ready",
      revision: "preview-revision",
      buildId: preview.snapshot.buildId,
      diagnostics: [],
    });
    expect([...preview.snapshot.files.keys()]).toEqual(["welcome.html"]);
    expect(new TextDecoder().decode(preview.snapshot.files.get("welcome.html"))).toContain(
      "<h1>Hello from Forme</h1>",
    );
  });
});
