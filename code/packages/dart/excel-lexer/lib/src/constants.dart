/// Constants, limits, and encoding bounds for the Excel formula lexer.
///
/// Standing Rule #20: All magic numbers, limits, and configuration values are
/// isolated in this module.
library;

/// Semantic version of the Excel lexer package.
const String excelLexerPackageVersion = '0.1.0';

/// Canonical default Excel edition version string.
const String defaultExcelVersion = 'excel';

/// Supported Excel editions list.
const List<String> supportedExcelVersions = <String>[defaultExcelVersion];

/// Maximum allowed source text length in UTF-16 code units (1 MB).
///
/// Prevents CWE-400 memory exhaustion attacks.
const int maxSourceLength = 1048576;

/// Minimum valid Unicode scalar value.
const int minUnicodeScalar = 0x0000;

/// Maximum valid Unicode scalar value.
const int maxUnicodeScalar = 0x10ffff;

/// High surrogate range lower bound.
const int highSurrogateMin = 0xd800;

/// Low surrogate range upper bound.
const int lowSurrogateMax = 0xdfff;

/// Token type emitted by the grammar for bare identifiers.
const String tokenTypeName = 'NAME';

/// Token type for identifiers directly followed by `(` (function calls).
const String tokenTypeFunctionName = 'FUNCTION_NAME';

/// Token type for identifiers directly followed by `[` (structured tables).
const String tokenTypeTableName = 'TABLE_NAME';

/// Token type for runs of ASCII space characters.
const String tokenTypeSpace = 'SPACE';

/// Literal opening parenthesis that promotes a name to a function name.
const String symbolLParen = '(';

/// Literal opening bracket that promotes a name to a table name.
const String symbolLBracket = '[';
