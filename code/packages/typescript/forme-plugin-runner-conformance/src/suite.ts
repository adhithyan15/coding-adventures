import { Buffer } from "node:buffer";
import {
  RunnerConformanceError,
  RunnerRemoteError,
  RunnerSession,
  type RunnerCommand,
} from "./driver.js";
import { CONFORMANCE_STAGE, CONFORMANCE_VECTORS, type ConformanceVector } from "./vectors.js";

export interface ConformanceReport {
  readonly runner: string;
  readonly runnerVersion: string;
  readonly vectors: readonly ConformanceVector[];
}

export async function runRunnerConformance(command: RunnerCommand): Promise<ConformanceReport> {
  try {
    await lifecycleAndValues(command);
    await capabilityMediation(command);
    await errorsAndCancellation(command);
    await streamShape(command, "stream", "Stream<ContentNode>", "Stream<ContentNode>");
    await streamShape(command, "stream-single", "Stream<ContentNode>", "ContentNode");
    await singleStream(command);
    await identityMismatch(command);
    await malformedResponseEnvelopes(command);
    await malformedPeer(command);
    await resourceBounds(command);
    return { runner: command.expectedRunner, runnerVersion: command.expectedRunnerVersion, vectors: CONFORMANCE_VECTORS };
  } catch (error) {
    if (error instanceof RunnerConformanceError) throw error;
    throw new RunnerConformanceError("runner failed the canonical conformance corpus", error);
  }
}

async function lifecycleAndValues(command: RunnerCommand): Promise<void> {
  const session = new RunnerSession(command, "single");
  try {
    await ready(session, command, "ContentNode", "ContentNode");
    const value = {
      bytes: new Uint8Array([0, 1, 2, 255]),
      collision: { $forme: "bytes", base64: "not-an-envelope" },
      nested: [null, true, 7, "text"],
    };
    equal(await session.request("stage.run", { input: { operation: "echo", value }, config: {}, streamId: 10 }), {
      kind: "single", value,
    }, "wire value round-trip");
    await dispose(session);
  } finally {
    await session.terminate();
  }
}

async function capabilityMediation(command: RunnerCommand): Promise<void> {
  const session = new RunnerSession(command, "single");
  const calls: Array<{ method: string; params: Readonly<Record<string, unknown>> }> = [];
  const watchStreamId = 701;
  session.onRequest(async (method, params) => {
    calls.push({ method, params });
    if (!Number.isSafeInteger(params.streamId) || Number(params.streamId) <= 0) throw new Error("missing active stream id");
    if (method === "ctx.env.get" && params.name === "DENIED") {
      equal({ method, params }, call("ctx.env.get", { name: "DENIED" }, 12), "capability denial request");
      throw new RunnerRemoteError(-32001, "capability denied", { capability: "env:DENIED" });
    }
    const replies: Record<string, unknown> = {
      "ctx.storage.read": { bytes: Buffer.from("hello").toString("base64") },
      "ctx.storage.write": null,
      "ctx.storage.exists": true,
      "ctx.storage.stat": { size: 5, mtimeMs: 0, type: "file" },
      "ctx.storage.list": [{ path: "posts/a.md", type: "file" }],
      "ctx.storage.watch": { kind: "stream-handle", streamId: watchStreamId },
      "ctx.storage.remove": null,
      "ctx.env.get": "allowed",
      "ctx.time.nowMs": 42,
      "ctx.time.nowIso": "1970-01-01T00:00:00.042Z",
      "ctx.filesystem.homeDir": "/home/test",
      "ctx.filesystem.tempDir": "/tmp/test",
      "ctx.filesystem.readAbsolute": { bytes: Buffer.from("absolute").toString("base64") },
      "ctx.filesystem.writeAbsolute": null,
      "ctx.shell.run": {
        exitCode: 0,
        stdout: Buffer.from("stdout").toString("base64"),
        stderr: Buffer.from("stderr").toString("base64"),
      },
      "ctx.network.fetch": {
        status: 201, statusText: "Created", headers: { "x-test": "yes" },
        bytes: Buffer.from("network").toString("base64"), url: "https://example.com/data",
      },
    };
    if (!Object.hasOwn(replies, method)) throw new Error(`unexpected capability method: ${method}`);
    return replies[method];
  });
  try {
    await ready(session, command, "ContentNode", "ContentNode");
    const run = session.request("stage.run", { input: { operation: "context" }, config: {}, streamId: 11 });
    await waitForNotification(session, "stream.start", { streamId: watchStreamId });
    session.notify("stream.value", {
      streamId: watchStreamId,
      value: { path: "posts/a.md", kind: "modified" },
    });
    const result = await run;
    match(result, {
      kind: "single",
      value: {
        bytes: new Uint8Array(Buffer.from("hello")), bounded: new Uint8Array(Buffer.from("hello")),
        exists: true, env: "allowed", envRequired: "allowed", nowMs: 42, home: "/home/test", status: 201,
        body: new Uint8Array(Buffer.from("network")),
        watched: [{ path: "posts/a.md", kind: "modified" }],
        shell: { exitCode: 0, stdout: new Uint8Array(Buffer.from("stdout")), stderr: new Uint8Array(Buffer.from("stderr")) },
      },
    }, "mediated capability result");
    const expected = [
      call("ctx.storage.read", { path: "posts/a.md" }),
      call("ctx.storage.write", { path: "out/a.md", bytes: "aGVsbG8=" }),
      call("ctx.storage.list", { path: "posts" }),
      call("ctx.storage.watch", { path: "posts" }),
      call("ctx.storage.remove", { path: "out/stale.md" }),
      call("ctx.network.fetch", {
        url: "https://example.com/data",
        init: { method: "POST", headers: { "x-test": "yes" }, body: "AQID" },
      }),
      call("ctx.filesystem.writeAbsolute", { path: "/safe/out", bytes: "aGVsbG8=" }),
      call("ctx.shell.run", { command: "tool", args: ["arg"], options: { stdin: "aGVsbG8=" } }),
      call("ctx.storage.read", { path: "posts/a.md" }),
      call("ctx.storage.exists", { path: "posts/a.md" }),
      call("ctx.storage.stat", { path: "posts/a.md" }),
      call("ctx.env.get", { name: "ALLOWED" }),
      call("ctx.env.get", { name: "ALLOWED" }),
      call("ctx.time.nowMs", {}),
      call("ctx.time.nowIso", {}),
      call("ctx.filesystem.homeDir", {}),
      call("ctx.filesystem.tempDir", {}),
      call("ctx.filesystem.readAbsolute", { path: "/safe/a" }),
      call("ctx.filesystem.readAbsolute", { path: "/safe/a" }),
    ];
    equal(calls, expected, "mediated capability call order");
    equal(session.notifications.filter(entry => entry.method === "log"), [{
      jsonrpc: "2.0", method: "log",
      params: { level: "info", message: "runner fixture", fields: { ok: true } },
    }], "wire logger notification");
    equal(session.notifications.filter(entry => entry.method === "stream.start" || entry.method === "stream.cancel"), [
      { jsonrpc: "2.0", method: "stream.start", params: { streamId: watchStreamId } },
      { jsonrpc: "2.0", method: "stream.cancel", params: { streamId: watchStreamId } },
    ], "storage watch lifecycle notifications");
    await expectCapabilityDenied(session.request("stage.run", {
      input: { operation: "capabilityDenied" }, config: {}, streamId: 12,
    }));
    await dispose(session);
  } finally {
    await session.terminate();
  }
}

async function waitForNotification(
  session: RunnerSession,
  method: string,
  params: Readonly<Record<string, unknown>>,
): Promise<void> {
  for (let attempt = 0; attempt < 3_000; attempt += 1) {
    if (session.notifications.some(entry => entry.method === method
        && JSON.stringify(entry.params) === JSON.stringify(params))) return;
    await new Promise(resolve => setTimeout(resolve, 1));
  }
  throw new RunnerConformanceError(`runner did not emit ${method}`);
}

async function identityMismatch(command: RunnerCommand): Promise<void> {
  const mismatches: ReadonlyArray<Readonly<Record<string, unknown>>> = [
    { pluginName: "@forme/wrong" },
    { pluginVersion: "9.9.9" },
    { apiVersion: CONFORMANCE_STAGE.apiVersion + 1 },
    { protocolVersion: CONFORMANCE_STAGE.protocolVersion + 1 },
  ];
  for (const mismatch of mismatches) {
    const session = new RunnerSession(command, "single");
    try {
      await expectRemote(session.request("handshake", {
        hostName: "forme-plugin-runner-conformance", hostVersion: "0.1.0",
        apiVersion: CONFORMANCE_STAGE.apiVersion, protocolVersion: CONFORMANCE_STAGE.protocolVersion,
        pluginName: CONFORMANCE_STAGE.pluginName, pluginVersion: CONFORMANCE_STAGE.pluginVersion,
        manifestHash: "sha256:conformance-manifest", instanceId: "conformance-1",
        trustTier: "unverified-third-party", ...mismatch,
      }), -32006, "manifest identity mismatch");
    } finally {
      await session.terminate();
    }
  }
}

async function malformedResponseEnvelopes(command: RunnerCommand): Promise<void> {
  for (const resultJson of [
    '{"$forme":"bytes","base64":"!"}',
    '{"$forme":"escaped-object","entries":[["x",1],["x",2]]}',
  ]) {
    const session = new RunnerSession(command, "single");
    session.onRequest(async (method, params, id) => {
      equal({ method, params }, call("ctx.env.get", { name: "MALFORMED" }, 13), "malformed response request");
      session.writeRaw(rawJsonFrame(`{"jsonrpc":"2.0","id":${id},"result":${resultJson}}`));
      return new Promise<never>(() => undefined);
    });
    try {
      await ready(session, command, "ContentNode", "ContentNode");
      await expectFailure(session.request("stage.run", {
        input: { operation: "malformedResponse" }, config: {}, streamId: 13,
      }), "runner accepted a malformed capability response envelope");
      const exit = await session.waitForExit();
      if (exit.code === 0 && exit.signal === null) {
        throw new RunnerConformanceError("runner accepted a malformed capability response envelope");
      }
    } finally {
      await session.terminate();
    }
  }
}

async function errorsAndCancellation(command: RunnerCommand): Promise<void> {
  const session = new RunnerSession(command, "single");
  try {
    await ready(session, command, "ContentNode", "ContentNode");
    await expectRemote(session.request("stage.run", { input: { operation: "error" }, config: {}, streamId: 20 }), -32900, "typed StageError");
    const pending = session.beginRequest("stage.run", { input: { operation: "waitForCancel" }, config: {}, streamId: 21 });
    await new Promise(resolve => setTimeout(resolve, 20));
    session.notify("$/cancelRequest", { id: pending.id, reason: "conformance cancellation" });
    await expectRemote(pending.result, -32800, "cooperative cancellation");
    await dispose(session);
  } finally {
    await session.terminate();
  }
}

async function streamShape(command: RunnerCommand, mode: string, consumes: string, produces: string): Promise<void> {
  const session = new RunnerSession(command, mode);
  try {
    await ready(session, command, consumes, produces);
    const run = session.request("stage.run", {
      input: { kind: "stream-handle", streamId: 30 }, config: {}, streamId: 31,
    });
    session.notify("stream.value", { streamId: 30, value: { n: 1 } });
    session.notify("stream.value", { streamId: 30, value: { n: 2 } });
    session.notify("stream.end", { streamId: 30 });
    const result = await run;
    if (mode === "stream") {
      equal(result, { kind: "stream", streamId: 31, produced: 2 }, "stream-stream result");
      const values = session.notifications.filter(entry => entry.method === "stream.value").map(entry => entry.params);
      equal(values, [{ streamId: 31, value: { n: 1 } }, { streamId: 31, value: { n: 2 } }], "stream-stream values");
    } else {
      equal(result, { kind: "single", value: { values: [{ n: 1 }, { n: 2 }] } }, "stream-single result");
    }
    await dispose(session);
  } finally {
    await session.terminate();
  }
}

async function singleStream(command: RunnerCommand): Promise<void> {
  const session = new RunnerSession(command, "single-stream");
  try {
    await ready(session, command, "ContentNode", "Stream<ContentNode>");
    equal(await session.request("stage.run", { input: { n: 1 }, config: {}, streamId: 41 }), {
      kind: "stream", streamId: 41, produced: 2,
    }, "single-stream result");
    const values = session.notifications.filter(entry => entry.method === "stream.value").map(entry => entry.params);
    equal(values, [{ streamId: 41, value: { n: 1 } }, { streamId: 41, value: { copy: { n: 1 } } }], "single-stream values");
    await dispose(session);
  } finally {
    await session.terminate();
  }
}

async function malformedPeer(command: RunnerCommand): Promise<void> {
  const session = new RunnerSession(command, "single");
  try {
    session.writeRaw(Buffer.from("Content-Length: 01\r\n\r\n{}", "ascii"));
    const exit = await session.waitForExit();
    if (exit.code === 0 && exit.signal === null) throw new RunnerConformanceError("runner accepted a non-canonical frame header");
  } finally {
    await session.terminate();
  }
}

async function resourceBounds(command: RunnerCommand): Promise<void> {
  const session = new RunnerSession(command, "single");
  try {
    session.writeRaw(Buffer.from("Content-Length: 4097\r\n\r\n", "ascii"));
    const exit = await session.waitForExit();
    if (exit.code === 0 && exit.signal === null) throw new RunnerConformanceError("runner accepted a frame above the canonical bound");
  } finally {
    await session.terminate();
  }
}

async function ready(
  session: RunnerSession,
  command: RunnerCommand,
  consumes: string,
  produces: string,
): Promise<void> {
  equal(await session.request("handshake", {
    hostName: "forme-plugin-runner-conformance", hostVersion: "0.1.0",
    apiVersion: CONFORMANCE_STAGE.apiVersion, protocolVersion: CONFORMANCE_STAGE.protocolVersion,
    pluginName: CONFORMANCE_STAGE.pluginName, pluginVersion: CONFORMANCE_STAGE.pluginVersion,
    manifestHash: "sha256:conformance-manifest", instanceId: "conformance-1",
    trustTier: "unverified-third-party",
  }), {
    pluginName: CONFORMANCE_STAGE.pluginName,
    pluginVersion: CONFORMANCE_STAGE.pluginVersion,
    apiVersion: CONFORMANCE_STAGE.apiVersion,
    protocolVersion: CONFORMANCE_STAGE.protocolVersion,
    runner: command.expectedRunner,
    runnerVersion: command.expectedRunnerVersion,
  }, "handshake");
  equal(await session.request("announce", {}), {
    stage: {
      id: CONFORMANCE_STAGE.id,
      consumes,
      produces,
      capabilities: CONFORMANCE_STAGE.capabilities,
      configSchemaHash: CONFORMANCE_STAGE.configSchemaHash,
    },
  }, "announcement");
  equal(await session.request("stage.init", { config: {}, instanceId: "conformance-1", logLevel: "info" }), null, "initialization");
}

async function dispose(session: RunnerSession): Promise<void> {
  equal(await session.request("stage.dispose", {}), null, "disposal");
  const exit = await session.waitForExit();
  if (exit.code !== 0 || exit.signal !== null) throw new RunnerConformanceError(`runner exited unsuccessfully after disposal: ${exit.stderr}`);
}

async function expectRemote(result: Promise<unknown>, code: number, label: string): Promise<void> {
  try {
    await result;
  } catch (error) {
    if (error instanceof RunnerRemoteError && error.rpcCode === code) return;
    throw new RunnerConformanceError(`${label} returned the wrong error`, error);
  }
  throw new RunnerConformanceError(`${label} unexpectedly succeeded`);
}

async function expectCapabilityDenied(result: Promise<unknown>): Promise<void> {
  try {
    await result;
  } catch (error) {
    if (error instanceof RunnerRemoteError && error.rpcCode === -32900) {
      const data = isObject(error.data) ? error.data : {};
      if (data.stageErrorCode === "CAPABILITY_DENIED") return;
    }
    throw new RunnerConformanceError("capability denial was not translated to a typed stage error", error);
  }
  throw new RunnerConformanceError("denied capability unexpectedly succeeded");
}

async function expectFailure(result: Promise<unknown>, message: string): Promise<void> {
  try { await result; }
  catch { return; }
  throw new RunnerConformanceError(message);
}

function call(method: string, params: Readonly<Record<string, unknown>>, streamId = 11): {
  method: string; params: Readonly<Record<string, unknown>>;
} {
  return { method, params: { ...params, streamId } };
}

function rawJsonFrame(json: string): Buffer {
  const payload = Buffer.from(json, "utf8");
  return Buffer.concat([Buffer.from(`Content-Length: ${payload.byteLength}\r\n\r\n`, "ascii"), payload]);
}

function equal(actual: unknown, expected: unknown, label: string): void {
  if (!deepEqual(actual, expected)) {
    throw new RunnerConformanceError(`${label} mismatch: expected ${render(expected)}, received ${render(actual)}`);
  }
}

function match(actual: unknown, expected: unknown, label: string): void {
  if (!isPartialMatch(actual, expected)) throw new RunnerConformanceError(`${label} mismatch`);
}

function deepEqual(left: unknown, right: unknown): boolean {
  if (left instanceof Uint8Array && right instanceof Uint8Array) return Buffer.from(left).equals(Buffer.from(right));
  if (Object.is(left, right)) return true;
  if (Array.isArray(left) && Array.isArray(right)) return left.length === right.length && left.every((value, index) => deepEqual(value, right[index]));
  if (isObject(left) && isObject(right)) {
    const leftKeys = Object.keys(left);
    const rightKeys = Object.keys(right);
    return leftKeys.length === rightKeys.length && leftKeys.every(key => Object.hasOwn(right, key) && deepEqual(left[key], right[key]));
  }
  return false;
}

function isPartialMatch(actual: unknown, expected: unknown): boolean {
  if (expected instanceof Uint8Array) return actual instanceof Uint8Array && deepEqual(actual, expected);
  if (Array.isArray(expected)) return Array.isArray(actual) && deepEqual(actual, expected);
  if (isObject(expected)) return isObject(actual) && Object.keys(expected).every(key => isPartialMatch(actual[key], expected[key]));
  return Object.is(actual, expected);
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value) && !(value instanceof Uint8Array);
}

function render(value: unknown): string {
  return JSON.stringify(value, (_key, entry) => entry instanceof Uint8Array ? Array.from(entry) : entry);
}
