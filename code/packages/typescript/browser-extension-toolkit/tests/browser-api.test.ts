import { describe, it, expect, beforeEach, vi } from "vitest";
import { getBrowserAPI } from "../src/browser-api";

/**
 * Browser API Shim Tests
 * ======================
 *
 * These tests verify that the cross-browser API shim correctly detects
 * which browser environment we're running in and returns the appropriate
 * global object.
 *
 * We test three scenarios:
 * 1. Firefox/Safari: `browser` global exists → return `browser`
 * 2. Chrome: only `chrome` global exists → return `chrome`
 * 3. Neither exists → throw an error
 *
 * Plus an edge case:
 * 4. Both exist (Firefox defines both) → prefer `browser`
 */

describe("getBrowserAPI", () => {
  beforeEach(() => {
    // Clean up any global mocks between tests.
    // We use `globalThis` because that's what the shim checks.
    const g = globalThis as Record<string, unknown>;
    delete g.browser;
    delete g.chrome;
  });

  it("returns the `browser` global when available (Firefox/Safari)", () => {
    const mockBrowser = {
      runtime: {
        getManifest: vi.fn(() => ({ name: "Test", version: "1.0" })),
        onInstalled: { addListener: vi.fn() },
      },
    };

    (globalThis as Record<string, unknown>).browser = mockBrowser;

    const api = getBrowserAPI();
    expect(api).toBe(mockBrowser);
  });

  it("returns the `chrome` global when `browser` is not available (Chrome)", () => {
    const mockChrome = {
      runtime: {
        getManifest: vi.fn(() => ({ name: "Test", version: "1.0" })),
        onInstalled: { addListener: vi.fn() },
      },
    };

    (globalThis as Record<string, unknown>).chrome = mockChrome;

    const api = getBrowserAPI();
    expect(api).toBe(mockChrome);
  });

  it("prefers `browser` over `chrome` when both exist", () => {
    // Firefox defines both `browser` (promise-based) and `chrome`
    // (callback-based). We should prefer the promise-based one.
    const mockBrowser = {
      runtime: {
        getManifest: vi.fn(() => ({ name: "Firefox", version: "1.0" })),
        onInstalled: { addListener: vi.fn() },
      },
    };
    const mockChrome = {
      runtime: {
        getManifest: vi.fn(() => ({ name: "Chrome", version: "1.0" })),
        onInstalled: { addListener: vi.fn() },
      },
    };

    const g = globalThis as Record<string, unknown>;
    g.browser = mockBrowser;
    g.chrome = mockChrome;

    const api = getBrowserAPI();
    expect(api).toBe(mockBrowser);
  });

  it("throws when no browser extension API is available", () => {
    // No globals set — simulates running in Node.js without mocking
    expect(() => getBrowserAPI()).toThrow("No browser extension API found");
  });

  it("ignores non-object values on globalThis", () => {
    // Edge case: `browser` or `chrome` might be set to a non-object
    // value by some other script
    const g = globalThis as Record<string, unknown>;
    g.browser = "not an object";
    g.chrome = 42;

    expect(() => getBrowserAPI()).toThrow("No browser extension API found");
  });
});
