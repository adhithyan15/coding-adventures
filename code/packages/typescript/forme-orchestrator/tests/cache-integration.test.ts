import { describe, expect, it, vi } from "vitest";
import { makeEntry, memoryCache, type CacheBackend } from "@coding-adventures/forme-cache";
import { KERNEL_API_VERSION, Kinds, streamOf } from "@coding-adventures/forme-types";
import { defineStage, silentLogger } from "@coding-adventures/forme-stage";
import { createOrchestrator } from "../src/index.js";
import type { PipelineConfig } from "@coding-adventures/forme-pipeline-config";

function config(stages: PipelineConfig["stages"]): PipelineConfig {
  return {
    name: "cache-test",
    settings: {
      storageRoot: "./",
      cacheDir: null,
      reproducibleBuild: true,
      maxConcurrency: null,
      logLevel: "error",
      bestEffort: false,
      deadlineMs: null,
    },
    stages,
  };
}

function source(read: () => readonly string[], calls: ReturnType<typeof vi.fn>) {
  return defineStage({
    name: "@test/cache-source",
    version: "1.0.0",
    apiVersion: KERNEL_API_VERSION,
    description: "mutable test source",
    consumes: Kinds.Void,
    produces: streamOf(Kinds.ContentSource),
    capabilities: [],
    configSchema: null,
    async *run() {
      calls();
      for (const path of read()) {
        yield {
          path,
          bytes: new TextEncoder().encode(path),
          mimeType: "text/plain",
          identity: "01952c0d-7e63-7000-8000-000000000000",
          revision: "blake2b:00",
          providerMeta: {},
        } as never;
      }
    },
  });
}

function transform(
  calls: ReturnType<typeof vi.fn>,
  capabilities: readonly string[] = [],
  version = "1.0.0",
) {
  return defineStage({
    name: capabilities.length === 0 ? "@test/cache-transform" : "@test/capability-transform",
    version,
    apiVersion: KERNEL_API_VERSION,
    description: "uppercases a source path",
    consumes: Kinds.ContentSource,
    produces: Kinds.ContentSource,
    capabilities: capabilities as never,
    configSchema: null,
    async run(input) {
      calls();
      return { ...(input as object), path: (input as { path: string }).path.toUpperCase() } as never;
    },
  });
}

function sink(calls: ReturnType<typeof vi.fn>) {
  return defineStage({
    name: "@test/cache-sink",
    version: "1.0.0",
    apiVersion: KERNEL_API_VERSION,
    description: "returns bytes in a deploy artifact",
    consumes: Kinds.ContentSource,
    produces: Kinds.DeployArtifact,
    capabilities: [],
    configSchema: null,
    async run(input) {
      calls();
      const path = (input as { path: string }).path;
      return {
        variant: { kind: "dist-tree" },
        files: { [path]: new TextEncoder().encode(path) },
        manifest: { routes: [], assets: [], buildTime: "fixed", buildId: "blake2b:00" },
      } as never;
    },
  });
}

describe("scheduler cache reuse", () => {
  it("reruns sources while reusing unchanged pure invocations per item", async () => {
    let items = ["a.md", "b.md"];
    const sourceCalls = vi.fn();
    const transformCalls = vi.fn();
    const sinkCalls = vi.fn();
    const cache = memoryCache();
    const orchestrator = createOrchestrator({ cache, logger: silentLogger() });
    const pipeline = await orchestrator.buildPipeline(config([
      { stage: source(() => items, sourceCalls) },
      { stage: transform(transformCalls) },
      { stage: sink(sinkCalls) },
    ]));

    const first = await orchestrator.runOnce(pipeline);
    expect(first.stages.map(stage => [stage.cacheHits, stage.cacheMisses])).toEqual([
      [0, 0], [0, 2], [0, 2],
    ]);
    const second = await orchestrator.runOnce(pipeline);
    expect(second.stages.map(stage => [stage.cacheHits, stage.cacheMisses])).toEqual([
      [0, 0], [2, 0], [2, 0],
    ]);
    expect(sourceCalls).toHaveBeenCalledTimes(2);
    expect(transformCalls).toHaveBeenCalledTimes(2);
    expect(sinkCalls).toHaveBeenCalledTimes(2);
    const outputs = second.outputs["@test/cache-sink"] as Array<{ files: Record<string, Uint8Array> }>;
    expect(outputs[0]!.files["A.MD"]).toEqual(new TextEncoder().encode("A.MD"));

    items = ["a.md", "c.md"];
    const third = await orchestrator.runOnce(pipeline);
    expect(third.stages.map(stage => [stage.cacheHits, stage.cacheMisses])).toEqual([
      [0, 0], [1, 1], [1, 1],
    ]);
    expect(transformCalls).toHaveBeenCalledTimes(3);
    expect(sinkCalls).toHaveBeenCalledTimes(3);

    await orchestrator.runOnce(pipeline, { useCache: false });
    expect(transformCalls).toHaveBeenCalledTimes(5);
    expect(sinkCalls).toHaveBeenCalledTimes(5);
    await orchestrator.dispose();
  });

  it("does not skip capability-bearing stages without an external-state contract", async () => {
    const sourceCalls = vi.fn();
    const transformCalls = vi.fn();
    const orchestrator = createOrchestrator({ logger: silentLogger() });
    const pipeline = await orchestrator.buildPipeline(config([
      { stage: source(() => ["a.md"], sourceCalls) },
      { stage: transform(transformCalls, ["storage:read"]) },
    ]));

    const first = await orchestrator.runOnce(pipeline);
    const second = await orchestrator.runOnce(pipeline);
    expect(transformCalls).toHaveBeenCalledTimes(2);
    expect(first.stages[1]).toMatchObject({ cacheHits: 0, cacheMisses: 0 });
    expect(second.stages[1]).toMatchObject({ cacheHits: 0, cacheMisses: 0 });
    await orchestrator.dispose();
  });

  it("invalidates naturally when stage configuration or version changes", async () => {
    const calls = vi.fn();
    const cache = memoryCache();
    const orchestrator = createOrchestrator({ cache, logger: silentLogger() });
    const sourceStage = source(() => ["a.md"], vi.fn());
    const build = (stage: ReturnType<typeof transform>, stageConfig: object) =>
      orchestrator.buildPipeline(config([
        { stage: sourceStage },
        { stage, config: stageConfig },
      ]));
    const original = await build(transform(calls), { flavor: "a" });

    expect((await orchestrator.runOnce(original)).stages[1]).toMatchObject({ cacheMisses: 1 });
    expect((await orchestrator.runOnce(await build(transform(calls), { flavor: "b" }))).stages[1])
      .toMatchObject({ cacheMisses: 1 });
    expect((await orchestrator.runOnce(await build(transform(calls, [], "2.0.0"), { flavor: "a" }))).stages[1])
      .toMatchObject({ cacheMisses: 1 });
    expect((await orchestrator.runOnce(original)).stages[1]).toMatchObject({ cacheHits: 1 });
    expect(calls).toHaveBeenCalledTimes(3);
    await orchestrator.dispose();
  });

  it("fails open for malformed payloads, invalidation failures, and read failures", async () => {
    const calls = vi.fn();
    const invalidate = vi.fn(async () => { throw new Error("cache invalidation unavailable"); });
    const put = vi.fn(async () => {});
    let reads = 0;
    const backend: CacheBackend = {
      async get() {
        reads++;
        if (reads === 1) return makeEntry(new TextEncoder().encode('["not-a-stage-output"]'));
        throw new Error("cache unavailable");
      },
      put,
      invalidate,
      async gc() { return 0; },
      async dispose() {},
    };
    const orchestrator = createOrchestrator({ cache: backend, logger: silentLogger() });
    const pipeline = await orchestrator.buildPipeline(config([
      { stage: source(() => ["a.md"], vi.fn()) },
      { stage: transform(calls) },
    ]));

    expect((await orchestrator.runOnce(pipeline)).outcome).toBe("success");
    expect(invalidate).toHaveBeenCalledTimes(1);
    expect((await orchestrator.runOnce(pipeline)).outcome).toBe("success");
    expect(calls).toHaveBeenCalledTimes(2);
    // Each successful run writes the per-invocation output, the materialized
    // instance checkpoint, and the project-level revision ledger.
    expect(put).toHaveBeenCalledTimes(6);
    await orchestrator.dispose();
  });

  it("executes normally when an output cannot be serialized", async () => {
    const calls = vi.fn();
    const unsupported = defineStage({
      name: "@test/unsupported-cache-output",
      version: "1.0.0",
      apiVersion: KERNEL_API_VERSION,
      description: "returns an intentionally unsupported nested value",
      consumes: Kinds.ContentSource,
      produces: Kinds.ContentSource,
      capabilities: [],
      configSchema: null,
      async run(input) {
        calls();
        return { ...(input as object), unsupported: new Map() } as never;
      },
    });
    const orchestrator = createOrchestrator({ logger: silentLogger() });
    const pipeline = await orchestrator.buildPipeline(config([
      { stage: source(() => ["a.md"], vi.fn()) },
      { stage: unsupported },
    ]));

    expect((await orchestrator.runOnce(pipeline)).outcome).toBe("success");
    expect((await orchestrator.runOnce(pipeline)).outcome).toBe("success");
    expect(calls).toHaveBeenCalledTimes(2);
    await orchestrator.dispose();
  });
});
