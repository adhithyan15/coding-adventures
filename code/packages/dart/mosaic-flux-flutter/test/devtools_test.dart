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
  group('devToolsMiddleware', () {
    test('callable', () {
      final m = devToolsMiddleware<_S>();
      expect(() => m(_Bump(), _S(0), _S(1)), returnsNormally);
    });

    test('accepts custom storeName', () {
      final m = devToolsMiddleware<_S>(storeName: 'my-grid');
      expect(() => m(_Bump(), _S(0), _S(1)), returnsNormally);
    });

    test('integrates with store', () {
      var probeRuns = 0;
      final store = MosaicStore<_S>(
        initialState: _S(0),
        middleware: [
          devToolsMiddleware<_S>(),
          (_, __, ___) => probeRuns++,
        ],
      );
      store.dispatch(_Bump());
      store.dispatch(_Bump());
      expect(probeRuns, 2);
      expect(store.state.v, 2);
    });
  });
}
