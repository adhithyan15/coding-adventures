/// Constants, version strings, limits, and structural bounds for the Lattice parser.
///
/// Standing Rule #20: All magic numbers, structural bounds, and limits are
/// defined as named constants here rather than scattered inline.
library;

/// The semantic version string of the Lattice parser package.
const String latticeParserPackageVersion = '0.1.0';

/// The canonical default Lattice edition version string.
const String defaultLatticeParserVersion = 'lattice';

/// Supported Lattice parser editions list.
const List<String> supportedLatticeParserVersions = <String>[
  defaultLatticeParserVersion,
];

/// Maximum allowed token count for an input stream.
///
/// Prevents CWE-400 resource exhaustion via huge token sequences.
const int maxTokenCount = 65536;

/// Maximum allowed delimiter nesting depth.
///
/// Prevents CWE-674 call stack exhaustion via deeply nested blocks or expressions.
const int maxNestingDepth = 256;

/// Canonical token type for left brace.
const String tokenTypeLBrace = 'LBRACE';

/// Canonical token type for right brace.
const String tokenTypeRBrace = 'RBRACE';

/// Canonical token type for left parenthesis.
const String tokenTypeLParen = 'LPAREN';

/// Canonical token type for function call opening (`name(`).
const String tokenTypeFunction = 'FUNCTION';

/// Canonical token type for right parenthesis.
const String tokenTypeRParen = 'RPAREN';

/// Literal character for left brace.
const String symbolLBrace = '{';

/// Literal character for right brace.
const String symbolRBrace = '}';

/// Literal character for left parenthesis.
const String symbolLParen = '(';

/// Literal character for right parenthesis.
const String symbolRParen = ')';
