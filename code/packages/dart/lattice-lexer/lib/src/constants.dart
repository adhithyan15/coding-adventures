/// Constants, limits, and encoding bounds for the Lattice lexical analyzer.
///
/// Standing Rule #20: All magic numbers, limits, and configuration values are
/// isolated in this module.
library;

/// Semantic version of the Lattice lexer package.
const String latticeLexerPackageVersion = '0.1.0';

/// Canonical default Lattice edition version string.
const String defaultLatticeVersion = 'lattice';

/// Supported Lattice editions list.
const List<String> supportedLatticeVersions = <String>[defaultLatticeVersion];

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
