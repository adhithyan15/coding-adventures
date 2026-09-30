import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import { Buffer } from "node:buffer";

const MAX_FRAME_BYTES = 4096;
const MAX_HEADER_BYTES = 512;
const MAX_STDERR_BYTES = 16 * 1024;
const MAX_STDOUT_BYTES = 256 * 1024;
const MAX_MESSAGES = 1024;
const MAX_NOTIFICATIONS = 256;
const MAX_QUEUED_MESSAGES = 64;
const DEFAULT_TIMEOUT_MS = 3_000;
const MAX_TIMEOUT_MS = 60_000;
const TERMINATION_GRACE_MS = 250;
const KILL_REAP_MS = 1_000;
const TAG = "$forme";

export interface RunnerCommand {
  readonly executable: string;
  readonly args: readonly string[];
  readonly cwd?: string;
  readonly env?: Readonly<Record<string, string>>;
  readonly modeArgument?: boolean;
  readonly timeoutMs?: number;
  readonly expectedRunner: string;
  readonly expectedRunnerVersion: string;
}

export class RunnerConformanceError extends Error {
  constructor(message: string, readonly cause?: unknown) {
    super(message);
    this.name = "RunnerConformanceError";
  }
}

export class RunnerRemoteError extends Error {
  constructor(readonly rpcCode: number, message: string, readonly data: unknown) {
    super(message);
    this.name = "RunnerRemoteError";
  }
}

interface Pending {
  readonly resolve: (value: unknown) => void;
  readonly reject: (error: unknown) => void;
  readonly timer: ReturnType<typeof setTimeout>;
}

export interface BegunRequest {
  readonly id: number;
  readonly result: Promise<unknown>;
}

export class RunnerSession {
  readonly notifications: Array<Readonly<Record<string, unknown>>> = [];
  private readonly child: ChildProcessWithoutNullStreams;
  private readonly decoder = new ConformanceFrameDecoder(MAX_FRAME_BYTES, MAX_HEADER_BYTES);
  private readonly pending = new Map<number, Pending>();
  private nextId = 1;
  private stderr = Buffer.alloc(0);
  private stdoutBytes = 0;
  private messageCount = 0;
  private queuedMessages = 0;
  private exited = false;
  private failure: unknown | null = null;
  private receiveChain: Promise<void> = Promise.resolve();
  private handler: (method: string, params: Readonly<Record<string, unknown>>, id: number) => Promise<unknown> = async () => {
    throw new RunnerConformanceError("runner issued an unexpected host request");
  };
  private readonly exitPromise: Promise<{ code: number | null; signal: NodeJS.Signals | null }>;
  private readonly timeoutMs: number;
  private terminationPromise: Promise<void> | null = null;

  constructor(command: RunnerCommand, mode: string) {
    this.timeoutMs = command.timeoutMs ?? DEFAULT_TIMEOUT_MS;
    if (!Number.isSafeInteger(this.timeoutMs) || this.timeoutMs <= 0 || this.timeoutMs > MAX_TIMEOUT_MS) {
      throw new RunnerConformanceError("runner timeout must be a positive safe integer no greater than 60000 ms");
    }
    const args = command.modeArgument === false ? [...command.args] : [...command.args, mode];
    this.child = spawn(command.executable, args, {
      cwd: command.cwd,
      env: command.env ? { ...command.env } : undefined,
      stdio: ["pipe", "pipe", "pipe"],
    });
    this.exitPromise = new Promise(resolve => {
      this.child.once("exit", (code, signal) => {
        this.exited = true;
        const suffix = this.stderr.byteLength === 0 ? "" : `: ${this.stderr.toString("utf8")}`;
        this.rejectPending(new RunnerConformanceError(`runner exited before completing the protocol${suffix}`));
        resolve({ code, signal });
      });
      this.child.once("error", error => {
        this.exited = true;
        this.fail(new RunnerConformanceError("runner process could not be started", error));
        resolve({ code: null, signal: null });
      });
    });
    this.child.stdout.on("data", (chunk: Buffer) => {
      if (this.failure !== null) return;
      try {
        this.stdoutBytes += chunk.byteLength;
        if (this.stdoutBytes > MAX_STDOUT_BYTES) {
          throw new RunnerConformanceError("runner stdout exceeds the conformance lifetime bound");
        }
        for (const message of this.decoder.push(chunk)) {
          this.messageCount += 1;
          if (this.messageCount > MAX_MESSAGES) {
            throw new RunnerConformanceError("runner message count exceeds the conformance lifetime bound");
          }
          this.queuedMessages += 1;
          if (this.queuedMessages > MAX_QUEUED_MESSAGES) {
            throw new RunnerConformanceError("runner queued work exceeds the conformance bound");
          }
          this.receiveChain = this.receiveChain.then(async () => {
            try {
              if (this.failure === null) await this.receive(message);
            } finally {
              this.queuedMessages -= 1;
            }
          }).catch(error => { this.failAndTerminate(error); });
        }
      } catch (error) {
        this.failAndTerminate(error);
      }
    });
    this.child.stderr.on("data", (chunk: Buffer) => {
      if (this.stderr.byteLength >= MAX_STDERR_BYTES) return;
      this.stderr = Buffer.concat([this.stderr, chunk.subarray(0, MAX_STDERR_BYTES - this.stderr.byteLength)]);
    });
    this.child.stdin.on("error", error => {
      if (!this.exited && this.terminationPromise === null) {
        this.failAndTerminate(new RunnerConformanceError("runner stdin failed", error));
      }
    });
  }

  onRequest(handler: (method: string, params: Readonly<Record<string, unknown>>, id: number) => Promise<unknown>): void {
    this.handler = handler;
  }

  beginRequest(method: string, params: unknown): BegunRequest {
    if (this.exited) throw new RunnerConformanceError("runner process is already closed");
    const id = this.nextId++;
    const result = new Promise<unknown>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        const error = new RunnerConformanceError(`runner request timed out: ${method}`);
        reject(error);
        this.failAndTerminate(error);
      }, this.timeoutMs);
      this.pending.set(id, { resolve, reject, timer });
    });
    this.write({ jsonrpc: "2.0", id, method, params });
    return { id, result };
  }

  request(method: string, params: unknown): Promise<unknown> {
    return this.beginRequest(method, params).result;
  }

  notify(method: string, params: unknown): void {
    this.write({ jsonrpc: "2.0", method, params });
  }

  writeRaw(bytes: Uint8Array): void {
    if (this.exited) throw new RunnerConformanceError("runner process is already closed");
    this.child.stdin.write(bytes);
  }

  async waitForExit(): Promise<{ code: number | null; signal: NodeJS.Signals | null; stderr: string }> {
    let exit = await this.exitWithin(this.timeoutMs);
    if (exit === null && this.terminationPromise !== null) {
      await this.terminationPromise;
      exit = await this.exitPromise;
    }
    if (exit === null) {
      const error = new RunnerConformanceError("runner did not exit");
      this.fail(error);
      await this.terminate();
      throw error;
    }
    return { ...exit, stderr: this.stderr.toString("utf8") };
  }

  async terminate(): Promise<void> {
    if (this.exited) return;
    this.terminationPromise ??= this.retire();
    await this.terminationPromise;
  }

  private write(message: Readonly<Record<string, unknown>>): void {
    this.child.stdin.write(encodeFrame(message, MAX_FRAME_BYTES));
  }

  private async receive(message: Readonly<Record<string, unknown>>): Promise<void> {
    if (message.jsonrpc !== "2.0") throw new RunnerConformanceError("runner emitted an invalid jsonrpc version");
    if (typeof message.method === "string") {
      const params = asRecord(message.params ?? {}, "runner request params");
      if (message.id === undefined) {
        if (this.notifications.length >= MAX_NOTIFICATIONS) {
          throw new RunnerConformanceError("runner notification count exceeds the conformance bound");
        }
        this.notifications.push(message);
        return;
      }
      if (typeof message.id !== "number" || !Number.isSafeInteger(message.id) || message.id >= 0) {
        throw new RunnerConformanceError("runner request ids must be negative safe integers");
      }
      try {
        const result = await this.handler(message.method, params, message.id);
        this.write({ jsonrpc: "2.0", id: message.id, result });
      } catch (error) {
        const fault = asFault(error);
        this.write({ jsonrpc: "2.0", id: message.id, error: fault });
      }
      return;
    }
    if (typeof message.id !== "number" || !Number.isSafeInteger(message.id) || message.id <= 0) {
      throw new RunnerConformanceError("runner response ids must be positive safe integers");
    }
    const pending = this.pending.get(message.id);
    if (!pending) throw new RunnerConformanceError("runner response has an unknown request id");
    this.pending.delete(message.id);
    clearTimeout(pending.timer);
    const hasResult = Object.prototype.hasOwnProperty.call(message, "result");
    const hasError = Object.prototype.hasOwnProperty.call(message, "error");
    if (hasResult === hasError) return pending.reject(new RunnerConformanceError("runner response must contain one result or error"));
    if (hasResult) return pending.resolve(message.result);
    const error = asRecord(message.error, "runner response error");
    if (typeof error.code !== "number" || typeof error.message !== "string") {
      return pending.reject(new RunnerConformanceError("runner response error is malformed"));
    }
    pending.reject(new RunnerRemoteError(error.code, error.message, error.data));
  }

  private fail(error: unknown): void {
    this.failure ??= error;
    this.rejectPending(error);
  }

  private rejectPending(error: unknown): void {
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(error);
    }
    this.pending.clear();
  }

  private failAndTerminate(error: unknown): void {
    this.fail(error);
    this.child.stdout.pause();
    void this.terminate().catch(() => undefined);
  }

  private async retire(): Promise<void> {
    this.child.stdin.destroy();
    this.child.stdout.pause();
    if (this.exited) return;
    this.child.kill("SIGTERM");
    if (await this.exitWithin(Math.min(this.timeoutMs, TERMINATION_GRACE_MS))) return;
    this.child.kill("SIGKILL");
    if (await this.exitWithin(KILL_REAP_MS)) return;
    throw new RunnerConformanceError("runner could not be reaped after SIGKILL");
  }

  private async exitWithin(timeoutMs: number): Promise<{ code: number | null; signal: NodeJS.Signals | null } | null> {
    if (this.exited) return this.exitPromise;
    return new Promise(resolve => {
      const timer = setTimeout(() => resolve(null), timeoutMs);
      timer.unref();
      void this.exitPromise.then(result => {
        clearTimeout(timer);
        resolve(result);
      });
    });
  }
}

export function encodeFrame(message: Readonly<Record<string, unknown>>, maxBytes = MAX_FRAME_BYTES): Buffer {
  assertWireValueWithinLimit(message, maxBytes);
  const payload = Buffer.from(JSON.stringify(encodeWireValue(message)), "utf8");
  if (payload.byteLength > maxBytes) throw new RunnerConformanceError("outbound frame exceeds the conformance bound");
  return Buffer.concat([Buffer.from(`Content-Length: ${payload.byteLength}\r\n\r\n`, "ascii"), payload]);
}

export function encodeWireValue(value: unknown): unknown {
  const seen = new Set<object>();
  const visit = (entry: unknown): unknown => {
    if (entry instanceof Uint8Array) return { [TAG]: "bytes", base64: Buffer.from(entry).toString("base64") };
    if (entry === null || typeof entry !== "object") return entry;
    if (seen.has(entry)) throw new RunnerConformanceError("wire value is circular");
    seen.add(entry);
    try {
      if (Array.isArray(entry)) return entry.map(visit);
      const record = entry as Record<string, unknown>;
      if (Object.prototype.hasOwnProperty.call(record, TAG)) {
        return { [TAG]: "escaped-object", entries: Object.entries(record).map(([key, child]) => [key, visit(child)]) };
      }
      return Object.fromEntries(Object.entries(record).map(([key, child]) => [key, visit(child)]));
    } finally {
      seen.delete(entry);
    }
  };
  return visit(value);
}

export function decodeWireValue(value: unknown): unknown {
  const visit = (entry: unknown): unknown => {
    if (entry === null || typeof entry !== "object") return entry;
    if (Array.isArray(entry)) return entry.map(visit);
    const record = entry as Record<string, unknown>;
    if (record[TAG] === "bytes" && Object.keys(record).length === 2 && typeof record.base64 === "string") {
      if (!canonicalBase64(record.base64)) throw new RunnerConformanceError("runner emitted a non-canonical binary envelope");
      return Uint8Array.from(Buffer.from(record.base64, "base64"));
    }
    if (record[TAG] === "escaped-object" && Object.keys(record).length === 2 && Array.isArray(record.entries)) {
      const result: Record<string, unknown> = {};
      for (const pair of record.entries) {
        if (!Array.isArray(pair) || pair.length !== 2 || typeof pair[0] !== "string" || Object.hasOwn(result, pair[0])) {
          throw new RunnerConformanceError("runner emitted an invalid escaped-object envelope");
        }
        Object.defineProperty(result, pair[0], { value: visit(pair[1]), enumerable: true, writable: true, configurable: true });
      }
      return result;
    }
    return Object.fromEntries(Object.entries(record).map(([key, child]) => [key, visit(child)]));
  };
  return visit(value);
}

export class ConformanceFrameDecoder {
  private buffered = Buffer.alloc(0);
  private expected: number | null = null;

  constructor(private readonly maxFrameBytes: number, private readonly maxHeaderBytes: number) {}

  push(chunk: Uint8Array): Array<Readonly<Record<string, unknown>>> {
    this.buffered = Buffer.concat([this.buffered, chunk]);
    const messages: Array<Readonly<Record<string, unknown>>> = [];
    while (true) {
      if (this.expected === null) {
        const end = this.buffered.indexOf("\r\n\r\n");
        if (end < 0) {
          if (this.buffered.byteLength > this.maxHeaderBytes) throw new RunnerConformanceError("runner header exceeds the conformance bound");
          break;
        }
        if (end > this.maxHeaderBytes) throw new RunnerConformanceError("runner header exceeds the conformance bound");
        const lines = this.buffered.subarray(0, end).toString("ascii").split("\r\n");
        const lengths = lines.filter(line => line.toLowerCase().startsWith("content-length:"));
        if (lengths.length !== 1) throw new RunnerConformanceError("runner frame needs one Content-Length");
        const value = lengths[0]!.slice(lengths[0]!.indexOf(":") + 1).trim();
        if (!/^(?:0|[1-9][0-9]*)$/.test(value)) throw new RunnerConformanceError("runner Content-Length is not canonical");
        this.expected = Number(value);
        if (!Number.isSafeInteger(this.expected) || this.expected > this.maxFrameBytes) {
          throw new RunnerConformanceError("runner frame exceeds the conformance bound");
        }
        this.buffered = this.buffered.subarray(end + 4);
      }
      if (this.buffered.byteLength < this.expected) break;
      const payload = this.buffered.subarray(0, this.expected);
      this.buffered = this.buffered.subarray(this.expected);
      this.expected = null;
      let decoded: unknown;
      try { decoded = decodeWireValue(JSON.parse(payload.toString("utf8"))); }
      catch (error) { throw new RunnerConformanceError("runner frame is not valid bounded JSON", error); }
      messages.push(asRecord(decoded, "runner message"));
    }
    return messages;
  }
}

function asRecord(value: unknown, label: string): Readonly<Record<string, unknown>> {
  if (typeof value !== "object" || value === null || Array.isArray(value) || ArrayBuffer.isView(value)) {
    throw new RunnerConformanceError(`${label} must be an object`);
  }
  return value as Readonly<Record<string, unknown>>;
}

function asFault(error: unknown): { code: number; message: string; data?: unknown } {
  if (error instanceof RunnerRemoteError) return { code: error.rpcCode, message: error.message, data: error.data };
  return { code: -32603, message: error instanceof Error ? error.message : "INTERNAL_ERROR" };
}

function canonicalBase64(value: string): boolean {
  return value.length % 4 === 0
    && /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(value)
    && Buffer.from(value, "base64").toString("base64") === value;
}

function assertWireValueWithinLimit(value: unknown, limit: number): void {
  let estimate = 0;
  const seen = new Set<object>();
  const add = (amount: number) => {
    estimate += amount;
    if (estimate > limit) throw new RunnerConformanceError("outbound frame exceeds the conformance bound");
  };
  const visit = (entry: unknown, arraySlot = false): void => {
    if (entry instanceof Uint8Array) {
      add(Buffer.byteLength('{"$forme":"bytes","base64":""}') + 4 * Math.ceil(entry.byteLength / 3));
      return;
    }
    if (entry === null) { add(4); return; }
    if (typeof entry === "string") { add(jsonStringBytes(entry)); return; }
    if (typeof entry === "number") { add(Number.isFinite(entry) ? String(entry).length : 4); return; }
    if (typeof entry === "boolean") { add(entry ? 4 : 5); return; }
    if (entry === undefined || typeof entry === "function" || typeof entry === "symbol") {
      if (arraySlot) add(4);
      return;
    }
    if (typeof entry === "bigint") throw new TypeError("BigInt is not JSON serializable");
    const object = entry as Record<string, unknown>;
    if (seen.has(object)) throw new RunnerConformanceError("wire value is circular");
    seen.add(object);
    try {
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
    } finally {
      seen.delete(object);
    }
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
