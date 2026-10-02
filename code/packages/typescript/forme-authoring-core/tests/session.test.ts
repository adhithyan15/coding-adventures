import { describe, expect, it } from "vitest";

import {
  AuthoringError,
  createAuthoringProject,
  openAuthoringSession,
  type AuthoringProject,
  type AuthoringStorage,
  type StoredAuthoringState,
} from "../src/index.js";

const PROJECT_ID = "01952c0d-7e63-7000-8000-000000000001";
const DOC_A = "01952c0d-7e63-7000-8000-000000000002";
const DOC_B = "01952c0d-7e63-7000-8000-000000000003";

class MemoryStorage implements AuthoringStorage {
  bytes: Uint8Array | null = null;
  revision: string | null = null;
  writes = 0;
  failNext: Error | null = null;

  async load(signal?: AbortSignal): Promise<StoredAuthoringState | null> {
    if (signal?.aborted) throw signal.reason;
    return this.bytes === null || this.revision === null
      ? null
      : { bytes: this.bytes.slice(), revision: this.revision };
  }

  async compareAndSwap(
    expectedRevision: string | null,
    bytes: Uint8Array,
    signal?: AbortSignal,
  ): Promise<{ readonly revision: string }> {
    if (signal?.aborted) throw signal.reason;
    if (this.failNext !== null) {
      const error = this.failNext;
      this.failNext = null;
      throw error;
    }
    if (expectedRevision !== this.revision) {
      throw new AuthoringError("STORAGE_CONFLICT", "The authoring project changed in another session.");
    }
    this.writes += 1;
    this.bytes = bytes.slice();
    this.revision = `revision-${this.writes}`;
    return { revision: this.revision };
  }
}

function initial(): AuthoringProject {
  return createAuthoringProject({ projectId: PROJECT_ID, title: "My site" });
}

function document(id = DOC_A, slug = "first") {
  return {
    id,
    slug,
    title: "First post",
    status: "draft" as const,
    body: { type: "document" as const, children: [] },
  };
}

describe("durable authoring sessions", () => {
  it("atomically creates and reopens a missing store", async () => {
    const storage = new MemoryStorage();
    const first = await openAuthoringSession({ storage, initialProject: initial() });

    expect(first.project).toEqual(initial());
    expect(first.canUndo).toBe(false);
    expect(first.canRedo).toBe(false);
    expect(storage.writes).toBe(1);

    const reopened = await openAuthoringSession({ storage });
    expect(reopened.project).toEqual(first.project);
    expect(reopened.storageRevision).toBe("revision-1");
  });

  it("requires an explicit initial project for a missing store", async () => {
    await expect(openAuthoringSession({ storage: new MemoryStorage() })).rejects.toMatchObject({
      code: "MISSING_INITIAL_PROJECT",
    });
  });

  it("persists semantic document and site commands", async () => {
    const storage = new MemoryStorage();
    const session = await openAuthoringSession({ storage, initialProject: initial() });

    await session.dispatch({ type: "create-document", document: document(), activate: true });
    await session.dispatch({ type: "update-document-metadata", documentId: DOC_A, title: "Renamed", slug: "renamed", status: "published" });
    await session.dispatch({ type: "replace-document-body", documentId: DOC_A, body: {
      type: "document",
      children: [{ type: "paragraph", children: [{ type: "text", value: "Saved" }] }],
    } });
    await session.dispatch({ type: "configure-site", title: "Configured", baseUrl: "https://example.com", themeId: "forme-classless" });
    await session.dispatch({ type: "set-active-document", documentId: null });

    expect(session.project.title).toBe("Configured");
    expect(session.project.documents[0]).toMatchObject({ title: "Renamed", slug: "renamed", status: "published" });
    expect(session.project.documents[0]!.body.children).toHaveLength(1);
    expect(session.project.activeDocumentId).toBeNull();
    expect(storage.writes).toBe(6);
  });

  it("persists undo and redo across restarts", async () => {
    const storage = new MemoryStorage();
    const session = await openAuthoringSession({ storage, initialProject: initial() });
    await session.dispatch({ type: "create-document", document: document(), activate: true });
    await session.dispatch({ type: "configure-site", title: "Changed", baseUrl: null, themeId: "forme-classless" });
    await session.undo();

    expect(session.project.title).toBe("My site");
    expect(session.canUndo).toBe(true);
    expect(session.canRedo).toBe(true);

    const reopened = await openAuthoringSession({ storage });
    expect(reopened.project.title).toBe("My site");
    expect(reopened.canUndo).toBe(true);
    expect(reopened.canRedo).toBe(true);
    await reopened.redo();
    expect(reopened.project.title).toBe("Changed");
  });

  it("clears redo after a new command and persists document removal", async () => {
    const storage = new MemoryStorage();
    const session = await openAuthoringSession({ storage, initialProject: initial() });
    await session.dispatch({ type: "create-document", document: document(), activate: true });
    await session.undo();
    await session.dispatch({ type: "create-document", document: document(DOC_B, "second"), activate: true });

    expect(session.canRedo).toBe(false);
    await session.dispatch({ type: "remove-document", documentId: DOC_B });
    expect(session.project.documents).toEqual([]);
    expect(session.project.activeDocumentId).toBeNull();
  });

  it("bounds retained history while preserving the current snapshot", async () => {
    const storage = new MemoryStorage();
    const session = await openAuthoringSession({ storage, initialProject: initial(), historyLimit: 3 });
    for (const title of ["One", "Two", "Three", "Four"]) {
      await session.dispatch({ type: "configure-site", title, baseUrl: null, themeId: "forme-classless" });
    }

    expect(session.project.title).toBe("Four");
    await session.undo();
    await session.undo();
    expect(session.project.title).toBe("Two");
    await expect(session.undo()).rejects.toMatchObject({ code: "NO_UNDO" });
  });

  it("serializes concurrent commands instead of losing updates", async () => {
    const storage = new MemoryStorage();
    const session = await openAuthoringSession({ storage, initialProject: initial() });

    await Promise.all([
      session.dispatch({ type: "create-document", document: document(DOC_A, "first"), activate: false }),
      session.dispatch({ type: "create-document", document: document(DOC_B, "second"), activate: true }),
    ]);

    expect(session.project.documents.map((item) => item.slug)).toEqual(["first", "second"]);
    expect(session.project.activeDocumentId).toBe(DOC_B);
  });

  it("leaves state unchanged on validation, write, conflict, and cancellation failures", async () => {
    const storage = new MemoryStorage();
    const session = await openAuthoringSession({ storage, initialProject: initial() });
    const before = session.project;

    await expect(session.dispatch({ type: "create-document", document: document(DOC_A, "Bad Slug"), activate: true })).rejects.toMatchObject({ code: "INVALID_PROJECT" });
    expect(session.project).toBe(before);

    storage.failNext = new Error("disk full");
    await expect(session.dispatch({ type: "configure-site", title: "Lost", baseUrl: null, themeId: "forme-classless" })).rejects.toThrow("disk full");
    expect(session.project).toBe(before);

    storage.revision = "external-change";
    await expect(session.dispatch({ type: "configure-site", title: "Conflict", baseUrl: null, themeId: "forme-classless" })).rejects.toMatchObject({ code: "STORAGE_CONFLICT" });
    expect(session.project).toBe(before);

    const controller = new AbortController();
    controller.abort(new Error("stop"));
    await expect(session.dispatch({ type: "configure-site", title: "Cancelled", baseUrl: null, themeId: "forme-classless" }, controller.signal)).rejects.toThrow("stop");
    expect(session.project).toBe(before);
  });

  it("fails closed on malformed, oversized, unsupported, or non-canonical stored state", async () => {
    const malformedCases = [
      new TextEncoder().encode("not json"),
      new TextEncoder().encode('{"schemaVersion":2}'),
      new TextEncoder().encode('{"cursor":0,"history":[],"schemaVersion":1}'),
      new TextEncoder().encode('{"cursor":1,"history":[{}],"schemaVersion":1}'),
      new Uint8Array(8 * 1024 * 1024 + 1),
    ];

    for (const bytes of malformedCases) {
      const storage = new MemoryStorage();
      storage.bytes = bytes;
      storage.revision = "stored";
      await expect(openAuthoringSession({ storage })).rejects.toBeInstanceOf(AuthoringError);
      expect(storage.writes).toBe(0);
    }
  });

  it("rejects unknown commands, missing documents, duplicate entries, invalid history limits, and empty undo/redo", async () => {
    const storage = new MemoryStorage();
    const session = await openAuthoringSession({ storage, initialProject: initial() });

    await expect(session.undo()).rejects.toMatchObject({ code: "NO_UNDO" });
    await expect(session.redo()).rejects.toMatchObject({ code: "NO_REDO" });
    await expect(session.dispatch({ type: "remove-document", documentId: DOC_A })).rejects.toMatchObject({ code: "DOCUMENT_NOT_FOUND" });
    await expect(session.dispatch({ type: "mystery" } as never)).rejects.toMatchObject({ code: "INVALID_COMMAND" });
    await session.dispatch({ type: "create-document", document: document(), activate: false });
    await expect(session.dispatch({ type: "create-document", document: document(), activate: false })).rejects.toMatchObject({ code: "INVALID_PROJECT" });
    await expect(openAuthoringSession({ storage: new MemoryStorage(), initialProject: initial(), historyLimit: 0 })).rejects.toMatchObject({ code: "INVALID_LIMIT" });
  });
});
