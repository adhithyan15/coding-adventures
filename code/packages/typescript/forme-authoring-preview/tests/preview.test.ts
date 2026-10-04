import {
  createAuthoringProject,
  openAuthoringSession,
  type AuthoringSession,
  type AuthoringStorage,
  type StoredAuthoringState,
} from "@coding-adventures/forme-authoring-core";
import type { Pipeline, RunResult, WatchOptions, WatchSession } from "@coding-adventures/forme-orchestrator";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  createAuthoringPreview,
  type AuthoringPreviewAttempt,
  type AuthoringPreviewCoordinator,
  type AuthoringPreviewInput,
  type AuthoringPreviewPublisher,
  type PreparedAuthoringPreview,
} from "../src/index.js";

const encoder = new TextEncoder();

class MemoryStorage implements AuthoringStorage {
  private stored: StoredAuthoringState | null = null;
  private revision = 0;

  async load(): Promise<StoredAuthoringState | null> {
    return this.stored === null
      ? null
      : { bytes: new Uint8Array(this.stored.bytes), revision: this.stored.revision };
  }

  async compareAndSwap(expectedRevision: string | null, bytes: Uint8Array): Promise<{ readonly revision: string }> {
    if (expectedRevision !== this.stored?.revision && !(expectedRevision === null && this.stored === null)) {
      throw new Error("conflict");
    }
    const revision = `storage-${++this.revision}`;
    this.stored = { bytes: new Uint8Array(bytes), revision };
    return { revision };
  }
}

function session(revision: string, title = revision): AuthoringSession {
  return {
    project: createAuthoringProject({ projectId: "01952c0d-7e63-7000-8000-000000000064", title }),
    storageRevision: revision,
    canUndo: false,
    canRedo: false,
    async dispatch() {},
    async undo() {},
    async redo() {},
  };
}

function result(
  outcome: RunResult["outcome"],
  buildId: string,
  options: { readonly html?: string; readonly errors?: RunResult["errors"]; readonly outputs?: RunResult["outputs"] } = {},
): RunResult {
  return {
    outcome,
    buildId: buildId as RunResult["buildId"],
    elapsedMs: 1,
    stages: [],
    errors: options.errors ?? [],
    outputs: options.outputs ?? (outcome === "success" ? {
      site: {
        variant: { kind: "dist-tree" },
        files: { "index.html": encoder.encode(options.html ?? `<h1>${buildId}</h1>`) },
      },
    } : {}),
  };
}

class ControlledWatch implements WatchSession {
  readonly stop = vi.fn(async () => {
    this.settle(result("cancelled", "cancelled"));
  });
  private settled = false;
  private resolve!: (value: IteratorResult<RunResult>) => void;
  private readonly next = new Promise<IteratorResult<RunResult>>(resolve => { this.resolve = resolve; });

  results(): AsyncIterable<RunResult> {
    const next = this.next;
    return { [Symbol.asyncIterator]: () => ({ next: () => next }) };
  }

  rebuild(): Promise<RunResult> {
    throw new Error("authoring preview must use the watch session's initial build");
  }

  settle(value: RunResult): void {
    if (this.settled) return;
    this.settled = true;
    this.resolve({ done: false, value });
  }
}

function immediateWatch(value: IteratorResult<RunResult> | Error): WatchSession {
  return {
    results() {
      return {
        [Symbol.asyncIterator]: () => ({
          next: async () => {
            if (value instanceof Error) throw value;
            return value;
          },
        }),
      };
    },
    async rebuild() { throw new Error("rebuild is not used"); },
    async stop() {},
  };
}

function harness(debounceMs = 0) {
  const watches: ControlledWatch[] = [];
  const watchCalls: Array<{ readonly pipeline: Pipeline; readonly options: WatchOptions }> = [];
  const inputs: AuthoringPreviewInput[] = [];
  const releases: Array<ReturnType<typeof vi.fn>> = [];
  const successes: Array<{ readonly revision: string; readonly buildId: string; readonly files: ReadonlyMap<string, Uint8Array> }> = [];
  const failures: Array<{ readonly revision: string; readonly diagnostics: readonly unknown[] }> = [];
  const publisher: AuthoringPreviewPublisher = {
    publish(snapshot, commit) { commit(() => { successes.push(snapshot); }); },
    publishFailure(failure, commit) { commit(() => { failures.push(failure); }); },
  };
  const coordinator = createAuthoringPreview({
    debounceMs,
    orchestrator: {
      watch(pipeline, options) {
        watchCalls.push({ pipeline, options });
        const controlled = new ControlledWatch();
        watches.push(controlled);
        return controlled;
      },
    },
    materializer: {
      async prepare(input, signal): Promise<PreparedAuthoringPreview> {
        expect(signal).toBeInstanceOf(AbortSignal);
        inputs.push(input);
        const release = vi.fn(async () => {});
        releases.push(release);
        return { pipeline: { revision: input.revision } as unknown as Pipeline, release };
      },
    },
    publisher,
  });
  return { coordinator, watches, watchCalls, inputs, releases, successes, failures, publisher };
}

async function flushDebounce(): Promise<void> {
  await vi.runOnlyPendingTimersAsync();
  await Promise.resolve();
}

afterEach(() => {
  vi.useRealTimers();
});

describe("pipeline-backed authoring preview", () => {
  it("publishes a real FM07 artifact snapshot with the exact persisted revision", async () => {
    vi.useFakeTimers();
    const h = harness();
    const pending = h.coordinator.request(session("rev-1", "First"));
    await flushDebounce();

    expect(h.inputs).toHaveLength(1);
    expect(h.inputs[0]).toMatchObject({ revision: "rev-1", project: { title: "First" } });
    expect(Object.isFrozen(h.inputs[0])).toBe(true);
    expect(Object.isFrozen(h.inputs[0]!.project)).toBe(true);
    expect(h.watchCalls).toHaveLength(1);
    h.watches[0]!.settle(result("success", "build-1"));

    await expect(pending).resolves.toEqual({
      outcome: "ready",
      revision: "rev-1",
      buildId: "build-1",
      diagnostics: [],
    });
    expect(new TextDecoder().decode(h.successes[0]!.files.get("index.html"))).toBe("<h1>build-1</h1>");
    expect(h.successes[0]).toMatchObject({ revision: "rev-1", buildId: "build-1" });
    expect(h.coordinator.state).toEqual({
      phase: "ready",
      activeRevision: "rev-1",
      lastGoodRevision: "rev-1",
      lastGoodBuildId: "build-1",
      diagnostics: [],
    });
  });

  it("accepts the real authoring-core Session prototype accessors", async () => {
    vi.useFakeTimers();
    const realSession = await openAuthoringSession({
      storage: new MemoryStorage(),
      initialProject: createAuthoringProject({
        projectId: "01952c0d-7e63-7000-8000-000000000065",
        title: "Real session",
      }),
    });
    const h = harness();
    const pending = h.coordinator.request(realSession);
    await flushDebounce();
    expect(h.inputs[0]).toMatchObject({ revision: "storage-1", project: { title: "Real session" } });
    h.watches[0]!.settle(result("success", "build-real-session"));
    await expect(pending).resolves.toMatchObject({ outcome: "ready", revision: "storage-1" });
  });

  it("coalesces a burst before preparation and resolves displaced requests as superseded", async () => {
    vi.useFakeTimers();
    const h = harness(25);
    const first = h.coordinator.request(session("rev-1"));
    const second = h.coordinator.request(session("rev-2"));
    const third = h.coordinator.request(session("rev-3"));
    await expect(first).resolves.toMatchObject({ outcome: "superseded", revision: "rev-1" });
    await expect(second).resolves.toMatchObject({ outcome: "superseded", revision: "rev-2" });
    await flushDebounce();
    expect(h.inputs.map(input => input.revision)).toEqual(["rev-3"]);
    h.watches[0]!.settle(result("success", "build-3"));
    await expect(third).resolves.toMatchObject({ outcome: "ready", revision: "rev-3" });
  });

  it("cancels and releases an active revision before building the latest request", async () => {
    vi.useFakeTimers();
    const h = harness();
    const first = h.coordinator.request(session("rev-1"));
    await flushDebounce();
    const second = h.coordinator.request(session("rev-2"));
    await flushDebounce();

    await expect(first).resolves.toMatchObject({ outcome: "superseded", revision: "rev-1" });
    expect(h.watches[0]!.stop).toHaveBeenCalledTimes(1);
    expect(h.releases[0]).toHaveBeenCalledTimes(1);
    expect(h.inputs.map(input => input.revision)).toEqual(["rev-1", "rev-2"]);
    h.watches[1]!.settle(result("success", "build-2"));
    await expect(second).resolves.toMatchObject({ outcome: "ready", revision: "rev-2" });
    expect(h.successes.map(item => item.revision)).toEqual(["rev-2"]);
  });

  it("does not wait for a stopped watch stream to settle before building the latest revision", async () => {
    vi.useFakeTimers();
    const stop = vi.fn(async () => {});
    const release = vi.fn(async () => {});
    let watchCalls = 0;
    const coordinator = createAuthoringPreview({
      debounceMs: 0,
      orchestrator: {
        watch() {
          watchCalls += 1;
          if (watchCalls > 1) return immediateWatch({ done: false, value: result("success", "build-new") });
          return {
            results() {
              return { [Symbol.asyncIterator]: () => ({ next: () => new Promise<IteratorResult<RunResult>>(() => {}) }) };
            },
            async rebuild() { return result("cancelled", "unused"); },
            stop,
          };
        },
      },
      materializer: {
        async prepare(input) {
          return { pipeline: {} as Pipeline, release: input.revision === "rev-hung" ? release : async () => {} };
        },
      },
      publisher: {
        publish(_snapshot, commit) { commit(() => {}); },
        publishFailure(_failure, commit) { commit(() => {}); },
      },
    });
    const first = coordinator.request(session("rev-hung"));
    await flushDebounce();
    const second = coordinator.request(session("rev-new"));
    await flushDebounce();
    await expect(first).resolves.toMatchObject({ outcome: "superseded" });
    await expect(second).resolves.toMatchObject({ outcome: "ready", revision: "rev-new" });
    expect(stop).toHaveBeenCalledOnce();
    expect(release).toHaveBeenCalledOnce();
  });

  it("poisons later builds when a superseded materialization cannot retire", async () => {
    vi.useFakeTimers();
    let prepareCalls = 0;
    const coordinator = createAuthoringPreview({
      debounceMs: 0,
      orchestrator: {
        watch() {
          return {
            results() {
              return { [Symbol.asyncIterator]: () => ({ next: () => new Promise<IteratorResult<RunResult>>(() => {}) }) };
            },
            async rebuild() { return result("cancelled", "unused"); },
            async stop() {},
          };
        },
      },
      materializer: {
        async prepare() {
          prepareCalls += 1;
          return {
            pipeline: {} as Pipeline,
            async release() { throw new Error("retirement secret"); },
          };
        },
      },
      publisher: {
        publish(_snapshot, commit) { commit(() => {}); },
        publishFailure(_failure, commit) { commit(() => {}); },
      },
    });
    const first = coordinator.request(session("rev-poison-old"));
    await flushDebounce();
    const blocked = coordinator.request(session("rev-poison-blocked"));
    await flushDebounce();
    await expect(first).resolves.toMatchObject({ outcome: "superseded" });
    await expect(blocked).resolves.toMatchObject({
      outcome: "failed",
      revision: "rev-poison-blocked",
      diagnostics: [{ code: "E_PREVIEW_RELEASE" }],
    });
    const later = await coordinator.request(session("rev-poison-later"));
    expect(later).toMatchObject({
      outcome: "failed",
      revision: "rev-poison-later",
      diagnostics: [{ code: "E_PREVIEW_RELEASE" }],
    });
    expect(prepareCalls).toBe(1);
    expect(JSON.stringify(later)).not.toContain("secret");
    expect(coordinator.state).toMatchObject({ phase: "failed", activeRevision: "rev-poison-later" });
  });

  it("prevents an async publisher from committing after a newer revision is requested", async () => {
    vi.useFakeTimers();
    const committed: string[] = [];
    let staleCommit!: (mutation: () => void) => boolean;
    let markPublishStarted!: () => void;
    const publishStarted = new Promise<void>(resolve => { markPublishStarted = resolve; });
    const watches: ControlledWatch[] = [];
    const coordinator = createAuthoringPreview({
      debounceMs: 0,
      orchestrator: {
        watch() {
          const watch = new ControlledWatch();
          watches.push(watch);
          return watch;
        },
      },
      materializer: {
        async prepare() { return { pipeline: {} as Pipeline, async release() {} }; },
      },
      publisher: {
        publish(snapshot, commit) {
          if (snapshot.revision !== "rev-old") return void commit(() => { committed.push(snapshot.revision); });
          staleCommit = commit;
          markPublishStarted();
          return new Promise<void>(() => {});
        },
        publishFailure(_failure, commit) { commit(() => {}); },
      },
    });
    const first = coordinator.request(session("rev-old"));
    await flushDebounce();
    watches[0]!.settle(result("success", "build-old"));
    await publishStarted;

    const second = coordinator.request(session("rev-new"));
    expect(coordinator.state).toMatchObject({ phase: "building", activeRevision: "rev-new" });
    expect(staleCommit(() => { committed.push("rev-old"); })).toBe(false);
    await expect(first).resolves.toMatchObject({ outcome: "superseded" });
    await flushDebounce();
    watches[1]!.settle(result("success", "build-new"));
    await expect(second).resolves.toMatchObject({ outcome: "ready", revision: "rev-new" });
    expect(committed).toEqual(["rev-new"]);
  });

  it("records a guarded commit before a hanging publisher is superseded", async () => {
    vi.useFakeTimers();
    const committed: string[] = [];
    let markCommitted!: () => void;
    const committedOld = new Promise<void>(resolve => { markCommitted = resolve; });
    const watches: ControlledWatch[] = [];
    const coordinator = createAuthoringPreview({
      debounceMs: 0,
      orchestrator: {
        watch() {
          const watch = new ControlledWatch();
          watches.push(watch);
          return watch;
        },
      },
      materializer: {
        async prepare() { return { pipeline: {} as Pipeline, async release() {} }; },
      },
      publisher: {
        publish(snapshot, commit) {
          commit(() => { committed.push(snapshot.revision); });
          if (snapshot.revision === "rev-committed-old") {
            markCommitted();
            return new Promise<void>(() => {});
          }
        },
        publishFailure(_failure, commit) { commit(() => {}); },
      },
    });
    const first = coordinator.request(session("rev-committed-old"));
    await flushDebounce();
    watches[0]!.settle(result("success", "build-committed-old"));
    await committedOld;
    expect(coordinator.state).toMatchObject({
      phase: "ready",
      activeRevision: "rev-committed-old",
      lastGoodRevision: "rev-committed-old",
      lastGoodBuildId: "build-committed-old",
    });

    const second = coordinator.request(session("rev-after-commit"));
    expect(coordinator.state).toMatchObject({
      phase: "building",
      activeRevision: "rev-after-commit",
      lastGoodRevision: "rev-committed-old",
      lastGoodBuildId: "build-committed-old",
    });
    await expect(first).resolves.toMatchObject({ outcome: "superseded" });
    await flushDebounce();
    watches[1]!.settle(result("success", "build-after-commit"));
    await expect(second).resolves.toMatchObject({ outcome: "ready" });
    expect(committed).toEqual(["rev-committed-old", "rev-after-commit"]);
  });

  it("preserves a reentrant newer build while recording the snapshot that became visible", async () => {
    vi.useFakeTimers();
    const watches: ControlledWatch[] = [];
    let second!: Promise<AuthoringPreviewAttempt>;
    let coordinator!: AuthoringPreviewCoordinator;
    coordinator = createAuthoringPreview({
      debounceMs: 0,
      orchestrator: {
        watch() {
          const watch = new ControlledWatch();
          watches.push(watch);
          return watch;
        },
      },
      materializer: {
        async prepare() { return { pipeline: {} as Pipeline, async release() {} }; },
      },
      publisher: {
        publish(snapshot, commit) {
          commit(() => {
            if (snapshot.revision === "rev-reentrant-old") {
              second = coordinator.request(session("rev-reentrant-new"));
            }
          });
        },
        publishFailure(_failure, commit) { commit(() => {}); },
      },
    });
    const first = coordinator.request(session("rev-reentrant-old"));
    await flushDebounce();
    watches[0]!.settle(result("success", "build-reentrant-old"));
    await expect(first).resolves.toMatchObject({ outcome: "superseded" });
    expect(coordinator.state).toMatchObject({
      phase: "building",
      activeRevision: "rev-reentrant-new",
      lastGoodRevision: "rev-reentrant-old",
      lastGoodBuildId: "build-reentrant-old",
    });
    await flushDebounce();
    watches[1]!.settle(result("success", "build-reentrant-new"));
    await expect(second).resolves.toMatchObject({ outcome: "ready" });
  });

  it("retains last-good output and binds bounded diagnostics to the failed revision", async () => {
    vi.useFakeTimers();
    const h = harness();
    const good = h.coordinator.request(session("rev-good"));
    await flushDebounce();
    h.watches[0]!.settle(result("success", "build-good"));
    await good;

    const manyErrors = Array.from({ length: 80 }, (_, index) => ({
      code: `E_${index}`,
      stageName: "stage",
      instanceId: "instance",
      message: "x".repeat(4_000),
      recoverable: false,
      fields: { secret: "must not escape" },
    }));
    const failed = h.coordinator.request(session("rev-bad"));
    await flushDebounce();
    h.watches[1]!.settle(result("failed", "build-bad", { errors: manyErrors }));
    const attempt = await failed;

    expect(attempt.outcome).toBe("failed");
    expect(attempt.diagnostics).toHaveLength(64);
    expect(attempt.diagnostics[0]).toEqual({
      severity: "error",
      code: "E_0",
      stageName: "stage",
      instanceId: "instance",
      message: "x".repeat(2_048),
    });
    expect(JSON.stringify(attempt)).not.toContain("secret");
    expect(h.successes).toHaveLength(1);
    expect(h.failures[0]).toMatchObject({ revision: "rev-bad" });
    expect(h.coordinator.state).toMatchObject({
      phase: "failed",
      activeRevision: "rev-bad",
      lastGoodRevision: "rev-good",
      lastGoodBuildId: "build-good",
    });
  });

  it("refuses malformed artifacts without replacing last-good output", async () => {
    vi.useFakeTimers();
    const h = harness();
    const good = h.coordinator.request(session("rev-good"));
    await flushDebounce();
    h.watches[0]!.settle(result("success", "build-good"));
    await good;

    const malformed = h.coordinator.request(session("rev-malformed"));
    await flushDebounce();
    h.watches[1]!.settle(result("success", "build-malformed", {
      outputs: { unsafe: { variant: { kind: "dist-tree" }, files: { "../escape": encoder.encode("x") } } },
    }));
    await expect(malformed).resolves.toMatchObject({
      outcome: "failed",
      revision: "rev-malformed",
      diagnostics: [{ code: "E_PREVIEW_OUTPUT" }],
    });
    expect(h.successes).toHaveLength(1);
    expect(h.coordinator.state).toMatchObject({ lastGoodRevision: "rev-good" });

    const accessorFiles = {};
    Object.defineProperty(accessorFiles, "index.html", {
      enumerable: true,
      get() { throw new Error("artifact secret"); },
    });
    const hostile = h.coordinator.request(session("rev-hostile-artifact"));
    await flushDebounce();
    h.watches[2]!.settle(result("success", "build-hostile-artifact", {
      outputs: { site: { variant: { kind: "dist-tree" }, files: accessorFiles } },
    }));
    await expect(hostile).resolves.toMatchObject({
      outcome: "failed",
      diagnostics: [{ code: "E_PREVIEW_OUTPUT" }],
    });
    expect(JSON.stringify(await hostile)).not.toContain("secret");
  });

  it("copies artifact bytes, exposes a non-mutating view, and enforces byte limits", async () => {
    vi.useFakeTimers();
    const h = harness();
    const producerBytes = encoder.encode("stable");
    const pending = h.coordinator.request(session("rev-copy"));
    await flushDebounce();
    h.watches[0]!.settle(result("success", "build-copy", {
      outputs: { site: { variant: { kind: "dist-tree" }, files: { "index.html": producerBytes } } },
    }));
    await expect(pending).resolves.toMatchObject({ outcome: "ready" });
    producerBytes.fill(0);
    const published = h.successes[0]!.files;
    expect(typeof (published as unknown as { set?: unknown }).set).toBe("undefined");
    expect(published.size).toBe(1);
    expect(published.has("index.html")).toBe(true);
    expect(published.has("missing.html")).toBe(false);
    expect([...published.keys()]).toEqual(["index.html"]);
    expect([...published.values()].map(bytes => new TextDecoder().decode(bytes))).toEqual(["stable"]);
    expect([...published.entries()].map(([path]) => path)).toEqual(["index.html"]);
    expect([...published].map(([path]) => path)).toEqual(["index.html"]);
    const visited: string[] = [];
    published.forEach((_bytes, path, map) => {
      expect(map).toBe(published);
      visited.push(path);
    });
    expect(visited).toEqual(["index.html"]);
    expect(published.get("missing.html")).toBeUndefined();
    const firstRead = published.get("index.html")!;
    firstRead.fill(0);
    expect(new TextDecoder().decode(published.get("index.html"))).toBe("stable");

    const spoofedBytes = new Uint8Array(16 * 1024 * 1024 + 1);
    Object.defineProperty(spoofedBytes, "byteLength", { value: 1 });
    const oversized = h.coordinator.request(session("rev-oversized"));
    await flushDebounce();
    h.watches[1]!.settle(result("success", "build-oversized", {
      outputs: {
        site: {
          variant: { kind: "dist-tree" },
          files: { "large.bin": spoofedBytes },
        },
      },
    }));
    await expect(oversized).resolves.toMatchObject({
      outcome: "failed",
      diagnostics: [{ code: "E_PREVIEW_OUTPUT" }],
    });
    expect(h.successes).toHaveLength(1);
  });

  it("rejects non-portable paths, folded and prefix collisions, and proxied output tables", async () => {
    vi.useFakeTimers();
    const h = harness();
    const byte = encoder.encode("x");
    const proxiedBytes = new Proxy(byte, {});
    const invalidOutputs: RunResult["outputs"][] = [
      { site: { variant: { kind: "dist-tree" }, files: { "/absolute.txt": byte } } },
      { site: { variant: { kind: "dist-tree" }, files: { "dir\\file.txt": byte } } },
      { site: { variant: { kind: "dist-tree" }, files: { "CON.txt": byte } } },
      { site: { variant: { kind: "dist-tree" }, files: { "bad:name": byte } } },
      { site: { variant: { kind: "dist-tree" }, files: { "trailing.": byte } } },
      { site: { variant: { kind: "dist-tree" }, files: { "__proto__/x": byte } } },
      { site: { variant: { kind: "dist-tree" }, files: { "bad\ud800.txt": byte } } },
      { site: { variant: { kind: "dist-tree" }, files: { "wrong.bin": "not bytes" } } },
      { site: { variant: { kind: "dist-tree" }, files: { "proxy.bin": proxiedBytes } } },
      { site: { variant: { kind: "dist-tree" }, files: { "A.txt": byte, "a.txt": byte } } },
      { site: { variant: { kind: "dist-tree" }, files: { assets: byte, "assets/app.js": byte } } },
      { site: { variant: { kind: "dist-tree" }, files: { a: byte, "a-0": byte, "a/x": byte } } },
      new Proxy({}, { ownKeys() { throw new Error("proxy secret"); } }),
    ];
    for (const [index, outputs] of invalidOutputs.entries()) {
      const pending = h.coordinator.request(session(`rev-portable-${index}`));
      await flushDebounce();
      h.watches[index]!.settle(result("success", `build-portable-${index}`, { outputs }));
      const attempt = await pending;
      expect(attempt).toMatchObject({ outcome: "failed", diagnostics: [{ code: "E_PREVIEW_OUTPUT" }] });
      expect(JSON.stringify(attempt)).not.toContain("secret");
    }
    expect(h.successes).toEqual([]);
  });

  it("fails closed on hostile diagnostics and never exposes thrown adapter text", async () => {
    vi.useFakeTimers();
    const h = harness();
    const hostileErrors: RunResult["errors"] = [];
    Object.defineProperty(hostileErrors, "0", { enumerable: true, get() { throw new Error("adapter secret"); } });
    Object.defineProperty(hostileErrors, "length", { value: 1 });
    const pending = h.coordinator.request(session("rev-hostile"));
    await flushDebounce();
    h.watches[0]!.settle(result("failed", "build-hostile", { errors: hostileErrors }));
    const attempt = await pending;
    expect(attempt).toMatchObject({
      outcome: "failed",
      diagnostics: [{ code: "E_PREVIEW_DIAGNOSTIC", message: "Preview diagnostics could not be inspected safely." }],
    });
    expect(JSON.stringify(attempt)).not.toContain("secret");
  });

  it("redacts preparation and publisher failures and releases each materialization once", async () => {
    vi.useFakeTimers();
    const prepareFailure = createAuthoringPreview({
      debounceMs: 0,
      orchestrator: { watch() { throw new Error("watch must not run"); } },
      materializer: { async prepare() { throw new Error("filesystem secret"); } },
      publisher: {
        publish(_snapshot, commit) { commit(() => {}); },
        publishFailure(_failure, commit) { commit(() => {}); },
      },
    });
    const pending = prepareFailure.request(session("rev-prepare"));
    await flushDebounce();
    await expect(pending).resolves.toMatchObject({
      outcome: "failed",
      diagnostics: [{ code: "E_PREVIEW_PREPARE", message: "Preview input could not be prepared." }],
    });

    const h = harness();
    const publishingCoordinator = createAuthoringPreview({
      debounceMs: 0,
      orchestrator: {
        watch(pipeline, options) {
          h.watchCalls.push({ pipeline, options });
          const controlled = new ControlledWatch();
          h.watches.push(controlled);
          return controlled;
        },
      },
      materializer: {
        async prepare(input) {
          h.inputs.push(input);
          const release = vi.fn(async () => {});
          h.releases.push(release);
          return { pipeline: {} as Pipeline, release };
        },
      },
      publisher: {
        publish() { throw new Error("publisher secret"); },
        publishFailure(_failure, commit) { commit(() => {}); },
      },
    });
    const publishing = publishingCoordinator.request(session("rev-publish"));
    await flushDebounce();
    h.watches[0]!.settle(result("success", "build-publish"));
    await expect(publishing).resolves.toMatchObject({
      outcome: "failed",
      diagnostics: [{ code: "E_PREVIEW_PUBLISH", message: "Preview output could not be published." }],
    });
    expect(h.releases[0]).toHaveBeenCalledTimes(1);
  });

  it("reports cancellation without publishing a failure", async () => {
    vi.useFakeTimers();
    const h = harness();
    const pending = h.coordinator.request(session("rev-cancelled"));
    await flushDebounce();
    h.watches[0]!.settle(result("cancelled", "build-cancelled"));
    await expect(pending).resolves.toMatchObject({ outcome: "cancelled", revision: "rev-cancelled" });
    expect(h.successes).toEqual([]);
    expect(h.failures).toEqual([]);
    expect(h.coordinator.state).toMatchObject({
      phase: "idle",
      activeRevision: "rev-cancelled",
      diagnostics: [],
    });
  });

  it("disposes idempotently, releases active work, and rejects later requests", async () => {
    vi.useFakeTimers();
    const h = harness();
    const pending = h.coordinator.request(session("rev-active"));
    await flushDebounce();
    const firstDispose = h.coordinator.dispose();
    const secondDispose = h.coordinator.dispose();
    expect(secondDispose).toBe(firstDispose);
    await firstDispose;
    await expect(pending).resolves.toMatchObject({ outcome: "cancelled", revision: "rev-active" });
    expect(h.watches[0]!.stop).toHaveBeenCalledTimes(1);
    expect(h.releases[0]).toHaveBeenCalledTimes(1);
    expect(h.coordinator.state.phase).toBe("disposed");
    await expect(h.coordinator.request(session("rev-late"))).rejects.toThrow("disposed");
  });

  it("reports failed final retirement in disposed state without waiting for the watch stream", async () => {
    vi.useFakeTimers();
    const coordinator = createAuthoringPreview({
      debounceMs: 0,
      orchestrator: {
        watch() {
          return {
            results() {
              return { [Symbol.asyncIterator]: () => ({ next: () => new Promise<IteratorResult<RunResult>>(() => {}) }) };
            },
            async rebuild() { return result("cancelled", "unused"); },
            async stop() {},
          };
        },
      },
      materializer: {
        async prepare() {
          return { pipeline: {} as Pipeline, async release() { throw new Error("dispose retirement secret"); } };
        },
      },
      publisher: {
        publish(_snapshot, commit) { commit(() => {}); },
        publishFailure(_failure, commit) { commit(() => {}); },
      },
    });
    const pending = coordinator.request(session("rev-dispose-retirement"));
    await flushDebounce();
    await coordinator.dispose();
    await expect(pending).resolves.toMatchObject({ outcome: "cancelled" });
    expect(coordinator.state).toMatchObject({
      phase: "disposed",
      activeRevision: null,
      diagnostics: [{ code: "E_PREVIEW_RELEASE" }],
    });
    expect(JSON.stringify(coordinator.state)).not.toContain("secret");
  });

  it("rejects unsafe revisions and invalid debounce bounds before host work", async () => {
    const h = harness();
    await expect(h.coordinator.request(session("bad\u202erevision"))).rejects.toThrow("could not be inspected safely");
    expect(h.inputs).toEqual([]);
    expect(() => harness(60_001)).toThrow("debounceMs");
  });

  it("rejects over-limit, empty, and malformed exact identities instead of truncating them", async () => {
    vi.useFakeTimers();
    const h = harness();
    await expect(h.coordinator.request(session("r".repeat(1_025), "Valid title"))).rejects.toThrow(
      "could not be inspected safely",
    );
    await expect(h.coordinator.request(session("\ud800", "Valid title"))).rejects.toThrow(
      "could not be inspected safely",
    );
    expect(h.inputs).toEqual([]);

    for (const [index, buildId] of ["b".repeat(1_025), "", "\ud800"].entries()) {
      const pending = h.coordinator.request(session(`rev-build-id-${index}`));
      await flushDebounce();
      h.watches[index]!.settle(result("success", buildId));
      await expect(pending).resolves.toMatchObject({
        outcome: "failed",
        diagnostics: [{ code: "E_PREVIEW_OUTPUT" }],
      });
    }
  });

  it("validates construction and captures only data methods", async () => {
    const valid = {
      orchestrator: { watch() { return immediateWatch({ done: true, value: undefined }); } },
      materializer: { async prepare() { return { pipeline: {} as Pipeline, async release() {} }; } },
      publisher: {
        publish(_snapshot, commit) { commit(() => {}); },
        publishFailure(_failure, commit) { commit(() => {}); },
      },
    };
    expect(() => createAuthoringPreview(null as never)).toThrow("options");
    expect(() => createAuthoringPreview({ ...valid, orchestrator: null as never })).toThrow("adapter methods");
    expect(() => createAuthoringPreview({ ...valid, publisher: {} as never })).toThrow("adapter methods");
    const accessor = {} as Record<string, unknown>;
    Object.defineProperty(accessor, "prepare", { get() { return async () => {}; } });
    expect(() => createAuthoringPreview({ ...valid, materializer: accessor as never })).toThrow("adapter methods");
    for (const debounceMs of [-1, 0.5, Number.NaN]) {
      expect(() => createAuthoringPreview({ ...valid, debounceMs })).toThrow("debounceMs");
    }
    const withDefault = createAuthoringPreview(valid);
    await withDefault.dispose();
  });

  it("invokes captured cleanup functions through intrinsics instead of shadowed call helpers", async () => {
    vi.useFakeTimers();
    const release = vi.fn(async () => {});
    const stop = vi.fn(async () => {});
    const results = vi.fn(() => ({
      [Symbol.asyncIterator]: () => ({
        next: async () => ({ done: false, value: result("success", "build-intrinsic") }),
      }),
    }));
    Object.defineProperty(release, "call", { value: () => Promise.resolve() });
    Object.defineProperty(stop, "bind", { value: () => () => Promise.resolve() });
    Object.defineProperty(results, "bind", { value: () => () => undefined });
    const coordinator = createAuthoringPreview({
      debounceMs: 0,
      orchestrator: {
        watch() {
          return { results, stop, async rebuild() { return result("cancelled", "unused"); } };
        },
      },
      materializer: { async prepare() { return { pipeline: {} as Pipeline, release }; } },
      publisher: {
        publish(_snapshot, commit) { commit(() => {}); },
        publishFailure(_failure, commit) { commit(() => {}); },
      },
    });
    const pending = coordinator.request(session("rev-intrinsic"));
    await flushDebounce();
    await expect(pending).resolves.toMatchObject({ outcome: "ready", buildId: "build-intrinsic" });
    expect(results).toHaveBeenCalledOnce();
    expect(stop).toHaveBeenCalledOnce();
    expect(release).toHaveBeenCalledOnce();
  });

  it("redacts hostile session and adapter inspection errors", async () => {
    const h = harness();
    await expect(h.coordinator.request({ project: session("present").project } as AuthoringSession)).rejects.toThrow(
      "authoring session could not be inspected safely",
    );
    const hostileSession = {};
    Object.defineProperty(hostileSession, "project", {
      enumerable: true,
      get() { throw new Error("session secret"); },
    });
    await expect(h.coordinator.request(hostileSession as AuthoringSession)).rejects.toThrow(
      "authoring session could not be inspected safely",
    );

    const hostileAdapter = new Proxy({}, {
      getOwnPropertyDescriptor() { throw new Error("adapter secret"); },
    });
    expect(() => createAuthoringPreview({
      debounceMs: 0,
      orchestrator: hostileAdapter as never,
      materializer: { async prepare() { throw new Error("unused"); } },
      publisher: {
        publish(_snapshot, commit) { commit(() => {}); },
        publishFailure(_failure, commit) { commit(() => {}); },
      },
    })).toThrow("preview adapter methods could not be inspected safely");

    const hostileOptions = new Proxy({}, {
      getOwnPropertyDescriptor() { throw new Error("options secret"); },
    });
    expect(() => createAuthoringPreview(hostileOptions as never)).toThrow(
      "preview options could not be inspected safely",
    );
  });

  it("cancels a request that is still waiting in the debounce window", async () => {
    vi.useFakeTimers();
    const h = harness(100);
    const pending = h.coordinator.request(session("rev-pending"));
    await h.coordinator.dispose();
    await expect(pending).resolves.toMatchObject({ outcome: "cancelled", revision: "rev-pending" });
    expect(h.inputs).toEqual([]);
  });

  it("rejects malformed materializer results before starting watch", async () => {
    vi.useFakeTimers();
    const malformed: unknown[] = [
      null,
      {},
      { pipeline: null, async release() {} },
      { pipeline: {}, release: "no" },
      { pipeline: {}, async release() {}, extra: true },
      Object.defineProperty({ async release() {} }, "pipeline", { enumerable: true, get() { return {}; } }),
      Object.assign(Object.create(null), { pipeline: {}, async release() {} }),
      new Proxy({ pipeline: {}, async release() {} }, { ownKeys() { throw new Error("proxy secret"); } }),
    ];
    for (const prepared of malformed) {
      let watched = false;
      const coordinator = createAuthoringPreview({
        debounceMs: 0,
        orchestrator: { watch() { watched = true; return immediateWatch({ done: true, value: undefined }); } },
        materializer: { async prepare() { return prepared as PreparedAuthoringPreview; } },
        publisher: {
          publish(_snapshot, commit) { commit(() => {}); },
          publishFailure(_failure, commit) { commit(() => {}); },
        },
      });
      const pending = coordinator.request(session(`rev-${malformed.indexOf(prepared)}`));
      await flushDebounce();
      await expect(pending).resolves.toMatchObject({
        outcome: "failed",
        diagnostics: [{ code: "E_PREVIEW_PREPARE" }],
      });
      expect(watched).toBe(false);
    }
  });

  it("releases preparation that resolves after its request was superseded", async () => {
    vi.useFakeTimers();
    let resolvePrepare!: (value: PreparedAuthoringPreview) => void;
    let prepareCalls = 0;
    const release = vi.fn(async () => {});
    const coordinator = createAuthoringPreview({
      debounceMs: 0,
      orchestrator: { watch() { return immediateWatch({ done: true, value: undefined }); } },
      materializer: {
        prepare() {
          prepareCalls += 1;
          if (prepareCalls > 1) return Promise.resolve({ pipeline: {} as Pipeline, async release() {} });
          return new Promise(resolve => { resolvePrepare = resolve; });
        },
      },
      publisher: {
        publish(_snapshot, commit) { commit(() => {}); },
        publishFailure(_failure, commit) { commit(() => {}); },
      },
    });
    const first = coordinator.request(session("rev-old"));
    await flushDebounce();
    const second = coordinator.request(session("rev-new"));
    resolvePrepare({ pipeline: {} as Pipeline, release });
    await flushDebounce();
    await expect(first).resolves.toMatchObject({ outcome: "superseded" });
    expect(release).toHaveBeenCalledTimes(1);
    await expect(second).resolves.toMatchObject({ outcome: "failed" });
    await coordinator.dispose();
  });

  it("ignores malformed preparation that resolves after supersession", async () => {
    vi.useFakeTimers();
    let resolvePrepare!: (value: PreparedAuthoringPreview) => void;
    let prepareCalls = 0;
    const coordinator = createAuthoringPreview({
      debounceMs: 0,
      orchestrator: { watch() { return immediateWatch({ done: true, value: undefined }); } },
      materializer: {
        prepare() {
          prepareCalls += 1;
          if (prepareCalls > 1) return Promise.resolve({ pipeline: {} as Pipeline, async release() {} });
          return new Promise(resolve => { resolvePrepare = resolve; });
        },
      },
      publisher: {
        publish(_snapshot, commit) { commit(() => {}); },
        publishFailure(_failure, commit) { commit(() => {}); },
      },
    });
    const first = coordinator.request(session("rev-old-malformed"));
    await flushDebounce();
    const second = coordinator.request(session("rev-new-after-malformed"));
    resolvePrepare({ pipeline: null, release: "invalid" } as never);
    await flushDebounce();
    await expect(first).resolves.toMatchObject({ outcome: "superseded" });
    await expect(second).resolves.toMatchObject({ outcome: "failed" });
    await coordinator.dispose();
  });

  it("maps watch startup, stream, empty-stream, and malformed-result failures", async () => {
    vi.useFakeTimers();
    const cases: Array<{ readonly expected: string; readonly watch: () => WatchSession }> = [
      { expected: "started", watch: () => { throw new Error("start secret"); } },
      { expected: "started", watch: () => ({ results() {}, rebuild() {}, get stop() { throw new Error("stop getter secret"); } }) as never },
      { expected: "started", watch: () => ({ results() {}, rebuild() {} }) as never },
      { expected: "complete", watch: () => immediateWatch(new Error("stream secret")) },
      { expected: "without a result", watch: () => immediateWatch({ done: true, value: undefined }) },
      { expected: "invalid result", watch: () => immediateWatch({ done: false, value: {} as RunResult }) },
      { expected: "invalid result", watch: () => immediateWatch({
        done: false,
        value: { ...result("success", "build"), outcome: "bogus" } as never,
      }) },
      { expected: "invalid", watch: () => immediateWatch({
        done: false,
        value: result("success", "bad\u202eid"),
      }) },
    ];
    for (const [index, item] of cases.entries()) {
      const failures: Array<{ readonly diagnostics: readonly { readonly message: string }[] }> = [];
      const coordinator = createAuthoringPreview({
        debounceMs: 0,
        orchestrator: { watch: item.watch },
        materializer: { async prepare() { return { pipeline: {} as Pipeline, async release() {} }; } },
        publisher: {
          publish(_snapshot, commit) { commit(() => {}); },
          publishFailure(failure, commit) { commit(() => { failures.push(failure); }); },
        },
      });
      const pending = coordinator.request(session(`rev-watch-${index}`));
      await flushDebounce();
      const attempt = await pending;
      expect(attempt.outcome).toBe("failed");
      expect(failures[0]!.diagnostics[0]!.message).toContain(item.expected);
      expect(JSON.stringify(attempt)).not.toContain("secret");
    }
  });

  it("settles and releases when iterator descriptors are hostile and stop throws synchronously", async () => {
    vi.useFakeTimers();
    const release = vi.fn(async () => {});
    const failures: string[] = [];
    const hostileResult = {};
    Object.defineProperty(hostileResult, "done", { enumerable: true, get() { throw new Error("iterator secret"); } });
    const coordinator = createAuthoringPreview({
      debounceMs: 0,
      orchestrator: {
        watch() {
          return {
            results() {
              return { [Symbol.asyncIterator]: () => ({ next: async () => hostileResult }) };
            },
            rebuild: async () => result("cancelled", "unused"),
            stop() { throw new Error("stop secret"); },
          } as WatchSession;
        },
      },
      materializer: { async prepare() { return { pipeline: {} as Pipeline, release }; } },
      publisher: {
        publish(_snapshot, commit) { commit(() => {}); },
        publishFailure(failure, commit) {
          commit(() => { failures.push(failure.diagnostics[0]!.message); });
        },
      },
    });
    const pending = coordinator.request(session("rev-hostile-iterator"));
    await flushDebounce();
    const attempt = await pending;
    expect(attempt).toMatchObject({ outcome: "failed", diagnostics: [{ code: "E_PREVIEW_CLEANUP" }] });
    expect(JSON.stringify(attempt)).not.toContain("secret");
    expect(failures[0]).toBe("Preview pipeline could not be retired safely.");
    expect(release).toHaveBeenCalledTimes(1);
  });

  it("fails closed when cleanup or failure publication does not complete safely", async () => {
    vi.useFakeTimers();
    const releaseCoordinator = createAuthoringPreview({
      debounceMs: 0,
      orchestrator: { watch() { return immediateWatch({ done: false, value: result("success", "build") }); } },
      materializer: {
        async prepare() { return { pipeline: {} as Pipeline, async release() { throw new Error("release secret"); } }; },
      },
      publisher: {
        publish() { throw new Error("must not publish"); },
        publishFailure(_failure, commit) { commit(() => {}); },
      },
    });
    const releasePending = releaseCoordinator.request(session("rev-release"));
    await flushDebounce();
    await expect(releasePending).resolves.toMatchObject({
      outcome: "failed",
      diagnostics: [{ code: "E_PREVIEW_RELEASE" }],
    });

    const failurePublisher = createAuthoringPreview({
      debounceMs: 0,
      orchestrator: { watch() { return immediateWatch({ done: false, value: result("failed", "build", { errors: [] }) }); } },
      materializer: { async prepare() { return { pipeline: {} as Pipeline, async release() {} }; } },
      publisher: {
        publish(_snapshot, commit) { commit(() => {}); },
        publishFailure() { throw new Error("failure publisher secret"); },
      },
    });
    const publishPending = failurePublisher.request(session("rev-failure-publish"));
    await flushDebounce();
    await expect(publishPending).resolves.toMatchObject({
      outcome: "failed",
      diagnostics: [{ code: "E_PREVIEW_PUBLISH" }],
    });
  });

  it("requires one synchronous guarded publisher commit and reports post-commit failure as indeterminate", async () => {
    vi.useFakeTimers();
    const cases: Array<{ readonly code: string; readonly publish: AuthoringPreviewPublisher["publish"] }> = [
      { code: "E_PREVIEW_PUBLISH", publish() {} },
      {
        code: "E_PREVIEW_PUBLISH_INDETERMINATE",
        publish(_snapshot, commit) {
          commit(() => {});
          throw new Error("after commit secret");
        },
      },
      {
        code: "E_PREVIEW_PUBLISH_INDETERMINATE",
        publish(_snapshot, commit) { commit(async () => {}); },
      },
    ];
    for (const [index, item] of cases.entries()) {
      const watch = new ControlledWatch();
      const coordinator = createAuthoringPreview({
        debounceMs: 0,
        orchestrator: { watch() { return watch; } },
        materializer: { async prepare() { return { pipeline: {} as Pipeline, async release() {} }; } },
        publisher: {
          publish: item.publish,
          publishFailure(_failure, commit) { commit(() => {}); },
        },
      });
      const pending = coordinator.request(session(`rev-guard-${index}`));
      await flushDebounce();
      watch.settle(result("success", `build-guard-${index}`));
      const attempt = await pending;
      expect(attempt).toMatchObject({ outcome: "failed", diagnostics: [{ code: item.code }] });
      expect(JSON.stringify(attempt)).not.toContain("secret");
      if (index === 1) {
        expect(coordinator.state).toMatchObject({
          phase: "failed",
          activeRevision: "rev-guard-1",
          lastGoodRevision: "rev-guard-1",
          lastGoodBuildId: "build-guard-1",
          diagnostics: [{ code: "E_PREVIEW_PUBLISH_INDETERMINATE" }],
        });
      }
    }
  });

  it("normalizes empty, malformed, and unsafe diagnostic fields", async () => {
    vi.useFakeTimers();
    const attempts = [
      result("failed", "bad\u202eid", { errors: [] }),
      { ...result("failed", "build"), errors: null as never },
      result("failed", "build", { errors: [{
        code: "bad\u202ecode",
        stageName: 7 as never,
        instanceId: "instance",
        message: "bad\u0000message",
        recoverable: false,
        fields: {},
      }] }),
    ];
    for (const [index, runResult] of attempts.entries()) {
      const coordinator = createAuthoringPreview({
        debounceMs: 0,
        orchestrator: { watch() { return immediateWatch({ done: false, value: runResult }); } },
        materializer: { async prepare() { return { pipeline: {} as Pipeline, async release() {} }; } },
        publisher: {
          publish(_snapshot, commit) { commit(() => {}); },
          publishFailure(_failure, commit) { commit(() => {}); },
        },
      });
      const pending = coordinator.request(session(`rev-diagnostic-${index}`));
      await flushDebounce();
      const attempt = await pending;
      expect(attempt.buildId).toBe(index === 0 ? null : "build");
      if (index === 0) expect(attempt.diagnostics[0]!.code).toBe("E_PREVIEW_BUILD");
      if (index === 1) expect(attempt.diagnostics[0]!.code).toBe("E_PREVIEW_DIAGNOSTIC");
      if (index === 2) expect(attempt.diagnostics[0]).toMatchObject({
        code: "E_PREVIEW_DIAGNOSTIC",
        stageName: "unknown-stage",
        message: "Preview build failed.",
      });
    }
  });

  it("closes the idle change stream used by the real watch boundary", async () => {
    vi.useFakeTimers();
    let changes: WatchOptions["changes"] | null = null;
    let iterator: AsyncIterator<unknown> | null = null;
    const coordinator = createAuthoringPreview({
      debounceMs: 0,
      orchestrator: {
        watch(_pipeline, options) {
          changes = options.changes;
          iterator = changes[Symbol.asyncIterator]();
          void iterator.next();
          return immediateWatch({ done: false, value: result("success", "build-idle") });
        },
      },
      materializer: { async prepare() { return { pipeline: {} as Pipeline, async release() {} }; } },
      publisher: {
        publish(_snapshot, commit) { commit(() => {}); },
        publishFailure(_failure, commit) { commit(() => {}); },
      },
    });
    const pending = coordinator.request(session("rev-idle"));
    await flushDebounce();
    await expect(pending).resolves.toMatchObject({ outcome: "ready" });
    expect(changes).not.toBeNull();
    await expect(iterator!.next()).resolves.toEqual({ done: true, value: undefined });
  });

  it("rejects null sessions and empty revisions without scheduling host work", async () => {
    const h = harness();
    await expect(h.coordinator.request(null as never)).rejects.toThrow("session");
    await expect(h.coordinator.request(session("", "Valid title"))).rejects.toThrow("could not be inspected safely");
    expect(h.inputs).toEqual([]);
  });
});
