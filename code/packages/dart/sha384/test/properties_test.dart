/// Property tests for `coding_adventures_sha384`, using the in-lane
/// `pbt` package: chunking equivalence, digest idempotence and
/// continuation, empty-update noops, determinism, avalanche, dispose
/// semantics, and counter-boundary failure — all with fixed seeds.
library;

import 'dart:typed_data';

import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_sha384/sha384.dart';
import 'package:test/test.dart';

void _check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

bool _equalBytes(List<int> a, List<int> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

void main() {
  group('properties', () {
    test('chunking never changes the digest', () {
      final gen = Gen.tuple2(Gen.bytes(0, 400), Gen.u8());
      final verdict = checkProperty<(Uint8List, int)>(
        BigInt.from(461),
        PbtOptions(numTests: 200, seed: BigInt.from(461)),
        (rng) => gen.sample(rng),
        (input) {
          final data = input.$1;
          final want = sha384(data);
          final cuts = [0, data.length];
          if (data.length > 2) {
            cuts.add(input.$2 % data.length);
            cuts.add((input.$2 * 7 + 1) % data.length);
          }
          cuts.sort();
          final h = Sha384Hasher();
          for (var i = 0; i + 1 < cuts.length; i++) {
            h.update(data.sublist(cuts[i], cuts[i + 1]));
          }
          return _equalBytes(h.digest(), want);
        },
      );
      _check('chunking', verdict);
    });

    test('digest is idempotent and continuation works', () {
      final gen = Gen.bytes(0, 300);
      final verdict = checkProperty<Uint8List>(
        BigInt.from(462),
        PbtOptions(numTests: 200, seed: BigInt.from(462)),
        (rng) => gen.sample(rng),
        (data) {
          final h = Sha384Hasher();
          final mid = data.length ~/ 2;
          h.update(data.sublist(0, mid));
          final prefix = h.digest();
          if (!_equalBytes(prefix, sha384(data.sublist(0, mid)))) {
            return false;
          }
          if (!_equalBytes(h.digest(), prefix)) return false;
          h.update(Uint8List(0));
          if (!_equalBytes(h.digest(), prefix)) return false;
          h.update(data.sublist(mid));
          return _equalBytes(h.digest(), sha384(data));
        },
      );
      _check('idempotent-continue', verdict);
    });

    test('one input bit flips the digest', () {
      final gen = Gen.tuple2(Gen.bytes(1, 200), Gen.u8());
      final verdict = checkProperty<(Uint8List, int)>(
        BigInt.from(463),
        PbtOptions(numTests: 200, seed: BigInt.from(463)),
        (rng) => gen.sample(rng),
        (input) {
          final data = input.$1;
          final base = sha384(data);
          final flipped = Uint8List.fromList(data);
          final at = input.$2 % flipped.length;
          flipped[at] ^= 1;
          return !_equalBytes(sha384(flipped), base);
        },
      );
      _check('avalanche', verdict);
    });

    test('digests are always 48 bytes', () {
      final gen = Gen.bytes(0, 300);
      final verdict = checkProperty<Uint8List>(
        BigInt.from(464),
        PbtOptions(numTests: 200, seed: BigInt.from(464)),
        (rng) => gen.sample(rng),
        (data) {
          if (sha384(data).length != 48) return false;
          if (sha384Hex(data).length != 96) return false;
          final h = Sha384Hasher()..update(data);
          return h.digest().length == 48;
        },
      );
      _check('digest-width', verdict);
    });

    test('dispose locks the hasher', () {
      final gen = Gen.bytes(0, 100);
      final verdict = checkProperty<Uint8List>(
        BigInt.from(465),
        PbtOptions(numTests: 200, seed: BigInt.from(465)),
        (rng) => gen.sample(rng),
        (data) {
          final h = Sha384Hasher()..update(data);
          if (h.isDisposed) return false;
          final before = h.digest();
          h.dispose();
          h.dispose();
          if (!h.isDisposed) return false;
          try {
            h.update(data);
            return false;
          } on StateError {
            // Expected.
          }
          try {
            h.digest();
            return false;
          } on StateError {
            // Expected.
          }
          return _equalBytes(before, sha384(data));
        },
      );
      _check('dispose-locks', verdict);
    });

    test('counter boundary fails transactionally', () {
      final gen = Gen.bytes(1, 64);
      final verdict = checkProperty<Uint8List>(
        BigInt.from(466),
        PbtOptions(numTests: 100, seed: BigInt.from(466)),
        (rng) => gen.sample(rng),
        (data) {
          final maxBytes = ((BigInt.one << 128) - BigInt.one) >> 3;
          final h = Sha384Hasher()..update(data);
          final before = h.digest();
          // One byte below the limit still absorbs.
          setSha384ByteCounterForTesting(
              h, maxBytes - BigInt.from(data.length));
          try {
            h.update(data);
          } on Sha384MessageTooLong {
            return false;
          }
          // At the limit, any nonempty update fails without changing state.
          setSha384ByteCounterForTesting(h, maxBytes);
          final frozen = h.digest();
          try {
            h.update([1]);
            return false;
          } on Sha384MessageTooLong {
            // Expected.
          }
          if (!_equalBytes(h.digest(), frozen)) return false;
          // Empty updates at the limit still succeed (zero-length add).
          try {
            h.update([]);
          } on Sha384MessageTooLong {
            return false;
          }
          return _equalBytes(h.digest(), frozen) &&
              _equalBytes(before, sha384(data));
        },
      );
      _check('counter-boundary', verdict);
    });
  });
}
