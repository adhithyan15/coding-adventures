import 'package:coding_adventures_excel_lexer/coding_adventures_excel_lexer.dart'
    show tokenizeExcel;
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';

import '_grammar.dart';
import 'constants.dart';
import 'errors.dart';

/// Resolves and validates an Excel edition version string for parsing.
///
/// Throws [InvalidExcelParserVersionException] if the version string does not match
/// the canonical supported version ('excel').
String resolveExcelParserVersion(String version) {
  if (version.trim().toLowerCase() != defaultExcelParserVersion) {
    throw const InvalidExcelParserVersionException();
  }
  return defaultExcelParserVersion;
}

/// Returns the embedded pre-compiled [ParserGrammar] for Excel.
ParserGrammar loadExcelParserGrammar({
  String version = defaultExcelParserVersion,
}) {
  resolveExcelParserVersion(version);
  return parserGrammar;
}

/// Returns the nearest non-space token before [index], or `null`.
Token? _previousSignificantToken(List<Token> tokens, int index) {
  for (var i = index - 1; i >= 0; i--) {
    if (tokens[i].type != tokenTypeSpace) {
      return tokens[i];
    }
  }
  return null;
}

/// Returns the nearest non-space token after [index], or `null`.
Token? _nextSignificantToken(List<Token> tokens, int index) {
  for (var i = index + 1; i < tokens.length; i++) {
    if (tokens[i].type != tokenTypeSpace) {
      return tokens[i];
    }
  }
  return null;
}

/// Reclassifies whole-column and whole-row range endpoints.
///
/// Literate why: `A:B` and `1:10` are valid Excel ranges, but the lexer can
/// only see a `NAME` or `NUMBER` next to a `COLON`. A `NAME` adjacent to a
/// colon (ignoring spaces) becomes `COLUMN_REF`; a `NUMBER` adjacent to a
/// colon becomes `ROW_REF`. This mirrors `normalizeExcelReferenceTokens` in
/// the TypeScript lane exactly.
List<Token> normalizeExcelReferenceTokens(List<Token> tokens) {
  final result = List<Token>.of(tokens);
  for (var i = 0; i < tokens.length; i++) {
    final token = tokens[i];
    if (token.type != tokenTypeName && token.type != tokenTypeNumber) {
      continue;
    }
    final previous = _previousSignificantToken(tokens, i);
    final next = _nextSignificantToken(tokens, i);
    final adjacentToColon =
        previous?.type == tokenTypeColon || next?.type == tokenTypeColon;
    if (!adjacentToColon) {
      continue;
    }
    result[i] = Token(
      type: token.type == tokenTypeName ? tokenTypeColumnRef : tokenTypeRowRef,
      value: token.value,
      line: token.line,
      column: token.column,
      flags: token.flags,
    );
  }
  return result;
}

/// Validates that token count and delimiter nesting depth remain within bounds.
///
/// Parentheses, braces and brackets share one depth counter so mixed nesting
/// such as `({([...` cannot multiply the effective recursion ceiling.
void _validateTokens(List<Token> tokens) {
  if (tokens.length > maxTokenCount) {
    throw const ExcelParseException.tokenCountExceeded();
  }

  var depth = 0;
  for (final token in tokens) {
    switch (token.type) {
      case tokenTypeLParen || tokenTypeLBrace || tokenTypeLBracket:
        depth++;
        if (depth > maxNestingDepth) {
          throw const ExcelParseException.nestingTooDeep();
        }
      case tokenTypeRParen || tokenTypeRBrace || tokenTypeRBracket:
        if (depth > 0) {
          depth--;
        }
    }
  }
}

/// Bounds-checks then normalizes [tokens] into an immutable list.
///
/// Validation runs first so a hostile token list is rejected in a single
/// linear pass before any further per-token work happens.
List<Token> _prepareTokens(List<Token> tokens) {
  _validateTokens(tokens);
  return List<Token>.unmodifiable(normalizeExcelReferenceTokens(tokens));
}

/// A grammar-driven syntax analyzer for Excel formulas.
class ExcelParser {
  /// Constructs a Excel parser by tokenizing [source].
  ExcelParser(
    String source, {
    String version = defaultExcelParserVersion,
    this.options,
  })  : version = resolveExcelParserVersion(version),
        tokens = _prepareTokens(
          tokenizeExcel(source, version: defaultExcelParserVersion),
        );

  /// Constructs a Excel parser directly from a pre-tokenized token stream.
  ExcelParser.fromTokens(
    List<Token> tokens, {
    String version = defaultExcelParserVersion,
    this.options,
  })  : version = resolveExcelParserVersion(version),
        tokens = _prepareTokens(tokens);

  /// Canonical Excel edition version string.
  final String version;

  /// The reference-normalized tokens to be parsed.
  final List<Token> tokens;

  /// Optional parser configuration options.
  final GrammarParserOptions? options;

  /// Parses the token stream into a generic [ASTNode] parse tree.
  ///
  /// Throws [ExcelParseException] on syntax errors or recursion limits.
  ASTNode parse() {
    final grammar = loadExcelParserGrammar(version: version);
    final parser = GrammarParser(tokens, grammar, options: options);
    try {
      return parser.parse();
    } on GrammarParseError catch (e) {
      throw ExcelParseException.syntaxError(
        line: e.token?.line,
        column: e.token?.column,
      );
    }
  }
}

/// Creates a configured [ExcelParser] for the given [source] text.
ExcelParser createExcelParser(
  String source, {
  String version = defaultExcelParserVersion,
  GrammarParserOptions? options,
}) =>
    ExcelParser(source, version: version, options: options);

/// Parses Excel [source] text and returns an [ASTNode] tree.
ASTNode parseExcel(
  String source, {
  String version = defaultExcelParserVersion,
  GrammarParserOptions? options,
}) =>
    createExcelParser(source, version: version, options: options).parse();

/// Parses a list of pre-tokenized [tokens] and returns an [ASTNode] tree.
ASTNode parseExcelTokens(
  List<Token> tokens, {
  String version = defaultExcelParserVersion,
  GrammarParserOptions? options,
}) =>
    ExcelParser.fromTokens(tokens, version: version, options: options).parse();

/// Parses an Excel formula; alias of [parseExcel] for TypeScript parity.
ASTNode parseExcelFormula(
  String source, {
  String version = defaultExcelParserVersion,
  GrammarParserOptions? options,
}) =>
    parseExcel(source, version: version, options: options);
