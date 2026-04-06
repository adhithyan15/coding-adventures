/**
 * EchoLanguage — the simplest possible Language implementation.
 *
 * An "echo" REPL does exactly what it says: it reads your input and prints it
 * back unchanged. This sounds trivial, but it is genuinely useful as:
 *
 * 1. A smoke-test harness — if EchoLanguage works end-to-end, your loop wiring
 *    is correct before you integrate a real evaluator.
 * 2. A reference implementation — developers building their own Language can
 *    copy-paste this file and extend it.
 * 3. A default fallback — a REPL framework ships with *something* runnable.
 *
 * Special command: `:quit`
 * ------------------------
 * EchoLanguage recognises the single built-in command `:quit` and returns
 * `{tag: "quit"}`. This convention (colon-prefixed meta-commands) is borrowed
 * from Haskell's GHCi, where `:q`, `:type`, and `:info` are REPL directives
 * that the evaluator handles before passing code to the compiler.
 */

import type { EvalResult, Language } from "./types.js";

/**
 * A Language that echoes every input back as output.
 *
 * - `:quit` → `{tag: "quit"}` (ends the session)
 * - anything else → `{tag: "ok", output: input}`
 *
 * @example
 * const lang = new EchoLanguage();
 * await lang.eval("hello");  // → { tag: "ok", output: "hello" }
 * await lang.eval(":quit");  // → { tag: "quit" }
 */
export class EchoLanguage implements Language {
  /**
   * Evaluate a single line of input.
   *
   * The method is declared `async` so it satisfies the `Language` interface
   * (which requires a `Promise<EvalResult>`) even though no I/O is actually
   * awaited. TypeScript wraps the return value in a resolved Promise for us.
   */
  async eval(input: string): Promise<EvalResult> {
    if (input === ":quit") {
      // The user has asked to end the session.
      return { tag: "quit" };
    }

    // Echo the input back unchanged. An "echo" language never produces an
    // error — every string is valid input by definition.
    return { tag: "ok", output: input };
  }
}
