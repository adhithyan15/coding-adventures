import { readFileSync } from "node:fs";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { App } from "../src/app/App";
import { loadMosaicModule, type MosaicHost } from "../../../../packages/rust/mosaic-app-wasm/js/mosaic-host.mjs";

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
afterEach(() => vi.unstubAllGlobals());
function deferred() {
  let resolve!: (host: MosaicHost) => void;
  const promise = new Promise<MosaicHost>(done => { resolve = done; });
  return { promise, resolve };
}
async function application() {
  const module = await loadMosaicModule(readFileSync("public/visicalc_mosaic_app.wasm"));
  return module.create();
}

it.each([false, true])("renders a generated startup screen in the preferred theme (dark=%s)", async dark => {
  vi.stubGlobal("matchMedia", () => ({ matches: dark }));
  const pending = deferred();
  const container = document.createElement("div"); document.body.append(container);
  const root = createRoot(container);
  const host = await application();
  const dispose = vi.spyOn(host, "dispose");
  try {
    await act(async () => root.render(<App load={() => pending.promise} />));
    expect(container.querySelector('[role="status"]')?.textContent).toContain("Opening your workbook");
    expect(container.querySelector("button")).toBeNull();
    const surface = container.querySelector('[role="status"] > div') as HTMLElement;
    expect(surface.style.background).toBe(dark ? "rgb(20, 34, 30)" : "rgb(244, 243, 237)");
    await act(async () => root.unmount());
    await act(async () => pending.resolve(host));
    expect(dispose).toHaveBeenCalledTimes(1);
  } finally { container.remove(); }
});

it("retries a failed load and focuses the real workbook without reloading the page", async () => {
  const host = await application();
  const pending = deferred();
  const load = vi.fn<() => Promise<MosaicHost>>()
    .mockRejectedValueOnce(new Error("low-level WASM failure"))
    .mockImplementationOnce(() => pending.promise);
  const container = document.createElement("div"); document.body.append(container);
  const root = createRoot(container);
  try {
    await act(async () => root.render(<App load={load} />));
    expect(container.querySelector('[role="alert"]')?.textContent).toContain("Check your connection and try again");
    expect(container.textContent).not.toContain("low-level WASM failure");
    const retry = container.querySelector<HTMLButtonElement>("button")!;
    expect(retry.textContent).toBe("Try again");
    await act(async () => { retry.focus(); retry.click(); });
    expect(load).toHaveBeenCalledTimes(2);
    expect(container.querySelector("button")).toBeNull();
    expect(container.textContent).toContain("Opening your workbook");
    await act(async () => pending.resolve(host));
    expect(container.querySelector('input[placeholder="Enter a value or formula"]')).not.toBeNull();
    expect(document.activeElement).toBe(container.querySelector('table[tabindex="0"]'));
  } finally { await act(async () => root.unmount()); container.remove(); }
});

it("catches synchronous load failures and disposes stale results from replaced loaders", async () => {
  const old = deferred();
  const oldHost = await application();
  const newHost = await application();
  const oldDispose = vi.spyOn(oldHost, "dispose");
  const container = document.createElement("div"); document.body.append(container);
  const root = createRoot(container);
  try {
    await act(async () => root.render(<App load={() => { throw new Error("sync failure"); }} />));
    expect(container.textContent).toContain("Your workbook couldn’t open");
    await act(async () => root.render(<App load={() => old.promise} />));
    expect(container.querySelector('[role="alert"]')).toBeNull();
    await act(async () => root.render(<App load={async () => newHost} />));
    await act(async () => old.resolve(oldHost));
    expect(oldDispose).toHaveBeenCalledTimes(1);
    expect(container.querySelector('table[tabindex="0"]')).not.toBeNull();
    expect(newHost.update.props.formula).toBe("15");
  } finally { await act(async () => root.unmount()); container.remove(); }
});
