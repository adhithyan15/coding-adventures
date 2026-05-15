/**
 * forme-cache — integrity tests
 */

import { describe, it, expect } from "vitest";
import {
  computeContentHash,
  makeEntry,
  verifyEntry,
} from "../src/index.js";

describe("computeContentHash", () => {
  it("returns a 64-char lower-case hex digest", () => {
    const h = computeContentHash(new Uint8Array([1, 2, 3]));
    expect(h).toMatch(/^[0-9a-f]{64}$/);
  });

  it("is deterministic", () => {
    const bytes = new Uint8Array([1, 2, 3, 4, 5]);
    expect(computeContentHash(bytes)).toBe(computeContentHash(bytes));
  });

  it("differs for any single-byte change", () => {
    expect(computeContentHash(new Uint8Array([1, 2, 3])))
      .not.toBe(computeContentHash(new Uint8Array([1, 2, 4])));
  });

  it("handles empty payload", () => {
    expect(computeContentHash(new Uint8Array(0)).length).toBe(64);
  });
});

describe("makeEntry", () => {
  it("populates derived fields consistently", () => {
    const payload = new Uint8Array([1, 2, 3]);
    const entry = makeEntry(payload, () => 1234);
    expect(entry.writtenMs).toBe(1234);
    expect(entry.sizeBytes).toBe(3);
    expect(entry.payload).toBe(payload);
    expect(entry.contentHash).toBe(computeContentHash(payload));
  });

  it("uses Date.now by default", () => {
    const before = Date.now();
    const entry = makeEntry(new Uint8Array([0]));
    const after = Date.now();
    expect(entry.writtenMs).toBeGreaterThanOrEqual(before);
    expect(entry.writtenMs).toBeLessThanOrEqual(after);
  });
});

describe("verifyEntry", () => {
  it("accepts a freshly-made entry", () => {
    expect(verifyEntry(makeEntry(new Uint8Array([1, 2, 3])))).toBe(true);
  });

  it("rejects entry with mismatched contentHash", () => {
    const entry = makeEntry(new Uint8Array([1, 2, 3]));
    expect(verifyEntry({ ...entry, contentHash: "0".repeat(64) })).toBe(false);
  });

  it("rejects entry with mismatched sizeBytes", () => {
    const entry = makeEntry(new Uint8Array([1, 2, 3]));
    expect(verifyEntry({ ...entry, sizeBytes: 99 })).toBe(false);
  });

  it("rejects entry whose payload was tampered with", () => {
    const original = makeEntry(new Uint8Array([1, 2, 3]));
    const tampered = { ...original, payload: new Uint8Array([1, 2, 4]) };
    expect(verifyEntry(tampered)).toBe(false);
  });

  it("accepts empty-payload entry", () => {
    expect(verifyEntry(makeEntry(new Uint8Array(0)))).toBe(true);
  });
});
