// The Journal web host against the REAL wasm runtime (J5c-2).
import { readFileSync } from "node:fs";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it } from "vitest";
import { App, hostClock, loadRuntime } from "../src/App";
import { STATE_KEY, UNREADABLE_KEY } from "../src/persistence";

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });

const wasm = readFileSync("public/journal_mosaic_app.wasm");
const load = () => loadRuntime(wasm, false);

/** A fresh in-memory Storage, so tests never share state. */
function memoryStorage(): Storage {
  const data = new Map<string, string>();
  return {
    get length() {
      return data.size;
    },
    clear: () => data.clear(),
    getItem: key => data.get(key) ?? null,
    key: index => [...data.keys()][index] ?? null,
    removeItem: key => void data.delete(key),
    setItem: (key, value) => void data.set(key, String(value)),
  };
}

let container: HTMLDivElement;
let root: Root;
beforeEach(() => {
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
});
afterEach(async () => {
  await act(async () => root.unmount());
  container.remove();
});

async function mount(storage: Storage) {
  await act(async () => root.render(<App load={load} storage={storage} />));
  // The wasm loads asynchronously; let it settle.
  await act(async () => new Promise(resolve => setTimeout(resolve, 0)));
}

function button(label: string): HTMLButtonElement {
  const found = [...container.querySelectorAll("button")].find(b => b.textContent === label);
  if (!found) throw new Error(`no button "${label}"`);
  return found;
}

async function type(field: HTMLInputElement | HTMLTextAreaElement, value: string) {
  const proto = field instanceof HTMLInputElement ? HTMLInputElement.prototype : HTMLTextAreaElement.prototype;
  await act(async () => {
    Object.getOwnPropertyDescriptor(proto, "value")!.set!.call(field, value);
    field.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

async function writeEntry(title: string, body: string) {
  await act(async () => button("New entry").click());
  await type(container.querySelector('input[aria-label="Title"]') as HTMLInputElement, title);
  await type(container.querySelector('textarea[aria-label="Entry"]') as HTMLTextAreaElement, body);
  await act(async () => button("Save").click());
}

it("opens on the empty state", async () => {
  await mount(memoryStorage());
  expect(container.textContent).toContain("No entries yet");
  expect(container.querySelector('input[aria-label="Title"]')).not.toBeNull();
});

it("writes and saves an entry onto the timeline, and stores it", async () => {
  const storage = memoryStorage();
  await mount(storage);
  await writeEntry("First light", "Written in the browser.");
  expect(container.textContent).not.toContain("No entries yet");
  expect(button("First light")).toBeTruthy();
  const stored = JSON.parse(storage.getItem(STATE_KEY)!);
  expect(stored.schema).toBe("journal-mosaic-app/state");
  expect(stored.text).toContain("First light");
});

it("restores the journal on the next visit", async () => {
  const storage = memoryStorage();
  await mount(storage);
  await writeEntry("Kept", "Across a reload.");
  await act(async () => root.unmount());
  root = createRoot(container);
  await mount(storage);
  expect(button("Kept")).toBeTruthy();
});

it("keeps an unreadable journal aside instead of losing it", async () => {
  const storage = memoryStorage();
  storage.setItem(STATE_KEY, '{"schema":"journal-mosaic-app/state","version":1,"text":"not json"}');
  await mount(storage);
  expect(container.textContent).toContain("No entries yet");
  expect(container.querySelector('[role="alert"]')?.textContent).toContain("kept aside");
  expect(storage.getItem(UNREADABLE_KEY)).toContain("not json");
  expect(storage.getItem(STATE_KEY)).toBeNull();
});

it("the clock shim never throws into the runtime", () => {
  expect(hostClock(() => 42)()).toBe(42);
  expect(hostClock(() => {
    throw new Error("no clock");
  })()).toBeNaN();
});
