import 'package:test/test.dart';

import 'fixture_snapshot.dart';

void main() {
  test('Node loads and parses the complete neutral fixture snapshot', () {
    final document = loadFixtureDocument();
    expect(document['schema'], 'dt17-hash-functions-v1');
    expect(document['oracle'], 'dt17-exact-arithmetic-v1');

    final exhaustive = document['exhaustive'] as Map<String, dynamic>;
    final domain = exhaustive['domain'] as Map<String, dynamic>;
    expect(domain['lengths'], [0, 1, 2]);
    expect(domain['order'], 'length-then-lexicographic-bytes');
    expect(domain['count'], 65793);
    expect(exhaustiveByteCount(exhaustive), 65793);

    final streams = exhaustive['expected_hex_streams'] as Map<String, dynamic>;
    const widths = <String, int>{
      'fnv1a_32': 8,
      'fnv1a_64': 16,
      'djb2': 16,
      'polynomial_rolling': 16,
      'polynomial_rolling_with_params': 16,
      'murmur3_32': 8,
      'murmur3_32_with_seed': 8,
      'siphash_2_4': 16,
    };
    expect(streams.keys.toSet(), widths.keys.toSet());

    for (final entry in widths.entries) {
      final stream = streams[entry.key] as String;
      final width = entry.value;
      validateExhaustiveHexStream(stream, count: 65793, width: width);
      var parsedValues = 0;
      for (final value in exhaustiveExpectedValues(
        stream,
        count: 65793,
        width: width,
      )) {
        expect(
            value >= BigInt.zero && value < (BigInt.one << (width * 4)), isTrue,
            reason: '${entry.key}[$parsedValues]');
        parsedValues++;
      }
      expect(parsedValues, 65793, reason: entry.key);
      for (final index in [0, 255, 256, 65792]) {
        final start = index * width;
        final value = stream.substring(start, start + width);
        final parsed = BigInt.parse(value, radix: 16);
        expect(parsed >= BigInt.zero && parsed < (BigInt.one << (width * 4)),
            isTrue,
            reason: '${entry.key}[$index]');
      }
    }

    expect(exhaustiveInputAt(0), isEmpty);
    expect(exhaustiveInputAt(1), [0]);
    expect(exhaustiveInputAt(255), [254]);
    expect(exhaustiveInputAt(256), [255]);
    expect(exhaustiveInputAt(257), [0, 0]);
    expect(exhaustiveInputAt(65792), [255, 255]);

    final sampleStream = streams['fnv1a_32'] as String;
    expect(
      () => validateExhaustiveHexStream(
        sampleStream.substring(0, sampleStream.length - 1),
        count: 65793,
        width: 8,
      ),
      throwsFormatException,
    );
    expect(
      () => validateExhaustiveHexStream(
        'g${sampleStream.substring(1)}',
        count: 65793,
        width: 8,
      ),
      throwsFormatException,
    );
  });
}
