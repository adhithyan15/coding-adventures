/// Package-wide constants, edition specifications, and validation bounds
/// for the Verilog parser.
///
/// Standing Rule #20: All magic numbers, limits, and configuration values are
/// isolated in this module.
library;

/// Semantic package version for coding_adventures_verilog_parser.
const String verilogParserPackageVersion = '0.1.0';

/// Canonical default Verilog specification edition for parsing.
const String defaultVerilogParserVersion = 'verilog';

/// Supported canonical Verilog specification editions for parsing.
const List<String> supportedVerilogParserVersions = [
  'verilog',
  '1995',
  '2001',
  '2005',
];

/// Case-insensitive, trimmed aliases mapped to canonical edition strings.
const Map<String, String> verilogParserVersionAliases = {
  '': 'verilog',
  'verilog': 'verilog',
  'default': 'verilog',
  '2005': '2005',
  'verilog2005': '2005',
  '1995': '1995',
  'verilog1995': '1995',
  '2001': '2001',
  'verilog2001': '2001',
};

/// Maximum allowed token count for a single Verilog compilation unit.
///
/// Prevents CWE-400 resource exhaustion from adversarial or excessively large token streams.
const int maxTokenCount = 65536;

/// Maximum allowed combined delimiter, keyword block, and unary operator nesting depth.
///
/// Prevents CWE-674 uncontrolled recursion stack overflow attacks.
const int maxNestingDepth = 64;

/// Token type names for parentheses.
const String tokenTypeLParen = 'LPAREN';
const String tokenTypeRParen = 'RPAREN';

/// Token type names for square brackets.
const String tokenTypeLBracket = 'LBRACKET';
const String tokenTypeRBracket = 'RBRACKET';

/// Token type names for curly braces.
const String tokenTypeLBrace = 'LBRACE';
const String tokenTypeRBrace = 'RBRACE';

/// Token type names for unary prefix operators.
const Set<String> prefixOperatorTokenTypes = {
  'BANG',
  'TILDE',
  'PLUS',
  'MINUS',
  'AMP',
  'PIPE',
  'CARET',
};

/// Keywords that open a scoped block in Verilog.
const Set<String> openingBlockKeywords = {
  'begin',
  'case',
  'casex',
  'casez',
  'module',
  'function',
  'task',
  'generate',
};

/// Keywords that close a scoped block in Verilog.
const Set<String> closingBlockKeywords = {
  'end',
  'endcase',
  'endmodule',
  'endfunction',
  'endtask',
  'endgenerate',
};
