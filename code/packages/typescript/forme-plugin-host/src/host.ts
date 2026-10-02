import { createHash } from "node:crypto";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { matchesCapability, type Capability } from "@coding-adventures/forme-capability";
import { CapabilityError, CancellationError, StageError } from "@coding-adventures/forme-errors";
import {
  computeManifestHash,
  resolveCapabilityTemplate,
  type Manifest,
  type StageContribution,
} from "@coding-adventures/forme-manifest";
import type { StageRef } from "@coding-adventures/forme-pipeline-config";
import {
  silentLogger,
  type Logger,
  type Stage,
  type StageContext,
  type StageInitContext,
} from "@coding-adventures/forme-stage";
import {
  KERNEL_API_VERSION,
  type JsonValue,
  type KindDescriptor,
} from "@coding-adventures/forme-types";
import { mediateCapabilityRequest } from "./capability-mediator.js";
import { discoverPlugins } from "./discovery.js";
import { PluginHostError, RpcFault, RpcRemoteError } from "./errors.js";
import { descriptorForKindReference } from "./kinds.js";
import type {
  DiscoveredPlugin,
  LaunchedPluginProcess,
  PluginHost,
  PluginHostOptions,
  VerifiedConfigSchemaSnapshot,
} from "./types.js";
import { RpcPeer, asRecord } from "./wire.js";
import { readGrantsFile } from "./persistent-authority.js";

export const FORME_PLUGIN_PROTOCOL_VERSION = 1 as const;
const MAX_CAPABILITY_STREAMS = 64;

interface ResolvedOptions {
  readonly processFactory: PluginHostOptions["processFactory"];
  readonly grants: Readonly<Record<string, readonly Capability[]>>;
  readonly logger: Logger;
  readonly capabilityApis: NonNullable<PluginHostOptions["capabilityApis"]>;
  readonly storageRoot?: string;
  readonly cacheDirectory?: string | null;
  readonly hostName: string;
  readonly hostVersion: string;
  readonly handshakeTimeoutMs: number;
  readonly requestTimeoutMs: number;
  readonly cancellationGracePeriodMs: number;
  readonly disposeGracePeriodMs: number;
  readonly killGracePeriodMs: number;
  readonly maxFrameBytes: number;
  readonly maxHeaderBytes: number;
  readonly maxBufferedStreamValues: number;
  readonly maxBufferedStreamBytes: number;
  readonly maxLogEntries: number;
  readonly maxLogBytes: number;
}

interface CapabilityStream {
  readonly iterator: AsyncIterator<unknown>;
  started: boolean;
  cancelled: boolean;
  completed: boolean;
  pump: Promise<void> | null;
}

class PluginHostImpl implements PluginHost {
  private readonly sessions = new Set<PluginSession>();
  private readonly internalPlugins: ReadonlyMap<string, DiscoveredPlugin>;
  readonly plugins: ReadonlyMap<string, DiscoveredPlugin>;
  private disposed = false;

  constructor(
    plugins: ReadonlyMap<string, DiscoveredPlugin>,
    private readonly options: ResolvedOptions,
  ) {
    // `createPluginHost` owns the freshly discovered map exclusively. Retain
    // it as the private verified snapshot and make only one defensive public
    // copy, avoiding a third aggregate copy of all plugin entry bytes.
    this.internalPlugins = plugins;
    this.plugins = clonePluginMap(this.internalPlugins);
  }

  async loadStage(
    ref: StageRef,
    instanceId?: string,
    instanceCapabilities?: readonly Capability[],
  ): Promise<Stage<KindDescriptor, KindDescriptor>> {
    if (this.disposed) throw new Error("PluginHost is disposed");
    const plugin = this.internalPlugins.get(ref.packageName);
    if (!plugin) {
      throw new PluginHostError("PLUGIN_NOT_FOUND", `plugin ${JSON.stringify(ref.packageName)} was not discovered`);
    }
    const stageId = selectStageId(plugin.manifest, ref.export);
    const contribution = plugin.manifest.contributes.stages.find(stage => stage.id === stageId);
    if (!contribution) {
      throw new PluginHostError("STAGE_NOT_FOUND", `plugin ${ref.packageName} does not export ${stageId}`);
    }
    const resolvedInstanceId = instanceId ?? `${plugin.manifest.plugin.name}/${contribution.id}`;
    if (resolvedInstanceId.length === 0) {
      throw new PluginHostError(
        "MANIFEST_INVALID",
        "instanceId must be a non-empty string",
        { instanceId: resolvedInstanceId },
      );
    }
    if (computeManifestHash(plugin.manifest, plugin.entryBytes) !== plugin.manifestHash) {
      throw new PluginHostError("MANIFEST_MISMATCH", "verified plugin snapshot was mutated");
    }
    const declared = resolveDeclaredCapabilities(plugin, this.options);
    const supplied = Object.hasOwn(this.options.grants, plugin.manifest.plugin.name)
      ? this.options.grants[plugin.manifest.plugin.name] ?? []
      : [];
    const requested = instanceCapabilities ?? declared.all;
    const effective = declared.all.filter(capability =>
      requested.some(grant => matchesCapability(grant, capability))
      && supplied.some(grant => matchesCapability(grant, capability)));
    for (const required of declared.required) {
      if (!effective.includes(required)) {
        throw new PluginHostError(
          "REQUIRED_CAPABILITY_DENIED",
          `required capability ${required} is not granted`,
          { plugin: plugin.manifest.plugin.name, capability: required },
        );
      }
    }
    const configSchemaSnapshot = loadConfigSchema(plugin, contribution);
    const metadata = {
      consumes: descriptorForKindReference(contribution.consumes, plugin.manifest),
      produces: descriptorForKindReference(contribution.produces, plugin.manifest),
      configSchema: configSchemaSnapshot?.schema ?? null,
      configSchemaHash: configSchemaSnapshot?.hash ?? null,
    };
    let session: PluginSession | null = null;
    const getSession = (): PluginSession => {
      if (this.disposed) throw new Error("PluginHost is disposed");
      if (session?.reusable) return session;
      if (session) this.sessions.delete(session);
      session = new PluginSession(
        plugin,
        contribution,
        resolvedInstanceId,
        effective,
        metadata.configSchemaHash,
        configSchemaSnapshot === null ? null : {
          relativePath: configSchemaSnapshot.relativePath,
          bytes: Uint8Array.from(configSchemaSnapshot.bytes),
          hash: configSchemaSnapshot.hash,
        },
        this.options,
      );
      this.sessions.add(session);
      return session;
    };

    const stage: Stage<KindDescriptor, KindDescriptor> = {
      name: `${plugin.manifest.plugin.name}/${contribution.id}`,
      version: plugin.manifest.plugin.version,
      implementationIdentity: stageImplementationIdentity(
        plugin.manifestHash,
        metadata.configSchemaHash,
      ),
      apiVersion: plugin.manifest.plugin.apiVersion,
      description: plugin.manifest.plugin.description ?? `Plugin stage ${contribution.id}`,
      consumes: metadata.consumes,
      produces: metadata.produces,
      capabilities: Object.freeze([...effective]),
      configSchema: metadata.configSchema,
      init: (config, context) => getSession().init(config, context),
      run: (input, config, context) => metadata.produces.name === "Stream"
        ? getSession().runStream(input, config, context)
        : getSession().runSingle(input, config, context),
      dispose: async () => {
        if (!session) return;
        const disposing = session;
        session = null;
        await disposing.dispose();
        this.sessions.delete(disposing);
      },
    };
    return Object.freeze(stage);
  }

  async dispose(): Promise<void> {
    if (this.disposed) return;
    this.disposed = true;
    const results = await Promise.allSettled([...this.sessions].map(session => session.dispose()));
    this.sessions.clear();
    const failures = results.filter((result): result is PromiseRejectedResult => result.status === "rejected");
    if (failures.length > 0) throw new AggregateError(failures.map(result => result.reason));
  }
}

export async function createPluginHost(options: PluginHostOptions): Promise<PluginHost> {
  options.signal?.throwIfAborted();
  if (options.roots.length === 0) {
    throw new PluginHostError("PLUGIN_NOT_FOUND", "at least one discovery root is required");
  }
  if (options.loadPersistentGrants === true && options.grants !== undefined) {
    throw new TypeError("loadPersistentGrants and grants are mutually exclusive");
  }
  const plugins = await discoverPlugins(options.roots, options.signal);
  options.signal?.throwIfAborted();
  const grants = options.loadPersistentGrants === true
    ? await loadInstalledGrants(plugins, options.signal)
    : options.grants ?? {};
  const resolved: ResolvedOptions = {
    processFactory: options.processFactory,
    grants,
    logger: options.logger ?? silentLogger(),
    capabilityApis: options.capabilityApis ?? {},
    storageRoot: options.storageRoot,
    cacheDirectory: options.cacheDirectory,
    hostName: options.hostName ?? "forme-orchestrator",
    hostVersion: options.hostVersion ?? "0.1.0",
    handshakeTimeoutMs: positive(options.handshakeTimeoutMs, 5_000),
    requestTimeoutMs: positive(options.requestTimeoutMs, 30_000),
    cancellationGracePeriodMs: positive(options.cancellationGracePeriodMs, 5_000),
    disposeGracePeriodMs: positive(options.disposeGracePeriodMs, 2_000),
    killGracePeriodMs: positive(options.killGracePeriodMs, 2_000),
    maxFrameBytes: positive(options.maxFrameBytes, 8 * 1024 * 1024),
    maxHeaderBytes: positive(options.maxHeaderBytes, 8 * 1024),
    maxBufferedStreamValues: positive(options.maxBufferedStreamValues, 64),
    maxBufferedStreamBytes: positive(options.maxBufferedStreamBytes, 8 * 1024 * 1024),
    maxLogEntries: positive(options.maxLogEntries, 1_024),
    maxLogBytes: positive(options.maxLogBytes, 256 * 1024),
  };
  return new PluginHostImpl(plugins, resolved);
}

async function loadInstalledGrants(
  plugins: ReadonlyMap<string, DiscoveredPlugin>,
  signal?: AbortSignal,
): Promise<Readonly<Record<string, readonly Capability[]>>> {
  const grants: Record<string, readonly Capability[]> = Object.create(null) as Record<string, readonly Capability[]>;
  for (const [name, plugin] of plugins) {
    signal?.throwIfAborted();
    const loaded = await readGrantsFile(join(plugin.rootDirectory, "grants.toml"), plugin.manifestHash);
    signal?.throwIfAborted();
    grants[name] = loaded.capabilities;
  }
  return Object.freeze(grants);
}

class PluginSession {
  private process: LaunchedPluginProcess | null = null;
  private peer: RpcPeer | null = null;
  private workingDirectory: string | null = null;
  private startPromise: Promise<void> | null = null;
  private initialized = false;
  private disposing = false;
  private disposed = false;
  private disposePromise: Promise<void> | null = null;
  private fault: Error | null = null;
  private activeContext: StageContext | null = null;
  private activeRunId: number | null = null;
  private readonly outputStreams = new Map<number, AsyncQueue<unknown>>();
  private readonly capabilityStreams = new Map<number, CapabilityStream>();
  private nextStreamId = 1;
  private logEntries = 0;
  private logBytes = 0;
  private readonly mutex = new AsyncMutex();

  constructor(
    private readonly plugin: DiscoveredPlugin,
    private readonly stage: StageContribution,
    private readonly instanceId: string,
    private readonly grants: readonly Capability[],
    private readonly configSchemaHash: string | null,
    private readonly configSchemaSnapshot: VerifiedConfigSchemaSnapshot | null,
    private readonly options: ResolvedOptions,
  ) {}

  get reusable(): boolean { return !this.disposed; }

  async init(config: unknown, _context: StageInitContext): Promise<void> {
    const release = await this.mutex.acquire();
    try {
      await this.ensureStarted();
      if (this.initialized) return;
      await this.request("stage.init", {
        config,
        instanceId: this.instanceId,
        logLevel: "info",
      });
      this.initialized = true;
    } catch (error) {
      await this.dispose();
      throw error;
    } finally {
      release();
    }
  }

  async runSingle(input: unknown, config: unknown, context: StageContext): Promise<unknown> {
    const release = await this.mutex.acquire();
    let failed = false;
    let clearCancellation = () => {};
    let inputIterator: AsyncIterator<unknown> | null = null;
    let stopPump = false;
    let inputCompleted = true;
    let pump: Promise<void> = Promise.resolve();
    try {
      await this.ensureInitialized(config);
      context.cancellation.throwIfCancelled();
      this.activeContext = context;
      const runId = this.allocateStreamId();
      this.activeRunId = runId;
      inputIterator = isAsyncIterable(input) ? input[Symbol.asyncIterator]() : null;
      const inputStreamId = inputIterator ? this.allocateStreamId() : null;
      const wireInput = inputIterator ? { kind: "stream-handle", streamId: inputStreamId } : input;
      const pending = this.peer!.beginRequest("stage.run", { input: wireInput, config, streamId: runId });
      clearCancellation = this.watchCancellation(context, pending.id);
      if (inputIterator) {
        inputCompleted = false;
        pump = this.pumpInput(inputIterator, inputStreamId!, () => stopPump)
          .then(() => { inputCompleted = true; });
      }
      const pumpFailure = pump.then(
        () => new Promise<never>(() => {}),
        error => Promise.reject(error),
      );
      const result = asRecord(await Promise.race([pending.response, pumpFailure]), "stage.run result");
      context.cancellation.throwIfCancelled();
      if (result.kind !== "single" || !Object.prototype.hasOwnProperty.call(result, "value")) {
        throw new PluginHostError("PROTOCOL_VIOLATION", "single-output stage returned a non-single result");
      }
      return result.value;
    } catch (error) {
      failed = true;
      throw translateRemoteError(error);
    } finally {
      clearCancellation();
      stopPump = true;
      if (inputIterator?.return) {
        await settlesWithin(
          Promise.resolve().then(() => inputIterator!.return!()),
          this.options.killGracePeriodMs,
        );
      }
      await this.closeCapabilityStreams();
      this.activeContext = null;
      this.activeRunId = null;
      if (failed || !inputCompleted || context.cancellation.cancelled) await this.dispose();
      await settlesWithin(pump, this.options.killGracePeriodMs);
      release();
    }
  }

  async *runStream(input: unknown, config: unknown, context: StageContext): AsyncIterable<unknown> {
    const release = await this.mutex.acquire();
    const outputStreamId = this.allocateStreamId();
    const queue = new AsyncQueue<unknown>(
      this.options.maxBufferedStreamValues,
      this.options.maxBufferedStreamBytes,
      value => estimateRetainedBytes(value, this.options.maxBufferedStreamBytes),
    );
    this.outputStreams.set(outputStreamId, queue);
    let pendingId: number | null = null;
    let clearCancellation = () => {};
    let inputIterator: AsyncIterator<unknown> | null = null;
    let stopPump = false;
    let pump: Promise<void> = Promise.resolve();
    let completedSuccessfully = false;
    try {
      await this.ensureInitialized(config);
      context.cancellation.throwIfCancelled();
      this.activeContext = context;
      const inputIsStream = isAsyncIterable(input);
      inputIterator = inputIsStream ? input[Symbol.asyncIterator]() : null;
      const inputStreamId = inputIterator ? this.allocateStreamId() : null;
      const wireInput = inputIsStream ? { kind: "stream-handle", streamId: inputStreamId } : input;
      const pending = this.peer!.beginRequest("stage.run", {
        input: wireInput,
        config,
        streamId: outputStreamId,
      });
      pendingId = pending.id;
      this.activeRunId = outputStreamId;
      clearCancellation = this.watchCancellation(context, pending.id);
      pump = inputIterator
        ? this.pumpInput(inputIterator, inputStreamId!, () => stopPump)
        : Promise.resolve();
      void Promise.all([pump, pending.response]).then(([, raw]) => {
        const result = asRecord(raw, "stage.run stream result");
        if (result.kind !== "stream" || result.streamId !== outputStreamId
            || !Number.isSafeInteger(result.produced)
            || result.produced !== queue.count) {
          throw new PluginHostError("PROTOCOL_VIOLATION", "stream completion metadata does not match delivered values");
        }
        queue.close();
      }).catch(error => queue.fail(translateRemoteError(error)));
      for await (const value of queue) yield value;
      context.cancellation.throwIfCancelled();
      completedSuccessfully = true;
    } finally {
      clearCancellation();
      const abandoned = !queue.completed;
      stopPump = true;
      if (inputIterator?.return) {
        await settlesWithin(
          Promise.resolve().then(() => inputIterator!.return!()),
          this.options.killGracePeriodMs,
        );
      }
      await this.closeCapabilityStreams();
      this.outputStreams.delete(outputStreamId);
      this.activeContext = null;
      this.activeRunId = null;
      if (pendingId !== null && abandoned) {
        void this.peer?.notify("$/cancelRequest", { id: pendingId }).catch(() => undefined);
      }
      if (abandoned || !completedSuccessfully || context.cancellation.cancelled) await this.dispose();
      await settlesWithin(pump, this.options.killGracePeriodMs);
      release();
    }
  }

  async dispose(): Promise<void> {
    this.disposePromise ??= this.disposeInternal();
    return this.disposePromise;
  }

  private async disposeInternal(): Promise<void> {
    if (this.disposed) return;
    this.disposing = true;
    try {
      if (this.startPromise) {
        await this.startPromise.catch(() => undefined);
      }
      if (this.peer && this.process && !this.fault) {
        await this.peer.request("stage.dispose", {}, this.options.disposeGracePeriodMs)
          .catch(() => undefined);
        if (!await settlesWithin(this.process.exited, this.options.disposeGracePeriodMs)) {
          this.process.signal("SIGTERM");
          if (!await settlesWithin(this.process.exited, this.options.killGracePeriodMs)) {
            this.process.signal("SIGKILL");
            await settlesWithin(this.process.exited, this.options.killGracePeriodMs);
          }
        }
      } else if (this.process) {
        this.process.signal("SIGKILL");
        await settlesWithin(this.process.exited, this.options.killGracePeriodMs);
      }
    } finally {
      await this.closeCapabilityStreams();
      this.disposed = true;
      this.disposing = false;
      this.peer?.fail(new Error("plugin session disposed"));
      this.process?.stdin.destroy();
      this.process?.stdout.destroy();
      this.process?.stderr.destroy();
      const cleanupTasks: Promise<unknown>[] = [];
      if (this.process?.cleanup) cleanupTasks.push(Promise.resolve().then(() => this.process!.cleanup!()));
      if (this.workingDirectory) {
        cleanupTasks.push(rm(this.workingDirectory, { recursive: true, force: true }));
      }
      const cleanup = await Promise.allSettled(cleanupTasks);
      const failures = cleanup.filter((result): result is PromiseRejectedResult => result.status === "rejected");
      if (failures.length > 0) throw new AggregateError(failures.map(result => result.reason));
    }
  }

  private async ensureInitialized(config: unknown): Promise<void> {
    await this.ensureStarted();
    if (!this.initialized) {
      await this.request("stage.init", { config, instanceId: this.instanceId, logLevel: "info" });
      this.initialized = true;
    }
  }

  private async ensureStarted(): Promise<void> {
    if (this.disposed) throw new Error("plugin session is disposed");
    if (this.fault) throw this.fault;
    this.startPromise ??= this.start().catch(error => {
      const fatal = error instanceof Error ? error : new Error(String(error));
      this.fatal(fatal);
      throw fatal;
    });
    return this.startPromise;
  }

  private async start(): Promise<void> {
    if (!this.options.processFactory) {
      throw new PluginHostError("SANDBOX_UNAVAILABLE", "no sandboxed plugin process factory is configured");
    }
    if (computeManifestHash(this.plugin.manifest, this.plugin.entryBytes) !== this.plugin.manifestHash) {
      throw new PluginHostError("MANIFEST_MISMATCH", "verified plugin snapshot was mutated");
    }
    const entryBytes = Uint8Array.from(this.plugin.entryBytes);
    const manifest = structuredClone(this.plugin.manifest);
    this.workingDirectory = await mkdtemp(join(tmpdir(), "forme-plugin-"));
    const launched = await this.options.processFactory.launch({
      plugin: {
        manifest,
        manifestHash: this.plugin.manifestHash,
        entryBytes,
      },
      stage: this.stage,
      instanceId: this.instanceId,
      workingDirectory: this.workingDirectory,
      resources: structuredClone(this.plugin.manifest.resources),
      configSchema: this.configSchemaSnapshot === null ? null : {
        relativePath: this.configSchemaSnapshot.relativePath,
        bytes: Uint8Array.from(this.configSchemaSnapshot.bytes),
        hash: this.configSchemaSnapshot.hash,
      },
    });
    this.process = launched;
    if (launched.isolation !== "sandboxed" || launched.isolationProvider.length === 0
        || launched.launchedManifestHash !== this.plugin.manifestHash
        || launched.launchedConfigSchemaHash !== this.configSchemaHash) {
      launched.signal("SIGKILL");
      throw new PluginHostError(
        "SANDBOX_UNAVAILABLE",
        "launcher did not attest the isolation boundary and exact verified plugin identity",
      );
    }
    this.peer = new RpcPeer({
      input: launched.stdin,
      output: launched.stdout,
      requestTimeoutMs: this.options.requestTimeoutMs,
      maxFrameBytes: this.options.maxFrameBytes,
      maxHeaderBytes: this.options.maxHeaderBytes,
      onRequest: (method, params) => {
        const request = asRecord(params, `${method} params`);
        if (!Number.isSafeInteger(request.streamId)
            || request.streamId !== this.activeRunId) {
          throw new RpcFault(-32001, "STALE_RUN_AUTHORITY", {
            expectedStreamId: this.activeRunId,
          });
        }
        return mediateCapabilityRequest(
          method,
          request,
          capabilityContext(this.activeContext, this.options.capabilityApis),
          this.grants,
          source => this.openCapabilityStream(source),
        );
      },
      onNotification: (method, params) => this.onNotification(method, params),
      onFatal: error => this.fatal(error),
    });
    this.peer.start();
    void this.readStderr(launched);
    void launched.exited.then(exit => {
      if (this.disposing || this.disposed) return;
      const error = new PluginHostError("PLUGIN_CRASHED", "plugin process exited unexpectedly", {
        code: exit.code,
        signal: exit.signal,
        plugin: this.plugin.manifest.plugin.name,
        instanceId: this.instanceId,
      });
      this.fatal(error, false);
    });

    const handshake = asRecord(await this.peer.request("handshake", {
      hostName: this.options.hostName,
      hostVersion: this.options.hostVersion,
      apiVersion: KERNEL_API_VERSION,
      protocolVersion: FORME_PLUGIN_PROTOCOL_VERSION,
      pluginName: this.plugin.manifest.plugin.name,
      pluginVersion: this.plugin.manifest.plugin.version,
      manifestHash: this.plugin.manifestHash,
      instanceId: this.instanceId,
      trustTier: "unverified-third-party",
    }, this.options.handshakeTimeoutMs), "handshake result");
    const expectedHandshake = {
      pluginName: this.plugin.manifest.plugin.name,
      pluginVersion: this.plugin.manifest.plugin.version,
      apiVersion: this.plugin.manifest.plugin.apiVersion,
      protocolVersion: FORME_PLUGIN_PROTOCOL_VERSION,
    };
    for (const [key, expected] of Object.entries(expectedHandshake)) {
      if (handshake[key] !== expected) {
        throw new PluginHostError("MANIFEST_MISMATCH", `handshake ${key} does not match the manifest`);
      }
    }
    if (typeof handshake.runner !== "string" || typeof handshake.runnerVersion !== "string") {
      throw new PluginHostError("PROTOCOL_VIOLATION", "handshake must identify the runner and version");
    }

    const announcement = asRecord(await this.peer.request("announce", {}), "announce result");
    const stage = asRecord(announcement.stage, "announced stage");
    const expectedCapabilities = declaredCapabilityStrings(this.plugin.manifest);
    if (stage.id !== this.stage.id
        || stage.consumes !== this.stage.consumes
        || stage.produces !== this.stage.produces
        || stage.configSchemaHash !== this.configSchemaHash
        || JSON.stringify(stage.capabilities) !== JSON.stringify(expectedCapabilities)) {
      throw new PluginHostError("MANIFEST_MISMATCH", "runtime announcement differs from plugin.toml");
    }
  }

  private async pumpInput(
    iterator: AsyncIterator<unknown>,
    streamId: number,
    stopped: () => boolean,
  ): Promise<void> {
    while (!stopped()) {
      const item = await iterator.next();
      if (item.done || stopped()) break;
      const value = item.value;
      await this.peer!.notify("stream.value", { streamId, value });
    }
    if (!stopped()) await this.peer!.notify("stream.end", { streamId });
  }

  private async request(method: string, params: unknown): Promise<unknown> {
    try {
      return await this.peer!.request(method, params);
    } catch (error) {
      throw translateRemoteError(error);
    }
  }

  private async onNotification(method: string, rawParams: unknown): Promise<void> {
    const params = asRecord(rawParams ?? {}, `${method} params`);
    if (method === "stream.value") {
      if (!Number.isSafeInteger(params.streamId)) {
        throw new PluginHostError("PROTOCOL_VIOLATION", "stream.value requires a safe streamId");
      }
      const queue = this.outputStreams.get(params.streamId as number);
      if (!queue) throw new PluginHostError("PROTOCOL_VIOLATION", "stream.value targets an inactive stream");
      queue.push(params.value);
      return;
    }
    if (method === "stream.start" || method === "stream.cancel") {
      if (!Number.isSafeInteger(params.streamId)) {
        throw new PluginHostError("PROTOCOL_VIOLATION", `${method} requires a safe streamId`);
      }
      const streamId = params.streamId as number;
      const stream = this.capabilityStreams.get(streamId);
      if (!stream) {
        throw new PluginHostError("PROTOCOL_VIOLATION", `${method} targets an inactive capability stream`);
      }
      if (method === "stream.start") {
        if (stream.started) {
          throw new PluginHostError("PROTOCOL_VIOLATION", "capability stream was started more than once");
        }
        stream.started = true;
        stream.pump = this.pumpCapabilityStream(streamId, stream);
      } else {
        await this.closeCapabilityStream(streamId, stream);
      }
      return;
    }
    if (method === "log") {
      const level = params.level;
      const message = params.message;
      if (!isLogLevel(level) || typeof message !== "string" || message.length > 16_384) {
        throw new PluginHostError("PROTOCOL_VIOLATION", "malformed plugin log notification");
      }
      const fields = isJsonRecord(params.fields) ? params.fields : {};
      const bytes = estimateRetainedBytes(params, this.options.maxLogBytes);
      if (this.logEntries + 1 > this.options.maxLogEntries
          || this.logBytes + bytes > this.options.maxLogBytes) {
        throw new PluginHostError("RESOURCE_LIMIT_EXCEEDED", "plugin exceeded its lifetime log budget");
      }
      this.logEntries += 1;
      this.logBytes += bytes;
      this.options.logger[level](message, {
        ...fields,
        source: "plugin",
        plugin: this.plugin.manifest.plugin.name,
        instance: this.instanceId,
      });
      return;
    }
    throw new PluginHostError("PROTOCOL_VIOLATION", `unknown plugin notification ${method}`);
  }

  private watchCancellation(context: StageContext, requestId: number): () => void {
    let active = true;
    let termTimer: ReturnType<typeof setTimeout> | null = null;
    const onAbort = () => {
      if (!active) return;
      void this.peer?.notify("$/cancelRequest", { id: requestId }).catch(() => undefined);
      termTimer = setTimeout(() => {
        if (!active) return;
        const error = new CancellationError(context.cancellation.reason ?? undefined);
        this.peer?.fail(error);
        this.process?.signal("SIGTERM");
        const process = this.process;
        if (process) {
          void settlesWithin(process.exited, this.options.killGracePeriodMs).then(exited => {
            if (!exited) process.signal("SIGKILL");
          });
        }
      }, this.options.cancellationGracePeriodMs);
    };
    context.cancellation.signal.addEventListener("abort", onAbort, { once: true });
    if (context.cancellation.signal.aborted) onAbort();
    return () => {
      active = false;
      context.cancellation.signal.removeEventListener("abort", onAbort);
      if (termTimer) clearTimeout(termTimer);
    };
  }

  private fatal(error: Error, kill = true): void {
    if (this.fault) return;
    this.fault = error;
    this.peer?.fail(error);
    for (const queue of this.outputStreams.values()) queue.fail(error);
    void this.closeCapabilityStreams().catch(() => undefined);
    if (kill) this.process?.signal("SIGKILL");
  }

  private openCapabilityStream(source: AsyncIterable<unknown>): Readonly<{
    kind: "stream-handle";
    streamId: number;
  }> {
    if (this.capabilityStreams.size >= MAX_CAPABILITY_STREAMS) {
      throw new RpcFault(-32003, "RESOURCE_LIMIT_EXCEEDED", {
        resource: "capability-streams",
        limit: MAX_CAPABILITY_STREAMS,
      });
    }
    const streamId = this.allocateStreamId();
    this.capabilityStreams.set(streamId, {
      iterator: source[Symbol.asyncIterator](),
      started: false,
      cancelled: false,
      completed: false,
      pump: null,
    });
    return Object.freeze({ kind: "stream-handle", streamId });
  }

  private async pumpCapabilityStream(streamId: number, stream: CapabilityStream): Promise<void> {
    try {
      while (!stream.cancelled) {
        const item = await stream.iterator.next();
        if (item.done || stream.cancelled) break;
        await this.peer!.notify("stream.value", { streamId, value: item.value });
      }
      if (!stream.cancelled) {
        stream.completed = true;
        await this.peer!.notify("stream.end", { streamId });
      }
    } catch {
      if (!stream.cancelled) {
        stream.completed = true;
        try {
          await this.peer!.notify("stream.error", {
            streamId,
            error: { code: "CAPABILITY_STREAM_ERROR", message: "storage watch failed" },
          });
        } catch (error) {
          this.fatal(translateRemoteError(error));
        }
      }
    }
  }

  private async closeCapabilityStream(streamId: number, stream: CapabilityStream): Promise<void> {
    if (stream.cancelled) return;
    stream.cancelled = true;
    this.capabilityStreams.delete(streamId);
    if (!stream.completed && stream.iterator.return) {
      await settlesWithin(
        this.returnCapabilityIterator(stream),
        this.options.disposeGracePeriodMs,
      );
    }
  }

  private async closeCapabilityStreams(): Promise<void> {
    const streams = [...this.capabilityStreams];
    this.capabilityStreams.clear();
    for (const [streamId, stream] of streams) await this.closeCapabilityStream(streamId, stream);
  }

  private async returnCapabilityIterator(stream: CapabilityStream): Promise<void> {
    try {
      await stream.iterator.return?.();
    } catch {
      // Trusted adapter cleanup errors cannot restore a retired capability stream.
    }
  }

  private async readStderr(process: LaunchedPluginProcess): Promise<void> {
    let bytes = 0;
    for await (const chunk of process.stderr) {
      if (bytes >= 64 * 1024) break;
      const text = Buffer.from(chunk).toString("utf8").slice(0, 16_384);
      bytes += Buffer.byteLength(text);
      this.options.logger.warn(text, {
        source: "plugin-stderr",
        plugin: this.plugin.manifest.plugin.name,
        instance: this.instanceId,
      });
    }
  }

  private allocateStreamId(): number {
    return this.nextStreamId++;
  }
}

function selectStageId(manifest: Manifest, requested: string | undefined): string {
  if (requested) return requested;
  if (manifest.contributes.stages.length === 1) return manifest.contributes.stages[0]!.id;
  const suffix = manifest.plugin.name.split("/").at(-1)!;
  return suffix;
}

function resolveDeclaredCapabilities(
  plugin: DiscoveredPlugin,
  options: ResolvedOptions,
): { readonly required: readonly Capability[]; readonly all: readonly Capability[] } {
  const usesStorageRoot = declaredCapabilityStrings(plugin.manifest)
    .some(capability => capability.includes("$storageRoot"));
  if (usesStorageRoot && !options.storageRoot) {
    throw new PluginHostError(
      "REQUIRED_CAPABILITY_DENIED",
      "plugin capability templates require an explicit storageRoot",
      { plugin: plugin.manifest.plugin.name },
    );
  }
  const env = {
    storageRoot: options.storageRoot ?? "",
    cacheDir: options.cacheDirectory ?? null,
    pluginDir: plugin.rootDirectory,
  };
  const required = plugin.manifest.capabilities.required.map(entry =>
    resolveCapabilityTemplate(capabilityEntry(entry), env));
  const optional = plugin.manifest.capabilities.optional.map(entry =>
    resolveCapabilityTemplate(capabilityEntry(entry), env));
  return { required, all: [...required, ...optional] };
}

function declaredCapabilityStrings(manifest: Manifest): readonly string[] {
  return [...manifest.capabilities.required, ...manifest.capabilities.optional]
    .map(capabilityEntry);
}

function capabilityEntry(entry: Manifest["capabilities"]["required"][number]): string {
  return entry.detail
    ? `${entry.realm}:${entry.scope}:${entry.detail}`
    : `${entry.realm}:${entry.scope}`;
}

function clonePluginMap(
  plugins: ReadonlyMap<string, DiscoveredPlugin>,
): ReadonlyMap<string, DiscoveredPlugin> {
  return new Map([...plugins].map(([name, plugin]) => {
    const clonedBytes = new Map<Uint8Array, Uint8Array>();
    const configSchemas = Object.freeze(Object.fromEntries(
      Object.entries(plugin.configSchemas).map(([stageId, schema]) => {
        let bytes = clonedBytes.get(schema.bytes);
        if (!bytes) {
          bytes = Uint8Array.from(schema.bytes);
          clonedBytes.set(schema.bytes, bytes);
        }
        return [stageId, Object.freeze({
          relativePath: schema.relativePath,
          bytes,
          hash: schema.hash,
        })];
      }),
    ));
    return [name, Object.freeze({
      ...plugin,
      manifest: deepFreeze(structuredClone(plugin.manifest)),
      entryBytes: Uint8Array.from(plugin.entryBytes),
      configSchemas,
    })];
  }));
}

function deepFreeze<T>(value: T): T {
  if (typeof value !== "object" || value === null || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) deepFreeze(child);
  return Object.freeze(value);
}

function loadConfigSchema(
  plugin: DiscoveredPlugin,
  stage: StageContribution,
): ({
  readonly schema: JsonValue;
  readonly hash: string;
  readonly relativePath: string;
  readonly bytes: Uint8Array;
} | null) {
  if (!stage.configSchema) return null;
  try {
    const snapshot = plugin.configSchemas[stage.id];
    if (!snapshot || snapshot.relativePath !== stage.configSchema
        || hashBytes(snapshot.bytes) !== snapshot.hash) {
      throw new TypeError("verified config schema snapshot is absent or mutated");
    }
    const schema = JSON.parse(Buffer.from(snapshot.bytes).toString("utf8")) as unknown;
    assertConfigSchema(schema);
    return {
      schema: deepFreeze(schema as JsonValue),
      hash: snapshot.hash,
      relativePath: snapshot.relativePath,
      bytes: Uint8Array.from(snapshot.bytes),
    };
  } catch (cause) {
    throw new PluginHostError("CONFIG_SCHEMA_INVALID", "config schema is invalid or escapes the plugin root", {
      plugin: plugin.manifest.plugin.name,
      stage: stage.id,
    }, { cause });
  }
}

const CONFIG_SCHEMA_TYPES = new Set([
  "string", "number", "integer", "boolean", "array", "object", "null",
]);

function assertConfigSchema(value: unknown, depth = 0): asserts value is Record<string, JsonValue> {
  if (!isJsonRecord(value) || depth > 256) throw new TypeError("config schema must be a bounded object schema");
  const schema = value as Record<string, unknown>;
  if (schema.type !== undefined) {
    const types = Array.isArray(schema.type) ? schema.type : [schema.type];
    if (types.length === 0 || types.some(type => typeof type !== "string" || !CONFIG_SCHEMA_TYPES.has(type))) {
      throw new TypeError("config schema type is invalid");
    }
  }
  if (schema.enum !== undefined && !Array.isArray(schema.enum)) throw new TypeError("config schema enum must be an array");
  if (schema.required !== undefined
      && (!Array.isArray(schema.required) || schema.required.some(name => typeof name !== "string"))) {
    throw new TypeError("config schema required must be a string array");
  }
  if (schema.additionalProperties !== undefined && typeof schema.additionalProperties !== "boolean") {
    throw new TypeError("config schema additionalProperties must be boolean");
  }
  if (schema.properties !== undefined) {
    if (!isJsonRecord(schema.properties)) throw new TypeError("config schema properties must be an object");
    for (const child of Object.values(schema.properties)) assertConfigSchema(child, depth + 1);
  }
  if (schema.items !== undefined) assertConfigSchema(schema.items, depth + 1);
  for (const keyword of ["oneOf", "anyOf", "allOf"] as const) {
    const children = schema[keyword];
    if (children === undefined) continue;
    if (!Array.isArray(children) || children.length === 0) {
      throw new TypeError(`config schema ${keyword} must be a non-empty schema array`);
    }
    for (const child of children) assertConfigSchema(child, depth + 1);
  }
  for (const keyword of ["minLength", "maxLength", "minItems", "maxItems"] as const) {
    const bound = schema[keyword];
    if (bound !== undefined && (!Number.isSafeInteger(bound) || (bound as number) < 0)) {
      throw new TypeError(`config schema ${keyword} must be a non-negative safe integer`);
    }
  }
  for (const keyword of ["minimum", "maximum"] as const) {
    const bound = schema[keyword];
    if (bound !== undefined && (typeof bound !== "number" || !Number.isFinite(bound))) {
      throw new TypeError(`config schema ${keyword} must be finite`);
    }
  }
  if (schema.pattern !== undefined) {
    throw new TypeError("config schema pattern is unsupported for untrusted plugins");
  }
}

function hashBytes(value: Uint8Array): string {
  return `sha256:${createHash("sha256").update(value).digest("hex")}`;
}

function stageImplementationIdentity(manifestHash: string, configSchemaHash: string | null): string {
  if (configSchemaHash === null) return manifestHash;
  return `sha256:${createHash("sha256")
    .update("forme-stage-identity-v1\0", "utf8")
    .update(manifestHash, "utf8")
    .update("\0", "utf8")
    .update(configSchemaHash, "utf8")
    .digest("hex")}`;
}

function translateRemoteError(error: unknown): Error {
  if (!(error instanceof RpcRemoteError)) return error instanceof Error ? error : new Error(String(error));
  if (error.rpcCode === -32800) return new CancellationError(error.message);
  if (error.rpcCode === -32001) {
    const data = isJsonRecord(error.data) ? error.data : {};
    return new CapabilityError({
      message: error.message,
      capability: typeof data.capability === "string" ? data.capability : "unknown",
    });
  }
  if (error.rpcCode === -32900) {
    const data = isJsonRecord(error.data) ? error.data : {};
    return new StageError({
      code: typeof data.stageErrorCode === "string" ? data.stageErrorCode : "PLUGIN_STAGE_ERROR",
      message: error.message,
      stageName: typeof data.stageName === "string" ? data.stageName : undefined,
      inputPath: typeof data.inputPath === "string" ? data.inputPath : undefined,
      recoverable: data.recoverable === true,
      fields: isJsonRecord(data.fields) ? data.fields as never : {},
    });
  }
  return new StageError({
    code: "PLUGIN_PROTOCOL_ERROR",
    message: error.message,
    fields: { rpcCode: error.rpcCode },
  });
}

class AsyncMutex {
  private tail: Promise<void> = Promise.resolve();

  async acquire(): Promise<() => void> {
    let release!: () => void;
    const current = new Promise<void>(resolve => { release = resolve; });
    const previous = this.tail;
    this.tail = previous.then(() => current);
    await previous;
    return release;
  }
}

class AsyncQueue<T> implements AsyncIterable<T> {
  private readonly values: Array<{ readonly value: T; readonly bytes: number }> = [];
  private readonly waiters: Array<{
    readonly resolve: (value: IteratorResult<T>) => void;
    readonly reject: (error: unknown) => void;
  }> = [];
  private error: Error | null = null;
  private closed = false;
  private bufferedBytes = 0;
  count = 0;

  constructor(
    private readonly limit: number,
    private readonly byteLimit = Number.MAX_SAFE_INTEGER,
    private readonly measure: (value: T) => number = () => 1,
  ) {}

  get completed(): boolean { return this.closed || this.error !== null; }

  push(value: T): void {
    if (this.completed) throw new PluginHostError("PROTOCOL_VIOLATION", "value arrived after stream completion");
    this.count += 1;
    const waiter = this.waiters.shift();
    if (waiter) waiter.resolve({ value, done: false });
    else {
      const bytes = this.measure(value);
      if (this.values.length >= this.limit || bytes > this.byteLimit - this.bufferedBytes) {
        const error = new PluginHostError(
          "RESOURCE_LIMIT_EXCEEDED",
          "plugin output stream exceeded its bounded count or byte buffer",
        );
        this.fail(error);
        throw error;
      }
      this.values.push({ value, bytes });
      this.bufferedBytes += bytes;
    }
  }

  close(): void {
    if (this.completed) return;
    this.closed = true;
    for (const waiter of this.waiters.splice(0)) waiter.resolve({ value: undefined, done: true });
  }

  fail(error: Error): void {
    if (this.completed) return;
    this.error = error;
    this.values.splice(0);
    this.bufferedBytes = 0;
    for (const waiter of this.waiters.splice(0)) waiter.reject(error);
  }

  [Symbol.asyncIterator](): AsyncIterator<T> {
    return {
      next: async () => {
        if (this.error) throw this.error;
        if (this.values.length > 0) {
          const entry = this.values.shift()!;
          this.bufferedBytes -= entry.bytes;
          return { value: entry.value, done: false };
        }
        if (this.closed) return { value: undefined, done: true };
        return new Promise<IteratorResult<T>>((resolve, reject) => {
          this.waiters.push({ resolve, reject });
        });
      },
    };
  }
}

function estimateRetainedBytes(value: unknown, limit: number): number {
  let bytes = 0;
  const seen = new Set<object>();
  const add = (amount: number): void => {
    bytes += amount;
    if (bytes > limit) {
      throw new PluginHostError("RESOURCE_LIMIT_EXCEEDED", "decoded plugin value exceeds its memory budget");
    }
  };
  const visit = (entry: unknown): void => {
    if (entry === null || typeof entry === "number" || typeof entry === "boolean") { add(8); return; }
    if (typeof entry === "string") { add(16 + Buffer.byteLength(entry)); return; }
    if (entry instanceof Uint8Array) { add(32 + entry.byteLength); return; }
    if (typeof entry !== "object" || entry === null) { add(8); return; }
    if (seen.has(entry)) throw new PluginHostError("PROTOCOL_VIOLATION", "decoded plugin value is cyclic");
    seen.add(entry);
    add(Array.isArray(entry) ? 32 : 64);
    for (const [key, child] of Object.entries(entry)) {
      add(8 + Buffer.byteLength(key));
      visit(child);
    }
    seen.delete(entry);
  };
  visit(value);
  return bytes;
}

function positive(value: number | undefined, fallback: number): number {
  const result = value ?? fallback;
  if (!Number.isSafeInteger(result) || result <= 0) {
    throw new RangeError("plugin host limits must be positive safe integers");
  }
  return result;
}

function isAsyncIterable(value: unknown): value is AsyncIterable<unknown> {
  return typeof value === "object" && value !== null
    && typeof (value as { [Symbol.asyncIterator]?: unknown })[Symbol.asyncIterator] === "function";
}

function isJsonRecord(value: unknown): value is Record<string, never> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isLogLevel(value: unknown): value is "trace" | "debug" | "info" | "warn" | "error" {
  return value === "trace" || value === "debug" || value === "info"
    || value === "warn" || value === "error";
}

function capabilityContext(
  context: StageContext | null,
  apis: ResolvedOptions["capabilityApis"],
): StageContext | null {
  if (!context) return null;
  return {
    ...context,
    storage: apis.storage ?? context.storage,
    network: apis.network ?? context.network,
    env: apis.env ?? context.env,
    filesystem: apis.filesystem ?? context.filesystem,
    shell: apis.shell ?? context.shell,
  };
}

async function settlesWithin(promise: Promise<unknown>, milliseconds: number): Promise<boolean> {
  let timer: ReturnType<typeof setTimeout> | null = null;
  try {
    return await Promise.race([
      promise.then(() => true, () => true),
      new Promise<boolean>(resolve => { timer = setTimeout(() => resolve(false), milliseconds); }),
    ]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}

/** @internal Test-only access to deterministic pure helpers and bounded queues. */
export const __testing = {
  AsyncQueue,
  positive,
  isAsyncIterable,
  isJsonRecord,
  isLogLevel,
  selectStageId,
  resolveDeclaredCapabilities,
  loadConfigSchema,
  assertConfigSchema,
  hashBytes,
  stageImplementationIdentity,
  estimateRetainedBytes,
  translateRemoteError,
  settlesWithin,
  capabilityContext,
};
