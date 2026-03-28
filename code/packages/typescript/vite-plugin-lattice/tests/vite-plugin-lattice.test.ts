/**
 * vite-plugin-lattice.test.ts — Unit tests for the Lattice Vite plugin.
 *
 * Tests the plugin factory function and its basic properties.
 * Transform behavior is tested via the Playwright e2e tests in the
 * todo-app (which exercise the full Vite + plugin pipeline).
 *
 * The transform hook now uses ssrLoadModule (async, requires server),
 * so we test the plugin's configuration and initialization here, not
 * the transpilation itself (which is covered by lattice-transpiler tests).
 */

import { describe, it, expect, vi } from "vitest";
import { latticePlugin } from "../src/index.js";

describe("latticePlugin", () => {
  it("returns a Vite plugin object with correct name", () => {
    const plugin = latticePlugin();
    expect(plugin.name).toBe("vite-plugin-lattice");
  });

  it("has a transform function", () => {
    const plugin = latticePlugin();
    expect(typeof plugin.transform).toBe("function");
  });

  it("has a handleHotUpdate function", () => {
    const plugin = latticePlugin();
    expect(typeof plugin.handleHotUpdate).toBe("function");
  });

  it("has a configureServer function", () => {
    const plugin = latticePlugin();
    expect(typeof plugin.configureServer).toBe("function");
  });

  describe("transform", () => {
    it("returns null for non-.lattice files", async () => {
      const plugin = latticePlugin();
      const transform = plugin.transform as (code: string, id: string) => Promise<unknown>;
      const ctx = { error: vi.fn() };

      expect(await transform.call(ctx, "body { color: red; }", "app.css")).toBeNull();
      expect(await transform.call(ctx, "const x = 1;", "app.ts")).toBeNull();
      expect(await transform.call(ctx, "<div/>", "app.tsx")).toBeNull();
    });
  });

  describe("options", () => {
    it("accepts empty options", () => {
      expect(() => latticePlugin()).not.toThrow();
    });

    it("accepts minified option", () => {
      expect(() => latticePlugin({ minified: true })).not.toThrow();
    });

    it("accepts indent option", () => {
      expect(() => latticePlugin({ indent: "\t" })).not.toThrow();
    });

    it("accepts both options", () => {
      expect(() => latticePlugin({ minified: true, indent: "    " })).not.toThrow();
    });
  });
});
