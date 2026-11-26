/// Property tests for `coding_adventures_event_loop`, using the
/// in-lane `pbt` package: delivery equals the scripted batches,
/// exit position, handler fan-out, source order, determinism, and
/// stop-handle termination — all with fixed seeds. Every script
/// ends with an exit sentinel so the loop always terminates.
library;

import 'package:coding_adventures_event_loop/event_loop.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

/// Scripted source replaying fixed batches, then silence.
class _Script implements EventSource<int> {
  final List<List<int>> batches;
  var index = 0;

  _Script(this.batches);

  @override
  List<int> poll() {
    if (index >= batches.length) {
      return [];
    }
    return batches[index++];
  }
}

bool _equalInts(List<int> a, List<int> b) {
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

/// Runs [batches] plus an exit sentinel, collecting deliveries.
List<int> _deliver(List<List<int>> batches) {
  final loop = EventLoop<int>();
  loop.addSource(_Script([
    ...batches,
    [-1]
  ]));
  final seen = <int>[];
  loop.onEvent((e) {
    if (e == -1) {
      return ControlFlow.exit;
    }
    seen.add(e);
    return ControlFlow.continueLoop;
  });
  loop.run();
  return seen;
}

void main() {
  group('properties', () {
    test('delivery equals scripted batches in order', () {
      final gen = Gen.tuple3(
        Gen.bytes(0, 8),
        Gen.bytes(0, 8),
        Gen.bytes(0, 8),
      );
      _check(
        'delivery order',
        checkProperty<(List<int>, List<int>, List<int>)>(
          BigInt.from(701),
          PbtOptions(numTests: 200, seed: BigInt.from(701)),
          (rng) {
            final t = gen.sample(rng);
            // Shift clear of the -1 sentinel (bytes are 0..255).
            return (
              t.$1.map((b) => b + 1).toList(),
              t.$2.map((b) => b + 1).toList(),
              t.$3.map((b) => b + 1).toList(),
            );
          },
          (input) => _equalInts(
            _deliver([input.$1, input.$2, input.$3]),
            [...input.$1, ...input.$2, ...input.$3],
          ),
        ),
      );
    });

    test('exit stops at the sentinel', () {
      final gen = Gen.tuple2(Gen.bytes(1, 8), Gen.bytes(1, 8));
      _check(
        'exit position',
        checkProperty<(List<int>, List<int>)>(
          BigInt.from(702),
          PbtOptions(numTests: 200, seed: BigInt.from(702)),
          (rng) {
            final t = gen.sample(rng);
            return (
              t.$1.map((b) => b + 1).toList(),
              t.$2.map((b) => b + 1).toList(),
            );
          },
          (input) {
            final loop = EventLoop<int>();
            loop.addSource(_Script([
              [...input.$1, -1],
              input.$2,
            ]));
            final seen = <int>[];
            loop.onEvent((e) {
              if (e == -1) {
                return ControlFlow.exit;
              }
              seen.add(e);
              return ControlFlow.continueLoop;
            });
            loop.run();
            // Everything before the sentinel, nothing after.
            return _equalInts(seen, input.$1);
          },
        ),
      );
    });

    test('every handler sees every event', () {
      final gen = Gen.tuple2(Gen.bytes(0, 8), Gen.u8());
      _check(
        'fan-out',
        checkProperty<(List<int>, int)>(
          BigInt.from(703),
          PbtOptions(numTests: 100, seed: BigInt.from(703)),
          (rng) {
            final t = gen.sample(rng);
            return (t.$1.map((b) => b + 1).toList(), t.$2);
          },
          (input) {
            final loop = EventLoop<int>();
            loop.addSource(_Script([
              input.$1,
              [-1]
            ]));
            final seenA = <int>[];
            final seenB = <int>[];
            final seenC = <int>[];
            for (final seen in [seenA, seenB, seenC]) {
              loop.onEvent((e) {
                if (e == -1) {
                  return ControlFlow.exit;
                }
                seen.add(e);
                return ControlFlow.continueLoop;
              });
            }
            loop.run();
            return _equalInts(seenA, input.$1) &&
                _equalInts(seenB, input.$1) &&
                _equalInts(seenC, input.$1);
          },
        ),
      );
    });

    test('sources interleave in registration order', () {
      final gen = Gen.tuple3(Gen.bytes(0, 6), Gen.bytes(0, 6), Gen.u8());
      _check(
        'source order',
        checkProperty<(List<int>, List<int>, int)>(
          BigInt.from(704),
          PbtOptions(numTests: 100, seed: BigInt.from(704)),
          (rng) {
            final t = gen.sample(rng);
            return (
              t.$1.map((b) => b + 1).toList(),
              t.$2.map((b) => b + 1).toList(),
              t.$3,
            );
          },
          (input) {
            final loop = EventLoop<int>();
            loop.addSource(_Script([input.$1]));
            loop.addSource(_Script([input.$2]));
            loop.addSource(_Script([
              [],
              [-1]
            ]));
            final seen = <int>[];
            loop.onEvent((e) {
              if (e == -1) {
                return ControlFlow.exit;
              }
              seen.add(e);
              return ControlFlow.continueLoop;
            });
            loop.run();
            // Iteration 1 merges batch 1 of each source in order;
            // iteration 2 carries only the sentinel.
            return _equalInts(seen, [...input.$1, ...input.$2]);
          },
        ),
      );
    });

    test('delivery is deterministic', () {
      final gen = Gen.tuple2(Gen.bytes(0, 8), Gen.bytes(0, 8));
      _check(
        'determinism',
        checkProperty<(List<int>, List<int>)>(
          BigInt.from(705),
          PbtOptions(numTests: 100, seed: BigInt.from(705)),
          (rng) {
            final t = gen.sample(rng);
            return (
              t.$1.map((b) => b + 1).toList(),
              t.$2.map((b) => b + 1).toList(),
            );
          },
          (input) => _equalInts(
            _deliver([input.$1, input.$2]),
            _deliver([input.$1, input.$2]),
          ),
        ),
      );
    });

    test('stop handle terminates infinite sources', () {
      final gen = Gen.u8();
      _check(
        'stop handle',
        checkProperty<int>(
          BigInt.from(706),
          PbtOptions(numTests: 60, seed: BigInt.from(706)),
          (rng) => gen.sample(rng),
          (limit) {
            final loop = EventLoop<int>();
            loop.addSource(_Counter());
            final handle = loop.stopHandle();
            var count = 0;
            final target = 1 + (limit % 20);
            loop.onEvent((_) {
              count += 1;
              if (count >= target) {
                handle.stop();
              }
              return ControlFlow.continueLoop;
            });
            loop.run();
            return count == target;
          },
        ),
      );
    });
  });
}

class _Counter implements EventSource<int> {
  var value = 0;

  @override
  List<int> poll() {
    value += 1;
    return [value];
  }
}
