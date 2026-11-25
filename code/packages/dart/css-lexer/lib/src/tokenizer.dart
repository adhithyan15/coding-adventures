/// CSS tokenization over the compiled grammar: the factory and
/// one-shot entry points. Token types keep their grammar names verbatim;
/// the final token is always `EOF`. Lex failures throw the engine's
/// [LexerError] with line, column, and a stable error id (the reference
/// panics; Dart lanes propagate the structured error instead).
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// Creates a [GrammarLexer] for [source] with the CSS grammar.
GrammarLexer createCssLexer(String source) =>
    GrammarLexer(source, tokenGrammar);

/// Tokenizes [source] in one shot.
/// Throws [LexerError] on unexpected sequences.
List<Token> tokenizeCss(String source) => createCssLexer(source).tokenize();
