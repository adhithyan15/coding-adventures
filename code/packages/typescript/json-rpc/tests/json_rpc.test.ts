/**
 * Comprehensive tests for @coding-adventures/json-rpc
 *
 * Test plan:
 *   1. ErrorCodes — correct integer values
 *   2. parseMessage — all four shapes, error cases
 *   3. messageToObject — round-trip serialisation
 *   4. MessageReader — single message, back-to-back, EOF, malformed JSON,
 *                      valid JSON that is not a message, missing Content-Length
 *   5. MessageWriter — correct Content-Length header, UTF-8, \r\n separator
 *   6. Server — request dispatch, notification dispatch, unknown method,
 *               handler returning ResponseError, handler throwing, round-trip
 */

import { describe, it, expect, vi } from "vitest";
import { PassThrough } from "node:stream";
import {
  ErrorCodes,
  parseMessage,
  messageToObject,
  JsonRpcError,
  MessageReader,
  MessageWriter,
  Server,
  type Message,
  type ResponseError,
} from "../src/index.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Build a Content-Length-framed buffer from a JSON string. */
function frame(json: string): Buffer {
  const payload = Buffer.from(json, "utf8");
  const header = Buffer.from(`Content-Length: ${payload.length}\r\n\r\n`, "ascii");
  return Buffer.concat([header, payload]);
}

/** Push framed bytes into a PassThrough stream, then end it. */
function makeReadable(frames: Buffer[]): PassThrough {
  const pt = new PassThrough();
  for (const f of frames) pt.push(f);
  pt.push(null); // EOF
  return pt;
}

/** Capture all bytes written to a PassThrough stream. */
function makeWritable(): { stream: PassThrough; output: () => string } {
  const pt = new PassThrough();
  const chunks: Buffer[] = [];
  pt.on("data", (c: Buffer) => chunks.push(c));
  return {
    stream: pt,
    output: () => Buffer.concat(chunks).toString("utf8"),
  };
}

/** Parse all framed JSON payloads from a raw output string. */
function parseFrames(raw: string): unknown[] {
  const results: unknown[] = [];
  let rest = raw;
  while (rest.length > 0) {
    const clMatch = /Content-Length: (\d+)\r\n\r\n/.exec(rest);
    if (!clMatch) break;
    const len = parseInt(clMatch[1]!, 10);
    const start = clMatch.index! + clMatch[0].length;
    const payload = rest.slice(start, start + len);
    results.push(JSON.parse(payload));
    rest = rest.slice(start + len);
  }
  return results;
}

// ===========================================================================
// 1. ErrorCodes
// ===========================================================================

describe("ErrorCodes", () => {
  it("ParseError is -32700", () => {
    expect(ErrorCodes.ParseError).toBe(-32700);
  });

  it("InvalidRequest is -32600", () => {
    expect(ErrorCodes.InvalidRequest).toBe(-32600);
  });

  it("MethodNotFound is -32601", () => {
    expect(ErrorCodes.MethodNotFound).toBe(-32601);
  });

  it("InvalidParams is -32602", () => {
    expect(ErrorCodes.InvalidParams).toBe(-32602);
  });

  it("InternalError is -32603", () => {
    expect(ErrorCodes.InternalError).toBe(-32603);
  });
});

// ===========================================================================
// 2. parseMessage
// ===========================================================================

describe("parseMessage", () => {
  it("parses a Request with numeric id", () => {
    const msg = parseMessage({ jsonrpc: "2.0", id: 1, method: "ping" });
    expect(msg).toEqual({ type: "request", id: 1, method: "ping", params: undefined });
  });

  it("parses a Request with string id", () => {
    const msg = parseMessage({ jsonrpc: "2.0", id: "abc", method: "ping" });
    expect(msg.type).toBe("request");
    if (msg.type === "request") expect(msg.id).toBe("abc");
  });

  it("parses a Request with params", () => {
    const msg = parseMessage({
      jsonrpc: "2.0",
      id: 2,
      method: "textDocument/hover",
      params: { position: { line: 0, character: 3 } },
    });
    expect(msg.type).toBe("request");
    if (msg.type === "request") {
      expect(msg.method).toBe("textDocument/hover");
      expect(msg.params).toEqual({ position: { line: 0, character: 3 } });
    }
  });

  it("parses a Notification (no id)", () => {
    const msg = parseMessage({ jsonrpc: "2.0", method: "textDocument/didOpen" });
    expect(msg.type).toBe("notification");
    if (msg.type === "notification") expect(msg.method).toBe("textDocument/didOpen");
  });

  it("parses a success Response", () => {
    const msg = parseMessage({ jsonrpc: "2.0", id: 3, result: { ok: true } });
    expect(msg.type).toBe("response");
    if (msg.type === "response") {
      expect(msg.id).toBe(3);
      expect(msg.result).toEqual({ ok: true });
    }
  });

  it("parses an error Response", () => {
    const msg = parseMessage({
      jsonrpc: "2.0",
      id: 4,
      error: { code: -32601, message: "Method not found" },
    });
    expect(msg.type).toBe("response");
    if (msg.type === "response") {
      expect(msg.error?.code).toBe(-32601);
      expect(msg.error?.message).toBe("Method not found");
    }
  });

  it("parses an error Response with null id", () => {
    const msg = parseMessage({
      jsonrpc: "2.0",
      id: null,
      error: { code: -32700, message: "Parse error" },
    });
    expect(msg.type).toBe("response");
    if (msg.type === "response") expect(msg.id).toBeNull();
  });

  it("throws InvalidRequest for non-object", () => {
    expect(() => parseMessage("hello")).toThrow(JsonRpcError);
    expect(() => parseMessage(42)).toThrow(JsonRpcError);
    expect(() => parseMessage(null)).toThrow(JsonRpcError);
    expect(() => parseMessage([1, 2])).toThrow(JsonRpcError);
  });

  it("throws InvalidRequest for unrecognised shape", () => {
    expect(() => parseMessage({ jsonrpc: "2.0" })).toThrow(JsonRpcError);
  });

  it("throws InvalidRequest when id is not string or number for Request", () => {
    expect(() =>
      parseMessage({ jsonrpc: "2.0", id: [], method: "ping" }),
    ).toThrow(JsonRpcError);
  });

  it("throws InvalidRequest when error object is malformed", () => {
    expect(() =>
      parseMessage({ jsonrpc: "2.0", id: 1, error: "not an object" }),
    ).toThrow(JsonRpcError);
  });
});

// ===========================================================================
// 3. messageToObject
// ===========================================================================

describe("messageToObject", () => {
  it("serialises a Request", () => {
    const obj = messageToObject({ type: "request", id: 1, method: "ping" });
    expect(obj).toEqual({ jsonrpc: "2.0", id: 1, method: "ping" });
  });

  it("includes params when present", () => {
    const obj = messageToObject({
      type: "request",
      id: 1,
      method: "ping",
      params: { x: 1 },
    });
    expect(obj["params"]).toEqual({ x: 1 });
  });

  it("serialises a Notification", () => {
    const obj = messageToObject({ type: "notification", method: "$/ping" });
    expect(obj).toEqual({ jsonrpc: "2.0", method: "$/ping" });
  });

  it("serialises a success Response", () => {
    const obj = messageToObject({ type: "response", id: 2, result: 42 });
    expect(obj).toEqual({ jsonrpc: "2.0", id: 2, result: 42 });
  });

  it("serialises an error Response", () => {
    const obj = messageToObject({
      type: "response",
      id: 3,
      error: { code: -32601, message: "Method not found" },
    });
    expect((obj["error"] as Record<string, unknown>)["code"]).toBe(-32601);
  });

  it("round-trips a Request through JSON.stringify and parseMessage", () => {
    const original: Message = {
      type: "request",
      id: 7,
      method: "textDocument/hover",
      params: { line: 0 },
    };
    const json = JSON.stringify(messageToObject(original));
    const parsed = parseMessage(JSON.parse(json));
    expect(parsed).toEqual(original);
  });
});

// ===========================================================================
// 4. MessageReader
// ===========================================================================

describe("MessageReader", () => {
  it("reads a single Request message", async () => {
    const json = JSON.stringify({ jsonrpc: "2.0", id: 1, method: "initialize" });
    const stream = makeReadable([frame(json)]);
    const reader = new MessageReader(stream);
    const msg = await reader.readMessage();
    expect(msg?.type).toBe("request");
    if (msg?.type === "request") expect(msg.method).toBe("initialize");
  });

  it("reads back-to-back messages", async () => {
    const j1 = JSON.stringify({ jsonrpc: "2.0", id: 1, method: "ping" });
    const j2 = JSON.stringify({ jsonrpc: "2.0", method: "notify" });
    const stream = makeReadable([frame(j1), frame(j2)]);
    const reader = new MessageReader(stream);
    const m1 = await reader.readMessage();
    const m2 = await reader.readMessage();
    expect(m1?.type).toBe("request");
    expect(m2?.type).toBe("notification");
  });

  it("returns null on EOF with no data", async () => {
    const stream = makeReadable([]);
    const reader = new MessageReader(stream);
    const msg = await reader.readMessage();
    expect(msg).toBeNull();
  });

  it("returns null after last message", async () => {
    const json = JSON.stringify({ jsonrpc: "2.0", id: 1, method: "ping" });
    const stream = makeReadable([frame(json)]);
    const reader = new MessageReader(stream);
    await reader.readMessage();
    const msg = await reader.readMessage();
    expect(msg).toBeNull();
  });

  it("throws ParseError on malformed JSON", async () => {
    const bad = Buffer.from("Content-Length: 5\r\n\r\n{bad}", "utf8");
    const stream = makeReadable([bad]);
    const reader = new MessageReader(stream);
    await expect(reader.readMessage()).rejects.toMatchObject({
      code: ErrorCodes.ParseError,
    });
  });

  it("throws InvalidRequest on valid JSON that is not a message", async () => {
    const json = JSON.stringify([1, 2, 3]); // JSON array — not a message
    const stream = makeReadable([frame(json)]);
    const reader = new MessageReader(stream);
    await expect(reader.readMessage()).rejects.toMatchObject({
      code: ErrorCodes.InvalidRequest,
    });
  });

  it("reads a Notification message", async () => {
    const json = JSON.stringify({ jsonrpc: "2.0", method: "textDocument/didOpen", params: {} });
    const stream = makeReadable([frame(json)]);
    const reader = new MessageReader(stream);
    const msg = await reader.readMessage();
    expect(msg?.type).toBe("notification");
  });

  it("reads a Response message", async () => {
    const json = JSON.stringify({ jsonrpc: "2.0", id: 5, result: { ok: true } });
    const stream = makeReadable([frame(json)]);
    const reader = new MessageReader(stream);
    const msg = await reader.readMessage();
    expect(msg?.type).toBe("response");
  });

  it("readRaw returns raw JSON string without parsing", async () => {
    const json = JSON.stringify({ jsonrpc: "2.0", id: 1, method: "raw" });
    const stream = makeReadable([frame(json)]);
    const reader = new MessageReader(stream);
    const raw = await reader.readRaw();
    expect(raw).toBe(json);
  });

  it("readRaw returns null on EOF", async () => {
    const stream = makeReadable([]);
    const reader = new MessageReader(stream);
    const raw = await reader.readRaw();
    expect(raw).toBeNull();
  });
});

// ===========================================================================
// 5. MessageWriter
// ===========================================================================

describe("MessageWriter", () => {
  it("writes correct Content-Length header", () => {
    const { stream, output } = makeWritable();
    const writer = new MessageWriter(stream);
    writer.writeMessage({ type: "request", id: 1, method: "ping" });
    const raw = output();
    const json = JSON.stringify({ jsonrpc: "2.0", id: 1, method: "ping" });
    const expectedLen = Buffer.from(json, "utf8").length;
    expect(raw).toContain(`Content-Length: ${expectedLen}`);
  });

  it("uses \\r\\n\\r\\n as header/body separator", () => {
    const { stream, output } = makeWritable();
    const writer = new MessageWriter(stream);
    writer.writeMessage({ type: "notification", method: "ping" });
    expect(output()).toContain("\r\n\r\n");
  });

  it("payload is valid UTF-8 JSON", () => {
    const { stream, output } = makeWritable();
    const writer = new MessageWriter(stream);
    writer.writeMessage({ type: "response", id: 1, result: { x: 42 } });
    const raw = output();
    const jsonStart = raw.indexOf("\r\n\r\n") + 4;
    const payload = raw.slice(jsonStart);
    expect(() => JSON.parse(payload)).not.toThrow();
  });

  it("Content-Length accounts for multi-byte Unicode", () => {
    const { stream, output } = makeWritable();
    const writer = new MessageWriter(stream);
    // "€" is U+20AC — 3 bytes in UTF-8 but 1 character.
    writer.writeMessage({ type: "notification", method: "ping", params: { s: "€" } });
    const raw = output();
    const clMatch = /Content-Length: (\d+)/.exec(raw)!;
    const claimed = parseInt(clMatch[1]!, 10);
    const jsonStart = raw.indexOf("\r\n\r\n") + 4;
    const actualBytes = Buffer.from(raw.slice(jsonStart), "utf8").length;
    expect(claimed).toBe(actualBytes);
  });

  it("writeRaw writes a pre-serialized JSON string", () => {
    const { stream, output } = makeWritable();
    const writer = new MessageWriter(stream);
    const json = '{"jsonrpc":"2.0","id":9,"result":null}';
    writer.writeRaw(json);
    const raw = output();
    expect(raw).toContain(json);
    expect(raw).toContain(`Content-Length: ${Buffer.from(json, "utf8").length}`);
  });
});

// ===========================================================================
// 6. Server
// ===========================================================================

describe("Server", () => {
  it("dispatches a Request to its handler and writes a Response", async () => {
    const reqJson = JSON.stringify({ jsonrpc: "2.0", id: 1, method: "ping" });
    const input = makeReadable([frame(reqJson)]);
    const { stream: output, output: getOutput } = makeWritable();

    const server = new Server(input, output);
    server.onRequest("ping", (_id, _params) => "pong");
    await server.serve();

    const frames = parseFrames(getOutput());
    expect(frames).toHaveLength(1);
    const resp = frames[0] as Record<string, unknown>;
    expect(resp["id"]).toBe(1);
    expect(resp["result"]).toBe("pong");
  });

  it("dispatches a Notification to its handler without writing a response", async () => {
    const notifJson = JSON.stringify({ jsonrpc: "2.0", method: "notify", params: { x: 1 } });
    const input = makeReadable([frame(notifJson)]);
    const { stream: output, output: getOutput } = makeWritable();

    const handlerSpy = vi.fn();
    const server = new Server(input, output);
    server.onNotification("notify", handlerSpy);
    await server.serve();

    expect(handlerSpy).toHaveBeenCalledWith({ x: 1 });
    expect(getOutput()).toBe(""); // No response written
  });

  it("sends -32601 for unknown request method", async () => {
    const reqJson = JSON.stringify({ jsonrpc: "2.0", id: 2, method: "unknown/method" });
    const input = makeReadable([frame(reqJson)]);
    const { stream: output, output: getOutput } = makeWritable();

    const server = new Server(input, output);
    await server.serve();

    const frames = parseFrames(getOutput());
    expect(frames).toHaveLength(1);
    const resp = frames[0] as Record<string, unknown>;
    const err = resp["error"] as Record<string, unknown>;
    expect(err["code"]).toBe(ErrorCodes.MethodNotFound);
  });

  it("sends error response when handler returns a ResponseError", async () => {
    const reqJson = JSON.stringify({ jsonrpc: "2.0", id: 3, method: "fail" });
    const input = makeReadable([frame(reqJson)]);
    const { stream: output, output: getOutput } = makeWritable();

    const server = new Server(input, output);
    server.onRequest("fail", (_id, _params) => {
      const err: ResponseError = { code: -32602, message: "Invalid params" };
      return err;
    });
    await server.serve();

    const frames = parseFrames(getOutput());
    const resp = frames[0] as Record<string, unknown>;
    const err = resp["error"] as Record<string, unknown>;
    expect(err["code"]).toBe(ErrorCodes.InvalidParams);
  });

  it("sends -32603 when handler throws", async () => {
    const reqJson = JSON.stringify({ jsonrpc: "2.0", id: 4, method: "boom" });
    const input = makeReadable([frame(reqJson)]);
    const { stream: output, output: getOutput } = makeWritable();

    const server = new Server(input, output);
    server.onRequest("boom", () => {
      throw new Error("Unexpected failure");
    });
    await server.serve();

    const frames = parseFrames(getOutput());
    const resp = frames[0] as Record<string, unknown>;
    const err = resp["error"] as Record<string, unknown>;
    expect(err["code"]).toBe(ErrorCodes.InternalError);
  });

  it("ignores unknown notification silently", async () => {
    const notifJson = JSON.stringify({ jsonrpc: "2.0", method: "unknown/notif" });
    const input = makeReadable([frame(notifJson)]);
    const { stream: output, output: getOutput } = makeWritable();

    const server = new Server(input, output);
    await server.serve();

    expect(getOutput()).toBe(""); // No response, no error
  });

  it("handles multiple requests in sequence", async () => {
    const req1 = JSON.stringify({ jsonrpc: "2.0", id: 10, method: "add", params: { a: 1, b: 2 } });
    const req2 = JSON.stringify({ jsonrpc: "2.0", id: 11, method: "add", params: { a: 3, b: 4 } });
    const input = makeReadable([frame(req1), frame(req2)]);
    const { stream: output, output: getOutput } = makeWritable();

    const server = new Server(input, output);
    server.onRequest("add", (_id, params) => {
      const p = params as { a: number; b: number };
      return p.a + p.b;
    });
    await server.serve();

    const frames = parseFrames(getOutput()) as Array<Record<string, unknown>>;
    expect(frames).toHaveLength(2);
    expect(frames[0]!["result"]).toBe(3);
    expect(frames[1]!["result"]).toBe(7);
  });

  it("is chainable via onRequest and onNotification", () => {
    const input = new PassThrough();
    const output = new PassThrough();
    input.push(null);

    const server = new Server(input, output);
    const result = server
      .onRequest("a", () => null)
      .onNotification("b", () => undefined);
    expect(result).toBe(server);
  });

  it("sends error response on malformed JSON framing", async () => {
    // A frame with a valid Content-Length but bad JSON inside
    const badPayload = Buffer.from("Content-Length: 5\r\n\r\n{bad}", "utf8");
    const input = new PassThrough();
    input.push(badPayload);
    input.push(null);

    const { stream: output, output: getOutput } = makeWritable();
    const server = new Server(input, output);
    await server.serve();

    const frames = parseFrames(getOutput());
    // Server should send an error response with Parse error code
    expect(frames.length).toBeGreaterThanOrEqual(1);
    const resp = frames[0] as Record<string, unknown>;
    const err = resp["error"] as Record<string, unknown>;
    expect(err["code"]).toBe(ErrorCodes.ParseError);
  });

  it("round-trip: Request → Server → Response parsed back correctly", async () => {
    const reqJson = JSON.stringify({
      jsonrpc: "2.0",
      id: 99,
      method: "echo",
      params: { msg: "hello" },
    });
    const input = makeReadable([frame(reqJson)]);
    const { stream: output, output: getOutput } = makeWritable();

    const server = new Server(input, output);
    server.onRequest("echo", (_id, params) => params);
    await server.serve();

    const frames = parseFrames(getOutput());
    const resp = frames[0] as Record<string, unknown>;
    expect(resp["id"]).toBe(99);
    expect(resp["result"]).toEqual({ msg: "hello" });
  });
});
