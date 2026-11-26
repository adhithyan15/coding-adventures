// Differential tests against CPython's hashlib.scrypt (OpenSSL
// provider): 48 seeded vectors over passwords, salts, cost
// parameters, and output lengths. Regenerate with
// `python3 test/generate_scrypt_vectors.py`.
library;

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_scrypt/scrypt.dart';
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
    File('test/scrypt_vectors.json').readAsStringSync(),
  ) as Map<String, dynamic>;
  final vectors = (fixture['vectors'] as List).cast<Map<String, dynamic>>();

  group('hashlib differential', () {
    test('fixture carries 48 diverse seeded vectors', () {
      expect(fixture['seed'], equals(20261130));
      expect(vectors, hasLength(48));
      final dks = <String>{};
      for (final v in vectors) {
        dks.add(v['dk'] as String);
      }
      expect(dks.length, equals(48));
    });

    test('all vectors match hashlib', () {
      for (var i = 0; i < vectors.length; i++) {
        final v = vectors[i];
        final got = scrypt(
          _unhex(v['password'] as String),
          _unhex(v['salt'] as String),
          n: v['n'] as int,
          r: v['r'] as int,
          p: v['p'] as int,
          dkLen: v['dk_len'] as int,
        );
        expect(_hex(got), equals(v['dk']), reason: 'vector $i');
      }
    });
  });
}
