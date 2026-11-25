/// Constants, limits, and encoding bounds for the TypeScript lexer.
///
/// Standing Rule #20: All magic numbers, limits, and configuration values are
/// isolated in this module.
library;

/// Semantic version of the TypeScript lexer package.
const String typescriptLexerPackageVersion = '0.1.0';

/// Canonical default TypeScript edition version string.
const String defaultTypescriptVersion = 'typescript';

/// Supported TypeScript edition version strings.
const List<String> supportedTypescriptVersions = <String>[
  'typescript',
  'ts1.0',
  'ts2.0',
  'ts3.0',
  'ts4.0',
  'ts5.0',
  'ts5.8',
];

/// Closed mapping from user-supplied edition strings to canonical version strings.
const Map<String, String> typescriptVersionAliases = <String, String>{
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
