// AUTO-GENERATED FILE - DO NOT EDIT
// Source: code/grammars/xml/xml.tokens
// Regenerate with: grammar-tools compile-tokens <source.tokens>
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final tokenGrammar = TokenGrammar(
  version: 1,
  caseInsensitive: false,
  definitions: [
    TokenDefinition(
        name: "TEXT",
        pattern: "[^<&]+",
        isRegex: true,
        lineNumber: 77,
        alias: null),
    TokenDefinition(
        name: "ENTITY_REF",
        pattern: "&[a-zA-Z][a-zA-Z0-9]*;",
        isRegex: true,
        lineNumber: 78,
        alias: null),
    TokenDefinition(
        name: "CHAR_REF_HEX",
        pattern: "&#x[0-9a-fA-F]+;",
        isRegex: true,
        lineNumber: 85,
        alias: "CHAR_REF"),
    TokenDefinition(
        name: "CHAR_REF_DEC",
        pattern: "&#[0-9]+;",
        isRegex: true,
        lineNumber: 86,
        alias: "CHAR_REF"),
    TokenDefinition(
        name: "COMMENT_START",
        pattern: "<!--",
        isRegex: false,
        lineNumber: 88,
        alias: null),
    TokenDefinition(
        name: "CDATA_START",
        pattern: "<![CDATA[",
        isRegex: false,
        lineNumber: 89,
        alias: null),
    TokenDefinition(
        name: "PI_START",
        pattern: "<?",
        isRegex: false,
        lineNumber: 90,
        alias: null),
    TokenDefinition(
        name: "CLOSE_TAG_START",
        pattern: "</",
        isRegex: false,
        lineNumber: 91,
        alias: null),
    TokenDefinition(
        name: "OPEN_TAG_START",
        pattern: "<",
        isRegex: false,
        lineNumber: 92,
        alias: null),
  ],
  keywords: const [],
  mode: null,
  skipDefinitions: [
    TokenDefinition(
        name: "WHITESPACE",
        pattern: "[ \\t\\r\\n]+",
        isRegex: true,
        lineNumber: 62,
        alias: null),
  ],
  reservedKeywords: const [],
  escapeMode: "none",
  errorDefinitions: const [],
  groups: {
    "tag": PatternGroup(name: "tag", definitions: [
      TokenDefinition(
          name: "TAG_NAME",
          pattern: "[a-zA-Z_][a-zA-Z0-9_:.-]*",
          isRegex: true,
          lineNumber: 107,
          alias: null),
      TokenDefinition(
          name: "ATTR_EQUALS",
          pattern: "=",
          isRegex: false,
          lineNumber: 108,
          alias: null),
      TokenDefinition(
          name: "ATTR_VALUE_DQ",
          pattern: "\"[^\"]*\"",
          isRegex: true,
          lineNumber: 109,
          alias: "ATTR_VALUE"),
      TokenDefinition(
          name: "ATTR_VALUE_SQ",
          pattern: "'[^']*'",
          isRegex: true,
          lineNumber: 110,
          alias: "ATTR_VALUE"),
      TokenDefinition(
          name: "TAG_CLOSE",
          pattern: ">",
          isRegex: false,
          lineNumber: 111,
          alias: null),
      TokenDefinition(
          name: "SELF_CLOSE",
          pattern: "/>",
          isRegex: false,
          lineNumber: 112,
          alias: null),
      TokenDefinition(
          name: "SLASH",
          pattern: "/",
          isRegex: false,
          lineNumber: 113,
          alias: null),
    ]),
    "comment": PatternGroup(name: "comment", definitions: [
      TokenDefinition(
          name: "COMMENT_END",
          pattern: "-->",
          isRegex: false,
          lineNumber: 133,
          alias: null),
      TokenDefinition(
          name: "COMMENT_TEXT",
          pattern: "[^-]+",
          isRegex: true,
          lineNumber: 134,
          alias: null),
      TokenDefinition(
          name: "COMMENT_DASH",
          pattern: "-",
          isRegex: true,
          lineNumber: 135,
          alias: "COMMENT_TEXT"),
    ]),
    "cdata": PatternGroup(name: "cdata", definitions: [
      TokenDefinition(
          name: "CDATA_END",
          pattern: "]]>",
          isRegex: false,
          lineNumber: 150,
          alias: null),
      TokenDefinition(
          name: "CDATA_TEXT",
          pattern: "[^\\]]+",
          isRegex: true,
          lineNumber: 151,
          alias: null),
      TokenDefinition(
          name: "CDATA_BRACK",
          pattern: "[\\]]",
          isRegex: true,
          lineNumber: 152,
          alias: "CDATA_TEXT"),
    ]),
    "pi": PatternGroup(name: "pi", definitions: [
      TokenDefinition(
          name: "PI_END",
          pattern: "?>",
          isRegex: false,
          lineNumber: 184,
          alias: null),
      TokenDefinition(
          name: "PI_TARGET",
          pattern: "[a-zA-Z_][a-zA-Z0-9_:.-]*",
          isRegex: true,
          lineNumber: 185,
          alias: null),
    ]),
    "pi_body": PatternGroup(name: "pi_body", definitions: [
      TokenDefinition(
          name: "PI_END",
          pattern: "?>",
          isRegex: false,
          lineNumber: 188,
          alias: null),
      TokenDefinition(
          name: "PI_TEXT",
          pattern: "[^?]+",
          isRegex: true,
          lineNumber: 189,
          alias: null),
      TokenDefinition(
          name: "PI_QMARK",
          pattern: "\\?",
          isRegex: true,
          lineNumber: 190,
          alias: "PI_TEXT"),
    ]),
  },
  caseSensitive: true,
  layoutKeywords: const [],
  contextKeywords: const [],
  softKeywords: const [],
);
