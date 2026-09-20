import { describe, expect, it, vi } from "vitest";
import { KERNEL_API_VERSION, Kinds, streamOf } from "@coding-adventures/forme-types";
import {
  createCancellationTokenSource,
  defineStage,
  silentLogger,
} from "@coding-adventures/forme-stage";
import type { PipelineConfig } from "@coding-adventures/forme-pipeline-config";
import { createOrchestrator } from "../src/index.js";

interface Deferred<T> {
  readonly promise: Promise<T>;
  readonly resolve: (value: T) => void;
  readonly reject: (error: unknown) => void;
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((accept, fail) => {
    resolve = accept;
    reject = fail;
  });
  return { promise, resolve, reject };
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

function config(
  stages: PipelineConfig["stages"],
  maxConcurrency: number | null,
  wires: PipelineConfig["wires"] = [],
): PipelineConfig {
  return {
    name: "scheduler-concurrency",
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

describe("concurrent scheduler", () => {
  it("starts DAG-ready stages in declaration order up to the shared limit", async () => {
    const gates = [deferred<void>(), deferred<void>(), deferred<void>()];
    const starts: string[] = [];
    const stages = ["a", "b", "c"].map((id, index) => ({
      id,
      stage: defineStage({
        name: `@test/ready-${id}`,
        version: "0.1.0",
        apiVersion: KERNEL_API_VERSION,
        description: `ready source ${id}`,
        consumes: Kinds.Void,
        produces: Kinds.ContentSource,
        capabilities: [],
        configSchema: null,
        async run() {
          starts.push(id);
          await gates[index]!.promise;
          return content(id);
        },
      }),
    }));
    const orchestrator = createOrchestrator({ logger: silentLogger() });
    const pipeline = await orchestrator.buildPipeline(config(stages, 2));
    const running = orchestrator.runOnce(pipeline);

    await vi.waitFor(() => expect(starts).toEqual(["a", "b"]));
    gates[0]!.resolve();
    await vi.waitFor(() => expect(starts).toEqual(["a", "b", "c"]));
    gates[1]!.resolve();
    gates[2]!.resolve();

    const result = await running;
    expect(result.outcome).toBe("success");
    expect(result.stages.map(stage => stage.instanceId)).toEqual(["a", "b", "c"]);
    expect(Object.keys(result.outputs)).toEqual(["a", "b", "c"]);
    await orchestrator.dispose();
  });

  it("runs stream items concurrently while preserving input order", async () => {
    const itemGates = new Map([
      ["a", deferred<void>()],
      ["b", deferred<void>()],
      ["c", deferred<void>()],
    ]);
    const starts: string[] = [];
    const source = defineStage({
      name: "@test/ordered-source",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "ordered stream source",
      consumes: Kinds.Void,
      produces: streamOf(Kinds.ContentSource),
      capabilities: [],
      configSchema: null,
      async *run() {
        for (const id of ["a", "b", "c"]) yield content(id);
      },
    });
    const transform = defineStage({
      name: "@test/ordered-transform",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "completes items out of order",
      consumes: Kinds.ContentSource,
      produces: Kinds.ContentSource,
      capabilities: [],
      configSchema: null,
      async run(input) {
        const path = (input as { path: string }).path;
        starts.push(path);
        await itemGates.get(path)!.promise;
        return content(path.toUpperCase());
      },
    });
    const orchestrator = createOrchestrator({ logger: silentLogger() });
    const pipeline = await orchestrator.buildPipeline(config([
      { id: "source", stage: source },
      { id: "transform", stage: transform },
    ], 2));
    const running = orchestrator.runOnce(pipeline);

    await vi.waitFor(() => expect(starts).toEqual(["a", "b"]));
    itemGates.get("b")!.resolve();
    await vi.waitFor(() => expect(starts).toEqual(["a", "b", "c"]));
    itemGates.get("c")!.resolve();
    itemGates.get("a")!.resolve();

    const result = await running;
    expect(result.outcome).toBe("success");
    expect((result.outputs.transform as Array<{ path: string }>).map(item => item.path))
      .toEqual(["A", "B", "C"]);
    expect(result.stages[1]).toMatchObject({
      instanceId: "transform",
      itemsConsumed: 3,
      itemsProduced: 3,
    });
    await orchestrator.dispose();
  });

  it("shares one permit budget across ready stages and per-item work", async () => {
    let active = 0;
    let peakActive = 0;
    const enter = (): void => {
      active += 1;
      peakActive = Math.max(peakActive, active);
    };
    const leave = (): void => { active -= 1; };
    const independentGate = deferred<void>();
    const itemGate = deferred<void>();
    const streamSource = defineStage({
      name: "@test/shared-stream",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "materialized source",
      consumes: Kinds.Void,
      produces: streamOf(Kinds.ContentSource),
      capabilities: [],
      configSchema: null,
      async *run() { yield content("a"); yield content("b"); },
    });
    const independent = defineStage({
      name: "@test/shared-independent",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "holds one shared permit",
      consumes: Kinds.Void,
      produces: Kinds.ContentSource,
      capabilities: [],
      configSchema: null,
      async run() {
        enter();
        try { await independentGate.promise; } finally { leave(); }
        return content("independent");
      },
    });
    const transform = defineStage({
      name: "@test/shared-transform",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "uses the same shared permits",
      consumes: Kinds.ContentSource,
      produces: Kinds.ContentSource,
      capabilities: [],
      configSchema: null,
      async run(input) {
        enter();
        try { await itemGate.promise; } finally { leave(); }
        return input;
      },
    });
    const orchestrator = createOrchestrator({ logger: silentLogger() });
    const pipeline = await orchestrator.buildPipeline(config([
      { id: "stream", stage: streamSource },
      { id: "independent", stage: independent },
      { id: "transform", stage: transform },
    ], 2, [{ from: { id: "stream" }, to: { id: "transform" } }]));
    const running = orchestrator.runOnce(pipeline);

    await vi.waitFor(() => expect(active).toBe(2));
    expect(peakActive).toBe(2);
    independentGate.resolve();
    itemGate.resolve();
    const result = await running;
    expect(result.outcome).toBe("success");
    expect(peakActive).toBe(2);
    await orchestrator.dispose();
  });

  it("does not start queued stages after a fatal failure", async () => {
    const starts: string[] = [];
    const stages = ["fatal", "later-a", "later-b"].map(id => ({
      id,
      stage: defineStage({
        name: `@test/${id}`,
        version: "0.1.0",
        apiVersion: KERNEL_API_VERSION,
        description: id,
        consumes: Kinds.Void,
        produces: Kinds.ContentSource,
        capabilities: [],
        configSchema: null,
        async run() {
          starts.push(id);
          if (id === "fatal") throw new Error("stop now");
          return content(id);
        },
      }),
    }));
    const orchestrator = createOrchestrator({ logger: silentLogger() });
    const pipeline = await orchestrator.buildPipeline(config(stages, 1));

    const result = await orchestrator.runOnce(pipeline);

    expect(result.outcome).toBe("failed");
    expect(starts).toEqual(["fatal"]);
    expect(result.stages.map(stage => stage.outcome)).toEqual(["failed", "skipped", "skipped"]);
    await orchestrator.dispose();
  });

  it("cancels queued stages deterministically", async () => {
    const cancellation = createCancellationTokenSource();
    const started = deferred<void>();
    const starts: string[] = [];
    const first = defineStage({
      name: "@test/cancel-running",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "waits for cancellation",
      consumes: Kinds.Void,
      produces: Kinds.ContentSource,
      capabilities: [],
      configSchema: null,
      async run(_input, _config, ctx) {
        starts.push("running");
        started.resolve();
        await new Promise<void>(resolve => ctx.cancellation.onCancel(resolve));
        ctx.cancellation.throwIfCancelled();
        return content("unreachable");
      },
    });
    const queued = defineStage({
      name: "@test/cancel-queued",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "must not start",
      consumes: Kinds.Void,
      produces: Kinds.ContentSource,
      capabilities: [],
      configSchema: null,
      async run() { starts.push("queued"); return content("queued"); },
    });
    const orchestrator = createOrchestrator({ logger: silentLogger() });
    const pipeline = await orchestrator.buildPipeline(config([
      { id: "running", stage: first },
      { id: "queued", stage: queued },
    ], 1));
    const running = orchestrator.runOnce(pipeline, { cancellation: cancellation.token });
    await started.promise;
    cancellation.cancel("test cancellation");

    const result = await running;
    expect(result.outcome).toBe("cancelled");
    expect(starts).toEqual(["running"]);
    expect(result.stages.map(stage => stage.outcome)).toEqual(["skipped", "skipped"]);
    await orchestrator.dispose();
  });
});
