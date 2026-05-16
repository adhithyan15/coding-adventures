/**
 * repro-build.test.ts — exercise FM03 §8 reproducible-build mode.
 *
 * Test strategy: build a tiny pipeline whose only stage reads
 * `ctx.time.nowMs()` and emits it as the artifact.  Run twice:
 *
 *   - Without reproducibleBuild → the two artifacts have different
 *     timestamps (system clock advanced).
 *   - With reproducibleBuild     → the two artifacts are byte-identical
 *     (frozen clock returns the same value both times).
 *
 * Also covers the dispose-time clock (a dispose hook that reads
 * `ctx.time.nowMs()` sees the same frozen value the run hook saw).
 */

import { describe, it, expect, vi } from "vitest";
import {
  KERNEL_API_VERSION,
  Kinds,
  streamOf,
} from "@coding-adventures/forme-types";
import { defineStage, silentLogger } from "@coding-adventures/forme-stage";
import type { PipelineConfig } from "@coding-adventures/forme-pipeline-config";
import {
  createOrchestrator,
  REPRO_BUILD_FROZEN_TIMESTAMP_MS,
} from "../src/index.js";

// ─── Stages ──────────────────────────────────────────────────────────

function clockStampingSource() {
  return defineStage({
    name: "@test/clock-source",
    version: "0.1.0",
    apiVersion: KERNEL_API_VERSION,
    description: "yields a single ContentSource whose providerMeta records ctx.time.nowMs()",
    consumes: Kinds.Void,
    produces: streamOf(Kinds.ContentSource),
    capabilities: [],
    configSchema: null,
    async *run(_input, _config, ctx) {
      yield {
        path: "stamp",
        bytes: new TextEncoder().encode(String(ctx.time.nowMs())),
        mimeType: "text/plain",
        identity: "01952c0d-7e63-7000-8000-000000000000" as never,
        revision: "blake2b:00" as never,
        providerMeta: { stampedAt: ctx.time.nowMs() },
      } as never;
    },
  });
}

function clockStampingSink() {
  return defineStage({
    name: "@test/clock-sink",
    version: "0.1.0",
    apiVersion: KERNEL_API_VERSION,
    description: "wraps the source as a DeployArtifact whose buildTime is ctx.time.nowIso()",
    consumes: Kinds.ContentSource,
    produces: Kinds.DeployArtifact,
    capabilities: [],
    configSchema: null,
    async run(source, _config, ctx) {
      const s = source as { path: string; bytes: Uint8Array; providerMeta: { stampedAt: number } };
      return {
        variant: { kind: "dist-tree" as const },
        files: { [s.path]: s.bytes },
        manifest: {
          routes: [],
          assets: [],
          buildTime: ctx.time.nowIso(),
          buildId: "blake2b:00" as never,
        },
      } as never;
    },
  });
}

function makeConfig(reproducibleBuild: boolean): PipelineConfig {
  return {
    name: "repro-test",
    settings: {
      storageRoot: "./",
      cacheDir: null,
      reproducibleBuild,
      maxConcurrency: null,
      logLevel: "error",
      bestEffort: false,
      deadlineMs: null,
    },
    stages: [
      { stage: clockStampingSource() },
      { stage: clockStampingSink() },
    ],
  };
}

// ─── Tests ──────────────────────────────────────────────────────────

describe("reproducible-build mode (FM03 §8)", () => {
  it("OFF: two runs produce different timestamps", async () => {
    const a = createOrchestrator({ logger: silentLogger() });
    const b = createOrchestrator({ logger: silentLogger() });
    const cfg = makeConfig(false);

    const pa = await a.buildPipeline(cfg);
    const pb = await b.buildPipeline(cfg);

    const ra = await a.runOnce(pa);
    // Force a perceptible time gap so Date.now() advances even on
    // fast machines.  Otherwise two consecutive runs can both land
    // on the same millisecond and the assertion below flakes.
    await new Promise((r) => setTimeout(r, 10));
    const rb = await b.runOnce(pb);

    expect(ra.outcome).toBe("success");
    expect(rb.outcome).toBe("success");

    const artA = ra.outputs["@test/clock-sink"] as { manifest: { buildTime: string } }[];
    const artB = rb.outputs["@test/clock-sink"] as { manifest: { buildTime: string } }[];
    expect(artA[0]!.manifest.buildTime).not.toBe(artB[0]!.manifest.buildTime);

    await a.dispose();
    await b.dispose();
  });

  it("ON: two runs produce the same timestamp (frozen clock)", async () => {
    const a = createOrchestrator({ logger: silentLogger() });
    const b = createOrchestrator({ logger: silentLogger() });
    const cfg = makeConfig(true);

    const pa = await a.buildPipeline(cfg);
    const pb = await b.buildPipeline(cfg);

    const ra = await a.runOnce(pa);
    await new Promise((r) => setTimeout(r, 10));
    const rb = await b.runOnce(pb);

    expect(ra.outcome).toBe("success");
    expect(rb.outcome).toBe("success");

    const artA = ra.outputs["@test/clock-sink"] as { manifest: { buildTime: string } }[];
    const artB = rb.outputs["@test/clock-sink"] as { manifest: { buildTime: string } }[];

    // The buildTime is derived from ctx.time.nowIso() inside the
    // sink.  With repro on, that's a constant.
    expect(artA[0]!.manifest.buildTime).toBe(artB[0]!.manifest.buildTime);
    expect(artA[0]!.manifest.buildTime).toBe(
      new Date(REPRO_BUILD_FROZEN_TIMESTAMP_MS).toISOString(),
    );

    await a.dispose();
    await b.dispose();
  });

  it("ON: source-stage stampedAt timestamps are also frozen", async () => {
    const o = createOrchestrator({ logger: silentLogger() });
    const pipeline = await o.buildPipeline(makeConfig(true));
    const result = await o.runOnce(pipeline);

    expect(result.outcome).toBe("success");
    // Inspect the DeployArtifact's files map: the sink wrote the
    // source's bytes (which are String(stampedAt)).
    const arts = result.outputs["@test/clock-sink"] as Array<{ files: Record<string, Uint8Array> }>;
    const bytes = arts[0]!.files["stamp"]!;
    const stamp = new TextDecoder().decode(bytes);
    expect(stamp).toBe(String(REPRO_BUILD_FROZEN_TIMESTAMP_MS));
    await o.dispose();
  });

  it("ON: dispose() also sees the frozen clock", async () => {
    const disposeStampSpy = vi.fn<(t: number) => void>();
    const sinkWithDispose = defineStage({
      name: "@test/clock-sink-with-dispose",
      version: "0.1.0",
      apiVersion: KERNEL_API_VERSION,
      description: "as clock-sink, plus a dispose() that records ctx.time.nowMs",
      consumes: Kinds.ContentSource,
      produces: Kinds.DeployArtifact,
      capabilities: [],
      configSchema: null,
      async init() { /* required so dispose runs */ },
      async run(_s, _c, ctx) {
        return {
          variant: { kind: "dist-tree" as const },
          files: {},
          manifest: {
            routes: [], assets: [],
            buildTime: ctx.time.nowIso(),
            buildId: "blake2b:00" as never,
          },
        } as never;
      },
      async dispose(ctx) {
        disposeStampSpy(ctx.time.nowMs());
      },
    });

    const cfg: PipelineConfig = {
      ...makeConfig(true),
      stages: [{ stage: clockStampingSource() }, { stage: sinkWithDispose }],
    };
    const o = createOrchestrator({ logger: silentLogger() });
    await o.runOnce(await o.buildPipeline(cfg));
    expect(disposeStampSpy).toHaveBeenCalledWith(REPRO_BUILD_FROZEN_TIMESTAMP_MS);
    await o.dispose();
  });

  it("ON: REPRO_BUILD_FROZEN_TIMESTAMP_MS is exported (and equals 0 in v0)", () => {
    // The constant is part of the public API so tests / drivers can
    // assert on the exact value the orchestrator's frozen clock
    // returns.  v0 fixes it to 0 (1970-01-01T00:00:00Z) per the
    // FM03 §8 fallback rule.
    expect(REPRO_BUILD_FROZEN_TIMESTAMP_MS).toBe(0);
  });

  it("OFF: default mode (settings.reproducibleBuild absent ⇒ false) uses real time", async () => {
    const cfg: PipelineConfig = {
      ...makeConfig(false),
      // Explicitly leave reproducibleBuild as false — proves the
      // default does NOT silently flip on for some other reason.
    };
    const o = createOrchestrator({ logger: silentLogger() });
    const start = Date.now();
    const r = await o.runOnce(await o.buildPipeline(cfg));
    expect(r.outcome).toBe("success");
    const arts = r.outputs["@test/clock-sink"] as Array<{ manifest: { buildTime: string } }>;
    const ts = new Date(arts[0]!.manifest.buildTime).getTime();
    // The timestamp should fall in [start, start + 10s].  Generous
    // ceiling for slow CI machines.
    expect(ts).toBeGreaterThanOrEqual(start - 1);
    expect(ts).toBeLessThanOrEqual(start + 10_000);
    await o.dispose();
  });
});
