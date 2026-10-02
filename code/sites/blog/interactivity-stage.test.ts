import { describe, expect, it } from "vitest";
import type { ContentNode, LogicalId, RevisionId } from "@coding-adventures/forme-types";
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
} from "@coding-adventures/forme-stage";
import attachBlogInteractivity, {
  PIPELINE_ISLAND_ASSET_ID,
  PIPELINE_ISLAND_SOURCE_PATH,
} from "./interactivity-stage.ts";

const CONTENT_ID = "01952c0d-7e63-7000-8000-000000000203" as LogicalId;
const REVISION = `blake2b:${"1".repeat(64)}` as RevisionId;

function node(sourcePath: string, assetRefs: ContentNode["assetRefs"] = []): ContentNode {
  return {
    identity: CONTENT_ID,
    revision: REVISION,
    document: { type: "document", children: [] },
    frontmatter: {},
    route: null,
    assetRefs,
    sourcePath,
  };
}

async function collect(...nodes: ContentNode[]): Promise<ContentNode[]> {
  async function* input() {
    yield* nodes;
  }
  const output = attachBlogInteractivity.run(
    input() as never,
    {} as never,
    context(),
  ) as AsyncIterable<ContentNode>;
  const result: ContentNode[] = [];
  for await (const item of output) result.push(item);
  return result;
}

function context() {
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

describe("blog interactivity asset composition", () => {
  it("attaches the reviewed script only to the Hello, Forme source", async () => {
    const target = node("2026-05-15-hello-forme.md");
    const staticPage = node("2026-05-12-why-forme.md");
    const [enhanced, unchanged] = await collect(target, staticPage);

    expect(enhanced?.assetRefs).toEqual([{
      id: PIPELINE_ISLAND_ASSET_ID,
      nodePath: [],
      role: "script",
      sourcePath: PIPELINE_ISLAND_SOURCE_PATH,
    }]);
    expect(enhanced?.revision).not.toBe(REVISION);
    expect(unchanged).toBe(staticPage);
  });

  it("is idempotent for the exact reviewed claim and rejects conflicting claims", async () => {
    const exact = {
      id: PIPELINE_ISLAND_ASSET_ID,
      nodePath: [],
      role: "script" as const,
      sourcePath: PIPELINE_ISLAND_SOURCE_PATH,
    };
    expect((await collect(node("2026-05-15-hello-forme.md", [exact])))[0]?.assetRefs)
      .toEqual([exact]);

    await expect(collect(node("2026-05-15-hello-forme.md", [{
      ...exact,
      sourcePath: "assets/other.js",
    }]))).rejects.toThrow(/conflicting script asset claim/);

    await expect(collect(node("2026-05-15-hello-forme.md", [exact, exact])))
      .rejects.toThrow(/conflicting script asset claim/);
  });
});
