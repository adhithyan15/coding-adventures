/**
 * @coding-adventures/forme-authoring-core
 *
 * FM09's capability-free authoring state boundary: bounded project parsing,
 * deterministic persistence, semantic immutable edits, compare-and-swap
 * autosave, and restart-safe undo/redo. Hosts inject storage; this package
 * never opens a file, socket, environment variable, or subprocess.
 */

export { canonicalAuthoringProject, createAuthoringProject, validateAuthoringProject } from "./project.js";
export { openAuthoringSession } from "./session.js";
export { AuthoringError } from "./error.js";
export type { AuthoringErrorCode } from "./error.js";
export { HARD_AUTHORING_LIMITS } from "./types.js";
export type {
  AuthoringCommand,
  AuthoringDocument,
  AuthoringDocumentStatus,
  AuthoringLimitOverrides,
  AuthoringLimits,
  AuthoringProject,
  AuthoringSession,
  AuthoringSiteConfig,
  AuthoringStorage,
  CreateAuthoringProjectInput,
  OpenAuthoringSessionOptions,
  StoredAuthoringState,
} from "./types.js";
