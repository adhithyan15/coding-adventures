/// Dartmouth BASIC tokenization over the compiled grammar: the two
/// post-hooks (line-number relabeling, REM suppression), the factory,
/// and the one-shot entry point. Token types keep their grammar names
/// verbatim (except relabeled line numbers, see below); the final
/// token is always `EOF`.
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// Promotes a line-leading `NUMBER` to `LINE_NUM`.
///
/// The grammar produces `NUMBER` for all bare integers, so this hook
/// promotes the first token of each line when it is an integer — the
/// BASIC line label (`10` in `10 PRINT X`) — and leaves every other
/// integer a `NUMBER`. Tracks a single `atLineStart` flag: true
/// initially and after every `NEWLINE`, cleared by any other token
/// (in that exact order, so a blank line's leading `NEWLINE` clears
/// then immediately re-arms it).
List<Token> relabelLineNumbers(List<Token> tokens) {
  final result = <Token>[];
  var atLineStart = true;
  for (final token in tokens) {
    var current = token;
    if (atLineStart && token.type == 'NUMBER') {
      current = Token(
        type: 'LINE_NUM',
        value: token.value,
        line: token.line,
        column: token.column,
        flags: token.flags,
      );
      atLineStart = false;
    } else if (atLineStart) {
      atLineStart = false;
    }
    if (current.type == 'NEWLINE') {
      atLineStart = true;
    }
    result.add(current);
  }
  return result;
}

/// Suppresses all tokens between a `REM` keyword and the end of its
/// line: the `REM` token itself is kept, the content is dropped, and
/// the terminating `NEWLINE` is kept (the parser needs it as a
/// statement terminator).
List<Token> suppressRemContent(List<Token> tokens) {
  final result = <Token>[];
  var suppressing = false;
  for (final token in tokens) {
    // While suppressing, only a NEWLINE survives (ending the REM
    // line); everything else — including a trailing EOF — is dropped,
    // exactly like the reference.
    if (suppressing) {
      if (token.type == 'NEWLINE') {
        suppressing = false;
        result.add(token);
      }
      continue;
    }
    result.add(token);
    if (token.type == 'KEYWORD' && token.value == 'REM') {
      suppressing = true;
    }
  }
  return result;
}

/// Creates a [GrammarLexer] for [source] with the Dartmouth BASIC
/// grammar and both post-tokenize hooks registered, in reference
/// order: line-number relabeling, then REM suppression.
GrammarLexer createDartmouthBasicLexer(String source) {
  final lexer = GrammarLexer(source, tokenGrammar);
  lexer.addPostTokenize(relabelLineNumbers);
  lexer.addPostTokenize(suppressRemContent);
  return lexer;
}

/// Tokenizes 1964 Dartmouth BASIC [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference panics;
/// Dart lanes propagate the structured error instead).
///
/// The reference offers only this panicking entry point (no `Result`
/// twin), so this lane likewise offers no nullable twin: use
/// [createDartmouthBasicLexer] + `tokenize()` when the call site
/// needs to handle errors explicitly.
List<Token> tokenizeDartmouthBasic(String source) =>
    createDartmouthBasicLexer(source).tokenize();
