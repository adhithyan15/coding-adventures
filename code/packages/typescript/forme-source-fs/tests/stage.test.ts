/**
 * forme-source-fs — stage integration tests
 *
 * Verifies the stage value-shape (defineStage produced the right
 * descriptor) and runs the stage against a real temp directory.
 */

import { afterEach, beforeEach, describe, it, expect } from "vitest";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Kinds, streamOf } from "@coding-adventures/forme-types";
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
import sourceFs from "../src/index.js";

let root: string;

beforeEach(async () => {
  root = await mkdtemp(join(tmpdir(), "forme-source-fs-stage-"));
});

afterEach(async () => {
  await rm(root, { recursive: true, force: true });
});

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

async function runAndCollect(config: { glob: string; root: string }) {
  const ctx = makeCtx();
  const out: unknown[] = [];
  const iter = sourceFs.run(undefined as never, config, ctx) as AsyncIterable<unknown>;
  for await (const v of iter) out.push(v);
  return out;
}

describe("sourceFs — stage shape", () => {
  it("declares Void in / streamOf(ContentSource) out", () => {
    expect(sourceFs.consumes).toEqual(Kinds.Void);
    expect(sourceFs.produces).toEqual(streamOf(Kinds.ContentSource));
  });

  it("declares storage:read capability", () => {
    expect(sourceFs.capabilities).toContain("storage:read");
  });

  it("apiVersion targets the kernel", () => {
    expect(sourceFs.apiVersion).toBe(1);
  });

  it("publishes external filesystem state", () => {
    expect(sourceFs.externalState).toBeTypeOf("function");
  });
});

describe("sourceFs — running", () => {
  it("emits a ContentSource per matching file", async () => {
    await writeFile(join(root, "a.md"), "hello");
    await writeFile(join(root, "b.md"), "world");
    await writeFile(join(root, "ignored.txt"), "x");
    const out = await runAndCollect({ glob: "**/*.md", root });
    expect(out.length).toBe(2);
    const sources = out as Array<{
      path: string; bytes: Uint8Array; mimeType: string | null;
      identity: string; revision: string; providerMeta: Record<string, unknown>;
    }>;
    expect(sources[0]!.path).toMatch(/\.md$/);
    expect(sources[0]!.mimeType).toBe("text/markdown");
    expect(sources[0]!.bytes).toBeInstanceOf(Uint8Array);
    expect(sources[0]!.identity).toMatch(/^[0-9a-f]{8}-/);
    expect(sources[0]!.revision).toMatch(/^blake2b:/);
    expect(typeof sources[0]!.providerMeta.mtimeMs).toBe("number");
    expect(typeof sources[0]!.providerMeta.sizeBytes).toBe("number");
  });

  it("paths are relative to root", async () => {
    await mkdir(join(root, "posts"), { recursive: true });
    await writeFile(join(root, "posts", "hello.md"), "x");
    const out = await runAndCollect({ glob: "**/*.md", root }) as Array<{ path: string }>;
    expect(out[0]!.path).toBe(join("posts", "hello.md"));
  });

  it("revision differs when file content differs", async () => {
    await writeFile(join(root, "a.md"), "hello");
    await writeFile(join(root, "b.md"), "world");
    const out = await runAndCollect({ glob: "**/*.md", root }) as Array<{ revision: string }>;
    expect(out[0]!.revision).not.toBe(out[1]!.revision);
  });

  it("revision stable for same content across two runs", async () => {
    await writeFile(join(root, "a.md"), "stable");
    const a = (await runAndCollect({ glob: "**/*.md", root })) as Array<{ revision: string }>;
    const b = (await runAndCollect({ glob: "**/*.md", root })) as Array<{ revision: string }>;
    expect(a[0]!.revision).toBe(b[0]!.revision);
  });

  it("identity differs across files (UUIDv7 unique)", async () => {
    await writeFile(join(root, "a.md"), "x");
    await writeFile(join(root, "b.md"), "x");
    const out = await runAndCollect({ glob: "**/*.md", root }) as Array<{ identity: string }>;
    expect(out[0]!.identity).not.toBe(out[1]!.identity);
  });

  it("emits nothing when no matching files", async () => {
    await writeFile(join(root, "a.txt"), "x");
    const out = await runAndCollect({ glob: "**/*.md", root });
    expect(out).toEqual([]);
  });

  it("rejects empty glob", async () => {
    await expect(
      runAndCollect({ glob: "", root }),
    ).rejects.toThrow(/non-empty/);
  });

  it("rejects unsupported glob patterns (delegates to walker)", async () => {
    await expect(
      runAndCollect({ glob: "*.md" as never, root }),
    ).rejects.toThrow(/not supported/);
  });

  it("default root falls back to process.cwd()", async () => {
    // chdir to the empty temp root so the default-cwd path is
    // exercised against known-empty content (no surprise matches
    // from whatever directory the tests happened to run from).
    const original = process.cwd();
    try {
      process.chdir(root);
      const ctx = makeCtx();
      const iter = sourceFs.run(undefined as never, { glob: "**/*.md" } as never, ctx) as AsyncIterable<unknown>;
      const collected: unknown[] = [];
      for await (const v of iter) collected.push(v);
      expect(collected).toEqual([]);
    } finally {
      process.chdir(original);
    }
  });

  it("honours cancellation mid-stream", async () => {
    for (let i = 0; i < 5; i++) {
      await writeFile(join(root, `${i}.md`), "x");
    }
    const cs = createCancellationTokenSource();
    const ctx = { ...makeCtx(), cancellation: cs.token };
    cs.cancel("test");
    const iter = sourceFs.run(undefined as never, { glob: "**/*.md", root }, ctx) as AsyncIterable<unknown>;
    await expect((async () => {
      for await (const _ of iter) { void _; }
    })()).rejects.toThrow();
  });

  it("publishes a sorted deterministic manifest and runs from the same snapshot", async () => {
    await mkdir(join(root, "a"), { recursive: true });
    await writeFile(join(root, "z.md"), "z-before");
    await writeFile(join(root, "a", "a.md"), "a-before");
    const config = { glob: "**/*.md", root };
    const ctx = makeCtx();
    const manifest = await sourceFs.externalState!(config, ctx);

    expect(manifest.version).toBe(1);
    expect(manifest.revision).toMatch(/^blake2b:[0-9a-f]{64}$/);
    expect(manifest.entries.map(entry => entry.locator)).toEqual([
      "a/a.md",
      "z.md",
    ]);

    // The hook and run share ctx.cache, so a source edit between the two does
    // not mix one observation with a different emitted snapshot.
    await writeFile(join(root, "z.md"), "z-after");
    const emitted: Array<{ path: string; revision: string; bytes: Uint8Array }> = [];
    for await (const source of sourceFs.run(undefined as never, config, ctx) as AsyncIterable<typeof emitted[number]>) {
      emitted.push(source);
    }
    const z = emitted.find(source => source.path === "z.md")!;
    expect(new TextDecoder().decode(z.bytes)).toBe("z-before");
    expect(z.revision).toBe(manifest.entries.find(entry => entry.locator === "z.md")!.revision);

    const changed = await sourceFs.externalState!(config, makeCtx());
    expect(changed.revision).not.toBe(manifest.revision);
  });
});
