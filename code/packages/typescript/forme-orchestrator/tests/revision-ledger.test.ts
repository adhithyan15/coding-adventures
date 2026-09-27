import { afterEach, describe, expect, it, vi } from "vitest";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { filesystemCache, makeEntry, memoryCache } from "@coding-adventures/forme-cache";
import { computeBinaryRevisionId, computeRevisionId } from "@coding-adventures/forme-identity";
import type { PipelineConfig } from "@coding-adventures/forme-pipeline-config";
import {
  defineStage,
  silentLogger,
  type StageContext,
} from "@coding-adventures/forme-stage";
import { KERNEL_API_VERSION, Kinds, streamOf } from "@coding-adventures/forme-types";
import { createOrchestrator } from "../src/index.js";
import { encodeCachedStageOutput } from "../src/cache-codec.js";
import { instanceCheckpointKey } from "../src/instance-checkpoint.js";
import { revisionLedgerKey } from "../src/revision-ledger.js";

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
    capabilities: ["storage:read"],
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

function transform(runCalls = vi.fn()) {
  return defineStage({
    name: "@test/revision-transform",
    version: "1.0.0",
    apiVersion: KERNEL_API_VERSION,
    description: "identity transform",
    consumes: Kinds.ContentSource,
    produces: Kinds.ContentSource,
    capabilities: [],
    configSchema: null,
    run(input) {
      runCalls();
      return input;
    },
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
    expect(second.stages.map(stage => stage.outcome)).toEqual(["skipped", "skipped"]);
    expect(second.stages[1]).toMatchObject({ cacheHits: 1, cacheMisses: 0 });
    expect(runCalls).toHaveBeenCalledTimes(1);

    current = { path: "post.md", text: "second" };
    const third = await runFresh();
    expect(third.buildId).not.toBe(first.buildId);
    expect(third.stages.map(stage => stage.inputChanged)).toEqual([true, true]);
    expect(third.stages.map(stage => stage.outcome)).toEqual(["success", "success"]);
    expect(runCalls).toHaveBeenCalledTimes(2);
  });

  it("runs only a changed source and its downstream closure", async () => {
    const cacheRoot = await mkdtemp(join(tmpdir(), "forme-affected-branches-"));
    roots.push(cacheRoot);
    let left = { path: "left.md", text: "left-v1" };
    let right = { path: "right.md", text: "right-v1" };
    const leftSourceCalls = vi.fn();
    const rightSourceCalls = vi.fn();
    const leftTransformCalls = vi.fn();
    const rightTransformCalls = vi.fn();
    const config: PipelineConfig = {
      ...pipelineConfig(observedSource(() => left)),
      name: "affected-branches-test",
      stages: [
        { id: "left-source", stage: observedSource(() => left, leftSourceCalls) },
        { id: "left-transform", stage: transform(leftTransformCalls) },
        { id: "right-source", stage: observedSource(() => right, rightSourceCalls) },
        { id: "right-transform", stage: transform(rightTransformCalls) },
      ],
      wires: [
        { from: { id: "left-source" }, to: { id: "left-transform" } },
        { from: { id: "right-source" }, to: { id: "right-transform" } },
      ],
    };

    const runFresh = async () => {
      const orchestrator = createOrchestrator({
        cache: filesystemCache(cacheRoot),
        logger: silentLogger(),
      });
      const result = await orchestrator.runOnce(await orchestrator.buildPipeline(config));
      await orchestrator.dispose();
      return result;
    };

    await runFresh();
    const unchanged = await runFresh();
    expect(unchanged.stages.map(stage => stage.outcome)).toEqual([
      "skipped", "skipped", "skipped", "skipped",
    ]);

    left = { ...left, text: "left-v2" };
    const changed = await runFresh();
    expect(changed.stages.map(stage => [stage.instanceId, stage.outcome])).toEqual([
      ["left-source", "success"],
      ["left-transform", "success"],
      ["right-source", "skipped"],
      ["right-transform", "skipped"],
    ]);
    expect(leftSourceCalls).toHaveBeenCalledTimes(2);
    expect(leftTransformCalls).toHaveBeenCalledTimes(2);
    expect(rightSourceCalls).toHaveBeenCalledTimes(1);
    expect(rightTransformCalls).toHaveBeenCalledTimes(1);
    expect(changed.outputs["right-transform"]).toBeDefined();
  });

  it("fails open when an unchanged instance checkpoint has the wrong output", async () => {
    let current = { path: "post.md", text: "stable" };
    const sourceCalls = vi.fn();
    const cache = memoryCache();
    const orchestrator = createOrchestrator({ cache, logger: silentLogger() });
    const pipeline = await orchestrator.buildPipeline(
      pipelineConfig(observedSource(() => current, sourceCalls)),
    );
    const first = await orchestrator.runOnce(pipeline);
    const sourceSummary = first.stages[0]!;
    const sourceInstance = pipeline.dag.instances.get("source")!;
    await cache.put(
      instanceCheckpointKey(
        revisionLedgerKey(pipeline),
        sourceInstance,
        sourceSummary.inputRevision!,
      ),
      makeEntry(encodeCachedStageOutput({ value: [], isStream: true })),
    );

    const second = await orchestrator.runOnce(pipeline);
    expect(second.outcome).toBe("success");
    expect(second.stages.map(stage => stage.outcome)).toEqual(["success", "skipped"]);
    expect(sourceCalls).toHaveBeenCalledTimes(2);
    expect(second.outputs.transform).toEqual(first.outputs.transform);
    await orchestrator.dispose();
  });

  it("reruns capability-bearing instances but restores their untouched pure downstream", async () => {
    const sourceCalls = vi.fn();
    const capabilityCalls = vi.fn();
    const downstreamCalls = vi.fn();
    const capabilityStage = defineStage({
      ...transform(),
      name: "@test/capability-transform",
      capabilities: ["storage:read"],
      run(input) {
        capabilityCalls();
        return input;
      },
    });
    const downstreamStage = transform(downstreamCalls);
    const cache = memoryCache();
    const orchestrator = createOrchestrator({ cache, logger: silentLogger() });
    const pipeline = await orchestrator.buildPipeline({
      ...pipelineConfig(observedSource(
        () => ({ path: "post.md", text: "stable" }),
        sourceCalls,
      )),
      name: "capability-boundary-test",
      stages: [
        { id: "source", stage: observedSource(
          () => ({ path: "post.md", text: "stable" }),
          sourceCalls,
        ) },
        { id: "capability", stage: capabilityStage },
        { id: "downstream", stage: downstreamStage },
      ],
    });

    await orchestrator.runOnce(pipeline);
    const second = await orchestrator.runOnce(pipeline);
    expect(second.stages.map(stage => stage.outcome)).toEqual([
      "skipped", "success", "skipped",
    ]);
    expect(sourceCalls).toHaveBeenCalledTimes(1);
    expect(capabilityCalls).toHaveBeenCalledTimes(2);
    expect(downstreamCalls).toHaveBeenCalledTimes(1);
    await orchestrator.dispose();
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
