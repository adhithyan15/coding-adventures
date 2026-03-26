// AUTO-GENERATED FILE - DO NOT EDIT
import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const JsonGrammar: ParserGrammar = {
  version: 1,
  rules: [
    {
      name: "value",
      lineNumber: 28,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "object" }, { type: "rule_reference", name: "array" }, { type: "token_reference", name: "STRING" }, { type: "token_reference", name: "NUMBER" }, { type: "token_reference", name: "TRUE" }, { type: "token_reference", name: "FALSE" }, { type: "token_reference", name: "NULL" }] }
    },
    {
      name: "object",
      lineNumber: 34,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LBRACE" }, { type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "pair" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "pair" }] } }] } }, { type: "token_reference", name: "RBRACE" }] }
    },
    {
      name: "pair",
      lineNumber: 38,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "STRING" }, { type: "token_reference", name: "COLON" }, { type: "rule_reference", name: "value" }] }
    },
    {
      name: "array",
      lineNumber: 42,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LBRACKET" }, { type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "value" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "value" }] } }] } }, { type: "token_reference", name: "RBRACKET" }] }
    },
  ]
};
