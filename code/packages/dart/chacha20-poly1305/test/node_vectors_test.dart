// Differential tests against Node.js built-in ChaCha20-Poly1305
// (OpenSSL provider): 90 seeded vectors over keys, nonces, AAD,
// and boundary-heavy plaintext lengths. Regenerate with
// `node test/generate_node_vectors.mjs`.
library;

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_chacha20_poly1305/chacha20_poly1305.dart';
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
      final triples = <String>{};
      for (final v in vectors) {
        triples.add('${v['key']}${v['ct']}${v['tag']}');
      }
      expect(triples.length, equals(90));
    });

    test('compat AEAD matches node on all vectors', () {
      for (var i = 0; i < vectors.length; i++) {
        final v = vectors[i];
        final (ct, tag) = aeadEncrypt(
          _unhex(v['pt'] as String),
          _unhex(v['key'] as String),
          _unhex(v['nonce'] as String),
          _unhex(v['aad'] as String),
        );
        expect(_hex(ct), equals(v['ct']), reason: 'vector $i ct');
        expect(_hex(tag), equals(v['tag']), reason: 'vector $i tag');
        expect(
            aeadDecrypt(
              ct,
              _unhex(v['key'] as String),
              _unhex(v['nonce'] as String),
              _unhex(v['aad'] as String),
              tag,
            ),
            equals(_unhex(v['pt'] as String)),
            reason: 'vector $i open');
      }
    });

    test('fixed AEAD matches node on all vectors', () {
      for (var i = 0; i < vectors.length; i++) {
        final v = vectors[i];
        final pt = _unhex(v['pt'] as String);
        final key = _unhex(v['key'] as String);
        final nonce = _unhex(v['nonce'] as String);
        final aad = _unhex(v['aad'] as String);
        final ct = Uint8List(pt.length);
        final tag = Uint8List(16);
        aeadEncryptFixed(pt, key, nonce, aad, ct, tag);
        expect(_hex(ct), equals(v['ct']), reason: 'vector $i ct');
        expect(_hex(tag), equals(v['tag']), reason: 'vector $i tag');
        final opened = Uint8List(pt.length);
        aeadDecryptFixed(ct, key, nonce, aad, tag, opened);
        expect(opened, equals(pt), reason: 'vector $i open');
      }
    });
  });
}
