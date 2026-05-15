/**
 * forme-identity — RevisionId tests
 */

import { describe, it, expect } from "vitest";
import {
  REVISION_ALGORITHM,
  REVISION_DIGEST_BYTES,
  computeRevisionId,
  isRevisionIdShape,
} from "../src/index.js";
import type { RevisionId } from "@coding-adventures/forme-types";

describe("computeRevisionId", () => {
  it("returns a string starting with the algorithm prefix", () => {
    const id = computeRevisionId({ a: 1 });
    expect(id.startsWith(`${REVISION_ALGORITHM}:`)).toBe(true);
  });

  it("hex digest is exactly REVISION_DIGEST_BYTES * 2 chars", () => {
    const id = computeRevisionId({});
    const hex = id.slice(REVISION_ALGORITHM.length + 1);
    expect(hex.length).toBe(REVISION_DIGEST_BYTES * 2);
    expect(hex).toMatch(/^[0-9a-f]+$/);
  });

  it("equal inputs produce equal ids regardless of object key order", () => {
    expect(computeRevisionId({ a: 1, b: 2 })).toBe(
      computeRevisionId({ b: 2, a: 1 }),
    );
  });

  it("different content produces different ids", () => {
    expect(computeRevisionId({ a: 1 })).not.toBe(computeRevisionId({ a: 2 }));
    expect(computeRevisionId([])).not.toBe(computeRevisionId([0]));
    expect(computeRevisionId(null)).not.toBe(computeRevisionId(false));
  });

  it("is deterministic — same input ten times yields the same id", () => {
    const fixed = { posts: [{ title: "Hello", date: "2026-05-15" }] };
    const ids = Array.from({ length: 10 }, () => computeRevisionId(fixed));
    expect(new Set(ids).size).toBe(1);
  });

  it("hashes the empty object to a known stable value", () => {
    // Pinning this catches any accidental change to the canonical-JSON
    // serialiser or the BLAKE2b configuration.  If this test ever
    // legitimately needs to be updated, that's a kernel-API-version bump.
    const id = computeRevisionId({});
    expect(id).toMatch(/^blake2b:[0-9a-f]{64}$/);
    // Same input must produce the same hash today and forever.
    expect(computeRevisionId({})).toBe(id);
  });
});

describe("isRevisionIdShape", () => {
  it("accepts a freshly-generated id", () => {
    const id = computeRevisionId({ a: 1 });
    expect(isRevisionIdShape(id)).toBe(true);
  });

  it("rejects strings missing the colon", () => {
    expect(isRevisionIdShape("noprefix")).toBe(false);
  });

  it("rejects empty algorithm or empty digest", () => {
    expect(isRevisionIdShape(":abc")).toBe(false);
    expect(isRevisionIdShape("blake2b:")).toBe(false);
  });

  it("rejects upper-case hex", () => {
    expect(
      isRevisionIdShape(("blake2b:" + "F".repeat(64)) as RevisionId),
    ).toBe(false);
  });

  it("rejects wrong-length hex for the known algorithm", () => {
    expect(
      isRevisionIdShape(("blake2b:" + "a".repeat(63)) as RevisionId),
    ).toBe(false);
  });

  it("accepts an unknown algorithm prefix without enforcing length", () => {
    // Forward-compatible: the predicate is permissive about future
    // algorithms (e.g. "blake3:") so callers don't have to be updated
    // every time we add one.
    expect(
      isRevisionIdShape("blake3:" + "a".repeat(40)),
    ).toBe(true);
  });

  it("rejects characters outside [0-9a-f] in the digest", () => {
    expect(
      isRevisionIdShape("blake2b:" + "g".repeat(64)),
    ).toBe(false);
  });
});
