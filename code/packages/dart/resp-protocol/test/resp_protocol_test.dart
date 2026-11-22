// Tests for coding_adventures_resp_protocol (DT23).
//
// Part 1 mirrors the sibling suites: wire encodings with exact bytes and
// consumed counts, command-array round-trips, inline commands, incremental
// buffering, and pipelined streams. Part 2 pins the null forms, nesting,
// binary safety (CRLF bytes inside bulk payloads), and truncation (null,
// never throw, on every prefix). Part 3 is the malformed battery: every
// corrupt shape must throw RespDecodeError, and strict integer parsing must
// reject trailing garbage. Part 4 runs a seeded fuzz round-trip sweep.

import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';

import 'package:coding_adventures_resp_protocol/coding_adventures_resp_protocol.dart';
import 'package:test/test.dart';

String wireOf(Uint8List bytes) => latin1.decode(bytes);

void main() {
  group('reference behavior', () {
    test('encodes and decodes RESP2 values with exact bytes', () {
      final ok = decode(encode(simpleString('OK')))!;
      expect(ok.value, simpleString('OK'));
      expect(ok.consumed, 5);
      expect(wireOf(encode(simpleString('OK'))), '+OK\r\n');

      final err = decode(encode(errorValue('ERR boom')))!;
      expect(err.value, errorValue('ERR boom'));
      expect(err.consumed, 11);
      expect(wireOf(encode(errorValue('ERR boom'))), '-ERR boom\r\n');

      final num = decode(encode(integer(42)))!;
      expect(num.value, integer(42));
      expect(num.consumed, 5);
      expect(wireOf(encode(integer(42))), ':42\r\n');

      final bulk = decode(encode(bulkString('foo')))!;
      expect(bulk.value, bulkString('foo'));
      expect(bulk.consumed, 9);
      expect(wireOf(encode(bulkString('foo'))), '\$3\r\nfoo\r\n');
    });

    test('round-trips command arrays', () {
      final frame = array([
        bulkString('SET'),
        bulkString('counter'),
        bulkString('1'),
      ]);
      final wire = encode(frame);
      final back = decode(wire)!;
      expect(back.value, frame);
      expect(back.consumed, wire.length);
    });

    test('decodes inline commands and blank lines', () {
      final ping = decode('PING\r\n')!;
      expect(ping.value, array([bulkString('PING')]));
      expect(ping.consumed, 6);
      final blank = decode('   \r\n')!;
      expect(blank.value, array([]));
      expect(blank.consumed, 5);
      final multi = decode('SET  key   value\r\n')!;
      expect(
        multi.value,
        array([bulkString('SET'), bulkString('key'), bulkString('value')]),
      );
    });

    test('supports incremental buffering', () {
      final decoder = RespDecoder();
      decoder.feed('*2\r\n\$4\r\nPING\r\n\$5\r\nhello\r\n');
      expect(decoder.hasMessage(), isTrue);
      expect(
        decoder.getMessage(),
        array([bulkString('PING'), bulkString('hello')]),
      );
      expect(decoder.hasMessage(), isFalse);
    });

    test('decodes multiple concatenated messages', () {
      final bytes = encode(simpleString('OK'));
      final combined = Uint8List(bytes.length * 2);
      combined.setRange(0, bytes.length, bytes);
      combined.setRange(bytes.length, combined.length, bytes);
      final result = decodeAll(combined);
      expect(result.values, [simpleString('OK'), simpleString('OK')]);
      expect(result.consumed, combined.length);
    });
  });

  group('nulls, nesting, and binary safety', () {
    test('null bulk strings and arrays round-trip', () {
      expect(wireOf(encode(bulkString(null))), '\$-1\r\n');
      expect(wireOf(encode(array(null))), '*-1\r\n');
      final nullBulk = decode('\$-1\r\n')!;
      expect(nullBulk.value, bulkString(null));
      expect(nullBulk.consumed, 5);
      final nullArray = decode('*-1\r\n')!;
      expect(nullArray.value, array(null));
      expect(nullArray.consumed, 5);
    });

    test('empty bulk strings and arrays are not null', () {
      final emptyBulk = decode(encode(bulkString('')))!;
      expect(emptyBulk.value, bulkString(Uint8List(0)));
      expect((emptyBulk.value as RespBulkString).value, isNotNull);
      final emptyArray = decode(encode(array([])))!;
      expect(emptyArray.value, array([]));
      expect(wireOf(encode(array([]))), '*0\r\n');
    });

    test('nested arrays round-trip', () {
      final nested = array([
        array([integer(1), integer(2)]),
        array([]),
        array(null),
        bulkString('deep'),
      ]);
      final back = decode(encode(nested))!;
      expect(back.value, nested);
      expect(back.consumed, encode(nested).length);
    });

    test('bulk payloads may contain CRLF bytes', () {
      final payload = Uint8List.fromList([0x61, 0x0d, 0x0a, 0x62, 0x00, 0xff]);
      final back = decode(encode(bulkString(payload)))!;
      expect(back.value, bulkString(payload));
    });

    test('negative and extreme integers round-trip', () {
      for (final n in [
        0,
        -1,
        -4223,
        9223372036854775807,
        -9223372036854775808
      ]) {
        final back = decode(encode(integer(n)))!;
        expect(back.value, integer(n));
      }
    });

    test('constructors copy caller bytes', () {
      final buf = Uint8List.fromList([1, 2, 3]);
      final v = bulkString(buf);
      buf[0] = 9;
      expect((v.value as Uint8List)[0], 1);
    });

    test('bulkString rejects mistyped values', () {
      expect(() => bulkString(42), throwsArgumentError);
    });

    test('value equality and strings', () {
      expect(simpleString('a') == simpleString('a'), isTrue);
      expect(simpleString('a') == simpleString('b'), isFalse);
      expect(simpleString('a') == errorValue('a'), isFalse);
      expect(errorValue('a') == errorValue('a'), isTrue);
      expect(integer(1) == integer(1), isTrue);
      expect(integer(1) == integer(2), isFalse);
      expect(bulkString(null) == bulkString(null), isTrue);
      expect(bulkString('a') == bulkString(null), isFalse);
      expect(bulkString(null) == bulkString('a'), isFalse);
      expect(
          bulkString(Uint8List.fromList([1])) ==
              bulkString(Uint8List.fromList([1, 2])),
          isFalse);
      expect(array(null) == array(null), isTrue);
      expect(array([]) == array(null), isFalse);
      expect(array(null) == array([]), isFalse);
      expect(array([integer(1)]) == array([integer(1), integer(2)]), isFalse);
      expect(simpleString('OK').toString(), contains('OK'));
      expect(errorValue('E').toString(), contains('E'));
      expect(integer(7).toString(), contains('7'));
      expect(bulkString('xy').toString(), contains('2 bytes'));
      expect(array([integer(1)]).toString(), contains('1 items'));
      expect(const RespDecodeError('m').toString(), contains('m'));
      expect(const RespEncodeError('m').toString(), contains('m'));
      // hashCode consistency with equality.
      expect(simpleString('a').hashCode, simpleString('a').hashCode);
      expect(bulkString(null).hashCode, bulkString(null).hashCode);
      expect(bulkString(Uint8List.fromList([1, 2])).hashCode,
          bulkString(Uint8List.fromList([1, 2])).hashCode);
      expect(
          bulkString(Uint8List.fromList([1])).hashCode ==
              bulkString(Uint8List.fromList([1, 2])).hashCode,
          isFalse);
      expect(array([integer(1)]).hashCode, array([integer(1)]).hashCode);
    });
  });

  group('truncation returns null, never throws', () {
    test('every prefix of a frame truncates cleanly', () {
      final frame = array([
        bulkString('SET'),
        bulkString('counter'),
        bulkString('1'),
      ]);
      final wire = encode(frame);
      for (var len = 0; len < wire.length; len++) {
        final prefix = Uint8List.fromList(wire.sublist(0, len));
        expect(decode(prefix), isNull, reason: 'prefix length $len');
      }
      // And the full frame decodes.
      expect(decode(wire)!.value, frame);
    });

    test('decodeAll stops at truncation with partial consumed', () {
      final first = encode(simpleString('OK'));
      final partial = Uint8List.fromList([0x24, 0x35, 0x0d]); // "$5\r" cut off
      final combined = Uint8List(first.length + partial.length);
      combined.setRange(0, first.length, first);
      combined.setRange(first.length, combined.length, partial);
      final result = decodeAll(combined);
      expect(result.values, [simpleString('OK')]);
      expect(result.consumed, first.length);
    });

    test('streaming decoder assembles byte by byte', () {
      final wire = encode(array([bulkString('PING'), integer(7)]));
      final decoder = RespDecoder();
      for (var i = 0; i < wire.length - 1; i++) {
        decoder.feed(Uint8List.fromList([wire[i]]));
        expect(decoder.hasMessage(), isFalse, reason: 'byte $i');
      }
      decoder.feed(Uint8List.fromList([wire[wire.length - 1]]));
      expect(decoder.hasMessage(), isTrue);
      expect(decoder.getMessage(), array([bulkString('PING'), integer(7)]));
    });

    test('decoder decodeAll drains the queue', () {
      final decoder = RespDecoder();
      final messages = decoder.decodeAll('+OK\r\n:1\r\n');
      expect(messages, [simpleString('OK'), integer(1)]);
      expect(decoder.hasMessage(), isFalse);
      expect(() => decoder.getMessage(), throwsA(isA<RespDecodeError>()));
    });
  });

  group('malformed battery throws RespDecodeError', () {
    test('bad integers, lengths, and framing', () {
      const bad = [
        ':\r\n', // empty integer
        ':12ab\r\n', // trailing garbage (strict lanes reject)
        ': 12\r\n', // leading space
        ':+12\r\n', // explicit plus
        ':99999999999999999999999\r\n', // out of 64-bit range
        ':\r\n', // empty again via second path
        '\$x\r\n', // non-numeric bulk length
        '\$-2\r\n', // negative bulk length
        '*-2\r\n', // negative array length
        '*x\r\n', // non-numeric array length
        '\$3\r\nfoo', // truncated body (handled below, must be null not throw)
      ];
      for (final input in bad.take(bad.length - 1)) {
        expect(() => decode(input), throwsA(isA<RespDecodeError>()),
            reason: 'input "$input"');
      }
      // Truncation (not malformation) returns null.
      expect(decode(bad.last), isNull);
    });

    test('missing trailing CRLF after bulk body', () {
      expect(() => decode('\$3\r\nfooXX'), throwsA(isA<RespDecodeError>()));
      expect(() => decode('\$3\r\nfoo\rX'), throwsA(isA<RespDecodeError>()));
    });

    test('feed propagates malformation immediately', () {
      final decoder = RespDecoder();
      expect(() => decoder.feed('\$-2\r\n'), throwsA(isA<RespDecodeError>()));
    });
  });

  group('seeded round-trip sweep', () {
    test('512 generated values survive encode/decode', () {
      final rng = Random(0xE59);
      RespValue genValue(int depth) {
        final kind = rng.nextInt(depth > 2 ? 4 : 5);
        switch (kind) {
          case 0:
            return simpleString('s${rng.nextInt(1000)}');
          case 1:
            return errorValue('E${rng.nextInt(100)}');
          case 2:
            return integer(rng.nextInt(1 << 31) * (rng.nextBool() ? 1 : -1));
          case 3:
            if (rng.nextInt(10) == 0) return bulkString(null);
            final len = rng.nextInt(24);
            return bulkString(Uint8List.fromList(
                List.generate(len, (_) => rng.nextInt(256))));
          default:
            if (rng.nextInt(10) == 0) return array(null);
            final len = rng.nextInt(5);
            return array(List.generate(len, (_) => genValue(depth + 1)));
        }
      }

      for (var i = 0; i < 512; i++) {
        final value = genValue(0);
        final wire = encode(value);
        final back = decode(wire)!;
        expect(back.value, value, reason: 'case $i');
        expect(back.consumed, wire.length, reason: 'case $i consumed');
      }
    });
  });
}
