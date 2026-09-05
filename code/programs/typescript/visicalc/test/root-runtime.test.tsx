import { readFileSync } from "node:fs";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it } from "vitest";
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
    expect(container.querySelectorAll("tbody tr")[4].querySelectorAll("td")[4].textContent).toBe("174");
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
    const rows = container.querySelectorAll("tbody tr");
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
