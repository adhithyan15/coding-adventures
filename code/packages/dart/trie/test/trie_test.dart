import 'package:coding_adventures_trie/trie.dart';
import 'package:test/test.dart';

void main() {
  group('construction and exact lookup', () {
    test('starts empty with a valid root', () {
      final trie = Trie<String>();

      expect(trie.count, 0);
      expect(trie.isEmpty, isTrue);
      expect(trie.search('anything'), isNull);
      expect(trie.containsKey('anything'), isFalse);
      expect(trie.startsWith(''), isFalse);
      expect(trie.allWords(), isEmpty);
      expect(trie.keys, isEmpty);
      expect(trie.entries, isEmpty);
      expect(trie.isValid(), isTrue);
      expect(trie.toString(), 'Trie(size: 0)');
    });

    test('distinguishes exact keys from paths and longer misses', () {
      final trie = Trie<int>()..insert('hello', 7);

      expect(trie.search('hello'), 7);
      expect(trie.containsKey('hello'), isTrue);
      expect(trie.search('hell'), isNull);
      expect(trie.containsKey('hell'), isFalse);
      expect(trie.search('hellos'), isNull);
      expect(trie.count, 1);
      expect(trie.isEmpty, isFalse);
      expect(trie.isValid(), isTrue);
    });

    test('upserts without changing count', () {
      final trie = Trie<String>()
        ..insert('app', 'first')
        ..insert('app', 'second');

      expect(trie.count, 1);
      expect(trie.search('app'), 'second');
      expect(trie['app'], 'second');
      expect(trie.isValid(), isTrue);
    });

    test('tracks nullable endpoint values independently from presence', () {
      final trie = Trie<String?>()..insert('nullable', null);

      expect(trie.search('nullable'), isNull);
      expect(trie.containsKey('nullable'), isTrue);
      expect(trie['nullable'], isNull);
      expect(trie.count, 1);

      trie.insert('nullable', 'now present');
      expect(trie.count, 1);
      expect(trie.search('nullable'), 'now present');
      expect(trie.isValid(), isTrue);
    });

    test('supports index assignment and rejects a missing indexed lookup', () {
      final trie = Trie<int>();
      trie['answer'] = 42;

      expect(trie['answer'], 42);
      expect(() => trie['missing'], throwsStateError);
      expect(trie.toString(), 'Trie(size: 1)');
    });
  });

  group('prefix operations and deterministic order', () {
    test('shares prefixes and returns scalar-lexicographic results', () {
      final trie = Trie<int>()
        ..insert('apply', 4)
        ..insert('apt', 5)
        ..insert('apple', 2)
        ..insert('app', 1)
        ..insert('application', 3)
        ..insert('banana', 6);

      expect(trie.startsWith('app'), isTrue);
      expect(trie.startsWith('apz'), isFalse);
      expect(
        trie.wordsWithPrefix('app'),
        <TrieEntry<int>>[
          ('app', 1),
          ('apple', 2),
          ('application', 3),
          ('apply', 4),
        ],
      );
      expect(
        trie.allWords(),
        <TrieEntry<int>>[
          ('app', 1),
          ('apple', 2),
          ('application', 3),
          ('apply', 4),
          ('apt', 5),
          ('banana', 6),
        ],
      );
      expect(trie.keys, [
        'app',
        'apple',
        'application',
        'apply',
        'apt',
        'banana',
      ]);
      expect(trie.entries, trie.allWords());
      expect(trie.wordsWithPrefix('missing'), isEmpty);
      expect(trie.isValid(), isTrue);
    });

    test('defines empty key and empty prefix behavior', () {
      final trie = Trie<String>()
        ..insert('branch', 'branch value')
        ..insert('', 'root value');

      expect(trie.containsKey(''), isTrue);
      expect(trie.search(''), 'root value');
      expect(trie.startsWith(''), isTrue);
      expect(
        trie.wordsWithPrefix(''),
        <TrieEntry<String>>[('', 'root value'), ('branch', 'branch value')],
      );
      expect(trie.count, 2);
      expect(trie.isValid(), isTrue);
    });

    test('uses Unicode scalars without normalization or locale collation', () {
      final trie = Trie<String>()
        ..insert('\u{1F600}', 'emoji')
        ..insert('\u00E9', 'precomposed')
        ..insert('e\u0301', 'decomposed')
        ..insert('a', 'ascii');

      expect(trie.search('\u{1F600}'), 'emoji');
      expect(trie.search('\u00E9'), 'precomposed');
      expect(trie.search('e\u0301'), 'decomposed');
      expect(
        trie.keys,
        ['a', 'e\u0301', '\u00E9', '\u{1F600}'],
      );
      expect(trie.count, 4);
      expect(trie.isValid(), isTrue);
    });

    test('is case sensitive', () {
      final trie = Trie<bool>()..insert('Hello', true);

      expect(trie.containsKey('Hello'), isTrue);
      expect(trie.containsKey('hello'), isFalse);
      expect(trie.startsWith('H'), isTrue);
      expect(trie.startsWith('h'), isFalse);
    });
  });

  group('longest prefix match', () {
    test('returns the deepest stored endpoint', () {
      final trie = Trie<int>()
        ..insert('a', 1)
        ..insert('ab', 2)
        ..insert('abc', 3)
        ..insert('abcd', 4)
        ..insert('xyz', 9);

      expect(trie.longestPrefixMatch('abcde'), ('abcd', 4));
      expect(trie.longestPrefixMatch('abc'), ('abc', 3));
      expect(trie.longestPrefixMatch('a'), ('a', 1));
      expect(trie.longestPrefixMatch('no-match'), isNull);
    });

    test('allows the empty key to be the longest available prefix', () {
      final trie = Trie<String>()
        ..insert('', 'fallback')
        ..insert('api', 'api route');

      expect(trie.longestPrefixMatch('unknown'), ('', 'fallback'));
      expect(trie.longestPrefixMatch('api/v1'), ('api', 'api route'));
      expect(trie.longestPrefixMatch(''), ('', 'fallback'));
    });
  });

  group('deletion and pruning', () {
    test('deletes a leaf and reports missing deletion as a no-op', () {
      final trie = Trie<int>()..insert('apple', 1);

      expect(trie.delete('apple'), isTrue);
      expect(trie.delete('apple'), isFalse);
      expect(trie.containsKey('apple'), isFalse);
      expect(trie.startsWith('a'), isFalse);
      expect(trie.count, 0);
      expect(trie.isEmpty, isTrue);
      expect(trie.isValid(), isTrue);
    });

    test('deletes an endpoint while preserving its descendants', () {
      final trie = Trie<int>()
        ..insert('app', 1)
        ..insert('apple', 2);

      expect(trie.delete('app'), isTrue);
      expect(trie.search('app'), isNull);
      expect(trie.search('apple'), 2);
      expect(trie.wordsWithPrefix('app'), <TrieEntry<int>>[('apple', 2)]);
      expect(trie.count, 1);
      expect(trie.isValid(), isTrue);
    });

    test('deletes a descendant while preserving a shared endpoint', () {
      final trie = Trie<int>()
        ..insert('app', 1)
        ..insert('apple', 2)
        ..insert('apt', 3);

      expect(trie.delete('apple'), isTrue);
      expect(trie.search('app'), 1);
      expect(trie.search('apt'), 3);
      expect(trie.startsWith('appl'), isFalse);
      expect(trie.count, 2);
      expect(trie.isValid(), isTrue);
    });

    test('deletes the empty key without removing non-empty branches', () {
      final trie = Trie<String>()
        ..insert('', 'root')
        ..insert('child', 'value');

      expect(trie.delete(''), isTrue);
      expect(trie.containsKey(''), isFalse);
      expect(trie.search('child'), 'value');
      expect(trie.startsWith(''), isTrue);
      expect(trie.count, 1);
      expect(trie.isValid(), isTrue);
    });

    test('returns to a canonical empty root after deleting every key', () {
      final trie = Trie<int>();
      for (final key in ['a', 'ab', 'abc', 'b', 'ba']) {
        trie.insert(key, key.length);
      }
      for (final key in ['ab', 'a', 'ba', 'abc', 'b']) {
        expect(trie.delete(key), isTrue);
        expect(trie.isValid(), isTrue);
      }

      expect(trie.count, 0);
      expect(trie.allWords(), isEmpty);
      expect(trie.startsWith(''), isFalse);
    });
  });

  test('long keys do not depend on recursive host-stack depth', () {
    final repeatedA = List<String>.filled(50000, 'a').join();
    final longKey = '$repeatedA\u{1F600}';
    final trie = Trie<int>()..insert(longKey, 99);

    expect(trie.search(longKey), 99);
    expect(
      trie.startsWith(List<String>.filled(40000, 'a').join()),
      isTrue,
    );
    expect(trie.longestPrefixMatch('${longKey}suffix'), (longKey, 99));
    expect(trie.allWords(), <TrieEntry<int>>[(longKey, 99)]);
    expect(trie.isValid(), isTrue);
    expect(trie.delete(longKey), isTrue);
    expect(trie.isValid(), isTrue);
    expect(trie.isEmpty, isTrue);
  });
}
