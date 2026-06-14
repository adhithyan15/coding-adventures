/**
 * StageError — the primary typed error a stage throws.
 *
 * Per FM01 §6.1, every stage error carries enough provenance for the
 * orchestrator and downstream drivers (CLI, dev server, editor) to
 * surface useful diagnostics:
 *
 *   - `code`         — machine-readable label (one of `ERROR_CODES`
 *                      or a stage-defined `<package>/<code>` string)
 *   - `message`      — human-readable explanation
 *   - `inputPath`    — file/route the error happened *for*
 *   - `inputId`      — LogicalId of the offending input, if known
 *   - `stageName`    — name of the stage that threw
 *   - `cause`        — the original throw, when wrapping a non-StageError
 *   - `recoverable`  — orchestrator may continue past this in
 *                      best-effort mode; non-recoverable always halts
 *                      the affected branch
 *   - `fields`       — open-ended structured context the stage chooses
 *                      to attach (line number, regex, parse position…)
 *
 * Optional fields default to `null` (or `false`/`{}`) on the instance —
 * never `undefined` — so consumers can read them without optional-chain
 * dance.  `cause` is the exception: it stays `undefined` when absent so
 * callers can distinguish "no cause" from "cause was the literal null."
 *
 * `toJson()` emits a stable structured representation suitable for
 * structured logs, telemetry events, and editor IPC.  The `cause` is
 * coerced to its `String()` form because arbitrary thrown values are
 * not generally JSON-serialisable; keeping richer data is a stage-side
 * choice via `fields`.
 *
 * === Why a class, not a discriminated union? ===
 *
 * Two reasons.  First, `instanceof StageError` is the cheap, idiomatic
 * way for the orchestrator's error boundary to discriminate "this is a
 * stage's typed error" from "this is something else I need to wrap."
 * Second, the class participates in the JS Error chain — uncaught
 * StageErrors print the stage name and code before any framework code
 * even runs, which is the right default behaviour at the edge of a
 * pipeline run.
 */

import type { JsonValue, LogicalId } from "@coding-adventures/forme-types";

/**
 * Initialiser for `StageError`.  Required: `code`, `message`.  Everything
 * else is optional and defaults sensibly on the instance.
 */
export interface StageErrorInit {
  readonly code: string;
  readonly message: string;
  readonly inputPath?: string;
  readonly inputId?: LogicalId;
  readonly stageName?: string;
  readonly cause?: unknown;
  readonly recoverable?: boolean;
  readonly fields?: Readonly<Record<string, JsonValue>>;
}

/**
 * Typed error thrown by stages.  Subclass `StageError` to add a
 * structured field (see `CapabilityError` for the canonical example).
 */
export class StageError extends Error {
  /** Machine-readable error code.  See `ERROR_CODES`. */
  readonly code: string;

  /** Source path of the offending input, if known.  Null otherwise. */
  readonly inputPath: string | null;

  /** Logical identity of the offending input, if known.  Null otherwise. */
  readonly inputId: LogicalId | null;

  /** Name of the stage that threw, populated by the orchestrator if absent. */
  readonly stageName: string | null;

  /**
   * The original thrown value when this StageError wraps a
   * non-StageError.  `undefined` when there is no underlying cause —
   * which is distinct from "cause was the literal null."
   */
  // Override the loose `unknown` from ES2022 Error.cause with the
  // exact type we want exposed to consumers.
  override readonly cause: unknown;

  /**
   * Whether the orchestrator MAY continue past this error in
   * best-effort mode.  `false` (default) halts the affected branch.
   */
  readonly recoverable: boolean;

  /**
   * Open-ended structured context attached by the throwing stage.
   * Frozen on construction so it survives async hops without surprise
   * mutation.
   */
  readonly fields: Readonly<Record<string, JsonValue>>;

  constructor(init: StageErrorInit) {
    // Pass cause through the standard ES2022 Error constructor so the
    // host runtime's stack chaining picks it up.
    super(init.message, init.cause === undefined ? undefined : { cause: init.cause });

    this.name        = this.constructor.name;
    this.code        = init.code;
    this.inputPath   = init.inputPath ?? null;
    this.inputId     = init.inputId   ?? null;
    this.stageName   = init.stageName ?? null;
    this.cause       = init.cause;
    this.recoverable = init.recoverable ?? false;
    this.fields      = Object.freeze({ ...(init.fields ?? {}) });
  }

  /**
   * Return a JSON-serialisable view suitable for structured logs and
   * telemetry.  `cause` is reduced to its `String()` form (since
   * arbitrary thrown values are not generally JSON-safe); richer data
   * lives in `fields`.
   */
  toJson(): JsonValue {
    return {
      name:        this.name,
      code:        this.code,
      message:     this.message,
      inputPath:   this.inputPath,
      inputId:     this.inputId,
      stageName:   this.stageName,
      recoverable: this.recoverable,
      fields:      this.fields as JsonValue,
      cause:       this.cause === undefined || this.cause === null
                     ? null
                     : String(this.cause),
    };
  }
}
