// Differential tests against Node.js built-in AES (OpenSSL
// provider): 120 seeded vectors (30 per mode) over keys, IVs,
// nonces, AAD, and boundary-heavy plaintext lengths. Regenerate
// with `node test/generate_node_vectors.mjs`.
library;

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_aes_modes/aes_modes.dart';
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

Uint8List? _maybeUnhex(Object? hex) =>
    hex == null ? null : _unhex(hex as String);

void main() {
  final fixture = jsonDecode(
    File('test/node_vectors.json').readAsStringSync(),
  ) as Map<String, dynamic>;
  final vectors = (fixture['vectors'] as List).cast<Map<String, dynamic>>();

  group('node differential', () {
    test('fixture carries 120 diverse seeded vectors', () {
      expect(fixture['seed'], equals(20261130));
      expect(vectors, hasLength(120));
      final triples = <String>{};
      for (final v in vectors) {
        triples.add('${v['key']}${v['ct']}${v['tag']}');
      }
      expect(triples.length, equals(120));
    });

    test('encrypt matches node on all vectors', () {
      for (var i = 0; i < vectors.length; i++) {
        final v = vectors[i];
        final key = _unhex(v['key'] as String);
        final pt = _unhex(v['pt'] as String);
        switch (v['mode'] as String) {
          case 'ecb':
            expect(_hex(ecbEncrypt(pt, key)), equals(v['ct']),
                reason: 'vector $i');
          case 'cbc':
            expect(_hex(cbcEncrypt(pt, key, _unhex(v['iv'] as String))),
                equals(v['ct']),
                reason: 'vector $i');
          case 'ctr':
            expect(_hex(ctrEncrypt(pt, key, _unhex(v['iv'] as String))),
                equals(v['ct']),
                reason: 'vector $i');
          case 'gcm':
            final (ct, tag) = gcmEncrypt(pt, key, _unhex(v['iv'] as String),
                _maybeUnhex(v['aad']) ?? Uint8List(0));
            expect(_hex(ct), equals(v['ct']), reason: 'vector $i ct');
            expect(_hex(tag), equals(v['tag']), reason: 'vector $i tag');
          default:
            fail('unknown mode ${v['mode']}');
        }
      }
    });

    test('decrypt matches node on all vectors', () {
      for (var i = 0; i < vectors.length; i++) {
        final v = vectors[i];
        final key = _unhex(v['key'] as String);
        final ct = _unhex(v['ct'] as String);
        switch (v['mode'] as String) {
          case 'ecb':
            expect(_hex(ecbDecrypt(ct, key)), equals(v['pt']),
                reason: 'vector $i');
          case 'cbc':
            expect(_hex(cbcDecrypt(ct, key, _unhex(v['iv'] as String))),
                equals(v['pt']),
                reason: 'vector $i');
          case 'ctr':
            expect(_hex(ctrDecrypt(ct, key, _unhex(v['iv'] as String))),
                equals(v['pt']),
                reason: 'vector $i');
          case 'gcm':
            expect(
                _hex(gcmDecrypt(
                    ct,
                    key,
                    _unhex(v['iv'] as String),
                    _maybeUnhex(v['aad']) ?? Uint8List(0),
                    _unhex(v['tag'] as String))),
                equals(v['pt']),
                reason: 'vector $i');
          default:
            fail('unknown mode ${v['mode']}');
        }
      }
    });
  });
}
