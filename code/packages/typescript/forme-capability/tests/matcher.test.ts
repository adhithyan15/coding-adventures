/**
 * forme-capability — matcher tests
 *
 * Each rule from the matcher.ts header gets explicit positive AND
 * negative coverage.
 */

import { describe, it, expect } from "vitest";
import { matchesCapability, parseCapability } from "../src/index.js";

describe("realm gate", () => {
  it("identical capabilities match", () => {
    expect(matchesCapability("storage:read", "storage:read")).toBe(true);
  });

  it("different realms never match", () => {
    expect(matchesCapability("storage:read", "network:read")).toBe(false);
    expect(matchesCapability("network:*", "env:FOO")).toBe(false);
  });
});

describe("scope wildcard (declared `*`)", () => {
  it("network:* covers any specific host", () => {
    expect(matchesCapability("network:*", "network:api.github.com")).toBe(true);
    expect(matchesCapability("network:*", "network:foo.com")).toBe(true);
  });

  it("env:* covers any specific env var", () => {
    expect(matchesCapability("env:*", "env:GITHUB_TOKEN")).toBe(true);
  });

  it("requested wildcard does NOT cover declared specific (asymmetric)", () => {
    expect(matchesCapability("env:GITHUB_TOKEN", "env:*")).toBe(false);
  });
});

describe("non-network detail handling", () => {
  it("2-seg declaration matches 2-seg request only", () => {
    expect(matchesCapability("storage:read", "storage:read")).toBe(true);
    expect(matchesCapability("storage:read", "storage:read:foo")).toBe(false);
  });

  it("3-seg declaration matches exact 3-seg request", () => {
    expect(matchesCapability("editor:command:save", "editor:command:save")).toBe(true);
    expect(matchesCapability("editor:command:save", "editor:command:open")).toBe(false);
  });

  it("3-seg declaration with detail wildcard covers any non-null detail", () => {
    expect(matchesCapability("editor:command:*", "editor:command:save")).toBe(true);
    expect(matchesCapability("editor:command:*", "editor:command:open")).toBe(true);
    // But not 2-seg requests:
    expect(matchesCapability("editor:command:*", "editor:command")).toBe(false);
  });

  it("3-seg declaration does NOT cover 2-seg request", () => {
    expect(matchesCapability("editor:command:save", "editor:command")).toBe(false);
  });

  it("scope mismatch fails even with detail wildcard", () => {
    expect(matchesCapability("editor:command:*", "editor:sidebar:foo")).toBe(false);
  });
});

describe("network host hierarchy (2-segment form)", () => {
  it("exact host match", () => {
    expect(matchesCapability("network:foo.com", "network:foo.com")).toBe(true);
  });

  it("declared host covers its subdomains", () => {
    expect(matchesCapability("network:foo.com", "network:api.foo.com")).toBe(true);
    expect(matchesCapability("network:foo.com", "network:a.b.foo.com")).toBe(true);
  });

  it("declared host does NOT cover its parent domain", () => {
    expect(matchesCapability("network:api.foo.com", "network:foo.com")).toBe(false);
  });

  it("similar but distinct host does not match", () => {
    expect(matchesCapability("network:foo.com", "network:notfoo.com")).toBe(false);
    // Substring trap: foo.com should NOT match xfoo.com.
    expect(matchesCapability("network:foo.com", "network:xfoo.com")).toBe(false);
  });

  it("case insensitive (DNS hosts are case-insensitive)", () => {
    expect(matchesCapability("network:Foo.Com", "network:API.FOO.COM")).toBe(true);
  });
});

describe("network subdomain wildcard `*.host`", () => {
  it("matches subdomains", () => {
    expect(matchesCapability("network:*.foo.com", "network:api.foo.com")).toBe(true);
    expect(matchesCapability("network:*.foo.com", "network:a.b.foo.com")).toBe(true);
  });

  it("does NOT match the bare host", () => {
    expect(matchesCapability("network:*.foo.com", "network:foo.com")).toBe(false);
  });

  it("does NOT match unrelated hosts", () => {
    expect(matchesCapability("network:*.foo.com", "network:foo.org")).toBe(false);
  });
});

describe("network with explicit scheme (3-segment form)", () => {
  it("exact scheme + exact host", () => {
    expect(matchesCapability(
      "network:https:foo.com", "network:https:foo.com",
    )).toBe(true);
  });

  it("scheme wildcard + exact host", () => {
    expect(matchesCapability(
      "network:*:foo.com", "network:https:foo.com",
    )).toBe(true);
    expect(matchesCapability(
      "network:*:foo.com", "network:http:foo.com",
    )).toBe(true);
  });

  it("scheme mismatch", () => {
    expect(matchesCapability(
      "network:https:foo.com", "network:http:foo.com",
    )).toBe(false);
  });

  it("scheme + host hierarchy", () => {
    expect(matchesCapability(
      "network:https:foo.com", "network:https:api.foo.com",
    )).toBe(true);
  });

  it("2-seg declaration does NOT cover 3-seg scheme-restricted request", () => {
    expect(matchesCapability(
      "network:foo.com", "network:https:foo.com",
    )).toBe(false);
  });

  it("3-seg declaration does NOT cover 2-seg request", () => {
    expect(matchesCapability(
      "network:https:foo.com", "network:foo.com",
    )).toBe(false);
  });
});

describe("non-network realms do NOT get host hierarchy", () => {
  it("storage:foo.com does NOT cover storage:api.foo.com", () => {
    expect(matchesCapability("storage:foo.com", "storage:api.foo.com")).toBe(false);
  });

  it("env:FOO does NOT cover env:FOO_BAR", () => {
    expect(matchesCapability("env:FOO", "env:FOO_BAR")).toBe(false);
  });
});

describe("malformed inputs return false (not throw)", () => {
  it("empty declared", () => {
    expect(matchesCapability("", "storage:read")).toBe(false);
  });
  it("empty requested", () => {
    expect(matchesCapability("storage:read", "")).toBe(false);
  });
  it("malformed declared", () => {
    expect(matchesCapability("storage::read", "storage:read")).toBe(false);
  });
});

describe("accepts pre-parsed capabilities", () => {
  it("works with two ParsedCapability arguments", () => {
    const a = parseCapability("network:*");
    const b = parseCapability("network:foo.com");
    expect(matchesCapability(a, b)).toBe(true);
  });

  it("works with mixed string and parsed", () => {
    const a = parseCapability("network:*");
    expect(matchesCapability(a, "network:api.example.com")).toBe(true);
  });
});
