/**
 * Public data contracts for FM09's UI-free authoring core.
 *
 * The core deliberately describes *what* changes, never *where* a project is
 * stored or *how* a control is rendered. That separation lets a Tauri host,
 * OPFS-backed web shell, and an in-memory test adapter share one correctness
 * boundary without granting the editor ambient capabilities.
 */

import type { DocumentNode } from "@coding-adventures/document-ast";

export interface AuthoringSiteConfig {
  readonly baseUrl: string | null;
  readonly themeId: string;
}

export interface AuthoringPublicationRecord {
  readonly authoringRevision: string;
  readonly manifestSha256: string;
  readonly targetId: string;
}

export interface AuthoringWorkflow {
  readonly lastPublication: AuthoringPublicationRecord | null;
}

export type AuthoringDocumentStatus = "draft" | "published";

export interface AuthoringDocument {
  readonly id: string;
  readonly slug: string;
  readonly title: string;
  readonly status: AuthoringDocumentStatus;
  readonly body: DocumentNode;
}

export interface AuthoringProject {
  readonly schemaVersion: 1;
  readonly projectId: string;
  readonly title: string;
  readonly site: AuthoringSiteConfig;
  readonly workflow: AuthoringWorkflow;
  readonly documents: readonly AuthoringDocument[];
  readonly activeDocumentId: string | null;
}

export interface CreateAuthoringProjectInput {
  readonly projectId: string;
  readonly title: string;
  readonly themeId?: string;
}

export interface AuthoringLimits {
  readonly maxJsonBytes: number;
  readonly maxDocuments: number;
  readonly maxDepth: number;
  readonly maxNodesPerDocument: number;
  readonly maxStringScalars: number;
  readonly maxTitleScalars: number;
  readonly maxSlugScalars: number;
  readonly maxUrlScalars: number;
  readonly maxHistoryEntries: number;
}

export type AuthoringLimitOverrides = Partial<AuthoringLimits>;

export const HARD_AUTHORING_LIMITS: Readonly<AuthoringLimits> = Object.freeze({
  maxJsonBytes: 8 * 1024 * 1024,
  maxDocuments: 1_000,
  maxDepth: 64,
  maxNodesPerDocument: 100_000,
  maxStringScalars: 1_048_576,
  maxTitleScalars: 512,
  maxSlugScalars: 2_048,
  maxUrlScalars: 2_048,
  maxHistoryEntries: 200,
});

export type AuthoringCommand =
  | {
      readonly type: "create-document";
      readonly document: AuthoringDocument;
      readonly activate: boolean;
    }
  | { readonly type: "remove-document"; readonly documentId: string }
  | {
      readonly type: "update-document-metadata";
      readonly documentId: string;
      readonly title: string;
      readonly slug: string;
      readonly status: AuthoringDocumentStatus;
    }
  | {
      readonly type: "replace-document-body";
      readonly documentId: string;
      readonly body: DocumentNode;
    }
  | {
      readonly type: "configure-site";
      readonly title: string;
      readonly baseUrl: string | null;
      readonly themeId: string;
    }
  | {
      readonly type: "set-active-document";
      readonly documentId: string | null;
    };

export interface AuthoringPublicationCommand {
  readonly type: "record-publication";
  readonly publication: AuthoringPublicationRecord;
}

/** Bytes and an opaque compare-and-swap token returned by a host adapter. */
export interface StoredAuthoringState {
  readonly bytes: Uint8Array;
  readonly revision: string;
}

export interface AuthoringStorage {
  load(signal?: AbortSignal): Promise<StoredAuthoringState | null>;
  /**
   * Rejection guarantees no publication. Cancellation is honored only before
   * the atomic commit point; after publication the adapter resolves with the
   * new revision even if the signal becomes aborted.
   */
  compareAndSwap(
    expectedRevision: string | null,
    bytes: Uint8Array,
    signal?: AbortSignal,
  ): Promise<{ readonly revision: string }>;
}

export interface OpenAuthoringSessionOptions {
  readonly storage: AuthoringStorage;
  readonly initialProject?: AuthoringProject;
  readonly historyLimit?: number;
  readonly limits?: AuthoringLimitOverrides;
  readonly signal?: AbortSignal;
}

export interface AuthoringSession {
  readonly project: AuthoringProject;
  readonly canUndo: boolean;
  readonly canRedo: boolean;
  readonly storageRevision: string;
  dispatch(command: AuthoringCommand, signal?: AbortSignal): Promise<void>;
  dispatchAtRevision(
    expectedRevision: string,
    command: AuthoringPublicationCommand,
    signal?: AbortSignal,
  ): Promise<void>;
  undo(signal?: AbortSignal): Promise<void>;
  redo(signal?: AbortSignal): Promise<void>;
}
