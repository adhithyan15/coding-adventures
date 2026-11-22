import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';
import 'constants.dart';
import 'errors.dart';

/// Resolves and validates a SQL edition version string.
///
/// Throws [InvalidSqlVersionException] if the version string does not match
/// the canonical supported version ('sql').
String resolveSqlVersion(String version) {
  if (version.trim().toLowerCase() != defaultSqlVersion) {
    throw const InvalidSqlVersionException();
  }
  return defaultSqlVersion;
}

/// Returns the embedded pre-compiled [TokenGrammar] for SQL.
TokenGrammar loadSqlTokenGrammar({String version = defaultSqlVersion}) {
  resolveSqlVersion(version);
  return tokenGrammar;
}

void _validateSource(String source) {
  if (source.length > maxSourceLength) {
    throw const InvalidSqlSourceException.sourceTooLarge();
  }

  for (final scalar in source.runes) {
    if (scalar < minUnicodeScalar ||
        scalar > maxUnicodeScalar ||
        (scalar >= highSurrogateMin && scalar <= lowSurrogateMax)) {
      throw const InvalidSqlSourceException.invalidSurrogates();
    }
  }
}

/// A grammar-driven lexical analyzer for ANSI SQL.
final class SqlLexer {
  /// Creates a SQL lexer for the given [source] and optional [version].
  SqlLexer(this.source, {this.version = defaultSqlVersion}) {
    _validateSource(source);
    resolveSqlVersion(version);
  }

  /// The SQL source code to tokenize.
  final String source;

  /// The SQL version string.
  final String version;

  /// Tokenizes the source text into an unmodifiable list of tokens.
  List<Token> tokenize() => tokenizeSql(source, version: version);
}

/// Factory function to create a [SqlLexer].
SqlLexer createSqlLexer(
  String source, {
  String version = defaultSqlVersion,
}) =>
    SqlLexer(source, version: version);

/// Alias for [createSqlLexer] matching uppercase naming convention.
SqlLexer createSQLLexer(
  String source, {
  String version = defaultSqlVersion,
}) =>
    createSqlLexer(source, version: version);

/// Tokenizes [source] SQL text into a list of [Token] objects.
///
/// Normalizes all [tokenTypeKeyword] values to uppercase while preserving the
/// exact casing of [tokenTypeName] identifiers.
List<Token> tokenizeSql(
  String source, {
  String version = defaultSqlVersion,
}) {
  _validateSource(source);
  resolveSqlVersion(version);
  final grammar = loadSqlTokenGrammar(version: version);

  final List<Token> rawTokens;
  try {
    rawTokens = grammarTokenize(source, grammar);
  } on LexerError catch (e) {
    throw SqlLexicalException(line: e.line, column: e.column);
  }

  final result = <Token>[];
  for (final tok in rawTokens) {
    if (tok.type == tokenTypeKeyword) {
      result.add(Token(
        type: tok.type,
        value: tok.value.toUpperCase(),
        line: tok.line,
        column: tok.column,
      ));
    } else if (tok.type == tokenTypeName &&
        tok.value.length >= 2 &&
        tok.value.startsWith(quoteDouble) &&
        tok.value.endsWith(quoteDouble)) {
      final inner = tok.value
          .substring(1, tok.value.length - 1)
          .replaceAll(quoteDoubleEscaped, quoteDouble);
      result.add(Token(
        type: tok.type,
        value: inner,
        line: tok.line,
        column: tok.column,
      ));
    } else {
      result.add(tok);
    }
  }

  return List<Token>.unmodifiable(result);
}

/// Alias for [tokenizeSql] matching uppercase naming convention.
List<Token> tokenizeSQL(
  String source, {
  String version = defaultSqlVersion,
}) =>
    tokenizeSql(source, version: version);
