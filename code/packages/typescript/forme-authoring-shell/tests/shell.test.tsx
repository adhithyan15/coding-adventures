import "@testing-library/jest-dom";

import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
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
    const confirmation = screen.getByRole("group", { name: "Confirm publication" });
    expect(within(confirmation).getByText("GitHub Pages")).toBeInTheDocument();
    expect(within(confirmation).getByText("example/site on gh-pages")).toBeInTheDocument();
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

  it.each([
    ["undefined workspace", async () => undefined],
    ["missing session", async () => ({ ...(await workspaceFixture()), session: undefined })],
    ["published active document", async () => {
      const workspace = await workspaceFixture();
      const project = {
        ...workspace.session.project,
        documents: workspace.session.project.documents.map((document) => ({ ...document, status: "published" as const })),
      };
      return { ...workspace, session: { ...workspace.session, project } };
    }],
    ["null preview", async () => ({ ...(await workspaceFixture()), preview: null })],
    ["missing preview request", async () => ({ ...(await workspaceFixture()), preview: { dispose: async () => {} } })],
    ["empty publishers", async () => ({ ...(await workspaceFixture()), publishers: [] })],
    ["too many publishers", async () => {
      const workspace = await workspaceFixture();
      return { ...workspace, publishers: Array.from({ length: 33 }, () => workspace.publishers[0]!) };
    }],
    ["duplicate target identities", async () => {
      const workspace = await workspaceFixture();
      return { ...workspace, publishers: [workspace.publishers[0]!, workspace.publishers[0]!] };
    }],
    ["invalid target identity", async () => {
      const workspace = await workspaceFixture();
      return { ...workspace, publishers: [{ ...workspace.publishers[0]!, target: { ...workspace.publishers[0]!.target, targetId: "Bad Target" } }] };
    }],
    ["unsafe target label", async () => {
      const workspace = await workspaceFixture();
      return { ...workspace, publishers: [{ ...workspace.publishers[0]!, target: { ...workspace.publishers[0]!.target, label: "bad\u202etarget" } }] };
    }],
    ["oversized target label", async () => {
      const workspace = await workspaceFixture();
      return { ...workspace, publishers: [{ ...workspace.publishers[0]!, target: { ...workspace.publishers[0]!.target, label: "🙂".repeat(513) } }] };
    }],
    ["missing publisher method", async () => {
      const workspace = await workspaceFixture();
      return { ...workspace, publishers: [{ target: workspace.publishers[0]!.target, dispose: async () => {} }] };
    }],
    ["non-loopback preview", async () => ({ ...(await workspaceFixture()), previewUrl: "https://example.test/preview" })],
    ["external HTTP preview", async () => ({ ...(await workspaceFixture()), previewUrl: "http://example.test/preview" })],
    ["credentialed preview", async () => ({ ...(await workspaceFixture()), previewUrl: "http://user@localhost/preview" })],
    ["fragment preview", async () => ({ ...(await workspaceFixture()), previewUrl: "http://localhost/preview#secret" })],
    ["malformed preview", async () => ({ ...(await workspaceFixture()), previewUrl: "not a URL" })],
    ["missing identity factory", async () => ({ ...(await workspaceFixture()), createDocumentIdentity: undefined })],
  ])("fails closed for a %s", async (_name, makeWorkspace) => {
    render(<AuthoringShell host={host(async () => await makeWorkspace() as AuthoringShellWorkspace)} />);
    expect(await screen.findByText("The authoring workspace could not be opened.")).toBeInTheDocument();
  });

  it("rejects accessor-backed host fields before calling them", () => {
    const unsafe = {} as AuthoringShellHost;
    Object.defineProperty(unsafe, "themes", { enumerable: true, get: () => [] });
    Object.defineProperty(unsafe, "open", { enumerable: true, value: async () => null });
    Object.defineProperty(unsafe, "create", { enumerable: true, value: async () => await workspaceFixture() });
    expect(() => render(<AuthoringShell host={unsafe} />)).toThrow("authoring shell host could not be inspected safely");
  });

  it("accepts every loopback spelling and normalizes its preview URL", async () => {
    for (const previewUrl of ["http://localhost:4173/", "http://[::1]:4173/"]) {
      const workspace = await workspaceFixture({ previewUrl });
      const view = render(<AuthoringShell host={host(async () => workspace)} />);
      expect(await screen.findByTitle("Site preview")).toHaveAttribute("src", previewUrl);
      view.unmount();
    }
  });

  it("reports every bounded preview outcome and redacts rejected work", async () => {
    const workspace = await workspaceFixture();
    vi.mocked(workspace.preview.request)
      .mockResolvedValueOnce({ outcome: "cancelled", revision: "r1", buildId: null, diagnostics: [] })
      .mockResolvedValueOnce({ outcome: "superseded", revision: "r1", buildId: null, diagnostics: [] })
      .mockResolvedValueOnce({ outcome: "failed", revision: "r1", buildId: null, diagnostics: [{ severity: "error", code: "BUILD", stageName: "emit", instanceId: "main", message: "Reviewed build failure" }] })
      .mockResolvedValueOnce({ outcome: "failed", revision: "r1", buildId: null, diagnostics: [] })
      .mockRejectedValueOnce(new Error("/secret/preview/path"));
    render(<AuthoringShell host={host(async () => workspace)} />);
    await screen.findByRole("heading", { name: "Forme authoring" });

    for (const expected of [
      "Preview was cancelled.",
      "Preview was superseded by a newer revision.",
      "Reviewed build failure",
      "Preview could not be prepared.",
      "Preview could not be prepared.",
    ]) {
      fireEvent.click(screen.getByRole("button", { name: "Refresh preview" }));
      await waitFor(() => expect(screen.getByText(expected)).toBeInTheDocument());
    }
    expect(screen.queryByText(/secret/)).not.toBeInTheDocument();
  });

  it("reports every bounded publication outcome and rejected pre-commit work", async () => {
    const workspace = await workspaceFixture();
    const publish = vi.mocked(workspace.publishers[0]!.publish);
    publish
      .mockResolvedValueOnce({ outcome: "cancelled", revision: "r1", manifestSha256: null, targetId: "github-pages", diagnostics: [] })
      .mockResolvedValueOnce({ outcome: "failed", revision: "r1", manifestSha256: null, targetId: "github-pages", diagnostics: [{ severity: "error", code: "DEPLOY", message: "Reviewed deploy failure" }] })
      .mockResolvedValueOnce({ outcome: "failed", revision: "r1", manifestSha256: null, targetId: "github-pages", diagnostics: [] })
      .mockResolvedValueOnce({ outcome: "indeterminate", revision: "r1", manifestSha256: "digest", targetId: "github-pages", diagnostics: [{ severity: "error", code: "UNCERTAIN", message: "Reconcile remote target" }] })
      .mockResolvedValueOnce({ outcome: "indeterminate", revision: "r1", manifestSha256: "digest", targetId: "github-pages", diagnostics: [] })
      .mockRejectedValueOnce(new Error("/secret/deploy/path"));
    render(<AuthoringShell host={host(async () => workspace)} />);
    await screen.findByRole("heading", { name: "Forme authoring" });

    for (const expected of [
      "Publication was cancelled.",
      "Reviewed deploy failure",
      "Publication failed.",
      "Reconcile remote target",
      "Publication state must be reconciled.",
      "Publication failed before its commit point.",
    ]) {
      fireEvent.click(screen.getByRole("button", { name: "Review publication" }));
      fireEvent.click(screen.getByRole("button", { name: "Publish to GitHub Pages" }));
      await waitFor(() => expect(screen.getByText(expected)).toBeInTheDocument());
    }
    expect(screen.queryByText(/secret/)).not.toBeInTheDocument();
  });

  it("selects and cancels a second complete publication review", async () => {
    const workspace = await workspaceFixture();
    const second: AuthoringPublisher = {
      target: { targetId: "s3", label: "S3 bucket", destination: "s3://reviewed-bucket/site" },
      state: publishState(),
      publish: vi.fn(async () => ({ outcome: "published", revision: "r1", manifestSha256: "digest", targetId: "s3", diagnostics: [] })),
      dispose: vi.fn(async () => {}),
    };
    const twoTargets = { ...workspace, publishers: [workspace.publishers[0]!, second] };
    render(<AuthoringShell host={host(async () => twoTargets)} />);
    await screen.findByRole("heading", { name: "Forme authoring" });

    fireEvent.click(screen.getByRole("button", { name: "Review publication" }));
    fireEvent.change(screen.getByLabelText("Publication target"), { target: { value: "s3" } });
    expect(screen.queryByRole("group", { name: "Confirm publication" })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Review publication" }));
    expect(screen.getByRole("button", { name: "Publish to S3 bucket" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("group", { name: "Confirm publication" })).not.toBeInTheDocument();
  });

  it("validates first-run input and redacts creation failures", async () => {
    const create = vi.fn().mockRejectedValue(new Error("/secret/create/path"));
    render(<AuthoringShell host={host(async () => null, create)} />);
    await screen.findByRole("heading", { name: "Create your Forme site" });

    fireEvent.change(screen.getByLabelText("Site title"), { target: { value: " invalid " } });
    fireEvent.click(screen.getByRole("button", { name: "Create site" }));
    expect(await screen.findByText("Enter a valid site title and select a reviewed theme.")).toBeInTheDocument();
    expect(create).not.toHaveBeenCalled();

    fireEvent.change(screen.getByLabelText("Site title"), { target: { value: "Valid title" } });
    fireEvent.click(screen.getByRole("button", { name: "Create site" }));
    expect(await screen.findByText("The authoring workspace could not be created.")).toBeInTheDocument();
    expect(screen.queryByText(/secret/)).not.toBeInTheDocument();
  });

  it("rejects missing first-run fields without invoking the host", async () => {
    const create = vi.fn(async () => await workspaceFixture());
    const view = render(<AuthoringShell host={host(async () => null, create)} />);
    await screen.findByRole("heading", { name: "Create your Forme site" });
    const title = screen.getByLabelText("Site title");
    fireEvent.change(title, { target: { value: "Valid title" } });
    title.removeAttribute("name");
    fireEvent.submit(view.container.querySelector("form")!);
    expect(await screen.findByText("Enter a valid site title and select a reviewed theme.")).toBeInTheDocument();

    title.setAttribute("name", "title");
    const theme = screen.getByLabelText("Theme");
    theme.removeAttribute("name");
    fireEvent.submit(view.container.querySelector("form")!);
    expect(create).not.toHaveBeenCalled();
  });

  it("ignores a loading rejection that settles after unmount", async () => {
    let reject!: (reason: unknown) => void;
    const pending = new Promise<AuthoringShellWorkspace | null>((_resolve, fail) => { reject = fail; });
    const view = render(<AuthoringShell host={host(async () => await pending)} />);
    view.unmount();
    reject(new Error("late failure"));
    await pending.catch(() => undefined);
  });

  it("settles preview and publication work safely after unmount", async () => {
    const workspace = await workspaceFixture();
    let resolvePreview!: (value: Awaited<ReturnType<AuthoringPreviewCoordinator["request"]>>) => void;
    const previewPending = new Promise<Awaited<ReturnType<AuthoringPreviewCoordinator["request"]>>>((done) => { resolvePreview = done; });
    vi.mocked(workspace.preview.request).mockReturnValueOnce(previewPending);
    const view = render(<AuthoringShell host={host(async () => workspace)} />);
    await screen.findByRole("heading", { name: "Forme authoring" });
    fireEvent.click(screen.getByRole("button", { name: "Refresh preview" }));
    view.unmount();
    resolvePreview({ outcome: "ready", revision: "r1", buildId: "build", diagnostics: [] });
    await previewPending;

    const publishWorkspace = await workspaceFixture();
    let rejectPublish!: (reason: unknown) => void;
    const publishPending = new Promise<Awaited<ReturnType<AuthoringPublisher["publish"]>>>((_resolve, fail) => { rejectPublish = fail; });
    vi.mocked(publishWorkspace.publishers[0]!.publish).mockReturnValueOnce(publishPending);
    const publication = render(<AuthoringShell host={host(async () => publishWorkspace)} />);
    await screen.findByRole("heading", { name: "Forme authoring" });
    fireEvent.click(screen.getByRole("button", { name: "Review publication" }));
    fireEvent.click(screen.getByRole("button", { name: "Publish to GitHub Pages" }));
    publication.unmount();
    rejectPublish(new Error("late publication rejection"));
    await publishPending.catch(() => undefined);
  });

  it("aborts pending creation and retires a workspace that resolves after unmount", async () => {
    const workspace = await workspaceFixture();
    let resolve!: (value: AuthoringShellWorkspace) => void;
    let capturedSignal: AbortSignal | undefined;
    const pending = new Promise<AuthoringShellWorkspace>((done) => { resolve = done; });
    const view = render(<AuthoringShell host={host(async () => null, async (_input, signal) => {
      capturedSignal = signal;
      return await pending;
    })} />);
    await screen.findByRole("heading", { name: "Create your Forme site" });
    fireEvent.change(screen.getByLabelText("Site title"), { target: { value: "Late site" } });
    fireEvent.click(screen.getByRole("button", { name: "Create site" }));
    view.unmount();
    expect(capturedSignal?.aborted).toBe(true);
    resolve(workspace);
    await waitFor(() => expect(workspace.preview.dispose).toHaveBeenCalledOnce());
    expect(workspace.publishers[0]!.dispose).toHaveBeenCalledOnce();
  });
});
