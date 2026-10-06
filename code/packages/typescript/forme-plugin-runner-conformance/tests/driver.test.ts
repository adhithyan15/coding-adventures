import { Buffer } from "node:buffer";
import { describe, expect, it } from "vitest";
import {
  ConformanceFrameDecoder,
  RunnerConformanceError,
  RunnerRemoteError,
  RunnerSession,
  decodeWireValue,
  encodeFrame,
  encodeWireValue,
  type RunnerCommand,
} from "../src/index.js";

function command(source: string, timeoutMs = 1_000): RunnerCommand {
  return {
    executable: process.execPath,
    args: ["--input-type=module", "--eval", source],
    modeArgument: false,
    timeoutMs,
    expectedRunner: "fake",
    expectedRunnerVersion: "0",
  };
}

function writer(messages: readonly Readonly<Record<string, unknown>>[], stderr = ""): string {
  const bytes = Buffer.concat(messages.map(message => encodeFrame(message))).toString("base64");
  return `process.stdin.once("data",()=>{process.stderr.write(${JSON.stringify(stderr)});process.stdout.write(Buffer.from(${JSON.stringify(bytes)},"base64"));});setTimeout(()=>{},5000);`;
}

describe("conformance wire codec", () => {
  it("round-trips bytes, arrays, ordinary objects, and reserved-looking objects", () => {
    const value = {
      bytes: new Uint8Array([0, 255]),
      list: [null, true, 4, "x"],
      reserved: { $forme: "bytes", base64: "plain", nested: { ok: true } },
    };
    expect(decodeWireValue(encodeWireValue(value))).toEqual(value);
    expect(decodeWireValue({ plain: { value: 1 } })).toEqual({ plain: { value: 1 } });
  });

  it("rejects circular values, oversized output, and malformed envelopes", () => {
    const circular: Record<string, unknown> = {};
    circular.self = circular;
    expect(() => encodeWireValue(circular)).toThrow(/circular/);
    expect(() => encodeFrame({ value: "too large" }, 1)).toThrow(/bound/);
    expect(() => encodeFrame({ value: new Uint8Array(4096) }, 4096)).toThrow(/bound/);
    expect(() => decodeWireValue({ $forme: "bytes", base64: "!" })).toThrow(/canonical/);
    expect(() => decodeWireValue({ $forme: "escaped-object", entries: [["x", 1], ["x", 2]] })).toThrow(/escaped-object/);
    expect(() => decodeWireValue({ $forme: "escaped-object", entries: ["bad"] })).toThrow(/escaped-object/);
  });

  it("preflights every JSON and binary shape before encoding", () => {
    const omitted = () => undefined;
    const rich = {
      nullValue: null,
      text: '"\\\b\f\n\r\t\u0001é漢😀\ud800',
      finite: 1.5,
      nonFinite: Number.NaN,
      yes: true,
      no: false,
      array: [undefined, omitted, Symbol("omitted"), 1, 2],
      object: { kept: 1, undefined, omitted, symbol: Symbol("omitted"), second: 2 },
      reserved: { $forme: "ordinary", first: 1, second: 2 },
      bytes: new Uint8Array([1, 2, 3]),
    };
    expect(() => encodeFrame(rich, 4096)).not.toThrow();
    expect(() => encodeFrame({ value: 1n }, 4096)).toThrow(/BigInt/);
    const circular: Record<string, unknown> = {};
    circular.self = circular;
    expect(() => encodeFrame(circular, 4096)).toThrow(/circular/);
  });

  it("decodes fragmented and adjacent frames with canonical bounds", () => {
    const decoder = new ConformanceFrameDecoder(128, 64);
    const first = encodeFrame({ jsonrpc: "2.0", id: 1, result: null });
    const second = encodeFrame({ jsonrpc: "2.0", method: "log", params: {} });
    expect(decoder.push(first.subarray(0, 8))).toEqual([]);
    expect(decoder.push(Buffer.concat([first.subarray(8), second]))).toEqual([
      { jsonrpc: "2.0", id: 1, result: null },
      { jsonrpc: "2.0", method: "log", params: {} },
    ]);
  });

  it.each([
    ["header bound", new ConformanceFrameDecoder(128, 4), Buffer.from("Content-Length: 1")],
    ["header terminator bound", new ConformanceFrameDecoder(128, 4), Buffer.from("12345\r\n\r\n")],
    ["missing length", new ConformanceFrameDecoder(128, 64), Buffer.from("X: 1\r\n\r\n")],
    ["duplicate length", new ConformanceFrameDecoder(128, 64), Buffer.from("Content-Length: 1\r\nContent-Length: 1\r\n\r\n")],
    ["noncanonical length", new ConformanceFrameDecoder(128, 64), Buffer.from("Content-Length: 01\r\n\r\n")],
    ["frame bound", new ConformanceFrameDecoder(1, 64), Buffer.from("Content-Length: 2\r\n\r\n")],
    ["invalid json", new ConformanceFrameDecoder(128, 64), Buffer.from("Content-Length: 1\r\n\r\n{")],
    ["non-object json", new ConformanceFrameDecoder(128, 64), Buffer.from("Content-Length: 4\r\n\r\nnull")],
  ])("rejects %s", (_label, decoder, bytes) => {
    expect(() => decoder.push(bytes)).toThrow(RunnerConformanceError);
  });
});

describe("runner session fail-closed behavior", () => {
  it("accepts a bounded response and records notifications", async () => {
    const session = new RunnerSession(command(writer([
      { jsonrpc: "2.0", method: "log", params: { message: "ok" } },
      { jsonrpc: "2.0", id: 1, result: { ok: true } },
    ], "diagnostic")), "ignored");
    await expect(session.request("test", {})).resolves.toEqual({ ok: true });
    expect(session.notifications).toEqual([{ jsonrpc: "2.0", method: "log", params: { message: "ok" } }]);
    await session.terminate();
    await expect(session.waitForExit()).resolves.toMatchObject({ signal: "SIGTERM", stderr: "diagnostic" });
    expect(() => session.beginRequest("late", {})).toThrow(/closed/);
    expect(() => session.writeRaw(new Uint8Array())).toThrow(/closed/);
    await session.terminate();
  });

  it("surfaces canonical remote errors", async () => {
    const session = new RunnerSession(command(writer([
      { jsonrpc: "2.0", id: 1, error: { code: -32001, message: "denied", data: { capability: "x" } } },
    ])), "ignored");
    await expect(session.request("test", {})).rejects.toEqual(expect.objectContaining<RunnerRemoteError>({
      name: "RunnerRemoteError", rpcCode: -32001, message: "denied", data: { capability: "x" },
    }));
    await session.terminate();
  });

  it.each([
    ["jsonrpc", { jsonrpc: "1.0", id: 1, result: null }],
    ["response id", { jsonrpc: "2.0", id: -1, result: null }],
    ["unknown id", { jsonrpc: "2.0", id: 2, result: null }],
    ["result xor error", { jsonrpc: "2.0", id: 1, result: null, error: null }],
    ["missing result", { jsonrpc: "2.0", id: 1 }],
    ["malformed error", { jsonrpc: "2.0", id: 1, error: { code: "bad", message: 1 } }],
    ["request id", { jsonrpc: "2.0", id: 1, method: "ctx.test", params: {} }],
    ["request params", { jsonrpc: "2.0", id: -1, method: "ctx.test", params: [] }],
  ])("rejects malformed %s messages", async (_label, message) => {
    const session = new RunnerSession(command(writer([message])), "ignored");
    await expect(session.request("test", {})).rejects.toBeInstanceOf(RunnerConformanceError);
    await session.terminate();
  });

  it("times out a silent runner and validates timeout configuration", async () => {
    expect(() => new RunnerSession(command("setTimeout(()=>{},5000)", 0), "ignored")).toThrow(/timeout/);
    expect(() => new RunnerSession(command("setTimeout(()=>{},5000)", 60_001), "ignored")).toThrow(/timeout/);
    const session = new RunnerSession(command("setTimeout(()=>{},5000)", 20), "ignored");
    await expect(session.request("silent", {})).rejects.toThrow(/timed out/);
    await session.terminate();
  });

  it("retires a timed-out runner and escalates when SIGTERM can be ignored", async () => {
    const session = new RunnerSession(command('process.on("SIGTERM",()=>{});setTimeout(()=>{},5000)', 200), "ignored");
    await expect(session.request("silent", {})).rejects.toThrow(/timed out/);
    // Windows terminates child processes immediately for SIGTERM, so the
    // handler cannot keep this fixture alive long enough to exercise SIGKILL.
    const expectedSignal = process.platform === "win32" ? "SIGTERM" : "SIGKILL";
    await expect(session.waitForExit()).resolves.toMatchObject({ signal: expectedSignal });
  });

  it("kills runners that exceed aggregate stdout and queued-work bounds", async () => {
    const stdoutFlood = new RunnerSession(command(
      'process.stdin.once("data",async()=>{const payload=Buffer.from(JSON.stringify({jsonrpc:"2.0",method:"log",params:{padding:"x".repeat(3900)}}));const frame=Buffer.concat([Buffer.from(`Content-Length: ${payload.length}\\r\\n\\r\\n`),payload]);for(let i=0;i<70;i++){process.stdout.write(frame);await new Promise(resolve=>setTimeout(resolve,1));}});setTimeout(()=>{},5000)',
      1_000,
    ), "ignored");
    await expect(stdoutFlood.request("test", {})).rejects.toThrow(/stdout exceeds/);
    await expect(stdoutFlood.waitForExit()).resolves.toMatchObject({ signal: expect.any(String) });

    const requests = Array.from({ length: 65 }, (_, index) => ({
      jsonrpc: "2.0", id: -(index + 1), method: "ctx.block", params: {},
    }));
    const queuedFlood = new RunnerSession(command(writer(requests), 1_000), "ignored");
    queuedFlood.onRequest(async () => new Promise<never>(() => undefined));
    await expect(queuedFlood.request("test", {})).rejects.toThrow(/queued work exceeds/);
    await queuedFlood.terminate();
  });

  it("kills runners that exceed the notification lifetime bound", async () => {
    const frames = Array.from({ length: 257 }, () => encodeFrame({
      jsonrpc: "2.0", method: "log", params: {},
    }).toString("base64"));
    const source = `process.stdin.once("data",async()=>{for(const frame of ${JSON.stringify(frames)}){process.stdout.write(Buffer.from(frame,"base64"));await new Promise(resolve=>setTimeout(resolve,1));}});setTimeout(()=>{},15000);`;
    // Windows timer granularity can make 257 deliberate yields exceed one
    // second even though the runner is healthy and continuously producing.
    const session = new RunnerSession(command(source, 10_000), "ignored");
    await expect(session.request("test", {})).rejects.toThrow(/notification count exceeds/);
    await session.terminate();
  });

  it("reports spawn failures", async () => {
    const session = new RunnerSession({
      ...command(""),
      executable: "/definitely/missing/forme-runner",
    }, "ignored");
    await expect(session.request("test", {})).rejects.toThrow(/could not be started/);
  });
});
