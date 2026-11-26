/// Twig tokenization over the compiled grammar: the factory, the
/// one-shot entry point, and the grammar spec accessor. Token types
/// keep their grammar names verbatim; the final token is always `EOF`.
library;

import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart' as grammar_data;

/// The compiled Twig token grammar: keyword lists, bracket pairs, and
/// patterns for downstream tools, straight from the lexer's own
/// runtime data — a single source of truth, no re-parsing.
TokenGrammar twigTokenGrammarSpec() => grammar_data.tokenGrammar;

/// Creates a [GrammarLexer] for [source] with the Twig grammar.
GrammarLexer createTwigLexer(String source) =>
    GrammarLexer(source, grammar_data.tokenGrammar);

/// Tokenizes [source] in one shot.
/// Throws [LexerError] on unrecognized input — the reference returns
/// `Result<_, LexerError>` (never panics on input, an anti-DoS
/// design), which is exactly Dart's throw.
List<Token> tokenizeTwig(String source) => createTwigLexer(source).tokenize();
