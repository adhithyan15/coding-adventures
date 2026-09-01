import { describe, expect, it } from "vitest";
import {
  decodeCachedStageOutput,
  decodeCacheValue,
  encodeCachedStageOutput,
  encodeCacheValue,
} from "../src/cache-codec.js";

describe("orchestrator cache codec", () => {
  it("round-trips Forme values with deterministic object ordering", () => {
    const first = encodeCacheValue({ z: -0, a: [undefined, new Uint8Array([0, 15, 255])] });
    const second = encodeCacheValue({ a: [undefined, new Uint8Array([0, 15, 255])], z: -0 });
    expect(first).toEqual(second);
    const decoded = decodeCacheValue(first) as { a: [undefined, Uint8Array]; z: number };
    expect(decoded.a[0]).toBeUndefined();
    expect(decoded.a[1]).toEqual(new Uint8Array([0, 15, 255]));
    expect(Object.is(decoded.z, -0)).toBe(true);
  });

  it("round-trips the versioned stage-output envelope", () => {
    const payload = encodeCachedStageOutput({ value: { html: "<p>ok</p>" }, isStream: true });
    expect(decodeCachedStageOutput(payload)).toEqual({
      value: { html: "<p>ok</p>" },
      isStream: true,
    });
  });

  it("rejects unsupported, cyclic, and malformed values", () => {
    expect(() => encodeCacheValue(Number.NaN)).toThrow(/non-finite/);
    expect(() => encodeCacheValue(new Map())).toThrow(/plain objects/);
    const cyclic: { self?: unknown } = {};
    cyclic.self = cyclic;
    expect(() => encodeCacheValue(cyclic)).toThrow(/cyclic/);
    expect(() => decodeCacheValue(new TextEncoder().encode('["bytes","xyz"]'))).toThrow(/bytes/);
    expect(() => decodeCachedStageOutput(encodeCacheValue({ schema: "future" }))).toThrow(/schema/);
  });

  it("decodes __proto__ as an own data property without prototype mutation", () => {
    const input = JSON.parse('{"__proto__":{"safe":true}}') as object;
    const decoded = decodeCacheValue(encodeCacheValue(input)) as Record<string, unknown>;
    expect(Object.getPrototypeOf(decoded)).toBe(Object.prototype);
    expect(Object.hasOwn(decoded, "__proto__")).toBe(true);
    expect(decoded.__proto__).toEqual({ safe: true });
  });
});
