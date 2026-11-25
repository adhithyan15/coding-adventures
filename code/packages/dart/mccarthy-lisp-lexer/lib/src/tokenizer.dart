/// McCarthy Lisp tokenization over the compiled grammar: the
/// grammar spec accessor, the factory, and the one-shot entry point.
/// Token types keep their grammar names verbatim; the final token is
/// always `EOF`.
library;

import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// The compiled McCarthy Lisp [TokenGrammar].
///
/// Exposed for tooling that wants to introspect the token set without
/// re-parsing the canonical `.tokens` file. (The reference caches the
/// build-time-compiled grammar in a `OnceLock`; a Dart top-level
/// `final` is already lazily initialized exactly once.)
TokenGrammar mccarthyTokenGrammarSpec() => tokenGrammar;

/// Creates a [GrammarLexer] for McCarthy Lisp [source].
/// Most callers want [tokenizeMcCarthy]; reach for this when the
/// lexer object itself is needed.
GrammarLexer createMcCarthyLexer(String source) =>
    GrammarLexer(source, tokenGrammar);

/// Tokenizes McCarthy 1960 Lisp [source] in one shot, returning the
/// full token stream including the trailing `EOF` token. Whitespace
/// and `;` line comments are skipped.
/// Throws [LexerError] on the first character matching no token rule
/// (lowercase, a bare `-`, anything non-Lisp), with 1-based line and
/// column (the reference returns `Err` there instead).
List<Token> tokenizeMcCarthy(String source) =>
    createMcCarthyLexer(source).tokenize();
