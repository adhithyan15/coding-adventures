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


it("renders selected-cell results and an atomic polite announcement from Rust", async () => {
  const module = await loadMosaicModule(readFileSync("public/visicalc_mosaic_app.wasm"));
  const host = module.create({ colorScheme: "light" });
  const container = document.createElement("div"); document.body.append(container);
  const root = createRoot(container);
  try {
    await act(async () => { root.render(<App load={async () => host} />); });
    const status = container.querySelector('[role="status"]')!;
    expect(status.getAttribute("aria-live")).toBe("polite");
    expect(status.getAttribute("aria-atomic")).toBe("true");
    const cell = container.querySelectorAll('tbody:not([data-mosaic-spacer]) tr')[0].querySelectorAll('td')[4];
    await act(async () => { (cell.firstElementChild as HTMLElement).click(); });
    expect(status.textContent).toBe("E1, 38, formula =SUM(A1:D1)");
    const summary = [...container.querySelectorAll('span')].find(node => node.textContent === status.textContent);
    expect(summary).toBeDefined();
    expect((summary as HTMLElement).style.textOverflow).toBe("ellipsis");
    const input = container.querySelector<HTMLInputElement>('input[placeholder="Enter a value or formula"]')!;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, "42");
      input.dispatchEvent(new Event("input", {bubbles:true}));
    });
    expect(status.textContent).toBe("");
    await act(async () => { input.dispatchEvent(new KeyboardEvent("keydown", {key:"Enter",bubbles:true})); });
    expect(status.textContent).toBe("Updated E1, 42");
    expect(summary!.textContent).toBe("E1, 42");
  } finally { await act(async () => root.unmount()); container.remove(); }
});

it.each(["light", "dark"] as const)("guides an empty workbook without replacing the editable grid (%s)", async colorScheme => {
  const module = await loadMosaicModule(readFileSync("public/visicalc_mosaic_app.wasm"));
  const host = module.create({ colorScheme });
  const container = document.createElement("div"); document.body.append(container);
  const root = createRoot(container);
  const introduction = () => [...container.querySelectorAll("h2")].find(h => h.textContent === "Room for your next idea");
  try {
    await act(async () => root.render(<App load={async () => host} />));
    expect(introduction()).toBeUndefined();
    const newWorkbook = [...container.querySelectorAll("button")].find(b => b.textContent === "New workbook")!;
    await act(async () => newWorkbook.click());
    expect(introduction()).toBeDefined();
    expect(container.textContent).toContain("a formula like =2+3");
    const table = container.querySelector("table")!;
    expect(table.querySelectorAll("thead th[scope='col']")).toHaveLength(26);
    const field = container.querySelector<HTMLInputElement>('input[placeholder="Enter a value or formula"]')!;
    const type = async (value: string) => act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(field, value);
      field.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await type("=2+3");
    expect(introduction()).toBeDefined();
    await act(async () => field.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })));
    expect(introduction()).toBeUndefined();
    expect(container.querySelector("table")).toBe(table);
    expect(table.querySelector("tbody:not([data-mosaic-spacer]) td")?.textContent).toBe("5");
    await type("");
    await act(async () => field.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })));
    expect(introduction()).toBeDefined();
  } finally { await act(async () => root.unmount()); container.remove(); }
});
