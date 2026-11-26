/// Port of the reference `rust/javascript-tokens` unit suite (19
/// tests, of which 18 mirror directly — `default_is_latest` has no
/// Dart counterpart since the language has no `Default` trait; the
/// lane default is `EsVersion.latest`), plus lane extras for the
/// nullable parse twin, renderings, and the underflow divergence.
library;

import 'package:coding_adventures_javascript_tokens/javascript_tokens.dart';
import 'package:test/test.dart';

void main() {
  group('es-version', () {
    test('latest is es2025', () {
      expect(EsVersion.latest, equals(EsVersion.es2025));
    });

    test('as str matches grammar filenames', () {
      expect(
        [for (final v in EsVersion.all) v.asStr()],
        equals([
          'es1',
          'es3',
          'es5',
          'es2015',
          'es2016',
          'es2017',
          'es2018',
          'es2019',
          'es2020',
          'es2021',
          'es2022',
          'es2023',
          'es2024',
          'es2025',
        ]),
      );
    });

    test('round trip through strings', () {
      for (final v in EsVersion.all) {
        expect(EsVersion.parse(v.asStr()), equals(v));
      }
    });

    test('empty string is rejected', () {
      expect(() => EsVersion.parse(''), throwsA(isA<UnknownEsVersion>()));
      expect(EsVersion.tryParse(''), isNull);
    });

    test('unknown strings are rejected', () {
      for (final bad in [
        'es2',
        'es5.1',
        'latest',
        'ES2025',
        'es2026',
        ' es2025',
      ]) {
        expect(() => EsVersion.parse(bad), throwsA(isA<UnknownEsVersion>()),
            reason: 'expected $bad to be rejected');
        expect(EsVersion.tryParse(bad), isNull, reason: bad);
      }
    });

    test('display matches as str', () {
      expect(EsVersion.es2020.toString(), equals('es2020'));
    });

    test('unknown error message includes valid set', () {
      try {
        EsVersion.parse('nope');
        fail('expected UnknownEsVersion');
      } on UnknownEsVersion catch (e) {
        expect(e.input, equals('nope'));
        expect(e.message, contains('"nope"'));
        expect(e.message, contains('"es2025"'));
        expect(e.toString(), equals(e.message));
      }
    });

    test('ord is chronological', () {
      expect(EsVersion.es1.compareTo(EsVersion.es3), lessThan(0));
      expect(EsVersion.es5.compareTo(EsVersion.es2015), lessThan(0));
      expect(EsVersion.es2015.compareTo(EsVersion.es2025), lessThan(0));
      expect(EsVersion.es2025.compareTo(EsVersion.es2025), equals(0));
      expect(EsVersion.es2025.compareTo(EsVersion.es1), greaterThan(0));
    });

    test('nullable twin agrees with parse', () {
      for (final v in EsVersion.all) {
        expect(EsVersion.tryParse(v.asStr()), equals(v));
      }
    });
  });

  group('span', () {
    test('constructs from byte range', () {
      const s = Span(10, 20);
      expect(s.start, equals(10));
      expect(s.end, equals(20));
    });

    test('len is end minus start', () {
      expect(const Span(10, 20).length, equals(10));
      expect(const Span(0, 1).length, equals(1));
      expect(const Span(42, 42).length, equals(0));
    });

    test('is empty when start equals end', () {
      expect(const Span(0, 0).isEmpty, isTrue);
      expect(const Span(99, 99).isEmpty, isTrue);
      expect(const Span(0, 1).isEmpty, isFalse);
      expect(const Span(10, 20).isEmpty, isFalse);
    });

    test('spans compare and hash by value', () {
      const a = Span(3, 7);
      const b = Span(3, 7);
      const c = Span(3, 8);
      expect(a, equals(b));
      expect(a, isNot(equals(c)));
      expect(a.hashCode, equals(b.hashCode));
      expect(identical(a, b), isTrue);
    });

    test('supports const construction', () {
      // Dart getters are not const-evaluable (unlike the reference's
      // const fns), so const-ness is pinned via canonicalization
      // instead of const-evaluated length/emptiness.
      const s = Span(2, 5);
      expect(s.length, equals(3));
      expect(s.isEmpty, isFalse);
      expect(s.toString(), equals('Span(2, 5)'));
    });

    test('ord is lexicographic', () {
      expect(const Span(0, 5).compareTo(const Span(0, 6)), lessThan(0));
      expect(const Span(0, 5).compareTo(const Span(1, 2)), lessThan(0));
      expect(const Span(5, 10).compareTo(const Span(5, 5)), greaterThan(0));
      expect(const Span(1, 2).compareTo(const Span(1, 2)), equals(0));
    });

    test('violated invariant yields negative length, not a panic', () {
      // The reference's u32 subtraction panics in debug builds; Dart
      // ints cannot underflow, so the violation surfaces as a
      // negative length instead. Still outside the contract.
      expect(const Span(20, 10).length, equals(-10));
    });
  });

  group('token-kind', () {
    test('is trivia exhaustive', () {
      const cases = [
        (TokenKind.name, false),
        (TokenKind.number, false),
        (TokenKind.string, false),
        (TokenKind.regex, false),
        (TokenKind.templateNoSub, false),
        (TokenKind.templateHead, false),
        (TokenKind.templateMiddle, false),
        (TokenKind.templateTail, false),
        (TokenKind.bigInt, false),
        (TokenKind.privateName, false),
        (TokenKind.keyword, false),
        (TokenKind.operator, false),
        (TokenKind.punctuation, false),
        (TokenKind.comment, true),
        (TokenKind.whitespace, true),
        (TokenKind.newline, true),
        (TokenKind.hashbang, false),
        (TokenKind.error, false),
        (TokenKind.eof, false),
        (TokenKindOther('anything'), false),
      ];
      for (final (kind, expected) in cases) {
        expect(kind.isTrivia, equals(expected),
            reason: 'isTrivia disagrees for $kind');
        // Self-equality and hashing for every variant (also keeps
        // value-semantics coverage at 100%).
        expect(kind, equals(kind));
        expect(kind.hashCode, equals(kind.hashCode));
      }
    });

    test('is eof only for eof', () {
      expect(TokenKind.eof.isEof, isTrue);
      expect(TokenKind.name.isEof, isFalse);
      expect(TokenKind.newline.isEof, isFalse);
      expect(const TokenKindOther('EOF').isEof, isFalse);
    });

    test('equality', () {
      expect(TokenKind.name, equals(TokenKind.name));
      expect(TokenKind.name, isNot(equals(TokenKind.number)));
      expect(const TokenKindOther('X'), equals(const TokenKindOther('X')));
      expect(
          const TokenKindOther('X'), isNot(equals(const TokenKindOther('Y'))));
    });

    test('usable as map key', () {
      final counts = <TokenKind, int>{};
      counts.update(TokenKind.name, (v) => v + 1, ifAbsent: () => 1);
      counts.update(TokenKind.name, (v) => v + 1, ifAbsent: () => 1);
      counts.update(TokenKind.number, (v) => v + 1, ifAbsent: () => 1);
      counts.update(const TokenKindOther('FOO'), (v) => v + 5,
          ifAbsent: () => 5);
      expect(counts[TokenKind.name], equals(2));
      expect(counts[TokenKind.number], equals(1));
      expect(counts[const TokenKindOther('FOO')], equals(5));
      expect(counts[const TokenKindOther('BAR')], isNull);
    });

    test('renderings are pinned', () {
      expect(TokenKind.name.toString(), equals('TokenKind.name'));
      expect(TokenKind.number.toString(), equals('TokenKind.number'));
      expect(TokenKind.string.toString(), equals('TokenKind.string'));
      expect(TokenKind.regex.toString(), equals('TokenKind.regex'));
      expect(TokenKind.templateNoSub.toString(),
          equals('TokenKind.templateNoSub'));
      expect(
          TokenKind.templateHead.toString(), equals('TokenKind.templateHead'));
      expect(TokenKind.templateMiddle.toString(),
          equals('TokenKind.templateMiddle'));
      expect(
          TokenKind.templateTail.toString(), equals('TokenKind.templateTail'));
      expect(TokenKind.bigInt.toString(), equals('TokenKind.bigInt'));
      expect(TokenKind.privateName.toString(), equals('TokenKind.privateName'));
      expect(TokenKind.keyword.toString(), equals('TokenKind.keyword'));
      expect(TokenKind.operator.toString(), equals('TokenKind.operator'));
      expect(TokenKind.punctuation.toString(), equals('TokenKind.punctuation'));
      expect(TokenKind.comment.toString(), equals('TokenKind.comment'));
      expect(TokenKind.whitespace.toString(), equals('TokenKind.whitespace'));
      expect(TokenKind.newline.toString(), equals('TokenKind.newline'));
      expect(TokenKind.hashbang.toString(), equals('TokenKind.hashbang'));
      expect(TokenKind.error.toString(), equals('TokenKind.error'));
      expect(TokenKind.eof.toString(), equals('TokenKind.eof'));
      expect(
          const TokenKindOther('X').toString(), equals('TokenKind.other(X)'));
    });
  });
}
