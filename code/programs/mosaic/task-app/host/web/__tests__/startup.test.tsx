// Startup states (#13695).
//
// The defect these cover: boot() awaited the WASM fetch, the compile, and the
// workspace restore before rendering anything, and floated the resulting
// promise. Any rejection therefore left #root empty with no message and no way
// back — the failure was visible only in the browser console.
//
// These tests assert the two things that fixes it: something is always painted,
// and a failure is recoverable without a manual reload.
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { StartupFailure, StartupLoading } from "../src/startup";

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
  vi.restoreAllMocks();
});

const render = (node: React.ReactNode) => act(() => root.render(node));

describe("startup states", () => {
  it("paints a loading state instead of an empty root", () => {
    render(<StartupLoading theme="light" />);

    expect(container.textContent).toContain("Starting Trestle");
    // Announced politely: it is progress, not a problem.
    const live = container.querySelector('[role="status"]');
    expect(live).not.toBeNull();
    expect(live?.getAttribute("aria-live")).toBe("polite");
  });

  it("states the failure, keeps the detail, and offers a way back", () => {
    const onRetry = vi.fn();
    render(
      <StartupFailure theme="light" detail="HTTP 404 Not Found" onRetry={onRetry} />,
    );

    expect(container.textContent).toContain("Trestle could not start.");
    // The reassurance matters as much as the error: a startup failure must not
    // read as data loss, because nothing has been written at this point.
    expect(container.textContent).toContain("Your saved tasks have not been changed");
    expect(container.textContent).toContain("HTTP 404 Not Found");

    const button = container.querySelector("button");
    expect(button?.textContent).toBe("Try again");
    act(() => button?.click());
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it("announces a failure assertively rather than politely", () => {
    render(<StartupFailure theme="dark" detail="x" onRetry={() => {}} />);

    const live = container.querySelector('[role="alert"]');
    expect(live?.getAttribute("aria-live")).toBe("assertive");
    expect(live?.getAttribute("aria-atomic")).toBe("true");
  });

  it("renders the detail as text, never as markup", () => {
    // The detail is an error message, and an error message can carry attacker-
    // influenced bytes (a URL, a server's response text). It must never reach
    // the DOM as HTML.
    render(
      <StartupFailure
        theme="light"
        detail={'<img src=x onerror="throw new Error(1)">'}
        onRetry={() => {}}
      />,
    );

    expect(container.querySelector("img")).toBeNull();
    expect(container.textContent).toContain('<img src=x onerror=');
  });

  it("uses the authored shell colours for each theme", () => {
    // Startup chrome is the one place the host draws outside mosstyle, so it is
    // worth pinning that it still matches app-shell in both themes.
    render(<StartupLoading theme="light" />);
    const light = container.querySelector('[role="status"]') as HTMLElement;
    expect(light.style.background).toBe("rgb(240, 235, 227)");

    render(<StartupLoading theme="dark" />);
    const dark = container.querySelector('[role="status"]') as HTMLElement;
    expect(dark.style.background).toBe("rgb(26, 23, 20)");
  });
});
