import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';
import 'constants.dart';
import 'errors.dart';

/// Resolves and validates a Lattice edition version string.
///
/// Throws [InvalidLatticeVersionException] if the version string does not match
/// the canonical supported version ('lattice').
String resolveLatticeVersion(String version) {
  if (version.trim().toLowerCase() != defaultLatticeVersion) {
    throw const InvalidLatticeVersionException();
  }
  return defaultLatticeVersion;
}

/// Returns the embedded pre-compiled [TokenGrammar] for Lattice.
TokenGrammar loadLatticeTokenGrammar({String version = defaultLatticeVersion}) {
  resolveLatticeVersion(version);
  return tokenGrammar;
}

void _validateSource(String source) {
  if (source.length > maxSourceLength) {
    throw const InvalidLatticeSourceException.sourceTooLarge();
  }

  for (final scalar in source.runes) {
    if (scalar < minUnicodeScalar ||
        scalar > maxUnicodeScalar ||
        (scalar >= highSurrogateMin && scalar <= lowSurrogateMax)) {
      throw const InvalidLatticeSourceException.invalidSurrogates();
    }
  }
}

/// A grammar-driven lexical analyzer for Lattice stylesheets.
final class LatticeLexer {
  /// Constructs a [LatticeLexer] for [source] text.
  LatticeLexer(this.source, {this.version = defaultLatticeVersion}) {
    _validateSource(source);
    resolveLatticeVersion(version);
  }

  /// The input stylesheet source code.
  final String source;

  /// The canonical resolved Lattice edition version string.
  final String version;

  /// Tokenizes the source text and returns the complete token stream.
  List<Token> tokenize() => tokenizeLattice(source, version: version);
}

/// Creates a [LatticeLexer] instance for [source].
LatticeLexer createLatticeLexer(
  String source, {
  String version = defaultLatticeVersion,
}) =>
    LatticeLexer(source, version: version);

/// Tokenizes [source] using the Lattice grammar.
List<Token> tokenizeLattice(
  String source, {
  String version = defaultLatticeVersion,
}) {
  _validateSource(source);
  resolveLatticeVersion(version);
  final grammar = loadLatticeTokenGrammar(version: version);

  try {
    return List<Token>.unmodifiable(grammarTokenize(source, grammar));
  } on LexerError catch (e) {
    throw LatticeLexicalException(line: e.line, column: e.column);
  }
}

/// Alias for [tokenizeLattice] matching TypeScript/Python naming.
List<Token> tokenizeLatticeLexer(
  String source, {
  String version = defaultLatticeVersion,
}) =>
    tokenizeLattice(source, version: version);
