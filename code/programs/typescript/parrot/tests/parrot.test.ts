/**
 * parrot.test.ts — test suite for the Parrot REPL program.
 *
 * ## Testing strategy
 *
 * We use I/O injection throughout. `runWithIo` from @coding-adventures/repl
 * accepts:
 *   - `inputFn`  — an async function returning string | null (null = EOF)
 *   - `outputFn` — a function receiving each output string
 *
 * This means we can test the full REPL loop without touching stdin or stdout,
 * without spawning subprocesses, and without mocking globals. Every test is
 * deterministic and runs in under a millisecond.
 *
 * ## Helper: runParrot
 *
 * `runParrot(inputs, mode?)` drives the REPL with a pre-canned list of inputs
 * and collects all output strings into an array. The test then asserts on the
 * collected output.
 *
 * ## What we test
 *
 * 1.  Basic echo — input is echoed back
 * 2.  Quit command — ":quit" ends the session
 * 3.  Multiple inputs — all are echoed in order
 * 4.  Sync mode — same behaviour as async mode
 * 5.  Async mode — default mode produces same results
 * 6.  Banner contains "Parrot" — the welcome text is correct
 * 7.  Line prompt contains parrot emoji — the prompt is correct
 * 8.  EOF exits gracefully — null input ends the session
 * 9.  Empty string is echoed — blank input is not ignored
 * 10. Multiple echoes before quit — outputs accumulate correctly
 * 11. ParrotPrompt.globalPrompt content — direct unit test
 * 12. ParrotPrompt.linePrompt format — direct unit test
 * 13. Session ends on :quit even with queued inputs
 * 14. Output collected correctly — no dropped lines
 * 15. Error result prints "ERROR: ..." (TS loop uses "ERROR:" prefix)
 */

import { describe, it, expect } from "vitest";
import { EchoLanguage, SilentWaiting, runWithIo } from "@coding-adventures/repl";
import { ParrotPrompt } from "../src/prompt.js";

// ---------------------------------------------------------------------------
// Test helper
// ---------------------------------------------------------------------------

/**
 * runParrot — drives the Parrot REPL with canned inputs and collects output.
 *
 * @param inputs  Array of strings (lines of input) terminated by null (EOF).
 *                If the last element is not null, the loop ends when the queue
 *                is exhausted (inputFn returns undefined, coerced to null).
 * @param mode    "async" (default) or "sync" — passed to runWithIo.
 * @returns       All strings passed to outputFn during the session.
 *
 * ## How the input queue works
 *
 * We copy the inputs array and use `shift()` to consume one element per call.
 * When the queue is empty, `shift()` returns `undefined`, which we treat as
 * `null` (EOF). Explicit `null` in the array also signals EOF.
 *
 * This mirrors how a real terminal works: when the user closes stdin (Ctrl-D),
 * the readline interface fires "close" and inputFn returns null.
 */
async function runParrot(
  inputs: Array<string | null>,
  mode: "async" | "sync" = "async"
): Promise<string[]> {
  const output: string[] = [];
  const queue = [...inputs];

  await runWithIo(
    new EchoLanguage(),
    new ParrotPrompt(),
    new SilentWaiting(),
    // inputFn: shift the next element; shift() returns undefined when empty,
    // which the ?? coalesces to null (EOF signal).
    async () => queue.length > 0 ? queue.shift()! : null,
    // outputFn: collect each output chunk for assertion.
    (text: string) => output.push(text),
    mode,
  );

  return output;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Parrot REPL", () => {
  // ── Test 1: basic echo ──────────────────────────────────────────────────
  it("echoes input back to the user", async () => {
    // Arrange: type "hello" then quit.
    // Act: run the REPL.
    const out = await runParrot(["hello", ":quit"]);

    // Assert: "hello" appears somewhere in the collected output.
    // The loop also outputs the prompt before each input, so `out` contains
    // both prompt strings and the echoed input.
    expect(out).toContain("hello");
  });

  // ── Test 2: quit ends the session ───────────────────────────────────────
  it("stops looping when :quit is typed", async () => {
    // Arrange: immediately quit.
    const out = await runParrot([":quit"]);

    // Assert: the session ended (no error thrown), and "hello" was NOT echoed
    // because no such input was provided.
    expect(out).not.toContain("hello");
  });

  // ── Test 3: multiple inputs echoed in order ─────────────────────────────
  it("echoes multiple inputs in the order they were typed", async () => {
    // Arrange: three lines then quit.
    const out = await runParrot(["alpha", "beta", "gamma", ":quit"]);

    // Assert: all three inputs appear in output, in order.
    const joined = out.join("");
    const alphaIdx = joined.indexOf("alpha");
    const betaIdx  = joined.indexOf("beta");
    const gammaIdx = joined.indexOf("gamma");

    expect(alphaIdx).toBeGreaterThanOrEqual(0);
    expect(betaIdx).toBeGreaterThan(alphaIdx);
    expect(gammaIdx).toBeGreaterThan(betaIdx);
  });

  // ── Test 4: sync mode ───────────────────────────────────────────────────
  it("works correctly in sync mode", async () => {
    // Sync mode bypasses the setInterval animation and awaits eval directly.
    // The output should be identical to async mode.
    const out = await runParrot(["sync-input", ":quit"], "sync");

    expect(out).toContain("sync-input");
  });

  // ── Test 5: async mode ──────────────────────────────────────────────────
  it("works correctly in async mode (the default)", async () => {
    // Explicitly pass "async" to test the mode parameter explicitly.
    const out = await runParrot(["async-input", ":quit"], "async");

    expect(out).toContain("async-input");
  });

  // ── Test 6: banner contains "Parrot" ────────────────────────────────────
  it("displays a banner containing the word Parrot", async () => {
    // The globalPrompt is printed before every input, including the first.
    // We join all output and check that "Parrot" appears.
    const out = await runParrot([":quit"]);

    expect(out.join("")).toContain("Parrot");
  });

  // ── Test 7: line prompt contains parrot emoji ───────────────────────────
  it("ParrotPrompt.linePrompt contains the parrot emoji", () => {
    // Unit test the prompt directly — no REPL loop needed.
    const prompt = new ParrotPrompt();

    expect(prompt.linePrompt()).toContain("🦜");
  });

  // ── Test 8: EOF exits gracefully ────────────────────────────────────────
  it("exits gracefully when inputFn returns null (EOF)", async () => {
    // Passing only null simulates the user pressing Ctrl-D immediately.
    // The loop should exit without throwing.
    const out = await runParrot([null]);

    // The prompt was printed once before the null was consumed.
    // We just verify no error was thrown and prompt was shown.
    expect(out.join("")).toContain("Parrot");
  });

  // ── Test 9: empty string is echoed ─────────────────────────────────────
  it("echoes an empty string back (blank input is not ignored)", async () => {
    // EchoLanguage returns [:ok, ""] for empty input, which the loop prints.
    const out = await runParrot(["", ":quit"]);

    // The output array should contain an empty-string entry from the echo.
    // (The loop calls outputFn("") for output: "")
    expect(out).toContain("");
  });

  // ── Test 10: multiple echoes before quit ────────────────────────────────
  it("accumulates multiple echoes before quitting", async () => {
    const out = await runParrot(["one", "two", "three", ":quit"]);

    // Verify each echo appears in the collected output.
    expect(out).toContain("one");
    expect(out).toContain("two");
    expect(out).toContain("three");
  });

  // ── Test 11: ParrotPrompt.globalPrompt content ──────────────────────────
  it("ParrotPrompt.globalPrompt returns the expected banner text", () => {
    const prompt = new ParrotPrompt();
    const text = prompt.globalPrompt();

    // The banner must contain the program name and usage hint.
    expect(text).toContain("Parrot REPL");
    expect(text).toContain(":quit");
    expect(text).toContain("🦜");
  });

  // ── Test 12: ParrotPrompt.linePrompt format ─────────────────────────────
  it("ParrotPrompt.linePrompt returns a non-empty prompt string with '>'", () => {
    const prompt = new ParrotPrompt();
    const text = prompt.linePrompt();

    expect(text.length).toBeGreaterThan(0);
    expect(text).toContain(">");
  });

  // ── Test 13: session ends on :quit even with queued inputs ──────────────
  it("stops processing queued inputs once :quit is received", async () => {
    // ":quit" is the second input; "after-quit" should never be echoed.
    const out = await runParrot([":quit", "after-quit"]);

    expect(out).not.toContain("after-quit");
  });

  // ── Test 14: output collected correctly ─────────────────────────────────
  it("collects the exact number of output calls expected", async () => {
    // With two inputs ("ping" and ":quit"):
    //   - globalPrompt is called once before "ping" → 1 output call
    //   - "ping" is echoed → 1 output call
    //   - globalPrompt is called once before ":quit" → 1 output call
    //   - ":quit" produces no output (loop exits cleanly)
    // Total: 3 output calls.
    const out = await runParrot(["ping", ":quit"], "sync");

    expect(out.length).toBe(3);
    expect(out[0]).toContain("Parrot");  // first prompt
    expect(out[1]).toBe("ping");         // echoed input
    expect(out[2]).toContain("Parrot");  // second prompt
  });

  // ── Test 15: error results print "ERROR: ..." ───────────────────────────
  it("prints ERROR: prefix for error results from a failing language", async () => {
    // We need a language that returns an error result. Rather than using
    // EchoLanguage (which never errors), we create a minimal stub here.
    //
    // The TypeScript REPL loop prints "ERROR: " + message for error results
    // (see loop.ts). We verify this prefix is present.
    const output: string[] = [];

    // Failing language: always returns an error for any input.
    const failingLanguage = {
      async eval(_input: string) {
        return { tag: "error" as const, message: "something went wrong" };
      },
    };

    await runWithIo(
      failingLanguage,
      new ParrotPrompt(),
      new SilentWaiting(),
      // One input then EOF.
      (() => {
        const q = ["trigger", null];
        return async () => q.shift() ?? null;
      })(),
      (text: string) => output.push(text),
      "sync",
    );

    // The loop should have printed "ERROR: something went wrong".
    expect(output.join("")).toContain("ERROR: something went wrong");
  });
});
