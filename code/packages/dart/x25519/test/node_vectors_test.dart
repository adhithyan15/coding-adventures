// Differential tests against Node.js built-in X25519 (OpenSSL
// provider): 100 seeded vectors over random scalar pairs, each with
// both public keys and the shared secret (computed by the oracle in
// both directions and asserted equal there). Regenerate with
// `node test/generate_node_vectors.mjs`.
library;

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_x25519/x25519.dart';
import 'package:test/test.dart';

Uint8List _unhex(String hex) {
  final out = Uint8List(hex.length ~/ 2);
  for (var i = 0; i < out.length; i++) {
    out[i] = int.parse(hex.substring(i * 2, i * 2 + 2), radix: 16);
  }
  return out;
}

String _hex(List<int> bytes) =>
    bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

void main() {
  final fixture = jsonDecode(
    File('test/node_vectors.json').readAsStringSync(),
  ) as Map<String, dynamic>;
  final vectors = (fixture['vectors'] as List).cast<Map<String, dynamic>>();

  group('node differential', () {
    test('fixture carries 100 diverse seeded vectors', () {
      expect(fixture['seed'], equals(20261129));
      expect(vectors, hasLength(100));
      // Guards the ed25519-lane failure mode (a generator bug that
      // shipped 100 all-zero vectors): the fixture must be diverse.
      final seeds = <String>{};
      final shareds = <String>{};
      for (final v in vectors) {
        seeds.add(v['seedA'] as String);
        seeds.add(v['seedB'] as String);
        shareds.add(v['shared'] as String);
      }
      expect(seeds.length, equals(200));
      expect(shareds.length, equals(100));
    });

    test('public keys match node on all vectors', () {
      for (var i = 0; i < vectors.length; i++) {
        final v = vectors[i];
        expect(
          _hex(x25519Base(_unhex(v['seedA'] as String))),
          equals(v['pubA']),
          reason: 'vector $i pubA',
        );
        expect(
          _hex(x25519Base(_unhex(v['seedB'] as String))),
          equals(v['pubB']),
          reason: 'vector $i pubB',
        );
      }
    });

    test('shared secrets match node in both directions', () {
      for (var i = 0; i < vectors.length; i++) {
        final v = vectors[i];
        final seedA = _unhex(v['seedA'] as String);
        final seedB = _unhex(v['seedB'] as String);
        final pubA = _unhex(v['pubA'] as String);
        final pubB = _unhex(v['pubB'] as String);
        expect(_hex(x25519(seedA, pubB)), equals(v['shared']),
            reason: 'vector $i A->B');
        expect(_hex(x25519(seedB, pubA)), equals(v['shared']),
            reason: 'vector $i B->A');
      }
    });
  });
}
