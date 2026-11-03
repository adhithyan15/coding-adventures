// The shared rng corpus (spec rng.md, "Conformance corpus"), read from its
// flat form code/specs/fixtures/rng-v1/cases.txt.
//
// The corpus pins exact outputs every language's rng package must reproduce;
// its results come from the independent reference
// code/scripts/rng_conformance.py. Each line is whitespace-separated:
//
//     case <id> <generator> <seed>
//     next_u32 <result>
//     next_u64 <result>
//     next_float <16 hex digits of the IEEE-754 bits>
//     next_int_in_range <min> <max> <result | error>
//     end
//
// `dart test` runs from the package directory. Dart's int is signed 64-bit,
// so unsigned values are compared through BigInt.

import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_rng/coding_adventures_rng.dart';
import 'package:test/test.dart';

/// The three generators behind one set of functions.
class _AnyRng {
  _AnyRng(String name, int seed)
      : _lcg = name == 'lcg' ? Lcg(seed) : null,
        _xs = name == 'xorshift64' ? Xorshift64(seed) : null,
        _pcg = name == 'pcg32' ? Pcg32(seed) : null {
    if (_lcg == null && _xs == null && _pcg == null) {
      throw ArgumentError('unknown generator $name');
    }
  }

  final Lcg? _lcg;
  final Xorshift64? _xs;
  final Pcg32? _pcg;

  int u32() => _lcg?.nextU32() ?? _xs?.nextU32() ?? _pcg!.nextU32();
  int u64() => _lcg?.nextU64() ?? _xs?.nextU64() ?? _pcg!.nextU64();
  double f64() => _lcg?.nextFloat() ?? _xs?.nextFloat() ?? _pcg!.nextFloat();
  int inRange(int min, int max) =>
      _lcg?.nextIntInRange(min, max) ??
      _xs?.nextIntInRange(min, max) ??
      _pcg!.nextIntInRange(min, max);
}

/// An unsigned 64-bit value held in a signed int, as a BigInt.
BigInt _unsigned(int v) => BigInt.from(v).toUnsigned(64);

String _floatBits(double x) {
  final bytes = ByteData(8)..setFloat64(0, x);
  return _unsigned(bytes.getInt64(0)).toRadixString(16).padLeft(16, '0');
}

void main() {
  test('every rng-v1 corpus case', () {
    var cases = 0;
    var calls = 0;
    var id = '';
    _AnyRng? g;
    for (final line in File('../../../specs/fixtures/rng-v1/cases.txt').readAsLinesSync()) {
      if (line.isEmpty || line.startsWith('#')) continue;
      final t = line.split(' ');
      switch (t[0]) {
        case 'case':
          id = t[1];
          g = _AnyRng(t[2], BigInt.parse(t[3]).toSigned(64).toInt());
          cases++;
        case 'end':
          g = null;
        case 'next_u32':
          calls++;
          expect(g!.u32().toString(), t[1], reason: id);
        case 'next_u64':
          calls++;
          expect(_unsigned(g!.u64()).toString(), t[1], reason: id);
        case 'next_float':
          calls++;
          expect(_floatBits(g!.f64()), t[1], reason: id);
        case 'next_int_in_range':
          calls++;
          final min = int.parse(t[1]);
          final max = int.parse(t[2]);
          final gen = g!;
          if (t[3] == 'error') {
            // Thrown before any draw, so the sequence carries on.
            expect(() => gen.inRange(min, max), throwsArgumentError, reason: id);
          } else {
            expect(gen.inRange(min, max).toString(), t[3], reason: id);
          }
        default:
          fail('malformed corpus line: $line');
      }
    }
    // Guard against a truncated or empty file passing vacuously.
    expect(cases, greaterThanOrEqualTo(27));
    expect(calls, greaterThanOrEqualTo(150));
  });

  group('regressions for the v1 range bug', () {
    const int64Min = -0x8000000000000000;
    const int64Max = 0x7FFFFFFFFFFFFFFF;

    test('the whole int64 domain', () {
      final g = Lcg(3);
      final values = List.generate(64, (_) => g.nextIntInRange(int64Min, int64Max));
      expect(values.any((v) => v < 0) && values.any((v) => v > 0), isTrue);
    });

    test('wide ranges reach beyond 2^32', () {
      final g = Pcg32(3);
      final values = List.generate(64, (_) => g.nextIntInRange(0, 1 << 40));
      expect(values.any((v) => v >= 1 << 32), isTrue);
    });

    test('span 2^63 + 1 terminates', () {
      final g = Xorshift64(3);
      for (var i = 0; i < 100; i++) {
        expect(g.nextIntInRange(int64Min, 0), lessThanOrEqualTo(0));
      }
    });
  });
}
