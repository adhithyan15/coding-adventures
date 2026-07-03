import 'package:test/test.dart';
import 'package:mosaic_flux_flutter/mosaic_flux.dart';

class _S {
  final int v;
  const _S(this.v);
}

class _Bump extends MosaicAction<_S> {
  @override
  _S apply(_S state) => _S(state.v + 1);
}

void main() {
  group('composeMiddleware', () {
    test('empty is no-op', () {
      final m = composeMiddleware<_S>([]);
      expect(() => m(_Bump(), _S(0), _S(1)), returnsNormally);
    });

    test('single middleware returned verbatim', () {
      final m1 = (MosaicAction<_S> a, _S p, _S n) {};
      expect(composeMiddleware<_S>([m1]), same(m1));
    });

    test('runs in registration order', () {
      final calls = <String>[];
      final composed = composeMiddleware<_S>([
        (_, __, ___) => calls.add('a'),
        (_, __, ___) => calls.add('b'),
        (_, __, ___) => calls.add('c'),
      ]);
      composed(_Bump(), _S(0), _S(1));
      expect(calls, ['a', 'b', 'c']);
    });

    test('isolates throws', () {
      final calls = <String>[];
      final composed = composeMiddleware<_S>([
        (_, __, ___) => calls.add('a'),
        (_, __, ___) => throw Exception('boom'),
        (_, __, ___) => calls.add('c'),
      ]);
      composed(_Bump(), _S(0), _S(1));
      expect(calls, ['a', 'c']);
    });
  });

  group('loggerMiddleware', () {
    test('does not throw', () {
      final m = loggerMiddleware<_S>();
      expect(() => m(_Bump(), _S(0), _S(1)), returnsNormally);
    });
  });
}
