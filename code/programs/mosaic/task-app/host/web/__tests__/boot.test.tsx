// boot() end-to-end over a failing start (#13695).
//
// startup.test.tsx covers what the two states look like. This covers the thing
// that was actually broken: the *sequencing*. Previously nothing rendered until
// every await had resolved, and the promise was floated, so a rejection left
// #root empty permanently.
//
// The success path is not reachable here — it needs a real compiled
// task_engine.wasm, which presentation-contract.test.ts and the live browser
// build already cover. What is asserted here is the property that failed:
// the root is never empty, and a failure is recoverable in-page.
import { act } from "react";
import { afterAll, afterEach, beforeAll, describe, expect, it, vi } from "vitest";

// Imported before #root exists, so the module-level auto-boot guard does not
// fire and each test drives boot() explicitly.
import { boot } from "../src/main";

// One #root for the whole file, matching production: the host creates a single
// React root per document and reuses it, so that retry does not call
// createRoot twice on a container React already owns. Recreating the element
// per test would leave boot() rendering into a detached node.
let root: HTMLDivElement;

beforeAll(() => {
  root = document.createElement("div");
  root.id = "root";
  document.body.appendChild(root);
});

afterAll(() => {
  root.remove();
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("boot", () => {
  it("paints the loading state before initialization resolves", async () => {
    let release!: () => void;
    const blocked = new Promise<Response>((resolve) => {
      release = () => resolve(new Response(null, { status: 503 }));
    });
    vi.stubGlobal("fetch", vi.fn(() => blocked));

    const running = boot();
    // Not awaited: this is the window that used to be a blank page.
    await act(async () => {});
    expect(root.textContent).toContain("Starting Trestle");

    release();
    await act(async () => {
      await running;
    });
  });

  it("replaces the blank page with a recoverable failure when the engine 404s", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response("not found", { status: 404, statusText: "Not Found" })),
    );

    await act(async () => {
      await boot();
    });

    expect(root.textContent).toContain("Trestle could not start.");
    // The status is surfaced rather than swallowed into a CompileError, which
    // is what a 404 used to become once the error page reached WebAssembly.
    expect(root.textContent).toContain("404");
    expect(root.querySelector("button")?.textContent).toBe("Try again");
  });

  it("reports a network failure instead of leaving the root empty", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => {
        throw new TypeError("Failed to fetch");
      }),
    );

    await act(async () => {
      await boot();
    });

    expect(root.textContent).toContain("Trestle could not start.");
    expect(root.textContent).toContain("Failed to fetch");
    expect(root.textContent.trim()).not.toBe("");
  });

  it("retries in place, without a manual reload", async () => {
    const fetchMock = vi.fn(async () => {
      throw new TypeError("Failed to fetch");
    });
    vi.stubGlobal("fetch", fetchMock);

    await act(async () => {
      await boot();
    });
    expect(fetchMock).toHaveBeenCalledTimes(1);

    await act(async () => {
      root.querySelector("button")?.click();
    });

    // The retry re-ran initialization rather than only re-rendering.
    expect(fetchMock.mock.calls.length).toBeGreaterThan(1);
    // Still failing, so the user is still told so rather than dropped to blank.
    expect(root.textContent).toContain("Trestle could not start.");
  });
});
