import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { validateAuthoringProject } from "@coding-adventures/forme-authoring-core";
import { canonicalDeployManifest } from "@coding-adventures/forme-deploy-runner-core";
import type { DeployArtifact } from "@coding-adventures/forme-types";
import {
  decodeWorkerRequest,
  encodeWorkerFailure,
  encodeWorkerSuccess,
  MAX_WORKER_MESSAGE_BYTES,
} from "../src/worker-protocol.js";

const project = validateAuthoringProject({
  schemaVersion: 1,
  projectId: "01952c0d-7e63-7000-8000-000000000068",
  title: "Worker fixture",
  site: { baseUrl: null, themeId: "forme-classless" },
  workflow: { lastPublication: null },
  activeDocumentId: null,
  documents: [],
});

function artifact(files: DeployArtifact["files"] = { "index.html": new Uint8Array() }): DeployArtifact {
  return {
    variant: { kind: "dist-tree" },
    files,
    manifest: {
      routes: [],
      assets: [],
      buildTime: "1970-01-01T00:00:00.000Z",
      buildId: "sha256:BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=" as never,
    },
  };
}

describe("bundled product worker protocol", () => {
  it("accepts only the closed versioned build request", () => {
    const request = decodeWorkerRequest(new TextEncoder().encode(JSON.stringify({
      schemaVersion: 1,
      project,
      revision: "sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
      output: "/native/workspace/output",
    })));

    expect(request).toEqual({
      schemaVersion: 1,
      project,
      revision: "sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
      output: "/native/workspace/output",
    });
    expect(Object.isFrozen(request)).toBe(true);
  });

  it.each([
    { schemaVersion: 2, project, revision: "revision-1", output: "/tmp/out" },
    { schemaVersion: 1, project, revision: "", output: "/tmp/out" },
    { schemaVersion: 1, project, revision: "revision-1", output: "relative/out" },
    { schemaVersion: 1, project, revision: "revision-1", output: "/tmp/out", extra: true },
  ])("rejects a malformed or expanded request %#", (value) => {
    expect(() => decodeWorkerRequest(new TextEncoder().encode(JSON.stringify(value)))).toThrow(
      /worker request is invalid/,
    );
  });

  it("rejects oversized input before parsing", () => {
    expect(() => decodeWorkerRequest(new Uint8Array(MAX_WORKER_MESSAGE_BYTES + 1))).toThrow(
      /worker request is invalid/,
    );
  });

  it.each([
    new Uint8Array(),
    new TextEncoder().encode("not-json"),
    new TextEncoder().encode("[]"),
    new TextEncoder().encode(JSON.stringify({
      schemaVersion: 1,
      project: {},
      revision: "revision-1",
      output: "/tmp/out",
    })),
    new TextEncoder().encode(JSON.stringify({
      schemaVersion: 1,
      project,
      revision: "revision\u0000",
      output: "/tmp/out",
    })),
  ])("rejects hostile request bytes %#", (bytes) => {
    expect(() => decodeWorkerRequest(bytes)).toThrow(/worker request is invalid/);
  });

  it("emits a deterministic metadata-only success record", () => {
    const artifact: DeployArtifact = {
      variant: { kind: "dist-tree" },
      files: {
        "z/index.html": new TextEncoder().encode("<h1>Z</h1>"),
        "a.html": new TextEncoder().encode("A"),
      },
      manifest: {
        routes: [],
        assets: [],
        buildTime: "1970-01-01T00:00:00.000Z",
        buildId: "sha256:BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=" as never,
      },
    };

    const first = encodeWorkerSuccess("revision-1", artifact);
    const second = encodeWorkerSuccess("revision-1", artifact);
    expect(first).toEqual(second);
    const response = JSON.parse(new TextDecoder().decode(first));
    const manifest = {
      version: 1,
      fileCount: 2,
      totalSizeBytes: 11,
      files: {
        "a.html": {
          outputPath: "a.html",
          contentType: "text/html; charset=utf-8",
          sizeBytes: 1,
          sha256: "VZrq0IJk1XldOQlxjN0Fq9SVcuhP5VWQ7vMaiKCP3/0=",
          source: "extra",
        },
        "z/index.html": {
          outputPath: "z/index.html",
          contentType: "text/html; charset=utf-8",
          sizeBytes: 10,
          sha256: "+RMwqug/PtAXANzJRRXzlPQcC0OW1mCOG1PFjZVpDMQ=",
          source: "extra",
        },
      },
    } as const;
    expect(response).toEqual({
      schemaVersion: 1,
      revision: "revision-1",
      buildId: artifact.manifest.buildId,
      manifestSha256: createHash("sha256")
        .update(canonicalDeployManifest(manifest))
        .digest("base64"),
      manifest,
      files: [
        {
          path: "a.html",
          size: 1,
          sha256: "559aead08264d5795d3909718cdd05abd49572e84fe55590eef31a88a08fdffd",
        },
        {
          path: "z/index.html",
          size: 10,
          sha256: "f91330aae83f3ed01700dcc94515f394f41c0b4396d6608e1b53c58d95690cc4",
        },
      ],
    });
    expect(new TextDecoder().decode(first)).not.toContain("native/workspace");
  });

  it("rejects a non-portable output path instead of describing it", () => {
    const artifact: DeployArtifact = {
      variant: { kind: "dist-tree" },
      files: { "../escape": new Uint8Array() },
      manifest: {
        routes: [],
        assets: [],
        buildTime: "1970-01-01T00:00:00.000Z",
        buildId: "revision-1" as never,
      },
    };
    expect(() => encodeWorkerSuccess("revision-1", artifact)).toThrow(/artifact path is invalid/);
  });

  it("rejects malformed artifact metadata before emitting a response", () => {
    expect(() => encodeWorkerSuccess("", artifact())).toThrow(/worker response is invalid/);
    expect(() => encodeWorkerSuccess("revision-1", {
      ...artifact(),
      variant: { kind: "pdf", pageCount: 1 },
    })).toThrow(/worker response is invalid/);
    expect(() => encodeWorkerSuccess("revision-1", {
      ...artifact(),
      files: { "index.html": "not-bytes" as never },
    })).toThrow(/worker response is invalid/);
    expect(() => encodeWorkerSuccess("revision-1", {
      ...artifact(),
      manifest: { ...artifact().manifest, buildId: "" as never },
    })).toThrow(/worker response is invalid/);
  });

  it.each([
    "/absolute", "windows\\path", "a/../escape", "a//b", "C:escape", "CON", "name.",
    "safe/\u202eunsafe",
  ])(
    "rejects non-portable artifact path %s",
    (path) => {
      expect(() => encodeWorkerSuccess("revision-1", artifact({ [path]: new Uint8Array() }))).toThrow(
        /artifact path is invalid/,
      );
    },
  );

  it.each([
    { "Page.html": new Uint8Array(), "page.html": new Uint8Array() } as Record<string, Uint8Array>,
    { assets: new Uint8Array(), "assets/site.css": new Uint8Array() } as Record<string, Uint8Array>,
  ])("rejects portable path identity collisions", (files) => {
    expect(() => encodeWorkerSuccess("revision-1", artifact(files))).toThrow(
      /artifact path is invalid/,
    );
  });

  it("emits one closed failure code without an exception detail", () => {
    expect(JSON.parse(new TextDecoder().decode(encodeWorkerFailure()))).toEqual({
      schemaVersion: 1,
      error: { code: "BUILD_FAILED" },
    });
  });

  it("assigns reviewed content types without inspecting file bytes", () => {
    const files = Object.fromEntries([
      "page.html",
      "style.css",
      "app.js",
      "data.json",
      "icon.svg",
      "logo.png",
      "photo.jpg",
      "second.jpeg",
      "archive.bin",
    ].map((path) => [path, new Uint8Array()]));
    const response = JSON.parse(new TextDecoder().decode(
      encodeWorkerSuccess("revision-1", artifact(files)),
    ));
    expect(Object.fromEntries(Object.entries(response.manifest.files).map(
      ([path, entry]) => [path, (entry as { contentType: string }).contentType],
    ))).toEqual({
      "app.js": "text/javascript; charset=utf-8",
      "archive.bin": "application/octet-stream",
      "data.json": "application/json; charset=utf-8",
      "icon.svg": "image/svg+xml",
      "logo.png": "image/png",
      "page.html": "text/html; charset=utf-8",
      "photo.jpg": "image/jpeg",
      "second.jpeg": "image/jpeg",
      "style.css": "text/css; charset=utf-8",
    });
  });
});
