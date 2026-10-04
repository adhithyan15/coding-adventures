import type { AuthoringProject, AuthoringSession } from "@coding-adventures/forme-authoring-core";
import type { PreviewSnapshot } from "@coding-adventures/forme-dev-server";
import type { Orchestrator, Pipeline } from "@coding-adventures/forme-orchestrator";

export interface AuthoringPreviewDiagnostic {
  readonly severity: "error";
  readonly code: string;
  readonly stageName: string;
  readonly instanceId: string;
  readonly message: string;
}

export type AuthoringPreviewOutcome = "ready" | "failed" | "cancelled" | "superseded";

export interface AuthoringPreviewAttempt {
  readonly outcome: AuthoringPreviewOutcome;
  readonly revision: string;
  readonly buildId: string | null;
  readonly diagnostics: readonly AuthoringPreviewDiagnostic[];
}

export interface AuthoringPreviewState {
  readonly phase: "idle" | "building" | "ready" | "failed" | "disposed";
  readonly activeRevision: string | null;
  readonly lastGoodRevision: string | null;
  readonly lastGoodBuildId: string | null;
  readonly diagnostics: readonly AuthoringPreviewDiagnostic[];
}

export interface AuthoringPreviewInput {
  readonly revision: string;
  readonly project: AuthoringProject;
}

export interface PreparedAuthoringPreview {
  readonly pipeline: Pipeline;
  release(): Promise<void>;
}

export interface AuthoringPreviewMaterializer {
  prepare(input: AuthoringPreviewInput, signal: AbortSignal): Promise<PreparedAuthoringPreview>;
}

export interface PublishedAuthoringPreview extends PreviewSnapshot {
  readonly revision: string;
}

export interface FailedAuthoringPreview {
  readonly revision: string;
  readonly diagnostics: readonly AuthoringPreviewDiagnostic[];
}

export interface AuthoringPreviewPublisher {
  publish(snapshot: PublishedAuthoringPreview): void | Promise<void>;
  publishFailure(failure: FailedAuthoringPreview): void | Promise<void>;
}

export interface CreateAuthoringPreviewOptions {
  readonly orchestrator: Pick<Orchestrator, "watch">;
  readonly materializer: AuthoringPreviewMaterializer;
  readonly publisher: AuthoringPreviewPublisher;
  readonly debounceMs?: number;
}

export interface AuthoringPreviewCoordinator {
  readonly state: AuthoringPreviewState;
  request(session: AuthoringSession): Promise<AuthoringPreviewAttempt>;
  dispose(): Promise<void>;
}

export function createAuthoringPreview(_options: CreateAuthoringPreviewOptions): AuthoringPreviewCoordinator {
  throw new Error("not implemented");
}
