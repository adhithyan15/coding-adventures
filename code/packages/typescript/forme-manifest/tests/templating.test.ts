import { describe, it, expect } from "vitest";
import {
  resolveCapabilityTemplate,
  hasTemplate,
  RECOGNISED_VARIABLES,
  ManifestError,
} from "../src/index.js";

const env = {
  storageRoot: "/abs/storage",
  cacheDir:    "/abs/cache",
  pluginDir:   "/abs/plugin",
};

describe("resolveCapabilityTemplate", () => {
  it("substitutes $storageRoot", () => {
    expect(resolveCapabilityTemplate("filesystem:read:$storageRoot", env))
      .toBe("filesystem:read:/abs/storage");
  });

  it("substitutes $cacheDir", () => {
    expect(resolveCapabilityTemplate("filesystem:read:$cacheDir", env))
      .toBe("filesystem:read:/abs/cache");
  });

  it("substitutes $pluginDir", () => {
    expect(resolveCapabilityTemplate("filesystem:read:$pluginDir", env))
      .toBe("filesystem:read:/abs/plugin");
  });

  it("handles $$ as literal dollar", () => {
    expect(resolveCapabilityTemplate("path:$$literal:$storageRoot", env))
      .toBe("path:$literal:/abs/storage");
  });

  it("handles multiple $$ in a row", () => {
    expect(resolveCapabilityTemplate("a$$$$b", env)).toBe("a$$b");
  });

  it("handles strings with no templates", () => {
    expect(resolveCapabilityTemplate("filesystem:read:/static/path", env))
      .toBe("filesystem:read:/static/path");
  });

  it("rejects unknown $variable", () => {
    expect(() => resolveCapabilityTemplate("$bogus", env))
      .toThrowError(/unrecognised template variable/);
  });

  it("rejects bare $ followed by non-identifier", () => {
    expect(() => resolveCapabilityTemplate("path:$ next", env))
      .toThrowError(/bare/);
  });

  it("rejects non-string input", () => {
    expect(() => resolveCapabilityTemplate(123 as unknown as string, env))
      .toThrow(ManifestError);
  });

  it("throws when $cacheDir referenced but env has cacheDir = null", () => {
    expect(() => resolveCapabilityTemplate("$cacheDir", { ...env, cacheDir: null }))
      .toThrowError(/cacheDir/);
  });

  it("throws when $storageRoot referenced but env has empty storageRoot", () => {
    expect(() => resolveCapabilityTemplate("$storageRoot", { ...env, storageRoot: "" }))
      .toThrowError(/storageRoot/);
  });

  it("throws when $pluginDir referenced but env has empty pluginDir", () => {
    expect(() => resolveCapabilityTemplate("$pluginDir", { ...env, pluginDir: "" }))
      .toThrowError(/pluginDir/);
  });

  it("substitutes multiple variables in one string", () => {
    expect(resolveCapabilityTemplate("$storageRoot/$pluginDir", env))
      .toBe("/abs/storage//abs/plugin");
  });

  it("identifier scan stops at non-identifier chars", () => {
    expect(resolveCapabilityTemplate("$storageRoot/foo", env))
      .toBe("/abs/storage/foo");
  });

  it("RECOGNISED_VARIABLES exposes the expected set", () => {
    expect([...RECOGNISED_VARIABLES].sort()).toEqual(
      ["cacheDir", "pluginDir", "storageRoot"]
    );
  });
});

describe("hasTemplate", () => {
  it("detects single $variable", () => {
    expect(hasTemplate("$storageRoot")).toBe(true);
  });

  it("ignores $$ escapes", () => {
    expect(hasTemplate("a$$b")).toBe(false);
  });

  it("detects template among $$ escapes", () => {
    expect(hasTemplate("$$literal $storageRoot $$")).toBe(true);
  });

  it("returns false on plain strings", () => {
    expect(hasTemplate("plain string")).toBe(false);
  });

  it("returns false on bare $ with no identifier following", () => {
    expect(hasTemplate("foo $ bar")).toBe(false);
  });

  it("returns false on empty string", () => {
    expect(hasTemplate("")).toBe(false);
  });
});
