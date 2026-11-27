/// Property tests for `coding_adventures_lisp_parser`, using the
/// in-lane `pbt` package: the failure vocabulary over hostile text,
/// forest shapes, determinism, the nesting boundary, positions,
/// valid-program parsing, error shapes, and API agreement — all with
/// fixed seeds.
library;

import 'package:coding_adventures_lisp_parser/lisp_parser.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

bool forestsEqual(List<SExpr> a, List<SExpr> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

/// Lisp-dense hostile text: every delimiter and atom shape plus
/// whitespace, comments, and out-of-alphabet noise.
Gen<String> hostileLisp(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    "'",
    '.',
    '42',
    '-7',
    'define',
    '+',
    'lambda',
    'x',
    '"hi"',
    '"unterminated',
    ' ',
    '\n',
    '\t',
    '\r',
    ';comment\n',
    '#',
    '@',
    '`',
    '\\',
    'é',
    '🎉',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

void main() {
  group('properties', () {
    test('parse fails closed with LispParseError only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(5501),
          PbtOptions(numTests: 400, seed: BigInt.from(5501)),
          (rng) => hostileLisp(0, 60).sample(rng),
          (input) {
            try {
              parseLisp(input);
            } on LispParseError {
              return true;
            }
            return true;
          },
        ),
      );
    });

    test('trailing comments never change the outcome', () {
      check(
        'comment-invariance',
        checkProperty<String>(
          BigInt.from(5502),
          PbtOptions(numTests: 400, seed: BigInt.from(5502)),
          (rng) => hostileLisp(0, 60).sample(rng),
          (input) {
            // Appending ' ;comment' cannot change the token stream
            // (a separator plus skipped comment), so both inputs
            // must parse to equal forests or both fail.
            final a = tryParseLisp(input);
            final b = tryParseLisp('$input ;comment');
            if (a == null || b == null) return a == null && b == null;
            return forestsEqual(a, b);
          },
        ),
      );
    });

    test('parse is deterministic', () {
      check(
        'determinism',
        checkProperty<String>(
          BigInt.from(5503),
          PbtOptions(numTests: 200, seed: BigInt.from(5503)),
          (rng) => hostileLisp(0, 60).sample(rng),
          (input) {
            final a = tryParseLisp(input);
            final b = tryParseLisp(input);
            if (a == null || b == null) return a == null && b == null;
            return forestsEqual(a, b);
          },
        ),
      );
    });

    test('nesting boundary is exactly forty-eight levels', () {
      check(
        'depth-boundary',
        checkProperty<int>(
          BigInt.from(5504),
          PbtOptions(numTests: 60, seed: BigInt.from(5504)),
          (rng) => smallInt(0, 60).sample(rng),
          (depth) {
            // The pre-scan refuses the moment structural depth
            // exceeds 48 — re-measured on this engine, matching the
            // reference exactly.
            final src = '${'(' * depth}x${')' * depth}';
            return (tryParseLisp(src) != null) == (depth <= 48);
          },
        ),
      );
    });

    test('quote runs count toward the boundary', () {
      check(
        'quote-boundary',
        checkProperty<int>(
          BigInt.from(5505),
          PbtOptions(numTests: 60, seed: BigInt.from(5505)),
          (rng) => smallInt(0, 60).sample(rng),
          (depth) {
            final src = '${"'" * depth}x';
            return (tryParseLisp(src) != null) == (depth <= 48);
          },
        ),
      );
    });

    test('valid programs always parse', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(5506),
          PbtOptions(numTests: 100, seed: BigInt.from(5506)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const programs = [
              '',
              '42',
              '(+ 1 2)',
              "'foo",
              '(a . b)',
              '(lambda (x) (* x x))',
              '(define x 10)\n(define y 20)\n(+ x y)\n',
              '"hello"',
              '(car (cons 1 2))',
              '; just a comment\n',
            ];
            return tryParseLisp(programs[index]) != null;
          },
        ),
      );
    });

    test('failures carry non-empty messages', () {
      check(
        'error-shape',
        checkProperty<String>(
          BigInt.from(5507),
          PbtOptions(numTests: 400, seed: BigInt.from(5507)),
          (rng) => hostileLisp(0, 60).sample(rng),
          (input) {
            try {
              parseLisp(input);
              return true;
            } on LispParseError catch (e) {
              return e.message.isNotEmpty;
            }
          },
        ),
      );
    });

    test('nullable APIs agree with the throwing APIs', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(5508),
          PbtOptions(numTests: 400, seed: BigInt.from(5508)),
          (rng) => hostileLisp(0, 60).sample(rng),
          (input) {
            final soft = tryParseLisp(input);
            try {
              final hard = parseLisp(input);
              if (soft == null) return false;
              if (!forestsEqual(soft, hard)) return false;
            } on LispParseError {
              if (soft != null) return false;
            }
            return true;
          },
        ),
      );
    });
  });
}
