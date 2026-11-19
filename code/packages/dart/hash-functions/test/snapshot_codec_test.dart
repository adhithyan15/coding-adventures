import 'dart:convert';

import 'package:test/test.dart';

import '../tool/snapshot_codec.dart';

void main() {
  test('base64 snapshot round-trips every byte value', () {
    final bytes = List<int>.generate(256, (value) => value);
    final source = renderFixtureSnapshotSource(bytes, chunkSize: 31);
    final chunks = RegExp("'([A-Za-z0-9+/=]*)'")
        .allMatches(source)
        .map((match) => match.group(1)!)
        .toList();

    expect(chunks, isNotEmpty);
    expect(base64Decode(chunks.join()), bytes);
    expect(decodeFixtureSnapshot(chunks), bytes);
  });

  test('snapshot source and chunk boundaries are deterministic', () {
    const bytes = [0, 0, 255, 1, 2, 3, 4, 5, 6];

    expect(
      renderFixtureSnapshotSource(bytes, chunkSize: 4),
      renderFixtureSnapshotSource(bytes, chunkSize: 4),
    );
    expect(
      renderFixtureSnapshotSource(bytes, chunkSize: 4),
      contains("'AAD/'"),
    );
  });

  test('snapshot generator rejects invalid chunk sizes', () {
    expect(
      () => renderFixtureSnapshotSource(const [], chunkSize: 0),
      throwsArgumentError,
    );
  });
}
