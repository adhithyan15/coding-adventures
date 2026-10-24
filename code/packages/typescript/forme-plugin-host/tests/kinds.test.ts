import { describe, expect, it } from "vitest";
import type {
  KindContribution,
  Manifest,
  StageContribution,
} from "@coding-adventures/forme-manifest";
import { Kinds } from "@coding-adventures/forme-types";
import {
  buildKindRegistry,
  descriptorForKindReference,
  PluginHostError,
} from "../src/index.js";

function manifest(
  pluginName: string,
  kinds: readonly KindContribution[],
  stages: readonly StageContribution[] = [],
): Manifest {
  return {
    manifestVersion: 1,
    plugin: {
      name: pluginName,
      version: "1.0.0",
      apiVersion: 2,
    },
    runtime: { kind: "node", entry: "./entry.mjs" },
    capabilities: { required: [], optional: [] },
    contributes: { stages, kinds },
  };
}

describe("buildKindRegistry", () => {
  it("binds direct and transitive parents to frozen descriptor snapshots", () => {
    const base = { name: "ext:blog-post", version: "1.1", subtypeOf: "ContentNode" };
    const child = { name: "ext:featured-post", version: "1.2", subtypeOf: "ext:blog-post" };
    const registry = buildKindRegistry([manifest("@example/content", [base, child])]);

    const baseRegistration = registry.lookup("ext:blog-post");
    const childRegistration = registry.lookup("ext:featured-post");
    expect(baseRegistration).toEqual({
      descriptor: { name: "ext:blog-post", version: "1.1" },
      parent: Kinds.ContentNode,
    });
    expect(childRegistration?.parent).toBe(baseRegistration?.descriptor);
    expect(Object.isFrozen(baseRegistration)).toBe(true);
    expect(Object.isFrozen(baseRegistration?.descriptor)).toBe(true);
    expect(Object.isFrozen(registry)).toBe(true);
    expect("set" in registry).toBe(false);

    // The registry owns snapshots, not live references into caller data.
    (base as { version: string }).version = "9.9";
    expect(registry.lookup("ext:blog-post")?.descriptor.version).toBe("1.1");
  });

  it("resolves a globally registered extension kind for another plugin's stage", () => {
    const owner = manifest("@example/content", [
      { name: "ext:blog-post", version: "1.0", subtypeOf: "ContentNode" },
    ]);
    const consumer = manifest("@example/render", [], [{
      id: "render",
      consumes: "ext:blog-post",
      produces: "RenderedPage",
    }]);
    const registry = buildKindRegistry([owner, consumer]);
    expect(descriptorForKindReference("ext:blog-post", consumer, registry))
      .toBe(registry.lookup("ext:blog-post")?.descriptor);
  });

  it("rejects duplicate global extension-kind names", () => {
    expect(() => buildKindRegistry([
      manifest("@example/one", [{ name: "ext:post", version: "1.0" }]),
      manifest("@example/two", [{ name: "ext:post", version: "1.1" }]),
    ])).toThrowError(PluginHostError);
    expect(() => buildKindRegistry([
      manifest("@example/one", [{ name: "ext:post", version: "1.0" }]),
      manifest("@example/two", [{ name: "ext:post", version: "1.1" }]),
    ])).toThrow(/duplicate kind name.*ext:post/i);
  });

  it("rejects missing subtype parents", () => {
    expect(() => buildKindRegistry([manifest("@example/one", [{
      name: "ext:post",
      version: "1.0",
      subtypeOf: "ext:missing",
    }])])).toThrow(/missing parent.*ext:missing/i);
  });

  it("rejects direct and transitive subtype cycles", () => {
    expect(() => buildKindRegistry([manifest("@example/direct", [{
      name: "ext:self",
      version: "1.0",
      subtypeOf: "ext:self",
    }])])).toThrow(/cycle.*ext:self/i);

    expect(() => buildKindRegistry([manifest("@example/cycle", [
      { name: "ext:a", version: "1.0", subtypeOf: "ext:b" },
      { name: "ext:b", version: "1.0", subtypeOf: "ext:c" },
      { name: "ext:c", version: "1.0", subtypeOf: "ext:a" },
    ])])).toThrow(/cycle.*ext:/i);
  });
});
