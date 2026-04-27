/**
 * compiler.ts — Compile TokenGrammar and ParserGrammar into TypeScript source.
 *
 * The grammar-tools library parses .tokens and .grammar files into in-memory
 * data structures. This module adds the *compile* step: given a parsed grammar
 * object, generate TypeScript source code that embeds the grammar as native
 * TypeScript object literals — no file I/O or parsing at runtime.
 *
 * Why compile grammars?
 * ---------------------
 *
 * The default workflow reads .tokens and .grammar files at startup. This has
 * three costs that compilation eliminates:
 *
 *   1. File I/O at startup — every process must find and open the files.
 *      Packages walk up the directory tree to find code/grammars/, which
 *      couples them to the repo layout.
 *
 *   2. Parse overhead at startup — the grammar is re-parsed every run.
 *
 *   3. Deployment coupling — .tokens and .grammar files must ship alongside
 *      the compiled bundle.
 *
 * The generated TypeScript file directly exports `TOKEN_GRAMMAR` or
 * `PARSER_GRAMMAR` as typed object literals and can be imported like any
 * other module.
 *
 * Generated output shape (json.tokens → json-tokens.ts):
 *
 *   // AUTO-GENERATED FILE — DO NOT EDIT
 *   // Source: json.tokens
 *
 *   import type { TokenGrammar } from "@coding-adventures/grammar-tools";
 *
 *   export const TOKEN_GRAMMAR: TokenGrammar = {
 *     version: 1,
 *     caseInsensitive: false,
 *     definitions: [
 *       { name: "STRING", pattern: "...", isRegex: true, lineNumber: 1 },
 *     ],
 *     keywords: [],
 *     skipDefinitions: [],
 *   };
 *
 * Design note: all exported functions are pure (no side effects). The caller
 * is responsible for writing the returned string to disk.
 */

import type {
  TokenDefinition,
  PatternGroup,
  TokenGrammar,
} from "./token-grammar.js";
import type {
  GrammarElement,
  GrammarRule,
  ParserGrammar,
} from "./parser-grammar.js";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Generate TypeScript source code embedding a TokenGrammar as native data.
 *
 * @param grammar    - A TokenGrammar object to compile.
 * @param sourceFile - The original .tokens filename for the header comment.
 *                     Pass "" to omit the Source line.
 * @returns A string of valid TypeScript source code. Write it to a .ts file.
 */
export function compileTokenGrammar(
  grammar: TokenGrammar,
  sourceFile = ""
): string {
  // Strip newlines so a crafted filename cannot break out of the comment line
  // and inject arbitrary code into the generated file.
  sourceFile = sourceFile.replace(/[\r\n]/g, "_");
  const sourceLine = sourceFile
    ? `// Source: ${sourceFile}\n`
    : "";

  const defsLit = tokenDefListLit(grammar.definitions, "  ");
  const skipLit = tokenDefListLit(grammar.skipDefinitions ?? [], "  ");
  const errLit = tokenDefListLit((grammar as any).errorDefinitions ?? [], "  ");
  const groupsLit = groupsObjLit(grammar.groups ?? {}, "  ");

  return [
    "// AUTO-GENERATED FILE \u2014 DO NOT EDIT",
    sourceLine + `// Regenerate with: grammar-tools compile-tokens ${sourceFile}`,
    "//",
    "// This file embeds a TokenGrammar as native TypeScript object literals.",
    "// Import it directly instead of reading and parsing the .tokens file at",
    "// runtime.",
    "",
    'import type { TokenGrammar } from "@coding-adventures/grammar-tools";',
    "",
    "export const TOKEN_GRAMMAR: TokenGrammar = {",
    `  version: ${grammar.version},`,
    `  caseInsensitive: ${grammar.caseInsensitive},`,
    `  caseSensitive: ${(grammar as any).caseSensitive ?? true},`,
    `  definitions: ${defsLit},`,
    `  keywords: ${JSON.stringify(grammar.keywords)},`,
    `  mode: ${grammar.mode !== undefined ? JSON.stringify(grammar.mode) : "undefined"},`,
    `  escapeMode: ${grammar.escapeMode !== undefined ? JSON.stringify(grammar.escapeMode) : "undefined"},`,
    `  skipDefinitions: ${skipLit},`,
    `  reservedKeywords: ${JSON.stringify(grammar.reservedKeywords ?? [])},`,
    `  layoutKeywords: ${JSON.stringify(grammar.layoutKeywords ?? [])},`,
    `  contextKeywords: ${JSON.stringify(grammar.contextKeywords ?? [])},`,
    `  errorDefinitions: ${errLit},`,
    `  groups: ${groupsLit},`,
    "};",
    "",
  ].join("\n");
}

/**
 * Generate TypeScript source code embedding a ParserGrammar as native data.
 *
 * @param grammar    - A ParserGrammar object to compile.
 * @param sourceFile - The original .grammar filename for the header comment.
 * @returns A string of valid TypeScript source code.
 */
export function compileParserGrammar(
  grammar: ParserGrammar,
  sourceFile = ""
): string {
  // Strip newlines so a crafted filename cannot break out of the comment line.
  sourceFile = sourceFile.replace(/[\r\n]/g, "_");
  const sourceLine = sourceFile
    ? `// Source: ${sourceFile}\n`
    : "";

  const rulesLit =
    grammar.rules.length === 0
      ? "[]"
      : "[\n" +
        grammar.rules.map((r) => grammarRuleLit(r, "  ")).join(",\n") +
        ",\n]";

  return [
    "// AUTO-GENERATED FILE \u2014 DO NOT EDIT",
    sourceLine + `// Regenerate with: grammar-tools compile-grammar ${sourceFile}`,
    "//",
    "// This file embeds a ParserGrammar as native TypeScript object literals.",
    "// Import it directly instead of reading and parsing the .grammar file at",
    "// runtime.",
    "",
    'import type { ParserGrammar } from "@coding-adventures/grammar-tools";',
    "",
    "export const PARSER_GRAMMAR: ParserGrammar = {",
    `  version: ${grammar.version},`,
    `  rules: ${rulesLit},`,
    "};",
    "",
  ].join("\n");
}

// ---------------------------------------------------------------------------
// Token grammar helpers
// ---------------------------------------------------------------------------

/**
 * Render one TokenDefinition as a TypeScript object literal.
 *
 * @param defn   - The token definition to render.
 * @param indent - Current indentation prefix.
 */
function tokenDefLit(defn: TokenDefinition, indent: string): string {
  const i = indent + "  ";
  const aliasLine = defn.alias !== undefined
    ? `\n${i}alias: ${JSON.stringify(defn.alias)},`
    : "";
  return (
    `${indent}{\n` +
    `${i}name: ${JSON.stringify(defn.name)},\n` +
    `${i}pattern: ${JSON.stringify(defn.pattern)},\n` +
    `${i}isRegex: ${defn.isRegex},\n` +
    `${i}lineNumber: ${defn.lineNumber},` +
    aliasLine +
    `\n${indent}}`
  );
}

/**
 * Render a list of TokenDefinitions as a TypeScript array literal.
 *
 * @param defs   - The list of token definitions.
 * @param indent - Current indentation prefix.
 */
function tokenDefListLit(
  defs: readonly TokenDefinition[],
  indent: string
): string {
  if (defs.length === 0) return "[]";
  const inner = indent + "  ";
  const items = defs.map((d) => tokenDefLit(d, inner)).join(",\n");
  return `[\n${items},\n${indent}]`;
}

/**
 * Render the groups map as a TypeScript object literal.
 *
 * @param groups - The pattern groups map.
 * @param indent - Current indentation prefix.
 */
function groupsObjLit(
  groups: Readonly<Record<string, PatternGroup>>,
  indent: string
): string {
  const keys = Object.keys(groups);
  if (keys.length === 0) return "{}";
  const inner = indent + "  ";
  const entries = keys.map((name) => {
    const group = groups[name];
    const defsLit = tokenDefListLit(group.definitions, inner + "  ");
    return (
      `${inner}${JSON.stringify(name)}: {\n` +
      `${inner}  name: ${JSON.stringify(group.name)},\n` +
      `${inner}  definitions: ${defsLit},\n` +
      `${inner}}`
    );
  });
  return `{\n${entries.join(",\n")},\n${indent}}`;
}

// ---------------------------------------------------------------------------
// Parser grammar helpers
// ---------------------------------------------------------------------------

/**
 * Render one GrammarRule as a TypeScript object literal.
 *
 * @param rule   - The grammar rule to render.
 * @param indent - Current indentation prefix.
 */
function grammarRuleLit(rule: GrammarRule, indent: string): string {
  const i = indent + "  ";
  const bodyLit = elementLit(rule.body, i);
  return (
    `${indent}{\n` +
    `${i}name: ${JSON.stringify(rule.name)},\n` +
    `${i}body: ${bodyLit},\n` +
    `${i}lineNumber: ${rule.lineNumber},\n` +
    `${indent}}`
  );
}

/**
 * Recursively render a GrammarElement as a TypeScript object literal.
 *
 * GrammarElement is a discriminated union — we switch on `element.type` to
 * choose the correct rendering for each node kind.
 *
 * @param element - The grammar element to render.
 * @param indent  - Current indentation prefix.
 */
function elementLit(element: GrammarElement, indent: string): string {
  const i = indent + "  ";
  switch (element.type) {
    case "rule_reference":
      return `{ type: "rule_reference", name: ${JSON.stringify(element.name)} }`;

    case "token_reference":
      return `{ type: "token_reference", name: ${JSON.stringify(element.name)} }`;

    case "literal":
      return `{ type: "literal", value: ${JSON.stringify(element.value)} }`;

    case "sequence": {
      const items = element.elements
        .map((e) => `${i}${elementLit(e, i)}`)
        .join(",\n");
      return (
        `{ type: "sequence", elements: [\n` +
        `${items},\n` +
        `${indent}] }`
      );
    }

    case "alternation": {
      const items = element.choices
        .map((c) => `${i}${elementLit(c, i)}`)
        .join(",\n");
      return (
        `{ type: "alternation", choices: [\n` +
        `${items},\n` +
        `${indent}] }`
      );
    }

    case "repetition": {
      const child = elementLit(element.element, i);
      return `{ type: "repetition", element: ${child} }`;
    }

    case "optional": {
      const child = elementLit(element.element, i);
      return `{ type: "optional", element: ${child} }`;
    }

    case "group": {
      const child = elementLit(element.element, i);
      return `{ type: "group", element: ${child} }`;
    }

    case "positive_lookahead": {
      const child = elementLit(element.element, i);
      return `{ type: "positive_lookahead", element: ${child} }`;
    }

    case "negative_lookahead": {
      const child = elementLit(element.element, i);
      return `{ type: "negative_lookahead", element: ${child} }`;
    }

    case "one_or_more": {
      const child = elementLit(element.element, i);
      return `{ type: "one_or_more", element: ${child} }`;
    }

    case "separated_repetition": {
      const elem = elementLit(element.element, i);
      const sep = elementLit(element.separator, i);
      return (
        `{ type: "separated_repetition", element: ${elem}, ` +
        `separator: ${sep}, atLeastOne: ${element.atLeastOne} }`
      );
    }
  }
}
