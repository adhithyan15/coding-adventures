/**
 * stage.test.ts — emit-fs integration tests.
 *
 * Uses a real OS tmpdir for write tests with cleanup.  Pretty much
 * the whole stage is I/O — there's not much point mocking node:fs
 * when the actual code path is "open a tmp dir and write to it".
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdtemp, readFile, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createHash } from "node:crypto";
import { Kinds, streamOf, type RenderedPage } from "@coding-adventures/forme-types";
import {
  createCancellationTokenSource,
  inMemoryCache,
  inMemoryEventBus,
  noOpTelemetryEmitter,
  silentLogger,
  systemClock,
  frozenClock,
  deniedEnvApi,
  deniedFilesystemApi,
  deniedNetworkApi,
  deniedShellApi,
  deniedStorageApi,
  type StageContext,
} from "@coding-adventures/forme-stage";
import emitFs from "../src/index.js";

let outDir: string;

beforeEach(async () => {
  outDir = await mkdtemp(join(tmpdir(), "forme-emit-fs-"));
});

afterEach(async () => {
  if (outDir) await rm(outDir, { recursive: true, force: true });
});

function makeCtx(overrides: Partial<StageContext> = {}): StageContext {
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
    ...overrides,
  };
}

let pageSeq = 0;
function makePage(opts: { route: string; html?: string; title?: string }): RenderedPage {
  pageSeq++;
  const id = `00000000-0000-7000-8000-${String(pageSeq).padStart(12, "0")}` as RenderedPage["source"];
  return {
    route: opts.route,
    html: opts.html ?? `<!DOCTYPE html>\n<html><head><title>${opts.title ?? "x"}</title></head><body>${opts.title ?? "x"}</body></html>\n`,
    usedStyle: [],
    usedIslands: [],
    usedAssets: [],
    meta: {
      title: opts.title ?? "x",
      description: null,
      canonicalUrl: null,
      openGraph: {},
      structured: [],
      extra: {},
    },
    source: id,
  };
}

async function* fromArray<T>(items: readonly T[]): AsyncGenerator<T, void, void> {
  for (const item of items) yield item;
}

async function runEmit(pages: RenderedPage[], ctx: StageContext = makeCtx()) {
  const out = await emitFs.run(
    fromArray(pages) as never,
    { outDir } as never,
    ctx,
  );
  return out as unknown as {
    variant: { kind: string };
    files: Record<string, Uint8Array>;
    manifest: {
      routes: Array<{ pattern: string; target: { kind: string; path: string }; islands: readonly unknown[]; css: readonly unknown[] }>;
      assets: readonly unknown[];
      buildTime: string;
      buildId: string;
    };
  };
}

describe("emitFs — stage shape", () => {
  it("declares Stream<RenderedPage> in / DeployArtifact out", () => {
    expect(emitFs.consumes).toEqual(streamOf(Kinds.RenderedPage));
    expect(emitFs.produces).toEqual(Kinds.DeployArtifact);
  });

  it("declares filesystem:write capability", () => {
    expect(emitFs.capabilities).toContain("filesystem:write");
  });

  it("targets apiVersion 1", () => {
    expect(emitFs.apiVersion).toBe(1);
  });

  it("has a configSchema requiring outDir", () => {
    expect(emitFs.configSchema).toMatchObject({
      type: "object",
      required: ["outDir"],
      properties: { outDir: { type: "string" } },
    });
  });
});

describe("emitFs — running", () => {
  it("writes one file per page with the expected bytes", async () => {
    const pages = [
      makePage({ route: "/a.html", html: "<p>a</p>" }),
      makePage({ route: "/b.html", html: "<p>b</p>" }),
    ];
    await runEmit(pages);
    const a = await readFile(resolve(outDir, "a.html"), "utf8");
    const b = await readFile(resolve(outDir, "b.html"), "utf8");
    expect(a).toBe("<p>a</p>");
    expect(b).toBe("<p>b</p>");
  });

  it("creates nested directories for nested routes", async () => {
    const pages = [
      makePage({ route: "/blog/2026/05/hello.html", html: "<p>nested</p>" }),
    ];
    await runEmit(pages);
    const s = await stat(resolve(outDir, "blog/2026/05/hello.html"));
    expect(s.isFile()).toBe(true);
  });

  it("emits an empty dist-tree DeployArtifact when no pages arrive", async () => {
    const out = await runEmit([]);
    expect(out.variant).toEqual({ kind: "dist-tree" });
    expect(Object.keys(out.files)).toEqual([]);
    expect(out.manifest.routes).toEqual([]);
  });

  it("emits one DeployRoute per written page (preserving stream order)", async () => {
    const pages = [
      makePage({ route: "/x.html" }),
      makePage({ route: "/y.html" }),
      makePage({ route: "/z.html" }),
    ];
    const out = await runEmit(pages);
    expect(out.manifest.routes.map((r) => r.pattern)).toEqual([
      "/x.html", "/y.html", "/z.html",
    ]);
    for (const r of out.manifest.routes) {
      expect(r.target.kind).toBe("file");
      expect(r.islands).toEqual([]);
      expect(r.css).toEqual([]);
    }
  });

  it("files map keys use POSIX separators", async () => {
    const pages = [makePage({ route: "/blog/2026/p.html", html: "<p>x</p>" })];
    const out = await runEmit(pages);
    const keys = Object.keys(out.files);
    expect(keys).toEqual(["blog/2026/p.html"]);
  });

  it("files map values are the written bytes", async () => {
    const html = "<p>hello-world</p>";
    const pages = [makePage({ route: "/h.html", html })];
    const out = await runEmit(pages);
    const bytes = out.files["h.html"];
    expect(bytes).toBeInstanceOf(Uint8Array);
    expect(new TextDecoder().decode(bytes!)).toBe(html);
  });

  it("buildTime is a valid ISO-8601 string", async () => {
    const out = await runEmit([makePage({ route: "/a.html" })]);
    expect(out.manifest.buildTime).toMatch(/^\d{4}-\d{2}-\d{2}T/);
    expect(() => new Date(out.manifest.buildTime).toISOString()).not.toThrow();
  });

  it("buildTime honours the frozen clock for reproducible builds", async () => {
    const fixed = "2026-05-15T00:00:00.000Z";
    const ctx = makeCtx({ time: frozenClock({ timestamp: Date.parse(fixed) }) });
    const out = await runEmit([makePage({ route: "/a.html" })], ctx);
    expect(out.manifest.buildTime).toBe(fixed);
  });

  it("buildId is a blake2b RevisionId", async () => {
    const out = await runEmit([makePage({ route: "/a.html" })]);
    expect(out.manifest.buildId).toMatch(/^blake2b:[0-9a-f]+$/);
  });

  it("buildId is deterministic across runs with the same input bytes", async () => {
    const html = "<p>stable</p>";
    const a = (await runEmit([makePage({ route: "/p.html", html })])).manifest.buildId;
    // Reset tmpdir between runs (afterEach cleans up before next beforeEach)
    // but use a fresh stage call — bytes are identical → buildId identical.
    await rm(outDir, { recursive: true, force: true });
    outDir = await mkdtemp(join(tmpdir(), "forme-emit-fs-"));
    const b = (await runEmit([makePage({ route: "/p.html", html })])).manifest.buildId;
    expect(a).toBe(b);
  });

  it("buildId changes when content changes", async () => {
    const a = (await runEmit([makePage({ route: "/p.html", html: "<p>A</p>" })])).manifest.buildId;
    await rm(outDir, { recursive: true, force: true });
    outDir = await mkdtemp(join(tmpdir(), "forme-emit-fs-"));
    const b = (await runEmit([makePage({ route: "/p.html", html: "<p>B</p>" })])).manifest.buildId;
    expect(a).not.toBe(b);
  });

  it("assets array is empty in v0", async () => {
    const out = await runEmit([makePage({ route: "/a.html" })]);
    expect(out.manifest.assets).toEqual([]);
  });
});

describe("emitFs — config validation", () => {
  it("rejects empty outDir", async () => {
    const ctx = makeCtx();
    await expect(
      emitFs.run(
        fromArray([makePage({ route: "/a.html" })]) as never,
        { outDir: "" } as never,
        ctx,
      ),
    ).rejects.toThrow(/non-empty string/);
  });

  it("rejects missing outDir", async () => {
    const ctx = makeCtx();
    await expect(
      emitFs.run(
        fromArray([makePage({ route: "/a.html" })]) as never,
        {} as never,
        ctx,
      ),
    ).rejects.toThrow(/non-empty string/);
  });
});

describe("emitFs — cancellation", () => {
  it("throws when cancellation is requested mid-stream", async () => {
    const cs = createCancellationTokenSource();
    const ctx = makeCtx({ cancellation: cs.token });
    cs.cancel("test");
    await expect(
      runEmit([makePage({ route: "/a.html" })], ctx),
    ).rejects.toThrow();
  });
});

describe("emitFs — sha256 helper", () => {
  it("the helper produces a stable hex digest matching node:crypto", async () => {
    const { sha256Hex } = await import("../src/index.js");
    const bytes = new TextEncoder().encode("hello");
    const expected = createHash("sha256").update(bytes).digest("hex");
    expect(sha256Hex(bytes)).toBe(expected);
  });
});

describe("emitFs — safety", () => {
  it("rejects a route that tries to traverse out of outDir", async () => {
    await expect(
      runEmit([makePage({ route: "/../../escape.html" })]),
    ).rejects.toThrow(/escape outDir/);
  });
});
