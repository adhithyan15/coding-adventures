/// Property tests for `coding_adventures_lisp_lexer`, using the in-lane
/// `pbt` package: the fail-closed vocabulary over hostile strings, the
/// EOF invariant, the space-join fixed point, token-shape rules, value
/// provenance, and purity — all with fixed seeds.
library;

import 'package:coding_adventures_lisp_lexer/lisp_lexer.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

/// Generates hostile strings: Lisp syntax, quotes, escapes, comments,
/// error characters, astral planes, and lone surrogates.
Gen<String> hostileStringGen(int minLen, int maxLen) {
  final codeUnit = Gen.oneOf([
    Gen.integer(BigInt.from(0x20), BigInt.from(0x7E)),
    Gen.integer(BigInt.from(0x09), BigInt.from(0x0D)),
    Gen.integer(BigInt.from(0xA0), BigInt.from(0x2FF)),
    Gen.integer(BigInt.from(0xD800), BigInt.from(0xDFFF)),
    Gen.constant(BigInt.from(0x22)),
    Gen.constant(BigInt.from(0x5C)),
    Gen.constant(BigInt.from(0x3B)),
  ]);
  return Gen.list(minLen, maxLen, codeUnit)
      .map((units) => String.fromCharCodes(units.map((u) => u.toInt())));
}

bool equalTokens(List<Token> a, List<Token> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

final _numberShape = RegExp(r'^-?[0-9]+$');
final _symbolShape = RegExp(r"^[a-zA-Z_+\-*/=<>!?&][a-zA-Z0-9_+\-*/=<>!?&]*$");

void main() {
  group('properties', () {
    test('tokenize fails closed on hostile strings', () {
      check(
        'vocabulary',
        checkProperty<String>(
          BigInt.from(1401),
          PbtOptions(numTests: 300, seed: BigInt.from(1401)),
          (rng) => hostileStringGen(0, 60).sample(rng),
          (input) {
            try {
              tokenize(input);
              return true;
            } on LexerError {
              return true;
            } catch (_) {
              return false;
            }
          },
        ),
      );
    });

    test('EOF is last, alone, and empty', () {
      check(
        'eof invariant',
        checkProperty<String>(
          BigInt.from(1402),
          PbtOptions(numTests: 300, seed: BigInt.from(1402)),
          (rng) => hostileStringGen(0, 60).sample(rng),
          (input) {
            List<Token> tokens;
            try {
              tokens = tokenize(input);
            } on LexerError {
              return true;
            }
            if (tokens.isEmpty) return false;
            if (tokens.last.tokenType != TokenType.eof) return false;
            if (tokens.last.value != '') return false;
            return tokens
                .take(tokens.length - 1)
                .every((t) => t.tokenType != TokenType.eof);
          },
        ),
      );
    });

    test('space-joined values re-tokenize identically', () {
      // Joining token values with spaces can neither merge tokens (the
      // spaces force separation) nor split them (each value is maximal),
      // so re-tokenizing is a fixed point on parsing inputs.
      check(
        'join fixed point',
        checkProperty<String>(
          BigInt.from(1403),
          PbtOptions(numTests: 300, seed: BigInt.from(1403)),
          (rng) => hostileStringGen(0, 60).sample(rng),
          (input) {
            List<Token> first;
            try {
              first = tokenize(input);
            } on LexerError {
              return true;
            }
            final joined = first.map((t) => t.value).join(' ');
            List<Token> second;
            try {
              second = tokenize(joined);
            } on LexerError {
              return false;
            }
            return equalTokens(first, second);
          },
        ),
      );
    });

    test('token values match their shapes', () {
      check(
        'shapes',
        checkProperty<String>(
          BigInt.from(1404),
          PbtOptions(numTests: 300, seed: BigInt.from(1404)),
          (rng) => hostileStringGen(0, 60).sample(rng),
          (input) {
            List<Token> tokens;
            try {
              tokens = tokenize(input);
            } on LexerError {
              return true;
            }
            for (final token in tokens) {
              switch (token.tokenType) {
                case TokenType.number:
                  if (!_numberShape.hasMatch(token.value)) return false;
                case TokenType.symbol:
                  if (!_symbolShape.hasMatch(token.value)) return false;
                case TokenType.string:
                  if (token.value.length < 2 ||
                      !token.value.startsWith('"') ||
                      !token.value.endsWith('"')) {
                    return false;
                  }
                case TokenType.lParen:
                  if (token.value != '(') return false;
                case TokenType.rParen:
                  if (token.value != ')') return false;
                case TokenType.quote:
                  if (token.value != "'") return false;
                case TokenType.dot:
                  if (token.value != '.') return false;
                case TokenType.eof:
                  if (token.value != '') return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('values come from the source runes', () {
      // Every value is a contiguous slice of the source's rune list
      // (comments and whitespace are the only thing ever dropped).
      check(
        'provenance',
        checkProperty<String>(
          BigInt.from(1405),
          PbtOptions(numTests: 200, seed: BigInt.from(1405)),
          (rng) => hostileStringGen(0, 60).sample(rng),
          (input) {
            List<Token> tokens;
            try {
              tokens = tokenize(input);
            } on LexerError {
              return true;
            }
            final runes = input.runes.toList();
            for (final token in tokens) {
              if (token.tokenType == TokenType.eof) continue;
              final value = token.value.runes.toList();
              var found = false;
              for (var i = 0; i + value.length <= runes.length; i++) {
                var match = true;
                for (var j = 0; j < value.length; j++) {
                  if (runes[i + j] != value[j]) {
                    match = false;
                    break;
                  }
                }
                if (match) {
                  found = true;
                  break;
                }
              }
              if (!found) return false;
            }
            return true;
          },
        ),
      );
    });

    test('tokenize is pure', () {
      check(
        'purity',
        checkProperty<String>(
          BigInt.from(1406),
          PbtOptions(numTests: 200, seed: BigInt.from(1406)),
          (rng) => hostileStringGen(0, 60).sample(rng),
          (input) {
            List<Token>? first;
            LexerError? firstError;
            try {
              first = tokenize(input);
            } on LexerError catch (e) {
              firstError = e;
            }
            try {
              final second = tokenize(input);
              if (firstError != null || !equalTokens(first!, second)) {
                return false;
              }
            } on LexerError catch (e) {
              if (firstError == null || e != firstError) return false;
            }
            return true;
          },
        ),
      );
    });
  });
}
