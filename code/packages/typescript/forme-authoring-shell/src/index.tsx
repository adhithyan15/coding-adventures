/** Capability-free first-run composition for FM09 authoring. */

import { useEffect, useMemo, useRef, useState } from "react";
import type { FormEvent, ReactNode } from "react";
import {
  validateAuthoringProject,
  type AuthoringSession,
} from "@coding-adventures/forme-authoring-core";
import {
  AUTHORING_EDITOR_CSS,
  AuthoringEditor,
  validateThemeOptions,
} from "@coding-adventures/forme-authoring-editor";
import type { EditorThemeOption } from "@coding-adventures/forme-authoring-editor";
import type {
  AuthoringPreviewAttempt,
  AuthoringPreviewCoordinator,
} from "@coding-adventures/forme-authoring-preview";
import type {
  AuthoringPublisher,
  AuthoringPublishAttempt,
  AuthoringPublishTargetReview,
} from "@coding-adventures/forme-authoring-publish";

export interface CreateAuthoringShellWorkspaceInput {
  readonly title: string;
  readonly themeId: string;
}

export interface AuthoringShellWorkspace {
  readonly session: AuthoringSession;
  readonly preview: AuthoringPreviewCoordinator;
  readonly previewUrl: string;
  readonly publishers: readonly AuthoringPublisher[];
  createDocumentIdentity(): string | Promise<string>;
}

export interface AuthoringShellHost {
  readonly themes: unknown;
  open(signal: AbortSignal): Promise<AuthoringShellWorkspace | null>;
  create(input: CreateAuthoringShellWorkspaceInput, signal: AbortSignal): Promise<AuthoringShellWorkspace>;
}

export interface AuthoringShellProps {
  readonly host: AuthoringShellHost;
}

export const AUTHORING_SHELL_CSS = `
.forme-authoring-shell { display: grid; gap: 1rem; max-width: 80rem; margin: 0 auto; padding: 1rem; }
.forme-authoring-shell__workspace { display: grid; gap: 1rem; grid-template-columns: minmax(0, 3fr) minmax(18rem, 2fr); }
.forme-authoring-shell__preview { width: 100%; min-height: 32rem; border: 1px solid currentColor; }
.forme-authoring-shell__actions { display: flex; flex-wrap: wrap; gap: .5rem; }
.forme-authoring-shell :focus-visible { outline: 3px solid currentColor; outline-offset: 3px; }
${AUTHORING_EDITOR_CSS}
`;

const MAX_TITLE_SCALARS = 512;
const MAX_URL_UNITS = 2_048;
const MAX_TARGETS = 32;
const UNSAFE_TEXT = /[\u0000-\u001f\u007f-\u009f\u061c\u200e\u200f\u202a-\u202e\u2066-\u2069]/u;
const APPLY = Reflect.apply;
const ABORT = AbortController.prototype.abort;

interface SafePreview {
  request(session: AuthoringSession): Promise<AuthoringPreviewAttempt>;
  dispose(): Promise<void>;
}

interface SafePublisher {
  readonly review: AuthoringPublishTargetReview;
  publish(session: AuthoringSession): Promise<AuthoringPublishAttempt>;
  dispose(): Promise<void>;
}

interface SafeWorkspace {
  readonly session: AuthoringSession;
  readonly preview: SafePreview;
  readonly previewUrl: string;
  readonly publishers: readonly SafePublisher[];
  createDocumentIdentity(): string | Promise<string>;
  dispose(): Promise<void>;
}

interface SafeHost {
  readonly themes: readonly EditorThemeOption[];
  open(signal: AbortSignal): Promise<AuthoringShellWorkspace | null>;
  create(input: CreateAuthoringShellWorkspaceInput, signal: AbortSignal): Promise<AuthoringShellWorkspace>;
}

type ViewState =
  | { readonly phase: "loading" }
  | { readonly phase: "first-run" }
  | { readonly phase: "error" }
  | { readonly phase: "ready"; readonly workspace: SafeWorkspace };

function captureMethod<T extends Function>(value: unknown, name: string, label: string): T {
  if (value === null || (typeof value !== "object" && typeof value !== "function")) throw new TypeError(`${label} is invalid`);
  let cursor: object | null = value as object;
  for (let depth = 0; cursor !== null && depth < 16; depth += 1) {
    const descriptor = Object.getOwnPropertyDescriptor(cursor, name);
    if (descriptor !== undefined) {
      if (!("value" in descriptor) || typeof descriptor.value !== "function") throw new TypeError(`${label} ${name} is invalid`);
      const method = descriptor.value as T;
      return ((...args: unknown[]) => APPLY(method, value, args)) as unknown as T;
    }
    cursor = Object.getPrototypeOf(cursor);
  }
  throw new TypeError(`${label} is missing ${name}`);
}

function dataField(value: unknown, name: string, label: string): unknown {
  if (value === null || typeof value !== "object") throw new TypeError(`${label} is invalid`);
  const descriptor = Object.getOwnPropertyDescriptor(value, name);
  if (descriptor === undefined || !("value" in descriptor) || !descriptor.enumerable) throw new TypeError(`${label} ${name} is invalid`);
  return descriptor.value;
}

function safeText(value: unknown, maximum: number, label: string): string {
  if (typeof value !== "string" || value.length === 0 || value.length > maximum * 2 || value.trim() !== value) {
    throw new TypeError(`${label} is invalid`);
  }
  let scalars = 0;
  for (const _scalar of value) scalars += 1;
  if (scalars > maximum || UNSAFE_TEXT.test(value)) throw new TypeError(`${label} is invalid`);
  return value;
}

function safePreviewUrl(value: unknown): string {
  const text = safeText(value, MAX_URL_UNITS, "preview URL");
  let parsed: URL;
  try { parsed = new URL(text); } catch { throw new TypeError("preview URL is invalid"); }
  if (parsed.protocol !== "http:" || parsed.username !== "" || parsed.password !== "" || parsed.hash !== "") {
    throw new TypeError("preview URL is invalid");
  }
  if (parsed.hostname !== "127.0.0.1" && parsed.hostname !== "localhost" && parsed.hostname !== "[::1]") {
    throw new TypeError("preview URL must use loopback HTTP");
  }
  return parsed.href;
}

function safeReview(value: unknown): AuthoringPublishTargetReview {
  const targetId = safeText(dataField(value, "targetId", "target review"), 256, "target identity");
  if (!/^[a-z0-9]+(?:[._-][a-z0-9]+)*$/.test(targetId)) throw new TypeError("target identity is invalid");
  return Object.freeze({
    targetId,
    label: safeText(dataField(value, "label", "target review"), 512, "target label"),
    destination: safeText(dataField(value, "destination", "target review"), 2_048, "target destination"),
  });
}

function prepareHost(value: AuthoringShellHost): SafeHost {
  try {
    const themes = validateThemeOptions(dataField(value, "themes", "authoring shell host"));
    const open = captureMethod<AuthoringShellHost["open"]>(value, "open", "authoring shell host");
    const create = captureMethod<AuthoringShellHost["create"]>(value, "create", "authoring shell host");
    return Object.freeze({ themes, open, create });
  } catch {
    throw new TypeError("authoring shell host could not be inspected safely");
  }
}

function admitWorkspace(value: AuthoringShellWorkspace): SafeWorkspace {
  try {
    const session = dataField(value, "session", "workspace") as AuthoringSession;
    const project = validateAuthoringProject(session.project);
    const active = project.documents.find((document) => document.id === project.activeDocumentId);
    if (active === undefined || active.status !== "draft") throw new TypeError("workspace must contain an active draft");
    const rawPreview = dataField(value, "preview", "workspace");
    const preview: SafePreview = Object.freeze({
      request: captureMethod<AuthoringPreviewCoordinator["request"]>(rawPreview, "request", "preview"),
      dispose: captureMethod<AuthoringPreviewCoordinator["dispose"]>(rawPreview, "dispose", "preview"),
    });
    const rawPublishers = dataField(value, "publishers", "workspace");
    if (!Array.isArray(rawPublishers) || rawPublishers.length < 1 || rawPublishers.length > MAX_TARGETS) {
      throw new TypeError("workspace publishers are invalid");
    }
    const identities = new Set<string>();
    const publishers = rawPublishers.map((publisher): SafePublisher => {
      const review = safeReview(dataField(publisher, "target", "publisher"));
      if (identities.has(review.targetId)) throw new TypeError("workspace target identities must be unique");
      identities.add(review.targetId);
      return Object.freeze({
        review,
        publish: captureMethod<AuthoringPublisher["publish"]>(publisher, "publish", "publisher"),
        dispose: captureMethod<AuthoringPublisher["dispose"]>(publisher, "dispose", "publisher"),
      });
    });
    const createDocumentIdentity = captureMethod<AuthoringShellWorkspace["createDocumentIdentity"]>(
      value, "createDocumentIdentity", "workspace",
    );
    let disposePromise: Promise<void> | null = null;
    return Object.freeze({
      session,
      preview,
      previewUrl: safePreviewUrl(dataField(value, "previewUrl", "workspace")),
      publishers: Object.freeze(publishers),
      createDocumentIdentity,
      dispose() {
        if (disposePromise !== null) return disposePromise;
        disposePromise = Promise.allSettled([
          preview.dispose(),
          ...publishers.map(async (publisher) => await publisher.dispose()),
        ]).then(() => undefined);
        return disposePromise;
      },
    });
  } catch {
    throw new TypeError("authoring workspace could not be inspected safely");
  }
}

function creationInput(form: HTMLFormElement, themes: readonly EditorThemeOption[]): CreateAuthoringShellWorkspaceInput {
  const values = new FormData(form);
  const title = safeText(String(values.get("title") ?? ""), MAX_TITLE_SCALARS, "site title");
  const themeId = String(values.get("themeId") ?? "");
  if (!themes.some((theme) => theme.id === themeId)) throw new TypeError("theme is invalid");
  return Object.freeze({ title, themeId });
}

function outcomeMessage(attempt: AuthoringPreviewAttempt): string {
  switch (attempt.outcome) {
    case "ready": return "Preview is ready.";
    case "cancelled": return "Preview was cancelled.";
    case "superseded": return "Preview was superseded by a newer revision.";
    case "failed": return attempt.diagnostics[0]?.message ?? "Preview could not be prepared.";
  }
}

function publicationMessage(attempt: AuthoringPublishAttempt): string {
  switch (attempt.outcome) {
    case "published": return "Site published.";
    case "cancelled": return "Publication was cancelled.";
    case "failed": return attempt.diagnostics[0]?.message ?? "Publication failed.";
    case "indeterminate": return attempt.diagnostics[0]?.message ?? "Publication state must be reconciled.";
  }
}

export function AuthoringShell(props: AuthoringShellProps): ReactNode {
  const host = useMemo(() => prepareHost(props.host), [props.host]);
  const [view, setView] = useState<ViewState>({ phase: "loading" });
  const [generation, setGeneration] = useState(0);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [targetId, setTargetId] = useState("");
  const [confirming, setConfirming] = useState(false);
  const actionAbort = useRef<AbortController | null>(null);
  const mounted = useRef(true);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      if (actionAbort.current !== null) APPLY(ABORT, actionAbort.current, []);
    };
  }, []);

  useEffect(() => {
    const abort = new AbortController();
    let live = true;
    setView({ phase: "loading" });
    void host.open(abort.signal).then((raw) => {
      if (raw === null) {
        if (live) setView({ phase: "first-run" });
        return;
      }
      const workspace = admitWorkspace(raw);
      if (!live) void workspace.dispose();
      else {
        setTargetId(workspace.publishers[0]!.review.targetId);
        setView({ phase: "ready", workspace });
      }
    }).catch(() => { if (live) setView({ phase: "error" }); });
    return () => { live = false; APPLY(ABORT, abort, []); };
  }, [host, generation]);

  const workspace = view.phase === "ready" ? view.workspace : null;
  useEffect(() => () => {
    if (workspace !== null) void workspace.dispose();
  }, [workspace]);

  const create = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    let input: CreateAuthoringShellWorkspaceInput;
    try { input = creationInput(event.currentTarget, host.themes); } catch {
      setMessage("Enter a valid site title and select a reviewed theme.");
      return;
    }
    setBusy(true);
    setMessage("");
    const abort = new AbortController();
    actionAbort.current = abort;
    void host.create(input, abort.signal).then((raw) => {
      const admitted = admitWorkspace(raw);
      if (!mounted.current || abort.signal.aborted) {
        void admitted.dispose();
        return;
      }
      setTargetId(admitted.publishers[0]!.review.targetId);
      setView({ phase: "ready", workspace: admitted });
    }).catch(() => {
      if (mounted.current) setMessage("The authoring workspace could not be created.");
    }).finally(() => {
      if (mounted.current) setBusy(false);
      if (actionAbort.current === abort) actionAbort.current = null;
    });
  };

  const runPreview = (): void => {
    if (workspace === null) return;
    setBusy(true);
    setMessage("");
    void workspace.preview.request(workspace.session)
      .then((attempt) => { if (mounted.current) setMessage(outcomeMessage(attempt)); })
      .catch(() => { if (mounted.current) setMessage("Preview could not be prepared."); })
      .finally(() => { if (mounted.current) setBusy(false); });
  };

  const selected = workspace?.publishers.find((publisher) => publisher.review.targetId === targetId)
    ?? workspace?.publishers[0] ?? null;
  const publish = (): void => {
    if (workspace === null || selected === null) return;
    setBusy(true);
    setMessage("");
    void selected.publish(workspace.session)
      .then((attempt) => { if (mounted.current) setMessage(publicationMessage(attempt)); })
      .catch(() => { if (mounted.current) setMessage("Publication failed before its commit point."); })
      .finally(() => {
        if (mounted.current) {
          setBusy(false);
          setConfirming(false);
        }
      });
  };

  return <main className="forme-authoring-shell">
    <style>{AUTHORING_SHELL_CSS}</style>
    {view.phase === "loading" && <p role="status">Opening your Forme workspace…</p>}
    {view.phase === "error" && <section>
      <h1>Forme could not start</h1>
      <p role="alert">The authoring workspace could not be opened.</p>
      <button type="button" onClick={() => setGeneration((value) => value + 1)}>Try again</button>
    </section>}
    {view.phase === "first-run" && <section>
      <h1>Create your Forme site</h1>
      <form onSubmit={create}>
        <fieldset disabled={busy}>
          <label>Site title<input name="title" required maxLength={MAX_TITLE_SCALARS * 2} /></label>
          <label>Theme<select name="themeId" defaultValue={host.themes[0]!.id}>
            {host.themes.map((theme) => <option key={theme.id} value={theme.id}>{theme.label}</option>)}
          </select></label>
          <button>Create site</button>
        </fieldset>
      </form>
    </section>}
    {workspace !== null && <>
      <header>
        <h1>Forme authoring</h1>
        <div className="forme-authoring-shell__actions">
          <button type="button" disabled={busy} onClick={runPreview}>Refresh preview</button>
          <button type="button" disabled={busy} onClick={() => setConfirming(true)}>Review publication</button>
        </div>
      </header>
      <div className="forme-authoring-shell__workspace">
        <AuthoringEditor
          session={workspace.session}
          themeOptions={host.themes}
          createDocumentIdentity={workspace.createDocumentIdentity}
        />
        <aside>
          <h2>Preview</h2>
          <iframe className="forme-authoring-shell__preview" title="Site preview" src={workspace.previewUrl} sandbox="allow-same-origin" />
          <h2>Publish</h2>
          <label>Publication target<select value={selected?.review.targetId ?? ""} disabled={busy} onChange={(event) => {
            setTargetId(event.currentTarget.value);
            setConfirming(false);
          }}>
            {workspace.publishers.map((publisher) => <option key={publisher.review.targetId} value={publisher.review.targetId}>
              {publisher.review.label}
            </option>)}
          </select></label>
          {selected !== null && <p>{selected.review.destination}</p>}
        </aside>
      </div>
      {confirming && selected !== null && <section role="group" aria-labelledby="publication-confirmation">
        <h2 id="publication-confirmation">Confirm publication</h2>
        <p>{selected.review.label}</p>
        <p>{selected.review.destination}</p>
        <div className="forme-authoring-shell__actions">
          <button type="button" disabled={busy} onClick={publish}>Publish to {selected.review.label}</button>
          <button type="button" disabled={busy} onClick={() => setConfirming(false)}>Cancel</button>
        </div>
      </section>}
    </>}
    <p role="status" aria-live="polite">{message}</p>
  </main>;
}

export const VERSION = "0.1.0";
