import { PassThrough, Readable, Writable } from "node:stream";
import { describe, expect, it, vi } from "vitest";
import {
  FrameDecoder,
  PluginHostError,
  RpcFault,
  RpcPeer,
  RpcRemoteError,
  decodeWireValue,
  encodeFrame,
  encodeWireValue,
} from "../src/index.js";
import { asRecord } from "../src/wire.js";

describe("Content-Length framing", () => {
  it("round-trips split UTF-8 frames and concatenated messages", () => {
    const first = encodeFrame({ jsonrpc: "2.0", id: 1, result: "héllo" });
    const second = encodeFrame({ jsonrpc: "2.0", method: "log", params: {} });
    const bytes = Buffer.concat([first, second]);
    const decoder = new FrameDecoder({ maxFrameBytes: 1024, maxHeaderBytes: 128 });
    const messages = [
      ...decoder.push(bytes.subarray(0, 11)),
      ...decoder.push(bytes.subarray(11, first.length + 3)),
      ...decoder.push(bytes.subarray(first.length + 3)),
    ];
    expect(messages).toEqual([
      { jsonrpc: "2.0", id: 1, result: "héllo" },
      { jsonrpc: "2.0", method: "log", params: {} },
    ]);
    decoder.finish();
  });

  it("round-trips binary values without colliding with reserved-looking objects", () => {
    const result = {
      bytes: new Uint8Array([0, 1, 254, 255]),
      collision: { $forme: "bytes", base64: "AA==" },
    };
    const decoder = new FrameDecoder({ maxFrameBytes: 1024, maxHeaderBytes: 128 });
    expect(decoder.push(encodeFrame({ jsonrpc: "2.0", id: 1, result })))
      .toEqual([{ jsonrpc: "2.0", id: 1, result }]);
  });

  it("rejects malformed binary envelopes", () => {
    const payload = Buffer.from(JSON.stringify({
      jsonrpc: "2.0", id: 1, result: { $forme: "bytes", base64: "***=" },
    }));
    const frame = Buffer.concat([
      Buffer.from(`Content-Length: ${payload.length}\r\n\r\n`), payload,
    ]);
    expect(() => new FrameDecoder({ maxFrameBytes: 1024, maxHeaderBytes: 128 }).push(frame))
      .toThrow(/binary wire envelope/);
  });

  it.each([
    { $forme: "escaped-object", entries: ["bad"] },
    { $forme: "escaped-object", entries: [[7, null]] },
    { $forme: "escaped-object", entries: [["same", 1], ["same", 2]] },
  ])("rejects malformed escaped-object envelope %#", (value) => {
    expect(() => decodeWireValue(value)).toThrow(/escaped-object wire envelope/);
  });

  it("rejects circular values before wire encoding", () => {
    const value: Record<string, unknown> = {};
    value.self = value;
    expect(() => encodeWireValue(value)).toThrow(/circular/);
  });

  it.each([
    ["missing length", Buffer.from("X-Test: 1\r\n\r\n{}")],
    ["duplicate length", Buffer.from("Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}")],
    ["invalid length", Buffer.from("Content-Length: -1\r\n\r\n")],
    ["non-decimal length", Buffer.from("Content-Length: 1x\r\n\r\n")],
    ["empty zero-length JSON", Buffer.from("Content-Length: 0\r\n\r\n")],
    ["invalid json", Buffer.from("Content-Length: 1\r\n\r\n{")],
  ])("rejects %s", (_name, bytes) => {
    const decoder = new FrameDecoder({ maxFrameBytes: 1024, maxHeaderBytes: 128 });
    expect(() => decoder.push(bytes)).toThrow(PluginHostError);
  });

  it("rejects oversized headers, payloads, and truncated EOF", () => {
    expect(() => new FrameDecoder({ maxFrameBytes: 2, maxHeaderBytes: 128 })
      .push(Buffer.from("Content-Length: 3\r\n\r\n{}x")))
      .toThrow(/FRAME_TOO_LARGE/);
    expect(() => new FrameDecoder({ maxFrameBytes: 1024, maxHeaderBytes: 8 })
      .push(Buffer.from("Content-Length: 2")))
      .toThrow(/HEADER_TOO_LARGE/);
    const decoder = new FrameDecoder({ maxFrameBytes: 1024, maxHeaderBytes: 128 });
    decoder.push(Buffer.from("Content-Length: 2\r\n\r\n{"));
    expect(() => decoder.finish()).toThrow(/TRUNCATED_FRAME/);
  });

  it("rejects a completed oversized header and non-object payload", () => {
    expect(new FrameDecoder({ maxFrameBytes: 1024, maxHeaderBytes: 128 }).push(Buffer.alloc(0)))
      .toEqual([]);
    expect(() => new FrameDecoder({ maxFrameBytes: 1024, maxHeaderBytes: 8 })
      .push(Buffer.from("Content-Length: 2\r\n\r\n{}")))
      .toThrow(/HEADER_TOO_LARGE/);
    const bytes = Buffer.from("[]");
    expect(() => new FrameDecoder({ maxFrameBytes: 1024, maxHeaderBytes: 128 })
      .push(Buffer.concat([Buffer.from(`Content-Length: ${bytes.length}\r\n\r\n`), bytes])))
      .toThrow(/PROTOCOL_VIOLATION/);
    expect(() => asRecord([], "value")).toThrow(/value must be an object/);
  });

  it("parses long linear whitespace prefixes without regular expressions", () => {
    const decoder = new FrameDecoder({ maxFrameBytes: 32, maxHeaderBytes: 2_048 });
    const header = Buffer.from(`Content-Length:${"\t".repeat(1_024)}2\r\n\r\n{}`, "ascii");
    expect(decoder.push(header)).toEqual([{}]);
  });

  it("decodes a large frame delivered one byte at a time", () => {
    const value = "x".repeat(256 * 1024);
    const frame = encodeFrame({ jsonrpc: "2.0", id: 1, result: value });
    const decoder = new FrameDecoder({ maxFrameBytes: 512 * 1024, maxHeaderBytes: 128 });
    const messages: Record<string, unknown>[] = [];
    for (const byte of frame) messages.push(...decoder.push(Uint8Array.of(byte)));
    decoder.finish();
    expect(messages).toEqual([{ jsonrpc: "2.0", id: 1, result: value }]);
  });
});

function peerHarness(onRequest: (method: string, params: unknown) => Promise<unknown> = async () => null) {
  const input = new PassThrough();
  const output = new PassThrough();
  const writes: Record<string, unknown>[] = [];
  input.on("data", chunk => {
    const decoder = new FrameDecoder({ maxFrameBytes: 1024, maxHeaderBytes: 128 });
    writes.push(...decoder.push(chunk));
  });
  const fatal = vi.fn();
  const notification = vi.fn();
  const peer = new RpcPeer({
    input,
    output,
    maxFrameBytes: 1024,
    maxHeaderBytes: 128,
    requestTimeoutMs: 30,
    onRequest,
    onNotification: notification,
    onFatal: fatal,
  });
  peer.start();
  peer.start();
  return { peer, input, output, writes, fatal, notification };
}

async function tick(): Promise<void> {
  await new Promise(resolve => setTimeout(resolve, 0));
}

describe("bidirectional JSON-RPC peer", () => {
  it("resolves results, rejects remote errors, and handles notifications", async () => {
    const h = peerHarness();
    const first = h.peer.beginRequest("one", {});
    h.output.write(encodeFrame({ jsonrpc: "2.0", id: first.id, result: 3 }));
    await expect(first.response).resolves.toBe(3);

    const second = h.peer.beginRequest("two", {});
    h.output.write(encodeFrame({ jsonrpc: "2.0", id: second.id, error: { code: -9, message: "no", data: 4 } }));
    await expect(second.response).rejects.toBeInstanceOf(RpcRemoteError);
    h.output.write(encodeFrame({ jsonrpc: "2.0", method: "note", params: { a: 1 } }));
    await tick();
    expect(h.notification).toHaveBeenCalledWith("note", { a: 1 });
    h.output.end();
  });

  it("serves plugin requests and serializes faults", async () => {
    const onRequest = vi.fn(async (method: string) => {
      if (method === "fault") throw new RpcFault(-32001, "DENIED", { capability: "x" });
      if (method === "explode") throw new Error("secret");
      return "ok";
    });
    const h = peerHarness(onRequest);
    h.output.write(encodeFrame({ jsonrpc: "2.0", id: -1, method: "good", params: {} }));
    h.output.write(encodeFrame({ jsonrpc: "2.0", id: -2, method: "fault", params: {} }));
    h.output.write(encodeFrame({ jsonrpc: "2.0", id: -3, method: "explode", params: {} }));
    await tick();
    expect(h.writes).toEqual(expect.arrayContaining([
      { jsonrpc: "2.0", id: -1, result: "ok" },
      { jsonrpc: "2.0", id: -2, error: { code: -32001, message: "DENIED", data: { capability: "x" } } },
      { jsonrpc: "2.0", id: -3, error: { code: -32603, message: "INTERNAL_ERROR" } },
    ]));
    h.output.end();
  });

  it.each([
    { jsonrpc: "1.0", id: 1, result: null },
    { jsonrpc: "2.0", id: -1, result: null },
    { jsonrpc: "2.0", id: {}, result: null },
    { jsonrpc: "2.0", id: "unknown", result: null },
    { jsonrpc: "2.0", id: 99, method: "bad" },
  ])("fails on malformed message %#", async (message) => {
    const h = peerHarness();
    h.output.write(encodeFrame(message));
    await tick();
    expect(h.fatal).toHaveBeenCalled();
    expect(() => h.peer.beginRequest("later", {})).toThrow();
    h.output.end();
  });

  it("rejects malformed responses and times out requests", async () => {
    const h = peerHarness();
    const missing = h.peer.beginRequest("missing", {});
    h.output.write(encodeFrame({ jsonrpc: "2.0", id: missing.id }));
    await expect(missing.response).rejects.toThrow(/exactly one/);
    const malformed = h.peer.beginRequest("malformed", {});
    h.output.write(encodeFrame({ jsonrpc: "2.0", id: malformed.id, error: { code: "bad" } }));
    await expect(malformed.response).rejects.toThrow(/malformed/);
    await expect(h.peer.request("handshake", {}, 1)).rejects.toThrow(/HANDSHAKE_TIMEOUT/);
    await expect(h.peer.request("slow", {}, 1)).rejects.toThrow(/REQUEST_TIMEOUT/);
    h.output.end();
  });

  it("rejects pending requests when failed and handles write failures", async () => {
    const h = peerHarness();
    const pending = h.peer.beginRequest("pending", {});
    h.peer.fail(new Error("closed"));
    h.peer.fail(new Error("ignored"));
    await expect(pending.response).rejects.toThrow("closed");

    const output = new PassThrough();
    const brokenInput = new Writable({ write(_chunk, _encoding, callback) { callback(new Error("write failed")); } });
    brokenInput.on("error", () => {});
    const peer = new RpcPeer({
      input: brokenInput,
      output,
      maxFrameBytes: 1024,
      maxHeaderBytes: 128,
      requestTimeoutMs: 30,
      onRequest: async () => null,
      onNotification: () => {},
      onFatal: () => {},
    });
    peer.start();
    await expect(peer.request("write", {})).rejects.toThrow("write failed");
    output.end();
  });

  it("rejects outbound frames above the negotiated bound", async () => {
    const output = new PassThrough();
    const peer = new RpcPeer({
      input: new PassThrough(),
      output,
      maxFrameBytes: 8,
      maxHeaderBytes: 128,
      requestTimeoutMs: 30,
      onRequest: async () => null,
      onNotification: () => {},
      onFatal: () => {},
    });
    peer.start();
    await expect(peer.notify("large", { value: "too large" })).rejects.toThrow(/FRAME_TOO_LARGE/);
    output.end();
  });

  it("counts non-finite numbers as JSON null during exact-bound preflight", async () => {
    const output = new PassThrough();
    const peer = new RpcPeer({
      input: new PassThrough(), output,
      maxFrameBytes: 53, maxHeaderBytes: 128, requestTimeoutMs: 30,
      onRequest: async () => null, onNotification: () => {}, onFatal: () => {},
    });
    peer.start();
    await expect(peer.notify("x", { value: Number.NaN })).rejects.toThrow(/FRAME_TOO_LARGE/);
    await expect(peer.notify("x", { value: Number.POSITIVE_INFINITY }))
      .rejects.toThrow(/FRAME_TOO_LARGE/);
    output.end();
  });

  it("preflights JSON shapes before allocating an outbound frame", async () => {
    const output = new PassThrough();
    const peer = new RpcPeer({
      input: new PassThrough(), output,
      maxFrameBytes: 1024, maxHeaderBytes: 128, requestTimeoutMs: 30,
      onRequest: async () => null, onNotification: () => {}, onFatal: () => {},
    });
    peer.start();
    await expect(peer.notify("shapes", {
      array: [undefined, true, false, null, 1, "\b\f\n\r\t\"\\\u0001é😀"],
      omitted: undefined,
      reserved: { $forme: "user", value: undefined },
    })).resolves.toBeUndefined();
    await expect(peer.notify("bigint", { value: 1n })).rejects.toThrow(/BigInt/);
    await expect(peer.notify("binary", { value: new Uint8Array(2_000) }))
      .rejects.toThrow(/FRAME_TOO_LARGE/);
    const circular: Record<string, unknown> = {};
    circular.self = circular;
    await expect(peer.notify("circular", circular)).rejects.toThrow(/circular/);
    output.end();
  });

  it("accepts string stream chunks", async () => {
    const input = new PassThrough();
    const output = new PassThrough();
    output.setEncoding("utf8");
    const peer = new RpcPeer({
      input,
      output,
      maxFrameBytes: 1024,
      maxHeaderBytes: 128,
      requestTimeoutMs: 30,
      onRequest: async () => null,
      onNotification: () => {},
      onFatal: () => {},
    });
    peer.start();
    const pending = peer.beginRequest("string", {});
    output.write(encodeFrame({ jsonrpc: "2.0", id: pending.id, result: "ok" }).toString("utf8"));
    await expect(pending.response).resolves.toBe("ok");
    output.end();
  });

  it("normalizes non-Error reader failures", async () => {
    const fatal = vi.fn();
    const output = Readable.from((async function* () {
      throw "reader failed";
    })());
    const peer = new RpcPeer({
      input: new PassThrough(),
      output,
      maxFrameBytes: 1024,
      maxHeaderBytes: 128,
      requestTimeoutMs: 30,
      onRequest: async () => null,
      onNotification: () => {},
      onFatal: fatal,
    });
    peer.start();
    await tick();
    expect(fatal.mock.calls[0]?.[0]).toEqual(new Error("reader failed"));
  });
});
