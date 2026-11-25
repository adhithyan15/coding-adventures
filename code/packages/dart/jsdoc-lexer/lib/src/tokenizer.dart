/// JSDoc tokenization over the compiled grammar: the factory and
/// the one-shot entry point. Token types keep their grammar names
/// verbatim; the final token is always `EOF`.
///
/// Tokenizes the INTERIOR of a `/** ... */` block comment. The
/// enclosing `/**` and `*/` markers and the per-line `* `
/// continuation prefixes are expected to be stripped by an upstream
/// comment-extractor stage; this lexer tolerates leftover `* `
/// line-starts via a skip pattern as a safety net.
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// Creates a [GrammarLexer] for [source] with the JSDoc grammar. The
/// caller is responsible for stripping the surrounding `/** */`
/// markers; see the library docs.
GrammarLexer createJsdocLexer(String source) =>
    GrammarLexer(source, tokenGrammar);

/// Tokenizes [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference panics;
/// Dart lanes propagate the structured error instead). Use
/// [createJsdocLexer] + `tokenize()` when the call site needs to
/// handle errors explicitly — the reference offers no `Result` twin.
List<Token> tokenizeJsdoc(String source) => createJsdocLexer(source).tokenize();
