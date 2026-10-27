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

async function mount(storage: Storage, environment?: typeof globalThis) {
  await act(async () => root.render(<App load={load} storage={storage} environment={environment} />));
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

/**
 * A timeline entry's Open button. Each row is a mosaic-pkg-card Card with an
 * "Open" button beside it, named "Open {title}" (the app's open-label row
 * field), so a row is found by its accessible name rather than its text.
 */
function entry(title: string): HTMLButtonElement {
  const found = container.querySelector(`button[aria-label="Open ${title}"]`) as HTMLButtonElement | null;
  if (!found) throw new Error(`no timeline entry "${title}"`);
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
  expect(entry("First light")).toBeTruthy();
  const stored = JSON.parse(storage.getItem(STATE_KEY)!);
  expect(stored.schema).toBe("journal-mosaic-app/state");
  expect(stored.text).toContain("First light");
});

it("draws each timeline entry as a Card with an Open button beside it", async () => {
  await mount(memoryStorage());
  await writeEntry("First light", "Walked to the lighthouse.");
  await type(container.querySelector('input[aria-label="Tags"]') as HTMLInputElement, "travel");
  await act(async () => button("Save").click());
  await act(async () => button("Star").click());

  const open = entry("First light");
  // One button per row, labelled "Open" and named after the entry; it is the
  // open entry, so it reports itself selected.
  expect(open.textContent).toBe("Open");
  expect(open.getAttribute("aria-pressed")).toBe("true");
  // The Card holds the title, the excerpt and the meta (tags); the badge sits
  // beside the button; the day is a heading above the card.
  const row = open.parentElement!.parentElement!;
  expect(row.textContent).toContain("First light");
  expect(row.textContent).toContain("Walked to the lighthouse.");
  expect(row.textContent).toContain("#travel");
  expect(open.parentElement!.textContent).toBe("Open★");
  expect([...container.querySelectorAll("h2")].some(h => /\d{4}$/.test(h.textContent ?? ""))).toBe(true);
  // The card is beside the button, never inside it (EM-8).
  expect(open.querySelector("div")).toBeNull();

  await act(async () => button("New entry").click());
  expect(entry("First light").getAttribute("aria-pressed")).toBe("false");
  await act(async () => entry("First light").click());
  expect((container.querySelector('input[aria-label="Title"]') as HTMLInputElement).value).toBe("First light");
});

it("restores the journal on the next visit", async () => {
  const storage = memoryStorage();
  await mount(storage);
  await writeEntry("Kept", "Across a reload.");
  await act(async () => root.unmount());
  root = createRoot(container);
  await mount(storage);
  expect(entry("Kept")).toBeTruthy();
});

it("searches the journal, says when nothing matches, and clears back to the timeline", async () => {
  await mount(memoryStorage());
  await writeEntry("Harbour walk", "Fog over the water.");
  await writeEntry("Groceries", "Bread, then the harbour.");
  await writeEntry("Unrelated", "Nothing here.");
  const search = container.querySelector('input[aria-label="Search entries"]') as HTMLInputElement;
  expect(search).not.toBeNull();

  await type(search, "harbour");
  expect(entry("Harbour walk")).toBeTruthy();
  expect(entry("Groceries")).toBeTruthy();
  expect(() => entry("Unrelated")).toThrow();
  // Ranked results carry no day headings; each hit shows its day instead.
  expect(container.textContent).toContain("Fog over the water.");

  await type(search, "harbour sunshine");
  expect(container.textContent).toContain("No entries match");
  expect(container.textContent).not.toContain("No entries yet");

  await act(async () => button("Clear").click());
  expect(entry("Unrelated")).toBeTruthy();
  expect(() => button("Clear")).toThrow();
});

it("stars an entry and filters the timeline to starred entries", async () => {
  await mount(memoryStorage());
  await writeEntry("Plain day", "Nothing special.");
  expect(container.textContent).not.toContain("No starred entries");
  await act(async () => button("Starred only").click());
  expect(container.textContent).toContain("No starred entries");
  expect(() => entry("Plain day")).toThrow();

  await act(async () => button("Starred only").click());
  await writeEntry("Good day", "Worth keeping.");
  await act(async () => button("Star").click());
  expect(button("Unstar")).toBeTruthy();
  expect(container.textContent).toContain("★");

  await act(async () => button("Starred only").click());
  expect(entry("Good day")).toBeTruthy();
  expect(() => entry("Plain day")).toThrow();
});

it("tags entries and filters the timeline by a tag", async () => {
  await mount(memoryStorage());
  const tagsField = () => container.querySelector('input[aria-label="Tags"]') as HTMLInputElement;
  const writeTagged = async (title: string, tags: string) => {
    await act(async () => button("New entry").click());
    await type(container.querySelector('input[aria-label="Title"]') as HTMLInputElement, title);
    await type(tagsField(), tags);
    await act(async () => button("Save").click());
  };
  await writeTagged("Beach", "travel");
  await writeTagged("Office", "work");
  expect(tagsField().value).toBe("work");
  expect(container.textContent).toContain("#travel");

  await act(async () => button("#travel (1)").click());
  expect(entry("Beach")).toBeTruthy();
  expect(() => entry("Office")).toThrow();

  await act(async () => button("#travel (1)").click());
  expect(entry("Office")).toBeTruthy();
});

it("adds a journal, files new entries there and switches between journals", async () => {
  await mount(memoryStorage());
  const field = (label: string) => container.querySelector(`input[aria-label="${label}"]`) as HTMLInputElement;
  const write = async (title: string) => {
    await act(async () => button("New entry").click());
    await type(field("Title"), title);
    await act(async () => button("Save").click());
  };
  await write("At home");
  await type(field("New journal name"), "personal");
  await act(async () => button("Add").click());
  expect(container.textContent).toContain("A journal named \u201cpersonal\u201d already exists.");

  await type(field("New journal name"), "Work");
  expect(container.textContent).not.toContain("already exists");
  await act(async () => button("Add").click());
  expect(field("New journal name").value).toBe("");
  expect(() => entry("At home")).toThrow();
  expect(container.textContent).toContain("No entries in this journal"); // J4h
  await write("Standup");
  expect(entry("Standup")).toBeTruthy();
  expect(container.textContent).not.toContain("No entries in this journal");

  await act(async () => button("Personal").click());
  expect(entry("At home")).toBeTruthy();
  expect(() => entry("Standup")).toThrow();
  await act(async () => button("All journals").click());
  expect(entry("At home")).toBeTruthy();
  expect(entry("Standup")).toBeTruthy();
});

it("moves an entry to another journal from the editor's picker", async () => {
  await mount(memoryStorage());
  const field = (label: string) => container.querySelector(`input[aria-label="${label}"]`) as HTMLInputElement;
  // The pane's switcher comes first in the DOM, so the editor's option is the last.
  const pickerOption = (label: string) => {
    const found = [...container.querySelectorAll("button")].filter(b => b.textContent === label);
    if (found.length < 2) throw new Error(`no picker option "${label}"`);
    return found[found.length - 1];
  };
  await act(async () => button("New entry").click());
  await type(field("Title"), "At home");
  await act(async () => button("Save").click());
  const personal = () => [...container.querySelectorAll("button")].filter(b => b.textContent === "Personal");
  expect(personal()).toHaveLength(1); // one journal: the pane's switcher only, no picker

  await type(field("New journal name"), "Work");
  await act(async () => button("Add").click());
  await act(async () => button("All journals").click());
  expect(personal()).toHaveLength(2);
  await act(async () => entry("At home").click());
  await act(async () => pickerOption("Work").click());
  await act(async () => button("Save").click());

  await act(async () => pickerOption("Personal").click());
  await act(async () => button("Cancel").click());
  await act(async () => button("Personal").click());
  expect(() => entry("At home")).toThrow();
  await act(async () => button("Work").click());
  expect(entry("At home")).toBeTruthy();
});

it("renames a journal, then deletes it and keeps its entries in Personal", async () => {
  await mount(memoryStorage());
  const field = (label: string) => container.querySelector(`input[aria-label="${label}"]`) as HTMLInputElement;
  await type(field("New journal name"), "Work");
  await act(async () => button("Add").click());
  await act(async () => button("New entry").click());
  await type(field("Title"), "Standup");
  await act(async () => button("Save").click());
  expect(() => button("Delete journal")).not.toThrow();

  await type(field("New journal name"), "Office");
  await act(async () => button("Rename").click());
  expect(() => button("Work")).toThrow();
  expect(button("Office")).toBeTruthy();

  await act(async () => button("Delete journal").click());
  expect(() => button("Office")).toThrow();
  expect(entry("Standup")).toBeTruthy(); // moved to Personal, shown under All journals
  await act(async () => button("Personal").click());
  expect(entry("Standup")).toBeTruthy();
  expect(() => button("Delete journal")).toThrow(); // Personal cannot be deleted
});

it("says why a bad date cannot be saved, then files the entry on the day typed", async () => {
  await mount(memoryStorage());
  const field = (label: string) => container.querySelector(`input[aria-label="${label}"]`) as HTMLInputElement;
  await act(async () => button("New entry").click());
  await type(field("Title"), "Back-dated");
  await type(field("Date"), "2026-02-30");
  await act(async () => button("Save").click());
  expect(container.textContent).toContain("Use a real date in YYYY-MM-DD format.");
  expect(container.textContent).toContain("No entries yet");

  await type(field("Date"), "2026-09-01");
  expect(container.textContent).not.toContain("Use a real date");
  await act(async () => button("Save").click());
  expect(container.textContent).toContain("Tuesday, 1 September 2026");
  expect(field("Date").value).toBe("2026-09-01");
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

// J6a: Export saves the whole journal through the standard files.save effect,
// answered by the browser platform library (mosaic-file-effects.mjs).
it("exports the journal to a file the person chooses, and says where", async () => {
  const written: Uint8Array[] = [];
  const picked: { suggestedName?: string; types?: unknown }[] = [];
  const environment = {
    ...globalThis,
    isSecureContext: true,
    navigator: { userActivation: { isActive: true } },
    showSaveFilePicker: async (options: { suggestedName?: string; types?: unknown }) => {
      picked.push(options);
      return {
        name: options.suggestedName,
        createWritable: async () => ({
          write: async (bytes: Uint8Array) => void written.push(bytes),
          close: async () => {},
          abort: async () => {},
        }),
      };
    },
  } as unknown as typeof globalThis;
  const storage = memoryStorage();
  await mount(storage, environment);
  await writeEntry("Exported", "Kept in a file.");
  await act(async () => button("Export").click());
  for (let waited = 0; !container.textContent?.includes("Exported to"); waited += 10) {
    if (waited > 2000) throw new Error("export never finished");
    await act(async () => new Promise(resolve => setTimeout(resolve, 10)));
  }
  expect(picked[0].suggestedName).toMatch(/^journal-\d{4}-\d{2}-\d{2}\.json$/);
  expect(picked[0].types).toEqual([{ accept: { "application/json": [".json"] } }]);
  const file = JSON.parse(new TextDecoder().decode(written[0]));
  expect(file.format).toBe("coding-adventures-journal");
  expect(file.schema).toBe("journal-mosaic-app/state");
  expect(JSON.stringify(file.state)).toContain("Exported");
  expect(container.textContent).toContain(`Exported to ${picked[0].suggestedName}`);
  // Saved again once the export was answered, with the same journal.
  expect(storage.getItem(STATE_KEY)).toContain("Exported");
});
