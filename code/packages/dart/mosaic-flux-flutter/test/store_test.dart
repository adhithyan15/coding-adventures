import 'package:test/test.dart';
import 'package:mosaic_flux_flutter/mosaic_flux.dart';

class _S {
  final int count;
  final String label;
  const _S({this.count = 0, this.label = ''});
  _S copyWith({int? count, String? label}) =>
      _S(count: count ?? this.count, label: label ?? this.label);
}

class _Increment extends MosaicAction<_S> {
  @override
  _S apply(_S state) => state.copyWith(count: state.count + 1);
}

class _SetLabel extends MosaicAction<_S> {
  final String label;
  _SetLabel(this.label);
  @override
  _S apply(_S state) => state.copyWith(label: label);
}

class _NoOp extends MosaicAction<_S> {
  @override
  _S apply(_S state) => state; // returns SAME ref
}

void main() {
  group('MosaicStore', () {
    test('starts at initial state', () {
      final store = MosaicStore<_S>(initialState: _S());
      expect(store.state.count, 0);
      expect(store.state.label, '');
    });

    test('dispatch applies action', () {
      final store = MosaicStore<_S>(initialState: _S());
      store.dispatch(_Increment());
      expect(store.state.count, 1);
    });

    test('payloaded action works', () {
      final store = MosaicStore<_S>(initialState: _S());
      store.dispatch(_SetLabel('hi'));
      expect(store.state.label, 'hi');
    });

    test('select returns projection', () {
      final store = MosaicStore<_S>(initialState: _S(count: 5));
      expect(store.select((s) => s.count), 5);
    });
  });

  group('MosaicStore — fine-grained subscription', () {
    test('fires on changed slice', () {
      final store = MosaicStore<_S>(initialState: _S());
      final received = <int>[];
      store.subscribe<int>(
        (s) => s.count,
        (v) => received.add(v),
        equality: (a, b) => a == b,
      );
      store.dispatch(_Increment());
      expect(received, [1]);
    });

    test('does NOT fire on unrelated change', () {
      final store = MosaicStore<_S>(initialState: _S());
      final received = <int>[];
      store.subscribe<int>(
        (s) => s.count,
        (v) => received.add(v),
        equality: (a, b) => a == b,
      );
      store.dispatch(_SetLabel('x'));
      expect(received, isEmpty);
    });

    test('unsubscribe stops notifications', () {
      final store = MosaicStore<_S>(initialState: _S());
      final received = <int>[];
      final unsub = store.subscribe<int>(
        (s) => s.count,
        (v) => received.add(v),
        equality: (a, b) => a == b,
      );
      store.dispatch(_Increment());
      unsub();
      store.dispatch(_Increment());
      expect(received, [1]);
    });

    test('no-op dispatch skips subscriber notification', () {
      final store = MosaicStore<_S>(initialState: _S());
      var calls = 0;
      store.subscribe<int>(
        (s) => s.count,
        (_) => calls++,
        equality: (a, b) => a == b,
      );
      store.dispatch(_NoOp());
      expect(calls, 0);
    });

    test('custom equality respected', () {
      final store = MosaicStore<_S>(initialState: _S());
      final received = <int>[];
      // Always-equal: callback never fires
      store.subscribe<int>(
        (s) => s.count,
        (v) => received.add(v),
        equality: (_, __) => true,
      );
      store.dispatch(_Increment());
      expect(received, isEmpty);
    });
  });

  group('MosaicStore — middleware', () {
    test('sees triple', () {
      final seen = <List<dynamic>>[];
      final store = MosaicStore<_S>(
        initialState: _S(),
        middleware: [
          (action, prev, next) =>
              seen.add([action.runtimeType, prev.count, next.count]),
        ],
      );
      store.dispatch(_Increment());
      expect(seen.length, 1);
      expect(seen[0][1], 0);
      expect(seen[0][2], 1);
    });

    test('runs on no-op dispatches', () {
      var count = 0;
      final store = MosaicStore<_S>(
        initialState: _S(),
        middleware: [(_, __, ___) => count++],
      );
      store.dispatch(_NoOp());
      expect(count, 1);
    });
  });

  group('MosaicStore — addListener', () {
    test('fires on every state change', () {
      final store = MosaicStore<_S>(initialState: _S());
      var calls = 0;
      store.addListener(() => calls++);
      store.dispatch(_Increment());
      store.dispatch(_SetLabel('x'));
      expect(calls, 2);
    });

    test('does not fire on no-op', () {
      final store = MosaicStore<_S>(initialState: _S());
      var calls = 0;
      store.addListener(() => calls++);
      store.dispatch(_NoOp());
      expect(calls, 0);
    });

    test('unsubscribe stops further listening', () {
      final store = MosaicStore<_S>(initialState: _S());
      var calls = 0;
      final unsub = store.addListener(() => calls++);
      store.dispatch(_Increment());
      unsub();
      store.dispatch(_Increment());
      expect(calls, 1);
    });
  });
}
