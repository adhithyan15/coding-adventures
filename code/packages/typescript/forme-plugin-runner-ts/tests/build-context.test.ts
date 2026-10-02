import { Buffer } from "node:buffer";
import { describe, expect, it, vi } from "vitest";
import { CapabilityError, CancellationError, StageError } from "@coding-adventures/forme-errors";
import { createCancellationTokenSource, neverCancelledToken } from "@coding-adventures/forme-stage";
import { buildWireContext, wireLogger } from "../src/build-context.js";
import { RunnerRpcRemoteError, type RunnerRpcPeer } from "../src/wire.js";

function peerWith(handler: (method: string, params: unknown) => unknown): RunnerRpcPeer {
  return {
    request: vi.fn(async (method: string, params: unknown) => handler(method, params)),
    notify: vi.fn(async () => undefined),
  } as unknown as RunnerRpcPeer;
}

async function collect<T>(values: AsyncIterable<T>): Promise<T[]> {
  const result: T[] = [];
  for await (const value of values) result.push(value);
  return result;
}

describe("wire-backed stage context", () => {
  it("mediates every context API and preserves stream identity", async () => {
    const calls: Array<[string, Record<string, unknown>]> = [];
    const peer = peerWith((method, rawParams) => {
      const params = rawParams as Record<string, unknown>;
      calls.push([method, params]);
      const responses: Record<string, unknown> = {
        "ctx.time.nowMs": 12,
        "ctx.time.nowIso": "1970-01-01T00:00:00.012Z",
        "ctx.storage.read": { bytes: "aGk=" },
        "ctx.storage.write": null,
        "ctx.storage.exists": true,
        "ctx.storage.list": [{ path: "a", type: "file" }],
        "ctx.storage.watch": { kind: "stream-handle", streamId: 701 },
        "ctx.storage.remove": null,
        "ctx.storage.stat": { size: 2, mtimeMs: 1, type: "file" },
        "ctx.env.get": params.name === "MISSING" ? null : "value",
        "ctx.filesystem.readAbsolute": { bytes: "aGk=" },
        "ctx.filesystem.writeAbsolute": null,
        "ctx.filesystem.homeDir": "/home/test",
        "ctx.filesystem.tempDir": "/tmp/test",
        "ctx.network.fetch": {
          status: 202,
          statusText: "Accepted",
          headers: { "x-test": "yes" },
          bytes: "b2s=",
          url: "https://example.com/result",
        },
        "ctx.shell.run": { exitCode: 0, stdout: "b3V0", stderr: "ZXJy" },
      };
      return responses[method];
    });
    const context = buildWireContext(peer, 77, neverCancelledToken(), async function* (handle) {
      expect(handle).toEqual({ kind: "stream-handle", streamId: 701 });
      yield { type: "changed", path: "a" };
    });

    expect(await context.time.nowMs()).toBe(12);
    expect(await context.time.nowIso()).toBe("1970-01-01T00:00:00.012Z");
    expect(context.time.monotonicMs()).toEqual(expect.any(Number));
    expect(await context.storage.read("a")).toEqual(new Uint8Array(Buffer.from("hi")));
    expect(await context.storage.readBounded("a", 2)).toHaveLength(2);
    await context.storage.write("b", new Uint8Array(Buffer.from("hi")));
    expect(await context.storage.exists("a")).toBe(true);
    expect(await collect(context.storage.list("."))).toEqual([{ path: "a", type: "file" }]);
    expect(await collect(context.storage.watch("."))).toEqual([{ type: "changed", path: "a" }]);
    await context.storage.remove("a");
    expect(await context.storage.stat("a")).toMatchObject({ size: 2 });
    expect(await context.env.get("PRESENT")).toBe("value");
    expect(await context.env.get("MISSING")).toBeUndefined();
    expect(await context.env.getOrThrow("PRESENT")).toBe("value");
    expect(await context.filesystem.readAbsolute("/a")).toHaveLength(2);
    expect(await context.filesystem.readAbsoluteBounded("/a", 2)).toHaveLength(2);
    await context.filesystem.writeAbsolute("/b", new Uint8Array(Buffer.from("hi")));
    expect(await context.filesystem.homeDir()).toBe("/home/test");
    expect(await context.filesystem.tempDir()).toBe("/tmp/test");
    const response = await context.network.fetch("https://example.com", {
      method: "POST", headers: { "X-A": "b" }, body: new Uint8Array(Buffer.from("in")),
    });
    expect(response.status).toBe(202);
    expect(response.url).toBe("https://example.com/result");
    expect(await response.text()).toBe("ok");
    const request = new Request("https://example.com/request", { method: "PUT", body: "body" });
    expect((await context.network.fetch(request, { headers: { "x-extra": "yes" } })).status).toBe(202);
    expect(await context.shell.run("tool", ["arg"], { stdin: new Uint8Array(Buffer.from("stdin")) }))
      .toEqual({ exitCode: 0, stdout: new Uint8Array(Buffer.from("out")), stderr: new Uint8Array(Buffer.from("err")) });
    expect((await context.network.fetch("https://example.com/no-options")).statusText).toBe("Accepted");
    expect((await context.network.fetch("https://example.com/string", { body: "text" })).status).toBe(202);
    expect(await context.shell.run("tool", [])).toMatchObject({ exitCode: 0 });
    expect(await context.shell.run("tool", [], {})).toMatchObject({ exitCode: 0 });
    expect(calls.every(([, params]) => params.streamId === 77)).toBe(true);
  });

  it("emits every log level, child fields, and ignores logging transport failure", async () => {
    const peer = peerWith(() => null);
    const notify = vi.mocked(peer.notify);
    notify.mockResolvedValueOnce(undefined).mockRejectedValueOnce(new Error("closed"));
    const logger = wireLogger(peer, { base: true });
    logger.trace("trace");
    logger.debug("debug");
    logger.info("info", { extra: 1 });
    logger.warn("warn");
    logger.error("error");
    logger.child({ child: "yes" }).info("child");
    await new Promise(resolve => setTimeout(resolve, 0));
    expect(notify).toHaveBeenCalledTimes(6);
    expect(notify).toHaveBeenLastCalledWith("log", {
      level: "info", message: "child", fields: { base: true, child: "yes" },
    });
  });

  it("enforces byte, bounded-read, response-shape, and network-body limits", async () => {
    let response: unknown = { bytes: "aGk=" };
    const peer = peerWith(() => response);
    const context = buildWireContext(peer, 1, neverCancelledToken());

    await expect(context.storage.readBounded("a", -1)).rejects.toBeInstanceOf(RangeError);
    await expect(context.filesystem.readAbsoluteBounded("/a", 1.5)).rejects.toBeInstanceOf(RangeError);
    await expect(context.storage.readBounded("a", 1)).rejects.toMatchObject({ code: "RESOURCE_LIMIT_EXCEEDED" });
    await expect(context.filesystem.readAbsoluteBounded("/a", 1)).rejects.toMatchObject({ code: "RESOURCE_LIMIT_EXCEEDED" });
    const huge = new Uint8Array(1024 * 1024 + 1);
    await expect(context.storage.write("a", huge)).rejects.toMatchObject({ code: "RESOURCE_LIMIT_EXCEEDED" });
    await expect(context.filesystem.writeAbsolute("/a", huge)).rejects.toMatchObject({ code: "RESOURCE_LIMIT_EXCEEDED" });
    await expect(context.network.fetch("https://example.com", { body: new URLSearchParams("a=b") }))
      .rejects.toMatchObject({ code: "INVALID_NETWORK_BODY" });

    response = { bytes: "not-base64" };
    await expect(context.storage.read("a")).rejects.toMatchObject({ code: "PLUGIN_PROTOCOL_ERROR" });
    response = "not-an-array";
    await expect(collect(context.storage.list("."))).rejects.toMatchObject({ code: "PLUGIN_PROTOCOL_ERROR" });
    await expect(collect(context.storage.watch("."))).rejects.toMatchObject({ code: "PLUGIN_PROTOCOL_ERROR" });
    response = 123;
    await expect(context.storage.exists("a")).rejects.toMatchObject({ code: "PLUGIN_PROTOCOL_ERROR" });
    await expect(context.env.get("A")).rejects.toMatchObject({ code: "PLUGIN_PROTOCOL_ERROR" });
    response = Number.NaN;
    await expect(context.time.nowMs()).rejects.toMatchObject({ code: "PLUGIN_PROTOCOL_ERROR" });
    response = null;
    await expect(context.env.getOrThrow("A")).rejects.toMatchObject({ code: "ENV_MISSING" });

    response = { status: 200, statusText: null, headers: ["bad"], bytes: "" };
    const minimal = await context.network.fetch("https://example.com");
    expect({ statusText: minimal.statusText, url: minimal.url, headers: [...minimal.headers] })
      .toEqual({ statusText: "", url: "", headers: [] });
    response = { exitCode: Number.NaN, stdout: "", stderr: "" };
    await expect(context.shell.run("tool", [])).rejects.toMatchObject({ code: "PLUGIN_PROTOCOL_ERROR" });
  });

  it("translates remote faults and observes cancellation before and after host calls", async () => {
    const cases = [
      [new RunnerRpcRemoteError(-32800, "stop", null), CancellationError],
      [new RunnerRpcRemoteError(-32001, "denied", { capability: "network:x" }), CapabilityError],
      [new RunnerRpcRemoteError(-32001, "denied", null), CapabilityError],
      [new RunnerRpcRemoteError(-32099, "bad", null), StageError],
      [new Error("local"), Error],
      ["primitive", Error],
    ] as const;
    for (const [failure, constructor] of cases) {
      const context = buildWireContext(peerWith(() => { throw failure; }), 1, neverCancelledToken());
      await expect(context.time.nowMs()).rejects.toBeInstanceOf(constructor);
    }

    const source = createCancellationTokenSource();
    source.cancel("before");
    const peer = peerWith(() => 1);
    const cancelled = buildWireContext(peer, 1, source.token);
    await expect(cancelled.time.nowMs()).rejects.toBeInstanceOf(CancellationError);
    expect(peer.request).not.toHaveBeenCalled();

    const after = createCancellationTokenSource();
    const afterPeer = peerWith(() => { after.cancel("after"); return 1; });
    await expect(buildWireContext(afterPeer, 1, after.token).time.nowMs())
      .rejects.toBeInstanceOf(CancellationError);
  });
});
