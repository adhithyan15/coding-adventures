// Differential tests against Node.js built-in AES-ECB (OpenSSL
// provider, no padding — exactly one block): 90 seeded vectors (30
// per key size) over random keys and blocks. Regenerate with
// `node test/generate_node_vectors.mjs`.
library;

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_aes/aes.dart';
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
    test('fixture carries 90 diverse seeded vectors', () {
      expect(fixture['seed'], equals(20261130));
      expect(vectors, hasLength(90));
      final cts = <String>{};
      for (final v in vectors) {
        cts.add(v['ct'] as String);
      }
      expect(cts.length, equals(90));
    });

    test('encrypt matches node on all vectors', () {
      for (var i = 0; i < vectors.length; i++) {
        final v = vectors[i];
        final got = encryptBlock(
          _unhex(v['plain'] as String),
          _unhex(v['key'] as String),
        );
        expect(_hex(got), equals(v['ct']), reason: 'vector $i');
      }
    });

    test('decrypt matches node on all vectors', () {
      for (var i = 0; i < vectors.length; i++) {
        final v = vectors[i];
        final got = decryptBlock(
          _unhex(v['ct'] as String),
          _unhex(v['key'] as String),
        );
        expect(_hex(got), equals(v['plain']), reason: 'vector $i');
      }
    });

    test('fixed path matches node on 128-bit vectors', () {
      for (var i = 0; i < vectors.length; i++) {
        final v = vectors[i];
        if (v['bits'] != 128) continue;
        final output = Uint8List(16);
        aes128EncryptBlockFixed(
          _unhex(v['plain'] as String),
          _unhex(v['key'] as String),
          output,
        );
        expect(_hex(output), equals(v['ct']), reason: 'vector $i');
      }
    });
  });
}
