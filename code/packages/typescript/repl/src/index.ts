/**
 * @coding-adventures/repl
 *
 * A minimal, fully-typed REPL (Read-Eval-Print Loop) framework for TypeScript.
 *
 * ## What is a REPL?
 *
 * A REPL is the interactive shell you get when you type `node`, `python3`,
 * `iex`, or `ghci` at a terminal. It repeats forever:
 *
 *   1. **R**ead   — display a prompt, wait for the user to type a line.
 *   2. **E**val   — pass the input to an evaluator (the Language).
 *   3. **P**rint  — display the result (or error message).
 *   4. **L**oop   — go back to step 1.
 *
 * ## Architecture
 *
 * The framework separates concerns into three pluggable interfaces:
 *
 * - `Language`  — knows how to evaluate one input string → EvalResult
 * - `Prompt`    — produces the "> " and "... " prompt strings
 * - `Waiting`   — drives an animation while the evaluator is busy
 *
 * I/O is injected as plain functions (`InputFn`, `OutputFn`), making the
 * loop fully testable without touching `process.stdin`/`stdout`.
 *
 * ## Quick start
 *
 * ```typescript
 * import { run, EchoLanguage } from "@coding-adventures/repl";
 *
 * // Start an interactive echo REPL in the terminal:
 * await run(new EchoLanguage());
 * ```
 *
 * ## Testing without a terminal
 *
 * ```typescript
 * import { runWithIo, EchoLanguage, DefaultPrompt, SilentWaiting } from "@coding-adventures/repl";
 *
 * const outputs: string[] = [];
 * let i = 0;
 * const inputs = ["hello", ":quit"];
 *
 * await runWithIo(
 *   new EchoLanguage(),
 *   new DefaultPrompt(),
 *   new SilentWaiting(),
 *   async () => inputs[i++] ?? null,
 *   (s) => outputs.push(s),
 * );
 * // outputs === ["> ", "hello", "> "]
 * ```
 */

// Types and interfaces
export type { EvalResult, Language, Prompt, Waiting, InputFn, OutputFn, ReplMode } from "./types.js";

// Loop functions
export { runWithIo, run } from "./loop.js";

// Built-in implementations
export { EchoLanguage } from "./echo_language.js";
export { DefaultPrompt } from "./default_prompt.js";
export { SilentWaiting } from "./silent_waiting.js";
