# radix-tree — DT14

A **radix tree** (also called a Patricia trie or compressed trie) is a
space-efficient variant of a trie (DT13) where chains of single-child nodes
are collapsed into single edges carrying multi-character labels.

## What is it?

Consider storing the keys `["search", "searcher", "searching"]`:

```
Trie (DT13) — 14 nodes:
  root → s → e → a → r → c → h (end)
                              ├── e → r (end)
                              └── i → n → g (end)

Radix Tree (DT14) — 4 nodes:
  root → "search" (end)
          ├── "er"  (end)
          └── "ing" (end)
```

The radix tree uses **O(n) nodes** (n = number of keys) instead of **O(total
characters)**. This is crucial for production systems like Redis, which stores
all of its keys in a radix tree (`rax.c`), and HTTP routers such as
gorilla/mux and actix-web.

## How it fits in the stack

```
DT13: trie            ← parent (radix tree compresses tries)
DT14: radix-tree      ← YOU ARE HERE
  └── DT15: suffix-tree  (radix tree of all suffixes of a string)
```

Depends on: `coding-adventures-trie` (DT13).

## Installation

```bash
pip install coding-adventures-radix-tree
```

## Usage

```python
from radix_tree import RadixTree

t: RadixTree[int] = RadixTree()

# Insert
t.insert("search", 1)
t.insert("searcher", 2)
t.insert("searching", 3)

# Exact lookup
t.search("search")    # → 1
t.search("sear")      # → None (not a key)

# Prefix queries
t.starts_with("sear")             # → True
t.words_with_prefix("search")     # → ["search", "searcher", "searching"]

# Longest prefix match (IP routing, URL dispatch)
t.insert("192.168", 10)
t.insert("192.168.1", 20)
t.longest_prefix_match("192.168.1.50")  # → "192.168.1"

# Delete
t.delete("searcher")  # → True
len(t)                # → 2

# Dict-like interface
"search" in t         # → True
list(t)               # → ["search", "searching"]  (sorted)
t.to_dict()           # → {"search": 1, "searching": 3}
```

## Public API

| Method | Description |
|--------|-------------|
| `insert(key, value)` | Store key → value; update if key exists |
| `search(key)` | Exact lookup; returns value or None |
| `delete(key)` | Remove key; returns True/False |
| `starts_with(prefix)` | True if any key starts with prefix |
| `words_with_prefix(prefix)` | All matching keys (sorted) |
| `longest_prefix_match(key)` | Longest stored key that is a prefix of key |
| `__len__()` | Number of stored keys — O(1) |
| `__contains__(key)` | Exact membership test |
| `__iter__()` | Keys in lexicographic order |
| `to_dict()` | Export to plain dict |

## Key Algorithms

### Insert — four cases

When inserting key `K` and an existing edge has label `L`:

| Case | Condition | Action |
|------|-----------|--------|
| 1 | No common prefix | Add `K` as a new edge |
| 2 | `L` is a prefix of `K` | Descend through edge, insert `K[len(L):]` |
| 3 | `K` is a prefix of `L` | Split `L` at `len(K)`: new node for `K` end |
| 4 | Partial overlap | Split at divergence point; two children |

### Delete — merge

After clearing `is_end`, a node with exactly one child and `is_end=False`
is merged with its single child, reversing a Case 3 split.

## Complexity

| Operation | Time |
|-----------|------|
| insert | O(k) |
| search | O(k) |
| delete | O(k) |
| starts_with | O(p) |
| words_with_prefix | O(p + result chars) |
| longest_prefix_match | O(k) |
| len | O(1) |

Where `k` = key length, `p` = prefix length.

Space: O(n · k) — same total characters as trie, but ~1–5 nodes per key
instead of ~k nodes per key.

## Running tests

```bash
uv venv .venv --no-project
uv pip install --python .venv -e ../trie
uv pip install --python .venv -e .[dev]
uv run --no-project python -m pytest tests/ -v
```

Coverage target: **95%+**.
