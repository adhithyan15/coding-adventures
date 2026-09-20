import { describe, expect, it, vi } from "vitest";
import { KERNEL_API_VERSION, Kinds, streamOf } from "@coding-adventures/forme-types";
import {
  createCancellationTokenSource,
  defineStage,
  silentLogger,
} from "@coding-adventures/forme-stage";
import { memoryCache, type CacheBackend } from "@coding-adventures/forme-cache";
import { StageError } from "@coding-adventures/forme-errors";
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

  it("drains and revisions the full stream when checkpoint writes fail open", async () => {
    let pulls = 0;
    const cache: CacheBackend = {
      async get() { return null; },
      async put() { throw new Error("cache unavailable"); },
      async invalidate() {},
      async gc() { return 0; },
      async dispose() {},
    };
    const source = defineStage({
      name: "@test/cache-fail-open-source",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "must be fully traversed despite checkpoint failure",
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
    const firstOnly = defineStage({
      name: "@test/first-only-collector",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "returns after one value",
      consumes: streamOf(Kinds.ContentSource),
      produces: Kinds.Collection,
      capabilities: [],
      configSchema: null,
      async run(input) {
        for await (const item of input as AsyncIterable<{ path: string }>) {
          return collection("first", [item.path]);
        }
        return collection("first", []);
      },
    });
    const orchestrator = createOrchestrator({ cache, logger: silentLogger() });
    const pipeline = await orchestrator.buildPipeline(config([
      { id: "source", stage: source },
      { id: "first", stage: firstOnly },
    ], 2));

    const result = await orchestrator.runOnce(pipeline);
    expect(result.outcome).toBe("success");
    expect(pulls).toBe(130);
    expect(result.stages[0]).toMatchObject({ itemsProduced: 130 });
    expect(result.stages[0]!.outputRevision).not.toBeNull();
    await orchestrator.dispose();
  });

  it.each([
    { label: "fatal", recoverable: false, expectedOutcome: "failed" as const },
    { label: "recoverable best-effort", recoverable: true, expectedOutcome: "partial" as const },
  ])("preserves buffered prefixes and attributes a $label source error once", async scenario => {
    const failure = new StageError({
      code: "SOURCE_END",
      message: "source failed after its prefix",
      recoverable: scenario.recoverable,
    });
    const seen = { left: 0, right: 0 };
    const source = defineStage({
      name: `@test/${scenario.label}-stream-source`,
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "throws after a long buffered prefix",
      consumes: Kinds.Void,
      produces: streamOf(Kinds.ContentSource),
      capabilities: [],
      configSchema: null,
      async *run() {
        for (let index = 0; index < 130; index++) yield content(String(index));
        throw failure;
      },
    });
    const collector = (side: "left" | "right") => defineStage({
      name: `@test/${side}-${scenario.label}-collector`,
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "drains the prefix before observing the source error",
      consumes: streamOf(Kinds.ContentSource),
      produces: Kinds.Collection,
      capabilities: [],
      configSchema: null,
      async run(input) {
        const paths: string[] = [];
        for await (const item of input as AsyncIterable<{ path: string }>) {
          paths.push(item.path);
          seen[side] += 1;
        }
        return collection(side, paths);
      },
    });
    const orchestrator = createOrchestrator({ logger: silentLogger() });
    const base = config([
      { id: "source", stage: source },
      { id: "left", stage: collector("left") },
      { id: "right", stage: collector("right") },
    ], 3, [
      { from: { id: "source" }, to: { id: "left" } },
      { from: { id: "source" }, to: { id: "right" } },
    ]);
    const pipeline = await orchestrator.buildPipeline({
      ...base,
      settings: { ...base.settings, bestEffort: scenario.recoverable },
    });

    const result = await Promise.race([
      orchestrator.runOnce(pipeline),
      new Promise<never>((_resolve, reject) =>
        setTimeout(() => reject(new Error("source failure did not settle")), 2_000)),
    ]);
    expect(result.outcome).toBe(scenario.expectedOutcome);
    expect(seen).toEqual({ left: 130, right: 130 });
    expect(result.errors).toHaveLength(1);
    expect(result.errors[0]).toMatchObject({ instanceId: "source", code: "SOURCE_END" });
    expect(result.stages.map(stage => stage.outcome)).toEqual(["failed", "skipped", "skipped"]);
    await orchestrator.dispose();
  });

  it("restores a scheduler-promoted per-item stream on the second run", async () => {
    const cache = memoryCache();
    let sourceRuns = 0;
    let transformRuns = 0;
    const manifest = {
      version: 1 as const,
      entries: [],
      revision: computeRevisionId({ version: 1, entries: [] }),
    };
    const source = defineStage({
      name: "@test/promoted-restore-source",
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
    const transform = defineStage({
      name: "@test/promoted-restore-transform",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "per-item stage promoted to a scheduler stream",
      consumes: Kinds.ContentSource,
      produces: Kinds.ContentSource,
      capabilities: [],
      configSchema: null,
      async run(input) { transformRuns += 1; return input; },
    });
    const collector = defineStage({
      name: "@test/promoted-restore-collector",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "reruns to exercise lazy restored input",
      consumes: streamOf(Kinds.ContentSource),
      produces: Kinds.Collection,
      capabilities: ["filesystem:read"] as never,
      configSchema: null,
      async run(input) {
        const paths: string[] = [];
        for await (const item of input as AsyncIterable<{ path: string }>) paths.push(item.path);
        return collection("promoted", paths);
      },
    });
    const orchestrator = createOrchestrator({ cache, logger: silentLogger() });
    const pipeline = await orchestrator.buildPipeline(config([
      { id: "source", stage: source },
      { id: "transform", stage: transform },
      { id: "collector", stage: collector },
    ], 3));

    expect((await orchestrator.runOnce(pipeline)).outcome).toBe("success");
    const second = await orchestrator.runOnce(pipeline);
    expect(second.outcome).toBe("success");
    expect(second.stages.map(stage => stage.outcome)).toEqual(["skipped", "skipped", "success"]);
    expect(sourceRuns).toBe(1);
    expect(transformRuns).toBe(2);
    expect(second.outputs.collector).toMatchObject({ meta: { paths: ["a", "b"] } });
    await orchestrator.dispose();
  });

  it.each(["null result", "throwing done getter"])(
    "attributes a hostile iterator $s to its source without hanging unclaimed branches",
    async variant => {
      const source = defineStage({
        name: `@test/hostile-${variant.replaceAll(" ", "-")}`,
        version: "0.1.0",
        apiVersion: KERNEL_API_VERSION,
        description: "returns a protocol-invalid async iterator result",
        consumes: Kinds.Void,
        produces: streamOf(Kinds.ContentSource),
        capabilities: [],
        configSchema: null,
        run() {
          return {
            [Symbol.asyncIterator]() {
              return {
                async next() {
                  if (variant === "null result") return null;
                  return Object.defineProperty({}, "done", {
                    get() { throw new Error("hostile done getter"); },
                  });
                },
                async return() { return { done: true, value: undefined }; },
              };
            },
          } as never;
        },
      });
      const collector = defineStage({
        name: `@test/hostile-${variant.replaceAll(" ", "-")}-collector`,
        version: "0.1.0",
        apiVersion: KERNEL_API_VERSION,
        description: "must not receive a duplicate diagnostic",
        consumes: streamOf(Kinds.ContentSource),
        produces: Kinds.Collection,
        capabilities: [],
        configSchema: null,
        async run(input) {
          for await (const _item of input as AsyncIterable<unknown>) { /* drain */ }
          return collection("hostile", []);
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
          setTimeout(() => reject(new Error("hostile iterator did not settle")), 2_000)),
      ]);
      expect(result.outcome).toBe("failed");
      expect(result.errors).toHaveLength(1);
      expect(result.errors[0]!.instanceId).toBe("source");
      expect(result.stages.map(stage => stage.outcome)).toEqual(["failed", "skipped"]);
      await orchestrator.dispose();
    },
  );

  it("keeps external cancellation during an active source pull classified as cancelled", async () => {
    const cancellation = createCancellationTokenSource();
    const firstSeen = deferred<void>();
    const source = defineStage({
      name: "@test/cancelled-active-pull-source",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "observes cancellation during its second pull",
      consumes: Kinds.Void,
      produces: streamOf(Kinds.ContentSource),
      capabilities: [],
      configSchema: null,
      async *run(_input, _config, ctx) {
        yield content("a");
        await new Promise<void>(resolve => {
          if (ctx.cancellation.cancelled) resolve();
          else ctx.cancellation.signal.addEventListener("abort", () => resolve(), { once: true });
        });
        ctx.cancellation.throwIfCancelled();
      },
    });
    const collector = defineStage({
      name: "@test/cancelled-active-pull-collector",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "starts the second source pull",
      consumes: streamOf(Kinds.ContentSource),
      produces: Kinds.Collection,
      capabilities: [],
      configSchema: null,
      async run(input) {
        for await (const _item of input as AsyncIterable<unknown>) firstSeen.resolve();
        return collection("cancelled", []);
      },
    });
    const orchestrator = createOrchestrator({ logger: silentLogger() });
    const pipeline = await orchestrator.buildPipeline(config([
      { id: "source", stage: source },
      { id: "collector", stage: collector },
    ], 2));
    const running = orchestrator.runOnce(pipeline, { cancellation: cancellation.token });
    await firstSeen.promise;
    cancellation.cancel("user stopped run");

    const result = await Promise.race([
      running,
      new Promise<never>((_resolve, reject) =>
        setTimeout(() => reject(new Error("cancelled stream did not settle")), 2_000)),
    ]);
    expect(result.outcome).toBe("cancelled");
    expect(result.errors).toEqual([]);
    await orchestrator.dispose();
  });

  it("awaits rejecting iterator cleanup without replacing cancellation", async () => {
    const cancellation = createCancellationTokenSource();
    const firstSeen = deferred<void>();
    const cleanupStarted = deferred<void>();
    const cleanupRelease = deferred<void>();
    let cleanupFinished = false;
    let disposedBeforeCleanup = false;
    const source = defineStage({
      name: "@test/rejecting-cancellation-cleanup-source",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "throws from generator cleanup after an awaited barrier",
      consumes: Kinds.Void,
      produces: streamOf(Kinds.ContentSource),
      capabilities: [],
      configSchema: null,
      async *run() {
        try {
          yield content("a");
          yield content("unused");
        } finally {
          cleanupStarted.resolve();
          await cleanupRelease.promise;
          cleanupFinished = true;
          throw new Error("cleanup failed after cancellation");
        }
      },
      async dispose() {
        disposedBeforeCleanup = !cleanupFinished;
      },
    });
    const collector = defineStage({
      name: "@test/rejecting-cancellation-cleanup-collector",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "returns after cancellation while holding an attached branch",
      consumes: streamOf(Kinds.ContentSource),
      produces: Kinds.Collection,
      capabilities: [],
      configSchema: null,
      async run(input, _config, ctx) {
        for await (const _item of input as AsyncIterable<unknown>) {
          firstSeen.resolve();
          await new Promise<void>(resolve => {
            if (ctx.cancellation.cancelled) resolve();
            else ctx.cancellation.signal.addEventListener("abort", () => resolve(), { once: true });
          });
          break;
        }
        return collection("cancelled-cleanup", []);
      },
    });
    const orchestrator = createOrchestrator({ logger: silentLogger() });
    const pipeline = await orchestrator.buildPipeline(config([
      { id: "source", stage: source },
      { id: "collector", stage: collector },
    ], 2));
    const running = orchestrator.runOnce(pipeline, { cancellation: cancellation.token });
    await firstSeen.promise;
    cancellation.cancel("user stopped run");
    await cleanupStarted.promise;
    expect(cleanupFinished).toBe(false);
    expect(disposedBeforeCleanup).toBe(false);
    cleanupRelease.resolve();

    const result = await Promise.race([
      running,
      new Promise<never>((_resolve, reject) =>
        setTimeout(() => reject(new Error("rejecting cleanup did not settle")), 2_000)),
    ]);
    expect(result.outcome).toBe("cancelled");
    expect(result.errors).toEqual([]);
    expect(cleanupFinished).toBe(true);
    expect(disposedBeforeCleanup).toBe(false);
    await orchestrator.dispose();
  });
});
