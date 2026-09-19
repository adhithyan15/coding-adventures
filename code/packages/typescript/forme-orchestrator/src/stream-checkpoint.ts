/**
 * Bounded, content-addressed checkpoints for stage streams.
 *
 * A materialized array is convenient for replay, but it makes a streaming
 * scheduler dishonest: memory still grows with every emitted value. This
 * module stores each value as a leaf and incrementally folds leaves into an
 * ordered binary tree. The writer keeps only a binary-counter frontier, so
 * its retained state grows logarithmically while completed nodes move to the
 * injected cache immediately.
 *
 * Publication is deliberately separate from construction. A caller may build
 * the tree before a downstream input revision is known, then publish the tiny
 * manifest at the final instance-checkpoint key only after the full run has
 * succeeded. A crash can therefore leave unreachable nodes, never a visible
 * half-checkpoint.
 */

import {
  cacheKey,
  makeEntry,
  type CacheBackend,
} from "@coding-adventures/forme-cache";
import { CancellationError } from "@coding-adventures/forme-errors";
import {
  computeBinaryRevisionId,
  isRevisionIdShape,
} from "@coding-adventures/forme-identity";
import type {
  CancellationToken,
  Logger,
} from "@coding-adventures/forme-stage";
import type { RevisionId } from "@coding-adventures/forme-types";
import {
  decodeCacheValue,
  encodeCacheValue,
} from "./cache-codec.js";

export const STREAM_CHECKPOINT_SCHEMA = "forme-stream-checkpoint-v1" as const;
export const STREAM_CHECKPOINT_NODE_SCHEMA = "forme-stream-checkpoint-node-v1" as const;

const STREAM_CHECKPOINT_REVISION_SCHEMA = "forme-stream-checkpoint-revision-v1";
const STREAM_NODE_STAGE_NAME = "@coding-adventures/forme-orchestrator/stream-checkpoint-node";
const STREAM_NODE_STAGE_VERSION = "1.0.0";
const CACHE_KEY_PATTERN = /^[0-9a-f]{64}$/;
const MAX_TREE_DEPTH = 53;

interface NodeRef {
  readonly key: string;
  readonly count: number;
}

interface LeafNode {
  readonly schema: typeof STREAM_CHECKPOINT_NODE_SCHEMA;
  readonly type: "leaf";
  readonly count: 1;
  readonly value: unknown;
}

interface BranchNode {
  readonly schema: typeof STREAM_CHECKPOINT_NODE_SCHEMA;
  readonly type: "branch";
  readonly count: number;
  readonly left: NodeRef;
  readonly right: NodeRef;
}

type StreamNode = LeafNode | BranchNode;

export interface StreamCheckpointManifest {
  readonly schema: typeof STREAM_CHECKPOINT_SCHEMA;
  readonly rootKey: string | null;
  readonly itemCount: number;
  readonly outputRevision: RevisionId;
}

export interface LoadedStreamCheckpoint {
  readonly itemCount: number;
  readonly outputRevision: RevisionId;
  readonly values: AsyncIterable<unknown>;
}

export interface StreamCheckpointWriter {
  /** Append one logical stream value in source order. */
  append(value: unknown): Promise<void>;
  /** Finish the ordered tree. Repeated calls return the same manifest. */
  finalize(): Promise<StreamCheckpointManifest>;
  /** Number of complete subtrees retained by the binary-counter frontier. */
  readonly retainedSubtrees: number;
}

/**
 * Create an incremental writer. Cache writes are awaited so a successful
 * `append` means the value is already durable enough to be referenced by a
 * later manifest. Failed appends leave the prior frontier intact and may leave
 * only unreachable content-addressed nodes behind.
 */
export function createStreamCheckpointWriter(
  backend: CacheBackend,
  cancellation: CancellationToken,
): StreamCheckpointWriter {
  const frontier: Array<NodeRef | undefined> = [];
  let itemCount = 0;
  let finalized: StreamCheckpointManifest | null = null;

  return {
    get retainedSubtrees() {
      return frontier.reduce((count, node) => count + (node === undefined ? 0 : 1), 0);
    },

    async append(value: unknown): Promise<void> {
      if (finalized !== null) throw new Error("stream checkpoint writer is finalized");
      cancellation.throwIfCancelled();
      if (itemCount >= Number.MAX_SAFE_INTEGER) {
        throw new RangeError("stream checkpoint item count exceeds Number.MAX_SAFE_INTEGER");
      }

      let node = await writeNode(backend, {
        schema: STREAM_CHECKPOINT_NODE_SCHEMA,
        type: "leaf",
        count: 1,
        value,
      }, cancellation);
      let level = 0;
      const consumedLevels: number[] = [];
      while (frontier[level] !== undefined) {
        const left = frontier[level]!;
        node = await writeBranch(backend, left, node, cancellation);
        consumedLevels.push(level);
        level++;
      }

      // Mutate the frontier only after every required node write succeeds.
      for (const consumed of consumedLevels) frontier[consumed] = undefined;
      frontier[level] = node;
      itemCount++;
    },

    async finalize(): Promise<StreamCheckpointManifest> {
      if (finalized !== null) return finalized;
      cancellation.throwIfCancelled();

      // Fold smallest to largest. A lower-level subtree contains the latest
      // values, so each larger (earlier) subtree becomes its left sibling.
      let root: NodeRef | null = null;
      for (let level = 0; level < frontier.length; level++) {
        const earlier = frontier[level];
        if (earlier === undefined) continue;
        root = root === null
          ? earlier
          : await writeBranch(backend, earlier, root, cancellation);
      }
      if ((root?.count ?? 0) !== itemCount) {
        throw new Error("stream checkpoint frontier count mismatch");
      }

      const rootKey = root?.key ?? null;
      finalized = Object.freeze({
        schema: STREAM_CHECKPOINT_SCHEMA,
        rootKey,
        itemCount,
        outputRevision: streamCheckpointRevision(rootKey, itemCount),
      });
      return finalized;
    },
  };
}

/** Publish a completed manifest last, making its already-written tree visible. */
export async function commitStreamCheckpoint(
  backend: CacheBackend,
  checkpointKey: string,
  manifest: StreamCheckpointManifest,
): Promise<void> {
  validateManifestShape(manifest);
  await backend.put(checkpointKey, makeEntry(encodeCacheValue(manifest)));
}

/**
 * Validate a complete tree before exposing its lazy iterable.
 *
 * Non-cancellation failures invalidate only the manifest and return `null`,
 * the same fail-open convention as materialized instance checkpoints. Tree
 * nodes are content-addressed and may be shared, so they are left for normal
 * integrity cleanup/GC unless the failing read itself identifies a bad key.
 */
export async function loadStreamCheckpoint(
  backend: CacheBackend,
  checkpointKey: string,
  expectedOutputRevision: RevisionId,
  cancellation: CancellationToken,
  logger: Logger,
): Promise<LoadedStreamCheckpoint | null> {
  try {
    cancellation.throwIfCancelled();
    const entry = await backend.get(checkpointKey);
    cancellation.throwIfCancelled();
    if (entry === null) return null;
    const manifest = decodeManifest(entry.payload);
    if (manifest.outputRevision !== expectedOutputRevision) {
      throw new InvalidStreamCheckpoint("output revision does not match the prior ledger");
    }
    if (manifest.outputRevision !== streamCheckpointRevision(manifest.rootKey, manifest.itemCount)) {
      throw new InvalidStreamCheckpoint("manifest output revision does not match its root");
    }

    if (manifest.rootKey === null) {
      if (manifest.itemCount !== 0) {
        throw new InvalidStreamCheckpoint("a non-empty stream has no root");
      }
    } else {
      if (manifest.itemCount === 0) {
        throw new InvalidStreamCheckpoint("an empty stream has a root");
      }
      await validateSubtree(
        backend,
        manifest.rootKey,
        manifest.itemCount,
        0,
        cancellation,
      );
    }

    return {
      itemCount: manifest.itemCount,
      outputRevision: manifest.outputRevision,
      values: {
        async *[Symbol.asyncIterator]() {
          if (manifest.rootKey === null) return;
          yield* iterateSubtree(
            backend,
            manifest.rootKey,
            manifest.itemCount,
            0,
            cancellation,
          );
        },
      },
    };
  } catch (error) {
    if (error instanceof CancellationError) throw error;
    try {
      await backend.invalidate(checkpointKey);
    } catch (invalidateError) {
      logger.warn("invalid stream checkpoint manifest could not be invalidated", {
        checkpointKey,
        error: String(error),
        invalidateError: String(invalidateError),
      });
      return null;
    }
    logger.warn("stream checkpoint failed open", {
      checkpointKey,
      error: String(error),
    });
    return null;
  }
}

/** Derive the portable, domain-separated key for one encoded tree node. */
export function streamCheckpointNodeKey(payload: Uint8Array): string {
  return cacheKey({
    stageName: STREAM_NODE_STAGE_NAME,
    stageVersion: STREAM_NODE_STAGE_VERSION,
    stageConfig: null,
    inputRevision: computeBinaryRevisionId(payload),
    capabilities: [],
  });
}

/** Commit to an ordered tree without re-encoding its complete value list. */
export function streamCheckpointRevision(
  rootKey: string | null,
  itemCount: number,
): RevisionId {
  if (!isSafeCount(itemCount)) {
    throw new RangeError("stream checkpoint item count must be a non-negative safe integer");
  }
  if (rootKey !== null && !CACHE_KEY_PATTERN.test(rootKey)) {
    throw new Error("stream checkpoint root key is malformed");
  }
  return computeBinaryRevisionId(encodeCacheValue({
    schema: STREAM_CHECKPOINT_REVISION_SCHEMA,
    rootKey,
    itemCount,
  }));
}

async function writeBranch(
  backend: CacheBackend,
  left: NodeRef,
  right: NodeRef,
  cancellation: CancellationToken,
): Promise<NodeRef> {
  const count = left.count + right.count;
  if (!Number.isSafeInteger(count)) {
    throw new RangeError("stream checkpoint branch count exceeds Number.MAX_SAFE_INTEGER");
  }
  if (left.count !== largestPowerOfTwoBelow(count)) {
    throw new Error("stream checkpoint writer attempted a non-canonical branch");
  }
  return writeNode(backend, {
    schema: STREAM_CHECKPOINT_NODE_SCHEMA,
    type: "branch",
    count,
    left,
    right,
  }, cancellation);
}

async function writeNode(
  backend: CacheBackend,
  node: StreamNode,
  cancellation: CancellationToken,
): Promise<NodeRef> {
  cancellation.throwIfCancelled();
  const payload = encodeCacheValue(node);
  const key = streamCheckpointNodeKey(payload);
  await backend.put(key, makeEntry(payload));
  cancellation.throwIfCancelled();
  return { key, count: node.count };
}

async function validateSubtree(
  backend: CacheBackend,
  key: string,
  expectedCount: number,
  depth: number,
  cancellation: CancellationToken,
): Promise<void> {
  const node = await readNode(backend, key, cancellation);
  assertNodeShape(node, expectedCount, depth);
  if (node.type === "leaf") return;
  await validateSubtree(backend, node.left.key, node.left.count, depth + 1, cancellation);
  await validateSubtree(backend, node.right.key, node.right.count, depth + 1, cancellation);
}

async function* iterateSubtree(
  backend: CacheBackend,
  key: string,
  expectedCount: number,
  depth: number,
  cancellation: CancellationToken,
): AsyncIterable<unknown> {
  const node = await readNode(backend, key, cancellation);
  assertNodeShape(node, expectedCount, depth);
  if (node.type === "leaf") {
    yield node.value;
    return;
  }
  yield* iterateSubtree(backend, node.left.key, node.left.count, depth + 1, cancellation);
  yield* iterateSubtree(backend, node.right.key, node.right.count, depth + 1, cancellation);
}

async function readNode(
  backend: CacheBackend,
  key: string,
  cancellation: CancellationToken,
): Promise<StreamNode> {
  cancellation.throwIfCancelled();
  if (!CACHE_KEY_PATTERN.test(key)) throw new InvalidStreamCheckpoint("node key is malformed");
  const entry = await backend.get(key);
  cancellation.throwIfCancelled();
  if (entry === null) throw new InvalidStreamCheckpoint(`node ${key} is missing or corrupt`);
  if (streamCheckpointNodeKey(entry.payload) !== key) {
    try { await backend.invalidate(key); } catch { /* manifest invalidation still follows */ }
    throw new InvalidStreamCheckpoint(`node ${key} does not match its content-derived key`);
  }
  return decodeNode(entry.payload);
}

function assertNodeShape(node: StreamNode, expectedCount: number, depth: number): void {
  if (depth > MAX_TREE_DEPTH) throw new InvalidStreamCheckpoint("tree exceeds maximum depth");
  if (node.count !== expectedCount) throw new InvalidStreamCheckpoint("node count mismatch");
  if (expectedCount === 1) {
    if (node.type !== "leaf") throw new InvalidStreamCheckpoint("count-one node is not a leaf");
    return;
  }
  if (node.type !== "branch") throw new InvalidStreamCheckpoint("multi-value node is not a branch");
  const expectedLeft = largestPowerOfTwoBelow(expectedCount);
  const expectedRight = expectedCount - expectedLeft;
  if (node.left.count !== expectedLeft || node.right.count !== expectedRight) {
    throw new InvalidStreamCheckpoint("branch is not in canonical left-complete form");
  }
}

function decodeManifest(payload: Uint8Array): StreamCheckpointManifest {
  const decoded = decodeCacheValue(payload);
  validateManifestShape(decoded);
  return decoded;
}

function validateManifestShape(value: unknown): asserts value is StreamCheckpointManifest {
  if (!isRecord(value) || value.schema !== STREAM_CHECKPOINT_SCHEMA) {
    throw new InvalidStreamCheckpoint("manifest schema is unsupported");
  }
  if (value.rootKey !== null && (typeof value.rootKey !== "string" || !CACHE_KEY_PATTERN.test(value.rootKey))) {
    throw new InvalidStreamCheckpoint("manifest root key is malformed");
  }
  if (!isSafeCount(value.itemCount)) {
    throw new InvalidStreamCheckpoint("manifest item count is malformed");
  }
  if (typeof value.outputRevision !== "string" || !isRevisionIdShape(value.outputRevision)) {
    throw new InvalidStreamCheckpoint("manifest output revision is malformed");
  }
}

function decodeNode(payload: Uint8Array): StreamNode {
  const value = decodeCacheValue(payload);
  if (!isRecord(value) || value.schema !== STREAM_CHECKPOINT_NODE_SCHEMA) {
    throw new InvalidStreamCheckpoint("node schema is unsupported");
  }
  if (value.type === "leaf") {
    if (value.count !== 1 || !Object.prototype.hasOwnProperty.call(value, "value")) {
      throw new InvalidStreamCheckpoint("leaf node is malformed");
    }
    return {
      schema: STREAM_CHECKPOINT_NODE_SCHEMA,
      type: "leaf",
      count: 1,
      value: value.value,
    };
  }
  if (value.type === "branch") {
    if (!isSafePositiveCount(value.count)) throw new InvalidStreamCheckpoint("branch count is malformed");
    return {
      schema: STREAM_CHECKPOINT_NODE_SCHEMA,
      type: "branch",
      count: value.count,
      left: decodeNodeRef(value.left, "left"),
      right: decodeNodeRef(value.right, "right"),
    };
  }
  throw new InvalidStreamCheckpoint("node type is unsupported");
}

function decodeNodeRef(value: unknown, side: string): NodeRef {
  if (!isRecord(value)
      || typeof value.key !== "string"
      || !CACHE_KEY_PATTERN.test(value.key)
      || !isSafePositiveCount(value.count)) {
    throw new InvalidStreamCheckpoint(`${side} child reference is malformed`);
  }
  return { key: value.key, count: value.count };
}

function largestPowerOfTwoBelow(count: number): number {
  if (!isSafePositiveCount(count) || count < 2) {
    throw new RangeError("a branch count must be a safe integer of at least two");
  }
  return 2 ** Math.floor(Math.log2(count - 1));
}

function isSafeCount(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function isSafePositiveCount(value: unknown): value is number {
  return isSafeCount(value) && value > 0;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

class InvalidStreamCheckpoint extends Error {
  constructor(message: string) {
    super(`invalid stream checkpoint: ${message}`);
    this.name = "InvalidStreamCheckpoint";
  }
}
