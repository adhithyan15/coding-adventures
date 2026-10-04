import "@testing-library/jest-dom";

import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import {
  createAuthoringProject,
  openAuthoringSession,
} from "@coding-adventures/forme-authoring-core";
import type {
  AuthoringSession,
  AuthoringStorage,
  StoredAuthoringState,
} from "@coding-adventures/forme-authoring-core";

import { AuthoringEditor } from "../src/editor.js";
import type { EditorPluginRequest } from "../src/plugin.js";

const PROJECT_ID = "018f47a0-9b6c-7def-9234-56789abcdef0";
const FIRST_DOCUMENT_ID = "018f47a0-9b6c-7def-9234-56789abcdef1";
const SECOND_DOCUMENT_ID = "018f47a0-9b6c-7def-9234-56789abcdef2";

class MemoryStorage implements AuthoringStorage {
  state: StoredAuthoringState | null = null;
  writes = 0;
  failNext: unknown = null;

  async load(): Promise<StoredAuthoringState | null> { return this.state; }

  async compareAndSwap(expectedRevision: string | null, bytes: Uint8Array): Promise<{ readonly revision: string }> {
    if (this.failNext !== null) {
      const failure = this.failNext;
      this.failNext = null;
      throw failure;
    }
    if (expectedRevision !== this.state?.revision && !(expectedRevision === null && this.state === null)) {
      throw new Error("conflict");
    }
    this.writes += 1;
    const revision = `r${this.writes}`;
    this.state = { bytes: new Uint8Array(bytes), revision };
    return { revision };
  }
}

async function sessionFixture(storage = new MemoryStorage()): Promise<{ session: AuthoringSession; storage: MemoryStorage }> {
  const session = await openAuthoringSession({
    storage,
    initialProject: createAuthoringProject({ projectId: PROJECT_ID, title: "Demo site" }),
  });
  await session.dispatch({
    type: "create-document",
    activate: true,
    document: {
      id: FIRST_DOCUMENT_ID,
      slug: "welcome",
      title: "Welcome",
      status: "draft",
      body: { type: "document", children: [] },
    },
  });
  return { session, storage };
}

const themes = [
  { id: "forme-classless", label: "Forme Classless" },
  { id: "high-contrast", label: "High contrast" },
];

describe("AuthoringEditor", () => {
  it("edits site and document settings through durable core commands", async () => {
    const { session } = await sessionFixture();
    render(<AuthoringEditor session={session} themeOptions={themes} createDocumentIdentity={() => SECOND_DOCUMENT_ID} />);

    fireEvent.change(screen.getByLabelText("Site title"), { target: { value: "Edited site" } });
    fireEvent.change(screen.getByLabelText("Base URL"), { target: { value: "https://example.com/docs" } });
    fireEvent.change(screen.getByLabelText("Theme"), { target: { value: "high-contrast" } });
    fireEvent.click(screen.getByRole("button", { name: "Save site settings" }));
    await screen.findByText("Site settings saved.");
    expect(session.project.title).toBe("Edited site");
    expect(session.project.site).toEqual({ baseUrl: "https://example.com/docs", themeId: "high-contrast" });

    fireEvent.change(screen.getByLabelText("Document title"), { target: { value: "Start here" } });
    fireEvent.change(screen.getByLabelText("Document slug"), { target: { value: "start-here" } });
    fireEvent.change(screen.getByLabelText("Document status"), { target: { value: "published" } });
    fireEvent.click(screen.getByRole("button", { name: "Save document settings" }));
    await screen.findByText("Document settings saved.");
    expect(session.project.documents[0]).toMatchObject({ title: "Start here", slug: "start-here", status: "published" });
  });

  it("creates, selects, and removes documents with keyboard-reachable controls", async () => {
    const { session } = await sessionFixture();
    render(<AuthoringEditor session={session} themeOptions={themes} createDocumentIdentity={() => SECOND_DOCUMENT_ID} />);

    fireEvent.change(screen.getByLabelText("New document title"), { target: { value: "Second" } });
    fireEvent.change(screen.getByLabelText("New document slug"), { target: { value: "second" } });
    fireEvent.click(screen.getByRole("button", { name: "Create document" }));
    await screen.findByText("Document created.");
    expect(session.project.activeDocumentId).toBe(SECOND_DOCUMENT_ID);
    expect(screen.getByRole("heading", { name: "Editing Second" })).toHaveFocus();

    fireEvent.click(screen.getByRole("button", { name: "Select Welcome" }));
    await screen.findByText("Document selected.");
    expect(session.project.activeDocumentId).toBe(FIRST_DOCUMENT_ID);
    expect(screen.getByRole("heading", { name: "Editing Welcome" })).toHaveFocus();

    fireEvent.click(screen.getByRole("button", { name: "Remove current document" }));
    await screen.findByText("Document removed.");
    expect(session.project.documents.map((document) => document.title)).toEqual(["Second"]);
  });

  it("inserts every required block and supports no-pointer reordering and removal", async () => {
    const { session } = await sessionFixture();
    render(<AuthoringEditor session={session} themeOptions={themes} createDocumentIdentity={() => SECOND_DOCUMENT_ID} />);

    const kinds = [
      ["paragraph", "Paragraph"], ["heading", "Heading"], ["list", "List"],
      ["image", "Image"], ["code_block", "Code block"], ["blockquote", "Blockquote"],
      ["table", "Table"], ["link", "Link"],
    ] as const;
    for (const [value, label] of kinds) {
      fireEvent.change(screen.getByLabelText("Block type"), { target: { value } });
      fireEvent.click(screen.getByRole("button", { name: "Add block" }));
      await screen.findByText(`${label} block added.`);
    }

    expect(session.project.documents[0]!.body.children).toHaveLength(8);
    expect(screen.getByRole("heading", { name: "Block 8: Link" })).toHaveFocus();
    const linkBlock = screen.getByRole("group", { name: "Block 8: Link" });
    fireEvent.click(within(linkBlock).getByRole("button", { name: "Move up" }));
    await screen.findByText("Block moved.");
    expect(session.project.documents[0]!.body.children[6]!.type).toBe("paragraph");
    expect(screen.getByRole("heading", { name: "Block 7: Link" })).toHaveFocus();

    const movedLink = screen.getByRole("group", { name: "Block 7: Link" });
    fireEvent.click(within(movedLink).getByRole("button", { name: "Remove block" }));
    await screen.findByText("Block removed.");
    expect(session.project.documents[0]!.body.children).toHaveLength(7);
  });

  it("edits paragraph, heading, list, image, code, quote, table, and link forms", async () => {
    const { session } = await sessionFixture();
    render(<AuthoringEditor session={session} themeOptions={themes} createDocumentIdentity={() => SECOND_DOCUMENT_ID} />);

    const cases = [
      ["paragraph", "Paragraph"], ["heading", "Heading"], ["list", "List"],
      ["image", "Image"], ["code_block", "Code block"], ["blockquote", "Blockquote"],
      ["table", "Table"], ["link", "Link"],
    ] as const;
    for (const [kind] of cases) {
      fireEvent.change(screen.getByLabelText("Block type"), { target: { value: kind } });
      fireEvent.click(screen.getByRole("button", { name: "Add block" }));
      await screen.findByText(/block added\./);
    }

    fireEvent.change(screen.getByLabelText("Paragraph text"), { target: { value: "A paragraph" } });
    fireEvent.click(screen.getByRole("button", { name: "Save paragraph" }));
    await screen.findByText("Paragraph saved.");

    fireEvent.change(screen.getByLabelText("Heading level"), { target: { value: "3" } });
    fireEvent.change(screen.getByLabelText("Heading text"), { target: { value: "A heading" } });
    fireEvent.click(screen.getByRole("button", { name: "Save heading" }));
    await screen.findByText("Heading saved.");

    fireEvent.change(screen.getByLabelText("List items, one per line"), { target: { value: "One\nTwo" } });
    fireEvent.click(screen.getByLabelText("Ordered list"));
    fireEvent.click(screen.getByRole("button", { name: "Save list" }));
    await screen.findByText("List saved.");

    fireEvent.change(screen.getByLabelText("Image URL"), { target: { value: "/photo.png" } });
    fireEvent.change(screen.getByLabelText("Image alternative text"), { target: { value: "A photo" } });
    fireEvent.click(screen.getByRole("button", { name: "Save image" }));
    await screen.findByText("Image saved.");

    fireEvent.change(screen.getByLabelText("Code language"), { target: { value: "typescript" } });
    fireEvent.change(screen.getByLabelText("Code"), { target: { value: "const answer = 42;" } });
    fireEvent.click(screen.getByRole("button", { name: "Save code block" }));
    await screen.findByText("Code block saved.");

    fireEvent.change(screen.getByLabelText("Quote text"), { target: { value: "Quoted" } });
    fireEvent.click(screen.getByRole("button", { name: "Save blockquote" }));
    await screen.findByText("Blockquote saved.");

    fireEvent.change(screen.getByLabelText("Table cells as tab-separated rows"), { target: { value: "Name\tValue\nAlpha\t1" } });
    fireEvent.click(screen.getByRole("button", { name: "Save table" }));
    await screen.findByText("Table saved.");

    fireEvent.change(screen.getByLabelText("Link text"), { target: { value: "Example" } });
    fireEvent.change(screen.getByLabelText("Link URL"), { target: { value: "https://example.com" } });
    fireEvent.click(screen.getByRole("button", { name: "Save link" }));
    await screen.findByText("Link saved.");

    const blocks = session.project.documents[0]!.body.children;
    expect(blocks[0]).toEqual({ type: "paragraph", children: [{ type: "text", value: "A paragraph" }] });
    expect(blocks[1]).toMatchObject({ type: "heading", level: 3 });
    expect(blocks[2]).toMatchObject({ type: "list", ordered: true, start: 1 });
    expect(blocks[3]).toMatchObject({ type: "paragraph", children: [{ type: "image", destination: "/photo.png", alt: "A photo" }] });
    expect(blocks[4]).toEqual({ type: "code_block", language: "typescript", value: "const answer = 42;\n" });
    expect(blocks[5]).toMatchObject({ type: "blockquote" });
    expect(blocks[6]).toMatchObject({ type: "table", align: [null, null] });
    expect(blocks[7]).toMatchObject({ type: "paragraph", children: [{ type: "link", destination: "https://example.com" }] });
  });

  it("persists undo and redo and announces a bounded failure without advancing state", async () => {
    const { session, storage } = await sessionFixture();
    render(<AuthoringEditor session={session} themeOptions={themes} createDocumentIdentity={() => SECOND_DOCUMENT_ID} />);

    fireEvent.change(screen.getByLabelText("Site title"), { target: { value: "Changed" } });
    fireEvent.click(screen.getByRole("button", { name: "Save site settings" }));
    await screen.findByText("Site settings saved.");
    fireEvent.click(screen.getByRole("button", { name: "Undo" }));
    await screen.findByText("Change undone.");
    expect(session.project.title).toBe("Demo site");
    fireEvent.click(screen.getByRole("button", { name: "Redo" }));
    await screen.findByText("Change redone.");
    expect(session.project.title).toBe("Changed");

    storage.failNext = new Error("/Users/private/token.txt super secret");
    fireEvent.change(screen.getByLabelText("Site title"), { target: { value: "Not saved" } });
    fireEvent.click(screen.getByRole("button", { name: "Save site settings" }));
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("The change could not be saved.");
    expect(alert).not.toHaveTextContent("token");
    expect(session.project.title).toBe("Changed");
  });

  it("renders declarative slots and gives the bridge only a frozen request", async () => {
    const { session } = await sessionFixture();
    let request: EditorPluginRequest | undefined;
    const execute = vi.fn(async (received: EditorPluginRequest) => {
      request = received;
      return { type: "set-active-document" as const, documentId: FIRST_DOCUMENT_ID };
    });
    render(<AuthoringEditor
      session={session}
      themeOptions={themes}
      createDocumentIdentity={() => SECOND_DOCUMENT_ID}
      pluginContributions={[
        { pluginId: "word-count", actionId: "summarize", slot: "document-toolbar", label: "Summarize document" },
        { pluginId: "block-tools", actionId: "inspect", slot: "block-toolbar", label: "Inspect block" },
        { pluginId: "site-tools", actionId: "audit", slot: "site-toolbar", label: "Audit site" },
      ]}
      pluginBridge={{ execute }}
    />);

    fireEvent.click(screen.getByRole("button", { name: "Summarize document" }));
    await screen.findByText("Plugin action saved.");
    expect(execute).toHaveBeenCalledOnce();
    expect(request).toBeDefined();
    expect(Reflect.ownKeys(request!).sort()).toEqual(["actionId", "pluginId", "project", "slot", "target"]);
    expect(Object.isFrozen(request)).toBe(true);
    expect(Object.isFrozen(request!.project)).toBe(true);
    expect(request!.target).toEqual({ documentId: FIRST_DOCUMENT_ID, blockIndex: null });

    fireEvent.change(screen.getByLabelText("Block type"), { target: { value: "paragraph" } });
    fireEvent.click(screen.getByRole("button", { name: "Add block" }));
    await screen.findByText("Paragraph block added.");
    const block = screen.getByRole("group", { name: "Block 1: Paragraph" });
    expect(within(block).getByRole("button", { name: "Inspect block" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Audit site" })).toBeEnabled();
  });

  it("exposes busy and live-region semantics while persistence is pending", async () => {
    const { session } = await sessionFixture();
    let release: (() => void) | undefined;
    const pending = new Promise<void>((resolve) => { release = resolve; });
    const originalDispatch = session.dispatch.bind(session);
    session.dispatch = vi.fn(async (command, signal) => {
      await pending;
      await originalDispatch(command, signal);
    });
    render(<AuthoringEditor session={session} themeOptions={themes} createDocumentIdentity={() => SECOND_DOCUMENT_ID} />);

    fireEvent.click(screen.getByRole("button", { name: "Save site settings" }));
    expect(screen.getByRole("region", { name: "Forme authoring editor" })).toHaveAttribute("aria-busy", "true");
    expect(screen.getByRole("status")).toHaveAttribute("aria-live", "polite");
    release!();
    await waitFor(() => expect(screen.getByRole("region", { name: "Forme authoring editor" })).toHaveAttribute("aria-busy", "false"));
  });
});
