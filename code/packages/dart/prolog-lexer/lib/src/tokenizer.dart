/// ISO Prolog tokenization over the compiled grammar: the factory
/// and the one-shot entry point. Token types keep their grammar names
/// verbatim; the final token is always `EOF`.
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// Creates a [GrammarLexer] for [source] with the ISO Prolog grammar.
GrammarLexer createIsoPrologLexer(String source) =>
    GrammarLexer(source, tokenGrammar);

/// Tokenizes [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference panics;
/// Dart lanes propagate the structured error instead).
List<Token> tokenizeIsoProlog(String source) =>
    createIsoPrologLexer(source).tokenize();
