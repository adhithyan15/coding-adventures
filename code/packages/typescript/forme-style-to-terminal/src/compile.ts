import {
  isExtensionKind,
  type StyleDocument,
  type StyleRuleId,
  type StyleWarning,
} from "@coding-adventures/forme-style-ir";
import type { TranslateOptions } from "./translate.js";
import { contextRecognised } from "./context-mapper.js";
import { propertyToTerminal } from "./property-mappers.js";

export interface AnsiStyle {
  readonly prefix: string;
  readonly suffix: string;
}

export interface CompileTerminalResult {
  readonly styles: ReadonlyMap<string, AnsiStyle>;
  readonly emittedRules: readonly StyleRuleId[];
  readonly warnings: readonly StyleWarning[];
}

/** Compile per-rule ANSI wrappers for a runtime renderer. */
export function compileTerminalStyles(
  doc: StyleDocument,
  options: TranslateOptions,
): CompileTerminalResult {
  const activeContexts = new Set(options.activeContexts);
  const used = options.usedRuleIds === undefined ? null : new Set<string>(options.usedRuleIds);
  const scope = options.scope ?? "";
  const styles = new Map<string, AnsiStyle>();
  const emittedRules: StyleRuleId[] = [];
  const warnings: StyleWarning[] = [];

  for (const rule of doc.rules) {
    if (used !== null && !used.has(rule.id)) continue;
    if (rule.context !== undefined) {
      if (!contextRecognised(rule.context)) {
        warnings.push({
          code: "EXT_CONTEXT_NOT_TRANSLATED",
          message: `context ${JSON.stringify(rule.context)} has no built-in terminal mapping; rule skipped`,
          ruleId: rule.id,
        });
        continue;
      }
      if (!activeContexts.has(rule.context)) continue;
    }

    const fragments: string[] = [];
    for (const property of rule.properties) {
      if (isExtensionKind(property.kind)) {
        warnings.push({
          code: "EXT_PROPERTY_NOT_TRANSLATED",
          message: `property kind ${JSON.stringify(property.kind)} has no built-in terminal mapping; skipped`,
          ruleId: rule.id,
          propertyKind: property.kind,
        });
        continue;
      }
      const result = propertyToTerminal(property, doc.tokens);
      if (result.ok) fragments.push(...result.sgr);
      else warnings.push({
        code: "PROPERTY_SKIPPED",
        message: result.warning,
        ruleId: rule.id,
        propertyKind: property.kind,
      });
    }

    const style = fragments.length === 0
      ? Object.freeze({ prefix: "", suffix: "" })
      : Object.freeze({ prefix: `\u001b[${fragments.join(";")}m`, suffix: "\u001b[0m" });
    // This is an in-memory Map, not generated TypeScript source. Preserve the
    // exact key so runtime consumers can look it up by the original rule id.
    styles.set(`${scope}${rule.id}`, style);
    if (fragments.length > 0) emittedRules.push(rule.id);
  }

  return Object.freeze({
    styles,
    emittedRules: Object.freeze(emittedRules),
    warnings: Object.freeze(warnings),
  });
}
