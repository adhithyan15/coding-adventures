import { describe, it, expect, beforeEach, vi } from "vitest";
import { initPopup } from "../src/popup/popup";

/**
 * Popup Tests
 * ===========
 *
 * Testing a browser extension popup requires mocking two things:
 *
 * 1. **The DOM** — The popup script manipulates HTML elements from
 *    popup.html. We set up a minimal DOM in each test.
 *
 * 2. **The browser API** — The popup calls `getBrowserAPI()` which
 *    looks for `chrome` or `browser` on `globalThis`. We mock these
 *    globals to simulate different browser environments.
 *
 * We test three scenarios:
 * - Chrome environment (chrome.* global)
 * - Outside extension context (no globals → graceful fallback)
 * - Missing DOM element (defensive coding → no crash)
 */

describe("initPopup", () => {
  beforeEach(() => {
    // Set up a minimal DOM that matches popup.html's structure.
    // We only need the elements that popup.ts actually touches.
    document.body.innerHTML = `
      <div class="container">
        <h1>Hello World!</h1>
        <p id="browser-info"></p>
      </div>
    `;

    // Clean up global mocks between tests.
    // Both `chrome` and `browser` might be set by previous tests.
    const g = globalThis as Record<string, unknown>;
    delete g.chrome;
    delete g.browser;
  });

  it("displays extension info when Chrome API is available", () => {
    // Simulate Chrome environment by setting the `chrome` global
    const g = globalThis as Record<string, unknown>;
    g.chrome = {
      runtime: {
        getManifest: () => ({
          name: "Hello World",
          version: "0.1.0",
        }),
        onInstalled: { addListener: vi.fn() },
      },
    };

    initPopup();

    const info = document.getElementById("browser-info");
    expect(info?.textContent).toBe("Running Hello World v0.1.0");
  });

  it("displays extension info when Firefox API is available", () => {
    // Simulate Firefox environment by setting the `browser` global
    const g = globalThis as Record<string, unknown>;
    g.browser = {
      runtime: {
        getManifest: () => ({
          name: "Hello World",
          version: "0.1.0",
        }),
        onInstalled: { addListener: vi.fn() },
      },
    };

    initPopup();

    const info = document.getElementById("browser-info");
    expect(info?.textContent).toBe("Running Hello World v0.1.0");
  });

  it("shows fallback message when outside extension context", () => {
    // No browser API mocked — getBrowserAPI() will throw
    initPopup();

    const info = document.getElementById("browser-info");
    expect(info?.textContent).toBe("Running outside extension context");
  });

  it("does not crash when browser-info element is missing", () => {
    // Remove the element that popup.ts looks for
    document.body.innerHTML = "<div>no info element here</div>";

    // Should complete without throwing
    expect(() => initPopup()).not.toThrow();
  });

  it("prefers browser API over chrome API", () => {
    // Simulate Firefox which defines both globals
    const g = globalThis as Record<string, unknown>;
    g.browser = {
      runtime: {
        getManifest: () => ({ name: "Firefox Version", version: "2.0.0" }),
        onInstalled: { addListener: vi.fn() },
      },
    };
    g.chrome = {
      runtime: {
        getManifest: () => ({ name: "Chrome Version", version: "1.0.0" }),
        onInstalled: { addListener: vi.fn() },
      },
    };

    initPopup();

    const info = document.getElementById("browser-info");
    expect(info?.textContent).toBe("Running Firefox Version v2.0.0");
  });
});
