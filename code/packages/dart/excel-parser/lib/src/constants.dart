/// Constants, version strings, limits, and structural bounds for the Excel parser.
///
/// Standing Rule #20: All magic numbers, structural bounds, and limits are
/// defined as named constants here rather than scattered inline.
library;

/// The semantic version string of the Excel parser package.
const String excelParserPackageVersion = '0.1.0';

/// The canonical default Excel edition version string.
const String defaultExcelParserVersion = 'excel';

/// Supported Excel parser editions list.
const List<String> supportedExcelParserVersions = <String>[
  defaultExcelParserVersion,
];

/// Maximum allowed token count for an input stream.
///
/// Prevents CWE-400 resource exhaustion via huge token sequences.
const int maxTokenCount = 65536;

/// Maximum allowed combined delimiter nesting depth.
///
/// Prevents CWE-674 call stack exhaustion via deeply nested parentheses,
/// array constants, or structured-reference brackets. The value matches
/// Excel's documented limit of 64 nested levels; the formula grammar descends
/// roughly twenty rule levels per nesting level, so 256 overflows the Dart VM
/// stack while 64 leaves a wide safety margin on every supported platform.
const int maxNestingDepth = 64;

/// Canonical token type for left parenthesis.
const String tokenTypeLParen = 'LPAREN';

/// Canonical token type for right parenthesis.
const String tokenTypeRParen = 'RPAREN';

/// Canonical token type for left brace (array constants).
const String tokenTypeLBrace = 'LBRACE';

/// Canonical token type for right brace (array constants).
const String tokenTypeRBrace = 'RBRACE';

/// Canonical token type for left bracket (nested structured references).
const String tokenTypeLBracket = 'LBRACKET';

/// Canonical token type for right bracket (nested structured references).
const String tokenTypeRBracket = 'RBRACKET';

/// Token type for identifiers before reference normalization.
const String tokenTypeName = 'NAME';

/// Token type for numeric literals before reference normalization.
const String tokenTypeNumber = 'NUMBER';

/// Token type for runs of ASCII space characters (insignificant for adjacency).
const String tokenTypeSpace = 'SPACE';

/// Token type for the range operator `:`.
const String tokenTypeColon = 'COLON';

/// Token type for whole-column range endpoints (`A` in `A:B`).
const String tokenTypeColumnRef = 'COLUMN_REF';

/// Token type for whole-row range endpoints (`1` in `1:10`).
const String tokenTypeRowRef = 'ROW_REF';
