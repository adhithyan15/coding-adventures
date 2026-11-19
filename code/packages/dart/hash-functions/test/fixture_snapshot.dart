import 'dart:convert';

import '../.dart_tool/hash_functions_fixture_snapshot.dart';
import '../tool/snapshot_codec.dart';

List<int> loadFixtureSnapshotBytes() =>
    decodeFixtureSnapshot(fixtureJsonBase64Chunks);

Map<String, dynamic> loadFixtureDocument() =>
    jsonDecode(utf8.decode(loadFixtureSnapshotBytes())) as Map<String, dynamic>;

int exhaustiveByteCount(Map<String, dynamic> fixture) {
  final lengths =
      (fixture['domain'] as Map<String, dynamic>)['lengths'] as List<dynamic>;
  return lengths.fold<int>(0, (count, rawLength) {
    final length = rawLength as int;
    var values = 1;
    for (var index = 0; index < length; index++) {
      values *= 256;
    }
    return count + values;
  });
}

List<int> exhaustiveInputAt(int index) {
  if (index < 0 || index >= 65793) {
    throw RangeError.range(index, 0, 65792, 'index');
  }
  if (index == 0) return const [];
  if (index <= 256) return [index - 1];
  final pairIndex = index - 257;
  return [pairIndex ~/ 256, pairIndex % 256];
}

void validateExhaustiveHexStream(
  String stream, {
  required int count,
  required int width,
}) {
  if (stream.length != count * width) {
    throw const FormatException('Exhaustive stream has the wrong length');
  }
  if (!RegExp(r'^[0-9a-f]+$').hasMatch(stream)) {
    throw const FormatException('Exhaustive stream is not lowercase hex');
  }
}

Iterable<BigInt> exhaustiveExpectedValues(
  String stream, {
  required int count,
  required int width,
}) sync* {
  validateExhaustiveHexStream(stream, count: count, width: width);
  for (var index = 0; index < count; index++) {
    final start = index * width;
    yield BigInt.parse(stream.substring(start, start + width), radix: 16);
  }
}
