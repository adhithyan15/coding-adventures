/// C parsing over the compiled grammar: the factory, the throwing
/// and nullable source one-shots, and the token-input entry point
/// with its input budget. The tree is rooted at `translation_unit`.
library;

import 'dart:convert';

import 'package:coding_adventures_c_lexer/c_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';

import '_grammar.dart';

/// Recursion-depth cap for the C [GrammarParser.
///
/// An interim, broad safety net shared with the reference: the
/// engine's own default cap of 128, not yet a bespoke value measured
/// from this grammar (tracked as a follow-up in the reference).
const int maxRuleDepth = 128;

/// Token-input budget: at most this many tokens per call.
const int maxInputTokens = 2000000;

/// Token-input budget: at most this many UTF-8 bytes per token field.
const int maxTokenFieldBytes = 64 * 1024;

/// Token-input budget: at most this many UTF-8 bytes of token text
/// in aggregate per call.
const int maxTotalTokenTextBytes = 64 * 1024 * 1024;

/// Checks the token-input budget independently, because a fixed token
/// count does not bound the size of the strings the packrat parser
/// may clone into its AST. Returns the human-readable refusal, or
/// `null` when the input is within budget.
///
/// Checks, in reference order: non-empty input, token count, no
/// premature EOF, per-field byte cap, aggregate text cap. Field bytes
/// are UTF-8 bytes; lone surrogates encode as U+FFFD substitutes,
/// which only ever over-counts — tripping the cap early, in the
/// fail-closed direction.
///
/// The reference also sums per-token `type_name`/`cv` metadata; Dart
/// tokens carry neither, but `type` is a free string here (where the
/// reference's base type is a fixed enum), so it is budget-checked as
/// the second field instead — the fail-closed adaptation.
String? checkTokenBudget(
  List<Token> tokens, {
  int maxTokens = maxInputTokens,
  int maxFieldBytes = maxTokenFieldBytes,
  int maxTextBytes = maxTotalTokenTextBytes,
}) {
  if (tokens.isEmpty) {
    return 'C token input is empty';
  }
  if (tokens.length > maxTokens) {
    return 'C token input exceeds $maxTokens tokens';
  }
  var textBytes = 0;
  for (var index = 0; index < tokens.length; index++) {
    final token = tokens[index];
    if (token.type == 'EOF' && index + 1 != tokens.length) {
      return 'C token input has premature EOF';
    }
    for (final field in [token.value, token.type]) {
      final fieldBytes = utf8.encode(field).length;
      if (fieldBytes > maxFieldBytes) {
        return 'C token field exceeds $maxFieldBytes bytes';
      }
      textBytes += fieldBytes;
      if (textBytes > maxTextBytes) {
        return 'C token text exceeds $maxTextBytes bytes';
      }
    }
  }
  return null;
}

/// Drops legacy directive lines: any line whose first token is `HASH`
/// is skipped whole (EOF is always kept). Preserves the original
/// source-input contract from when the lexer skipped directives; the
/// token-input API never calls this filter.
List<Token> stripLegacyDirectiveLines(List<Token> tokens) {
  final result = <Token>[];
  int? lastLine;
  var skipLine = false;
  for (final token in tokens) {
    if (token.type == 'EOF') {
      result.add(token);
      continue;
    }
    if (lastLine != token.line) {
      lastLine = token.line;
      skipLine = token.type == 'HASH';
    }
    if (!skipLine) {
      result.add(token);
    }
  }
  return result;
}

/// Creates a [GrammarParser] wired to the C grammar and the
/// directive-stripped tokens of [source], with the recursion-depth
/// guard enabled so pathologically deep nesting fails cleanly
/// instead of overflowing the stack.
/// Throws [LexerError] when [source] does not lex (the reference
/// factory panics there too).
GrammarParser createCParser(String source) {
  final tokens = stripLegacyDirectiveLines(tokenizeC(source));
  return GrammarParser(
    tokens,
    parserGrammar,
    options: const GrammarParserOptions(maxDepth: maxRuleDepth),
  );
}

/// Parses C source text into an [ASTNode] rooted at
/// `translation_unit`: directive-strip, token budget, then parse —
/// the reference's `try_parse_c(...).unwrap()` path.
/// Throws [LexerError] on unrecognized input and [GrammarParseError]
/// on syntax errors or budget refusals (the reference panics with
/// the message; Dart lanes propagate the structured errors instead).
/// Note [createCParser] follows the reference factory exactly and
/// does NOT budget-check: use this entry point (or [tryParseC]) for
/// untrusted input.
ASTNode parseC(String source) {
  final tokens = stripLegacyDirectiveLines(tokenizeC(source));
  final budgetRefusal = checkTokenBudget(tokens);
  if (budgetRefusal != null) {
    throw GrammarParseError(budgetRefusal);
  }
  return GrammarParser(
    tokens,
    parserGrammar,
    options: const GrammarParserOptions(maxDepth: maxRuleDepth),
  ).parse();
}

/// Parses C source text, returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `try_parse_c` with
/// idiomatic Dart nullability (catch [LexerError] or
/// [GrammarParseError] for the reason).
ASTNode? tryParseC(String source) {
  try {
    return parseC(source);
  } on LexerError {
    return null;
  } on GrammarParseError {
    return null;
  }
}

/// Parses an already-tokenized C stream, such as the directive-free
/// output of a preprocessor dialect. Keeps the same compiled grammar
/// and recursion cap as [tryParseC], and never reconstructs or
/// re-lexes source text.
///
/// The token budget runs before packrat memoization can amplify the
/// cost of a caller-supplied stream. Returns `null` instead of
/// throwing (catch [GrammarParseError] for the reason; budget
/// refusals are indistinguishable from syntax failures by design —
/// the message carries the cause).
ASTNode? tryParseCTokens(List<Token> tokens) {
  if (checkTokenBudget(tokens) != null) return null;
  try {
    return GrammarParser(
      tokens,
      parserGrammar,
      options: const GrammarParserOptions(maxDepth: maxRuleDepth),
    ).parse();
  } on GrammarParseError {
    return null;
  }
}
