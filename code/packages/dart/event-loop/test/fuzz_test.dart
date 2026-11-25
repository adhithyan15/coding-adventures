/// Differential fuzz campaign for `coding_adventures_event_loop`,
/// using the in-lane `fuzz` package: hostile byte strings decode
/// into multi-source batch scripts driving the loop, and the model
/// asserts delivery order, exit position, fan-out, and handler
/// short-circuit — with no errors escaping at all (the API is
/// total). An exit sentinel always terminates the script, so the
/// campaign cannot hang. Fixed seed, so it reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_event_loop/event_loop.dart';
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:test/test.dart';

bool _equalInts(List<int> a, List<int> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

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

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    if (input.length < 4) {
      return const OutcomePass();
    }
    // Two hostile batch scripts over positive events (0 kept free
    // as the exit sentinel), plus a sentinel-only third source.
    List<int> events(int offset, int length) => List<int>.generate(
        length, (i) => input[(offset + i) % input.length] + 1);
    final aLen = input[0] % 9;
    final bLen = input[1] % 9;
    final a = events(2, aLen);
    final b = events(2 + aLen, bLen);
    cov.recordEdge(a.length % 8);
    cov.recordEdge(10 + (b.length % 8));

    final loop = EventLoop<int>();
    loop.addSource(_Script([a]));
    loop.addSource(_Script([b]));
    loop.addSource(_Script([
      [],
      [0],
    ]));
    final seenA = <int>[];
    final seenB = <int>[];
    var calls = 0;
    loop.onEvent((e) {
      calls += 1;
      if (e == 0) {
        return ControlFlow.exit;
      }
      seenA.add(e);
      return ControlFlow.continueLoop;
    });
    loop.onEvent((e) {
      if (e == 0) {
        return ControlFlow.exit;
      }
      seenB.add(e);
      return ControlFlow.continueLoop;
    });
    loop.run();

    // Both handlers saw the merged batches in order; the second
    // handler never saw the sentinel (first handler exits first).
    final want = [...a, ...b];
    if (!_equalInts(seenA, want) || !_equalInts(seenB, want)) {
      return const OutcomeCrash('model-divergence', ['delivery']);
    }
    // The first handler saw every event plus the sentinel (which
    // stopped dispatch before the second handler ran).
    if (calls != want.length + 1) {
      return const OutcomeCrash('model-divergence', ['call-count']);
    }

    cov.recordEdge(20 + (calls % 16));
    return const OutcomePass();
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList([1, 2, 3, 4]),
      Uint8List.fromList(List<int>.filled(16, 0x07)),
      Uint8List.fromList(List<int>.generate(48, (i) => (i * 7 + 3) & 0xff)),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed
/// 711); longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 96,
          seed: seed ?? BigInt.from(711)),
      _target,
    );

void main() {
  group('fuzz campaign', () {
    test('bounded run: model agrees, no crashes', () {
      final report = runCampaign();
      expect(report.crashes, isEmpty,
          reason: report.crashes
              .map((c) =>
                  '${c.signature} ${c.kind} ${c.frames} input=${c.minimalInput}')
              .join('; '));
      expect(report.iterations, greaterThan(100));
      expect(report.totalEdges, greaterThan(5));
    });
  });
}
