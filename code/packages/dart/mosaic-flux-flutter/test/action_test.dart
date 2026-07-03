import 'package:test/test.dart';
import 'package:mosaic_flux_flutter/mosaic_flux.dart';

class _CounterState {
  final int count;
  const _CounterState(this.count);
}

class _Increment extends MosaicAction<_CounterState> {
  @override
  _CounterState apply(_CounterState state) => _CounterState(state.count + 1);
}

class _Add extends MosaicAction<_CounterState> {
  final int amount;
  _Add(this.amount);
  @override
  _CounterState apply(_CounterState state) =>
      _CounterState(state.count + amount);
}

void main() {
  group('MosaicAction', () {
    test('apply returns next state without mutating input', () {
      final initial = _CounterState(5);
      final next = _Increment().apply(initial);
      expect(next.count, 6);
      expect(initial.count, 5);
    });

    test('payload accessible', () {
      final action = _Add(7);
      expect(action.amount, 7);
      expect(action.apply(_CounterState(3)).count, 10);
    });

    test('deterministic', () {
      final state = _CounterState(0);
      final action = _Add(5);
      expect(action.apply(state).count, action.apply(state).count);
    });
  });
}
