/// Package-wide constants, edition specifications, and validation bounds
/// for the Verilog lexical analyzer and preprocessor.
library;

/// Semantic package version for coding_adventures_verilog_lexer.
const String verilogLexerPackageVersion = '0.1.0';

/// Canonical default Verilog specification edition.
const String defaultVerilogVersion = '2005';

/// Supported canonical Verilog specification editions.
const List<String> supportedVerilogVersions = ['1995', '2001', '2005'];

/// Case-insensitive, trimmed aliases mapped to canonical edition strings.
const Map<String, String> verilogVersionAliases = {
  '': '2005',
  'verilog': '2005',
  '2005': '2005',
  'verilog2005': '2005',
  '1995': '1995',
  'verilog1995': '1995',
  '2001': '2001',
  'verilog2001': '2001',
};

/// Maximum source length in UTF-16 code units (1 MB).
///
/// Prevents denial-of-service via unbounded input or recursive macro expansion bombs (CWE-400).
const int maxSourceLength = 1048576;

/// Maximum conditional preprocessor nesting depth (`ifdef / `ifndef).
const int maxConditionalDepth = 64;

/// Lower bound of the Unicode UTF-16 surrogate range (U+D800).
const int minSurrogateCodePoint = 0xD800;

/// Upper bound of the Unicode UTF-16 surrogate range (U+DFFF).
const int maxSurrogateCodePoint = 0xDFFF;

/// Lower bound of high surrogate code units (U+D800).
const int minHighSurrogate = 0xD800;

/// Upper bound of high surrogate code units (U+DBFF).
const int maxHighSurrogate = 0xDBFF;

/// Lower bound of low surrogate code units (U+DC00).
const int minLowSurrogate = 0xDC00;

/// Upper bound of low surrogate code units (U+DFFF).
const int maxLowSurrogate = 0xDFFF;
