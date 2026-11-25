/// Perl tokenization over the compiled grammars: the unversioned
/// LANG81 one-shot with its source cap, and the LANG84 release
/// tables. Token types keep their grammar names verbatim; the final
/// token is always `EOF`.
library;

import 'dart:convert';

import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

export '_grammar.dart'
    show perlGrammarClass, perlReleaseTokenGrammar, supportedPerlReleases;

/// Maximum source bytes before lexing the initial Perl subset.
/// Counted in UTF-8 bytes, exactly like the reference's
/// `source.len()` (Dart's `String.length` counts UTF-16 units, so it
/// would undercount astral text).
const int maxPerlSourceBytes = 64 * 1024;

/// The unversioned LANG81 token grammar (`perl.tokens`).
TokenGrammar unversionedPerlTokenGrammar() => tokenGrammarUnversioned;

/// Tokenizes the accepted Perl source syntax with the unversioned
/// LANG81 grammar, without invoking a host Perl.
/// Throws [ArgumentError] (with the reference's exact message) when
/// [source] exceeds [maxPerlSourceBytes], and [LexerError] on
/// unrecognized input (the reference wraps the latter in a String;
/// Dart lanes propagate the structured error instead).
List<Token> tokenizePerl(String source) {
  if (utf8.encode(source).length > maxPerlSourceBytes) {
    throw ArgumentError(
      'Perl source exceeds the $maxPerlSourceBytes-byte native pilot limit',
    );
  }
  return GrammarLexer(source, tokenGrammarUnversioned).tokenize();
}
