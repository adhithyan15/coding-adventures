import { describe, it, expect } from "vitest";
import { webExtensionPlugin, buildManifests } from "../src/vite-plugin";
import type { ManifestV3 } from "../src/manifest-transformer";

/**
 * Vite Plugin Tests
 * =================
 *
 * These tests verify:
 * 1. The plugin factory returns a valid Vite plugin object
 * 2. The `buildManifests` utility produces correct per-browser output
 */

describe("webExtensionPlugin", () => {
  it("returns a plugin with the correct name", () => {
    const plugin = webExtensionPlugin();
    expect(plugin.name).toBe("vite-plugin-web-extension");
  });

  it("has configResolved hook", () => {
    const plugin = webExtensionPlugin();
    expect(typeof plugin.configResolved).toBe("function");
  });

  it("has generateBundle hook", () => {
    const plugin = webExtensionPlugin();
    expect(typeof plugin.generateBundle).toBe("function");
  });

  it("accepts custom options", () => {
    const plugin = webExtensionPlugin({
      manifest: "custom-manifest.json",
      browsers: ["chrome", "firefox"],
    });
    expect(plugin.name).toBe("vite-plugin-web-extension");
  });

  it("configResolved stores the project root", () => {
    const plugin = webExtensionPlugin();
    // Should not throw when called with a config object
    expect(() => plugin.configResolved?.({ root: "/some/path" })).not.toThrow();
  });

  it("generateBundle does not throw", () => {
    const plugin = webExtensionPlugin();
    plugin.configResolved?.({ root: "/test" });
    expect(() => plugin.generateBundle?.({}, {})).not.toThrow();
  });
});

describe("buildManifests", () => {
  const BASE: ManifestV3 = {
    manifest_version: 3,
    name: "Test",
    version: "1.0.0",
    browser_specific_settings: {
      gecko: { id: "test@example.com" },
    },
  };

  it("builds for all three browsers by default", () => {
    const results = buildManifests(BASE);
    expect(results).toHaveLength(3);

    const browsers = results.map(([browser]) => browser);
    expect(browsers).toContain("chrome");
    expect(browsers).toContain("firefox");
    expect(browsers).toContain("safari");
  });

  it("builds for specified browsers only", () => {
    const results = buildManifests(BASE, ["chrome"]);
    expect(results).toHaveLength(1);
    expect(results[0][0]).toBe("chrome");
  });

  it("chrome manifest has no browser_specific_settings", () => {
    const results = buildManifests(BASE);
    const [, chromeManifest] = results.find(([b]) => b === "chrome")!;
    expect(chromeManifest.browser_specific_settings).toBeUndefined();
  });

  it("firefox manifest retains gecko id", () => {
    const results = buildManifests(BASE);
    const [, firefoxManifest] = results.find(([b]) => b === "firefox")!;
    expect(firefoxManifest.browser_specific_settings?.gecko?.id).toBe(
      "test@example.com",
    );
  });

  it("safari manifest has no browser_specific_settings", () => {
    const results = buildManifests(BASE);
    const [, safariManifest] = results.find(([b]) => b === "safari")!;
    expect(safariManifest.browser_specific_settings).toBeUndefined();
  });

  it("all manifests preserve core fields", () => {
    const results = buildManifests(BASE);
    for (const [, manifest] of results) {
      expect(manifest.manifest_version).toBe(3);
      expect(manifest.name).toBe("Test");
      expect(manifest.version).toBe("1.0.0");
    }
  });
});
