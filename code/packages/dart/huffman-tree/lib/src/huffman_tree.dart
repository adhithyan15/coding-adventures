/// DT27: Huffman tree construction with deterministic tie-breaking.
///
/// A Huffman tree gives each symbol a prefix-free bit string where common
/// symbols get shorter codes than rare ones. The implementation here keeps the
/// build deterministic across languages by using the spec's exact tie-break
/// order:
///
/// 1. lower weight first
/// 2. leaves before internal nodes at equal weight
/// 3. lower symbol value first among equal-weight leaves
/// 4. FIFO order among equal-weight internal nodes
///
/// That determinism matters because CMP04 Huffman compression stores only the
/// canonical code lengths. If every language builds the same tree shape, every
/// language also derives the same canonical table.

/// A node in the Huffman tree.
sealed class Node {
  const Node();

  /// The combined weight of this node's subtree.
  int get weight;
}

/// A leaf node that stores one symbol and its frequency.
final class Leaf extends Node {
  const Leaf(this.symbol, this.weight);

  /// Symbol value carried by this leaf.
  final int symbol;

  @override
  final int weight;

  @override
  bool operator ==(Object other) =>
      other is Leaf && other.symbol == symbol && other.weight == weight;

  @override
  int get hashCode => Object.hash(symbol, weight);

  @override
  String toString() => 'Leaf(symbol: $symbol, weight: $weight)';
}

/// An internal node created by merging two lighter subtrees.
final class Internal extends Node {
  const Internal({
    required this.left,
    required this.right,
    required this.weight,
    required this.order,
  });

  /// Left subtree, corresponding to a `'0'` edge while encoding.
  final Node left;

  /// Right subtree, corresponding to a `'1'` edge while encoding.
  final Node right;

  @override
  final int weight;

  /// Monotonic creation order used for FIFO tie-breaking.
  final int order;

  @override
  bool operator ==(Object other) =>
      other is Internal &&
      other.left == left &&
      other.right == right &&
      other.weight == weight &&
      other.order == order;

  @override
  int get hashCode => Object.hash(left, right, weight, order);

  @override
  String toString() =>
      'Internal(weight: $weight, order: $order, left: $left, right: $right)';
}

/// Deterministic Huffman tree with helpers for code generation and decoding.
final class HuffmanTree {
  const HuffmanTree._(this._root, this._symbolCount);

  final Node _root;
  final int _symbolCount;

  /// Build a Huffman tree from `(symbol, frequency)` pairs.
  ///
  /// Frequencies must be positive, and each symbol may appear at most once.
  /// Duplicate symbols would create an invalid code table where one symbol had
  /// multiple leaves, so the constructor rejects them up front.
  static HuffmanTree build(List<(int, int)> weights) {
    if (weights.isEmpty) {
      throw ArgumentError('weights must not be empty');
    }

    final seenSymbols = <int>{};
    final heap = _MinHeap<_HeapEntry>(_compareHeapEntries);
    var orderCounter = 0;

    for (final (symbol, frequency) in weights) {
      if (frequency <= 0) {
        throw ArgumentError(
          'frequency must be positive; got symbol=$symbol, freq=$frequency',
        );
      }
      if (!seenSymbols.add(symbol)) {
        throw ArgumentError('symbol $symbol appears more than once');
      }
      heap.push(_HeapEntry.leaf(Leaf(symbol, frequency)));
    }

    while (heap.length > 1) {
      final left = heap.pop().node;
      final right = heap.pop().node;
      final internal = Internal(
        left: left,
        right: right,
        weight: left.weight + right.weight,
        order: orderCounter,
      );
      orderCounter += 1;
      heap.push(_HeapEntry.internal(internal));
    }

    return HuffmanTree._(heap.pop().node, weights.length);
  }

  /// Return `{symbol -> bit_string}` from a left=`0`, right=`1` tree walk.
  ///
  /// Single-symbol trees use `'0'` by convention. A zero-length code would be
  /// mathematically valid, but one-bit codes keep the teaching examples and the
  /// compression wire format straightforward.
  Map<int, String> codeTable() {
    final table = <int, String>{};
    _walkTree(_root, '', table);
    return Map<int, String>.unmodifiable(table);
  }

  /// Return the bit string for [symbol], or `null` if the symbol is absent.
  String? codeFor(int symbol) => _findCode(_root, symbol, '');

  /// Return the canonical Huffman code table.
  ///
  /// Canonical codes preserve the code lengths from the tree walk but normalize
  /// the actual bit patterns so the decoder only needs `(symbol, length)` pairs
  /// to reconstruct the table.
  Map<int, String> canonicalCodeTable() {
    final lengths = <int, int>{};
    _collectLengths(_root, 0, lengths);

    if (lengths.length == 1) {
      final symbol = lengths.keys.single;
      return Map<int, String>.unmodifiable(<int, String>{symbol: '0'});
    }

    final sorted = lengths.entries.toList()
      ..sort(
        (left, right) => left.value != right.value
            ? left.value.compareTo(right.value)
            : left.key.compareTo(right.key),
      );

    var codeValue = 0;
    var previousLength = sorted.first.value;
    final result = <int, String>{};

    for (final entry in sorted) {
      final length = entry.value;
      if (length > previousLength) {
        codeValue <<= length - previousLength;
      }

      result[entry.key] = codeValue.toRadixString(2).padLeft(length, '0');
      codeValue += 1;
      previousLength = length;
    }

    return Map<int, String>.unmodifiable(result);
  }

  /// Decode exactly [count] symbols from a bit string.
  ///
  /// The DT27 spec intentionally allows `decodeAll('', 1)` for a single-leaf
  /// tree. The symbol is already known at the root, so the implementation emits
  /// it and only consumes a `'0'` bit when one is present.
  List<int> decodeAll(String bits, int count) {
    if (count < 0)
      throw RangeError.value(count, 'count', 'Must be non-negative.');

    final result = <int>[];
    var current = _root;
    var position = 0;
    final singleLeaf = _root is Leaf;

    while (result.length < count) {
      if (current is Leaf) {
        result.add(current.symbol);
        current = _root;
        if (singleLeaf && position < bits.length) {
          position += 1;
        }
        continue;
      }

      if (position >= bits.length) {
        throw FormatException(
          'Bit stream exhausted after ${result.length} symbols; expected $count.',
        );
      }

      final bit = bits[position];
      if (bit != '0' && bit != '1') {
        throw FormatException(
          'Bit stream must contain only 0 and 1 characters; got "$bit".',
        );
      }

      position += 1;
      final branch = current as Internal;
      current = bit == '0' ? branch.left : branch.right;
    }

    return List<int>.unmodifiable(result);
  }

  /// Sum of all leaf frequencies.
  int weight() => _root.weight;

  /// Maximum depth among all leaves.
  int depth() => _maxDepth(_root, 0);

  /// Number of distinct symbols in the tree.
  int symbolCount() => _symbolCount;

  /// Leaves listed left-to-right alongside their walk-based codes.
  List<(int, String)> leaves() {
    final table = codeTable();
    final result = <(int, String)>[];
    _inOrderLeaves(_root, result, table);
    return List<(int, String)>.unmodifiable(result);
  }

  /// Check the core DT27 invariants.
  ///
  /// This is primarily a testing helper:
  /// - internal nodes must be full binary nodes
  /// - internal weights must equal the sum of their children
  /// - no symbol may appear more than once
  /// - the stored symbol count must match the actual leaf count
  bool isValid() {
    final seenSymbols = <int>{};
    final leafCount = _validateNode(_root, seenSymbols);
    return leafCount == _symbolCount;
  }
}

void _walkTree(Node node, String prefix, Map<int, String> table) {
  if (node is Leaf) {
    table[node.symbol] = prefix.isEmpty ? '0' : prefix;
    return;
  }

  final branch = node as Internal;
  _walkTree(branch.left, '${prefix}0', table);
  _walkTree(branch.right, '${prefix}1', table);
}

String? _findCode(Node node, int symbol, String prefix) {
  if (node is Leaf) {
    return node.symbol == symbol ? (prefix.isEmpty ? '0' : prefix) : null;
  }

  final branch = node as Internal;
  final left = _findCode(branch.left, symbol, '${prefix}0');
  return left ?? _findCode(branch.right, symbol, '${prefix}1');
}

void _collectLengths(Node node, int depth, Map<int, int> lengths) {
  if (node is Leaf) {
    lengths[node.symbol] = depth == 0 ? 1 : depth;
    return;
  }

  final branch = node as Internal;
  _collectLengths(branch.left, depth + 1, lengths);
  _collectLengths(branch.right, depth + 1, lengths);
}

int _maxDepth(Node node, int depth) {
  if (node is Leaf) {
    return depth;
  }

  final branch = node as Internal;
  final leftDepth = _maxDepth(branch.left, depth + 1);
  final rightDepth = _maxDepth(branch.right, depth + 1);
  return leftDepth > rightDepth ? leftDepth : rightDepth;
}

void _inOrderLeaves(
  Node node,
  List<(int, String)> result,
  Map<int, String> table,
) {
  if (node is Leaf) {
    result.add((node.symbol, table[node.symbol]!));
    return;
  }

  final branch = node as Internal;
  _inOrderLeaves(branch.left, result, table);
  _inOrderLeaves(branch.right, result, table);
}

int _validateNode(Node node, Set<int> seenSymbols) {
  if (node is Leaf) {
    return seenSymbols.add(node.symbol) ? 1 : -1;
  }

  final branch = node as Internal;
  final leftLeaves = _validateNode(branch.left, seenSymbols);
  if (leftLeaves < 0) return -1;

  final rightLeaves = _validateNode(branch.right, seenSymbols);
  if (rightLeaves < 0) return -1;

  if (branch.weight != branch.left.weight + branch.right.weight) return -1;

  return leftLeaves + rightLeaves;
}

final class _HeapEntry {
  const _HeapEntry._({
    required this.weight,
    required this.isInternal,
    required this.symbolOrMax,
    required this.orderOrMax,
    required this.node,
  });

  factory _HeapEntry.leaf(Leaf leaf) {
    return _HeapEntry._(
      weight: leaf.weight,
      isInternal: 0,
      symbolOrMax: leaf.symbol,
      orderOrMax: _maxSentinel,
      node: leaf,
    );
  }

  factory _HeapEntry.internal(Internal internal) {
    return _HeapEntry._(
      weight: internal.weight,
      isInternal: 1,
      symbolOrMax: _maxSentinel,
      orderOrMax: internal.order,
      node: internal,
    );
  }

  final int weight;
  final int isInternal;
  final int symbolOrMax;
  final int orderOrMax;
  final Node node;
}

const int _maxSentinel = 1 << 30;

int _compareHeapEntries(_HeapEntry left, _HeapEntry right) {
  if (left.weight != right.weight) {
    return left.weight.compareTo(right.weight);
  }
  if (left.isInternal != right.isInternal) {
    return left.isInternal.compareTo(right.isInternal);
  }
  if (left.symbolOrMax != right.symbolOrMax) {
    return left.symbolOrMax.compareTo(right.symbolOrMax);
  }
  return left.orderOrMax.compareTo(right.orderOrMax);
}

/// Tiny binary min-heap used only inside tree construction.
///
/// The Dart lane does not yet have a shared heap package, so DT27 keeps the
/// priority queue local and intentionally small.
final class _MinHeap<T> {
  _MinHeap(this._compare);

  final int Function(T left, T right) _compare;
  final List<T> _values = <T>[];

  int get length => _values.length;

  void push(T value) {
    _values.add(value);
    _bubbleUp(_values.length - 1);
  }

  T pop() {
    if (_values.isEmpty) throw StateError('Cannot pop from an empty heap.');

    final first = _values.first;
    final last = _values.removeLast();
    if (_values.isNotEmpty) {
      _values[0] = last;
      _bubbleDown(0);
    }
    return first;
  }

  void _bubbleUp(int index) {
    var current = index;
    while (current > 0) {
      final parent = (current - 1) ~/ 2;
      if (_compare(_values[current], _values[parent]) >= 0) {
        return;
      }
      final temporary = _values[current];
      _values[current] = _values[parent];
      _values[parent] = temporary;
      current = parent;
    }
  }

  void _bubbleDown(int index) {
    var current = index;
    while (true) {
      final left = current * 2 + 1;
      final right = left + 1;
      var smallest = current;

      if (left < _values.length &&
          _compare(_values[left], _values[smallest]) < 0) {
        smallest = left;
      }
      if (right < _values.length &&
          _compare(_values[right], _values[smallest]) < 0) {
        smallest = right;
      }
      if (smallest == current) {
        return;
      }

      final temporary = _values[current];
      _values[current] = _values[smallest];
      _values[smallest] = temporary;
      current = smallest;
    }
  }
}
