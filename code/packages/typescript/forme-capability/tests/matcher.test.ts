/**
 * forme-capability — matcher tests
 *
 * Each rule from the matcher.ts header gets explicit positive AND
 * negative coverage.
 */

import assert from "node:assert/strict";
import { describe, test as it } from "node:test";
import { matchesCapability, parseCapability } from "../src/index.js";

describe("realm gate", () => {
  it("identical capabilities match", () => {
    assert.equal(matchesCapability("storage:read", "storage:read"), true);
  });

  it("different realms never match", () => {
    assert.equal(matchesCapability("storage:read", "network:read"), false);
    assert.equal(matchesCapability("network:*", "env:FOO"), false);
  });
});

describe("scope wildcard (declared `*`)", () => {
  it("network:* covers any specific host", () => {
    assert.equal(matchesCapability("network:*", "network:api.github.com"), true);
    assert.equal(matchesCapability("network:*", "network:foo.com"), true);
  });

  it("env:* covers any specific env var", () => {
    assert.equal(matchesCapability("env:*", "env:GITHUB_TOKEN"), true);
  });

  it("requested wildcard does NOT cover declared specific (asymmetric)", () => {
    assert.equal(matchesCapability("env:GITHUB_TOKEN", "env:*"), false);
  });
});

describe("non-network detail handling", () => {
  it("2-seg declaration matches 2-seg request only", () => {
    assert.equal(matchesCapability("storage:read", "storage:read"), true);
    assert.equal(matchesCapability("storage:read", "storage:read:foo"), false);
  });

  it("3-seg declaration matches exact 3-seg request", () => {
    assert.equal(matchesCapability("editor:command:save", "editor:command:save"), true);
    assert.equal(matchesCapability("editor:command:save", "editor:command:open"), false);
  });

  it("3-seg declaration with detail wildcard covers any non-null detail", () => {
    assert.equal(matchesCapability("editor:command:*", "editor:command:save"), true);
    assert.equal(matchesCapability("editor:command:*", "editor:command:open"), true);
    // But not 2-seg requests:
    assert.equal(matchesCapability("editor:command:*", "editor:command"), false);
  });

  it("3-seg declaration does NOT cover 2-seg request", () => {
    assert.equal(matchesCapability("editor:command:save", "editor:command"), false);
  });

  it("scope mismatch fails even with detail wildcard", () => {
    assert.equal(matchesCapability("editor:command:*", "editor:sidebar:foo"), false);
  });
});

describe("network host hierarchy (2-segment form)", () => {
  it("exact host match", () => {
    assert.equal(matchesCapability("network:foo.com", "network:foo.com"), true);
  });

  it("declared host covers its subdomains", () => {
    assert.equal(matchesCapability("network:foo.com", "network:api.foo.com"), true);
    assert.equal(matchesCapability("network:foo.com", "network:a.b.foo.com"), true);
  });

  it("declared host does NOT cover its parent domain", () => {
    assert.equal(matchesCapability("network:api.foo.com", "network:foo.com"), false);
  });

  it("similar but distinct host does not match", () => {
    assert.equal(matchesCapability("network:foo.com", "network:notfoo.com"), false);
    // Substring trap: foo.com should NOT match xfoo.com.
    assert.equal(matchesCapability("network:foo.com", "network:xfoo.com"), false);
  });

  it("case insensitive (DNS hosts are case-insensitive)", () => {
    assert.equal(matchesCapability("network:Foo.Com", "network:API.FOO.COM"), true);
  });
});

describe("network subdomain wildcard `*.host`", () => {
  it("matches subdomains", () => {
    assert.equal(matchesCapability("network:*.foo.com", "network:api.foo.com"), true);
    assert.equal(matchesCapability("network:*.foo.com", "network:a.b.foo.com"), true);
  });

  it("does NOT match the bare host", () => {
    assert.equal(matchesCapability("network:*.foo.com", "network:foo.com"), false);
  });

  it("does NOT match unrelated hosts", () => {
    assert.equal(matchesCapability("network:*.foo.com", "network:foo.org"), false);
  });
});

describe("network with explicit scheme (3-segment form)", () => {
  it("exact scheme + exact host", () => {
    assert.ok(matchesCapability(
      "network:https:foo.com", "network:https:foo.com",
    ));
  });

  it("scheme wildcard + exact host", () => {
    assert.ok(matchesCapability(
      "network:*:foo.com", "network:https:foo.com",
    ));
    assert.ok(matchesCapability(
      "network:*:foo.com", "network:http:foo.com",
    ));
  });

  it("scheme mismatch", () => {
    assert.ok(!matchesCapability(
      "network:https:foo.com", "network:http:foo.com",
    ));
  });

  it("scheme + host hierarchy", () => {
    assert.ok(matchesCapability(
      "network:https:foo.com", "network:https:api.foo.com",
    ));
  });

  it("2-seg declaration does NOT cover 3-seg scheme-restricted request", () => {
    assert.ok(!matchesCapability(
      "network:foo.com", "network:https:foo.com",
    ));
  });

  it("3-seg declaration does NOT cover 2-seg request", () => {
    assert.ok(!matchesCapability(
      "network:https:foo.com", "network:foo.com",
    ));
  });
});

describe("non-network realms do NOT get host hierarchy", () => {
  it("storage:foo.com does NOT cover storage:api.foo.com", () => {
    assert.equal(matchesCapability("storage:foo.com", "storage:api.foo.com"), false);
  });

  it("env:FOO does NOT cover env:FOO_BAR", () => {
    assert.equal(matchesCapability("env:FOO", "env:FOO_BAR"), false);
  });
});

describe("malformed inputs return false (not throw)", () => {
  it("empty declared", () => {
    assert.equal(matchesCapability("", "storage:read"), false);
  });
  it("empty requested", () => {
    assert.equal(matchesCapability("storage:read", ""), false);
  });
  it("malformed declared", () => {
    assert.equal(matchesCapability("storage::read", "storage:read"), false);
  });
});

describe("accepts pre-parsed capabilities", () => {
  it("works with two ParsedCapability arguments", () => {
    const a = parseCapability("network:*");
    const b = parseCapability("network:foo.com");
    assert.equal(matchesCapability(a, b), true);
  });

  it("works with mixed string and parsed", () => {
    const a = parseCapability("network:*");
    assert.equal(matchesCapability(a, "network:api.example.com"), true);
  });
});
