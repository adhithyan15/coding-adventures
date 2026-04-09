# coding-adventures-trie

Prefix tree (trie) mapping string keys to values with O(k) insert, search, and prefix queries.

## What It Is

A trie (from re**trie**val, pronounced "try") is a tree where each path from the root to
a node spells out a string prefix. Unlike a hash map, a trie physically shares common
prefixes among all keys that begin the same way.

This makes prefix operations trivial:
- "Find all words starting with 'app'" navigates to the 'app' node in O(3), then collects
  all words in that subtree — instead of scanning all n keys like a hash map would.

## When to Use

Use a trie when:
- You need autocomplete / prefix search
- You need to find the longest stored key that is a prefix of a given string (IP routing)
- You want to check quickly whether any stored key starts with a given prefix

Use a hash map instead when you only need exact lookups and don't care about prefixes.

## Installation

```bash
pip install coding-adventures-trie
```

## Usage

```python
from trie import Trie

t: Trie[int] = Trie()
t.insert("app", 1)
t.insert("apple", 2)
t.insert("apply", 3)
t.insert("apt", 4)
t.insert("banana", 5)

# Exact search
t.search("app")         # → 1
t.search("ap")          # → None (no complete key "ap")

# Prefix check
t.starts_with("app")    # → True
t.starts_with("xyz")    # → False

# Autocomplete (lexicographic order)
t.words_with_prefix("app")  # → [("app", 1), ("apple", 2), ("apply", 3)]

# Longest prefix match (IP routing, URL dispatch)
t.insert("192", "iface0")
t.insert("192.168", "iface1")
t.insert("192.168.1", "iface2")
t.longest_prefix_match("192.168.1.5")  # → ("192.168.1", "iface2")
t.longest_prefix_match("192.168.2.1")  # → ("192.168", "iface1")

# Delete
t.delete("app")    # returns True; "apple" and "apply" still exist
t.delete("xyz")    # returns False (not found)

# Dict-like interface
t["new_key"] = 99
val = t["new_key"]    # → 99
del t["new_key"]
"apple" in t          # → True

# Iteration (lexicographic order)
list(t)               # all keys sorted
list(t.items())       # all (key, value) pairs sorted

# Size
len(t)                # number of unique keys
bool(t)               # True if non-empty
```

## API

| Method | Time | Description |
|--------|------|-------------|
| `insert(key, value)` | O(k) | Insert or update key |
| `search(key)` | O(k) | Exact match; returns value or None |
| `delete(key)` | O(k) | Remove key; returns False if not found |
| `starts_with(prefix)` | O(p) | True if any key starts with prefix |
| `words_with_prefix(prefix)` | O(p + results) | All keys with prefix, sorted |
| `longest_prefix_match(s)` | O(k) | Longest stored key that is prefix of s |
| `all_words()` | O(n·k) | All (key, value) pairs sorted |
| `len(t)` | O(1) | Number of unique keys |
| `t[key]`, `t[key] = v`, `del t[key]` | O(k) | Dict-like access |
| `key in t` | O(k) | Membership test |
| `iter(t)`, `t.items()` | O(n·k) | Iterate keys / pairs |
| `is_valid()` | O(n·k) | Verify internal invariants (testing) |

Where k = key length, p = prefix length, n = number of keys.

## How It Works

Each node in the trie is `_TrieNode(children: dict[str, _TrieNode], is_end: bool, value)`.
The dict-based design (vs a fixed 26-slot array) uses less memory and supports any
character set including Unicode.

Deletion prunes back any now-useless nodes (no children and not a word endpoint) to keep
memory bounded.

## Layer Position (DT Series)

```
DT02: tree
DT13: trie          ← [THIS PACKAGE]
  └── DT14: radix-tree  (compressed trie)
        └── DT15: suffix-tree
```

## Running Tests

```bash
uv run python -m pytest tests/ -v
```
