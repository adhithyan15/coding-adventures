import { PassThrough, Writable } from "node:stream";
import { describe, expect, it, vi } from "vitest";
import {
  FrameDecoder,
  RunnerProtocolError,
  RunnerRpcPeer,
  RunnerRpcRemoteError,
  asRecord,
  decodeWireValue,
  encodeFrame,
  encodeWireValue,
  type JsonRpcMessage,
} from "../src/wire.js";

function framedPayload(value: unknown): Buffer {
  const payload = Buffer.from(JSON.stringify(value));
  return Buffer.concat([Buffer.from(`Content-Length: ${payload.length}\r\n\r\n`), payload]);
}

class OutputMessages {
  readonly values: JsonRpcMessage[] = [];
  private readonly waiters: Array<(message: JsonRpcMessage) => void> = [];
  private readonly decoder = new FrameDecoder({ maxFrameBytes: 4096, maxHeaderBytes: 128 });

  constructor(output: PassThrough) {
    output.on("data", chunk => {
      for (const message of this.decoder.push(chunk)) {
        const waiter = this.waiters.shift();
        if (waiter) waiter(message);
        else this.values.push(message);
      }
    });
  }

  next(): Promise<JsonRpcMessage> {
    const value = this.values.shift();
    return value ? Promise.resolve(value) : new Promise(resolve => this.waiters.push(resolve));
  }
}

describe("bounded framed wire protocol", () => {
  it("round-trips binary, escaped objects, arrays, and fragmented frames", () => {
    const original = {
      bytes: new Uint8Array([0, 1, 255]),
      collision: { $forme: "bytes", base64: "plain", nested: [new Uint8Array([2])] },
    };
    expect(decodeWireValue(encodeWireValue(original))).toEqual(original);
    const frame = encodeFrame({ jsonrpc: "2.0", result: original });
    const decoder = new FrameDecoder({ maxFrameBytes: 4096, maxHeaderBytes: 128 });
    expect(decoder.push(frame.subarray(0, 10))).toEqual([]);
    expect(decoder.push(frame.subarray(10))).toEqual([{ jsonrpc: "2.0", result: original }]);
    decoder.finish();

    const two = Buffer.concat([encodeFrame({ a: 1 }), encodeFrame({ b: 2 })]);
    expect(new FrameDecoder({ maxFrameBytes: 100, maxHeaderBytes: 100 }).push(two))
      .toEqual([{ a: 1 }, { b: 2 }]);

    const largeMessage = { jsonrpc: "2.0", result: "x".repeat(16 * 1024) };
    const largeFrame = encodeFrame(largeMessage);
    const fragmented = new FrameDecoder({ maxFrameBytes: 32 * 1024, maxHeaderBytes: 128 });
    let decoded: JsonRpcMessage[] = [];
    for (const byte of largeFrame) decoded = decoded.concat(fragmented.push(Uint8Array.of(byte)));
    expect(decoded).toEqual([largeMessage]);
    fragmented.finish();
  });

  it("rejects ambiguous envelopes and malformed framing", () => {
    const invalidValues = [
      { $forme: "bytes", base64: "!" },
      { $forme: "escaped-object", entries: [["a"]] },
      { $forme: "escaped-object", entries: [["a", 1], ["a", 2]] },
      { $forme: "escaped-object", entries: [[1, 2]] },
    ];
    for (const value of invalidValues) expect(() => decodeWireValue(value)).toThrow(RunnerProtocolError);
    const circular: Record<string, unknown> = {};
    circular.self = circular;
    expect(() => encodeWireValue(circular)).toThrow(/circular/);
    expect(() => encodeFrame({ value: 1n })).toThrow(/BigInt/);
    expect(asRecord({ ok: true }, "value")).toEqual({ ok: true });
    expect(() => asRecord([], "value")).toThrow(/must be an object/);

    const limits = { maxFrameBytes: 100, maxHeaderBytes: 64 };
    const badFrames = [
      Buffer.from("X: 1\r\n\r\n{}"),
      Buffer.from("Content-Length: 02\r\n\r\n{}"),
      Buffer.from("Content-Length: -1\r\n\r\n"),
      Buffer.from("Content-Length: 2 \r\n\r\n{}"),
      Buffer.from("Content-Length:\r\n\r\n"),
      Buffer.from("Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}"),
      framedPayload("scalar"),
      framedPayload({ $forme: "bytes", base64: "!" }),
      Buffer.from("Content-Length: 1\r\n\r\n{"),
    ];
    for (const frame of badFrames) {
      expect(() => new FrameDecoder(limits).push(frame)).toThrow(RunnerProtocolError);
    }
    const unfinished = new FrameDecoder(limits);
    unfinished.push(Buffer.from("Content-Length: 2\r\n\r\n{"));
    expect(() => unfinished.finish()).toThrow(/middle of a frame/);

    expect(() => encodeFrame({ result: "\0".repeat(10) }, 40)).toThrow(/configured bound/);
    expect(encodeFrame({ values: [undefined, Number.NaN, true, false, "é", "漢", "😀", "\ud800"] }, 200))
      .toEqual(expect.any(Buffer));
    expect(encodeFrame({ omitted: undefined, keep: 1 }, 100)).toEqual(expect.any(Buffer));
    expect(new FrameDecoder(limits).push(Buffer.from("CONTENT-LENGTH:\t2\r\n\r\n{}")))
      .toEqual([{}]);
  });

  it("serves host requests and maps outbound responses", async () => {
    const input = new PassThrough();
    const output = new PassThrough();
    const messages = new OutputMessages(output);
    const notified = vi.fn();
    let peer!: RunnerRpcPeer;
    peer = new RunnerRpcPeer({
      input, output, maxFrameBytes: 4096, maxHeaderBytes: 128,
      onNotification: notified,
      onRequest: async (_id, method) => {
        if (method === "fault") throw { rpcCode: -32001, message: "DENIED", data: { capability: "x" } };
        if (method === "throw") throw new Error("boom");
        if (method === "primitive") throw "bad";
        if (method === "dispose") { peer.requestStopAfterResponse(); return null; }
        return { ok: true };
      },
    });
    const running = peer.run();
    input.write(encodeFrame({ jsonrpc: "2.0", method: "notice", params: { n: 1 } }));
    await vi.waitFor(() => expect(notified).toHaveBeenCalledWith("notice", { n: 1 }));

    input.write(encodeFrame({ jsonrpc: "2.0", id: 1, method: "ok", params: {} }));
    expect(await messages.next()).toEqual({ jsonrpc: "2.0", id: 1, result: { ok: true } });
    input.write(encodeFrame({ jsonrpc: "2.0", id: 2, method: "fault", params: {} }));
    expect(await messages.next()).toEqual({
      jsonrpc: "2.0", id: 2, error: { code: -32001, message: "DENIED", data: { capability: "x" } },
    });
    input.write(encodeFrame({ jsonrpc: "2.0", id: 3, method: "throw", params: {} }));
    expect(await messages.next()).toEqual({ jsonrpc: "2.0", id: 3, error: { code: -32603, message: "boom" } });
    input.write(encodeFrame({ jsonrpc: "2.0", id: 30, method: "primitive", params: {} }));
    expect(await messages.next()).toEqual({ jsonrpc: "2.0", id: 30, error: { code: -32603, message: "INTERNAL_ERROR" } });

    const request = peer.request("ctx.test", { x: 1 });
    const outbound = await messages.next();
    expect(outbound).toMatchObject({ jsonrpc: "2.0", id: -1, method: "ctx.test" });
    input.write(encodeFrame({ jsonrpc: "2.0", id: -1, result: 42 }));
    await expect(request).resolves.toBe(42);

    const remote = peer.request("ctx.remote", {});
    await messages.next();
    input.write(encodeFrame({ jsonrpc: "2.0", id: -2, error: { code: -32001, message: "no", data: { why: 1 } } }));
    await expect(remote).rejects.toEqual(new RunnerRpcRemoteError(-32001, "no", { why: 1 }));

    input.write(encodeFrame({ jsonrpc: "2.0", id: 4, method: "dispose", params: {} }));
    expect(await messages.next()).toEqual({ jsonrpc: "2.0", id: 4, result: null });
    await expect(running).resolves.toBeUndefined();
  });

  it("fails pending work on protocol and transport errors", async () => {
    const input = new PassThrough();
    const output = new PassThrough();
    const messages = new OutputMessages(output);
    const peer = new RunnerRpcPeer({
      input, output, maxFrameBytes: 4096, maxHeaderBytes: 128,
      onNotification: () => undefined,
      onRequest: async () => null,
    });
    const running = peer.run();
    const pending = peer.request("ctx.wait", {});
    await messages.next();
    input.write(encodeFrame({ jsonrpc: "1.0", id: -1, result: null }));
    await expect(pending).rejects.toBeInstanceOf(RunnerProtocolError);
    await expect(running).rejects.toBeInstanceOf(RunnerProtocolError);
    expect(() => peer.notify("after", {})).toThrow(RunnerProtocolError);

    const badOutput = new Writable({ write(_chunk, _encoding, callback) { callback(new Error("write failed")); } });
    badOutput.on("error", () => undefined);
    const peer2 = new RunnerRpcPeer({
      input: new PassThrough(), output: badOutput, maxFrameBytes: 100, maxHeaderBytes: 100,
      onNotification: () => undefined, onRequest: async () => null,
    });
    await expect(peer2.request("x", {})).rejects.toThrow("write failed");
    peer2.fail(new Error("closed"));
    peer2.fail(new Error("ignored"));
    expect(() => peer2.request("x", {})).toThrow("closed");
  });

  it("rejects every invalid JSON-RPC id and response shape", async () => {
    const messages: JsonRpcMessage[] = [
      { jsonrpc: "2.0", id: 0, method: "bad" },
      { jsonrpc: "2.0", id: 1, result: null },
      { jsonrpc: "2.0", id: -1, result: null },
    ];
    for (const message of messages) {
      const input = new PassThrough();
      const peer = new RunnerRpcPeer({
        input, output: new PassThrough(), maxFrameBytes: 1000, maxHeaderBytes: 100,
        onNotification: () => undefined, onRequest: async () => null,
      });
      const running = peer.run();
      input.end(encodeFrame(message));
      await expect(running).rejects.toBeInstanceOf(RunnerProtocolError);
    }

    for (const response of [
      { jsonrpc: "2.0", id: -1 },
      { jsonrpc: "2.0", id: -1, result: null, error: null },
      { jsonrpc: "2.0", id: -1, error: { code: "bad", message: "no" } },
      { jsonrpc: "2.0", id: -1, error: { code: -1, message: 2 } },
    ]) {
      const input = new PassThrough();
      const output = new PassThrough();
      const outbound = new OutputMessages(output);
      const peer = new RunnerRpcPeer({
        input, output, maxFrameBytes: 1000, maxHeaderBytes: 100,
        onNotification: () => undefined, onRequest: async () => null,
      });
      const running = peer.run();
      const pending = peer.request("ctx", {});
      await outbound.next();
      input.write(encodeFrame(response));
      await expect(pending).rejects.toBeInstanceOf(RunnerProtocolError);
      peer.requestStopAfterResponse();
      input.end();
      await expect(running).resolves.toBeUndefined();
    }
  });
});
