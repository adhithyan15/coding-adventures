import process from "node:process";
import type { Readable, Writable } from "node:stream";
import { CancellationError, StageError, isCancellationError } from "@coding-adventures/forme-errors";
import {
  createCancellationTokenSource,
  neverCancelledToken,
  type Stage,
  type StageInitContext,
} from "@coding-adventures/forme-stage";
import { KERNEL_API_VERSION, type KindDescriptor } from "@coding-adventures/forme-types";
import { buildWireContext, wireLogger } from "./build-context.js";
import { RunnerProtocolError, RunnerRpcPeer, asRecord, encodeWireValue } from "./wire.js";

const PROTOCOL_VERSION = 1;
const RUNNER_NAME = "@coding-adventures/forme-plugin-runner-ts";
const RUNNER_VERSION = "1.0.0";

export interface RunPluginOptions {
  readonly input?: Readable;
  readonly output?: Writable;
  readonly argv?: readonly string[];
  readonly installSignalHandlers?: boolean;
  readonly maxFrameBytes?: number;
  readonly maxHeaderBytes?: number;
  readonly maxBufferedStreamValues?: number;
  readonly maxBufferedStreamBytes?: number;
  readonly signalShutdownTimeoutMs?: number;
}

type AnyStage = Stage<KindDescriptor, KindDescriptor>;

export async function runPlugin(stage: AnyStage, options: RunPluginOptions = {}): Promise<void> {
  validateStage(stage);
  const argv = options.argv ?? process.argv;
  const stageId = argv[2] ?? localStageId(stage.name);
  const configSchemaHash = argv[3] === undefined || argv[3] === "-" ? null : argv[3];
  if (!isToken(stageId, 128)) throw new RunnerProtocolError("runner stage id is invalid");
  if (configSchemaHash !== null && !isToken(configSchemaHash, 256)) throw new RunnerProtocolError("runner schema identity is invalid");
  const signalShutdownTimeoutMs = positive(options.signalShutdownTimeoutMs, 1_000);
  const state = new RunnerState(stage, stageId, configSchemaHash, {
    maxBufferedStreamValues: positive(options.maxBufferedStreamValues, 64),
    maxBufferedStreamBytes: positive(options.maxBufferedStreamBytes, 8 * 1024 * 1024),
  });
  let peer!: RunnerRpcPeer;
  peer = new RunnerRpcPeer({
    input: options.input ?? process.stdin,
    output: options.output ?? process.stdout,
    maxFrameBytes: positive(options.maxFrameBytes, 8 * 1024 * 1024),
    maxHeaderBytes: positive(options.maxHeaderBytes, 8 * 1024),
    onRequest: (id, method, params): Promise<unknown> => state.request(peer, id, method, params),
    onNotification: (method, params) => state.notification(method, params),
  });
  state.attach(peer);
  const cleanupSignals = options.installSignalHandlers === false
    ? () => {}
    : installSignalHandlers(state, peer, signalShutdownTimeoutMs);
  try { await peer.run(); }
  finally { cleanupSignals(); }
}

class RunnerState {
  private peer: RunnerRpcPeer | null = null;
  private phase: "spawned" | "handshaken" | "announced" | "initializing" | "initialized" | "disposing" | "disposed" | "failed" = "spawned";
  private active: { requestId: number; source: ReturnType<typeof createCancellationTokenSource> } | null = null;
  private readonly inputs = new Map<number, AsyncQueue<unknown>>();
  private readonly capabilityInputIds = new Set<number>();
  private readonly retiringCapabilityInputIds = new Set<number>();
  private lastConfig: unknown = {};
  private activeCompletion: Promise<void> | null = null;
  private resolveActiveCompletion: (() => void) | null = null;
  private lifecycleCompletion: Promise<void> | null = null;
  private resolveLifecycleCompletion: (() => void) | null = null;
  private shutdownPromise: Promise<void> | null = null;

  constructor(
    private readonly stage: AnyStage,
    private readonly stageId: string,
    private readonly configSchemaHash: string | null,
    private readonly limits: { maxBufferedStreamValues: number; maxBufferedStreamBytes: number },
  ) {}

  attach(peer: RunnerRpcPeer): void { this.peer = peer; }

  async request(peer: RunnerRpcPeer, id: number, method: string, rawParams: unknown): Promise<unknown> {
    const params = asRecord(rawParams ?? {}, `${method} params`);
    switch (method) {
      case "handshake": return this.handshake(params);
      case "announce": return this.announce();
      case "stage.init": return this.init(peer, params);
      case "stage.run": return this.run(peer, id, params);
      case "stage.dispose": {
        const result = await this.dispose(peer);
        peer.requestStopAfterResponse();
        return result;
      }
      default: throw new RpcFault(-32601, "METHOD_NOT_FOUND", { method });
    }
  }

  async notification(method: string, rawParams: unknown): Promise<void> {
    const params = asRecord(rawParams ?? {}, `${method} params`);
    if (method === "$/cancelRequest") {
      if (this.active && params.id === this.active.requestId) {
        this.active.source.cancel(typeof params.reason === "string" ? params.reason : undefined);
      }
      return;
    }
    if (method === "stream.value") {
      const streamId = safeId(params.streamId, "streamId");
      const queue = this.inputQueue(params.streamId);
      if (!queue) return;
      try {
        await queue.push(params.value);
      } catch (error) {
        if (this.retiringCapabilityInputIds.has(streamId)) return;
        throw error;
      }
      return;
    }
    if (method === "stream.end") {
      const queue = this.inputQueue(params.streamId);
      if (!queue) return;
      queue.end();
      return;
    }
    if (method === "stream.error") {
      const queue = this.inputQueue(params.streamId);
      if (!queue) return;
      queue.fail(new StageError({ code: "UPSTREAM_STREAM_ERROR", message: "host input stream failed" }));
      return;
    }
    throw new RpcFault(-32004, "PROTOCOL_VIOLATION", { method });
  }

  cancel(reason: string): void {
    this.active?.source.cancel(reason);
    const error = new CancellationError(reason);
    for (const streamId of this.capabilityInputIds) this.inputs.get(streamId)?.fail(error);
  }

  shutdown(peer: RunnerRpcPeer): Promise<void> {
    if (this.shutdownPromise) return this.shutdownPromise;
    this.shutdownPromise = (async () => {
      this.cancel("plugin process received termination signal");
      for (const queue of this.inputs.values()) queue.end();
      const lifecycle = this.lifecycleCompletion;
      if (lifecycle) await lifecycle;
      const completion = this.activeCompletion;
      if (completion) await completion;
      if (this.phase !== "disposed") await this.dispose(peer);
      peer.shutdown();
    })();
    return this.shutdownPromise;
  }

  private handshake(params: Record<string, unknown>): unknown {
    if (this.phase !== "spawned") throw new RpcFault(-32004, "PROTOCOL_VIOLATION", { phase: this.phase });
    const expected = {
      pluginName: this.stage.name,
      pluginVersion: this.stage.version,
      apiVersion: this.stage.apiVersion,
      protocolVersion: PROTOCOL_VERSION,
    };
    for (const [key, value] of Object.entries(expected)) {
      if (params[key] !== value) throw new RpcFault(-32006, "MANIFEST_MISMATCH", { field: key });
    }
    this.phase = "handshaken";
    return { ...expected, runner: RUNNER_NAME, runnerVersion: RUNNER_VERSION };
  }

  private announce(): unknown {
    if (this.phase !== "handshaken") throw new RpcFault(-32004, "PROTOCOL_VIOLATION", { phase: this.phase });
    this.phase = "announced";
    return {
      stage: {
        id: this.stageId,
        consumes: kindReference(this.stage.consumes),
        produces: kindReference(this.stage.produces),
        capabilities: [...this.stage.capabilities],
        configSchemaHash: this.configSchemaHash,
      },
    };
  }

  private async init(peer: RunnerRpcPeer, params: Record<string, unknown>): Promise<null> {
    if (this.phase !== "announced" && this.phase !== "initialized") {
      throw new RpcFault(-32004, "PROTOCOL_VIOLATION", { phase: this.phase });
    }
    if (this.phase === "initialized") return null;
    this.lastConfig = params.config;
    this.phase = "initializing";
    this.lifecycleCompletion = new Promise(resolve => { this.resolveLifecycleCompletion = resolve; });
    try {
      if (this.stage.init) await this.stage.init(params.config, initContext(peer, params.config));
      this.phase = "initialized";
      return null;
    } catch (error) {
      this.phase = "failed";
      throw error;
    } finally {
      this.resolveLifecycleCompletion?.();
      this.resolveLifecycleCompletion = null;
      this.lifecycleCompletion = null;
    }
  }

  private async run(peer: RunnerRpcPeer, id: number, params: Record<string, unknown>): Promise<unknown> {
    if (this.phase !== "initialized" || this.active) throw new RpcFault(-32004, "PROTOCOL_VIOLATION", { phase: this.phase });
    const streamId = safeId(params.streamId, "streamId");
    const source = createCancellationTokenSource();
    this.active = { requestId: id, source };
    this.activeCompletion = new Promise(resolve => { this.resolveActiveCompletion = resolve; });
    this.lastConfig = params.config;
    let inputStreamId: number | null = null;
    try {
      let input = params.input;
      if (this.stage.consumes.name === "Stream") {
        if (!isCanonicalStreamHandle(input)) throw new RunnerProtocolError("stream stage requires a canonical stream handle");
        inputStreamId = safeId(input.streamId, "input streamId");
        const queue = new AsyncQueue<unknown>(this.limits.maxBufferedStreamValues, this.limits.maxBufferedStreamBytes);
        this.inputs.set(inputStreamId, queue);
        input = queue;
      }
      const context = buildWireContext(
        peer,
        streamId,
        source.token,
        handle => this.openCapabilityStream(peer, streamId, handle),
      );
      const output = await this.stage.run(input as never, params.config, context);
      source.token.throwIfCancelled();
      if (this.stage.produces.name === "Stream") {
        if (!isAsyncIterable(output)) throw new RunnerProtocolError("stream stage returned a non-stream value");
        let produced = 0;
        for await (const value of output) {
          source.token.throwIfCancelled();
          await peer.notify("stream.value", { streamId, value });
          produced += 1;
        }
        return { kind: "stream", streamId, produced };
      }
      if (isAsyncIterable(output)) throw new RunnerProtocolError("single stage returned a stream value");
      return { kind: "single", value: output };
    } catch (error) {
      throw stageFault(error);
    } finally {
      await this.closeCapabilityStreams(peer, streamId);
      if (inputStreamId !== null) {
        this.inputs.get(inputStreamId)?.end();
        this.inputs.delete(inputStreamId);
      }
      this.active = null;
      this.resolveActiveCompletion?.();
      this.resolveActiveCompletion = null;
      this.activeCompletion = null;
    }
  }

  private async dispose(peer: RunnerRpcPeer): Promise<null> {
    if (this.phase === "disposed") return null;
    if (this.phase === "initializing" || this.phase === "disposing" || this.active) {
      throw new RpcFault(-32004, "PROTOCOL_VIOLATION", { phase: this.phase });
    }
    const initialized = this.phase === "initialized";
    this.phase = "disposing";
    this.lifecycleCompletion = new Promise(resolve => { this.resolveLifecycleCompletion = resolve; });
    for (const queue of this.inputs.values()) queue.end();
    this.inputs.clear();
    try {
      if (this.stage.dispose && initialized) {
        await this.stage.dispose(initContext(peer, this.lastConfig));
      }
      this.phase = "disposed";
      return null;
    } catch (error) {
      this.phase = "failed";
      throw error;
    } finally {
      this.resolveLifecycleCompletion?.();
      this.resolveLifecycleCompletion = null;
      this.lifecycleCompletion = null;
    }
  }

  private inputQueue(rawId: unknown): AsyncQueue<unknown> | null {
    const id = safeId(rawId, "streamId");
    if (this.retiringCapabilityInputIds.has(id)) return null;
    const queue = this.inputs.get(id);
    if (!queue) throw new RpcFault(-32004, "PROTOCOL_VIOLATION", { streamId: id });
    return queue;
  }

  private openCapabilityStream(peer: RunnerRpcPeer, ownerRunId: number, handle: unknown): AsyncIterable<unknown> {
    if (!isCanonicalStreamHandle(handle)) throw new RunnerProtocolError("storage.watch result is malformed");
    const streamId = safeId(handle.streamId, "capability streamId");
    if (streamId === 0 || this.inputs.has(streamId)) {
      throw new RunnerProtocolError("storage.watch returned a duplicate stream handle");
    }
    const queue = new AsyncQueue<unknown>(this.limits.maxBufferedStreamValues, this.limits.maxBufferedStreamBytes);
    this.inputs.set(streamId, queue);
    this.capabilityInputIds.add(streamId);
    const state = this;
    return {
      async *[Symbol.asyncIterator]() {
        let started = false;
        try {
          await peer.notify("stream.start", { streamId });
          started = true;
          for await (const value of queue) yield value;
        } finally {
          await state.closeCapabilityStream(peer, ownerRunId, streamId, started);
        }
      },
    };
  }

  private async closeCapabilityStream(
    peer: RunnerRpcPeer,
    ownerRunId: number,
    streamId: number,
    notify: boolean,
  ): Promise<void> {
    if (!this.capabilityInputIds.delete(streamId)) return;
    this.retiringCapabilityInputIds.add(streamId);
    this.inputs.get(streamId)?.end();
    this.inputs.delete(streamId);
    try {
      if (notify) await peer.request("stream.cancel", {
        streamId: ownerRunId,
        capabilityStreamId: streamId,
      });
      this.retiringCapabilityInputIds.delete(streamId);
    } catch (error) {
      peer.fail(new RunnerProtocolError("host did not acknowledge capability stream cancellation"));
      throw error;
    }
  }

  private async closeCapabilityStreams(peer: RunnerRpcPeer, ownerRunId: number): Promise<void> {
    await Promise.all([...this.capabilityInputIds].map(streamId =>
      this.closeCapabilityStream(peer, ownerRunId, streamId, true)));
  }
}

class AsyncQueue<T> implements AsyncIterable<T>, AsyncIterator<T> {
  private readonly values: Array<{ value: T; bytes: number }> = [];
  private readonly readers: Array<{ resolve(result: IteratorResult<T>): void; reject(error: unknown): void }> = [];
  private readonly writers: Array<() => void> = [];
  private retainedBytes = 0;
  private done = false;
  private error: unknown = null;

  constructor(private readonly maxValues: number, private readonly maxBytes: number) {}

  async push(value: T): Promise<void> {
    if (this.done) throw new RunnerProtocolError("stream.value targets a completed stream");
    const bytes = estimateBytes(value, this.maxBytes);
    while (!this.done && (this.values.length >= this.maxValues || this.retainedBytes + bytes > this.maxBytes)) {
      await new Promise<void>(resolve => this.writers.push(resolve));
    }
    if (this.done) throw new RunnerProtocolError("stream.value targets a completed stream");
    const reader = this.readers.shift();
    if (reader) reader.resolve({ value, done: false });
    else { this.values.push({ value, bytes }); this.retainedBytes += bytes; }
  }

  end(): void {
    if (this.done) return;
    this.done = true;
    for (const reader of this.readers.splice(0)) reader.resolve({ value: undefined, done: true });
    for (const writer of this.writers.splice(0)) writer();
  }

  fail(error: unknown): void {
    if (this.done) return;
    this.error = error;
    this.done = true;
    for (const reader of this.readers.splice(0)) reader.reject(error);
    for (const writer of this.writers.splice(0)) writer();
  }

  async next(): Promise<IteratorResult<T>> {
    const entry = this.values.shift();
    if (entry) {
      this.retainedBytes -= entry.bytes;
      this.writers.shift()?.();
      return { value: entry.value, done: false };
    }
    if (this.error) throw this.error;
    if (this.done) return { value: undefined, done: true };
    return new Promise<IteratorResult<T>>((resolve, reject) => this.readers.push({ resolve, reject }));
  }

  async return(): Promise<IteratorResult<T>> {
    this.values.length = 0;
    this.retainedBytes = 0;
    this.end();
    return { value: undefined, done: true };
  }

  [Symbol.asyncIterator](): AsyncIterator<T> { return this; }
}

class RpcFault extends Error {
  constructor(readonly rpcCode: number, message: string, readonly data?: unknown) {
    super(message);
    this.name = "RpcFault";
  }
}

function initContext(peer: RunnerRpcPeer, config: unknown): StageInitContext {
  const { cancellation: _cancellation, cache: _cache, ...context } = buildWireContext(peer, 0, neverCancelledToken());
  return { ...context, config };
}

function stageFault(error: unknown): RpcFault {
  if (isCancellationError(error)) return new RpcFault(-32800, error.message, {});
  if (error instanceof StageError) {
    return new RpcFault(-32900, error.message, {
      stageErrorCode: error.code,
      inputPath: error.inputPath,
      inputId: error.inputId,
      stageName: error.stageName,
      recoverable: error.recoverable,
      fields: error.fields,
    });
  }
  if (error instanceof RunnerProtocolError) return new RpcFault(-32004, "PROTOCOL_VIOLATION", {});
  return new RpcFault(-32603, "Plugin stage failed", {});
}

function kindReference(descriptor: KindDescriptor): string {
  if (descriptor.name !== "Stream") return descriptor.name;
  if (!descriptor.inner || descriptor.inner.name === "Stream") throw new RunnerProtocolError("stream descriptor requires one non-stream inner kind");
  return `Stream<${descriptor.inner.name}>`;
}

function validateStage(stage: AnyStage): void {
  if (!isToken(stage.name, 256) || !isToken(stage.version, 128) || !isToken(stage.description, 16_384)) {
    throw new RunnerProtocolError("stage metadata is invalid");
  }
  if (stage.apiVersion !== KERNEL_API_VERSION) throw new RunnerProtocolError("stage apiVersion is unsupported");
  if (!Array.isArray(stage.capabilities) || stage.capabilities.length > 256
      || stage.capabilities.some(value => !isToken(value, 512))) {
    throw new RunnerProtocolError("stage capabilities are invalid");
  }
  kindReference(stage.consumes);
  kindReference(stage.produces);
}

function installSignalHandlers(state: RunnerState, peer: RunnerRpcPeer, timeoutMs: number): () => void {
  let stopping = false;
  let shutdownTimer: ReturnType<typeof setTimeout> | null = null;
  const onSignal = () => {
    if (stopping) return;
    stopping = true;
    shutdownTimer = setTimeout(() => peer.shutdown(), timeoutMs);
    shutdownTimer.unref();
    void state.shutdown(peer)
      .catch(() => peer.shutdown())
      .finally(() => {
        if (shutdownTimer) clearTimeout(shutdownTimer);
        shutdownTimer = null;
      });
  };
  process.on("SIGINT", onSignal);
  process.on("SIGTERM", onSignal);
  return () => {
    process.off("SIGINT", onSignal);
    process.off("SIGTERM", onSignal);
    if (shutdownTimer) clearTimeout(shutdownTimer);
  };
}

function localStageId(name: string): string {
  const index = name.lastIndexOf("/");
  return index >= 0 ? name.slice(index + 1) : name;
}

function isToken(value: unknown, maxLength: number): value is string {
  return typeof value === "string" && value.length > 0 && value.length <= maxLength && !value.includes("\0");
}

function safeId(value: unknown, label: string): number {
  if (!Number.isSafeInteger(value) || (value as number) < 0) throw new RpcFault(-32602, "INVALID_PARAMS", { field: label });
  return value as number;
}

function isCanonicalStreamHandle(value: unknown): value is { kind: "stream-handle"; streamId: number } {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    && Object.keys(value).length === 2
    && (value as { kind?: unknown }).kind === "stream-handle"
    && Object.prototype.hasOwnProperty.call(value, "streamId");
}

function isAsyncIterable(value: unknown): value is AsyncIterable<unknown> {
  return typeof value === "object" && value !== null && Symbol.asyncIterator in value;
}

function estimateBytes(value: unknown, limit: number): number {
  let bytes: number;
  try { bytes = Buffer.byteLength(JSON.stringify(encodeWireValue(value))); }
  catch { throw new RunnerProtocolError("stream value is not JSON serializable"); }
  if (bytes > limit) throw new RunnerProtocolError("stream value exceeds configured bound");
  return bytes;
}

function positive(value: number | undefined, fallback: number): number {
  const resolved = value ?? fallback;
  if (!Number.isSafeInteger(resolved) || resolved <= 0) throw new RangeError("runner limit must be a positive safe integer");
  return resolved;
}
