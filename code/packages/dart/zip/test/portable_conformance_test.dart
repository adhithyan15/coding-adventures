import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_zip/coding_adventures_zip.dart';
import 'package:test/test.dart';

final Map<String, dynamic> _fixtures = jsonDecode(
  File('../../../specs/fixtures/zip-raw-rfc1951-v1/cases.json')
      .readAsStringSync(),
) as Map<String, dynamic>;

Uint8List _fromHex(String value) {
  final result = Uint8List(value.length ~/ 2);
  for (var i = 0; i < result.length; i++) {
    result[i] = int.parse(value.substring(i * 2, i * 2 + 2), radix: 16);
  }
  return result;
}

Uint8List _materialize(Map<String, dynamic> output) {
  final hex = output['hex'];
  if (hex is String) return _fromHex(hex);
  return Uint8List(output['count'] as int)
    ..fillRange(
      0,
      output['count'] as int,
      int.parse(output['repeat_hex'] as String, radix: 16),
    );
}

void main() {
  group('ZIP raw RFC 1951 v1 language-neutral fixtures', () {
    test('keeps the public hard and default caps at 256 MiB', () {
      expect(rawInflateMaxOutput, 256 * 1024 * 1024);
      expect(defaultMaxOutputBytes, rawInflateMaxOutput);
      expect(_fixtures['limits'], {
        'default_max_output': rawInflateMaxOutput,
        'hard_max_output': rawInflateMaxOutput,
      });
    });

    for (final value in _fixtures['cases'] as List<dynamic>) {
      final fixture = value as Map<String, dynamic>;
      final id = fixture['id'] as String;
      final operation = fixture['operation'] as String;
      final expected = fixture['expected'] as Map<String, dynamic>;

      if (operation == 'inflate') {
        test('inflates $id', () {
          final result = rawInflateCounted(
            _fromHex(fixture['input_hex'] as String),
            maxOutput: fixture['max_output'] as int? ?? defaultMaxOutputBytes,
          );
          expect(
            result.output,
            _materialize(expected['output'] as Map<String, dynamic>),
          );
          expect(result.bytesConsumed, expected['bytes_consumed']);
        });
      } else if (operation == 'inflate-error') {
        test('fails closed for $id', () {
          try {
            rawInflateCounted(
              _fromHex(fixture['input_hex'] as String),
              maxOutput: fixture['max_output'] as int? ?? defaultMaxOutputBytes,
            );
            fail('fixture unexpectedly decoded');
          } on RawInflateError catch (error) {
            expect(error.code, expected['error_id']);
            expect(error.message, isNot(matches(RegExp(r'(?:0x|[0-9]{2,})'))));
          }
        });
      } else if (operation == 'deflate-interoperability') {
        test('foreign-decodes $id', () {
          final compressed = rawDeflate(
            _fromHex(fixture['input_hex'] as String),
          );
          final decoded = ZLibDecoder(raw: true).convert(compressed);
          expect(
            decoded,
            _materialize(expected['output'] as Map<String, dynamic>),
          );
        });
      } else {
        test('checks CRC-32 for $id', () {
          var actual = int.parse(
            fixture['initial_crc32_hex'] as String? ?? '00000000',
            radix: 16,
          );
          for (final chunk in fixture['chunks_hex'] as List<dynamic>) {
            actual = crc32(_fromHex(chunk as String), actual);
          }
          expect(
            actual.toRadixString(16).padLeft(8, '0'),
            expected['crc32_hex'],
          );
        });
      }
    }
  });
}
