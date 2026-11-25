// Differential tests against Node.js built-in HKDF (OpenSSL
// provider): 60 seeded vectors (30 per digest) over boundary-heavy
// salts, IKMs, infos, and output lengths, including the full
// 255-block chain per digest. Regenerate with
// `node test/generate_node_vectors.mjs`.
library;

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_hkdf/hkdf.dart';
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
    test('fixture carries 60 diverse seeded vectors', () {
      expect(fixture['seed'], equals(20261130));
      expect(vectors, hasLength(60));
      final okms = <String>{};
      for (final v in vectors) {
        okms.add(v['okm'] as String);
      }
      expect(okms.length, equals(60));
    });

    test('combined output matches node on all vectors', () {
      for (var i = 0; i < vectors.length; i++) {
        final v = vectors[i];
        final algorithm = v['digest'] == 'sha256'
            ? HashAlgorithm.sha256
            : HashAlgorithm.sha512;
        final got = hkdf(
          _unhex(v['salt'] as String),
          _unhex(v['ikm'] as String),
          _unhex(v['info'] as String),
          v['length'] as int,
          algorithm,
        );
        expect(_hex(got), equals(v['okm']), reason: 'vector $i');
      }
    });

    test('manual extract-then-expand matches node on all vectors', () {
      for (var i = 0; i < vectors.length; i++) {
        final v = vectors[i];
        final algorithm = v['digest'] == 'sha256'
            ? HashAlgorithm.sha256
            : HashAlgorithm.sha512;
        final prk = hkdfExtract(
          _unhex(v['salt'] as String),
          _unhex(v['ikm'] as String),
          algorithm,
        );
        final got = hkdfExpand(
          prk,
          _unhex(v['info'] as String),
          v['length'] as int,
          algorithm,
        );
        expect(_hex(got), equals(v['okm']), reason: 'vector $i');
      }
    });
  });
}
