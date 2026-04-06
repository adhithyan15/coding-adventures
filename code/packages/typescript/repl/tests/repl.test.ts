/**
 * Tests for the REPL framework.
 *
 * These tests verify the complete REPL loop end-to-end using injected I/O.
 * No stdin, stdout, or process globals are touched — everything goes through
 * `InputFn` / `OutputFn` callbacks, making the tests fast, deterministic,
 * and side-effect-free.
 *
 * Test design: each test drives `runWithIo` with a predetermined sequence of
 * inputs and captures all output in an array. We then assert on that array.
 *
 * The `makeInputFn` and `makeOutputCapture` helpers below are the standard
 * test harness used by all test cases.
 */

import { describe, it, expect } from "vitest";
import {
  runWithIo,
  EchoLanguage,
  DefaultPrompt,
  SilentWaiting,
} from "../src/index.js";
import type { EvalResult, InputFn, OutputFn, Language } from "../src/index.js";

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/**
 * Build an InputFn that yields items from `inputs` one at a time.
 *
 * Once all items are exhausted, subsequent calls return `null` (simulating
 * EOF / Ctrl-D on a real terminal).
 *
 * @example
 * const inputFn = makeInputFn(["hello", ":quit"]);
 * await inputFn(); // "hello"
 * await inputFn(); // ":quit"
 * await inputFn(); // null  ← EOF
 */
function makeInputFn(inputs: (string | null)[]): InputFn {
  let i = 0;
  return async () => inputs[i++] ?? null;
}

/**
 * Build an OutputFn that records every call in an array.
 *
 * Returns both the function and the live array, so tests can push outputs
 * and then inspect `outputs` after the loop exits.
 *
 * @example
 * const { fn, outputs } = makeOutputCapture();
 * fn("hello");
 * fn("world");
 * // outputs === ["hello", "world"]
 */
function makeOutputCapture(): { fn: OutputFn; outputs: string[] } {
  const outputs: string[] = [];
  return { fn: (s: string) => outputs.push(s), outputs };
}

/** Convenience: create the standard built-in Language/Prompt/Waiting trio. */
function makeDefaults() {
  return {
    language: new EchoLanguage(),
    prompt: new DefaultPrompt(),
    waiting: new SilentWaiting(),
  };
}

// ---------------------------------------------------------------------------
// Test suite
// ---------------------------------------------------------------------------

describe("REPL framework", () => {
  // ─── Test 1 ─────────────────────────────────────────────────────────────

  it("echoes a single input line back as output", async () => {
    /**
     * The most basic smoke test: type "hello", get "hello" back.
     *
     * Expected interaction:
     *   > hello        ← prompt + user input (we inject "hello")
     *   hello          ← EchoLanguage echoes it
     *   >              ← prompt for next turn
     *   (EOF)          ← loop exits
     *
     * The captured outputs will be: ["> ", "hello", "> "]
     * We only assert on the echoed value to keep the test focused.
     */
    const { language, prompt, waiting } = makeDefaults();
    const { fn, outputs } = makeOutputCapture();

    // Feed "hello" then EOF (null) so the loop runs exactly one eval turn.
    await runWithIo(language, prompt, waiting, makeInputFn(["hello", null]), fn);

    // The echo output "hello" should appear somewhere in the captured output.
    expect(outputs).toContain("hello");
  });

  // ─── Test 2 ─────────────────────────────────────────────────────────────

  it("terminates cleanly when :quit is entered", async () => {
    /**
     * `:quit` is EchoLanguage's built-in exit command.
     *
     * The loop should stop immediately after receiving `{tag: "quit"}` —
     * it should NOT prompt for another line, and it should NOT print anything
     * for the `:quit` input itself.
     *
     * We pass a second input ("unreachable") to verify the loop truly exits
     * after `:quit` and does not process further inputs.
     */
    const { language, prompt, waiting } = makeDefaults();
    const { fn, outputs } = makeOutputCapture();

    await runWithIo(
      language,
      prompt,
      waiting,
      makeInputFn([":quit", "unreachable"]),
      fn
    );

    // The string "unreachable" must never appear in the output.
    expect(outputs).not.toContain("unreachable");

    // ":quit" itself must not be echoed (quit produces no output).
    expect(outputs).not.toContain(":quit");
  });

  // ─── Test 3 ─────────────────────────────────────────────────────────────

  it("handles multiple turns correctly before quitting", async () => {
    /**
     * A multi-turn session: ["hello", "world", ":quit"].
     *
     * The loop should:
     * 1. Print "> ", eval "hello" → print "hello"
     * 2. Print "> ", eval "world" → print "world"
     * 3. Print "> ", eval ":quit" → exit (no output for :quit)
     *
     * Both "hello" and "world" must appear in the output, in order.
     */
    const { language, prompt, waiting } = makeDefaults();
    const { fn, outputs } = makeOutputCapture();

    await runWithIo(
      language,
      prompt,
      waiting,
      makeInputFn(["hello", "world", ":quit"]),
      fn
    );

    const helloIdx = outputs.indexOf("hello");
    const worldIdx = outputs.indexOf("world");

    expect(helloIdx).toBeGreaterThanOrEqual(0); // "hello" must appear
    expect(worldIdx).toBeGreaterThanOrEqual(0); // "world" must appear
    expect(helloIdx).toBeLessThan(worldIdx);    // "hello" comes before "world"
  });

  // ─── Test 4 ─────────────────────────────────────────────────────────────

  it("prints nothing when eval returns ok with null output", async () => {
    /**
     * Some expressions produce no visible output (e.g., assignments like
     * `x = 42` in Python don't print anything). The EvalResult in those cases
     * is `{tag: "ok", output: null}`.
     *
     * We use a custom Language that always returns null output to verify that
     * the loop stays silent — no blank lines, no "undefined", nothing.
     *
     * The only outputs should be the prompt strings ("> ").
     */
    const silentLanguage: Language = {
      async eval(_input: string): Promise<EvalResult> {
        return { tag: "ok", output: null };
      },
    };

    const { prompt, waiting } = makeDefaults();
    const { fn, outputs } = makeOutputCapture();

    // One turn: "assign" produces null output, then EOF exits.
    await runWithIo(
      silentLanguage,
      prompt,
      waiting,
      makeInputFn(["assign", null]),
      fn
    );

    // The only things in outputs should be the prompt(s). There should be no
    // blank strings or unexpected content from the null output.
    const nonPromptOutputs = outputs.filter((s) => s !== "> ");
    expect(nonPromptOutputs).toHaveLength(0);
  });

  // ─── Test 5 ─────────────────────────────────────────────────────────────

  it("prefixes error results with 'ERROR: '", async () => {
    /**
     * When a Language returns `{tag: "error", message: "bad"}`, the loop
     * must display "ERROR: bad" to the user.
     *
     * The prefix makes errors visually distinct from normal output, which is
     * important for usability. We use a custom Language that always errors
     * to test this path directly.
     */
    const errorLanguage: Language = {
      async eval(_input: string): Promise<EvalResult> {
        return { tag: "error", message: "bad" };
      },
    };

    const { prompt, waiting } = makeDefaults();
    const { fn, outputs } = makeOutputCapture();

    await runWithIo(
      errorLanguage,
      prompt,
      waiting,
      makeInputFn(["anything", null]),
      fn
    );

    // The error message must appear with the "ERROR: " prefix.
    expect(outputs).toContain("ERROR: bad");
  });

  // ─── Test 6 ─────────────────────────────────────────────────────────────

  it("handles unexpected exceptions from eval without crashing", async () => {
    /**
     * Real evaluators can throw unexpectedly — bugs, out-of-memory, etc.
     * The loop must catch any rejection and convert it to an error message,
     * then continue running (it should NOT crash or exit).
     *
     * We test this by injecting a Language that throws on the first call and
     * returns ":quit" on the second. After the exception is handled, the loop
     * must still process the ":quit" input and exit cleanly.
     *
     * This verifies two things:
     * 1. The exception is caught and formatted as "ERROR: <message>".
     * 2. The loop continues running after an error (it's resilient).
     */
    let callCount = 0;
    const throwingLanguage: Language = {
      async eval(input: string): Promise<EvalResult> {
        callCount++;
        if (callCount === 1) {
          // First call: throw an unexpected error.
          throw new Error("unexpected boom");
        }
        // Second call: normal exit.
        if (input === ":quit") return { tag: "quit" };
        return { tag: "ok", output: input };
      },
    };

    const { prompt, waiting } = makeDefaults();
    const { fn, outputs } = makeOutputCapture();

    await runWithIo(
      throwingLanguage,
      prompt,
      waiting,
      makeInputFn(["crash", ":quit"]),
      fn
    );

    // The exception must have been caught and reported as an error.
    const errorOutputs = outputs.filter((s) => s.startsWith("ERROR: "));
    expect(errorOutputs).toHaveLength(1);
    expect(errorOutputs[0]).toContain("unexpected boom");

    // The loop must have continued to process ":quit" (callCount should be 2).
    expect(callCount).toBe(2);
  });
});

// ---------------------------------------------------------------------------
// Built-in implementations — unit tests
// ---------------------------------------------------------------------------

describe("EchoLanguage", () => {
  it("echoes arbitrary strings", async () => {
    const lang = new EchoLanguage();
    const result = await lang.eval("test input");
    expect(result).toEqual({ tag: "ok", output: "test input" });
  });

  it("returns quit for :quit", async () => {
    const lang = new EchoLanguage();
    const result = await lang.eval(":quit");
    expect(result).toEqual({ tag: "quit" });
  });

  it("echoes empty strings", async () => {
    const lang = new EchoLanguage();
    const result = await lang.eval("");
    expect(result).toEqual({ tag: "ok", output: "" });
  });
});

describe("DefaultPrompt", () => {
  it("returns '> ' for globalPrompt", () => {
    const p = new DefaultPrompt();
    expect(p.globalPrompt()).toBe("> ");
  });

  it("returns '... ' for linePrompt", () => {
    const p = new DefaultPrompt();
    expect(p.linePrompt()).toBe("... ");
  });
});

describe("SilentWaiting", () => {
  it("tickMs returns 100", () => {
    const w = new SilentWaiting();
    expect(w.tickMs()).toBe(100);
  });

  it("start returns an object", () => {
    const w = new SilentWaiting();
    const state = w.start();
    expect(typeof state).toBe("object");
  });

  it("tick returns the state unchanged", () => {
    const w = new SilentWaiting();
    const state = w.start();
    const next = w.tick(state);
    // SilentWaiting returns the same reference.
    expect(next).toBe(state);
  });

  it("stop does not throw", () => {
    const w = new SilentWaiting();
    const state = w.start();
    expect(() => w.stop(state)).not.toThrow();
  });
});

// ---------------------------------------------------------------------------
// Sync mode tests
// ---------------------------------------------------------------------------

describe("REPL sync mode", () => {
  // ─── Sync Test 1 ──────────────────────────────────────────────────────────

  it("sync mode: echo works", async () => {
    /**
     * In sync mode, `language.eval` is awaited directly with no setInterval.
     * This test verifies that the echo output is still produced correctly —
     * the mode change only affects *how* the eval is awaited, not what the
     * loop does with the result.
     *
     * We pass `mode: "sync"` and a real SilentWaiting (which will be ignored).
     * The loop must still print the echoed value.
     */
    const language = new EchoLanguage();
    const prompt = new DefaultPrompt();
    const waiting = new SilentWaiting();
    const outputs: string[] = [];
    const outputFn = (s: string) => outputs.push(s);

    await runWithIo(
      language,
      prompt,
      waiting,
      makeInputFn(["hello", null]),
      outputFn,
      "sync"
    );

    // The echo result must appear in outputs, just as it does in async mode.
    expect(outputs).toContain("hello");
  });

  // ─── Sync Test 2 ──────────────────────────────────────────────────────────

  it("sync mode: quit works", async () => {
    /**
     * `:quit` must terminate the loop in sync mode, exactly as in async mode.
     *
     * We supply `:quit` as the first input followed by `"unreachable"`.
     * The loop must exit after the quit signal and never process the second
     * input.
     */
    const language = new EchoLanguage();
    const prompt = new DefaultPrompt();
    const waiting = new SilentWaiting();
    const outputs: string[] = [];
    const outputFn = (s: string) => outputs.push(s);

    await runWithIo(
      language,
      prompt,
      waiting,
      makeInputFn([":quit", "unreachable"]),
      outputFn,
      "sync"
    );

    // "unreachable" must never appear — the loop exited after `:quit`.
    expect(outputs).not.toContain("unreachable");

    // `:quit` itself must not be echoed.
    expect(outputs).not.toContain(":quit");
  });

  // ─── Sync Test 3 ──────────────────────────────────────────────────────────

  it("sync mode: null waiting allowed", async () => {
    /**
     * In sync mode the Waiting interface is completely bypassed, so passing
     * `null` for `waiting` must be accepted without error.
     *
     * This is the primary use-case for sync mode: scripted / batch evaluation
     * where no spinner is desired and no Waiting implementation is at hand.
     *
     * We pass `waiting: null` and `mode: "sync"` and verify that the loop
     * still echoes input correctly.
     */
    const language = new EchoLanguage();
    const prompt = new DefaultPrompt();
    const outputs: string[] = [];
    const outputFn = (s: string) => outputs.push(s);

    // TypeScript: null is assignable to `Waiting | null` as declared.
    await runWithIo(
      language,
      prompt,
      null,          // waiting explicitly null — safe in sync mode
      makeInputFn(["world", null]),
      outputFn,
      "sync"
    );

    // Echo result must still appear even with null waiting.
    expect(outputs).toContain("world");
  });
});
