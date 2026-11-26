/// Unit tests for `coding_adventures_test_matchers`.
///
/// Mirrors the TypeScript reference's `matchers.test.ts` group-for-group,
/// adapted for the documented Dart porting breaks (no `undefined`, generic
/// `toBeInstanceOf`, bool-only truthiness, `Type` arguments for `toThrow`).
/// Groups marked EXTRA cover Dart-only branches (BigInt/int mixing, string
/// indices, handmade cyclic values, custom matchers) needed for the 95%
/// line+branch coverage gate.
library;

import 'dart:typed_data';

import 'package:coding_adventures_test_matchers/coding_adventures_test_matchers.dart'
    hide expect;
import 'package:coding_adventures_test_matchers/coding_adventures_test_matchers.dart'
    as m;
import 'package:test/test.dart';

/// Runs [fn], expecting it to throw our [AssertionError], and returns it.
///
/// Using try/catch (instead of `throwsA`) keeps the failure itself available
/// for field assertions.
AssertionError catchFailure(void Function() fn) {
  try {
    fn();
  } on AssertionError catch (e) {
    return e;
  }
  fail('expected AssertionError, but nothing was thrown');
}

/// Async version of [catchFailure]: robust to the failure surfacing either
/// synchronously at call time or as a future error.
Future<AssertionError> catchFailureAsync(Future<void> Function() fn) async {
  try {
    await fn();
  } on AssertionError catch (e) {
    return e;
  }
  fail('expected AssertionError, but nothing was thrown');
}

class _Foo {
  final int val;
  _Foo(this.val);
}

class _Bar {
  final int val;
  _Bar(this.val);
}

class _Animal {}

class _Dog extends _Animal {}

class _WeirdLength {
  String get length => 'not-an-int';
}

void main() {
  group('toBe (identity)', () {
    test('compares primitive identity', () {
      m.expect(42).toBe(42);
      m.expect('hello').toBe('hello');
      m.expect(true).toBe(true);
      m.expect(null).toBe(null);
      m.expect(double.nan).toBe(double.nan);

      m.expect(42).not.toBe(43);
      m.expect('hello').not.toBe('world');
      m.expect(true).not.toBe(false);
      m.expect(null).not.toBe(false);
    });

    test('distinguishes +0.0 and -0.0 per Object.is', () {
      m.expect(0.0).toBe(0.0);
      m.expect(-0.0).toBe(-0.0);
      m.expect(0.0).not.toBe(-0.0);
      m.expect(-0.0).not.toBe(0.0);
    });

    test('compares reference identity for objects', () {
      final obj = {'a': 1};
      m.expect(obj).toBe(obj);
      m.expect({'a': 1}).not.toBe({'a': 1});
    });

    test('EXTRA: nonzero doubles and one-sided NaN', () {
      m.expect(1.5).toBe(1.5);
      m.expect(1.5).not.toBe(2.5);
      m.expect(double.nan).not.toBe(1.0);
      m.expect(1.0).not.toBe(double.nan);
    });

    test('EXTRA: BigInt and string value equality, int/double split', () {
      // Dart does not canonicalize these, so identity alone would be wrong.
      m.expect(BigInt.from(7)).toBe(BigInt.from(7));
      m.expect('a${'b'}c').toBe('abc');
      // int 1 and double 1.0 are different types in Dart (one number type
      // in JavaScript), so toBe distinguishes them. Documented break.
      m.expect(1).not.toBe(1.0);
      m.expect(1.0).not.toBe(1);
    });

    test('throws AssertionError on failure', () {
      final err = catchFailure(() => m.expect(1).toBe(2));
      expect(err, isA<AssertionError>());
      expect(err.runtimeType, AssertionError);
      expect(err.toString(), 'AssertionError: expected 1 to be 2');
      expect(err.actual, 1);
      expect(err.expected, 2);
      expect(err.operator, 'toBe');

      final negated = catchFailure(() => m.expect(1).not.toBe(1));
      expect(negated, isA<AssertionError>());
      expect(negated.message, 'not expected 1 to be 1');
    });
  });

  group('toEqual & deepEqual', () {
    test('compares deeply equal primitives and objects', () {
      m.expect({
        'a': 1,
        'b': [2, 3]
      }).toEqual({
        'a': 1,
        'b': [2, 3]
      });
      m.expect([1, 2, 3]).toEqual([1, 2, 3]);
      m
          .expect(Uint8List.fromList([1, 2, 3]))
          .toEqual(Uint8List.fromList([1, 2, 3]));
      m
          .expect(BigInt.parse('12345678901234567890'))
          .toEqual(BigInt.parse('12345678901234567890'));

      m.expect({'a': 1}).not.toEqual({'a': 2});
      m.expect([1, 2]).not.toEqual([1, 2, 3]);
      m.expect([1, 2]).not.toEqual([1, 4]);
      m
          .expect(Uint8List.fromList([1, 2]))
          .not
          .toEqual(Uint8List.fromList([1, 3]));
      m
          .expect(Uint8List.fromList([1, 2]))
          .not
          .toEqual(Uint8List.fromList([1, 2, 3]));
      m.expect(1).not.toEqual('1');
      m.expect(null).not.toEqual(false);
      m.expect('text').not.toEqual(Uint8List.fromList([116]));
      // int/double are distinct kinds in Dart (documented break).
      m.expect(1).not.toEqual(1.0);
    });

    test('compares DateTime, RegExp, and Error objects correctly', () {
      m.expect(DateTime.utc(2026)).toEqual(DateTime.utc(2026));
      m.expect(DateTime.utc(2026)).not.toEqual(DateTime.utc(2020));

      m.expect(RegExp('abc')).toEqual(RegExp('abc'));
      m.expect(RegExp('abc')).not.toEqual(RegExp('abc', caseSensitive: false));
      m.expect(RegExp('abc')).not.toEqual(RegExp('xyz'));

      m.expect(StateError('test error')).toEqual(StateError('test error'));
      m.expect(StateError('test error')).not.toEqual(StateError('other error'));
      m
          .expect(StateError('test error'))
          .not
          .toEqual(ArgumentError('test error'));
      // Exceptions take the error kind alongside Errors.
      m.expect(FormatException('bad')).toEqual(FormatException('bad'));
      m.expect(FormatException('bad')).not.toEqual(Exception('bad'));
    });

    test('handles DAGs with shared diamond references in O(N) time', () {
      Map<String, Object?> node = {'val': 0};
      for (var i = 0; i < 20; i++) {
        node = {'left': node, 'right': node, 'val': i + 1};
      }
      // Without post-completion memoization this re-traversal is 2^20
      // leaves and the test would hang; with it, it is instant.
      m.expect(node).toEqual(node);
    });

    test('enforces maximum nesting depth ceiling', () {
      Map<String, Object?> deep = {'val': 0};
      for (var i = 0; i < 70; i++) {
        deep = {'next': deep};
      }
      expect(
          () => toValue(deep),
          throwsA(isA<StateError>().having((e) => e.message, 'message',
              contains('Maximum nesting depth 64 exceeded'))));
    });

    test('handles Sets and Maps', () {
      final setA = {1, 2, 3};
      final setB = {1, 2, 3};
      m.expect(setA).toEqual(setB);

      final mapA = {'k1': 'v1', 'k2': 2};
      final mapB = {'k2': 2, 'k1': 'v1'};
      m.expect(mapA).toEqual(mapB);

      final mapC = {'k1': 'v1', 'k2': 3};
      m.expect(mapA).not.toEqual(mapC);

      final mapD = {'k1': 'v1'};
      m.expect(mapA).not.toEqual(mapD);

      final mapE = {'unmatched': 'v1', 'k2': 2};
      m.expect(mapA).not.toEqual(mapE);
    });

    test('EXTRA: duplicate map entries pair up distinctly', () {
      // Bipartite matching: each entry of `a` must claim a DISTINCT equal
      // entry of `b`. A naive "every entry matches some entry" check would
      // wrongly pass [{x,x}] vs [{x,y}].
      m.expect([
        {'k': 1},
        {'k': 1}
      ]).toEqual([
        {'k': 1},
        {'k': 1}
      ]);
      m
          .expect([
            {'k': 1},
            {'k': 1}
          ])
          .not
          .toEqual([
            {'k': 1},
            {'k': 2}
          ]);
    });

    test('handles deepEqual standalone helper', () {
      expect(deepEqual({'x': 1}, {'x': 1}), isTrue);
      expect(deepEqual([1, 2], [1, 3]), isFalse);
      expect(deepEqual(1, '1'), isFalse);
    });

    test('EXTRA: bools, doubles, kind mismatches, and byte shapes', () {
      m.expect(true).toEqual(true);
      m.expect(true).not.toEqual(false);
      m.expect(1.5).toEqual(1.5);
      m.expect(1.5).not.toEqual(2.5);
      // Canonicalization: NaN equals NaN, -0.0 equals +0.0 here (unlike toBe).
      m.expect(double.nan).toEqual(double.nan);
      m.expect(-0.0).toEqual(0.0);
      // List-vs-other and map-vs-other kind mismatches, both orders.
      expect(deepEqual([1], {'a': 1}), isFalse);
      expect(deepEqual({'a': 1}, [1]), isFalse);
      expect(deepEqual([1], 1), isFalse);
      expect(deepEqual(1, [1]), isFalse);
      expect(deepEqual({'a': 1}, 'x'), isFalse);
      expect(deepEqual('x', {'a': 1}), isFalse);
      // Same-instance bytes take the identical fast path; empty bytes take
      // the zero-trip loop path.
      final shared = Uint8List.fromList([9]);
      m.expect(shared).toEqual(shared);
      expect(deepEqual(Uint8List(0), Uint8List(0)), isTrue);
    });

    test('handles circular references gracefully', () {
      final circA = <String, Object?>{'x': 1};
      circA['self'] = circA;
      final circB = <String, Object?>{'x': 1};
      circB['self'] = circB;

      m.expect(circA).toEqual(circB);
      // The cycle collapses to a "[Circular]" text leaf, as in the reference.
      expect(encodeValue(toValue(circA)), contains('[Circular]'));
    });

    test('throws on toEqual mismatch', () {
      final err = catchFailure(() => m.expect({'x': 1}).toEqual({'x': 2}));
      expect(err, isA<AssertionError>());
      expect(err.operator, 'toEqual');
      expect(err.message, contains('to equal'));
    });
  });

  group('toStrictEqual', () {
    test('verifies structural and type equality including nested objects', () {
      final foo1 = _Foo(10);
      final foo2 = _Foo(10);
      final bar = _Bar(10);

      m.expect(foo1).toStrictEqual(foo2);
      m.expect(foo1).not.toStrictEqual(bar);

      // Nested type mismatch must be caught.
      m.expect({'child': foo1}).not.toStrictEqual({'child': bar});
    });

    test('EXTRA: lists, maps, and cyclic structures', () {
      // Same shape, same kinds: pass.
      m.expect([
        {'a': 1}
      ]).toStrictEqual([
        {'a': 1}
      ]);
      // List vs map: kind mismatch.
      m.expect([1]).not.toStrictEqual({'0': 1});
      // Length mismatches (values already differ, but the type check fires
      // first with the prototype message).
      final listErr = catchFailure(() => m.expect([1, 2]).toStrictEqual([1]));
      expect(listErr.message, contains('matching prototypes'));
      final mapErr = catchFailure(
          () => m.expect({'a': 1, 'b': 2}).toStrictEqual({'a': 1}));
      expect(mapErr.message, contains('matching prototypes'));
      // Generics are erased: List<Object> and List<int> share the list
      // kind, as all arrays share Array.prototype in JavaScript.
      m.expect(<Object>[1]).toStrictEqual(<int>[1]);
      // Cyclic structures terminate via the coinductive pair guard.
      final cycA = <String, Object?>{'x': 1};
      cycA['self'] = cycA;
      final cycB = <String, Object?>{'x': 1};
      cycB['self'] = cycB;
      m.expect(cycA).toStrictEqual(cycB);
    });

    test('throws when types differ', () {
      final err = catchFailure(() => m.expect(_Foo(1)).toStrictEqual(_Bar(1)));
      expect(err, isA<AssertionError>());
      expect(err.message, contains('matching prototypes'));
      expect(err.operator, 'toStrictEqual');
    });

    test('EXTRA: nulls, primitives, sets, functions, nested mismatches', () {
      // Null against anything passes the type check (values decide).
      m.expect(null).not.toStrictEqual(5);
      m.expect(5).not.toStrictEqual(null);
      catchFailure(() => m.expect(null).toStrictEqual(5));
      // Composite against primitive passes the type check too.
      m.expect([1]).not.toStrictEqual(5);
      m.expect(5).not.toStrictEqual([1]);
      // Sets share the set kind.
      m.expect({1, 2}).toStrictEqual({1, 2});
      m.expect({1, 2}).not.toStrictEqual({1, 3});
      // Functions share the function kind; same-signature closures render
      // identically ("Closure: () => int"), so they pass — unlike the
      // reference, which compares function source (documented break: Dart
      // exposes no source text).
      int Function() c1 = () => 1;
      int Function() c2 = () => 2;
      m.expect(c1).toStrictEqual(c2);
      int Function(int) c3 = (x) => x;
      m.expect(c1).not.toStrictEqual(c3);
      // Nested element/value mismatches fail the recursion.
      m
          .expect([
            [1]
          ])
          .not
          .toStrictEqual([
            [1, 2]
          ]);
      m
          .expect([
            [1]
          ])
          .not
          .toStrictEqual([
            {'a': 1}
          ]);
      m
          .expect({
            'a': [1]
          })
          .not
          .toStrictEqual({
            'a': [1, 2]
          });
    });

    test('EXTRA: value mismatch after matching types reports toEqual', () {
      // Mirrors the reference quirk: toStrictEqual delegates to toEqual, so
      // a pure value mismatch reports operator "toEqual".
      final err =
          catchFailure(() => m.expect({'x': 1}).toStrictEqual({'x': 2}));
      expect(err.operator, 'toEqual');
    });
  });

  group('truthiness & presence matchers', () {
    test('toBeTruthy and toBeFalsy are bool-only (documented break)', () {
      // JavaScript coerces (1, "str", {} are truthy); Dart has no truthiness
      // coercion, so ONLY the booleans themselves pass.
      m.expect(true).toBeTruthy();
      m.expect(false).toBeFalsy();

      for (final nonBool in [
        1,
        0,
        'str',
        '',
        null,
        <int>[],
        {'a': 1}
      ]) {
        catchFailure(() => m.expect(nonBool).toBeTruthy());
        catchFailure(() => m.expect(nonBool).toBeFalsy());
      }

      m.expect(false).not.toBeTruthy();
      m.expect(true).not.toBeFalsy();

      catchFailure(() => m.expect(false).toBeTruthy());
      catchFailure(() => m.expect(true).toBeFalsy());
    });

    test('toBeNull', () {
      m.expect(null).toBeNull();
      m.expect(42).not.toBeNull();

      catchFailure(() => m.expect(42).toBeNull());
    });

    test('toBeNaN', () {
      m.expect(double.nan).toBeNaN();
      m.expect(42).not.toBeNaN();
      m.expect('hello').not.toBeNaN();

      catchFailure(() => m.expect(42).toBeNaN());
      catchFailure(() => m.expect(double.nan).not.toBeNaN());
    });

    test('toBeInstanceOf', () {
      final dog = _Dog();

      m.expect(dog).toBeInstanceOf<_Dog>();
      m.expect(dog).toBeInstanceOf<_Animal>();
      m.expect(dog).not.toBeInstanceOf<RegExp>();

      final err = catchFailure(() => m.expect(dog).toBeInstanceOf<RegExp>());
      expect(err.operator, 'toBeInstanceOf');
      expect(err.message, contains('RegExp'));
    });
  });

  group('numeric relations', () {
    test('handles int comparisons', () {
      m.expect(10).toBeGreaterThan(5);
      m.expect(10).toBeGreaterThanOrEqual(10);
      m.expect(10).toBeGreaterThanOrEqual(9);
      m.expect(5).toBeLessThan(10);
      m.expect(5).toBeLessThanOrEqual(5);
      m.expect(4).toBeLessThanOrEqual(5);

      m.expect(5).not.toBeGreaterThan(10);
      m.expect(10).not.toBeLessThan(5);

      catchFailure(() => m.expect(5).toBeGreaterThan(10));
      catchFailure(() => m.expect(10).toBeLessThan(5));
    });

    test('handles BigInt comparisons', () {
      m.expect(BigInt.from(100)).toBeGreaterThan(BigInt.from(50));
      m.expect(BigInt.from(100)).toBeGreaterThanOrEqual(BigInt.from(100));
      m.expect(BigInt.from(50)).toBeLessThan(BigInt.from(100));
      m.expect(BigInt.from(50)).toBeLessThanOrEqual(BigInt.from(50));
    });

    test('EXTRA: mixed numerics, strings, NaN, and non-numerics', () {
      // int/BigInt mix exactly.
      m.expect(100).toBeGreaterThan(BigInt.from(50));
      m.expect(BigInt.from(50)).toBeLessThan(100);
      // double/BigInt mix via toDouble.
      m.expect(100.5).toBeGreaterThan(BigInt.from(50));
      m.expect(BigInt.from(50)).toBeLessThan(100.5);
      // Strings compare lexicographically (mirrors JavaScript relational).
      m.expect('b').toBeGreaterThan('a');
      m.expect('a').not.toBeGreaterThan('b');
      // NaN comparisons are always false, never throw (JavaScript rule).
      m.expect(double.nan).not.toBeGreaterThan(0);
      m.expect(double.nan).not.toBeLessThanOrEqual(double.nan);
      m.expect(0).not.toBeGreaterThan(double.nan);
      // Incomparable pairs mismatch instead of throwing.
      catchFailure(() => m.expect('a').toBeGreaterThan(5));
      catchFailure(() => m.expect(5).toBeLessThan('a'));
      catchFailure(() => m.expect(null).toBeGreaterThan(1));
    });

    test('EXTRA: inclusive comparisons fail cleanly', () {
      catchFailure(() => m.expect(5).toBeGreaterThanOrEqual(10));
      m.expect(5).not.toBeGreaterThanOrEqual(10);
      final geErr =
          catchFailure(() => m.expect(5).not.toBeGreaterThanOrEqual(5));
      expect(geErr.message, 'not expected 5 >= 5');
      catchFailure(() => m.expect(10).toBeLessThanOrEqual(5));
      m.expect(10).not.toBeLessThanOrEqual(5);
      final leErr = catchFailure(() => m.expect(5).not.toBeLessThanOrEqual(10));
      expect(leErr.message, 'not expected 5 <= 10');
    });

    test('toBeCloseTo floating-point comparisons', () {
      m.expect(0.1 + 0.2).toBeCloseTo(0.3, 2);
      m.expect(3.14159).toBeCloseTo(3.14, 2);
      m.expect(3.14159).toBeCloseTo(3.1416, 3);
      m.expect(10).not.toBeCloseTo(20, 1);

      final err = catchFailure(() => m.expect(10).toBeCloseTo(20, 1));
      expect(err.operator, 'toBeCloseTo');
      expect(err.message, contains('within tolerance'));
    });

    test('EXTRA: toBeCloseTo with non-numeric actual mismatches cleanly', () {
      catchFailure(() => m.expect('close').toBeCloseTo(1.0));
      catchFailure(() => m.expect(null).toBeCloseTo(1.0));
    });
  });

  group('collection & string matchers', () {
    test('toContain for strings, lists, sets, maps', () {
      m.expect('hello world').toContain('world');
      m.expect('hello world').not.toContain('moon');

      m.expect([1, 2, 3]).toContain(2);
      m.expect([1, 2, 3]).not.toContain(4);

      final s = {'apple', 'banana'};
      m.expect(s).toContain('apple');
      m.expect(s).not.toContain('orange');

      final testMap = {'key1': 100};
      m.expect(testMap).toContain('key1');
      m.expect(testMap).not.toContain('key2');

      // Deep containment: elements compare structurally.
      m.expect([
        {'a': 1}
      ]).toContain({'a': 1});

      final err = catchFailure(() => m.expect([1, 2]).toContain(3));
      expect(err.operator, 'toContain');
    });

    test('EXTRA: toContain on non-collections mismatches', () {
      catchFailure(() => m.expect(42).toContain(4));
      catchFailure(() => m.expect(null).toContain(null));
    });

    test('toHaveLength', () {
      m.expect([1, 2, 3]).toHaveLength(3);
      m.expect('hello').toHaveLength(5);
      m.expect(<int>[]).toHaveLength(0);
      m.expect([1]).not.toHaveLength(2);
      m.expect({'a': 1, 'b': 2}).toHaveLength(2);
      m.expect({1, 2}).toHaveLength(2);

      final err = catchFailure(() => m.expect([1]).toHaveLength(2));
      expect(err.operator, 'toHaveLength');
      expect(err.actual, 1);
      expect(err.expected, 2);
    });

    test('EXTRA: toHaveLength without a length reads null', () {
      final err = catchFailure(() => m.expect(42).toHaveLength(2));
      expect(err.message, 'expected length 2, got null');
      // A non-integer length also mismatches instead of throwing.
      catchFailure(() => m.expect(_WeirdLength()).toHaveLength(2));
    });

    test('toMatch strings and regex', () {
      m.expect('hello world').toMatch('world');
      m.expect('hello world').toMatch(RegExp('^hello'));
      m.expect('hello world').not.toMatch(RegExp('planet'));

      // Dart regexes carry no lastIndex state, so reuse always matches.
      final reused = RegExp('hello');
      m.expect('hello world').toMatch(reused);
      m.expect('hello world').toMatch(reused);

      final err =
          catchFailure(() => m.expect('hello').toMatch(RegExp('planet')));
      expect(err.operator, 'toMatch');
      expect(err.message, 'expected "hello" to match /planet/gu');
      catchFailure(() => m.expect(123).toMatch('123'));
    });

    test('toHaveProperty presence and value checks', () {
      final obj = {
        'foo': 'bar',
        'count': 42,
        'nested': {'x': 1}
      };
      m.expect(obj).toHaveProperty('foo');
      m.expect(obj).toHaveProperty('foo', 'bar');
      m.expect(obj).toHaveProperty('count', 42);
      m.expect(obj).toHaveProperty('nested', {'x': 1});

      m.expect(obj).not.toHaveProperty('nonexistent');
      m.expect(obj).not.toHaveProperty('foo', 'baz');

      // Dart maps have no prototype chain, so these never leak.
      m.expect(<String, Object?>{}).not.toHaveProperty('toString');
      m.expect(<String, Object?>{}).not.toHaveProperty('length');

      catchFailure(() => m.expect(obj).toHaveProperty('missing'));
      catchFailure(() => m.expect(obj).toHaveProperty('foo', 'wrong'));
      catchFailure(() => m.expect(null).toHaveProperty('foo'));
    });

    test('EXTRA: toHaveProperty on lists and explicit nulls', () {
      // Lists accept integer indices.
      m.expect(['a', 'b']).toHaveProperty(0);
      m.expect(['a', 'b']).toHaveProperty(1, 'b');
      catchFailure(() => m.expect(['a', 'b']).toHaveProperty(5));
      catchFailure(() => m.expect(['a', 'b']).toHaveProperty(1, 'z'));
      // ... but not string indices (the reference accepts "0" via
      // hasOwnProperty; Dart lists are int-indexed — documented break).
      catchFailure(() => m.expect(['a', 'b']).toHaveProperty('0'));
      // An explicitly passed null COMPARES (omission uses a marker).
      m.expect({'k': null}).toHaveProperty('k', null);
      catchFailure(() => m.expect({'k': 1}).toHaveProperty('k', null));
      // Null KEYS work too: Dart maps allow them (found by the fuzzer with
      // input [215, 127], which decodes to {null: null}).
      m.expect({null: 'v'}).toHaveProperty(null);
      m.expect({null: 'v'}).toHaveProperty(null, 'v');
      catchFailure(() => m.expect({'a': 1}).toHaveProperty(null));
      catchFailure(() => m.expect(['a']).toHaveProperty(null));
      // Non-collections fail with the object message.
      final err = catchFailure(() => m.expect(42).toHaveProperty('foo'));
      expect(err.message, contains('expected object with property'));
      // Negating a present key fails with the presence message.
      final negated =
          catchFailure(() => m.expect(['a', 'b']).not.toHaveProperty(0));
      expect(negated.message, 'not object has property 0');
    });
  });

  group('toThrow exceptions', () {
    test('catches thrown errors and validates message/type', () {
      void throwState() => throw StateError('unauthorized access to resource');
      void throwArgument() => throw ArgumentError('invalid type specified');
      void throwStr() => throw 'custom string error';
      int noThrow() => 42;

      m.expect(throwState).toThrow();
      m.expect(throwState).toThrow('unauthorized');
      m.expect(throwState).toThrow(RegExp('access to resource'));
      m.expect(throwState).toThrow(StateError);
      m
          .expect(throwState)
          .toThrow(StateError('unauthorized access to resource'));
      catchFailure(
          () => m.expect(throwState).toThrow(StateError('wrong message')));
      catchFailure(() => m
          .expect(throwState)
          .toThrow(ArgumentError('unauthorized access to resource')));

      m.expect(throwArgument).toThrow(ArgumentError);
      m.expect(throwStr).toThrow('custom string');

      m.expect(noThrow).not.toThrow();

      catchFailure(() => m.expect(noThrow).toThrow());
      catchFailure(() => m.expect(throwState).not.toThrow());
      catchFailure(() => m.expect(throwState).toThrow('missing text'));
      catchFailure(() => m.expect(throwState).toThrow(RegExp('missing regex')));
      catchFailure(() => m.expect(throwState).toThrow(ArgumentError));
      final nonFn = catchFailure(() => m.expect(123).toThrow());
      expect(nonFn.message, 'toThrow() requires a function');
    });

    test('EXTRA: error roots, quirk fall-through, and thrown null', () {
      void throwState() => throw StateError('boom');
      // The Error/Exception roots match any error/exception (mirrors the
      // reference's instanceof, which Dart cannot express for Type values).
      m.expect(throwState).toThrow(Error);
      void throwFormat() => throw FormatException('bad');
      m.expect(throwFormat).toThrow(Exception);
      m.expect(throwFormat).not.toThrow(Error);
      // An expected value of any other type performs NO assertion — the
      // reference falls through silently, and so do we.
      m.expect(throwState).toThrow(42);
      // A thrown null still counts as throwing.
      void throwNull() => throw null as dynamic;
      m.expect(throwNull).toThrow();
      catchFailure(() => m.expect(throwNull).toThrow(StateError));
    });
  });

  group('async matchers (resolves & rejects)', () {
    test('resolves handles async values across all matchers', () async {
      await m.expect(Future.value(42)).resolves.toBe(42);
      await m.expect(Future.value({'a': 1})).resolves.toEqual({'a': 1});
      await m.expect(Future.value({'a': 1})).resolves.toStrictEqual({'a': 1});
      await m.expect(Future.value(true)).resolves.toBeTruthy();
      await m.expect(Future.value(false)).resolves.toBeFalsy();
      await m.expect(Future.value(null)).resolves.toBeNull();
      await m
          .expect(Future.value(StateError('msg')))
          .resolves
          .toBeInstanceOf<StateError>();
      await m.expect(Future.value(double.nan)).resolves.toBeNaN();
      await m.expect(Future.value(10)).resolves.toBeGreaterThan(5);
      await m.expect(Future.value(10)).resolves.toBeGreaterThanOrEqual(10);
      await m.expect(Future.value(5)).resolves.toBeLessThan(10);
      await m.expect(Future.value(5)).resolves.toBeLessThanOrEqual(5);
      await m.expect(Future.value(3.14)).resolves.toBeCloseTo(3.14, 2);
      await m.expect(Future.value([1, 2])).resolves.toHaveLength(2);
      await m.expect(Future.value([1, 2])).resolves.toContain(2);
      await m.expect(Future.value('foobar')).resolves.toMatch(RegExp('bar'));
      await m.expect(Future.value({'key': 1})).resolves.toHaveProperty('key');
      await m
          .expect(Future.value({'key': 1}))
          .resolves
          .toHaveProperty('key', 1);

      await m.expect(Future.value(42)).resolves.not.toBe(43);
      await catchFailureAsync(
          () => m.expect(Future.value(42)).resolves.toBe(43));
    });

    test('EXTRA: resolves flattens nested futures and plain values', () async {
      final nested = Future<Future<int>>.value(Future<int>.value(7));
      await m.expect(nested).resolves.toBe(7);
      // A non-future actual resolves immediately, like Promise.resolve.
      await m.expect(42).resolves.toBe(42);
    });

    test('EXTRA: resolves propagates rejections to the awaiting matcher',
        () async {
      final bad = Future<int>.error(StateError('async boom'));
      var sawStateError = false;
      try {
        await m.expect(bad).resolves.toBe(1);
      } on StateError {
        sawStateError = true;
      }
      expect(sawStateError, isTrue);
    });

    test('rejects handles rejected futures', () async {
      Future<int> rejected() => Future.error(StateError('failure'));

      await m.expect(rejected()).rejects.toBeInstanceOf<StateError>();
      await m.expect(rejected()).rejects.toThrow('failure');
      await m.expect(rejected()).rejects.toThrow(RegExp('fail'));
      await m.expect(rejected()).rejects.toThrow(StateError);
      await m.expect(rejected()).rejects.toThrow(Error);
      await m.expect(rejected()).rejects.toThrow(StateError('failure'));

      await catchFailureAsync(
          () => m.expect(Future.value(42)).resolves.toThrow());
    });

    test('EXTRA: rejects on a resolving future fails cleanly', () async {
      final err = await catchFailureAsync(
          () => m.expect(Future.value(42)).rejects.toBe(42));
      expect(err.message, 'expected promise to reject, but it resolved');
      expect(err.actual, 42);
    });
  });

  group('deep equality utilities', () {
    test('canonicalizes NaN patterns', () {
      final qNaNBits = BigInt.parse('7ff8000000000001', radix: 16);
      final canonicalBits = canonicalF64(qNaNBits);
      expect(canonicalBits, BigInt.parse('7ff8000000000000', radix: 16));

      final floatBits = numberToF64Bits(3.14);
      expect(canonicalF64(floatBits), floatBits);
      expect(f64BitsToNumber(floatBits), 3.14);
    });

    test('EXTRA: canonicalF64 fixes -0 and leaves the rest alone', () {
      expect(canonicalF64(numberToF64Bits(-0.0)), numberToF64Bits(0.0));
      expect(canonicalF64(numberToF64Bits(double.infinity)),
          numberToF64Bits(double.infinity));
      expect(canonicalF64(numberToF64Bits(double.negativeInfinity)),
          numberToF64Bits(double.negativeInfinity));
      // Negative-NaN and signaling NaN collapse to the quiet NaN.
      expect(canonicalF64(BigInt.parse('fff8000000000000', radix: 16)),
          BigInt.parse('7ff8000000000000', radix: 16));
      expect(canonicalF64(BigInt.parse('7ff0000000000001', radix: 16)),
          BigInt.parse('7ff8000000000000', radix: 16));
    });

    test('EXTRA: f64 bit round-trip is bit-exact', () {
      for (final v in [
        0.0,
        -0.0,
        1.0,
        -1.5,
        3.14,
        double.infinity,
        double.negativeInfinity,
        5e-324, // smallest subnormal
        1.7976931348623157e308, // largest finite
      ]) {
        expect(f64BitsToNumber(numberToF64Bits(v)), v);
        expect(numberToF64Bits(f64BitsToNumber(numberToF64Bits(v))),
            numberToF64Bits(v));
      }
      // NaN payloads survive the converters (canonicalization is separate).
      final payload = BigInt.parse('7ff8000000000001', radix: 16);
      expect(numberToF64Bits(f64BitsToNumber(payload)), payload);
    });

    test('encodes values accurately', () {
      expect(encodeValue(const NullValue()), 'null');
      expect(encodeValue(const BoolValue(true)), 'true');
      expect(encodeValue(const IntValue('42')), '42n');
      expect(encodeValue(const TextValue('hi')), '"hi"');
      expect(encodeValue(BytesValue(Uint8List.fromList([0xde, 0xad]))),
          'Uint8List([dead])');
      expect(encodeValue(const ListValue([IntValue('1')])), '[1n]');
    });

    test('EXTRA: encodes every remaining kind', () {
      expect(encodeValue(F64Value(numberToF64Bits(0.0))), '0');
      expect(encodeValue(F64Value(numberToF64Bits(-0.0))), '0');
      expect(encodeValue(F64Value(numberToF64Bits(double.nan))), 'NaN');
      expect(
          encodeValue(F64Value(numberToF64Bits(double.infinity))), 'Infinity');
      expect(encodeValue(F64Value(numberToF64Bits(double.negativeInfinity))),
          '-Infinity');
      expect(encodeValue(F64Value(numberToF64Bits(3.14))), '3.14');
      expect(encodeValue(DateValue(0)), 'Date(1970-01-01T00:00:00.000Z)');
      expect(encodeValue(const RegexpValue('a+', 'giu')), '/a+/giu');
      expect(encodeValue(const ErrorValue('StateError', 'Bad state: x')),
          'StateError: Bad state: x');
      expect(
          encodeValue(const MapValue([
            (TextValue('k'), IntValue('1')),
            (IntValue('2'), BoolValue(false)),
          ])),
          '{"k" => 1n, 2n => false}');
      expect(encodeValue(toValue(RegExp('x'))), '/x/gu');
    });

    test('EXTRA: encodes handmade cyclic values as [Circular]', () {
      // toValue output is always finite (cycles collapse during conversion),
      // but a hand-built cyclic Value must still print instead of hanging.
      final items = <Value>[];
      final node = ListValue(items);
      items.add(node);
      expect(encodeValue(node), '[[Circular]]');
      final entries = <(Value, Value)>[];
      final mapNode = MapValue(entries);
      entries.add((const TextValue('self'), mapNode));
      expect(encodeValue(mapNode), '{"self" => [Circular]}');
    });

    test('EXTRA: regexpFlags covers all flag combinations', () {
      expect(regexpFlags(RegExp('a')), 'gu');
      expect(regexpFlags(RegExp('a', caseSensitive: false)), 'giu');
      expect(regexpFlags(RegExp('a', multiLine: true)), 'gmu');
      expect(regexpFlags(RegExp('a', caseSensitive: false, multiLine: true)),
          'gimu');
    });
  });

  group('EXTRA: toValue branch coverage', () {
    test('converts every leaf kind', () {
      expect(toValue(null), isA<NullValue>());
      expect(toValue(true), isA<BoolValue>());
      expect(toValue(7), isA<IntValue>());
      expect(toValue(BigInt.two), isA<IntValue>());
      expect(toValue(1.5), isA<F64Value>());
      expect(toValue('s'), isA<TextValue>());
      expect(toValue(Uint8List(0)), isA<BytesValue>());
      expect(toValue(DateTime.utc(2020)), isA<DateValue>());
      expect(toValue(RegExp('r')), isA<RegexpValue>());
      expect(toValue(StateError('e')), isA<ErrorValue>());
      expect(toValue(Exception('e')), isA<ErrorValue>());
      expect(toValue([1]), isA<ListValue>());
      expect(toValue({1}), isA<ListValue>());
      expect(toValue({'k': 1}), isA<MapValue>());
      // Functions and arbitrary objects render as text.
      expect(toValue(() {}), isA<TextValue>());
      expect(toValue(_Foo(1)), isA<TextValue>());
      expect(toValue(Symbol('sym')), isA<TextValue>());
    });

    test('memoizes shared references by identity', () {
      final shared = [1];
      final root = [shared, shared];
      final converted = toValue(root) as ListValue;
      expect(converted.items[0], same(converted.items[1]));
    });
  });

  group('EXTRA: custom matchers via assertResult', () {
    test('passes and fails through MatcherResult', () {
      m.expect(5).assertResult(const MatcherResult(
          pass: true, message: 'unreachable', operator: 'custom'));
      final err = catchFailure(() => m.expect(5).assertResult(
          const MatcherResult(
              pass: false,
              message: 'expected five to be five-ish',
              actual: 5,
              expected: 'five-ish',
              operator: 'custom')));
      expect(err.message, 'expected five to be five-ish');
      expect(err.actual, 5);
      expect(err.expected, 'five-ish');
      expect(err.operator, 'custom');
      // Negation flips and prefixes, like every built-in matcher.
      final negated = catchFailure(() => m.expect(5).not.assertResult(
          const MatcherResult(pass: true, message: 'five is five-ish')));
      expect(negated.message, 'not five is five-ish');
      expect(negated.operator, isNull);
    });
  });
}
