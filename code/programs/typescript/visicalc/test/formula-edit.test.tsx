import { readFileSync } from "node:fs";
import { loadMosaicModule, type MosaicHost, type MosaicModule } from "../../../../packages/rust/mosaic-app-wasm/js/mosaic-host.mjs";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { App } from "../src/app/App";

import contract from "../../../mosaic/visicalc/fixtures/presentation-contract-v1.json";

// Exercise generated controls and the actual bundled Rust WASM engine. A mocked
// setCell would miss lost commits and dependent-formula recomputation.
Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
let container: HTMLDivElement;
let root: Root;
let host: MosaicHost;
let module: MosaicModule;
beforeEach(async () => {
  module = await loadMosaicModule(readFileSync("public/visicalc_mosaic_app.wasm"));
  host = module.create({ colorScheme: "dark" });
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  await act(async () => { root.render(<App load={async () => host} />); });
});
function visibleSelection() {
  const address = container.firstElementChild!.children[1].firstElementChild!.textContent!;
  const match = /^([A-Z]+)(\d+)$/.exec(address)!;
  const row = Number(match[2]) - 1;
  const col = [...match[1]].reduce((value, letter) => value * 26 + letter.charCodeAt(0) - 64, 0) - 1;
  const rows = [...container.querySelectorAll("tbody tr")];
  const relativeRow = rows.findIndex((tr) => [...tr.querySelectorAll("td > div")]
    .some((div) => (div as HTMLElement).style.background === "rgb(38, 79, 120)"
      || (div as HTMLElement).style.background === "rgb(31, 79, 63)"));
  expect(relativeRow, "selection must be in the rendered slice").toBeGreaterThanOrEqual(0);
  return { row, col, offset: row - relativeRow };
}

it("replays the shared presentation contract through generated controls", async () => {
  expect(contract.schemaVersion).toBe(1);
  let editSurface: "formula" | "grid" = "formula";
  for (const step of contract.steps) {
    if (step.event) {
      const { type, payload } = step.event;
      switch (type) {
        case "navigate": {
          const target = payload as { row: number; col: number };
          const current = visibleSelection();
          if (target.row >= current.offset && target.row < current.offset + 30) {
            await act(async () => {
              (cell(target.row - current.offset, target.col).firstElementChild as HTMLElement).click();
            });
          } else {
            await gridKey(target.row < current.row ? "ArrowUp" : "ArrowDown", Math.abs(target.row - current.row));
            await gridKey(target.col < current.col ? "ArrowLeft" : "ArrowRight", Math.abs(target.col - current.col));
          }
          break;
        }
        case "editStart": {
          const target = payload as { row: number; col: number };
          const current = visibleSelection();
          expect({ row: current.row, col: current.col }, step.id).toEqual(target);
          await gridKey("F2");
          editSurface = "grid";
          break;
        }
        case "formulaChange":
          await change((payload as { value: string }).value, editSurface === "grid"
            ? container.querySelector<HTMLInputElement>("tbody input")! : formulaField());
          break;
        case "commit": await press("Enter"); break;
        case "cancel": await press("Escape"); break;
        case "editCommit":
        case "editCancel": {
          const input = container.querySelector<HTMLInputElement>("tbody input")!;
          expect(input, step.id).not.toBeNull();
          await act(async () => {
            input.dispatchEvent(new KeyboardEvent("keydown", {
              key: type === "editCommit" ? "Enter" : "Escape", bubbles: true,
            }));
          });
          editSurface = "formula";
          break;
        }
        default: throw new Error(`Unimplemented fixture event: ${type}`);
      }
    }
    const { slots, engine } = step.expected;
    expect(visibleSelection(), step.id).toEqual({
      row: slots.selectedRow, col: slots.selectedCol, offset: slots.viewportOffset,
    });
    expect(formulaField().value, step.id).toBe(slots.formula);
    expect(!!container.querySelector("tbody input"), step.id).toBe(slots.editing);
    expect(container.querySelectorAll("tbody tr").length, step.id).toBe(slots.viewportSize);
    // Check every displayed cell against the real engine's requested slice.
    // During editing the generated input replaces that one display string.
    const displayed = host.update.props["viewport-rows"] as string[][];
    for (let row = 0; row < displayed.length; row++) {
      for (let col = 0; col < displayed[row].length; col++) {
        if (!cell(row, col).querySelector("input")) {
          expect(cell(row, col).textContent, `${step.id}: visible ${row},${col}`).toBe(displayed[row][col]);
        }
      }
    }
    // Probe a restored copy so uncommitted edits cannot masquerade as source.
    const probe = module.create({ restoredSnapshot: host.snapshot() });
    try {
      for (const [address, [raw, display]] of Object.entries(engine)) {
        const match = /^([A-Z])(\d+)$/.exec(address)!;
        const row = Number(match[2]) - 1, col = match[1].charCodeAt(0) - 65;
        const update = probe.dispatch("navigate", { row, col });
        expect(update.props.formula, `${step.id}: ${address} source`).toBe(raw);
        const offset = Number(update.props["viewport-offset"]);
        expect((update.props["viewport-rows"] as string[][])[row - offset][col], `${step.id}: ${address} value`).toBe(display);
      }
    } finally { probe.dispose(); }
  }
});
afterEach(async () => {
  await act(async () => { root?.unmount(); });
  container?.remove();
});

function formulaField() {
  return container.querySelector<HTMLInputElement>('input[placeholder="Enter a value or formula"]')!;
}

function cell(row: number, col: number) {
  return container.querySelectorAll("tbody tr")[row].querySelectorAll("td")[col];
}

async function change(value: string, input = formulaField()) {
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

async function gridKey(key: string, times = 1) {
  // Flush each key so the next event sees the new presentation cursor.
  for (let index = 0; index < times; index++) {
    await act(async () => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
    });
  }
}

describe("viewport workbook coordinates", () => {
  it("reveals row 31 and translates a click in the shifted slice before committing", async () => {
    await gridKey("ArrowDown", 30);
    expect(container.textContent).toContain("A31");
    expect(container.querySelectorAll("tbody tr")).toHaveLength(30);
    expect((cell(29, 0).firstElementChild as HTMLElement).style.background).toBe("rgb(38, 79, 120)");
    // The first visible row is workbook row 2, not row 1.
    await act(async () => { (cell(0, 0).firstElementChild as HTMLElement).click(); });
    expect(formulaField().value).toBe("8");
    await change("25");
    await press("Enter");
    expect(cell(0, 0).textContent).toBe("25");
    await gridKey("ArrowUp");
    expect(formulaField().value).toBe("15");
    expect(cell(0, 0).textContent).toBe("15");
    expect(cell(1, 0).textContent).toBe("25");
    expect(cell(4, 4).textContent).toBe("186");
  });

  it("reveals the next row after an inline commit at the viewport edge", async () => {
    await gridKey("ArrowDown", 29);
    await gridKey("F2");
    const editor = cell(29, 0).querySelector<HTMLInputElement>("input")!;
    expect(editor).not.toBeNull();
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(editor, "42");
      editor.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => {
      editor.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    });
    expect(container.textContent).toContain("A31");
    expect(cell(28, 0).textContent).toBe("42");
    expect((cell(29, 0).firstElementChild as HTMLElement).style.background).toBe("rgb(38, 79, 120)");
    expect(container.querySelectorAll("input")).toHaveLength(1);
    await gridKey("ArrowUp");
    expect(formulaField().value).toBe("42");
  });

  it("rejects invalid navigation and viewport requests without changing the workbook", () => {
    const snapshot = host.snapshot();
    for (const payload of [{row:1000,col:0}, {row:-1,col:0}, {row:0,col:26}]) {
      expect(() => host.dispatch("navigate", payload)).toThrow();
    }
    for (const offset of [-1, 1000, 4.8]) {
      expect(() => host.dispatch("scroll", { offset })).toThrow();
    }
    expect(host.snapshot()).toEqual(snapshot);
    expect(host.dispatch("navigate", {row:99,col:25}).props["viewport-offset"]).toBe(70);
  });
});