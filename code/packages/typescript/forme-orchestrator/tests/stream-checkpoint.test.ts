import { describe, expect, it, vi } from "vitest";
import {
  makeEntry,
  memoryCache,
  type CacheBackend,
  type CacheEntry,
} from "@coding-adventures/forme-cache";
import {
  createCancellationTokenSource,
  neverCancelledToken,
  silentLogger,
} from "@coding-adventures/forme-stage";
import { CancellationError } from "@coding-adventures/forme-errors";
import {
  decodeCacheValue,
  encodeCacheValue,
} from "../src/cache-codec.js";
import {
  commitStreamCheckpoint,
  createStreamCheckpointWriter,
  loadStreamCheckpoint,
  streamCheckpointNodeKey,
  streamCheckpointRevision,
  type StreamCheckpointManifest,
} from "../src/stream-checkpoint.js";

const CHECKPOINT_KEY = "ab".repeat(32);

describe("bounded stream checkpoints", () => {
  it.each([0, 1, 2, 3, 63, 64, 65, 257])(
    "round-trips %i ordered values with an O(log n) writer frontier",
    async itemCount => {
      const cache = memoryCache();
      const writer = createStreamCheckpointWriter(cache, neverCancelledToken());
      let largestFrontier = 0;
      const expected = Array.from({ length: itemCount }, (_, index) => ({
        index,
        bytes: new Uint8Array([index % 251]),
      }));

      for (const value of expected) {
        await writer.append(value);
        largestFrontier = Math.max(largestFrontier, writer.retainedSubtrees);
      }
      const manifest = await writer.finalize();
      await commitStreamCheckpoint(cache, CHECKPOINT_KEY, manifest);

      const restored = await loadStreamCheckpoint(
        cache,
        CHECKPOINT_KEY,
        manifest.outputRevision,
        neverCancelledToken(),
        silentLogger(),
      );
      expect(restored).not.toBeNull();
      expect(restored!.itemCount).toBe(itemCount);
      expect(await collect(restored!.values)).toEqual(expected);
      expect(largestFrontier).toBeLessThanOrEqual(
        itemCount === 0 ? 0 : Math.floor(Math.log2(itemCount)) + 1,
      );
      await cache.dispose();
    },
  );

  it("deduplicates equal content without losing duplicate positions", async () => {
    const cache = countingCache(memoryCache());
    const writer = createStreamCheckpointWriter(cache, neverCancelledToken());
    for (let index = 0; index < 96; index++) await writer.append({ value: "same" });
    const manifest = await writer.finalize();
    await commitStreamCheckpoint(cache, CHECKPOINT_KEY, manifest);

    const restored = await loadStreamCheckpoint(
      cache,
      CHECKPOINT_KEY,
      manifest.outputRevision,
      neverCancelledToken(),
      silentLogger(),
    );
    expect(await collect(restored!.values)).toHaveLength(96);
    // Ninety-six logical leaves share one content-addressed leaf entry. Branch
    // shapes are shared too, so physical writes stay far below item count.
    expect(cache.uniquePutKeys.size).toBeLessThan(20);
    await cache.dispose();
  });

  it("validates first and then reads values lazily", async () => {
    const cache = countingCache(memoryCache());
    const writer = createStreamCheckpointWriter(cache, neverCancelledToken());
    for (let index = 0; index < 129; index++) await writer.append(index);
    const manifest = await writer.finalize();
    await commitStreamCheckpoint(cache, CHECKPOINT_KEY, manifest);

    const restored = await loadStreamCheckpoint(
      cache,
      CHECKPOINT_KEY,
      manifest.outputRevision,
      neverCancelledToken(),
      silentLogger(),
    );
    expect(cache.getCalls).toBeGreaterThan(129);

    cache.getCalls = 0;
    const iterator = restored!.values[Symbol.asyncIterator]();
    await expect(iterator.next()).resolves.toEqual({ done: false, value: 0 });
    expect(cache.getCalls).toBeLessThan(12);
    await iterator.return?.();
    await cache.dispose();
  });

  it("invalidates the manifest when a referenced node is missing", async () => {
    const cache = memoryCache();
    const manifest = await writeRange(cache, 5);
    await commitStreamCheckpoint(cache, CHECKPOINT_KEY, manifest);
    await cache.invalidate(manifest.rootKey!);

    expect(await loadStreamCheckpoint(
      cache,
      CHECKPOINT_KEY,
      manifest.outputRevision,
      neverCancelledToken(),
      silentLogger(),
    )).toBeNull();
    expect(await cache.get(CHECKPOINT_KEY)).toBeNull();
    await cache.dispose();
  });

  it("rejects a node stored under a key that does not match its content", async () => {
    const cache = memoryCache();
    const manifest = await writeRange(cache, 4);
    await commitStreamCheckpoint(cache, CHECKPOINT_KEY, manifest);
    await cache.put(
      manifest.rootKey!,
      makeEntry(encodeCacheValue({
        schema: "forme-stream-checkpoint-node-v1",
        type: "leaf",
        count: 1,
        value: "replacement",
      })),
    );

    expect(await loadStreamCheckpoint(
      cache,
      CHECKPOINT_KEY,
      manifest.outputRevision,
      neverCancelledToken(),
      silentLogger(),
    )).toBeNull();
    expect(await cache.get(CHECKPOINT_KEY)).toBeNull();
    await cache.dispose();
  });

  it("rejects a content-addressed but non-canonical tree shape", async () => {
    const cache = memoryCache();
    const leafPayload = (value: number) => encodeCacheValue({
      schema: "forme-stream-checkpoint-node-v1",
      type: "leaf",
      count: 1,
      value,
    });
    const leaves = await Promise.all([0, 1, 2].map(async value => {
      const payload = leafPayload(value);
      const key = streamCheckpointNodeKey(payload);
      await cache.put(key, makeEntry(payload));
      return key;
    }));
    const rightPayload = encodeCacheValue({
      schema: "forme-stream-checkpoint-node-v1",
      type: "branch",
      count: 2,
      left: { key: leaves[1], count: 1 },
      right: { key: leaves[2], count: 1 },
    });
    const rightKey = streamCheckpointNodeKey(rightPayload);
    await cache.put(rightKey, makeEntry(rightPayload));
    // Count three canonically requires a two-value left subtree and one-value
    // right subtree. This root encodes the opposite shape with valid hashes.
    const rootPayload = encodeCacheValue({
      schema: "forme-stream-checkpoint-node-v1",
      type: "branch",
      count: 3,
      left: { key: leaves[0], count: 1 },
      right: { key: rightKey, count: 2 },
    });
    const rootKey = streamCheckpointNodeKey(rootPayload);
    await cache.put(rootKey, makeEntry(rootPayload));
    const manifest: StreamCheckpointManifest = {
      schema: "forme-stream-checkpoint-v1",
      rootKey,
      itemCount: 3,
      outputRevision: streamCheckpointRevision(rootKey, 3),
    };
    await commitStreamCheckpoint(cache, CHECKPOINT_KEY, manifest);

    expect(await loadStreamCheckpoint(
      cache,
      CHECKPOINT_KEY,
      manifest.outputRevision,
      neverCancelledToken(),
      silentLogger(),
    )).toBeNull();
    await cache.dispose();
  });

  it("fails open on a mismatched expected output revision", async () => {
    const cache = memoryCache();
    const manifest = await writeRange(cache, 3);
    await commitStreamCheckpoint(cache, CHECKPOINT_KEY, manifest);

    expect(await loadStreamCheckpoint(
      cache,
      CHECKPOINT_KEY,
      "blake2b:" + "00".repeat(32) as never,
      neverCancelledToken(),
      silentLogger(),
    )).toBeNull();
    expect(await cache.get(CHECKPOINT_KEY)).toBeNull();
    await cache.dispose();
  });

  it("rejects a manifest revision that does not commit to its root", async () => {
    const cache = memoryCache();
    const written = await writeRange(cache, 3);
    const dishonest: StreamCheckpointManifest = {
      ...written,
      outputRevision: streamCheckpointRevision(null, 3),
    };
    await commitStreamCheckpoint(cache, CHECKPOINT_KEY, dishonest);

    expect(await loadStreamCheckpoint(
      cache,
      CHECKPOINT_KEY,
      dishonest.outputRevision,
      neverCancelledToken(),
      silentLogger(),
    )).toBeNull();
    await cache.dispose();
  });

  it.each([
    ["unsupported schema", { schema: "other", rootKey: null, itemCount: 0, outputRevision: "blake2b:" + "00".repeat(32) }],
    ["malformed root", { schema: "forme-stream-checkpoint-v1", rootKey: "not-a-key", itemCount: 1, outputRevision: "blake2b:" + "00".repeat(32) }],
    ["malformed count", { schema: "forme-stream-checkpoint-v1", rootKey: null, itemCount: -1, outputRevision: "blake2b:" + "00".repeat(32) }],
    ["malformed revision", { schema: "forme-stream-checkpoint-v1", rootKey: null, itemCount: 0, outputRevision: "wrong" }],
  ])("fails open on a %s manifest", async (_label, rawManifest) => {
    const cache = memoryCache();
    await cache.put(CHECKPOINT_KEY, makeEntry(encodeCacheValue(rawManifest)));
    expect(await loadStreamCheckpoint(
      cache,
      CHECKPOINT_KEY,
      "blake2b:" + "00".repeat(32) as never,
      neverCancelledToken(),
      silentLogger(),
    )).toBeNull();
    await cache.dispose();
  });

  it("rejects inconsistent empty/non-empty root metadata", async () => {
    const cache = memoryCache();
    const fakeRoot = "01".repeat(32);
    for (const manifest of [
      {
        schema: "forme-stream-checkpoint-v1" as const,
        rootKey: null,
        itemCount: 1,
        outputRevision: streamCheckpointRevision(null, 1),
      },
      {
        schema: "forme-stream-checkpoint-v1" as const,
        rootKey: fakeRoot,
        itemCount: 0,
        outputRevision: streamCheckpointRevision(fakeRoot, 0),
      },
    ]) {
      await commitStreamCheckpoint(cache, CHECKPOINT_KEY, manifest);
      expect(await loadStreamCheckpoint(
        cache,
        CHECKPOINT_KEY,
        manifest.outputRevision,
        neverCancelledToken(),
        silentLogger(),
      )).toBeNull();
    }
    await cache.dispose();
  });

  it.each([
    ["unsupported node schema", { schema: "other", type: "leaf", count: 1, value: 0 }, 1],
    ["malformed leaf", { schema: "forme-stream-checkpoint-node-v1", type: "leaf", count: 2, value: 0 }, 2],
    ["unsupported node type", { schema: "forme-stream-checkpoint-node-v1", type: "other", count: 1, value: 0 }, 1],
    ["malformed branch count", { schema: "forme-stream-checkpoint-node-v1", type: "branch", count: 0, left: {}, right: {} }, 2],
    ["malformed child reference", {
      schema: "forme-stream-checkpoint-node-v1",
      type: "branch",
      count: 2,
      left: { key: "bad", count: 1 },
      right: { key: "01".repeat(32), count: 1 },
    }, 2],
  ])("fails open on an %s", async (_label, rawNode, itemCount) => {
    const cache = memoryCache();
    const payload = encodeCacheValue(rawNode);
    const rootKey = streamCheckpointNodeKey(payload);
    await cache.put(rootKey, makeEntry(payload));
    const manifest: StreamCheckpointManifest = {
      schema: "forme-stream-checkpoint-v1",
      rootKey,
      itemCount,
      outputRevision: streamCheckpointRevision(rootKey, itemCount),
    };
    await commitStreamCheckpoint(cache, CHECKPOINT_KEY, manifest);

    expect(await loadStreamCheckpoint(
      cache,
      CHECKPOINT_KEY,
      manifest.outputRevision,
      neverCancelledToken(),
      silentLogger(),
    )).toBeNull();
    await cache.dispose();
  });

  it("reports but still fails open when manifest invalidation fails", async () => {
    const inner = memoryCache();
    await inner.put(CHECKPOINT_KEY, makeEntry(encodeCacheValue({ schema: "wrong" })));
    const logger = { ...silentLogger(), warn: vi.fn() };
    const cache: CacheBackend = {
      ...delegatingCache(inner),
      async invalidate() { throw new Error("read-only cache"); },
    };

    expect(await loadStreamCheckpoint(
      cache,
      CHECKPOINT_KEY,
      "blake2b:" + "00".repeat(32) as never,
      neverCancelledToken(),
      logger,
    )).toBeNull();
    expect(logger.warn).toHaveBeenCalledWith(
      "invalid stream checkpoint manifest could not be invalidated",
      expect.objectContaining({ invalidateError: "Error: read-only cache" }),
    );
    await inner.dispose();
  });

  it("validates public revision arguments", () => {
    expect(() => streamCheckpointRevision(null, -1)).toThrow("non-negative safe integer");
    expect(() => streamCheckpointRevision("bad", 1)).toThrow("root key is malformed");
  });

  it("rejects path-shaped manifest keys before any backend operation", async () => {
    const backend = {
      get: vi.fn(),
      put: vi.fn(),
      invalidate: vi.fn(),
      gc: vi.fn(),
      dispose: vi.fn(),
    } as unknown as CacheBackend;
    const writerCache = memoryCache();
    const manifest = await writeRange(writerCache, 0);

    await expect(commitStreamCheckpoint(backend, "../escape", manifest))
      .rejects.toThrow("64-character lowercase hexadecimal digest");
    await expect(loadStreamCheckpoint(
      backend,
      "../escape",
      manifest.outputRevision,
      neverCancelledToken(),
      silentLogger(),
    )).rejects.toThrow("64-character lowercase hexadecimal digest");
    expect(backend.get).not.toHaveBeenCalled();
    expect(backend.put).not.toHaveBeenCalled();
    expect(backend.invalidate).not.toHaveBeenCalled();
    await writerCache.dispose();
  });

  it("propagates cancellation without invalidating a valid manifest", async () => {
    const inner = memoryCache();
    const manifest = await writeRange(inner, 32);
    await commitStreamCheckpoint(inner, CHECKPOINT_KEY, manifest);
    const cancellation = createCancellationTokenSource();
    let reads = 0;
    const cache: CacheBackend = {
      ...delegatingCache(inner),
      async get(key) {
        if (++reads === 5) cancellation.cancel("test cancellation");
        return inner.get(key);
      },
    };

    await expect(loadStreamCheckpoint(
      cache,
      CHECKPOINT_KEY,
      manifest.outputRevision,
      cancellation.token,
      silentLogger(),
    )).rejects.toBeInstanceOf(CancellationError);
    expect(await inner.get(CHECKPOINT_KEY)).not.toBeNull();
    await inner.dispose();
  });

  it("observes cancellation again during lazy iteration", async () => {
    const cache = memoryCache();
    const manifest = await writeRange(cache, 3);
    await commitStreamCheckpoint(cache, CHECKPOINT_KEY, manifest);
    const cancellation = createCancellationTokenSource();
    const restored = await loadStreamCheckpoint(
      cache,
      CHECKPOINT_KEY,
      manifest.outputRevision,
      cancellation.token,
      silentLogger(),
    );
    cancellation.cancel("stop replay");
    await expect(restored!.values[Symbol.asyncIterator]().next())
      .rejects.toBeInstanceOf(CancellationError);
    expect(await cache.get(CHECKPOINT_KEY)).not.toBeNull();
    await cache.dispose();
  });

  it("rejects appends after finalization and finalizes idempotently", async () => {
    const cache = memoryCache();
    const writer = createStreamCheckpointWriter(cache, neverCancelledToken());
    await writer.append("one");
    const first = await writer.finalize();
    await expect(writer.finalize()).resolves.toEqual(first);
    await expect(writer.append("two")).rejects.toThrow("finalized");
    await cache.dispose();
  });
});

async function writeRange(cache: CacheBackend, count: number): Promise<StreamCheckpointManifest> {
  const writer = createStreamCheckpointWriter(cache, neverCancelledToken());
  for (let index = 0; index < count; index++) await writer.append(index);
  return writer.finalize();
}

async function collect(values: AsyncIterable<unknown>): Promise<unknown[]> {
  const result: unknown[] = [];
  for await (const value of values) result.push(value);
  return result;
}

function delegatingCache(inner: CacheBackend): CacheBackend {
  return {
    get: key => inner.get(key),
    put: (key, entry) => inner.put(key, entry),
    invalidate: key => inner.invalidate(key),
    invalidatePrefix: prefix => inner.invalidatePrefix?.(prefix) ?? Promise.resolve(),
    gc: olderThanMs => inner.gc(olderThanMs),
    dispose: () => inner.dispose(),
  };
}

function countingCache(inner: CacheBackend): CacheBackend & {
  getCalls: number;
  readonly uniquePutKeys: Set<string>;
} {
  const result = {
    ...delegatingCache(inner),
    getCalls: 0,
    uniquePutKeys: new Set<string>(),
    async get(key: string): Promise<CacheEntry | null> {
      result.getCalls++;
      return inner.get(key);
    },
    async put(key: string, entry: CacheEntry): Promise<void> {
      result.uniquePutKeys.add(key);
      await inner.put(key, entry);
    },
  };
  return result;
}
