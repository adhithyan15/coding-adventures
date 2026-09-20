import { describe, expect, it, vi } from "vitest";
import { KERNEL_API_VERSION, Kinds, streamOf } from "@coding-adventures/forme-types";
import { defineStage, silentLogger } from "@coding-adventures/forme-stage";
import { memoryCache } from "@coding-adventures/forme-cache";
import { computeRevisionId } from "@coding-adventures/forme-identity";
import type { PipelineConfig } from "@coding-adventures/forme-pipeline-config";
import { createOrchestrator } from "../src/index.js";

interface Deferred<T> {
  readonly promise: Promise<T>;
  readonly resolve: (value: T) => void;
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(accept => { resolve = accept; });
  return { promise, resolve };
}

function content(path: string): never {
  return {
    path,
    bytes: new TextEncoder().encode(path),
    mimeType: "text/plain",
    identity: "01952c0d-7e63-7000-8000-000000000000",
    revision: "blake2b:00",
    providerMeta: {},
  } as never;
}

function collection(name: string, paths: readonly string[]): never {
  return {
    name,
    entries: [],
    discriminant: "test",
    meta: { paths: [...paths] },
  } as never;
}

function config(
  stages: PipelineConfig["stages"],
  maxConcurrency: number,
  wires: PipelineConfig["wires"] = [],
): PipelineConfig {
  return {
    name: "live-stream-scheduler",
    settings: {
      storageRoot: "./",
      cacheDir: null,
      reproducibleBuild: false,
      maxConcurrency,
      logLevel: "error",
      bestEffort: false,
      deadlineMs: null,
    },
    stages,
    wires,
  };
}

describe("live stream scheduler integration", () => {
  it("starts a per-item consumer before its producer stream completes", async () => {
    const releaseSecond = deferred<void>();
    const firstConsumer = deferred<void>();
    let producerCompleted = false;
    const starts: string[] = [];
    const source = defineStage({
      name: "@test/live-source",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "pauses after its first value",
      consumes: Kinds.Void,
      produces: streamOf(Kinds.ContentSource),
      capabilities: [],
      configSchema: null,
      async *run() {
        yield content("a");
        await releaseSecond.promise;
        yield content("b");
        producerCompleted = true;
      },
    });
    const transform = defineStage({
      name: "@test/live-transform",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "observes values as they arrive",
      consumes: Kinds.ContentSource,
      produces: Kinds.ContentSource,
      capabilities: [],
      configSchema: null,
      async run(input) {
        const path = (input as { path: string }).path;
        starts.push(path);
        if (path === "a") firstConsumer.resolve();
        return input;
      },
    });
    const orchestrator = createOrchestrator({ logger: silentLogger() });
    const pipeline = await orchestrator.buildPipeline(config([
      { id: "source", stage: source },
      { id: "transform", stage: transform },
    ], 2));
    const running = orchestrator.runOnce(pipeline);

    try {
      await vi.waitFor(() => expect(starts).toEqual(["a"]));
      expect(producerCompleted).toBe(false);
    } finally {
      releaseSecond.resolve();
    }

    const result = await running;
    expect(result.outcome).toBe("success");
    expect((result.outputs.transform as Array<{ path: string }>).map(item => item.path))
      .toEqual(["a", "b"]);
    await orchestrator.dispose();
  });

  it("makes stream collector progress with maxConcurrency one", async () => {
    const source = defineStage({
      name: "@test/one-permit-source",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "three-value source",
      consumes: Kinds.Void,
      produces: streamOf(Kinds.ContentSource),
      capabilities: [],
      configSchema: null,
      async *run() {
        for (const path of ["a", "b", "c"]) yield content(path);
      },
    });
    const collector = defineStage({
      name: "@test/one-permit-collector",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "collects while yielding its only permit",
      consumes: streamOf(Kinds.ContentSource),
      produces: Kinds.Collection,
      capabilities: [],
      configSchema: null,
      async run(input) {
        const paths: string[] = [];
        for await (const item of input as AsyncIterable<{ path: string }>) paths.push(item.path);
        return collection("one-permit", paths);
      },
    });
    const orchestrator = createOrchestrator({ logger: silentLogger() });
    const pipeline = await orchestrator.buildPipeline(config([
      { id: "source", stage: source },
      { id: "collector", stage: collector },
    ], 1));

    const result = await Promise.race([
      orchestrator.runOnce(pipeline),
      new Promise<never>((_resolve, reject) =>
        setTimeout(() => reject(new Error("one-permit stream deadlock")), 1_000)),
    ]);

    expect(result.outcome).toBe("success");
    expect(result.outputs.collector).toMatchObject({ meta: { paths: ["a", "b", "c"] } });
    await orchestrator.dispose();
  });

  it("backpressures one traversal across fast, slow, and checkpoint branches", async () => {
    const releaseSlow = deferred<void>();
    let pulls = 0;
    let fastSeen = 0;
    let slowSeen = 0;
    const source = defineStage({
      name: "@test/backpressure-source",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "more than one default stream window",
      consumes: Kinds.Void,
      produces: streamOf(Kinds.ContentSource),
      capabilities: [],
      configSchema: null,
      async *run() {
        for (let index = 0; index < 130; index++) {
          pulls += 1;
          yield content(String(index));
        }
      },
    });
    const slow = defineStage({
      name: "@test/slow-collector",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "holds its branch after one value",
      consumes: streamOf(Kinds.ContentSource),
      produces: Kinds.Collection,
      capabilities: [],
      configSchema: null,
      async run(input) {
        const paths: string[] = [];
        for await (const item of input as AsyncIterable<{ path: string }>) {
          paths.push(item.path);
          slowSeen += 1;
          if (slowSeen === 1) await releaseSlow.promise;
        }
        return collection("slow", paths);
      },
    });
    const fast = defineStage({
      name: "@test/fast-collector",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "drains as quickly as possible",
      consumes: streamOf(Kinds.ContentSource),
      produces: Kinds.Collection,
      capabilities: [],
      configSchema: null,
      async run(input) {
        const paths: string[] = [];
        for await (const item of input as AsyncIterable<{ path: string }>) {
          paths.push(item.path);
          fastSeen += 1;
        }
        return collection("fast", paths);
      },
    });
    const orchestrator = createOrchestrator({ logger: silentLogger() });
    const pipeline = await orchestrator.buildPipeline(config([
      { id: "source", stage: source },
      { id: "slow", stage: slow },
      { id: "fast", stage: fast },
    ], 4, [
      { from: { id: "source" }, to: { id: "slow" } },
      { from: { id: "source" }, to: { id: "fast" } },
    ]));
    const running = orchestrator.runOnce(pipeline);

    try {
      await vi.waitFor(() => expect(fastSeen).toBeGreaterThanOrEqual(64));
      expect(slowSeen).toBe(1);
      expect(pulls).toBeLessThanOrEqual(66);
    } finally {
      releaseSlow.resolve();
    }

    const result = await running;
    expect(result.outcome).toBe("success");
    expect(pulls).toBe(130);
    expect((result.outputs.fast as { meta: { paths: string[] } }).meta.paths).toHaveLength(130);
    expect((result.outputs.slow as { meta: { paths: string[] } }).meta.paths).toHaveLength(130);
    await orchestrator.dispose();
  });

  it("restores an unchanged source stream while a conservative collector reruns", async () => {
    const cache = memoryCache();
    let sourceRuns = 0;
    let collectorRuns = 0;
    const manifest = {
      version: 1 as const,
      entries: [],
      revision: computeRevisionId({ version: 1, entries: [] }),
    };
    const source = defineStage({
      name: "@test/restored-stream-source",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "unchanged observed source",
      consumes: Kinds.Void,
      produces: streamOf(Kinds.ContentSource),
      capabilities: [],
      configSchema: null,
      async externalState() { return manifest; },
      async *run() {
        sourceRuns += 1;
        yield content("a");
        yield content("b");
      },
    });
    const collector = defineStage({
      name: "@test/conservative-stream-collector",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "reruns while consuming a restored stream",
      consumes: streamOf(Kinds.ContentSource),
      produces: Kinds.Collection,
      capabilities: ["filesystem:read"] as never,
      configSchema: null,
      async run(input) {
        collectorRuns += 1;
        const paths: string[] = [];
        for await (const item of input as AsyncIterable<{ path: string }>) paths.push(item.path);
        return collection("restored", paths);
      },
    });
    const orchestrator = createOrchestrator({ cache, logger: silentLogger() });
    const pipeline = await orchestrator.buildPipeline(config([
      { id: "source", stage: source },
      { id: "collector", stage: collector },
    ], 2));

    expect((await orchestrator.runOnce(pipeline)).outcome).toBe("success");
    const second = await orchestrator.runOnce(pipeline);

    expect(second.outcome).toBe("success");
    expect(second.stages.map(stage => stage.outcome)).toEqual(["skipped", "success"]);
    expect(sourceRuns).toBe(1);
    expect(collectorRuns).toBe(2);
    expect(second.outputs.collector).toMatchObject({ meta: { paths: ["a", "b"] } });
    await orchestrator.dispose();
  });
});
