import {
  createAuthoringProject,
  openAuthoringSession,
  validateAuthoringRevision,
} from "@coding-adventures/forme-authoring-core";
import type {
  AuthoringSession,
  AuthoringStorage,
  StoredAuthoringState,
} from "@coding-adventures/forme-authoring-core";
import type {
  AuthoringShellHost,
  AuthoringShellWorkspace,
  CreateAuthoringShellWorkspaceInput,
} from "@coding-adventures/forme-authoring-shell";
import type {
  AuthoringPreviewAttempt,
  AuthoringPreviewCoordinator,
  AuthoringPreviewState,
} from "@coding-adventures/forme-authoring-preview";
import type {
  AuthoringPublisher,
  AuthoringPublishAttempt,
  AuthoringPublishState,
  AuthoringPublishTargetReview,
} from "@coding-adventures/forme-authoring-publish";

export interface DesktopBridge {
  readonly previewUrl: string;
  invoke<T>(command: string, arguments_?: unknown): Promise<T>;
}

const NO_PRELOAD = Symbol("no preload");
const IDLE_PREVIEW_STATE: AuthoringPreviewState = Object.freeze({
  phase: "idle",
  activeRevision: null,
  lastGoodRevision: null,
  lastGoodBuildId: null,
  diagnostics: Object.freeze([]),
});

export function createDesktopHost(bridge: DesktopBridge): AuthoringShellHost {
  const themes = Object.freeze([Object.freeze({ id: "forme-classless", label: "Forme Classless" })]);
  return Object.freeze({
    themes,
    async open(signal: AbortSignal): Promise<AuthoringShellWorkspace | null> {
      abortIfNeeded(signal);
      const probe = new NativeAuthoringStorage(bridge);
      const loaded = await probe.load(signal);
      if (loaded === null) return null;
      const session = await openAuthoringSession({
        storage: new NativeAuthoringStorage(bridge, loaded),
        signal,
      });
      return await desktopWorkspace(bridge, session, signal);
    },
    async create(
      input: CreateAuthoringShellWorkspaceInput,
      signal: AbortSignal,
    ): Promise<AuthoringShellWorkspace> {
      abortIfNeeded(signal);
      const projectId = await createIdentity(bridge);
      const documentId = await createIdentity(bridge);
      const initialProject = createAuthoringProject({
        projectId,
        title: input.title,
        themeId: input.themeId,
      });
      const session = await openAuthoringSession({
        storage: new NativeAuthoringStorage(bridge),
        initialProject,
        signal,
      });
      await session.dispatch({
        type: "create-document",
        document: {
          id: documentId,
          slug: "welcome",
          title: "Welcome",
          status: "draft",
          body: {
            type: "document",
            children: [{
              type: "paragraph",
              children: [{ type: "text", value: "Welcome to your new Forme site." }],
            }],
          },
        },
        activate: true,
      }, signal);
      return await desktopWorkspace(bridge, session, signal);
    },
  });
}

class NativeAuthoringStorage implements AuthoringStorage {
  readonly #bridge: DesktopBridge;
  #preloaded: StoredAuthoringState | null | typeof NO_PRELOAD;

  constructor(
    bridge: DesktopBridge,
    preloaded: StoredAuthoringState | null | typeof NO_PRELOAD = NO_PRELOAD,
  ) {
    this.#bridge = bridge;
    this.#preloaded = preloaded;
  }

  async load(signal?: AbortSignal): Promise<StoredAuthoringState | null> {
    abortIfNeeded(signal);
    if (this.#preloaded !== NO_PRELOAD) {
      const loaded = this.#preloaded;
      this.#preloaded = NO_PRELOAD;
      return loaded;
    }
    const raw = await this.#bridge.invoke<unknown>("project_load");
    abortIfNeeded(signal);
    if (raw === null) return null;
    const response = exactRecord(raw, ["bytesBase64", "revision"], "project load response");
    return Object.freeze({
      bytes: decodeBase64(response.bytesBase64),
      revision: validateAuthoringRevision(response.revision),
    });
  }

  async compareAndSwap(
    expectedRevision: string | null,
    bytes: Uint8Array,
    signal?: AbortSignal,
  ): Promise<{ readonly revision: string }> {
    abortIfNeeded(signal);
    const raw = await this.#bridge.invoke<unknown>("project_compare_and_swap", {
      request: {
        expectedRevision,
        bytesBase64: encodeBase64(bytes),
      },
    });
    const response = exactRecord(raw, ["revision"], "project save response");
    return Object.freeze({ revision: validateAuthoringRevision(response.revision) });
  }
}

async function desktopWorkspace(
  bridge: DesktopBridge,
  session: AuthoringSession,
  signal: AbortSignal,
): Promise<AuthoringShellWorkspace> {
  let disposed = false;
  const targets = await reviewedTargets(bridge, signal);
  const preview: AuthoringPreviewCoordinator = Object.freeze({
    state: IDLE_PREVIEW_STATE,
    async request(requested: AuthoringSession): Promise<AuthoringPreviewAttempt> {
      if (disposed || requested !== session) throw new Error("the desktop workspace is unavailable");
      const raw = await bridge.invoke<unknown>("preview_build", {
        request: {
          revision: requested.storageRevision,
        },
      });
      const response = exactRecord(
        raw,
        ["outcome", "revision", "buildId", "diagnostics"],
        "preview response",
      );
      if (response.outcome !== "ready" || !Array.isArray(response.diagnostics) || response.diagnostics.length !== 0) {
        throw new TypeError("preview response is invalid");
      }
      const buildId = validateAuthoringRevision(response.buildId);
      const revision = validateAuthoringRevision(response.revision);
      return Object.freeze({ outcome: "ready", revision, buildId, diagnostics: Object.freeze([]) });
    },
    async dispose(): Promise<void> {},
  });
  const publishers = targets.map((target) => nativePublisher(bridge, session, target, () => disposed));
  return Object.freeze({
    session,
    preview,
    previewUrl: bridge.previewUrl,
    publishers: Object.freeze(publishers),
    async createDocumentIdentity(): Promise<string> {
      if (disposed) throw new Error("the desktop workspace is unavailable");
      return createIdentity(bridge);
    },
    async dispose(): Promise<void> {
      if (disposed) return;
      disposed = true;
      await Promise.all(publishers.map(async (publisher) => await publisher.dispose()));
      await bridge.invoke("workspace_dispose");
    },
  });
}

async function reviewedTargets(
  bridge: DesktopBridge,
  signal: AbortSignal,
): Promise<readonly AuthoringPublishTargetReview[]> {
  abortIfNeeded(signal);
  const listed = await bridge.invoke<unknown>("target_list");
  abortIfNeeded(signal);
  if (!Array.isArray(listed)) throw new TypeError("native target list is invalid");
  const values = listed.length > 0 ? listed : [await bridge.invoke<unknown>("target_configure")];
  abortIfNeeded(signal);
  const seen = new Set<string>();
  return Object.freeze(values.map((value) => {
    const record = exactRecord(value, ["targetId", "label", "destination"], "target review");
    if (typeof record.targetId !== "string"
      || !/^[a-z0-9]+(?:[._-][a-z0-9]+)*$/.test(record.targetId)
      || typeof record.label !== "string"
      || record.label.length === 0
      || typeof record.destination !== "string"
      || record.destination.length === 0
      || seen.has(record.targetId)) {
      throw new TypeError("target review is invalid");
    }
    seen.add(record.targetId);
    return Object.freeze({
      targetId: record.targetId,
      label: record.label,
      destination: record.destination,
    });
  }));
}

function nativePublisher(
  bridge: DesktopBridge,
  workspaceSession: AuthoringSession,
  target: AuthoringPublishTargetReview,
  isDisposed: () => boolean,
): AuthoringPublisher {
  let currentState: AuthoringPublishState = publishState("idle", null, null);
  let active = false;
  let disposed = false;
  let poisoned = false;
  let activeSettlement: Promise<void> | null = null;
  return Object.freeze({
    target,
    get state(): AuthoringPublishState { return currentState; },
    async publish(requested: AuthoringSession, signal?: AbortSignal): Promise<AuthoringPublishAttempt> {
      if (disposed || isDisposed() || requested !== workspaceSession) {
        throw new Error("the desktop publisher is unavailable");
      }
      if (poisoned) {
        return publishAttempt(
          "indeterminate", requested.storageRevision, null, target.targetId, "E_PUBLISH_RECONCILE",
        );
      }
      if (active) return publishAttempt("failed", requested.storageRevision, null, target.targetId, "E_PUBLISH_BUSY");
      abortIfNeeded(signal);
      active = true;
      let settleActive!: () => void;
      activeSettlement = new Promise<void>((resolve) => { settleActive = resolve; });
      const revision = requested.storageRevision;
      currentState = publishState("building", revision, null);
      try {
        const raw = await bridge.invoke<unknown>("target_publish", {
          request: { targetId: target.targetId, revision },
        });
        if (disposed || isDisposed()) {
          poisoned = true;
          currentState = publishState("indeterminate", revision, null);
          return publishAttempt(
            "indeterminate", revision, null, target.targetId, "E_PUBLISH_RECONCILE",
          );
        }
        const response = exactRecord(
          raw,
          ["outcome", "revision", "manifestSha256", "targetId", "diagnostics"],
          "publication response",
        );
        if (response.revision !== revision
          || response.targetId !== target.targetId
          || !Array.isArray(response.diagnostics)) {
          throw new TypeError("publication response is invalid");
        }
        const diagnostics = response.diagnostics.map((value) => {
          const item = exactRecord(value, ["severity", "code", "message"], "publication diagnostic");
          if (item.severity !== "error" || typeof item.code !== "string" || typeof item.message !== "string") {
            throw new TypeError("publication diagnostic is invalid");
          }
          return Object.freeze({ severity: "error" as const, code: item.code, message: item.message });
        });
        if (diagnostics.length > 1) throw new TypeError("publication diagnostics are invalid");
        if (response.outcome === "failed") {
          if (response.manifestSha256 !== null || diagnostics.length !== 1) {
            throw new TypeError("failed publication response is invalid");
          }
          currentState = Object.freeze({
            phase: "failed", revision, manifestSha256: null, diagnostics: Object.freeze(diagnostics),
          });
          return Object.freeze({
            outcome: "failed", revision, manifestSha256: null, targetId: target.targetId,
            diagnostics: Object.freeze(diagnostics),
          });
        }
        if (response.outcome === "indeterminate") {
          poisoned = true;
          if (response.manifestSha256 !== null && !validSha256Base64(response.manifestSha256)) {
            throw new TypeError("indeterminate publication response is invalid");
          }
          if (diagnostics.length !== 1) throw new TypeError("indeterminate publication response is invalid");
          currentState = Object.freeze({
            phase: "indeterminate", revision, manifestSha256: response.manifestSha256,
            diagnostics: Object.freeze(diagnostics),
          });
          return Object.freeze({
            outcome: "indeterminate", revision, manifestSha256: response.manifestSha256,
            targetId: target.targetId, diagnostics: Object.freeze(diagnostics),
          });
        }
        if (response.outcome !== "published"
          || !validSha256Base64(response.manifestSha256)
          || diagnostics.length !== 0) {
          throw new TypeError("published publication response is invalid");
        }
        const digest = response.manifestSha256;
        currentState = publishState("recording", revision, digest);
        try {
          await requested.dispatchAtRevision(revision, {
            type: "record-publication",
            publication: { authoringRevision: revision, manifestSha256: digest, targetId: target.targetId },
          }, signal);
        } catch {
          poisoned = true;
          currentState = publishState("indeterminate", revision, digest);
          return publishAttempt("indeterminate", revision, digest, target.targetId, "E_PUBLISH_RECORD");
        }
        currentState = publishState("published", revision, digest);
        return publishAttempt("published", revision, digest, target.targetId);
      } catch {
        poisoned = true;
        currentState = publishState("indeterminate", revision, null);
        return publishAttempt("indeterminate", revision, null, target.targetId, "E_PUBLISH_RECONCILE");
      } finally {
        active = false;
        settleActive();
        activeSettlement = null;
      }
    },
    async dispose(): Promise<void> {
      disposed = true;
      await activeSettlement;
      currentState = publishState("disposed", null, null);
    },
  });
}

function publishState(
  phase: AuthoringPublishState["phase"],
  revision: string | null,
  manifestSha256: string | null,
): AuthoringPublishState {
  return Object.freeze({ phase, revision, manifestSha256, diagnostics: Object.freeze([]) });
}

function publishAttempt(
  outcome: AuthoringPublishAttempt["outcome"],
  revision: string,
  manifestSha256: string | null,
  targetId: string,
  code?: string,
): AuthoringPublishAttempt {
  const diagnostics = code === undefined ? Object.freeze([]) : Object.freeze([Object.freeze({
    severity: "error" as const,
    code,
    message: "Publication state must be reconciled before retrying.",
  })]);
  return Object.freeze({ outcome, revision, manifestSha256, targetId, diagnostics });
}

function validSha256Base64(value: unknown): value is string {
  if (typeof value !== "string" || value.length !== 44) return false;
  try { return atob(value).length === 32; } catch { return false; }
}

async function createIdentity(bridge: DesktopBridge): Promise<string> {
  const value = await bridge.invoke<unknown>("identity_create");
  if (typeof value !== "string" || !/^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(value)) {
    throw new TypeError("native identity is invalid");
  }
  return value;
}

function exactRecord(
  value: unknown,
  expectedKeys: readonly string[],
  label: string,
): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} is invalid`);
  }
  const keys = Object.keys(value).sort();
  const expected = [...expectedKeys].sort();
  if (keys.length !== expected.length || keys.some((key, index) => key !== expected[index])) {
    throw new TypeError(`${label} is invalid`);
  }
  return value as Record<string, unknown>;
}

function encodeBase64(bytes: Uint8Array): string {
  let binary = "";
  for (let offset = 0; offset < bytes.length; offset += 16_384) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + 16_384));
  }
  return btoa(binary);
}

function decodeBase64(value: unknown): Uint8Array {
  if (typeof value !== "string" || value.length === 0 || value.length > 11_184_812) {
    throw new TypeError("project bytes are invalid");
  }
  let binary: string;
  try { binary = atob(value); } catch { throw new TypeError("project bytes are invalid"); }
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) bytes[index] = binary.charCodeAt(index);
  return bytes;
}

function abortIfNeeded(signal?: AbortSignal): void {
  if (signal?.aborted === true) throw signal.reason ?? new DOMException("Aborted", "AbortError");
}
