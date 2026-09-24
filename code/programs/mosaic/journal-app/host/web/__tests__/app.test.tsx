// The Journal web host against the REAL wasm runtime (J5c-2).
import { readFileSync } from "node:fs";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it } from "vitest";
import { App, browserUtcOffsetMinutes, hostClock, loadRuntime } from "../src/App";
import { keptAside, setAside, STATE_KEY } from "../src/persistence";

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
  // The wasm compiles and loads asynchronously (slowest the first time):
  // wait until the app has started, not for a fixed number of ticks.
  for (let waited = 0; container.textContent?.includes("Opening your journal"); waited += 10) {
    if (waited > 5000) throw new Error("Journal did not start");
    await act(async () => new Promise(resolve => setTimeout(resolve, 10)));
  }
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

it("searches the journal, says when nothing matches, and clears back to the timeline", async () => {
  await mount(memoryStorage());
  await writeEntry("Harbour walk", "Fog over the water.");
  await writeEntry("Groceries", "Bread, then the harbour.");
  await writeEntry("Unrelated", "Nothing here.");
  const search = container.querySelector('input[aria-label="Search entries"]') as HTMLInputElement;
  expect(search).not.toBeNull();

  await type(search, "harbour");
  expect(button("Harbour walk")).toBeTruthy();
  expect(button("Groceries")).toBeTruthy();
  expect(() => button("Unrelated")).toThrow();
  // Ranked results carry no day headings; each hit shows its day instead.
  expect(container.textContent).toContain("Fog over the water.");

  await type(search, "harbour sunshine");
  expect(container.textContent).toContain("No entries match");
  expect(container.textContent).not.toContain("No entries yet");

  await act(async () => button("Clear").click());
  expect(button("Unrelated")).toBeTruthy();
  expect(() => button("Clear")).toThrow();
});

it("stars an entry and filters the timeline to starred entries", async () => {
  await mount(memoryStorage());
  await writeEntry("Plain day", "Nothing special.");
  expect(container.textContent).not.toContain("No starred entries");
  await act(async () => button("Starred only").click());
  expect(container.textContent).toContain("No starred entries");
  expect(() => button("Plain day")).toThrow();

  await act(async () => button("Starred only").click());
  await writeEntry("Good day", "Worth keeping.");
  await act(async () => button("Star").click());
  expect(button("Unstar")).toBeTruthy();
  expect(container.textContent).toContain("★");

  await act(async () => button("Starred only").click());
  expect(button("Good day")).toBeTruthy();
  expect(() => button("Plain day")).toThrow();
});

it("keeps an unreadable journal aside instead of losing it", async () => {
  const storage = memoryStorage();
  storage.setItem(STATE_KEY, '{"schema":"journal-mosaic-app/state","version":1,"text":"not json"}');
  await mount(storage);
  expect(container.textContent).toContain("No entries yet");
  expect(container.querySelector('[role="alert"]')?.textContent).toContain("kept aside");
  expect(keptAside(storage)).toEqual([expect.stringContaining("not json")]);
  expect(storage.getItem(STATE_KEY)).toBeNull();
});

it("never overwrites a journal kept aside earlier", () => {
  const storage = memoryStorage();
  storage.setItem(STATE_KEY, "first");
  setAside(storage, "first", 1);
  storage.setItem(STATE_KEY, "second");
  setAside(storage, "second", 1); // the same instant: still its own key
  setAside(storage, "second", 2); // the same value: not kept twice
  expect(keptAside(storage)).toEqual(["first", "second"]);
});

it("stops saving once another tab changes the journal", async () => {
  const storage = memoryStorage();
  await mount(storage);
  await writeEntry("Here", "Tab A.");
  // Another tab writes a newer journal.
  storage.setItem(STATE_KEY, "newer journal from tab B");
  await act(async () => window.dispatchEvent(new StorageEvent("storage", { key: STATE_KEY })));
  expect(container.querySelector('[role="alert"]')?.textContent).toContain("another tab");
  await act(async () => button("New entry").click());
  expect(storage.getItem(STATE_KEY)).toBe("newer journal from tab B");
});

it("the clock shim never throws into the runtime", () => {
  expect(hostClock(() => 42)()).toBe(42);
  expect(hostClock(() => {
    throw new Error("no clock");
  })()).toBeNaN();
});

it("files an entry under the browser's local day, not UTC's", async () => {
  // 01:00 UTC on Thursday 24 September 2026 is still Wednesday evening in
  // New York (UTC-5).
  const now = () => Date.UTC(2026, 8, 24, 1);
  const newYork = () => loadRuntime(wasm, false, { now, utcOffsetMinutes: -300 });
  await act(async () => root.render(<App load={newYork} storage={memoryStorage()} />));
  for (let waited = 0; container.textContent?.includes("Opening your journal"); waited += 10) {
    if (waited > 5000) throw new Error("Journal did not start");
    await act(async () => new Promise(resolve => setTimeout(resolve, 10)));
  }
  await writeEntry("Evening", "Local day.");
  expect(container.textContent).toContain("Wednesday, 23 September 2026");
});

it("recalls last year's entry on this day, and opens it", async () => {
  const storage = memoryStorage();
  const at = (ms: number) => () => loadRuntime(wasm, false, { now: () => ms, utcOffsetMinutes: 0 });
  const mountAt = async (ms: number) => {
    await act(async () => root.render(<App load={at(ms)} storage={storage} />));
    for (let waited = 0; container.textContent?.includes("Opening your journal"); waited += 10) {
      if (waited > 5000) throw new Error("Journal did not start");
      await act(async () => new Promise(resolve => setTimeout(resolve, 10)));
    }
  };
  // Written on 24 September 2025...
  await mountAt(Date.UTC(2025, 8, 24, 12));
  await writeEntry("Harbour at dawn", "A year ago today.");
  expect(container.textContent).not.toContain("On this day");
  await act(async () => root.unmount());
  root = createRoot(container);
  // ...recalled on 24 September 2026.
  await mountAt(Date.UTC(2026, 8, 24, 12));
  expect(container.textContent).toContain("On this day");
  expect(container.textContent).toContain("1 year ago · 24 Sep 2025");
  await act(async () => button("New entry").click());
  // The entry is in both lists; the first button is the recall's.
  const recalled = [...container.querySelectorAll("button")].find(b => b.textContent === "Harbour at dawn")!;
  await act(async () => recalled.click());
  expect((container.querySelector('input[aria-label="Title"]') as HTMLInputElement).value).toBe("Harbour at dawn");
});

it("reads the browser's offset east of UTC, and leaves out an implausible one", () => {
  const at = (minutesWest: number) => ({ getTimezoneOffset: () => minutesWest }) as unknown as Date;
  expect(browserUtcOffsetMinutes(at(300))).toBe(-300); // New York in winter
  expect(browserUtcOffsetMinutes(at(-330))).toBe(330); // India
  expect(browserUtcOffsetMinutes(at(0))).toBe(0);
  expect(browserUtcOffsetMinutes(at(900))).toBeUndefined();
  expect(browserUtcOffsetMinutes(at(Number.NaN))).toBeUndefined();
});
