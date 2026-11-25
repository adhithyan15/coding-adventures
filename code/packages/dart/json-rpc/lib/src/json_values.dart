/// Shared JSON-value machinery: deep equality and hashing with serde
/// semantics, plus the two fail-closed input guards.
///
/// Public so the sibling libraries can reach it; hidden from the package
/// barrel.
library;

/// Deep equality for decoded JSON values, mirroring `serde_json::Value`
/// equality: maps compare order-independently, lists in order, and numbers
/// compare by value AND representation — `1` (int) does not equal `1.0`
/// (double), because Dart's `==` would say otherwise while serde keeps
/// integer and float representations distinct.
bool jsonValuesEqual(Object? a, Object? b) {
  if (a is Map && b is Map) {
    if (a.length != b.length) return false;
    for (final key in a.keys) {
      if (!b.containsKey(key)) return false;
      if (!jsonValuesEqual(a[key], b[key])) return false;
    }
    return true;
  }
  if (a is List && b is List) {
    if (a.length != b.length) return false;
    for (var i = 0; i < a.length; i++) {
      if (!jsonValuesEqual(a[i], b[i])) return false;
    }
    return true;
  }
  if (a is num && b is num) {
    return a.runtimeType == b.runtimeType && a == b;
  }
  return a == b;
}

/// Hash consistent with [jsonValuesEqual]: map entries hash
/// order-independently.
int jsonValueHash(Object? value) {
  if (value is Map) {
    return Object.hashAllUnordered(
        value.entries.map((e) => Object.hash(e.key, jsonValueHash(e.value))));
  }
  if (value is List) {
    return Object.hashAll(value.map(jsonValueHash));
  }
  if (value is num) {
    return Object.hash(value.runtimeType, value);
  }
  return value.hashCode;
}

/// Rejects JSON text containing integer literals outside the signed
/// 64-bit range.
///
/// `dart:convert` decodes such literals as lossy doubles (serde keeps
/// exact u64), silently corrupting ids and values — so hostile or merely
/// large inputs fail closed here with a [FormatException] instead. The
/// scan is JSON-aware: digits inside strings never match. Fractions and
/// exponents are doubles on both sides and pass through untouched.
void checkJsonIntegers(String text) {
  var i = 0;
  final n = text.length;
  while (i < n) {
    final c = text.codeUnitAt(i);
    if (c == 0x22) {
      // Skip the string literal, honoring backslash escapes.
      i++;
      while (i < n) {
        final s = text.codeUnitAt(i);
        if (s == 0x5C) {
          i += 2;
        } else if (s == 0x22) {
          i++;
          break;
        } else {
          i++;
        }
      }
      continue;
    }
    final isDigit = c >= 0x30 && c <= 0x39;
    final isSign = (c == 0x2D || c == 0x2B) &&
        i + 1 < n &&
        text.codeUnitAt(i + 1) >= 0x30 &&
        text.codeUnitAt(i + 1) <= 0x39;
    if (!isDigit && !isSign) {
      i++;
      continue;
    }
    var j = isSign ? i + 1 : i;
    while (j < n) {
      final d = text.codeUnitAt(j);
      if (d < 0x30 || d > 0x39) break;
      j++;
    }
    // A fraction or exponent makes it a double on both sides: skip the
    // whole double literal, since digits after `.`/`e` are not integers.
    if (j < n &&
        (text.codeUnitAt(j) == 0x2E ||
            text.codeUnitAt(j) == 0x65 ||
            text.codeUnitAt(j) == 0x45)) {
      var k = j + 1;
      if (text.codeUnitAt(j) == 0x2E) {
        while (k < n) {
          final d = text.codeUnitAt(k);
          if (d < 0x30 || d > 0x39) break;
          k++;
        }
        if (k >= n ||
            (text.codeUnitAt(k) != 0x65 && text.codeUnitAt(k) != 0x45)) {
          i = k;
          continue;
        }
        k++;
      }
      // Past the `e`: optional sign, then the exponent digits.
      if (k < n && (text.codeUnitAt(k) == 0x2B || text.codeUnitAt(k) == 0x2D)) {
        k++;
      }
      while (k < n) {
        final d = text.codeUnitAt(k);
        if (d < 0x30 || d > 0x39) break;
        k++;
      }
      i = k;
      continue;
    }
    final digits = isSign ? text.substring(i + 1, j) : text.substring(i, j);
    if (!_fitsInt64(isSign && c == 0x2D, digits)) {
      throw FormatException(
          'integer literal out of range: ${text.substring(i, j)}');
    }
    i = j;
  }
}

/// Whether the digit string fits int64 (negated when [negative]).
bool _fitsInt64(bool negative, String digits) {
  var start = 0;
  while (start < digits.length && digits.codeUnitAt(start) == 0x30) {
    start++;
  }
  if (start == digits.length) return true;
  const max = '9223372036854775807';
  const min = '9223372036854775808';
  final limit = negative ? min : max;
  final length = digits.length - start;
  if (length != limit.length) return length < limit.length;
  for (var i = 0; i < length; i++) {
    final d = digits.codeUnitAt(start + i);
    final l = limit.codeUnitAt(i);
    if (d != l) return d < l;
  }
  return true;
}

/// Maximum JSON nesting depth, matching serde's default recursion limit.
///
/// `dart:convert` happily decodes 100k-deep arrays that would overflow a
/// recursive walker, so the walk below is iterative and this cap fails
/// closed first. Bounding depth at parse also bounds every downstream
/// recursive walk (`jsonValuesEqual`, `jsonValueHash`) over parsed
/// values.
const int maxJsonDepth = 128;

/// Rejects decoded values containing lone UTF-16 surrogates or nesting
/// past [maxJsonDepth].
///
/// `dart:convert` admits `\uD800`-style lone escapes that serde rejects,
/// and such strings corrupt `utf8.encode` downstream (it substitutes).
/// The walk is iterative with an explicit stack, so hostile nesting
/// cannot overflow the call stack. Fail closed with a [FormatException].
void checkNoLoneSurrogates(Object? value) {
  final stack = [(value, 0)];
  while (stack.isNotEmpty) {
    final (current, depth) = stack.removeLast();
    if (depth > maxJsonDepth) {
      throw const FormatException('JSON nesting too deep');
    }
    if (current is String) {
      _checkString(current);
    } else if (current is Map) {
      for (final entry in current.entries) {
        stack.add((entry.key, depth + 1));
        stack.add((entry.value, depth + 1));
      }
    } else if (current is List) {
      for (var i = current.length - 1; i >= 0; i--) {
        stack.add((current[i], depth + 1));
      }
    }
  }
}

/// Rejects lone surrogates in one string.
void _checkString(String text) {
  final units = text.codeUnits;
  for (var i = 0; i < units.length; i++) {
    final u = units[i];
    if (u >= 0xD800 && u <= 0xDBFF) {
      if (i + 1 >= units.length ||
          units[i + 1] < 0xDC00 ||
          units[i + 1] > 0xDFFF) {
        throw const FormatException('lone surrogate in JSON string');
      }
      i++;
    } else if (u >= 0xDC00 && u <= 0xDFFF) {
      throw const FormatException('lone surrogate in JSON string');
    }
  }
}
