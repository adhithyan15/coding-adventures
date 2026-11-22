import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_typescript_lexer/coding_adventures_typescript_lexer.dart'
    show tokenizeTypescript;

import '_grammar.dart' as g_generic;
import '_grammar_ts1_0.dart' as g_ts1_0;
import '_grammar_ts2_0.dart' as g_ts2_0;
import '_grammar_ts3_0.dart' as g_ts3_0;
import '_grammar_ts4_0.dart' as g_ts4_0;
import '_grammar_ts5_0.dart' as g_ts5_0;
import '_grammar_ts5_8.dart' as g_ts5_8;
import 'constants.dart';
import 'errors.dart';

/// Resolves and validates a TypeScript edition version string for parsing.
///
/// Returns canonical version string. Throws [InvalidTypescriptParserVersionException]
/// if unsupported.
String resolveTypescriptParserVersion([String? version]) {
  final normalized = (version ?? '').trim().toLowerCase();
  final resolved = typescriptParserVersionAliases[normalized];
  if (resolved == null) {
    throw const InvalidTypescriptParserVersionException();
  }
  return resolved;
}

/// Returns the embedded pre-compiled [ParserGrammar] for [version].
ParserGrammar loadTypescriptParserGrammar({String? version}) {
  final resolved = resolveTypescriptParserVersion(version);
  return switch (resolved) {
    'typescript' => g_generic.parserGrammar,
    'ts1.0' => g_ts1_0.parserGrammar,
    'ts2.0' => g_ts2_0.parserGrammar,
    'ts3.0' => g_ts3_0.parserGrammar,
    'ts4.0' => g_ts4_0.parserGrammar,
    'ts5.0' => g_ts5_0.parserGrammar,
    'ts5.8' => g_ts5_8.parserGrammar,
    _ => throw const InvalidTypescriptParserVersionException(),
  };
}

bool _isPrefixOperator(Token token) {
  return switch (token.type) {
    tokenTypeBang ||
    tokenTypeTilde ||
    tokenTypePlus ||
    tokenTypeMinus ||
    tokenTypePlusPlus ||
    tokenTypeMinusMinus =>
      true,
    _ => prefixOperatorKeywords.contains(token.value),
  };
}

/// Validates that token count and combined nesting depth remain within bounds.
///
/// Parentheses, braces, brackets, and consecutive prefix operators share one
/// combined depth counter so mixed nesting cannot multiply the effective
/// recursion ceiling.
void _validateTokens(List<Token> tokens) {
  if (tokens.length > maxTokenCount) {
    throw const TypescriptParseException.tokenCountExceeded();
  }

  var depth = 0;
  var consecutivePrefixOps = 0;

  for (final token in tokens) {
    switch (token.type) {
      case tokenTypeLParen || tokenTypeLBrace || tokenTypeLBracket:
        consecutivePrefixOps = 0;
        depth++;
        if (depth > maxNestingDepth) {
          throw const TypescriptParseException.nestingTooDeep();
        }
      case tokenTypeRParen || tokenTypeRBrace || tokenTypeRBracket:
        consecutivePrefixOps = 0;
        if (depth > 0) {
          depth--;
        }
      default:
        if (_isPrefixOperator(token)) {
          consecutivePrefixOps++;
          if (depth + consecutivePrefixOps > maxNestingDepth) {
            throw const TypescriptParseException.nestingTooDeep();
          }
        } else {
          consecutivePrefixOps = 0;
        }
    }
  }
}

List<Token> _prepareTokens(List<Token> tokens) {
  _validateTokens(tokens);
  return List<Token>.unmodifiable(tokens);
}

/// A grammar-driven syntax analyzer for TypeScript.
class TypescriptParser {
  /// Constructs a TypeScript parser by tokenizing [source].
  TypescriptParser(
    String source, {
    String? version,
    this.options,
  })  : version = resolveTypescriptParserVersion(version),
        tokens = _prepareTokens(
          tokenizeTypescript(source, version: version),
        );

  /// Constructs a TypeScript parser directly from a pre-tokenized token stream.
  TypescriptParser.fromTokens(
    List<Token> tokens, {
    String? version,
    this.options,
  })  : version = resolveTypescriptParserVersion(version),
        tokens = _prepareTokens(tokens);

  /// Canonical TypeScript edition version string.
  final String version;

  /// The immutable tokens to be parsed.
  final List<Token> tokens;

  /// Optional parser configuration options.
  final GrammarParserOptions? options;

  /// Parses the token stream into a generic [ASTNode] parse tree.
  ///
  /// Throws [TypescriptParseException] on syntax errors or recursion limits.
  ASTNode parse() {
    final grammar = loadTypescriptParserGrammar(version: version);
    final parser = GrammarParser(tokens, grammar, options: options);
    try {
      return parser.parse();
    } on GrammarParseError catch (e) {
      throw TypescriptParseException.syntaxError(
        line: e.token?.line,
        column: e.token?.column,
      );
    } on StackOverflowError {
      throw const TypescriptParseException.nestingTooDeep();
    }
  }
}

/// Creates a configured [TypescriptParser] for [source] text.
TypescriptParser createTypescriptParser(
  String source, {
  String? version,
  GrammarParserOptions? options,
}) =>
    TypescriptParser(source, version: version, options: options);

/// Alias for [createTypescriptParser] with PascalCase casing.
TypescriptParser createTypeScriptParser(
  String source, {
  String? version,
  GrammarParserOptions? options,
}) =>
    createTypescriptParser(source, version: version, options: options);

/// Parses TypeScript [source] text and returns an [ASTNode] tree.
ASTNode parseTypescript(
  String source, {
  String? version,
  GrammarParserOptions? options,
}) =>
    createTypescriptParser(source, version: version, options: options).parse();

/// Alias for [parseTypescript] with PascalCase casing.
ASTNode parseTypeScript(
  String source, {
  String? version,
  GrammarParserOptions? options,
}) =>
    parseTypescript(source, version: version, options: options);

/// Parses pre-tokenized [tokens] and returns an [ASTNode] tree.
ASTNode parseTypescriptTokens(
  List<Token> tokens, {
  String? version,
  GrammarParserOptions? options,
}) =>
    TypescriptParser.fromTokens(tokens, version: version, options: options)
        .parse();

/// Alias for [parseTypescriptTokens] with PascalCase casing.
ASTNode parseTypeScriptTokens(
  List<Token> tokens, {
  String? version,
  GrammarParserOptions? options,
}) =>
    parseTypescriptTokens(tokens, version: version, options: options);
