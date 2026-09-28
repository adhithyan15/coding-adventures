import { describe, expect, it, vi } from "vitest";
import { KERNEL_API_VERSION, Kinds } from "@coding-adventures/forme-types";
import { defineStage } from "@coding-adventures/forme-stage";
import type { CacheBackend } from "@coding-adventures/forme-cache";
import type { PipelineConfig } from "@coding-adventures/forme-pipeline-config";
import { createOrchestrator } from "../src/index.js";

const resolved = defineStage({
  name: "@example/plugin/source",
  version: "1.0.0",
  apiVersion: KERNEL_API_VERSION,
  description: "resolved fixture",
  consumes: Kinds.Void,
  produces: Kinds.DeployArtifact,
  capabilities: ["storage:read"],
  configSchema: null,
  run() {
    return {
      variant: { kind: "dist-tree" },
      files: {},
      manifest: { routes: [], assets: [], buildTime: "", buildId: "blake2b:00" },
    } as never;
  },
});

const config: PipelineConfig = {
  name: "plugin-resolution",
  settings: {
    storageRoot: ".",
    cacheDir: null,
    reproducibleBuild: true,
    maxConcurrency: 1,
    logLevel: "error",
    bestEffort: false,
    deadlineMs: null,
  },
  stages: [{
    id: "plugin-source",
    stage: { kind: "stage-ref", packageName: "@example/plugin", export: "source" },
    capabilities: ["storage:read"],
  }],
};

describe("PluginHost resolution", () => {
  it("resolves every StageRef before config validation and disposes the host", async () => {
    const loadStage = vi.fn(async () => resolved as never);
    const dispose = vi.fn(async () => undefined);
    const orchestrator = createOrchestrator({ pluginHost: { loadStage, dispose } });
    const pipeline = await orchestrator.buildPipeline(config);
    expect(loadStage).toHaveBeenCalledWith(
      config.stages[0]!.stage,
      "plugin-source",
      ["storage:read"],
    );
    expect(pipeline.config.stages[0]!.stage).toBe(resolved);
    await orchestrator.dispose();
    expect(dispose).toHaveBeenCalledOnce();
  });

  it("lets the resolved stage name become the id for an unnamed plugin stage", async () => {
    const loadStage = vi.fn(async () => resolved as never);
    const orchestrator = createOrchestrator({ pluginHost: { loadStage } });
    await orchestrator.buildPipeline({
      ...config,
      stages: [{ stage: config.stages[0]!.stage }],
    });
    expect(loadStage).toHaveBeenCalledWith(config.stages[0]!.stage, undefined, undefined);
    await orchestrator.dispose();
  });

  it("keeps unresolved StageRefs fail-closed when no host is configured", async () => {
    const orchestrator = createOrchestrator();
    await expect(orchestrator.buildPipeline(config)).rejects.toThrow(/plugin host|StageRef/i);
    await orchestrator.dispose();
  });

  it("rejects malformed instance authority before resolving plugin code", async () => {
    const loadStage = vi.fn(async () => resolved as never);
    const orchestrator = createOrchestrator({ pluginHost: { loadStage } });
    await expect(orchestrator.buildPipeline({
      ...config,
      stages: [{ ...config.stages[0]!, capabilities: "storage:read" } as never],
    })).rejects.toThrow(/capabilities/);
    expect(loadStage).not.toHaveBeenCalled();
    await orchestrator.dispose();
  });

  it("attempts every owned-resource cleanup when one disposer fails", async () => {
    const cacheDispose = vi.fn(async () => { throw new Error("cache cleanup failed"); });
    const hostDispose = vi.fn(async () => undefined);
    const cache: CacheBackend = {
      async get() { return null; }, async put() {}, async invalidate() {},
      async gc() { return 0; }, dispose: cacheDispose,
    };
    const orchestrator = createOrchestrator({
      cache,
      pluginHost: { loadStage: async () => resolved as never, dispose: hostDispose },
    });
    await expect(orchestrator.dispose()).rejects.toBeInstanceOf(AggregateError);
    expect(cacheDispose).toHaveBeenCalledOnce();
    expect(hostDispose).toHaveBeenCalledOnce();
  });
});
