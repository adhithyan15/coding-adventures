import { isAbsolute } from "node:path";
import { matchesCapability, type Capability } from "@coding-adventures/forme-capability";
import type { StageContext } from "@coding-adventures/forme-stage";
import { RpcFault } from "./errors.js";
import { asRecord } from "./wire.js";

const CAPABILITY_DENIED = -32001;
const INVALID_PATH = -32002;
const INVALID_PARAMS = -32602;
const METHOD_NOT_FOUND = -32601;
const MAX_MEDIATED_ITEMS = 4_096;
const MAX_NETWORK_BODY_BYTES = 1024 * 1024;
const MAX_MEDIATED_BYTES = 1024 * 1024;
const MAX_NETWORK_REDIRECTS = 10;

export async function mediateCapabilityRequest(
  method: string,
  rawParams: unknown,
  context: StageContext | null,
  grants: readonly Capability[],
): Promise<unknown> {
  if (!context) deny("lifecycle:run", "capability calls are allowed only during stage.run");
  const params = asRecord(rawParams ?? {}, `${method} params`);
  switch (method) {
    case "ctx.storage.read": {
      authorize(grants, "storage:read");
      const path = storagePath(params.path);
      const stat = await context.storage.stat(path);
      if (stat.type !== "file" || stat.size > MAX_MEDIATED_BYTES) resourceLimit("storage-read-bytes");
      context.cancellation.throwIfCancelled();
      const bytes = boundedBytes(
        await context.storage.readBounded(path, MAX_MEDIATED_BYTES),
        "storage-read-bytes",
      );
      context.cancellation.throwIfCancelled();
      return { bytes: Buffer.from(bytes).toString("base64") };
    }
    case "ctx.storage.write": {
      authorize(grants, "storage:write");
      const path = storagePath(params.path);
      context.cancellation.throwIfCancelled();
      await context.storage.write(path, decodeBytes(params.bytes, "storage-write-bytes"));
      context.cancellation.throwIfCancelled();
      return null;
    }
    case "ctx.storage.exists":
      authorize(grants, "storage:read");
      return context.storage.exists(storagePath(params.path));
    case "ctx.storage.stat":
      authorize(grants, "storage:read");
      return context.storage.stat(storagePath(params.path));
    case "ctx.storage.list": {
      authorize(grants, "storage:read");
      return collectBounded(context.storage.list(storagePath(params.path)));
    }
    case "ctx.storage.watch": {
      authorize(grants, "storage:read");
      storagePath(params.path);
      throw new RpcFault(METHOD_NOT_FOUND, "METHOD_NOT_FOUND", {
        method,
        deferredTo: "FM-B015",
      });
    }
    case "ctx.storage.remove":
      authorize(grants, "storage:write");
      await context.storage.remove(storagePath(params.path));
      return null;
    case "ctx.env.get": {
      const name = stringParam(params.name, "name");
      authorize(grants, `env:${name}`);
      return (await context.env.get(name)) ?? null;
    }
    case "ctx.filesystem.readAbsolute": {
      authorize(grants, "filesystem:user");
      const path = absolutePath(params.path);
      context.cancellation.throwIfCancelled();
      const bytes = boundedBytes(
        await context.filesystem.readAbsoluteBounded(path, MAX_MEDIATED_BYTES),
        "filesystem-read-bytes",
      );
      context.cancellation.throwIfCancelled();
      return { bytes: Buffer.from(bytes).toString("base64") };
    }
    case "ctx.filesystem.writeAbsolute": {
      authorize(grants, "filesystem:user");
      context.cancellation.throwIfCancelled();
      await context.filesystem.writeAbsolute(
        absolutePath(params.path),
        decodeBytes(params.bytes, "filesystem-write-bytes"),
      );
      context.cancellation.throwIfCancelled();
      return null;
    }
    case "ctx.filesystem.homeDir":
      authorize(grants, "filesystem:user");
      return context.filesystem.homeDir();
    case "ctx.filesystem.tempDir":
      authorize(grants, "filesystem:user");
      return context.filesystem.tempDir();
    case "ctx.network.fetch":
      return mediateFetch(context, grants, params);
    case "ctx.time.nowMs":
      authorize(grants, "system:time:wallclock");
      return context.time.nowMs();
    case "ctx.time.nowIso":
      authorize(grants, "system:time:wallclock");
      return context.time.nowIso();
    case "ctx.time.monotonicMs":
      return context.time.monotonicMs();
    case "ctx.shell.run":
      deny("system:shell", "third-party shell execution is forbidden");
      break;
    default:
      throw new RpcFault(METHOD_NOT_FOUND, "METHOD_NOT_FOUND", { method });
  }
}

async function mediateFetch(
  context: StageContext,
  grants: readonly Capability[],
  params: Record<string, unknown>,
): Promise<unknown> {
  const urlString = stringParam(params.url, "url");
  let url: URL;
  try {
    url = new URL(urlString);
  } catch {
    throw new RpcFault(INVALID_PARAMS, "INVALID_PARAMS", { field: "url" });
  }
  const initRecord = params.init === undefined ? {} : asRecord(params.init, "fetch init");
  const method = initRecord.method === undefined ? undefined : stringParam(initRecord.method, "method");
  const headersRecord = initRecord.headers === undefined ? undefined : asRecord(initRecord.headers, "headers");
  const headers = headersRecord
    ? Object.fromEntries(Object.entries(headersRecord).map(([key, value]) => [key, stringParam(value, key)]))
    : undefined;
  const body = initRecord.body === undefined ? undefined : decodeBytes(initRecord.body);
  let response: Response | null = null;
  for (let redirects = 0; redirects <= MAX_NETWORK_REDIRECTS; redirects += 1) {
    authorizeNetworkUrl(grants, url);
    response = await context.network.fetch(url.toString(), {
      method,
      headers,
      body: body as unknown as BodyInit | undefined,
      redirect: "manual",
      signal: context.cancellation.signal,
    });
    const finalUrl = response.url ? new URL(response.url) : url;
    try {
      authorizeNetworkUrl(grants, finalUrl);
    } catch (error) {
      await cancelResponseBody(response);
      throw error;
    }
    const location = response.headers.get("location");
    if (response.status < 300 || response.status >= 400 || !location) break;
    await cancelResponseBody(response);
    if (redirects === MAX_NETWORK_REDIRECTS) {
      throw new RpcFault(-32003, "RESOURCE_LIMIT_EXCEEDED", {
        resource: "network-redirects",
        limit: MAX_NETWORK_REDIRECTS,
      });
    }
    const nextUrl = new URL(location, url);
    authorizeNetworkUrl(grants, nextUrl);
    if (nextUrl.origin !== url.origin) {
      deny("network:redirect-cross-origin", "cross-origin redirects are not replayed by the plugin host");
    }
    const normalizedMethod = (method ?? "GET").toUpperCase();
    if (body !== undefined || (normalizedMethod !== "GET" && normalizedMethod !== "HEAD")) {
      deny("network:redirect-replay", "redirects with methods or bodies are not replayed by the plugin host");
    }
    url = nextUrl;
  }
  if (!response) {
    throw new RpcFault(-32603, "INTERNAL_ERROR");
  }
  const bytes = await readBoundedResponseBody(response, context.cancellation.signal);
  return {
    status: response.status,
    statusText: response.statusText,
    headers: Object.fromEntries(response.headers.entries()),
    bytes: Buffer.from(bytes).toString("base64"),
  };
}

async function cancelResponseBody(response: Response): Promise<void> {
  await response.body?.cancel().catch(() => undefined);
}

function authorizeNetworkUrl(grants: readonly Capability[], url: URL): void {
  const scheme = url.protocol.slice(0, -1);
  const allowed = grants.some(grant =>
    matchesCapability(grant, `network:${url.hostname}`)
    || matchesCapability(grant, `network:${scheme}:${url.hostname}`));
  if (!allowed) deny(`network:${scheme}:${url.hostname}`);
}

async function readBoundedResponseBody(response: Response, signal: AbortSignal): Promise<Uint8Array> {
  if (!response.body) return new Uint8Array();
  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let total = 0;
  try {
    while (true) {
      if (signal.aborted) throw new DOMException("Request cancelled", "AbortError");
      const { done, value } = await reader.read();
      if (done) break;
      total += value.byteLength;
      if (total > MAX_NETWORK_BODY_BYTES) {
        resourceLimit("network-response-bytes", MAX_NETWORK_BODY_BYTES);
      }
      chunks.push(value);
    }
  } catch (error) {
    await reader.cancel(error).catch(() => undefined);
    throw error;
  } finally {
    reader.releaseLock();
  }
  const result = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    result.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return result;
}

function authorize(grants: readonly Capability[], requested: Capability): void {
  if (!grants.some(grant => matchesCapability(grant, requested))) deny(requested);
}

function deny(capability: string, reason = "capability is not in plugin's granted set"): never {
  throw new RpcFault(CAPABILITY_DENIED, "CAPABILITY_DENIED", { capability, reason });
}

function storagePath(value: unknown): string {
  const path = stringParam(value, "path");
  if (path.length === 0 || path.includes("\0") || isAbsolute(path)
      || path.split(/[\\/]/).includes("..")) {
    throw new RpcFault(INVALID_PATH, "INVALID_PATH", { path });
  }
  return path;
}

function absolutePath(value: unknown): string {
  const path = stringParam(value, "path");
  if (!isAbsolute(path) || path.includes("\0")) {
    throw new RpcFault(INVALID_PATH, "INVALID_PATH", { path });
  }
  return path;
}

function stringParam(value: unknown, field: string): string {
  if (typeof value !== "string") {
    throw new RpcFault(INVALID_PARAMS, "INVALID_PARAMS", { field });
  }
  return value;
}

function decodeBytes(value: unknown, resource = "mediated-bytes"): Uint8Array {
  const encoded = stringParam(value, "bytes");
  if (!/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(encoded)) {
    throw new RpcFault(INVALID_PARAMS, "INVALID_PARAMS", { field: "bytes" });
  }
  if (encoded.length > Math.ceil(MAX_MEDIATED_BYTES / 3) * 4) resourceLimit(resource);
  return boundedBytes(Buffer.from(encoded, "base64"), resource);
}

function boundedBytes(bytes: Uint8Array, resource: string): Uint8Array {
  if (bytes.byteLength > MAX_MEDIATED_BYTES) resourceLimit(resource);
  return bytes;
}

function resourceLimit(resource: string, limit = MAX_MEDIATED_BYTES): never {
  throw new RpcFault(-32003, "RESOURCE_LIMIT_EXCEEDED", { resource, limit });
}

async function collectBounded<T>(values: AsyncIterable<T>): Promise<readonly T[]> {
  const result: T[] = [];
  for await (const value of values) {
    if (result.length >= MAX_MEDIATED_ITEMS) {
      throw new RpcFault(-32003, "RESOURCE_LIMIT_EXCEEDED", {
        resource: "mediated-stream-items",
        limit: MAX_MEDIATED_ITEMS,
      });
    }
    result.push(value);
  }
  return result;
}
