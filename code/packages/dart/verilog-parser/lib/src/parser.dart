import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_verilog_lexer/verilog_lexer.dart'
    show tokenizeVerilog;

import '_grammar.dart' as g_verilog;
import '_grammar_1995.dart' as g_1995;
import '_grammar_2001.dart' as g_2001;
import '_grammar_2005.dart' as g_2005;
import 'constants.dart';
import 'errors.dart';

/// Resolves and validates a Verilog edition version string for parsing.
///
/// Returns canonical edition string ('1995', '2001', '2005').
/// Throws [InvalidVerilogParserVersionException] if unsupported.
String resolveVerilogParserVersion([String? version]) {
  final normalized = (version ?? '').trim().toLowerCase();
  final resolved = verilogParserVersionAliases[normalized];
  if (resolved == null) {
    throw const InvalidVerilogParserVersionException();
  }
  return resolved;
}

/// Returns the embedded precompiled [ParserGrammar] for [version].
ParserGrammar loadVerilogParserGrammar({String? version}) {
  final resolved = resolveVerilogParserVersion(version);
  return switch (resolved) {
    '1995' => g_1995.parserGrammar,
    '2001' => g_2001.parserGrammar,
    '2005' => g_2005.parserGrammar,
    'verilog' => g_verilog.parserGrammar,
    _ => throw const InvalidVerilogParserVersionException(),
  };
}

bool _isOpeningDelimiter(Token token) {
  return switch (token.type) {
    tokenTypeLParen || tokenTypeLBracket || tokenTypeLBrace => true,
    _ => switch (token.value) {
        '(' || '[' || '{' => true,
        _ => false,
      },
  };
}

bool _isClosingDelimiter(Token token) {
  return switch (token.type) {
    tokenTypeRParen || tokenTypeRBracket || tokenTypeRBrace => true,
    _ => switch (token.value) {
        ')' || ']' || '}' => true,
        _ => false,
      },
  };
}

bool _isOpeningBlock(Token token) {
  final val = token.value.toLowerCase();
  return openingBlockKeywords.contains(val);
}

bool _isClosingBlock(Token token) {
  final val = token.value.toLowerCase();
  return closingBlockKeywords.contains(val);
}

bool _isPrefixOperator(Token token) {
  if (prefixOperatorTokenTypes.contains(token.type)) return true;
  return switch (token.value) {
    '!' || '~' || '+' || '-' || '&' || '|' || '^' => true,
    _ => false,
  };
}

/// Validates that token count and combined nesting depth remain within safe bounds.
///
/// Delimiters, keyword blocks, and consecutive unary prefix operators share a
/// single combined depth counter to prevent CWE-674 recursion stack overflow attacks.
void _validateTokens(List<Token> tokens) {
  if (tokens.length > maxTokenCount) {
    throw const VerilogParseException.tokenCountExceeded();
  }

  var depth = 0;
  var consecutivePrefixOps = 0;

  for (final token in tokens) {
    if (_isOpeningDelimiter(token) || _isOpeningBlock(token)) {
      consecutivePrefixOps = 0;
      depth++;
      if (depth > maxNestingDepth) {
        throw const VerilogParseException.nestingTooDeep();
      }
    } else if (_isClosingDelimiter(token) || _isClosingBlock(token)) {
      consecutivePrefixOps = 0;
      if (depth > 0) {
        depth--;
      }
    } else if (_isPrefixOperator(token)) {
      consecutivePrefixOps++;
      if (depth + consecutivePrefixOps > maxNestingDepth) {
        throw const VerilogParseException.nestingTooDeep();
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

/// A grammar-driven syntax analyzer for Verilog HDL.
class VerilogParser {
  /// Constructs a Verilog parser by tokenizing [source].
  VerilogParser(
    String source, {
    String? version,
    this.options,
  })  : version = resolveVerilogParserVersion(version),
        tokens = _prepareTokens(
          tokenizeVerilog(source, version: version),
        );

  /// Constructs a Verilog parser directly from an existing list of [tokens].
  VerilogParser.fromTokens(
    List<Token> tokens, {
    String? version,
    this.options,
  })  : version = resolveVerilogParserVersion(version),
        tokens = _prepareTokens(tokens);

  /// Canonical Verilog edition version string.
  final String version;

  /// The immutable tokens to be parsed.
  final List<Token> tokens;

  /// Optional parser configuration options.
  final GrammarParserOptions? options;

  /// Parses the token stream into a generic [ASTNode] parse tree.
  ///
  /// Throws [VerilogParseException] on syntax errors or recursion limits.
  ASTNode parse() {
    final grammar = loadVerilogParserGrammar(version: version);
    final parser = GrammarParser(tokens, grammar, options: options);
    try {
      return parser.parse();
    } on GrammarParseError catch (e) {
      throw VerilogParseException.syntaxError(
        line: e.token?.line,
        column: e.token?.column,
      );
    } on StackOverflowError {
      throw const VerilogParseException.nestingTooDeep();
    }
  }
}

/// Parses Verilog [source] code into an [ASTNode] tree.
ASTNode parseVerilog(
  String source, {
  String? version,
  GrammarParserOptions? options,
}) =>
    VerilogParser(source, version: version, options: options).parse();

/// Parses a pre-tokenized Verilog [tokens] stream into an [ASTNode] tree.
ASTNode parseVerilogTokens(
  List<Token> tokens, {
  String? version,
  GrammarParserOptions? options,
}) =>
    VerilogParser.fromTokens(tokens, version: version, options: options)
        .parse();

/// Creates a [VerilogParser] for [source] text.
VerilogParser createVerilogParser(
  String source, {
  String? version,
  GrammarParserOptions? options,
}) =>
    VerilogParser(source, version: version, options: options);

/// Creates a [VerilogParser] from a list of [tokens].
VerilogParser createVerilogParserFromTokens(
  List<Token> tokens, {
  String? version,
  GrammarParserOptions? options,
}) =>
    VerilogParser.fromTokens(tokens, version: version, options: options);
