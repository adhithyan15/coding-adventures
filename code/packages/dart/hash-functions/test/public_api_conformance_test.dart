import 'package:coding_adventures_hash_functions/coding_adventures_hash_functions.dart';
import 'package:test/test.dart';

import 'fixture_snapshot.dart';

const _exhaustiveCount = 65793;
const _streamWidths = <String, int>{
  'fnv1a_32': 8,
  'fnv1a_64': 16,
  'djb2': 16,
  'polynomial_rolling': 16,
  'polynomial_rolling_with_params': 16,
  'murmur3_32': 8,
  'murmur3_32_with_seed': 8,
  'siphash_2_4': 16,
};

List<int> _decodeHex(String hex) => [
      for (var offset = 0; offset < hex.length; offset += 2)
        int.parse(hex.substring(offset, offset + 2), radix: 16),
    ];

int _parseInt(Object? value) {
  final text = value.toString();
  if (text.startsWith('0x')) return int.parse(text.substring(2), radix: 16);
  return int.parse(text);
}

BigInt _parseBigInt(Object? value) => BigInt.parse(value.toString());

Map<String, dynamic> _parametersFor(
  Map<String, dynamic> record,
  String operation,
) {
  final raw = record['parameters'];
  if (raw is! Map<String, dynamic>) return const {};
  final nested = raw[operation];
  if (nested is Map<String, dynamic>) return nested;
  return raw;
}

BigInt _hash(String operation, List<int> data,
    [Map<String, dynamic> parameters = const {}]) {
  switch (operation) {
    case 'fnv1a_32':
      return BigInt.from(fnv1a32(data));
    case 'fnv1a_64':
      return fnv1a64(data);
    case 'djb2':
      return djb2(data);
    case 'polynomial_rolling':
      return polynomialRolling(data);
    case 'polynomial_rolling_with_params':
      return polynomialRollingWithParams(
        data,
        base: _parseBigInt(parameters['base']),
        modulus: _parseBigInt(parameters['modulus']),
      );
    case 'murmur3_32':
      return BigInt.from(murmur3_32(data));
    case 'murmur3_32_with_seed':
      return BigInt.from(murmur3_32(data, seed: _parseInt(parameters['seed'])));
    case 'siphash_2_4':
      return siphash2_4(
        data,
        _decodeHex(parameters['key_hex'] as String),
      );
    default:
      throw StateError('Unknown DT17 operation: $operation');
  }
}

void _checkHashRecord(Map<String, dynamic> record) {
  final data = _decodeHex(record['data_hex'] as String);
  final expected = record['expected_hex'];
  if (expected is String) {
    final operation = record['op'] as String;
    final parameters = _parametersFor(record, operation);
    expect(
      _hash(operation, data, parameters),
      BigInt.parse(expected, radix: 16),
      reason: '${record['id']}/$operation',
    );
    return;
  }
  final expectedOutputs = expected as Map<String, dynamic>;
  for (final entry in expectedOutputs.entries) {
    final parameters = _parametersFor(record, entry.key);
    expect(
      _hash(entry.key, data, parameters),
      BigInt.parse(entry.value as String, radix: 16),
      reason: '${record['id']}/${entry.key}',
    );
  }
}

void _checkPublicOutput(
  String operation,
  String stream,
  int index,
  int width,
  BigInt actual,
) {
  final start = index * width;
  final expected =
      BigInt.parse(stream.substring(start, start + width), radix: 16);
  expect(actual, expected, reason: '$operation[$index]');
}

void main() {
  final fixture = loadFixtureDocument();

  test('public hash APIs match all 526344 exhaustive outputs', () {
    final exhaustive = fixture['exhaustive'] as Map<String, dynamic>;
    final count =
        (exhaustive['domain'] as Map<String, dynamic>)['count'] as int;
    expect(count, _exhaustiveCount);
    final streams = exhaustive['expected_hex_streams'] as Map<String, dynamic>;
    final fixed = exhaustive['fixed_parameters'] as Map<String, dynamic>;

    for (final entry in _streamWidths.entries) {
      final operation = entry.key;
      final width = entry.value;
      final stream = streams[operation] as String;
      validateExhaustiveHexStream(stream, count: count, width: width);
      for (var index = 0; index < count; index++) {
        final input = exhaustiveInputAt(index);
        final parameters = switch (operation) {
          'polynomial_rolling_with_params' => {
              'base': fixed['polynomial_base'],
              'modulus': fixed['polynomial_modulus'],
            },
          'murmur3_32_with_seed' => {
              'seed': fixed['murmur3_32_with_seed'],
            },
          'siphash_2_4' => {
              'key_hex': fixed['siphash_2_4_key_hex'],
            },
          _ => const <String, dynamic>{},
        };
        _checkPublicOutput(
          operation,
          stream,
          index,
          width,
          _hash(operation, input, parameters),
        );
      }
    }
  });

  test(
      'the public comparison path rejects valid-hex corruption and wrong dispatch',
      () {
    final exhaustive = fixture['exhaustive'] as Map<String, dynamic>;
    final streams = exhaustive['expected_hex_streams'] as Map<String, dynamic>;
    final fnvStream = streams['fnv1a_32'] as String;
    final replacement = fnvStream[0] == '0' ? '1' : '0';
    final corrupted = '$replacement${fnvStream.substring(1)}';
    expect(
        () => validateExhaustiveHexStream(
              corrupted,
              count: _exhaustiveCount,
              width: 8,
            ),
        returnsNormally);
    expect(
      () => _checkPublicOutput(
          'fnv1a_32', corrupted, 0, 8, BigInt.from(fnv1a32(const []))),
      throwsA(isA<TestFailure>()),
    );

    final polynomialStream =
        streams['polynomial_rolling_with_params'] as String;
    const input = [1, 0];
    const index = 513;
    final fixed = exhaustive['fixed_parameters'] as Map<String, dynamic>;
    final expectedDispatch = _hash('polynomial_rolling_with_params', input, {
      'base': fixed['polynomial_base'],
      'modulus': fixed['polynomial_modulus'],
    });
    _checkPublicOutput('polynomial_rolling_with_params', polynomialStream,
        index, 16, expectedDispatch);
    expect(
      () => _checkPublicOutput(
        'polynomial_rolling_with_params',
        polynomialStream,
        index,
        16,
        polynomialRolling(input),
      ),
      throwsA(isA<TestFailure>()),
    );
  });

  test('literal, directed, generated and byte-mutation cases call public APIs',
      () {
    for (final section in [
      'known_answers',
      'directed',
      'generated',
    ]) {
      final raw = fixture[section];
      final records = raw is Map<String, dynamic>
          ? raw['cases'] as List<dynamic>
          : raw as List<dynamic>;
      for (final record in records.cast<Map<String, dynamic>>()) {
        _checkHashRecord(record);
      }
    }
    final mutationPairs = (fixture['mutations']
        as Map<String, dynamic>)['cases'] as List<dynamic>;
    for (final pair in mutationPairs.cast<Map<String, dynamic>>()) {
      _checkHashRecord(pair['original'] as Map<String, dynamic>);
      _checkHashRecord(pair['mutated'] as Map<String, dynamic>);
    }
  });

  test('DJB2 64-bit wrapping matches the shared long input vector', () {
    final directed =
        (fixture['directed'] as List<dynamic>).cast<Map<String, dynamic>>();
    _checkHashRecord(
      directed.singleWhere((record) => record['id'] == 'boundary-length-17'),
    );
  });

  test('Murmur3 block and tail match the shared five-byte vector', () {
    final directed =
        (fixture['directed'] as List<dynamic>).cast<Map<String, dynamic>>();
    _checkHashRecord(
      directed.singleWhere((record) => record['id'] == 'boundary-length-5'),
    );
  });

  test('UTF-8 convenience functions match the neutral UTF-8 cases', () {
    final cases = fixture['utf8_cases'] as List<dynamic>;
    for (final raw in cases.cast<Map<String, dynamic>>()) {
      final operation = raw['op'] as String;
      final text = raw['text'] as String;
      final parameters = _parametersFor(raw, operation);
      final BigInt actual = switch (operation) {
        'hash_str_fnv1a_32' => BigInt.from(hashStringFnv1a32(text)),
        'hash_str_siphash' => hashStringSiphash(
            text,
            _decodeHex(parameters['key_hex'] as String),
          ),
        _ => throw StateError('Unknown string operation: $operation'),
      };
      expect(
        actual,
        BigInt.parse(raw['expected_hex'] as String, radix: 16),
        reason: raw['id'] as String,
      );
    }
  });

  test('all six public strategy wrappers forward and report output width', () {
    final data = [0, 1, 0xff, 0x80, 0];
    const key = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    final wrappers = <(String, dynamic, int, BigInt)>[
      ('FNV-1a 32', Fnv1a32(), 32, BigInt.from(fnv1a32(data))),
      ('FNV-1a 64', Fnv1a64(), 64, fnv1a64(data)),
      ('DJB2', Djb2(), 64, djb2(data)),
      ('Murmur3 32', Murmur3_32(), 32, BigInt.from(murmur3_32(data))),
      ('polynomial default', PolynomialRolling(), 64, polynomialRolling(data)),
      ('SipHash-2-4', SipHash24(key), 64, siphash2_4(data, key)),
    ];
    for (final (label, wrapper, width, expected) in wrappers) {
      expect(wrapper.outputBits, width, reason: label);
      expect(BigInt.parse(wrapper.hash(data).toString()), expected,
          reason: label);
    }

    final parameterized = PolynomialRolling.withParams(
      base: BigInt.from(257),
      modulus: BigInt.parse('2305843009213693951'),
    );
    expect(parameterized.outputBits, 64);
    expect(
      parameterized.hash(data),
      polynomialRollingWithParams(
        data,
        base: BigInt.from(257),
        modulus: BigInt.parse('2305843009213693951'),
      ),
    );
  });

  test('public functions and wrappers preserve int versus BigInt result types',
      () {
    const data = [0, 1, 0x80, 0xff];
    const key = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    expect(fnv1a32(data), isA<int>());
    expect(murmur3_32(data), isA<int>());
    expect(fnv1a64(data), isA<BigInt>());
    expect(djb2(data), isA<BigInt>());
    expect(polynomialRolling(data), isA<BigInt>());
    expect(
        polynomialRollingWithParams(
          data,
          base: BigInt.from(257),
          modulus: BigInt.parse('2305843009213693951'),
        ),
        isA<BigInt>());
    expect(siphash2_4(data, key), isA<BigInt>());
    expect(Fnv1a32().hash(data), isA<int>());
    expect(Murmur3_32().hash(data), isA<int>());
    expect(Fnv1a64().hash(data), isA<BigInt>());
    expect(Djb2().hash(data), isA<BigInt>());
    expect(PolynomialRolling().hash(data), isA<BigInt>());
    expect(SipHash24(key).hash(data), isA<BigInt>());
  });

  test('SipHash wrappers snapshot caller key ownership', () {
    final callerKey = List<int>.generate(16, (index) => index);
    final originalKey = List<int>.of(callerKey);
    final wrapper = SipHash24(callerKey);
    final data = [0, 0xff, 0x80, 3];
    final expected = siphash2_4(data, originalKey);
    callerKey[0] ^= 0xff;
    expect(wrapper.hash(data), expected);
  });

  test('public validation covers analysis, wrapper and surrogate branches', () {
    for (final invalidString in [
      String.fromCharCodes([0xd800, 0x41]),
      String.fromCharCodes([0xdc00]),
    ]) {
      expect(() => hashStringFnv1a32(invalidString), throwsArgumentError);
    }
    expect(
      () => avalancheScore(
        hashFn: (_) => BigInt.zero,
        outputBits: 1,
        samples: const [[]],
      ),
      throwsArgumentError,
    );
    expect(
      () => avalancheScore(
        hashFn: (_) => BigInt.zero,
        outputBits: 1,
        samples: const [
          [-1]
        ],
      ),
      throwsArgumentError,
    );
    expect(
      () => distributionTest(
        hashFn: (_) => BigInt.zero,
        inputs: const [
          [256]
        ],
        numBuckets: 2,
      ),
      throwsArgumentError,
    );
    expect(() => Murmur3_32(seed: -1), throwsArgumentError);
    expect(() => SipHash24(List<int>.filled(15, 0)), throwsArgumentError);
    expect(
      () => SipHash24([...List<int>.filled(15, 0), 256]),
      throwsArgumentError,
    );
  });

  test('analysis helpers match exact fixture counts and rational answers', () {
    final cases = fixture['analysis'] as List<dynamic>;
    for (final raw in cases.cast<Map<String, dynamic>>()) {
      final callback = raw['callback'] as Map<String, dynamic>;
      final BigInt Function(List<int>) hashFn = switch (callback['kind']) {
        'synthetic' => (data) => BigInt.from(data.first),
        'hash' when callback['op'] == 'fnv1a_32' => (data) =>
            BigInt.from(fnv1a32(data)),
        _ => throw StateError('Unknown analysis callback: $callback'),
      };
      final expected = raw['expected'] as Map<String, dynamic>;
      if (raw['kind'] == 'avalanche') {
        final samples = (raw['samples_hex'] as List<dynamic>)
            .cast<String>()
            .map(_decodeHex)
            .toList();
        final outputBits = raw['output_bits'] as int;
        final (flipped, compared) = _exactAvalancheCounts(
          hashFn,
          samples,
          outputBits,
        );
        expect(flipped.toString(), expected['flipped_bits'],
            reason: raw['id'] as String);
        expect(compared.toString(), expected['compared_bits'],
            reason: raw['id'] as String);
        expect(
          avalancheScore(
              hashFn: hashFn, outputBits: outputBits, samples: samples),
          flipped / compared,
          reason: raw['id'] as String,
        );
      } else {
        final inputs = (raw['inputs_hex'] as List<dynamic>)
            .cast<String>()
            .map(_decodeHex)
            .toList();
        final bucketCount = raw['bucket_count'] as int;
        final (buckets, numerator, denominator) = _exactDistribution(
          hashFn,
          inputs,
          bucketCount,
        );
        expect(buckets, expected['bucket_counts'], reason: raw['id'] as String);
        expect(numerator.toString(), expected['chi_squared_numerator'],
            reason: raw['id'] as String);
        expect(denominator.toString(), expected['chi_squared_denominator'],
            reason: raw['id'] as String);
        expect(
          distributionTest(
              hashFn: hashFn, inputs: inputs, numBuckets: bucketCount),
          numerator / denominator,
          reason: raw['id'] as String,
        );
      }
    }
  });

  test(
      'distributionTest uses sparse exact buckets through the portable int limit',
      () {
    const maxPortableBuckets = 9007199254740991;
    final maxBuckets = BigInt.from(maxPortableBuckets);
    final oneInput = distributionTest(
      hashFn: (_) => BigInt.zero,
      inputs: const [
        [0xff]
      ],
      numBuckets: maxPortableBuckets,
    );
    expect(oneInput, BigInt.from(maxPortableBuckets - 1).toDouble());

    final twoIdentical = distributionTest(
      hashFn: (_) => BigInt.zero,
      inputs: const [
        [],
        [0, 1]
      ],
      numBuckets: maxPortableBuckets,
    );
    expect(twoIdentical,
        (maxBuckets * BigInt.from(2) - BigInt.from(2)).toDouble());

    // This exactly representable BigInt is rounded when converted to a Node
    // number. Its correct bucket is 1; unsafe int arithmetic aliases it with
    // bucket 2 and changes the observed statistic.
    final nearPrecisionLimit = distributionTest(
      hashFn: (data) =>
          data.isEmpty ? BigInt.from(2) : BigInt.parse('18014398509481983'),
      inputs: const [
        [],
        [1]
      ],
      numBuckets: maxPortableBuckets,
    );
    expect(nearPrecisionLimit, BigInt.from(maxPortableBuckets - 2).toDouble());

    const smallInputs = [
      [0],
      [1],
      [2]
    ];
    final smallCallback =
        (List<int> data) => BigInt.from(data.single == 0 ? 0 : 4);
    final (counts, numerator, denominator) =
        _exactDistribution(smallCallback, smallInputs, 5);
    expect(counts, [1, 0, 0, 0, 2]);
    expect(
      distributionTest(
        hashFn: smallCallback,
        inputs: smallInputs,
        numBuckets: 5,
      ),
      numerator / denominator,
    );

    for (final invalid in [-1, 0, maxPortableBuckets + 1]) {
      expect(
        () => distributionTest(
          hashFn: (_) => BigInt.zero,
          inputs: const [[]],
          numBuckets: invalid,
        ),
        throwsArgumentError,
        reason: 'numBuckets=$invalid',
      );
    }
  });

  for (final raw in (fixture['dart_only_invalid'] as List<dynamic>)
      .cast<Map<String, dynamic>>()) {
    test('ArgumentError: ${raw['id']}', () {
      if (raw['id'] == 'string-unpaired-surrogate') {
        final invalidString = String.fromCharCodes(
          (raw['code_points'] as List<dynamic>).cast<int>(),
        );
        expect(() => hashStringFnv1a32(invalidString), throwsArgumentError);
      } else {
        expect(() => _invokeInvalidCase(raw), throwsArgumentError);
      }
    });
  }
}

(int, int) _exactAvalancheCounts(
  BigInt Function(List<int>) hashFn,
  List<List<int>> samples,
  int outputBits,
) {
  final mask = (BigInt.one << outputBits) - BigInt.one;
  var flipped = 0;
  var compared = 0;
  for (final sample in samples) {
    final original = hashFn(sample);
    if (original.isNegative || original > mask) {
      throw StateError('analysis fixture callback output is outside width');
    }
    for (var bit = 0; bit < sample.length * 8; bit++) {
      final changed = List<int>.of(sample);
      changed[bit ~/ 8] ^= 1 << (bit % 8);
      final result = hashFn(changed);
      if (result.isNegative || result > mask) {
        throw StateError('analysis fixture callback output is outside width');
      }
      flipped += (original ^ result).bitLength == 0
          ? 0
          : _populationCount(original ^ result);
      compared += outputBits;
    }
  }
  return (flipped, compared);
}

int _populationCount(BigInt value) {
  var count = 0;
  var current = value;
  while (current > BigInt.zero) {
    current &= current - BigInt.one;
    count++;
  }
  return count;
}

(List<int>, BigInt, BigInt) _exactDistribution(
  BigInt Function(List<int>) hashFn,
  List<List<int>> inputs,
  int bucketCount,
) {
  final buckets = List<int>.filled(bucketCount, 0);
  for (final input in inputs) {
    final result = hashFn(input);
    if (result.isNegative || result > ((BigInt.one << 64) - BigInt.one)) {
      throw StateError('distribution fixture callback output is outside u64');
    }
    final bucket = (result % BigInt.from(bucketCount)).toInt();
    buckets[bucket]++;
  }
  final total = BigInt.from(inputs.length);
  final k = BigInt.from(bucketCount);
  var numerator = BigInt.zero;
  for (final count in buckets) {
    final difference = k * BigInt.from(count) - total;
    numerator += difference * difference;
  }
  var denominator = k * total;
  final divisor = _gcd(numerator, denominator);
  numerator ~/= divisor;
  denominator ~/= divisor;
  return (buckets, numerator, denominator);
}

BigInt _gcd(BigInt left, BigInt right) {
  var a = left;
  var b = right;
  while (b != BigInt.zero) {
    final next = a % b;
    a = b;
    b = next;
  }
  return a == BigInt.zero ? BigInt.one : a;
}

void _invokeInvalidCase(Map<String, dynamic> item) {
  final operation = item['op'] as String;
  switch (operation) {
    case 'fnv1a_32':
      fnv1a32((item['data'] as List<dynamic>).cast<int>());
      break;
    case 'murmur3_32_with_seed':
      murmur3_32(const [], seed: _parseInt(item['seed']));
      break;
    case 'polynomial_rolling_with_params':
      polynomialRollingWithParams(
        const [],
        base: _parseBigInt(item['base']),
        modulus: _parseBigInt(item['modulus']),
      );
      break;
    case 'siphash_2_4':
      siphash2_4(const [], _decodeHex(item['key_hex'] as String));
      break;
    case 'hash_str_fnv1a_32':
      hashStringFnv1a32(String.fromCharCodes(
        (item['code_points'] as List<dynamic>).cast<int>(),
      ));
      break;
    case 'avalanche':
      final samples = (item['samples_hex'] as List<dynamic>?)
              ?.cast<String>()
              .map(_decodeHex)
              .toList() ??
          const <List<int>>[];
      final result = item['callback_result'] == null
          ? BigInt.zero
          : _parseBigInt(item['callback_result']);
      avalancheScore(
        hashFn: (_) => result,
        outputBits: item['output_bits'] as int? ?? 32,
        samples: samples,
      );
      break;
    case 'distribution':
      final inputs = (item['inputs_hex'] as List<dynamic>?)
              ?.cast<String>()
              .map(_decodeHex)
              .toList() ??
          const <List<int>>[];
      final result = item['callback_result'] == null
          ? BigInt.zero
          : _parseBigInt(item['callback_result']);
      distributionTest(
        hashFn: (_) => result,
        inputs: inputs,
        numBuckets: item['bucket_count'] as int,
      );
      break;
    default:
      throw StateError('Unsupported invalid API fixture: $operation');
  }
}
