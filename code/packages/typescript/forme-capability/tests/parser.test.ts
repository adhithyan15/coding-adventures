/**
 * forme-capability — parser tests
 */

import { describe, it, expect } from "vitest";
import { parseCapability, tryParseCapability } from "../src/index.js";

describe("parseCapability — happy paths", () => {
  it("two-segment capability", () => {
    expect(parseCapability("storage:read")).toEqual({
      realm: "storage", scope: "read", detail: null, wildcard: false, raw: "storage:read",
    });
  });

  it("three-segment capability", () => {
    expect(parseCapability("network:https:api.example.com")).toEqual({
      realm: "network", scope: "https", detail: "api.example.com",
      wildcard: false, raw: "network:https:api.example.com",
    });
  });

  it("scope wildcard", () => {
    const p = parseCapability("network:*");
    expect(p.scope).toBe("*");
    expect(p.wildcard).toBe(true);
  });

  it("detail wildcard", () => {
    const p = parseCapability("network:https:*");
    expect(p.detail).toBe("*");
    expect(p.wildcard).toBe(true);
  });

  it("env var with all-caps name", () => {
    const p = parseCapability("env:GITHUB_TOKEN");
    expect(p).toEqual({
      realm: "env", scope: "GITHUB_TOKEN", detail: null,
      wildcard: false, raw: "env:GITHUB_TOKEN",
    });
  });

  it("host wildcard does NOT set wildcard:true (only pure-segment * does)", () => {
    const p = parseCapability("network:*.google.com");
    expect(p.scope).toBe("*.google.com");
    expect(p.wildcard).toBe(false);
  });
});

describe("parseCapability — rejection", () => {
  it("throws RangeError on empty string", () => {
    expect(() => parseCapability("")).toThrow(RangeError);
  });

  it("throws RangeError on single segment", () => {
    expect(() => parseCapability("storage")).toThrow(RangeError);
  });

  it("throws RangeError on too many segments", () => {
    expect(() => parseCapability("a:b:c:d")).toThrow(RangeError);
  });

  it("throws on empty segment", () => {
    expect(() => parseCapability("storage:")).toThrow(RangeError);
    expect(() => parseCapability(":read")).toThrow(RangeError);
    expect(() => parseCapability("a::b")).toThrow(RangeError);
  });

  it("throws on whitespace anywhere", () => {
    expect(() => parseCapability("storage: read")).toThrow(RangeError);
    expect(() => parseCapability("storage:read ")).toThrow(RangeError);
    expect(() => parseCapability("storage:read\n")).toThrow(RangeError);
  });

  it("throws on control characters", () => {
    expect(() => parseCapability("storage:re\x00ad")).toThrow(RangeError);
  });

  it("throws on non-string input", () => {
    // @ts-expect-error — runtime check should still catch this.
    expect(() => parseCapability(null)).toThrow(RangeError);
    // @ts-expect-error
    expect(() => parseCapability(undefined)).toThrow(RangeError);
    // @ts-expect-error
    expect(() => parseCapability(42)).toThrow(RangeError);
  });
});

describe("tryParseCapability", () => {
  it("returns the parsed view on success", () => {
    expect(tryParseCapability("storage:read")?.realm).toBe("storage");
  });

  it("returns null on malformed input instead of throwing", () => {
    expect(tryParseCapability("")).toBeNull();
    expect(tryParseCapability("noColon")).toBeNull();
    expect(tryParseCapability("a:b:c:d")).toBeNull();
  });
});
