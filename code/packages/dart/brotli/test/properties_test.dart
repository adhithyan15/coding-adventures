/// Property tests for `coding_adventures_brotli`, using the
/// in-lane `pbt` package: round-trips over structured inputs,
/// determinism, context classification, idempotent empty handling,
/// wire header shape, and compression monotonicity on runs — all
/// with fixed seeds. Inputs stay small enough to avoid the
/// fail-closed 256-symbol buckets (pinned separately in unit
/// tests).
library;

import 'dart:typed_data';

import 'package:coding_adventures_brotli/brotli.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

bool _equalBytes(List<int> a, List<int> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

void _check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

void main() {
  group('properties', () {
    test('round trip over small alphabets', () {
      final gen = Gen.tuple2(Gen.bytes(0, 120), Gen.u8());
      _check(
        'round trip',
        checkProperty<(Uint8List, int)>(
          BigInt.from(721),
          PbtOptions(numTests: 100, seed: BigInt.from(721)),
          (rng) => gen.sample(rng),
          // Small alphabet (letter bytes) keeps every context
          // bucket far from the 256-symbol guard.
          (input) => _equalBytes(
            decompress(compress(input.$1.map((b) => 0x61 + (b % 26)).toList())),
            input.$1.map((b) => 0x61 + (b % 26)).toList(),
          ),
        ),
      );
    });

    test('determinism', () {
      final gen = Gen.bytes(0, 120);
      _check(
        'determinism',
        checkProperty<Uint8List>(
          BigInt.from(722),
          PbtOptions(numTests: 60, seed: BigInt.from(722)),
          (rng) => Uint8List.fromList(
              gen.sample(rng).map((b) => 0x41 + (b % 26)).toList()),
          (input) => _equalBytes(compress(input), compress(input)),
        ),
      );
    });

    test('context classification is total', () {
      final gen = Gen.u8();
      _check(
        'contexts',
        checkProperty<int>(
          BigInt.from(723),
          PbtOptions(numTests: 256, seed: BigInt.from(723)),
          (rng) => gen.sample(rng),
          (b) {
            final ctx = literalContext(b);
            if (ctx < 0 || ctx > 3) return false;
            if (b >= 0x61 && b <= 0x7a) return ctx == 3;
            if (b >= 0x41 && b <= 0x5a) return ctx == 2;
            if (b >= 0x30 && b <= 0x39) return ctx == 1;
            return ctx == 0;
          },
        ),
      );
    });

    test('header carries the original length', () {
      final gen = Gen.bytes(1, 120);
      _check(
        'header length',
        checkProperty<Uint8List>(
          BigInt.from(724),
          PbtOptions(numTests: 60, seed: BigInt.from(724)),
          (rng) =>
              Uint8List.fromList(gen.sample(rng).map((b) => b % 128).toList()),
          (input) {
            final wire = compress(input);
            if (wire.length < 10) return false;
            final len =
                (wire[0] << 24) | (wire[1] << 16) | (wire[2] << 8) | wire[3];
            return len == input.length;
          },
        ),
      );
    });

    test('longer runs compress no worse per byte', () {
      final gen = Gen.u8();
      _check(
        'run monotonicity',
        checkProperty<int>(
          BigInt.from(725),
          PbtOptions(numTests: 40, seed: BigInt.from(725)),
          (rng) => gen.sample(rng),
          (n) {
            final short =
                Uint8List.fromList(List<int>.filled(64, 0x41 + (n % 26)));
            final long =
                Uint8List.fromList(List<int>.filled(128, 0x41 + (n % 26)));
            final shortRatio = compress(short).length / short.length;
            final longRatio = compress(long).length / long.length;
            // Longer runs amortize the fixed table overhead.
            return longRatio <= shortRatio + 0.05;
          },
        ),
      );
    });

    test('empty input has the exact 13-byte wire form', () {
      final gen = Gen.u8();
      _check(
        'empty wire',
        checkProperty<int>(
          BigInt.from(726),
          PbtOptions(numTests: 10, seed: BigInt.from(726)),
          (rng) => gen.sample(rng),
          (_) => _equalBytes(
              compress([]), [0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 63, 1, 0]),
        ),
      );
    });
  });
}
