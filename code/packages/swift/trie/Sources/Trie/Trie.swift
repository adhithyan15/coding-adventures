// ============================================================================
// Trie.swift — Prefix tree (trie) for string keys with prefix operations
// ============================================================================
//
// A trie stores string keys as paths through a tree: each edge is labelled with
// one character, and a node flagged `isEnd` marks where a stored key finishes.
// This makes prefix questions cheap — "does any key start with `app`?" is just
// a walk to the `app` node — which is why tries back autocomplete, routers, and
// dictionaries.
//
// This is the Swift port of the `trie` package in the coding-adventures
// monorepo. Like the Rust reference (which keys on `char`, i.e. Unicode
// scalars), this trie keys on `Unicode.Scalar` rather than `Character`
// (grapheme clusters) — so a combining sequence like "e" + U+0301 and the
// precomposed "é" (U+00E9) remain distinct keys, exactly as in the reference.
// Children are visited in scalar order, so `keys()`, `allWords()`, and
// `wordsWithPrefix(_:)` come back lexicographically sorted.

/// A prefix tree mapping `String` keys to values of type `Value`.
public struct Trie<Value> {

    // A node is a value type; a Dictionary of the same struct type is allowed in
    // Swift because Dictionary provides the heap indirection.
    private struct Node {
        var children: [Unicode.Scalar: Node] = [:]
        var isEnd = false
        var value: Value?
    }

    private var root = Node()
    private var size = 0

    /// Create an empty trie.
    public init() {}

    // ── Mutation ─────────────────────────────────────────────────────────────

    /// Insert `key` with `value`, overwriting any existing value for `key`.
    public mutating func insert(_ key: String, value: Value) {
        let scalars = Array(key.unicodeScalars)
        if Self.insert(&root, scalars, 0, value) { size += 1 }
    }

    private static func insert(_ node: inout Node, _ scalars: [Unicode.Scalar],
                               _ depth: Int, _ value: Value) -> Bool {
        if depth == scalars.count {
            let wasNew = !node.isEnd
            node.isEnd = true
            node.value = value
            return wasNew
        }
        let ch = scalars[depth]
        var child = node.children[ch] ?? Node()
        let wasNew = insert(&child, scalars, depth + 1, value)
        node.children[ch] = child
        return wasNew
    }

    /// Remove `key`. Returns `true` if it was present, pruning now-empty nodes.
    @discardableResult
    public mutating func delete(_ key: String) -> Bool {
        guard keyExists(key) else { return false }
        let scalars = Array(key.unicodeScalars)
        _ = Self.delete(&root, scalars, 0)
        size -= 1
        return true
    }

    private static func delete(_ node: inout Node, _ scalars: [Unicode.Scalar],
                               _ depth: Int) -> Bool {
        if depth == scalars.count {
            node.isEnd = false
            node.value = nil
            return node.children.isEmpty
        }
        let ch = scalars[depth]
        if var child = node.children[ch] {
            if delete(&child, scalars, depth + 1) {
                node.children[ch] = nil
            } else {
                node.children[ch] = child
            }
        }
        return node.children.isEmpty && !node.isEnd
    }

    // ── Lookup ───────────────────────────────────────────────────────────────

    /// The value stored for exactly `key`, or `nil` if `key` is not a stored key.
    public func search(_ key: String) -> Value? {
        guard let node = findNode(key), node.isEnd else { return nil }
        return node.value
    }

    /// Whether `key` is a stored key (an exact match, not merely a prefix).
    public func containsKey(_ key: String) -> Bool { keyExists(key) }

    /// Whether any stored key begins with `prefix`. The empty prefix matches
    /// whenever the trie is non-empty.
    public func startsWith(_ prefix: String) -> Bool {
        if prefix.isEmpty { return size > 0 }
        return findNode(prefix) != nil
    }

    /// The most specific stored key that is a prefix of `string`, with its value
    /// (or `nil` if no stored key is a prefix of `string`).
    public func longestPrefixMatch(_ string: String) -> (String, Value)? {
        var node = root
        var best: (String, Value)? = node.isEnd ? node.value.map { ("", $0) } : nil
        var current = String.UnicodeScalarView()
        for ch in string.unicodeScalars {
            guard let next = node.children[ch] else { break }
            current.append(ch)
            node = next
            if node.isEnd, let value = node.value {
                best = (String(current), value)
            }
        }
        return best
    }

    // ── Enumeration (lexicographically sorted) ───────────────────────────────

    /// Every stored key beginning with `prefix`, paired with its value, sorted.
    public func wordsWithPrefix(_ prefix: String) -> [(String, Value)] {
        guard let node = findNode(prefix) else { return [] }
        var results: [(String, Value)] = []
        Self.collect(node, Array(prefix.unicodeScalars), &results)
        return results
    }

    /// Every stored key paired with its value, sorted.
    public func allWords() -> [(String, Value)] {
        var results: [(String, Value)] = []
        Self.collect(root, [], &results)
        return results
    }

    /// Every stored key, sorted.
    public func keys() -> [String] { allWords().map { $0.0 } }

    private static func collect(_ node: Node, _ current: [Unicode.Scalar],
                                _ results: inout [(String, Value)]) {
        if node.isEnd, let value = node.value {
            results.append((scalarsToString(current), value))
        }
        for ch in node.children.keys.sorted(by: { $0.value < $1.value }) {
            collect(node.children[ch]!, current + [ch], &results)
        }
    }

    // ── Introspection ────────────────────────────────────────────────────────

    /// The number of stored keys.
    public var count: Int { size }

    /// Whether the trie stores no keys.
    public var isEmpty: Bool { size == 0 }

    /// Consistency check: the number of `isEnd` nodes equals the tracked size.
    public func isValid() -> Bool { Self.countEndpoints(root) == size }

    // ── Internals ────────────────────────────────────────────────────────────

    private func findNode(_ key: String) -> Node? {
        var node = root
        for ch in key.unicodeScalars {
            guard let next = node.children[ch] else { return nil }
            node = next
        }
        return node
    }

    private func keyExists(_ key: String) -> Bool { findNode(key)?.isEnd ?? false }

    private static func countEndpoints(_ node: Node) -> Int {
        var count = node.isEnd ? 1 : 0
        for child in node.children.values { count += countEndpoints(child) }
        return count
    }

    private static func scalarsToString(_ scalars: [Unicode.Scalar]) -> String {
        var view = String.UnicodeScalarView()
        view.append(contentsOf: scalars)
        return String(view)
    }
}

extension Trie: CustomStringConvertible {
    public var description: String {
        let preview = allWords().prefix(5).map { $0.0 }
        return "Trie(\(size) keys: \(preview))"
    }
}
