// AUTO-GENERATED FILE - DO NOT EDIT
import type { TokenGrammar } from "@coding-adventures/grammar-tools";

export const TomlTokens: TokenGrammar = {
  definitions: [
    { name: "ML_BASIC_STRING", pattern: "\"\"\"([^\\\\]|\\\\(.|\\n)|\\n)*?\"\"\"", isRegex: true, lineNumber: 60 },
    { name: "ML_LITERAL_STRING", pattern: "'''[\\s\\S]*?'''", isRegex: true, lineNumber: 61 },
    { name: "BASIC_STRING", pattern: "\"([^\"\\\\\\n]|\\\\.)*\"", isRegex: true, lineNumber: 70 },
    { name: "LITERAL_STRING", pattern: "'[^'\\n]*'", isRegex: true, lineNumber: 71 },
    { name: "OFFSET_DATETIME", pattern: "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}(\\.\\d+)?(Z|[+-]\\d{2}:\\d{2})", isRegex: true, lineNumber: 91 },
    { name: "LOCAL_DATETIME", pattern: "\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}(\\.\\d+)?", isRegex: true, lineNumber: 92 },
    { name: "LOCAL_DATE", pattern: "\\d{4}-\\d{2}-\\d{2}", isRegex: true, lineNumber: 93 },
    { name: "LOCAL_TIME", pattern: "\\d{2}:\\d{2}:\\d{2}(\\.\\d+)?", isRegex: true, lineNumber: 94 },
    { name: "FLOAT_SPECIAL", pattern: "[+-]?(inf|nan)", isRegex: true, lineNumber: 109, alias: "FLOAT" },
    { name: "FLOAT_EXP", pattern: "[+-]?([0-9](_?[0-9])*)(\\.[0-9](_?[0-9])*)?[eE][+-]?[0-9](_?[0-9])*", isRegex: true, lineNumber: 110, alias: "FLOAT" },
    { name: "FLOAT_DEC", pattern: "[+-]?([0-9](_?[0-9])*)\\.([0-9](_?[0-9])*)", isRegex: true, lineNumber: 111, alias: "FLOAT" },
    { name: "HEX_INTEGER", pattern: "0x[0-9a-fA-F](_?[0-9a-fA-F])*", isRegex: true, lineNumber: 123, alias: "INTEGER" },
    { name: "OCT_INTEGER", pattern: "0o[0-7](_?[0-7])*", isRegex: true, lineNumber: 124, alias: "INTEGER" },
    { name: "BIN_INTEGER", pattern: "0b[01](_?[01])*", isRegex: true, lineNumber: 125, alias: "INTEGER" },
    { name: "INTEGER", pattern: "[+-]?[0-9](_?[0-9])*", isRegex: true, lineNumber: 126 },
    { name: "TRUE", pattern: "true", isRegex: false, lineNumber: 137 },
    { name: "FALSE", pattern: "false", isRegex: false, lineNumber: 138 },
    { name: "BARE_KEY", pattern: "[A-Za-z0-9_-]+", isRegex: true, lineNumber: 152 },
    { name: "EQUALS", pattern: "=", isRegex: false, lineNumber: 162 },
    { name: "DOT", pattern: ".", isRegex: false, lineNumber: 163 },
    { name: "COMMA", pattern: ",", isRegex: false, lineNumber: 164 },
    { name: "LBRACKET", pattern: "[", isRegex: false, lineNumber: 165 },
    { name: "RBRACKET", pattern: "]", isRegex: false, lineNumber: 166 },
    { name: "LBRACE", pattern: "{", isRegex: false, lineNumber: 167 },
    { name: "RBRACE", pattern: "}", isRegex: false, lineNumber: 168 },
  ],
  keywords: [],
  escapeMode: "none",
  skipDefinitions: [
    { name: "COMMENT", pattern: "#[^\\n]*", isRegex: true, lineNumber: 28 },
    { name: "WHITESPACE", pattern: "[ \\t]+", isRegex: true, lineNumber: 29 },
  ],
  version: 1,
  caseInsensitive: false
};
