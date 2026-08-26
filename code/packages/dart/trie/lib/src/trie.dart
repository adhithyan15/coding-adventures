/// One key-value endpoint returned by a [Trie].
typedef TrieEntry<V> = (String, V);

const _invalidScalarMessage =
    'Trie input must contain only Unicode scalar values.';

Iterable<int> _checkedRunes(String input) {
  // Validate the complete host string before traversal. This rejects a lone
  // surrogate even when an earlier scalar is already absent from the trie,
  // while returning the SDK's lazy rune iterable avoids a full scalar list.
  for (final scalar in input.runes) {
    if (scalar < 0 ||
        scalar > 0x10ffff ||
        (scalar >= 0xd800 && scalar <= 0xdfff)) {
      throw ArgumentError(_invalidScalarMessage);
    }
  }
  return input.runes;
}

/// A node in the prefix tree.
///
/// The map key is a Unicode scalar value, not a UTF-16 code unit. Keeping
/// endpoint presence in [isEnd] is important: `null` can be a legitimate value
/// when the trie is instantiated as `Trie<T?>`.
final class _TrieNode<V> {
  final Map<int, _TrieNode<V>> children = <int, _TrieNode<V>>{};
  bool isEnd = false;
  V? value;
}

/// One explicit depth-first-search frame used by result enumeration.
///
/// A recursive collector is pleasantly short, but a trie can be as deep as its
/// longest key. The explicit frame stack keeps valid long keys independent of
/// the host call-stack limit while a single mutable [path] stores the current
/// scalar sequence.
final class _TraversalFrame<V> {
  _TraversalFrame(this.node)
      : childScalars = node.children.keys.toList(growable: false)..sort();

  final _TrieNode<V> node;
  final List<int> childScalars;
  var nextChild = 0;
  var emittedEndpoint = false;
}

/// A mutable prefix tree mapping exact string keys to values.
///
/// Each edge owns one Unicode scalar value. Keys are neither normalized nor
/// locale-collated, so precomposed and decomposed spellings remain distinct.
/// Enumeration follows numeric scalar order and emits a prefix before any of
/// its descendants.
final class Trie<V> {
  final _TrieNode<V> _root = _TrieNode<V>();
  var _count = 0;

  /// Number of complete keys stored in the trie.
  int get count => _count;

  /// Whether the trie contains no complete keys.
  bool get isEmpty => _count == 0;

  /// Insert [key] with [value], replacing an existing value in place.
  ///
  /// The empty string is a valid key and terminates at the root. Inserting an
  /// existing endpoint does not change [count].
  void insert(String key, V value) {
    // Validate the complete key before changing any node so malformed UTF-16
    // cannot leave a partially inserted path behind.
    final scalars = _checkedRunes(key).toList(growable: false);
    var node = _root;
    for (final scalar in scalars) {
      node = node.children.putIfAbsent(scalar, _TrieNode<V>.new);
    }

    if (!node.isEnd) {
      node.isEnd = true;
      _count++;
    }
    node.value = value;
  }

  /// Return the value stored at [key], or `null` when no endpoint exists.
  ///
  /// Use [containsKey] when `V` itself is nullable and absence must be
  /// distinguished from a present `null` value.
  V? search(String key) {
    final node = _findNode(key);
    return node == null || !node.isEnd ? null : node.value;
  }

  /// Whether [key] is a complete stored key.
  bool containsKey(String key) => _findNode(key)?.isEnd ?? false;

  /// Return the value at [key], throwing when the key is absent.
  V operator [](String key) {
    final node = _findNode(key);
    if (node == null || !node.isEnd) {
      throw StateError('Trie key not found.');
    }
    return node.value as V;
  }

  /// Insert or replace [key] with [value].
  void operator []=(String key, V value) => insert(key, value);

  /// Remove [key] and prune nodes that no longer serve another endpoint.
  ///
  /// Returns `true` when a key was removed and `false` for a missing-key no-op.
  bool delete(String key) {
    final nodes = <_TrieNode<V>>[_root];
    final scalars = <int>[];
    var node = _root;

    for (final scalar in _checkedRunes(key)) {
      final child = node.children[scalar];
      if (child == null) return false;
      scalars.add(scalar);
      nodes.add(child);
      node = child;
    }
    if (!node.isEnd) return false;

    node
      ..isEnd = false
      ..value = null;
    _count--;

    for (var index = scalars.length - 1; index >= 0; index--) {
      final child = nodes[index + 1];
      if (child.isEnd || child.children.isNotEmpty) break;
      nodes[index].children.remove(scalars[index]);
    }
    return true;
  }

  /// Whether at least one stored key begins with [prefix].
  ///
  /// Every string begins with the empty prefix, but an empty trie contains no
  /// keys; consequently `startsWith('')` is equivalent to `count > 0`.
  bool startsWith(String prefix) {
    if (prefix.isEmpty) return _count > 0;
    return _findNode(prefix) != null;
  }

  /// All endpoints beginning with [prefix], in Unicode-scalar order.
  List<TrieEntry<V>> wordsWithPrefix(String prefix) {
    final path = <int>[];
    var node = _root;
    for (final scalar in _checkedRunes(prefix)) {
      final child = node.children[scalar];
      if (child == null) {
        return List<TrieEntry<V>>.unmodifiable(<TrieEntry<V>>[]);
      }
      path.add(scalar);
      node = child;
    }
    return _collect(node, path);
  }

  /// All endpoints in Unicode-scalar order.
  List<TrieEntry<V>> allWords() => wordsWithPrefix('');

  /// A sorted snapshot of every stored key.
  List<String> get keys => List<String>.unmodifiable(
        allWords().map((entry) => entry.$1),
      );

  /// A sorted snapshot of every stored key-value endpoint.
  List<TrieEntry<V>> get entries => allWords();

  /// Return the deepest stored key that prefixes [text].
  TrieEntry<V>? longestPrefixMatch(String text) {
    final traversedScalars = <int>[];
    var node = _root;
    var found = node.isEnd;
    var matchedLength = 0;
    V? matchedValue = node.value;

    for (final scalar in _checkedRunes(text)) {
      final child = node.children[scalar];
      if (child == null) break;
      traversedScalars.add(scalar);
      node = child;
      if (node.isEnd) {
        found = true;
        matchedLength = traversedScalars.length;
        matchedValue = node.value;
      }
    }

    if (!found) return null;
    return (
      String.fromCharCodes(traversedScalars.take(matchedLength)),
      matchedValue as V,
    );
  }

  /// Verify endpoint counts, scalar edges, and pruning invariants.
  ///
  /// This intentionally avoids recursion so validation can examine the same
  /// long keys accepted by the public operations.
  bool isValid() {
    final stack = <(_TrieNode<V>, bool)>[(_root, true)];
    var endpoints = 0;

    while (stack.isNotEmpty) {
      final current = stack.removeLast();
      final node = current.$1;
      final isRoot = current.$2;

      if (node.isEnd) {
        endpoints++;
      } else {
        if (node.value != null) return false;
        if (!isRoot && node.children.isEmpty) return false;
      }

      for (final child in node.children.entries) {
        final scalar = child.key;
        if (scalar < 0 ||
            scalar > 0x10ffff ||
            (scalar >= 0xd800 && scalar <= 0xdfff)) {
          return false;
        }
        stack.add((child.value, false));
      }
    }

    return endpoints == _count;
  }

  _TrieNode<V>? _findNode(String key) {
    var node = _root;
    for (final scalar in _checkedRunes(key)) {
      final child = node.children[scalar];
      if (child == null) return null;
      node = child;
    }
    return node;
  }

  List<TrieEntry<V>> _collect(_TrieNode<V> start, List<int> path) {
    final results = <TrieEntry<V>>[];
    final stack = <_TraversalFrame<V>>[_TraversalFrame<V>(start)];

    while (stack.isNotEmpty) {
      final frame = stack.last;
      if (!frame.emittedEndpoint) {
        frame.emittedEndpoint = true;
        if (frame.node.isEnd) {
          results.add((String.fromCharCodes(path), frame.node.value as V));
        }
      }

      if (frame.nextChild == frame.childScalars.length) {
        stack.removeLast();
        if (stack.isNotEmpty) path.removeLast();
        continue;
      }

      final scalar = frame.childScalars[frame.nextChild++];
      path.add(scalar);
      stack.add(_TraversalFrame<V>(frame.node.children[scalar]!));
    }

    return List<TrieEntry<V>>.unmodifiable(results);
  }

  /// A structural summary that never exposes stored keys or values.
  @override
  String toString() => 'Trie(size: $_count)';
}
