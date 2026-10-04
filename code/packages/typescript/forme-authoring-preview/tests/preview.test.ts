import { createAuthoringProject, type AuthoringSession } from "@coding-adventures/forme-authoring-core";
import type { Pipeline, RunResult, WatchOptions, WatchSession } from "@coding-adventures/forme-orchestrator";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  createAuthoringPreview,
  type AuthoringPreviewInput,
  type AuthoringPreviewPublisher,
  type PreparedAuthoringPreview,
} from "../src/index.js";

const encoder = new TextEncoder();

function session(revision: string, title = revision): AuthoringSession {
  return {
    project: createAuthoringProject({ projectId: "project", title }),
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

function harness(debounceMs = 0) {
  const watches: ControlledWatch[] = [];
  const watchCalls: Array<{ readonly pipeline: Pipeline; readonly options: WatchOptions }> = [];
  const inputs: AuthoringPreviewInput[] = [];
  const releases: Array<ReturnType<typeof vi.fn>> = [];
  const successes: Array<{ readonly revision: string; readonly buildId: string; readonly files: ReadonlyMap<string, Uint8Array> }> = [];
  const failures: Array<{ readonly revision: string; readonly diagnostics: readonly unknown[] }> = [];
  const publisher: AuthoringPreviewPublisher = {
    publish(snapshot) { successes.push(snapshot); },
    publishFailure(failure) { failures.push(failure); },
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
      publisher: { publish() {}, publishFailure() {} },
    });
    const pending = prepareFailure.request(session("rev-prepare"));
    await flushDebounce();
    await expect(pending).resolves.toMatchObject({
      outcome: "failed",
      diagnostics: [{ code: "E_PREVIEW_PREPARE", message: "Preview input could not be prepared." }],
    });

    const h = harness();
    h.publisher.publish = () => { throw new Error("publisher secret"); };
    const publishing = h.coordinator.request(session("rev-publish"));
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
  });

  it("disposes idempotently, releases active work, and rejects later requests", async () => {
    vi.useFakeTimers();
    const h = harness();
    const pending = h.coordinator.request(session("rev-active"));
    await flushDebounce();
    await h.coordinator.dispose();
    await h.coordinator.dispose();
    await expect(pending).resolves.toMatchObject({ outcome: "cancelled", revision: "rev-active" });
    expect(h.watches[0]!.stop).toHaveBeenCalledTimes(1);
    expect(h.releases[0]).toHaveBeenCalledTimes(1);
    expect(h.coordinator.state.phase).toBe("disposed");
    await expect(h.coordinator.request(session("rev-late"))).rejects.toThrow("disposed");
  });

  it("rejects unsafe revisions and invalid debounce bounds before host work", async () => {
    const h = harness();
    await expect(h.coordinator.request(session("bad\u202erevision"))).rejects.toThrow("revision");
    expect(h.inputs).toEqual([]);
    expect(() => harness(60_001)).toThrow("debounceMs");
  });
});
