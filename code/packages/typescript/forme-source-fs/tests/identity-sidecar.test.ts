/** Exercise FM01 §7.2 filesystem identity persistence. */

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { mkdtemp, readFile, rename, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  createCancellationTokenSource,
  deniedEnvApi,
  deniedFilesystemApi,
  deniedNetworkApi,
  deniedShellApi,
  deniedStorageApi,
  inMemoryCache,
  inMemoryEventBus,
  noOpTelemetryEmitter,
  silentLogger,
  systemClock,
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

interface SourceResult {
  readonly path: string;
  readonly identity: string;
  readonly revision: string;
}

async function runAndCollect(
  config: { glob: string; root: string; persistIdentities?: boolean },
): Promise<SourceResult[]> {
  const out: SourceResult[] = [];
  const iter = sourceFs.run(undefined as never, config, makeCtx()) as AsyncIterable<SourceResult>;
  for await (const value of iter) out.push(value);
  return out;
}

const STABLE_ID_1 = "01952c0d-7e63-7000-8000-000000000001";
const STABLE_ID_2 = "01952c0d-7e63-7000-8000-000000000002";

describe("sourceFs identity sidecars", () => {
  it("reads a valid sidecar and ignores forward-compatible fields", async () => {
    await writeFile(join(root, "hello.md"), "body");
    await writeFile(join(root, ".hello.md.id.json"), JSON.stringify({
      logicalId: STABLE_ID_1,
      createdAt: "2026-05-16T00:00:00Z",
      someFutureField: { nested: true },
    }));
    const out = await runAndCollect({ glob: "**/*.md", root });
    expect(out).toHaveLength(1);
    expect(out[0]!.identity).toBe(STABLE_ID_1);
  });

  it("creates a missing sidecar and reuses its identity across runs", async () => {
    await writeFile(join(root, "hello.md"), "body");
    const first = await runAndCollect({ glob: "**/*.md", root });
    const second = await runAndCollect({ glob: "**/*.md", root });
    const sidecar = JSON.parse(await readFile(join(root, ".hello.md.id.json"), "utf-8"));
    expect(first[0]!.identity).toBe(second[0]!.identity);
    expect(first[0]!.identity).toBe(sidecar.logicalId);
  });

  it("concurrent first reads converge on the exclusively created identity", async () => {
    await writeFile(join(root, "hello.md"), "body");
    const [first, second] = await Promise.all([
      runAndCollect({ glob: "**/*.md", root }),
      runAndCollect({ glob: "**/*.md", root }),
    ]);
    const sidecar = JSON.parse(await readFile(join(root, ".hello.md.id.json"), "utf-8"));
    expect(first[0]!.identity).toBe(second[0]!.identity);
    expect(first[0]!.identity).toBe(sidecar.logicalId);
  });

  it("resolves distinct sidecars per file", async () => {
    await writeFile(join(root, "a.md"), "A");
    await writeFile(join(root, "b.md"), "B");
    await writeFile(join(root, ".a.md.id.json"), JSON.stringify({ logicalId: STABLE_ID_1 }));
    await writeFile(join(root, ".b.md.id.json"), JSON.stringify({ logicalId: STABLE_ID_2 }));
    const out = await runAndCollect({ glob: "**/*.md", root });
    const byPath = new Map(out.map(source => [source.path, source.identity]));
    expect(byPath.get("a.md")).toBe(STABLE_ID_1);
    expect(byPath.get("b.md")).toBe(STABLE_ID_2);
  });

  it.each([
    ["invalid JSON", "not-json{", "contains invalid JSON"],
    ["empty JSON", "", "contains invalid JSON"],
    ["non-object JSON", JSON.stringify("text"), "is not a JSON object"],
    ["missing logicalId", JSON.stringify({ note: "no id" }), "missing/malformed logicalId"],
    ["non-string logicalId", JSON.stringify({ logicalId: 42 }), "missing/malformed logicalId"],
    [
      "non-v7 logicalId",
      JSON.stringify({ logicalId: "01952c0d-7e63-4000-8000-000000000001" }),
      "missing/malformed logicalId",
    ],
  ])("rejects %s without overwriting it", async (_label, text, diagnostic) => {
    await writeFile(join(root, "hello.md"), "body");
    const sidecarPath = join(root, ".hello.md.id.json");
    await writeFile(sidecarPath, text);
    await expect(runAndCollect({ glob: "**/*.md", root })).rejects.toThrow(diagnostic);
    expect(await readFile(sidecarPath, "utf-8")).toBe(text);
  });

  it("persistIdentities=false neither reads nor writes sidecars", async () => {
    await writeFile(join(root, "hello.md"), "body");
    await writeFile(join(root, ".hello.md.id.json"), "malformed");
    const first = await runAndCollect({ glob: "**/*.md", root, persistIdentities: false });
    const second = await runAndCollect({ glob: "**/*.md", root, persistIdentities: false });
    expect(first[0]!.identity).not.toBe(second[0]!.identity);
    expect(await readFile(join(root, ".hello.md.id.json"), "utf-8")).toBe("malformed");
  });

  it("does not emit hidden identity sidecars as content", async () => {
    await writeFile(join(root, "hello.md"), "body");
    await writeFile(join(root, ".hello.md.id.json"), JSON.stringify({ logicalId: STABLE_ID_1 }));
    const out = await runAndCollect({ glob: "**/*.md", root });
    expect(out.map(source => source.path)).toEqual(["hello.md"]);
  });

  it("preserves identity and revision when content and sidecar move together", async () => {
    await writeFile(join(root, "before.md"), "same bytes");
    const before = await runAndCollect({ glob: "**/*.md", root });
    await rename(join(root, "before.md"), join(root, "after.md"));
    await rename(join(root, ".before.md.id.json"), join(root, ".after.md.id.json"));
    const after = await runAndCollect({ glob: "**/*.md", root });
    expect(after[0]!.identity).toBe(before[0]!.identity);
    expect(after[0]!.revision).toBe(before[0]!.revision);
  });

  it("preserves identity but changes revision after an edit", async () => {
    await writeFile(join(root, "hello.md"), "before");
    const before = await runAndCollect({ glob: "**/*.md", root });
    await writeFile(join(root, "hello.md"), "after");
    const after = await runAndCollect({ glob: "**/*.md", root });
    expect(after[0]!.identity).toBe(before[0]!.identity);
    expect(after[0]!.revision).not.toBe(before[0]!.revision);
  });
});
