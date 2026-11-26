// Unit tests for the zeroize lane: the portable reference tests
// mirrored case-for-case, adapted where the platform differs (explicit
// dispose instead of Drop; no String/int/Option impls — immutable
// values cannot be wiped, documented in the library docs).

import 'dart:typed_data';

import 'package:coding_adventures_zeroize/zeroize.dart';
import 'package:test/test.dart';

bool _isAllZero(List<int> bytes) => bytes.every((b) => b == 0);

void main() {
  group('zeroize', () {
    test('byte buffer is zeroed', () {
      final buf = Uint8List.fromList(List<int>.filled(32, 0xaa));
      zeroizeBytes(buf);
      expect(buf, equals(Uint8List(32)));
    });

    test('empty buffer is a noop', () {
      final empty = Uint8List(0);
      zeroizeBytes(empty);
      expect(empty, isEmpty);
    });

    test('extension wipes in place', () {
      final buf = Uint8List.fromList(List<int>.filled(16, 0xff));
      buf.zeroize();
      expect(_isAllZero(buf), isTrue);
    });

    test('whole fixed buffer is scrubbed (no hidden tail)', () {
      // Uint8List is fixed-length: fillRange covers every byte, so the
      // reference's capacity-tail concern has no equivalent. Length is
      // preserved (unlike Vec::clear, there is nothing to truncate to).
      final buf = Uint8List.fromList(List<int>.filled(64, 0x55));
      buf.zeroize();
      expect(buf.length, equals(64));
      expect(_isAllZero(buf), isTrue);
    });

    test('wrapper reads and mutates through', () {
      final key =
          ZeroizingBytes(Uint8List.fromList(List<int>.filled(32, 0x42)));
      expect(key.bytes[0], equals(0x42));
      expect(key.bytes.length, equals(32));
      expect(key.isDisposed, isFalse);

      final key2 = ZeroizingBytes(Uint8List(4));
      key2.bytes[0] = 1;
      key2.bytes[1] = 2;
      expect(key2.bytes, equals([1, 2, 0, 0]));
    });

    test('dispose wipes the caller-owned buffer', () {
      final owned = Uint8List.fromList(List<int>.filled(48, 0xcd));
      final guard = ZeroizingBytes(owned);
      guard.dispose();
      expect(guard.isDisposed, isTrue);
      expect(_isAllZero(owned), isTrue);
    });

    test('takeBytes opts out of the wipe', () {
      final guard =
          ZeroizingBytes(Uint8List.fromList(List<int>.filled(16, 0x77)));
      final taken = guard.takeBytes();
      expect(taken, equals(List<int>.filled(16, 0x77)));
      expect(guard.isDisposed, isTrue);
    });

    test('dispose in finally wipes on exception paths', () {
      // The reference relies on Drop during unwinding; Dart has no
      // destructors, so the documented pattern is try/finally. This
      // pins that the pattern actually wipes when the body throws.
      final owned = Uint8List.fromList(List<int>.filled(24, 0xab));
      final guard = ZeroizingBytes(owned);
      void run() {
        try {
          throw StateError('simulated mid-operation failure');
        } finally {
          guard.dispose();
        }
      }

      expect(run, throwsStateError);
      expect(_isAllZero(owned), isTrue);
    });

    test('dispose is idempotent', () {
      final guard = ZeroizingBytes(Uint8List.fromList([1, 2, 3]));
      guard.dispose();
      guard.dispose();
      expect(guard.isDisposed, isTrue);
    });

    test('use after dispose throws StateError', () {
      final guard = ZeroizingBytes(Uint8List.fromList([1]));
      guard.dispose();
      expect(() => guard.bytes, throwsStateError);
      expect(() => guard.takeBytes(), throwsStateError);
    });

    test('wrappers use identity, never value equality', () {
      final a = ZeroizingBytes(Uint8List.fromList([1]));
      final b = ZeroizingBytes(Uint8List.fromList([1]));
      expect(a == b, isFalse);
      expect(identical(a, a), isTrue);
    });

    test('views alias the wiped region', () {
      // A sublist view shares storage: wiping through either handle
      // affects the shared bytes. Documents aliasing semantics.
      final buf = Uint8List.fromList([1, 2, 3, 4]);
      final view = Uint8List.sublistView(buf, 1, 3);
      view.zeroize();
      expect(buf, equals([1, 0, 0, 4]));
    });
  });
}
