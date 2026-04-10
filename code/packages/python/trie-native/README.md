# coding-adventures-trie-native

Rust-backed trie for Python via the repo's zero-dependency `python-bridge`.

## What It Provides

- A native `Trie` class backed by the Rust [trie](../../rust/trie/) crate
- Arbitrary Python object values, including `None`
- Prefix operations, longest-prefix match, iteration, and dict-like item access

## Building

```bash
cargo build --release
```

Or run the package `BUILD` script to build and execute the tests.

## Usage

```python
from trie_native import KeyNotFoundError, Trie

t = Trie()
t.insert("app", 1)
t.insert("apple", 2)
t.insert("", "root")

assert t.search("apple") == 2
assert t.starts_with("app")
assert t.words_with_prefix("app") == [("app", 1), ("apple", 2)]
assert t.longest_prefix_match("applepie") == ("apple", 2)

t["banana"] = 3
assert t["banana"] == 3
del t["banana"]
```
