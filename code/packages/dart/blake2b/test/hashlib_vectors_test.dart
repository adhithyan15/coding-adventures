// Differential tests against CPython's hashlib.blake2b: 200 seeded
// vectors over boundary-heavy message lengths, digest sizes, and
// key/salt/personal shapes. Regenerate with
// `python3 test/generate_hashlib_vectors.py`.
library;

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_blake2b/blake2b.dart';
import 'package:test/test.dart';

Uint8List _unhex(String hex) {
  final out = Uint8List(hex.length ~/ 2);
  for (var i = 0; i < out.length; i++) {
    out[i] = int.parse(hex.substring(i * 2, i * 2 + 2), radix: 16);
  }
  return out;
}

String _hex(Uint8List bytes) =>
    bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

void main() {
  final fixture = jsonDecode(
    File('test/hashlib_vectors.json').readAsStringSync(),
  ) as Map<String, dynamic>;
  final vectors = (fixture['vectors'] as List).cast<Map<String, dynamic>>();

  group('hashlib differential', () {
    test('fixture carries 200 seeded vectors', () {
      expect(fixture['seed'], equals(20261124));
      expect(vectors, hasLength(200));
    });

    test('all vectors match hashlib', () {
      for (var i = 0; i < vectors.length; i++) {
        final v = vectors[i];
        final msg = _unhex(v['msg'] as String);
        final key = _unhex(v['key'] as String);
        final salt = _unhex(v['salt'] as String);
        final personal = _unhex(v['personal'] as String);
        final got = blake2b(
          msg,
          Blake2bOptions(
            digestSize: v['digest_size'] as int,
            key: key.isEmpty ? null : key,
            salt: salt.isEmpty ? null : salt,
            personal: personal.isEmpty ? null : personal,
          ),
        );
        expect(_hex(got), equals(v['digest']), reason: 'vector $i');
      }
    });

    test('streamed vectors match one-shot', () {
      // Every 7th vector, fed in odd chunks, must equal one-shot.
      for (var i = 0; i < vectors.length; i += 7) {
        final v = vectors[i];
        final msg = _unhex(v['msg'] as String);
        final options = Blake2bOptions(
          digestSize: v['digest_size'] as int,
          key: _unhex(v['key'] as String),
          salt: _unhex(v['salt'] as String),
          personal: _unhex(v['personal'] as String),
        );
        final h = Blake2bHasher(options);
        var offset = 0;
        var chunk = 1;
        while (offset < msg.length) {
          final end = (offset + chunk).clamp(0, msg.length);
          h.update(msg.sublist(offset, end));
          offset = end;
          chunk = (chunk * 3 + 1) % 137 + 1;
        }
        expect(_hex(h.digest()), equals(v['digest']), reason: 'vector $i');
      }
    });
  });
}
