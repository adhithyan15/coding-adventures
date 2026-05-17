/**
 * translate.ts — `translateToLatex(doc, options)` (FM04 §9.3).
 *
 * The public entry point.  Walks `doc.rules` in declaration order
 * and emits a LaTeX preamble fragment shaped like:
 *
 *   % forme-style-to-latex generated preamble
 *   % --- context flags ---
 *   \newif\ifprint \newif\ifscreen ... etc
 *
 *   % --- rules ---
 *   % rule "body" — selector: node-type paragraph
 *   \newcommand{\formeNodeParagraph}{%
 *     \color{RGB}{31,35,40}%
 *     \fontsize{12pt}{14.4pt}\selectfont%
 *   }
 *
 *   % rule "headline-print" — selector: heading level 1, context: print
 *   \ifprint
 *   \newcommand{\formeHeadingOne}{%
 *     \fontsize{18pt}{21.6pt}\selectfont%
 *   }
 *   \fi
 *
 * Filtering (mirrors the CSS translator):
 *
 * 1. `activeContexts` — rules with a `context` field apply only when
 *    their context is in the active set.  Rules without `context`
 *    always apply.  Unknown / `ext:*` contexts WARN + SKIP.
 * 2. `usedRuleIds` — when set, only rules whose id is in the list
 *    are emitted (per-page slicing input from FM06's AOT compiler).
 *
 * Rules whose selector has no LaTeX preamble equivalent
 * (composition / structural selectors — `and`, `or`, `not`,
 * `nth`, `child-of`, `descendant-of`, `adjacent`) emit a warning
 * and are skipped per FM04 §9.6.
 *
 * Properties that warn-skip don't fail the rule — the rule still
 * emits with whatever properties did succeed.  A rule with zero
 * successful properties is suppressed (no empty `\newcommand`).
 *
 * Output is deterministic: same input → byte-identical bytes,
 * driving FM03 reproducible builds.
 *
 * @module translate
 */

import {
  isExtensionKind,
  type StyleDocument, type StyleRule, type StyleRuleId, type StyleWarning,
} from "@coding-adventures/forme-style-ir";
import { propertyToLatex } from "./property-mappers.js";
import { selectorToLatex } from "./selector-mapper.js";
import { contextToLatex, CONTEXT_FLAG_DECLARATIONS } from "./context-mapper.js";
import { escapeLatexText } from "./escape.js";

// ─── Public options + result ─────────────────────────────────────────────

/**
 * Translator options.  Same shape as `forme-style-to-css`'s
 * `TranslateOptions` — redeclared locally per FM04 §9.1 (each
 * translator owns its own `TranslateOptions` / `TranslateResult`
 * rather than importing from a shared package, avoiding a circular
 * dependency for a single-method contract).
 */
export interface TranslateOptions {
  readonly activeContexts: readonly string[];
  readonly usedRuleIds?: readonly StyleRuleId[];
  /**
   * Optional LaTeX command-name prefix applied to every emitted
   * macro.  Used by per-page scoping (`{\page<hash> \formeNodeParagraph
   * ...}`).  The prefix is concatenated verbatim — caller must
   * supply a valid LaTeX command-name fragment (`\foo`, no spaces,
   * no specials).  Empty / undefined ⇒ no scoping.
   *
   * **Caller-trusted**: this value is not escaped.  The AOT compiler
   * derives the prefix from a hash of the page route — already safe;
   * anyone wiring this translator from a different source must
   * escape themselves.
   */
  readonly scope?: string;
}

/**
 * Translator output.  `output` is the LaTeX preamble fragment;
 * `emittedRules` lists which rule ids actually made it (consumers
 * intersect with their per-page `usedStyle`); `warnings` reports
 * everything skipped or degraded.
 */
export interface TranslateResult<Out> {
  readonly output: Out;
  readonly emittedRules: readonly StyleRuleId[];
  readonly warnings: readonly StyleWarning[];
}

/**
 * Translate a `StyleDocument` into a LaTeX preamble fragment.
 */
export function translateToLatex(
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

  // Bucket rules by context — same shape as the CSS translator.
  //   "" → unconditional
  //   "\\ifprint" → guarded by \ifprint ... \fi
  const buckets = new Map<string, RuleEmit[]>();
  buckets.set("", []);

  for (const rule of doc.rules) {
    if (usedSet !== null && !usedSet.has(rule.id)) continue;
    if (rule.context !== undefined && !activeContextSet.has(rule.context)) continue;

    let bucketKey = "";
    if (rule.context !== undefined) {
      const cond = contextToLatex(rule.context);
      if (cond === null) {
        warnings.push({
          code: "EXT_CONTEXT_NOT_TRANSLATED",
          message: `context ${JSON.stringify(rule.context)} has no built-in LaTeX mapping; rule skipped`,
          ruleId: rule.id,
        });
        continue;
      }
      bucketKey = cond;
    }

    const block = emitRule(rule, doc, scope, warnings);
    if (block === null) continue;     // every property warn-skipped or
                                      // selector unmappable
    let bucket = buckets.get(bucketKey);
    if (!bucket) {
      bucket = [];
      buckets.set(bucketKey, bucket);
    }
    bucket.push({ ruleId: rule.id, block });
    emittedRules.push(rule.id);
  }

  // Assemble output.
  const parts: string[] = [];
  parts.push("% forme-style-to-latex generated preamble");
  parts.push("");
  parts.push("% --- context flags ---");
  for (const decl of CONTEXT_FLAG_DECLARATIONS) parts.push(decl);
  parts.push("");
  parts.push("% --- rules ---");

  // Unconditional first.
  const unconditional = buckets.get("") ?? [];
  for (const emit of unconditional) parts.push(emit.block);

  // Then context-guarded buckets in insertion order.
  for (const [key, emits] of buckets) {
    if (key === "" || emits.length === 0) continue;
    parts.push("");
    parts.push(key);
    for (const emit of emits) parts.push(emit.block);
    parts.push("\\fi");
  }

  const output = parts.join("\n");
  return {
    output,
    emittedRules: Object.freeze(emittedRules),
    warnings: Object.freeze(warnings),
  };
}

// ─── helpers ─────────────────────────────────────────────────────────────

interface RuleEmit {
  readonly ruleId: StyleRuleId;
  readonly block: string;
}

/**
 * Format one rule as `% comment\n\newcommand{<macro>}{<body>}`.
 * Returns null if every property warn-skipped, or if the selector
 * itself has no LaTeX equivalent.
 */
function emitRule(
  rule: StyleRule,
  doc: StyleDocument,
  scope: string,
  warnings: StyleWarning[],
): string | null {
  const selEmit = selectorToLatex(rule.selector);
  if (!selEmit.ok) {
    warnings.push({
      code: "SELECTOR_SKIPPED",
      message: selEmit.warning,
      ruleId: rule.id,
    });
    return null;
  }

  const commands: string[] = [];
  for (const prop of rule.properties) {
    if (isExtensionKind(prop.kind)) {
      warnings.push({
        code: "EXT_PROPERTY_NOT_TRANSLATED",
        message: `property kind ${JSON.stringify(prop.kind)} has no built-in LaTeX mapping; skipped`,
        ruleId: rule.id,
        propertyKind: prop.kind,
      });
      continue;
    }

    const emit = propertyToLatex(prop, doc.tokens);
    if (emit.ok) {
      // `prop.important` has no LaTeX equivalent.  We trail a comment
      // on the line so traceability survives.
      const importantTrailer = prop.important ? "  % !important" : "";
      commands.push(`  ${emit.commands}%${importantTrailer}`);
    } else {
      warnings.push({
        code: "PROPERTY_SKIPPED",
        message: emit.warning,
        ruleId: rule.id,
        propertyKind: prop.kind,
      });
    }
  }

  if (commands.length === 0) return null;

  // Apply scope by prepending the scope command to the macro name.
  // scope="\\Scope" + macro="\\formeNodeParagraph" → "\\Scope\\formeNodeParagraph"
  // (Caller is responsible for `scope` being a well-formed LaTeX
  //  command-name fragment.)
  const macroName = scope.length > 0
    ? `${scope}${selEmit.macroName}`
    : selEmit.macroName;

  // Escape rule.id for the comment (it's a branded string but
  // theoretically attacker-controllable via a hand-rolled IR).
  const safeId = escapeLatexText(rule.id);
  // The selector description is safe-by-construction in
  // selector-mapper but escape defensively all the same.
  const safeDesc = escapeLatexText(selEmit.description);

  return [
    `% rule "${safeId}" — ${safeDesc}`,
    `\\newcommand{${macroName}}{%`,
    ...commands,
    `}`,
  ].join("\n");
}
