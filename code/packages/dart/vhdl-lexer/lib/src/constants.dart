/// Constants, limits, and encoding bounds for the VHDL lexical analyzer.
///
/// Standing Rule #20: All magic numbers, limits, and configuration values are
/// isolated in this module.
library;

/// Semantic version of the VHDL lexer package.
const String vhdlLexerPackageVersion = '0.1.0';

/// Canonical default VHDL edition (IEEE 1076-2008), matching the TypeScript lane.
const String defaultVhdlVersion = '2008';

/// Supported canonical VHDL editions.
const List<String> supportedVhdlVersions = <String>[
  '1987',
  '1993',
  '2002',
  '2008',
  '2019'
];

/// Maps every accepted caller spelling to its canonical edition.
///
/// The bare year is the canonical spelling used by the TypeScript lane; the
/// `vhdlNNNN` spelling is the one used by the shared neutral fixtures.
const Map<String, String> vhdlVersionAliases = <String, String>{
  '1987': '1987',
  'vhdl1987': '1987',
  '1993': '1993',
  'vhdl1993': '1993',
  '2002': '2002',
  'vhdl2002': '2002',
  '2008': '2008',
  'vhdl2008': '2008',
  '2019': '2019',
  'vhdl2019': '2019',
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

/// Token type for VHDL identifiers.
const String tokenTypeName = 'NAME';

/// Token type for VHDL reserved words.
const String tokenTypeKeyword = 'KEYWORD';
