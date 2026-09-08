import { readFileSync } from "node:fs";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { App } from "../src/app/App";
import { loadMosaicModule } from "../../../../packages/rust/mosaic-app-wasm/js/mosaic-host.mjs";

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
afterEach(() => vi.unstubAllGlobals());

async function mount() {
  const module = await loadMosaicModule(readFileSync("public/visicalc_mosaic_app.wasm"));
  const host = module.create({ protocolVersion: 2, colorScheme: "light" });
  const container = document.createElement("div"); document.body.append(container);
  const root = createRoot(container);
  await act(async () => { root.render(<App load={async () => host} />); });
  return { host, container,
    async click(label: string) {
      const button = [...container.querySelectorAll("button")].find(button => button.textContent === label);
      expect(button).toBeDefined();
      await act(async () => { button!.click(); });
    },
    async edit(value: string, commit = false) {
      const input = container.querySelector<HTMLInputElement>('input[placeholder="Enter a value or formula"]')!;
      await act(async () => {
        Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value);
        input.dispatchEvent(new Event("input", { bubbles: true }));
      });
      if (commit) await act(async () => { input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })); });
    },
    async dispose() { await act(async () => root.unmount()); container.remove(); },
  };
}

function fileEnvironment() {
  vi.stubGlobal("isSecureContext", true);
  vi.stubGlobal("navigator", { userActivation: { isActive: true } });
}

it("saves through generated controls and opens the actual bytes in a fresh Rust instance", async () => {
  fileEnvironment();
  let stored: Uint8Array = new Uint8Array();
  const save = vi.fn(async () => ({ name: "Budget.visicalc", createWritable: async () => ({
    async write(bytes: Uint8Array) { stored = new Uint8Array(bytes); }, async close() {}, async abort() {},
  }) }));
  vi.stubGlobal("showSaveFilePicker", save);
  vi.stubGlobal("showOpenFilePicker", vi.fn(async () => [{ getFile: async () => ({
    name: "Budget.visicalc", size: stored.length, arrayBuffer: async () => stored.slice().buffer,
  }) }]));
  const original = await mount();
  try {
    await original.edit("20");
    await original.click("Save");
    expect(save).not.toHaveBeenCalled();
    expect(original.container.textContent).toContain("Press Enter to apply this edit");
    await original.edit("20", true);
    await original.click("Save");
    expect(save).toHaveBeenCalledTimes(1);
    expect(original.container.textContent).toContain("Saved Budget.visicalc");
    expect(JSON.parse(new TextDecoder().decode(stored)).schema).toBe("visicalc-mosaic-app/state");
  } finally { await original.dispose(); }
  const fresh = await mount();
  try {
    await fresh.click("New workbook");
    await fresh.click("Open");
    expect(fresh.host.update.props.formula).toBe("20");
    expect((fresh.host.update.props["viewport-rows"] as string[][])[4][4]).toBe("174");
    expect(fresh.container.textContent).toContain("Opened Budget.visicalc");
    expect(fresh.host.snapshot()).not.toBeNull();
  } finally { await fresh.dispose(); }
});

it("cancelled, denied and malformed Open retain the edit and allow a later retry", async () => {
  fileEnvironment();
  const app = await mount();
  try {
    await app.edit("still editing");
    const before = app.host.snapshot();
    for (const name of ["AbortError", "NotAllowedError"]) {
      vi.stubGlobal("showOpenFilePicker", vi.fn(async () => { throw new DOMException("denied", name); }));
      await app.click("Open");
      expect(app.host.update.props.formula).toBe("still editing");
      expect(app.host.update.props.editing).toBe(true);
      expect(app.host.snapshot()).toEqual(before);
      expect(app.container.textContent).toContain(name === "AbortError" ? "cancelled" : "Could not open");
    }
    const bytes = new TextEncoder().encode('{"schema":"unrelated","version":1,"bytes":[]}');
    vi.stubGlobal("showOpenFilePicker", vi.fn(async () => [{ getFile: async () => ({
      name: "bad.visicalc", size: bytes.length, arrayBuffer: async () => bytes.buffer,
    }) }]));
    await app.click("Open");
    expect(app.container.textContent).toContain("Choose a VisiCalc workbook; your work is unchanged");
    expect(app.host.update.props.formula).toBe("still editing");
    expect(app.host.snapshot()).toEqual(before);
    await app.edit("23", true);
    expect(app.host.update.props.formula).toBe("23");
  } finally { await app.dispose(); }
});

it("reports unsupported browser file dialogs through the shared status and announcement", async () => {
  vi.stubGlobal("isSecureContext", false);
  const app = await mount();
  try {
    await app.click("Open");
    expect(app.container.textContent).toContain("File dialogs are unavailable");
    expect(app.container.querySelector('[aria-live="polite"]')!.textContent).toContain("Could not open");
    expect(app.host.snapshot()).not.toBeNull();
  } finally { await app.dispose(); }
});
