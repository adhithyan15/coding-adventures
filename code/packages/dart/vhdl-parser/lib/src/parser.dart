import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_vhdl_lexer/coding_adventures_vhdl_lexer.dart'
    show tokenizeVhdl;

import '_grammar_1987.dart' as g1987;
import '_grammar_1993.dart' as g1993;
import '_grammar_2002.dart' as g2002;
import '_grammar_2008.dart' as g2008;
import '_grammar_2019.dart' as g2019;
import 'constants.dart';
import 'errors.dart';

/// Embedded pre-compiled parser grammars keyed by canonical edition.
final Map<String, ParserGrammar> _grammars = <String, ParserGrammar>{
  '1987': g1987.parserGrammar,
  '1993': g1993.parserGrammar,
  '2002': g2002.parserGrammar,
  '2008': g2008.parserGrammar,
  '2019': g2019.parserGrammar,
};

/// Resolves and validates a VHDL edition string to its canonical year.
///
/// A null or blank [version] selects [defaultVhdlParserVersion]. Throws
/// [InvalidVhdlParserVersionException] for anything else, without echoing the
/// caller's value.
String resolveVhdlParserVersion([String? version]) {
  final requested = version?.trim().toLowerCase() ?? '';
  if (requested.isEmpty) {
    return defaultVhdlParserVersion;
  }
  final canonical = vhdlParserVersionAliases[requested];
  if (canonical == null) {
    throw const InvalidVhdlParserVersionException();
  }
  return canonical;
}

/// Returns the embedded pre-compiled [ParserGrammar] for a VHDL edition.
ParserGrammar loadVhdlParserGrammar({String? version}) =>
    _grammars[resolveVhdlParserVersion(version)]!;

/// Validates that token count and combined nesting depth remain within bounds.
///
/// Literate why: VHDL nests without delimiters, so counting only parentheses
/// (the way the brace/paren frontends do) would let `if a then if b then ...`
/// or `not not not x` recurse arbitrarily deep, exactly like the `FUNCTION`
/// token bypass found in the Lattice parser. The depth is therefore:
///
/// * `(` increments and `)` decrements;
/// * an opener keyword (see [openerKeywords]) increments unless it directly
///   follows `end`; `end <opener>` (or `end postponed process`) decrements;
/// * an `if ... generate` header counts once, through its `generate`;
/// * a run of consecutive prefix operators (`not`, `abs`, `+`, `-`) adds its
///   length on top of the current depth.
///
/// All decrements clamp at zero so stray closers cannot drive the counter
/// negative and hide later nesting. A single linear pass keeps hostile token
/// lists cheap to reject.
void _validateTokens(List<Token> tokens) {
  if (tokens.length > maxTokenCount) {
    throw const VhdlParseException.tokenCountExceeded();
  }

  var depth = 0;
  var prefixRun = 0;
  var unresolvedIfs = 0;
  var closerIndex = -1;

  for (var i = 0; i < tokens.length; i++) {
    final token = tokens[i];
    final isKeyword = token.type == tokenTypeKeyword;

    if (isKeyword && token.value == keywordEnd) {
      closerIndex = _closedKeywordIndex(tokens, i);
      if (closerIndex >= 0 && depth > 0) {
        depth--;
      }
    } else if (isKeyword && i == closerIndex) {
      // The keyword of `end if` / `end loop` / ...: already accounted for.
    } else if (token.type == tokenTypeLParen) {
      depth++;
    } else if (token.type == tokenTypeRParen) {
      if (depth > 0) {
        depth--;
      }
    } else if (isKeyword && token.value == keywordThen) {
      if (unresolvedIfs > 0) {
        unresolvedIfs--;
      }
    } else if (isKeyword && token.value == keywordGenerate) {
      if (unresolvedIfs > 0) {
        // `if cond generate`: the `if` was provisionally counted; the
        // `generate` replaces it so the pair opens exactly one level.
        unresolvedIfs--;
        depth--;
      }
      depth++;
    } else if (isKeyword && openerKeywords.contains(token.value)) {
      depth++;
      if (token.value == keywordIf) {
        unresolvedIfs++;
      }
    }

    prefixRun = _isPrefixOperator(token) ? prefixRun + 1 : 0;
    if (depth + prefixRun > maxNestingDepth) {
      throw const VhdlParseException.nestingTooDeep();
    }
  }
}

/// Returns the index of the keyword closed by the `end` at [endIndex], or -1.
///
/// `end if;`, `end loop;`, `end process;`, ... close the keyword directly
/// after `end`; `end postponed process;` closes the one after `postponed`.
int _closedKeywordIndex(List<Token> tokens, int endIndex) {
  var target = endIndex + 1;
  if (target < tokens.length &&
      tokens[target].type == tokenTypeKeyword &&
      tokens[target].value == keywordPostponed) {
    target++;
  }
  if (target < tokens.length &&
      tokens[target].type == tokenTypeKeyword &&
      openerKeywords.contains(tokens[target].value)) {
    return target;
  }
  return -1;
}

bool _isPrefixOperator(Token token) =>
    token.type == tokenTypePlus ||
    token.type == tokenTypeMinus ||
    (token.type == tokenTypeKeyword &&
        prefixOperatorKeywords.contains(token.value));

/// Bounds-checks [tokens] and freezes them into an immutable list.
List<Token> _prepareTokens(List<Token> tokens) {
  _validateTokens(tokens);
  return List<Token>.unmodifiable(tokens);
}

/// A grammar-driven syntax analyzer for VHDL source.
class VhdlParser {
  /// Constructs a VHDL parser by tokenizing [source].
  VhdlParser(String source, {String? version, this.options})
      : version = resolveVhdlParserVersion(version),
        tokens = _prepareTokens(
          tokenizeVhdl(source, version: resolveVhdlParserVersion(version)),
        );

  /// Constructs a VHDL parser directly from a pre-tokenized token stream.
  VhdlParser.fromTokens(List<Token> tokens, {String? version, this.options})
      : version = resolveVhdlParserVersion(version),
        tokens = _prepareTokens(tokens);

  /// Canonical VHDL edition.
  final String version;

  /// The tokens to be parsed.
  final List<Token> tokens;

  /// Optional parser configuration options.
  final GrammarParserOptions? options;

  /// Parses the token stream into a generic [ASTNode] parse tree.
  ///
  /// Throws [VhdlParseException] on syntax errors or recursion limits. A stack
  /// overflow that slips past the pre-parse nesting check is reported as
  /// [VhdlParseException.nestingTooDeep] rather than crashing the caller.
  ASTNode parse() {
    final grammar = loadVhdlParserGrammar(version: version);
    final parser = GrammarParser(tokens, grammar, options: options);
    try {
      return parser.parse();
    } on GrammarParseError catch (e) {
      throw VhdlParseException.syntaxError(
        line: e.token?.line,
        column: e.token?.column,
      );
    } on StackOverflowError {
      throw const VhdlParseException.nestingTooDeep();
    }
  }
}

/// Creates a configured [VhdlParser] for the given [source] text.
VhdlParser createVhdlParser(
  String source, {
  String? version,
  GrammarParserOptions? options,
}) =>
    VhdlParser(source, version: version, options: options);

/// Parses VHDL [source] text and returns an [ASTNode] tree.
ASTNode parseVhdl(
  String source, {
  String? version,
  GrammarParserOptions? options,
}) =>
    createVhdlParser(source, version: version, options: options).parse();

/// Parses a list of pre-tokenized [tokens] and returns an [ASTNode] tree.
ASTNode parseVhdlTokens(
  List<Token> tokens, {
  String? version,
  GrammarParserOptions? options,
}) =>
    VhdlParser.fromTokens(tokens, version: version, options: options).parse();
