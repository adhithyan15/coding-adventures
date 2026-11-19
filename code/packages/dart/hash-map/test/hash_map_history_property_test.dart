import 'package:coding_adventures_hash_map/coding_adventures_hash_map.dart';
import 'package:test/test.dart';

import 'hash_map_shared_fixture_test.dart' as fixture;

const _historyAlphabet = '0123456789abcdefghijklmnopq';
const _historyStrategyNames = ['separateChaining', 'linearProbing'];

/// SplitMix64 uses BigInt arithmetic, so the generated stream is identical on
/// the Dart VM and JavaScript. This is deterministic generated testing, not
/// coverage-guided fuzzing, and the test package does not provide shrinking.
class _SplitMix64 {
  _SplitMix64(BigInt seed) : _state = seed & _mask64;

  static final BigInt _mask64 = (BigInt.one << 64) - BigInt.one;
  static final BigInt _gamma = BigInt.parse('9e3779b97f4a7c15', radix: 16);
  static final BigInt _mix1 = BigInt.parse('bf58476d1ce4e5b9', radix: 16);
  static final BigInt _mix2 = BigInt.parse('94d049bb133111eb', radix: 16);

  BigInt _state;

  BigInt next64() {
    _state = (_state + _gamma) & _mask64;
    var value = _state;
    value = ((value ^ (value >> 30)) * _mix1) & _mask64;
    value = ((value ^ (value >> 27)) * _mix2) & _mask64;
    return value ^ (value >> 31);
  }

  int nextInt(int maximum) => (next64() % BigInt.from(maximum)).toInt();
}

void main() {
  test(
    'exhaustive DT18 histories match the association-list oracle and every version',
    () {
      final recipe =
          fixture.hashMapFixture['history_recipe'] as Map<String, dynamic>;
      final operations =
          (recipe['operations'] as List<dynamic>).cast<Map<String, dynamic>>();
      final keyIds = (recipe['key_ids'] as List<dynamic>).cast<String>();
      final valueIds = (recipe['value_ids'] as List<dynamic>).cast<String>();
      final lengthRows = (recipe['lengths'] as List<dynamic>).cast<int>();
      final historyCount = recipe['history_count'] as int;
      final streams =
          recipe['expected_semantic_streams'] as Map<String, dynamic>;
      final physicalStreams =
          recipe['expected_physical_order_streams'] as Map<String, dynamic>;
      final capacityStreams =
          recipe['expected_capacity_streams'] as Map<String, dynamic>;
      final physicalTable = (recipe['physical_order_table'] as List<dynamic>)
          .cast<Map<String, dynamic>>();
      final keySlots = {
        for (var index = 0; index < keyIds.length; index++) keyIds[index]: index
      };
      final valueSlots = {
        for (var index = 0; index < valueIds.length; index++)
          valueIds[index]: index + 1,
      };
      final keyByEncoding = <String, String>{};
      for (final row in fixture.fixtureKeys.entries) {
        keyByEncoding.putIfAbsent(
          row.value['encoded_hex'] as String,
          () => row.key,
        );
      }
      for (final strategyName in _historyStrategyNames) {
        expect((streams[strategyName] as String).length, historyCount);
        expect((physicalStreams[strategyName] as String).length, historyCount);
        expect((capacityStreams[strategyName] as String).length, historyCount);
      }
      expect(operations, hasLength(9));

      var ordinalOffset = 0;
      var seenHistories = 0;
      var currentStateChecks = 0;
      var earlierVersionChecks = 0;
      var queryAssertions = 0;
      for (final length in lengthRows) {
        final sequenceCount = _power(operations.length, length);
        for (var rank = 0; rank < sequenceCount; rank++) {
          final digits = _baseNineDigits(rank, length);
          expect(digits, hasLength(length));
          expect(digits.every((digit) => digit >= 0 && digit < 9), isTrue);
          expect(_baseNineRank(digits), rank,
              reason: 'length=$length rank=$rank is bijectively encoded');

          final ordinal = ordinalOffset + rank;
          final generatedOrdinal = _ordinalFor(
              digits, operations.length, recipe['max_length'] as int);
          expect(generatedOrdinal, ordinal,
              reason: 'length=$length rank=$rank ordinal');
          for (final strategyName in _historyStrategyNames) {
            final strategy = fixture.fixtureStrategy(strategyName);
            var current = HashMap<Object?>(
              capacity: recipe['initial_capacity'] as int,
              strategy: strategy,
              hashAlgorithm: fixture.fixtureAlgorithm(
                fixture.fixtureHashConfiguration(
                  recipe['hash_configuration_id'] as String,
                ),
              ),
            );
            var model = List<int>.filled(keyIds.length, 0);
            final versions = <HashMap<Object?>>[current];
            final modelVersions = <List<int>>[List<int>.of(model)];
            // Every sequence starts from the one canonical empty history at
            // ordinal zero, regardless of the full sequence's length block.
            final versionOrdinals = <int>[0];

            _assertHistoryState(
              current,
              model,
              0,
              strategyName,
              streams,
              physicalStreams,
              capacityStreams,
              physicalTable,
              keyIds,
              keySlots,
              valueSlots,
              keyByEncoding,
            );
            currentStateChecks++;
            queryAssertions += keyIds.length * 2;

            for (var prefix = 0; prefix < length; prefix++) {
              final operationIndex = digits[prefix];
              final operation = operations[operationIndex];
              final keyId = operation['key_id'] as String;
              final keyIndex = keySlots[keyId]!;
              if (operation['op'] == 'set') {
                final valueId = operation['value_id'] as String;
                final valueCode = valueSlots[valueId]!;
                final concrete = fixture.fixtureValue(valueId);
                current = current.setting(
                  concrete,
                  forKey: fixture.fixtureKey(keyId),
                );
                model[keyIndex] = valueCode;
              } else {
                current =
                    current.removingValueForKey(fixture.fixtureKey(keyId));
                model[keyIndex] = 0;
              }
              versions.add(current);
              modelVersions.add(List<int>.of(model));
              final prefixDigits = digits.sublist(0, prefix + 1);
              final prefixOrdinal = _ordinalFor(
                prefixDigits,
                operations.length,
                recipe['max_length'] as int,
              );
              versionOrdinals.add(prefixOrdinal);

              _assertHistoryState(
                current,
                model,
                prefixOrdinal,
                strategyName,
                streams,
                physicalStreams,
                capacityStreams,
                physicalTable,
                keyIds,
                keySlots,
                valueSlots,
                keyByEncoding,
              );
              currentStateChecks++;
              queryAssertions += keyIds.length * 2;

              // Every older immutable version is checked after every prefix.
              for (var version = 0; version < prefix + 1; version++) {
                _assertHistoryState(
                  versions[version],
                  modelVersions[version],
                  versionOrdinals[version],
                  strategyName,
                  streams,
                  physicalStreams,
                  capacityStreams,
                  physicalTable,
                  keyIds,
                  keySlots,
                  valueSlots,
                  keyByEncoding,
                );
                earlierVersionChecks++;
                queryAssertions += keyIds.length * 2;
              }
            }
          }
          seenHistories++;
        }
        ordinalOffset += sequenceCount;
      }
      expect(seenHistories, historyCount);
      expect(ordinalOffset, historyCount);
      expect(currentStateChecks, 780554);
      expect(earlierVersionChecks, 1911942);
      expect(currentStateChecks + earlierVersionChecks, 2692496);
      expect(queryAssertions, 16154976);
      print(
        'DT18 histories: $seenHistories; current-state checks: '
        '$currentStateChecks; prior-version checks: $earlierVersionChecks; '
        'public has/value queries: $queryAssertions; strategies: 2.',
      );
    },
    timeout: const Timeout(Duration(minutes: 20)),
  );

  test(
    'seeded replayable map histories match a descriptor association list '
    '(seed=0x445431385f6d6170, cases=512, ops=96)',
    () {
      const seed = '445431385f6d6170';
      final random = _SplitMix64(BigInt.parse(seed, radix: 16));
      final byteKeys = List.generate(
        12,
        (index) => HashMapKey.bytes([index, index * 7 & 0xff]),
      );
      final stringKeys = List.generate(
        4,
        (index) => HashMapKey.string('generated-$index-🙂'),
      );
      final keys = <HashMapKey>[...byteKeys, ...stringKeys];
      final identities = <String>[
        for (var index = 0; index < byteKeys.length; index++)
          'bytes:$index:${index * 7 & 0xff}',
        for (var index = 0; index < stringKeys.length; index++)
          'string:generated-$index-🙂',
      ];
      final model = <_ModelEntry>[];
      var assertions = 0;

      for (var caseIndex = 0; caseIndex < 512; caseIndex++) {
        final strategy = caseIndex.isEven
            ? CollisionStrategy.separateChaining
            : CollisionStrategy.linearProbing;
        var actual = HashMap<Object?>(
          capacity: 7,
          strategy: strategy,
          hashAlgorithm: const HashAlgorithm.fnv1a32(),
        );
        model.clear();
        for (var operation = 0; operation < 96; operation++) {
          final slot = random.nextInt(keys.length);
          final identity = identities[slot];
          if (random.nextInt(4) == 0) {
            actual = actual.removingValueForKey(keys[slot]);
            model.removeWhere((entry) => entry.key == identity);
          } else {
            final selector = random.nextInt(5);
            final Object? value = switch (selector) {
              0 => null,
              1 => -1,
              2 => 0,
              3 => 1,
              _ => random.nextInt(1000),
            };
            actual = actual.setting(value, forKey: keys[slot]);
            final previous = model.indexWhere((entry) => entry.key == identity);
            if (previous < 0) {
              model.add(_ModelEntry(identity, value));
            } else {
              model[previous] = _ModelEntry(identity, value);
            }
          }
          for (var querySlot = 0; querySlot < keys.length; querySlot++) {
            final expectedIndex =
                model.indexWhere((entry) => entry.key == identities[querySlot]);
            final expectedPresent = expectedIndex >= 0;
            expect(
              actual.has(keys[querySlot]),
              expectedPresent,
              reason:
                  'seed=0x$seed case=$caseIndex op=$operation key=$querySlot',
            );
            expect(
              actual.valueForKey(keys[querySlot]),
              expectedPresent ? model[expectedIndex].value : null,
              reason:
                  'seed=0x$seed case=$caseIndex op=$operation value=$querySlot',
            );
            assertions += 2;
          }
          expect(actual.count, model.length,
              reason: 'seed=0x$seed case=$caseIndex op=$operation count');
        }

        // Reproducibly mutate caller-owned byte lists after key construction.
        // The local expected framing is encoded from the saved source bytes,
        // never from the public key's equality or encodedBytes getter.
        final payload = List<int>.generate(
          random.nextInt(65),
          (_) => random.nextInt(256),
        );
        final savedPayload = List<int>.of(payload);
        final ownedKey = HashMapKey.bytes(payload);
        final ownershipMap = HashMap<int>(
          capacity: 3,
          strategy: strategy,
          hashAlgorithm: const HashAlgorithm.fnv1a32(),
        ).setting(caseIndex, forKey: ownedKey);
        final changedPayload = List<int>.of(payload);
        if (changedPayload.isEmpty) {
          changedPayload.add(1);
        } else {
          changedPayload[0] ^= 1;
        }
        payload
          ..clear()
          ..addAll(changedPayload);
        expect(ownedKey.encodedBytes, _independentEncoding(0, savedPayload),
            reason: 'seed=0x$seed case=$caseIndex copied source bytes');
        expect(ownershipMap.has(HashMapKey.bytes(savedPayload)), isTrue,
            reason: 'seed=0x$seed case=$caseIndex original key remains');
        expect(ownershipMap.has(HashMapKey.bytes(changedPayload)), isFalse,
            reason: 'seed=0x$seed case=$caseIndex changed key is distinct');
      }
      expect(assertions, 512 * 96 * keys.length * 2);
      print(
        'DT18 deterministic generated sequences: seed=0x$seed, '
        '512 cases × 96 operations plus 512 byte-list mutation cases; '
        'public query assertions=$assertions. '
        'Replayable; no coverage feedback or shrinking.',
      );
    },
  );
}

class _ModelEntry {
  const _ModelEntry(this.key, this.value);
  final String key;
  final Object? value;
}

List<int> _independentEncoding(int tag, List<int> payload) => [
      tag,
      for (var shift = 0; shift < 64; shift += 8)
        payload.length >> shift & 0xff,
      ...payload,
    ];

void _assertHistoryState(
  HashMap<Object?> map,
  List<int> model,
  int ordinal,
  String strategy,
  Map<String, dynamic> semanticStreams,
  Map<String, dynamic> physicalStreams,
  Map<String, dynamic> capacityStreams,
  List<Map<String, dynamic>> physicalTable,
  List<String> keyIds,
  Map<String, int> keySlots,
  Map<String, int> valueSlots,
  Map<String, String> keyByEncoding,
) {
  final semanticStream = semanticStreams[strategy] as String;
  final physicalStream = physicalStreams[strategy] as String;
  final capacityStream = capacityStreams[strategy] as String;
  expect(ordinal, inInclusiveRange(0, semanticStream.length - 1));
  final expectedStateCode = model[0] + 3 * model[1] + 9 * model[2];
  expect(
    _historyAlphabet.indexOf(semanticStream[ordinal]),
    expectedStateCode,
    reason: 'strategy=$strategy ordinal=$ordinal independent semantic model',
  );
  expect(map.count, model.where((value) => value != 0).length,
      reason: 'strategy=$strategy ordinal=$ordinal count');
  final expectedCapacity = int.parse(capacityStream[ordinal], radix: 36);
  expect(map.capacity, expectedCapacity,
      reason: 'strategy=$strategy ordinal=$ordinal capacity stream');
  final physicalIndex = int.parse(physicalStream[ordinal], radix: 16);
  final wantedIds =
      (physicalTable[physicalIndex]['key_ids'] as List<dynamic>).cast<String>();
  final actualIds = map.entries
      .map((entry) => keyByEncoding[fixture.fixtureHex(entry.key.encodedBytes)])
      .toList();
  expect(actualIds, wantedIds,
      reason: 'strategy=$strategy ordinal=$ordinal physical order');

  // Public has/get calls audit every model descriptor, including stored nulls.
  for (final keyId in keyIds) {
    final slot = keySlots[keyId]!;
    final present = model[slot] != 0;
    expect(map.has(fixture.fixtureKey(keyId)), present,
        reason: 'strategy=$strategy ordinal=$ordinal key=$keyId has');
    final expectedValue = present
        ? fixture.fixtureValue(
            valueSlots.entries
                .singleWhere((entry) => entry.value == model[slot])
                .key,
          )
        : null;
    expect(map.valueForKey(fixture.fixtureKey(keyId)), expectedValue,
        reason: 'strategy=$strategy ordinal=$ordinal key=$keyId get');
  }
}

int _power(int base, int exponent) {
  var value = 1;
  for (var index = 0; index < exponent; index++) {
    value *= base;
  }
  return value;
}

List<int> _baseNineDigits(int rank, int length) {
  var remaining = rank;
  final digits = List<int>.filled(length, 0);
  for (var index = length - 1; index >= 0; index--) {
    digits[index] = remaining % 9;
    remaining ~/= 9;
  }
  if (remaining != 0) {
    throw StateError(
        'Rank $rank does not fit a base-9 tuple of length $length');
  }
  return digits;
}

int _baseNineRank(List<int> digits) {
  var rank = 0;
  for (final digit in digits) {
    rank = rank * 9 + digit;
  }
  return rank;
}

int _ordinalFor(List<int> digits, int base, int maximumLength) {
  var offset = 0;
  var power = 1;
  for (var length = 0; length < digits.length; length++) {
    offset += power;
    power *= base;
  }
  if (digits.length > maximumLength) {
    throw StateError('History length exceeds frozen maximum.');
  }
  return offset + _baseNineRank(digits);
}
