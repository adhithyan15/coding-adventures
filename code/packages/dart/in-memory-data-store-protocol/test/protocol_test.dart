// Tests for coding_adventures_in_memory_data_store_protocol.
//
// Frames (construction, uppercasing, copies, equality), part conversion
// both directions, RESP conversions for every scalar kind plus the null
// and non-scalar rejections, text parse/format round-trips, and a seeded
// sweep proving wire bytes survive frame conversion unchanged.

import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';

import 'package:coding_adventures_in_memory_data_store_protocol/coding_adventures_in_memory_data_store_protocol.dart';
import 'package:coding_adventures_resp_protocol/coding_adventures_resp_protocol.dart'
    as resp;
import 'package:test/test.dart';

Uint8List bytes(String text) => Uint8List.fromList(utf8.encode(text));

void main() {
  group('frames', () {
    test('construction uppercases and copies', () {
      final arg = Uint8List.fromList([1, 2, 3]);
      final frame = CommandFrame('  set ', [arg]);
      expect(frame.command, 'SET');
      arg[0] = 9;
      expect(frame.args[0][0], 1);
      expect(frame.toString(), contains('SET'));
    });

    test('fromParts splits command from args', () {
      final frame = CommandFrame.fromParts([
        bytes('get'),
        bytes('k'),
        Uint8List.fromList([0xff])
      ])!;
      expect(frame.command, 'GET');
      expect(frame.args.length, 2);
      expect(frame.args[1], [0xff]);
      expect(CommandFrame.fromParts([]), isNull);
    });

    test('fromParts uppercases ASCII bytes only', () {
      final frame = CommandFrame.fromParts([
        Uint8List.fromList([0x73, 0x65, 0x54, 0x80])
      ])!;
      expect(frame.command.codeUnits.take(3), [0x53, 0x45, 0x54]);
      expect(frame.command.codeUnits[3], 0x80);
    });

    test('equality compares command and bytes', () {
      final a = CommandFrame('SET', [bytes('k')]);
      final b = CommandFrame('set', [bytes('k')]);
      expect(a, b);
      expect(a.hashCode, b.hashCode);
      expect(a == CommandFrame('GET', [bytes('k')]), isFalse);
      expect(a == CommandFrame('SET', [bytes('v')]), isFalse);
      expect(a == CommandFrame('SET', [bytes('k'), bytes('x')]), isFalse);
      expect(a == 'not a frame', isFalse);
    });
  });

  group('RESP conversion', () {
    test('commandToResp builds bulk arrays', () {
      final frame = CommandFrame('PING', []);
      expect(
        commandToResp(frame),
        resp.array([resp.bulkString('PING')]),
      );
      final set = CommandFrame('SET', [bytes('k'), bytes('v')]);
      final wire = resp.encode(commandToResp(set));
      expect(
        utf8.decode(wire),
        '*3\r\n\$3\r\nSET\r\n\$1\r\nk\r\n\$1\r\nv\r\n',
      );
    });

    test('commandFromResp accepts scalar arrays', () {
      final frame = commandFromResp(resp.array([
        resp.bulkString('set'),
        resp.simpleString('k'),
        resp.integer(7),
        resp.errorValue('note'),
      ]))!;
      expect(frame.command, 'SET');
      expect(frame.args.length, 3);
      expect(utf8.decode(frame.args[1]), '7');
    });

    test('commandFromResp rejects nulls, empties, and nesting', () {
      expect(commandFromResp(resp.array(null)), isNull);
      expect(commandFromResp(resp.array([])), isNull);
      expect(commandFromResp(resp.simpleString('OK')), isNull);
      expect(commandFromResp(resp.integer(1)), isNull);
      expect(commandFromResp(resp.array([resp.bulkString(null)])), isNull);
      expect(
          commandFromResp(resp.array([
            resp.array([resp.integer(1)])
          ])),
          isNull);
    });

    test('respValueToBytes renders every scalar kind', () {
      expect(respValueToBytes(resp.simpleString('OK')), bytes('OK'));
      expect(respValueToBytes(resp.errorValue('E')), bytes('E'));
      expect(respValueToBytes(resp.integer(-42)), bytes('-42'));
      expect(respValueToBytes(resp.bulkString(Uint8List.fromList([0, 255]))),
          [0, 255]);
      expect(respValueToBytes(resp.bulkString(null)), isNull);
      expect(respValueToBytes(resp.array([])), isNull);
    });

    test('protocolError builds error values', () {
      expect(protocolError('bad'), resp.errorValue('bad'));
    });

    test('frame RESP round-trip preserves bytes', () {
      final frame = CommandFrame('SET', [
        Uint8List.fromList([0x0d, 0x0a, 0x00, 0xff]),
        bytes('plain'),
      ]);
      final back = commandFromResp(commandToResp(frame))!;
      expect(back, frame);
    });
  });

  group('text helpers', () {
    test('parse and format round-trip', () {
      final frame = parseFrameText(['set', 'k', 'v']);
      expect(frame.command, 'SET');
      expect(formatFrame(frame), 'SET k v');
      expect(() => parseFrameText([]), throwsArgumentError);
    });
  });

  group('seeded sweep', () {
    test('256 frames survive wire conversion', () {
      final rng = Random(0xDA7A);
      for (var i = 0; i < 256; i++) {
        final nameLen = 1 + rng.nextInt(8);
        final name = String.fromCharCodes(
            List.generate(nameLen, (_) => 0x61 + rng.nextInt(26)));
        final argCount = rng.nextInt(5);
        final args = List.generate(argCount, (_) {
          final len = rng.nextInt(16);
          return Uint8List.fromList(
              List.generate(len, (_) => rng.nextInt(256)));
        });
        final frame = CommandFrame(name, args);
        final back = commandFromResp(commandToResp(frame))!;
        expect(back, frame, reason: 'case $i');
      }
    });
  });
}
