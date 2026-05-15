/**
 * `StageContext` and `StageInitContext` — the runtime bundle every
 * stage receives (FM01 §4.1).
 *
 * The context is built by the orchestrator (or plugin host) on every
 * stage invocation and handed to the stage's `run` method.  It bundles
 * every capability-gated API the stage might use plus the un-gated
 * facilities (logger, cancellation, clock, cache, telemetry, events).
 *
 * === Why interfaces, not classes ===
 *
 * The orchestrator owns context construction.  It plugs in the
 * concrete impls — its own `StorageApi` for source-fs paths, its own
 * `NetworkApi` with capability-checked fetch, etc.  This package
 * declares only the *shape* the stage sees so stages compile and test
 * in isolation against the contract.
 *
 * === StageInitContext ===
 *
 * `init` and `dispose` hooks (FM01 §3.1) receive a slimmer context:
 * no cancellation token (init runs before cancel can fire; dispose
 * runs *during* shutdown so cancellation is moot), no per-invocation
 * cache (init/dispose are once-per-stage, not once-per-input).  In
 * exchange they get the validated `config` directly so they don't
 * need to re-parse it.
 */

import type { JsonValue } from "@coding-adventures/forme-types";
import type { CancellationToken } from "./cancellation.js";
import type { Cache } from "./cache.js";
import type { Clock } from "./clock.js";
import type { EventBus } from "./event-bus.js";
import type { Logger } from "./logger.js";
import type { TelemetryEmitter } from "./telemetry.js";
import type {
  EnvApi,
  FilesystemApi,
  NetworkApi,
  ShellApi,
  StorageApi,
} from "./capability-apis.js";

/** Per-invocation context handed to `Stage.run`. */
export interface StageContext {
  /** Diagnostic logger.  No capability needed. */
  readonly logger: Logger;
  /** Cancellation signal.  Stages call `throwIfCancelled()` at safe points. */
  readonly cancellation: CancellationToken;
  /** Wall-clock + monotonic time.  Frozen in reproducible-build mode. */
  readonly time: Clock;
  /** Stage-local cache for derived computations. */
  readonly cache: Cache;
  /** Per-stage telemetry emitter (capability-gated; denied = no-op). */
  readonly telemetry: TelemetryEmitter;

  /** Capability-gated APIs.  Denied wrappers throw `CapabilityError` per call. */
  readonly storage: StorageApi;
  readonly network: NetworkApi;
  readonly env: EnvApi;
  readonly filesystem: FilesystemApi;
  readonly shell: ShellApi;

  /** Cross-stage coordination (NOT a data channel). */
  readonly events: EventBus;
}

/**
 * Context handed to `init` / `dispose`.  Same as `StageContext` minus
 * the per-invocation `cancellation` and `cache`, plus the validated
 * `config` value (init doesn't need to re-parse what the orchestrator
 * already validated against `configSchema`).
 */
export interface StageInitContext extends Omit<StageContext, "cancellation" | "cache"> {
  /** Validated configuration value. */
  readonly config: JsonValue | unknown;
}
