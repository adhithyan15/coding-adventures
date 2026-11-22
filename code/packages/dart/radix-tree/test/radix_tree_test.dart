import 'dart:collection';
import 'dart:math' as math;

import 'package:coding_adventures_radix_tree/radix_tree.dart';
import 'package:test/test.dart';

void main() {
  group('5-Class Radix-Tree Security Resilience Showcase', () {
    test('Class 1: CWE-400 Deep-Key Stack Overflow & Key-Length DoS', () {
      final tree = RadixTree<int>();

      // Exact ceiling: 65,536 scalars is valid and accepted
      final atCeilingKey = 'a' * maxKeyScalars;
      expect(tree.insert(atCeilingKey, 1), isTrue);
      expect(tree.search(atCeilingKey), equals(1));
      expect(tree.delete(atCeilingKey), isTrue);
      expect(tree.isEmpty, isTrue);

      // Over ceiling: 65,537 scalars is rejected across all public methods
      final overCeilingKey = 'a' * (maxKeyScalars + 1);
      expect(
        () => tree.insert(overCeilingKey, 1),
        throwsA(isA<InvalidRadixKeyException>().having(
          (e) => e.message,
          'message',
          messageKeyTooLong,
        )),
      );
      expect(() => tree.put(overCeilingKey, 1),
          throwsA(isA<InvalidRadixKeyException>()));
      expect(() => tree.search(overCeilingKey),
          throwsA(isA<InvalidRadixKeyException>()));
      expect(() => tree.get(overCeilingKey),
          throwsA(isA<InvalidRadixKeyException>()));
      expect(() => tree.containsKey(overCeilingKey),
          throwsA(isA<InvalidRadixKeyException>()));
      expect(
          () => tree[overCeilingKey], throwsA(isA<InvalidRadixKeyException>()));
      expect(() => tree[overCeilingKey] = 1,
          throwsA(isA<InvalidRadixKeyException>()));
      expect(() => tree.delete(overCeilingKey),
          throwsA(isA<InvalidRadixKeyException>()));
      expect(() => tree.startsWith(overCeilingKey),
          throwsA(isA<InvalidRadixKeyException>()));
      expect(() => tree.wordsWithPrefix(overCeilingKey),
          throwsA(isA<InvalidRadixKeyException>()));
      expect(() => tree.keysWithPrefix(overCeilingKey),
          throwsA(isA<InvalidRadixKeyException>()));
      expect(() => tree.longestPrefixMatch(overCeilingKey),
          throwsA(isA<InvalidRadixKeyException>()));
      expect(() => tree.longestPrefixOf(overCeilingKey),
          throwsA(isA<InvalidRadixKeyException>()));

      // 50,000-scalar deep key: safe without call stack recursion
      const deepScalarCount = 50000;
      final deepKey = 'z' * deepScalarCount;
      expect(tree.insert(deepKey, 42), isTrue);
      expect(tree.count, equals(1));
      expect(tree.search(deepKey), equals(42));
      expect(tree.containsKey(deepKey), isTrue);
      expect(tree.startsWith(deepKey.substring(0, 25000)), isTrue);

      final prefixResults = tree.wordsWithPrefix(deepKey.substring(0, 100));
      expect(prefixResults.length, equals(1));
      expect(prefixResults.first.key, equals(deepKey));
      expect(prefixResults.first.value, equals(42));

      final lpm = tree.longestPrefixMatch('${deepKey}_tail');
      expect(lpm, isNotNull);
      expect(lpm!.key, equals(deepKey));

      expect(tree.delete(deepKey), isTrue);
      expect(tree.isEmpty, isTrue);
      expect(tree.isValid(), isTrue);
    });

    test('Class 2: Surrogate-Pair Tearing & Malformed UTF-16 Smuggling', () {
      final tree = RadixTree<int>()..insert('baseline', 10);

      final loneLead = String.fromCharCode(0xd800);
      final loneTrail = String.fromCharCode(0xdc00);
      final reversedPair = String.fromCharCodes([0xdc00, 0xd800]);
      final midSurrogate = 'safe${loneLead}test';
      final trailingSurrogate = 'baseline$loneTrail';

      for (final invalid in [
        loneLead,
        loneTrail,
        reversedPair,
        midSurrogate,
        trailingSurrogate,
      ]) {
        expect(
          () => tree.insert(invalid, 99),
          throwsA(isA<InvalidRadixKeyException>().having(
            (e) => e.message,
            'message',
            messageInvalidScalar,
          )),
        );
        expect(() => tree.search(invalid),
            throwsA(isA<InvalidRadixKeyException>()));
        expect(() => tree.containsKey(invalid),
            throwsA(isA<InvalidRadixKeyException>()));
        expect(() => tree[invalid], throwsA(isA<InvalidRadixKeyException>()));
        expect(() => tree.delete(invalid),
            throwsA(isA<InvalidRadixKeyException>()));
        expect(() => tree.startsWith(invalid),
            throwsA(isA<InvalidRadixKeyException>()));
        expect(() => tree.wordsWithPrefix(invalid),
            throwsA(isA<InvalidRadixKeyException>()));
        expect(() => tree.longestPrefixMatch(invalid),
            throwsA(isA<InvalidRadixKeyException>()));
      }

      // Verify tree state remained uncorrupted
      expect(tree.count, equals(1));
      expect(tree.search('baseline'), equals(10));
      expect(tree.isValid(), isTrue);

      // Astral emoji test: 😀 (U+1F600) vs 😁 (U+1F601)
      // Both share high surrogate 0xD83D in UTF-16, but in RadixTree they are
      // atomic 32-bit Unicode scalars. Edges must NOT split on 0xD83D!
      final emojiTree = RadixTree<String>()
        ..insert('😀', 'grinning')
        ..insert('😁', 'beaming');

      expect(
          emojiTree.nodeCount(), equals(3)); // root + 2 distinct child leaves
      expect(emojiTree.edgeCount(), equals(2));
      expect(emojiTree.search('😀'), equals('grinning'));
      expect(emojiTree.search('😁'), equals('beaming'));
      expect(emojiTree.isValid(), isTrue);

      // Strict Unicode scalar ordering: 'a' < 'e\u0301' < '\u00E9' < '\u{1F600}'
      final orderedTree = RadixTree<int>()
        ..insert('😀', 4)
        ..insert('\u00e9', 3)
        ..insert('e\u0301', 2)
        ..insert('a', 1);

      expect(orderedTree.keys, equals(['a', 'e\u0301', '\u00e9', '😀']));
    });

    test('Class 3: Compression & Re-Compression Structural Integrity', () {
      final tree = RadixTree<int>()
        ..insert('search', 1)
        ..insert('searcher', 2)
        ..insert('searching', 3);

      // In an uncompressed trie, ["search", "searcher", "searching"] requires 14 nodes.
      // In RadixTree, it requires exactly 4 nodes and 3 edges:
      // root -> "search" (isEnd) -> "er" (isEnd) and "ing" (isEnd)
      expect(tree.nodeCount(), equals(4));
      expect(tree.edgeCount(), equals(3));
      expect(tree.isValid(), isTrue);

      // Deletion 1: Delete "searching".
      // Node "search" now has only 1 child ("er"), but isEnd is true, so no merge occurs.
      expect(tree.delete('searching'), isTrue);
      expect(tree.nodeCount(), equals(3));
      expect(tree.edgeCount(), equals(2));
      expect(tree.isValid(), isTrue);

      // Deletion 2: Delete "search".
      // Node "search" now has isEnd == false and 1 child ("er").
      // Merge-back re-compresses root -> "search" -> "er" into root -> "searcher"!
      expect(tree.delete('search'), isTrue);
      expect(tree.nodeCount(), equals(2)); // root + node for "searcher"
      expect(tree.edgeCount(), equals(1));
      expect(tree.search('searcher'), equals(2));
      expect(tree.isValid(), isTrue);

      // Deletion 3: Delete "searcher". Tree returns to empty root.
      expect(tree.delete('searcher'), isTrue);
      expect(tree.nodeCount(), equals(1));
      expect(tree.edgeCount(), equals(0));
      expect(tree.isEmpty, isTrue);
      expect(tree.isValid(), isTrue);

      // Cascading multi-level merge-back test
      final cascade = RadixTree<int>()
        ..insert('alpha/beta/gamma/one', 1)
        ..insert('alpha/beta/gamma/two', 2);
      expect(cascade.nodeCount(), equals(4));
      expect(cascade.edgeCount(), equals(3));

      // Deleting 'alpha/beta/gamma/two' collapses the inner node into single edge
      expect(cascade.delete('alpha/beta/gamma/two'), isTrue);
      expect(cascade.nodeCount(), equals(2));
      expect(cascade.edgeCount(), equals(1));
      expect(cascade.search('alpha/beta/gamma/one'), equals(1));
      expect(cascade.isValid(), isTrue);
    });

    test('Class 4: Nullable Value Ambiguity & Caller-Mutation Isolation', () {
      final tree = RadixTree<String?>()
        ..insert('explicitNull', null)
        ..insert('validValue', 'present');

      // Decouple presence from nullability
      expect(tree.containsKey('explicitNull'), isTrue);
      expect(tree.search('explicitNull'), isNull);
      expect(tree['explicitNull'], isNull);

      expect(tree.containsKey('missingKey'), isFalse);
      expect(tree.search('missingKey'), isNull);
      expect(
          () => tree['missingKey'], throwsA(isA<RadixKeyNotFoundException>()));

      // Mutation isolation on returned snapshots
      final keys = tree.keys;
      expect(() => (keys as dynamic).add('tamper'), throwsUnsupportedError);

      final entries = tree.entries;
      expect(() => (entries as dynamic).add(const RadixEntry('tamper', null)),
          throwsUnsupportedError);

      final prefixWords = tree.wordsWithPrefix('exp');
      expect(() => (prefixWords as dynamic).clear(), throwsUnsupportedError);

      final prefixKeys = tree.keysWithPrefix('exp');
      expect(() => (prefixKeys as dynamic).clear(), throwsUnsupportedError);

      final map = tree.toMap();
      expect(() => (map as dynamic)['new'] = 'tamper', throwsUnsupportedError);
    });

    test('Class 5: CWE-209 Payload-Blind Confidentiality', () {
      final tree = RadixTree<String>()
        ..insert('superSecretToken', 'secretValue');

      // operator [] on missing secret key throws closed, static exception
      try {
        final _ = tree['absentSecretToken'];
        fail('Expected RadixKeyNotFoundException');
      } on RadixKeyNotFoundException catch (e) {
        expect(e.errorCode, equals(RadixTreeErrorCode.keyNotFound));
        expect(e.errorId, equals('key-not-found'));
        expect(e.message, equals('Radix tree key not found.'));
        expect(e.toString(), contains('Radix tree key not found.'));
        expect(e.toString().contains('absentSecretToken'), isFalse);
      }

      // Invalid scalar exception never reflects secret payload
      try {
        tree.search('secret\ud800key');
        fail('Expected InvalidRadixKeyException');
      } on InvalidRadixKeyException catch (e) {
        expect(e.errorCode, equals(RadixTreeErrorCode.invalidKey));
        expect(e.errorId, equals('invalid-key'));
        expect(e.message,
            equals('Radix tree key contains an invalid Unicode scalar.'));
        expect(e.toString().contains('secret'), isFalse);
      }

      // RadixTree.toString() is strictly structural and redacted
      final treeStr = tree.toString();
      expect(
          treeStr, equals('RadixTree(count: 1, nodeCount: 2, edgeCount: 1)'));
      expect(treeStr.contains('superSecretToken'), isFalse);
      expect(treeStr.contains('secretValue'), isFalse);

      // RadixEntry.toString() uses redacted placeholders
      const entry = RadixEntry('confidentialPassword', 'superSecret');
      expect(entry.toString(), equals('RadixEntry(<redacted>, <redacted>)'));
      expect(entry.toString().contains('confidentialPassword'), isFalse);
      expect(entry.toString().contains('superSecret'), isFalse);
    });
  });

  group('RadixTree Core Operations & Edge Cases', () {
    test('supports empty string root key', () {
      final tree = RadixTree<int>();
      expect(tree.insert('', 100), isTrue);
      expect(tree.count, equals(1));
      expect(tree.size, equals(1));
      expect(tree.search(''), equals(100));
      expect(tree.get(''), equals(100));
      expect(tree.containsKey(''), isTrue);
      expect(tree[''], equals(100));
      expect(tree.isValid(), isTrue);

      // Update empty string value
      expect(tree.insert('', 200), isFalse);
      expect(tree.count, equals(1));
      expect(tree[''], equals(200));

      // Prefix matches
      expect(tree.startsWith(''), isTrue);
      expect(tree.wordsWithPrefix(''), equals([const RadixEntry('', 200)]));

      // Longest prefix match on empty string
      final lpm = tree.longestPrefixMatch('anything');
      expect(lpm, isNotNull);
      expect(lpm!.key, equals(''));
      expect(lpm.value, equals(200));
      expect(tree.longestPrefixOf('anything'), equals(''));

      // Delete empty string
      expect(tree.delete(''), isTrue);
      expect(tree.isEmpty, isTrue);
      expect(tree.delete(''), isFalse);
    });

    test('handles all 4 DT14 insertion cases and exact updates', () {
      final tree = RadixTree<String>();

      // Case 1: No common prefix ("apple" vs "banana")
      expect(tree.insert('apple', 'fruit'), isTrue);
      expect(tree.insert('banana', 'fruit'), isTrue);
      expect(tree.count, equals(2));
      expect(tree.isValid(), isTrue);

      // Case 2: Edge label is a prefix of new key ("apple" -> "applesauce")
      expect(tree.insert('applesauce', 'food'), isTrue);
      expect(tree.count, equals(3));
      expect(tree.search('applesauce'), equals('food'));
      expect(tree.isValid(), isTrue);

      // Case 3: New key is strict prefix of edge label ("banana" split by "ban")
      expect(tree.insert('ban', 'action'), isTrue);
      expect(tree.count, equals(4));
      expect(tree.search('ban'), equals('action'));
      expect(tree.search('banana'), equals('fruit'));
      expect(tree.isValid(), isTrue);

      // Case 4: Partial match ("banana" vs "bandana")
      expect(tree.insert('bandana', 'cloth'), isTrue);
      expect(tree.count, equals(5));
      expect(tree.search('bandana'), equals('cloth'));
      expect(tree.search('banana'), equals('fruit'));
      expect(tree.search('ban'), equals('action'));
      expect(tree.isValid(), isTrue);

      // Exact match update
      expect(tree.insert('apple', 'green_fruit'), isFalse);
      expect(tree.count, equals(5));
      expect(tree.search('apple'), equals('green_fruit'));
      expect(tree.isValid(), isTrue);
    });

    test('constructors: fromEntries and fromMap', () {
      final entries = [
        const RadixEntry('k1', 1),
        const RadixEntry('k2', 2),
      ];
      final tree1 = RadixTree.fromEntries(entries);
      expect(tree1.count, equals(2));
      expect(tree1['k1'], equals(1));
      expect(tree1['k2'], equals(2));

      final map = {'a': 10, 'b': 20, 'c': 30};
      final tree2 = RadixTree.fromMap(map);
      expect(tree2.count, equals(3));
      expect(tree2.toMap(), equals(map));
    });

    test('longestPrefixMatch and longestPrefixOf IP routing behavior', () {
      final routingTable = RadixTree<String>()
        ..insert('10', 'net-10')
        ..insert('10.0', 'net-10.0')
        ..insert('10.0.0', 'net-10.0.0');

      expect(routingTable.longestPrefixOf('10.0.0.1'), equals('10.0.0'));
      expect(routingTable.longestPrefixMatch('10.0.0.1')?.value,
          equals('net-10.0.0'));

      expect(routingTable.longestPrefixOf('10.0.1.1'), equals('10.0'));
      expect(routingTable.longestPrefixOf('10.1.1.1'), equals('10'));
      expect(routingTable.longestPrefixOf('192.168.1.1'), isNull);
      expect(routingTable.longestPrefixMatch('192.168.1.1'), isNull);
    });

    test('startsWith and prefix queries', () {
      final tree = RadixTree<int>()
        ..insert('cat', 1)
        ..insert('caterpillar', 2)
        ..insert('catastrophe', 3)
        ..insert('dog', 4);

      expect(tree.isNotEmpty, isTrue);
      expect(tree.startsWith('cat'), isTrue);
      expect(tree.startsWith('cater'), isTrue);
      expect(tree.startsWith('catastrophic'), isFalse);
      expect(tree.startsWith('cow'), isFalse);
      expect(tree.startsWith('zebra'), isFalse);

      // Edge partial match mid-label divergence in wordsWithPrefix
      expect(tree.wordsWithPrefix('catb'), isEmpty);

      // Exact node boundary in wordsWithPrefix
      expect(tree.wordsWithPrefix('cat').length, equals(3));

      expect(tree.keysWithPrefix('cat'),
          equals(['cat', 'catastrophe', 'caterpillar']));
      expect(tree.keysWithPrefix('dog'), equals(['dog']));
      expect(tree.keysWithPrefix('missing'), isEmpty);
      expect(tree.wordsWithPrefix('missing'), isEmpty);
    });

    test('delete missing key or non-matching prefix is a no-op', () {
      final tree = RadixTree<int>()
        ..insert('apple', 1)
        ..insert('application', 2);

      expect(tree.delete('app'), isFalse);
      expect(tree.delete('apples'), isFalse);
      expect(tree.delete('banana'), isFalse);
      expect(tree.count, equals(2));
    });

    test('RadixEntry accessors, equality, and hash code', () {
      const e1 = RadixEntry('foo', 42);
      const e2 = RadixEntry('foo', 42);
      const e3 = RadixEntry('foo', 99);
      const e4 = RadixEntry('bar', 42);

      expect(e1.$1, equals('foo'));
      expect(e1.$2, equals(42));
      expect(e1.key, equals('foo'));
      expect(e1.value, equals(42));
      expect(e1, equals(e2));
      expect(e1.hashCode, equals(e2.hashCode));
      expect(e1 == e3, isFalse);
      expect(e1 == e4, isFalse);
    });

    test('operator []= updates and inserts correctly', () {
      final tree = RadixTree<int>();
      tree['alpha'] = 1;
      expect(tree['alpha'], equals(1));
      tree['alpha'] = 2;
      expect(tree['alpha'], equals(2));
      expect(tree.count, equals(1));
    });
  });

  group('Deterministic PRNG Differential Property Test', () {
    test(
        '200 operations differential test vs SplayTreeMap with continuous isValid()',
        () {
      final tree = RadixTree<int>();
      final oracle = SplayTreeMap<String, int>();
      final rng = math.Random(42);

      final sampleWords = [
        '',
        'a',
        'aa',
        'ab',
        'abc',
        'abcd',
        'abce',
        'abd',
        'b',
        'ba',
        'banana',
        'bandana',
        'cat',
        'cater',
        'caterpillar',
        'dog',
        'dorm',
        '😀',
        '😁',
        '😀😁',
        'é',
        'e\u0301',
      ];

      for (var op = 0; op < 200; op++) {
        final action = rng.nextInt(6);
        final key = sampleWords[rng.nextInt(sampleWords.length)];

        switch (action) {
          case 0: // insert / put
          case 1:
            final value = rng.nextInt(1000);
            final treeNew = tree.insert(key, value);
            final oracleNew = !oracle.containsKey(key);
            oracle[key] = value;
            expect(treeNew, equals(oracleNew));
            expect(tree.count, equals(oracle.length));
            expect(tree.isValid(), isTrue);
            break;

          case 2: // delete
            final treeDeleted = tree.delete(key);
            final oracleDeleted = oracle.remove(key) != null;
            expect(treeDeleted, equals(oracleDeleted));
            expect(tree.count, equals(oracle.length));
            expect(tree.isValid(), isTrue);
            break;

          case 3: // search & containsKey
            final treeVal = tree.search(key);
            final oracleVal = oracle[key];
            expect(treeVal, equals(oracleVal));
            expect(tree.containsKey(key), equals(oracle.containsKey(key)));
            break;

          case 4: // keys and entries ordering
            expect(tree.keys, equals(oracle.keys.toList()));
            expect(tree.toMap(), equals(oracle));
            break;

          case 5: // prefix search
            final runes = key.runes.toList();
            final prefix = runes.isEmpty
                ? ''
                : String.fromCharCodes(
                    runes.take(rng.nextInt(runes.length + 1)));
            final expectedKeys =
                oracle.keys.where((k) => k.startsWith(prefix)).toList();
            expect(tree.keysWithPrefix(prefix), equals(expectedKeys));
            expect(tree.startsWith(prefix), equals(expectedKeys.isNotEmpty));
            break;
        }
      }

      expect(tree.isValid(), isTrue);
    });
  });

  group('10,000-Iteration Adversarial Mutation Fuzz Loop', () {
    test('10,000 random mutations preserve invariants and correctness', () {
      final tree = RadixTree<int>();
      final oracle = <String, int>{};
      final rng = math.Random(1337);

      final alphabet = ['a', 'b', 'c', 'd', 'e', '😀', '😁', 'e\u0301'];

      String randomKey() {
        if (rng.nextInt(20) == 0) return '';
        final len = rng.nextInt(6) + 1;
        final buffer = StringBuffer();
        for (var i = 0; i < len; i++) {
          buffer.write(alphabet[rng.nextInt(alphabet.length)]);
        }
        return buffer.toString();
      }

      for (var iter = 0; iter < 10000; iter++) {
        final key = randomKey();
        if (rng.nextBool()) {
          final val = rng.nextInt(500);
          tree.insert(key, val);
          oracle[key] = val;
        } else {
          tree.delete(key);
          oracle.remove(key);
        }

        if (iter % 1000 == 0) {
          expect(tree.count, equals(oracle.length));
          expect(tree.isValid(), isTrue);
        }
      }

      expect(tree.count, equals(oracle.length));
      expect(tree.isValid(), isTrue);

      for (final MapEntry(:key, :value) in oracle.entries) {
        expect(tree.search(key), equals(value));
      }
    });
  });
}
