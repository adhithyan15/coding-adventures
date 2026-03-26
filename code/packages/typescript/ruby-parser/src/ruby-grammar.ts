// AUTO-GENERATED FILE - DO NOT EDIT
import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const RubyGrammar: ParserGrammar = {
  version: 1,
  rules: [
    {
      name: "program",
      lineNumber: 22,
      body: { type: "repetition", element: { type: "rule_reference", name: "statement" } }
    },
    {
      name: "statement",
      lineNumber: 23,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "assignment" }, { type: "rule_reference", name: "method_call" }, { type: "rule_reference", name: "expression_stmt" }] }
    },
    {
      name: "assignment",
      lineNumber: 24,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "EQUALS" }, { type: "rule_reference", name: "expression" }] }
    },
    {
      name: "method_call",
      lineNumber: 25,
      body: { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "KEYWORD" }] } }, { type: "token_reference", name: "LPAREN" }, { type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "expression" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "expression" }] } }] } }, { type: "token_reference", name: "RPAREN" }] }
    },
    {
      name: "expression_stmt",
      lineNumber: 26,
      body: { type: "rule_reference", name: "expression" }
    },
    {
      name: "expression",
      lineNumber: 27,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "term" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "PLUS" }, { type: "token_reference", name: "MINUS" }] } }, { type: "rule_reference", name: "term" }] } }] }
    },
    {
      name: "term",
      lineNumber: 28,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "factor" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "STAR" }, { type: "token_reference", name: "SLASH" }] } }, { type: "rule_reference", name: "factor" }] } }] }
    },
    {
      name: "factor",
      lineNumber: 29,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "NUMBER" }, { type: "token_reference", name: "STRING" }, { type: "token_reference", name: "NAME" }, { type: "token_reference", name: "KEYWORD" }, { type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "expression" }, { type: "token_reference", name: "RPAREN" }] }] }
    },
  ]
};
