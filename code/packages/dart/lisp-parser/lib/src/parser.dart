/// Lisp parsing over the compiled grammar: the typed [SExpr]
/// forest, the throwing and nullable source/token entry points, and
/// the DoS-hardening nesting pre-scan.
///
/// This library is a *thin wrapper*, not a hand-written parser — the
/// Dart lane of the Rust `lisp-parser` crate, powered by the same
/// language definition (`code/grammars/lisp`). Parsing happens in two
/// stages: the shared [GrammarParser] turns the converted token
/// stream into a generic concrete syntax tree, then the extractor
/// lowers that CST onto the public [SExpr] API.
library;

import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_lisp_lexer/lisp_lexer.dart' as lisp;
import 'package:coding_adventures_parser/parser.dart';

import '_grammar.dart';
import 'extractor.dart';
import 'sexpr.dart';

export 'extractor.dart' show LispParseError;
export 'sexpr.dart';

/// Maximum structural nesting (parens + pending quotes) admitted
/// before the grammar parser runs.
///
/// The engine is invoked WITHOUT its own depth cap (matching the
/// reference, whose default cap is effectively infinite), so this
/// pre-scan is the guard: over-deep input gets a catchable
/// [LispParseError] instead of overflowing the stack. Each structural
/// level costs the engine several rule frames, so 48 keeps worst-case
/// rule depth (~147) clear of the engine's documented overflow band
/// with margin to spare, while still dwarfing any real program. Flat
/// lists stay cheap however long: siblings share one level.
const int maxNestingDepth = 48;

/// The build-time-compiled Lisp [ParserGrammar].
///
/// Re-exported for tooling that needs to inspect the rules without
/// re-parsing the canonical `.grammar` file. (The reference caches
/// the compiled grammar in a `OnceLock`; a Dart top-level `final` is
/// already lazily initialized exactly once.)
ParserGrammar lispParserGrammar() => parserGrammar;

/// Open-nesting markers for [_checkNestingDepth]'s explicit stack:
/// an open paren (list) or a pending quote. Plain integer constants
/// rather than an enum, whose synthetic constructor would never
/// execute and could never be covered by tests.
const int _parenCtx = 0;
const int _quoteCtx = 1;

/// Reject over-deep token streams before invoking the engine.
///
/// Counts combined paren depth plus pending quotes (`''''x` nests
/// via `quoted = QUOTE sexpr` without consuming a paren). A
/// completed S-expression discharges the quotes wrapping it; closing
/// a list pops to its paren.
void _checkNestingDepth(List<lisp.Token> tokens) {
  final stack = <int>[];

  void dischargeQuotes() {
    while (stack.isNotEmpty && stack.last == _quoteCtx) {
      stack.removeLast();
    }
  }

  void popToParen() {
    while (stack.isNotEmpty && stack.last == _quoteCtx) {
      stack.removeLast();
    }
    if (stack.isNotEmpty && stack.last == _parenCtx) {
      stack.removeLast();
    }
    dischargeQuotes();
  }

  for (final t in tokens) {
    switch (t.tokenType) {
      case lisp.TokenType.lParen:
        stack.add(_parenCtx);
      case lisp.TokenType.rParen:
        popToParen();
      case lisp.TokenType.quote:
        stack.add(_quoteCtx);
      case lisp.TokenType.number:
      case lisp.TokenType.symbol:
      case lisp.TokenType.string:
        dischargeQuotes();
      case lisp.TokenType.dot:
      case lisp.TokenType.eof:
        break;
    }
    if (stack.length > maxNestingDepth) {
      throw LispParseError(
          'structural nesting (parens + quotes) exceeds MAX_NESTING_DEPTH '
          '($maxNestingDepth) — refusing to invoke the parser to avoid '
          'stack overflow');
    }
  }
}

/// Convert Lisp tokens to engine tokens.
///
/// Engine matching keys on `type`; positions are synthesized (this
/// API never reported any) since only the message survives the
/// error mapping.
List<Token> _toEngineTokens(List<lisp.Token> tokens) {
  final out = <Token>[];
  for (var i = 0; i < tokens.length; i++) {
    final token = tokens[i];
    out.add(Token(
      type: token.tokenType.displayName,
      value: token.value,
      line: 1,
      column: i + 1,
    ));
  }
  return out;
}

/// Parse Lisp source code into a vector of S-expressions.
///
/// This is the main entry point. It tokenizes the source text and
/// then parses the token stream into an AST.
///
/// Throws [LispParseError] if the source has syntax errors (unmatched
/// parentheses, unexpected tokens, etc.) or lexer errors
/// (unrecognized characters).
List<SExpr> parseLisp(String source) {
  final List<lisp.Token> tokens;
  try {
    tokens = lisp.tokenize(source);
  } on lisp.LexerError catch (e) {
    throw LispParseError('Lexer error: $e');
  }
  return parseLispTokens(tokens);
}

/// Parse a pre-tokenized token stream into S-expressions.
///
/// Use this when you already have tokens from the lexer and want to
/// skip re-tokenization.
List<SExpr> parseLispTokens(List<lisp.Token> tokens) {
  if (tokens.isEmpty) {
    // `program = { sexpr }` matches empty input. (The hand reader
    // indexed past the end here — a panic this migration retires.)
    return [];
  }
  _checkNestingDepth(tokens);
  final engineTokens = _toEngineTokens(tokens);
  final ASTNode cst;
  try {
    cst = GrammarParser(engineTokens, parserGrammar).parse();
  } on GrammarParseError catch (e) {
    throw LispParseError(e.message);
  }
  return extractProgram(cst);
}

/// Parse Lisp source code, returning `null` instead of throwing.
/// Mirrors [parseLisp] with idiomatic Dart nullability (catch
/// [LispParseError] for the reason).
List<SExpr>? tryParseLisp(String source) {
  try {
    return parseLisp(source);
  } on LispParseError {
    return null;
  }
}

/// Parse a pre-tokenized token stream, returning `null` instead of
/// throwing. Mirrors [parseLispTokens] with idiomatic Dart
/// nullability (catch [LispParseError] for the reason).
List<SExpr>? tryParseLispTokens(List<lisp.Token> tokens) {
  try {
    return parseLispTokens(tokens);
  } on LispParseError {
    return null;
  }
}
