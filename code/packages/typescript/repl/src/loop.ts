/**
 * The REPL event loop.
 *
 * This module contains two exported functions:
 *
 *   `runWithIo` — the fully injectable loop (takes explicit I/O functions).
 *   `run`       — the convenience wrapper that wires `process.stdin`/`stdout`.
 *
 * How the async loop works
 * ─────────────────────────
 * The core challenge of an async REPL is that we want TWO things to happen
 * simultaneously while the evaluator is running:
 *
 * 1. The evaluator does its work (possibly slow: compiling, network, etc.).
 * 2. The waiting animation ticks (updating a spinner on the terminal).
 *
 * JavaScript is single-threaded, so "simultaneously" means interleaving.
 * We use `setInterval` to schedule the animation ticks while awaiting the
 * eval Promise. This works because `await` yields to the event loop, which
 * can then fire the interval callbacks between microtasks.
 *
 *   ┌─── await evalPromise ──────────────────────────────────────────────┐
 *   │  tick tick tick tick tick tick tick tick tick tick tick tick tick   │
 *   └───────────────────────────────────────────────────────────────────┘
 *   clearInterval → waiting.stop()
 *
 * Why `clearInterval` before `stop`?
 * ────────────────────────────────────
 * We clear the interval *before* calling `stop` to guarantee that no `tick`
 * fires after `stop`. If a `tick` fired after `stop`, a real spinner
 * implementation might rewrite a line that `stop` had already erased — a
 * classic race condition. The JavaScript event loop is cooperative, so
 * synchronously clearing the interval then calling `stop` is safe.
 *
 * Error handling
 * ──────────────
 * `language.eval` returns a Promise. We attach a `.catch` handler that
 * converts any unexpected rejection into an `{tag: "error", message: ...}`
 * result. This means the loop *never* throws — all errors surface as
 * EvalResult values, keeping control-flow simple and predictable.
 */

import type {
  EvalResult,
  Language,
  Prompt,
  Waiting,
  InputFn,
  OutputFn,
  ReplMode,
} from "./types.js";
import { DefaultPrompt } from "./default_prompt.js";
import { SilentWaiting } from "./silent_waiting.js";

// ---------------------------------------------------------------------------
// runStep — evaluate one input with the waiting animation
// ---------------------------------------------------------------------------

/**
 * Evaluate a single input string and drive the waiting animation until done.
 *
 * This is intentionally kept as a separate function so it can be unit-tested
 * in isolation from the I/O loop. The loop calls `runStep` once per turn.
 *
 * Steps:
 * 1. Start the eval Promise (do not await yet).
 * 2. Call `waiting.start()` to get the initial animation state.
 * 3. Start a `setInterval` that calls `waiting.tick()` every `tickMs` ms.
 * 4. `await` the eval Promise (yielding to the event loop, allowing ticks).
 * 5. Clear the interval — no more ticks will fire.
 * 6. Call `waiting.stop()` to clean up the animation.
 * 7. Return the EvalResult.
 *
 * @param language  The evaluator to call.
 * @param waiting   The animation driver.
 * @param input     The raw input string from the user.
 * @returns         A Promise that resolves to an EvalResult.
 */
async function runStep(
  language: Language,
  waiting: Waiting,
  input: string
): Promise<EvalResult> {
  // Fire the eval Promise immediately, before starting the interval.
  // We must NOT await here yet — we want the interval to be running while
  // eval is in flight.
  const evalPromise = language.eval(input).catch(
    (e: unknown): EvalResult => ({
      tag: "error",
      // Convert anything throwable to a string. In practice this is usually
      // an Error object (so `String(e)` gives "Error: message"), but it could
      // also be a plain string or any other value.
      message: String(e),
    })
  );

  // Start the animation. The initial state is opaque — we never inspect it.
  let state = waiting.start();

  // Schedule periodic animation ticks. The interval fires asynchronously
  // while we await the eval Promise below.
  const interval = setInterval(() => {
    state = waiting.tick(state);
  }, waiting.tickMs());

  // Await the eval result. While this Promise is pending, the JavaScript
  // event loop can execute the setInterval callbacks above.
  const result = await evalPromise;

  // Stop ticking BEFORE calling stop(), to prevent a post-stop tick.
  clearInterval(interval);
  waiting.stop(state);

  return result;
}

// ---------------------------------------------------------------------------
// runWithIo — the main loop with injected I/O
// ---------------------------------------------------------------------------

/**
 * Run the REPL loop with fully injected I/O.
 *
 * This is the primary entry point for testing and for any host environment
 * that wants fine-grained control over how input is read and output is written.
 *
 * The loop terminates when:
 * - `inputFn()` returns `null` (EOF — the input stream is exhausted), or
 * - `language.eval()` returns `{tag: "quit"}`.
 *
 * Output behaviour per EvalResult:
 * - `{tag: "ok", output: string}` → call `outputFn(output)`
 * - `{tag: "ok", output: null}`   → silent (nothing printed)
 * - `{tag: "error", message}`     → call `outputFn("ERROR: " + message)`
 * - `{tag: "quit"}`               → exit the loop (no output)
 *
 * The prompt is printed before each input request, including on the first
 * turn. This matches the behaviour of standard REPLs: you always see a prompt
 * before you're expected to type.
 *
 * @param language  The evaluator. Must implement the Language interface.
 * @param prompt    Provides the prompt strings shown to the user.
 * @param waiting   Drives the animation shown during evaluation. Ignored in
 *                  sync mode — may be `null` when `mode` is `"sync"`.
 * @param inputFn   Async function that reads the next line; returns null on EOF.
 * @param outputFn  Synchronous function that writes one string to output.
 * @param mode      `"async"` (default) uses setInterval-based animation;
 *                  `"sync"` awaits eval directly with no animation overhead.
 *
 * @example
 * await runWithIo(
 *   new EchoLanguage(),
 *   new DefaultPrompt(),
 *   new SilentWaiting(),
 *   async () => "hello",
 *   (s) => console.log(s),
 * );
 *
 * @example — sync mode (no waiting animation required)
 * await runWithIo(
 *   new EchoLanguage(),
 *   new DefaultPrompt(),
 *   null,
 *   async () => "hello",
 *   (s) => console.log(s),
 *   "sync",
 * );
 */
export async function runWithIo(
  language: Language,
  prompt: Prompt,
  waiting: Waiting | null,
  inputFn: InputFn,
  outputFn: OutputFn,
  mode: ReplMode = "async"
): Promise<void> {
  while (true) {
    // Display the prompt before reading input.
    outputFn(prompt.globalPrompt());

    // Read the next line of input. null signals EOF (Ctrl-D in a terminal).
    const input = await inputFn();
    if (input === null) break;

    // Evaluate the input and act on the result.
    // In async mode: use setInterval-driven animation via runStep.
    // In sync mode: await eval directly — no animation, no interval overhead.
    let result: EvalResult;
    if (mode === "sync") {
      result = await language.eval(input).catch(
        (e: unknown): EvalResult => ({
          tag: "error",
          message: String(e),
        })
      );
    } else {
      // waiting is guaranteed non-null in async mode by the API contract.
      result = await runStep(language, waiting!, input);
    }

    if (result.tag === "quit") {
      // The language or the user has requested exit.
      break;
    }

    if (result.tag === "error") {
      // Prefix errors with "ERROR: " so they're visually distinct from normal
      // output. Real REPLs often use colour here; we keep it simple.
      outputFn("ERROR: " + result.message);
    }

    if (result.tag === "ok" && result.output !== null) {
      // Print the result. If output is null (e.g., a void expression like an
      // assignment), we stay silent — no blank line is printed.
      outputFn(result.output);
    }
  }
}

// ---------------------------------------------------------------------------
// run — convenience entry point wired to process.stdin/stdout
// ---------------------------------------------------------------------------

/**
 * Run the REPL in a standard Node.js terminal, reading from `process.stdin`
 * and writing to `process.stdout`.
 *
 * This function wraps `runWithIo` with readline-based I/O so callers don't
 * have to wire up readline themselves. It's the "batteries included" entry
 * point for building a real interactive REPL.
 *
 * Uses `readline.createInterface` which supports:
 * - Line editing (arrow keys, backspace, etc.)
 * - History (up/down arrows cycle through previous inputs)
 * - Ctrl-D to signal EOF (graceful exit)
 *
 * @param language  The evaluator to use.
 * @param prompt    The prompt provider (defaults to DefaultPrompt).
 * @param waiting   The animation driver (defaults to SilentWaiting).
 *
 * @example
 * // Minimal interactive echo REPL:
 * import { run, EchoLanguage } from "@coding-adventures/repl";
 * await run(new EchoLanguage());
 */
/* v8 ignore start */
export async function run(
  language: Language,
  prompt: Prompt = new DefaultPrompt(),
  waiting: Waiting = new SilentWaiting()
): Promise<void> {
  // We import readline lazily to avoid pulling it into environments (like
  // browsers) that don't have Node.js built-ins. The REPL framework's core
  // logic is environment-agnostic; only `run()` depends on Node.js.
  const { createInterface } = await import("node:readline");

  const rl = createInterface({
    input: process.stdin,
    output: process.stdout,
    terminal: process.stdin.isTTY,
  });

  // Buffer lines from readline into a queue so our async inputFn can pull
  // them one at a time. This decouples readline's event-based API from our
  // Promise-based loop.
  const lineQueue: (string | null)[] = [];

  // Resolvers waiting for the next line. If the queue is empty when inputFn
  // is called, we park a resolver here until readline delivers a line.
  const waiters: Array<(line: string | null) => void> = [];

  const enqueue = (line: string | null) => {
    if (waiters.length > 0) {
      // A resolver is already waiting — hand the line directly.
      waiters.shift()!(line);
    } else {
      lineQueue.push(line);
    }
  };

  rl.on("line", (line) => enqueue(line));
  rl.on("close", () => enqueue(null)); // EOF

  const inputFn: InputFn = () =>
    new Promise<string | null>((resolve) => {
      if (lineQueue.length > 0) {
        resolve(lineQueue.shift()!);
      } else {
        waiters.push(resolve);
      }
    });

  // Write each output string followed by a newline, matching the convention
  // that terminal output ends each logical unit with \n.
  const outputFn: OutputFn = (s: string) => process.stdout.write(s + "\n");

  await runWithIo(language, prompt, waiting, inputFn, outputFn);

  rl.close();
}
/* v8 ignore stop */
