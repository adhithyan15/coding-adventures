/**
 * @coding-adventures/forme-errors
 *
 * Forme kernel error model.  Three classes plus a frozen code table:
 *
 *   StageError           — typed error a stage throws.  Carries provenance
 *                          (stage, input, recoverable flag, structured
 *                          fields).  toJson() emits a stable structured
 *                          form for logs and telemetry.
 *
 *   CapabilityError      — subclass with code locked to "CAPABILITY_DENIED"
 *                          and recoverable forced false.  Carries the
 *                          offending capability string.
 *
 *   CancellationError    — propagated for cancellation; *not* a StageError
 *                          so the orchestrator's error boundary can let
 *                          it unwind without retry/fallback machinery.
 *
 *   ERROR_CODES          — frozen vocabulary of kernel-blessed codes.
 *                          Stages SHOULD use these when their failure
 *                          matches; otherwise they MAY define their own
 *                          under a `<package>/<code>` namespace.
 *
 * See FM01 §6 for the design rationale.
 */

export { ERROR_CODES } from "./codes.js";
export type { KernelErrorCode } from "./codes.js";

export { StageError } from "./stage-error.js";
export type { StageErrorInit } from "./stage-error.js";

export { CapabilityError } from "./capability-error.js";
export type { CapabilityErrorInit } from "./capability-error.js";

export { CancellationError, isCancellationError } from "./cancellation-error.js";
