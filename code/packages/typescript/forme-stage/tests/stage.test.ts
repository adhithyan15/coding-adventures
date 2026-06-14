/**
 * forme-stage — Stage interface + defineStage tests
 */

import { describe, it, expect } from "vitest";
import { Kinds, streamOf } from "@coding-adventures/forme-types";
import {
  defineStage,
  inMemoryCache,
  inMemoryEventBus,
  neverCancelledToken,
  noOpTelemetryEmitter,
  silentLogger,
  systemClock,
  deniedEnvApi,
  deniedFilesystemApi,
  deniedNetworkApi,
  deniedShellApi,
  deniedStorageApi,
} from "../src/index.js";
import type { Stage, StageContext } from "../src/index.js";

function makeContext(): StageContext {
  return {
    logger: silentLogger(),
    cancellation: neverCancelledToken(),
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

describe("defineStage — type narrowing", () => {
  it("returns the same object reference (identity at runtime)", () => {
    const obj = {
      name: "test",
      version: "0.1.0",
      apiVersion: 1,
      description: "x",
      consumes: Kinds.Void,
      produces: Kinds.ContentSource,
      capabilities: [],
      configSchema: null,
      run() { throw new Error("not called"); },
    };
    const stage = defineStage(obj);
    expect(stage).toBe(obj);
  });

  it("preserves the precise consumes/produces descriptor types", () => {
    const stage = defineStage({
      name: "src",
      version: "0.1.0",
      apiVersion: 1,
      description: "produces a single content source",
      consumes: Kinds.Void,
      produces: Kinds.ContentSource,
      capabilities: [],
      configSchema: null,
      async run() {
        return {
          path: "x", bytes: new Uint8Array(), mimeType: null,
          identity: "01952c0d-7e63-7000-8000-000000000000" as never,
          revision: "blake2b:00" as never,
          providerMeta: {},
        };
      },
    });
    expect(stage.produces.name).toBe("ContentSource");
  });
});

describe("Stage execution paths", () => {
  it("synchronous one-to-one", async () => {
    const stage: Stage<typeof Kinds.Void, typeof Kinds.RenderedPage> = {
      name: "sync",
      version: "0.1.0",
      apiVersion: 1,
      description: "x",
      consumes: Kinds.Void,
      produces: Kinds.RenderedPage,
      capabilities: [],
      configSchema: null,
      run(_input, _config, _ctx) {
        return {
          route: "/", html: "<!doctype html>",
          usedStyle: [], usedIslands: [], usedAssets: [],
          meta: {
            title: "x", description: null, canonicalUrl: null,
            openGraph: {}, structured: [], extra: {},
          },
          source: "01952c0d-7e63-7000-8000-000000000000" as never,
        };
      },
    };
    const result = await stage.run(undefined as never, null, makeContext());
    expect((result as { html: string }).html).toBe("<!doctype html>");
  });

  it("async one-to-one returns a promise", async () => {
    const stage = defineStage({
      name: "async",
      version: "0.1.0",
      apiVersion: 1,
      description: "x",
      consumes: Kinds.Void,
      produces: Kinds.ContentSource,
      capabilities: [],
      configSchema: null,
      async run() {
        return {
          path: "x", bytes: new Uint8Array(), mimeType: null,
          identity: "01952c0d-7e63-7000-8000-000000000000" as never,
          revision: "blake2b:00" as never,
          providerMeta: {},
        };
      },
    });
    const result = await stage.run(undefined as never, null, makeContext());
    expect(result.path).toBe("x");
  });

  it("streaming output via AsyncIterable", async () => {
    const stage = defineStage({
      name: "stream",
      version: "0.1.0",
      apiVersion: 1,
      description: "x",
      consumes: Kinds.Void,
      produces: streamOf(Kinds.ContentSource),
      capabilities: [],
      configSchema: null,
      async *run() {
        for (let i = 0; i < 3; i++) {
          yield {
            path: `${i}.md`, bytes: new Uint8Array(), mimeType: null,
            identity: "01952c0d-7e63-7000-8000-000000000000" as never,
            revision: "blake2b:00" as never,
            providerMeta: {},
          } as never;
        }
      },
    });
    const out: unknown[] = [];
    const iter = stage.run(undefined as never, null, makeContext()) as AsyncIterable<unknown>;
    for await (const v of iter) out.push(v);
    expect(out.length).toBe(3);
  });
});

describe("Stage lifecycle hooks (init/dispose)", () => {
  it("init and dispose are optional", () => {
    const stage = defineStage({
      name: "no-hooks",
      version: "0.1.0",
      apiVersion: 1,
      description: "x",
      consumes: Kinds.Void,
      produces: Kinds.ContentSource,
      capabilities: [],
      configSchema: null,
      async run() {
        return {
          path: "x", bytes: new Uint8Array(), mimeType: null,
          identity: "01952c0d-7e63-7000-8000-000000000000" as never,
          revision: "blake2b:00" as never,
          providerMeta: {},
        };
      },
    });
    expect(stage.init).toBeUndefined();
    expect(stage.dispose).toBeUndefined();
  });

  it("init/dispose are async functions when present", async () => {
    let initCalled = false;
    let disposeCalled = false;
    const stage = defineStage({
      name: "with-hooks",
      version: "0.1.0",
      apiVersion: 1,
      description: "x",
      consumes: Kinds.Void,
      produces: Kinds.ContentSource,
      capabilities: [],
      configSchema: null,
      async run() {
        return {
          path: "x", bytes: new Uint8Array(), mimeType: null,
          identity: "01952c0d-7e63-7000-8000-000000000000" as never,
          revision: "blake2b:00" as never,
          providerMeta: {},
        };
      },
      async init() { initCalled = true; },
      async dispose() { disposeCalled = true; },
    });
    const ctx = { ...makeContext(), config: null };
    // Note: cast needed because StageInitContext omits cancellation/cache.
    await stage.init?.(null, ctx as never);
    await stage.dispose?.(ctx as never);
    expect(initCalled).toBe(true);
    expect(disposeCalled).toBe(true);
  });
});
