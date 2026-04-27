import 'package:coding_adventures_grammar_tools/grammar_tools.dart';

final tokenGrammar = TokenGrammar(
  version: 1,
  caseInsensitive: false,
  caseSensitive: true,
  definitions: [
    const TokenDefinition(
      name: 'STRING',
      pattern: r'"([^"\\]|\\["\\\x2fbfnrt]|\\u[0-9a-fA-F]{4})*"',
      isRegex: true,
      lineNumber: 25,
    ),
    const TokenDefinition(
      name: 'NUMBER',
      pattern: r'-?[0-9]+\.?[0-9]*[eE]?[-+]?[0-9]*',
      isRegex: true,
      lineNumber: 31,
    ),
    const TokenDefinition(
      name: 'TRUE',
      pattern: 'true',
      isRegex: false,
      lineNumber: 35,
    ),
    const TokenDefinition(
      name: 'FALSE',
      pattern: 'false',
      isRegex: false,
      lineNumber: 36,
    ),
    const TokenDefinition(
      name: 'NULL',
      pattern: 'null',
      isRegex: false,
      lineNumber: 37,
    ),
    const TokenDefinition(
      name: 'LBRACE',
      pattern: '{',
      isRegex: false,
      lineNumber: 43,
    ),
    const TokenDefinition(
      name: 'RBRACE',
      pattern: '}',
      isRegex: false,
      lineNumber: 44,
    ),
    const TokenDefinition(
      name: 'LBRACKET',
      pattern: '[',
      isRegex: false,
      lineNumber: 45,
    ),
    const TokenDefinition(
      name: 'RBRACKET',
      pattern: ']',
      isRegex: false,
      lineNumber: 46,
    ),
    const TokenDefinition(
      name: 'COLON',
      pattern: ':',
      isRegex: false,
      lineNumber: 47,
    ),
    const TokenDefinition(
      name: 'COMMA',
      pattern: ',',
      isRegex: false,
      lineNumber: 48,
    ),
  ],
  keywords: const [],
  mode: null,
  skipDefinitions: [
    const TokenDefinition(
      name: 'WHITESPACE',
      pattern: r'[ \t\r\n]+',
      isRegex: true,
      lineNumber: 59,
    ),
  ],
  reservedKeywords: const [],
  escapeMode: 'none',
  errorDefinitions: const [],
  groups: const {},
  contextKeywords: const [],
  softKeywords: const [],
);
