/// Constants, version strings, limits, and structural bounds for the SQL parser.
///
/// Standing Rule #20: All magic numbers, structural bounds, and limits are
/// defined as named constants here rather than scattered inline.
library;

/// The semantic version string of the SQL parser package.
const String sqlParserPackageVersion = '0.1.0';

/// The canonical default SQL edition version string.
const String defaultSqlVersion = 'sql';

/// Maximum allowed token count for an input stream.
///
/// Prevents CWE-400 resource exhaustion via huge token sequences.
const int maxTokenCount = 65536;

/// Maximum allowed parenthetical nesting depth.
///
/// Prevents CWE-674 call stack exhaustion via deeply nested expressions.
const int maxNestingDepth = 256;

/// Canonical token type for left parenthesis.
const String tokenTypeLParen = 'LPAREN';

/// Canonical token type for right parenthesis.
const String tokenTypeRParen = 'RPAREN';

/// Literal character for left parenthesis.
const String symbolLParen = '(';

/// Literal character for right parenthesis.
const String symbolRParen = ')';
