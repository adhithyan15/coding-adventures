// AUTO-GENERATED FILE - DO NOT EDIT
import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const PythonGrammar: ParserGrammar = {
  version: 1,
  rules: [
    {
      name: "program",
      lineNumber: 17,
      body: { type: "repetition", element: { type: "rule_reference", name: "statement" } }
    },
    {
      name: "statement",
      lineNumber: 18,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "assignment" }, { type: "rule_reference", name: "expression_stmt" }] }
    },
    {
      name: "assignment",
      lineNumber: 19,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "NAME" }, { type: "token_reference", name: "EQUALS" }, { type: "rule_reference", name: "expression" }] }
    },
    {
      name: "expression_stmt",
      lineNumber: 20,
      body: { type: "rule_reference", name: "expression" }
    },
    {
      name: "expression",
      lineNumber: 21,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "term" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "PLUS" }, { type: "token_reference", name: "MINUS" }] } }, { type: "rule_reference", name: "term" }] } }] }
    },
    {
      name: "term",
      lineNumber: 22,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "factor" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "group", element: { type: "alternation", choices: [{ type: "token_reference", name: "STAR" }, { type: "token_reference", name: "SLASH" }] } }, { type: "rule_reference", name: "factor" }] } }] }
    },
    {
      name: "factor",
      lineNumber: 23,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "NUMBER" }, { type: "token_reference", name: "STRING" }, { type: "token_reference", name: "NAME" }, { type: "sequence", elements: [{ type: "token_reference", name: "LPAREN" }, { type: "rule_reference", name: "expression" }, { type: "token_reference", name: "RPAREN" }] }] }
    },
  ]
};
