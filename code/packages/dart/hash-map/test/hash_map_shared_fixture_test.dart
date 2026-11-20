import 'dart:collection' show IterableBase, ListBase;
import 'dart:convert';

import 'package:coding_adventures_hash_map/coding_adventures_hash_map.dart';
import 'package:test/test.dart';

import '../.dart_tool/hash_map_fixture_snapshot.dart';

/// Parsed canonical DT18 bytes shared by the VM and compiled Node test runs.
/// The tests never open a file or substitute adapter-generated observations.
final Map<String, dynamic> hashMapFixture = jsonDecode(
  utf8.decode(base64Decode(hashMapFixtureJsonBase64Chunks.join())),
) as Map<String, dynamic>;

String fixtureHex(List<int> bytes) =>
    bytes.map((byte) => byte.toRadixString(16).padLeft(2, '0')).join();

List<int> fixtureBytes(String hex) => [
      for (var index = 0; index < hex.length; index += 2)
        int.parse(hex.substring(index, index + 2), radix: 16),
    ];

HashAlgorithm fixtureAlgorithm(Map<String, dynamic> configuration) {
  switch (configuration['kind']) {
    case 'fnv1a32':
      return const HashAlgorithm.fnv1a32();
    case 'murmur3':
      return HashAlgorithm.murmur3(
        seed: int.parse(configuration['seed_hex'] as String, radix: 16),
      );
    case 'sipHash24':
      return HashAlgorithm.sipHash24(
        key: fixtureBytes(configuration['key_hex'] as String),
      );
    default:
      throw StateError('Unknown fixture hash configuration: $configuration');
  }
}

Map<String, Map<String, dynamic>> get fixtureKeys => {
      for (final row in hashMapFixture['key_cases'] as List<dynamic>)
        (row as Map<String, dynamic>)['id'] as String: row,
    };

Map<String, dynamic> fixtureKeyRow(String id) {
  final row = fixtureKeys[id];
  if (row == null) throw StateError('Unknown fixture key id: $id');
  return row;
}

HashMapKey fixtureKey(String id) {
  final row = fixtureKeyRow(id);
  if (row['kind'] == 'bytes') {
    return HashMapKey.bytes(fixtureBytes(row['payload_hex'] as String));
  }
  if (row['kind'] == 'string') return HashMapKey.string(row['text'] as String);
  throw StateError('Unknown fixture key row: $row');
}

Map<String, Map<String, dynamic>> get fixtureValues => {
      for (final row in hashMapFixture['values'] as List<dynamic>)
        (row as Map<String, dynamic>)['id'] as String: row,
    };

Object? fixtureValue(String id) {
  final row = fixtureValues[id];
  if (row == null) throw StateError('Unknown fixture value id: $id');
  return row['value'];
}

HashMap<Object?> fixtureMap(Map<String, dynamic> configuration) {
  final algorithmId = configuration['hash_configuration_id'];
  final algorithm = algorithmId == null
      ? null
      : fixtureAlgorithm(
          (hashMapFixture['hash_configurations'] as List<dynamic>)
              .cast<Map<String, dynamic>>()
              .singleWhere((row) => row['id'] == algorithmId),
        );
  return HashMap<Object?>(
    capacity: configuration['capacity'] as int? ?? 16,
    strategy: fixtureStrategy(configuration['strategy'] as String?),
    hashAlgorithm: algorithm,
  );
}

CollisionStrategy fixtureStrategy(String? value) => switch (value) {
      null || 'separateChaining' => CollisionStrategy.separateChaining,
      'linearProbing' => CollisionStrategy.linearProbing,
      _ => throw StateError('Unknown collision strategy: $value'),
    };

Map<String, dynamic> fixtureHashConfiguration(String id) =>
    (hashMapFixture['hash_configurations'] as List<dynamic>)
        .cast<Map<String, dynamic>>()
        .singleWhere((row) => row['id'] == id);

void expectFixtureState(
  HashMap<Object?> map,
  Map<String, dynamic> expected, {
  String reason = '',
}) {
  expect(map.count, expected['count'], reason: '$reason count');
  expect(map.capacity, expected['capacity'], reason: '$reason capacity');
  final ratio = (expected['load_factor_ratio'] as List<dynamic>).cast<int>();
  expect(map.loadFactor, ratio[0] / ratio[1], reason: '$reason load factor');
  final entries = expected['entries'] as List<dynamic>;
  expect(map.entries, hasLength(entries.length), reason: '$reason entry count');
  for (var index = 0; index < entries.length; index++) {
    final wanted = entries[index] as Map<String, dynamic>;
    final actual = map.entries[index];
    expect(
      fixtureHex(actual.key.encodedBytes),
      fixtureKeyRow(wanted['key_id'] as String)['encoded_hex'],
      reason: '$reason entry $index key/order',
    );
    expect(actual.value, wanted['value'], reason: '$reason entry $index value');
  }
}

void expectFixtureError(Object error, Map<String, dynamic> expected) {
  final id = expected['id'] as String;
  final fields = expected['fields'] as Map<String, dynamic>;
  switch (id) {
    case 'invalid_capacity':
      expect(error, isA<InvalidCapacity>());
      expect((error as InvalidCapacity).capacity, fields['capacity']);
    case 'capacity_limit_exceeded':
      expect(error, isA<CapacityLimitExceeded>());
    case 'invalid_sip_hash_key_length':
      expect(error, isA<InvalidSipHashKeyLength>());
      expect(
        (error as InvalidSipHashKeyLength).actualLength,
        fields['actual_length'],
      );
    case 'invalid_murmur_seed':
      expect(error, isA<InvalidMurmurSeed>());
      expect((error as InvalidMurmurSeed).seed, fields['seed']);
    case 'invalid_byte':
      expect(error, isA<InvalidByte>());
      expect((error as InvalidByte).index, fields['index']);
      expect(error.value, fields['value']);
    case 'malformed_utf16':
      expect(error, isA<MalformedUtf16>());
      expect(
          (error as MalformedUtf16).codeUnitOffset, fields['code_unit_offset']);
    case 'key_too_long':
      expect(error, isA<KeyTooLong>());
      expect((error as KeyTooLong).actual, fields['actual']);
      expect(error.maximum, fields['maximum']);
    default:
      fail('Unknown expected error id: $id');
  }
}

void expectPublicHashAnswer(
  HashAlgorithm algorithm,
  List<int> bytes,
  String expectedHex, {
  String reason = '',
}) {
  final actualHex = algorithm
      .validatedHash(bytes)
      .toRadixString(16)
      .padLeft(expectedHex.length, '0');
  expect(actualHex, expectedHex, reason: reason);
}

void main() {
  test('fixture snapshot preserves the canonical JSON bytes', () {
    final bytes = base64Decode(hashMapFixtureJsonBase64Chunks.join());
    expect(bytes, hasLength(469237));
    expect(
      _sha256Hex(const [0x61, 0x62, 0x63]),
      'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad',
      reason: 'SHA-256 implementation known answer',
    );
    expect(
      _sha256Hex(bytes),
      '4b8e2ebaa80e50c0d680d1f1c7ef7f8eb96944a46356dc30db31be4e2daee86a',
      reason:
          'decoded generated snapshot equals frozen canonical fixture bytes',
    );
    expect(hashMapFixture['schema'], 'dt18-hash-map-v1');
    expect(hashMapFixture['oracle'], 'dt18-association-list-v1');
    expect((hashMapFixture['dart_only_cases'] as List<dynamic>), hasLength(9));
  });

  test('public hash algorithms match every shared fixture answer', () {
    for (final row in hashMapFixture['hash_answers'] as List<dynamic>) {
      final answer = row as Map<String, dynamic>;
      final configuration = fixtureHashConfiguration(
        answer['configuration_id'] as String,
      );
      expectPublicHashAnswer(
        fixtureAlgorithm(configuration),
        fixtureBytes(answer['data_hex'] as String),
        answer['expected_hex'] as String,
        reason: answer['id'] as String,
      );
    }
    for (final row in hashMapFixture['encoded_key_hashes'] as List<dynamic>) {
      final answer = row as Map<String, dynamic>;
      final configuration = fixtureHashConfiguration(
        answer['configuration_id'] as String,
      );
      final key = fixtureKey(answer['key_id'] as String);
      expect(fixtureHex(key.encodedBytes), answer['input_hex']);
      expectPublicHashAnswer(
        fixtureAlgorithm(configuration),
        key.encodedBytes,
        answer['expected_hex'] as String,
        reason: answer['id'] as String,
      );
    }
  });

  test('public configurations preserve defaults, parameters, and equality', () {
    for (final row in hashMapFixture['configuration_cases'] as List<dynamic>) {
      final item = row as Map<String, dynamic>;
      final arguments = item['arguments'] as Map<String, dynamic>;
      final expected = item['expected'] as Map<String, dynamic>;
      final map = fixtureMap(arguments);
      expect(map.capacity, expected['capacity'], reason: item['id'] as String);
      expect(map.strategy.name, expected['strategy']);
      final hashRow = fixtureHashConfiguration(
        expected['hash_configuration_id'] as String,
      );
      expect(
        map.hashAlgorithm,
        equals(fixtureAlgorithm(hashRow)),
        reason: '${item['id']} hash parameters',
      );
      expect(map.configuration.capacity, map.capacity);
      expect(map.configuration.strategy, map.strategy);
      expect(map.configuration.hashAlgorithm, equals(map.hashAlgorithm));
    }
    expect(
      HashMapLimits.deterministicSipHashKey,
      fixtureBytes('636f6465782d647431382d6b65792121'),
    );
  });

  test(
      'public key, algorithm, and configuration equality/hashCode are coherent',
      () {
    final fnv = const HashAlgorithm.fnv1a32();
    final sameFnv = const HashAlgorithm.fnv1a32();
    final murmur = HashAlgorithm.murmur3(seed: 7);
    final sameMurmur = HashAlgorithm.murmur3(seed: 7);
    final otherMurmur = HashAlgorithm.murmur3(seed: 8);
    final sipKey = List<int>.generate(16, (index) => index);
    final sip = HashAlgorithm.sipHash24(key: sipKey);
    final sameSip = HashAlgorithm.sipHash24(key: List<int>.of(sipKey));
    final changedSipKey = List<int>.of(sipKey)..[15] = 14;
    final changedSip = HashAlgorithm.sipHash24(key: changedSipKey);
    final shortSip = HashAlgorithm.sipHash24(key: sipKey.sublist(0, 15));

    for (final pair in [
      (fnv, sameFnv),
      (murmur, sameMurmur),
      (sip, sameSip),
    ]) {
      expect(pair.$1, pair.$2);
      expect(pair.$1.hashCode, pair.$2.hashCode);
    }
    expect(fnv, isNot(murmur));
    expect(murmur, isNot(otherMurmur));
    expect(sip, isNot(changedSip));
    expect(sip, isNot(shortSip));
    expect(murmur.hashCode, isNot(otherMurmur.hashCode));

    final key = HashMapKey.bytes([1, 2, 3]);
    final sameKey = HashMapKey.bytes([1, 2, 3]);
    final changedKey = HashMapKey.bytes([1, 2, 4]);
    final shortKey = HashMapKey.bytes([1, 2]);
    final stringKey = HashMapKey.string('\u0001\u0002\u0003');
    expect(key, key); // identity path
    expect(key, sameKey);
    expect(key.hashCode, sameKey.hashCode);
    expect(key, isNot(changedKey));
    expect(key, isNot(shortKey));
    expect(key, isNot(stringKey)); // equal payload, different public tag

    HashMapConfiguration config({
      int capacity = 7,
      CollisionStrategy strategy = CollisionStrategy.separateChaining,
      HashAlgorithm? algorithm,
    }) =>
        HashMap<Object?>(
          capacity: capacity,
          strategy: strategy,
          hashAlgorithm: algorithm ?? fnv,
        ).configuration;

    final configuration = config();
    final sameConfiguration = config();
    expect(configuration, sameConfiguration);
    expect(configuration.hashCode, sameConfiguration.hashCode);
    expect(configuration, isNot(config(capacity: 8)));
    expect(
      configuration,
      isNot(config(strategy: CollisionStrategy.linearProbing)),
    );
    expect(configuration, isNot(config(algorithm: murmur)));
  });

  test('an isolated low surrogate is rejected at its public string offset', () {
    final error = _captureError(
      () => HashMapKey.string(String.fromCharCodes([0xdc00])),
    );
    expect(error, isA<MalformedUtf16>());
    expect((error as MalformedUtf16).codeUnitOffset, 0);
  });

  test('public maps match every directed operation and physical order', () {
    for (final row in hashMapFixture['operation_cases'] as List<dynamic>) {
      final item = row as Map<String, dynamic>;
      var map = fixtureMap(item['configuration'] as Map<String, dynamic>);
      expectFixtureState(
        map,
        item['initial_state'] as Map<String, dynamic>,
        reason: '${item['id']} initial',
      );
      for (final rawStep in item['steps'] as List<dynamic>) {
        final step = rawStep as Map<String, dynamic>;
        final key = fixtureKey(step['key_id'] as String);
        switch (step['op']) {
          case 'set':
            late HashMap<Object?> updated;
            expect(
              () => updated = map.setting(
                fixtureValue(step['value_id'] as String),
                forKey: key,
              ),
              returnsNormally,
              reason: '${item['id']} ${step['op']} ${step['key_id']}',
            );
            map = updated;
          case 'remove':
            map = map.removingValueForKey(key);
          case 'get':
            final observation = (step['expected']
                as Map<String, dynamic>)['observation'] as Map<String, dynamic>;
            expect(map.has(key), observation['has']);
            expect(map.valueForKey(key), observation['value']);
          case 'has':
            final observation = (step['expected']
                as Map<String, dynamic>)['observation'] as Map<String, dynamic>;
            expect(map.has(key), observation['has']);
          default:
            fail('Unknown public operation: ${step['op']}');
        }
        final expected = step['expected'] as Map<String, dynamic>;
        expect(expected['error_id'], isNull);
        expectFixtureState(
          map,
          expected['state'] as Map<String, dynamic>,
          reason: '${item['id']} ${step['op']}',
        );
      }
    }
  });

  test('fromEntries and merge preserve source semantics and snapshots', () {
    for (final rawCase
        in hashMapFixture['from_entries_cases'] as List<dynamic>) {
      final item = rawCase as Map<String, dynamic>;
      final configuration = item['configuration'] as Map<String, dynamic>;
      final entries = (item['source_entries'] as List<dynamic>).map((raw) {
        final row = raw as Map<String, dynamic>;
        return MapEntry<HashMapKey, Object?>(
          fixtureKey(row['key_id'] as String),
          fixtureValue(row['value_id'] as String),
        );
      });
      final map = HashMap<Object?>.fromEntries(
        entries,
        capacity: configuration['capacity'] as int,
        strategy: fixtureStrategy(configuration['strategy'] as String),
        hashAlgorithm: fixtureAlgorithm(
          fixtureHashConfiguration(
            configuration['hash_configuration_id'] as String,
          ),
        ),
      );
      expectFixtureState(
        map,
        item['expected_state'] as Map<String, dynamic>,
        reason: item['id'] as String,
      );
    }

    for (final rawCase in hashMapFixture['merge_cases'] as List<dynamic>) {
      final item = rawCase as Map<String, dynamic>;
      HashMap<Object?> build(Map<String, dynamic> side) {
        final config = side['configuration'] as Map<String, dynamic>;
        final source = (side['source_entries'] as List<dynamic>).map((raw) {
          final row = raw as Map<String, dynamic>;
          return MapEntry<HashMapKey, Object?>(
            fixtureKey(row['key_id'] as String),
            fixtureValue(row['value_id'] as String),
          );
        });
        return HashMap<Object?>.fromEntries(
          source,
          capacity: config['capacity'] as int,
          strategy: fixtureStrategy(config['strategy'] as String),
          hashAlgorithm: fixtureAlgorithm(
            fixtureHashConfiguration(config['hash_configuration_id'] as String),
          ),
        );
      }

      final left = build(item['left'] as Map<String, dynamic>);
      final right = build(item['right'] as Map<String, dynamic>);
      final keyIdByEncoding = <String, String>{};
      for (final entry in fixtureKeys.entries) {
        keyIdByEncoding.putIfAbsent(
          entry.value['encoded_hex'] as String,
          () => entry.key,
        );
      }
      expect(
        right.entries
            .map((entry) => keyIdByEncoding[fixtureHex(entry.key.encodedBytes)])
            .toList(),
        item['expected_right_physical_order'],
      );
      final result = left.merging(right);
      final expected = item['expected_result'] as Map<String, dynamic>;
      final expectedConfiguration =
          expected['configuration'] as Map<String, dynamic>;
      expect(result.capacity, expectedConfiguration['capacity']);
      expect(result.strategy.name, expectedConfiguration['strategy']);
      expect(
        result.hashAlgorithm,
        equals(fixtureAlgorithm(fixtureHashConfiguration(
          expectedConfiguration['hash_configuration_id'] as String,
        ))),
      );
      expectFixtureState(result, expected['state'] as Map<String, dynamic>);
      if (item['inputs_unchanged'] == true) {
        expectFixtureState(
          left,
          (item['left'] as Map<String, dynamic>)['expected_state']
              as Map<String, dynamic>,
        );
        expectFixtureState(
          right,
          (item['right'] as Map<String, dynamic>)['expected_state']
              as Map<String, dynamic>,
        );
      }
    }
  });

  test('key and map ownership cases use detached public snapshots', () {
    final source = fixtureBytes('00');
    final key = HashMapKey.bytes(source);
    source[0] = 255;
    expect(fixtureHex(key.encodedBytes), '00010000000000000000');

    final exposed = key.encodedBytes;
    exposed[0] = 255;
    expect(fixtureHex(key.encodedBytes), '00010000000000000000');

    final sourceSipKey = fixtureBytes('000102030405060708090a0b0c0d0e0f');
    final sip = HashAlgorithm.sipHash24(key: sourceSipKey);
    sourceSipKey[0] = 255;
    expectPublicHashAnswer(sip, const [], '726fdb47dd0e0e31');

    final map = HashMap<Object?>(
      capacity: 4,
      strategy: CollisionStrategy.separateChaining,
      hashAlgorithm: const HashAlgorithm.fnv1a32(),
    )
        .setting(7, forKey: fixtureKey('k0'))
        .setting(11, forKey: fixtureKey('k1'));
    final entries = map.entries;
    final keys = map.keys;
    final values = map.values;
    entries.clear();
    keys.clear();
    values.clear();
    expect(map.count, 2);
    expect(map.valueForKey(fixtureKey('k0')), 7);
    expect(map.valueForKey(fixtureKey('k1')), 11);

    final object = <String, int>{'field': 1};
    final original =
        HashMap<Object?>(hashAlgorithm: const HashAlgorithm.fnv1a32())
            .setting(object, forKey: fixtureKey('k0'));
    final changed = original.setting(2, forKey: fixtureKey('k1'));
    object['field'] = 2;
    expect((original.valueForKey(fixtureKey('k0')) as Map)['field'], 2);
    expect((changed.valueForKey(fixtureKey('k0')) as Map)['field'], 2);
    expect(original.has(fixtureKey('k1')), isFalse);
  });

  test('byte and string key tags remain distinct under a forced collision', () {
    final byteKey = HashMapKey.bytes([0x61]);
    final stringKey = HashMapKey.string('a');
    final map = HashMap<String>(
      capacity: 1,
      strategy: CollisionStrategy.separateChaining,
      hashAlgorithm: const HashAlgorithm.fnv1a32(),
    ).setting('byte', forKey: byteKey).setting('string', forKey: stringKey);
    expect(map.count, 2);
    expect(map.has(byteKey), isTrue);
    expect(map.has(stringKey), isTrue);
    expect(map.valueForKey(byteKey), 'byte');
    expect(map.valueForKey(stringKey), 'string');
  });

  test('byte keys validate the one captured caller snapshot', () {
    final changing = _ChangingByteList([7, 256]);
    final key = HashMapKey.bytes(changing);
    expect(changing.elementReads, 1);
    expect(fixtureHex(key.encodedBytes), '00010000000000000007');
    final map = HashMap<int>(
      capacity: 3,
      hashAlgorithm: const HashAlgorithm.fnv1a32(),
    ).setting(1, forKey: key);
    expect(map.has(HashMapKey.bytes([7])), isTrue);
    expect(map.valueForKey(HashMapKey.bytes([7])), 1);

    final invalidFirst = _ChangingByteList([256]);
    final error = _captureError(() => HashMapKey.bytes(invalidFirst));
    expect(error, isA<InvalidByte>());
    expect((error as InvalidByte).index, 0);
    expect(error.value, 256);
    expect(invalidFirst.elementReads, 1);
  });

  test('public deletion of an absent valid key leaves the map unchanged', () {
    for (final strategy in CollisionStrategy.values) {
      final original = HashMap<int>(
        capacity: 7,
        strategy: strategy,
        hashAlgorithm: const HashAlgorithm.fnv1a32(),
      ).setting(7, forKey: HashMapKey.bytes([1]));
      final removed = original.removingValueForKey(HashMapKey.bytes([2]));
      expect(removed.count, original.count);
      expect(removed.capacity, original.capacity);
      expect(removed.has(HashMapKey.bytes([1])), isTrue);
      expect(removed.valueForKey(HashMapKey.bytes([1])), 7);
      expect(original.has(HashMapKey.bytes([2])), isFalse);
    }
  });

  test(
      'public placement preserves exact wide hash reduction on non-power capacity',
      () {
    const capacity = 257;
    const entryCount = 180; // Below the linear-probing 75 percent threshold.
    final algorithm = HashAlgorithm.sipHash24(
      key: fixtureBytes('000102030405060708090a0b0c0d0e0f'),
    );
    final expectedSlots = List<int?>.filled(capacity, null);
    final source = <MapEntry<HashMapKey, int>>[];
    for (var value = 0; value < entryCount; value++) {
      final key = HashMapKey.bytes([value >> 8, value & 0xff]);
      source.add(MapEntry<HashMapKey, int>(key, value));
      final start =
          (algorithm.validatedHash(key.encodedBytes) % BigInt.from(capacity))
              .toInt();
      var slot = start;
      while (expectedSlots[slot] != null) {
        slot = (slot + 1) % capacity;
      }
      expectedSlots[slot] = value;
    }
    final map = HashMap<int>.fromEntries(
      source,
      capacity: capacity,
      strategy: CollisionStrategy.linearProbing,
      hashAlgorithm: algorithm,
    );
    expect(map.capacity, capacity);
    expect(map.count, entryCount);
    expect(map.entries.map((entry) => entry.value).toList(), [
      for (final value in expectedSlots)
        if (value != null) value,
    ]);
  });

  test('shared error cases reach public constructors and operations', () {
    for (final rawCase in hashMapFixture['error_cases'] as List<dynamic>) {
      final item = rawCase as Map<String, dynamic>;
      final config = item['configuration'] as Map<String, dynamic>;
      final algorithm = fixtureAlgorithm(
          config['hash_configuration'] as Map<String, dynamic>);
      switch (item['operation']) {
        case 'construct':
          final error = _captureError(() => HashMap<Object?>(
                capacity: config['capacity'] as int,
                strategy: fixtureStrategy(config['strategy'] as String),
                hashAlgorithm: algorithm,
              ));
          expectFixtureError(
            error,
            item['expected_error'] as Map<String, dynamic>,
          );
        case 'set':
          final map = HashMap<Object?>(
            capacity: config['capacity'] as int,
            strategy: fixtureStrategy(config['strategy'] as String),
            hashAlgorithm: algorithm,
          ).setting(7, forKey: fixtureKey('bytes-a'));
          expectFixtureState(map, item['before_state'] as Map<String, dynamic>);
          final recipe = item['key_recipe'] as Map<String, dynamic>;
          final oversized = HashMapKey.bytes(
            List<int>.filled(recipe['length'] as int, recipe['byte'] as int),
          );
          expectFixtureError(
            _captureError(() => map.setting(item['value'], forKey: oversized)),
            item['expected_error'] as Map<String, dynamic>,
          );
          expectFixtureState(
              map, item['expected_state'] as Map<String, dynamic>);
        case 'get-has-remove':
          final map = HashMap<Object?>(
            capacity: config['capacity'] as int,
            strategy: fixtureStrategy(config['strategy'] as String),
            hashAlgorithm: algorithm,
          ).setting(7, forKey: fixtureKey('bytes-a'));
          final recipe = item['key_recipe'] as Map<String, dynamic>;
          final oversized = HashMapKey.bytes(
            List<int>.filled(recipe['length'] as int, recipe['byte'] as int),
          );
          final observation =
              item['expected_observation'] as Map<String, dynamic>;
          expect(map.has(oversized), observation['has']);
          expect(map.valueForKey(oversized), observation['value']);
          final removed = map.removingValueForKey(oversized);
          expect(removed.has(oversized), isFalse);
          expectFixtureState(
              removed, item['expected_state'] as Map<String, dynamic>);
          expectFixtureState(
              map, item['expected_state'] as Map<String, dynamic>);
        default:
          fail('Unknown shared error operation: ${item['operation']}');
      }
    }
  });

  test('Dart-only malformed arguments reach their real public operations', () {
    final cases = (hashMapFixture['dart_only_cases'] as List<dynamic>)
        .cast<Map<String, dynamic>>();
    expect(cases, hasLength(9));
    final executedIds = <String>{};
    for (final item in cases) {
      final id = item['id'] as String;
      switch (id) {
        case 'negative-byte-before-later-overflow-byte':
          expectFixtureError(
            _captureError(() => HashMapKey.bytes(
                  (item['raw_byte_values'] as List<dynamic>).cast<int>(),
                )),
            item['expected_error'] as Map<String, dynamic>,
          );
        case 'invalid-byte-in-sixteen-byte-siphash-key':
          final algorithm = HashAlgorithm.sipHash24(
            key: (item['raw_key_values'] as List<dynamic>).cast<int>(),
          );
          expect(item['constructor_result'], 'success');
          expectFixtureError(
            _captureError(() => algorithm.validatedHash(
                  fixtureBytes(item['validation_data_hex'] as String),
                )),
            item['expected_error'] as Map<String, dynamic>,
          );
        case 'negative-murmur-seed':
        case 'murmur-seed-above-uint32':
          final algorithm = HashAlgorithm.murmur3(seed: item['seed'] as int);
          expect(item['constructor_result'], 'success');
          expectFixtureError(
            _captureError(() => algorithm.validatedHash(
                  fixtureBytes(item['validation_data_hex'] as String),
                )),
            item['expected_error'] as Map<String, dynamic>,
          );
        case 'malformed-utf16-surrogate':
          final units = (item['utf16_code_units'] as List<dynamic>).cast<int>();
          expectFixtureError(
            _captureError(() => HashMapKey.string(String.fromCharCodes(units))),
            item['expected_error'] as Map<String, dynamic>,
          );
        case 'invalid-capacity-precedes-iterable-request':
          final requests = _IteratorRequestCounter();
          expectFixtureError(
            _captureError(() => HashMap<Object?>.fromEntries(
                  requests,
                  capacity: 0,
                  strategy: CollisionStrategy.separateChaining,
                  hashAlgorithm: const HashAlgorithm.fnv1a32(),
                )),
            item['expected_error'] as Map<String, dynamic>,
          );
          expect(requests.iteratorRequests, item['expected_iterator_requests']);
        case 'iterable-exception-propagates-after-one-entry':
          final source = _ThrowAfterOneIterable([
            MapEntry<HashMapKey, Object?>(fixtureKey('k0'), fixtureValue('v0')),
          ]);
          expect(
            () => HashMap<Object?>.fromEntries(
              source,
              capacity: 4,
              strategy: CollisionStrategy.separateChaining,
              hashAlgorithm: const HashAlgorithm.fnv1a32(),
            ),
            throwsA(isA<FixtureIteratorError>()),
          );
          expect(source.iteratorRequests, item['expected_iterator_requests']);
          expect(source.entriesYielded, item['expected_entries_yielded']);
        case 'iterable-consumed-once':
          final rows = (item['source_entries'] as List<dynamic>)
              .cast<Map<String, dynamic>>();
          final source = _CountingIterable([
            for (final row in rows)
              MapEntry<HashMapKey, Object?>(
                fixtureKey(row['key_id'] as String),
                fixtureValue(row['value_id'] as String),
              ),
          ]);
          final map = HashMap<Object?>.fromEntries(
            source,
            capacity: 4,
            strategy: CollisionStrategy.separateChaining,
            hashAlgorithm: const HashAlgorithm.fnv1a32(),
          );
          expect(source.iteratorRequests, item['expected_iterator_requests']);
          expect(map.count, item['expected_count']);
        case 'from-entries-stops-after-first-invalid-entry':
          final rows = (item['source_entries'] as List<dynamic>)
              .cast<Map<String, dynamic>>();
          final overlong = rows[1]['key_recipe'] as Map<String, dynamic>;
          final source = _ThrowAfterInvalidIterable([
            MapEntry<HashMapKey, Object?>(fixtureKey('k0'), fixtureValue('v0')),
            MapEntry<HashMapKey, Object?>(
              HashMapKey.bytes(List<int>.filled(
                overlong['length'] as int,
                overlong['byte'] as int,
              )),
              fixtureValue(rows[1]['value_id'] as String),
            ),
          ]);
          expectFixtureError(
            _captureError(() => HashMap<Object?>.fromEntries(
                  source,
                  capacity: 4,
                  strategy: CollisionStrategy.separateChaining,
                  hashAlgorithm: const HashAlgorithm.fnv1a32(),
                )),
            item['expected_error'] as Map<String, dynamic>,
          );
          expect(source.iteratorRequests, item['expected_iterator_requests']);
          expect(source.entriesYielded, item['expected_entries_yielded']);
          expect(source.advancesAfterInvalidEntry, 0);
        default:
          fail('Unimplemented Dart-only public case: $id');
      }
      executedIds.add(id);
    }
    expect(executedIds, hasLength(9));
    expect(executedIds, containsAll(cases.map((row) => row['id'] as String)));
    print('DT18 Dart-only public cases executed: ${cases.length}/9.');
  });

  test('well-formed wrong expected hash fails at the public comparator', () {
    final control = (hashMapFixture['negative_controls'] as List<dynamic>)
        .cast<Map<String, dynamic>>()
        .single;
    final source = (hashMapFixture['hash_answers'] as List<dynamic>)
        .cast<Map<String, dynamic>>()
        .singleWhere((row) => row['id'] == control['source_hash_answer_id']);
    final bytes = fixtureBytes(source['data_hex'] as String);
    final configuration = fixtureHashConfiguration(
      source['configuration_id'] as String,
    );
    final actual = fixtureAlgorithm(configuration).validatedHash(bytes);
    final expected = int.parse(source['expected_hex'] as String, radix: 16) ^
        int.parse(control['expected_xor_mask'] as String, radix: 16);
    expect(control['well_formed'], isTrue);
    expect(control['required_stage'], 'actual-public-hash-comparator');
    expect(
      () => expect(
        actual.toRadixString(16).padLeft(8, '0'),
        expected.toRadixString(16).padLeft(8, '0'),
        reason: 'actual-public-hash-comparator',
      ),
      throwsA(isA<TestFailure>()),
    );
  });
}

Object _captureError(void Function() operation) {
  try {
    operation();
  } catch (error) {
    return error;
  }
  fail('Expected public operation to throw.');
}

class FixtureIteratorError implements Exception {}

class _IteratorRequestCounter
    extends IterableBase<MapEntry<HashMapKey, Object?>> {
  int iteratorRequests = 0;

  @override
  Iterator<MapEntry<HashMapKey, Object?>> get iterator {
    iteratorRequests++;
    throw FixtureIteratorError();
  }
}

class _ChangingByteList extends ListBase<int> {
  _ChangingByteList(this.readSequence);

  final List<int> readSequence;
  int elementReads = 0;

  @override
  int get length => 1;

  @override
  set length(int value) => throw UnsupportedError('fixed test list');

  @override
  int operator [](int index) {
    if (index != 0) throw RangeError.index(index, this);
    final sequenceIndex = elementReads++;
    return readSequence[
        sequenceIndex < readSequence.length ? sequenceIndex : 0];
  }

  @override
  void operator []=(int index, int value) =>
      throw UnsupportedError('fixed test list');
}

class _CountingIterable extends IterableBase<MapEntry<HashMapKey, Object?>> {
  _CountingIterable(this.items);
  final List<MapEntry<HashMapKey, Object?>> items;
  int iteratorRequests = 0;

  @override
  Iterator<MapEntry<HashMapKey, Object?>> get iterator {
    iteratorRequests++;
    return items.iterator;
  }
}

class _ThrowAfterOneIterable
    extends IterableBase<MapEntry<HashMapKey, Object?>> {
  _ThrowAfterOneIterable(this.items);
  final List<MapEntry<HashMapKey, Object?>> items;
  int iteratorRequests = 0;
  int entriesYielded = 0;

  @override
  Iterator<MapEntry<HashMapKey, Object?>> get iterator {
    iteratorRequests++;
    return _ThrowAfterOneIterator(items, (count) => entriesYielded = count);
  }
}

class _ThrowAfterOneIterator
    implements Iterator<MapEntry<HashMapKey, Object?>> {
  _ThrowAfterOneIterator(this.items, this.onYield);
  final List<MapEntry<HashMapKey, Object?>> items;
  final void Function(int) onYield;
  int _index = -1;

  @override
  MapEntry<HashMapKey, Object?> get current => items[_index];

  @override
  bool moveNext() {
    if (_index < items.length - 1) {
      _index++;
      onYield(_index + 1);
      return true;
    }
    throw FixtureIteratorError();
  }
}

class _ThrowAfterInvalidIterable
    extends IterableBase<MapEntry<HashMapKey, Object?>> {
  _ThrowAfterInvalidIterable(this.items);
  final List<MapEntry<HashMapKey, Object?>> items;
  int iteratorRequests = 0;
  int entriesYielded = 0;
  int advancesAfterInvalidEntry = 0;

  @override
  Iterator<MapEntry<HashMapKey, Object?>> get iterator {
    iteratorRequests++;
    return _ThrowAfterInvalidIterator(
      items,
      (yielded, afterInvalid) {
        entriesYielded = yielded;
        advancesAfterInvalidEntry = afterInvalid;
      },
    );
  }
}

class _ThrowAfterInvalidIterator
    implements Iterator<MapEntry<HashMapKey, Object?>> {
  _ThrowAfterInvalidIterator(this.items, this.onProgress);
  final List<MapEntry<HashMapKey, Object?>> items;
  final void Function(int yielded, int afterInvalid) onProgress;
  int _index = -1;

  @override
  MapEntry<HashMapKey, Object?> get current => items[_index];

  @override
  bool moveNext() {
    if (_index < items.length - 1) {
      _index++;
      onProgress(_index + 1, 0);
      return true;
    }
    onProgress(items.length, 1);
    throw FixtureIteratorError();
  }
}

const _sha256Mask = 0xffffffff;
const _sha256RoundConstants = <int>[
  0x428a2f98,
  0x71374491,
  0xb5c0fbcf,
  0xe9b5dba5,
  0x3956c25b,
  0x59f111f1,
  0x923f82a4,
  0xab1c5ed5,
  0xd807aa98,
  0x12835b01,
  0x243185be,
  0x550c7dc3,
  0x72be5d74,
  0x80deb1fe,
  0x9bdc06a7,
  0xc19bf174,
  0xe49b69c1,
  0xefbe4786,
  0x0fc19dc6,
  0x240ca1cc,
  0x2de92c6f,
  0x4a7484aa,
  0x5cb0a9dc,
  0x76f988da,
  0x983e5152,
  0xa831c66d,
  0xb00327c8,
  0xbf597fc7,
  0xc6e00bf3,
  0xd5a79147,
  0x06ca6351,
  0x14292967,
  0x27b70a85,
  0x2e1b2138,
  0x4d2c6dfc,
  0x53380d13,
  0x650a7354,
  0x766a0abb,
  0x81c2c92e,
  0x92722c85,
  0xa2bfe8a1,
  0xa81a664b,
  0xc24b8b70,
  0xc76c51a3,
  0xd192e819,
  0xd6990624,
  0xf40e3585,
  0x106aa070,
  0x19a4c116,
  0x1e376c08,
  0x2748774c,
  0x34b0bcb5,
  0x391c0cb3,
  0x4ed8aa4a,
  0x5b9cca4f,
  0x682e6ff3,
  0x748f82ee,
  0x78a5636f,
  0x84c87814,
  0x8cc70208,
  0x90befffa,
  0xa4506ceb,
  0xbef9a3f7,
  0xc67178f2,
];

String _sha256Hex(List<int> bytes) {
  final padded = List<int>.of(bytes)..add(0x80);
  while (padded.length % 64 != 56) {
    padded.add(0);
  }
  final bitLength = BigInt.from(bytes.length) * BigInt.from(8);
  for (var shift = 56; shift >= 0; shift -= 8) {
    padded.add(((bitLength >> shift) & BigInt.from(0xff)).toInt());
  }

  final state = <int>[
    0x6a09e667,
    0xbb67ae85,
    0x3c6ef372,
    0xa54ff53a,
    0x510e527f,
    0x9b05688c,
    0x1f83d9ab,
    0x5be0cd19,
  ];
  for (var offset = 0; offset < padded.length; offset += 64) {
    final words = List<int>.filled(64, 0);
    for (var index = 0; index < 16; index++) {
      final start = offset + index * 4;
      words[index] = ((padded[start] << 24) |
              (padded[start + 1] << 16) |
              (padded[start + 2] << 8) |
              padded[start + 3]) &
          _sha256Mask;
    }
    for (var index = 16; index < 64; index++) {
      final x = words[index - 15];
      final y = words[index - 2];
      final sigma0 = _rotateRight32(x, 7) ^ _rotateRight32(x, 18) ^ (x >>> 3);
      final sigma1 = _rotateRight32(y, 17) ^ _rotateRight32(y, 19) ^ (y >>> 10);
      words[index] = (words[index - 16] + sigma0 + words[index - 7] + sigma1) &
          _sha256Mask;
    }

    var a = state[0];
    var b = state[1];
    var c = state[2];
    var d = state[3];
    var e = state[4];
    var f = state[5];
    var g = state[6];
    var h = state[7];
    for (var index = 0; index < 64; index++) {
      final sum1 =
          _rotateRight32(e, 6) ^ _rotateRight32(e, 11) ^ _rotateRight32(e, 25);
      final choose = (e & f) ^ ((~e) & g);
      final temp1 =
          (h + sum1 + choose + _sha256RoundConstants[index] + words[index]) &
              _sha256Mask;
      final sum0 =
          _rotateRight32(a, 2) ^ _rotateRight32(a, 13) ^ _rotateRight32(a, 22);
      final majority = (a & b) ^ (a & c) ^ (b & c);
      final temp2 = (sum0 + majority) & _sha256Mask;
      h = g;
      g = f;
      f = e;
      e = (d + temp1) & _sha256Mask;
      d = c;
      c = b;
      b = a;
      a = (temp1 + temp2) & _sha256Mask;
    }
    state[0] = (state[0] + a) & _sha256Mask;
    state[1] = (state[1] + b) & _sha256Mask;
    state[2] = (state[2] + c) & _sha256Mask;
    state[3] = (state[3] + d) & _sha256Mask;
    state[4] = (state[4] + e) & _sha256Mask;
    state[5] = (state[5] + f) & _sha256Mask;
    state[6] = (state[6] + g) & _sha256Mask;
    state[7] = (state[7] + h) & _sha256Mask;
  }
  return state.map((word) => word.toRadixString(16).padLeft(8, '0')).join();
}

int _rotateRight32(int value, int distance) =>
    ((value >>> distance) | (value << (32 - distance))) & _sha256Mask;
