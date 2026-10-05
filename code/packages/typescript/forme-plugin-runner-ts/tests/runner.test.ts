import { PassThrough } from "node:stream";
import { describe, expect, it } from "vitest";
import { CancellationError, StageError } from "@coding-adventures/forme-errors";
import { defineStage } from "@coding-adventures/forme-stage";
import { KERNEL_API_VERSION, Kinds, streamOf } from "@coding-adventures/forme-types";
import {
  FrameDecoder,
  encodeFrame,
  runPlugin,
  type JsonRpcMessage,
  type RunPluginOptions,
} from "../src/index.js";

class HostDriver {
  readonly input = new PassThrough();
  readonly output = new PassThrough();
  readonly notifications: JsonRpcMessage[] = [];
  private readonly decoder = new FrameDecoder({ maxFrameBytes: 1024 * 1024, maxHeaderBytes: 1024 });
  private readonly pending = new Map<number, { resolve(value: unknown): void; reject(error: unknown): void }>();
  private nextId = 1;
  handler: (method: string, params: Record<string, unknown>) => Promise<unknown> = async () => null;
  notificationHandler: (method: string, params: Record<string, unknown>) => void = () => undefined;

  constructor() {
    this.output.on("data", chunk => {
      for (const message of this.decoder.push(chunk)) void this.receive(message);
    });
  }

  request(method: string, params: unknown): Promise<unknown> {
    const id = this.nextId++;
    const result = new Promise<unknown>((resolve, reject) => this.pending.set(id, { resolve, reject }));
    this.input.write(encodeFrame({ jsonrpc: "2.0", id, method, params }));
    return result;
  }

  notify(method: string, params: unknown): void {
    this.input.write(encodeFrame({ jsonrpc: "2.0", method, params }));
  }

  close(): void { this.input.end(); }

  private async receive(message: JsonRpcMessage): Promise<void> {
    if (typeof message.method === "string") {
      if (message.id === undefined) {
        this.notifications.push(message);
        this.notificationHandler(message.method, (message.params ?? {}) as Record<string, unknown>);
        return;
      }
      try {
        const result = await this.handler(message.method, (message.params ?? {}) as Record<string, unknown>);
        this.input.write(encodeFrame({ jsonrpc: "2.0", id: message.id, result }));
      } catch (error) {
        const fault = error as { code?: number; message?: string; data?: unknown };
        this.input.write(encodeFrame({
          jsonrpc: "2.0",
          id: message.id,
          error: { code: fault.code ?? -32603, message: fault.message ?? "INTERNAL_ERROR", data: fault.data },
        }));
      }
      return;
    }
    const pending = typeof message.id === "number" ? this.pending.get(message.id) : undefined;
    if (!pending) return;
    this.pending.delete(message.id as number);
    if (message.error) pending.reject(message.error);
    else pending.resolve(message.result);
  }
}

function start(
  stage: ReturnType<typeof defineStage>,
  driver = new HostDriver(),
  overrides: Partial<RunPluginOptions> = {},
) {
  const completed = runPlugin(stage, {
    input: driver.input,
    output: driver.output,
    argv: ["node", "plugin.mjs", "echo", "sha256:schema"],
    installSignalHandlers: false,
    maxFrameBytes: 1024 * 1024,
    maxHeaderBytes: 1024,
    ...overrides,
  });
  return { driver, completed };
}

const echoStage = defineStage({
  name: "@example/echo",
  version: "1.0.0",
  apiVersion: KERNEL_API_VERSION,
  description: "runner conformance echo",
  consumes: Kinds.ContentNode,
  produces: Kinds.ContentNode,
  capabilities: ["storage:read", "storage:write", "env:ALLOWED", "filesystem:user", "network:example.com", "system:time:wallclock"],
  configSchema: { type: "object" },
  async run(input, _config, ctx) {
    const value = input as Record<string, unknown>;
    if (value.stageError) throw new StageError({ code: "FIXTURE", message: "fixture failed", fields: { line: 3 } });
    if (value.waitForCancel) {
      while (true) {
        ctx.cancellation.throwIfCancelled();
        await new Promise(resolve => setTimeout(resolve, 2));
      }
    }
    if (value.exerciseContext) {
      const bytes = await ctx.storage.read("posts/a.md");
      await ctx.storage.write("out/a.md", bytes);
      const entries = [];
      for await (const entry of ctx.storage.list("posts")) entries.push(entry);
      const response = await ctx.network.fetch("https://example.com/data");
      const watched = [];
      for await (const change of ctx.storage.watch("posts")) {
        watched.push(change);
        break;
      }
      ctx.logger.info("runner fixture", { ok: true });
      return {
        bytes,
        exists: await ctx.storage.exists("posts/a.md"),
        stat: await ctx.storage.stat("posts/a.md"),
        entries,
        watched,
        env: await ctx.env.get("ALLOWED"),
        nowMs: await ctx.time.nowMs(),
        nowIso: await ctx.time.nowIso(),
        monotonic: ctx.time.monotonicMs(),
        home: await ctx.filesystem.homeDir(),
        temp: await ctx.filesystem.tempDir(),
        absolute: await ctx.filesystem.readAbsolute("/safe/a"),
        status: response.status,
        body: new Uint8Array(await response.arrayBuffer()),
      } as never;
    }
    return input;
  },
});

async function handshake(driver: HostDriver): Promise<void> {
  await expect(driver.request("handshake", {
    hostName: "test", hostVersion: "1", apiVersion: 2, protocolVersion: 1,
    pluginName: "@example/echo", pluginVersion: "1.0.0", manifestHash: "sha256:x",
    instanceId: "echo-1", trustTier: "unverified-third-party",
  })).resolves.toMatchObject({
    pluginName: "@example/echo", pluginVersion: "1.0.0", apiVersion: 2,
    protocolVersion: 1, runner: "@coding-adventures/forme-plugin-runner-ts",
  });
  await expect(driver.request("announce", {})).resolves.toEqual({
    stage: {
      id: "echo",
      consumes: "ContentNode",
      produces: "ContentNode",
      capabilities: echoStage.capabilities,
      configSchemaHash: "sha256:schema",
    },
  });
  await expect(driver.request("stage.init", { config: {}, instanceId: "echo-1", logLevel: "info" }))
    .resolves.toBeNull();
}

describe("TypeScript plugin runner", () => {
  it("implements handshake, announcement, init, single run, and disposal", async () => {
    const { driver, completed } = start(echoStage);
    await handshake(driver);
    await expect(driver.request("stage.run", { input: { ok: true }, config: {}, streamId: 10 }))
      .resolves.toEqual({ kind: "single", value: { ok: true } });
    await expect(driver.request("stage.dispose", {})).resolves.toBeNull();
    await expect(driver.request("stage.dispose", {})).resolves.toBeNull();
    driver.close();
    await expect(completed).resolves.toBeUndefined();
  });

  it("provides the complete wire-backed context and binary envelopes", async () => {
    const { driver, completed } = start(echoStage);
    let watchResult: unknown = { kind: "stream-handle", streamId: 701 };
    driver.notificationHandler = (method, params) => {
      if (method === "stream.start") {
        driver.notify("stream.value", {
          streamId: params.streamId,
          value: { path: "posts/a.md", kind: "modified" },
        });
        for (let index = 0; index < 8; index += 1) {
          driver.notify("stream.value", {
            streamId: params.streamId,
            value: { path: `posts/late-${index}.md`, kind: "modified" },
          });
        }
      }
    };
    driver.handler = async (method, params) => {
      if (method === "stream.cancel") {
        expect(params).toEqual({ streamId: 11, capabilityStreamId: 701 });
        return null;
      }
      const replies: Record<string, unknown> = {
        "ctx.storage.read": { bytes: Buffer.from("hello").toString("base64") },
        "ctx.storage.write": null,
        "ctx.storage.exists": true,
        "ctx.storage.stat": { size: 5, mtimeMs: 0, type: "file" },
        "ctx.storage.list": [{ path: "posts/a.md", type: "file" }],
        "ctx.storage.watch": watchResult,
        "ctx.env.get": "secret",
        "ctx.time.nowMs": 42,
        "ctx.time.nowIso": "1970-01-01T00:00:00.042Z",
        "ctx.filesystem.homeDir": "/home/test",
        "ctx.filesystem.tempDir": "/tmp/test",
        "ctx.filesystem.readAbsolute": { bytes: Buffer.from("absolute").toString("base64") },
        "ctx.network.fetch": {
          status: 201, statusText: "Created", headers: { "x-test": "yes" },
          bytes: Buffer.from("network").toString("base64"), url: "https://example.com/data",
        },
      };
      expect([11, 12, 13]).toContain(params.streamId);
      return replies[method];
    };
    await handshake(driver);
    const result = await driver.request("stage.run", {
      input: { exerciseContext: true, collision: { $forme: "bytes", base64: "plain" } },
      config: {}, streamId: 11,
    }) as { value: Record<string, unknown> };
    expect(result.value).toMatchObject({
      bytes: new Uint8Array(Buffer.from("hello")), env: "secret", nowMs: 42,
      home: "/home/test", status: 201, body: new Uint8Array(Buffer.from("network")),
      watched: [{ path: "posts/a.md", kind: "modified" }],
    });
    expect(driver.notifications).toContainEqual(expect.objectContaining({
      method: "log", params: { level: "info", message: "runner fixture", fields: { ok: true } },
    }));
    watchResult = {};
    await expect(driver.request("stage.run", {
      input: { exerciseContext: true }, config: {}, streamId: 12,
    })).rejects.toMatchObject({ code: -32004 });
    watchResult = { kind: "stream-handle", streamId: 0 };
    await expect(driver.request("stage.run", {
      input: { exerciseContext: true }, config: {}, streamId: 13,
    })).rejects.toMatchObject({ code: -32004 });
    await driver.request("stage.dispose", {});
    driver.close();
    await completed;
  });

  it("bridges stream input and output in order", async () => {
    const streamStage = defineStage({
      ...echoStage,
      consumes: streamOf(Kinds.ContentNode),
      produces: streamOf(Kinds.ContentNode),
      async *run(input) { for await (const value of input) yield value; },
    });
    const { driver, completed } = start(streamStage);
    await driver.request("handshake", {
      hostName: "test", hostVersion: "1", apiVersion: 2, protocolVersion: 1,
      pluginName: "@example/echo", pluginVersion: "1.0.0", manifestHash: "sha256:x",
      instanceId: "echo-1", trustTier: "unverified-third-party",
    });
    await driver.request("announce", {});
    await driver.request("stage.init", { config: {}, instanceId: "echo-1", logLevel: "info" });
    const run = driver.request("stage.run", {
      input: { kind: "stream-handle", streamId: 20 }, config: {}, streamId: 21,
    });
    driver.notify("stream.value", { streamId: 20, value: { n: 1 } });
    driver.notify("stream.value", { streamId: 20, value: { n: 2 } });
    driver.notify("stream.end", { streamId: 20 });
    await expect(run).resolves.toEqual({ kind: "stream", streamId: 21, produced: 2 });
    expect(driver.notifications.filter(message => message.method === "stream.value").map(message => message.params))
      .toEqual([{ streamId: 21, value: { n: 1 } }, { streamId: 21, value: { n: 2 } }]);
    const failed = driver.request("stage.run", {
      input: { kind: "stream-handle", streamId: 22 }, config: {}, streamId: 23,
    });
    driver.notify("stream.error", { streamId: 22 });
    await expect(failed).rejects.toMatchObject({ code: -32900 });
    await driver.request("stage.dispose", {});
    driver.close();
    await completed;
  });

  it("translates StageError and cooperative cancellation", async () => {
    const { driver, completed } = start(echoStage);
    await handshake(driver);
    await expect(driver.request("stage.run", { input: { stageError: true }, config: {}, streamId: 30 }))
      .rejects.toMatchObject({ code: -32900, data: { stageErrorCode: "FIXTURE", fields: { line: 3 } } });
    const cancelled = driver.request("stage.run", { input: { waitForCancel: true }, config: {}, streamId: 31 });
    await new Promise(resolve => setTimeout(resolve, 10));
    await expect(driver.request("stage.run", { input: {}, config: {}, streamId: 32 }))
      .rejects.toMatchObject({ code: -32004 });
    driver.notify("$/cancelRequest", { id: 5 });
    await expect(cancelled).rejects.toMatchObject({ code: -32800 });
    await driver.request("stage.dispose", {});
    driver.close();
    await completed;
  });

  it("rejects malformed frames and bounds outbound payloads", async () => {
    const decoder = new FrameDecoder({ maxFrameBytes: 8, maxHeaderBytes: 32 });
    expect(() => decoder.push(Buffer.from("Content-Length: 9\r\n\r\n123456789")))
      .toThrow(/configured bound/);
    expect(() => encodeFrame({ jsonrpc: "2.0", result: new Uint8Array(100) }, 32))
      .toThrow(/configured bound/);
    expect(() => new FrameDecoder({ maxFrameBytes: 100, maxHeaderBytes: 8 })
      .push(Buffer.from("Content-Length: 1\r\n\r\n{}"))).toThrow(/header/);
    expect(new CancellationError("x").reason).toBe("x");
  });

  it("refuses a stage targeting legacy kernel API v1", async () => {
    const legacy = { ...echoStage, apiVersion: 1 };
    await expect(runPlugin(legacy as typeof echoStage, { installSignalHandlers: false }))
      .rejects.toThrow(/apiVersion is unsupported/);
  });

  it("rejects invalid stage metadata, descriptors, argv, and resource limits", async () => {
    const invalidStages = [
      { ...echoStage, name: "" },
      { ...echoStage, version: "" },
      { ...echoStage, description: "x".repeat(16_385) },
      { ...echoStage, apiVersion: KERNEL_API_VERSION + 1 },
      { ...echoStage, capabilities: "bad" },
      { ...echoStage, capabilities: Array.from({ length: 257 }, () => "x") },
      { ...echoStage, capabilities: [""] },
      { ...echoStage, consumes: { name: "Stream" } },
      { ...echoStage, produces: { name: "Stream", inner: { name: "Stream", inner: Kinds.ContentNode } } },
    ];
    for (const stage of invalidStages) {
      await expect(runPlugin(stage as ReturnType<typeof defineStage>, { installSignalHandlers: false }))
        .rejects.toBeInstanceOf(Error);
    }
    await expect(runPlugin(echoStage, { argv: ["node", "x", ""], installSignalHandlers: false }))
      .rejects.toThrow(/stage id/);
    await expect(runPlugin(echoStage, { argv: ["node", "x", "echo", ""], installSignalHandlers: false }))
      .rejects.toThrow(/schema identity/);
    for (const overrides of [
      { maxFrameBytes: 0 }, { maxHeaderBytes: -1 },
      { maxBufferedStreamValues: 1.5 }, { maxBufferedStreamBytes: Number.MAX_VALUE },
      { signalShutdownTimeoutMs: 0 },
    ]) {
      await expect(runPlugin(echoStage, { ...overrides, installSignalHandlers: false }))
        .rejects.toBeInstanceOf(RangeError);
    }
  });

  it("enforces lifecycle order, manifest identity, parameter ids, and known methods", async () => {
    const { driver, completed } = start(echoStage);
    await expect(driver.request("announce", {})).rejects.toMatchObject({ code: -32004 });
    await expect(driver.request("handshake", {
      pluginName: "wrong", pluginVersion: "1.0.0", apiVersion: 2, protocolVersion: 1,
    })).rejects.toMatchObject({ code: -32006, data: { field: "pluginName" } });
    await expect(driver.request("handshake", {
      pluginName: "@example/echo", pluginVersion: "wrong", apiVersion: 2, protocolVersion: 1,
    })).rejects.toMatchObject({ code: -32006, data: { field: "pluginVersion" } });
    await expect(driver.request("handshake", {
      pluginName: "@example/echo", pluginVersion: "1.0.0", apiVersion: 1, protocolVersion: 1,
    })).rejects.toMatchObject({ code: -32006, data: { field: "apiVersion" } });
    await expect(driver.request("handshake", {
      pluginName: "@example/echo", pluginVersion: "1.0.0", apiVersion: 2, protocolVersion: 2,
    })).rejects.toMatchObject({ code: -32006, data: { field: "protocolVersion" } });
    await handshake(driver);
    await expect(driver.request("handshake", {})).rejects.toMatchObject({ code: -32004 });
    await expect(driver.request("stage.init", { config: { once: true } })).resolves.toBeNull();
    await expect(driver.request("unknown", {})).rejects.toMatchObject({ code: -32601 });
    await expect(driver.request("unknown", [])).rejects.toMatchObject({ code: -32603 });
    await expect(driver.request("stage.run", { input: {}, config: {}, streamId: -1 }))
      .rejects.toMatchObject({ code: -32602, data: { field: "streamId" } });
    driver.notify("$/cancelRequest", { id: 999 });
    driver.notify("$/cancelRequest", { id: 999, reason: "ignored" });
    await expect(driver.request("stage.dispose", {})).resolves.toBeNull();
    driver.close();
    await completed;
  });

  it("calls init and dispose once and announces default id without a schema hash", async () => {
    let initialized = 0;
    let disposed = 0;
    const lifecycle = defineStage({
      ...echoStage,
      name: "echo",
      async init(config, context) {
        initialized += 1;
        expect(context.config).toEqual(config);
        context.logger.debug("init");
      },
      async dispose(context) {
        disposed += 1;
        expect(context.config).toEqual({ configured: true });
      },
    });
    const driver = new HostDriver();
    const { completed } = start(lifecycle, driver, { argv: ["node", "plugin.mjs"] });
    await driver.request("handshake", {
      pluginName: "echo", pluginVersion: "1.0.0", apiVersion: 2, protocolVersion: 1,
    });
    await expect(driver.request("announce", {})).resolves.toMatchObject({
      stage: { id: "echo", configSchemaHash: null },
    });
    await driver.request("stage.init", { config: { configured: true } });
    await driver.request("stage.init", { config: { ignored: true } });
    await driver.request("stage.dispose", {});
    driver.close();
    await completed;
    expect({ initialized, disposed }).toEqual({ initialized: 1, disposed: 1 });
  });

  it("rejects concurrent init and dispose requests while hooks are awaiting", async () => {
    let releaseInit!: () => void;
    let releaseDispose!: () => void;
    const initGate = new Promise<void>(resolve => { releaseInit = resolve; });
    const disposeGate = new Promise<void>(resolve => { releaseDispose = resolve; });
    let initCalls = 0;
    let disposeCalls = 0;
    const delayed = defineStage({
      ...echoStage,
      async init() { initCalls += 1; await initGate; },
      async dispose() { disposeCalls += 1; await disposeGate; },
    });
    const { driver, completed } = start(delayed);
    await driver.request("handshake", {
      pluginName: "@example/echo", pluginVersion: "1.0.0", apiVersion: 2, protocolVersion: 1,
    });
    await driver.request("announce", {});
    const initializing = driver.request("stage.init", { config: {} });
    await new Promise(resolve => setTimeout(resolve, 0));
    await expect(driver.request("stage.init", { config: {} })).rejects.toMatchObject({
      code: -32004, data: { phase: "initializing" },
    });
    await expect(driver.request("stage.dispose", {})).rejects.toMatchObject({
      code: -32004, data: { phase: "initializing" },
    });
    releaseInit();
    await expect(initializing).resolves.toBeNull();
    expect(initCalls).toBe(1);

    const disposing = driver.request("stage.dispose", {});
    await new Promise(resolve => setTimeout(resolve, 0));
    await expect(driver.request("stage.dispose", {})).rejects.toMatchObject({
      code: -32004, data: { phase: "disposing" },
    });
    releaseDispose();
    await expect(disposing).resolves.toBeNull();
    driver.close();
    await completed;
    expect(disposeCalls).toBe(1);
  });

  it("maps generic failures and rejects mismatched single and stream stage outputs", async () => {
    const cases = [
      defineStage({ ...echoStage, async run() { throw new Error("secret"); } }),
      defineStage({ ...echoStage, async *run() { yield { bad: true } as never; } }),
      defineStage({ ...echoStage, produces: streamOf(Kinds.ContentNode), async run() { return { bad: true } as never; } }),
    ];
    const expectedCodes = [-32603, -32004, -32004];
    for (const [index, stage] of cases.entries()) {
      const { driver, completed } = start(stage);
      await driver.request("handshake", {
        pluginName: "@example/echo", pluginVersion: "1.0.0", apiVersion: 2, protocolVersion: 1,
      });
      await driver.request("announce", {});
      await driver.request("stage.init", { config: {} });
      await expect(driver.request("stage.run", { input: {}, config: {}, streamId: 1 }))
        .rejects.toMatchObject({ code: expectedCodes[index] });
      await driver.request("stage.dispose", {});
      driver.close();
      await completed;
    }
  });

  it("maps upstream stream errors and bounds buffered binary values by encoded size", async () => {
    const streamStage = defineStage({
      ...echoStage,
      consumes: streamOf(Kinds.ContentNode),
      async run(input) {
        for await (const value of input) return value as never;
        return null as never;
      },
    });
    const { driver, completed } = start(streamStage, new HostDriver(), {
      maxBufferedStreamValues: 1,
      maxBufferedStreamBytes: 32,
    });
    await driver.request("handshake", {
      pluginName: "@example/echo", pluginVersion: "1.0.0", apiVersion: 2, protocolVersion: 1,
    });
    await driver.request("announce", {});
    await driver.request("stage.init", { config: {} });
    const run = driver.request("stage.run", {
      input: { kind: "stream-handle", streamId: 50 }, config: {}, streamId: 51,
    });
    driver.notify("stream.error", { streamId: 50 });
    await expect(run).rejects.toMatchObject({ code: -32900, data: { stageErrorCode: "UPSTREAM_STREAM_ERROR" } });
    driver.notify("stream.value", { streamId: 999, value: 1 });
    await expect(completed).rejects.toBeInstanceOf(Error);

    const driver2 = new HostDriver();
    const second = start(streamStage, driver2, { maxBufferedStreamBytes: 16 });
    await driver2.request("handshake", {
      pluginName: "@example/echo", pluginVersion: "1.0.0", apiVersion: 2, protocolVersion: 1,
    });
    await driver2.request("announce", {});
    await driver2.request("stage.init", { config: {} });
    const run2 = driver2.request("stage.run", {
      input: { kind: "stream-handle", streamId: 60 }, config: {}, streamId: 61,
    });
    driver2.notify("stream.value", { streamId: 60, value: new Uint8Array(20) });
    void run2;
    await expect(second.completed).rejects.toBeInstanceOf(Error);
  });

  it("binds canonical stream handles to declared Stream inputs only", async () => {
    const ordinary = start(echoStage);
    await handshake(ordinary.driver);
    const handleShaped = { kind: "stream-handle", streamId: 80 };
    await expect(ordinary.driver.request("stage.run", {
      input: handleShaped, config: {}, streamId: 81,
    })).resolves.toEqual({ kind: "single", value: handleShaped });
    await ordinary.driver.request("stage.dispose", {});
    ordinary.driver.close();
    await ordinary.completed;

    const streamStage = defineStage({
      ...echoStage,
      consumes: streamOf(Kinds.ContentNode),
      async run(input) {
        for await (const value of input) return value as never;
        return null as never;
      },
    });
    const streaming = start(streamStage);
    await streaming.driver.request("handshake", {
      pluginName: "@example/echo", pluginVersion: "1.0.0", apiVersion: 2, protocolVersion: 1,
    });
    await streaming.driver.request("announce", {});
    await streaming.driver.request("stage.init", { config: {} });
    await expect(streaming.driver.request("stage.run", {
      input: { ordinary: true }, config: {}, streamId: 82,
    })).rejects.toMatchObject({ code: -32004 });
    await expect(streaming.driver.request("stage.run", {
      input: { kind: "stream-handle", streamId: 83, extra: true }, config: {}, streamId: 84,
    })).rejects.toMatchObject({ code: -32004 });
    await streaming.driver.request("stage.dispose", {});
    streaming.driver.close();
    await streaming.completed;
  });

  it("performs one-shot signal shutdown for active and idle stages", async () => {
    const beforeInt = process.listenerCount("SIGINT");
    const beforeTerm = process.listenerCount("SIGTERM");
    let activeDisposals = 0;
    const activeStage = defineStage({
      ...echoStage,
      async dispose() { activeDisposals += 1; },
    });
    const { driver, completed } = start(activeStage, new HostDriver(), {
      installSignalHandlers: true,
      signalShutdownTimeoutMs: 100,
    });
    expect(process.listenerCount("SIGINT")).toBe(beforeInt + 1);
    expect(process.listenerCount("SIGTERM")).toBe(beforeTerm + 1);
    await handshake(driver);
    const running = driver.request("stage.run", { input: { waitForCancel: true }, config: {}, streamId: 70 });
    await new Promise(resolve => setTimeout(resolve, 10));
    process.emit("SIGINT");
    process.emit("SIGINT");
    await expect(running).rejects.toMatchObject({ code: -32800 });
    await completed;
    expect(activeDisposals).toBe(1);
    expect(process.listenerCount("SIGINT")).toBe(beforeInt);
    expect(process.listenerCount("SIGTERM")).toBe(beforeTerm);

    let idleDisposals = 0;
    const idleStage = defineStage({ ...echoStage, async dispose() { idleDisposals += 1; } });
    const idle = start(idleStage, new HostDriver(), {
      installSignalHandlers: true,
      signalShutdownTimeoutMs: 100,
    });
    await handshake(idle.driver);
    process.emit("SIGTERM");
    await idle.completed;
    expect(idleDisposals).toBe(1);

    const failing = start(defineStage({
      ...echoStage,
      async dispose() { throw new Error("cleanup failed"); },
    }), new HostDriver(), { installSignalHandlers: true, signalShutdownTimeoutMs: 100 });
    await handshake(failing.driver);
    process.emit("SIGTERM");
    await failing.completed;

    const preInit = start(echoStage, new HostDriver(), {
      installSignalHandlers: true,
      signalShutdownTimeoutMs: 100,
    });
    process.emit("SIGTERM");
    await preInit.completed;

    const stuck = start(defineStage({
      ...echoStage,
      async run() { return new Promise<never>(() => undefined); },
    }), new HostDriver(), { installSignalHandlers: true, signalShutdownTimeoutMs: 5 });
    await handshake(stuck.driver);
    void stuck.driver.request("stage.run", { input: {}, config: {}, streamId: 71 });
    await new Promise(resolve => setTimeout(resolve, 0));
    process.emit("SIGTERM");
    await stuck.completed;
  });
});
