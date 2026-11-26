/// Bit-exact structural comparison for Dart values.
///
/// This module is the Dart lane of the TypeScript `deep_equal` module from
/// the reference `test-matchers` package. Three public entry points:
///
/// * [toValue] — normalize any Dart value into a [Value] tree.
/// * [valuesEqual] — compare two [Value] trees for structural equality.
/// * [deepEqual] — the composition: normalize both sides, then compare.
///
/// plus the float-bit utilities [numberToF64Bits], [f64BitsToNumber],
/// [canonicalF64], and the diagnostic printer [encodeValue].
library;

import 'dart:convert';
import 'dart:typed_data';

import 'value.dart';

/// Maximum nesting depth [toValue] descends before throwing a [StateError].
///
/// Mirrors the TypeScript reference's depth-64 limit exactly.
const int maxDepth = 64;

/// Convert a Dart `double` to its raw IEEE-754 bit pattern.
///
/// The pattern is returned as an unsigned [BigInt] in the range
/// `0 .. 2^64 - 1`. The conversion is bit-exact: NaN payloads and the sign
/// of zero survive the round trip through [f64BitsToNumber].
BigInt numberToF64Bits(double v) {
  final bytes = ByteData(8)..setFloat64(0, v, Endian.little);
  // getUint64 returns a signed int on 64-bit platforms; mask up through
  // BigInt so the top bit reads as unsigned.
  return BigInt.from(bytes.getUint64(0, Endian.little)).toUnsigned(64);
}

/// Convert a raw IEEE-754 bit pattern back to a Dart `double`.
///
/// Inverse of [numberToF64Bits]; the round trip is bit-exact for every
/// input, including NaN payloads and signed zeros.
double f64BitsToNumber(BigInt bits) {
  // toSigned (not a bare toInt, which saturates out-of-range values):
  // setUint64 takes a signed int64, so the top bit must wrap into the
  // negative range rather than clamp to maxInt64.
  final signed = bits.toUnsigned(64).toSigned(64).toInt();
  final bytes = ByteData(8)..setUint64(0, signed, Endian.little);
  return bytes.getFloat64(0, Endian.little);
}

/// Canonicalize a raw f64 bit pattern for comparison.
///
/// Any NaN (exponent all ones, nonzero mantissa) collapses to the quiet-NaN
/// pattern `0x7ff8000000000000`, so NaN payloads and NaN signs never
/// distinguish two values; −0 collapses to +0. Every other pattern —
/// infinities, subnormals, ordinary values — passes through unchanged.
BigInt canonicalF64(BigInt bits) {
  final b = bits.toUnsigned(64);
  final exponent = (b >> 52) & BigInt.from(0x7ff);
  final mantissa = b & ((BigInt.one << 52) - BigInt.one);
  final isNaN = exponent == BigInt.from(0x7ff) && mantissa != BigInt.zero;
  if (isNaN) {
    return BigInt.parse('7ff8000000000000', radix: 16);
  }
  // -0.0 is 0x8000000000000000; +0.0 is 0x0.
  if (b == (BigInt.one << 63)) {
    return BigInt.zero;
  }
  return b;
}

/// Normalize any Dart value into a [Value] tree.
///
/// The mapping from Dart types to [Value] kinds (Dart's counterpart of the
/// TypeScript reference's `toValue` table):
///
/// | Dart input              | Kind     | Notes                              |
/// |-------------------------|----------|------------------------------------|
/// | `null`                  | null     |                                    |
/// | `bool`                  | bool     |                                    |
/// | `int`, `BigInt`         | int      | exact decimal text                 |
/// | `double`                | f64      | raw bits via [numberToF64Bits]     |
/// | `String`                | text     |                                    |
/// | `Uint8List`             | bytes    | shared by reference                |
/// | `DateTime`              | date     | whole millis since the epoch       |
/// | `RegExp`                | regexp   | source + probed flags              |
/// | `Error`, `Exception`    | error    | runtime-type name + `toString()`   |
/// | `List`, `Set`           | list     | element-wise, in iteration order   |
/// | `Map`                   | map      | entry-wise, in iteration order     |
/// | anything else           | text     | `toString()` (incl. functions)     |
///
/// Cycles (a collection that — directly or indirectly — contains itself)
/// collapse to `text("[Circular]")`, exactly like the reference: a revisit
/// to a collection already on the conversion stack produces a text leaf, so
/// converted trees are always finite. Shared references (the same collection
/// reachable twice, but not cyclically) are memoized after conversion, so
/// DAGs convert in linear time. Nesting deeper than [maxDepth] throws a
/// [StateError].
Value toValue(Object? input) => _ToValue().convert(input, 0);

/// Compare two [Value] trees for structural equality.
///
/// Kinds must match, then kind-specific payloads compare recursively. Maps
/// compare order-insensitively: every entry of `a` must find a distinct
/// structurally equal entry in `b` (bipartite matching, so duplicate entries
/// pair up correctly), and the entry counts must be equal.
bool valuesEqual(Value a, Value b) {
  if (a is NullValue || b is NullValue) {
    return a is NullValue && b is NullValue;
  }
  if (a is BoolValue || b is BoolValue) {
    return a is BoolValue && b is BoolValue && a.value == b.value;
  }
  if (a is IntValue || b is IntValue) {
    // Decimal text is canonical: Dart ints and BigInts render without
    // padding, exponents, or alternate bases.
    return a is IntValue && b is IntValue && a.text == b.text;
  }
  if (a is F64Value || b is F64Value) {
    return a is F64Value &&
        b is F64Value &&
        canonicalF64(a.bits) == canonicalF64(b.bits);
  }
  if (a is BytesValue || b is BytesValue) {
    return a is BytesValue && b is BytesValue && _bytesEqual(a.value, b.value);
  }
  if (a is TextValue || b is TextValue) {
    return a is TextValue && b is TextValue && a.value == b.value;
  }
  if (a is DateValue || b is DateValue) {
    return a is DateValue && b is DateValue && a.ms == b.ms;
  }
  if (a is RegexpValue || b is RegexpValue) {
    return a is RegexpValue &&
        b is RegexpValue &&
        a.source == b.source &&
        a.flags == b.flags;
  }
  if (a is ErrorValue || b is ErrorValue) {
    return a is ErrorValue &&
        b is ErrorValue &&
        a.name == b.name &&
        a.message == b.message;
  }
  if (a is ListValue || b is ListValue) {
    if (a is! ListValue || b is! ListValue) return false;
    if (a.items.length != b.items.length) return false;
    for (var i = 0; i < a.items.length; i++) {
      if (!valuesEqual(a.items[i], b.items[i])) return false;
    }
    return true;
  }
  if (a is MapValue || b is MapValue) {
    // Defensive and currently unreachable (Map is the last kind branch, so
    // any mixed pair already exited at the other kind's earlier branch):
    // kept to mirror the reference structure and to stay correct if a new
    // kind is ever added below.
    if (a is! MapValue || b is! MapValue) return false;
    if (a.entries.length != b.entries.length) return false;
    // Order-insensitive matching: each entry of `a` claims a distinct
    // equal entry of `b`. The `used` array keeps duplicates honest.
    final used = List<bool>.filled(b.entries.length, false);
    outer:
    for (final entryA in a.entries) {
      for (var j = 0; j < b.entries.length; j++) {
        if (used[j]) continue;
        if (valuesEqual(entryA.$1, b.entries[j].$1) &&
            valuesEqual(entryA.$2, b.entries[j].$2)) {
          used[j] = true;
          continue outer;
        }
      }
      return false;
    }
    return true;
  }
  return false;
}

/// Compare two Dart values for deep structural equality.
///
/// Normalizes both sides with [toValue], then compares with [valuesEqual].
/// Shorthand for `valuesEqual(toValue(a), toValue(b))`.
bool deepEqual(Object? a, Object? b) => valuesEqual(toValue(a), toValue(b));

/// Render a [Value] tree as a single-line diagnostic string.
///
/// Cyclic structures print as `[Circular]`; maps print with `=>` between
/// keys and values, matching the TypeScript reference's `encodeValue`
/// output format exactly.
String encodeValue(Value v) => _encodeValue(v, Set.identity());

String _encodeValue(Value v, Set<Value> seen) {
  if (v is NullValue) return 'null';
  if (v is BoolValue) return v.value ? 'true' : 'false';
  if (v is IntValue) return '${v.text}n';
  if (v is F64Value) {
    final canonical = canonicalF64(v.bits);
    if (canonical == BigInt.zero) return '0';
    final asDouble = f64BitsToNumber(canonical);
    if (asDouble.isNaN) return 'NaN';
    if (asDouble == double.infinity) return 'Infinity';
    if (asDouble == double.negativeInfinity) return '-Infinity';
    // Dart's double toString already renders shortest-round-trip digits,
    // matching the reference format for finite values.
    return asDouble.toString();
  }
  if (v is BytesValue) {
    final hex =
        v.value.map((byte) => byte.toRadixString(16).padLeft(2, '0')).join();
    return 'Uint8List([$hex])';
  }
  if (v is TextValue) return jsonEncode(v.value);
  if (v is DateValue) {
    return 'Date(${DateTime.fromMillisecondsSinceEpoch(v.ms, isUtc: true).toIso8601String()})';
  }
  if (v is RegexpValue) return '/${v.source}/${v.flags}';
  if (v is ErrorValue) return '${v.name}: ${v.message}';
  if (v is ListValue) {
    if (!seen.add(v)) return '[Circular]';
    try {
      return '[${v.items.map((item) => _encodeValue(item, seen)).join(', ')}]';
    } finally {
      seen.remove(v);
    }
  }
  if (v is MapValue) {
    if (!seen.add(v)) return '[Circular]';
    try {
      final parts = v.entries.map(
        (entry) =>
            '${_encodeValue(entry.$1, seen)} => ${_encodeValue(entry.$2, seen)}',
      );
      return '{${parts.join(', ')}}';
    } finally {
      seen.remove(v);
    }
  }
  // Unreachable: the branches above exhaust the sealed [Value] hierarchy,
  // but the explicit fallthrough keeps the analyzer honest if a new kind
  // is ever added without updating this printer.
  return '<unknown>';
}

bool _bytesEqual(Uint8List a, Uint8List b) {
  if (identical(a, b)) return true;
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

/// The conversion state for one [toValue] call: an active conversion stack
/// for cycle detection plus a memo table for shared references.
class _ToValue {
  /// Memo table: source collection (by identity) → converted [Value].
  ///
  /// Entries are installed when conversion *completes* (mirroring the
  /// reference's post-completion `memo.set`), so DAGs convert in linear time
  /// while in-progress cycles still hit [_active] first and collapse to
  /// `"[Circular]"`.
  final Expando<Value> _memo = Expando<Value>();

  /// The collections currently being converted, for cycle detection.
  ///
  /// A revisit to a member collapses to `text("[Circular]")` — the reference
  /// behavior — which keeps every converted tree finite.
  final Set<Object> _active = Set.identity();

  Value convert(Object? input, int depth) {
    if (depth > maxDepth) {
      throw StateError('Maximum nesting depth $maxDepth exceeded');
    }
    if (input == null) return const NullValue();
    if (input is bool) return BoolValue(input);
    if (input is int) return IntValue(input.toString());
    if (input is BigInt) return IntValue(input.toString());
    if (input is double) return F64Value(numberToF64Bits(input));
    if (input is String) return TextValue(input);
    if (input is Uint8List) return BytesValue(input);
    if (input is DateTime) {
      return DateValue(input.millisecondsSinceEpoch);
    }
    if (input is RegExp) {
      return RegexpValue(input.pattern, regexpFlags(input));
    }
    if (input is Error || input is Exception) {
      return ErrorValue(input.runtimeType.toString(), input.toString());
    }
    if (input is List || input is Set || input is Map) {
      if (_active.contains(input)) {
        return const TextValue('[Circular]');
      }
      final memoized = _memo[input];
      if (memoized != null) return memoized;
      _active.add(input);
      try {
        final Value result;
        if (input is List) {
          result = ListValue([
            for (final element in input) convert(element, depth + 1),
          ]);
        } else if (input is Set) {
          result = ListValue([
            for (final element in input) convert(element, depth + 1),
          ]);
        } else {
          final map = input as Map;
          result = MapValue([
            for (final entry in map.entries)
              (convert(entry.key, depth + 1), convert(entry.value, depth + 1)),
          ]);
        }
        _memo[input] = result;
        return result;
      } finally {
        _active.remove(input);
      }
    }
    // Functions and arbitrary objects: compare by rendering.
    return TextValue(input.toString());
  }
}

/// Recover a Dart [RegExp]'s flags in the canonical `gimsuy` order.
///
/// Dart exposes the `i` and `m` flags directly ([RegExp.isCaseSensitive],
/// [RegExp.isMultiLine]); the remaining flags are fixed by the platform:
///
/// | Flag | Value in Dart                                      |
/// |------|---------------------------------------------------|
/// | `g`  | always present — matching is global (`allMatches`) |
/// | `i`  | present iff `isCaseSensitive` is false             |
/// | `m`  | present iff `isMultiLine` is true                  |
/// | `s`  | always absent — `.` never matches `\n` in Dart     |
/// | `u`  | always present — strings are UTF-16                |
/// | `y`  | always absent — Dart has no sticky flag            |
String regexpFlags(RegExp re) {
  final flags = StringBuffer('g');
  if (!re.isCaseSensitive) flags.write('i');
  if (re.isMultiLine) flags.write('m');
  // Dart has no dotAll flag: `.` never matches `\n`.
  // Dart strings are UTF-16: matching is always Unicode-aware.
  flags.write('u');
  // Dart has no sticky flag.
  return flags.toString();
}
