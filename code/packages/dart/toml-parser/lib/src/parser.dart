/// TOML parsing over the compiled grammar: the error type, the
/// factories, and the throwing and nullable one-shots. Every tree is
/// rooted at `document`.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_toml_lexer/toml_lexer.dart';

import '_grammar.dart';

/// Recursion-depth cap for the TOML [GrammarParser], mirroring the
/// reference's measured `MAX_RULE_DEPTH` of 165: both nested arrays
/// (`a = [[[[1]]]]`, cycling `value -> array -> array_values ->
/// value`) and nested inline tables (`a = { b = { c = 1 } }`,
/// cycling `value -> inline_table -> keyval -> value`) parse cleanly
/// to 53 levels (54 trips the cap) — the two shapes landing on the
/// identical boundary, ~30% below their shared measured
/// native-stack floor, exactly as in the reference.
///
/// The cap counts rule frames identically in both engines (same
/// grammar cascade, same counting), so both measured boundaries hold
/// here too.
const int maxRuleDepth = 165;

/// Stable payload-blind failure from TOML tokenization or parsing.
/// Mirrors the reference's `TomlParseError` enum, including its
/// `PartialEq` (identity here: both failures are compile-time
/// constants) and its refusal to echo the offending source.
class TomlParseError implements Exception {
  const TomlParseError._(this.message);

  /// The source could not be tokenized as TOML.
  static const TomlParseError lexical =
      TomlParseError._('TOML tokenization failed');

  /// The token stream did not satisfy the TOML grammar.
  static const TomlParseError syntax = TomlParseError._('TOML parsing failed');

  final String message;

  @override
  String toString() => message;
}

/// Creates a [GrammarParser] wired to the TOML grammar and the
/// tokens of [source], with the recursion-depth guard enabled so
/// pathologically deep nesting fails cleanly instead of overflowing
/// the stack.
/// Throws [TomlParseError.lexical] when [source] does not lex (the
/// reference factory panics there too).
GrammarParser createTomlParser(String source) {
  final parser = tryCreateTomlParser(source);
  if (parser == null) {
    throw TomlParseError.lexical;
  }
  return parser;
}

/// Creates a [GrammarParser] like [createTomlParser], returning `null`
/// instead of throwing when [source] does not lex. Mirrors the
/// reference's `Result`-returning `try_create_toml_parser` with
/// idiomatic Dart nullability.
GrammarParser? tryCreateTomlParser(String source) {
  try {
    final tokens = tokenizeToml(source);
    return GrammarParser(
      tokens,
      parserGrammar,
      options: const GrammarParserOptions(maxDepth: maxRuleDepth),
    );
  } on LexerError {
    return null;
  }
}

/// Parses TOML source text into an [ASTNode] rooted at `document`.
/// Throws [TomlParseError.lexical] on unrecognized input and
/// [TomlParseError.syntax] on grammar violations (the reference
/// panics on both with the same payload-blind messages).
ASTNode parseToml(String source) {
  final parser = tryCreateTomlParser(source);
  if (parser == null) {
    throw TomlParseError.lexical;
  }
  try {
    return parser.parse();
  } on GrammarParseError {
    throw TomlParseError.syntax;
  }
}

/// Parses TOML source text, returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `try_parse_toml` with
/// idiomatic Dart nullability (call [parseToml] for the
/// lexical-versus-syntax reason).
ASTNode? tryParseToml(String source) {
  final parser = tryCreateTomlParser(source);
  if (parser == null) {
    return null;
  }
  try {
    return parser.parse();
  } on GrammarParseError {
    return null;
  }
}
