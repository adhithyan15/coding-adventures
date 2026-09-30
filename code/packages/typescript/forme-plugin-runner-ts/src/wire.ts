import type { Readable, Writable } from "node:stream";

export type JsonRpcId = number | string;
export type JsonRpcMessage = Readonly<Record<string, unknown>>;

export interface FrameLimits {
  readonly maxFrameBytes: number;
  readonly maxHeaderBytes: number;
}

export class RunnerProtocolError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "RunnerProtocolError";
  }
}

export class RunnerRpcRemoteError extends Error {
  constructor(
    readonly rpcCode: number,
    message: string,
    readonly data: unknown,
  ) {
    super(message);
    this.name = "RunnerRpcRemoteError";
  }
}

const TAG = "$forme";

export function encodeFrame(message: JsonRpcMessage, maxFrameBytes = Number.MAX_SAFE_INTEGER): Buffer {
  assertWireValueWithinLimit(message, maxFrameBytes);
  const payload = Buffer.from(JSON.stringify(encodeWireValue(message)), "utf8");
  if (payload.byteLength > maxFrameBytes) throw new RunnerProtocolError("wire payload exceeds configured bound");
  return Buffer.concat([
    Buffer.from(`Content-Length: ${payload.byteLength}\r\n\r\n`, "ascii"),
    payload,
  ]);
}

export function encodeWireValue(value: unknown): unknown {
  const seen = new Set<object>();
  const encode = (entry: unknown): unknown => {
    if (entry instanceof Uint8Array) return { [TAG]: "bytes", base64: Buffer.from(entry).toString("base64") };
    if (entry === null || typeof entry !== "object") return entry;
    if (seen.has(entry)) throw new TypeError("circular JSON value");
    seen.add(entry);
    try {
      if (Array.isArray(entry)) return entry.map(encode);
      const record = entry as Record<string, unknown>;
      if (Object.prototype.hasOwnProperty.call(record, TAG)) {
        return { [TAG]: "escaped-object", entries: Object.entries(record).map(([key, child]) => [key, encode(child)]) };
      }
      return Object.fromEntries(Object.entries(record).map(([key, child]) => [key, encode(child)]));
    } finally {
      seen.delete(entry);
    }
  };
  return encode(value);
}

export function decodeWireValue(value: unknown): unknown {
  const decode = (entry: unknown): unknown => {
    if (entry === null || typeof entry !== "object") return entry;
    if (Array.isArray(entry)) return entry.map(decode);
    const record = entry as Record<string, unknown>;
    if (record[TAG] === "bytes" && Object.keys(record).length === 2 && typeof record.base64 === "string") {
      if (!isCanonicalBase64(record.base64)) throw new RunnerProtocolError("invalid binary wire envelope");
      return Uint8Array.from(Buffer.from(record.base64, "base64"));
    }
    if (record[TAG] === "escaped-object" && Object.keys(record).length === 2 && Array.isArray(record.entries)) {
      const result: Record<string, unknown> = {};
      for (const pair of record.entries) {
        if (!Array.isArray(pair) || pair.length !== 2 || typeof pair[0] !== "string"
            || Object.prototype.hasOwnProperty.call(result, pair[0])) {
          throw new RunnerProtocolError("invalid escaped-object wire envelope");
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
          if (headerBytes.length > this.limits.maxHeaderBytes) throw new RunnerProtocolError("wire header exceeds configured bound");
          this.headerBuffer = Buffer.from(headerBytes);
          break;
        }
        if (headerEnd > this.limits.maxHeaderBytes) throw new RunnerProtocolError("wire header exceeds configured bound");
        const header = headerBytes.subarray(0, headerEnd).toString("ascii");
        incoming = headerBytes.subarray(headerEnd + 4);
        this.headerBuffer = Buffer.alloc(0);
        const values = header.split("\r\n")
          .map(parseContentLength)
          .filter((length): length is string => length !== null);
        if (values.length !== 1 || !isCanonicalDecimal(values[0]!)) {
          throw new RunnerProtocolError("frame requires exactly one canonical Content-Length");
        }
        const length = Number(values[0]);
        if (!Number.isSafeInteger(length) || length > this.limits.maxFrameBytes) {
          throw new RunnerProtocolError("wire payload exceeds configured bound");
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
      let decoded: unknown;
      try { decoded = decodeWireValue(JSON.parse(payload.toString("utf8"))); }
      catch (error) {
        if (error instanceof RunnerProtocolError) throw error;
        throw new RunnerProtocolError("wire payload is not valid JSON");
      }
      if (!isRecord(decoded)) throw new RunnerProtocolError("JSON-RPC message must be an object");
      messages.push(decoded);
    }
    return messages;
  }

  finish(): void {
    if (this.headerBuffer.length !== 0 || this.expectedLength !== null) throw new RunnerProtocolError("wire ended in the middle of a frame");
  }
}

function parseContentLength(line: string): string | null {
  const prefix = "content-length:";
  if (line.length < prefix.length || line.slice(0, prefix.length).toLowerCase() !== prefix) return null;
  let offset = prefix.length;
  while (line[offset] === " " || line[offset] === "\t") offset += 1;
  return line.slice(offset);
}

function isCanonicalDecimal(value: string): boolean {
  if (value === "0") return true;
  if (value.length === 0 || value.charCodeAt(0) < 0x31 || value.charCodeAt(0) > 0x39) return false;
  for (let index = 1; index < value.length; index += 1) {
    const code = value.charCodeAt(index);
    if (code < 0x30 || code > 0x39) return false;
  }
  return true;
}

interface Pending {
  readonly resolve: (value: unknown) => void;
  readonly reject: (error: unknown) => void;
}

export interface RunnerRpcPeerOptions extends FrameLimits {
  readonly input: Readable;
  readonly output: Writable;
  readonly onRequest: (id: number, method: string, params: unknown) => Promise<unknown>;
  readonly onNotification: (method: string, params: unknown) => Promise<void> | void;
}

export class RunnerRpcPeer {
  private readonly decoder: FrameDecoder;
  private readonly pending = new Map<number, Pending>();
  private nextRequestId = -1;
  private failed: Error | null = null;
  private stopAfterResponse = false;

  constructor(private readonly options: RunnerRpcPeerOptions) {
    this.decoder = new FrameDecoder(options);
  }

  async run(): Promise<void> {
    try {
      for await (const chunk of this.options.input) {
        for (const message of this.decoder.push(typeof chunk === "string" ? Buffer.from(chunk) : chunk)) {
          await this.receive(message);
        }
      }
      this.decoder.finish();
      if (!this.stopAfterResponse) this.fail(new RunnerProtocolError("host wire closed unexpectedly"));
    } catch (error) {
      const failure = error instanceof Error ? error : new Error(String(error));
      this.fail(failure);
      if (!this.stopAfterResponse) throw failure;
    }
  }

  request(method: string, params: unknown): Promise<unknown> {
    this.assertOpen();
    const id = this.nextRequestId--;
    const result = new Promise<unknown>((resolve, reject) => this.pending.set(id, { resolve, reject }));
    void this.write({ jsonrpc: "2.0", id, method, params }).catch(error => {
      const pending = this.pending.get(id);
      this.pending.delete(id);
      pending?.reject(error);
    });
    return result;
  }

  notify(method: string, params: unknown): Promise<void> {
    this.assertOpen();
    return this.write({ jsonrpc: "2.0", method, params });
  }

  requestStopAfterResponse(): void { this.stopAfterResponse = true; }

  shutdown(): void {
    this.stopAfterResponse = true;
    this.options.input.destroy();
  }

  fail(error: Error): void {
    if (this.failed) return;
    this.failed = error;
    for (const pending of this.pending.values()) pending.reject(error);
    this.pending.clear();
  }

  private async receive(message: JsonRpcMessage): Promise<void> {
    if (message.jsonrpc !== "2.0") throw new RunnerProtocolError("jsonrpc must equal 2.0");
    if (typeof message.method === "string") {
      if (message.id === undefined) {
        await this.options.onNotification(message.method, message.params);
        return;
      }
      if (typeof message.id !== "number" || !Number.isSafeInteger(message.id) || message.id <= 0) {
        throw new RunnerProtocolError("host request ids must be positive safe integers");
      }
      void this.answer(message.id, message.method, message.params);
      return;
    }
    if (typeof message.id !== "number" || !Number.isSafeInteger(message.id) || message.id >= 0) {
      throw new RunnerProtocolError("plugin response ids must be negative safe integers");
    }
    const pending = this.pending.get(message.id);
    if (!pending) throw new RunnerProtocolError("response has an unknown request id");
    this.pending.delete(message.id);
    const hasResult = Object.prototype.hasOwnProperty.call(message, "result");
    const hasError = Object.prototype.hasOwnProperty.call(message, "error");
    if (hasResult === hasError) return pending.reject(new RunnerProtocolError("response must contain exactly one of result or error"));
    if (!hasError) return pending.resolve(message.result);
    const error = message.error;
    if (!isRecord(error) || typeof error.code !== "number" || typeof error.message !== "string") {
      return pending.reject(new RunnerProtocolError("malformed JSON-RPC error"));
    }
    pending.reject(new RunnerRpcRemoteError(error.code, error.message, error.data));
  }

  private async answer(id: number, method: string, params: unknown): Promise<void> {
    try {
      const result = await this.options.onRequest(id, method, params);
      await this.write({ jsonrpc: "2.0", id, result });
    } catch (error) {
      const fault = toRpcError(error);
      await this.write({ jsonrpc: "2.0", id, error: fault });
    } finally {
      if (this.stopAfterResponse) this.options.input.destroy();
    }
  }

  private async write(message: JsonRpcMessage): Promise<void> {
    const frame = encodeFrame(message, this.options.maxFrameBytes);
    await new Promise<void>((resolve, reject) => {
      this.options.output.write(frame, error => error ? reject(error) : resolve());
    });
  }

  private assertOpen(): void { if (this.failed) throw this.failed; }
}

function toRpcError(error: unknown): { code: number; message: string; data?: unknown } {
  if (isRecord(error) && typeof error.rpcCode === "number" && typeof error.message === "string") {
    return { code: error.rpcCode, message: error.message, data: error.data };
  }
  return { code: -32603, message: error instanceof Error ? error.message : "INTERNAL_ERROR" };
}

function isCanonicalBase64(value: string): boolean {
  return value.length % 4 === 0
    && /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(value)
    && Buffer.from(value, "base64").toString("base64") === value;
}

function assertWireValueWithinLimit(value: unknown, limit: number): void {
  let estimate = 0;
  const seen = new Set<object>();
  const add = (amount: number) => {
    estimate += amount;
    if (estimate > limit) throw new RunnerProtocolError("wire payload exceeds configured bound");
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
    if (entry === undefined || typeof entry === "function" || typeof entry === "symbol") {
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
    } else if (Object.prototype.hasOwnProperty.call(object, TAG)) {
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
        if (child === undefined || typeof child === "function" || typeof child === "symbol") continue;
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

export function asRecord(value: unknown, label: string): Record<string, unknown> {
  if (!isRecord(value)) throw new RunnerProtocolError(`${label} must be an object`);
  return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
