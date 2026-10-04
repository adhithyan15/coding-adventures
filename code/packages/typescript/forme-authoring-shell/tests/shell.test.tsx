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
const DIGEST = "a".repeat(64);

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
    publish: vi.fn(async (value) => ({ outcome: "published" as const, revision: value.storageRevision, manifestSha256: DIGEST, targetId: "github-pages", diagnostics: [] })),
    dispose: vi.fn(async () => {}),
  };
  return {
    session,
    preview,
    previewUrl: "http://127.0.0.1:4173/preview/",
    publishers: [publisher],
    createDocumentIdentity: vi.fn(() => NEXT_DOCUMENT_ID),
    dispose: vi.fn(async () => { await Promise.all([preview.dispose(), publisher.dispose()]); }),
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
  it("resolves the published package entrypoint", async () => {
    const entrypoint = await import("@coding-adventures/forme-authoring-shell");
    expect(entrypoint.VERSION).toBe("0.1.0");
  });

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

  it("admits only one same-tick creation, preview, or publication action", async () => {
    let resolveCreate!: (value: AuthoringShellWorkspace) => void;
    const createdWorkspace = await workspaceFixture();
    const createPending = new Promise<AuthoringShellWorkspace>((done) => { resolveCreate = done; });
    const create = vi.fn(() => createPending);
    const firstRun = render(<AuthoringShell host={host(async () => null, create)} />);
    await screen.findByRole("heading", { name: "Create your Forme site" });
    fireEvent.change(screen.getByLabelText("Site title"), { target: { value: "One site" } });
    const createButton = screen.getByRole("button", { name: "Create site" });
    fireEvent.click(createButton);
    fireEvent.click(createButton);
    await waitFor(() => expect(create).toHaveBeenCalledOnce());
    resolveCreate(createdWorkspace);
    await screen.findByRole("heading", { name: "Forme authoring" });
    firstRun.unmount();

    const workspace = await workspaceFixture();
    let resolvePreview!: (value: Awaited<ReturnType<AuthoringPreviewCoordinator["request"]>>) => void;
    const previewPending = new Promise<Awaited<ReturnType<AuthoringPreviewCoordinator["request"]>>>((done) => { resolvePreview = done; });
    vi.mocked(workspace.preview.request).mockReturnValueOnce(previewPending);
    const actions = render(<AuthoringShell host={host(async () => workspace)} />);
    await screen.findByRole("heading", { name: "Forme authoring" });
    const previewButton = screen.getByRole("button", { name: "Refresh preview" });
    fireEvent.click(previewButton);
    fireEvent.click(previewButton);
    await waitFor(() => expect(workspace.preview.request).toHaveBeenCalledOnce());
    resolvePreview({ outcome: "ready", revision: workspace.session.storageRevision, buildId: "build", diagnostics: [] });
    await screen.findByText("Preview is ready.");

    let resolvePublish!: (value: Awaited<ReturnType<AuthoringPublisher["publish"]>>) => void;
    const publishPending = new Promise<Awaited<ReturnType<AuthoringPublisher["publish"]>>>((done) => { resolvePublish = done; });
    vi.mocked(workspace.publishers[0]!.publish).mockReturnValueOnce(publishPending);
    fireEvent.click(screen.getByRole("button", { name: "Review publication" }));
    const publishButton = screen.getByRole("button", { name: "Publish to GitHub Pages" });
    fireEvent.click(publishButton);
    fireEvent.click(publishButton);
    await waitFor(() => expect(workspace.publishers[0]!.publish).toHaveBeenCalledOnce());
    resolvePublish({ outcome: "published", revision: workspace.session.storageRevision, manifestSha256: DIGEST, targetId: "github-pages", diagnostics: [] });
    await screen.findByText("Site published.");
    actions.unmount();
  });

  it("previews the exact active session and reports the bounded result", async () => {
    const workspace = await workspaceFixture();
    render(<AuthoringShell host={host(async () => workspace)} />);
    await screen.findByRole("heading", { name: "Forme authoring" });

    fireEvent.click(screen.getByRole("button", { name: "Refresh preview" }));
    expect(await screen.findByText("Preview is ready." )).toBeInTheDocument();
    const previewSession = vi.mocked(workspace.preview.request).mock.calls[0]![0];
    expect(previewSession).not.toBe(workspace.session);
    expect(previewSession.project).toEqual(workspace.session.project);
    expect(previewSession.storageRevision).toBe(workspace.session.storageRevision);
  });

  it("refreshes the admitted session snapshot after every captured mutation", async () => {
    const workspace = await workspaceFixture();
    render(<AuthoringShell host={host(async () => workspace)} />);
    await screen.findByRole("heading", { name: "Forme authoring" });
    fireEvent.click(screen.getByRole("button", { name: "Refresh preview" }));
    await screen.findByText("Preview is ready.");
    const admitted = vi.mocked(workspace.preview.request).mock.calls[0]![0];
    const revision = admitted.storageRevision;
    await admitted.dispatchAtRevision(revision, {
      type: "record-publication",
      publication: {
        authoringRevision: revision,
        manifestSha256: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        targetId: "github-pages",
      },
    });
    expect(admitted.project.documents[0]!.status).toBe("published");
    await admitted.undo();
    expect(admitted.project.documents[0]!.status).toBe("draft");
    await admitted.redo();
    expect(admitted.project.documents[0]!.status).toBe("published");
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
    const [publishSession, signal] = vi.mocked(workspace.publishers[0]!.publish).mock.calls[0]!;
    expect(publishSession).not.toBe(workspace.session);
    expect(publishSession.storageRevision).toBe(workspace.session.storageRevision);
    expect(signal).toBeInstanceOf(AbortSignal);
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
    const open = vi.fn(async (signal: AbortSignal) => {
      expect(signal.aborted).toBe(false);
      return await pending;
    });
    const view = render(<AuthoringShell host={host(open)} />);
    await waitFor(() => expect(open).toHaveBeenCalledOnce());
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

  it("redacts synchronous host and coordinator failures behind Promise boundaries", async () => {
    const openFailure = host((() => { throw new Error("/secret/open-sync"); }) as AuthoringShellHost["open"]);
    const opened = render(<AuthoringShell host={openFailure} />);
    expect(await screen.findByText("The authoring workspace could not be opened.")).toBeInTheDocument();
    opened.unmount();

    const createFailure = host(async () => null, (() => { throw new Error("/secret/create-sync"); }) as AuthoringShellHost["create"]);
    const created = render(<AuthoringShell host={createFailure} />);
    await screen.findByRole("heading", { name: "Create your Forme site" });
    fireEvent.change(screen.getByLabelText("Site title"), { target: { value: "Safe title" } });
    fireEvent.click(screen.getByRole("button", { name: "Create site" }));
    expect(await screen.findByText("The authoring workspace could not be created.")).toBeInTheDocument();
    created.unmount();

    const previewWorkspace = await workspaceFixture({
      preview: { state: state(), request: (() => { throw new Error("/secret/preview-sync"); }) as AuthoringPreviewCoordinator["request"], dispose: vi.fn(async () => {}) },
    });
    const previewed = render(<AuthoringShell host={host(async () => previewWorkspace)} />);
    await screen.findByRole("heading", { name: "Forme authoring" });
    fireEvent.click(screen.getByRole("button", { name: "Refresh preview" }));
    expect(await screen.findByText("Preview could not be prepared.")).toBeInTheDocument();
    previewed.unmount();

    const publishWorkspace = await workspaceFixture();
    publishWorkspace.publishers[0]!.publish = (() => { throw new Error("/secret/publish-sync"); }) as AuthoringPublisher["publish"];
    render(<AuthoringShell host={host(async () => publishWorkspace)} />);
    await screen.findByRole("heading", { name: "Forme authoring" });
    fireEvent.click(screen.getByRole("button", { name: "Review publication" }));
    fireEvent.click(screen.getByRole("button", { name: "Publish to GitHub Pages" }));
    expect(await screen.findByText("Publication failed before its commit point.")).toBeInTheDocument();
    expect(screen.queryByText(/secret/)).not.toBeInTheDocument();
  });

  it("retires a resolved workspace when later admission fails", async () => {
    const workspace = await workspaceFixture({ previewUrl: "http://example.test/" });
    render(<AuthoringShell host={host(async () => workspace)} />);
    expect(await screen.findByText("The authoring workspace could not be opened.")).toBeInTheDocument();
    await waitFor(() => expect(workspace.dispose).toHaveBeenCalledOnce());
    expect(workspace.preview.dispose).toHaveBeenCalledOnce();
    expect(workspace.publishers[0]!.dispose).toHaveBeenCalledOnce();
  });

  it("fails closed when a workspace descriptor trap prevents safe admission", async () => {
    const workspace = await workspaceFixture();
    const trapped = new Proxy(workspace, {
      getOwnPropertyDescriptor() { throw new Error("/secret/proxy-trap"); },
    });
    render(<AuthoringShell host={host(async () => trapped)} />);
    expect(await screen.findByText("The authoring workspace could not be opened.")).toBeInTheDocument();
    expect(screen.queryByText(/secret/)).not.toBeInTheDocument();
  });

  it("fails closed for sparse publisher arrays and synchronous disposal failure", async () => {
    const workspace = await workspaceFixture();
    const publishers = Array(2) as AuthoringPublisher[];
    publishers[1] = workspace.publishers[0]!;
    const malformed = {
      ...workspace,
      publishers,
      dispose: (() => { throw new Error("/secret/dispose-sync"); }) as AuthoringShellWorkspace["dispose"],
    };
    render(<AuthoringShell host={host(async () => malformed)} />);
    expect(await screen.findByText("The authoring workspace could not be opened.")).toBeInTheDocument();
    expect(screen.queryByText(/secret/)).not.toBeInTheDocument();
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
    const revision = workspace.session.storageRevision;
    vi.mocked(workspace.preview.request)
      .mockResolvedValueOnce({ outcome: "cancelled", revision, buildId: null, diagnostics: [] })
      .mockResolvedValueOnce({ outcome: "superseded", revision, buildId: null, diagnostics: [] })
      .mockResolvedValueOnce({ outcome: "failed", revision, buildId: null, diagnostics: [{ severity: "error", code: "BUILD", stageName: "emit", instanceId: "main", message: "Reviewed build failure" }] })
      .mockResolvedValueOnce({ outcome: "failed", revision, buildId: null, diagnostics: [] })
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

  it.each([
    ["stale revision", (revision: string) => ({ outcome: "ready", revision: `${revision}-stale`, buildId: "build", diagnostics: [] })],
    ["missing build identity", (revision: string) => ({ outcome: "ready", revision, buildId: null, diagnostics: [] })],
    ["unknown outcome", (revision: string) => ({ outcome: "spoofed", revision, buildId: null, diagnostics: [] })],
    ["oversized diagnostic", (revision: string) => ({ outcome: "failed", revision, buildId: null, diagnostics: [{ severity: "error", code: "BUILD", stageName: "emit", instanceId: "main", message: "x".repeat(2_049) }] })],
    ["accessor diagnostic", (revision: string) => {
      const diagnostic = { severity: "error", code: "BUILD", stageName: "emit", instanceId: "main" } as Record<string, unknown>;
      Object.defineProperty(diagnostic, "message", { enumerable: true, get: () => "/secret/accessor" });
      return { outcome: "failed", revision, buildId: null, diagnostics: [diagnostic] };
    }],
  ])("rejects a hostile preview result with %s", async (_name, attempt) => {
    const workspace = await workspaceFixture();
    vi.mocked(workspace.preview.request).mockResolvedValueOnce(attempt(workspace.session.storageRevision) as never);
    render(<AuthoringShell host={host(async () => workspace)} />);
    await screen.findByRole("heading", { name: "Forme authoring" });
    fireEvent.click(screen.getByRole("button", { name: "Refresh preview" }));
    expect(await screen.findByText("Preview could not be prepared.")).toBeInTheDocument();
    expect(screen.queryByText(/secret|x{100}/)).not.toBeInTheDocument();
  });

  it("reports every bounded publication outcome and rejected pre-commit work", async () => {
    const workspace = await workspaceFixture();
    const revision = workspace.session.storageRevision;
    const publish = vi.mocked(workspace.publishers[0]!.publish);
    publish
      .mockResolvedValueOnce({ outcome: "cancelled", revision, manifestSha256: null, targetId: "github-pages", diagnostics: [] })
      .mockResolvedValueOnce({ outcome: "failed", revision, manifestSha256: null, targetId: "github-pages", diagnostics: [{ severity: "error", code: "DEPLOY", message: "Reviewed deploy failure" }] })
      .mockResolvedValueOnce({ outcome: "failed", revision, manifestSha256: null, targetId: "github-pages", diagnostics: [] })
      .mockResolvedValueOnce({ outcome: "indeterminate", revision, manifestSha256: DIGEST, targetId: "github-pages", diagnostics: [{ severity: "error", code: "UNCERTAIN", message: "Reconcile remote target" }] })
      .mockResolvedValueOnce({ outcome: "indeterminate", revision, manifestSha256: DIGEST, targetId: "github-pages", diagnostics: [] })
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

  it.each([
    ["stale revision", (revision: string) => ({ outcome: "published", revision: `${revision}-stale`, manifestSha256: DIGEST, targetId: "github-pages", diagnostics: [] })],
    ["wrong target", (revision: string) => ({ outcome: "published", revision, manifestSha256: DIGEST, targetId: "other", diagnostics: [] })],
    ["invalid digest", (revision: string) => ({ outcome: "published", revision, manifestSha256: "digest", targetId: "github-pages", diagnostics: [] })],
    ["missing committed digest", (revision: string) => ({ outcome: "published", revision, manifestSha256: null, targetId: "github-pages", diagnostics: [] })],
    ["uncommitted digest", (revision: string) => ({ outcome: "failed", revision, manifestSha256: DIGEST, targetId: "github-pages", diagnostics: [] })],
    ["unknown outcome", (revision: string) => ({ outcome: "spoofed", revision, manifestSha256: null, targetId: "github-pages", diagnostics: [] })],
    ["oversized diagnostic", (revision: string) => ({ outcome: "failed", revision, manifestSha256: null, targetId: "github-pages", diagnostics: [{ severity: "error", code: "DEPLOY", message: "x".repeat(2_049) }] })],
  ])("rejects a hostile publication result with %s", async (_name, attempt) => {
    const workspace = await workspaceFixture();
    vi.mocked(workspace.publishers[0]!.publish).mockResolvedValueOnce(attempt(workspace.session.storageRevision) as never);
    render(<AuthoringShell host={host(async () => workspace)} />);
    await screen.findByRole("heading", { name: "Forme authoring" });
    fireEvent.click(screen.getByRole("button", { name: "Review publication" }));
    fireEvent.click(screen.getByRole("button", { name: "Publish to GitHub Pages" }));
    expect(await screen.findByText("Publication failed before its commit point.")).toBeInTheDocument();
    expect(screen.queryByText(/x{100}/)).not.toBeInTheDocument();
  });

  it("selects and cancels a second complete publication review", async () => {
    const workspace = await workspaceFixture();
    const second: AuthoringPublisher = {
      target: { targetId: "s3", label: "S3 bucket", destination: "s3://reviewed-bucket/site" },
      state: publishState(),
      publish: vi.fn(async (value) => ({ outcome: "published", revision: value.storageRevision, manifestSha256: DIGEST, targetId: "s3", diagnostics: [] })),
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

  it("snapshots mutable session getters instead of exposing the raw session", async () => {
    const workspace = await workspaceFixture();
    const raw = workspace.session;
    let projectReads = 0;
    let revisionReads = 0;
    const changing = {
      get project() {
        projectReads += 1;
        if (projectReads > 1) throw new Error("/secret/project-getter");
        return raw.project;
      },
      get canUndo() { return raw.canUndo; },
      get canRedo() { return raw.canRedo; },
      get storageRevision() {
        revisionReads += 1;
        if (revisionReads > 1) throw new Error("/secret/revision-getter");
        return raw.storageRevision;
      },
      dispatch: raw.dispatch.bind(raw),
      dispatchAtRevision: raw.dispatchAtRevision.bind(raw),
      undo: raw.undo.bind(raw),
      redo: raw.redo.bind(raw),
    } satisfies AuthoringSession;
    const changingWorkspace = { ...workspace, session: changing };
    render(<AuthoringShell host={host(async () => changingWorkspace)} />);
    expect(await screen.findByRole("heading", { name: "Editing Welcome" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Refresh preview" }));
    expect(await screen.findByText("Preview is ready.")).toBeInTheDocument();
    expect(projectReads).toBe(1);
    expect(revisionReads).toBe(1);
    expect(screen.queryByText(/secret/)).not.toBeInTheDocument();
  });

  it("invokes the document identity callback without workspace authority", async () => {
    const workspace = await workspaceFixture();
    let receiver: unknown = Symbol("unset");
    const identity = function (this: unknown): string {
      receiver = this;
      return NEXT_DOCUMENT_ID;
    };
    const bounded = { ...workspace, createDocumentIdentity: identity };
    render(<AuthoringShell host={host(async () => bounded)} />);
    await screen.findByRole("heading", { name: "Forme authoring" });
    fireEvent.change(screen.getByLabelText("New document title"), { target: { value: "Second" } });
    fireEvent.change(screen.getByLabelText("New document slug"), { target: { value: "second" } });
    fireEvent.click(screen.getByRole("button", { name: "Create document" }));
    expect(await screen.findByRole("heading", { name: "Editing Second" })).toBeInTheDocument();
    expect(receiver).toBeUndefined();
  });

  it("ignores a stale action settlement after host replacement", async () => {
    const first = await workspaceFixture();
    const second = await workspaceFixture({ previewUrl: "http://localhost:4999/new/" });
    let resolvePreview!: (value: Awaited<ReturnType<AuthoringPreviewCoordinator["request"]>>) => void;
    const pending = new Promise<Awaited<ReturnType<AuthoringPreviewCoordinator["request"]>>>((done) => { resolvePreview = done; });
    vi.mocked(first.preview.request).mockReturnValueOnce(pending);
    const view = render(<AuthoringShell host={host(async () => first)} />);
    await screen.findByRole("heading", { name: "Forme authoring" });
    fireEvent.click(screen.getByRole("button", { name: "Refresh preview" }));
    await waitFor(() => expect(first.preview.request).toHaveBeenCalledOnce());

    view.rerender(<AuthoringShell host={host(async () => second)} />);
    expect(await screen.findByTitle("Site preview")).toHaveAttribute("src", second.previewUrl);
    await waitFor(() => expect(first.dispose).toHaveBeenCalledOnce());
    resolvePreview({ outcome: "ready", revision: first.session.storageRevision, buildId: "stale", diagnostics: [] });
    await pending;
    await waitFor(() => expect(screen.queryByText("Preview is ready.")).not.toBeInTheDocument());
  });

  it("redacts workspace cleanup failure during replacement", async () => {
    const first = await workspaceFixture({
      dispose: vi.fn(async () => { throw new Error("/secret/cleanup"); }),
    });
    const second = await workspaceFixture({ previewUrl: "http://localhost:5000/next/" });
    const view = render(<AuthoringShell host={host(async () => first)} />);
    await screen.findByRole("heading", { name: "Forme authoring" });
    view.rerender(<AuthoringShell host={host(async () => second)} />);
    expect(await screen.findByText("Workspace cleanup did not complete; reload before continuing.")).toBeInTheDocument();
    expect(screen.queryByText(/secret/)).not.toBeInTheDocument();
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
    await waitFor(() => expect(capturedSignal).toBeDefined());
    view.unmount();
    expect(capturedSignal?.aborted).toBe(true);
    resolve(workspace);
    await waitFor(() => expect(workspace.preview.dispose).toHaveBeenCalledOnce());
    expect(workspace.publishers[0]!.dispose).toHaveBeenCalledOnce();
  });
});
