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
  preserveRejectedWorkspace,
  RECOVERY_WORKSPACE_KEY,
  saveWorkspace,
  WORKSPACE_KEY,
} from "../src/persistence";

describe("workspace persistence", () => {
  it("returns undefined on a first visit", async () => {
    const { storage } = await openWorkspaceStorage();
    expect(await loadWorkspace(storage)).toBeUndefined();
  });

  it("describes the volatile fallback in plain language", async () => {
    const session = await openWorkspaceStorage();
    expect(session.durable).toBe(false);
    expect(session.status).toBe("Temporary session only");
    expect(session.location).toContain("closing or reloading removes these changes");
    expect(session.warning).toContain("Durable local storage is unavailable");
  });

  it("round-trips an engine snapshot plus host state", async () => {
    const { storage } = await openWorkspaceStorage();
    saveWorkspace(storage, makeWorkspaceRecord('{"id":"project"}', ["t1", "t2"], 2, 123));

    const rec = await loadWorkspace(storage);
    expect(rec).toBeDefined();
    expect(rec!.id).toBe(WORKSPACE_KEY);
    expect(rec!.snapshot).toBe('{"id":"project"}');
    expect(rec!.order).toEqual(["t1", "t2"]);
    expect(rec!.counter).toBe(2);
    expect(rec!.savedAt).toBe(123);
  });

  it("round-trips the active project so a reload returns you to it", async () => {
    // The engine deliberately keeps this cursor out of its snapshot (two hosts on the
    // same data may sit on different projects), which makes remembering it the host's
    // job. Without it, a reload silently drops you back on the first project and your
    // tasks look like they vanished.
    const { storage } = await openWorkspaceStorage();
    saveWorkspace(storage, makeWorkspaceRecord("{}", [], 0, 1, "p2"));
    expect((await loadWorkspace(storage))!.activeProject).toBe("p2");
  });

  it("omits the active project when there isn't one, so old records still load", () => {
    // Records written before projects existed have no such field; the key must be
    // absent rather than `undefined` so the stored shape stays clean.
    const rec = makeWorkspaceRecord("{}", [], 0, 1);
    expect("activeProject" in rec).toBe(false);
  });

  it("copies the order array so the record can't alias the live list", () => {
    const order = ["a"];
    const rec = makeWorkspaceRecord("{}", order, 1, 0);
    order.push("b");
    expect(rec.order).toEqual(["a"]);
  });

  it("preserves a rejected record under the fixed recovery key", async () => {
    const { storage } = await openWorkspaceStorage();
    const rejected = makeWorkspaceRecord("not-an-engine-snapshot", ["t1"], 1, 10);
    await preserveRejectedWorkspace(storage, rejected);

    const recovery = await storage.get<typeof rejected>("workspace", RECOVERY_WORKSPACE_KEY);
    expect(recovery).toMatchObject({
      id: RECOVERY_WORKSPACE_KEY,
      snapshot: "not-an-engine-snapshot",
      order: ["t1"],
      counter: 1,
      savedAt: 10,
    });
  });

  it("keeps a single workspace record, overwriting on re-save", async () => {
    const { storage } = await openWorkspaceStorage();
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

  it("reports a failed background save instead of rejecting silently", async () => {
    const warning = await new Promise<string>((resolve) => {
      saveWorkspace(
        { put: () => Promise.reject(new Error("quota exhausted")) } as any,
        makeWorkspaceRecord("{}", [], 0, 1),
        resolve,
      );
    });
    expect(warning).toBe("Could not save changes to local storage: quota exhausted");
  });
});
