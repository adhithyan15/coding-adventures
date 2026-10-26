/**
 * startup-recovery.test.ts — a saved workspace the engine rejects is never
 * written over.
 *
 * At startup the host loads `workspace/web` into the engine. When the engine
 * rejects it, the host keeps the record under the first free recovery key and
 * starts fresh; when it cannot keep it, it must not save at all, because the
 * rejected record is still at `workspace/web` and the first edit would replace
 * it:
 *
 *   recovery copy written?   after the first edit
 *   ----------------------   -----------------------------------------------
 *   yes (first free key)     web = the new workspace; the copy is untouched
 *   no (all keys taken)      web = still the rejected record; nothing saved
 *
 * These boot the real Web Components host with the real scheduling engine
 * (built by this package's BUILD) over in-memory storage, then make one edit
 * through the host facade the emitted runtime uses.
 */
import { readFile } from "node:fs/promises";
import path from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";

import { MemoryStorage } from "@coding-adventures/indexeddb/src/browser.js";
import {
  MAX_RECOVERY_WORKSPACES,
  makeWorkspaceRecord,
  recoveryWorkspaceKey,
  type WorkspaceRecord,
} from "../src/persistence";
import { bootWebComponentHost } from "../src/webcomponent";

// The test config aliases `src/task-engine.mjs` to `stubs/task-engine.mjs`, a
// stub that refuses to run, so that only tests which mean to construct the real
// engine do. This one does (the recovery path starts from the engine rejecting
// a snapshot), so it mocks the stub back to the real accessor.
vi.mock("./stubs/task-engine.mjs", async () =>
  // @ts-expect-error JavaScript ABI accessor has no source declaration file.
  import("../../../../../../packages/rust/task-wasm/js/task-engine.mjs"),
);

const REJECTED = "not-an-engine-snapshot";

const wasmPath = path.resolve(
  process.cwd(),
  "../../../../../packages/rust/target/wasm32-unknown-unknown/release/task_wasm.wasm",
);

const settle = async () => {
  await new Promise((resolve) => setTimeout(resolve, 0));
  await new Promise((resolve) => setTimeout(resolve, 0));
};

afterEach(() => {
  document.body.replaceChildren();
  delete window.mosaicHost;
});

/** Storage holding a rejected live record, plus `recoveries` kept earlier. */
async function seededStorage(recoveries: number): Promise<MemoryStorage> {
  const storage = new MemoryStorage([{ name: "workspace", keyPath: "id" }]);
  await storage.open();
  await storage.put("workspace", makeWorkspaceRecord(REJECTED, ["t1"], 1, 10));
  for (let index = 0; index < recoveries; index += 1) {
    await storage.put("workspace", {
      ...makeWorkspaceRecord(`kept ${index}`, [], 0, index),
      id: recoveryWorkspaceKey(index),
    });
  }
  return storage;
}

/** Boot the host over `storage`, add one task through it, and let saves land. */
async function bootAndEdit(storage: MemoryStorage): Promise<Record<string, unknown>> {
  const wasm = await readFile(wasmPath);
  document.body.innerHTML = `<p id="startup-status" role="status"></p>`;
  await bootWebComponentHost({
    fetchWasm: async () =>
      wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength) as ArrayBuffer,
    openStorage: async () => ({
      storage,
      durable: true,
      status: "Saved locally on this device",
      location: "Startup recovery acceptance storage",
      warning: "",
    }),
    theme: "light",
    now: () => 1_700_000_000_000,
  });
  expect(
    document.body.dataset.webcomponentState,
    document.getElementById("startup-status")?.textContent ?? "",
  ).toBe("ready");
  const host = window.mosaicHost!;
  await host.handleEvent({
    component: "TaskApp",
    event: { type: "newTaskNameChange", value: "Written after startup" },
  });
  const result = (await host.handleEvent({
    component: "TaskApp",
    event: { type: "addTask" },
  })) as { props: Record<string, unknown> };
  await settle();
  return result.props;
}

const live = (storage: MemoryStorage) =>
  storage.get<WorkspaceRecord>("workspace", "web") as Promise<WorkspaceRecord>;

describe("startup recovery of a rejected workspace", () => {
  it("keeps the rejected record under the first free key, then saves normally", async () => {
    const storage = await seededStorage(1);
    const props = await bootAndEdit(storage);

    expect(JSON.stringify(props)).toContain("workspace/web-corrupt.1");
    expect(
      (await storage.get<WorkspaceRecord>("workspace", "web-corrupt"))!.snapshot,
    ).toBe("kept 0");
    expect(
      (await storage.get<WorkspaceRecord>("workspace", "web-corrupt.1"))!.snapshot,
    ).toBe(REJECTED);
    const saved = await live(storage);
    expect(saved.snapshot).not.toBe(REJECTED);
    expect(saved.snapshot).toContain("Written after startup");
  });

  it("saves nothing over the rejected record when it cannot be kept", async () => {
    const storage = await seededStorage(MAX_RECOVERY_WORKSPACES);
    const props = await bootAndEdit(storage);

    expect(JSON.stringify(props)).toContain("will not save this session");
    expect((await live(storage)).snapshot).toBe(REJECTED);
    expect(await storage.get("workspace", "web-corrupt.10")).toBeUndefined();
  });
});
