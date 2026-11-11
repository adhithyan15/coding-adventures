/**
 * forme-capability — realms catalogue tests
 */

import assert from "node:assert/strict";
import { describe, test as it } from "node:test";
import {
  FIRST_PARTY_ONLY,
  KERNEL_REALMS,
  SENSITIVE,
  isFirstPartyOnly,
  isKernelRealm,
  isSensitive,
} from "../src/index.js";

describe("KERNEL_REALMS", () => {
  it("is the FM01 §5.2 set", () => {
    assert.deepEqual(new Set(KERNEL_REALMS), new Set([
      "storage", "network", "env", "filesystem",
      "system", "content", "editor", "telemetry", "plugin",
    ]));
  });

  it("is frozen — runtime mutation rejected", () => {
    assert.throws(() => {
      // @ts-expect-error — readonly at the type level.
      KERNEL_REALMS.push("bogus");
    }, TypeError);
  });
});

describe("isKernelRealm", () => {
  it("accepts every kernel realm", () => {
    for (const realm of KERNEL_REALMS) {
      assert.equal(isKernelRealm(realm), true);
    }
  });

  it("rejects unknown realms", () => {
    assert.equal(isKernelRealm("ext:custom"), false);
    assert.equal(isKernelRealm("bogus"), false);
    assert.equal(isKernelRealm(""), false);
  });
});

describe("first-party-only and sensitive predicates", () => {
  it("isFirstPartyOnly recognises the spec list", () => {
    for (const cap of FIRST_PARTY_ONLY) {
      assert.equal(isFirstPartyOnly(cap), true);
    }
    assert.equal(isFirstPartyOnly("storage:read"), false);
  });

  it("isSensitive recognises the spec list", () => {
    for (const cap of SENSITIVE) {
      assert.equal(isSensitive(cap), true);
    }
    assert.equal(isSensitive("network:api.github.com"), false);
    assert.equal(isSensitive("storage:read"), false);
  });

  it("system:shell is both first-party-only and sensitive", () => {
    assert.equal(isFirstPartyOnly("system:shell"), true);
    assert.equal(isSensitive("system:shell"), true);
  });
});
