@TestOn('vm')
library;

import 'dart:io';

import 'package:test/test.dart';

import 'fixture_snapshot.dart';

void main() {
  test('generated snapshot bytes exactly match the neutral corpus', () {
    final canonical = File(
      '../../../specs/fixtures/hash-functions-v1/cases.json',
    ).readAsBytesSync();
    expect(loadFixtureSnapshotBytes(), canonical);
  });
}
