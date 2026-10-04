import "@testing-library/jest-dom";

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import {
  createAuthoringProject,
  openAuthoringSession,
  type AuthoringSession,
  type AuthoringStorage,
  type StoredAuthoringState,
} from "@coding-adventures/forme-authoring-core";
import type {
  AuthoringPreviewCoordinator,
  AuthoringPreviewState,
} from "@coding-adventures/forme-authoring-preview";
import type {
  AuthoringPublisher,
  AuthoringPublishState,
} from "@coding-adventures/forme-authoring-publish";

import {
  AuthoringShell,
  type AuthoringShellHost,
  type AuthoringShellWorkspace,
} from "../src/index.js";

const PROJECT_ID = "018f47a0-9b6c-7def-9234-56789abcdef0";
const DOCUMENT_ID = "018f47a0-9b6c-7def-9234-56789abcdef1";
const NEXT_DOCUMENT_ID = "018f47a0-9b6c-7def-9234-56789abcdef2";

class MemoryStorage implements AuthoringStorage {
  state: StoredAuthoringState | null = null;
  writes = 0;
  async load(): Promise<StoredAuthoringState | null> { return this.state; }
  async compareAndSwap(expected: string | null, bytes: Uint8Array): Promise<{ readonly revision: string }> {
    if (expected !== this.state?.revision && !(expected === null && this.state === null)) throw new Error("conflict");
    const revision = `r${++this.writes}`;
    this.state = { bytes: bytes.slice(), revision };
    return { revision };
  }
}

async function sessionFixture(): Promise<AuthoringSession> {
  const session = await openAuthoringSession({
    storage: new MemoryStorage(),
    initialProject: createAuthoringProject({ projectId: PROJECT_ID, title: "Demo site" }),
  });
  await session.dispatch({
    type: "create-document",
    activate: true,
    document: {
      id: DOCUMENT_ID,
      slug: "welcome",
      title: "Welcome",
      status: "draft",
      body: { type: "document", children: [] },
    },
  });
  return session;
}

function state(): AuthoringPreviewState {
  return { phase: "idle", activeRevision: null, lastGoodRevision: null, lastGoodBuildId: null, diagnostics: [] };
}

function publishState(): AuthoringPublishState {
  return { phase: "idle", revision: null, manifestSha256: null, diagnostics: [] };
}

async function workspaceFixture(overrides: Partial<AuthoringShellWorkspace> = {}) {
  const session = await sessionFixture();
  const preview: AuthoringPreviewCoordinator = {
    state: state(),
    request: vi.fn(async (value) => ({ outcome: "ready" as const, revision: value.storageRevision, buildId: "build-1", diagnostics: [] })),
    dispose: vi.fn(async () => {}),
  };
  const publisher: AuthoringPublisher = {
    target: { targetId: "github-pages", label: "GitHub Pages", destination: "example/site on gh-pages" },
    state: publishState(),
    publish: vi.fn(async (value) => ({ outcome: "published" as const, revision: value.storageRevision, manifestSha256: "digest", targetId: "github-pages", diagnostics: [] })),
    dispose: vi.fn(async () => {}),
  };
  return {
    session,
    preview,
    previewUrl: "http://127.0.0.1:4173/preview/",
    publishers: [publisher],
    createDocumentIdentity: vi.fn(() => NEXT_DOCUMENT_ID),
    ...overrides,
  } satisfies AuthoringShellWorkspace;
}

function host(open: AuthoringShellHost["open"], create: AuthoringShellHost["create"] = async () => await workspaceFixture()): AuthoringShellHost {
  return {
    themes: [
      { id: "forme-classless", label: "Forme Classless" },
      { id: "high-contrast", label: "High contrast" },
    ],
    open,
    create,
  };
}

describe("AuthoringShell", () => {
  it("opens an existing workspace and renders only reviewed product controls", async () => {
    const workspace = await workspaceFixture();
    render(<AuthoringShell host={host(async () => workspace)} />);

    expect(await screen.findByRole("heading", { name: "Forme authoring" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Editing Welcome" })).toBeInTheDocument();
    expect(screen.getByTitle("Site preview")).toHaveAttribute("src", workspace.previewUrl);
    expect(screen.getByText("example/site on gh-pages")).toBeInTheDocument();
  });

  it("creates a first workspace from closed frozen title and theme input", async () => {
    const workspace = await workspaceFixture();
    const create = vi.fn(async (input) => {
      expect(Object.isFrozen(input)).toBe(true);
      expect(input).toEqual({ title: "My first site", themeId: "high-contrast" });
      return workspace;
    });
    render(<AuthoringShell host={host(async () => null, create)} />);

    expect(await screen.findByRole("heading", { name: "Create your Forme site" })).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Site title"), { target: { value: "My first site" } });
    fireEvent.change(screen.getByLabelText("Theme"), { target: { value: "high-contrast" } });
    fireEvent.click(screen.getByRole("button", { name: "Create site" }));
    expect(await screen.findByRole("heading", { name: "Forme authoring" })).toBeInTheDocument();
    expect(create).toHaveBeenCalledOnce();
  });

  it("previews the exact active session and reports the bounded result", async () => {
    const workspace = await workspaceFixture();
    render(<AuthoringShell host={host(async () => workspace)} />);
    await screen.findByRole("heading", { name: "Forme authoring" });

    fireEvent.click(screen.getByRole("button", { name: "Refresh preview" }));
    expect(await screen.findByText("Preview is ready." )).toBeInTheDocument();
    expect(workspace.preview.request).toHaveBeenCalledWith(workspace.session);
  });

  it("requires a separate complete target confirmation before publication", async () => {
    const workspace = await workspaceFixture();
    render(<AuthoringShell host={host(async () => workspace)} />);
    await screen.findByRole("heading", { name: "Forme authoring" });

    fireEvent.click(screen.getByRole("button", { name: "Review publication" }));
    expect(screen.getByRole("heading", { name: "Confirm publication" })).toBeInTheDocument();
    expect(screen.getByText("GitHub Pages")).toBeInTheDocument();
    expect(screen.getByText("example/site on gh-pages")).toBeInTheDocument();
    expect(workspace.publishers[0]!.publish).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Publish to GitHub Pages" }));
    expect(await screen.findByText("Site published." )).toBeInTheDocument();
    expect(workspace.publishers[0]!.publish).toHaveBeenCalledWith(workspace.session);
  });

  it("redacts host failures and remains retryable", async () => {
    const workspace = await workspaceFixture();
    const open = vi.fn()
      .mockRejectedValueOnce(new Error("/secret/profile path"))
      .mockResolvedValueOnce(workspace);
    render(<AuthoringShell host={host(open)} />);

    expect(await screen.findByText("The authoring workspace could not be opened.")).toBeInTheDocument();
    expect(screen.queryByText(/secret/)).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    expect(await screen.findByRole("heading", { name: "Forme authoring" })).toBeInTheDocument();
  });

  it("aborts loading and retires every admitted coordinator exactly once", async () => {
    const workspace = await workspaceFixture();
    let resolve!: (value: AuthoringShellWorkspace) => void;
    const pending = new Promise<AuthoringShellWorkspace>((done) => { resolve = done; });
    const view = render(<AuthoringShell host={host(async (signal) => {
      expect(signal.aborted).toBe(false);
      return await pending;
    })} />);
    view.unmount();
    resolve(workspace);

    await waitFor(() => expect(workspace.preview.dispose).toHaveBeenCalledOnce());
    expect(workspace.publishers[0]!.dispose).toHaveBeenCalledOnce();
  });
});
