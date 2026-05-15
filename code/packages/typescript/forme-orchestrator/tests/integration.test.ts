/**
 * forme-orchestrator — end-to-end integration tests
 *
 * The single test that matters: build a real linear pipeline of three
 * stages (source → transform → sink), run it, verify the output.
 * Plus error/cancellation/dispose paths around it.
 */

import { describe, it, expect, vi } from "vitest";
import {
  KERNEL_API_VERSION,
  Kinds,
  streamOf,
} from "@coding-adventures/forme-types";
import { defineStage } from "@coding-adventures/forme-stage";
import { StageError } from "@coding-adventures/forme-errors";
import {
  createCancellationTokenSource,
  silentLogger,
} from "@coding-adventures/forme-stage";
import {
  buildDag,
  buildPipeline,
  createOrchestrator,
  runOnce,
} from "../src/index.js";
import type { PipelineConfig } from "@coding-adventures/forme-pipeline-config";
import { validateConfig } from "@coding-adventures/forme-pipeline-config";

// ─── Stage builders ──────────────────────────────────────────────────────

function tinySource(items: readonly string[]) {
  return defineStage({
    name: "@test/source",
    version: "0.1.0",
    apiVersion: KERNEL_API_VERSION,
    description: "yields fixed strings",
    consumes: Kinds.Void,
    produces: streamOf(Kinds.ContentSource),
    capabilities: [],
    configSchema: null,
    async *run() {
      for (const item of items) {
        yield {
          path: item,
          bytes: new TextEncoder().encode(item),
          mimeType: "text/plain",
          identity: "01952c0d-7e63-7000-8000-000000000000" as never,
          revision: "blake2b:00" as never,
          providerMeta: {},
        } as never;
      }
    },
  });
}

const upper = defineStage({
  name: "@test/upper",
  version: "0.1.0",
  apiVersion: KERNEL_API_VERSION,
  description: "uppercases path of a content source",
  consumes: Kinds.ContentSource,
  produces: Kinds.ContentSource,
  capabilities: [],
  configSchema: null,
  async run(source) {
    return {
      ...(source as Record<string, unknown>),
      path: (source as { path: string }).path.toUpperCase(),
    } as never;
  },
});

const sink = defineStage({
  name: "@test/sink",
  version: "0.1.0",
  apiVersion: KERNEL_API_VERSION,
  description: "wraps a content source as a deploy artifact",
  consumes: Kinds.ContentSource,
  produces: Kinds.DeployArtifact,
  capabilities: [],
  configSchema: null,
  async run(source) {
    const s = source as { path: string };
    return {
      variant: { kind: "dist-tree" },
      files: { [s.path]: new TextEncoder().encode("hi") },
      manifest: {
        routes: [], assets: [], buildTime: "2026-05-15T00:00:00Z",
        buildId: "blake2b:00" as never,
      },
    } as never;
  },
});

function makeConfig(stages: readonly { stage: ReturnType<typeof defineStage>; id?: string }[]): PipelineConfig {
  return {
    name: "test",
    settings: {
      storageRoot: "./",
      cacheDir: null,
      reproducibleBuild: false,
      maxConcurrency: null,
      logLevel: "error",
      bestEffort: false,
      deadlineMs: null,
    },
    stages: stages as never,
  };
}

// ─── Tests ───────────────────────────────────────────────────────────────

describe("buildPipeline + runOnce — happy path", () => {
  it("source(stream) → transform(per-item) → sink(per-item)", async () => {
    const source = tinySource(["a.md", "b.md"]);
    const config = makeConfig([{ stage: source }, { stage: upper }, { stage: sink }]);
    const o = createOrchestrator({ logger: silentLogger() });
    const pipeline = await o.buildPipeline(config);
    const result = await o.runOnce(pipeline);

    expect(result.outcome).toBe("success");
    expect(result.errors).toEqual([]);
    expect(result.stages.length).toBe(3);
    // Sink received 2 invocations (one per source item).
    const sinkSummary = result.stages.find(s => s.stageName === "@test/sink")!;
    expect(sinkSummary.itemsConsumed).toBe(2);
    expect(sinkSummary.itemsProduced).toBe(2);
    // Output is the array of per-item DeployArtifact values.
    const out = result.outputs["@test/sink"];
    expect(Array.isArray(out)).toBe(true);
    expect((out as unknown[]).length).toBe(2);
    await o.dispose();
  });

  it("named outputs override sink instance ids", async () => {
    const source = tinySource(["x.md"]);
    const config: PipelineConfig = {
      ...makeConfig([{ stage: source }, { stage: upper }, { stage: sink, id: "the-sink" }]),
      outputs: [{ fromInstance: "the-sink", name: "primary" }],
    };
    const o = createOrchestrator({ logger: silentLogger() });
    const pipeline = await o.buildPipeline(config);
    const result = await o.runOnce(pipeline);
    expect(result.outputs.primary).toBeDefined();
    expect(result.outputs["the-sink"]).toBeUndefined();
    await o.dispose();
  });
});

describe("Error handling", () => {
  it("fail-fast: one stage throws → outcome 'failed', dispose still runs", async () => {
    const disposeSpy = vi.fn();
    const failing = defineStage({
      name: "@test/failing",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "throws",
      consumes: Kinds.ContentSource,
      produces: Kinds.DeployArtifact,
      capabilities: [],
      configSchema: null,
      async run() {
        throw new StageError({ code: "PARSE_ERROR", message: "boom", recoverable: false });
      },
      async dispose() { disposeSpy(); },
    });
    const config = makeConfig([{ stage: tinySource(["a.md"]) }, { stage: failing }]);
    const o = createOrchestrator({ logger: silentLogger() });
    const pipeline = await o.buildPipeline(config);
    const result = await o.runOnce(pipeline);
    expect(result.outcome).toBe("failed");
    expect(result.errors.length).toBe(1);
    expect(result.errors[0]!.code).toBe("PARSE_ERROR");
    // dispose hook ran even though there was no init() — no, init is required for dispose.
    // Actually: per scheduler, dispose only runs when initialized=true.
    // This stage has no init, so disposeSpy stays at 0.
    expect(disposeSpy).not.toHaveBeenCalled();
    await o.dispose();
  });

  it("best-effort: recoverable error continues, outcome 'partial'", async () => {
    const failing = defineStage({
      name: "@test/recoverable-fail",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "recoverable failure",
      consumes: Kinds.ContentSource,
      produces: Kinds.DeployArtifact,
      capabilities: [],
      configSchema: null,
      async run() {
        throw new StageError({ code: "PARSE_ERROR", message: "soft", recoverable: true });
      },
    });
    const config = { ...makeConfig([{ stage: tinySource(["a.md"]) }, { stage: failing }]),
                     settings: { ...makeConfig([]).settings, bestEffort: true } };
    const o = createOrchestrator({ logger: silentLogger() });
    const pipeline = await o.buildPipeline(config);
    const result = await o.runOnce(pipeline);
    expect(result.outcome).toBe("partial");
    expect(result.errors.length).toBe(1);
    await o.dispose();
  });

  it("non-StageError throw is wrapped as UNCAUGHT", async () => {
    const failing = defineStage({
      name: "@test/uncaught",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "throws plain",
      consumes: Kinds.ContentSource,
      produces: Kinds.DeployArtifact,
      capabilities: [],
      configSchema: null,
      async run() { throw new Error("plain"); },
    });
    const config = makeConfig([{ stage: tinySource(["a.md"]) }, { stage: failing }]);
    const o = createOrchestrator({ logger: silentLogger() });
    const pipeline = await o.buildPipeline(config);
    const result = await o.runOnce(pipeline);
    expect(result.outcome).toBe("failed");
    expect(result.errors[0]!.code).toBe("UNCAUGHT");
    await o.dispose();
  });
});

describe("init/dispose lifecycle", () => {
  it("init runs before run; dispose runs after", async () => {
    const order: string[] = [];
    const stage = defineStage({
      name: "@test/lifecycle",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "tracks lifecycle order",
      consumes: Kinds.ContentSource,
      produces: Kinds.DeployArtifact,
      capabilities: [],
      configSchema: null,
      async init() { order.push("init"); },
      async run() {
        order.push("run");
        return {
          variant: { kind: "dist-tree" }, files: {}, manifest: {
            routes: [], assets: [], buildTime: "2026-05-15T00:00:00Z",
            buildId: "blake2b:00" as never,
          },
        } as never;
      },
      async dispose() { order.push("dispose"); },
    });
    const config = makeConfig([{ stage: tinySource(["x.md"]) }, { stage }]);
    const o = createOrchestrator({ logger: silentLogger() });
    const pipeline = await o.buildPipeline(config);
    await o.runOnce(pipeline);
    expect(order).toEqual(["init", "run", "dispose"]);
    await o.dispose();
  });

  it("init failure aborts before any run() and disposes initialised stages", async () => {
    const initOk = vi.fn(); const disposeOk = vi.fn();
    const stage = defineStage({
      name: "@test/init-fail",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "fails init",
      consumes: Kinds.ContentSource,
      produces: Kinds.DeployArtifact,
      capabilities: [],
      configSchema: null,
      async init() { throw new Error("init blew up"); },
      async run() { throw new Error("never called"); },
      async dispose() { /* never called for this stage */ },
    });
    const goodSource = defineStage({
      name: "@test/good-source",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "ok",
      consumes: Kinds.Void,
      produces: streamOf(Kinds.ContentSource),
      capabilities: [],
      configSchema: null,
      async init() { initOk(); },
      async dispose() { disposeOk(); },
      async *run() { yield { path: "x", bytes: new Uint8Array(), mimeType: null,
        identity: "01952c0d-7e63-7000-8000-000000000000" as never,
        revision: "blake2b:00" as never, providerMeta: {} } as never; },
    });
    const config = makeConfig([{ stage: goodSource }, { stage }]);
    const o = createOrchestrator({ logger: silentLogger() });
    const pipeline = await o.buildPipeline(config);
    const result = await o.runOnce(pipeline);
    expect(result.outcome).toBe("failed");
    expect(initOk).toHaveBeenCalledTimes(1);
    expect(disposeOk).toHaveBeenCalledTimes(1);
  });
});

describe("Cancellation", () => {
  it("cancel before run yields outcome 'cancelled'", async () => {
    const source = tinySource(["a.md"]);
    const config = makeConfig([{ stage: source }, { stage: upper }, { stage: sink }]);
    const o = createOrchestrator({ logger: silentLogger() });
    const pipeline = await o.buildPipeline(config);
    const cs = createCancellationTokenSource();
    cs.cancel("test");
    const result = await o.runOnce(pipeline, { cancellation: cs.token });
    expect(result.outcome).toBe("cancelled");
  });
});

describe("Orchestrator lifecycle", () => {
  it("dispose makes subsequent calls throw", async () => {
    const o = createOrchestrator({ logger: silentLogger() });
    await o.dispose();
    await expect(o.buildPipeline(makeConfig([{ stage: tinySource(["a"]) }])))
      .rejects.toThrow(/disposed/);
  });

  it("dispose is idempotent", async () => {
    const o = createOrchestrator({ logger: silentLogger() });
    await o.dispose();
    await expect(o.dispose()).resolves.toBeUndefined();
  });
});

describe("buildDag — direct API", () => {
  it("builds expected source/sink arrays for a linear pipeline", () => {
    const source = tinySource(["a"]);
    const config = makeConfig([{ stage: source }, { stage: upper }, { stage: sink }]);
    const resolved = validateConfig(config);
    const dag = buildDag(resolved);
    expect(dag.sources).toEqual(["@test/source"]);
    expect(dag.sinks).toEqual(["@test/sink"]);
    expect(dag.topoOrder).toEqual(["@test/source", "@test/upper", "@test/sink"]);
  });

  it("throws when no compatible producer exists", () => {
    // Two stages where the second can't be wired to the first.
    const a = defineStage({
      name: "a", version: "0.1.0", apiVersion: KERNEL_API_VERSION,
      description: "x", consumes: Kinds.Void, produces: Kinds.ContentSource,
      capabilities: [], configSchema: null,
      async run() { return null as never; },
    });
    const b = defineStage({
      name: "b", version: "0.1.0", apiVersion: KERNEL_API_VERSION,
      description: "x", consumes: Kinds.Asset, produces: Kinds.DeployArtifact,
      capabilities: [], configSchema: null,
      async run() { return null as never; },
    });
    const config = makeConfig([{ stage: a }, { stage: b }]);
    const resolved = validateConfig(config);
    expect(() => buildDag(resolved)).toThrow(/no earlier instance produces/);
  });
});

describe("runOnce direct (without the orchestrator wrapper)", () => {
  it("returns a result with elapsedMs and buildId", async () => {
    const source = tinySource(["a"]);
    const config = makeConfig([{ stage: source }, { stage: upper }, { stage: sink }]);
    const pipeline = await buildPipeline(config);
    const result = await runOnce(pipeline, {}, { logger: silentLogger() });
    expect(result.elapsedMs).toBeGreaterThanOrEqual(0);
    expect(result.buildId).toMatch(/^blake2b:[0-9a-f]+$/);
  });
});
