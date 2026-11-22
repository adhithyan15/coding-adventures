/// Constants, version strings, limits, and structural bounds for the VHDL parser.
///
/// Standing Rule #20: All magic numbers, structural bounds, and limits are
/// defined as named constants here rather than scattered inline.
library;

/// The semantic version string of the VHDL parser package.
const String vhdlParserPackageVersion = '0.1.0';

/// Canonical default VHDL edition (IEEE 1076-2008), matching the TypeScript lane.
const String defaultVhdlParserVersion = '2008';

/// Supported canonical VHDL parser editions.
const List<String> supportedVhdlParserVersions = <String>[
  '1987',
  '1993',
  '2002',
  '2008',
  '2019'
];

/// Maps every accepted caller spelling to its canonical edition.
const Map<String, String> vhdlParserVersionAliases = <String, String>{
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

/// Maximum allowed token count for an input stream.
///
/// Prevents CWE-400 resource exhaustion via huge token sequences.
const int maxTokenCount = 65536;

/// Maximum allowed combined nesting depth.
///
/// Prevents CWE-674 call stack exhaustion. One counter covers parentheses,
/// block-opening statements (`if`, `case`, `loop`, `process`, `block`,
/// `generate`) and runs of prefix operators, because VHDL nests without any
/// delimiter at all (`if a then if b then ...`, `not not not x`). Probed on the
/// Dart VM: parenthesized expressions overflow the stack between 256 and 384
/// levels, so 64 leaves a wide safety margin on every supported platform.
const int maxNestingDepth = 64;

/// Canonical token type for left parenthesis.
const String tokenTypeLParen = 'LPAREN';

/// Canonical token type for right parenthesis.
const String tokenTypeRParen = 'RPAREN';

/// Canonical token type for reserved words (lowercased by the lexer).
const String tokenTypeKeyword = 'KEYWORD';

/// Canonical token type for the unary and binary plus operator.
const String tokenTypePlus = 'PLUS';

/// Canonical token type for the unary and binary minus operator.
const String tokenTypeMinus = 'MINUS';

/// Keyword that closes a block-opening statement (`end if`, `end loop`, ...).
const String keywordEnd = 'end';

/// Keyword that may separate `end` from `process` (`end postponed process`).
const String keywordPostponed = 'postponed';

/// Keyword that ends the condition of an `if` statement.
const String keywordThen = 'then';

/// Keyword that ends the header of a generate statement.
const String keywordGenerate = 'generate';

/// Keyword that opens an `if` statement or an `if ... generate` header.
const String keywordIf = 'if';

/// Keywords that open a nesting level and are closed by `end <keyword>`.
const Set<String> openerKeywords = <String>{
  keywordIf,
  'case',
  'loop',
  'process',
  'block',
  keywordGenerate,
};

/// Keywords that act as prefix operators and may chain without delimiters.
const Set<String> prefixOperatorKeywords = <String>{'not', 'abs'};
