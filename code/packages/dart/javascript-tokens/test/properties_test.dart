/// Property tests for `coding_adventures_javascript_tokens`, using
/// the in-lane `pbt` package: version round-trips, ordering laws,
/// error-message shape, span arithmetic laws, equality/hash
/// agreement, the trivia table, and `Other` payload laws — all with
/// fixed seeds.
library;

import 'package:coding_adventures_javascript_tokens/javascript_tokens.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

/// Hostile short strings: near-miss versions plus noise.
Gen<String> hostileId(int minLen, int maxLen) {
  const pieces = [
    'es',
    'ES',
    '1',
    '2',
    '3',
    '5',
    '20',
    '2015',
    '2025',
    '2026',
    '.',
    '1',
    'latest',
    ' ',
    '',
    'e',
    's',
    'es5.1',
    '_',
    '-',
    'é',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

int sign(int x) => x == 0 ? 0 : (x < 0 ? -1 : 1);

void main() {
  group('properties', () {
    test('versions round-trip through strings', () {
      check(
        'version-round-trip',
        checkProperty<int>(
          BigInt.from(6301),
          PbtOptions(numTests: 200, seed: BigInt.from(6301)),
          (rng) => smallInt(0, EsVersion.all.length - 1).sample(rng),
          (index) {
            final v = EsVersion.all[index];
            return EsVersion.parse(v.asStr()) == v &&
                EsVersion.tryParse(v.asStr()) == v;
          },
        ),
      );
    });

    test('version ordering matches declaration order', () {
      check(
        'version-order',
        checkProperty<List<int>>(
          BigInt.from(6302),
          PbtOptions(numTests: 200, seed: BigInt.from(6302)),
          (rng) =>
              Gen.list(2, 2, smallInt(0, EsVersion.all.length - 1)).sample(rng),
          (pair) {
            final a = EsVersion.all[pair[0]];
            final b = EsVersion.all[pair[1]];
            return sign(a.compareTo(b)) == sign(pair[0].compareTo(pair[1]));
          },
        ),
      );
    });

    test('version names are pairwise distinct', () {
      check(
        'version-distinct',
        checkProperty<List<int>>(
          BigInt.from(6303),
          PbtOptions(numTests: 200, seed: BigInt.from(6303)),
          (rng) =>
              Gen.list(2, 2, smallInt(0, EsVersion.all.length - 1)).sample(rng),
          (pair) {
            if (pair[0] == pair[1]) return true;
            return EsVersion.all[pair[0]].asStr() !=
                EsVersion.all[pair[1]].asStr();
          },
        ),
      );
    });

    test('unknown versions fail with the full message', () {
      check(
        'version-error-shape',
        checkProperty<String>(
          BigInt.from(6304),
          PbtOptions(numTests: 400, seed: BigInt.from(6304)),
          (rng) => hostileId(0, 12).sample(rng),
          (id) {
            final parsed = EsVersion.tryParse(id);
            if (parsed != null) {
              return parsed.asStr() == id;
            }
            try {
              EsVersion.parse(id);
            } on UnknownEsVersion catch (e) {
              return e.input == id &&
                  e.message.contains('"$id"') &&
                  e.message.contains('"es2025"') &&
                  e.toString() == e.message;
            }
            return false;
          },
        ),
      );
    });

    test('span arithmetic laws hold', () {
      check(
        'span-laws',
        checkProperty<List<int>>(
          BigInt.from(6305),
          PbtOptions(numTests: 400, seed: BigInt.from(6305)),
          (rng) => Gen.list(2, 2, smallInt(0, 100000)).sample(rng),
          (pair) {
            final lo = pair[0] < pair[1] ? pair[0] : pair[1];
            final hi = pair[0] < pair[1] ? pair[1] : pair[0];
            const s = Span(0, 0);
            final span = Span(lo, hi);
            return span.length == hi - lo &&
                span.isEmpty == (lo == hi) &&
                s.compareTo(s) == 0;
          },
        ),
      );
    });

    test('span ordering is lexicographic and total', () {
      check(
        'span-order',
        checkProperty<List<int>>(
          BigInt.from(6306),
          PbtOptions(numTests: 400, seed: BigInt.from(6306)),
          (rng) => Gen.list(4, 4, smallInt(0, 1000)).sample(rng),
          (q) {
            final a = Span(q[0], q[1]);
            final b = Span(q[2], q[3]);
            final expected = q[0] != q[2]
                ? sign(q[0].compareTo(q[2]))
                : sign(q[1].compareTo(q[3]));
            if (sign(a.compareTo(b)) != expected) return false;
            // Antisymmetry.
            if (sign(b.compareTo(a)) != -expected) return false;
            // Equality agrees with comparison.
            return (a == b) == (expected == 0);
          },
        ),
      );
    });

    test('equal spans hash equally', () {
      check(
        'span-hash',
        checkProperty<List<int>>(
          BigInt.from(6307),
          PbtOptions(numTests: 200, seed: BigInt.from(6307)),
          (rng) => Gen.list(2, 2, smallInt(0, 100000)).sample(rng),
          (pair) {
            final a = Span(pair[0], pair[1]);
            final b = Span(pair[0], pair[1]);
            return a == b && a.hashCode == b.hashCode;
          },
        ),
      );
    });

    test('trivia table holds for sampled kinds', () {
      check(
        'trivia-table',
        checkProperty<int>(
          BigInt.from(6308),
          PbtOptions(numTests: 200, seed: BigInt.from(6308)),
          (rng) => smallInt(0, 19).sample(rng),
          (index) {
            const kinds = [
              TokenKind.name,
              TokenKind.number,
              TokenKind.string,
              TokenKind.regex,
              TokenKind.templateNoSub,
              TokenKind.templateHead,
              TokenKind.templateMiddle,
              TokenKind.templateTail,
              TokenKind.bigInt,
              TokenKind.privateName,
              TokenKind.keyword,
              TokenKind.operator,
              TokenKind.punctuation,
              TokenKind.comment,
              TokenKind.whitespace,
              TokenKind.newline,
              TokenKind.hashbang,
              TokenKind.error,
              TokenKind.eof,
              TokenKindOther('X'),
            ];
            const expected = [
              false,
              false,
              false,
              false,
              false,
              false,
              false,
              false,
              false,
              false,
              false,
              false,
              false,
              true,
              true,
              true,
              false,
              false,
              false,
              false,
            ];
            final kind = kinds[index];
            return kind.isTrivia == expected[index] &&
                kind.isEof == (kind == TokenKind.eof);
          },
        ),
      );
    });

    test('other payload laws hold', () {
      check(
        'other-laws',
        checkProperty<String>(
          BigInt.from(6309),
          PbtOptions(numTests: 200, seed: BigInt.from(6309)),
          (rng) => hostileId(0, 24).sample(rng),
          (name) {
            final a = TokenKindOther(name);
            final b = TokenKindOther(name);
            final c = TokenKindOther('${name}!');
            return a == b &&
                a.hashCode == b.hashCode &&
                a != c &&
                !a.isEof &&
                !a.isTrivia;
          },
        ),
      );
    });
  });
}
