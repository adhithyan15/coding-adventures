import 'package:coding_adventures_huffman_tree/huffman_tree.dart';
import 'package:test/test.dart';

void main() {
  group('HuffmanTree.build validation', () {
    test('rejects empty weights', () {
      expect(
        () => HuffmanTree.build(<(int, int)>[]),
        throwsA(
          isA<ArgumentError>().having(
            (error) => error.message,
            'message',
            'weights must not be empty',
          ),
        ),
      );
    });

    test('rejects non-positive frequencies', () {
      expect(
        () => HuffmanTree.build(<(int, int)>[(42, 0)]),
        throwsA(
          isA<ArgumentError>().having(
            (error) => error.message,
            'message',
            contains('symbol=42, freq=0'),
          ),
        ),
      );
    });

    test('rejects duplicate symbols', () {
      expect(
        () => HuffmanTree.build(<(int, int)>[(65, 2), (65, 1)]),
        throwsA(
          isA<ArgumentError>().having(
            (error) => error.message,
            'message',
            contains('symbol 65 appears more than once'),
          ),
        ),
      );
    });
  });

  group('single-symbol tree', () {
    final tree = HuffmanTree.build(<(int, int)>[(65, 5)]);

    test('reports basic inspection values', () {
      expect(tree.symbolCount(), 1);
      expect(tree.weight(), 5);
      expect(tree.depth(), 0);
      expect(tree.isValid(), isTrue);
    });

    test('uses 0 for both walk and canonical tables', () {
      expect(tree.codeTable(), <int, String>{65: '0'});
      expect(tree.canonicalCodeTable(), <int, String>{65: '0'});
      expect(tree.codeFor(65), '0');
      expect(tree.codeFor(99), isNull);
    });

    test('decodes repeated symbols, even with an empty bit string', () {
      expect(tree.decodeAll('000', 3), <int>[65, 65, 65]);
      expect(tree.decodeAll('', 1), <int>[65]);
    });

    test('returns the in-order leaves', () {
      expect(tree.leaves(), <(int, String)>[(65, '0')]);
    });
  });

  test('Leaf and Internal value types expose stable equality and text', () {
    const left = Leaf(65, 3);
    const right = Leaf(66, 2);
    const internalA = Internal(left: left, right: right, weight: 5, order: 7);
    const internalB = Internal(left: left, right: right, weight: 5, order: 7);

    expect(left, const Leaf(65, 3));
    expect(left.hashCode, const Leaf(65, 3).hashCode);
    expect(left.toString(), 'Leaf(symbol: 65, weight: 3)');
    expect(internalA, internalB);
    expect(internalA.hashCode, internalB.hashCode);
    expect(
      internalA.toString(),
      'Internal(weight: 5, order: 7, left: Leaf(symbol: 65, weight: 3), right: Leaf(symbol: 66, weight: 2))',
    );
  });

  group('AAABBC example', () {
    final tree = HuffmanTree.build(<(int, int)>[(65, 3), (66, 2), (67, 1)]);

    test('builds the expected deterministic walk table', () {
      expect(tree.symbolCount(), 3);
      expect(tree.weight(), 6);
      expect(tree.depth(), 2);
      expect(tree.codeTable(), <int, String>{65: '0', 67: '10', 66: '11'});
      expect(tree.codeFor(67), '10');
      expect(tree.codeFor(66), '11');
      expect(tree.codeFor(99), isNull);
    });

    test('builds the expected canonical table', () {
      expect(tree.canonicalCodeTable(), <int, String>{
        65: '0',
        66: '10',
        67: '11',
      });
    });

    test('decodes the expected symbol sequence', () {
      expect(tree.decodeAll('001011', 4), <int>[65, 65, 67, 66]);
      expect(
        () => tree.decodeAll('1', 1),
        throwsA(
          isA<FormatException>().having(
            (error) => error.message,
            'message',
            contains('Bit stream exhausted'),
          ),
        ),
      );
    });

    test('lists leaves in-order and validates structure', () {
      expect(tree.leaves(), <(int, String)>[(65, '0'), (67, '10'), (66, '11')]);
      expect(tree.isValid(), isTrue);
    });
  });

  group('tie-breaking', () {
    test('prefers leaves over internal nodes at equal weight', () {
      final tree = HuffmanTree.build(<(int, int)>[(65, 1), (66, 1), (67, 2)]);
      expect(tree.codeTable(), <int, String>{67: '0', 65: '10', 66: '11'});
    });

    test('prefers lower symbols among equal-weight leaves', () {
      final tree = HuffmanTree.build(<(int, int)>[(66, 1), (65, 1)]);
      expect(tree.codeFor(65), '0');
      expect(tree.codeFor(66), '1');
    });

    test('uses FIFO order for equal-weight internal nodes', () {
      final tree = HuffmanTree.build(<(int, int)>[
        (65, 1),
        (66, 1),
        (67, 1),
        (68, 1),
      ]);
      expect(tree.codeTable(), <int, String>{
        65: '00',
        66: '01',
        67: '10',
        68: '11',
      });
      expect(tree.canonicalCodeTable(), tree.codeTable());
    });
  });

  test('larger trees remain prefix-free and round-trip through decodeAll', () {
    final tree = HuffmanTree.build(<(int, int)>[
      (65, 15),
      (66, 7),
      (67, 6),
      (68, 6),
      (69, 5),
    ]);
    final table = tree.codeTable();
    final codes = table.values.toList();

    for (var left = 0; left < codes.length; left += 1) {
      for (var right = 0; right < codes.length; right += 1) {
        if (left == right) {
          continue;
        }
        expect(codes[right].startsWith(codes[left]), isFalse);
      }
    }

    final message = <int>[65, 65, 66, 67, 69];
    final bits = message.map((symbol) => table[symbol]!).join();
    expect(tree.decodeAll(bits, message.length), message);
    expect(tree.isValid(), isTrue);
  });

  test('decodeAll rejects characters other than 0 and 1', () {
    final tree = HuffmanTree.build(<(int, int)>[(65, 2), (66, 1)]);
    expect(
      () => tree.decodeAll('2', 1),
      throwsA(
        isA<FormatException>().having(
          (error) => error.message,
          'message',
          contains('must contain only 0 and 1'),
        ),
      ),
    );
  });

  test('decodeAll rejects negative counts', () {
    final tree = HuffmanTree.build(<(int, int)>[(65, 1)]);
    expect(
      () => tree.decodeAll('0', -1),
      throwsA(
        isA<RangeError>().having(
          (error) => error.message,
          'message',
          contains('Must be non-negative'),
        ),
      ),
    );
  });
}
