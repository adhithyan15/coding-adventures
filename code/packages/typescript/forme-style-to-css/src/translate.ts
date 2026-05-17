/**
 * translate.ts — `translateToCss(doc, options)` (FM04 §9.2).
 *
 * The public entry point.  Walks `doc.rules` in declaration order,
 * filters by:
 *
 * 1. **Active contexts.**  A rule with a `context` field applies only
 *    if its context is in `options.activeContexts` (or has no
 *    context).  Standard contexts get wrapped in `@media`; `ext:*`
 *    contexts have no built-in mapping and emit a warning + skip.
 * 2. **Used-rule-ids (optional).**  If `options.usedRuleIds` is set,
 *    only rules whose id is in that list are emitted.  This is how
 *    FM06's AOT compiler does per-page CSS slicing.
 *
 * Each surviving rule produces a CSS rule block:
 *
 *     <scope?> <selector-css> {
 *       <decl>;
 *       <decl>;
 *     }
 *
 * Rules sharing a `@media` context are grouped under a single
 * `@media` block in declaration order — this matches what hand-
 * authored CSS looks like and avoids context-bleeding (interleaving
 * rules across contexts would change cascade semantics).
 *
 * Properties whose `TokenRef`s can't be resolved emit warnings and
 * are skipped (FM04 §9.6); unknown property kinds (other than
 * kernel-known) emit warnings and are skipped.  Translator never
 * throws on Style IR shape issues — that's the validator's job.
 *
 * Output is deterministic: same input → byte-identical bytes
 * (drives FM03 reproducible builds).
 *
 * @module translate
 */

import {
  isExtensionKind,
  type StyleDocument, type StyleRule, type StyleRuleId, type StyleWarning,
} from "@coding-adventures/forme-style-ir";
import { propertyToCss } from "./property-mappers.js";
import { selectorToCss } from "./selector-mapper.js";
import { contextToMedia } from "./context-mapper.js";

// ─── Public options + result ─────────────────────────────────────────────

/**
 * Translator options.
 */
export interface TranslateOptions {
  /**
   * Contexts the consumer wants active.  Rules with a `context`
   * field apply only if their context is in this list.  Rules with
   * NO context always apply.  Empty list ⇒ no contexts active (only
   * unconditional rules emit).
   */
  readonly activeContexts: readonly string[];
  /**
   * If set, ONLY rules whose id is in this list are emitted.  This
   * is the FM06 per-page CSS slicing input — the renderer
   * accumulates `usedStyle` ids per page; the AOT compiler hands
   * them here.  Order doesn't matter; ids not present in the rules
   * list are silently ignored.
   */
  readonly usedRuleIds?: readonly StyleRuleId[];
  /**
   * Optional CSS selector prefix applied to every selector.  Used by
   * per-page CSS scoping (`/page-abc123 .blockquote { ... }`).  The
   * prefix is inserted with a space, treating the scope as a CSS
   * descendant.  Empty string / undefined ⇒ no scoping.
   *
   * **Caller-trusted**: this value is concatenated verbatim into the
   * output without escaping.  Callers must supply a valid CSS
   * selector fragment (e.g. `"#page-abc123"`, `".my-scope"`).  The
   * AOT compiler that drives this stage produces the prefix from a
   * hash of the page route — already safe — but anyone wiring this
   * translator from a different source must escape themselves.
   */
  readonly scope?: string;
}

/**
 * Translator output.  `output` is the full CSS string;
 * `emittedRules` lists which rule ids actually made it (consumers
 * can intersect with their per-page `usedStyle`); `warnings` reports
 * everything that was skipped or degraded.
 */
export interface TranslateResult<Out> {
  readonly output: Out;
  readonly emittedRules: readonly StyleRuleId[];
  readonly warnings: readonly StyleWarning[];
}

/**
 * The big one.  Takes a `StyleDocument` and emits CSS plus metadata.
 */
export function translateToCss(
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

  // Bucket rules by context.  Order within each bucket is the
  // source-order index, preserving FM04 §4.9 specificity.
  //
  //   bucket key:
  //     ""       — unconditional (no context)
  //     "@media print"  — etc., one per kernel-known context
  //
  // We also emit ext:* contexts under a per-context bucket, but
  // they get warned-and-skipped at the moment because no
  // extension-context-to-CSS translator exists yet.  The bucket
  // collection keeps the path open for plugin extensions later.
  const buckets = new Map<string, RuleEmit[]>();
  buckets.set("", []);

  for (const rule of doc.rules) {
    if (usedSet !== null && !usedSet.has(rule.id)) continue;
    if (rule.context !== undefined && !activeContextSet.has(rule.context)) continue;

    let bucketKey = "";
    if (rule.context !== undefined) {
      const mediaBody = contextToMedia(rule.context);
      if (mediaBody === null) {
        // Unknown / ext: context → warn + skip.
        warnings.push({
          code: "EXT_CONTEXT_NOT_TRANSLATED",
          message: `context ${JSON.stringify(rule.context)} has no built-in CSS mapping; rule skipped`,
          ruleId: rule.id,
        });
        continue;
      }
      bucketKey = `@media ${mediaBody}`;
    }

    const block = emitRule(rule, doc, scope, warnings);
    if (block === null) continue;     // every property warned-and-skipped
    let bucket = buckets.get(bucketKey);
    if (!bucket) {
      bucket = [];
      buckets.set(bucketKey, bucket);
    }
    bucket.push({ ruleId: rule.id, block });
    emittedRules.push(rule.id);
  }

  // Assemble output.  Unconditional rules first (per FM04 §9.2's
  // "rules declared later win" semantics — keeping unconditional
  // rules above @media blocks preserves cascade behaviour in the
  // emitted CSS).  Then `@media` blocks in insertion order.
  const parts: string[] = [];
  const unconditional = buckets.get("") ?? [];
  for (const emit of unconditional) parts.push(emit.block);

  for (const [key, emits] of buckets) {
    if (key === "" || emits.length === 0) continue;
    const inner = emits.map((e) => indent(e.block, 2)).join("\n");
    parts.push(`${key} {\n${inner}\n}`);
  }

  // Join with blank lines for readability.  No trailing newline —
  // the file-level emitter adds one if it wants.
  const output = parts.join("\n\n");
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
 * Format one rule as a CSS rule block.  Returns null if every
 * property warned-and-skipped (don't emit empty rule blocks).
 */
function emitRule(
  rule: StyleRule,
  doc: StyleDocument,
  scope: string,
  warnings: StyleWarning[],
): string | null {
  const decls: string[] = [];

  for (const prop of rule.properties) {
    // Unknown ext: kinds → warn + skip.
    if (isExtensionKind(prop.kind)) {
      warnings.push({
        code: "EXT_PROPERTY_NOT_TRANSLATED",
        message: `property kind ${JSON.stringify(prop.kind)} has no built-in CSS mapping; skipped`,
        ruleId: rule.id,
        propertyKind: prop.kind,
      });
      continue;
    }

    const emit = propertyToCss(prop, doc.tokens);
    if (emit.ok) {
      decls.push(emit.declaration);
    } else {
      warnings.push({
        code: "PROPERTY_SKIPPED",
        message: emit.warning,
        ruleId: rule.id,
        propertyKind: prop.kind,
      });
    }
  }

  if (decls.length === 0) return null;

  const sel = selectorToCss(rule.selector);
  const scoped = scope.length > 0 ? scopedSelector(scope, sel) : sel;
  const body = decls.map((d) => `  ${d};`).join("\n");
  return `${scoped} {\n${body}\n}`;
}

/**
 * Apply a scope prefix to a CSS selector.  Comma-separated paths
 * (from `or` selectors) each get the scope applied independently.
 *
 *   scope="#page-1"  sel="p, blockquote"  →  "#page-1 p, #page-1 blockquote"
 */
function scopedSelector(scope: string, sel: string): string {
  return sel.split(",").map((s) => `${scope} ${s.trim()}`).join(", ");
}

/**
 * Indent every line of `s` by `n` spaces.  Used for `@media`
 * blocks' inner content.
 */
function indent(s: string, n: number): string {
  const pad = " ".repeat(n);
  return s.split("\n").map((line) => pad + line).join("\n");
}
