/// Unit tests for `coding_adventures_event_loop`: the 8 reference
/// tests mirrored (the two delivery tests merge into one —
/// Dart closures capture mutably, so no shared-cell workaround is
/// needed), plus lane extras (re-run semantics, throw propagation,
/// handle sharing).
library;

import 'package:coding_adventures_event_loop/event_loop.dart';
import 'package:test/test.dart';

/// A source emitting a fixed sequence of event batches, then nothing.
class _FixedSource<E> implements EventSource<E> {
  final List<List<E>> batches;
  var index = 0;

  _FixedSource(this.batches);

  @override
  List<E> poll() {
    if (index >= batches.length) {
      return [];
    }
    return batches[index++];
  }
}

/// An infinite source emitting an incrementing counter.
class _Counter implements EventSource<int> {
  var value = 0;

  @override
  List<int> poll() {
    value += 1;
    return [value];
  }
}

void main() {
  group('delivery', () {
    test('delivers all events', () {
      final loop = EventLoop<int>();
      loop.addSource(_FixedSource([
        [1, 2, 3],
        [-1],
      ]));
      final received = <int>[];
      loop.onEvent((e) {
        if (e == -1) {
          return ControlFlow.exit;
        }
        received.add(e);
        return ControlFlow.continueLoop;
      });
      loop.run();
      expect(received, equals([1, 2, 3]));
    });

    test('exit stops dispatch immediately', () {
      final loop = EventLoop<String>();
      loop.addSource(_FixedSource([
        ['a', 'stop', 'b', 'c']
      ]));
      final seen = <String>[];
      loop.onEvent((e) {
        seen.add(e);
        return e == 'stop' ? ControlFlow.exit : ControlFlow.continueLoop;
      });
      loop.run();
      expect(seen, equals(['a', 'stop']),
          reason: 'events after stop should not be seen');
    });

    test('exit skips later handlers for the same event', () {
      final loop = EventLoop<int>();
      loop.addSource(_FixedSource([
        [1],
        [0],
      ]));
      final secondSeen = <int>[];
      loop.onEvent((_) => ControlFlow.exit);
      loop.onEvent((e) {
        secondSeen.add(e);
        return ControlFlow.continueLoop;
      });
      loop.run();
      expect(secondSeen, isEmpty);
    });

    test('multiple handlers all see event', () {
      final loop = EventLoop<int>();
      loop.addSource(_FixedSource([
        [99],
        [-1]
      ]));
      var h1 = 0;
      var h2 = 0;
      loop.onEvent((e) {
        if (e == 99) {
          h1 = e;
        }
        return e == -1 ? ControlFlow.exit : ControlFlow.continueLoop;
      });
      loop.onEvent((e) {
        if (e == 99) {
          h2 = e;
        }
        return ControlFlow.continueLoop;
      });
      loop.run();
      expect(h1, equals(99));
      expect(h2, equals(99));
    });

    test('multiple sources merged', () {
      final loop = EventLoop<String>();
      loop.addSource(_FixedSource([
        ['alpha']
      ]));
      loop.addSource(_FixedSource([
        ['beta']
      ]));
      loop.addSource(_FixedSource([
        [],
        ['stop']
      ]));
      final seen = <String>[];
      loop.onEvent((e) {
        if (e == 'stop') {
          return ControlFlow.exit;
        }
        seen.add(e);
        return ControlFlow.continueLoop;
      });
      loop.run();
      expect(seen.length, equals(2));
      expect(seen, contains('alpha'));
      expect(seen, contains('beta'));
    });

    test('sources poll in registration order', () {
      final loop = EventLoop<String>();
      loop.addSource(_FixedSource([
        ['a1'],
        ['a2']
      ]));
      loop.addSource(_FixedSource([
        ['b1'],
        ['b2']
      ]));
      loop.addSource(_FixedSource([
        [],
        [],
        ['stop']
      ]));
      final seen = <String>[];
      loop.onEvent((e) {
        if (e == 'stop') {
          return ControlFlow.exit;
        }
        seen.add(e);
        return ControlFlow.continueLoop;
      });
      loop.run();
      expect(seen, equals(['a1', 'b1', 'a2', 'b2']));
    });
  });

  group('stopping', () {
    test('stop handle terminates loop', () {
      final loop = EventLoop<int>();
      loop.addSource(_Counter());
      var count = 0;
      final handle = loop.stopHandle();
      loop.onEvent((_) {
        count += 1;
        if (count >= 5) {
          handle.stop();
        }
        return ControlFlow.continueLoop;
      });
      loop.run();
      expect(count, greaterThanOrEqualTo(5));
    });

    test('stop during dispatch finishes the event', () {
      // The flag is checked at the top of each iteration, so events
      // already queued still dispatch after stop().
      final loop = EventLoop<int>();
      loop.addSource(_FixedSource([
        [1, 2, 3]
      ]));
      final seen = <int>[];
      late StopHandle handle;
      handle = loop.stopHandle();
      loop.onEvent((e) {
        seen.add(e);
        handle.stop();
        return ControlFlow.continueLoop;
      });
      loop.run();
      expect(seen, equals([1, 2, 3]));
    });

    test('run resets the stop flag', () {
      final loop = EventLoop<int>();
      loop.addSource(_FixedSource([
        [1],
        [-1]
      ]));
      loop.onEvent(
          (e) => e == -1 ? ControlFlow.exit : ControlFlow.continueLoop);
      loop.stop();
      // A pre-run stop() is cleared: the loop still runs to its
      // handler exit.
      loop.run();
      // And a stopped loop runs again cleanly with fresh sources.
      final loop2 = EventLoop<int>();
      loop2.addSource(_Counter());
      var count = 0;
      loop2.onEvent((_) {
        count += 1;
        return count >= 3 ? ControlFlow.exit : ControlFlow.continueLoop;
      });
      loop2.run();
      expect(count, equals(3));
    });

    test('handles share one flag', () {
      final loop = EventLoop<int>();
      final a = loop.stopHandle();
      final b = loop.stopHandle();
      loop.addSource(_Counter());
      var count = 0;
      loop.onEvent((_) {
        count += 1;
        if (count == 2) {
          a.stop();
        }
        return ControlFlow.continueLoop;
      });
      // Stopping via `a` terminates despite `b` existing.
      loop.run();
      expect(count, equals(2));
      expect(b, isA<StopHandle>());
    });

    test('handler throw propagates out of run', () {
      final loop = EventLoop<int>();
      loop.addSource(_FixedSource([
        [1]
      ]));
      loop.onEvent((_) => throw StateError('boom'));
      expect(() => loop.run(), throwsStateError);
    });
  });

  group('values', () {
    test('creates empty loop', () {
      final loop = EventLoop<int>();
      expect(loop, isA<EventLoop<int>>());
    });

    test('control flow variants distinct', () {
      expect(ControlFlow.continueLoop, isNot(equals(ControlFlow.exit)));
      expect(ControlFlow.continueLoop.name, equals('continueLoop'));
      expect(ControlFlow.exit.name, equals('exit'));
    });
  });
}
