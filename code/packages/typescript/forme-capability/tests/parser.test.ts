/**
 * forme-capability — parser tests
 */

import assert from "node:assert/strict";
import { describe, test as it } from "node:test";
import { parseCapability, tryParseCapability } from "../src/index.js";

describe("parseCapability — happy paths", () => {
  it("two-segment capability", () => {
    assert.deepEqual(parseCapability("storage:read"), {
      realm: "storage", scope: "read", detail: null, wildcard: false, raw: "storage:read",
    });
  });

  it("three-segment capability", () => {
    assert.deepEqual(parseCapability("network:https:api.example.com"), {
      realm: "network", scope: "https", detail: "api.example.com",
      wildcard: false, raw: "network:https:api.example.com",
    });
  });

  it("scope wildcard", () => {
    const p = parseCapability("network:*");
    assert.equal(p.scope, "*");
    assert.equal(p.wildcard, true);
  });

  it("detail wildcard", () => {
    const p = parseCapability("network:https:*");
    assert.equal(p.detail, "*");
    assert.equal(p.wildcard, true);
  });

  it("env var with all-caps name", () => {
    const p = parseCapability("env:GITHUB_TOKEN");
    assert.deepEqual(p, {
      realm: "env", scope: "GITHUB_TOKEN", detail: null,
      wildcard: false, raw: "env:GITHUB_TOKEN",
    });
  });

  it("host wildcard does NOT set wildcard:true (only pure-segment * does)", () => {
    const p = parseCapability("network:*.google.com");
    assert.equal(p.scope, "*.google.com");
    assert.equal(p.wildcard, false);
  });
});

describe("parseCapability — rejection", () => {
  it("throws RangeError on empty string", () => {
    assert.throws(() => parseCapability(""), RangeError);
  });

  it("throws RangeError on single segment", () => {
    assert.throws(() => parseCapability("storage"), RangeError);
  });

  it("throws RangeError on too many segments", () => {
    assert.throws(() => parseCapability("a:b:c:d"), RangeError);
  });

  it("throws on empty segment", () => {
    assert.throws(() => parseCapability("storage:"), RangeError);
    assert.throws(() => parseCapability(":read"), RangeError);
    assert.throws(() => parseCapability("a::b"), RangeError);
  });

  it("throws on whitespace anywhere", () => {
    assert.throws(() => parseCapability("storage: read"), RangeError);
    assert.throws(() => parseCapability("storage:read "), RangeError);
    assert.throws(() => parseCapability("storage:read\n"), RangeError);
  });

  it("throws on control characters", () => {
    assert.throws(() => parseCapability("storage:re\x00ad"), RangeError);
  });

  it("throws on non-string input", () => {
    // @ts-expect-error — runtime check should still catch this.
    assert.throws(() => parseCapability(null), RangeError);
    // @ts-expect-error
    assert.throws(() => parseCapability(undefined), RangeError);
    // @ts-expect-error
    assert.throws(() => parseCapability(42), RangeError);
  });
});

describe("tryParseCapability", () => {
  it("returns the parsed view on success", () => {
    assert.equal(tryParseCapability("storage:read")?.realm, "storage");
  });

  it("returns null on malformed input instead of throwing", () => {
    assert.equal(tryParseCapability(""), null);
    assert.equal(tryParseCapability("noColon"), null);
    assert.equal(tryParseCapability("a:b:c:d"), null);
  });
});
