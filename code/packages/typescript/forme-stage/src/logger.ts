/**
 * Logger — diagnostic output for stages.
 *
 * Per FM01 §4.2: the logger is *always* available (no capability gate)
 * because it produces observable side effects for humans but does not
 * influence pipeline outputs.  The contract is small:
 *
 *   - Five named levels: trace, debug, info, warn, error
 *   - Each method takes a message + optional structured `fields`
 *   - `child(fields)` returns a logger that pre-mixes the given fields
 *     into every subsequent emit (orchestrator uses this to scope
 *     `{ stage, instance, inputId }` automatically per invocation)
 *
 * === Default implementation: ConsoleLogger ===
 *
 * Writes structured JSON to stderr.  Each emission is one line, one
 * object: `{ level, message, ts, ...fields }`.  Stderr because logs
 * shouldn't pollute the orchestrator's stdout (which downstream tooling
 * may parse for build artifacts).
 *
 * The default level threshold is `info`.  Set the `level` field of
 * `consoleLogger({ level: ... })` to suppress emits below that level.
 *
 * === Why JSON, not pretty-print? ===
 *
 * Drivers (CLI, dev server, editor) format logs for their own UIs.  The
 * orchestrator hands every stage the same logger, and the stage doesn't
 * know which driver is consuming the output.  Structured JSON travels
 * cleanly through `jq`, `lnav`, telemetry pipelines, and anything that
 * eventually wants to grep, filter, or render.  Pretty-printing is
 * applied at the *driver* layer, not here.
 */

import type { JsonValue } from "@coding-adventures/forme-types";

/** Five-level severity ladder.  Names match FM01 §4.2. */
export const LOG_LEVELS = Object.freeze([
  "trace", "debug", "info", "warn", "error",
] as const);
export type LogLevel = (typeof LOG_LEVELS)[number];

/**
 * Logger contract.  Every level method accepts a message plus optional
 * structured fields; `child` returns a derived logger that pre-mixes
 * the given fields into all subsequent emits.
 */
export interface Logger {
  trace(message: string, fields?: Record<string, JsonValue>): void;
  debug(message: string, fields?: Record<string, JsonValue>): void;
  info(message: string, fields?: Record<string, JsonValue>): void;
  warn(message: string, fields?: Record<string, JsonValue>): void;
  error(message: string, fields?: Record<string, JsonValue>): void;
  child(fields: Record<string, JsonValue>): Logger;
}

// ─── Console implementation ───────────────────────────────────────────────

export interface ConsoleLoggerOptions {
  /** Minimum level to emit.  Default: "info". */
  readonly level?: LogLevel;
  /** Where to write (defaults to console).  Test hook. */
  readonly write?: (line: string) => void;
  /** Clock for the `ts` field.  Default: Date.now. */
  readonly now?: () => number;
}

const LEVEL_RANK: Record<LogLevel, number> = {
  trace: 0, debug: 1, info: 2, warn: 3, error: 4,
};

class ConsoleLoggerImpl implements Logger {
  constructor(
    private readonly threshold: number,
    private readonly write: (line: string) => void,
    private readonly now: () => number,
    private readonly baseFields: Record<string, JsonValue>,
  ) {}

  trace(message: string, fields?: Record<string, JsonValue>): void { this.emit("trace", message, fields); }
  debug(message: string, fields?: Record<string, JsonValue>): void { this.emit("debug", message, fields); }
  info(message: string, fields?: Record<string, JsonValue>): void  { this.emit("info",  message, fields); }
  warn(message: string, fields?: Record<string, JsonValue>): void  { this.emit("warn",  message, fields); }
  error(message: string, fields?: Record<string, JsonValue>): void { this.emit("error", message, fields); }

  child(fields: Record<string, JsonValue>): Logger {
    return new ConsoleLoggerImpl(
      this.threshold,
      this.write,
      this.now,
      { ...this.baseFields, ...fields },
    );
  }

  private emit(level: LogLevel, message: string, fields?: Record<string, JsonValue>): void {
    if (LEVEL_RANK[level] < this.threshold) return;
    const payload = {
      level,
      message,
      ts: this.now(),
      ...this.baseFields,
      ...(fields ?? {}),
    };
    this.write(JSON.stringify(payload));
  }
}

/**
 * Build a Logger that writes structured JSON lines to stderr (or to
 * the supplied `write` hook).  Suitable as the kernel's default logger.
 */
export function consoleLogger(options: ConsoleLoggerOptions = {}): Logger {
  const level = options.level ?? "info";
  const threshold = LEVEL_RANK[level];
  const write = options.write ?? defaultWrite;
  const now   = options.now   ?? Date.now;
  return new ConsoleLoggerImpl(threshold, write, now, {});
}

function defaultWrite(line: string): void {
  // eslint-disable-next-line no-console
  console.error(line);
}

// ─── Silent logger ────────────────────────────────────────────────────────

/**
 * A no-op logger that drops every emission.  Useful for tests that want
 * to silence stage diagnostics, and for stages run inside reproducible
 * builds where logs would pollute the artifact.
 */
export function silentLogger(): Logger {
  return SILENT;
}

const SILENT: Logger = {
  trace() {}, debug() {}, info() {}, warn() {}, error() {},
  child() { return SILENT; },
};
