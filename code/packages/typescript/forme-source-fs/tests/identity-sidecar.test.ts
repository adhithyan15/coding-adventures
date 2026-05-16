/**
 * identity-sidecar.test.ts — exercise the read-side identity
 * persistence (FM01 §7.2 sidecar reading) added in v0.2.0.
 *
 * Strategy: stage a tempdir with various sidecar configurations
 * (present / missing / malformed / wrong-shape / wrong-uuid-version)
 * and verify the stage produces the expected LogicalId in each case.
 */

import { afterEach, beforeEach, describe, it, expect } from "vitest";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
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
  root = await mkdtemp(join(tmpdir(), "forme-source-fs-id-"));
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

async function runAndCollect(
  config: { glob: string; root: string; persistIdentities?: boolean },
): Promise<Array<{ path: string; identity: string }>> {
  const ctx = makeCtx();
  const out: Array<{ path: string; identity: string }> = [];
  const iter = sourceFs.run(undefined as never, config, ctx) as AsyncIterable<{ path: string; identity: string }>;
  for await (const v of iter) out.push(v);
  return out;
}

/** A valid UUIDv7 (version nibble `7`, variant nibble in `[89ab]`). */
const STABLE_ID_1 = "01952c0d-7e63-7000-8000-000000000001";
const STABLE_ID_2 = "01952c0d-7e63-7000-8000-000000000002";

describe("sourceFs — identity sidecar (read side)", () => {
  it("reads a valid sidecar and uses the persisted LogicalId", async () => {
    await writeFile(join(root, "hello.md"), "body");
    await writeFile(
      join(root, ".hello.md.id.json"),
      JSON.stringify({ logicalId: STABLE_ID_1 }),
    );
    const out = await runAndCollect({ glob: "**/*.md", root });
    expect(out).toHaveLength(1);
    expect(out[0]!.identity).toBe(STABLE_ID_1);
  });

  it("produces the same identity across two runs when sidecar is present", async () => {
    await writeFile(join(root, "hello.md"), "body");
    await writeFile(
      join(root, ".hello.md.id.json"),
      JSON.stringify({ logicalId: STABLE_ID_1 }),
    );
    const a = await runAndCollect({ glob: "**/*.md", root });
    const b = await runAndCollect({ glob: "**/*.md", root });
    expect(a[0]!.identity).toBe(b[0]!.identity);
  });

  it("produces DIFFERENT identities across two runs when sidecar is missing", async () => {
    await writeFile(join(root, "hello.md"), "body");
    const a = await runAndCollect({ glob: "**/*.md", root });
    const b = await runAndCollect({ glob: "**/*.md", root });
    expect(a[0]!.identity).not.toBe(b[0]!.identity);
  });

  it("multiple files with distinct sidecars get their own identities", async () => {
    await writeFile(join(root, "a.md"), "A");
    await writeFile(join(root, "b.md"), "B");
    await writeFile(join(root, ".a.md.id.json"), JSON.stringify({ logicalId: STABLE_ID_1 }));
    await writeFile(join(root, ".b.md.id.json"), JSON.stringify({ logicalId: STABLE_ID_2 }));
    const out = await runAndCollect({ glob: "**/*.md", root });
    const byPath = new Map(out.map((s) => [s.path, s.identity]));
    expect(byPath.get("a.md")).toBe(STABLE_ID_1);
    expect(byPath.get("b.md")).toBe(STABLE_ID_2);
  });

  it("falls back to fresh id when sidecar contains malformed JSON", async () => {
    await writeFile(join(root, "hello.md"), "body");
    await writeFile(join(root, ".hello.md.id.json"), "not-json{");
    const out = await runAndCollect({ glob: "**/*.md", root });
    expect(out).toHaveLength(1);
    // Some fresh UUIDv7.
    expect(out[0]!.identity).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
    expect(out[0]!.identity).not.toBe(STABLE_ID_1);
  });

  it("falls back to fresh id when sidecar is not an object", async () => {
    await writeFile(join(root, "hello.md"), "body");
    await writeFile(join(root, ".hello.md.id.json"), JSON.stringify("just-a-string"));
    const out = await runAndCollect({ glob: "**/*.md", root });
    expect(out[0]!.identity).not.toBe("just-a-string");
  });

  it("falls back when logicalId field is missing", async () => {
    await writeFile(join(root, "hello.md"), "body");
    await writeFile(join(root, ".hello.md.id.json"), JSON.stringify({ note: "no id" }));
    const out = await runAndCollect({ glob: "**/*.md", root });
    expect(out[0]!.identity).not.toBe("");
    expect(out[0]!.identity).toMatch(/^[0-9a-f]{8}-/);
  });

  it("falls back when logicalId is wrong shape (v4 instead of v7)", async () => {
    // UUIDv4 has version nibble 4; we require 7.
    const v4 = "01952c0d-7e63-4000-8000-000000000001";
    await writeFile(join(root, "hello.md"), "body");
    await writeFile(join(root, ".hello.md.id.json"), JSON.stringify({ logicalId: v4 }));
    const out = await runAndCollect({ glob: "**/*.md", root });
    expect(out[0]!.identity).not.toBe(v4);
  });

  it("falls back when logicalId is a number, not a string", async () => {
    await writeFile(join(root, "hello.md"), "body");
    await writeFile(join(root, ".hello.md.id.json"), JSON.stringify({ logicalId: 42 }));
    const out = await runAndCollect({ glob: "**/*.md", root });
    expect(out[0]!.identity).toMatch(/^[0-9a-f]{8}-/);
  });

  it("ignores unknown sidecar fields (forward-compat)", async () => {
    await writeFile(join(root, "hello.md"), "body");
    await writeFile(
      join(root, ".hello.md.id.json"),
      JSON.stringify({
        logicalId: STABLE_ID_1,
        createdAt: "2026-05-16T00:00:00Z",
        note: "human-readable",
        someFutureField: { nested: true },
      }),
    );
    const out = await runAndCollect({ glob: "**/*.md", root });
    expect(out[0]!.identity).toBe(STABLE_ID_1);
  });

  it("persistIdentities=false → always generates fresh, ignoring sidecar", async () => {
    await writeFile(join(root, "hello.md"), "body");
    await writeFile(join(root, ".hello.md.id.json"), JSON.stringify({ logicalId: STABLE_ID_1 }));
    const out = await runAndCollect({ glob: "**/*.md", root, persistIdentities: false });
    expect(out[0]!.identity).not.toBe(STABLE_ID_1);
  });

  it("sidecar files themselves are NOT emitted as content sources", async () => {
    // Sidecars start with a dot — the walker should skip them as
    // hidden files.  Verify this stays true.
    await writeFile(join(root, "hello.md"), "body");
    await writeFile(join(root, ".hello.md.id.json"), JSON.stringify({ logicalId: STABLE_ID_1 }));
    const out = await runAndCollect({ glob: "**/*.md", root });
    expect(out).toHaveLength(1);
    expect(out[0]!.path).toBe("hello.md");
  });

  it("empty sidecar file → fresh id (treated as missing)", async () => {
    await writeFile(join(root, "hello.md"), "body");
    await writeFile(join(root, ".hello.md.id.json"), "");
    const out = await runAndCollect({ glob: "**/*.md", root });
    expect(out[0]!.identity).toMatch(/^[0-9a-f]{8}-/);
  });
});
