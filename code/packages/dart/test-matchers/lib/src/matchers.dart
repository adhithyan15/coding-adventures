/// Jest/Vitest-style expectations with bit-exact equality.
///
/// This module is the Dart lane of the TypeScript `matchers` module from the
/// reference `test-matchers` package. The entry point is [expect], which
/// builds an [Expectation]; async assertions go through [Expectation.resolves]
/// and [Expectation.rejects], which build an [AsyncExpectation].
///
/// Two deliberate breaks from the TypeScript API, both forced by the Dart
/// type system (see the package README for the full porting table):
///
/// * There is no `toBeUndefined`/`toBeDefined`: Dart has no `undefined`.
/// * [Expectation.toBeInstanceOf] takes a type argument
///   (`toBeInstanceOf<StateError>()`) instead of a constructor value, and
///   [Expectation.toThrow] takes a [Type] object (`toThrow(StateError)`
///   still reads the same at the call site).
library;

import 'dart:convert';
import 'dart:math' as math;

import 'core_assertion.dart' as core_assertion;

import 'deep_equal.dart';

/// The error thrown when an expectation fails.
///
/// Carries the human-readable [message] plus the structured [actual],
/// [expected], and [operator] fields, mirroring the TypeScript reference. It
/// extends Dart's own [core_assertion.AssertionError], so a failure is
/// caught both by `on AssertionError` (this class, when this library is
/// imported) and by `on AssertionError` from `dart:core` anywhere.
class AssertionError extends core_assertion.AssertionError {
  /// The value the expectation ran against (when meaningful).
  final Object? actual;

  /// The value the expectation compared against (when meaningful).
  final Object? expected;

  /// The matcher that failed (e.g. `"toEqual"`), if any.
  final String? operator;

  /// Creates a failure with [message] and optional structured fields.
  AssertionError(
    String super.message, [
    this.actual,
    this.expected,
    this.operator,
  ]);

  @override
  String toString() => 'AssertionError: $message';
}

/// The outcome of one matcher evaluation: the extension point for custom
/// matchers built on [Expectation.assertResult].
class MatcherResult {
  /// Whether the (unnegated) matcher condition held.
  final bool pass;

  /// The failure message, used when the assertion (after negation) fails.
  final String message;

  /// The value the matcher ran against.
  final Object? actual;

  /// The value the matcher compared against.
  final Object? expected;

  /// The matcher name (e.g. `"toEqual"`).
  final String? operator;

  /// Creates a matcher outcome.
  const MatcherResult({
    required this.pass,
    required this.message,
    this.actual,
    this.expected,
    this.operator,
  });
}

/// `Object.is` semantics for Dart values: the rule behind [Expectation.toBe].
///
/// * doubles: NaN equals NaN, but −0.0 and +0.0 differ (via [double.isNegative]);
/// * strings and [BigInt]s: value equality (Dart does not canonicalize them,
///   so [identical] alone would be wrong);
/// * everything else: [identical].
///
/// In particular `1` (int) and `1.0` (double) are *not* the same value here:
/// Dart gives them different types, where JavaScript has only one number
/// type. Cross-type numeric closeness is [Expectation.toBeCloseTo]'s job.
bool _sameValue(Object? a, Object? b) {
  if (a is double || b is double) {
    if (a is double && b is double) {
      if (a.isNaN && b.isNaN) return true;
      if (a.isNaN || b.isNaN) return false;
      if (a == 0 && b == 0) return a.isNegative == b.isNegative;
      return a == b;
    }
    return false;
  }
  if (a is String && b is String) return a == b;
  if (a is BigInt && b is BigInt) return a == b;
  return identical(a, b);
}

/// Total ordering comparison for the relational matchers.
///
/// Returns the [Comparable.compareTo] result for `(num, num)`,
/// `(BigInt, BigInt)`, exact mixed `(int, BigInt)` pairs, `(String, String)`
/// lexicographic pairs (mirroring JavaScript's relational string comparison),
/// and mixed `(double, BigInt)` pairs via [BigInt.toDouble]. Anything else —
/// including any NaN operand (JavaScript relational comparisons with NaN are
/// always false) and mixed string/number pairs (JavaScript coercion has no
/// Dart analog) — returns `null`, which every relational matcher reads as a
/// mismatch rather than throwing.
int? _compareValues(Object? a, Object? e) {
  if ((a is double && a.isNaN) || (e is double && e.isNaN)) return null;
  if (a is num && e is num) return a.compareTo(e);
  if (a is BigInt && e is BigInt) return a.compareTo(e);
  if (a is int && e is BigInt) return BigInt.from(a).compareTo(e);
  if (a is BigInt && e is int) return a.compareTo(BigInt.from(e));
  if (a is double && e is BigInt) return a.compareTo(e.toDouble());
  if (a is BigInt && e is double) return a.toDouble().compareTo(e);
  if (a is String && e is String) return a.compareTo(e);
  return null;
}

/// An identity-compared pair, for cycle detection in [_matchingTypes].
class _IdentityPair {
  final Object? first;
  final Object? second;
  const _IdentityPair(this.first, this.second);

  @override
  bool operator ==(Object other) =>
      other is _IdentityPair &&
      identical(first, other.first) &&
      identical(second, other.second);

  @override
  int get hashCode =>
      Object.hash(identityHashCode(first), identityHashCode(second));
}

/// Whether two values share the same composite "prototype".
///
/// The Dart counterpart of the reference's `hasMatchingPrototypes`, and the
/// first half of [Expectation.toStrictEqual] (the second half is [toEqual]).
/// Identical values match; primitives always match (their values were already
/// compared); then both sides must share one composite kind — every [List]
/// shares the list kind regardless of type arguments (as in JavaScript,
/// where all arrays share `Array.prototype`), likewise [Set], [Map], and
/// [Function]; every other object must share its exact runtime type. Lists
/// additionally require equal lengths with recursively matching elements,
/// and maps require equal lengths with recursively matching values.
///
/// Pairs already on the comparison path match coinductively, so cyclic
/// structures terminate.
bool _matchingTypes(Object? a, Object? b, Set<_IdentityPair> seen) {
  if (identical(a, b)) return true;
  if (a == null || b == null) return true;
  if (a is bool || a is num || a is String || a is BigInt) return true;
  if (b is bool || b is num || b is String || b is BigInt) return true;
  if (!seen.add(_IdentityPair(a, b))) return true;
  try {
    if (_compositeKind(a) != _compositeKind(b)) return false;
    if (a is List && b is List) {
      if (a.length != b.length) return false;
      for (var i = 0; i < a.length; i++) {
        if (!_matchingTypes(a[i], b[i], seen)) return false;
      }
      return true;
    }
    if (a is Map && b is Map) {
      if (a.length != b.length) return false;
      for (final key in a.keys) {
        if (!_matchingTypes(a[key], b[key], seen)) return false;
      }
      return true;
    }
    return true;
  } finally {
    seen.remove(_IdentityPair(a, b));
  }
}

/// The erased composite kind of a non-null, non-primitive value.
///
/// Collections and functions share one kind each (generics erased); every
/// other object is keyed by its exact runtime type.
Object _compositeKind(Object value) {
  if (value is List) return 'list';
  if (value is Set) return 'set';
  if (value is Map) return 'map';
  if (value is Function) return 'function';
  return value.runtimeType;
}

/// Marker for "argument not passed", distinguishing an omitted optional
/// argument from an explicitly passed `null`.
///
/// A private type with a single canonical instance: user code cannot name
/// the type, so it cannot forge the marker.
class _Absent {
  const _Absent();
}

/// The canonical [_Absent] marker instance.
const _absentValue = _Absent();

/// A synchronous expectation over one actual value.
///
/// Created by [expect]; every matcher either returns normally (pass) or
/// throws [AssertionError] (fail). [not] negates the next matcher,
/// [resolves] and [rejects] switch to async assertions.
class Expectation<T> {
  /// The value under test.
  final T actual;

  /// Whether the next matcher is negated.
  final bool _isNot;

  /// Creates an expectation over [actual], negated iff [isNot].
  Expectation(this.actual, [this._isNot = false]);

  /// The negation of this expectation.
  Expectation<T> get not => Expectation<T>(actual, !_isNot);

  /// Assert against the value a future completes with.
  ///
  /// `actual` may be a [Future] (at any nesting depth — nesting is
  /// flattened) or a plain value. If the future completes with an error,
  /// awaiting matchers rethrow that error, exactly like the reference.
  AsyncExpectation<Object?> get resolves =>
      AsyncExpectation<Object?>(_flatten(actual), _isNot, false);

  /// Assert against the error a future completes with.
  ///
  /// If the future completes with a value instead, awaiting matchers throw
  /// `AssertionError("expected promise to reject, but it resolved")`.
  AsyncExpectation<Object?> get rejects {
    final future = _flatten(actual).then<Object?>(
      (val) => throw AssertionError(
        'expected promise to reject, but it resolved',
        val,
      ),
      onError: (Object e, StackTrace s) => e,
    );
    return AsyncExpectation<Object?>(future, _isNot, true);
  }

  /// Flatten nested futures, mirroring `Promise.resolve` flattening.
  static Future<Object?> _flatten(Object? value) async {
    dynamic current = value;
    while (current is Future) {
      current = await current;
    }
    return current;
  }

  /// Evaluate [result], honoring negation.
  ///
  /// The extension point for custom matchers: compute a [MatcherResult] and
  /// hand it here; a failing (post-negation) result throws [AssertionError]
  /// with `"not "` prefixed to the message under [not], exactly like the
  /// reference's `assert`.
  void assertResult(MatcherResult result) {
    _assertLazy(
      result.pass,
      () => result.message,
      result.actual,
      result.expected,
      result.operator,
    );
  }

  /// Evaluate one matcher outcome, honoring negation.
  ///
  /// When the post-negation outcome fails, throws [AssertionError] with the
  /// lazily built message (`"not "`-prefixed under [not]) and the
  /// structured fields.
  void _assertLazy(
    bool pass,
    String Function() messageFn, [
    Object? actual,
    Object? expected,
    String? operator,
  ]) {
    final passed = _isNot ? !pass : pass;
    if (!passed) {
      final message = _isNot ? 'not ${messageFn()}' : messageFn();
      throw AssertionError(message, actual, expected, operator);
    }
  }

  /// Passes iff [actual] is the [expected] value ([_sameValue]).
  void toBe(Object? expected) {
    final pass = _sameValue(actual, expected);
    _assertLazy(
      pass,
      () => 'expected $actual to be $expected',
      actual,
      expected,
      'toBe',
    );
  }

  /// Passes iff [actual] deep-equals [expected] ([deepEqual]).
  void toEqual(Object? expected) {
    final aVal = toValue(actual);
    final eVal = toValue(expected);
    final pass = valuesEqual(aVal, eVal);
    _assertLazy(
      pass,
      () => 'expected ${encodeValue(aVal)} to equal ${encodeValue(eVal)}',
      actual,
      expected,
      'toEqual',
    );
  }

  /// Passes iff [actual] deep-equals [expected] with matching types.
  ///
  /// Types are checked first ([_matchingTypes]); on success this delegates
  /// to [toEqual], so a value mismatch reports operator `"toEqual"` —
  /// mirroring the reference quirk-for-quirk.
  void toStrictEqual(Object? expected) {
    if (!_matchingTypes(actual, expected, <_IdentityPair>{})) {
      _assertLazy(
        false,
        () => 'expected objects to have matching prototypes',
        actual,
        expected,
        'toStrictEqual',
      );
      return;
    }
    toEqual(expected);
  }

  /// Passes iff [actual] is identical to `true`.
  ///
  /// Dart has no truthy/falsy coercion: only the boolean `true` passes.
  /// This is the one matcher whose rule differs from the reference in
  /// substance rather than merely in type-system mapping.
  void toBeTruthy() {
    final pass = identical(actual, true);
    _assertLazy(
      pass,
      () => 'expected $actual to be truthy',
      actual,
      null,
      'toBeTruthy',
    );
  }

  /// Passes iff [actual] is identical to `false`.
  ///
  /// As with [toBeTruthy], no coercion: only the boolean `false` passes.
  void toBeFalsy() {
    final pass = identical(actual, false);
    _assertLazy(
      pass,
      () => 'expected $actual to be falsy',
      actual,
      null,
      'toBeFalsy',
    );
  }

  /// Passes iff [actual] is `null`.
  void toBeNull() {
    final pass = identical(actual, null);
    _assertLazy(
      pass,
      () => 'expected $actual to be null',
      actual,
      null,
      'toBeNull',
    );
  }

  /// Passes iff [actual] is an instance of `U`.
  ///
  /// The type argument replaces the reference's constructor value:
  /// `expect(x).toBeInstanceOf<StateError>()`.
  void toBeInstanceOf<U>() {
    final pass = actual is U;
    _assertLazy(
      pass,
      () => 'expected value to be instance of $U',
      actual,
      null,
      'toBeInstanceOf',
    );
  }

  /// Passes iff [actual] is a NaN double.
  void toBeNaN() {
    final target = actual;
    final pass = target is double && target.isNaN;
    _assertLazy(
      pass,
      () => 'expected $target to be NaN',
      target,
      null,
      'toBeNaN',
    );
  }

  /// Passes iff [actual] is greater than [n] ([_compareValues]).
  void toBeGreaterThan(Object? n) {
    final cmp = _compareValues(actual, n);
    _assertLazy(
      cmp != null && cmp > 0,
      () => 'expected $actual > $n',
      actual,
      n,
      'toBeGreaterThan',
    );
  }

  /// Passes iff [actual] is greater than or equal to [n] ([_compareValues]).
  void toBeGreaterThanOrEqual(Object? n) {
    final cmp = _compareValues(actual, n);
    _assertLazy(
      cmp != null && cmp >= 0,
      () => 'expected $actual >= $n',
      actual,
      n,
      'toBeGreaterThanOrEqual',
    );
  }

  /// Passes iff [actual] is less than [n] ([_compareValues]).
  void toBeLessThan(Object? n) {
    final cmp = _compareValues(actual, n);
    _assertLazy(
      cmp != null && cmp < 0,
      () => 'expected $actual < $n',
      actual,
      n,
      'toBeLessThan',
    );
  }

  /// Passes iff [actual] is less than or equal to [n] ([_compareValues]).
  void toBeLessThanOrEqual(Object? n) {
    final cmp = _compareValues(actual, n);
    _assertLazy(
      cmp != null && cmp <= 0,
      () => 'expected $actual <= $n',
      actual,
      n,
      'toBeLessThanOrEqual',
    );
  }

  /// Passes iff [actual] is within `10^-precision / 2` of [expected].
  ///
  /// A non-numeric [actual] never throws here: it simply mismatches, exactly
  /// like the reference (whose subtraction yields NaN).
  void toBeCloseTo(num expected, [int precision = 2]) {
    final tolerance = math.pow(10, -precision) / 2;
    final target = actual;
    final pass = target is num && ((target - expected).abs() < tolerance);
    _assertLazy(
      pass,
      () =>
          'expected $target to be close to $expected within tolerance $tolerance',
      target,
      expected,
      'toBeCloseTo',
    );
  }

  /// Passes iff [actual] contains [item].
  ///
  /// Strings contain substrings; lists and sets contain deep-equal elements
  /// ([deepEqual]); maps contain deep-equal keys.
  void toContain(Object? item) {
    var pass = false;
    final target = actual;
    if (target is String && item is String) {
      pass = target.contains(item);
    } else if (target is List) {
      final itemVal = toValue(item);
      pass = target.any((x) => valuesEqual(toValue(x), itemVal));
    } else if (target is Set) {
      final itemVal = toValue(item);
      pass = target.any((x) => valuesEqual(toValue(x), itemVal));
    } else if (target is Map) {
      final itemVal = toValue(item);
      pass = target.keys.any((k) => valuesEqual(toValue(k), itemVal));
    }
    _assertLazy(
      pass,
      () => 'expected collection to contain $item',
      target,
      item,
      'toContain',
    );
  }

  /// Passes iff [actual]'s `length` equals [expectedLength].
  ///
  /// The length is read duck-typed (mirroring the reference's optional
  /// `.length` read): anything without an integer `length` simply
  /// mismatches. Note Dart closures have no `.length` (JavaScript functions
  /// report their arity), so functions never match here.
  void toHaveLength(int expectedLength) {
    int? actualLen;
    try {
      final len = (actual as dynamic).length;
      if (len is int) actualLen = len;
    } catch (_) {
      actualLen = null;
    }
    final pass = actualLen == expectedLength;
    _assertLazy(
      pass,
      () => 'expected length $expectedLength, got $actualLen',
      actualLen,
      expectedLength,
      'toHaveLength',
    );
  }

  /// Passes iff the string [actual] contains [pattern].
  ///
  /// [Pattern] covers both reference cases: [String] (substring) and
  /// [RegExp] (match). A non-string [actual] throws [AssertionError]
  /// immediately (`"toMatch() requires actual to be a string"`).
  void toMatch(Pattern pattern) {
    final target = actual;
    if (target is! String) {
      throw AssertionError('toMatch() requires actual to be a string', target);
    }
    final pass = target.contains(pattern);
    final patternText = pattern is RegExp
        ? '/${pattern.pattern}/${regexpFlags(pattern)}'
        : pattern.toString();
    _assertLazy(
      pass,
      () => 'expected ${jsonEncode(target)} to match $patternText',
      target,
      pattern,
      'toMatch',
    );
  }

  /// Passes iff [actual] has property [key], optionally equal to [expectedValue].
  ///
  /// Maps check [Map.containsKey] ([key] may be `null`: Dart maps allow null
  /// keys); lists accept integer indices. Anything else fails with
  /// `"expected object with property ..."`. When [expectedValue] is passed
  /// (even as an explicit `null` — omission is detected by marker, not by
  /// null), the property value must deep-equal it. Note lists require
  /// integer keys: the reference accepts string indices (`"0"`) via
  /// `hasOwnProperty`, but Dart lists are int-indexed.
  void toHaveProperty(Object? key, [Object? expectedValue = _absentValue]) {
    final hasValue = !identical(expectedValue, _absentValue);
    final target = actual;
    var hasProp = false;
    Object? propVal;
    if (target is Map) {
      hasProp = target.containsKey(key);
      if (hasProp) propVal = target[key];
    } else if (target is List && key is int) {
      hasProp = key >= 0 && key < target.length;
      if (hasProp) propVal = target[key];
    }
    if (target is! Map && target is! List) {
      _assertLazy(
        false,
        () => 'expected object with property ${jsonEncode(key)}, got $target',
        target,
        null,
        'toHaveProperty',
      );
      return;
    }
    if (!hasProp) {
      _assertLazy(
        false,
        () => 'expected object to have own property ${jsonEncode(key)}',
        target,
        null,
        'toHaveProperty',
      );
      return;
    }
    if (hasValue) {
      final propValue = propVal;
      final want = expectedValue;
      final match = valuesEqual(toValue(propValue), toValue(want));
      _assertLazy(
        match,
        () =>
            'expected property ${jsonEncode(key)} to equal ${encodeValue(toValue(want))}, got ${encodeValue(toValue(propValue))}',
        propValue,
        want,
        'toHaveProperty',
      );
      return;
    }
    _assertLazy(
      true,
      () => 'object has property ${jsonEncode(key)}',
      target,
      null,
      'toHaveProperty',
    );
  }

  /// Passes iff the zero-argument closure [actual] throws.
  ///
  /// A non-function [actual] throws [AssertionError] immediately
  /// (`"toThrow() requires a function"`). With no [expected], any throw —
  /// error, exception, or arbitrary value — passes. With [expected]:
  ///
  /// * [String]: the thrown message must include it;
  /// * [RegExp]: the thrown message must match it;
  /// * [Error]/[Exception] instance: the thrown value must share its exact
  ///   runtime type and its `toString()`;
  /// * [Type]: the thrown value must share that exact runtime type —
  ///   except `Error` and `Exception` themselves, which match any error or
  ///   exception respectively (mirroring the reference's `instanceof`, which
  ///   Dart cannot express for arbitrary types without mirrors).
  ///
  /// The thrown message is the value's `toString()` (the reference uses
  /// `.message`, which Dart errors lack uniformly). An [expected] of any
  /// other type performs no assertion — mirroring the reference's
  /// fall-through — and only zero-argument closures are supported.
  void toThrow([Object? expected]) {
    final target = actual;
    if (target is! Function) {
      throw AssertionError('toThrow() requires a function', target);
    }
    var threw = false;
    Object? caught;
    try {
      Function.apply(target, const []);
    } catch (err) {
      threw = true;
      caught = err;
    }

    if (!threw) {
      _assertLazy(
        false,
        () => 'expected function to throw an error, but it returned normally',
        null,
        null,
        'toThrow',
      );
      return;
    }

    if (expected == null) {
      _assertLazy(true, () => 'function threw an error', null, null, 'toThrow');
      return;
    }

    final errMessage = caught.toString();

    if (expected is String) {
      final pass = errMessage.contains(expected);
      _assertLazy(
        pass,
        () =>
            'expected error message to include ${jsonEncode(expected)}, got ${jsonEncode(errMessage)}',
        errMessage,
        expected,
        'toThrow',
      );
    } else if (expected is RegExp) {
      final pass = expected.hasMatch(errMessage);
      _assertLazy(
        pass,
        () =>
            'expected error message to match /${expected.pattern}/${regexpFlags(expected)}, got ${jsonEncode(errMessage)}',
        errMessage,
        expected.toString(),
        'toThrow',
      );
    } else if (expected is Error || expected is Exception) {
      final pass = caught.runtimeType == expected.runtimeType &&
          errMessage == expected.toString();
      _assertLazy(
        pass,
        () => 'expected error matching $expected, got $caught',
        caught,
        expected,
        'toThrow',
      );
    } else if (expected is Type) {
      final pass = _typeMatches(caught, expected);
      _assertLazy(
        pass,
        () => 'expected error to be instance of $expected',
        caught,
        null,
        'toThrow',
      );
    }
    // Any other expected type: no assertion, mirroring the reference.
  }

  /// Whether [caught] counts as an instance of the [Type] [type].
  ///
  /// Exact runtime-type equality, except the two roots: [Error] matches any
  /// error and [Exception] matches any exception.
  static bool _typeMatches(Object? caught, Type type) {
    if (type == Error) return caught is Error;
    if (type == Exception) return caught is Exception;
    return caught != null && caught.runtimeType == type;
  }
}

/// An async expectation: every matcher awaits the value first, then
/// delegates to [Expectation].
///
/// Created by [Expectation.resolves] (assert against the completion value)
/// and [Expectation.rejects] (assert against the rejection error).
class AsyncExpectation<T> {
  final Future<T> _future;
  final bool _isNot;
  final bool _isRejection;

  /// Creates an async expectation over [future].
  AsyncExpectation(this._future, this._isNot, this._isRejection);

  /// The negation of this expectation.
  AsyncExpectation<T> get not =>
      AsyncExpectation<T>(_future, !_isNot, _isRejection);

  /// Awaits the value, then runs the sync matcher of the same name.
  Future<void> toBe(Object? expected) async {
    final val = await _future;
    Expectation(val, _isNot).toBe(expected);
  }

  /// Awaits the value, then runs the sync matcher of the same name.
  Future<void> toEqual(Object? expected) async {
    final val = await _future;
    Expectation(val, _isNot).toEqual(expected);
  }

  /// Awaits the value, then runs the sync matcher of the same name.
  Future<void> toStrictEqual(Object? expected) async {
    final val = await _future;
    Expectation(val, _isNot).toStrictEqual(expected);
  }

  /// Awaits the value, then runs the sync matcher of the same name.
  Future<void> toBeTruthy() async {
    final val = await _future;
    Expectation(val, _isNot).toBeTruthy();
  }

  /// Awaits the value, then runs the sync matcher of the same name.
  Future<void> toBeFalsy() async {
    final val = await _future;
    Expectation(val, _isNot).toBeFalsy();
  }

  /// Awaits the value, then runs the sync matcher of the same name.
  Future<void> toBeNull() async {
    final val = await _future;
    Expectation(val, _isNot).toBeNull();
  }

  /// Awaits the value, then runs the sync matcher of the same name.
  Future<void> toBeInstanceOf<U>() async {
    final val = await _future;
    Expectation(val, _isNot).toBeInstanceOf<U>();
  }

  /// Awaits the value, then runs the sync matcher of the same name.
  Future<void> toBeNaN() async {
    final val = await _future;
    Expectation(val, _isNot).toBeNaN();
  }

  /// Awaits the value, then runs the sync matcher of the same name.
  Future<void> toBeGreaterThan(Object? n) async {
    final val = await _future;
    Expectation(val, _isNot).toBeGreaterThan(n);
  }

  /// Awaits the value, then runs the sync matcher of the same name.
  Future<void> toBeGreaterThanOrEqual(Object? n) async {
    final val = await _future;
    Expectation(val, _isNot).toBeGreaterThanOrEqual(n);
  }

  /// Awaits the value, then runs the sync matcher of the same name.
  Future<void> toBeLessThan(Object? n) async {
    final val = await _future;
    Expectation(val, _isNot).toBeLessThan(n);
  }

  /// Awaits the value, then runs the sync matcher of the same name.
  Future<void> toBeLessThanOrEqual(Object? n) async {
    final val = await _future;
    Expectation(val, _isNot).toBeLessThanOrEqual(n);
  }

  /// Awaits the value, then runs the sync matcher of the same name.
  Future<void> toBeCloseTo(num expected, [int precision = 2]) async {
    final val = await _future;
    Expectation(val, _isNot).toBeCloseTo(expected, precision);
  }

  /// Awaits the value, then runs the sync matcher of the same name.
  Future<void> toContain(Object? item) async {
    final val = await _future;
    Expectation(val, _isNot).toContain(item);
  }

  /// Awaits the value, then runs the sync matcher of the same name.
  Future<void> toHaveLength(int expectedLength) async {
    final val = await _future;
    Expectation(val, _isNot).toHaveLength(expectedLength);
  }

  /// Awaits the value, then runs the sync matcher of the same name.
  Future<void> toMatch(Pattern pattern) async {
    final val = await _future;
    Expectation(val, _isNot).toMatch(pattern);
  }

  /// Awaits the value, then runs the sync matcher of the same name.
  Future<void> toHaveProperty(
    Object? key, [
    Object? expectedValue = _absentValue,
  ]) async {
    final val = await _future;
    Expectation(val, _isNot).toHaveProperty(key, expectedValue);
  }

  /// Awaits the rejection error, then runs [Expectation.toThrow] against it.
  ///
  /// Only valid on [Expectation.rejects]: calling it on
  /// [Expectation.resolves] throws [AssertionError] immediately
  /// (`"toThrow() must be called on rejects, not resolves"`).
  Future<void> toThrow([Object? expected]) async {
    if (!_isRejection) {
      throw AssertionError('toThrow() must be called on rejects, not resolves');
    }
    final val = await _future;
    Expectation(() {
      throw val as Object;
    }, _isNot)
        .toThrow(expected);
  }
}

/// Creates an [Expectation] over [actual].
Expectation<T> expect<T>(T actual) => Expectation<T>(actual);
