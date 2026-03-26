// AUTO-GENERATED FILE - DO NOT EDIT
import { TokenGrammar, TokenDefinition, PatternGroup } from "./token-grammar.js";
import { ParserGrammar, GrammarElement, GrammarRule } from "./parser-grammar.js";

/**
 * Compiles a TokenGrammar object into a TypeScript source file containing the exported object.
 */
export function compileTokensToTypeScript(grammar: TokenGrammar, exportName: string): string {
  const lines: string[] = [];
  lines.push(`// AUTO-GENERATED FILE - DO NOT EDIT`);
  lines.push(`import type { TokenGrammar } from "@coding-adventures/grammar-tools";`);
  lines.push(``);
  lines.push(`export const ${exportName}: TokenGrammar = {`);

  // definitions
  lines.push(`  definitions: [`);
  for (const def of grammar.definitions) {
    lines.push(`    ${compileTokenDef(def)},`);
  }
  lines.push(`  ],`);

  // keywords
  lines.push(`  keywords: ${JSON.stringify(grammar.keywords)},`);

  // mode
  if (grammar.mode !== undefined) {
    lines.push(`  mode: ${JSON.stringify(grammar.mode)},`);
  }

  // escapeMode
  if (grammar.escapeMode !== undefined) {
    lines.push(`  escapeMode: ${JSON.stringify(grammar.escapeMode)},`);
  }

  // skipDefinitions
  if (grammar.skipDefinitions) {
    lines.push(`  skipDefinitions: [`);
    for (const def of grammar.skipDefinitions) {
      lines.push(`    ${compileTokenDef(def)},`);
    }
    lines.push(`  ],`);
  }

  // reservedKeywords
  if (grammar.reservedKeywords) {
    lines.push(`  reservedKeywords: ${JSON.stringify(grammar.reservedKeywords)},`);
  }

  // groups
  if (grammar.groups) {
    lines.push(`  groups: {`);
    for (const [gname, group] of Object.entries(grammar.groups)) {
      lines.push(`    ${JSON.stringify(gname)}: {`);
      lines.push(`      name: ${JSON.stringify(group.name)},`);
      lines.push(`      definitions: [`);
      for (const def of group.definitions) {
        lines.push(`        ${compileTokenDef(def)},`);
      }
      lines.push(`      ],`);
      lines.push(`    },`);
    }
    lines.push(`  },`);
  }

  // Properties
  if (grammar.caseSensitive !== undefined) {
    lines.push(`  caseSensitive: ${grammar.caseSensitive},`);
  }
  lines.push(`  version: ${grammar.version},`);
  lines.push(`  caseInsensitive: ${grammar.caseInsensitive}`);

  lines.push(`};`);
  return lines.join("\n") + "\n";
}

/**
 * Compiles a ParserGrammar object into a TypeScript source file containing the exported object.
 */
export function compileParserToTypeScript(grammar: ParserGrammar, exportName: string): string {
  const lines: string[] = [];
  lines.push(`// AUTO-GENERATED FILE - DO NOT EDIT`);
  lines.push(`import type { ParserGrammar } from "@coding-adventures/grammar-tools";`);
  lines.push(``);
  lines.push(`export const ${exportName}: ParserGrammar = {`);
  lines.push(`  version: ${grammar.version},`);
  lines.push(`  rules: [`);
  for (const rule of grammar.rules) {
    lines.push(`    {`);
    lines.push(`      name: ${JSON.stringify(rule.name)},`);
    lines.push(`      lineNumber: ${rule.lineNumber},`);
    lines.push(`      body: ${compileEbnfNode(rule.body)}`);
    lines.push(`    },`);
  }
  lines.push(`  ]`);
  lines.push(`};`);
  return lines.join("\n") + "\n";
}

function compileTokenDef(d: TokenDefinition): string {
  if (d.alias) {
    return `{ name: ${JSON.stringify(d.name)}, pattern: ${JSON.stringify(d.pattern)}, isRegex: ${d.isRegex}, lineNumber: ${d.lineNumber}, alias: ${JSON.stringify(d.alias)} }`;
  } else {
    return `{ name: ${JSON.stringify(d.name)}, pattern: ${JSON.stringify(d.pattern)}, isRegex: ${d.isRegex}, lineNumber: ${d.lineNumber} }`;
  }
}

function compileEbnfNode(node: GrammarElement): string {
  switch (node.type) {
    case "rule_reference":
      return `{ type: "rule_reference", name: ${JSON.stringify(node.name)} }`;
    case "token_reference":
      return `{ type: "token_reference", name: ${JSON.stringify(node.name)} }`;
    case "literal":
      return `{ type: "literal", value: ${JSON.stringify(node.value)} }`;
    case "sequence": {
      const elements = node.elements.map(compileEbnfNode).join(", ");
      return `{ type: "sequence", elements: [${elements}] }`;
    }
    case "alternation": {
      const choices = node.choices.map(compileEbnfNode).join(", ");
      return `{ type: "alternation", choices: [${choices}] }`;
    }
    case "repetition":
      return `{ type: "repetition", element: ${compileEbnfNode(node.element)} }`;
    case "optional":
      return `{ type: "optional", element: ${compileEbnfNode(node.element)} }`;
    case "group":
      return `{ type: "group", element: ${compileEbnfNode(node.element)} }`;
    default:
      // Should never happen due to TS discriminated unions, but needed for runtime fallback.
      throw new Error(`Unknown node type`);
  }
}
