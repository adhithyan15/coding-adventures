/**
 * forme-capability — realms catalogue tests
 */

import { describe, it, expect } from "vitest";
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
    expect(new Set(KERNEL_REALMS)).toEqual(new Set([
      "storage", "network", "env", "filesystem",
      "system", "content", "editor", "telemetry", "plugin",
    ]));
  });

  it("is frozen — runtime mutation rejected", () => {
    expect(() => {
      // @ts-expect-error — readonly at the type level.
      KERNEL_REALMS.push("bogus");
    }).toThrow(TypeError);
  });
});

describe("isKernelRealm", () => {
  it("accepts every kernel realm", () => {
    for (const realm of KERNEL_REALMS) {
      expect(isKernelRealm(realm)).toBe(true);
    }
  });

  it("rejects unknown realms", () => {
    expect(isKernelRealm("ext:custom")).toBe(false);
    expect(isKernelRealm("bogus")).toBe(false);
    expect(isKernelRealm("")).toBe(false);
  });
});

describe("first-party-only and sensitive predicates", () => {
  it("isFirstPartyOnly recognises the spec list", () => {
    for (const cap of FIRST_PARTY_ONLY) {
      expect(isFirstPartyOnly(cap)).toBe(true);
    }
    expect(isFirstPartyOnly("storage:read")).toBe(false);
  });

  it("isSensitive recognises the spec list", () => {
    for (const cap of SENSITIVE) {
      expect(isSensitive(cap)).toBe(true);
    }
    expect(isSensitive("network:api.github.com")).toBe(false);
    expect(isSensitive("storage:read")).toBe(false);
  });

  it("system:shell is both first-party-only and sensitive", () => {
    expect(isFirstPartyOnly("system:shell")).toBe(true);
    expect(isSensitive("system:shell")).toBe(true);
  });
});
