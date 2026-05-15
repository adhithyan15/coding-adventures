/**
 * @coding-adventures/forme-stage
 *
 * The heart of the Forme kernel.  Defines what a Stage *is*, what its
 * runtime context (`StageContext`) looks like, and provides the
 * default in-memory implementations of every un-gated facility plus
 * the denied-wrapper factories for every capability-gated API.
 *
 * Two surfaces matter to consumers:
 *
 *   - **Stage authors** import `Stage`, `defineStage`, `StageOutput`,
 *     `StageContext`, `StageInitContext`, plus the API interfaces
 *     they intend to use (`StorageApi`, `NetworkApi`, etc.).
 *
 *   - **Orchestrator authors** additionally import the default
 *     implementations (`consoleLogger`, `systemClock`, `inMemoryCache`,
 *     `inMemoryEventBus`, `noOpTelemetryEmitter`, `createCancellationTokenSource`)
 *     and the denied wrappers (`deniedStorageApi`, etc.) to compose
 *     `StageContext` instances per invocation.
 *
 * See FM01 §3-4 for the design.  See per-module headers for the
 * rationale behind each implementation choice.
 */

// ─── Stage contract ───────────────────────────────────────────────────────
export { defineStage } from "./stage.js";
export type { JsonSchema, Stage, StageOutput } from "./stage.js";

// ─── StageContext shapes ──────────────────────────────────────────────────
export type { StageContext, StageInitContext } from "./context.js";

// ─── Logger ───────────────────────────────────────────────────────────────
export {
  LOG_LEVELS,
  consoleLogger,
  silentLogger,
} from "./logger.js";
export type { ConsoleLoggerOptions, LogLevel, Logger } from "./logger.js";

// ─── Cancellation ─────────────────────────────────────────────────────────
export {
  createCancellationTokenSource,
  neverCancelledToken,
} from "./cancellation.js";
export type { CancellationToken, CancellationTokenSource } from "./cancellation.js";

// ─── Clock ────────────────────────────────────────────────────────────────
export { frozenClock, systemClock } from "./clock.js";
export type { Clock, FrozenClockOptions } from "./clock.js";

// ─── Cache ────────────────────────────────────────────────────────────────
export { inMemoryCache } from "./cache.js";
export type { Cache } from "./cache.js";

// ─── Telemetry ────────────────────────────────────────────────────────────
export {
  callbackTelemetryEmitter,
  noOpTelemetryEmitter,
} from "./telemetry.js";
export type { TelemetryEmitter } from "./telemetry.js";

// ─── EventBus ─────────────────────────────────────────────────────────────
export { inMemoryEventBus } from "./event-bus.js";
export type { EventBus } from "./event-bus.js";

// ─── Capability-gated APIs ────────────────────────────────────────────────
export {
  deniedEnvApi,
  deniedFilesystemApi,
  deniedNetworkApi,
  deniedShellApi,
  deniedStorageApi,
} from "./capability-apis.js";
export type {
  EnvApi,
  FilesystemApi,
  NetworkApi,
  ShellApi,
  ShellOptions,
  ShellResult,
  StorageApi,
  StorageChange,
  StorageEntry,
  StorageStat,
} from "./capability-apis.js";
