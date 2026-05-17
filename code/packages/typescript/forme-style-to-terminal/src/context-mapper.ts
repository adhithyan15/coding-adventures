/**
 * context-mapper.ts — context name → terminal-relevance verdict (FM04 §9.4).
 *
 * Terminals don't have CSS-style `@media` queries or LaTeX-style
 * `\if<flag>` conditionals — the terminal IS what it is at render
 * time.  Context handling is therefore *filter-only*: the translator
 * keeps a rule if its context is in `activeContexts`, drops it
 * otherwise.  No per-bucket emission machinery.
 *
 * Of the seven kernel contexts:
 *
 *   - **screen** — terminals are always "screen".  Always relevant.
 *   - **dark** / **high-contrast** — terminals may be either; the
 *     consumer says so via `activeContexts`.  Relevant when active.
 *   - **print** / **narrow** / **wide** / **reduced-motion** — no
 *     terminal equivalent.  If a rule carries one of these contexts
 *     and the context is in `activeContexts`, we honour the request
 *     and emit the rule; if not, we drop it (same as any context).
 *     We don't emit a warning — the caller knows what they're doing
 *     by listing them.
 *   - **ext:\*** — warn-skip per FM04 §9.6 (translator doesn't know
 *     the extension's semantics).
 *
 * So `contextToTerminal(name)` returns true for kernel contexts and
 * null for `ext:*`.  The translator's filter logic is the standard
 * `activeContexts.has(rule.context)` check; this module exists
 * mainly to flag ext: contexts for warn-skip.
 *
 * @module context-mapper
 */

import {
  isExtensionContext, isRecognisedContext,
} from "@coding-adventures/forme-style-ir";

/**
 * Decide whether a context name is recognised at the terminal
 * translator level.  Returns true for any kernel-blessed context,
 * false for `ext:*` (caller warn-skips).
 */
export function contextRecognised(name: string): boolean {
  if (isExtensionContext(name)) return false;
  return isRecognisedContext(name);
}
