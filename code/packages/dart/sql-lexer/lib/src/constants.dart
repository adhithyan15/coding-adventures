/// Constants, version strings, limits, and structural bounds for the SQL lexer.
///
/// Standing Rule #20: All magic numbers, structural bounds, and limits are
/// defined as named constants here rather than scattered inline.
library;

/// The semantic version string of the SQL lexer package.
const String sqlLexerPackageVersion = '0.1.0';

/// The canonical default SQL edition version string.
const String defaultSqlVersion = 'sql';

/// Maximum allowed source code length in UTF-16 code units (1 MB).
///
/// Prevents CWE-400 resource exhaustion via arbitrarily large SQL inputs.
const int maxSourceLength = 1048576;

/// Minimum valid Unicode scalar code point.
const int minUnicodeScalar = 0x0000;

/// Maximum valid Unicode scalar code point (U+10FFFF).
const int maxUnicodeScalar = 0x10ffff;

/// Inclusive lower bound of the UTF-16 surrogate range (U+D800).
const int highSurrogateMin = 0xd800;

/// Inclusive upper bound of the UTF-16 surrogate range (U+DFFF).
const int lowSurrogateMax = 0xdfff;

/// Canonical token type for SQL reserved keywords.
const String tokenTypeKeyword = 'KEYWORD';

/// Canonical token type for SQL identifiers.
const String tokenTypeName = 'NAME';

/// Double-quote character for SQLite-style quoted identifiers.
const String quoteDouble = '"';

/// Escaped double-quote sequence inside SQLite-style quoted identifiers.
const String quoteDoubleEscaped = '""';
