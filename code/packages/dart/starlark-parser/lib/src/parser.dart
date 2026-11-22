import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_starlark_lexer/starlark_lexer.dart'
    show tokenizeStarlark;

import '_grammar.dart' as g_starlark;
import 'constants.dart';
import 'errors.dart';

/// Resolves and validates a Starlark edition version string for parsing.
String resolveStarlarkParserVersion([String? version]) {
  if (version == null) {
    return defaultStarlarkParserVersion;
  }
  final normalized = version.trim().toLowerCase();
  if (normalized == 'starlark' || normalized == '1') {
    return defaultStarlarkParserVersion;
  }
  throw const InvalidStarlarkParserVersionException();
}

/// Returns the embedded precompiled [ParserGrammar] for Starlark.
ParserGrammar loadStarlarkParserGrammar({String? version}) {
  resolveStarlarkParserVersion(version);
  return g_starlark.parserGrammar;
}

bool _isOpeningDelimiter(Token token) {
  if (token.type == 'STRING' || token.type == 'COMMENT') return false;
  return switch (token.type) {
    tokenTypeLParen ||
    tokenTypeLBracket ||
    tokenTypeLBrace ||
    tokenTypeIndent =>
      true,
    _ => switch (token.value) {
        '(' || '[' || '{' => true,
        _ => false,
      },
  };
}

bool _isClosingDelimiter(Token token) {
  if (token.type == 'STRING' || token.type == 'COMMENT') return false;
  return switch (token.type) {
    tokenTypeRParen ||
    tokenTypeRBracket ||
    tokenTypeRBrace ||
    tokenTypeDedent =>
      true,
    _ => switch (token.value) {
        ')' || ']' || '}' => true,
        _ => false,
      },
  };
}

bool _isPrefixOperator(Token token) {
  if (token.type == 'STRING' || token.type == 'COMMENT') return false;
  if (prefixOperatorTokenTypes.contains(token.type)) return true;
  return switch (token.value) {
    'not' || '+' || '-' || '~' => true,
    _ => false,
  };
}

void _validateTokens(List<Token> tokens) {
  if (tokens.length > maxTokenCount) {
    throw const StarlarkParseException.tokenCountExceeded();
  }

  var depth = 0;
  var consecutivePrefixOps = 0;

  for (final token in tokens) {
    if (_isOpeningDelimiter(token)) {
      consecutivePrefixOps = 0;
      depth++;
      if (depth > maxNestingDepth) {
        throw const StarlarkParseException.nestingTooDeep();
      }
    } else if (_isClosingDelimiter(token)) {
      consecutivePrefixOps = 0;
      if (depth > 0) {
        depth--;
      }
    } else if (_isPrefixOperator(token)) {
      consecutivePrefixOps++;
      if (depth + consecutivePrefixOps > maxNestingDepth) {
        throw const StarlarkParseException.nestingTooDeep();
      }
    } else {
      consecutivePrefixOps = 0;
    }
  }
}

List<Token> _prepareTokens(List<Token> tokens) {
  _validateTokens(tokens);
  return List<Token>.unmodifiable(tokens);
}

/// A grammar-driven syntax analyzer for Starlark.
class StarlarkParser {
  /// Constructs a Starlark parser by tokenizing [source].
  StarlarkParser(
    String source, {
    String? version,
    this.options,
  })  : version = resolveStarlarkParserVersion(version),
        tokens = _prepareTokens(
          tokenizeStarlark(source, version: version),
        );

  /// Constructs a Starlark parser directly from an existing list of [tokens].
  StarlarkParser.fromTokens(
    List<Token> tokens, {
    String? version,
    this.options,
  })  : version = resolveStarlarkParserVersion(version),
        tokens = _prepareTokens(tokens);

  /// Canonical Starlark edition version string.
  final String version;

  /// The immutable tokens to be parsed.
  final List<Token> tokens;

  /// Optional parser configuration options.
  final GrammarParserOptions? options;

  /// Parses the token stream into a generic [ASTNode] parse tree.
  ASTNode parse() {
    final grammar = loadStarlarkParserGrammar(version: version);
    final parser = GrammarParser(tokens, grammar, options: options);
    try {
      return parser.parse();
    } on GrammarParseError catch (e) {
      throw StarlarkParseException.syntaxError(
        line: e.token?.line,
        column: e.token?.column,
      );
    } on StackOverflowError {
      throw const StarlarkParseException.nestingTooDeep();
    }
  }
}

/// Parses Starlark [source] code into an [ASTNode] tree.
ASTNode parseStarlark(
  String source, {
  String? version,
  GrammarParserOptions? options,
}) =>
    StarlarkParser(source, version: version, options: options).parse();

/// Parses a pre-tokenized Starlark [tokens] stream into an [ASTNode] tree.
ASTNode parseStarlarkTokens(
  List<Token> tokens, {
  String? version,
  GrammarParserOptions? options,
}) =>
    StarlarkParser.fromTokens(tokens, version: version, options: options)
        .parse();

/// Creates a [StarlarkParser] for [source] text.
StarlarkParser createStarlarkParser(
  String source, {
  String? version,
  GrammarParserOptions? options,
}) =>
    StarlarkParser(source, version: version, options: options);

/// Creates a [StarlarkParser] from a list of [tokens].
StarlarkParser createStarlarkParserFromTokens(
  List<Token> tokens, {
  String? version,
  GrammarParserOptions? options,
}) =>
    StarlarkParser.fromTokens(tokens, version: version, options: options);
