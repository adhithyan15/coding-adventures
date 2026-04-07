/**
 * DefaultPrompt — conventional "> " and "... " prompt strings.
 *
 * The choice of `> ` as a primary prompt and `... ` as a continuation prompt
 * is a near-universal convention in interactive interpreters:
 *
 *   Language      Primary   Continuation
 *   ──────────    ───────   ────────────
 *   Python        >>>       ...
 *   Node.js       >         ...
 *   Ruby          irb(main) irb(main)*
 *   Haskell GHCi  Prelude>  Prelude|
 *   Elixir iex    iex(1)>   ...(1)>
 *
 * This implementation uses single `>` (space) because it's the most
 * recognisable minimal form — what you'd see in a Node.js or Deno shell.
 */

import type { Prompt } from "./types.js";

/**
 * A Prompt that uses `"> "` as the global prompt and `"... "` as the line
 * (continuation) prompt.
 *
 * @example
 * const prompt = new DefaultPrompt();
 * prompt.globalPrompt(); // ">"
 * prompt.linePrompt();   // "... "
 */
export class DefaultPrompt implements Prompt {
  /**
   * The primary prompt displayed at the start of each new expression.
   *
   * The trailing space separates the sigil from the user's cursor, improving
   * readability when the terminal echoes back typed characters.
   */
  globalPrompt(): string {
    return "> ";
  }

  /**
   * The continuation prompt displayed when a multi-line expression is
   * incomplete and more lines are expected.
   *
   * `"... "` visually suggests "I'm waiting for more" while the indentation
   * (three dots + space) aligns continuation text slightly to the right of
   * the primary prompt, providing a subtle visual hierarchy.
   */
  linePrompt(): string {
    return "... ";
  }
}
