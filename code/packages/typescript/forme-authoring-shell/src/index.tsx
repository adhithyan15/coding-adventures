/** Capability-free first-run composition for FM09 authoring. */

import { useEffect, useMemo, useRef, useState } from "react";
import type { FormEvent, ReactNode } from "react";
import {
  validateAuthoringManifestSha256,
  validateAuthoringRevision,
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
  dispose(): Promise<void>;
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
}

interface SafePublisher {
  readonly review: AuthoringPublishTargetReview;
  publish(session: AuthoringSession, signal?: AbortSignal): Promise<AuthoringPublishAttempt>;
}

interface SafeWorkspace {
  readonly session: AuthoringSession;
  readonly preview: SafePreview;
  readonly previewUrl: string;
  readonly publishers: readonly SafePublisher[];
  readonly poisoned: Promise<void>;
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
  | { readonly phase: "poisoned" }
  | { readonly phase: "ready"; readonly workspace: SafeWorkspace };

interface ActionToken {
  readonly abort: AbortController;
  readonly workspace: SafeWorkspace | null;
}

class WorkspacePoisonedError extends Error {}

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

/** A callback handed to the editor must not receive the workspace as `this`. */
function captureOwnCallback<T extends Function>(value: unknown, name: string, label: string): T {
  if (value === null || typeof value !== "object") throw new TypeError(`${label} is invalid`);
  const descriptor = Object.getOwnPropertyDescriptor(value, name);
  if (descriptor === undefined || !("value" in descriptor) || typeof descriptor.value !== "function") {
    throw new TypeError(`${label} ${name} is invalid`);
  }
  const callback = descriptor.value as T;
  return ((...args: unknown[]) => APPLY(callback, undefined, args)) as unknown as T;
}

function captureReader(value: unknown, name: string, label: string): () => unknown {
  if (value === null || (typeof value !== "object" && typeof value !== "function")) throw new TypeError(`${label} is invalid`);
  let cursor: object | null = value as object;
  for (let depth = 0; cursor !== null && depth < 16; depth += 1) {
    const descriptor = Object.getOwnPropertyDescriptor(cursor, name);
    if (descriptor !== undefined) {
      if ("value" in descriptor) {
        const captured = descriptor.value;
        return () => captured;
      }
      if (typeof descriptor.get !== "function") throw new TypeError(`${label} ${name} is invalid`);
      const getter = descriptor.get;
      return () => APPLY(getter, value, []);
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

function denseArray(value: unknown, maximum: number, label: string): readonly unknown[] {
  let array: boolean;
  try { array = Array.isArray(value); } catch { throw new TypeError(`${label} is invalid`); }
  if (!array || value === null || typeof value !== "object") throw new TypeError(`${label} is invalid`);
  let lengthDescriptor: PropertyDescriptor | undefined;
  try { lengthDescriptor = Object.getOwnPropertyDescriptor(value, "length"); } catch {
    throw new TypeError(`${label} is invalid`);
  }
  if (lengthDescriptor === undefined || !("value" in lengthDescriptor)
    || !Number.isSafeInteger(lengthDescriptor.value) || lengthDescriptor.value < 0
    || lengthDescriptor.value > maximum) throw new TypeError(`${label} is invalid`);
  const length = lengthDescriptor.value as number;
  const result: unknown[] = [];
  for (let index = 0; index < length; index += 1) {
    let descriptor: PropertyDescriptor | undefined;
    try { descriptor = Object.getOwnPropertyDescriptor(value, String(index)); } catch {
      throw new TypeError(`${label} is invalid`);
    }
    if (descriptor === undefined || !("value" in descriptor) || !descriptor.enumerable) {
      throw new TypeError(`${label} is sparse or accessor-backed`);
    }
    result.push(descriptor.value);
  }
  return Object.freeze(result);
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

interface SessionSnapshot {
  readonly project: AuthoringSession["project"];
  readonly canUndo: boolean;
  readonly canRedo: boolean;
  readonly storageRevision: string;
}

/**
 * The editor and coordinators share this narrow facade. A raw host session is
 * never retained in React props: its state is sampled once at admission and
 * again only after a captured mutation settles.
 */
function admitSession(value: unknown): { readonly session: AuthoringSession; readonly poisoned: Promise<void> } {
  const readProject = captureReader(value, "project", "authoring session");
  const readCanUndo = captureReader(value, "canUndo", "authoring session");
  const readCanRedo = captureReader(value, "canRedo", "authoring session");
  const readRevision = captureReader(value, "storageRevision", "authoring session");
  const dispatch = captureMethod<AuthoringSession["dispatch"]>(value, "dispatch", "authoring session");
  const dispatchAtRevision = captureMethod<AuthoringSession["dispatchAtRevision"]>(value, "dispatchAtRevision", "authoring session");
  const undo = captureMethod<AuthoringSession["undo"]>(value, "undo", "authoring session");
  const redo = captureMethod<AuthoringSession["redo"]>(value, "redo", "authoring session");
  const snapshot = (): SessionSnapshot => {
    const canUndo = readCanUndo();
    const canRedo = readCanRedo();
    if (typeof canUndo !== "boolean" || typeof canRedo !== "boolean") throw new TypeError("authoring session flags are invalid");
    return Object.freeze({
      project: validateAuthoringProject(readProject()),
      canUndo,
      canRedo,
      storageRevision: validateAuthoringRevision(readRevision()),
    });
  };
  let current = snapshot();
  let poison!: () => void;
  const poisonedPromise = new Promise<void>((resolve) => { poison = resolve; });
  let isPoisoned = false;
  const mutate = async (operation: () => Promise<void>): Promise<void> => {
    if (isPoisoned) throw new WorkspacePoisonedError("authoring workspace requires reload");
    await Promise.resolve().then(operation);
    try { current = snapshot(); } catch {
      isPoisoned = true;
      poison();
      throw new WorkspacePoisonedError("authoring workspace state is indeterminate");
    }
  };
  const facade: AuthoringSession = {
    get project() { return current.project; },
    get canUndo() { return current.canUndo; },
    get canRedo() { return current.canRedo; },
    get storageRevision() { return current.storageRevision; },
    dispatch: (command, signal) => mutate(async () => await dispatch(command, signal)),
    dispatchAtRevision: (revision, command, signal) => mutate(async () => await dispatchAtRevision(revision, command, signal)),
    undo: (signal) => mutate(async () => await undo(signal)),
    redo: (signal) => mutate(async () => await redo(signal)),
  };
  return Object.freeze({ session: Object.freeze(facade), poisoned: poisonedPromise });
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

async function admitWorkspace(value: AuthoringShellWorkspace): Promise<SafeWorkspace> {
  let retirement: (() => Promise<void>) | null = null;
  try {
    const dispose = captureOwnCallback<AuthoringShellWorkspace["dispose"]>(value, "dispose", "workspace");
    let retirementPromise: Promise<void> | null = null;
    retirement = () => {
      if (retirementPromise !== null) return retirementPromise;
      retirementPromise = Promise.resolve().then(async () => await dispose());
      return retirementPromise;
    };
    const admittedSession = admitSession(dataField(value, "session", "workspace"));
    const session = admittedSession.session;
    const project = session.project;
    const active = project.documents.find((document) => document.id === project.activeDocumentId);
    if (active === undefined || active.status !== "draft") throw new TypeError("workspace must contain an active draft");
    const rawPreview = dataField(value, "preview", "workspace");
    const preview: SafePreview = Object.freeze({
      request: captureMethod<AuthoringPreviewCoordinator["request"]>(rawPreview, "request", "preview"),
    });
    const rawPublishers = denseArray(dataField(value, "publishers", "workspace"), MAX_TARGETS, "workspace publishers");
    if (rawPublishers.length < 1) {
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
      });
    });
    const createDocumentIdentity = captureOwnCallback<AuthoringShellWorkspace["createDocumentIdentity"]>(
      value, "createDocumentIdentity", "workspace",
    );
    return Object.freeze({
      session,
      preview,
      previewUrl: safePreviewUrl(dataField(value, "previewUrl", "workspace")),
      publishers: Object.freeze(publishers),
      poisoned: admittedSession.poisoned,
      createDocumentIdentity,
      dispose: retirement,
    });
  } catch {
    if (retirement !== null) {
      try { await retirement(); } catch { throw new WorkspacePoisonedError("workspace retirement did not complete"); }
    } else if (value !== null && (typeof value === "object" || typeof value === "function")) {
      throw new WorkspacePoisonedError("workspace retirement is unavailable");
    }
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

function diagnosticMessage(value: unknown, preview: boolean): string {
  const diagnostics = denseArray(value, 64, "action diagnostics");
  if (diagnostics.length === 0) return "";
  const diagnostic = diagnostics[0];
  if (dataField(diagnostic, "severity", "action diagnostic") !== "error") throw new TypeError("action diagnostic severity is invalid");
  safeText(dataField(diagnostic, "code", "action diagnostic"), 128, "diagnostic code");
  if (preview) {
    safeText(dataField(diagnostic, "stageName", "preview diagnostic"), 256, "diagnostic stage");
    safeText(dataField(diagnostic, "instanceId", "preview diagnostic"), 256, "diagnostic instance");
  }
  return safeText(dataField(diagnostic, "message", "action diagnostic"), 2_048, "diagnostic message");
}

function outcomeMessage(value: unknown, expectedRevision: string): string {
  const outcome = dataField(value, "outcome", "preview attempt");
  if (validateAuthoringRevision(dataField(value, "revision", "preview attempt")) !== expectedRevision) {
    throw new TypeError("preview attempt revision is stale");
  }
  const buildId = dataField(value, "buildId", "preview attempt");
  if (buildId !== null) safeText(buildId, 1_024, "preview build identity");
  if ((outcome === "ready") !== (buildId !== null)) throw new TypeError("preview build identity does not match its outcome");
  const diagnostic = diagnosticMessage(dataField(value, "diagnostics", "preview attempt"), true);
  switch (outcome) {
    case "ready": return "Preview is ready.";
    case "cancelled": return "Preview was cancelled.";
    case "superseded": return "Preview was superseded by a newer revision.";
    case "failed": return diagnostic || "Preview could not be prepared.";
    default: throw new TypeError("preview attempt outcome is invalid");
  }
}

function publicationMessage(value: unknown, expectedRevision: string, expectedTargetId: string): string {
  const outcome = dataField(value, "outcome", "publication attempt");
  if (validateAuthoringRevision(dataField(value, "revision", "publication attempt")) !== expectedRevision) {
    throw new TypeError("publication attempt revision is stale");
  }
  if (dataField(value, "targetId", "publication attempt") !== expectedTargetId) {
    throw new TypeError("publication attempt target is stale");
  }
  const manifestSha256 = dataField(value, "manifestSha256", "publication attempt");
  if (manifestSha256 !== null) validateAuthoringManifestSha256(manifestSha256);
  if (outcome === "published" && manifestSha256 === null) throw new TypeError("published attempt is missing its digest");
  const diagnostic = diagnosticMessage(dataField(value, "diagnostics", "publication attempt"), false);
  switch (outcome) {
    case "published": return "Site published.";
    case "cancelled": return "Publication was cancelled.";
    case "failed": return diagnostic || "Publication failed.";
    case "indeterminate": return diagnostic || "Publication state must be reconciled.";
    default: throw new TypeError("publication attempt outcome is invalid");
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
  const action = useRef<ActionToken | null>(null);
  const workspaceRef = useRef<SafeWorkspace | null>(null);
  const mounted = useRef(true);

  const enterPoisonedState = (): void => {
    if (!mounted.current) return;
    const active = workspaceRef.current;
    workspaceRef.current = null;
    if (action.current !== null) APPLY(ABORT, action.current.abort, []);
    action.current = null;
    setBusy(false);
    setConfirming(false);
    setMessage("");
    setView({ phase: "poisoned" });
    if (active !== null) void active.dispose().catch(() => undefined);
  };

  const activateWorkspace = (workspace: SafeWorkspace): void => {
    workspaceRef.current = workspace;
    void workspace.poisoned.then(() => {
      if (!mounted.current || workspaceRef.current !== workspace) return;
      enterPoisonedState();
    });
    setTargetId(workspace.publishers[0]!.review.targetId);
    setView({ phase: "ready", workspace });
  };

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      if (action.current !== null) APPLY(ABORT, action.current.abort, []);
      action.current = null;
      const workspace = workspaceRef.current;
      workspaceRef.current = null;
      if (workspace !== null) void workspace.dispose().catch(() => undefined);
    };
  }, []);

  useEffect(() => {
    const abort = new AbortController();
    let live = true;
    if (action.current !== null) APPLY(ABORT, action.current.abort, []);
    action.current = null;
    setBusy(false);
    setConfirming(false);
    setMessage("");
    setView({ phase: "loading" });
    const previous = workspaceRef.current;
    workspaceRef.current = null;
    void (async () => {
      try {
        if (previous !== null) {
          try { await previous.dispose(); } catch { throw new WorkspacePoisonedError("workspace retirement did not complete"); }
        }
        if (!live) return;
        const raw = await Promise.resolve().then(async () => await host.open(abort.signal));
        if (raw === null) {
          if (live) setView({ phase: "first-run" });
          return;
        }
        const workspace = await admitWorkspace(raw);
        if (!live) {
          try { await workspace.dispose(); } catch { enterPoisonedState(); }
          return;
        }
        activateWorkspace(workspace);
      } catch (error) {
        if (error instanceof WorkspacePoisonedError) enterPoisonedState();
        else if (live) setView({ phase: "error" });
      }
    })();
    return () => {
      live = false;
      APPLY(ABORT, abort, []);
      if (action.current !== null) APPLY(ABORT, action.current.abort, []);
      action.current = null;
    };
  }, [host, generation]);

  const workspace = view.phase === "ready" ? view.workspace : null;

  const create = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    let input: CreateAuthoringShellWorkspaceInput;
    try { input = creationInput(event.currentTarget, host.themes); } catch {
      setMessage("Enter a valid site title and select a reviewed theme.");
      return;
    }
    if (action.current !== null) return;
    const token: ActionToken = { abort: new AbortController(), workspace: null };
    action.current = token;
    setBusy(true);
    setMessage("");
    void Promise.resolve().then(async () => await host.create(input, token.abort.signal))
      .then(async (raw) => await admitWorkspace(raw)).then(async (admitted) => {
      if (!mounted.current || action.current !== token || token.abort.signal.aborted) {
        try { await admitted.dispose(); } catch {
          enterPoisonedState();
        }
        return;
      }
      activateWorkspace(admitted);
    }).catch((error: unknown) => {
      if (error instanceof WorkspacePoisonedError) enterPoisonedState();
      else if (mounted.current && action.current === token) setMessage("The authoring workspace could not be created.");
    }).finally(() => {
      if (action.current === token) {
        action.current = null;
        if (mounted.current) setBusy(false);
      }
    });
  };

  const runPreview = (): void => {
    if (workspace === null || action.current !== null) return;
    const token: ActionToken = { abort: new AbortController(), workspace };
    action.current = token;
    const revision = workspace.session.storageRevision;
    setBusy(true);
    setMessage("");
    void Promise.resolve().then(async () => await workspace.preview.request(workspace.session))
      .then((attempt) => {
        const nextMessage = outcomeMessage(attempt, revision);
        if (mounted.current && action.current === token) setMessage(nextMessage);
      })
      .catch(() => {
        if (mounted.current && action.current === token) setMessage("Preview could not be prepared.");
      }).finally(() => {
        if (action.current === token) {
          action.current = null;
          if (mounted.current) setBusy(false);
        }
      });
  };

  const selected = workspace?.publishers.find((publisher) => publisher.review.targetId === targetId)
    ?? workspace?.publishers[0] ?? null;
  const publish = (): void => {
    if (workspace === null || selected === null || action.current !== null) return;
    const token: ActionToken = { abort: new AbortController(), workspace };
    action.current = token;
    const revision = workspace.session.storageRevision;
    const selectedTargetId = selected.review.targetId;
    setBusy(true);
    setMessage("");
    void Promise.resolve().then(async () => await selected.publish(workspace.session, token.abort.signal))
      .then((attempt) => {
        const nextMessage = publicationMessage(attempt, revision, selectedTargetId);
        if (mounted.current && action.current === token) setMessage(nextMessage);
      })
      .catch(() => {
        if (mounted.current && action.current === token) setMessage("Publication state must be reconciled.");
      })
      .finally(() => {
        if (action.current === token) {
          action.current = null;
          if (mounted.current) {
            setBusy(false);
            setConfirming(false);
          }
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
    {view.phase === "poisoned" && <section>
      <h1>Forme must be reloaded</h1>
      <p role="alert">Workspace state could not be retired or verified safely. Reload before continuing.</p>
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
