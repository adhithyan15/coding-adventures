// Differential tests against Node.js built-in Ed25519 (OpenSSL
// provider): 100 seeded vectors over random seeds and boundary-heavy
// message lengths. Regenerate with
// `node test/generate_node_vectors.mjs`.
library;

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_ed25519/ed25519.dart';
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
    test('fixture carries 100 seeded vectors', () {
      expect(fixture['seed'], equals(20261125));
      expect(vectors, hasLength(100));
    });

    test('keygen matches node on all vectors', () {
      for (var i = 0; i < vectors.length; i++) {
        final v = vectors[i];
        final (pub, _) = generateKeypair(_unhex(v['seed'] as String));
        expect(_hex(pub.bytes), equals(v['pub']), reason: 'vector $i');
      }
    });

    test('signatures match node on all vectors', () {
      for (var i = 0; i < vectors.length; i++) {
        final v = vectors[i];
        final (_, sec) = generateKeypair(_unhex(v['seed'] as String));
        final sig = sign(_unhex(v['msg'] as String), sec);
        expect(_hex(sig.bytes), equals(v['sig']), reason: 'vector $i');
      }
    });

    test('node signatures verify in Dart on all vectors', () {
      for (var i = 0; i < vectors.length; i++) {
        final v = vectors[i];
        final pub = Ed25519PublicKey(_unhex(v['pub'] as String));
        final sig = Ed25519Signature(_unhex(v['sig'] as String));
        expect(
          verify(_unhex(v['msg'] as String), sig, pub),
          isTrue,
          reason: 'vector $i',
        );
      }
    });
  });
}
