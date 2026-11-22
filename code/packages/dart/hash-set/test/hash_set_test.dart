import 'dart:convert';
import 'dart:math';

import 'package:coding_adventures_hash_set/coding_adventures_hash_set.dart';
import 'package:test/test.dart';

void main() {
  final leftKeys = [
    HashMapKey.string('alpha'),
    HashMapKey.string('beta'),
    HashMapKey.string('gamma'),
  ];

  group('DT19 HashSet Conformance Tests', () {
    test('construction discards duplicates and preserves exact identity', () {
      final composed = HashMapKey.string('\u{00E9}');
      final decomposed = HashMapKey.string('e\u{0301}');
      final set = HashSet.fromElements([
        HashMapKey.string('alpha'),
        HashMapKey.string('alpha'),
        composed,
        decomposed,
        HashMapKey.bytes(utf8.encode('alpha')),
      ]);

      expect(set.count, equals(4));
      expect(set.contains(HashMapKey.string('alpha')), isTrue);
      expect(set.contains(HashMapKey.bytes(utf8.encode('alpha'))), isTrue);
      expect(set.contains(composed), isTrue);
      expect(set.contains(decomposed), isTrue);
    });

    test('adding, removing, and discarding are persistent', () {
      final empty = HashSet(
        capacity: 4,
        strategy: CollisionStrategy.linearProbing,
        hashAlgorithm: const HashAlgorithm.fnv1a32(),
      );
      final one = empty.adding(HashMapKey.string('alpha'));
      final duplicate = one.adding(HashMapKey.string('alpha'));
      final two = duplicate.adding(HashMapKey.string('beta'));
      final removed = two.removing(HashMapKey.string('alpha'));
      final discardedMissing = removed.discarding(HashMapKey.string('missing'));

      expect(empty.isEmpty, isTrue);
      expect(empty.contains(HashMapKey.string('alpha')), isFalse);
      expect(one.count, equals(1));
      expect(duplicate.count, equals(1));
      expect(two.count, equals(2));
      expect(two.contains(HashMapKey.string('alpha')), isTrue);
      expect(removed.contains(HashMapKey.string('alpha')), isFalse);
      expect(discardedMissing.count, equals(1));
    });

    test('elements use underlying deterministic storage order', () {
      final set = HashSet.fromElements(
        [
          HashMapKey.string('b'),
          HashMapKey.string('d'),
          HashMapKey.string('a'),
          HashMapKey.string('c'),
        ],
        capacity: 4,
        strategy: CollisionStrategy.separateChaining,
        hashAlgorithm: const HashAlgorithm.fnv1a32(),
      );

      expect(
        set.elements,
        equals([
          HashMapKey.string('a'),
          HashMapKey.string('b'),
          HashMapKey.string('c'),
          HashMapKey.string('d'),
        ]),
      );
      expect(set.configuration.strategy,
          equals(CollisionStrategy.separateChaining));
      expect(set.configuration.hashAlgorithm,
          equals(const HashAlgorithm.fnv1a32()));
      expect(set.strategy, equals(CollisionStrategy.separateChaining));
      expect(set.hashAlgorithm, equals(const HashAlgorithm.fnv1a32()));
    });

    test('union is persistent and keeps receiver profile', () {
      final left = HashSet.fromElements(
        leftKeys,
        capacity: 4,
        strategy: CollisionStrategy.linearProbing,
        hashAlgorithm: HashAlgorithm.murmur3(seed: 7),
      );
      final right = HashSet.fromElements([
        HashMapKey.string('beta'),
        HashMapKey.string('delta'),
      ]);
      final union = left.union(right);

      expect(union.count, equals(4));
      for (final key in [...leftKeys, HashMapKey.string('delta')]) {
        expect(union.contains(key), isTrue);
      }
      expect(left.count, equals(3));
      expect(right.count, equals(2));
      expect(union.configuration.strategy,
          equals(CollisionStrategy.linearProbing));
      expect(union.configuration.hashAlgorithm,
          equals(HashAlgorithm.murmur3(seed: 7)));
    });

    test('intersection, difference, and symmetricDifference', () {
      final left = HashSet.fromElements(leftKeys);
      final right = HashSet.fromElements([
        HashMapKey.string('beta'),
        HashMapKey.string('delta'),
      ]);

      final intersection = left.intersection(right);
      expect(intersection.count, equals(1));
      expect(intersection.contains(HashMapKey.string('beta')), isTrue);

      final difference = left.difference(right);
      expect(difference.count, equals(2));
      expect(difference.contains(HashMapKey.string('alpha')), isTrue);
      expect(difference.contains(HashMapKey.string('gamma')), isTrue);

      final symmetric = left.symmetricDifference(right);
      expect(symmetric.count, equals(3));
      expect(symmetric.contains(HashMapKey.string('alpha')), isTrue);
      expect(symmetric.contains(HashMapKey.string('gamma')), isTrue);
      expect(symmetric.contains(HashMapKey.string('delta')), isTrue);
      expect(symmetric.contains(HashMapKey.string('beta')), isFalse);
    });

    test('every algebra result retains the receiver profile', () {
      final left = HashSet.fromElements(
        leftKeys,
        capacity: 8,
        strategy: CollisionStrategy.linearProbing,
        hashAlgorithm: HashAlgorithm.murmur3(seed: 42),
      );
      final right = HashSet.fromElements(
        [
          HashMapKey.string('beta'),
          HashMapKey.string('delta'),
        ],
        capacity: 4,
        strategy: CollisionStrategy.separateChaining,
        hashAlgorithm: const HashAlgorithm.fnv1a32(),
      );

      final results = [
        left.union(right),
        left.intersection(right),
        left.difference(right),
        left.symmetricDifference(right),
      ];
      for (final result in results) {
        expect(result.configuration.strategy,
            equals(CollisionStrategy.linearProbing));
        expect(result.configuration.hashAlgorithm,
            equals(HashAlgorithm.murmur3(seed: 42)));
      }
    });

    test('relations include empty and reflexive cases', () {
      final empty = HashSet();
      final left = HashSet.fromElements([
        HashMapKey.string('alpha'),
        HashMapKey.string('beta'),
      ]);
      final sameDifferentOrder = HashSet.fromElements([
        HashMapKey.string('beta'),
        HashMapKey.string('alpha'),
      ]);
      final larger = left.adding(HashMapKey.string('gamma'));
      final disjoint = HashSet.fromElements([HashMapKey.string('delta')]);

      expect(empty.isSubsetOf(left), isTrue);
      expect(empty.isSubset(left), isTrue);
      expect(empty.isSubsetOf(empty), isTrue);
      expect(left.isSubsetOf(larger), isTrue);
      expect(larger.isSubsetOf(left), isFalse);
      expect(larger.isSupersetOf(left), isTrue);
      expect(larger.isSuperset(left), isTrue);
      expect(left.isDisjointWith(disjoint), isTrue);
      expect(left.isDisjoint(disjoint), isTrue);
      expect(left.isDisjointWith(larger), isFalse);
      expect(left.equals(sameDifferentOrder), isTrue);
      expect(left == sameDifferentOrder, isTrue);
      expect(left.hashCode, equals(sameDifferentOrder.hashCode));
      expect(empty.hashCode, isA<int>());
      expect(left.equals(larger), isFalse);
      expect(left == larger, isFalse);
    });

    test('empty set algebra laws', () {
      final empty = HashSet();
      final populated = HashSet.fromElements(leftKeys);

      expect(empty.union(populated).equals(populated), isTrue);
      expect(populated.union(empty).equals(populated), isTrue);
      expect(empty.intersection(populated).isEmpty, isTrue);
      expect(empty.difference(populated).isEmpty, isTrue);
      expect(populated.difference(empty).equals(populated), isTrue);
      expect(empty.symmetricDifference(populated).equals(populated), isTrue);
    });

    test('both collision strategies survive removal and algebra', () {
      for (final strategy in [
        CollisionStrategy.separateChaining,
        CollisionStrategy.linearProbing,
      ]) {
        final set = HashSet.fromElements(
          [
            HashMapKey.string('k4'),
            HashMapKey.string('k14'),
            HashMapKey.string('k21'),
          ],
          capacity: 8,
          strategy: strategy,
          hashAlgorithm: const HashAlgorithm.fnv1a32(),
        );
        final removed = set.removing(HashMapKey.string('k4'));
        final restored = removed.union(
          HashSet.fromElements([HashMapKey.string('k4')]),
        );

        expect(removed.contains(HashMapKey.string('k14')), isTrue);
        expect(removed.contains(HashMapKey.string('k21')), isTrue);
        expect(removed.contains(HashMapKey.string('k4')), isFalse);
        expect(restored.equals(set), isTrue);
      }
    });

    test('invalid construction and oversized elements use map errors', () {
      expect(() => HashSet(capacity: 0), throwsA(isA<InvalidCapacity>()));
      expect(
        () => HashSet(hashAlgorithm: HashAlgorithm.sipHash24(key: [0])),
        throwsA(isA<InvalidSipHashKeyLength>()),
      );

      final set = HashSet();
      final oversized = HashMapKey.bytes(
        List<int>.filled(HashMapLimits.maximumKeyByteCount + 1, 0),
      );
      expect(() => set.adding(oversized), throwsA(isA<KeyTooLong>()));
      expect(set.contains(oversized), isFalse);
      expect(set.removing(oversized).count, equals(0));
    });
  });

  group('Standing Rule #20 & Diagnostics', () {
    test('diagnostics catalog and sealed exception hierarchy', () {
      final exCap = HashSetException.invalidCapacity();
      expect(exCap.code, equals(HashSetErrorCode.invalidCapacity));
      expect(exCap.errorId, equals('invalid-capacity'));
      expect(exCap.message, contains('initial capacity'));
      expect(exCap.toString(), equals(exCap.message));

      final exBatch = HashSetException.invalidBatchElements();
      expect(exBatch.code, equals(HashSetErrorCode.invalidBatchElements));
      expect(exBatch.errorId, equals('invalid-batch-elements'));

      final exDisjoint = HashSetException.disjointFailed();
      expect(exDisjoint.code, equals(HashSetErrorCode.disjointFailed));
      expect(exDisjoint.errorId, equals('disjoint-failed'));

      final exIncon = HashSetException.internalInconsistency();
      expect(exIncon.code, equals(HashSetErrorCode.internalInconsistency));
      expect(exIncon.errorId, equals('internal-inconsistency'));

      // Check constants
      expect(redactedPlaceholder, equals('<redacted>'));
      expect(invalidPlaceholder, equals('<invalid>'));

      // Check enum values
      expect(HashSetErrorCode.values.length, equals(4));
      for (final code in HashSetErrorCode.values) {
        expect(code.name, isNotEmpty);
      }
    });
  });

  group('5-Class Security Resilience Showcase', () {
    test('Class 1: Memory Exhaustion / Allocation Bomb Defense', () {
      expect(
        () => HashSet(capacity: HashMapLimits.maximumCapacity + 1),
        throwsA(isA<CapacityLimitExceeded>()),
      );
    });

    test('Class 2: Stack Exhaustion Defense (Iterative lookups)', () {
      final set = HashSet.fromElements([
        for (var i = 0; i < 100; i++) HashMapKey.string('key_$i'),
      ]);
      for (var i = 0; i < 50000; i++) {
        expect(set.contains(HashMapKey.string('key_${i % 100}')), isTrue);
      }
    });

    test('Class 3: Key Integrity & Encoding Defense', () {
      final rawKey = HashMapKey.bytes(utf8.encode('secret'));
      final strKey = HashMapKey.string('secret');
      final set = HashSet.fromElements([rawKey, strKey]);
      expect(set.count, equals(2));
      expect(set.contains(rawKey), isTrue);
      expect(set.contains(strKey), isTrue);

      // Malformed UTF-16 surrogate rejection
      expect(
        () => HashMapKey.string('\uD800'),
        throwsA(isA<MalformedUtf16>()),
      );
    });

    test('Class 4: Boundary & State Machine Defense', () {
      final empty = HashSet();
      expect(empty.isEmpty, isTrue);
      expect(empty.isNotEmpty, isFalse);
      expect(empty.count, equals(0));
      expect(empty.capacity, equals(16));
      expect(empty.removing(HashMapKey.string('none')).isEmpty, isTrue);
      expect(empty.contains(HashMapKey.string('none')), isFalse);
    });

    test('Class 5: Diagnostic Payload Blindness', () {
      final ex = HashSetException.invalidCapacity();
      expect(ex.message, isNot(contains('<redacted>')));
      expect(ex.message, isNot(contains('<invalid>')));
      expect(ex.message.startsWith('hash-set:'), isTrue);
    });
  });

  group('Property & Fuzz Testing', () {
    test('200-Case Deterministic PRNG Property Test', () {
      final rng = Random(1337);

      for (var round = 0; round < 200; round++) {
        final countA = rng.nextInt(15);
        final countB = rng.nextInt(15);

        final rawA = <String>{};
        while (rawA.length < countA) {
          rawA.add('elem_${rng.nextInt(30)}');
        }

        final rawB = <String>{};
        while (rawB.length < countB) {
          rawB.add('elem_${rng.nextInt(30)}');
        }

        final keysA = rawA.map(HashMapKey.string).toList();
        final keysB = rawB.map(HashMapKey.string).toList();

        final setA = HashSet.fromElements(keysA);
        final setB = HashSet.fromElements(keysB);

        // Union identity: A ∪ B == B ∪ A
        final unionAB = setA.union(setB);
        final unionBA = setB.union(setA);
        expect(unionAB.equals(unionBA), isTrue);
        expect(unionAB.count, equals(rawA.union(rawB).length));

        // Intersection identity: A ∩ B == B ∩ A
        final interAB = setA.intersection(setB);
        final interBA = setB.intersection(setA);
        expect(interAB.equals(interBA), isTrue);
        expect(interAB.count, equals(rawA.intersection(rawB).length));

        // Difference: A \ B
        final diffAB = setA.difference(setB);
        expect(diffAB.count, equals(rawA.difference(rawB).length));

        // Subset: A is subset of A ∪ B
        expect(setA.isSubsetOf(unionAB), isTrue);
        expect(setB.isSubsetOf(unionAB), isTrue);

        // Disjoint: (A \ B) is disjoint with B
        expect(diffAB.isDisjointWith(setB), isTrue);
      }
    });

    test('10,000-Iteration Adversarial Mutation Fuzz Loop', () {
      final rng = Random(42);
      var set = HashSet();

      for (var iter = 0; iter < 10000; iter++) {
        final op = rng.nextInt(5);
        final keyNum = rng.nextInt(100);
        final key = HashMapKey.string('fuzz_$keyNum');

        switch (op) {
          case 0:
            set = set.adding(key);
            expect(set.contains(key), isTrue);
            break;
          case 1:
            set = set.removing(key);
            expect(set.contains(key), isFalse);
            break;
          case 2:
            set = set.discarding(key);
            expect(set.contains(key), isFalse);
            break;
          case 3:
            final other = HashSet.fromElements([
              HashMapKey.string('fuzz_${rng.nextInt(50)}'),
              HashMapKey.string('fuzz_${rng.nextInt(50)}'),
            ]);
            final union = set.union(other);
            expect(union.count >= set.count, isTrue);
            break;
          case 4:
            final other = HashSet.fromElements([
              HashMapKey.string('fuzz_${rng.nextInt(50)}'),
            ]);
            final inter = set.intersection(other);
            expect(inter.count <= set.count, isTrue);
            break;
        }
      }
    });
  });
}
