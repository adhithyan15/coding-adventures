/// Constants, limits, and token identifiers for the TypeScript parser.
///
/// Standing Rule #20: All magic numbers, limits, and configuration values are
/// isolated in this module.
library;

/// Semantic version of the TypeScript parser package.
const String typescriptParserPackageVersion = '0.1.0';

/// Canonical default TypeScript edition version string for parsing.
const String defaultTypescriptParserVersion = 'typescript';

/// Supported TypeScript edition version strings for parsing.
const List<String> supportedTypescriptParserVersions = <String>[
  'typescript',
  'ts1.0',
  'ts2.0',
  'ts3.0',
  'ts4.0',
  'ts5.0',
  'ts5.8',
];

/// Closed mapping from user-supplied edition strings to canonical version strings.
const Map<String, String> typescriptParserVersionAliases = <String, String>{
  'typescript': 'typescript',
  '': 'typescript',
  'ts1.0': 'ts1.0',
  '1.0': 'ts1.0',
  'ts2.0': 'ts2.0',
  '2.0': 'ts2.0',
  'ts3.0': 'ts3.0',
  '3.0': 'ts3.0',
  'ts4.0': 'ts4.0',
  '4.0': 'ts4.0',
  'ts5.0': 'ts5.0',
  '5.0': 'ts5.0',
  'ts5.8': 'ts5.8',
  '5.8': 'ts5.8',
};

/// Maximum allowed token count for a single TypeScript source compilation.
///
/// Prevents CWE-400 memory exhaustion attacks from adversarial streams.
const int maxTokenCount = 65536;

/// Maximum allowed delimiter and prefix-operator nesting depth.
///
/// Prevents CWE-674 uncontrolled recursion stack overflow attacks.
///
/// Probe findings on Dart VM:
/// In `ts1.0..5.8.grammar`, expressions evaluate through a deep ladder:
/// `assignment_expression` -> `conditional_expression` -> ... -> `primary_expression`
/// traversing ~19 grammar rules per parenthesis level. With GrammarParser internal
/// call frames (~2-3 frames per rule), each level consumes ~40-50 stack frames.
/// The Dart VM call stack overflows at ~200-300 levels of nested parentheses or
/// chained prefix operators (`StackOverflowError`).
///
/// Setting `maxNestingDepth = 64` bounds parser execution to < 3,000 frames (< 25%
/// of VM capacity) and under 15 ms execution time, while accommodating 100% of
/// realistic TypeScript source code constructs.
const int maxNestingDepth = 64;

/// Token type names for parentheses.
const String tokenTypeLParen = 'LPAREN';
const String tokenTypeRParen = 'RPAREN';

/// Token type names for curly braces.
const String tokenTypeLBrace = 'LBRACE';
const String tokenTypeRBrace = 'RBRACE';

/// Token type names for brackets.
const String tokenTypeLBracket = 'LBRACKET';
const String tokenTypeRBracket = 'RBRACKET';

/// Token type names for prefix operators.
const String tokenTypeBang = 'BANG';
const String tokenTypeTilde = 'TILDE';
const String tokenTypePlus = 'PLUS';
const String tokenTypeMinus = 'MINUS';
const String tokenTypePlusPlus = 'PLUS_PLUS';
const String tokenTypeMinusMinus = 'MINUS_MINUS';

/// Keyword values that act as unary prefix operators in TypeScript.
const Set<String> prefixOperatorKeywords = <String>{
  'typeof',
  'void',
  'delete',
  'await',
};
