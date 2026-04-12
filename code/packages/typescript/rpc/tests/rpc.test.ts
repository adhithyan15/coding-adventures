/**
 * Comprehensive tests for @coding-adventures/rpc
 *
 * Test plan:
 *   1. RpcErrorCodes — correct integer values
 *   2. RpcError — throwable error class
 *   3. MockCodec — in-memory codec using JSON.stringify/parse internally
 *   4. MockFramer — in-memory framer using Node.js Buffers
 *   5. RpcServer:
 *        a. dispatches request to handler and writes response
 *        b. unknown method → -32601 MethodNotFound
 *        c. handler throws → -32603 InternalError (panic safety)
 *        d. dispatches notification to handler, no response written
 *        e. unknown notification silently dropped (no response, no error)
 *        f. codec decode error → error response with null id
 *        g. framer readFrame error → error response with null id
 *        h. multiple requests in sequence
 *        i. onRequest/onNotification are chainable
 *   6. RpcClient:
 *        a. request() encodes message and returns decoded result
 *        b. request() throws RpcClientError on server error response
 *        c. request() throws RpcClientError on connection closed (EOF)
 *        d. notify() encodes and writes without waiting for response
 *        e. onNotification() handler called for server-push notifications
 *        f. request ids are auto-generated and monotonically increasing
 *        g. unknown server-push notification is silently dropped
 *   7. Message type round-trips through MockCodec
 */

import { describe, it, expect, vi } from "vitest";
import {
  RpcErrorCodes,
  RpcError,
  RpcServer,
  RpcClient,
  RpcClientError,
} from "../src/index.js";
import type {
  RpcCodec,
  RpcFramer,
  RpcMessage,
  RpcId,
} from "../src/index.js";

// ===========================================================================
// Mock implementations
//
// These are the pluggable codec and framer used throughout the tests. The
// point of the `rpc` package is that the server and client are agnostic to
// the codec and framer implementations — so using mocks here exercises that
// interface contract without coupling the tests to any specific format.
// ===========================================================================

// ---------------------------------------------------------------------------
// MockCodec — JSON.stringify / JSON.parse under the hood
//
// The codec translates between RpcMessage<unknown> and Uint8Array. Since
// this package is codec-agnostic, using JSON internally is fine for tests —
// the important thing is that the interface is pluggable.
// ---------------------------------------------------------------------------

/**
 * Wire-format discriminator tags used by MockCodec.
 * The JSON shape is {"kind":"...", "id":..., "method":..., ...}.
 */
type WireMsg =
  | { kind: "request"; id: RpcId; method: string; params?: unknown }
  | { kind: "response"; id: RpcId; result: unknown }
  | { kind: "error"; id: RpcId | null; code: number; message: string; data?: unknown }
  | { kind: "notification"; method: string; params?: unknown };

/**
 * A test codec that uses JSON internally.
 *
 * encode: JSON.stringify + Buffer.from
 * decode: Buffer.toString + JSON.parse + shape validation
 */
class MockCodec implements RpcCodec<unknown> {
  encode(msg: RpcMessage<unknown>): Uint8Array {
    // Map our typed RpcMessage to a plain wire object.
    let wire: WireMsg;
    switch (msg.kind) {
      case "request":
        wire = { kind: "request", id: msg.id, method: msg.method };
        if (msg.params !== undefined) wire.params = msg.params;
        break;
      case "response":
        wire = { kind: "response", id: msg.id, result: msg.result };
        break;
      case "error":
        wire = { kind: "error", id: msg.id, code: msg.code, message: msg.message };
        if (msg.data !== undefined) wire.data = msg.data;
        break;
      case "notification":
        wire = { kind: "notification", method: msg.method };
        if (msg.params !== undefined) wire.params = msg.params;
        break;
    }
    return Buffer.from(JSON.stringify(wire), "utf8");
  }

  decode(data: Uint8Array): RpcMessage<unknown> {
    let wire: WireMsg;
    try {
      wire = JSON.parse(Buffer.from(data).toString("utf8")) as WireMsg;
    } catch {
      throw new RpcError(RpcErrorCodes.ParseError, "MockCodec: invalid JSON");
    }

    if (!wire || typeof wire !== "object" || !("kind" in wire)) {
      throw new RpcError(RpcErrorCodes.InvalidRequest, "MockCodec: missing kind");
    }

    switch (wire.kind) {
      case "request":
        if (typeof wire.id !== "string" && typeof wire.id !== "number") {
          throw new RpcError(RpcErrorCodes.InvalidRequest, "MockCodec: bad request id");
        }
        return { kind: "request", id: wire.id, method: wire.method, params: wire.params };
      case "response":
        if (typeof wire.id !== "string" && typeof wire.id !== "number") {
          throw new RpcError(RpcErrorCodes.InvalidRequest, "MockCodec: bad response id");
        }
        return { kind: "response", id: wire.id, result: wire.result };
      case "error":
        return { kind: "error", id: wire.id, code: wire.code, message: wire.message, data: wire.data };
      case "notification":
        return { kind: "notification", method: wire.method, params: wire.params };
      default:
        throw new RpcError(RpcErrorCodes.InvalidRequest, "MockCodec: unknown kind");
    }
  }
}

// ---------------------------------------------------------------------------
// MockFramer — in-memory queue of byte frames
//
// readFrame() pops from a pre-loaded queue of Uint8Array frames.
// writeFrame() pushes to a written-frames list.
//
// This lets tests drive the server/client loop with predefined inputs and
// inspect exactly what frames were written out.
// ---------------------------------------------------------------------------

/**
 * An in-memory framer for testing.
 *
 * Preload `inputFrames` with the frames the server/client will receive.
 * After the test, inspect `writtenFrames` to verify what was sent.
 */
class MockFramer implements RpcFramer {
  /** Queue of frames to return from readFrame(). null = EOF. */
  private readonly inputFrames: Array<Uint8Array | null>;
  /** All frames that have been written via writeFrame(). */
  readonly writtenFrames: Uint8Array[] = [];
  private readIndex = 0;

  constructor(inputFrames: Array<Uint8Array | null> = []) {
    // Always end with null (EOF) so the server loop terminates.
    this.inputFrames = inputFrames.at(-1) === null
      ? inputFrames
      : [...inputFrames, null];
  }

  readFrame(): Uint8Array | null {
    if (this.readIndex >= this.inputFrames.length) return null;
    return this.inputFrames[this.readIndex++]!;
  }

  writeFrame(data: Uint8Array): void {
    this.writtenFrames.push(data);
  }
}

// ---------------------------------------------------------------------------
// Helper: encode a message into a frame using MockCodec
// ---------------------------------------------------------------------------
const codec = new MockCodec();

function encodeFrame(msg: RpcMessage<unknown>): Uint8Array {
  return codec.encode(msg);
}

function decodeFrame(frame: Uint8Array): RpcMessage<unknown> {
  return codec.decode(frame);
}

// ===========================================================================
// 1. RpcErrorCodes
// ===========================================================================

describe("RpcErrorCodes", () => {
  it("ParseError is -32700", () => {
    expect(RpcErrorCodes.ParseError).toBe(-32700);
  });

  it("InvalidRequest is -32600", () => {
    expect(RpcErrorCodes.InvalidRequest).toBe(-32600);
  });

  it("MethodNotFound is -32601", () => {
    expect(RpcErrorCodes.MethodNotFound).toBe(-32601);
  });

  it("InvalidParams is -32602", () => {
    expect(RpcErrorCodes.InvalidParams).toBe(-32602);
  });

  it("InternalError is -32603", () => {
    expect(RpcErrorCodes.InternalError).toBe(-32603);
  });
});

// ===========================================================================
// 2. RpcError
// ===========================================================================

describe("RpcError", () => {
  it("is throwable and has code and message", () => {
    const err = new RpcError(RpcErrorCodes.ParseError, "bad bytes");
    expect(err).toBeInstanceOf(Error);
    expect(err).toBeInstanceOf(RpcError);
    expect(err.code).toBe(-32700);
    expect(err.message).toBe("bad bytes");
    expect(err.name).toBe("RpcError");
  });

  it("can be caught and inspected", () => {
    let caught: unknown;
    try {
      throw new RpcError(RpcErrorCodes.MethodNotFound, "no such method");
    } catch (e) {
      caught = e;
    }
    expect(caught).toBeInstanceOf(RpcError);
    if (caught instanceof RpcError) {
      expect(caught.code).toBe(-32601);
    }
  });
});

// ===========================================================================
// 3. MockCodec round-trips
// ===========================================================================

describe("MockCodec (round-trips)", () => {
  it("encodes and decodes RpcRequest", () => {
    const orig: RpcMessage<unknown> = {
      kind: "request", id: 1, method: "ping", params: { x: 42 },
    };
    const decoded = decodeFrame(encodeFrame(orig));
    expect(decoded).toEqual(orig);
  });

  it("encodes and decodes RpcResponse", () => {
    const orig: RpcMessage<unknown> = {
      kind: "response", id: 2, result: "pong",
    };
    const decoded = decodeFrame(encodeFrame(orig));
    expect(decoded).toEqual(orig);
  });

  it("encodes and decodes RpcErrorResponse", () => {
    const orig: RpcMessage<unknown> = {
      kind: "error", id: 3, code: -32601, message: "Method not found",
    };
    const decoded = decodeFrame(encodeFrame(orig));
    expect(decoded).toEqual(orig);
  });

  it("encodes and decodes RpcNotification", () => {
    const orig: RpcMessage<unknown> = {
      kind: "notification", method: "update", params: [1, 2, 3],
    };
    const decoded = decodeFrame(encodeFrame(orig));
    expect(decoded).toEqual(orig);
  });

  it("throws ParseError on non-JSON bytes", () => {
    expect(() => codec.decode(Buffer.from("{bad json", "utf8")))
      .toThrow(RpcError);
  });

  it("ParseError has code -32700", () => {
    try {
      codec.decode(Buffer.from("not json at all!!!", "utf8"));
    } catch (e) {
      expect(e).toBeInstanceOf(RpcError);
      if (e instanceof RpcError) expect(e.code).toBe(-32700);
    }
  });

  it("throws InvalidRequest for JSON without kind", () => {
    expect(() => codec.decode(Buffer.from('{"foo":"bar"}', "utf8")))
      .toThrow(RpcError);
  });
});

// ===========================================================================
// 4. MockFramer
// ===========================================================================

describe("MockFramer", () => {
  it("returns frames in order", () => {
    const f1 = Buffer.from("frame1");
    const f2 = Buffer.from("frame2");
    const framer = new MockFramer([f1, f2]);
    expect(framer.readFrame()).toEqual(f1);
    expect(framer.readFrame()).toEqual(f2);
    expect(framer.readFrame()).toBeNull();
  });

  it("returns null on EOF immediately when empty", () => {
    const framer = new MockFramer([]);
    expect(framer.readFrame()).toBeNull();
  });

  it("stores written frames", () => {
    const framer = new MockFramer([]);
    const data = Buffer.from("hello");
    framer.writeFrame(data);
    expect(framer.writtenFrames).toHaveLength(1);
    expect(framer.writtenFrames[0]).toEqual(data);
  });
});

// ===========================================================================
// 5. RpcServer
// ===========================================================================

describe("RpcServer", () => {
  // Helper: run server with given input frames and return written frames decoded.
  function runServer(
    inputFrames: Array<Uint8Array | null>,
    setup: (server: RpcServer<unknown>) => void,
  ): RpcMessage<unknown>[] {
    const framer = new MockFramer(inputFrames);
    const server = new RpcServer(new MockCodec(), framer);
    setup(server);
    server.serve();
    return framer.writtenFrames.map((f) => decodeFrame(f));
  }

  // -------------------------------------------------------------------------
  // 5a. Dispatches request to handler and writes response
  // -------------------------------------------------------------------------

  it("dispatches a Request to its handler and writes a success Response", () => {
    const responses = runServer(
      [encodeFrame({ kind: "request", id: 1, method: "ping" })],
      (server) => server.onRequest("ping", (_id, _params) => "pong"),
    );
    expect(responses).toHaveLength(1);
    const resp = responses[0]!;
    expect(resp.kind).toBe("response");
    if (resp.kind === "response") {
      expect(resp.id).toBe(1);
      expect(resp.result).toBe("pong");
    }
  });

  it("passes params to the handler", () => {
    const responses = runServer(
      [encodeFrame({ kind: "request", id: 2, method: "add", params: { a: 3, b: 4 } })],
      (server) => server.onRequest("add", (_id, params) => {
        const { a, b } = params as { a: number; b: number };
        return a + b;
      }),
    );
    const resp = responses[0]!;
    expect(resp.kind).toBe("response");
    if (resp.kind === "response") expect(resp.result).toBe(7);
  });

  it("passes the correct id to the handler", () => {
    let receivedId: RpcId | undefined;
    runServer(
      [encodeFrame({ kind: "request", id: 99, method: "check" })],
      (server) => server.onRequest("check", (id, _params) => {
        receivedId = id;
        return "ok";
      }),
    );
    expect(receivedId).toBe(99);
  });

  // -------------------------------------------------------------------------
  // 5b. Unknown method → -32601 MethodNotFound
  // -------------------------------------------------------------------------

  it("sends -32601 MethodNotFound for unregistered request method", () => {
    const responses = runServer(
      [encodeFrame({ kind: "request", id: 3, method: "unknown/method" })],
      (_server) => { /* no handlers registered */ },
    );
    expect(responses).toHaveLength(1);
    const resp = responses[0]!;
    expect(resp.kind).toBe("error");
    if (resp.kind === "error") {
      expect(resp.id).toBe(3);
      expect(resp.code).toBe(RpcErrorCodes.MethodNotFound);
    }
  });

  // -------------------------------------------------------------------------
  // 5c. Handler throws → -32603 InternalError (panic safety)
  // -------------------------------------------------------------------------

  it("sends -32603 InternalError when handler throws an Error", () => {
    const responses = runServer(
      [encodeFrame({ kind: "request", id: 4, method: "boom" })],
      (server) => server.onRequest("boom", () => {
        throw new Error("Unexpected failure");
      }),
    );
    const resp = responses[0]!;
    expect(resp.kind).toBe("error");
    if (resp.kind === "error") {
      expect(resp.code).toBe(RpcErrorCodes.InternalError);
      expect(resp.id).toBe(4);
    }
  });

  it("sends -32603 InternalError when handler throws a non-Error value", () => {
    const responses = runServer(
      [encodeFrame({ kind: "request", id: 5, method: "bad" })],
      (server) => server.onRequest("bad", () => {
        throw "just a string panic"; // eslint-disable-line @typescript-eslint/only-throw-error
      }),
    );
    const resp = responses[0]!;
    expect(resp.kind).toBe("error");
    if (resp.kind === "error") expect(resp.code).toBe(-32603);
  });

  it("continues serving after a handler throws (does not crash)", () => {
    const responses = runServer(
      [
        encodeFrame({ kind: "request", id: 6, method: "boom" }),
        encodeFrame({ kind: "request", id: 7, method: "safe" }),
      ],
      (server) => {
        server.onRequest("boom", () => { throw new Error("boom!"); });
        server.onRequest("safe", () => "ok");
      },
    );
    // Both requests should produce responses
    expect(responses).toHaveLength(2);
    const [first, second] = responses;
    expect(first!.kind).toBe("error");
    expect(second!.kind).toBe("response");
  });

  // -------------------------------------------------------------------------
  // 5d. Dispatches notification to handler, no response written
  // -------------------------------------------------------------------------

  it("dispatches Notification to its handler and writes NO response", () => {
    const spy = vi.fn();
    const framer = new MockFramer([
      encodeFrame({ kind: "notification", method: "ping", params: { t: 1 } }),
    ]);
    const server = new RpcServer(new MockCodec(), framer);
    server.onNotification("ping", spy);
    server.serve();

    // Handler was called with the params
    expect(spy).toHaveBeenCalledOnce();
    expect(spy).toHaveBeenCalledWith({ t: 1 });

    // No response frames written
    expect(framer.writtenFrames).toHaveLength(0);
  });

  // -------------------------------------------------------------------------
  // 5e. Unknown notification silently dropped
  // -------------------------------------------------------------------------

  it("silently drops an unknown Notification (no response, no error)", () => {
    const framer = new MockFramer([
      encodeFrame({ kind: "notification", method: "unknown/notif" }),
    ]);
    const server = new RpcServer(new MockCodec(), framer);
    server.serve(); // no handlers registered

    expect(framer.writtenFrames).toHaveLength(0);
  });

  // -------------------------------------------------------------------------
  // 5f. Codec decode error → error response with null id
  // -------------------------------------------------------------------------

  it("sends error response with null id when codec cannot decode a frame", () => {
    const badFrame = Buffer.from("this is not valid JSON or a valid frame");
    const framer = new MockFramer([badFrame]);
    const server = new RpcServer(new MockCodec(), framer);
    server.serve();

    expect(framer.writtenFrames).toHaveLength(1);
    const resp = decodeFrame(framer.writtenFrames[0]!);
    expect(resp.kind).toBe("error");
    if (resp.kind === "error") {
      expect(resp.id).toBeNull();
      expect(resp.code).toBe(RpcErrorCodes.ParseError);
    }
  });

  // -------------------------------------------------------------------------
  // 5g. Framer readFrame error → error response with null id
  // -------------------------------------------------------------------------

  it("sends error response with null id when framer throws on readFrame", () => {
    // Build a framer that throws on the first readFrame() call, then returns
    // null (EOF) on subsequent calls. This simulates a malformed frame header.
    let readCount = 0;
    const writtenByErrorFramer: Uint8Array[] = [];
    const errorFramer: RpcFramer = {
      readFrame() {
        readCount++;
        if (readCount === 1) {
          throw new RpcError(RpcErrorCodes.ParseError, "Framing error");
        }
        return null; // EOF — server exits cleanly after the error
      },
      writeFrame(data: Uint8Array) {
        writtenByErrorFramer.push(data);
      },
    };

    const server = new RpcServer(new MockCodec(), errorFramer);
    server.serve();

    expect(writtenByErrorFramer.length).toBeGreaterThanOrEqual(1);
    const resp = decodeFrame(writtenByErrorFramer[0]!);
    expect(resp.kind).toBe("error");
    if (resp.kind === "error") {
      expect(resp.id).toBeNull();
      expect(resp.code).toBe(RpcErrorCodes.ParseError);
    }
  });

  // -------------------------------------------------------------------------
  // 5h. Multiple requests in sequence
  // -------------------------------------------------------------------------

  it("handles multiple requests in sequence", () => {
    const responses = runServer(
      [
        encodeFrame({ kind: "request", id: 10, method: "echo", params: "hello" }),
        encodeFrame({ kind: "request", id: 11, method: "echo", params: "world" }),
      ],
      (server) => server.onRequest("echo", (_id, params) => params),
    );
    expect(responses).toHaveLength(2);
    expect(responses[0]!.kind).toBe("response");
    expect(responses[1]!.kind).toBe("response");
    if (responses[0]!.kind === "response") expect(responses[0]!.result).toBe("hello");
    if (responses[1]!.kind === "response") expect(responses[1]!.result).toBe("world");
  });

  it("handles interleaved requests and notifications", () => {
    const notifSpy = vi.fn();
    const responses = runServer(
      [
        encodeFrame({ kind: "request", id: 20, method: "ping" }),
        encodeFrame({ kind: "notification", method: "event" }),
        encodeFrame({ kind: "request", id: 21, method: "ping" }),
      ],
      (server) => {
        server.onRequest("ping", () => "pong");
        server.onNotification("event", notifSpy);
      },
    );
    // Two request responses, zero notification responses
    expect(responses).toHaveLength(2);
    expect(notifSpy).toHaveBeenCalledOnce();
  });

  // -------------------------------------------------------------------------
  // 5i. onRequest/onNotification are chainable
  // -------------------------------------------------------------------------

  it("onRequest and onNotification return `this` for chaining", () => {
    const framer = new MockFramer([]);
    const server = new RpcServer(new MockCodec(), framer);
    const result = server
      .onRequest("a", () => null)
      .onNotification("b", () => undefined)
      .onRequest("c", () => 42);
    expect(result).toBe(server);
  });

  // -------------------------------------------------------------------------
  // Additional edge cases
  // -------------------------------------------------------------------------

  it("discards incoming Response messages in server-only mode (no write)", () => {
    const responses = runServer(
      [encodeFrame({ kind: "response", id: 1, result: "ignored" })],
      (_server) => { /* no handlers */ },
    );
    // Server received a response but should not write anything in response
    expect(responses).toHaveLength(0);
  });

  it("discards incoming ErrorResponse messages in server-only mode", () => {
    const responses = runServer(
      [encodeFrame({ kind: "error", id: 1, code: -32601, message: "not found" })],
      (_server) => { /* no handlers */ },
    );
    expect(responses).toHaveLength(0);
  });

  it("notification handler error is silently swallowed", () => {
    const framer = new MockFramer([
      encodeFrame({ kind: "notification", method: "boom" }),
    ]);
    const server = new RpcServer(new MockCodec(), framer);
    server.onNotification("boom", () => { throw new Error("handler exploded"); });
    // Should not throw
    expect(() => server.serve()).not.toThrow();
    // No response written
    expect(framer.writtenFrames).toHaveLength(0);
  });

  it("handles request with no params (params is undefined)", () => {
    let receivedParams: unknown = "SENTINEL";
    runServer(
      [encodeFrame({ kind: "request", id: 30, method: "noparams" })],
      (server) => server.onRequest("noparams", (_id, params) => {
        receivedParams = params;
        return null;
      }),
    );
    expect(receivedParams).toBeUndefined();
  });
});

// ===========================================================================
// 6. RpcClient
// ===========================================================================

describe("RpcClient", () => {
  // Helper: create a client pointing at a MockFramer that has pre-loaded responses.
  function makeClient(responseFrames: Array<Uint8Array | null>): {
    client: RpcClient<unknown>;
    framer: MockFramer;
  } {
    const framer = new MockFramer(responseFrames);
    const client = new RpcClient(new MockCodec(), framer);
    return { client, framer };
  }

  // -------------------------------------------------------------------------
  // 6a. request() encodes message and returns decoded result
  // -------------------------------------------------------------------------

  it("request() encodes a Request frame and returns the result on success", () => {
    // Pre-load the framer with the response the "server" would send.
    const { client, framer } = makeClient([
      encodeFrame({ kind: "response", id: 1, result: "pong" }),
    ]);

    const result = client.request("ping");

    // The result should be what the mock server returned.
    expect(result).toBe("pong");

    // The client should have written one Request frame.
    expect(framer.writtenFrames).toHaveLength(1);
    const sentMsg = decodeFrame(framer.writtenFrames[0]!);
    expect(sentMsg.kind).toBe("request");
    if (sentMsg.kind === "request") {
      expect(sentMsg.method).toBe("ping");
      expect(sentMsg.id).toBe(1);
    }
  });

  it("request() passes params in the encoded frame", () => {
    const { client, framer } = makeClient([
      encodeFrame({ kind: "response", id: 1, result: 7 }),
    ]);

    client.request("add", { a: 3, b: 4 });

    const sent = decodeFrame(framer.writtenFrames[0]!);
    expect(sent.kind).toBe("request");
    if (sent.kind === "request") expect(sent.params).toEqual({ a: 3, b: 4 });
  });

  // -------------------------------------------------------------------------
  // 6b. request() throws RpcClientError on server error response
  // -------------------------------------------------------------------------

  it("request() throws RpcClientError when server sends an error response", () => {
    const { client } = makeClient([
      encodeFrame({ kind: "error", id: 1, code: -32601, message: "Method not found" }),
    ]);

    expect(() => client.request("missing")).toThrow(RpcClientError);
  });

  it("RpcClientError has correct code and message", () => {
    const { client } = makeClient([
      encodeFrame({ kind: "error", id: 1, code: -32602, message: "Invalid params", data: "field x" }),
    ]);

    try {
      client.request("bad");
    } catch (e) {
      expect(e).toBeInstanceOf(RpcClientError);
      if (e instanceof RpcClientError) {
        expect(e.code).toBe(-32602);
        expect(e.message).toBe("Invalid params");
        expect(e.data).toBe("field x");
      }
    }
  });

  // -------------------------------------------------------------------------
  // 6c. request() throws RpcClientError on EOF before response
  // -------------------------------------------------------------------------

  it("request() throws RpcClientError when connection closes before response", () => {
    // No response frames at all — EOF immediately.
    const { client } = makeClient([]);

    expect(() => client.request("ping")).toThrow(RpcClientError);
  });

  it("EOF error has InternalError code", () => {
    const { client } = makeClient([]);

    try {
      client.request("ping");
    } catch (e) {
      expect(e).toBeInstanceOf(RpcClientError);
      if (e instanceof RpcClientError) {
        expect(e.code).toBe(RpcErrorCodes.InternalError);
      }
    }
  });

  // -------------------------------------------------------------------------
  // 6d. notify() encodes and writes without waiting for response
  // -------------------------------------------------------------------------

  it("notify() writes a Notification frame and does not block for a response", () => {
    // No frames needed — notify() should not read anything.
    const { client, framer } = makeClient([]);

    client.notify("ping");

    expect(framer.writtenFrames).toHaveLength(1);
    const sent = decodeFrame(framer.writtenFrames[0]!);
    expect(sent.kind).toBe("notification");
    if (sent.kind === "notification") expect(sent.method).toBe("ping");
  });

  it("notify() includes params in the frame", () => {
    const { client, framer } = makeClient([]);

    client.notify("log", { level: "info", msg: "hello" });

    const sent = decodeFrame(framer.writtenFrames[0]!);
    expect(sent.kind).toBe("notification");
    if (sent.kind === "notification") {
      expect(sent.params).toEqual({ level: "info", msg: "hello" });
    }
  });

  // -------------------------------------------------------------------------
  // 6e. onNotification() handler called for server-push notifications
  // -------------------------------------------------------------------------

  it("onNotification() handler is called for server-push notifications during request()", () => {
    const notifSpy = vi.fn();

    // The "server" sends a notification then the response.
    const { client } = makeClient([
      encodeFrame({ kind: "notification", method: "event", params: { x: 1 } }),
      encodeFrame({ kind: "response", id: 1, result: "done" }),
    ]);

    client.onNotification("event", notifSpy);
    const result = client.request("wait");

    expect(result).toBe("done");
    expect(notifSpy).toHaveBeenCalledOnce();
    expect(notifSpy).toHaveBeenCalledWith({ x: 1 });
  });

  it("multiple server-push notifications are dispatched before response", () => {
    const spy = vi.fn();

    const { client } = makeClient([
      encodeFrame({ kind: "notification", method: "progress", params: { pct: 25 } }),
      encodeFrame({ kind: "notification", method: "progress", params: { pct: 50 } }),
      encodeFrame({ kind: "notification", method: "progress", params: { pct: 75 } }),
      encodeFrame({ kind: "response", id: 1, result: "complete" }),
    ]);

    client.onNotification("progress", spy);
    client.request("longOp");

    expect(spy).toHaveBeenCalledTimes(3);
  });

  // -------------------------------------------------------------------------
  // 6f. Request ids are auto-generated and monotonically increasing
  // -------------------------------------------------------------------------

  it("request ids start at 1 and increment for each call", () => {
    // Two requests: we need two responses, each matching the expected id.
    const { client, framer } = makeClient([
      encodeFrame({ kind: "response", id: 1, result: "first" }),
      encodeFrame({ kind: "response", id: 2, result: "second" }),
    ]);

    client.request("a");
    client.request("b");

    const sentFrames = framer.writtenFrames.map((f) => decodeFrame(f));
    expect(sentFrames).toHaveLength(2);

    const [req1, req2] = sentFrames;
    expect(req1!.kind).toBe("request");
    expect(req2!.kind).toBe("request");
    if (req1!.kind === "request" && req2!.kind === "request") {
      expect(req1!.id).toBe(1);
      expect(req2!.id).toBe(2);
    }
  });

  // -------------------------------------------------------------------------
  // 6g. Unknown server-push notification is silently dropped
  // -------------------------------------------------------------------------

  it("unknown server-push notification is silently dropped", () => {
    const { client } = makeClient([
      encodeFrame({ kind: "notification", method: "unknown/event" }),
      encodeFrame({ kind: "response", id: 1, result: "ok" }),
    ]);

    // No handler registered for "unknown/event".
    // The unknown notification should be silently skipped and the response
    // for our request should still be returned correctly.
    let result: unknown;
    expect(() => {
      result = client.request("ping");
    }).not.toThrow();
    expect(result).toBe("ok");
  });

  it("onNotification is chainable", () => {
    const { client } = makeClient([]);
    const result = client
      .onNotification("a", () => {})
      .onNotification("b", () => {});
    expect(result).toBe(client);
  });

  it("notify() without params sends undefined params", () => {
    const { client, framer } = makeClient([]);
    client.notify("fire");
    const sent = decodeFrame(framer.writtenFrames[0]!);
    expect(sent.kind).toBe("notification");
    if (sent.kind === "notification") {
      expect(sent.method).toBe("fire");
      // params is omitted / undefined
      expect(sent.params).toBeUndefined();
    }
  });
});

// ===========================================================================
// 7. RpcClientError
// ===========================================================================

describe("RpcClientError", () => {
  it("is an Error subclass", () => {
    const err = new RpcClientError(-32601, "not found");
    expect(err).toBeInstanceOf(Error);
    expect(err).toBeInstanceOf(RpcClientError);
    expect(err.name).toBe("RpcClientError");
    expect(err.code).toBe(-32601);
    expect(err.message).toBe("not found");
  });

  it("stores optional data field", () => {
    const err = new RpcClientError(-32602, "bad params", { field: "x" });
    expect(err.data).toEqual({ field: "x" });
  });

  it("data is undefined when not provided", () => {
    const err = new RpcClientError(-32603, "internal");
    expect(err.data).toBeUndefined();
  });
});
