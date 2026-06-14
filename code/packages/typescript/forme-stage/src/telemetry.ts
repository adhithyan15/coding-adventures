/**
 * Telemetry — structured event emission (FM01 §4.6).
 *
 * Capability-gated: a stage that calls `emit` without declaring
 * `telemetry:emit` receives a no-op emitter.  The host wires the
 * concrete emitter from the orchestrator's telemetry sink.
 *
 * Event names are namespaced strings (`<stage>.<event>` is the
 * convention, e.g. `parse-markdown.frontmatter-parsed`).  Fields are
 * structured JSON; consumers (the host's telemetry sink) decide what
 * to do with them — drop, aggregate, ship to analytics.
 */

import type { JsonValue } from "@coding-adventures/forme-types";

/** Telemetry emitter contract. */
export interface TelemetryEmitter {
  emit(event: string, fields: Record<string, JsonValue>): void;
}

/** No-op emitter — drops every event.  Used when the capability is denied. */
export function noOpTelemetryEmitter(): TelemetryEmitter {
  return NOOP;
}

const NOOP: TelemetryEmitter = {
  emit() { /* drop */ },
};

/**
 * Build a telemetry emitter that forwards every event to the supplied
 * sink.  Useful for tests (capture all emits in an array) and for
 * orchestrator wiring (forward to the real telemetry pipeline).
 */
export function callbackTelemetryEmitter(
  sink: (event: string, fields: Record<string, JsonValue>) => void,
): TelemetryEmitter {
  return {
    emit(event, fields) { sink(event, fields); },
  };
}
