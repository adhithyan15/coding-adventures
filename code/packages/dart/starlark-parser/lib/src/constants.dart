/// Constants and configuration boundaries for the Starlark parser.
///
/// Complies with Standing Rule #20: magic numbers and defaults are centralized here.
library;

/// Default version identifier for Starlark parser grammar resolution.
const String defaultStarlarkParserVersion = 'starlark';

/// Supported version identifiers accepted by [resolveStarlarkParserVersion].
const List<String> supportedStarlarkParserVersions = ['starlark', '1'];

/// Maximum token count allowed for a single parse tree generation.
///
/// Mitigates CWE-400 memory and CPU exhaustion from excessively large inputs.
const int maxTokenCount = 65536;

/// Maximum combined nesting depth allowed for delimiters, block scopes, and unary operators.
///
/// Mitigates CWE-674 recursive stack exhaustion before call stack limit is reached.
const int maxNestingDepth = 64;

/// Token types representing opening group delimiters.
const String tokenTypeLParen = 'LPAREN';
const String tokenTypeLBracket = 'LBRACKET';
const String tokenTypeLBrace = 'LBRACE';
const String tokenTypeIndent = 'INDENT';

/// Token types representing closing group delimiters.
const String tokenTypeRParen = 'RPAREN';
const String tokenTypeRBracket = 'RBRACKET';
const String tokenTypeRBrace = 'RBRACE';
const String tokenTypeDedent = 'DEDENT';

/// Token types representing unary prefix operators.
const Set<String> prefixOperatorTokenTypes = {
  'PLUS',
  'MINUS',
  'TILDE',
  'NOT',
};
