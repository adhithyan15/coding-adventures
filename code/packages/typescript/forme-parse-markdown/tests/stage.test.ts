/**
 * stage.test.ts — integration tests for the parseMarkdown stage.
 *
 * Verifies:
 *   - stage shape (consumes/produces/capabilities/apiVersion)
 *   - ContentSource → ContentNode round-trip (with and without
 *     frontmatter)
 *   - identity pass-through, revision recomputation rules
 *   - document tree shape (heading + paragraph)
 *   - config defaults (gfm:true implicit)
 *   - revision changes on body edit, frontmatter edit, path move
 *   - frontmatter values appear in node.frontmatter
 *   - BOM is stripped before frontmatter detection
 */

import { describe, it, expect } from "vitest";
import { Kinds } from "@coding-adventures/forme-types";
import {
  createCancellationTokenSource,
  inMemoryCache,
  inMemoryEventBus,
  noOpTelemetryEmitter,
  silentLogger,
  systemClock,
  deniedEnvApi,
  deniedFilesystemApi,
  deniedNetworkApi,
  deniedShellApi,
  deniedStorageApi,
  type StageContext,
} from "@coding-adventures/forme-stage";
import { generateLogicalId, computeRevisionId } from "@coding-adventures/forme-identity";
import parseMarkdown from "../src/index.js";

function makeCtx(): StageContext {
  return {
    logger: silentLogger(),
    cancellation: createCancellationTokenSource().token,
    time: systemClock(),
    cache: inMemoryCache(),
    telemetry: noOpTelemetryEmitter(),
    storage: deniedStorageApi(),
    network: deniedNetworkApi(),
    env: deniedEnvApi(),
    filesystem: deniedFilesystemApi(),
    shell: deniedShellApi(),
    events: inMemoryEventBus(),
  };
}

function makeSource(text: string, path = "posts/hello.md") {
  const bytes = new TextEncoder().encode(text);
  return {
    path,
    bytes,
    mimeType: "text/markdown",
    identity: generateLogicalId(),
    revision: computeRevisionId({ path, bytes: Array.from(bytes) }),
    providerMeta: { mtimeMs: 0, sizeBytes: bytes.byteLength },
  };
}

function run(text: string, path?: string, config: object = {}) {
  const src = makeSource(text, path);
  const out = parseMarkdown.run(src as never, config as never, makeCtx());
  // Stage returns synchronously (single value).  Cast to the
  // ContentNode-ish shape we want to assert on.
  return { src, node: out as unknown as {
    identity: string;
    revision: string;
    document: { type: string; children: Array<{ type: string; [k: string]: unknown }> };
    frontmatter: Record<string, unknown>;
    route: string | null;
    assetRefs: readonly unknown[];
    sourcePath: string;
  } };
}

describe("parseMarkdown — stage shape", () => {
  it("declares ContentSource in / ContentNode out", () => {
    expect(parseMarkdown.consumes).toEqual(Kinds.ContentSource);
    expect(parseMarkdown.produces).toEqual(Kinds.ContentNode);
  });

  it("declares no capabilities (pure transform)", () => {
    expect(parseMarkdown.capabilities).toEqual([]);
  });

  it("targets kernel apiVersion 1", () => {
    expect(parseMarkdown.apiVersion).toBe(1);
  });

  it("has a configSchema (gfm boolean)", () => {
    expect(parseMarkdown.configSchema).toMatchObject({
      type: "object",
      properties: { gfm: { type: "boolean" } },
    });
  });
});

describe("parseMarkdown — running, no frontmatter", () => {
  it("emits a ContentNode whose document is a parsed AST", () => {
    const { node } = run("# Hello\n\nWorld.\n");
    expect(node.document.type).toBe("document");
    expect(node.document.children.length).toBeGreaterThan(0);
    expect(node.document.children[0]!.type).toBe("heading");
    expect(node.document.children[1]!.type).toBe("paragraph");
  });

  it("frontmatter is empty when source has none", () => {
    const { node } = run("# Hello\n");
    expect(node.frontmatter).toEqual({});
  });

  it("route is null (parser does not assign routes)", () => {
    const { node } = run("# Hello\n");
    expect(node.route).toBeNull();
  });

  it("assetRefs is empty in v0", () => {
    const { node } = run("# Hello\n![alt](x.png)\n");
    expect(node.assetRefs).toEqual([]);
  });

  it("sourcePath passes through from the source", () => {
    const { node } = run("# Hello\n", "blog/2026/post.md");
    expect(node.sourcePath).toBe("blog/2026/post.md");
  });

  it("identity passes through unchanged", () => {
    const { src, node } = run("# x\n");
    expect(node.identity).toBe(src.identity);
  });
});

describe("parseMarkdown — running, with frontmatter", () => {
  it("extracts frontmatter and strips it from the body", () => {
    const md = "---\ntitle: Hello\ndate: 2026-05-15\n---\n# Heading\n\nBody.\n";
    const { node } = run(md);
    expect(node.frontmatter).toEqual({ title: "Hello", date: "2026-05-15" });
    expect(node.document.type).toBe("document");
    expect(node.document.children[0]!.type).toBe("heading");
  });

  it("malformed frontmatter is preserved into the body verbatim", () => {
    // Missing closing fence — should treat whole thing as body.
    const md = "---\ntitle: Hello\n\n# Heading\n";
    const { node } = run(md);
    expect(node.frontmatter).toEqual({});
    // The first child should be a paragraph beginning with "---",
    // proving the fence text leaked into the body.
    const first = node.document.children[0]!;
    expect(["paragraph", "thematic_break"]).toContain(first.type);
  });

  it("strips a UTF-8 BOM before detecting frontmatter", () => {
    const md = "﻿---\ntitle: BOM\n---\nbody\n";
    const { node } = run(md);
    expect(node.frontmatter).toEqual({ title: "BOM" });
  });
});

describe("parseMarkdown — revision discipline", () => {
  it("revision is recomputed (not equal to source revision)", () => {
    // The source revision hashes bytes; the node revision hashes the
    // parsed document — different inputs, different hashes (essentially
    // always).
    const { src, node } = run("# Hello\n");
    expect(node.revision).not.toBe(src.revision);
    expect(node.revision).toMatch(/^blake2b:/);
  });

  it("same input bytes + same path → same revision (deterministic)", () => {
    const md = "---\ntitle: x\n---\n# Same\n";
    const a = run(md, "p.md").node.revision;
    const b = run(md, "p.md").node.revision;
    expect(a).toBe(b);
  });

  it("body edit changes the revision", () => {
    const a = run("# Hello\n", "p.md").node.revision;
    const b = run("# Hello!\n", "p.md").node.revision;
    expect(a).not.toBe(b);
  });

  it("frontmatter edit changes the revision", () => {
    const a = run("---\ntitle: A\n---\n# x\n", "p.md").node.revision;
    const b = run("---\ntitle: B\n---\n# x\n", "p.md").node.revision;
    expect(a).not.toBe(b);
  });

  it("path move changes the revision (collectors care)", () => {
    const md = "# Same\n";
    const a = run(md, "a.md").node.revision;
    const b = run(md, "b.md").node.revision;
    expect(a).not.toBe(b);
  });
});

describe("parseMarkdown — config", () => {
  it("works with no config supplied", () => {
    const { node } = run("# x\n");
    expect(node.document.type).toBe("document");
  });

  it("accepts but ignores gfm:false in v0 (forward-compat)", () => {
    // Same input, different config → same document.  We don't yet
    // route the flag into gfm-parser; this test pins the surface so a
    // future change is a deliberate diff.
    const md = "# x\n";
    const withFlag = run(md, undefined, { gfm: false }).node;
    const withoutFlag = run(md).node;
    expect(withFlag.document).toEqual(withoutFlag.document);
  });
});
