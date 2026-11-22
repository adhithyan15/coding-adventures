import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_sql_lexer/sql_lexer.dart' show tokenizeSql;

import '_grammar.dart';
import 'constants.dart';
import 'errors.dart';

/// Resolves and validates a SQL edition version string for parsing.
///
/// Throws [InvalidSqlParserVersionException] if the version string does not match
/// the canonical supported version ('sql').
String resolveSqlVersion(String version) {
  if (version.trim().toLowerCase() != defaultSqlVersion) {
    throw const InvalidSqlParserVersionException();
  }
  return defaultSqlVersion;
}

/// Returns the embedded pre-compiled [ParserGrammar] for SQL.
ParserGrammar loadSqlParserGrammar({String version = defaultSqlVersion}) {
  resolveSqlVersion(version);
  return parserGrammar;
}

/// Validates that token count and parenthetical nesting depth remain within bounds.
void _validateTokens(List<Token> tokens) {
  if (tokens.length > maxTokenCount) {
    throw const SqlParseException.tokenCountExceeded();
  }

  var depth = 0;
  for (final token in tokens) {
    if (token.type == tokenTypeLParen || token.value == symbolLParen) {
      depth++;
      if (depth > maxNestingDepth) {
        throw const SqlParseException.nestingTooDeep();
      }
    } else if (token.type == tokenTypeRParen || token.value == symbolRParen) {
      if (depth > 0) {
        depth--;
      }
    }
  }
}

/// A grammar-driven syntax analyzer for ANSI SQL.
class SqlParser {
  /// Constructs a SQL parser by tokenizing [source].
  SqlParser(String source, {String version = defaultSqlVersion, this.options})
      : version = resolveSqlVersion(version),
        tokens = tokenizeSql(source, version: defaultSqlVersion) {
    _validateTokens(tokens);
  }

  /// Constructs a SQL parser directly from a pre-tokenized token stream.
  SqlParser.fromTokens(
    List<Token> tokens, {
    String version = defaultSqlVersion,
    this.options,
  })  : version = resolveSqlVersion(version),
        tokens = List<Token>.unmodifiable(tokens) {
    _validateTokens(this.tokens);
  }

  /// Canonical SQL edition version string.
  final String version;

  /// The tokens to be parsed.
  final List<Token> tokens;

  /// Optional parser configuration options.
  final GrammarParserOptions? options;

  /// Parses the token stream into a generic [ASTNode] parse tree.
  ///
  /// Throws [SqlParseException] on syntax errors or recursion limits.
  ASTNode parse() {
    final grammar = loadSqlParserGrammar(version: version);
    final parser = GrammarParser(tokens, grammar, options: options);
    try {
      return parser.parse();
    } on GrammarParseError catch (e) {
      throw SqlParseException.syntaxError(
        line: e.token?.line,
        column: e.token?.column,
      );
    }
  }
}

/// Creates a configured [SqlParser] for the given [source] text.
SqlParser createSqlParser(
  String source, {
  String version = defaultSqlVersion,
  GrammarParserOptions? options,
}) =>
    SqlParser(source, version: version, options: options);

/// Alias for [createSqlParser] matching SQL capitalization conventions.
SqlParser createSQLParser(
  String source, {
  String version = defaultSqlVersion,
  GrammarParserOptions? options,
}) =>
    createSqlParser(source, version: version, options: options);

/// Parses SQL [source] text and returns an [ASTNode] tree.
ASTNode parseSql(
  String source, {
  String version = defaultSqlVersion,
  GrammarParserOptions? options,
}) =>
    createSqlParser(source, version: version, options: options).parse();

/// Alias for [parseSql] matching SQL capitalization conventions.
ASTNode parseSQL(
  String source, {
  String version = defaultSqlVersion,
  GrammarParserOptions? options,
}) =>
    parseSql(source, version: version, options: options);

/// Parses a list of pre-tokenized [tokens] and returns an [ASTNode] tree.
ASTNode parseSqlTokens(
  List<Token> tokens, {
  String version = defaultSqlVersion,
  GrammarParserOptions? options,
}) =>
    SqlParser.fromTokens(tokens, version: version, options: options).parse();
