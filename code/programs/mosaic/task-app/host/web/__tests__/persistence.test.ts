/**
 * persistence.test.ts — round-trips the host persistence seam.
 *
 * These run under jsdom, where `indexedDB` is absent, so `openWorkspaceStorage`
 * exercises its in-memory fallback path — which is exactly the behaviour we want
 * to guarantee for private-browsing / SSR / test environments. The record shape,
 * ordering, counter, and single-record overwrite semantics are all asserted here;
 * the live IndexedDB path is verified end-to-end in a browser (see CHANGELOG).
 */
import { describe, it, expect } from "vitest";
import {
  loadWorkspace,
  makeWorkspaceRecord,
  openWorkspaceStorage,
  saveWorkspace,
  WORKSPACE_KEY,
} from "../src/persistence";

describe("workspace persistence", () => {
  it("returns undefined on a first visit", async () => {
    const storage = await openWorkspaceStorage();
    expect(await loadWorkspace(storage)).toBeUndefined();
  });

  it("round-trips an engine snapshot plus host state", async () => {
    const storage = await openWorkspaceStorage();
    saveWorkspace(storage, makeWorkspaceRecord('{"id":"project"}', ["t1", "t2"], 2, 123));

    const rec = await loadWorkspace(storage);
    expect(rec).toBeDefined();
    expect(rec!.id).toBe(WORKSPACE_KEY);
    expect(rec!.snapshot).toBe('{"id":"project"}');
    expect(rec!.order).toEqual(["t1", "t2"]);
    expect(rec!.counter).toBe(2);
    expect(rec!.savedAt).toBe(123);
  });

  it("copies the order array so the record can't alias the live list", () => {
    const order = ["a"];
    const rec = makeWorkspaceRecord("{}", order, 1, 0);
    order.push("b");
    expect(rec.order).toEqual(["a"]);
  });

  it("keeps a single workspace record, overwriting on re-save", async () => {
    const storage = await openWorkspaceStorage();
    saveWorkspace(storage, makeWorkspaceRecord("{}", [], 0, 1));
    saveWorkspace(storage, makeWorkspaceRecord('{"v":2}', ["x"], 5, 2));
    await Promise.resolve(); // let the fire-and-forget writes settle

    const all = await storage.getAll(/* store */ "workspace");
    expect(all).toHaveLength(1);
    const rec = await loadWorkspace(storage);
    expect(rec!.snapshot).toBe('{"v":2}');
    expect(rec!.order).toEqual(["x"]);
    expect(rec!.counter).toBe(5);
  });
});
