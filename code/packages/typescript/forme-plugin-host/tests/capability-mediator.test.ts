import { describe, expect, it, vi } from "vitest";
import {
  frozenClock,
  inMemoryCache,
  inMemoryEventBus,
  neverCancelledToken,
  noOpTelemetryEmitter,
  silentLogger,
  type StageContext,
} from "@coding-adventures/forme-stage";
import { mediateCapabilityRequest, RpcFault } from "../src/index.js";

function makeContext(): StageContext {
  return {
    logger: silentLogger(),
    cancellation: neverCancelledToken(),
    time: frozenClock({ timestamp: 123 }),
    cache: inMemoryCache(),
    telemetry: noOpTelemetryEmitter(),
    storage: {
      async read(path) { return new TextEncoder().encode(path); },
      async readBounded(path, maxBytes) {
        return new TextEncoder().encode(path).subarray(0, maxBytes);
      },
      async write() {},
      async exists(path) { return path === "yes"; },
      async *list(path) { yield { path, type: "file" }; },
      async *watch(path) { yield { path, kind: "added" }; },
      async remove() {},
      async stat() { return { size: 4, mtimeMs: 5, type: "file" }; },
    },
    network: {
      async fetch() {
        return new Response("ok", { status: 201, headers: { "x-test": "yes" } });
      },
    },
    env: {
      get(name) { return name === "ALLOWED" ? "value" : undefined; },
      getOrThrow(name) { return name; },
    },
    filesystem: {
      async readAbsolute(path) { return new TextEncoder().encode(path); },
      async readAbsoluteBounded(path, maxBytes) {
        return new TextEncoder().encode(path).subarray(0, maxBytes);
      },
      async writeAbsolute() {},
      homeDir() { return "/home/test"; },
      tempDir() { return "/tmp/test"; },
    },
    shell: { async run() { return { exitCode: 0, stdout: new Uint8Array(), stderr: new Uint8Array() }; } },
    events: inMemoryEventBus(),
  };
}

describe("capability mediation", () => {
  it("mediates every storage operation and encodes bytes", async () => {
    const ctx = makeContext();
    const grants = ["storage:read", "storage:write"];
    await expect(mediateCapabilityRequest("ctx.storage.read", { path: "a.md" }, ctx, grants))
      .resolves.toEqual({ bytes: Buffer.from("a.md").toString("base64") });
    await expect(mediateCapabilityRequest("ctx.storage.write", {
      path: "a.md", bytes: Buffer.from("x").toString("base64"),
    }, ctx, grants)).resolves.toBeNull();
    await expect(mediateCapabilityRequest("ctx.storage.exists", { path: "yes" }, ctx, grants))
      .resolves.toBe(true);
    await expect(mediateCapabilityRequest("ctx.storage.stat", { path: "a" }, ctx, grants))
      .resolves.toMatchObject({ size: 4 });
    await expect(mediateCapabilityRequest("ctx.storage.list", { path: "a" }, ctx, grants))
      .resolves.toEqual([{ path: "a", type: "file" }]);
    await expect(mediateCapabilityRequest("ctx.storage.watch", { path: "a" }, ctx, grants))
      .rejects.toMatchObject({ rpcCode: -32601 });
    await expect(mediateCapabilityRequest("ctx.storage.remove", { path: "a" }, ctx, grants))
      .resolves.toBeNull();
  });

  it.each(["../escape", "/absolute", "a/../../b", "nul\0byte", ""])(
    "rejects unsafe storage path %j", async (path) => {
      await expect(mediateCapabilityRequest(
        "ctx.storage.read", { path }, makeContext(), ["storage:read"],
      )).rejects.toMatchObject({ rpcCode: -32002 });
    },
  );

  it("denies missing grants and calls outside run", async () => {
    await expect(mediateCapabilityRequest(
      "ctx.storage.read", { path: "a" }, makeContext(), [],
    )).rejects.toMatchObject({ rpcCode: -32001 });
    await expect(mediateCapabilityRequest(
      "ctx.storage.read", { path: "a" }, null, ["storage:read"],
    )).rejects.toBeInstanceOf(RpcFault);
    await expect(mediateCapabilityRequest(
      "ctx.time.monotonicMs", null, makeContext(), [],
    )).resolves.toBe(0);
  });

  it("mediates env, filesystem, and clocks", async () => {
    const ctx = makeContext();
    await expect(mediateCapabilityRequest("ctx.env.get", { name: "ALLOWED" }, ctx, ["env:ALLOWED"]))
      .resolves.toBe("value");
    await expect(mediateCapabilityRequest("ctx.env.get", { name: "MISSING" }, ctx, ["env:MISSING"]))
      .resolves.toBeNull();
    await expect(mediateCapabilityRequest("ctx.filesystem.readAbsolute", { path: "/a" }, ctx, ["filesystem:user"]))
      .resolves.toEqual({ bytes: Buffer.from("/a").toString("base64") });
    await expect(mediateCapabilityRequest("ctx.filesystem.writeAbsolute", {
      path: "/a", bytes: Buffer.from("x").toString("base64"),
    }, ctx, ["filesystem:user"])).resolves.toBeNull();
    await expect(mediateCapabilityRequest("ctx.filesystem.homeDir", {}, ctx, ["filesystem:user"]))
      .resolves.toBe("/home/test");
    await expect(mediateCapabilityRequest("ctx.filesystem.tempDir", {}, ctx, ["filesystem:user"]))
      .resolves.toBe("/tmp/test");
    await expect(mediateCapabilityRequest("ctx.time.nowMs", {}, ctx, ["system:time:wallclock"]))
      .resolves.toBe(123);
    await expect(mediateCapabilityRequest("ctx.time.nowIso", {}, ctx, ["system:time:wallclock"]))
      .resolves.toBe(new Date(123).toISOString());
    await expect(mediateCapabilityRequest("ctx.time.monotonicMs", {}, ctx, []))
      .resolves.toBe(0);
  });

  it("validates filesystem, string, and base64 parameters", async () => {
    const ctx = makeContext();
    await expect(mediateCapabilityRequest(
      "ctx.filesystem.readAbsolute", { path: "relative" }, ctx, ["filesystem:user"],
    )).rejects.toMatchObject({ rpcCode: -32002 });
    await expect(mediateCapabilityRequest(
      "ctx.env.get", { name: 1 }, ctx, ["env:*"] as never,
    )).rejects.toMatchObject({ rpcCode: -32602 });
    await expect(mediateCapabilityRequest(
      "ctx.storage.write", { path: "a", bytes: "%%%" }, ctx, ["storage:write"],
    )).rejects.toMatchObject({ rpcCode: -32602 });
  });

  it("bounds mediated storage and filesystem byte transfers", async () => {
    const exactBytes = new Uint8Array(1024 * 1024);
    const exact = makeContext();
    const exactStorageRead = vi.fn(async () => exactBytes);
    const exactFilesystemRead = vi.fn(async () => exactBytes);
    exact.storage.stat = async () => ({ size: exactBytes.byteLength, mtimeMs: 0, type: "file" });
    exact.storage.readBounded = exactStorageRead;
    exact.filesystem.readAbsoluteBounded = exactFilesystemRead;
    await expect(mediateCapabilityRequest(
      "ctx.storage.read", { path: "exact" }, exact, ["storage:read"],
    )).resolves.toMatchObject({ bytes: Buffer.from(exactBytes).toString("base64") });
    expect(exactStorageRead).toHaveBeenCalledWith("exact", 1024 * 1024);
    await expect(mediateCapabilityRequest(
      "ctx.filesystem.readAbsolute", { path: "/exact" }, exact, ["filesystem:user"],
    )).resolves.toMatchObject({ bytes: Buffer.from(exactBytes).toString("base64") });
    expect(exactFilesystemRead).toHaveBeenCalledWith("/exact", 1024 * 1024);

    const tooLarge = new Uint8Array(1024 * 1024 + 1);
    const ctx = makeContext();
    ctx.storage.stat = async () => ({ size: tooLarge.byteLength, mtimeMs: 0, type: "file" });
    await expect(mediateCapabilityRequest(
      "ctx.storage.read", { path: "large" }, ctx, ["storage:read"],
    )).rejects.toMatchObject({ rpcCode: -32003 });

    const postStat = makeContext();
    postStat.storage.readBounded = async () => tooLarge;
    await expect(mediateCapabilityRequest(
      "ctx.storage.read", { path: "grew" }, postStat, ["storage:read"],
    )).rejects.toMatchObject({ rpcCode: -32003 });

    const filesystem = makeContext();
    filesystem.filesystem.readAbsoluteBounded = async () => tooLarge;
    await expect(mediateCapabilityRequest(
      "ctx.filesystem.readAbsolute", { path: "/large" }, filesystem, ["filesystem:user"],
    )).rejects.toMatchObject({ rpcCode: -32003 });

    const encoded = Buffer.from(tooLarge).toString("base64");
    await expect(mediateCapabilityRequest(
      "ctx.storage.write", { path: "large", bytes: encoded }, makeContext(), ["storage:write"],
    )).rejects.toMatchObject({ rpcCode: -32003 });
    await expect(mediateCapabilityRequest(
      "ctx.network.fetch",
      { url: "https://example.com", init: { body: encoded } },
      makeContext(),
      ["network:example.com"],
    )).rejects.toMatchObject({ rpcCode: -32003 });
  });

  it("mediates a granted network request and bounds responses", async () => {
    const ctx = makeContext();
    await expect(mediateCapabilityRequest("ctx.network.fetch", {
      url: "https://api.example.com/a",
      init: { method: "POST", headers: { "x-a": "b" }, bytes: undefined },
    }, ctx, ["network:example.com"])).resolves.toMatchObject({
      status: 201,
      bytes: Buffer.from("ok").toString("base64"),
    });
    await expect(mediateCapabilityRequest(
      "ctx.network.fetch", { url: "not a url" }, ctx, ["network:*"] as never,
    )).rejects.toMatchObject({ rpcCode: -32602 });
    await expect(mediateCapabilityRequest(
      "ctx.network.fetch", { url: "https://evil.test" }, ctx, ["network:example.com"],
    )).rejects.toMatchObject({ rpcCode: -32001 });

    const huge = makeContext();
    huge.network.fetch = vi.fn(async () => new Response(new Uint8Array(1024 * 1024 + 1)));
    await expect(mediateCapabilityRequest(
      "ctx.network.fetch", { url: "https://example.com" }, huge, ["network:example.com"],
    )).rejects.toMatchObject({ rpcCode: -32003 });

    const emptyBase = makeContext();
    const empty: StageContext = {
      ...emptyBase,
      network: { async fetch() { return new Response(null, { status: 204 }); } },
    };
    await expect(mediateCapabilityRequest(
      "ctx.network.fetch", { url: "https://example.com" }, empty, ["network:example.com"],
    )).resolves.toMatchObject({ status: 204, bytes: "" });
  });

  it("authorizes every redirect target and enforces a redirect bound", async () => {
    const redirect = (location: string) => {
      const cancel = vi.fn();
      const body = new ReadableStream<Uint8Array>({ cancel });
      return { cancel, response: new Response(body, { status: 302, headers: { location } }) };
    };
    const denied = makeContext();
    const deniedRedirect = redirect("https://evil.test/next");
    const deniedFetch = vi.fn(async () => deniedRedirect.response);
    denied.network.fetch = deniedFetch;
    await expect(mediateCapabilityRequest(
      "ctx.network.fetch",
      { url: "https://api.example.com/start" },
      denied,
      ["network:example.com"],
    )).rejects.toMatchObject({ rpcCode: -32001 });
    expect(deniedFetch).toHaveBeenCalledOnce();
    expect(deniedRedirect.cancel).toHaveBeenCalledOnce();

    const allowed = makeContext();
    const allowedRedirect = redirect("/done");
    const allowedFetch = vi.fn(async (url: string) => url.endsWith("/start")
      ? allowedRedirect.response
      : new Response("done", { status: 200 }));
    allowed.network.fetch = allowedFetch;
    await expect(mediateCapabilityRequest(
      "ctx.network.fetch",
      { url: "https://api.example.com/start" },
      allowed,
      ["network:example.com"],
    )).resolves.toMatchObject({ status: 200, bytes: Buffer.from("done").toString("base64") });
    expect(allowedFetch).toHaveBeenCalledTimes(2);
    expect(allowedRedirect.cancel).toHaveBeenCalledOnce();

    const loop = makeContext();
    const loopCancels: Array<ReturnType<typeof vi.fn>> = [];
    loop.network.fetch = vi.fn(async () => {
      const next = redirect("/again");
      loopCancels.push(next.cancel);
      return next.response;
    });
    await expect(mediateCapabilityRequest(
      "ctx.network.fetch",
      { url: "https://api.example.com/start" },
      loop,
      ["network:example.com"],
    )).rejects.toMatchObject({ rpcCode: -32003 });
    expect(loopCancels).toHaveLength(11);
    expect(loopCancels.every(cancel => cancel.mock.calls.length === 1)).toBe(true);

    const replay = makeContext();
    const replayRedirect = redirect("/done");
    replay.network.fetch = vi.fn(async () => replayRedirect.response);
    await expect(mediateCapabilityRequest(
      "ctx.network.fetch",
      { url: "https://api.example.com/start", init: { method: "POST" } },
      replay,
      ["network:example.com"],
    )).rejects.toMatchObject({ rpcCode: -32001 });
    expect(replayRedirect.cancel).toHaveBeenCalledOnce();

    const finalUrl = makeContext();
    finalUrl.network.fetch = vi.fn(async () => ({
      url: "https://evil.test/final",
      status: 200,
      statusText: "OK",
      headers: new Headers(),
      body: null,
    } as Response));
    await expect(mediateCapabilityRequest(
      "ctx.network.fetch",
      { url: "https://api.example.com/start" },
      finalUrl,
      ["network:example.com"],
    )).rejects.toMatchObject({ rpcCode: -32001 });
  });

  it("passes bounded request bodies and aborts response readers", async () => {
    const bodyContext = makeContext();
    const fetch = vi.fn(async () => new Response("ok"));
    bodyContext.network.fetch = fetch;
    await mediateCapabilityRequest("ctx.network.fetch", {
      url: "https://example.com",
      init: { body: Buffer.from("body").toString("base64") },
    }, bodyContext, ["network:example.com"]);
    expect(fetch.mock.calls[0]?.[1]?.body).toEqual(Buffer.from("body"));

    const cancelledBase = makeContext();
    const source = new ReadableStream<Uint8Array>({
      start(controller) { controller.enqueue(new Uint8Array([1])); },
      cancel() { throw new Error("cancel failed"); },
    });
    const controller = new AbortController();
    controller.abort();
    const cancelled: StageContext = {
      ...cancelledBase,
      cancellation: {
        cancelled: true, reason: "cancelled", signal: controller.signal,
        throwIfCancelled() {}, onCancel() {},
      },
    };
    cancelled.network.fetch = vi.fn(async () => new Response(source));
    await expect(mediateCapabilityRequest(
      "ctx.network.fetch", { url: "https://example.com" }, cancelled, ["network:example.com"],
    )).rejects.toThrow(/cancelled/i);
  });

  it("denies shell, rejects unknown methods, and bounds mediated iterables", async () => {
    await expect(mediateCapabilityRequest("ctx.shell.run", {}, makeContext(), ["system:shell"]))
      .rejects.toMatchObject({ rpcCode: -32001 });
    await expect(mediateCapabilityRequest("ctx.nope", {}, makeContext(), []))
      .rejects.toMatchObject({ rpcCode: -32601 });
    const ctx = makeContext();
    ctx.storage.list = async function* () {
      for (let i = 0; i <= 4_096; i++) yield { path: String(i), type: "file" };
    };
    await expect(mediateCapabilityRequest(
      "ctx.storage.list", { path: "a" }, ctx, ["storage:read"],
    )).rejects.toMatchObject({ rpcCode: -32003 });
  });
});
