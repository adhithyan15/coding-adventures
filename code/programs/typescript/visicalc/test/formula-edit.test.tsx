import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { App } from "../src/app/App";

// Exercise generated controls and the actual bundled Rust WASM engine. A mocked
// setCell would miss lost commits and dependent-formula recomputation.
Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
let container: HTMLDivElement;
let root: Root;

beforeEach(async () => {
  const bundle = readFileSync(
    resolve("../visicalc-html/vendor/spreadsheet-engine-wasm.js"),
    "utf8",
  );
  new Function(bundle)();
  await window.__spreadsheetEngineReady;
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  await act(async () => { root.render(<App />); });
});

afterEach(async () => {
  await act(async () => { root?.unmount(); });
  container?.remove();
});

function formulaField() {
  return container.querySelector<HTMLInputElement>('input[placeholder="Enter formula"]')!;
}

function cell(row: number, col: number) {
  return container.querySelectorAll("tbody tr")[row].querySelectorAll("td")[col];
}

async function change(value: string) {
  const input = formulaField();
  await act(async () => {
    input.focus();
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

async function press(key: string) {
  await act(async () => {
    formulaField().dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
  });
}

describe("direct formula-bar editing", () => {
  it("starts an edit and commits once through the generated field to the real engine", async () => {
    expect(cell(4, 4).textContent).toBe("169");
    await change("20");
    expect(formulaField().value).toBe("20");
    await press("Enter");
    expect(cell(0, 0).textContent).toBe("20");
    expect(cell(0, 4).textContent).toBe("43");
    expect(cell(4, 4).textContent).toBe("174");
    await press("Enter");
    expect(container.querySelectorAll("input")).toHaveLength(1);
    expect(cell(4, 4).textContent).toBe("174");
  });

  it("shows formula source and restores it on Escape without mutating dependents", async () => {
    await act(async () => { (cell(0, 4).firstElementChild as HTMLElement).click(); });
    expect(formulaField().value).toBe("=SUM(A1:D1)");
    await change("999");
    expect(formulaField().value).toBe("999");
    await press("Escape");
    expect(formulaField().value).toBe("=SUM(A1:D1)");
    expect(cell(0, 4).textContent).toBe("38");
    expect(cell(4, 4).textContent).toBe("169");
  });

  it("leaves arrow keys in the formula input instead of navigating the grid", async () => {
    formulaField().focus();
    await press("ArrowRight");
    expect(formulaField().value).toBe("15");
    await change("21");
    await press("Enter");
    expect(cell(0, 0).textContent).toBe("21");
    expect(cell(0, 1).textContent).toBe("3");
  });

  it("can clear a populated cell and recompute the budget", async () => {
    await change("");
    await press("Enter");
    expect(cell(0, 0).textContent).toBe("");
    expect(cell(0, 4).textContent).toBe("23");
    expect(cell(4, 4).textContent).toBe("154");
  });
});
