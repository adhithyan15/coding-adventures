/// Property tests for `coding_adventures_zeroize`, using the in-lane
/// `pbt` package: wipe totality, idempotence, take/dispose state
/// machines, and aliasing — all with fixed seeds.
library;

import 'dart:typed_data';

import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_zeroize/zeroize.dart';
import 'package:test/test.dart';

void _check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

bool _isAllZero(List<int> bytes) => bytes.every((b) => b == 0);

void main() {
  group('properties', () {
    test('zeroize always leaves all-zero bytes', () {
      final verdict = checkProperty<Uint8List>(
        BigInt.from(421),
        PbtOptions(numTests: 300, seed: BigInt.from(421)),
        (rng) => Gen.bytes(0, 300).sample(rng),
        (bytes) {
          final before = bytes.length;
          zeroizeBytes(bytes);
          return bytes.length == before && _isAllZero(bytes);
        },
      );
      _check('wipe-totality', verdict);
    });

    test('extension and function wipe identically', () {
      final verdict = checkProperty<Uint8List>(
        BigInt.from(422),
        PbtOptions(numTests: 300, seed: BigInt.from(422)),
        (rng) => Gen.bytes(0, 300).sample(rng),
        (bytes) {
          final twin = Uint8List.fromList(bytes);
          zeroizeBytes(bytes);
          twin.zeroize();
          if (bytes.length != twin.length) return false;
          for (var i = 0; i < bytes.length; i++) {
            if (bytes[i] != 0 || twin[i] != 0) return false;
          }
          return true;
        },
      );
      _check('api-agreement', verdict);
    });

    test('dispose state machine is exact', () {
      final verdict = checkProperty<Uint8List>(
        BigInt.from(423),
        PbtOptions(numTests: 300, seed: BigInt.from(423)),
        (rng) => Gen.bytes(0, 100).sample(rng),
        (bytes) {
          final guard = ZeroizingBytes(bytes);
          if (guard.isDisposed) return false;
          // Reads work before disposal.
          if (!identical(guard.bytes, bytes)) return false;
          guard.dispose();
          if (!guard.isDisposed) return false;
          if (!_isAllZero(bytes)) return false;
          // Second dispose is safe; reads and takes throw.
          guard.dispose();
          try {
            guard.bytes;
            return false;
          } on StateError {
            // Expected.
          }
          try {
            guard.takeBytes();
            return false;
          } on StateError {
            // Expected.
          }
          return true;
        },
      );
      _check('dispose-machine', verdict);
    });

    test('takeBytes preserves bytes and disposes', () {
      final verdict = checkProperty<Uint8List>(
        BigInt.from(424),
        PbtOptions(numTests: 300, seed: BigInt.from(424)),
        (rng) => Gen.bytes(0, 100).sample(rng),
        (bytes) {
          final snapshot = Uint8List.fromList(bytes);
          final guard = ZeroizingBytes(bytes);
          final taken = guard.takeBytes();
          if (!identical(taken, bytes)) return false;
          if (taken.length != snapshot.length) return false;
          for (var i = 0; i < taken.length; i++) {
            if (taken[i] != snapshot[i]) return false;
          }
          if (!guard.isDisposed) return false;
          // Disposing after taking is a safe noop on already-dead state.
          guard.dispose();
          return true;
        },
      );
      _check('take-semantics', verdict);
    });

    test('forgotten dispose leaves bytes live (documented)', () {
      // The anti-property that justifies the docs: without dispose(),
      // nothing wipes. A never-disposed wrapper must still read back
      // the original bytes — proving no magic finalizer exists.
      final verdict = checkProperty<Uint8List>(
        BigInt.from(425),
        PbtOptions(numTests: 200, seed: BigInt.from(425)),
        (rng) => Gen.bytes(1, 100).sample(rng),
        (bytes) {
          final snapshot = Uint8List.fromList(bytes);
          final guard = ZeroizingBytes(bytes);
          final live = guard.bytes;
          if (live.length != snapshot.length) return false;
          for (var i = 0; i < live.length; i++) {
            if (live[i] != snapshot[i]) return false;
          }
          return !guard.isDisposed;
        },
      );
      _check('no-magic-wipe', verdict);
    });
  });
}
