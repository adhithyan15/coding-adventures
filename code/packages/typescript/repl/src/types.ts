/**
 * Core types for the REPL framework.
 *
 * A REPL (Read-Eval-Print Loop) is the interactive programming environment
 * you get when you type `node`, `python3`, `iex`, or `ghci` at the terminal.
 * The shell prints a prompt, you type an expression, the evaluator runs it,
 * the result is printed, and the cycle repeats.
 *
 * This module defines the three pluggable interfaces that decouple the loop
 * from any particular language, prompt style, or waiting animation.
 *
 * Architecture overview:
 *
 *   ┌─────────┐   input    ┌──────────┐   EvalResult   ┌──────────┐
 *   │ InputFn │ ─────────▶ │ Language │ ─────────────▶ │  Loop    │
 *   └─────────┘            └──────────┘                └──────────┘
 *        │                                                   │
 *        │                 ┌──────────┐                      │ output
 *        │                 │  Prompt  │ ◀────────────────────┤
 *        │                 └──────────┘                      │
 *        │                 ┌──────────┐                      │
 *        └───────────────▶ │ Waiting  │ ◀────────────────────┘
 *                          └──────────┘
 */

// ---------------------------------------------------------------------------
// EvalResult — the three outcomes of evaluating one line
// ---------------------------------------------------------------------------

/**
 * The result returned by a `Language` evaluator for a single input.
 *
 * Three cases, inspired by Rust's `Result` / discriminated unions:
 *
 * - `ok`    — evaluation succeeded. `output` is the string to display, or
 *             `null` if there is nothing to print (e.g., an assignment).
 * - `error` — evaluation failed. `message` describes what went wrong.
 * - `quit`  — the user signalled end-of-session (e.g., typed `:quit`).
 *
 * Using a tagged union (`tag` field) instead of exceptions keeps control-flow
 * explicit and easy to test without try/catch noise in the loop.
 */
export type EvalResult =
  | { tag: "ok"; output: string | null }
  | { tag: "error"; message: string }
  | { tag: "quit" };

// ---------------------------------------------------------------------------
// Language — the pluggable evaluator
// ---------------------------------------------------------------------------

/**
 * A `Language` knows how to evaluate one line (or expression) of input.
 *
 * The `eval` method is **async** because real evaluators often need to:
 * - spawn a child process (e.g., calling out to a Python interpreter),
 * - perform network I/O (e.g., querying a remote type-checker),
 * - compile code in the background.
 *
 * Implementors must never throw synchronously; any error should resolve
 * to `{tag: "error", message: ...}`. The loop additionally wraps the promise
 * in a `.catch()` to guard against unexpected rejections.
 *
 * @example
 * class MyLanguage implements Language {
 *   async eval(input: string): Promise<EvalResult> {
 *     if (input === "exit") return { tag: "quit" };
 *     return { tag: "ok", output: `You typed: ${input}` };
 *   }
 * }
 */
export interface Language {
  eval(input: string): Promise<EvalResult>;
}

// ---------------------------------------------------------------------------
// Prompt — the string(s) shown before the user types
// ---------------------------------------------------------------------------

/**
 * A `Prompt` produces the text displayed to the user before each input.
 *
 * Two variants are provided because many REPLs switch prompts when a
 * multi-line expression is incomplete — the "continuation" or "line" prompt
 * signals that the REPL is waiting for more input rather than starting fresh.
 *
 *   > let x =          ← globalPrompt (first line of a new expression)
 *   ...   42           ← linePrompt   (continuation of an incomplete expression)
 *   > x                ← globalPrompt again
 *   42
 *
 * This framework currently uses `globalPrompt` at the start of every turn.
 * `linePrompt` is provided for implementors who extend the loop to handle
 * multi-line input.
 */
export interface Prompt {
  /** The primary prompt shown at the start of each new input. */
  globalPrompt(): string;

  /** The continuation prompt shown when more lines are expected. */
  linePrompt(): string;
}

// ---------------------------------------------------------------------------
// Waiting — an animation shown while the evaluator works
// ---------------------------------------------------------------------------

/**
 * A `Waiting` drives an animation (spinner, progress dots, etc.) that plays
 * while an async evaluation is in flight.
 *
 * The design follows a **state machine** pattern to stay compatible with
 * both `setInterval`-based ticking and manual frame advances in tests:
 *
 *   start() → initial state
 *   tick(state) → next state     (called repeatedly by the loop)
 *   stop(state) → void           (clean up — erase the spinner, etc.)
 *
 * `tickMs()` tells the loop how often to call `tick`. A lower value gives
 * a smoother animation; a higher value reduces CPU overhead during long evals.
 *
 * State is opaque (`unknown`) so different implementations can store whatever
 * they need — a frame index, a reference to the terminal cursor, etc. — without
 * leaking their internals into the loop.
 *
 * @example
 * class SpinnerWaiting implements Waiting {
 *   private readonly frames = ['|', '/', '-', '\\'];
 *   start() { return 0; }
 *   tick(state: unknown) {
 *     const frame = (state as number + 1) % this.frames.length;
 *     process.stdout.write(`\r${this.frames[frame]} thinking...`);
 *     return frame;
 *   }
 *   tickMs() { return 80; }
 *   stop(_state: unknown) { process.stdout.write('\r' + ' '.repeat(20) + '\r'); }
 * }
 */
export interface Waiting {
  /** Called once when evaluation begins. Returns the initial animation state. */
  start(): unknown;

  /**
   * Called every `tickMs()` milliseconds while evaluation is in progress.
   * Receives the current state and must return the next state.
   */
  tick(state: unknown): unknown;

  /** How often (in milliseconds) the loop should call `tick`. */
  tickMs(): number;

  /** Called once when evaluation completes. Use this to erase the animation. */
  stop(state: unknown): void;
}

// ---------------------------------------------------------------------------
// I/O function types
// ---------------------------------------------------------------------------

/**
 * A function that produces the next line of user input asynchronously.
 *
 * Returns `null` to signal end-of-input (EOF), which causes the loop to exit
 * gracefully. This mirrors the behaviour of `readline.Interface`'s `close`
 * event and makes it easy to inject a fake input source in tests.
 */
export type InputFn = () => Promise<string | null>;

/**
 * A function that writes one string to the output (no trailing newline added
 * by the framework — include `\n` in the string if you want a newline).
 *
 * Using a function rather than `console.log` directly makes the loop fully
 * testable without monkey-patching globals.
 */
export type OutputFn = (s: string) => void;

// ---------------------------------------------------------------------------
// ReplMode — controls how the loop runs evaluation
// ---------------------------------------------------------------------------

/**
 * Controls the evaluation strategy used by `runWithIo`.
 *
 * - `"async"` (default) — the original setInterval-based behaviour. The eval
 *   Promise runs concurrently with the Waiting animation, allowing a spinner
 *   or progress indicator to tick while the evaluator is busy. `waiting` must
 *   be a non-null `Waiting` implementation in this mode.
 *
 * - `"sync"` — a simpler path that `await`s the eval Promise directly with no
 *   animation overhead. The Waiting interface is completely bypassed, so
 *   `waiting` may be `null`. This is useful for:
 *     - Scripted / batch evaluation where no spinner is wanted.
 *     - Testing environments where real timers would slow tests down.
 *     - Embedding the REPL in contexts where `setInterval` is unavailable.
 *
 * Error handling is identical in both modes: any rejection from `language.eval`
 * is caught and converted to `{tag: "error", message: ...}`.
 */
export type ReplMode = "async" | "sync";
