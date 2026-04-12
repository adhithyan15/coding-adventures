# coding-adventures-radix-tree-native

Rust-backed radix tree for Python via the repo's zero-dependency `python-bridge`.

## What It Provides

- A native `RadixTree` class backed by the Rust [radix-tree](../../rust/radix-tree/) crate
- Arbitrary Python object values
- Prefix queries, longest-prefix match, iteration, and `to_dict`

## Usage

```python
from radix_tree_native import RadixTree

tree = RadixTree()
tree.insert("app", 1)
tree.insert("apple", 2)

assert tree.search("apple") == 2
assert tree.words_with_prefix("app") == ["app", "apple"]
assert tree.longest_prefix_match("applepie") == "apple"
assert tree.to_dict() == {"app": 1, "apple": 2}
```
