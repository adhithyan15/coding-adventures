import 'dart:io';

import 'snapshot_codec.dart';

const _defaultInput = '../../../specs/fixtures/hash-functions-v1/cases.json';
const _defaultOutput = '.dart_tool/hash_functions_fixture_snapshot.dart';

/// Writes a deterministic, Node-loadable Dart constant snapshot of [input].
void generateFixtureSnapshot({
  required File input,
  required File output,
}) {
  final bytes = input.readAsBytesSync();
  output.parent.createSync(recursive: true);
  output.writeAsStringSync(renderFixtureSnapshotSource(bytes));
}

void main(List<String> arguments) {
  if (arguments.length > 2) {
    stderr.writeln(
      'Usage: dart run tool/generate_fixture_snapshot.dart [input [output]]',
    );
    exitCode = 64;
    return;
  }

  final input = File(arguments.isEmpty ? _defaultInput : arguments[0]);
  final output = File(arguments.length < 2 ? _defaultOutput : arguments[1]);
  generateFixtureSnapshot(input: input, output: output);
}
