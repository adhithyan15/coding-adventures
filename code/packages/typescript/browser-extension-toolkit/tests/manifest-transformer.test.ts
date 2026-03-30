import { describe, it, expect } from "vitest";
import { transformManifest, type ManifestV3 } from "../src/manifest-transformer";

/**
 * Manifest Transformer Tests
 * ==========================
 *
 * These tests verify that the manifest transformer correctly produces
 * browser-specific manifest variants from a base manifest.
 *
 * The base manifest contains ALL fields, including browser-specific ones.
 * Each browser transform should produce a clean manifest with only the
 * fields that browser supports.
 */

/** A complete base manifest with all fields populated. */
const BASE_MANIFEST: ManifestV3 = {
  manifest_version: 3,
  name: "Test Extension",
  version: "1.0.0",
  description: "A test extension for unit testing",
  action: {
    default_popup: "popup.html",
    default_icon: {
      "16": "icons/icon-16.png",
      "48": "icons/icon-48.png",
      "128": "icons/icon-128.png",
    },
  },
  icons: {
    "16": "icons/icon-16.png",
    "48": "icons/icon-48.png",
    "128": "icons/icon-128.png",
  },
  background: {
    service_worker: "service-worker.js",
  },
  permissions: ["storage"],
  browser_specific_settings: {
    gecko: {
      id: "test-ext@coding-adventures",
      strict_min_version: "109.0",
    },
  },
};

describe("transformManifest", () => {
  // ======================================================================
  // Chrome transforms
  // ======================================================================

  describe("chrome", () => {
    it("removes browser_specific_settings", () => {
      const result = transformManifest(BASE_MANIFEST, "chrome");
      expect(result.browser_specific_settings).toBeUndefined();
    });

    it("preserves all standard fields", () => {
      const result = transformManifest(BASE_MANIFEST, "chrome");
      expect(result.manifest_version).toBe(3);
      expect(result.name).toBe("Test Extension");
      expect(result.version).toBe("1.0.0");
      expect(result.description).toBe("A test extension for unit testing");
      expect(result.action?.default_popup).toBe("popup.html");
      expect(result.icons).toEqual(BASE_MANIFEST.icons);
      expect(result.background?.service_worker).toBe("service-worker.js");
      expect(result.permissions).toEqual(["storage"]);
    });

    it("does not modify the original manifest", () => {
      const original = { ...BASE_MANIFEST };
      transformManifest(BASE_MANIFEST, "chrome");
      expect(BASE_MANIFEST).toEqual(original);
    });
  });

  // ======================================================================
  // Firefox transforms
  // ======================================================================

  describe("firefox", () => {
    it("preserves browser_specific_settings with gecko id", () => {
      const result = transformManifest(BASE_MANIFEST, "firefox");
      expect(result.browser_specific_settings?.gecko?.id).toBe(
        "test-ext@coding-adventures",
      );
    });

    it("preserves all standard fields", () => {
      const result = transformManifest(BASE_MANIFEST, "firefox");
      expect(result.manifest_version).toBe(3);
      expect(result.name).toBe("Test Extension");
      expect(result.version).toBe("1.0.0");
      expect(result.action?.default_popup).toBe("popup.html");
      expect(result.background?.service_worker).toBe("service-worker.js");
    });

    it("throws when gecko id is missing", () => {
      const noGecko: ManifestV3 = {
        manifest_version: 3,
        name: "Test",
        version: "1.0.0",
      };

      expect(() => transformManifest(noGecko, "firefox")).toThrow(
        "Firefox manifest requires browser_specific_settings.gecko.id",
      );
    });

    it("throws when browser_specific_settings exists but gecko.id is empty", () => {
      const emptyGecko: ManifestV3 = {
        manifest_version: 3,
        name: "Test",
        version: "1.0.0",
        browser_specific_settings: {
          gecko: {},
        },
      };

      expect(() => transformManifest(emptyGecko, "firefox")).toThrow(
        "Firefox manifest requires browser_specific_settings.gecko.id",
      );
    });
  });

  // ======================================================================
  // Safari transforms
  // ======================================================================

  describe("safari", () => {
    it("removes browser_specific_settings", () => {
      const result = transformManifest(BASE_MANIFEST, "safari");
      expect(result.browser_specific_settings).toBeUndefined();
    });

    it("preserves all standard fields", () => {
      const result = transformManifest(BASE_MANIFEST, "safari");
      expect(result.manifest_version).toBe(3);
      expect(result.name).toBe("Test Extension");
      expect(result.version).toBe("1.0.0");
      expect(result.action?.default_popup).toBe("popup.html");
      expect(result.background?.service_worker).toBe("service-worker.js");
    });
  });

  // ======================================================================
  // Edge cases
  // ======================================================================

  describe("edge cases", () => {
    it("handles manifest with no optional fields", () => {
      const minimal: ManifestV3 = {
        manifest_version: 3,
        name: "Minimal",
        version: "1.0.0",
      };

      const chrome = transformManifest(minimal, "chrome");
      expect(chrome.manifest_version).toBe(3);
      expect(chrome.name).toBe("Minimal");
      expect(chrome.browser_specific_settings).toBeUndefined();
    });

    it("preserves unknown/custom fields", () => {
      const withCustom: ManifestV3 = {
        manifest_version: 3,
        name: "Custom",
        version: "1.0.0",
        browser_specific_settings: {
          gecko: { id: "custom@test" },
        },
        custom_field: "preserved",
      };

      const chrome = transformManifest(withCustom, "chrome");
      expect(chrome.custom_field).toBe("preserved");

      const firefox = transformManifest(withCustom, "firefox");
      expect(firefox.custom_field).toBe("preserved");
    });

    it("handles content_scripts field", () => {
      const withContentScripts: ManifestV3 = {
        manifest_version: 3,
        name: "Content",
        version: "1.0.0",
        browser_specific_settings: {
          gecko: { id: "content@test" },
        },
        content_scripts: [
          {
            matches: ["https://*.example.com/*"],
            js: ["content.js"],
            css: ["content.css"],
            run_at: "document_idle",
          },
        ],
      };

      const chrome = transformManifest(withContentScripts, "chrome");
      expect(chrome.content_scripts).toHaveLength(1);
      expect(chrome.content_scripts![0].matches).toEqual([
        "https://*.example.com/*",
      ]);
    });
  });
});
