import { spawn } from "node:child_process";
import { access, cp, mkdtemp, mkdir, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { Writable } from "node:stream";
import { describe, expect, it } from "vitest";
import {
  createCancellationTokenSource,
  deniedEnvApi,
  deniedFilesystemApi,
  deniedNetworkApi,
  deniedShellApi,
  frozenClock,
  inMemoryCache,
  inMemoryEventBus,
  neverCancelledToken,
  noOpTelemetryEmitter,
  silentLogger,
  type StageContext,
  type StageInitContext,
  type StorageApi,
} from "@coding-adventures/forme-stage";
import { CapabilityError, CancellationError, StageError } from "@coding-adventures/forme-errors";
import {
  createPluginHost,
  PluginHostError,
  type PluginLaunchRequest,
  type PluginProcessFactory,
} from "../src/index.js";

const fixtureRoot = fileURLToPath(new URL("./fixtures", import.meta.url));
const runnerRoot = fileURLToPath(new URL("../../forme-plugin-runner-ts", import.meta.url));

function processFactory(
  mismatch = false,
  flags: readonly string[] = [],
  signals: NodeJS.Signals[] = [],
  blockCancellationWrite = false,
  exits: Array<{ code: number | null; signal: NodeJS.Signals | null }> = [],
): PluginProcessFactory {
  return {
    async launch(request: PluginLaunchRequest) {
      const stagedEntry = join(request.workingDirectory, "plugin.mjs");
      await writeFile(stagedEntry, request.plugin.entryBytes);
      if (request.configSchema) {
        const stagedSchema = join(request.workingDirectory, request.configSchema.relativePath);
        await mkdir(dirname(stagedSchema), { recursive: true });
        await writeFile(stagedSchema, request.configSchema.bytes);
      }
      if (request.plugin.manifest.plugin.name === "@example/sdk") {
        await cp(join(runnerRoot, "dist"), join(request.workingDirectory, "runner"), { recursive: true });
        const packagesRoot = dirname(runnerRoot);
        for (const packageName of ["forme-types", "forme-errors", "forme-stage"]) {
          const target = join(request.workingDirectory, "node_modules", "@coding-adventures", packageName);
          await mkdir(target, { recursive: true });
          await cp(join(packagesRoot, packageName, "dist"), join(target, "dist"), { recursive: true });
          await writeFile(join(target, "package.json"), JSON.stringify({
            name: `@coding-adventures/${packageName}`,
            type: "module",
            main: "dist/index.js",
          }));
        }
      }
      const child = spawn(process.execPath, [
        stagedEntry,
        request.stage.id,
        request.configSchema?.hash ?? "-",
        ...(mismatch ? ["--mismatch"] : []),
        ...flags,
      ], {
        cwd: request.workingDirectory,
        env: { PATH: process.env.PATH ?? "" },
        stdio: ["pipe", "pipe", "pipe"],
      });
      const stdin = blockCancellationWrite
        ? new Writable({
          write(chunk, _encoding, callback) {
            if ((chunk as Buffer).includes("$/cancelRequest")) return;
            child.stdin.write(chunk, callback);
          },
          destroy(error, callback) {
            child.stdin.destroy();
            callback(error);
          },
        })
        : child.stdin;
      return {
        isolation: "sandboxed",
        isolationProvider: "fm-b014-test-fixture",
        launchedManifestHash: request.plugin.manifestHash,
        launchedConfigSchemaHash: request.configSchema?.hash ?? null,
        stdin,
        stdout: child.stdout,
        stderr: child.stderr,
        exited: new Promise((resolve) => child.once("exit", (code, signal) => {
          exits.push({ code, signal });
          resolve({ code, signal });
        })),
        signal(value) {
          signals.push(value);
          if (value !== "SIGTERM" || !flags.includes("--ignore-term")) child.kill(value);
        },
      };
    },
  };
}

const storage: StorageApi = {
  async read(path) { return new TextEncoder().encode(`read:${path}`); },
  async readBounded(path, maxBytes) {
    return new TextEncoder().encode(`read:${path}`).subarray(0, maxBytes);
  },
  async write() {},
  async exists() { return true; },
  async *list() {},
  async *watch() {},
  async remove() {},
  async stat() { return { size: 1, mtimeMs: 0, type: "file" }; },
};

function context(cancellation = neverCancelledToken()): StageContext {
  return {
    logger: silentLogger(),
    cancellation,
    time: frozenClock({ timestamp: 0 }),
    cache: inMemoryCache(),
    telemetry: noOpTelemetryEmitter(),
    storage,
    network: deniedNetworkApi(),
    env: deniedEnvApi(),
    filesystem: deniedFilesystemApi(),
    shell: deniedShellApi(),
    events: inMemoryEventBus(),
  };
}

function initContext(): StageInitContext {
  const { cancellation: _cancellation, cache: _cache, ...rest } = context();
  return { ...rest, config: {} };
}

async function makeHost(factory: PluginProcessFactory | undefined = processFactory()) {
  return createPluginHost({
    roots: [fixtureRoot],
    grants: { "@example/echo": ["storage:read"] },
    processFactory: factory,
    handshakeTimeoutMs: 1_000,
    requestTimeoutMs: 1_000,
    cancellationGracePeriodMs: 30,
    disposeGracePeriodMs: 100,
    killGracePeriodMs: 100,
  });
}

describe("plugin host cross-process contract", () => {
  it("runs a real TypeScript SDK stage end to end", async () => {
    const host = await createPluginHost({
      roots: [fixtureRoot],
      grants: { "@example/sdk": ["storage:read"] },
      capabilityApis: { storage },
      processFactory: processFactory(),
    });
    const stage = await host.loadStage({
      kind: "stage-ref", packageName: "@example/sdk", export: "echo",
    }, "sdk-e2e");
    const result = await stage.run({ readPath: "posts/sdk.md" } as never, {}, context()) as {
      bytes: Uint8Array;
    };
    expect(result.bytes).toEqual(new TextEncoder().encode("read:posts/sdk.md"));
    await stage.dispose?.(initContext());
    await host.dispose();
  });

  it("validates roots, limits, stage identities, and disposed hosts", async () => {
    await expect(createPluginHost({ roots: [] })).rejects.toThrow(/at least one/);
    await expect(createPluginHost({ roots: [fixtureRoot], requestTimeoutMs: 0 }))
      .rejects.toBeInstanceOf(RangeError);
    const host = await makeHost();
    await expect(host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "")).rejects.toThrow(/non-empty/);
    const defaultStage = await host.loadStage({
      kind: "stage-ref", packageName: "@example/echo",
    }, "default");
    expect(defaultStage.name).toBe("@example/echo/echo");
    expect(defaultStage.configSchema).toEqual({ type: "object", additionalProperties: true });
    expect(() => {
      (defaultStage.configSchema as { type: string }).type = "string";
    }).toThrow(TypeError);
    expect(defaultStage.configSchema).toEqual({ type: "object", additionalProperties: true });
    expect(defaultStage.implementationIdentity).toMatch(/^sha256:[0-9a-f]{64}$/);
    const derivedStage = await host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    });
    expect(derivedStage.name).toBe("@example/echo/echo");
    await defaultStage.dispose?.(initContext());
    await host.dispose();
    await host.dispose();
    expect(() => derivedStage.run({} as never, {}, context())).toThrow(/disposed/);
    await expect(host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "late")).rejects.toThrow(/disposed/);
  });

  it("resolves without launch and fails closed without a sandbox launcher", async () => {
    const host = await createPluginHost({
      roots: [fixtureRoot],
      grants: { "@example/echo": ["storage:read"] },
    });
    const stage = await host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "echo-1");
    expect(stage.name).toBe("@example/echo/echo");
    await expect(stage.init?.({}, initContext())).rejects.toThrow(/SANDBOX_UNAVAILABLE/);
    await host.dispose();
  });

  it("keeps the launch snapshot private from public discovery metadata", async () => {
    const host = await makeHost();
    const exposed = host.plugins.get("@example/echo")!;
    exposed.entryBytes[0] = exposed.entryBytes[0]! ^ 0xff;
    expect(() => {
      (exposed.manifest.plugin as { name: string }).name = "@example/mutated";
    }).toThrow(TypeError);
    const stage = await host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "snapshot-private");
    await expect(stage.run({ ok: true } as never, {}, context())).resolves.toEqual({ ok: true });
    await stage.dispose?.(initContext());
    await host.dispose();
  });

  it("handshakes, mediates granted storage, denies env, and forwards diagnostics", async () => {
    const lines: string[] = [];
    const host = await createPluginHost({
      roots: [fixtureRoot],
      grants: { "@example/echo": ["storage:read"] },
      processFactory: processFactory(),
      logger: {
        ...silentLogger(),
        info(message) { lines.push(message); },
      },
    });
    const stage = await host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "echo-2");
    await stage.init?.({}, initContext());
    await stage.init?.({}, initContext());
    await expect(stage.run({ readPath: "posts/a.md" } as never, {}, context()))
      .resolves.toEqual({
        readPath: "posts/a.md",
        bytes: Buffer.from("read:posts/a.md").toString("base64"),
      });
    await expect(stage.run({ probeDenied: true } as never, {}, context()))
      .resolves.toEqual({ deniedCode: -32001 });
    await expect(stage.run({ probeStale: true } as never, {}, context()))
      .resolves.toEqual({ staleCode: -32001 });
    await expect(stage.run({ ok: true } as never, {}, context()))
      .resolves.toEqual({ ok: true });
    await expect(stage.run({ noLogFields: true } as never, {}, context()))
      .resolves.toEqual({ noLogFields: true });
    await expect(stage.run({ malformedSingle: true } as never, {}, context()))
      .rejects.toThrow(/non-single/);
    await expect(stage.run({ remoteErrorCode: -32001, remoteErrorData: { capability: "env:X" } } as never, {}, context()))
      .rejects.toBeInstanceOf(CapabilityError);
    await expect(stage.run({ remoteErrorCode: -32900, remoteErrorData: { stageErrorCode: "FIXTURE" } } as never, {}, context()))
      .rejects.toBeInstanceOf(StageError);
    await expect(stage.run({ remoteErrorCode: -123 } as never, {}, context()))
      .rejects.toBeInstanceOf(StageError);
    await expect(stage.run({ stderr: true } as never, {}, context()))
      .resolves.toEqual({ stderr: true });
    expect(lines).toContain("fixture echo");
    await stage.dispose?.(initContext());
    await expect(stage.run({ restarted: true } as never, {}, context()))
      .resolves.toEqual({ restarted: true });
    await stage.dispose?.(initContext());
    await host.dispose();
  });

  it("fails closed when a plugin exceeds its lifetime log budget", async () => {
    const host = await createPluginHost({
      roots: [fixtureRoot],
      grants: { "@example/echo": ["storage:read"] },
      processFactory: processFactory(),
      maxLogEntries: 2,
      maxLogBytes: 1_024,
    });
    const stage = await host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "log-flood");
    await expect(stage.run({ logFlood: 3 } as never, {}, context()))
      .rejects.toThrow(/log budget/);
    await host.dispose();
  });

  it("round-trips typed streams without materializing the input", async () => {
    const host = await makeHost();
    const stage = await host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "stream",
    }, "stream-1");
    expect(stage.consumes.name).toBe("Stream");
    expect(stage.produces.name).toBe("Stream");
    await stage.init?.({}, initContext());
    async function* input() {
      yield { n: 1 } as never;
      yield { n: 2 } as never;
    }
    const output = stage.run(input() as never, {}, context()) as AsyncIterable<unknown>;
    const values: unknown[] = [];
    for await (const value of output) values.push(value);
    expect(values).toEqual([{ n: 1 }, { n: 2 }]);
    const empty = stage.run({ scalar: true } as never, {}, context()) as AsyncIterable<unknown>;
    const emptyValues: unknown[] = [];
    for await (const value of empty) emptyValues.push(value);
    expect(emptyValues).toEqual([]);
    const malformed = stage.run({ badStreamMeta: true } as never, {}, context()) as AsyncIterable<unknown>;
    await expect(async () => {
      for await (const _value of malformed) { /* no-op */ }
    }).rejects.toThrow(/metadata/);
    await stage.dispose?.(initContext());
    await host.dispose();
  });

  it("bridges typed stream input into a single-output stage", async () => {
    const host = await makeHost();
    const stage = await host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "reduce",
    }, "reduce-1");
    expect(stage.consumes.name).toBe("Stream");
    expect(stage.produces.name).toBe("ContentNode");
    const result = await stage.run((async function* () {
      yield { n: 1 } as never;
      yield { n: 2 } as never;
    })() as never, {}, context());
    expect(result).toEqual({ count: 2, values: [{ n: 1 }, { n: 2 }] });
    await stage.dispose?.(initContext());
    await host.dispose();
  });

  it("round-trips binary values and reserved-looking objects", async () => {
    const host = await makeHost();
    const single = await host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "binary-single");
    const collision = { $forme: "bytes", base64: "not-binary", nested: new Uint8Array([9]) };
    const singleResult = await single.run({ bytes: new Uint8Array([0, 1, 255]), collision } as never, {}, context()) as {
      bytes: Uint8Array; collision: typeof collision;
    };
    expect(singleResult.bytes).toEqual(new Uint8Array([0, 1, 255]));
    expect(singleResult.collision).toEqual(collision);

    const stream = await host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "stream",
    }, "binary-stream");
    const values: unknown[] = [];
    for await (const value of stream.run((async function* () {
      yield { bytes: new Uint8Array([2, 3, 4]) } as never;
    })() as never, {}, context()) as AsyncIterable<unknown>) values.push(value);
    expect(values).toEqual([{ bytes: new Uint8Array([2, 3, 4]) }]);
    await single.dispose?.(initContext());
    await stream.dispose?.(initContext());
    await host.dispose();
  });

  it("retires a stream session and closes upstream input after early abandonment", async () => {
    const host = await makeHost();
    const stage = await host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "stream",
    }, "stream-abandon");
    let returned = false;
    async function* input() {
      try {
        yield { n: 1 } as never;
        yield { n: 2 } as never;
      } finally {
        returned = true;
      }
    }
    for await (const _value of stage.run(input() as never, {}, context()) as AsyncIterable<unknown>) {
      break;
    }
    expect(returned).toBe(true);
    const values: unknown[] = [];
    for await (const value of stage.run((async function* () {
      yield { restarted: true } as never;
    })() as never, {}, context()) as AsyncIterable<unknown>) values.push(value);
    expect(values).toEqual([{ restarted: true }]);
    await stage.dispose?.(initContext());
    await host.dispose();
  });

  it("releases the stream mutex when an upstream iterator return throws synchronously", async () => {
    const host = await makeHost();
    const stage = await host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "stream",
    }, "stream-throwing-return");
    const hostile = {
      [Symbol.asyncIterator]() {
        let emitted = false;
        return {
          async next() {
            if (emitted) return { value: undefined, done: true as const };
            emitted = true;
            return { value: { n: 1 }, done: false as const };
          },
          return() { throw new Error("hostile return"); },
        };
      },
    };
    for await (const _value of stage.run(hostile as never, {}, context()) as AsyncIterable<unknown>) break;
    const values: unknown[] = [];
    for await (const value of stage.run((async function* () {
      yield { restarted: true } as never;
    })() as never, {}, context()) as AsyncIterable<unknown>) values.push(value);
    expect(values).toEqual([{ restarted: true }]);
    await host.dispose();
  });

  it("bounds abandoned-stream cleanup when the cancellation write stalls", async () => {
    const signals: NodeJS.Signals[] = [];
    const exits: Array<{ code: number | null; signal: NodeJS.Signals | null }> = [];
    const host = await makeHost(processFactory(false, [], signals, true, exits));
    const stage = await host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "stream",
    }, "stream-stalled-cancel");
    const output = stage.run((async function* () {
      yield { n: 1 } as never;
      yield { n: 2 } as never;
    })() as never, {}, context()) as AsyncIterable<unknown>;
    const iterator = output[Symbol.asyncIterator]();
    await expect(iterator.next()).resolves.toEqual({ value: { n: 1 }, done: false });
    const returned = iterator.return!();
    await expect(Promise.race([
      returned,
      new Promise((_, reject) => setTimeout(() => reject(new Error("cleanup hung")), 1_000)),
    ])).resolves.toEqual({ value: undefined, done: true });
    expect(
      exits.length > 0
      || signals.some(signal => signal === "SIGTERM" || signal === "SIGKILL"),
    ).toBe(true);
    await host.dispose();
  });

  it("kills announcement mismatches and surfaces plugin crashes", async () => {
    const mismatched = await makeHost(processFactory(true));
    const bad = await mismatched.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "bad");
    await expect(bad.init?.({}, initContext())).rejects.toThrow(/MANIFEST_MISMATCH/);
    await mismatched.dispose();

    const badRunnerHost = await makeHost(processFactory(false, ["--invalid-runner"]));
    const badRunner = await badRunnerHost.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "bad-runner");
    await expect(badRunner.init?.({}, initContext())).rejects.toThrow(/identify the runner/);
    await badRunnerHost.dispose();

    const badAnnounceHost = await makeHost(processFactory(false, ["--announce-mismatch"]));
    const badAnnounce = await badAnnounceHost.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "bad-announce");
    await expect(badAnnounce.init?.({}, initContext())).rejects.toThrow(/runtime announcement/);
    await badAnnounceHost.dispose();

    const host = await makeHost();
    const stage = await host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "crash");
    await stage.init?.({}, initContext());
    await expect(stage.run({ crash: true } as never, {}, context()))
      .rejects.toMatchObject({ code: "PLUGIN_CRASHED" });
    await expect(stage.run({ ok: true } as never, {}, context()))
      .resolves.toEqual({ ok: true });
    await host.dispose();
  });

  it("escalates ignored cancellation and rejects the run as cancellation", async () => {
    const signals: NodeJS.Signals[] = [];
    const host = await makeHost(processFactory(false, ["--ignore-term"], signals));
    const stage = await host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "hang");
    await stage.init?.({}, initContext());
    const source = createCancellationTokenSource();
    const run = Promise.resolve(stage.run({ hang: true } as never, {}, context(source.token)));
    setTimeout(() => source.cancel("test cancellation"), 10);
    await expect(run).rejects.toBeInstanceOf(CancellationError);
    await new Promise(resolve => setTimeout(resolve, 130));
    expect(signals).toEqual(expect.arrayContaining(["SIGTERM", "SIGKILL"]));
    await expect(stage.run({ restarted: true } as never, {}, context()))
      .resolves.toEqual({ restarted: true });
    await host.dispose();
  });

  it("intersects policy grants with per-instance capability requests", async () => {
    const host = await createPluginHost({
      roots: [fixtureRoot],
      grants: { "@example/echo": ["storage:read", "env:FIXTURE_SECRET"] },
      capabilityApis: { storage },
      processFactory: processFactory(),
    });
    const stage = await host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "narrowed", ["storage:read"]);
    expect(stage.capabilities).toEqual(["storage:read"]);
    await expect(stage.run({ probeDenied: true } as never, {}, context()))
      .resolves.toEqual({ deniedCode: -32001 });
    await stage.dispose?.(initContext());
    await host.dispose();
  });

  it("ignores late cancellation callbacks after a completed run", async () => {
    const host = await makeHost();
    const stage = await host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "late-cancel");
    const source = createCancellationTokenSource();
    await expect(stage.run({ ok: true } as never, {}, context(source.token)))
      .resolves.toEqual({ ok: true });
    source.cancel("already complete");
    await stage.dispose?.(initContext());
    await host.dispose();
  });

  it("auto-initializes on first run and handles cancellation without a reason", async () => {
    const host = await makeHost();
    const stage = await host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "auto-init");
    await expect(stage.run({ automatic: true } as never, {}, context()))
      .resolves.toEqual({ automatic: true });
    const source = createCancellationTokenSource();
    const run = Promise.resolve(stage.run({ hang: true } as never, {}, context(source.token)));
    setTimeout(() => source.cancel(), 10);
    await expect(run).rejects.toBeInstanceOf(CancellationError);
    await host.dispose();
  });

  it("retires and cleanly relaunches after an init timeout", async () => {
    let launches = 0;
    const factory: PluginProcessFactory = {
      launch(request) {
        launches += 1;
        return processFactory(false, launches === 1 ? ["--ignore-init"] : []).launch(request);
      },
    };
    const host = await createPluginHost({
      roots: [fixtureRoot],
      grants: { "@example/echo": ["storage:read"] },
      processFactory: factory,
      requestTimeoutMs: 30,
      disposeGracePeriodMs: 30,
      killGracePeriodMs: 30,
    });
    const stage = await host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "init-retry");
    await expect(stage.init?.({}, initContext())).rejects.toThrow(/REQUEST_TIMEOUT/);
    await expect(stage.run({ recovered: true } as never, {}, context()))
      .resolves.toEqual({ recovered: true });
    expect(launches).toBe(2);
    await stage.dispose?.(initContext());
    await host.dispose();
  });

  it.each([
    ["badStreamNotification"],
    ["inactiveStreamNotification"],
    ["invalidLog"],
    ["unknownNotification"],
  ])("isolates malformed %s notifications", async (key) => {
    const host = await makeHost();
    const stage = await host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, `malformed-${key}`);
    await stage.init?.({}, initContext());
    await expect(stage.run({ [key]: true } as never, {}, context()))
      .rejects.toThrow();
    await host.dispose();
  });

  it("escalates disposal from RPC timeout through TERM and KILL", async () => {
    const termHost = await makeHost(processFactory(false, ["--ignore-dispose"]));
    const termStage = await termHost.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "term-dispose");
    await termStage.init?.({}, initContext());
    await termHost.dispose();

    const killHost = await makeHost(processFactory(false, ["--ignore-dispose", "--ignore-term"]));
    const killStage = await killHost.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "kill-dispose");
    await killStage.init?.({}, initContext());
    await killHost.dispose();
  });

  it("removes the working directory even when launcher cleanup rejects", async () => {
    let workingDirectory = "";
    const factory: PluginProcessFactory = {
      async launch(request) {
        workingDirectory = request.workingDirectory;
        const launched = await processFactory(false, ["--ignore-dispose"]).launch(request);
        return {
          ...launched,
          async cleanup() { throw new Error("cleanup failed"); },
        };
      },
    };
    const host = await makeHost(factory);
    const stage = await host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "cleanup-failure");
    await stage.init?.({}, initContext());
    await expect(host.dispose()).rejects.toBeInstanceOf(AggregateError);
    await expect(access(workingDirectory)).rejects.toMatchObject({ code: "ENOENT" });
  });

  it("rejects unknown plugins, exports, missing grants, and false isolation attestations", async () => {
    const host = await makeHost();
    await expect(host.loadStage({
      kind: "stage-ref", packageName: "@example/missing", export: "echo",
    }, "x")).rejects.toBeInstanceOf(PluginHostError);
    await expect(host.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "missing",
    }, "x")).rejects.toThrow(/STAGE_NOT_FOUND/);
    await host.dispose();

    const noGrants = await createPluginHost({ roots: [fixtureRoot], processFactory: processFactory() });
    await expect(noGrants.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "x")).rejects.toThrow(/REQUIRED_CAPABILITY_DENIED/);

    const inheritedRoot = await mkdtemp(join(tmpdir(), "forme-plugin-inherited-grants-"));
    const inheritedPlugin = join(inheritedRoot, "constructor");
    await mkdir(inheritedPlugin);
    await writeFile(join(inheritedPlugin, "entry.mjs"), "// not launched\n");
    await writeFile(join(inheritedPlugin, "plugin.toml"), `manifestVersion = 1
[plugin]
name = "constructor"
version = "1.0.0"
apiVersion = 1
[runtime]
kind = "node"
entry = "./entry.mjs"
[[capabilities.required]]
realm = "storage"
scope = "read"
reason = "regression"
[[contributes.stages]]
id = "echo"
consumes = "ContentNode"
produces = "ContentNode"
`);
    const inherited = await createPluginHost({ roots: [inheritedRoot], grants: {} });
    await expect(inherited.loadStage({
      kind: "stage-ref", packageName: "constructor", export: "echo",
    }, "constructor")).rejects.toThrow(/REQUIRED_CAPABILITY_DENIED/);
    await inherited.dispose();

    const undefinedGrant = await createPluginHost({
      roots: [fixtureRoot],
      grants: { "@example/echo": undefined as never },
    });
    await expect(undefinedGrant.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "undefined-grant")).rejects.toThrow(/REQUIRED_CAPABILITY_DENIED/);
    await undefinedGrant.dispose();

    const lyingFactory: PluginProcessFactory = {
      async launch(request) {
        const launched = await processFactory().launch(request);
        return { ...launched, isolation: "none" };
      },
    };
    const unsandboxed = await makeHost(lyingFactory);
    const stage = await unsandboxed.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "x");
    await expect(stage.init?.({}, initContext())).rejects.toThrow(/SANDBOX_UNAVAILABLE/);
    await unsandboxed.dispose();

    const wrongBytesFactory: PluginProcessFactory = {
      async launch(request) {
        const launched = await processFactory().launch(request);
        return { ...launched, launchedManifestHash: "blake2b:wrong" };
      },
    };
    const wrongBytes = await makeHost(wrongBytesFactory);
    const wrongBytesStage = await wrongBytes.loadStage({
      kind: "stage-ref", packageName: "@example/echo", export: "echo",
    }, "wrong-bytes");
    await expect(wrongBytesStage.init?.({}, initContext())).rejects.toThrow(/verified plugin identity/);
    await wrongBytes.dispose();
  });

  it("loads bounded config schemas and rejects invalid schemas", async () => {
    async function schemaRoot(schema: string): Promise<string> {
      const root = await mkdtemp(join(tmpdir(), "forme-plugin-schema-"));
      const path = join(root, "schema");
      await mkdir(path);
      await writeFile(join(path, "entry.mjs"), "// not launched\n");
      await writeFile(join(path, "schema.json"), schema);
      await writeFile(join(path, "plugin.toml"), `manifestVersion = 1
[plugin]
name = "@example/schema"
version = "1.0.0"
apiVersion = 1
[runtime]
kind = "node"
entry = "./entry.mjs"
[[contributes.stages]]
id = "schema"
consumes = "ContentNode"
produces = "ContentNode"
configSchema = "./schema.json"
`);
      return root;
    }
    const validRoot = await schemaRoot('{"type":"object"}');
    const valid = await createPluginHost({ roots: [validRoot] });
    await writeFile(join(validRoot, "schema", "schema.json"), "{");
    const stage = await valid.loadStage({
      kind: "stage-ref", packageName: "@example/schema", export: "schema",
    }, "schema");
    expect(stage.configSchema).toEqual({ type: "object" });
    await valid.dispose();

    const invalid = await createPluginHost({ roots: [await schemaRoot("{")] });
    await expect(invalid.loadStage({
      kind: "stage-ref", packageName: "@example/schema", export: "schema",
    }, "schema")).rejects.toThrow(/CONFIG_SCHEMA_INVALID/);
    await invalid.dispose();

    for (const malformed of ["42", '{"type":7}', '{"properties":{"x":false}}']) {
      const malformedHost = await createPluginHost({ roots: [await schemaRoot(malformed)] });
      await expect(malformedHost.loadStage({
        kind: "stage-ref", packageName: "@example/schema", export: "schema",
      }, "schema")).rejects.toThrow(/CONFIG_SCHEMA_INVALID/);
      await malformedHost.dispose();
    }
  });
});
