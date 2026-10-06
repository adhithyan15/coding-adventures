import { Buffer } from "node:buffer";
import { CapabilityError, CancellationError, StageError } from "@coding-adventures/forme-errors";
import {
  inMemoryCache,
  inMemoryEventBus,
  noOpTelemetryEmitter,
  type CancellationToken,
  type EnvApi,
  type FilesystemApi,
  type Logger,
  type NetworkApi,
  type ShellApi,
  type StageContext,
  type StorageApi,
} from "@coding-adventures/forme-stage";
import type { JsonValue } from "@coding-adventures/forme-types";
import { RunnerRpcPeer, RunnerRpcRemoteError, asRecord } from "./wire.js";

const MAX_MEDIATED_BYTES = 1024 * 1024;

export function buildWireContext(
  peer: RunnerRpcPeer,
  streamId: number,
  cancellation: CancellationToken,
  openStream: OpenStream = unavailableStream,
): StageContext {
  const request = async (method: string, params: Record<string, unknown> = {}): Promise<unknown> => {
    cancellation.throwIfCancelled();
    try {
      const result = await peer.request(method, { ...params, streamId });
      cancellation.throwIfCancelled();
      return result;
    } catch (error) {
      throw translateRemote(error);
    }
  };
  return {
    logger: wireLogger(peer),
    cancellation,
    time: {
      async nowMs() { return numberResult(await request("ctx.time.nowMs"), "time.nowMs"); },
      async nowIso() { return stringResult(await request("ctx.time.nowIso"), "time.nowIso"); },
      monotonicMs() { return performance.now(); },
    },
    cache: inMemoryCache(),
    telemetry: noOpTelemetryEmitter(),
    storage: storageApi(request, openStream),
    network: networkApi(request),
    env: envApi(request),
    filesystem: filesystemApi(request),
    shell: shellApi(request),
    events: inMemoryEventBus(),
  };
}

export function wireLogger(peer: RunnerRpcPeer, base: Record<string, JsonValue> = {}): Logger {
  const emit = (level: string, message: string, fields?: Record<string, JsonValue>) => {
    void peer.notify("log", { level, message, fields: { ...base, ...(fields ?? {}) } }).catch(() => undefined);
  };
  return {
    trace: (message, fields) => emit("trace", message, fields),
    debug: (message, fields) => emit("debug", message, fields),
    info: (message, fields) => emit("info", message, fields),
    warn: (message, fields) => emit("warn", message, fields),
    error: (message, fields) => emit("error", message, fields),
    child(fields) { return wireLogger(peer, { ...base, ...fields }); },
  };
}

function storageApi(request: Request, openStream: OpenStream): StorageApi {
  return {
    async read(path) { return bytesResult(await request("ctx.storage.read", { path }), "storage.read"); },
    async readBounded(path, maxBytes) {
      assertBound(maxBytes);
      const bytes = bytesResult(await request("ctx.storage.read", { path }), "storage.readBounded");
      if (bytes.byteLength > maxBytes) throw new StageError({ code: "RESOURCE_LIMIT_EXCEEDED", message: "storage value exceeds requested bound" });
      return bytes;
    },
    async write(path, bytes) {
      assertBytes(bytes);
      await request("ctx.storage.write", { path, bytes: Buffer.from(bytes).toString("base64") });
    },
    async exists(path) { return booleanResult(await request("ctx.storage.exists", { path }), "storage.exists"); },
    async *list(path) {
      const entries = await request("ctx.storage.list", { path });
      if (!Array.isArray(entries)) throw protocol("storage.list result");
      for (const entry of entries) yield entry as never;
    },
    async *watch(path) {
      const handle = await request("ctx.storage.watch", { path });
      for await (const entry of openStream(handle)) yield entry as never;
    },
    async remove(path) { await request("ctx.storage.remove", { path }); },
    async stat(path) { return asRecord(await request("ctx.storage.stat", { path }), "storage.stat result") as never; },
  };
}

type OpenStream = (handle: unknown) => AsyncIterable<unknown>;

async function* unavailableStream(): AsyncIterable<never> {
  throw protocol("storage.watch lifecycle");
}

function envApi(request: Request): EnvApi {
  return {
    async get(name) {
      const value = await request("ctx.env.get", { name });
      if (value === null) return undefined;
      return stringResult(value, "env.get");
    },
    async getOrThrow(name) {
      const value = await this.get(name);
      if (value === undefined) throw new StageError({ code: "ENV_MISSING", message: `Environment variable ${name} is not set` });
      return value;
    },
  };
}

function filesystemApi(request: Request): FilesystemApi {
  return {
    async readAbsolute(path) { return bytesResult(await request("ctx.filesystem.readAbsolute", { path }), "filesystem.readAbsolute"); },
    async readAbsoluteBounded(path, maxBytes) {
      assertBound(maxBytes);
      const bytes = bytesResult(await request("ctx.filesystem.readAbsolute", { path }), "filesystem.readAbsoluteBounded");
      if (bytes.byteLength > maxBytes) throw new StageError({ code: "RESOURCE_LIMIT_EXCEEDED", message: "filesystem value exceeds requested bound" });
      return bytes;
    },
    async writeAbsolute(path, bytes) {
      assertBytes(bytes);
      await request("ctx.filesystem.writeAbsolute", { path, bytes: Buffer.from(bytes).toString("base64") });
    },
    async homeDir() { return stringResult(await request("ctx.filesystem.homeDir"), "filesystem.homeDir"); },
    async tempDir() { return stringResult(await request("ctx.filesystem.tempDir"), "filesystem.tempDir"); },
  };
}

function networkApi(request: Request): NetworkApi {
  return {
    async fetch(input, init) {
      const url = typeof input === "string" ? input : input.url;
      const sourceInit = typeof input === "string" ? init : { method: input.method, headers: input.headers, ...init };
      const headers = sourceInit?.headers === undefined
        ? undefined
        : Object.fromEntries(new Headers(sourceInit.headers).entries());
      let body: string | undefined;
      if (sourceInit?.body !== undefined && sourceInit.body !== null) {
        if (typeof sourceInit.body === "string") body = Buffer.from(sourceInit.body).toString("base64");
        else if (sourceInit.body instanceof Uint8Array) body = Buffer.from(sourceInit.body).toString("base64");
        else throw new StageError({ code: "INVALID_NETWORK_BODY", message: "plugin fetch body must be a string or Uint8Array" });
      }
      const result = asRecord(await request("ctx.network.fetch", {
        url,
        init: { method: sourceInit?.method, headers, body },
      }), "network.fetch result");
      const bytes = decodeBase64(result.bytes, "network.fetch bytes");
      const response = new Response(bytes as unknown as BodyInit, {
        status: numberResult(result.status, "network.fetch status"),
        statusText: typeof result.statusText === "string" ? result.statusText : "",
        headers: isStringRecord(result.headers) ? result.headers : {},
      });
      if (typeof result.url === "string") Object.defineProperty(response, "url", { value: result.url });
      return response;
    },
  };
}

function shellApi(request: Request): ShellApi {
  return {
    async run(command, args, options) {
      const result = asRecord(await request("ctx.shell.run", {
        command, args, options: options ? {
          ...options,
          stdin: options.stdin ? Buffer.from(options.stdin).toString("base64") : undefined,
        } : undefined,
      }), "shell.run result");
      return {
        exitCode: numberResult(result.exitCode, "shell.run exitCode"),
        stdout: decodeBase64(result.stdout, "shell.run stdout"),
        stderr: decodeBase64(result.stderr, "shell.run stderr"),
      };
    },
  };
}

type Request = (method: string, params?: Record<string, unknown>) => Promise<unknown>;

function bytesResult(value: unknown, label: string): Uint8Array {
  return decodeBase64(asRecord(value, `${label} result`).bytes, `${label} bytes`);
}

function decodeBase64(value: unknown, label: string): Uint8Array {
  if (typeof value !== "string" || !isCanonicalBase64(value)) throw protocol(label);
  const bytes = Uint8Array.from(Buffer.from(value, "base64"));
  assertBytes(bytes);
  return bytes;
}

function isCanonicalBase64(value: string): boolean {
  return value.length % 4 === 0
    && /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(value)
    && Buffer.from(value, "base64").toString("base64") === value;
}

function translateRemote(error: unknown): Error {
  if (!(error instanceof RunnerRpcRemoteError)) return error instanceof Error ? error : new Error(String(error));
  const data = typeof error.data === "object" && error.data !== null ? error.data as Record<string, unknown> : {};
  if (error.rpcCode === -32800) return new CancellationError(error.message);
  if (error.rpcCode === -32001) return new CapabilityError({
    message: error.message,
    capability: typeof data.capability === "string" ? data.capability : "unknown",
  });
  return new StageError({ code: "PLUGIN_PROTOCOL_ERROR", message: error.message, fields: { rpcCode: error.rpcCode } });
}

function assertBound(value: number): void {
  if (!Number.isSafeInteger(value) || value < 0 || value > MAX_MEDIATED_BYTES) throw new RangeError("maxBytes must be a safe integer within the mediated byte limit");
}

function assertBytes(value: Uint8Array): void {
  if (value.byteLength > MAX_MEDIATED_BYTES) throw new StageError({ code: "RESOURCE_LIMIT_EXCEEDED", message: "mediated bytes exceed the configured bound" });
}

function stringResult(value: unknown, label: string): string {
  if (typeof value !== "string") throw protocol(label);
  return value;
}

function numberResult(value: unknown, label: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) throw protocol(label);
  return value;
}

function booleanResult(value: unknown, label: string): boolean {
  if (typeof value !== "boolean") throw protocol(label);
  return value;
}

function isStringRecord(value: unknown): value is Record<string, string> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    && Object.values(value).every(entry => typeof entry === "string");
}

function protocol(label: string): StageError {
  return new StageError({ code: "PLUGIN_PROTOCOL_ERROR", message: `${label} is malformed` });
}
