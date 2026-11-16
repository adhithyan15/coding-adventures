/// Conformance of the Dart port to TRIG-REF-1, bit for bit (PHY00a 6.5).
///
/// Scope: the Dart VM only (JIT and AOT).  Nothing here has been run under
/// dart2js, so no bit-exactness is claimed on the web, and `@TestOn('vm')`
/// keeps the suite from pretending otherwise.
///
/// The 62,154 vectors arrive as generated Dart source (this package declares no
/// filesystem capability), decoded by the validated loader.  Every row is
/// compared as integers, never as printed floats; a reference NaN
/// (`7ff8000000000000`) matches any NaN.  `sin` and `cos` are also checked
/// against the correctly rounded `cr` column within 3 ulp, `radians` and
/// `degrees` within 1 ulp.  `tan` makes no accuracy claim.
@TestOn('vm')
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_trig/src/constants.dart';
import 'package:coding_adventures_trig/trig.dart' as trig;
import 'package:test/test.dart';

import 'generated/trig_reference_vectors.dart';
import 'support/vector_loader.dart';

final ByteData _scratch = ByteData(8);

/// The double with the bits `hex` (16 lowercase hex digits), via 32-bit halves.
double fromHex(String hex) {
  _scratch.setUint32(0, int.parse(hex.substring(0, 8), radix: 16));
  _scratch.setUint32(4, int.parse(hex.substring(8), radix: 16));
  return _scratch.getFloat64(0);
}

/// The 16 hex digits of the bits of [value].
String toHex(double value) {
  _scratch.setFloat64(0, value);
  return _scratch.getUint32(0).toRadixString(16).padLeft(8, '0') +
      _scratch.getUint32(4).toRadixString(16).padLeft(8, '0');
}

/// Position of [value] in the total order of doubles (-0 and +0 coincide).
int ordered(double value) {
  _scratch.setFloat64(0, value);
  final hi = _scratch.getUint32(0);
  final magnitude = (hi & 0x7fffffff) * 0x100000000 + _scratch.getUint32(4);
  return (hi >> 31) == 1 ? -magnitude : magnitude;
}

int ulpSteps(double a, double b) => (ordered(a) - ordered(b)).abs();

const Map<String, int> ulpLimit = {
  'sin': 3,
  'cos': 3,
  'radians': 1,
  'degrees': 1,
};

/// Ops that deliberately make no accuracy claim even where a `cr` exists.
const Set<String> noAccuracyClaim = {'tan'};

double apply(String op, String x) {
  if (op == 'atan2') {
    final parts = x.split(',');
    return trig.atan2(fromHex(parts[0]), fromHex(parts[1]));
  }
  final v = fromHex(x);
  switch (op) {
    case 'sin':
      return trig.sin(v);
    case 'cos':
      return trig.cos(v);
    case 'tan':
      return trig.tan(v);
    case 'radians':
      return trig.radians(v);
    case 'degrees':
      return trig.degrees(v);
    case 'sqrt':
      return trig.sqrt(v);
    case 'atan':
      return trig.atan(v);
    default:
      throw StateError('unknown op $op');
  }
}

/// Returns a failure description for [v], or null when it conforms.
String? check(Vector v) {
  if (v.y == 'error') {
    try {
      final got = apply(v.op, v.x);
      return '${v.op}(${v.x}) returned ${toHex(got)}, wanted an error';
    } on ArgumentError {
      return null;
    }
  }
  final got = apply(v.op, v.x);
  if (v.y == '7ff8000000000000') {
    return got.isNaN ? null : '${v.op}(${v.x}) = ${toHex(got)}, wanted NaN';
  }
  final bits = toHex(got);
  return bits == v.y ? null : '${v.op}(${v.x}) = $bits, wanted ${v.y}';
}

void main() {
  final groups = <String, String>{
    'edge': trigReferenceEdgeBase64,
    'sweep-binade': trigReferenceSweepBinadeBase64,
    'sweep-cw': trigReferenceSweepCwBase64,
    'sweep-prng': trigReferenceSweepPrngBase64,
  };
  final loaded = <String, List<Vector>>{
    for (final e in groups.entries)
      e.key: decodeGroup(
        e.key,
        e.value,
        rows: trigReferenceRows[e.key]!,
        opRows: trigReferenceOpRows[e.key]!,
      ),
  };

  group('TRIG-REF-1 vectors, bit for bit', () {
    for (final entry in loaded.entries) {
      for (final op in trigReferenceOpRows[entry.key]!.keys) {
        test('${entry.key} $op', () {
          final failures = <String>[];
          for (final v in entry.value.where((v) => v.op == op)) {
            final why = check(v);
            if (why != null) failures.add(why);
          }
          expect(failures, isEmpty, reason: '${failures.length} rows failed');
        });
      }
    }

    test('every one of the 62,154 vectors was loaded', () {
      expect(
        loaded.values.fold<int>(0, (a, b) => a + b.length),
        62154,
      );
    });
  });

  group('accuracy against the correctly rounded value', () {
    test('sin and cos within 3 ulp, radians and degrees within 1 ulp', () {
      final failures = <String>[];
      var checked = 0;
      for (final rows in loaded.values) {
        for (final v in rows) {
          if (v.cr == '-' || v.y == 'error') continue;
          if (noAccuracyClaim.contains(v.op)) continue;
          final limit = ulpLimit[v.op];
          // A cr value with no stated limit is a hole in this test, not a pass.
          if (limit == null) {
            failures.add('${v.op} has a cr value but no accuracy limit');
            continue;
          }
          checked++;
          final steps = ulpSteps(apply(v.op, v.x), fromHex(v.cr));
          if (steps > limit) {
            failures.add('${v.op}(${v.x}): $steps steps from cr, limit $limit');
          }
        }
      }
      expect(failures, isEmpty);
      expect(checked, greaterThan(40000));
    });

    test('tan makes no accuracy claim: only its bits are pinned', () {
      // The pole sentinel is a convention (PHY00a 3.8), not an approximation,
      // so rows may carry a cr that tan is not required to approach.
      expect(noAccuracyClaim, {'tan'});
      expect(ulpLimit.containsKey('tan'), isFalse);
    });
  });

  group('the loader refuses bad input', () {
    String enc(String text) => base64Encode(utf8.encode(text));

    test('an unknown op', () {
      expect(
        () => decodeGroup(
          'x',
          enc('exp\t0000000000000000\t0000000000000000\t-\t\n'),
          rows: 1,
          opRows: {'exp': 1},
        ),
        throwsFormatException,
      );
    });

    test('an unknown flag, a short row, upper-case hex, a wrong count', () {
      const ok = 'sin\t0000000000000000\t0000000000000000\t-\t';
      expect(
        () =>
            decodeGroup('x', enc('$ok' 'bogus\n'), rows: 1, opRows: {'sin': 1}),
        throwsFormatException,
      );
      expect(
        () => decodeGroup('x', enc('sin\t0000000000000000\t0000000000000000\n'),
            rows: 1, opRows: {'sin': 1}),
        throwsFormatException,
      );
      expect(
        () => decodeGroup(
            'x', enc('sin\tABCDEF0000000000\t0000000000000000\t-\t\n'),
            rows: 1, opRows: {'sin': 1}),
        throwsFormatException,
      );
      expect(
        () => decodeGroup('x', enc('$ok\n'), rows: 2, opRows: {'sin': 2}),
        throwsFormatException,
      );
      expect(
        () => decodeGroup('x', enc('$ok\n'), rows: 1, opRows: {'sin': 2}),
        throwsFormatException,
      );
    });

    test('an oversized group', () {
      final big = List.filled(maxGroupBytes + 1, 0x61);
      expect(
        () => decodeGroup('x', base64Encode(big), rows: 1, opRows: {}),
        throwsFormatException,
      );
    });
  });

  group('constants, bit for bit (PHY00a 6.5 e)', () {
    test('every constant equals the manifest bits', () {
      expect(
          referenceConstants.keys.toSet(), trigReferenceConstants.keys.toSet());
      for (final e in trigReferenceConstants.entries) {
        expect(toHex(referenceConstants[e.key]!), e.value, reason: e.key);
      }
    });
  });

  group('atan keeps the left-to-right term grouping (PHY00a 3.9)', () {
    // The legacy quotient-first grouping `term *= -t^2 * (2n-1) / (2n+1)`
    // returned ...0.08187573246385522 for the first of these; the specified
    // grouping returns ...24.  No shared vector exercises it, so pin it here.
    test('inputs where the two groupings differ in the last bit', () {
      final expected = <double, double>{
        0.08205917940820592: 0.08187573246385524,
        0.08508914910850891: 0.08488468348354855,
        0.15570844291557084: 0.15446804772596162,
      };
      for (final e in expected.entries) {
        expect(toHex(trig.atan(e.key)), toHex(e.value), reason: '${e.key}');
      }
    });
  });

  group('behaviour that changed under TRIG-REF-1', () {
    test('atan2 of a NaN is NaN, not +-pi/2', () {
      expect(trig.atan2(1.0, double.nan).isNaN, isTrue);
      expect(trig.atan2(-1.0, double.nan).isNaN, isTrue);
      expect(trig.atan2(double.nan, 1.0).isNaN, isTrue);
      expect(trig.atan2(double.nan, double.nan).isNaN, isTrue);
    });

    test('sin, cos and tan are NaN beyond 2^29, NaN and infinity', () {
      for (final f in [trig.sin, trig.cos, trig.tan]) {
        expect(f(536870912.0 * 2.0).isNaN, isTrue);
        expect(f(double.nan).isNaN, isTrue);
        expect(f(double.infinity).isNaN, isTrue);
        expect(f(double.negativeInfinity).isNaN, isTrue);
        expect(f(1e22).isNaN, isTrue);
      }
      expect(trig.sin(536870912.0).isNaN, isFalse);
    });

    test('signed zero: sin(-0) is -0', () {
      expect(toHex(trig.sin(-0.0)), '8000000000000000');
      expect(toHex(trig.tan(-0.0)), '8000000000000000');
    });
  });
}
