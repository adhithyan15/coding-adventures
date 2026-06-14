/**
 * CapabilityError — thrown when a stage attempts a capability-gated
 * operation it has no declaration for.
 *
 * Subclass of `StageError` with two contracts pinned by FM01 §6.2:
 *
 *   1. `code` is forced to `"CAPABILITY_DENIED"`.  Even if the caller
 *      supplies a different code in the init it is overwritten — a
 *      capability error is always identifiable by its code alone.
 *   2. `recoverable` is forced to `false`.  Best-effort mode does not
 *      soften capability errors — they indicate a misconfigured plugin
 *      manifest, not a runtime hiccup, and silently continuing past
 *      one is a security smell.  Same reasoning the FM01 §9.4
 *      orchestrator-handling section pins.
 *
 * The `capability` field carries the offending capability string so
 * the install/error UX can offer "this stage needs <capability>;
 * grant it?" affordances.
 */

import { StageError, type StageErrorInit } from "./stage-error.js";
import { ERROR_CODES } from "./codes.js";

/**
 * Initialiser for `CapabilityError`.  Inherits every field from
 * `StageErrorInit` plus the required `capability`.  The supplied
 * `code` and `recoverable` are ignored (always `"CAPABILITY_DENIED"`
 * and `false` respectively).
 */
export interface CapabilityErrorInit
  extends Omit<StageErrorInit, "code" | "recoverable"> {
  /** The capability string the stage attempted but had not declared. */
  readonly capability: string;
}

export class CapabilityError extends StageError {
  /** The capability string that was denied. */
  readonly capability: string;

  constructor(init: CapabilityErrorInit) {
    super({
      ...init,
      code:        ERROR_CODES.CAPABILITY_DENIED,
      recoverable: false,
    });
    this.capability = init.capability;
  }

  override toJson() {
    const base = super.toJson() as Record<string, unknown>;
    return { ...base, capability: this.capability } as ReturnType<StageError["toJson"]>;
  }
}
