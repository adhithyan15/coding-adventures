import type { Readable, Writable } from "node:stream";
import { PluginHostError, RpcFault, RpcRemoteError } from "./errors.js";

export type JsonRpcId = number | string;
export type JsonRpcMessage = Readonly<Record<string, unknown>>;

export interface FrameLimits {
  readonly maxFrameBytes: number;
  readonly maxHeaderBytes: number;
}

export function encodeFrame(message: JsonRpcMessage): Buffer {
  return encodeWireFrame(encodeWireValue(message));
}

function encodeWireFrame(message: unknown): Buffer {
  const payload = Buffer.from(JSON.stringify(message), "utf8");
  return Buffer.concat([
    Buffer.from(`Content-Length: ${payload.length}\r\n\r\n`, "ascii"),
    payload,
  ]);
}

export class FrameDecoder {
  private headerBuffer = Buffer.alloc(0);
  private expectedLength: number | null = null;
  private payloadBuffer: Buffer | null = null;
  private payloadOffset = 0;

  constructor(private readonly limits: FrameLimits) {}

  push(chunk: Uint8Array): JsonRpcMessage[] {
    let incoming = Buffer.from(chunk);
    const messages: JsonRpcMessage[] = [];
    while (incoming.length > 0) {
      if (this.expectedLength === null) {
        const headerBytes = this.headerBuffer.length === 0
          ? incoming
          : Buffer.concat([this.headerBuffer, incoming]);
        const headerEnd = headerBytes.indexOf("\r\n\r\n");
        if (headerEnd < 0) {
          if (headerBytes.length > this.limits.maxHeaderBytes) {
            throw new PluginHostError("HEADER_TOO_LARGE", "wire header exceeds configured bound");
          }
          this.headerBuffer = Buffer.from(headerBytes);
          break;
        }
        if (headerEnd > this.limits.maxHeaderBytes) {
          throw new PluginHostError("HEADER_TOO_LARGE", "wire header exceeds configured bound");
        }
        const header = headerBytes.subarray(0, headerEnd).toString("ascii");
        incoming = headerBytes.subarray(headerEnd + 4);
        this.headerBuffer = Buffer.alloc(0);
        const lengths = header.split("\r\n")
          .map(line => /^Content-Length:\s*(.*)$/i.exec(line))
          .filter((match): match is RegExpExecArray => match !== null);
        if (lengths.length !== 1 || !/^(0|[1-9][0-9]*)$/.test(lengths[0]![1]!)) {
          throw new PluginHostError("PROTOCOL_VIOLATION", "frame requires exactly one decimal Content-Length");
        }
        const length = Number(lengths[0]![1]);
        if (!Number.isSafeInteger(length) || length > this.limits.maxFrameBytes) {
          throw new PluginHostError("FRAME_TOO_LARGE", "wire payload exceeds configured bound", { length });
        }
        this.expectedLength = length;
        this.payloadBuffer = Buffer.allocUnsafe(length);
        this.payloadOffset = 0;
      }
      const remaining = this.expectedLength - this.payloadOffset;
      const consumed = Math.min(remaining, incoming.length);
      incoming.copy(this.payloadBuffer!, this.payloadOffset, 0, consumed);
      this.payloadOffset += consumed;
      incoming = incoming.subarray(consumed);
      if (this.payloadOffset < this.expectedLength) break;
      const payload = this.payloadBuffer!;
      this.expectedLength = null;
      this.payloadBuffer = null;
      this.payloadOffset = 0;
      let parsed: unknown;
      try {
        parsed = JSON.parse(payload.toString("utf8"));
      } catch (cause) {
        throw new PluginHostError("PROTOCOL_VIOLATION", "frame payload is not valid JSON", {}, { cause });
      }
      parsed = decodeWireValue(parsed);
      if (!isRecord(parsed)) {
        throw new PluginHostError("PROTOCOL_VIOLATION", "JSON-RPC message must be an object");
      }
      messages.push(parsed);
    }
    return messages;
  }

  finish(): void {
    if (this.headerBuffer.length !== 0 || this.expectedLength !== null) {
      throw new PluginHostError("TRUNCATED_FRAME", "wire ended in the middle of a frame");
    }
  }
}

interface PendingRequest {
  readonly resolve: (value: unknown) => void;
  readonly reject: (error: unknown) => void;
  readonly timer: ReturnType<typeof setTimeout>;
}

export interface RpcPeerOptions extends FrameLimits {
  readonly input: Writable;
  readonly output: Readable;
  readonly requestTimeoutMs: number;
  readonly onRequest: (method: string, params: unknown) => Promise<unknown>;
  readonly onNotification: (method: string, params: unknown) => Promise<void> | void;
  readonly onFatal: (error: Error) => void;
}

export interface PendingRpc {
  readonly id: number;
  readonly response: Promise<unknown>;
}

export class RpcPeer {
  private readonly decoder: FrameDecoder;
  private readonly pending = new Map<JsonRpcId, PendingRequest>();
  private nextRequestId = 1;
  private failed: Error | null = null;
  private reader: Promise<void> | null = null;

  constructor(private readonly options: RpcPeerOptions) {
    this.decoder = new FrameDecoder(options);
  }

  start(): void {
    if (this.reader) return;
    this.reader = this.readLoop();
  }

  async request(method: string, params: unknown, timeoutMs = this.options.requestTimeoutMs): Promise<unknown> {
    return this.beginRequest(method, params, timeoutMs).response;
  }

  beginRequest(method: string, params: unknown, timeoutMs = this.options.requestTimeoutMs): PendingRpc {
    this.assertOpen();
    const id = this.nextRequestId++;
    const response = new Promise<unknown>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new PluginHostError(
          method === "handshake" ? "HANDSHAKE_TIMEOUT" : "REQUEST_TIMEOUT",
          `${method} did not respond within ${timeoutMs}ms`,
        ));
      }, timeoutMs);
      this.pending.set(id, { resolve, reject, timer });
    });
    void this.write({ jsonrpc: "2.0", id, method, params }).catch(error => {
      const pending = this.pending.get(id);
      if (pending) {
        clearTimeout(pending.timer);
        this.pending.delete(id);
        pending.reject(error);
      }
    });
    return { id, response };
  }

  async notify(method: string, params: unknown): Promise<void> {
    this.assertOpen();
    await this.write({ jsonrpc: "2.0", method, params });
  }

  fail(error: Error): void {
    if (this.failed) return;
    this.failed = error;
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(error);
    }
    this.pending.clear();
  }

  private async readLoop(): Promise<void> {
    try {
      for await (const chunk of this.options.output) {
        const bytes = typeof chunk === "string" ? Buffer.from(chunk) : chunk;
        for (const message of this.decoder.push(bytes)) {
          await this.handle(message);
        }
      }
      this.decoder.finish();
    } catch (error) {
      const fatal = error instanceof Error ? error : new Error(String(error));
      this.fail(fatal);
      this.options.onFatal(fatal);
    }
  }

  private async handle(message: JsonRpcMessage): Promise<void> {
    if (message.jsonrpc !== "2.0") {
      throw new PluginHostError("PROTOCOL_VIOLATION", "jsonrpc must equal 2.0");
    }
    if (typeof message.method === "string") {
      if (message.id === undefined) {
        await this.options.onNotification(message.method, message.params);
        return;
      }
      if (typeof message.id !== "number" || !Number.isSafeInteger(message.id) || message.id >= 0) {
        throw new PluginHostError("PROTOCOL_VIOLATION", "plugin request ids must be negative safe integers");
      }
      try {
        const result = await this.options.onRequest(message.method, message.params);
        await this.write({ jsonrpc: "2.0", id: message.id, result });
      } catch (error) {
        const fault = error instanceof RpcFault
          ? error
          : new RpcFault(-32603, "INTERNAL_ERROR");
        await this.write({
          jsonrpc: "2.0",
          id: message.id,
          error: { code: fault.rpcCode, message: fault.message, data: fault.data },
        });
      }
      return;
    }

    const id = message.id;
    if ((typeof id !== "number" && typeof id !== "string") || !this.pending.has(id)) {
      throw new PluginHostError("PROTOCOL_VIOLATION", "response has an unknown request id");
    }
    const pending = this.pending.get(id)!;
    this.pending.delete(id);
    clearTimeout(pending.timer);
    const hasResult = Object.prototype.hasOwnProperty.call(message, "result");
    const hasError = Object.prototype.hasOwnProperty.call(message, "error");
    if (hasResult === hasError) {
      pending.reject(new PluginHostError(
        "PROTOCOL_VIOLATION",
        "response must contain exactly one of result or error",
      ));
      return;
    }
    if (hasError) {
      const error = message.error;
      if (!isRecord(error) || typeof error.code !== "number" || typeof error.message !== "string") {
        pending.reject(new PluginHostError("PROTOCOL_VIOLATION", "malformed JSON-RPC error"));
      } else {
        pending.reject(new RpcRemoteError(error.code, error.message, error.data));
      }
    } else {
      pending.resolve(message.result);
    }
  }

  private async write(message: JsonRpcMessage): Promise<void> {
    assertWireValueWithinLimit(message, this.options.maxFrameBytes);
    const encoded = encodeWireValue(message);
    assertJsonWithinLimit(encoded, this.options.maxFrameBytes);
    const frame = encodeWireFrame(encoded);
    await new Promise<void>((resolve, reject) => {
      this.options.input.write(frame, (error?: Error | null) => {
        if (error) reject(error); else resolve();
      });
    });
  }

  private assertOpen(): void {
    if (this.failed) throw this.failed;
  }
}

const FORME_WIRE_TAG = "$forme";

/** Encode binary Forme values without colliding with ordinary user objects. */
export function encodeWireValue(value: unknown): unknown {
  const seen = new Set<object>();
  const encode = (entry: unknown): unknown => {
    if (entry instanceof Uint8Array) {
      return { [FORME_WIRE_TAG]: "bytes", base64: Buffer.from(entry).toString("base64") };
    }
    if (entry === null || typeof entry !== "object") return entry;
    if (seen.has(entry)) throw new TypeError("circular JSON value");
    seen.add(entry);
    try {
      if (Array.isArray(entry)) return entry.map(encode);
      const record = entry as Record<string, unknown>;
      if (Object.prototype.hasOwnProperty.call(record, FORME_WIRE_TAG)) {
        return {
          [FORME_WIRE_TAG]: "escaped-object",
          entries: Object.entries(record).map(([key, child]) => [key, encode(child)]),
        };
      }
      return Object.fromEntries(Object.entries(record).map(([key, child]) => [key, encode(child)]));
    } finally {
      seen.delete(entry);
    }
  };
  return encode(value);
}

/** Decode the reserved Forme wire envelope, including escaped user objects. */
export function decodeWireValue(value: unknown): unknown {
  const decode = (entry: unknown): unknown => {
    if (entry === null || typeof entry !== "object") return entry;
    if (Array.isArray(entry)) return entry.map(decode);
    const record = entry as Record<string, unknown>;
    if (record[FORME_WIRE_TAG] === "bytes"
        && Object.keys(record).length === 2
        && typeof record.base64 === "string") {
      if (!isCanonicalBase64(record.base64)) {
        throw new PluginHostError("PROTOCOL_VIOLATION", "invalid binary wire envelope");
      }
      return Uint8Array.from(Buffer.from(record.base64, "base64"));
    }
    if (record[FORME_WIRE_TAG] === "escaped-object"
        && Object.keys(record).length === 2
        && Array.isArray(record.entries)) {
      const result: Record<string, unknown> = {};
      for (const pair of record.entries) {
        if (!Array.isArray(pair) || pair.length !== 2 || typeof pair[0] !== "string"
            || Object.prototype.hasOwnProperty.call(result, pair[0])) {
          throw new PluginHostError("PROTOCOL_VIOLATION", "invalid escaped-object wire envelope");
        }
        Object.defineProperty(result, pair[0], {
          value: decode(pair[1]), enumerable: true, configurable: true, writable: true,
        });
      }
      return result;
    }
    return Object.fromEntries(Object.entries(record).map(([key, child]) => [key, decode(child)]));
  };
  return decode(value);
}

function isCanonicalBase64(value: string): boolean {
  if (value.length % 4 !== 0 || !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(value)) {
    return false;
  }
  return Buffer.from(value, "base64").toString("base64") === value;
}

function assertWireValueWithinLimit(value: unknown, limit: number): void {
  let bytes = 0;
  const seen = new Set<object>();
  const add = (amount: number): void => {
    bytes += amount;
    if (bytes > limit) {
      throw new PluginHostError("FRAME_TOO_LARGE", "outbound wire payload exceeds configured bound", {
        length: bytes,
      });
    }
  };
  const visit = (entry: unknown, arraySlot = false): void => {
    if (entry instanceof Uint8Array) {
      add(Buffer.byteLength('{"$forme":"bytes","base64":""}') + 4 * Math.ceil(entry.byteLength / 3));
      return;
    }
    if (entry === null) { add(4); return; }
    if (typeof entry === "string") { add(jsonStringBytes(entry)); return; }
    if (typeof entry === "number") { add(jsonNumberBytes(entry)); return; }
    if (typeof entry === "boolean") { add(entry ? 4 : 5); return; }
    if (typeof entry === "undefined" || typeof entry === "function" || typeof entry === "symbol") {
      if (arraySlot) add(4);
      return;
    }
    if (typeof entry === "bigint") throw new TypeError("BigInt is not JSON serializable");
    const object = entry as Record<string, unknown>;
    if (seen.has(object)) throw new TypeError("circular JSON value");
    seen.add(object);
    if (Array.isArray(object)) {
      add(2);
      for (let index = 0; index < object.length; index += 1) {
        if (index > 0) add(1);
        visit(object[index], true);
      }
    } else if (Object.prototype.hasOwnProperty.call(object, FORME_WIRE_TAG)) {
      add(Buffer.byteLength('{"$forme":"escaped-object","entries":[') + 2);
      let first = true;
      for (const [key, child] of Object.entries(object)) {
        if (!first) add(1);
        first = false;
        add(2 + jsonStringBytes(key) + 1);
        visit(child, true);
        add(1);
      }
    } else {
      add(2);
      let first = true;
      for (const [key, child] of Object.entries(object)) {
        if (typeof child === "undefined" || typeof child === "function" || typeof child === "symbol") continue;
        if (!first) add(1);
        first = false;
        add(jsonStringBytes(key) + 1);
        visit(child);
      }
    }
    seen.delete(object);
  };
  visit(value);
}

function assertJsonWithinLimit(value: unknown, limit: number): void {
  let bytes = 0;
  const seen = new Set<object>();
  const add = (amount: number): void => {
    bytes += amount;
    if (bytes > limit) {
      throw new PluginHostError("FRAME_TOO_LARGE", "outbound wire payload exceeds configured bound", {
        length: bytes,
      });
    }
  };
  const visit = (entry: unknown, arraySlot = false): void => {
    if (entry === null) { add(4); return; }
    if (typeof entry === "string") { add(jsonStringBytes(entry)); return; }
    if (typeof entry === "number") { add(jsonNumberBytes(entry)); return; }
    if (typeof entry === "boolean") { add(entry ? 4 : 5); return; }
    if (typeof entry === "undefined" || typeof entry === "function" || typeof entry === "symbol") {
      if (arraySlot) add(4);
      return;
    }
    if (typeof entry === "bigint") throw new TypeError("BigInt is not JSON serializable");
    const object = entry as Record<string, unknown>;
    if (seen.has(object)) throw new TypeError("circular JSON value");
    seen.add(object);
    if (Array.isArray(object)) {
      add(2);
      for (let index = 0; index < object.length; index += 1) {
        if (index > 0) add(1);
        visit(object[index], true);
      }
    } else {
      add(2);
      let first = true;
      for (const [key, child] of Object.entries(object)) {
        if (typeof child === "undefined" || typeof child === "function" || typeof child === "symbol") continue;
        if (!first) add(1);
        first = false;
        add(jsonStringBytes(key) + 1);
        visit(child);
      }
    }
    seen.delete(object);
  };
  visit(value);
}

function jsonStringBytes(value: string): number {
  let bytes = 2;
  for (let index = 0; index < value.length; index += 1) {
    const code = value.charCodeAt(index);
    if (code === 0x22 || code === 0x5c || code === 0x08 || code === 0x0c
        || code === 0x0a || code === 0x0d || code === 0x09) bytes += 2;
    else if (code < 0x20 || (code >= 0xd800 && code <= 0xdfff
      && !(code <= 0xdbff && index + 1 < value.length
        && value.charCodeAt(index + 1) >= 0xdc00 && value.charCodeAt(index + 1) <= 0xdfff))) bytes += 6;
    else if (code < 0x80) bytes += 1;
    else if (code < 0x800) bytes += 2;
    else if (code >= 0xd800 && code <= 0xdbff) { bytes += 4; index += 1; }
    else bytes += 3;
  }
  return bytes;
}

function jsonNumberBytes(value: number): number {
  return Number.isFinite(value) ? String(value).length : 4;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function asRecord(value: unknown, label: string): Record<string, unknown> {
  if (!isRecord(value)) throw new PluginHostError("PROTOCOL_VIOLATION", `${label} must be an object`);
  return value;
}
