import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar_1987.dart' as g1987;
import '_grammar_1993.dart' as g1993;
import '_grammar_2002.dart' as g2002;
import '_grammar_2008.dart' as g2008;
import '_grammar_2019.dart' as g2019;
import 'constants.dart';
import 'errors.dart';

/// Embedded pre-compiled token grammars keyed by canonical edition.
///
/// Literate why: embedding the compiled grammars means lexing never reads the
/// filesystem, so the package needs no capabilities and cannot be redirected
/// by a hostile path or a swapped file.
final Map<String, TokenGrammar> _grammars = <String, TokenGrammar>{
  '1987': g1987.tokenGrammar,
  '1993': g1993.tokenGrammar,
  '2002': g2002.tokenGrammar,
  '2008': g2008.tokenGrammar,
  '2019': g2019.tokenGrammar,
};

/// Resolves and validates a VHDL edition string to its canonical year.
///
/// A null or blank [version] selects [defaultVhdlVersion]. Accepts the bare
/// year (`2008`) and the `vhdlNNNN` spelling case-insensitively, ignoring
/// surrounding whitespace. Throws [InvalidVhdlVersionException] otherwise,
/// without echoing the caller's value.
String resolveVhdlVersion([String? version]) {
  final requested = version?.trim().toLowerCase() ?? '';
  if (requested.isEmpty) {
    return defaultVhdlVersion;
  }
  final canonical = vhdlVersionAliases[requested];
  if (canonical == null) {
    throw const InvalidVhdlVersionException();
  }
  return canonical;
}

/// Returns the embedded pre-compiled [TokenGrammar] for a VHDL edition.
TokenGrammar loadVhdlTokenGrammar({String? version}) =>
    _grammars[resolveVhdlVersion(version)]!;

void _validateSource(String source) {
  if (source.length > maxSourceLength) {
    throw const InvalidVhdlSourceException.sourceTooLarge();
  }

  for (final scalar in source.runes) {
    if (scalar < minUnicodeScalar ||
        scalar > maxUnicodeScalar ||
        (scalar >= highSurrogateMin && scalar <= lowSurrogateMax)) {
      throw const InvalidVhdlSourceException.invalidSurrogates();
    }
  }
}

/// A grammar-driven lexical analyzer for VHDL source.
final class VhdlLexer {
  /// Constructs a [VhdlLexer] for [source] text.
  ///
  /// Source bounds are validated before the version is resolved, matching the
  /// precedence of every other frontend lane.
  VhdlLexer(this.source, {String? version})
      : version = _prepare(source, version);

  /// The input VHDL source code.
  final String source;

  /// The canonical resolved VHDL edition.
  final String version;

  static String _prepare(String source, String? version) {
    _validateSource(source);
    return resolveVhdlVersion(version);
  }

  /// Tokenizes the source text and returns the complete token stream.
  List<Token> tokenize() => tokenizeVhdl(source, version: version);
}

/// Creates a [VhdlLexer] instance for [source].
VhdlLexer createVhdlLexer(String source, {String? version}) =>
    VhdlLexer(source, version: version);

/// Tokenizes [source] using the grammar of the requested VHDL edition.
///
/// Identifiers and keywords are case-insensitive in VHDL, so every `NAME` and
/// `KEYWORD` value is lowercased and re-typed as `KEYWORD` exactly when the
/// lowered text is in the edition's keyword list.
List<Token> tokenizeVhdl(String source, {String? version}) {
  _validateSource(source);
  final grammar = loadVhdlTokenGrammar(version: version);

  try {
    return List<Token>.unmodifiable(
      _normalizeCasing(
        grammarTokenize(source, grammar),
        grammar.keywords.toSet(),
      ),
    );
  } on LexerError catch (e) {
    throw VhdlLexicalException(line: e.line, column: e.column);
  }
}

/// Alias of [tokenizeVhdl] matching TypeScript/Python naming.
List<Token> tokenizeVhdlLexer(String source, {String? version}) =>
    tokenizeVhdl(source, version: version);

List<Token> _normalizeCasing(List<Token> tokens, Set<String> keywords) {
  return <Token>[
    for (final token in tokens)
      if (token.type == tokenTypeName || token.type == tokenTypeKeyword)
        _lowered(token, keywords)
      else
        token,
  ];
}

Token _lowered(Token token, Set<String> keywords) {
  final lowered = token.value.toLowerCase();
  return Token(
    type: keywords.contains(lowered) ? tokenTypeKeyword : tokenTypeName,
    value: lowered,
    line: token.line,
    column: token.column,
    flags: token.flags,
  );
}
