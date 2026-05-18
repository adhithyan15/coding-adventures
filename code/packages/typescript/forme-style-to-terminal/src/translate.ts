/**
 * translate.ts — `translateToTerminal(doc, options)` (FM04 §9.4).
 *
 * The public entry point.  Walks `doc.rules` and emits a TypeScript
 * module source string shaped like:
 *
 *   // forme-style-to-terminal generated
 *   //
 *   // Per-rule ANSI SGR wrappers.  Consumers look up by rule id and
 *   // emit `entry.prefix + content + entry.suffix` around the
 *   // styled text.
 *
 *   export interface AnsiStyle {
 *     readonly prefix: string;   // SGR setting style
 *     readonly suffix: string;   // SGR reset (always \x1b[0m)
 *   }
 *
 *   export const formeStyles: ReadonlyMap<string, AnsiStyle> = new Map([
 *     // rule "body" — node-type:paragraph
 *     ["body", { prefix: "\x1b[38;2;31;35;40m", suffix: "\x1b[0m" }],
 *     // rule "headline" — heading-level:1
 *     ["headline", { prefix: "\x1b[1;38;2;9;105;218m", suffix: "\x1b[0m" }],
 *   ]);
 *
 * Filtering is the same shape as the CSS / LaTeX translators:
 *
 *   1. `activeContexts` — rules with a `context` field apply only
 *      when their context is in the active set (or has no context).
 *      Unknown / `ext:*` contexts WARN + SKIP.
 *   2. `usedRuleIds` — when set, only those rule ids emit (FM06
 *      per-page slicing).
 *
 * Per-property warn-skips don't fail the rule — the rule still
 * emits with whatever SGR fragments did succeed.  A rule with NO
 * successful SGR fragments (every property warn-skipped, or all
 * succeeded but contributed no SGR) emits with an empty prefix
 * (`prefix: ""`) — the consumer's wrapping is then a no-op.
 *
 * **Security**: all caller-controlled strings (rule ids, selector
 * descriptions) route through `escapeTsString` so neither a raw
 * ANSI ESC nor a TS-string-literal escape (`\` / `"`) can land in
 * the output.  Colour values are integer SGR parameters — safe by
 * construction.
 *
 * @module translate
 */

import {
  isExtensionKind,
  type StyleDocument, type StyleRule, type StyleRuleId, type StyleWarning,
} from "@coding-adventures/forme-style-ir";
import { propertyToTerminal } from "./property-mappers.js";
import { selectorDescription } from "./selector-mapper.js";
import { contextRecognised } from "./context-mapper.js";
import { escapeTsString, sanitiseKey } from "./escape.js";

// ─── Public options + result ─────────────────────────────────────────────

/**
 * Translator options.  Same shape as the CSS / LaTeX translators —
 * redeclared locally per FM04 §9.1.
 */
export interface TranslateOptions {
  readonly activeContexts: readonly string[];
  readonly usedRuleIds?: readonly StyleRuleId[];
  /**
   * Optional prefix string concatenated in front of every Map key.
   * Used by per-page scoping (e.g. `scope = "page-abc123."` →
   * `"page-abc123.body"`).
   *
   * **Caller-trusted**: this value is escaped for the TS string
   * literal (the standard `\` / `"` escapes apply) but is otherwise
   * concatenated verbatim.  The AOT compiler that drives this stage
   * derives the prefix from a hash of the page route — already safe;
   * anyone wiring this translator from a different source must
   * sanitise themselves.
   */
  readonly scope?: string;
}

/**
 * Translator output.  `output` is the TS module source string;
 * `emittedRules` lists which rule ids actually made it.
 */
export interface TranslateResult<Out> {
  readonly output: Out;
  readonly emittedRules: readonly StyleRuleId[];
  readonly warnings: readonly StyleWarning[];
}

/**
 * Translate a `StyleDocument` into a TypeScript module source string
 * exporting a `Map<RuleId, AnsiStyle>`.
 */
export function translateToTerminal(
  doc: StyleDocument,
  options: TranslateOptions,
): TranslateResult<string> {
  const activeContextSet = new Set(options.activeContexts);
  const usedSet = options.usedRuleIds === undefined
    ? null
    : new Set<string>(options.usedRuleIds);
  const scope = options.scope ?? "";

  const warnings: StyleWarning[] = [];
  const emittedRules: StyleRuleId[] = [];
  const entries: string[] = [];

  for (const rule of doc.rules) {
    if (usedSet !== null && !usedSet.has(rule.id)) continue;

    if (rule.context !== undefined) {
      if (!contextRecognised(rule.context)) {
        warnings.push({
          code: "EXT_CONTEXT_NOT_TRANSLATED",
          message: `context ${JSON.stringify(rule.context)} has no built-in terminal mapping; rule skipped`,
          ruleId: rule.id,
        });
        continue;
      }
      if (!activeContextSet.has(rule.context)) continue;
    }

    const sgrFragments: string[] = [];
    for (const prop of rule.properties) {
      if (isExtensionKind(prop.kind)) {
        warnings.push({
          code: "EXT_PROPERTY_NOT_TRANSLATED",
          message: `property kind ${JSON.stringify(prop.kind)} has no built-in terminal mapping; skipped`,
          ruleId: rule.id,
          propertyKind: prop.kind,
        });
        continue;
      }

      const emit = propertyToTerminal(prop, doc.tokens);
      if (emit.ok) {
        for (const sgr of emit.sgr) sgrFragments.push(sgr);
      } else {
        warnings.push({
          code: "PROPERTY_SKIPPED",
          message: emit.warning,
          ruleId: rule.id,
          propertyKind: prop.kind,
        });
      }
    }

    // A rule that warn-skipped EVERYTHING still emits — with empty
    // prefix — so consumers can look it up and get a no-op wrap.
    // This matches the CSS/LaTeX translators' "emit-if-anything-
    // survived" rule INVERTED for terminal because Map lookup
    // failing would be more confusing than a no-op.  We still don't
    // add the id to emittedRules when there's nothing to emit —
    // that signals "we acknowledged but had nothing".
    const prefix = sgrFragments.length === 0
      ? ""
      : `\\x1b[${sgrFragments.join(";")}m`;
    const suffix = sgrFragments.length === 0 ? "" : "\\x1b[0m";

    // Only count rules that produced *some* output in emittedRules.
    if (sgrFragments.length > 0) emittedRules.push(rule.id);

    const safeKey   = sanitiseKey(`${scope}${rule.id}`);
    const safeDesc  = escapeTsString(selectorDescription(rule.selector));
    const safeRuleId = escapeTsString(rule.id);
    entries.push(
      `  // rule "${safeRuleId}" — ${safeDesc}\n` +
      `  ["${safeKey}", { prefix: "${prefix}", suffix: "${suffix}" }],`,
    );
  }

  const output = [
    "// forme-style-to-terminal generated",
    "//",
    "// Per-rule ANSI SGR wrappers.  Consumers look up by rule id and",
    "// emit `entry.prefix + content + entry.suffix` around the styled",
    "// text.  The prefix/suffix pair forms one SGR \"set then reset\".",
    "",
    "export interface AnsiStyle {",
    "  readonly prefix: string;",
    "  readonly suffix: string;",
    "}",
    "",
    "export const formeStyles: ReadonlyMap<string, AnsiStyle> = new Map([",
    ...entries,
    "]);",
    "",
  ].join("\n");

  return {
    output,
    emittedRules: Object.freeze(emittedRules),
    warnings: Object.freeze(warnings),
  };
}
