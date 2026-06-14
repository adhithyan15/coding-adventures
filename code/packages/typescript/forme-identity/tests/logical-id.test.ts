/**
 * forme-identity — LogicalId tests
 */

import { afterEach, describe, it, expect, vi } from "vitest";
import {
  buildLogicalIdFrom,
  generateLogicalId,
  isLogicalIdShape,
} from "../src/index.js";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("generateLogicalId", () => {
  it("returns a UUIDv7-shaped string", () => {
    const id = generateLogicalId();
    expect(isLogicalIdShape(id)).toBe(true);
  });

  it("two consecutive ids are distinct", () => {
    const a = generateLogicalId();
    const b = generateLogicalId();
    expect(a).not.toBe(b);
  });

  it("throws a clear error when crypto.getRandomValues is unavailable", () => {
    vi.stubGlobal("crypto", undefined);
    expect(() => generateLogicalId()).toThrow(
      /globalThis.crypto.getRandomValues is not available/,
    );
  });

  it("throws when crypto exists but lacks getRandomValues", () => {
    vi.stubGlobal("crypto", { /* deliberately incomplete */ });
    expect(() => generateLogicalId()).toThrow(
      /globalThis.crypto.getRandomValues is not available/,
    );
  });

  it("ids generated in chronological order sort lexicographically", () => {
    // Use buildLogicalIdFrom for time control instead of trying to race
    // generateLogicalId — same-millisecond calls don't guarantee monotone
    // sort under v7's random tail.
    const earlier = buildLogicalIdFrom(1_000_000_000_000, new Uint8Array(10));
    const later   = buildLogicalIdFrom(1_000_000_000_001, new Uint8Array(10));
    expect(earlier < later).toBe(true);
  });
});

describe("buildLogicalIdFrom", () => {
  it("encodes the timestamp into the first 6 bytes (12 hex chars)", () => {
    const ts = 0x0123_4567_89ab; // 48 bits, distinctive pattern
    const id = buildLogicalIdFrom(ts, new Uint8Array(10));
    // First 8 chars + next 4 chars (with hyphen) is the timestamp.
    const hex = id.replace(/-/g, "").slice(0, 12);
    expect(hex).toBe("0123456789ab");
  });

  it("stamps the version nibble to 7", () => {
    const id = buildLogicalIdFrom(0, new Uint8Array(10).fill(0xff));
    // Position 14 (after second hyphen) is the version nibble.
    expect(id[14]).toBe("7");
  });

  it("stamps the variant nibble to 8|9|a|b regardless of input", () => {
    // All-ones random tail: variant byte starts as 0xff; stamp forces top
    // two bits to 0b10, so byte 8 becomes 0xbf, and the top nibble is
    // therefore 'b'.
    const id = buildLogicalIdFrom(0, new Uint8Array(10).fill(0xff));
    expect(id[19]).toBe("b");

    // All-zero random tail: variant byte starts as 0x00; stamp forces
    // 0x80, top nibble '8'.
    const id2 = buildLogicalIdFrom(0, new Uint8Array(10));
    expect(id2[19]).toBe("8");
  });

  it("preserves the random tail bytes outside the variant nibble", () => {
    // tail[0..1] → bytes 6..7 (3rd group; byte 6's high nibble becomes 7)
    // tail[2]    → byte 8     (4th group; top two bits become 0b10)
    // tail[3]    → byte 9     (4th group; preserved verbatim)
    // tail[4..9] → bytes 10..15 (5th group; preserved verbatim)
    const tail = new Uint8Array([
      0x00, 0x00,                                 // 6, 7
      0xab,                                       // 8 → 0xab & 0x3f | 0x80 = 0xab
      0xcd,                                       // 9 → 0xcd preserved
      0xef, 0x01, 0x23, 0x45, 0x67, 0x89,         // 10..15
    ]);
    const id = buildLogicalIdFrom(0, tail);
    // 4th group = byte 8 + byte 9 = "ab" + "cd" = "abcd".
    expect(id.split("-")[3]).toBe("abcd");
    // 5th group = bytes 10..15 = "ef0123456789".
    expect(id.split("-")[4]).toBe("ef0123456789");
  });

  it("rejects out-of-range timestamps", () => {
    expect(() => buildLogicalIdFrom(-1, new Uint8Array(10))).toThrow(RangeError);
    expect(() => buildLogicalIdFrom(2 ** 48, new Uint8Array(10))).toThrow(RangeError);
    expect(() => buildLogicalIdFrom(1.5, new Uint8Array(10))).toThrow(RangeError);
  });

  it("rejects wrong-length random tails", () => {
    expect(() => buildLogicalIdFrom(0, new Uint8Array(9))).toThrow(RangeError);
    expect(() => buildLogicalIdFrom(0, new Uint8Array(11))).toThrow(RangeError);
  });
});

describe("isLogicalIdShape", () => {
  it("accepts a freshly-generated id", () => {
    expect(isLogicalIdShape(generateLogicalId())).toBe(true);
  });
  it("accepts a known-good fixture", () => {
    expect(isLogicalIdShape("01952c0d-7e63-7000-8000-000000000000")).toBe(true);
  });
  it("rejects upper-case hex", () => {
    expect(isLogicalIdShape("01952C0D-7E63-7000-8000-000000000000")).toBe(false);
  });
  it("rejects v4 (version nibble != 7)", () => {
    expect(isLogicalIdShape("01952c0d-7e63-4000-8000-000000000000")).toBe(false);
  });
  it("rejects bad variant nibble", () => {
    expect(isLogicalIdShape("01952c0d-7e63-7000-c000-000000000000")).toBe(false);
  });
  it("rejects wrong-length groups", () => {
    expect(isLogicalIdShape("01952c0d-7e6-7000-8000-000000000000")).toBe(false);
    expect(isLogicalIdShape("01952c0d-7e63-7000-8000-00000000000")).toBe(false);
  });
});
