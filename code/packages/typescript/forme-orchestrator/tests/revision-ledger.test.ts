import { afterEach, describe, expect, it, vi } from "vitest";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { filesystemCache } from "@coding-adventures/forme-cache";
import { computeBinaryRevisionId, computeRevisionId } from "@coding-adventures/forme-identity";
import type { PipelineConfig } from "@coding-adventures/forme-pipeline-config";
import {
  defineStage,
  silentLogger,
  type StageContext,
} from "@coding-adventures/forme-stage";
import { KERNEL_API_VERSION, Kinds, streamOf } from "@coding-adventures/forme-types";
import { createOrchestrator } from "../src/index.js";

const roots: string[] = [];
afterEach(async () => {
  await Promise.all(roots.splice(0).map(root => rm(root, { recursive: true, force: true })));
});

interface MutableSourceValue {
  readonly path: string;
  readonly text: string;
}

const ID = "01952c0d-7e63-7000-8000-000000000001";

function observedSource(read: () => MutableSourceValue, runCalls = vi.fn()) {
  const snapshot = async (ctx: StageContext) =>
    ctx.cache.getOrCompute("snapshot", async () => {
      const value = read();
      const bytes = new TextEncoder().encode(value.text);
      const revision = computeBinaryRevisionId(bytes);
      return {
        value: {
          path: value.path,
          bytes,
          mimeType: "text/plain",
          identity: ID,
          revision,
          providerMeta: {},
        },
        manifest: {
          version: 1 as const,
          entries: [{ locator: value.path, identity: ID, revision }],
          revision: computeRevisionId({
            version: 1,
            entries: [{ locator: value.path, identity: ID, revision }],
          }),
        },
      };
    });

  return defineStage({
    name: "@test/observed-source",
    version: "1.0.0",
    apiVersion: KERNEL_API_VERSION,
    description: "source with deterministic external observation",
    consumes: Kinds.Void,
    produces: streamOf(Kinds.ContentSource),
    capabilities: [],
    configSchema: null,
    async externalState(_config, ctx) {
      return (await snapshot(ctx)).manifest;
    },
    async *run(_input, _config, ctx) {
      runCalls();
      yield (await snapshot(ctx)).value as never;
    },
  });
}

function transform() {
  return defineStage({
    name: "@test/revision-transform",
    version: "1.0.0",
    apiVersion: KERNEL_API_VERSION,
    description: "identity transform",
    consumes: Kinds.ContentSource,
    produces: Kinds.ContentSource,
    capabilities: [],
    configSchema: null,
    run(input) { return input; },
  });
}

function pipelineConfig(source: ReturnType<typeof observedSource>): PipelineConfig {
  return {
    name: "revision-ledger-test",
    settings: {
      storageRoot: "./",
      cacheDir: ".forme-cache",
      reproducibleBuild: true,
      maxConcurrency: null,
      logLevel: "error",
      bestEffort: false,
      deadlineMs: null,
    },
    stages: [
      { id: "source", stage: source },
      { id: "transform", stage: transform() },
    ],
  };
}

describe("external source state and persistent revision ledger", () => {
  it("compares revisions across fresh orchestrators and derives buildId from source state", async () => {
    const cacheRoot = await mkdtemp(join(tmpdir(), "forme-revision-ledger-"));
    roots.push(cacheRoot);
    let current = { path: "post.md", text: "first" };
    const runCalls = vi.fn();
    const source = observedSource(() => current, runCalls);
    const config = pipelineConfig(source);

    const runFresh = async () => {
      const orchestrator = createOrchestrator({
        cache: filesystemCache(cacheRoot),
        logger: silentLogger(),
      });
      const result = await orchestrator.runOnce(await orchestrator.buildPipeline(config));
      await orchestrator.dispose();
      return result;
    };

    const first = await runFresh();
    expect(first.stages.map(stage => stage.inputChanged)).toEqual([null, null]);
    expect(first.stages[0]).toMatchObject({
      externalStateRevision: expect.stringMatching(/^blake2b:/),
      inputRevision: expect.stringMatching(/^blake2b:/),
      outputRevision: expect.stringMatching(/^blake2b:/),
    });

    const second = await runFresh();
    expect(second.buildId).toBe(first.buildId);
    expect(second.stages.map(stage => stage.inputChanged)).toEqual([false, false]);
    expect(second.stages[1]).toMatchObject({ cacheHits: 1, cacheMisses: 0 });

    current = { path: "post.md", text: "second" };
    const third = await runFresh();
    expect(third.buildId).not.toBe(first.buildId);
    expect(third.stages.map(stage => stage.inputChanged)).toEqual([true, true]);
    expect(runCalls).toHaveBeenCalledTimes(3);
  });

  it("rejects a source manifest whose digest does not match its entries", async () => {
    const runCalls = vi.fn();
    const bad = defineStage({
      name: "@test/bad-observed-source",
      version: "1.0.0",
      apiVersion: KERNEL_API_VERSION,
      description: "publishes a dishonest manifest",
      consumes: Kinds.Void,
      produces: streamOf(Kinds.ContentSource),
      capabilities: [],
      configSchema: null,
      async externalState() {
        return {
          version: 1,
          revision: computeRevisionId({ wrong: true }),
          entries: [{
            locator: "post.md",
            identity: ID,
            revision: computeRevisionId({ text: "post" }),
          }],
        };
      },
      async *run() { runCalls(); },
    });
    const orchestrator = createOrchestrator({ logger: silentLogger() });
    const result = await orchestrator.runOnce(await orchestrator.buildPipeline(pipelineConfig(bad as never)));
    expect(result.outcome).toBe("failed");
    expect(result.errors[0]?.message).toContain("revision does not match canonical entries");
    expect(runCalls).not.toHaveBeenCalled();
    await orchestrator.dispose();
  });
});
