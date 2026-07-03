import 'package:test/test.dart';
import 'package:mosaic_flux_flutter/mosaic_flux.dart';

class _S {
  final int a;
  final int b;
  final String label;
  const _S({this.a = 0, this.b = 0, this.label = ''});
}

void main() {
  group('createSelector1', () {
    test('recomputes on change', () {
      var calls = 0;
      final doubled = createSelector1<_S, int, int>(
        (s) => s.a,
        (a) {
          calls++;
          return a * 2;
        },
      );
      expect(doubled(_S(a: 5)), 10);
      expect(doubled(_S(a: 7)), 14);
      expect(calls, 2);
    });

    test('caches on stable input', () {
      var calls = 0;
      final doubled = createSelector1<_S, int, int>(
        (s) => s.a,
        (a) {
          calls++;
          return a * 2;
        },
      );
      final s = _S(a: 5);
      doubled(s);
      doubled(s);
      doubled(s);
      expect(calls, 1);
    });

    test('caches across state refs with same projected input', () {
      var calls = 0;
      final doubled = createSelector1<_S, int, int>(
        (s) => s.a,
        (a) {
          calls++;
          return a * 2;
        },
      );
      doubled(_S(a: 5, b: 0));
      doubled(_S(a: 5, b: 999, label: 'different'));
      expect(calls, 1);
    });
  });

  group('createSelector2', () {
    test('recomputes when either changes', () {
      var calls = 0;
      final sum = createSelector2<_S, int, int, int>(
        (s) => s.a,
        (s) => s.b,
        (a, b) {
          calls++;
          return a + b;
        },
      );
      expect(sum(_S(a: 1, b: 2)), 3);
      expect(sum(_S(a: 1, b: 5)), 6);
      expect(sum(_S(a: 4, b: 5)), 9);
      expect(calls, 3);
    });

    test('caches on stable inputs', () {
      var calls = 0;
      final sum = createSelector2<_S, int, int, int>(
        (s) => s.a,
        (s) => s.b,
        (a, b) {
          calls++;
          return a + b;
        },
      );
      final s = _S(a: 1, b: 2);
      sum(s);
      sum(s);
      expect(calls, 1);
    });
  });

  group('createSelector3', () {
    test('recomputes when any changes', () {
      var calls = 0;
      final fmt = createSelector3<_S, int, int, String, String>(
        (s) => s.a,
        (s) => s.b,
        (s) => s.label,
        (a, b, lbl) {
          calls++;
          return '$lbl:${a + b}';
        },
      );
      expect(fmt(_S(a: 1, b: 2, label: 'x')), 'x:3');
      expect(fmt(_S(a: 1, b: 2, label: 'x')), 'x:3');
      expect(fmt(_S(a: 1, b: 2, label: 'y')), 'y:3');
      expect(calls, 2);
    });
  });
}
