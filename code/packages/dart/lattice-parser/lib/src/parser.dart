import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lattice_lexer/coding_adventures_lattice_lexer.dart'
    show tokenizeLattice;
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';

import '_grammar.dart';
import 'constants.dart';
import 'errors.dart';

/// Resolves and validates a Lattice edition version string for parsing.
///
/// Throws [InvalidLatticeParserVersionException] if the version string does not match
/// the canonical supported version ('lattice').
String resolveLatticeParserVersion(String version) {
  if (version.trim().toLowerCase() != defaultLatticeParserVersion) {
    throw const InvalidLatticeParserVersionException();
  }
  return defaultLatticeParserVersion;
}

/// Returns the embedded pre-compiled [ParserGrammar] for Lattice.
ParserGrammar loadLatticeParserGrammar({
  String version = defaultLatticeParserVersion,
}) {
  resolveLatticeParserVersion(version);
  return parserGrammar;
}

/// Validates that token count and delimiter nesting depth remain within bounds.
void _validateTokens(List<Token> tokens) {
  if (tokens.length > maxTokenCount) {
    throw const LatticeParseException.tokenCountExceeded();
  }

  var braceDepth = 0;
  var parenDepth = 0;
  for (final token in tokens) {
    if (token.type == tokenTypeLBrace || token.value == symbolLBrace) {
      braceDepth++;
      if (braceDepth > maxNestingDepth) {
        throw const LatticeParseException.nestingTooDeep();
      }
    } else if (token.type == tokenTypeRBrace || token.value == symbolRBrace) {
      if (braceDepth > 0) {
        braceDepth--;
      }
    } else if (token.type == tokenTypeLParen ||
        token.type == tokenTypeFunction ||
        token.value.endsWith(symbolLParen)) {
      parenDepth++;
      if (parenDepth > maxNestingDepth) {
        throw const LatticeParseException.nestingTooDeep();
      }
    } else if (token.type == tokenTypeRParen || token.value == symbolRParen) {
      if (parenDepth > 0) {
        parenDepth--;
      }
    }
  }
}

/// A grammar-driven syntax analyzer for Lattice stylesheets.
class LatticeParser {
  /// Constructs a Lattice parser by tokenizing [source].
  LatticeParser(
    String source, {
    String version = defaultLatticeParserVersion,
    this.options,
  })  : version = resolveLatticeParserVersion(version),
        tokens = tokenizeLattice(source, version: defaultLatticeParserVersion) {
    _validateTokens(tokens);
  }

  /// Constructs a Lattice parser directly from a pre-tokenized token stream.
  LatticeParser.fromTokens(
    List<Token> tokens, {
    String version = defaultLatticeParserVersion,
    this.options,
  })  : version = resolveLatticeParserVersion(version),
        tokens = List<Token>.unmodifiable(tokens) {
    _validateTokens(this.tokens);
  }

  /// Canonical Lattice edition version string.
  final String version;

  /// The tokens to be parsed.
  final List<Token> tokens;

  /// Optional parser configuration options.
  final GrammarParserOptions? options;

  /// Parses the token stream into a generic [ASTNode] parse tree.
  ///
  /// Throws [LatticeParseException] on syntax errors or recursion limits.
  ASTNode parse() {
    final grammar = loadLatticeParserGrammar(version: version);
    final parser = GrammarParser(tokens, grammar, options: options);
    try {
      return parser.parse();
    } on GrammarParseError catch (e) {
      throw LatticeParseException.syntaxError(
        line: e.token?.line,
        column: e.token?.column,
      );
    }
  }
}

/// Creates a configured [LatticeParser] for the given [source] text.
LatticeParser createLatticeParser(
  String source, {
  String version = defaultLatticeParserVersion,
  GrammarParserOptions? options,
}) =>
    LatticeParser(source, version: version, options: options);

/// Parses Lattice [source] text and returns an [ASTNode] tree.
ASTNode parseLattice(
  String source, {
  String version = defaultLatticeParserVersion,
  GrammarParserOptions? options,
}) =>
    createLatticeParser(source, version: version, options: options).parse();

/// Parses a list of pre-tokenized [tokens] and returns an [ASTNode] tree.
ASTNode parseLatticeTokens(
  List<Token> tokens, {
  String version = defaultLatticeParserVersion,
  GrammarParserOptions? options,
}) =>
    LatticeParser.fromTokens(
      tokens,
      version: version,
      options: options,
    ).parse();
