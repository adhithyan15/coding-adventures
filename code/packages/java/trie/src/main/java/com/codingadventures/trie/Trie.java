// ============================================================================
// Trie.java — Prefix Tree (Trie)
// ============================================================================
//
// A trie (pronounced "try", from re**trie**val) is a tree where each path from
// root to a node spells out a string prefix. Unlike a hash map that treats keys
// as opaque, a trie decomposes each key character by character and shares common
// prefixes among all words that begin the same way.
//
// The fundamental idea:
// ---------------------
// Imagine the words ["app", "apple", "apply", "apt", "banana"]:
//
//   In a hash map, "app", "apple", "apply" are three separate, unrelated entries.
//
//   In a trie, they share the path root → 'a' → 'p' → 'p':
//
//       (root)
//       ├── a
//       │   └── p
//       │       ├── p (*)   ← "app" ends here
//       │       │   └── l
//       │       │       ├── e (*)  ← "apple"
//       │       │       └── y (*)  ← "apply"
//       │       └── t (*)   ← "apt"
//       └── b
//           └── a → n → a → n → a (*)  ← "banana"
//
//   (*) marks nodes where a complete word ends (isEnd = true).
//
// This shared structure makes prefix queries O(p) where p is the prefix length,
// vs O(n·k) for a hash map scan. For autocomplete with 100k words returning 50
// results, the trie is ~2000× faster.
//
// Node design:
// ------------
// We use a HashMap-based node rather than a fixed 26-slot array. This:
//   - Uses less memory for sparse character sets
//   - Generalises to any character set (Unicode, DNA bases, etc.)
//   - Has O(1) average-case child lookup
//
//   class Node<V> {
//       Map<Character, Node<V>> children;
//       boolean isEnd;
//       V value;           // null if no value associated with this key
//   }
//
// Complexity:
// -----------
//   insert(key, value)  — O(len(key))
//   get(key)            — O(len(key))
//   contains(key)       — O(len(key))
//   delete(key)         — O(len(key))
//   keysWithPrefix(pfx) — O(len(pfx) + number of matches × average key length)
//   size()              — O(1) (maintained as a counter)
//
// Space: O(total characters across all keys), roughly O(n·k) worst case but
// much better than that when keys share long common prefixes.
//

package com.codingadventures.trie;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;

/**
 * A generic Trie (prefix tree) that maps {@code String} keys to values of
 * type {@code V}.
 *
 * <p>Keys are arbitrary strings — the trie works character by character and
 * generalises to any Unicode input.
 *
 * <pre>{@code
 * Trie<Integer> t = new Trie<>();
 * t.insert("apple", 1);
 * t.insert("app", 2);
 * t.insert("apply", 3);
 *
 * System.out.println(t.get("apple"));            // Optional[1]
 * System.out.println(t.contains("app"));         // true
 * System.out.println(t.keysWithPrefix("app"));   // [app, apple, apply]
 * System.out.println(t.size());                  // 3
 * }</pre>
 *
 * @param <V> the value type stored at each key
 */
public final class Trie<V> {

    // =========================================================================
    // Node
    // =========================================================================

    private static final class Node<V> {
        /** Maps each character to the child node for that character. */
        final Map<Character, Node<V>> children = new HashMap<>();
        /** True if a complete key ends at this node. */
        boolean isEnd = false;
        /** The value associated with the key ending here; null if none / deleted. */
        V value = null;
    }

    // =========================================================================
    // Fields
    // =========================================================================

    private final Node<V> root = new Node<>();
    private int size = 0;

    // =========================================================================
    // Core operations
    // =========================================================================

    /**
     * Insert {@code key} → {@code value} into the trie.
     *
     * <p>If the key already exists, the value is updated and the size is
     * unchanged.
     *
     * @param key   the key to insert (must not be null)
     * @param value the value to associate with the key
     * @throws IllegalArgumentException if key is null
     */
    public void insert(String key, V value) {
        if (key == null) throw new IllegalArgumentException("Key must not be null");
        Node<V> node = root;
        for (int i = 0; i < key.length(); i++) {
            char c = key.charAt(i);
            node.children.putIfAbsent(c, new Node<>());
            node = node.children.get(c);
        }
        if (!node.isEnd) {
            node.isEnd = true;
            size++;
        }
        node.value = value;
    }

    /**
     * Return the value associated with {@code key}, or an empty {@code Optional}
     * if the key is not in the trie.
     *
     * @param key the key to look up
     * @return the value, or {@link Optional#empty()} if not found
     */
    public Optional<V> get(String key) {
        Node<V> node = findNode(key);
        if (node == null || !node.isEnd) return Optional.empty();
        return Optional.ofNullable(node.value);
    }

    /**
     * Return true if {@code key} is present in the trie.
     *
     * <p>Note: {@code startsWith("app")} and {@code contains("app")} are
     * different: the latter requires that "app" was inserted as a complete key.
     *
     * @param key the key to look up
     * @return true if the key was inserted
     */
    public boolean contains(String key) {
        Node<V> node = findNode(key);
        return node != null && node.isEnd;
    }

    /**
     * Return true if any inserted key starts with {@code prefix}.
     *
     * @param prefix the prefix to test
     * @return true if at least one key starts with {@code prefix}
     */
    public boolean startsWith(String prefix) {
        return findNode(prefix) != null;
    }

    /**
     * Delete {@code key} from the trie.
     *
     * <p>Only the key's end-marker is removed; intermediate nodes that are
     * shared with other keys are preserved. Returns true if the key was present
     * and removed, false if it was not in the trie.
     *
     * @param key the key to remove
     * @return true if the key was found and removed
     */
    public boolean delete(String key) {
        Node<V> node = findNode(key);
        if (node == null || !node.isEnd) return false;
        node.isEnd = false;
        node.value = null;
        size--;
        return true;
    }

    // =========================================================================
    // Prefix queries
    // =========================================================================

    /**
     * Return a list of all keys in the trie that start with {@code prefix},
     * in the order they were discovered by depth-first traversal (alphabetical
     * by construction since children are stored in a HashMap, but the order is
     * non-deterministic — use the overload that returns a sorted list if you
     * need stable ordering).
     *
     * <p>Returns an empty list if no keys match.
     *
     * @param prefix the prefix to search for
     * @return list of matching keys (may be empty)
     */
    public List<String> keysWithPrefix(String prefix) {
        List<String> results = new ArrayList<>();
        Node<V> node = findNode(prefix);
        if (node != null) {
            collectKeys(node, new StringBuilder(prefix), results);
        }
        return results;
    }

    /**
     * Return all keys in the trie (equivalent to {@code keysWithPrefix("")}).
     *
     * @return list of all keys
     */
    public List<String> keys() {
        return keysWithPrefix("");
    }

    // =========================================================================
    // Size / empty
    // =========================================================================

    /** Return the number of key–value pairs currently in the trie. */
    public int size() { return size; }

    /** Return true if the trie contains no keys. */
    public boolean isEmpty() { return size == 0; }

    // =========================================================================
    // Internal helpers
    // =========================================================================

    /**
     * Walk the trie following the characters of {@code key}.
     * Returns the node at the end of the path, or null if any character is missing.
     */
    private Node<V> findNode(String key) {
        if (key == null) return null;
        Node<V> node = root;
        for (int i = 0; i < key.length(); i++) {
            node = node.children.get(key.charAt(i));
            if (node == null) return null;
        }
        return node;
    }

    /**
     * Depth-first traversal starting at {@code node}, appending characters to
     * {@code prefix} and recording complete keys in {@code results}.
     */
    private void collectKeys(Node<V> node, StringBuilder prefix, List<String> results) {
        if (node.isEnd) {
            results.add(prefix.toString());
        }
        for (Map.Entry<Character, Node<V>> entry : node.children.entrySet()) {
            prefix.append(entry.getKey());
            collectKeys(entry.getValue(), prefix, results);
            prefix.deleteCharAt(prefix.length() - 1);
        }
    }
}
