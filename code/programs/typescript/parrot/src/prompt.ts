/**
 * prompt.ts — the personality layer for the Parrot REPL.
 *
 * The `Prompt` interface (from @coding-adventures/repl) defines two methods:
 *
 *   globalPrompt() — shown before the user types each new line.
 *   linePrompt()   — shown when a multi-line expression is being continued.
 *
 * ParrotPrompt gives the REPL its character: a parrot repeats everything you
 * say, so the prompts use parrot emoji and parrot-themed text to reinforce
 * the theme.
 *
 * ## Why a separate file?
 *
 * Putting the prompt in its own module makes it independently testable. The
 * tests can import ParrotPrompt directly and verify its text without running
 * the full REPL loop. This follows the single-responsibility principle: each
 * file has one reason to change.
 */

import type { Prompt } from "@coding-adventures/repl";

/**
 * ParrotPrompt implements the Prompt interface with parrot-themed strings.
 *
 * It is intentionally minimal: all it does is return static strings. The
 * interesting behaviour (echoing, quitting) lives in EchoLanguage.
 *
 * ## Prompt text design
 *
 * `globalPrompt()` returns a two-line banner followed by a blank line:
 *
 *   🦜 Parrot REPL
 *   I repeat everything you say! Type :quit to exit.
 *
 * This banner is printed before every input line. In a real terminal this
 * would be distracting — you'd normally only print it once at startup. For
 * a demo REPL, printing it each turn keeps the output consistent and makes
 * each test case self-contained.
 *
 * `linePrompt()` returns the short inline prompt "🦜 > " that appears on
 * every new line. The emoji acts as a visual anchor that ties the prompt
 * back to the parrot theme.
 */
export class ParrotPrompt implements Prompt {
  /**
   * globalPrompt — the banner shown before each new input.
   *
   * Returns a multi-line string ending with "\n\n" so there's a blank line
   * between the banner and any output from the previous evaluation. The
   * trailing newlines are included in the prompt text (not added by the loop)
   * because the loop writes prompt strings verbatim with no added whitespace.
   *
   * @returns A parrot-themed banner string.
   */
  globalPrompt(): string {
    return "🦜 Parrot REPL\nI repeat everything you say! Type :quit to exit.\n\n";
  }

  /**
   * linePrompt — the continuation prompt for multi-line input.
   *
   * EchoLanguage never produces multi-line sessions, so this prompt is
   * provided for completeness. It matches the parrot theme with the same
   * emoji and a ">" separator.
   *
   * @returns A short inline prompt string.
   */
  linePrompt(): string {
    return "🦜 > ";
  }
}
