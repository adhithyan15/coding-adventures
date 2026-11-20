@TestOn('vm')
library;

import 'dart:convert';
import 'dart:io';

import 'package:test/test.dart';

import '../tool/generate_fixture_snapshot.dart';
import '../tool/snapshot_codec.dart';

void main() {
  test('generator preserves arbitrary fixture bytes exactly', () {
    final directory = Directory.systemTemp.createTempSync('dt17-snapshot-');
    try {
      final input =
          File('${directory.path}${Platform.pathSeparator}input.json');
      final output =
          File('${directory.path}${Platform.pathSeparator}snapshot.dart');
      const bytes = [0x7b, 0xff, 0x00, 0x0d, 0x0a, 0x7d];
      input.writeAsBytesSync(bytes);

      generateFixtureSnapshot(input: input, output: output);
      final generated = output.readAsStringSync();
      final chunks = RegExp("'([A-Za-z0-9+/=]*)'")
          .allMatches(generated)
          .map((match) => match.group(1)!)
          .toList();

      expect(utf8.encode(generated).length, greaterThan(bytes.length));
      expect(decodeFixtureSnapshot(chunks), bytes);
    } finally {
      directory.deleteSync(recursive: true);
    }
  });
}
