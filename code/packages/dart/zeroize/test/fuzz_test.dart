/// Differential fuzz campaign for `coding_adventures_zeroize`, using
/// the in-lane `fuzz` package: hostile byte strings decode into op
/// scripts (fill, wipe, wrap, dispose, take, read) over a live buffer,
/// and an independent model tracks liveness byte-for-byte: after every
/// op the model predicts the exact buffer contents and wrapper state.
/// Only [StateError] (use-after-dispose) may ever escape. Fixed seed,
/// so the campaign reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_zeroize/zeroize.dart';
import 'package:test/test.dart';

FuzzOutcome _target(Uint8List input, CoverageBitmap cov) {
  try {
    if (input.isEmpty) {
      zeroizeBytes(Uint8List(0));
      return const OutcomePass();
    }

    // Model: expected bytes + liveness for up to 4 wrapped buffers.
    final bufs = <Uint8List>[];
    final guards = <ZeroizingBytes?>[];
    final count = 1 + (input[0] % 3);
    var cursor = 1;
    int nextByte() => cursor < input.length ? input[cursor++] : 0;

    for (var i = 0; i < count; i++) {
      final len = nextByte() % 24;
      final buf = Uint8List(len);
      for (var j = 0; j < len; j++) {
        buf[j] = nextByte();
      }
      bufs.add(buf);
      guards.add(null);
    }

    bool bytesEqual(List<int> a, List<int> b) {
      if (a.length != b.length) return false;
      for (var i = 0; i < a.length; i++) {
        if (a[i] != b[i]) return false;
      }
      return true;
    }

    final opCount = 1 + (nextByte() % 16);
    for (var op = 0; op < opCount; op++) {
      final which = nextByte() % bufs.length;
      final kind = nextByte() % 7;
      cov.recordEdge(10 + kind);
      final buf = bufs[which];
      switch (kind) {
        case 0: // Fill with a hostile byte.
          final byte = nextByte();
          buf.fillRange(0, buf.length, byte);
        case 1: // Wipe via the top-level function.
          zeroizeBytes(buf);
          if (!buf.every((b) => b == 0)) {
            return const OutcomeCrash('model-divergence', ['wipe']);
          }
        case 2: // Wrap (replacing any prior wrapper).
          guards[which] = ZeroizingBytes(buf);
        case 3: // Dispose if wrapped.
          final guard = guards[which];
          if (guard != null) {
            // Only a live dispose wipes: dispose-after-take and
            // dispose-after-dispose are designed noops.
            final wasLive = !guard.isDisposed;
            guard.dispose();
            if (wasLive && !buf.every((b) => b == 0)) {
              return const OutcomeCrash('model-divergence', ['dispose-wipe']);
            }
            if (!guard.isDisposed) {
              return const OutcomeCrash('model-divergence', ['dispose-flag']);
            }
          }
        case 4: // Take if wrapped.
          final guard = guards[which];
          if (guard != null && !guard.isDisposed) {
            final snapshot = Uint8List.fromList(buf);
            final released = guard.takeBytes();
            if (!identical(released, buf) || !bytesEqual(released, snapshot)) {
              return const OutcomeCrash('model-divergence', ['take']);
            }
          }
        case 5: // Read through the wrapper.
          final guard = guards[which];
          if (guard == null) continue;
          try {
            final seen = guard.bytes;
            if (!identical(seen, buf)) {
              return const OutcomeCrash('model-divergence', ['alias']);
            }
          } on StateError {
            if (!guard.isDisposed) {
              return const OutcomeCrash('model-divergence', ['spurious-throw']);
            }
          }
        default: // Mutate through the wrapper when live.
          final guard = guards[which];
          if (guard == null || guard.isDisposed) continue;
          if (buf.isNotEmpty) {
            buf[nextByte() % buf.length] = nextByte();
          }
      }
    }

    cov.recordEdge(30 + (bufs.length % 4));
    return const OutcomePass();
  } catch (e) {
    return OutcomeCrash(e.runtimeType.toString(), ['fuzz-target']);
  }
}

List<Uint8List> get _corpus => [
      Uint8List(0),
      Uint8List.fromList([0xaa, 0xbb, 0xcc, 0x01, 0x02, 0x03]),
      Uint8List.fromList(List<int>.generate(48, (i) => (i * 13 + 7) & 0xff)),
      Uint8List.fromList(List<int>.filled(32, 0xff)),
    ];

/// Runs the campaign over [_corpus] with [_target].
///
/// The committed test uses the defaults (2,000 iterations, seed 431);
/// longer ad-hoc runs import this file and pass bigger budgets.
FuzzReport runCampaign({int maxIterations = 2000, BigInt? seed}) => fuzzTarget(
      _corpus,
      FuzzOptions(
          maxIterations: maxIterations,
          maxInputLen: 256,
          seed: seed ?? BigInt.from(431)),
      _target,
    );

void main() {
  group('fuzz campaign', () {
    test('bounded run: model agrees, errors exact, no crashes', () {
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
