// AUTO-GENERATED FILE - DO NOT EDIT
import type { ParserGrammar } from "@coding-adventures/grammar-tools";

export const TomlGrammar: ParserGrammar = {
  version: 1,
  rules: [
    {
      name: "document",
      lineNumber: 38,
      body: { type: "repetition", element: { type: "alternation", choices: [{ type: "token_reference", name: "NEWLINE" }, { type: "rule_reference", name: "expression" }] } }
    },
    {
      name: "expression",
      lineNumber: 49,
      body: { type: "alternation", choices: [{ type: "rule_reference", name: "array_table_header" }, { type: "rule_reference", name: "table_header" }, { type: "rule_reference", name: "keyval" }] }
    },
    {
      name: "keyval",
      lineNumber: 57,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "key" }, { type: "token_reference", name: "EQUALS" }, { type: "rule_reference", name: "value" }] }
    },
    {
      name: "key",
      lineNumber: 65,
      body: { type: "sequence", elements: [{ type: "rule_reference", name: "simple_key" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "DOT" }, { type: "rule_reference", name: "simple_key" }] } }] }
    },
    {
      name: "simple_key",
      lineNumber: 82,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "BARE_KEY" }, { type: "token_reference", name: "BASIC_STRING" }, { type: "token_reference", name: "LITERAL_STRING" }, { type: "token_reference", name: "TRUE" }, { type: "token_reference", name: "FALSE" }, { type: "token_reference", name: "INTEGER" }, { type: "token_reference", name: "FLOAT" }, { type: "token_reference", name: "OFFSET_DATETIME" }, { type: "token_reference", name: "LOCAL_DATETIME" }, { type: "token_reference", name: "LOCAL_DATE" }, { type: "token_reference", name: "LOCAL_TIME" }] }
    },
    {
      name: "table_header",
      lineNumber: 92,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LBRACKET" }, { type: "rule_reference", name: "key" }, { type: "token_reference", name: "RBRACKET" }] }
    },
    {
      name: "array_table_header",
      lineNumber: 104,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LBRACKET" }, { type: "token_reference", name: "LBRACKET" }, { type: "rule_reference", name: "key" }, { type: "token_reference", name: "RBRACKET" }, { type: "token_reference", name: "RBRACKET" }] }
    },
    {
      name: "value",
      lineNumber: 121,
      body: { type: "alternation", choices: [{ type: "token_reference", name: "BASIC_STRING" }, { type: "token_reference", name: "ML_BASIC_STRING" }, { type: "token_reference", name: "LITERAL_STRING" }, { type: "token_reference", name: "ML_LITERAL_STRING" }, { type: "token_reference", name: "INTEGER" }, { type: "token_reference", name: "FLOAT" }, { type: "token_reference", name: "TRUE" }, { type: "token_reference", name: "FALSE" }, { type: "token_reference", name: "OFFSET_DATETIME" }, { type: "token_reference", name: "LOCAL_DATETIME" }, { type: "token_reference", name: "LOCAL_DATE" }, { type: "token_reference", name: "LOCAL_TIME" }, { type: "rule_reference", name: "array" }, { type: "rule_reference", name: "inline_table" }] }
    },
    {
      name: "array",
      lineNumber: 140,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LBRACKET" }, { type: "rule_reference", name: "array_values" }, { type: "token_reference", name: "RBRACKET" }] }
    },
    {
      name: "array_values",
      lineNumber: 142,
      body: { type: "sequence", elements: [{ type: "repetition", element: { type: "token_reference", name: "NEWLINE" } }, { type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "value" }, { type: "repetition", element: { type: "token_reference", name: "NEWLINE" } }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "repetition", element: { type: "token_reference", name: "NEWLINE" } }, { type: "rule_reference", name: "value" }, { type: "repetition", element: { type: "token_reference", name: "NEWLINE" } }] } }, { type: "optional", element: { type: "token_reference", name: "COMMA" } }, { type: "repetition", element: { type: "token_reference", name: "NEWLINE" } }] } }] }
    },
    {
      name: "inline_table",
      lineNumber: 162,
      body: { type: "sequence", elements: [{ type: "token_reference", name: "LBRACE" }, { type: "optional", element: { type: "sequence", elements: [{ type: "rule_reference", name: "keyval" }, { type: "repetition", element: { type: "sequence", elements: [{ type: "token_reference", name: "COMMA" }, { type: "rule_reference", name: "keyval" }] } }] } }, { type: "token_reference", name: "RBRACE" }] }
    },
  ]
};
