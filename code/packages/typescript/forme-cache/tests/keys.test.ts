/**
 * forme-cache — key derivation tests
 */

import { describe, it, expect } from "vitest";
import {
  CACHE_KEY_DIGEST_BYTES,
  CACHE_KEY_VERSION,
  cacheKey,
  capabilitySetHash,
} from "../src/index.js";
import type { RevisionId } from "@coding-adventures/forme-types";

const REV = "blake2b:cafebabe" as RevisionId;

function input(overrides: Partial<Parameters<typeof cacheKey>[0]> = {}) {
  return {
    stageName: "@forme/parse-markdown",
    stageVersion: "0.1.0",
    stageConfig: { gfm: true },
    inputRevision: REV,
    capabilities: ["storage:read"],
    ...overrides,
  };
}

describe("cacheKey", () => {
  it("returns a 64-char lower-case hex digest", () => {
    const key = cacheKey(input());
    expect(key).toMatch(/^[0-9a-f]{64}$/);
    expect(key.length).toBe(CACHE_KEY_DIGEST_BYTES * 2);
  });

  it("is deterministic — same inputs ⇒ same key", () => {
    expect(cacheKey(input())).toBe(cacheKey(input()));
  });

  it("differs when stage name differs", () => {
    expect(cacheKey(input())).not.toBe(cacheKey(input({ stageName: "other" })));
  });

  it("differs when stage version differs", () => {
    expect(cacheKey(input())).not.toBe(cacheKey(input({ stageVersion: "0.2.0" })));
  });

  it("differs when config differs (any field)", () => {
    expect(cacheKey(input())).not.toBe(cacheKey(input({ stageConfig: { gfm: false } })));
  });

  it("differs when input revision differs", () => {
    expect(cacheKey(input())).not.toBe(cacheKey(input({ inputRevision: "blake2b:deadbeef" as RevisionId })));
  });

  it("differs when capability set differs", () => {
    expect(cacheKey(input())).not.toBe(cacheKey(input({ capabilities: ["storage:write"] })));
  });

  it("config key-order does not affect the key (canonical JSON)", () => {
    const a = cacheKey(input({ stageConfig: { a: 1, b: 2 } }));
    const b = cacheKey(input({ stageConfig: { b: 2, a: 1 } }));
    expect(a).toBe(b);
  });

  it("capability order does not affect the key", () => {
    const a = cacheKey(input({ capabilities: ["a", "b", "c"] }));
    const b = cacheKey(input({ capabilities: ["c", "a", "b"] }));
    expect(a).toBe(b);
  });

  it("CACHE_KEY_VERSION pins to forme-cache-v1", () => {
    // This is the kernel-version barrier — bumping it is a deliberate
    // global cache-flush and must be a code-reviewed change.
    expect(CACHE_KEY_VERSION).toBe("forme-cache-v1");
  });

  it("differs when capability count changes", () => {
    expect(cacheKey(input({ capabilities: [] })))
      .not.toBe(cacheKey(input({ capabilities: ["storage:read"] })));
  });
});

describe("capabilitySetHash", () => {
  it("returns a 64-char hex digest", () => {
    const h = capabilitySetHash(["storage:read", "network:*"]);
    expect(h).toMatch(/^[0-9a-f]{64}$/);
  });

  it("empty input is a valid hash (not empty string)", () => {
    expect(capabilitySetHash([]).length).toBe(64);
  });

  it("order does not matter", () => {
    expect(capabilitySetHash(["a", "b"])).toBe(capabilitySetHash(["b", "a"]));
  });

  it("different sets ⇒ different hashes", () => {
    expect(capabilitySetHash(["a"])).not.toBe(capabilitySetHash(["a", "b"]));
  });

  it("does not mutate the input array", () => {
    const caps = ["b", "a"];
    capabilitySetHash(caps);
    expect(caps).toEqual(["b", "a"]); // unchanged
  });
});
