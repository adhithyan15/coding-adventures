import { readFileSync } from "node:fs";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import { App } from "../src/app/App";
import { loadMosaicModule } from "../../../../packages/rust/mosaic-app-wasm/js/mosaic-host.mjs";
Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
it("edits through the generated root and real Rust lifecycle", async () => {
  const module = await loadMosaicModule(readFileSync("public/visicalc_mosaic_app.wasm"));
  const host = module.create({ colorScheme: "light" });
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  try {
    await act(async () => { root.render(<App load={async () => host} />); });
    const input = container.querySelector<HTMLInputElement>('input[placeholder="Enter a value or formula"]')!;
    expect(input.value).toBe("15");
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, "20");
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => { input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })); });
    expect(container.querySelectorAll("tbody:not([data-mosaic-spacer]) tr")[4].querySelectorAll("td")[4].textContent).toBe("174");
    expect(host.update.props.formula).toBe("20");
  } finally {
    await act(async () => root.unmount());
    container.remove();
  }
});

it("keeps Z100 edits and absolute labels independent of the row-header column", async () => {
  const module = await loadMosaicModule(readFileSync("public/visicalc_mosaic_app.wasm"));
  const host = module.create({ colorScheme: "dark" });
  host.dispatch("navigate", { row: 99, col: 25 });
  host.dispatch("resizeViewport", { rows: 3 });
  const container = document.createElement("div"); document.body.append(container);
  const root = createRoot(container);
  try {
    await act(async () => { root.render(<App load={async () => host} />); });
    expect([...container.querySelectorAll("tbody th[scope='row']")].map(cell => cell.textContent)).toEqual(["98", "99", "100"]);
    expect(container.querySelectorAll("thead th[scope='col']")).toHaveLength(26);
    const rows = container.querySelectorAll("tbody:not([data-mosaic-spacer]) tr");
    expect(rows[2].querySelectorAll("td")).toHaveLength(26);
    await act(async () => { (rows[2].querySelectorAll("td")[25].firstElementChild as HTMLElement).click(); });
    expect(host.update.props["cell-address"]).toBe("Z100");
    const input = container.querySelector<HTMLInputElement>('input[placeholder="Enter a value or formula"]')!;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, "123");
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => { input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })); });
    expect(rows[2].querySelectorAll("td")[25].textContent).toBe("123");
    expect(rows[2].querySelectorAll("td")[24].textContent).toBe("");
    expect(container.querySelectorAll("tbody th input")).toHaveLength(0);
  } finally { await act(async () => root.unmount()); container.remove(); }
});


it("owns worksheet shortcuts only with table focus and restores focus after cell editing", async () => {
  const module = await loadMosaicModule(readFileSync("public/visicalc_mosaic_app.wasm"));
  const host = module.create({ colorScheme: "light" });
  const container = document.createElement("div"); document.body.append(container);
  const root = createRoot(container);
  const frames: FrameRequestCallback[] = [];
  vi.stubGlobal("requestAnimationFrame", (fn: FrameRequestCallback) => { frames.push(fn); return frames.length; });
  try {
    await act(async () => { root.render(<App load={async () => host} />); });
    const table = container.querySelector("table")!;
    expect(table.tabIndex).toBe(0); expect(table.getAttribute("aria-label")).toBe("Data table");
    const key = async (target: EventTarget, key: string, shiftKey = false) => {
      await act(async () => { target.dispatchEvent(new KeyboardEvent("keydown", {key,shiftKey,bubbles:true,cancelable:true})); });
    };
    await key(window, "ArrowRight"); expect(host.update.props["cell-address"]).toBe("A1");
    await act(async () => { (table.querySelector("tbody:not([data-mosaic-spacer]) td > div") as HTMLElement).click(); });
    expect(document.activeElement).toBe(table);
    await key(table, "ArrowRight"); expect(host.update.props["cell-address"]).toBe("B1");
    await key(table, "End"); expect(host.update.props["cell-address"]).toBe("Z1");
    await key(table, "Home"); expect(host.update.props["cell-address"]).toBe("A1");
    const formula = container.querySelector<HTMLInputElement>('input[placeholder="Enter a value or formula"]')!;
    formula.focus(); await key(formula, "End"); expect(host.update.props["cell-address"]).toBe("A1");
    table.focus(); await key(table, "H", true);
    const editor = table.querySelector("input")!;
    expect(document.activeElement).toBe(editor); expect(editor.value).toBe("H");
    await key(editor, "Escape");
    await act(async () => { frames.splice(0).forEach(fn => fn(0)); });
    expect(table.querySelector("input")).toBeNull(); expect(document.activeElement).toBe(table);
    expect(host.update.props.formula).toBe("15");
    await key(table, "ArrowDown"); expect(host.update.props["cell-address"]).toBe("A2");
  } finally { await act(async () => root.unmount()); container.remove(); vi.unstubAllGlobals(); }
});
