// rng_test.dart — Tests for Lcg, Xorshift64, and Pcg32
// =====================================================
//
// Three categories:
//
//   1. Known-vector tests — first three outputs for seed=1 must exactly match
//      the reference Go implementation. These catch arithmetic bugs instantly.
//
//   2. Property tests — output range, float bounds, u64 construction, etc.
//
//   3. Behavioural tests — seed=0 fixup for Xorshift64, determinism,
//      independence across different seeds.

import 'package:test/test.dart';
import 'package:coding_adventures_rng/coding_adventures_rng.dart';

void main() {
  // ─── LCG ──────────────────────────────────────────────────────────────────

  group('Lcg', () {
    // Reference values from the Go implementation for seed=1:
    //   [1817669548, 2187888307, 2784682393]
    test('nextU32 matches known vectors for seed=1', () {
      final g = Lcg(1);
      expect(g.nextU32(), equals(1817669548));
      expect(g.nextU32(), equals(2187888307));
      expect(g.nextU32(), equals(2784682393));
    });

    test('nextU32 is deterministic for the same seed', () {
      final a = Lcg(0);
      final b = Lcg(0);
      for (var i = 0; i < 20; i++) {
        expect(a.nextU32(), equals(b.nextU32()));
      }
    });

    test('different seeds produce different first outputs', () {
      final a = Lcg(1);
      final b = Lcg(2);
      expect(a.nextU32(), isNot(equals(b.nextU32())));
    });

    test('nextU32 output stays in [0, 2^32)', () {
      final g = Lcg(42);
      for (var i = 0; i < 1000; i++) {
        final v = g.nextU32();
        expect(v, greaterThanOrEqualTo(0));
        expect(v, lessThan(0x100000000));
      }
    });

    test('nextU64 equals (hi << 32) | lo from two nextU32 calls', () {
      final g1 = Lcg(1);
      final u64 = g1.nextU64();

      final g2 = Lcg(1);
      final hi = g2.nextU32();
      final lo = g2.nextU32();
      expect(u64, equals((hi << 32) | lo));
    });

    test('nextFloat returns values in [0.0, 1.0)', () {
      final g = Lcg(7);
      for (var i = 0; i < 1000; i++) {
        final f = g.nextFloat();
        expect(f, greaterThanOrEqualTo(0.0));
        expect(f, lessThan(1.0));
      }
    });

    test('nextFloat spans both halves of [0,1)', () {
      final g = Lcg(1);
      var sawLow = false;
      var sawHigh = false;
      for (var i = 0; i < 200; i++) {
        final f = g.nextFloat();
        if (f < 0.5) sawLow = true;
        if (f >= 0.5) sawHigh = true;
      }
      expect(sawLow && sawHigh, isTrue);
    });

    test('nextIntInRange(1, 6) stays within bounds', () {
      final g = Lcg(9);
      for (var i = 0; i < 1000; i++) {
        final v = g.nextIntInRange(1, 6);
        expect(v, greaterThanOrEqualTo(1));
        expect(v, lessThanOrEqualTo(6));
      }
    });

    test('nextIntInRange(1, 6) covers all six values', () {
      final g = Lcg(0);
      final seen = <int>{};
      for (var i = 0; i < 10000; i++) {
        seen.add(g.nextIntInRange(1, 6));
      }
      for (var v = 1; v <= 6; v++) {
        expect(seen.contains(v), isTrue, reason: 'value $v never seen');
      }
    });

    test('nextIntInRange(42, 42) always returns 42', () {
      final g = Lcg(5);
      for (var i = 0; i < 20; i++) {
        expect(g.nextIntInRange(42, 42), equals(42));
      }
    });

    test('nextIntInRange works with negative range', () {
      final g = Lcg(3);
      for (var i = 0; i < 500; i++) {
        final v = g.nextIntInRange(-10, -1);
        expect(v, greaterThanOrEqualTo(-10));
        expect(v, lessThanOrEqualTo(-1));
      }
    });
  });

  // ─── Xorshift64 ───────────────────────────────────────────────────────────

  group('Xorshift64', () {
    // Reference values from the Go implementation for seed=1:
    //   [1082269761, 201397313, 1854285353]
    test('nextU32 matches known vectors for seed=1', () {
      final g = Xorshift64(1);
      expect(g.nextU32(), equals(1082269761));
      expect(g.nextU32(), equals(201397313));
      expect(g.nextU32(), equals(1854285353));
    });

    test('seed=0 is replaced with 1, matching seed=1 sequence', () {
      final g0 = Xorshift64(0);
      final g1 = Xorshift64(1);
      expect(g0.nextU32(), equals(g1.nextU32()));
      expect(g0.nextU32(), equals(g1.nextU32()));
    });

    test('nextU32 is deterministic for the same seed', () {
      final a = Xorshift64(99);
      final b = Xorshift64(99);
      for (var i = 0; i < 20; i++) {
        expect(a.nextU32(), equals(b.nextU32()));
      }
    });

    test('nextU32 output stays in [0, 2^32)', () {
      final g = Xorshift64(42);
      for (var i = 0; i < 1000; i++) {
        final v = g.nextU32();
        expect(v, greaterThanOrEqualTo(0));
        expect(v, lessThan(0x100000000));
      }
    });

    test('nextU64 equals (hi << 32) | lo', () {
      final g1 = Xorshift64(17);
      final u64 = g1.nextU64();

      final g2 = Xorshift64(17);
      final hi = g2.nextU32();
      final lo = g2.nextU32();
      expect(u64, equals((hi << 32) | lo));
    });

    test('nextFloat returns values in [0.0, 1.0)', () {
      final g = Xorshift64(13);
      for (var i = 0; i < 1000; i++) {
        final f = g.nextFloat();
        expect(f, greaterThanOrEqualTo(0.0));
        expect(f, lessThan(1.0));
      }
    });

    test('nextIntInRange(0, 99) stays within bounds', () {
      final g = Xorshift64(3);
      for (var i = 0; i < 1000; i++) {
        final v = g.nextIntInRange(0, 99);
        expect(v, greaterThanOrEqualTo(0));
        expect(v, lessThanOrEqualTo(99));
      }
    });

    test('nextIntInRange(1, 6) covers all six values', () {
      final g = Xorshift64(0);
      final seen = <int>{};
      for (var i = 0; i < 10000; i++) {
        seen.add(g.nextIntInRange(1, 6));
      }
      for (var v = 1; v <= 6; v++) {
        expect(seen.contains(v), isTrue, reason: 'value $v never seen');
      }
    });

    test('nextIntInRange(-5, -5) always returns -5', () {
      final g = Xorshift64(7);
      for (var i = 0; i < 20; i++) {
        expect(g.nextIntInRange(-5, -5), equals(-5));
      }
    });
  });

  // ─── Pcg32 ────────────────────────────────────────────────────────────────

  group('Pcg32', () {
    // Reference values from the Go implementation for seed=1:
    //   [1412771199, 1791099446, 124312908]
    test('nextU32 matches known vectors for seed=1', () {
      final g = Pcg32(1);
      expect(g.nextU32(), equals(1412771199));
      expect(g.nextU32(), equals(1791099446));
      expect(g.nextU32(), equals(124312908));
    });

    test('nextU32 is deterministic for the same seed', () {
      final a = Pcg32(0);
      final b = Pcg32(0);
      for (var i = 0; i < 20; i++) {
        expect(a.nextU32(), equals(b.nextU32()));
      }
    });

    test('different seeds produce different first outputs', () {
      final a = Pcg32(1);
      final b = Pcg32(2);
      expect(a.nextU32(), isNot(equals(b.nextU32())));
    });

    test('nextU32 output stays in [0, 2^32)', () {
      final g = Pcg32(42);
      for (var i = 0; i < 1000; i++) {
        final v = g.nextU32();
        expect(v, greaterThanOrEqualTo(0));
        expect(v, lessThan(0x100000000));
      }
    });

    test('nextU64 equals (hi << 32) | lo', () {
      final g1 = Pcg32(55);
      final u64 = g1.nextU64();

      final g2 = Pcg32(55);
      final hi = g2.nextU32();
      final lo = g2.nextU32();
      expect(u64, equals((hi << 32) | lo));
    });

    test('nextFloat returns values in [0.0, 1.0)', () {
      final g = Pcg32(42);
      for (var i = 0; i < 1000; i++) {
        final f = g.nextFloat();
        expect(f, greaterThanOrEqualTo(0.0));
        expect(f, lessThan(1.0));
      }
    });

    test('nextFloat spans both halves of [0,1)', () {
      final g = Pcg32(1);
      var sawLow = false;
      var sawHigh = false;
      for (var i = 0; i < 100; i++) {
        final f = g.nextFloat();
        if (f < 0.5) sawLow = true;
        if (f >= 0.5) sawHigh = true;
      }
      expect(sawLow && sawHigh, isTrue);
    });

    test('nextIntInRange(1, 6) stays within bounds', () {
      final g = Pcg32(9);
      for (var i = 0; i < 1000; i++) {
        final v = g.nextIntInRange(1, 6);
        expect(v, greaterThanOrEqualTo(1));
        expect(v, lessThanOrEqualTo(6));
      }
    });

    test('nextIntInRange(1, 6) covers all six values', () {
      final g = Pcg32(0);
      final seen = <int>{};
      for (var i = 0; i < 10000; i++) {
        seen.add(g.nextIntInRange(1, 6));
      }
      for (var v = 1; v <= 6; v++) {
        expect(seen.contains(v), isTrue, reason: 'value $v never seen');
      }
    });

    test('nextIntInRange(100, 100) always returns 100', () {
      final g = Pcg32(111);
      for (var i = 0; i < 20; i++) {
        expect(g.nextIntInRange(100, 100), equals(100));
      }
    });

    test('nextIntInRange works with negative range', () {
      final g = Pcg32(22);
      for (var i = 0; i < 500; i++) {
        final v = g.nextIntInRange(-20, -10);
        expect(v, greaterThanOrEqualTo(-20));
        expect(v, lessThanOrEqualTo(-10));
      }
    });

    // Cross-generator sanity: same seed but different algorithms → different output.
    test('all three generators produce different first outputs for seed=1', () {
      final lcg = Lcg(1);
      final xor = Xorshift64(1);
      final pcg = Pcg32(1);
      final a = lcg.nextU32();
      final b = xor.nextU32();
      final c = pcg.nextU32();
      expect(a, isNot(equals(b)));
      expect(b, isNot(equals(c)));
      expect(a, isNot(equals(c)));
    });
  });
}
